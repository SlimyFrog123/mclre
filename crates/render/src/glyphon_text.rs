//! Glyphon-based TTF font text rendering using cosmic-text.
//!
//! This module provides a proper TTF font rendering system using Glyphon (cosmic-text).
//! Uses Glyphon 0.7 which is compatible with wgpu 23.

use glyphon::{
    Attrs, Buffer, Cache, Color, Family, FontSystem, Metrics, RenderError as GlyphonRenderError,
    Shaping, SwashCache, TextArea, TextAtlas, TextRenderer, Viewport,
};
use wgpu::{Device, MultisampleState, Queue, RenderPass, TextureFormat};

/// Re-export Glyphon color type for convenience.
pub use glyphon::Color as GlyphonColor;
/// Re-export Glyphon font family type for convenience.
pub use glyphon::Family as GlyphonFamily;

/// Glyphon text renderer for the UI system.
pub struct GlyphonTextRenderer {
    font_system: FontSystem,
    text_atlas: TextAtlas,
    text_renderer: TextRenderer,
    swash_cache: SwashCache,
    cache: Cache,
    viewport: Viewport,
}

impl GlyphonTextRenderer {
    /// Creates a new Glyphon text renderer.
    pub fn new(device: &Device, queue: &Queue, format: TextureFormat) -> Self {
        let font_system = FontSystem::new();
        let cache = Cache::new(device);
        let mut text_atlas = TextAtlas::new(device, queue, &cache, format);
        let text_renderer =
            TextRenderer::new(&mut text_atlas, device, MultisampleState::default(), None);
        let swash_cache = SwashCache::new();
        let viewport = Viewport::new(device, &cache);

        Self {
            font_system,
            text_atlas,
            text_renderer,
            swash_cache,
            cache,
            viewport,
        }
    }

    /// Updates the viewport resolution.
    pub fn update_viewport(&mut self, queue: &Queue, width: u32, height: u32) {
        self.viewport
            .update(queue, glyphon::Resolution { width, height });
    }

    /// Prepares a text buffer for rendering.
    pub fn prepare_buffer(
        &mut self,
        text: &str,
        size: f32,
        color: GlyphonColor,
        family: Family,
        bounds: glyphon::TextBounds,
    ) -> GlyphonTextBuffer {
        let mut buffer = Buffer::new(&mut self.font_system, Metrics::new(size, size * 1.2));
        buffer.set_text(
            &mut self.font_system,
            text,
            Attrs::new().family(family).color(color),
            Shaping::Advanced,
        );
        GlyphonTextBuffer { buffer, bounds }
    }

    /// Prepares a simple text area (uses default bounds).
    pub fn prepare_simple_buffer(
        &mut self,
        text: &str,
        size: f32,
        color: GlyphonColor,
        family: Family,
    ) -> GlyphonTextBuffer {
        let mut buffer = Buffer::new(&mut self.font_system, Metrics::new(size, size * 1.2));
        buffer.set_text(
            &mut self.font_system,
            text,
            Attrs::new().family(family).color(color),
            Shaping::Advanced,
        );
        GlyphonTextBuffer {
            buffer,
            bounds: glyphon::TextBounds::default(),
        }
    }

    /// Renders all prepared text buffers at a common position offset.
    pub fn render(
        &mut self,
        device: &Device,
        queue: &Queue,
        render_pass: &mut RenderPass<'_>,
        buffers: &[&GlyphonTextBuffer],
        position: (f32, f32),
        _viewport_size: (u32, u32),
    ) -> Result<(), GlyphonRenderError> {
        self.render_with_positions(
            device,
            queue,
            render_pass,
            buffers.iter().map(|b| (*b, position)),
        )
    }

    /// Renders text buffers with individual positions.
    /// Each buffer is rendered at its specified position.
    pub fn render_with_positions<'a>(
        &mut self,
        device: &Device,
        queue: &Queue,
        render_pass: &mut RenderPass<'_>,
        buffers: impl IntoIterator<Item = (&'a GlyphonTextBuffer, (f32, f32))>,
    ) -> Result<(), GlyphonRenderError> {
        let viewport = &self.viewport;

        // Prepare all buffers with their individual positions
        self.text_renderer
            .prepare(
                device,
                queue,
                &mut self.font_system,
                &mut self.text_atlas,
                viewport,
                buffers.into_iter().map(|(b, pos)| {
                    let mut area = TextArea {
                        buffer: &b.buffer,
                        left: pos.0,
                        top: pos.1,
                        scale: 1.0,
                        bounds: b.bounds,
                        default_color: GlyphonColor::rgb(0xFF, 0xFF, 0xFF),
                        custom_glyphs: &[],
                    };
                    area
                }),
                &mut self.swash_cache,
            )
            .map_err(|e| GlyphonRenderError::RemovedFromAtlas)?;

        // Render all prepared text
        self.text_renderer
            .render(&self.text_atlas, viewport, render_pass)?;

        Ok(())
    }

