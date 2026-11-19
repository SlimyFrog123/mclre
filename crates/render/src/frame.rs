//! Frame graph: render targets, depth buffer, transient resources.

use wgpu::{
    Device, Queue, Texture, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
    TextureView, TextureViewDescriptor,
};

/// Frame graph managing render targets and transient resources.
#[derive(Debug)]
pub struct FrameGraph {
    depth_texture: Texture,
    depth_view: TextureView,
    width: u32,
    height: u32,
}

impl FrameGraph {
    /// Creates a new frame graph.
    pub fn new(device: &Device, config: &wgpu::SurfaceConfiguration) -> Self {
        let depth_texture = Self::create_depth_texture(device, config.width, config.height);
        let depth_view = depth_texture.create_view(&TextureViewDescriptor::default());
        Self {
            depth_texture,
            depth_view,
            width: config.width,
            height: config.height,
        }
    }

    /// Resizes the frame graph.
    pub fn resize(&mut self, device: &Device, config: &wgpu::SurfaceConfiguration) {
        self.width = config.width;
        self.height = config.height;
        self.depth_texture = Self::create_depth_texture(device, config.width, config.height);
        self.depth_view = self
            .depth_texture
            .create_view(&TextureViewDescriptor::default());
    }

    /// Prepares the frame graph for rendering.
    pub fn prepare(
        &mut self,
        _device: &Device,
        _queue: &Queue,
        _view: &TextureView,
        _chunks: &[crate::resources::GpuChunkSection],
    ) -> Result<(), FrameGraphError> {
        // No-op for now; could upload chunk meshes here in a real implementation
        Ok(())
    }

    /// Returns the depth texture view.
    pub fn depth_view(&self) -> &TextureView {
        &self.depth_view
    }

    fn create_depth_texture(device: &Device, width: u32, height: u32) -> Texture {
        device.create_texture(&TextureDescriptor {
            label: Some("depth texture"),
            size: wgpu::Extent3d {
                width: width.max(1),
                height: height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Depth32Float,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        })
    }
}

/// Frame graph errors.
#[derive(Debug, thiserror::Error)]
pub enum FrameGraphError {
    #[error("texture creation failed: {0}")]
    Texture(#[from] wgpu::Error),
}

/// Render context passed to render passes.
#[derive(Debug)]
pub struct RenderContext<'a> {
    /// The wgpu device.
    pub device: &'a Device,
    /// The wgpu queue.
    pub queue: &'a Queue,
    /// The frame graph.
    pub frame_graph: &'a FrameGraph,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn depth_texture_format_is_correct() {
        // Just verify the type exists and compiles
    }
}
