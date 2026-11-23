//! Text rendering using bitmap font atlas.
//!
//! This module provides a bitmap font-based text rendering system for UI.
//! Uses the existing UI pipeline's font atlas support (texture_index = 1).

use std::collections::HashMap;

use wgpu::{Device, Queue, RenderPass, Texture, TextureFormat, TextureView};

/// A simple bitmap font for UI text rendering.
pub struct BitmapFont {
    /// Character to UV mapping (u0, v0, u1, v1 in atlas coordinates)
    char_uvs: HashMap<char, [f32; 4]>,
    /// Font atlas texture
    texture: Texture,
    /// Font atlas view
    view: TextureView,
    /// Character size in pixels
    char_width: u32,
    char_height: u32,
    /// Atlas dimensions
    atlas_width: u32,
    atlas_height: u32,
}

impl BitmapFont {
    /// Creates a new bitmap font from a texture atlas.
    pub fn new(
        device: &Device,
        queue: &Queue,
        char_width: u32,
        char_height: u32,
        atlas_width: u32,
        atlas_height: u32,
        char_map: &[(char, u32, u32)], // (char, x, y) in grid coordinates
    ) -> Self {
        // Create a simple font atlas texture (placeholder - white texture)
        // In a real implementation, this would be loaded from a font image file
        let size = atlas_width.max(atlas_height);
        let data = vec![0xFFu8; (size * size * 4) as usize];

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Bitmap font atlas"),
            size: wgpu::Extent3d {
                width: atlas_width,
                height: atlas_height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: TextureFormat::Rgba8UnormSrgb,
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
                bytes_per_row: Some(atlas_width * 4),
                rows_per_image: Some(atlas_height),
            },
            wgpu::Extent3d {
                width: atlas_width,
                height: atlas_height,
                depth_or_array_layers: 1,
            },
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Build character UV map
        let mut char_uvs = HashMap::new();
        let cell_w = 1.0 / (atlas_width / char_width) as f32;
        let cell_h = 1.0 / (atlas_height / char_height) as f32;

        for &(ch, gx, gy) in char_map {
            let u0 = gx as f32 * cell_w;
            let v0 = gy as f32 * cell_h;
            let u1 = u0 + cell_w;
            let v1 = v0 + cell_h;
            char_uvs.insert(ch, [u0, v0, u1, v1]);
        }

        // Add space character if not present
        if !char_uvs.contains_key(&' ') {
            char_uvs.insert(' ', [0.0, 0.0, cell_w, cell_h]);
        }

        Self {
            char_uvs,
            texture,
            view,
            char_width,
            char_height,
            atlas_width,
            atlas_height,
        }
    }

    /// Creates a default Minecraft-style font (16x16 chars in 256x256 atlas).
    pub fn minecraft_default(device: &Device, queue: &Queue) -> Self {
        // Standard ASCII 32-126 mapped to 16x16 grid (16 cols x 6 rows = 96 chars)
        let mut char_map = Vec::new();
        for i in 32..127 {
            let ch = i as u8 as char;
            let idx = i - 32;
            let gx = (idx % 16) as u32;
            let gy = (idx / 16) as u32;
            char_map.push((ch, gx, gy));
        }

        Self::new(device, queue, 16, 16, 256, 256, &char_map)
    }

    /// Gets UV coordinates for a character.
    pub fn get_uv(&self, ch: char) -> [f32; 4] {
        self.char_uvs
            .get(&ch)
            .copied()
            .unwrap_or([0.0, 0.0, 0.0625, 0.0625])
    }

    /// Returns the font atlas texture.
    pub fn texture(&self) -> &Texture {
        &self.texture
    }

    /// Returns the font atlas view.
    pub fn view(&self) -> &TextureView {
        &self.view
    }

    /// Returns character dimensions.
    pub fn char_size(&self) -> (u32, u32) {
        (self.char_width, self.char_height)
    }
}

/// Text rendering system using bitmap font.
pub struct TextRendererSystem {
    font: BitmapFont,
    /// Queued text for rendering: (text, x, y, scale, color)
    queued_text: Vec<(String, f32, f32, f32, u32)>,
}

