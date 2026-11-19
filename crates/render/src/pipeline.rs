//! Render pipelines: chunk pipeline, UI pipeline, etc.

use crate::frame::FrameGraph;
use crate::resources::{ChunkVertex, MeshBuffers};
use wgpu::{Device, RenderPass, TextureFormat};

/// Chunk rendering pipeline.
pub struct ChunkPipeline {
    render_pipeline: wgpu::RenderPipeline,
}

impl ChunkPipeline {
    /// Creates a new chunk pipeline.
    pub fn new(device: &Device, format: TextureFormat) -> Result<Self, PipelineError> {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("chunk shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/chunk.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("chunk bind group layout"),
            entries: &[
                // View uniform (bind group 0, set in renderer)
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("chunk pipeline layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = {
            let targets: &[Option<wgpu::ColorTargetState>] = &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })];

            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("chunk pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: "vs_main",
                    buffers: &[ChunkVertex::desc()],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: "fs_main",
                    targets,
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: Some(wgpu::Face::Back),
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
            })
        };

        Ok(Self { render_pipeline })
    }

    /// Renders chunk sections.
    pub fn render<'a>(
        &'a self,
        render_pass: &mut RenderPass<'a>,
        frame_graph: &FrameGraph,
        chunks: &[crate::resources::GpuChunkSection],
    ) {
        render_pass.set_pipeline(&self.render_pipeline);

        for chunk in chunks {
            if chunk.is_empty || chunk.mesh.index_count == 0 {
                continue;
            }
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
    #[error("shader error: {0}")]
    Shader(String),
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
