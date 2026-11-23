//! wgpu renderer: staged frame graph (Iris-mappable), chunk meshing, AO/smooth lighting, viewports.

#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]

pub mod chunk_mesh;
pub mod frame;
pub mod glyphon_text;
pub mod pipeline;
pub mod resources;
pub mod text;
pub mod texture_atlas;
pub mod ui_pipeline;
pub mod view;

pub use frame::{FrameGraph, FrameGraphError, RenderContext};
pub use glyphon_text::{
    GlyphonColor, GlyphonFamily, GlyphonTextBuffer, GlyphonTextError, GlyphonTextRenderer,
};
pub use pipeline::{ChunkPipeline, PipelineError};
pub use resources::{GpuChunkSection, MeshBuffers};
pub use text::{BitmapFont, TextRendererSystem, TextWidget, UiTextRenderer};
pub use texture_atlas::{AtlasError, TextureAtlas};
pub use ui_pipeline::{PipelineError as UiPipelineError, UiPipeline, UiVertex};
pub use view::{View, ViewUniform, Viewport};

pub use wgpu::Texture;

use std::collections::HashMap;
use std::sync::Arc;

use mclre_world::{ChunkColumn, ChunkPos};
use wgpu::{Device, Queue, RenderPass, Surface, SurfaceConfiguration, TextureFormat};

/// The renderer backend, owning the wgpu device, queue, and surface.
pub struct Renderer {
    device: Arc<Device>,
    queue: Arc<Queue>,
    surface: Surface<'static>,
    config: SurfaceConfiguration,
    format: TextureFormat,
    frame_graph: FrameGraph,
    chunk_pipeline: ChunkPipeline,
    ui_pipeline: UiPipeline,
    view_uniform_buffer: wgpu::Buffer,
    view_bind_group: wgpu::BindGroup,
    /// Texture atlas for block textures.
    texture_atlas: TextureAtlas,
}

/// Creates a default font atlas texture for text rendering.
fn create_font_atlas_texture(device: &Device, queue: &Queue) -> wgpu::Texture {
    // Create a simple 256x256 white texture as placeholder font atlas
    // In a full implementation, this would be generated from a TTF font using Glyphon
    let size: u32 = 256;
    let data = vec![0xFFu8; (size * size * 4) as usize]; // White texture

    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Font atlas placeholder"),
        size: wgpu::Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });

    queue.write_texture(
        wgpu::ImageCopyTexture {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &data,
        wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(size * 4),
            rows_per_image: Some(size),
        },
        wgpu::Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
    );

    texture
}