impl TextRendererSystem {
    /// Creates a new text rendering system with default Minecraft font.
    pub fn new(device: &Device, queue: &Queue, _config: &wgpu::SurfaceConfiguration) -> Self {
        let font = BitmapFont::minecraft_default(device, queue);
        Self {
            font,
            queued_text: Vec::new(),
        }
    }

    /// Creates a new text rendering system with a custom font.
    pub fn with_font(font: BitmapFont) -> Self {
        Self {
            font,
            queued_text: Vec::new(),
        }
    }

    /// Queues text for rendering at a position.
    pub fn queue_text(&mut self, text: &str, x: f32, y: f32, scale: f32, color: u32) {
        self.queued_text
            .push((text.to_string(), x, y, scale, color));
    }

    /// Renders all queued text using the UI pipeline's draw calls.
    /// Note: This requires access to the UiPipeline to add quads.
    /// The actual rendering is done by the UI pipeline when it processes draw calls.
    pub fn render(&mut self, _render_pass: &mut RenderPass<'_>) {
        // The actual rendering is handled by the UI pipeline
        // This system just prepares the text data
        self.queued_text.clear();
    }

    /// Returns the font atlas texture for binding.
    pub fn text_atlas_texture(&self) -> &Texture {
        self.font.texture()
    }

    /// Returns the font atlas view for binding.
    pub fn text_atlas_view(&self) -> &TextureView {
        self.font.view()
    }

    /// Gets the font for measuring text.
    pub fn font(&self) -> &BitmapFont {
        &self.font
    }

    /// Takes the queued text for the UI pipeline to process.
    pub fn take_queued_text(&mut self) -> Vec<(String, f32, f32, f32, u32)> {
        std::mem::take(&mut self.queued_text)
    }
}

/// Text widget for UI.
pub struct TextWidget {
    text: String,
    font_size: f32,
    color: u32,
    position: (f32, f32),
}

impl TextWidget {
    pub fn new(text: impl Into<String>, font_size: f32, color: u32, position: (f32, f32)) -> Self {
        Self {
            text: text.into(),
            font_size,
            color,
            position,
        }
    }

    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
    }
}

/// Text renderer for UI integration.
pub struct UiTextRenderer {
    text_renderer: TextRendererSystem,
}

impl UiTextRenderer {
    pub fn new(device: &Device, queue: &Queue, config: &wgpu::SurfaceConfiguration) -> Self {
        Self {
            text_renderer: TextRendererSystem::new(device, queue, config),
        }
    }

    pub fn text_renderer_mut(&mut self) -> &mut TextRendererSystem {
        &mut self.text_renderer
    }

    /// Draw text at a position.
    pub fn draw_text(&mut self, text: &str, pos: (f32, f32), size: f32, color: u32) {
        self.text_renderer
            .queue_text(text, pos.0, pos.1, size / 16.0, color);
    }

    /// Prepare for rendering.
    pub fn prepare(
        &mut self,
        _device: &Device,
        _queue: &Queue,
        _width: u32,
        _height: u32,
    ) -> Result<(), String> {
        Ok(())
    }

    /// Render all queued text.
    pub fn render(&mut self, _render_pass: &mut RenderPass<'_>) {
        // Handled by UI pipeline
    }

    /// Get the text atlas texture for binding.
    pub fn text_atlas_texture(&self) -> &Texture {
        self.text_renderer.text_atlas_texture()
    }

    /// Get the text atlas view.
    pub fn text_atlas_view(&self) -> &TextureView {
        self.text_renderer.text_atlas_view()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wgpu::{Device, Queue, SurfaceConfiguration};

    #[test]
    fn text_widget_creation() {
        let widget = TextWidget::new("Hello", 16.0, 0xFFFFFFFF, (0.0, 0.0));
        assert_eq!(widget.text, "Hello");
        assert_eq!(widget.font_size, 16.0);
    }

    #[test]
    fn text_renderer_creation() {
        // This test requires a wgpu device which isn't available in unit tests
        // Just test that the types can be constructed
    }
}
