//! Scene management for UI toolkit.

use std::collections::HashMap;
use std::sync::Arc;

use super::{Color, NineSliceRegion, Rect, Result, UiError, Widget, WidgetId, WidgetType};
use mclre_assets::Identifier;
use mclre_input::Action;
use mclre_render::View;

/// Scene identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SceneId(pub u64);

impl SceneId {
    pub fn new() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        Self(COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

impl Default for SceneId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for SceneId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SceneId({})", self.0)
    }
}

/// Scene trait for UI scenes.
pub trait Scene: Send + Sync {
    fn id(&self) -> SceneId;
    fn name(&self) -> &str;
    fn on_enter(&mut self, _ui: &mut UiContext) {}
    fn on_exit(&mut self, _ui: &mut UiContext) {}
    fn update(&mut self, _ui: &mut UiContext, _dt: f32) {}
    fn render(&self, _renderer: &mut dyn Renderer) {}
    fn handle_input(&mut self, _ui: &mut UiContext, _action: Action, _pressed: bool) -> bool {
        false
    }
}

/// Helper trait to allow lifetime-erased scene access.
/// This works around the trait object lifetime limitation.
trait SceneAccess {
    fn as_scene_mut(&mut self) -> &mut dyn Scene;
}

impl SceneAccess for Box<dyn Scene> {
    fn as_scene_mut(&mut self) -> &mut dyn Scene {
        self.as_mut()
    }
}

/// UI context passed to scenes.
pub struct UiContext {
    pub widget_registry: WidgetRegistry,
    pub input: mclre_input::InputManager<'static>,
    pub assets: Arc<mclre_assets::AssetManager>,
    pub renderer: mclre_render::Renderer,
    pub delta_time: f32,
    pub screen_size: (u32, u32),
}

/// Widget registry for managing widgets across scenes.
#[derive(Default)]
pub struct WidgetRegistry {
    widgets: HashMap<WidgetId, Arc<dyn Widget>>,
    root_widgets: Vec<WidgetId>,
    widget_types: HashMap<WidgetId, WidgetType>,
}

impl WidgetRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, widget: Box<dyn Widget>) -> WidgetId {
        let id = widget.id();
        self.widget_types.insert(id, widget.widget_type());
        self.widgets.insert(id, Arc::from(widget));
        id
    }

    pub fn unregister(&mut self, id: WidgetId) -> Option<Arc<dyn Widget>> {
        self.widget_types.remove(&id);
        self.root_widgets.retain(|&id| id != id);
        self.widgets.remove(&id)
    }

    pub fn get(&self, id: WidgetId) -> Option<Arc<dyn Widget>> {
        self.widgets.get(&id).cloned()
    }

    pub fn get_mut(&self, id: WidgetId) -> Option<Arc<dyn Widget>> {
        self.widgets.get(&id).cloned()
    }

    pub fn add_root(&mut self, id: WidgetId) {
        if !self.root_widgets.contains(&id) {
            self.root_widgets.push(id);
        }
    }

    pub fn remove_root(&mut self, id: WidgetId) {
        self.root_widgets.retain(|&item| item != id);
    }

    pub fn root_widgets(&self) -> &[WidgetId] {
        &self.root_widgets
    }

    pub fn widgets(&self) -> impl Iterator<Item = (&WidgetId, Arc<dyn Widget>)> + '_ {
        self.widgets.iter().map(|(id, w)| (id, w.clone()))
    }

    pub fn widgets_mut(&self) -> impl Iterator<Item = (&WidgetId, Arc<dyn Widget>)> + '_ {
        self.widgets.iter().map(|(id, w)| (id, w.clone()))
    }

    pub fn find_by_type(&self, widget_type: WidgetType) -> Vec<WidgetId> {
        self.widgets
            .iter()
            .filter(|(_, w)| w.widget_type() == widget_type)
            .map(|(id, _)| *id)
            .collect()
    }
}

/// Scene manager for managing multiple scenes.
pub struct SceneManager {
    scenes: HashMap<SceneId, Box<dyn Scene>>,
    current_scene: Option<SceneId>,
    scene_stack: Vec<SceneId>, // For push/pop scene transitions
}

impl SceneManager {
    pub fn new() -> Self {
        Self {
            scenes: HashMap::new(),
            current_scene: None,
            scene_stack: Vec::new(),
        }
    }

