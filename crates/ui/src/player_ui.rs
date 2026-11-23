//! Per-player UI context for split-screen support.

use std::sync::Arc;

use super::{
    Color, NineSliceRegion, Rect, Result, SceneManager, ScreenManager, UiContext, UiError, Widget,
    WidgetId, WidgetRegistry, WidgetType,
};
use mclre_render::UiPipeline;

/// Per-player UI context for split-screen support.
/// Each player in split-screen mode has their own UI context,
/// input manager, screen manager, and viewport.
pub struct PlayerUIContext {
    /// Player index (0-3 for up to 4 players)
    pub player_index: usize,
    /// Player's UI context
    pub ui_context: UiContext,
    /// Player's screen manager
    pub screen_manager: ScreenManager,
    /// Player's viewport rectangle (in screen coordinates)
    pub viewport: Rect,
    /// Whether this player is active
    pub active: bool,
}

impl PlayerUIContext {
    /// Creates a new player UI context for split-screen.
    pub fn new(
        player_index: usize,
        viewport: Rect,
        assets: Arc<mclre_assets::AssetManager>,
    ) -> Self {
        let ui_context = UiContext::for_player(0, viewport, assets.clone());
        let screen_manager = ScreenManager::new();

        Self {
            player_index,
            ui_context,
            screen_manager,
            viewport,
            active: true,
        }
    }

    /// Gets a mutable reference to the player's UI context.
    pub fn ui_context_mut(&mut self) -> &mut UiContext {
        &mut self.ui_context
    }

    /// Gets a mutable reference to the player's screen manager.
    pub fn screen_manager_mut(&mut self) -> &mut ScreenManager {
        &mut self.screen_manager
    }

    /// Gets the player's viewport.
    pub fn viewport(&self) -> Rect {
        self.viewport
    }

    /// Sets the player's viewport.
    pub fn set_viewport(&mut self, viewport: Rect) {
        self.viewport = viewport;
        self.ui_context.screen_size = (viewport.width as u32, viewport.height as u32);
    }

    /// Checks if the player is active.
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Sets the player's active state.
    pub fn set_active(&mut self, active: bool) {
        self.active = active;
    }

    /// Gets the player index.
    pub fn player_index(&self) -> usize {
        self.player_index
    }

    /// Handles input for this player's UI.
    pub fn handle_input(&mut self, action: mclre_input::Action, pressed: bool) -> bool {
        if !self.active {
            return false;
        }
        let context = &mut self.ui_context;
        self.screen_manager.handle_input(context, action, pressed)
    }

    /// Updates this player's UI.
    pub fn update(&mut self, dt: f32) {
        if !self.active {
            return;
        }
        self.ui_context.delta_time = dt;
        self.screen_manager.update(&mut self.ui_context, dt);
    }

    /// Renders this player's UI to their viewport.
    pub fn render(&mut self, renderer: &mut dyn crate::scene::Renderer) {
        if !self.active {
            return;
        }
        // TODO: Set viewport on renderer
        self.screen_manager.render(renderer);
    }

    /// Registers a screen for this player.
    pub fn register_screen(
        &mut self,
        screen_type: super::ScreenType,
        screen: Box<dyn super::Screen>,
    ) {
        self.screen_manager.register_screen(screen_type, screen);
    }

    /// Pushes a screen for this player.
    pub fn push_screen(&mut self, screen_type: super::ScreenType) -> Result<()> {
        self.screen_manager
            .push_screen(screen_type, &mut self.ui_context)
    }

    /// Pops a screen for this player.
    pub fn pop_screen(&mut self) -> Result<()> {
        self.screen_manager.pop_screen(&mut self.ui_context)
    }
}

#[cfg(test)]
mod tests {
    use super::Rect;
    use super::*;
    use mclre_assets::AssetManager;

    #[test]
    fn player_ui_context_creation() {
        let assets = Arc::new(mclre_assets::AssetManager::new());
        let viewport = Rect::new(0.0, 0.0, 960.0, 540.0);
        let ctx = PlayerUIContext::new(0, viewport, assets);

        assert_eq!(ctx.player_index(), 0);
        assert_eq!(ctx.viewport().width, 960.0);
        assert_eq!(ctx.viewport().height, 540.0);
        assert!(ctx.is_active());
    }
}

impl crate::scene::Renderer for UiPipeline {
    fn draw_rect(&mut self, rect: Rect, color: Color) {
        let color_u32 = (color.r as u32) << 24
            | (color.g as u32) << 16
            | (color.b as u32) << 8
            | (color.a as u32);
        self.add_quad(
            [rect.x, rect.y, rect.width, rect.height],
            [1.0, 1.0, 1.0, 1.0],
            color_u32,
            0,
        );
    }

    fn draw_text(&mut self, text: &str, pos: (f32, f32), size: f32, color: Color) {
        let color_u32 = (color.r as u32) << 24
            | (color.g as u32) << 16
            | (color.b as u32) << 8
            | (color.a as u32);
        let text_width = text.len() as f32 * size * 0.6;
        let text_height = size;
        self.add_quad(
            [pos.0, pos.1, text_width, text_height],
            [1.0, 1.0, 1.0, 1.0],
            color_u32,
            0,
        );
    }

    fn draw_image(
        &mut self,
        texture: &mclre_render::Texture,
        rect: Rect,
        uv: [f32; 4],
        color: Color,
    ) {
        let color_u32 = (color.r as u32) << 24
            | (color.g as u32) << 16
            | (color.b as u32) << 8
            | (color.a as u32);
        self.add_quad([rect.x, rect.y, rect.width, rect.height], uv, color_u32, 0);
    }

    fn draw_nine_slice(
        &mut self,
        atlas: &mclre_render::TextureAtlas,
        region: NineSliceRegion,
        rect: Rect,
        color: Color,
    ) {
        let color_u32 = (color.r as u32) << 24
            | (color.g as u32) << 16
            | (color.b as u32) << 8
            | (color.a as u32);
        self.add_quad(
            [rect.x, rect.y, rect.width, rect.height],
            [0.0, 0.0, 1.0, 1.0],
            color_u32,
            0,
        );
    }
}
