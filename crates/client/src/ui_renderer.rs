//! UI renderer adapter that implements the UI Renderer trait using mclre_render.

use mclre_render::{BitmapFont, Renderer as GpuRenderer, UiPipeline};
use mclre_ui::{Color, Rect, nine_slice::NineSliceRegion, scene::Renderer as UiRendererTrait};

/// UI renderer that adapts mclre_render to the UI Renderer trait.
pub struct UiRenderer<'a> {
    ui_pipeline: &'a mut UiPipeline,
    font: BitmapFont,
}

impl<'a> UiRenderer<'a> {
    pub fn new(gpu_renderer: &'a mut GpuRenderer) -> Self {
        // Create a bitmap font for text rendering first (immutable borrows)
        let font = BitmapFont::minecraft_default(gpu_renderer.device(), gpu_renderer.queue());
        // Then get mutable reference to UI pipeline
        let ui_pipeline = gpu_renderer.ui_pipeline_mut();
        Self { ui_pipeline, font }
    }
}

impl<'a> UiRendererTrait for UiRenderer<'a> {
    fn draw_rect(&mut self, rect: Rect, color: Color) {
        let color_u32 = color.to_u32();
        self.ui_pipeline.add_quad(
            [rect.x, rect.y, rect.width, rect.height],
            [0.0, 0.0, 1.0, 1.0], // Full texture (white pixel)
            color_u32,
            0, // texture_index = 0 for UI atlas
        );
    }

    fn draw_text(&mut self, text: &str, pos: (f32, f32), size: f32, color: Color) {
        let color_u32 = color.to_u32();
        let scale = size / 16.0;

        self.ui_pipeline
            .add_text(text, pos.0, pos.1, scale, color_u32, &self.font);
    }

    fn draw_image(
        &mut self,
        _texture: &mclre_render::Texture,
        rect: Rect,
        uv: [f32; 4],
        color: Color,
    ) {
        let color_u32 = color.to_u32();
        self.ui_pipeline
            .add_quad([rect.x, rect.y, rect.width, rect.height], uv, color_u32, 0);
    }

    fn draw_nine_slice(
        &mut self,
        _atlas: &mclre_render::TextureAtlas,
        _region: NineSliceRegion,
        _rect: Rect,
        _color: Color,
    ) {
        // Nine-slice rendering for UI panels/buttons
        // Would need to split into 9 regions and render each with appropriate UV coords
    }
}

// Extension trait for Color
trait ColorExt {
    fn to_u32(&self) -> u32;
}

impl ColorExt for Color {
    fn to_u32(&self) -> u32 {
        ((self.a as u32) << 24) | ((self.r as u32) << 16) | ((self.g as u32) << 8) | (self.b as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // Tests would require a GPU device
}