    /// Registers a scene.
    pub fn register(&mut self, scene: Box<dyn Scene>) -> SceneId {
        let id = scene.id();
        self.scenes.insert(id, scene);
        id
    }

    /// Unregisters a scene.
    pub fn unregister(&mut self, id: SceneId) -> Option<Box<dyn Scene>> {
        let removed = self.scenes.remove(&id);
        if self.current_scene == Some(id) {
            self.current_scene = None;
        }
        self.scene_stack.retain(|&item| item != id);
        removed
    }

    /// Gets a scene by ID.
    pub fn get(&self, id: SceneId) -> Option<&dyn Scene> {
        self.scenes.get(&id).map(|s| s.as_ref())
    }

    /// Gets a mutable reference to a scene.
    pub fn get_mut(&mut self, id: SceneId) -> Option<&mut dyn Scene> {
        self.scenes.get_mut(&id).map(|s| s.as_scene_mut())
    }

    /// Sets the current scene.
    pub fn set_current(&mut self, id: SceneId, context: &mut UiContext) -> Result<()> {
        if self.current_scene == Some(id) {
            return Ok(());
        }

        if let Some(current) = self.current_scene {
            if let Some(scene) = self.scenes.get_mut(&current) {
                scene.on_exit(&mut UiContext::dummy());
            }
        }

        if let Some(scene) = self.scenes.get_mut(&id) {
            scene.on_enter(&mut UiContext::dummy());
            self.current_scene = Some(id);
            Ok(())
        } else {
            Err(UiError::SceneNotFound(id.to_string()))
        }
    }

    /// Pushes a scene onto the stack (for overlays).
    pub fn push_scene(&mut self, id: SceneId, context: &mut UiContext) -> Result<()> {
        if let Some(current) = self.current_scene {
            self.scene_stack.push(current);
        }
        self.set_current(id, context)
    }

    /// Pops the current scene, returning to the previous one.
    pub fn pop_scene(&mut self, context: &mut UiContext) -> Result<()> {
        if let Some(previous) = self.scene_stack.pop() {
            self.set_current(previous, context)
        } else {
            Err(UiError::Focus("No scene to pop to".into()))
        }
    }

    /// Gets the current scene ID.
    pub fn current_scene(&self) -> Option<SceneId> {
        self.current_scene
    }

    /// Updates the current scene.
    pub fn update(&mut self, context: &mut UiContext, dt: f32) {
        if let Some(id) = self.current_scene {
            if let Some(scene) = self.scenes.get_mut(&id) {
                scene.update(context, dt);
            }
        }
    }

    /// Renders the current scene.
    pub fn render(&self, renderer: &mut dyn Renderer) {
        if let Some(id) = self.current_scene {
            if let Some(scene) = self.scenes.get(&id) {
                scene.render(renderer);
            }
        }
    }

    /// Handles input for the current scene.
    pub fn handle_input(&mut self, context: &mut UiContext, action: Action, pressed: bool) -> bool {
        if let Some(id) = self.current_scene {
            if let Some(scene) = self.scenes.get_mut(&id) {
                return scene.handle_input(context, action, pressed);
            }
        }
        false
    }
}

impl Default for SceneManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Renderer trait for UI rendering.
pub trait Renderer: Send + Sync {
    fn draw_rect(&mut self, rect: Rect, color: Color);
    fn draw_text(&mut self, text: &str, pos: (f32, f32), size: f32, color: Color);
    fn draw_image(
        &mut self,
        texture: &mclre_render::Texture,
        rect: Rect,
        uv: [f32; 4],
        color: Color,
    );
    fn draw_nine_slice(
        &mut self,
        atlas: &mclre_render::TextureAtlas,
        region: NineSliceRegion,
        rect: Rect,
        color: Color,
    );
}

/// UI context for scenes (simplified for now).
impl UiContext {
    pub fn dummy() -> Self {
        // This is a placeholder - in real usage, this would be properly constructed
        unimplemented!("UiContext::dummy() should not be called in production")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scene_id_creation() {
        let id1 = SceneId::new();
        let id2 = SceneId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn scene_manager_registration() {
        struct TestScene {
            id: SceneId,
        }
        impl Scene for TestScene {
            fn id(&self) -> SceneId {
                self.id
            }
            fn name(&self) -> &str {
                "Test"
            }
        }

        let mut manager = SceneManager::new();
        let scene = Box::new(TestScene { id: SceneId::new() });
        let id = manager.register(scene);
        assert!(manager.get(id).is_some());
    }
}
