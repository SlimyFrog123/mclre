//! wgpu renderer: staged frame graph (Iris-mappable), chunk meshing, AO/smooth lighting, viewports.

#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]

pub mod chunk_mesh;
pub mod frame;
pub mod pipeline;
pub mod resources;
pub mod text;
pub mod texture_atlas;
pub mod view;

pub use frame::{FrameGraph, FrameGraphError, RenderContext};
pub use pipeline::{ChunkPipeline, PipelineError};
pub use resources::{GpuChunkSection, MeshBuffers};
pub use text::{TextRendererSystem, TextWidget};
pub use texture_atlas::{AtlasError, TextureAtlas};
pub use view::{View, ViewUniform};

use std::sync::Arc;

use wgpu::{Device, Queue, Surface, SurfaceConfiguration, TextureFormat};

/// The renderer backend, owning the wgpu device, queue, and surface.
pub struct Renderer {
    device: Arc<Device>,
    queue: Arc<Queue>,
    surface: Surface<'static>,
    config: SurfaceConfiguration,
    format: TextureFormat,
    frame_graph: FrameGraph,
    chunk_pipeline: ChunkPipeline,
    view_uniform_buffer: wgpu::Buffer,
    view_bind_group: wgpu::BindGroup,
    /// Texture atlas for block textures.
    texture_atlas: TextureAtlas,
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

        Ok(Self {
            device,
            queue,
            surface,
            config,
            format,
            frame_graph,
            chunk_pipeline,
            view_uniform_buffer,
            view_bind_group,
            texture_atlas,
        })
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

    /// Renders a frame with the given view and chunk sections.
    pub fn render(&mut self, view: &View, chunks: &[GpuChunkSection]) -> Result<(), RendererError> {
        // Update view uniform
        let view_uniform = ViewUniform::from(view);
        // Convert [f32; 16] to bytes without unsafe by using a temporary array
        let mut bytes = [0u8; 64];
        let view_slice = view_uniform.view_proj.as_slice();
        for (i, &f) in view_slice.iter().enumerate() {
            let f_bytes = f.to_ne_bytes();
            bytes[i * 4..i * 4 + 4].copy_from_slice(&f_bytes);
        }
        self.queue
            .write_buffer(&self.view_uniform_buffer, 0, &bytes);

        let frame = self.surface.get_current_texture()?;
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("frame encoder"),
            });

        // Prepare frame graph
        self.frame_graph
            .prepare(&self.device, &self.queue, &view, chunks)?;

        // Render pass
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("main render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: self.frame_graph.depth_view(),
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_bind_group(0, &self.view_bind_group, &[]);
            self.chunk_pipeline.render(
                &mut render_pass,
                &self.frame_graph,
                chunks,
                self.texture_atlas.bind_group(),
            );
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        frame.present();
        Ok(())
    }

    /// Returns the wgpu device.
    pub fn device(&self) -> &Device {
        &self.device
    }

    /// Returns the wgpu queue.
    pub fn queue(&self) -> &Queue {
        &self.queue
    }

    /// Returns the render surface width.
    pub fn width(&self) -> u32 {
        self.config.width
    }

    /// Returns the render surface height.
    pub fn height(&self) -> u32 {
        self.config.height
    }
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
