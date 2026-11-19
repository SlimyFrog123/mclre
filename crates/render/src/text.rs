//! Text rendering using Glyphon (cosmic-text).
//!
//! This module is currently a placeholder. The glyphon crate has version
//! compatibility issues with wgpu 22. Text rendering will be implemented
//! once the ecosystem stabilizes.

use wgpu::{Device, Queue, SurfaceConfiguration, TextureFormat};

/// Placeholder text rendering system.
pub struct TextRendererSystem {
    _phantom: std::marker::PhantomData<()>,
}

impl TextRendererSystem {
    /// Creates a new text rendering system (placeholder).
    pub fn new(_device: &Device, _queue: &Queue, _config: &SurfaceConfiguration) -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }

    /// Placeholder for preparing a text buffer.
    pub fn prepare_buffer(&mut self, _text: &str, _font_size: f32, _color: u32) -> u32 {
        0 // Placeholder
    }

    /// Placeholder for preparing the text renderer.
    pub fn prepare(
        &mut self,
        _device: &wgpu::Device,
        _queue: &wgpu::Queue,
        _resolution: u32,
    ) -> Result<(), String> {
        Ok(()) // Placeholder
    }

    /// Placeholder for rendering text.
    pub fn render_text(&mut self, _render_pass: &mut wgpu::RenderPass<'_>, _buffer: u32) {
        // Placeholder
    }

    /// Returns a dummy text atlas handle.
    pub fn text_atlas(&self) -> u32 {
        0 // Placeholder
    }
}

/// Simple text widget for UI (placeholder).
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_widget_creation() {
        let widget = TextWidget::new("Hello", 16.0, 0xFFFFFFFF, (0.0, 0.0));
        assert_eq!(widget.text, "Hello");
        assert_eq!(widget.font_size, 16.0);
    }
}
