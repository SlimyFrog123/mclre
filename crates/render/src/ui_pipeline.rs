//! UI rendering pipeline.

use bytemuck::{Pod, Zeroable};
use std::sync::Arc;
use wgpu::{Device, Queue, RenderPass, TextureFormat};

/// Maximum number of UI draw calls per frame.
const MAX_UI_DRAW_CALLS: usize = 1024;

/// UI vertex for 2D rendering.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct UiVertex {
    pub position: [f32; 2],
    pub uv: [f32; 2],
    pub color: u32,
}

/// UI draw call for batching.
#[derive(Copy, Clone, Debug)]
pub struct UiDrawCall {
    pub vertex_offset: u32,
    pub vertex_count: u32,
    pub index_count: u32,
    pub texture_index: u32, // 0 = atlas, 1 = font atlas
    pub pipeline_type: u32, // 0 = textured quad, 1 = nine-slice, 2 = text
}

/// UI pipeline for rendering UI elements.
pub struct UiPipeline {
    render_pipeline: wgpu::RenderPipeline,
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    uniform_bind_group_layout: wgpu::BindGroupLayout,
    texture_bind_group: wgpu::BindGroup,
    texture_bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    vertex_data: Vec<UiVertex>,
    index_data: Vec<u32>,
    draw_calls: Vec<UiDrawCall>,
    vertex_count: usize,
    index_count: usize,
    max_vertices: usize,
    max_indices: usize,
    // Font atlas for text rendering
    font_atlas_texture: Option<wgpu::Texture>,
    font_atlas_view: Option<wgpu::TextureView>,
    // Atlas texture (block/item textures)
    atlas_texture: Option<wgpu::Texture>,
    atlas_view: Option<wgpu::TextureView>,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct UiUniform {
    screen_size: [f32; 2],
    _padding: [f32; 2],
}

impl UiPipeline {
    /// Creates a new UI pipeline.
    pub fn new(
        device: Arc<Device>,
        queue: Arc<Queue>,
        format: TextureFormat,
    ) -> Result<Self, PipelineError> {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("UI shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/ui.wgsl").into()),
        });

        // Uniform bind group layout
        let uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("UI uniform bind group layout"),
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

        // Texture bind group layout (for atlas and font atlas)
        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("UI texture bind group layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("UI pipeline layout"),
            bind_group_layouts: &[&uniform_bind_group_layout, &texture_bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("UI render pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<UiVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float32x2,
                        },
                        wgpu::VertexAttribute {
                            offset: 8,
                            shader_location: 1,
                            format: wgpu::VertexFormat::Float32x2,
                        },
                        wgpu::VertexAttribute {
                            offset: 16,
                            shader_location: 2,
                            format: wgpu::VertexFormat::Uint32,
                        },
                    ],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: TextureFormat::Bgra8UnormSrgb,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::Always,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // Create uniform buffer
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("UI uniform buffer"),
            size: std::mem::size_of::<UiUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create bind group
        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("UI uniform bind group"),
            layout: &uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        // Create texture bind group layout (for atlas and font atlas)
        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("UI texture bind group layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        // Create placeholder textures for atlas and font atlas (will be replaced when set_atlas/set_font_atlas is called)
        let placeholder_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("UI placeholder texture"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let placeholder_view =
            placeholder_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let placeholder_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("UI placeholder sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("UI sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        // Create texture bind group with placeholder textures
        let texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("UI texture bind group"),
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&placeholder_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&placeholder_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        // Create vertex and index buffers
        let max_vertices = 65536;
        let max_indices = 65536 * 6 / 4; // 6 indices per quad

        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("UI vertex buffer"),
            size: (std::mem::size_of::<UiVertex>() * 65536) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("UI index buffer"),
            size: (std::mem::size_of::<u32>() * 65536 * 6 / 4) as u64,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Ok(Self {
            render_pipeline,
            device,
            queue,
            vertex_buffer,
            index_buffer,
            uniform_buffer,
            uniform_bind_group,
            uniform_bind_group_layout,
            texture_bind_group,
            texture_bind_group_layout,
            sampler,
            vertex_data: Vec::with_capacity(65536),
            index_data: Vec::with_capacity(65536 * 6 / 4),
            draw_calls: Vec::with_capacity(MAX_UI_DRAW_CALLS),
            vertex_count: 0,
            index_count: 0,
            max_vertices: 65536,
            max_indices: 65536 * 6 / 4,
            font_atlas_texture: None,
            font_atlas_view: None,
            atlas_texture: None,
            atlas_view: None,
        })
    }

    /// Begin a new frame - clear buffers.
    pub fn begin_frame(&mut self) {
        self.vertex_data.clear();
        self.index_data.clear();
        self.draw_calls.clear();
        self.vertex_count = 0;
        self.index_count = 0;
    }

    /// Set the UI atlas texture.
    pub fn set_atlas(&mut self, texture: &wgpu::Texture) {
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        self.atlas_view = Some(view);
        self.update_texture_bind_group();
    }

    /// Set the font atlas texture.
    pub fn set_font_atlas(&mut self, texture: &wgpu::Texture) {
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        self.font_atlas_view = Some(view);
        self.update_texture_bind_group();
    }

    /// Update the texture bind group with current atlas textures.
    fn update_texture_bind_group(&mut self) {
        // Use placeholder view if atlas not set yet
        let atlas_view = self.atlas_view.as_ref().unwrap_or_else(|| {
            // This should not happen in normal operation, but handle gracefully
            static PLACEHOLDER: std::sync::OnceLock<wgpu::TextureView> = std::sync::OnceLock::new();
            PLACEHOLDER.get_or_init(|| {
                let placeholder_texture = self.device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("UI placeholder texture"),
                    size: wgpu::Extent3d {
                        width: 1,
                        height: 1,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                    view_formats: &[],
                });
                placeholder_texture.create_view(&wgpu::TextureViewDescriptor::default())
            })
        });

        let font_view = self.font_atlas_view.as_ref().unwrap_or_else(|| {
            // Same placeholder for font atlas
            atlas_view
        });

        self.texture_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("UI texture bind group"),
            layout: &self.texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(atlas_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(font_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });
    }

    /// Add a textured quad.
    pub fn add_quad(
        &mut self,
        rect: [f32; 4], // x, y, width, height
        uv: [f32; 4],   // u0, v0, u1, v1
        color: u32,
        texture_index: u32,
    ) {
        let _base_vertex = self.vertex_count as u32;

        let [x, y, _w, _h] = rect;
        let [u0, v0, u1, v1] = uv;

        // Vertex order for triangle strip: bottom-left, bottom-right, top-left, top-right

        self.vertex_data.push(UiVertex {
            position: [x, y + rect[3]],
            uv: [u0, v1],
            color,
        });
        self.vertex_data.push(UiVertex {
            position: [x + rect[2], y + rect[3]],
            uv: [u1, v1],
            color,
        });
        self.vertex_data.push(UiVertex {
            position: [x, y],
            uv: [u0, v0],
            color,
        });
        self.vertex_data.push(UiVertex {
            position: [x + rect[2], y],
            uv: [u1, v0],
            color,
        });

        self.vertex_count += 4;

        // Indices for two triangles
        let base = self.vertex_count as u32 - 4;
        self.index_data.extend_from_slice(&[
            base,
            base + 1,
            base + 2,
            base + 2,
            base + 1,
            base + 3,
        ]);
        self.index_count += 6;

        // Track draw call
        if let Some(last) = self.draw_calls.last_mut() {
            if last.texture_index == texture_index && last.pipeline_type == 0 {
                last.vertex_count += 4;
                last.index_count += 6;
            } else {
                self.draw_calls.push(UiDrawCall {
                    vertex_offset: (self.vertex_count - 4) as u32,
                    vertex_count: 4,
                    index_count: 6,
                    texture_index,
                    pipeline_type: 0,
                });
            }
        } else {
            self.draw_calls.push(UiDrawCall {
                vertex_offset: (self.vertex_count - 4) as u32,
                vertex_count: 4,
                index_count: 6,
                texture_index,
                pipeline_type: 0,
            });
        }
    }

    /// Add text using bitmap font.
    /// Each character is rendered as a quad with the appropriate UV from the font atlas.
    pub fn add_text(
        &mut self,
        text: &str,
        x: f32,
        y: f32,
        scale: f32,
        color: u32,
        font: &crate::text::BitmapFont,
    ) {
        let (char_w, char_h) = font.char_size();
        let char_w_f = char_w as f32 * scale;
        let char_h_f = char_h as f32 * scale;

        let mut cursor_x = x;

        for ch in text.chars() {
            if ch == '\n' {
                cursor_x = x;
                // Move down by line height (character height * 1.2 for line spacing)
                continue; // For simplicity, we'll skip newlines for now
            }

            let uv = font.get_uv(ch);

            self.add_quad(
                [cursor_x, y, char_w_f, char_h_f],
                uv,
                color,
                1, // texture_index = 1 for font atlas
            );

            cursor_x += char_w_f;
        }
    }

    /// Upload buffers to GPU.
    pub fn upload(&mut self, _device: &Device, queue: &Queue) {
        if self.vertex_count > 0 {
            queue.write_buffer(
                &self.vertex_buffer,
                0,
                bytemuck::cast_slice(&self.vertex_data),
            );
            queue.write_buffer(
                &self.index_buffer,
                0,
                bytemuck::cast_slice(&self.index_data),
            );
        }
    }

    /// Update uniform buffer.
    pub fn update_uniform(&mut self, queue: &Queue, screen_width: f32, screen_height: f32) {
        let uniform = UiUniform {
            screen_size: [screen_width, screen_height],
            _padding: [0.0, 0.0],
        };
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniform));
    }

    /// Render the UI.
    pub fn render(&mut self, render_pass: &mut RenderPass<'_>) {
        if self.vertex_count == 0 {
            return;
        }

        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
        render_pass.set_bind_group(1, &self.texture_bind_group, &[]);

        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

        // Render each draw call
        let mut index_offset = 0;

        for call in &self.draw_calls {
            render_pass.draw_indexed(
                index_offset..index_offset + call.index_count,
                call.vertex_offset as i32,
                0..1,
            );
            index_offset += call.index_count;
        }
    }
}

/// Pipeline errors.
#[derive(Debug, thiserror::Error)]
pub enum PipelineError {
    #[error("wgpu error: {0}")]
    Wgpu(#[from] wgpu::Error),
}