    /// Renders a single text buffer at a position.
    pub fn render_buffer(
        &mut self,
        device: &Device,
        queue: &Queue,
        render_pass: &mut RenderPass<'_>,
        buffer: &GlyphonTextBuffer,
        position: (f32, f32),
    ) -> Result<(), glyphon::RenderError> {
        self.render(device, queue, render_pass, &[buffer], position, (0, 0))
    }

    /// Gets mutable access to the font system.
    pub fn font_system(&mut self) -> &mut FontSystem {
        &mut self.font_system
    }

    /// Gets mutable access to the text atlas.
    pub fn text_atlas(&mut self) -> &mut TextAtlas {
        &mut self.text_atlas
    }

    /// Gets the cache.
    pub fn cache(&self) -> &Cache {
        &self.cache
    }
}

/// A text buffer for rendering text with Glyphon.
#[derive(Clone)]
pub struct GlyphonTextBuffer {
    buffer: Buffer,
    bounds: glyphon::TextBounds,
}

impl GlyphonTextBuffer {
    /// Creates a new text buffer.
    pub fn new(
        font_system: &mut FontSystem,
        family: Family,
        size: f32,
        text: &str,
        color: GlyphonColor,
    ) -> Self {
        let mut buffer = Buffer::new(font_system, Metrics::new(size, size * 1.2));
        buffer.set_text(
            font_system,
            text,
            Attrs::new().family(family).color(color),
            Shaping::Advanced,
        );
        Self {
            buffer,
            bounds: glyphon::TextBounds::default(),
        }
    }

    /// Updates the text content.
    pub fn set_text(
        &mut self,
        font_system: &mut FontSystem,
        text: &str,
        color: GlyphonColor,
        family: Family,
    ) {
        self.buffer.set_text(
            font_system,
            text,
            Attrs::new().family(family).color(color),
            Shaping::Advanced,
        );
    }

    /// Sets the position and size bounds.
    pub fn set_bounds(&mut self, bounds: glyphon::TextBounds) {
        self.bounds = bounds;
    }

    /// Gets the underlying buffer.
    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }

    /// Gets a mutable reference to the underlying buffer.
    pub fn buffer_mut(&mut self) -> &mut Buffer {
        &mut self.buffer
    }
}

/// Builder for creating GlyphonTextBuffers with common parameters.
pub struct GlyphonTextBufferBuilder {
    font_system: FontSystem,
    size: f32,
    color: GlyphonColor,
}

impl GlyphonTextBufferBuilder {
    /// Creates a new builder with default settings.
    pub fn new() -> Self {
        let font_system = FontSystem::new();
        Self {
            font_system,
            size: 16.0,
            color: GlyphonColor::rgb(0xFF, 0xFF, 0xFF),
        }
    }

    /// Sets the font size.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    /// Sets the text color.
    pub fn color(mut self, color: GlyphonColor) -> Self {
        self.color = color;
        self
    }

    /// Creates a text buffer from this builder using the default sans-serif font.
    pub fn build(&mut self, text: &str) -> GlyphonTextBuffer {
        GlyphonTextBuffer::new(
            &mut self.font_system,
            Family::SansSerif,
            self.size,
            text,
            self.color,
        )
    }

    /// Gets access to the font system for advanced usage.
    pub fn font_system(&mut self) -> &mut FontSystem {
        &mut self.font_system
    }
}

impl Default for GlyphonTextBufferBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Error type for Glyphon text rendering.
#[derive(Debug, thiserror::Error)]
pub enum GlyphonTextError {
    #[error("Glyphon render error: {0}")]
    Render(#[from] GlyphonRenderError),
    #[error("Font not found: {0}")]
    FontNotFound(String),
    #[error("Surface error: {0}")]
    Surface(#[from] wgpu::SurfaceError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use glyphon::Color as GlyphonColor;

    #[test]
    fn text_buffer_creation() {
        let mut font_system = FontSystem::new();
        let buffer = GlyphonTextBuffer::new(
            &mut font_system,
            Family::SansSerif,
            16.0,
            "Hello, World!",
            GlyphonColor::rgb(0xFF, 0xFF, 0xFF),
        );
        // Just test that it compiles and creates without panic
        let _ = buffer.buffer();
    }

    #[test]
    fn glyph_renderer_creation() {
        // Just test that the type compiles
        let _renderer: GlyphonTextRenderer;
    }
}