impl Renderer {
    /// Creates a new renderer for the given window.
    pub async fn new(window: Arc<winit::window::Window>) -> Result<Self, RendererError> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone())?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or(RendererError::NoAdapter)?;

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("mclre render device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: wgpu::MemoryHints::Performance,
                },
                None,
            )
            .await?;

        let device = Arc::new(device);
        let queue = Arc::new(queue);

        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .find(|f| {
                matches!(
                    f,
                    TextureFormat::Bgra8UnormSrgb | TextureFormat::Rgba8UnormSrgb
                )
            })
            .copied()
            .unwrap_or(TextureFormat::Bgra8UnormSrgb);

        let size = window.inner_size();
        let config = SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let frame_graph = FrameGraph::new(&device, &config);
        let chunk_pipeline = ChunkPipeline::new(&device, format)?;
        let mut ui_pipeline = UiPipeline::new(device.clone(), queue.clone(), format)?;

        // View uniform buffer
        let view_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("view uniform"),
            size: std::mem::size_of::<ViewUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

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

        let view_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("view bind group"),
            layout: &view_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: view_uniform_buffer.as_entire_binding(),
            }],
        });

        // Create texture atlas before moving device/queue
        let texture_atlas = TextureAtlas::default(&device, &queue)?;

        // Create font atlas texture for text rendering
        let font_atlas_texture = create_font_atlas_texture(&device, &queue);

        // Set atlas textures on UI pipeline (ui_pipeline is mutable here)
        ui_pipeline.set_atlas(texture_atlas.texture());
        ui_pipeline.set_font_atlas(&font_atlas_texture);

        Ok(Self {
            device,
            queue,
            surface,
            config,
            format,
            frame_graph,
            chunk_pipeline,
            ui_pipeline,
            view_uniform_buffer,
            view_bind_group,
            texture_atlas,
        })
    }

    /// Sets a custom texture atlas from the asset system.
    ///
    /// This allows the asset system (mclre-assets) to build a texture atlas
    /// on the CPU and upload it to the GPU.
    pub fn set_texture_atlas(
        &mut self,
        texture: &mclre_assets::Texture,
        uvs: HashMap<String, [f32; 4]>,
        padding: u32,
    ) -> Result<(), RendererError> {
        self.texture_atlas =
            TextureAtlas::from_texture(&self.device, &self.queue, texture, uvs, padding)?;

        // Create a properly ordered UV array for the shader.
        // Assign explicit layer indices to basic blocks first for consistent rendering.
        let mut uv_array = vec![[0.0f32; 4]; 512];

        // Define priority order for basic blocks (layer indices 0-15)
        let basic_blocks = [
            "minecraft:block/stone",
            "minecraft:block/dirt",
            "minecraft:block/grass_block_top",
            "minecraft:block/bedrock",
            "minecraft:block/cobblestone",
            "minecraft:block/sand",
            "minecraft:block/gravel",
            "minecraft:block/oak_log",
            "minecraft:block/oak_planks",
            "minecraft:block/crafting_table",
            "minecraft:block/furnace",
            "minecraft:block/chest",
            "minecraft:block/glass",
            "minecraft:block/coal_ore",
            "minecraft:block/iron_ore",
            "minecraft:block/gold_ore",
        ];

        let mut layer = 0;
        // Assign basic blocks first
        for name in basic_blocks {
            if let Some(uv) = self.texture_atlas.uvs(name) {
                uv_array[layer] = uv;
                layer += 1;
            }
        }

        // Add remaining textures
        for (_, uv) in self.texture_atlas.all_uvs() {
            if layer >= 512 {
                break;
            }
            // Skip already assigned basic blocks
            let is_basic = basic_blocks
                .iter()
                .any(|b| self.texture_atlas.uvs(b) == Some(uv));
            if !is_basic {
                uv_array[layer] = uv;
                layer += 1;
            }
        }

        // Update the chunk pipeline with the UV regions
        self.chunk_pipeline.update_atlas_uvs(&self.queue, &uv_array);

        Ok(())
    }

    /// Resizes the surface.
    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
        self.frame_graph.resize(&self.device, &self.config);
    }

    /// Prepares the UI pipeline for a new frame.
    pub fn ui_begin_frame(&mut self) {
        self.ui_pipeline.begin_frame();
    }

    /// Adds text to the UI pipeline using the bitmap font.
    pub fn ui_draw_text(&mut self, text: &str, x: f32, y: f32, size: f32, color: u32) {
        // The font is created in the UiPipeline during initialization
        // We need to access the font from the TextRendererSystem
        // For now, we'll use a simple approach - the text rendering is handled
        // by the UiTextRenderer which manages its own font
    }

    /// Renders the UI pipeline.
    pub fn ui_render(&mut self, render_pass: &mut RenderPass<'_>) {
        self.ui_pipeline.render(render_pass);
    }

    /// Gets a mutable reference to the UI pipeline for advanced usage.
    pub fn ui_pipeline_mut(&mut self) -> &mut UiPipeline {
        &mut self.ui_pipeline
    }

    /// Renders the UI for a specific viewport using the current UI pipeline data.
    /// The UI pipeline must have been prepared (begin_frame + draw calls + upload) before calling this.
    pub fn render_ui_viewport(&mut self, render_pass: &mut RenderPass<'_>, viewport: &Viewport) {
        render_pass.set_viewport(
            viewport.x,
            viewport.y,
            viewport.width,
            viewport.height,
            0.0,
            1.0,
        );
        self.ui_pipeline.render(render_pass);
    }

    /// Prepares a chunk for rendering by meshing and uploading to GPU.
    pub fn prepare_chunk(
        &mut self,
        view: &View,
        chunk: &Arc<ChunkColumn>,
        pos: ChunkPos,
    ) -> Result<(), RendererError> {
        use crate::chunk_mesh::mesh_chunk;
        use crate::resources::GpuChunkSection;
        use mclre_content_vanilla::block_states;

        let block_state_model = block_states().ok();
        let gpu_sections = mesh_chunk(
            &self.device,
            &self.queue,
            chunk,
            pos,
            block_state_model.as_ref(),
        )?;
        self.frame_graph.add_chunk_sections(pos, gpu_sections);
        Ok(())
    }

    /// Renders a frame with the given view and chunk sections.
    pub fn render(&mut self, view: &View, chunks: &[GpuChunkSection]) -> Result<(), RendererError> {
        self.render_views(
            &[*view],
            chunks,
            None::<fn(usize, &Viewport, &mut UiPipeline)>,
            None::<fn(usize, &Viewport, &mut RenderPass<'_>)>,
        )
    }

    /// Renders a frame with multiple views (split-screen).
    /// Each view is rendered to its own viewport on the surface.
    /// Optional `ui_callback` is called for each viewport to populate per-viewport UI data.
    /// The callback receives (view_index, viewport, &mut UiPipeline).
    /// Optional `glyphon_callback` is called after UI rendering for each viewport to render Glyphon text.
    /// The callback receives (view_index, viewport, &mut RenderPass).
    /// The callback should use its captured GlyphonTextRenderer to render text.
    pub fn render_views<F, G>(
        &mut self,
        views: &[View],
        chunks: &[GpuChunkSection],
        ui_callback: Option<F>,
        mut glyphon_callback: Option<G>,
    ) -> Result<(), RendererError>
    where
        F: FnMut(usize, &Viewport, &mut UiPipeline),
        G: FnMut(usize, &Viewport, &mut RenderPass<'_>),
    {
        if views.is_empty() {
            return Err(RendererError::Pipeline(PipelineError::Shader(
                "At least one view required for rendering".to_string(),
            )));
        }

        let frame = self.surface.get_current_texture()?;
        let frame_view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("frame encoder"),
            });

        // Prepare frame graph (done once for all views)
        // Use the first view for frame graph preparation
        self.frame_graph
            .prepare(&self.device, &self.queue, &frame_view, chunks)?;

        // Calculate viewport for each view
        let screen_width = self.config.width as f32;
        let screen_height = self.config.height as f32;
        let viewports = Self::calculate_viewports(views.len(), screen_width, screen_height);

        // Render each view to its viewport
        for (i, view) in views.iter().enumerate() {
            // Update view uniform
            let view_uniform = ViewUniform::from(view);
            let mut bytes = [0u8; 64];
            let view_slice = view_uniform.view_proj.as_slice();
            for (j, &f) in view_slice.iter().enumerate() {
                let f_bytes = f.to_ne_bytes();
                bytes[j * 4..j * 4 + 4].copy_from_slice(&f_bytes);
            }
            self.queue
                .write_buffer(&self.view_uniform_buffer, 0, &bytes);

            let viewport = &viewports[i];

            // Main render pass (world) for this viewport
            {
                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some(&format!("main render pass - view {}", i)),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &frame_view,
                        resolve_target: None,
                        ops: if i == 0 {
                            wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                store: wgpu::StoreOp::Store,
                            }
                        } else {
                            wgpu::Operations {
                                load: wgpu::LoadOp::Load,
                                store: wgpu::StoreOp::Store,
                            }
                        },
                    })],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: self.frame_graph.depth_view(),
                        depth_ops: Some(wgpu::Operations {
                            load: if i == 0 {
                                wgpu::LoadOp::Clear(1.0)
                            } else {
                                wgpu::LoadOp::Load
                            },
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });

                // Set viewport for this view
                render_pass.set_viewport(
                    viewport.x,
                    viewport.y,
                    viewport.width,
                    viewport.height,
                    0.0,
                    1.0,
                );

                render_pass.set_bind_group(0, &self.view_bind_group, &[]);
                self.chunk_pipeline.render(
                    &mut render_pass,
                    &self.queue,
                    &self.frame_graph,
                    chunks,
                    self.texture_atlas.bind_group(),
                );
            }
        }

        // UI render pass (overlay) - render UI for each view
        // Supports per-viewport UI via optional callback
        if let Some(mut ui_callback) = ui_callback {
            // Per-viewport UI: callback provides UI data for each viewport
            for (i, viewport) in viewports.iter().enumerate() {
                // Begin UI frame for this viewport
                self.ui_pipeline.begin_frame();

                // Call the callback to populate UI for this viewport
                ui_callback(i, viewport, &mut self.ui_pipeline);

                // Upload UI data
                self.ui_pipeline.upload(&self.device, &self.queue);

                // Render UI to this viewport
                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some(&format!("UI render pass - view {}", i)),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &frame_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load, // Keep the world render
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: self.frame_graph.depth_view(),
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });

                render_pass.set_viewport(
                    viewport.x,
                    viewport.y,
                    viewport.width,
                    viewport.height,
                    0.0,
                    1.0,
                );

                self.ui_pipeline.render(&mut render_pass);

                // Glyphon text render pass for this viewport
                if let Some(mut glyphon_callback) = glyphon_callback.as_mut() {
                    // Create a new render pass for Glyphon (renders on top of UI)
                    let mut glyphon_render_pass =
                        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: Some(&format!("Glyphon render pass - view {}", i)),
                            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                view: &frame_view,
                                resolve_target: None,
                                ops: wgpu::Operations {
                                    load: wgpu::LoadOp::Load,
                                    store: wgpu::StoreOp::Store,
                                },
                            })],
                            depth_stencil_attachment: Some(
                                wgpu::RenderPassDepthStencilAttachment {
                                    view: self.frame_graph.depth_view(),
                                    depth_ops: Some(wgpu::Operations {
                                        load: wgpu::LoadOp::Load,
                                        store: wgpu::StoreOp::Store,
                                    }),
                                    stencil_ops: None,
                                },
                            ),
                            timestamp_writes: None,
                            occlusion_query_set: None,
                        });

                    glyphon_render_pass.set_viewport(
                        viewport.x,
                        viewport.y,
                        viewport.width,
                        viewport.height,
                        0.0,
                        1.0,
                    );

                    // Call the Glyphon callback with the render pass
                    // The callback captures its own GlyphonTextRenderer
                    glyphon_callback(i, viewport, &mut glyphon_render_pass);
                }
            }
        } else {
            // Legacy: render the same UI to all viewports (for backward compatibility)
            self.ui_pipeline.begin_frame();
            self.ui_pipeline.upload(&self.device, &self.queue);

            for (i, viewport) in viewports.iter().enumerate() {
                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some(&format!("UI render pass - view {}", i)),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &frame_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load, // Keep the world render
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: self.frame_graph.depth_view(),
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });

                render_pass.set_viewport(
                    viewport.x,
                    viewport.y,
                    viewport.width,
                    viewport.height,
                    0.0,
                    1.0,
                );

                self.ui_pipeline.render(&mut render_pass);

                // Glyphon text render pass for this viewport (legacy)
                if let Some(mut glyphon_callback) = glyphon_callback.as_mut() {
                    let mut glyphon_render_pass =
                        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: Some(&format!("Glyphon render pass - view {}", i)),
                            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                view: &frame_view,
                                resolve_target: None,
                                ops: wgpu::Operations {
                                    load: wgpu::LoadOp::Load,
                                    store: wgpu::StoreOp::Store,
                                },
                            })],
                            depth_stencil_attachment: Some(
                                wgpu::RenderPassDepthStencilAttachment {
                                    view: self.frame_graph.depth_view(),
                                    depth_ops: Some(wgpu::Operations {
                                        load: wgpu::LoadOp::Load,
                                        store: wgpu::StoreOp::Store,
                                    }),
                                    stencil_ops: None,
                                },
                            ),
                            timestamp_writes: None,
                            occlusion_query_set: None,
                        });

                    glyphon_render_pass.set_viewport(
                        viewport.x,
                        viewport.y,
                        viewport.width,
                        viewport.height,
                        0.0,
                        1.0,
                    );

                    glyphon_callback(i, viewport, &mut glyphon_render_pass);
                }
            }
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        frame.present();
        Ok(())
    }

    /// Calculates viewport rectangles for split-screen rendering.
    /// Supports 1-4 players in standard layouts.
    pub fn calculate_viewports(
        count: usize,
        screen_width: f32,
        screen_height: f32,
    ) -> Vec<Viewport> {
        match count {
            1 => vec![Viewport {
                x: 0.0,
                y: 0.0,
                width: screen_width,
                height: screen_height,
            }],
            2 => vec![
                // Horizontal split (top/bottom)
                Viewport {
                    x: 0.0,
                    y: 0.0,
                    width: screen_width,
                    height: screen_height / 2.0,
                },
                Viewport {
                    x: 0.0,
                    y: screen_height / 2.0,
                    width: screen_width,
                    height: screen_height / 2.0,
                },
            ],
            3 => vec![
                // Top player full width, bottom two side by side
                Viewport {
                    x: 0.0,
                    y: 0.0,
                    width: screen_width,
                    height: screen_height / 2.0,
                },
                Viewport {
                    x: 0.0,
                    y: screen_height / 2.0,
                    width: screen_width / 2.0,
                    height: screen_height / 2.0,
                },
                Viewport {
                    x: screen_width / 2.0,
                    y: screen_height / 2.0,
                    width: screen_width / 2.0,
                    height: screen_height / 2.0,
                },
            ],
            4 => vec![
                // 2x2 grid
                Viewport {
                    x: 0.0,
                    y: 0.0,
                    width: screen_width / 2.0,
                    height: screen_height / 2.0,
                },
                Viewport {
                    x: screen_width / 2.0,
                    y: 0.0,
                    width: screen_width / 2.0,
                    height: screen_height / 2.0,
                },
                Viewport {
                    x: 0.0,
                    y: screen_height / 2.0,
                    width: screen_width / 2.0,
                    height: screen_height / 2.0,
                },
                Viewport {
                    x: screen_width / 2.0,
                    y: screen_height / 2.0,
                    width: screen_width / 2.0,
                    height: screen_height / 2.0,
                },
            ],
            _ => {
                // Fallback: single view
                vec![Viewport {
                    x: 0.0,
                    y: 0.0,
                    width: screen_width,
                    height: screen_height,
                }]
            }
        }
    }
    /// Returns the wgpu device.
    pub fn device(&self) -> &Device {
        &self.device
    }

    /// Returns the wgpu queue.
    pub fn queue(&self) -> &Queue {
        &self.queue
    }

    /// Returns a cloned Arc to the wgpu device.
    pub fn device_arc(&self) -> Arc<Device> {
        Arc::clone(&self.device)
    }

    /// Returns a cloned Arc to the wgpu queue.
    pub fn queue_arc(&self) -> Arc<Queue> {
        Arc::clone(&self.queue)
    }

    /// Returns the render surface width.
    pub fn width(&self) -> u32 {
        self.config.width
    }

    /// Returns the render surface height.
    pub fn height(&self) -> u32 {
        self.config.height
    }

    /// Returns the surface texture format.
    pub fn format(&self) -> TextureFormat {
        self.format
    }

    /// Renders Glyphon text buffers after the UI pass.
    /// This should be called after the UI render passes for all views.
    /// Buffers are rendered with their individual positions.
    pub fn render_glyphon(
        &mut self,
        glyph_renderer: &mut GlyphonTextRenderer,
        buffers: &[(&GlyphonTextBuffer, (f32, f32))],
        viewport: &Viewport,
    ) -> Result<(), GlyphonTextError> {
        let frame = self.surface.get_current_texture()?;
        let frame_view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("glyphon render encoder"),
            });

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Glyphon text render pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &frame_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: self.frame_graph.depth_view(),
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass.set_viewport(
            viewport.x,
            viewport.y,
            viewport.width,
            viewport.height,
            0.0,
            1.0,
        );

        // Render Glyphon text with individual positions
        glyph_renderer.render_with_positions(
            &self.device,
            &self.queue,
            &mut render_pass,
            buffers.iter().map(|(b, pos)| (*b, *pos)),
        )?;

        self.queue.submit(std::iter::once(encoder.finish()));
        Ok(())
    }
}

