//! Render pipelines: chunk pipeline, UI pipeline, etc.

use crate::frame::FrameGraph;
use crate::resources::ChunkVertex;
use crate::texture_atlas::TextureAtlas;
use bytemuck;
use wgpu::{Device, RenderPass, TextureFormat};

/// Maximum number of texture atlas regions supported.
const MAX_ATLAS_REGIONS: usize = 512;

/// Maximum chunk sections drawn in one frame.
/// Limited by max uniform buffer binding size (65536) / dynamic offset alignment (256) = 256.
const MAX_CHUNKS_PER_FRAME: usize = 256;

/// Alignment required for dynamic uniform buffer offsets.
const UNIFORM_OFFSET_ALIGNMENT: u64 = 256;

/// Chunk rendering pipeline.
pub struct ChunkPipeline {
    render_pipeline: wgpu::RenderPipeline,
    /// Uniform buffer for atlas UV regions.
    atlas_uv_buffer: wgpu::Buffer,
    /// Bind group for atlas UV regions.
    atlas_uv_bind_group: wgpu::BindGroup,
    /// Uniform buffer for chunk section origin.
    pub chunk_origin_buffer: wgpu::Buffer,
    /// Bind group for chunk section origin.
    chunk_origin_bind_group: wgpu::BindGroup,
}

impl ChunkPipeline {
    /// Creates a new chunk pipeline.
    pub fn new(device: &Device, format: TextureFormat) -> Result<Self, PipelineError> {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("chunk shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/chunk.wgsl").into()),
        });

        // Bind group 0: View uniform
        let view_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("view bind group layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        // Bind group 1: Texture atlas
        let atlas_bind_group_layout = TextureAtlas::create_bind_group_layout(device);

        // Bind group 2: Atlas UV regions
        let atlas_uv_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("atlas uv bind group layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        // Bind group 3: Chunk section origin
        let chunk_origin_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("chunk origin bind group layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        min_binding_size: wgpu::BufferSize::new(16),
                    },
                    count: None,
                }],
            });

        // Create uniform buffer for atlas UV regions (4 f32 per region: u_min, v_min, u_max, v_max)
        let atlas_uv_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("atlas uv regions"),
            size: (MAX_ATLAS_REGIONS * 4 * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let atlas_uv_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("atlas uv bind group"),
            layout: &atlas_uv_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: atlas_uv_buffer.as_entire_binding(),
            }],
        });

        // Create uniform buffer for chunk origin (vec3<i32> = 16 bytes due to alignment).
        // Each chunk section gets its own 256-byte-aligned slot so the origin can be
        // selected per draw with a dynamic offset.
        let chunk_origin_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("chunk origins"),
            size: MAX_CHUNKS_PER_FRAME as u64 * UNIFORM_OFFSET_ALIGNMENT,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let chunk_origin_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("chunk origin bind group"),
            layout: &chunk_origin_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &chunk_origin_buffer,
                    offset: 0,
                    // Size of one ChunkOrigin (vec3<i32> = 16 bytes with alignment)
                    size: Some(wgpu::BufferSize::new(16).unwrap()),
                }),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("chunk pipeline layout"),
            bind_group_layouts: &[
                &view_bind_group_layout,
                &atlas_bind_group_layout,
                &atlas_uv_bind_group_layout,
                &chunk_origin_bind_group_layout,
            ],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("chunk pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[ChunkVertex::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                // TODO: re-enable back-face culling once every face winding in
                // chunk_mesh.rs is verified; several faces are currently inverted.
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });
        Ok(Self {
            render_pipeline,
            atlas_uv_buffer,
            atlas_uv_bind_group,
            chunk_origin_buffer,
            chunk_origin_bind_group,
        })
    }

    /// Updates the atlas UV regions buffer.
    pub fn update_atlas_uvs(&self, queue: &wgpu::Queue, uvs: &[[f32; 4]]) {
        let mut data = vec![0.0f32; MAX_ATLAS_REGIONS * 4];
        for (i, uv) in uvs.iter().enumerate() {
            if i >= MAX_ATLAS_REGIONS {
                break;
            }
            data[i * 4] = uv[0];
            data[i * 4 + 1] = uv[1];
            data[i * 4 + 2] = uv[2];
            data[i * 4 + 3] = uv[3];
        }
        queue.write_buffer(&self.atlas_uv_buffer, 0, bytemuck::cast_slice(&data));
    }

    /// Renders chunk sections.
    /// Updates the chunk origin uniform for each chunk before drawing using dynamic offsets.
    pub fn render<'a>(
        &'a self,
        render_pass: &mut RenderPass<'a>,
        queue: &wgpu::Queue,
        _frame_graph: &FrameGraph,
        chunks: &[crate::resources::GpuChunkSection],
        atlas_bind_group: &wgpu::BindGroup,
    ) {
        render_pass.set_pipeline(&self.render_pipeline);
        // Bind group 0 (view) is set by the renderer
        render_pass.set_bind_group(1, atlas_bind_group, &[]);
        render_pass.set_bind_group(2, &self.atlas_uv_bind_group, &[]);

        // Render all non-empty chunks, updating chunk origin per chunk via dynamic offset
        for (i, chunk) in chunks.iter().enumerate() {
            if chunk.is_empty || chunk.mesh.index_count == 0 {
                continue;
            }

            // Update chunk origin uniform for this chunk at its aligned offset
            let origin: [i32; 3] = [chunk.position.x, chunk.position.y, chunk.position.z];
            let offset = (i as u64) * UNIFORM_OFFSET_ALIGNMENT;
            queue.write_buffer(
                &self.chunk_origin_buffer,
                offset,
                bytemuck::bytes_of(&origin),
            );

            // Bind with dynamic offset for this chunk
            render_pass.set_bind_group(3, &self.chunk_origin_bind_group, &[offset as u32]);

            render_pass.set_vertex_buffer(0, chunk.mesh.vertices.slice(..));
            render_pass.set_index_buffer(chunk.mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..chunk.mesh.index_count, 0, 0..1);
        }
    }
}

impl ChunkVertex {
    /// Returns the vertex buffer layout descriptor.
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<ChunkVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Uint32, // position
                },
                wgpu::VertexAttribute {
                    offset: 4,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Uint32, // normal_ao
                },
                wgpu::VertexAttribute {
                    offset: 8,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Uint32, // tex_coord
                },
            ],
        }
    }
}

/// Pipeline creation errors.
#[derive(Debug, thiserror::Error)]
pub enum PipelineError {
    /// Shader compilation or loading error.
    #[error("shader error: {0}")]
    Shader(String),
    /// Pipeline creation failed.
    #[error("pipeline creation failed: {0}")]
    Creation(#[from] wgpu::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vertex_desc_is_correct() {
        let desc = ChunkVertex::desc();
        assert_eq!(desc.array_stride, 12); // 3 x u32
        assert_eq!(desc.attributes.len(), 3);
    }
}