/// Render command for the render thread.
#[derive(Debug)]
pub enum RenderCommand {
    /// Prepare a chunk for rendering.
    PrepareChunk {
        view: View,
        chunk: Arc<ChunkColumn>,
        pos: ChunkPos,
    },
    /// Render a frame.
    RenderFrame {
        views: Vec<View>,
        chunks: Vec<GpuChunkSection>,
    },
}

/// Errors from renderer creation and operation.
#[derive(Debug, thiserror::Error)]
pub enum RendererError {
    #[error("wgpu error: {0}")]
    Wgpu(#[from] wgpu::Error),
    #[error("no suitable adapter found")]
    NoAdapter,
    #[error("pipeline error: {0}")]
    Pipeline(#[from] PipelineError),
    #[error("UI pipeline error: {0}")]
    UiPipeline(#[from] UiPipelineError),
    #[error("surface error: {0}")]
    Surface(#[from] wgpu::SurfaceError),
    #[error("frame graph error: {0}")]
    FrameGraph(#[from] FrameGraphError),
    #[error("surface creation error: {0}")]
    CreateSurface(#[from] wgpu::CreateSurfaceError),
    #[error("device request error: {0}")]
    RequestDevice(#[from] wgpu::RequestDeviceError),
    #[error("texture atlas error: {0}")]
    TextureAtlas(#[from] AtlasError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn view_uniform_size_is_correct() {
        assert_eq!(std::mem::size_of::<ViewUniform>(), 64); // 4x4 f32 matrix
    }
}
