//! Game screens matching LCE reference.

use crate::{
    Color, FocusDirection, FocusManager, Focusable, Margin, Padding, Rect, Result, UiContext,
    UiError, Widget, WidgetId, WidgetType,
    layout::{
        AlignItems, FlexContainer, FlexDirection, JustifyContent, LayoutConstraints, LayoutEngine,
        Layoutable,
    },
    scene::{Renderer, Scene, SceneId, SceneManager},
    widget::{BaseWidget, Button, Container, Label, Panel},
};
use mclre_assets::Identifier;
use mclre_input::Action;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::info;

// Screen transition types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenTransition {
    None,
    TitleToMainMenu,
    TitleToLoadWorld,
    TitleToSettings,
    TitleToHelpOptions,
    TitleToCredits,
    TitleToQuit,
    MainMenuToPlay,
    MainMenuToMultiplayer,
    MainMenuToLoadGame,
    MainMenuToOptions,
    MainMenuToTexturePacks,
    MainMenuToHelpOptions,
    MainMenuToQuit,
    PauseToResume,
    PauseToOptions,
    PauseToHelpOptions,
    PauseToQuitTitle,
    Back,
}

/// Trait for game screens.
pub trait Screen: Send + Sync {
    fn id(&self) -> SceneId;
    fn name(&self) -> &str;
    fn on_enter(&mut self, _ui: &mut UiContext) {}
    fn on_exit(&mut self, _ui: &mut UiContext) {}
    fn update(&mut self, _ui: &mut UiContext, _dt: f32) {}
    fn render(&self, _renderer: &mut dyn Renderer) {}
    fn handle_input(
        &mut self,
        _ui: &mut UiContext,
        _action: mclre_input::Action,
        _pressed: bool,
    ) -> bool {
        false
    }

    /// Take the requested screen transition, if any.
    fn take_requested_transition(&mut self) -> ScreenTransition {
        ScreenTransition::None
    }

    /// Downcast to Any for type-safe access to concrete screen types.
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

/// Screen IDs for common game screens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ScreenType {
    Title,
    MainMenu,
    LoadWorld,
    CreateWorld,
    WorldOptions,
    Pause,
    Settings,
    Controls,
    TexturePacks,
    HostOptions,
    Hud,
    Inventory,
    Crafting,
    Furnace,
    Enchanting,
    Anvil,
    Brewing,
    Trading,
    Chat,
    HelpOptions,
    Credits,
    Multiplayer,
}

impl ScreenType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ScreenType::Title => "title",
            ScreenType::MainMenu => "main_menu",
            ScreenType::LoadWorld => "load_world",
            ScreenType::CreateWorld => "create_world",
            ScreenType::WorldOptions => "world_options",
            ScreenType::Pause => "pause",
            ScreenType::Settings => "settings",
            ScreenType::Controls => "controls",
            ScreenType::TexturePacks => "texture_packs",
            ScreenType::HostOptions => "host_options",
            ScreenType::Hud => "hud",
            ScreenType::Inventory => "inventory",
            ScreenType::Crafting => "crafting",
            ScreenType::Furnace => "furnace",
            ScreenType::Enchanting => "enchanting",
            ScreenType::Anvil => "anvil",
            ScreenType::Brewing => "brewing",
            ScreenType::Trading => "trading",
            ScreenType::Chat => "chat",
            ScreenType::HelpOptions => "help_options",
            ScreenType::Credits => "credits",
            ScreenType::Multiplayer => "multiplayer",
        }
    }
}

/// Base screen implementation with common functionality.
pub struct BaseScreen {
    id: ScreenType,
    scene_id: SceneId,
    root_widget: WidgetId,
    focus_manager: FocusManager,
    widgets: HashMap<WidgetId, Box<dyn Widget + Send + Sync>>,
    visible: bool,
    background_color: Color,
    fullscreen: bool,
    requested_transition: ScreenTransition,
}

impl BaseScreen {
    pub fn new(screen_type: ScreenType) -> Self {
        Self {
            id: screen_type,
            scene_id: SceneId::new(),
            root_widget: WidgetId::new(),
            focus_manager: FocusManager::new(),
            widgets: HashMap::new(),
            visible: true,
            background_color: Color::new(0, 0, 0, 180), // Semi-transparent black
            fullscreen: false,
            requested_transition: ScreenTransition::None,
        }
    }

    pub fn root_widget(&self) -> WidgetId {
        self.root_widget
    }

    pub fn focus_manager(&mut self) -> &mut FocusManager {
        &mut self.focus_manager
    }

    pub fn set_root_widget(&mut self, widget: WidgetId) {
        self.root_widget = widget;
    }

    pub fn add_widget(&mut self, widget: Box<dyn Widget + Send + Sync>) -> WidgetId {
        let id = widget.id();
        self.widgets.insert(id, widget);
        id
    }

    pub fn get_widget(&self, id: WidgetId) -> Option<&Box<dyn Widget + Send + Sync>> {
        self.widgets.get(&id)
    }

    pub fn get_widget_mut(&mut self, id: WidgetId) -> Option<&mut Box<dyn Widget + Send + Sync>> {
        self.widgets.get_mut(&id)
    }

    pub fn background_color(&self) -> Color {
        self.background_color
    }

    pub fn set_background_color(&mut self, color: Color) {
        self.background_color = color;
    }

    pub fn is_fullscreen(&self) -> bool {
        self.fullscreen
    }

    pub fn set_fullscreen(&mut self, fullscreen: bool) {
        self.fullscreen = fullscreen;
    }

    pub fn take_requested_transition(&mut self) -> ScreenTransition {
        let transition = self.requested_transition;
        self.requested_transition = ScreenTransition::None;
        transition
    }

    pub fn request_transition(&mut self, transition: ScreenTransition) {
        self.requested_transition = transition;
    }

    /// Default handle_input for UI navigation.
    /// Screens can call this from their handle_input implementation.
    pub fn handle_ui_navigation(&mut self, action: mclre_input::Action, pressed: bool) -> bool {
        if !pressed {
            return false;
        }

        match action {
            mclre_input::Action::NavigateUp
            | mclre_input::Action::NavigateDown
            | mclre_input::Action::NavigateLeft
            | mclre_input::Action::NavigateRight => {
                let direction = match action {
                    mclre_input::Action::NavigateUp => FocusDirection::Up,
                    mclre_input::Action::NavigateDown => FocusDirection::Down,
                    mclre_input::Action::NavigateLeft => FocusDirection::Left,
                    mclre_input::Action::NavigateRight => FocusDirection::Right,
                    _ => return false,
                };
                let _ = self.focus_manager.move_focus(direction);
                true
            }
            mclre_input::Action::NavigateSelect
            | mclre_input::Action::UseItem
            | mclre_input::Action::Attack => {
                if let Some(focused) = self.focus_manager.focused() {
                    if let Some(widget) = self.get_widget(focused) {
                        if let Some(on_click) = widget.on_click_fn() {
                            on_click();
                            return true;
                        }
                    }
                }
                false
            }
            mclre_input::Action::NavigateBack | mclre_input::Action::Pause => {
                // Default: request back transition
                self.request_transition(ScreenTransition::Back);
                true
            }
            mclre_input::Action::NavigateNext => {
                let _ = self.focus_manager.move_focus(FocusDirection::Next);
                true
            }
            mclre_input::Action::NavigatePrevious => {
                let _ = self.focus_manager.move_focus(FocusDirection::Previous);
                true
            }
            _ => false,
        }
    }

    /// Recursively render the widget tree.
    pub fn render_widgets(&self, renderer: &mut dyn crate::scene::Renderer) {
        self.render_widget_recursive(renderer, self.root_widget);
    }

    fn render_widget_recursive(
        &self,
        renderer: &mut dyn crate::scene::Renderer,
        widget_id: WidgetId,
    ) {
        if let Some(widget) = self.widgets.get(&widget_id) {
            let rect = widget.rect();
            if !widget.visible() {
                return;
            }

            // Draw widget based on type
            match widget.widget_type() {
                crate::WidgetType::Button => {
                    if let Some(btn) = widget.as_any().downcast_ref::<crate::widget::Button>() {
                        // Draw button background
                        let bg_color = crate::Color::rgba(
                            btn.background_color[0],
                            btn.background_color[1],
                            btn.background_color[2],
                            btn.background_color[3],
                        );
                        renderer.draw_rect(rect, bg_color);
                        // Draw button text
                        let text_color = crate::Color::rgba(
                            btn.text_color[0],
                            btn.text_color[1],
                            btn.text_color[2],
                            btn.text_color[3],
                        );
                        let text_pos = (rect.x + rect.width * 0.5, rect.y + rect.height * 0.5);
                        renderer.draw_text(&btn.label, text_pos, btn.font_size, text_color);
                    }
                }
                crate::WidgetType::Label => {
                    if let Some(label) = widget.as_any().downcast_ref::<crate::widget::Label>() {
                        let text_color = crate::Color::rgba(
                            label.color[0],
                            label.color[1],
                            label.color[2],
                            label.color[3],
                        );
                        renderer.draw_text(
                            &label.text,
                            (rect.x, rect.y),
                            label.font_size,
                            text_color,
                        );
                    }
                }
                crate::WidgetType::Panel => {
                    if let Some(panel) = widget.as_any().downcast_ref::<crate::widget::Panel>() {
                        let bg_color = crate::Color::rgba(
                            panel.background_color[0],
                            panel.background_color[1],
                            panel.background_color[2],
                            panel.background_color[3],
                        );
                        renderer.draw_rect(rect, bg_color);
                        if panel.border_width > 0.0 {
                            let border_color = crate::Color::rgba(
                                panel.border_color[0],
                                panel.border_color[1],
                                panel.border_color[2],
                                panel.border_color[3],
                            );
                            // Draw border as four thin rects
                            renderer.draw_rect(
                                crate::Rect::new(rect.x, rect.y, rect.width, panel.border_width),
                                border_color,
                            );
                            renderer.draw_rect(
                                crate::Rect::new(
                                    rect.x,
                                    rect.y + rect.height - panel.border_width,
                                    rect.width,
                                    panel.border_width,
                                ),
                                border_color,
                            );
                            renderer.draw_rect(
                                crate::Rect::new(rect.x, rect.y, panel.border_width, rect.height),
                                border_color,
                            );
                            renderer.draw_rect(
                                crate::Rect::new(
                                    rect.x + rect.width - panel.border_width,
                                    rect.y,
                                    panel.border_width,
                                    rect.height,
                                ),
                                border_color,
                            );
                        }
                    }
                }
                crate::WidgetType::Image => {
                    if let Some(img) = widget.as_any().downcast_ref::<crate::widget::Image>() {
                        let tint =
                            crate::Color::rgba(img.tint[0], img.tint[1], img.tint[2], img.tint[3]);
                        // Note: texture lookup would need asset manager; for now skip if no texture
                        if let Some(_texture_id) = &img.texture_id {
                            // TODO: Look up texture from asset manager
                            // renderer.draw_image(texture, rect, img.uv_rect.unwrap_or([0.0, 0.0, 1.0, 1.0]), tint);
                        }
                    }
                }
                _ => {}
            }

            // Recursively render children
            for child_id in widget.children() {
                self.render_widget_recursive(renderer, *child_id);
            }
        }
    }
}

/// Main title screen matching LCE reference.
pub struct TitleScreen {
    base: BaseScreen,
    version_text: String,
    copyright_text: String,
    selected_option: usize,
    options: Vec<String>,
}

impl TitleScreen {
    pub fn new() -> Self {
        let mut base = BaseScreen::new(ScreenType::Title);
        base.set_fullscreen(true);
        base.background_color = Color::new(0, 0, 0, 255); // Solid black

        let mut screen = Self {
            base,
            version_text: "Minecraft: Legacy Console Edition".to_string(),
            copyright_text: "Copyright Mojang AB".to_string(),
            selected_option: 0,
            options: vec![
                "Play Game".to_string(),
                "Load Game".to_string(),
                "Settings".to_string(),
                "Help & Options".to_string(),
                "Credits".to_string(),
                "Quit Game".to_string(),
            ],
        };

        // Create title screen layout
        screen.build_layout();
        screen
    }

    fn build_layout(&mut self) {
        // Create main container
        let mut container = Container::new();
        container.set_rect(Rect::new(0.0, 0.0, 1920.0, 1080.0));

        // Title logo area
        let mut title_label = Label::new("MINECRAFT")
            .font_size(72.0)
            .color([255, 255, 255, 255]);
        title_label.set_rect(Rect::new(660.0, 150.0, 600.0, 100.0));
        let title_id = self.base.add_widget(Box::new(title_label));

        // Version text
        let mut version_label = Label::new("Legacy Console Edition")
            .font_size(24.0)
            .color([200, 200, 200, 255]);
        version_label.set_rect(Rect::new(720.0, 240.0, 480.0, 40.0));
        let _ = self.base.add_widget(Box::new(version_label));

        // Copyright
        let mut copyright_label = Label::new("Copyright Mojang AB")
            .font_size(16.0)
            .color([128, 128, 128, 255]);
        copyright_label.set_rect(Rect::new(740.0, 1000.0, 440.0, 30.0));
        let _ = self.base.add_widget(Box::new(copyright_label));

        // Options panel
        let mut panel = Panel::new()
            .background_color([32, 32, 32, 240])
            .border_color([128, 128, 128, 255])
            .border_width(2.0);
        panel.set_rect(Rect::new(660.0, 320.0, 600.0, 480.0));
        let panel_id = self.base.add_widget(Box::new(panel));

        // Create option buttons
        let mut option_buttons = Vec::new();
        for (i, option) in self.options.iter().enumerate() {
            let y = 360.0 + i as f32 * 70.0;
            let option_text = option.clone();
            let mut button = Button::new(option_text.clone())
                .font_size(28.0)
                .background_color([64, 64, 64, 255])
                .text_color([255, 255, 255, 255]);

            // Add on_click handler based on option text
            let transition = match option_text.as_str() {
                "Play Game" => ScreenTransition::TitleToMainMenu,
                "Load Game" => ScreenTransition::TitleToLoadWorld,
                "Settings" => ScreenTransition::TitleToSettings,
                "Help & Options" => ScreenTransition::TitleToHelpOptions,
                "Credits" => ScreenTransition::TitleToCredits,
                "Quit Game" => ScreenTransition::TitleToQuit,
                _ => ScreenTransition::None,
            };
            button = button.on_click(move || {
                // The transition will be handled by checking the focused widget
                // when UseItem/Enter is pressed
            });

            button.set_rect(Rect::new(710.0, y, 500.0, 55.0));
            let button_id = self.base.add_widget(Box::new(button));
            self.base
                .focus_manager
                .register_focusable(button_id, i as i32);
            option_buttons.push(button_id);
        }

        // Register panel and title as focusable too (but not actually focusable)
        // Set first button as focused
        if let Some(first) = option_buttons.first() {
            let _ = self.base.focus_manager.set_focus(Some(*first));
        }

        // Set container as root
        let mut container = Container::new();
        container.set_rect(Rect::new(0.0, 0.0, 1920.0, 1080.0));
        container.add_child(title_id);
        container.add_child(panel_id);
        for btn_id in &option_buttons {
            container.add_child(*btn_id);
        }

        let root_id = self.base.add_widget(Box::new(container));
        self.base.set_root_widget(root_id);
    }
}

impl Screen for TitleScreen {
    fn id(&self) -> SceneId {
        self.base.scene_id
    }

    fn name(&self) -> &str {
        "Title Screen"
    }

    fn on_enter(&mut self, _ui: &mut UiContext) {
        // Initialize title screen
    }

    fn on_exit(&mut self, _ui: &mut UiContext) {
        // Cleanup
    }

    fn update(&mut self, _ui: &mut UiContext, _dt: f32) {
        // Update animations, etc.
    }

    fn render(&self, renderer: &mut dyn crate::scene::Renderer) {
        // Draw fullscreen background
        if self.base.is_fullscreen() {
            let bg = self.base.background_color();
            renderer.draw_rect(crate::Rect::new(0.0, 0.0, 1920.0, 1080.0), bg);
        }
        // Render widget tree
        self.base.render_widgets(renderer);
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn handle_input(
        &mut self,
        _ui: &mut UiContext,
        action: mclre_input::Action,
        pressed: bool,
    ) -> bool {
        // First, try the base UI navigation handler
        if self.base.handle_ui_navigation(action, pressed) {
            return true;
        }

        // Screen-specific handling
        if !pressed {
            return false;
        }

        match action {
            mclre_input::Action::NavigateSelect
            | mclre_input::Action::UseItem
            | mclre_input::Action::Attack => {
                if let Some(focused) = self.base.focus_manager.focused() {
                    if let Some(widget) = self.base.get_widget(focused) {
                        if let Some(on_click) = widget.on_click_fn() {
                            on_click();

                            // Check the focused widget's label to determine transition
                            if let Some(label_widget) = widget.as_any().downcast_ref::<Button>() {
                                let label = label_widget.label.clone();
                                let transition = match label.as_str() {
                                    "Play Game" => ScreenTransition::TitleToMainMenu,
                                    "Load Game" => ScreenTransition::TitleToLoadWorld,
                                    "Settings" => ScreenTransition::TitleToSettings,
                                    "Help & Options" => ScreenTransition::TitleToHelpOptions,
                                    "Credits" => ScreenTransition::TitleToCredits,
                                    "Quit Game" => ScreenTransition::TitleToQuit,
                                    _ => ScreenTransition::None,
                                };
                                self.base.request_transition(transition);
                            }
                            return true;
                        }
                    }
                }
                false
            }
            mclre_input::Action::Pause => {
                // On title screen, cancel does nothing
                true
            }
            _ => false,
        }
    }
}

/// Main menu screen.
pub struct MainMenuScreen {
    base: BaseScreen,
    options: Vec<String>,
    selected_option: usize,
}

impl MainMenuScreen {
    pub fn new() -> Self {
        let mut base = BaseScreen::new(ScreenType::MainMenu);
        base.set_fullscreen(true);

        let mut screen = Self {
            base,
            options: vec![
                "Play".to_string(),
                "Multiplayer".to_string(),
                "Load Game".to_string(),
                "Settings".to_string(),
                "Texture Packs".to_string(),
                "Help & Options".to_string(),
                "Quit".to_string(),
            ],
            selected_option: 0,
        };

        screen.build_layout();
        screen
    }

    fn build_layout(&mut self) {
        // Similar to title screen but with different options
        let mut container = Container::new();
        container.set_rect(Rect::new(0.0, 0.0, 1920.0, 1080.0));

        // Title
        let mut title_label = Label::new("MINECRAFT")
            .font_size(72.0)
            .color([255, 255, 255, 255]);
        title_label.set_rect(Rect::new(660.0, 150.0, 600.0, 100.0));
        let title_id = self.base.add_widget(Box::new(title_label));

        // Version text
        let mut version_label = Label::new("Legacy Console Edition")
            .font_size(24.0)
            .color([200, 200, 200, 255]);
        version_label.set_rect(Rect::new(720.0, 240.0, 480.0, 40.0));
        let _ = self.base.add_widget(Box::new(version_label));

        // Options panel
        let mut panel = Panel::new()
            .background_color([32, 32, 32, 240])
            .border_color([128, 128, 128, 255])
            .border_width(2.0);
        panel.set_rect(Rect::new(660.0, 320.0, 600.0, 560.0));
        let panel_id = self.base.add_widget(Box::new(panel));

        // Create option buttons
        let mut option_buttons = Vec::new();
        for (i, option) in self.options.iter().enumerate() {
            let y = 360.0 + i as f32 * 70.0;
            let option_text = option.clone();
            let mut button = Button::new(option_text.clone())
                .font_size(28.0)
                .background_color([64, 64, 64, 255])
                .text_color([255, 255, 255, 255]);

            // Add on_click handler based on option text
            let transition = match option_text.as_str() {
                "Play" => ScreenTransition::MainMenuToPlay,
                "Multiplayer" => ScreenTransition::MainMenuToMultiplayer,
                "Load Game" => ScreenTransition::MainMenuToLoadGame,
                "Settings" => ScreenTransition::MainMenuToOptions,
                "Texture Packs" => ScreenTransition::MainMenuToTexturePacks,
                "Help & Options" => ScreenTransition::MainMenuToHelpOptions,
                "Quit" => ScreenTransition::MainMenuToQuit,
                _ => ScreenTransition::None,
            };
            button = button.on_click(move || {
                // The transition will be handled by checking the focused widget
                // when UseItem/Enter is pressed
            });

            button.set_rect(Rect::new(710.0, y, 500.0, 55.0));
            let button_id = self.base.add_widget(Box::new(button));
            self.base
                .focus_manager
                .register_focusable(button_id, i as i32);
            option_buttons.push(button_id);
        }

        // Set container as root
        let mut container = Container::new();
        container.set_rect(Rect::new(0.0, 0.0, 1920.0, 1080.0));
        container.add_child(title_id);
        container.add_child(panel_id);
        for btn_id in &option_buttons {
            container.add_child(*btn_id);
        }

        let root_id = self.base.add_widget(Box::new(container));
        self.base.set_root_widget(root_id);

        // Set first button as focused
        if let Some(first) = option_buttons.first() {
            let _ = self.base.focus_manager.set_focus(Some(*first));
        }
    }
}

impl Screen for MainMenuScreen {
    fn id(&self) -> SceneId {
        self.base.scene_id
    }

    fn name(&self) -> &str {
        "Main Menu"
    }

    fn on_enter(&mut self, _ui: &mut UiContext) {}
    fn on_exit(&mut self, _ui: &mut UiContext) {}
    fn update(&mut self, _ui: &mut UiContext, _dt: f32) {}
    fn render(&self, _renderer: &mut dyn crate::scene::Renderer) {}

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn handle_input(
        &mut self,
        _ui: &mut UiContext,
        action: mclre_input::Action,
        pressed: bool,
    ) -> bool {
        // First, try the base UI navigation handler
        if self.base.handle_ui_navigation(action, pressed) {
            return true;
        }

        // Screen-specific handling
        if !pressed {
            return false;
        }
        match action {
            mclre_input::Action::NavigateSelect | mclre_input::Action::UseItem => {
                if let Some(focused) = self.base.focus_manager.focused() {
                    if let Some(widget) = self.base.get_widget(focused) {
                        if let Some(on_click) = widget.on_click_fn() {
                            on_click();

                            // Check the focused widget's label to determine transition
                            if let Some(label_widget) = widget.as_any().downcast_ref::<Button>() {
                                let label = label_widget.label.clone();
                                let transition = match label.as_str() {
                                    "Play" => ScreenTransition::MainMenuToPlay,
                                    "Load Game" => ScreenTransition::MainMenuToLoadGame,
                                    "Settings" => ScreenTransition::MainMenuToOptions,
                                    "Texture Packs" => ScreenTransition::MainMenuToTexturePacks,
                                    "Help & Options" => ScreenTransition::MainMenuToHelpOptions,
                                    "Quit" => ScreenTransition::MainMenuToQuit,
                                    _ => ScreenTransition::None,
                                };
                                self.base.request_transition(transition);
                            }
                            return true;
                        }
                    }
                }
                false
            }
            mclre_input::Action::Pause => {
                self.base.focus_manager.set_focus(None);
                true
            }
            _ => false,
        }
    }
}

/// Pause menu overlay.
pub struct PauseScreen {
    base: BaseScreen,
    options: Vec<String>,
    selected_option: usize,
}

impl PauseScreen {
    pub fn new() -> Self {
        let mut base = BaseScreen::new(ScreenType::Pause);
        base.set_fullscreen(false); // Overlay, not fullscreen
        base.background_color = Color::new(0, 0, 0, 180);

        let mut screen = Self {
            base,
            options: vec![
                "Resume Game".to_string(),
                "Save and Quit to Title".to_string(),
                "Options".to_string(),
                "Help & Options".to_string(),
            ],
            selected_option: 0,
        };

        screen.build_layout();
        screen
    }

    fn build_layout(&mut self) {
        // Pause menu centered on screen
        let mut panel = Panel::new()
            .background_color([32, 32, 32, 255])
            .border_color([255, 215, 0, 255]) // Gold border
            .border_width(3.0);
        panel.set_rect(Rect::new(560.0, 240.0, 800.0, 480.0));
        let panel_id = self.base.add_widget(Box::new(panel));

        // Title
        let mut title = Label::new("Game Paused")
            .font_size(48.0)
            .color([255, 255, 255, 255]);
        title.set_rect(Rect::new(660.0, 280.0, 600.0, 80.0));
        let title_id = self.base.add_widget(Box::new(title));

        // Options
        let mut option_buttons = Vec::new();
        for (i, option) in self.options.iter().enumerate() {
            let y = 380.0 + i as f32 * 80.0;
            let mut button = Button::new(option.clone())
                .font_size(32.0)
                .background_color([64, 64, 64, 255])
                .text_color([255, 255, 255, 255]);
            button.set_rect(Rect::new(660.0, y, 480.0, 65.0));
            let button_id = self.base.add_widget(Box::new(button));
            self.base
                .focus_manager
                .register_focusable(button_id, i as i32);
            option_buttons.push(button_id);
        }

        // Container
        let mut container = Container::new();
        container.set_rect(Rect::new(0.0, 0.0, 1920.0, 1080.0));
        container.add_child(title_id);
        container.add_child(panel_id);
        for btn_id in &option_buttons {
            container.add_child(*btn_id);
        }

        let root_id = self.base.add_widget(Box::new(container));
        self.base.set_root_widget(root_id);

        // Focus first option
        if let Some(first) = option_buttons.first() {
            let _ = self.base.focus_manager.set_focus(Some(*first));
        }
    }
}

impl Screen for PauseScreen {
    fn id(&self) -> SceneId {
        self.base.scene_id
    }

    fn name(&self) -> &str {
        "Pause Menu"
    }

    fn on_enter(&mut self, _ui: &mut UiContext) {}
    fn on_exit(&mut self, _ui: &mut UiContext) {}
    fn update(&mut self, _ui: &mut UiContext, _dt: f32) {}
    fn render(&self, _renderer: &mut dyn crate::scene::Renderer) {}

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn handle_input(
        &mut self,
        _ui: &mut UiContext,
        action: mclre_input::Action,
        pressed: bool,
    ) -> bool {
        // First, try the base UI navigation handler
        if self.base.handle_ui_navigation(action, pressed) {
            return true;
        }

        // Screen-specific handling
        if !pressed {
            return false;
        }
        match action {
            mclre_input::Action::NavigateSelect | mclre_input::Action::UseItem => {
                if let Some(focused) = self.base.focus_manager.focused() {
                    if let Some(widget) = self.base.get_widget(focused) {
                        if let Some(on_click) = widget.on_click_fn() {
                            on_click();
                            return true;
                        }
                    }
                }
                false
            }
            mclre_input::Action::Pause => {
                // Resume game
                true
            }
            _ => false,
        }
    }
}

/// HUD overlay for in-game display.
pub struct HudScreen {
    base: BaseScreen,
    health: i32,
    max_health: i32,
    hunger: i32,
    max_hunger: i32,
    armor: i32,
    air: i32,
    max_air: i32,
    xp_level: i32,
    xp_progress: f32,
    hotbar_selected: usize,
    hotbar_items: Vec<Option<HotbarSlot>>,
    xp_total: u32,
}

#[derive(Debug, Clone)]
pub struct HotbarSlot {
    pub item_id: u32,
    pub count: u8,
    pub damage: Option<u16>,
}

impl HudScreen {
    pub fn new() -> Self {
        let mut base = BaseScreen::new(ScreenType::Inventory); // Using Inventory type for HUD
        base.set_fullscreen(true);
        base.background_color = Color::TRANSPARENT;

        Self {
            base,
            health: 20,
            max_health: 20,
            hunger: 20,
            max_hunger: 20,
            armor: 0,
            air: 300,
            max_air: 300,
            xp_level: 1,
            xp_progress: 0.0,
            hotbar_selected: 0,
            hotbar_items: vec![None; 9],
            xp_total: 0,
        }
    }

    pub fn set_health(&mut self, health: i32, max: i32) {
        self.health = health.clamp(0, max);
        self.max_health = max;
    }

    pub fn set_hunger(&mut self, hunger: i32, max: i32) {
        self.hunger = hunger.clamp(0, max);
        self.max_hunger = max;
    }

    pub fn set_armor(&mut self, armor: i32) {
        self.armor = armor.clamp(0, 20);
    }

    pub fn set_air(&mut self, air: i32, max: i32) {
        self.air = air.clamp(0, max);
        self.max_air = max;
    }

    pub fn set_xp(&mut self, level: i32, progress: f32, total: u32) {
        self.xp_level = level;
        self.xp_progress = progress.clamp(0.0, 1.0);
        self.xp_total = total;
    }

    pub fn set_hotbar_slot(&mut self, index: usize, item: Option<HotbarSlot>) {
        if index < self.hotbar_items.len() {
            self.hotbar_items[index] = item;
        }
    }

    pub fn select_hotbar_slot(&mut self, index: usize) {
        if index < self.hotbar_items.len() {
            self.hotbar_selected = index;
        }
    }
}

impl Screen for HudScreen {
    fn id(&self) -> SceneId {
        self.base.scene_id
    }

    fn name(&self) -> &str {
        "HUD"
    }

    fn on_enter(&mut self, _ui: &mut UiContext) {}
    fn on_exit(&mut self, _ui: &mut UiContext) {}
    fn update(&mut self, _ui: &mut UiContext, _dt: f32) {}
    fn render(&self, _renderer: &mut dyn crate::scene::Renderer) {}

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn handle_input(
        &mut self,
        _ui: &mut UiContext,
        _action: mclre_input::Action,
        _pressed: bool,
    ) -> bool {
        false // HUD doesn't handle input directly
    }
}

/// Load world screen - shows saved worlds list.
pub struct LoadWorldScreen {
    base: BaseScreen,
    world_list: Vec<WorldEntry>,
    selected_index: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct WorldEntry {
    pub name: String,
    pub path: String,
    pub last_played: String,
    pub game_mode: String,
    pub version: String,
    pub icon: Option<Identifier>,
}

impl LoadWorldScreen {
    pub fn new() -> Self {
        let mut base = BaseScreen::new(ScreenType::LoadWorld);
        base.set_fullscreen(true);

        let mut screen = Self {
            base,
            world_list: Vec::new(),
            selected_index: None,
        };

        screen.build_layout();
        screen
    }

    fn build_layout(&mut self) {
        let mut container = Container::new();
        container.set_rect(Rect::new(0.0, 0.0, 1920.0, 1080.0));

        // Title
        let mut title = Label::new("Select World")
            .font_size(48.0)
            .color([255, 255, 255, 255]);
        title.set_rect(Rect::new(660.0, 50.0, 600.0, 80.0));
        let title_id = self.base.add_widget(Box::new(title));

        // World list panel
        let mut panel = Panel::new()
            .background_color([32, 32, 32, 240])
            .border_color([128, 128, 128, 255])
            .border_width(2.0);
        panel.set_rect(Rect::new(260.0, 160.0, 1400.0, 720.0));
        let panel_id = self.base.add_widget(Box::new(panel));

        // Buttons panel
        let mut btn_panel = Panel::new()
            .background_color([32, 32, 32, 240])
            .border_color([128, 128, 128, 255])
            .border_width(2.0);
        btn_panel.set_rect(Rect::new(260.0, 900.0, 1400.0, 120.0));
        let btn_panel_id = self.base.add_widget(Box::new(btn_panel));

        // Action buttons
        let mut buttons = Vec::new();
        let button_labels = ["Play", "Edit", "Delete", "Re-Create", "Cancel"];
        for (i, label) in button_labels.iter().enumerate() {
            let x = 360.0 + i as f32 * 280.0;
            let mut btn = Button::new(label.to_string())
                .font_size(24.0)
                .background_color([64, 64, 64, 255])
                .text_color([255, 255, 255, 255]);
            btn.set_rect(Rect::new(x, 920.0, 240.0, 60.0));
            let btn_id = self.base.add_widget(Box::new(btn));
            self.base.focus_manager.register_focusable(btn_id, i as i32);
            buttons.push(btn_id);
        }

        // World list entries (placeholder - would be populated from saves)
        let mut entry_buttons = Vec::new();
        for (i, entry) in self.world_list.iter().enumerate() {
            let y = 200.0 + i as f32 * 80.0;
            if y > 800.0 {
                break;
            }

            let mut entry_btn = Button::new(entry.name.clone())
                .font_size(20.0)
                .background_color([48, 48, 48, 255])
                .text_color([255, 255, 255, 255]);
            entry_btn.set_rect(Rect::new(300.0, y, 1300.0, 70.0));
            let btn_id = self.base.add_widget(Box::new(entry_btn));
            self.base
                .focus_manager
                .register_focusable(btn_id, 10 + i as i32);
            entry_buttons.push(btn_id);
        }

        // Container
        let mut container = Container::new();
        container.set_rect(Rect::new(0.0, 0.0, 1920.0, 1080.0));
        container.add_child(title_id);
        container.add_child(panel_id);
        container.add_child(btn_panel_id);
        for btn_id in &buttons {
            container.add_child(*btn_id);
        }
        for btn_id in &entry_buttons {
            container.add_child(*btn_id);
        }

        let root_id = self.base.add_widget(Box::new(container));
        self.base.set_root_widget(root_id);

        // Focus first action button
        if let Some(first) = buttons.first() {
            let _ = self.base.focus_manager.set_focus(Some(*first));
        }
    }

    pub fn refresh_world_list(&mut self) {
        // TODO: Load actual world saves from disk
        self.world_list.clear();
        // Placeholder entries
        self.world_list.push(WorldEntry {
            name: "New World".to_string(),
            path: "saves/new_world".to_string(),
            last_played: "Today 14:30".to_string(),
            game_mode: "Survival".to_string(),
            version: "1.0.0".to_string(),
            icon: None,
        });
        self.world_list.push(WorldEntry {
            name: "Creative World".to_string(),
            path: "saves/creative".to_string(),
            last_played: "Yesterday".to_string(),
            game_mode: "Creative".to_string(),
            version: "1.0.0".to_string(),
            icon: None,
        });
        // Rebuild layout would be needed here
    }
}

impl Screen for LoadWorldScreen {
    fn id(&self) -> SceneId {
        self.base.scene_id
    }

    fn name(&self) -> &str {
        "Load World"
    }

    fn on_enter(&mut self, _ui: &mut UiContext) {
        self.refresh_world_list();
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn handle_input(
        &mut self,
        _ui: &mut UiContext,
        action: mclre_input::Action,
        pressed: bool,
    ) -> bool {
        if !pressed {
            return false;
        }
        match action {
            mclre_input::Action::NavigateUp
            | mclre_input::Action::NavigateDown
            | mclre_input::Action::NavigateLeft
            | mclre_input::Action::NavigateRight => {
                let direction = match action {
                    mclre_input::Action::NavigateUp => FocusDirection::Up,
                    mclre_input::Action::NavigateDown => FocusDirection::Down,
                    mclre_input::Action::NavigateLeft => FocusDirection::Left,
                    mclre_input::Action::NavigateRight => FocusDirection::Right,
                    _ => return false,
                };
                let _ = self.base.focus_manager.move_focus(direction);
                true
            }
            mclre_input::Action::NavigateSelect | mclre_input::Action::UseItem => {
                let _ = self.base.focus_manager.set_focus(None);
                true
            }
            _ => false,
        }
    }
}

/// Create world screen.
pub struct CreateWorldScreen {
    base: BaseScreen,
    world_name: String,
    seed: String,
    game_mode: usize,  // 0=Survival, 1=Creative, 2=Adventure
    world_type: usize, // 0=Default, 1=Superflat, 2=Large Biomes, etc.
    generate_structures: bool,
    bonus_chest: bool,
    cheats_allowed: bool,
}

impl CreateWorldScreen {
    pub fn new() -> Self {
        let mut base = BaseScreen::new(ScreenType::CreateWorld);
        base.set_fullscreen(true);

        let mut screen = Self {
            base,
            world_name: "New World".to_string(),
            seed: String::new(),
            game_mode: 0,
            world_type: 0,
            generate_structures: true,
            bonus_chest: false,
            cheats_allowed: false,
        };

        screen.build_layout();
        screen
    }

    fn build_layout(&mut self) {
        let mut container = Container::new();
        container.set_rect(Rect::new(0.0, 0.0, 1920.0, 1080.0));

        // Title
        let mut title = Label::new("Create New World")
            .font_size(48.0)
            .color([255, 255, 255, 255]);
        title.set_rect(Rect::new(660.0, 50.0, 600.0, 80.0));
        let title_id = self.base.add_widget(Box::new(title));

        // Main panel
        let mut panel = Panel::new()
            .background_color([32, 32, 32, 240])
            .border_color([128, 128, 128, 255])
            .border_width(2.0);
        panel.set_rect(Rect::new(460.0, 160.0, 1000.0, 720.0));
        let panel_id = self.base.add_widget(Box::new(panel));

        // Buttons panel
        let mut btn_panel = Panel::new()
            .background_color([32, 32, 32, 240])
            .border_color([128, 128, 128, 255])
            .border_width(2.0);
        btn_panel.set_rect(Rect::new(460.0, 900.0, 1000.0, 120.0));
        let btn_panel_id = self.base.add_widget(Box::new(btn_panel));

        // Form fields
        let mut focusables = Vec::new();

        // World Name
        let mut name_label = Label::new("World Name:")
            .font_size(24.0)
            .color([255, 255, 255, 255]);
        name_label.set_rect(Rect::new(500.0, 200.0, 300.0, 40.0));
        let name_label_id = self.base.add_widget(Box::new(name_label));

        let mut name_input = Button::new(self.world_name.clone())
            .font_size(24.0)
            .background_color([48, 48, 48, 255])
            .text_color([255, 255, 255, 255]);
        name_input.set_rect(Rect::new(500.0, 240.0, 900.0, 50.0));
        let name_input_id = self.base.add_widget(Box::new(name_input));
        self.base.focus_manager.register_focusable(name_input_id, 0);
        focusables.push(name_input_id);

        // Seed
        let mut seed_label = Label::new("Seed (optional):")
            .font_size(24.0)
            .color([255, 255, 255, 255]);
        seed_label.set_rect(Rect::new(500.0, 310.0, 300.0, 40.0));
        let seed_label_id = self.base.add_widget(Box::new(seed_label));

        let mut seed_input = Button::new(self.seed.clone())
            .font_size(24.0)
            .background_color([48, 48, 48, 255])
            .text_color([200, 200, 200, 255]);
        seed_input.set_rect(Rect::new(500.0, 350.0, 900.0, 50.0));
        let seed_input_id = self.base.add_widget(Box::new(seed_input));
        self.base.focus_manager.register_focusable(seed_input_id, 1);
        focusables.push(seed_input_id);

        // Game Mode
        let mut gm_label = Label::new("Game Mode:")
            .font_size(24.0)
            .color([255, 255, 255, 255]);
        gm_label.set_rect(Rect::new(500.0, 420.0, 300.0, 40.0));
        let gm_label_id = self.base.add_widget(Box::new(gm_label));

        let game_modes = ["Survival", "Creative", "Adventure"];
        for (i, mode) in game_modes.iter().enumerate() {
            let x = 500.0 + i as f32 * 310.0;
            let mut btn = Button::new(mode.to_string())
                .font_size(22.0)
                .background_color(if i == self.game_mode {
                    [0, 128, 0, 255]
                } else {
                    [64, 64, 64, 255]
                })
                .text_color([255, 255, 255, 255]);
            btn.set_rect(Rect::new(x, 460.0, 280.0, 50.0));
            let btn_id = self.base.add_widget(Box::new(btn));
            self.base
                .focus_manager
                .register_focusable(btn_id, 2 + i as i32);
            focusables.push(btn_id);
        }

        // World Type
        let mut wt_label = Label::new("World Type:")
            .font_size(24.0)
            .color([255, 255, 255, 255]);
        wt_label.set_rect(Rect::new(500.0, 530.0, 300.0, 40.0));
        let wt_label_id = self.base.add_widget(Box::new(wt_label));

        let world_types = ["Default", "Superflat", "Large Biomes", "Single Biome"];
        for (i, wtype) in world_types.iter().enumerate() {
            let x = 500.0 + i as f32 * 230.0;
            let mut btn = Button::new(wtype.to_string())
                .font_size(20.0)
                .background_color(if i == self.world_type {
                    [0, 128, 0, 255]
                } else {
                    [64, 64, 64, 255]
                })
                .text_color([255, 255, 255, 255]);
            btn.set_rect(Rect::new(x, 570.0, 200.0, 50.0));
            let btn_id = self.base.add_widget(Box::new(btn));
            self.base
                .focus_manager
                .register_focusable(btn_id, 5 + i as i32);
            focusables.push(btn_id);
        }

        // Options checkboxes
        let options = [
            ("Generate Structures", self.generate_structures),
            ("Bonus Chest", self.bonus_chest),
            ("Allow Cheats", self.cheats_allowed),
        ];
        for (i, (label, enabled)) in options.iter().enumerate() {
            let y = 650.0 + i as f32 * 50.0;
            let checkbox_label = if *enabled { "[X] " } else { "[ ] " };
            let mut btn = Button::new(format!("{}{}", checkbox_label, label))
                .font_size(22.0)
                .background_color([48, 48, 48, 255])
                .text_color([255, 255, 255, 255]);
            btn.set_rect(Rect::new(500.0, y, 900.0, 40.0));
            let btn_id = self.base.add_widget(Box::new(btn));
            self.base
                .focus_manager
                .register_focusable(btn_id, 9 + i as i32);
            focusables.push(btn_id);
        }

        // Action buttons
        let mut buttons = Vec::new();
        let button_labels = ["Create New World", "Cancel"];
        for (i, label) in button_labels.iter().enumerate() {
            let x = 500.0 + i as f32 * 480.0;
            let mut btn = Button::new(label.to_string())
                .font_size(24.0)
                .background_color(if i == 0 {
                    [0, 100, 0, 255]
                } else {
                    [100, 0, 0, 255]
                })
                .text_color([255, 255, 255, 255]);
            btn.set_rect(Rect::new(x, 920.0, 440.0, 60.0));
            let btn_id = self.base.add_widget(Box::new(btn));
            self.base
                .focus_manager
                .register_focusable(btn_id, 12 + i as i32);
            buttons.push(btn_id);
        }

        // Container
        let mut container = Container::new();
        container.set_rect(Rect::new(0.0, 0.0, 1920.0, 1080.0));
        container.add_child(title_id);
        container.add_child(panel_id);
        container.add_child(btn_panel_id);
        container.add_child(name_label_id);
        container.add_child(seed_label_id);
        container.add_child(gm_label_id);
        container.add_child(wt_label_id);
        for btn_id in &focusables {
            container.add_child(*btn_id);
        }
        for btn_id in &buttons {
            container.add_child(*btn_id);
        }

        let root_id = self.base.add_widget(Box::new(container));
        self.base.set_root_widget(root_id);

        if let Some(first) = focusables.first() {
            let _ = self.base.focus_manager.set_focus(Some(*first));
        }
    }
}

impl Screen for CreateWorldScreen {
    fn id(&self) -> SceneId {
        self.base.scene_id
    }

    fn name(&self) -> &str {
        "Create World"
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn handle_input(
        &mut self,
        _ui: &mut UiContext,
        action: mclre_input::Action,
        pressed: bool,
    ) -> bool {
        if !pressed {
            return false;
        }
        match action {
            mclre_input::Action::NavigateUp
            | mclre_input::Action::NavigateDown
            | mclre_input::Action::NavigateLeft
            | mclre_input::Action::NavigateRight => {
                let direction = match action {
                    mclre_input::Action::NavigateUp => FocusDirection::Up,
                    mclre_input::Action::NavigateDown => FocusDirection::Down,
                    mclre_input::Action::NavigateLeft => FocusDirection::Left,
                    mclre_input::Action::NavigateRight => FocusDirection::Right,
                    _ => return false,
                };
                let _ = self.base.focus_manager.move_focus(direction);
                true
            }
            mclre_input::Action::NavigateSelect | mclre_input::Action::UseItem => {
                let _ = self.base.focus_manager.set_focus(None);
                true
            }
            _ => false,
        }
    }
}

/// World options screen (when editing a world).
pub struct WorldOptionsScreen {
    base: BaseScreen,
}

impl WorldOptionsScreen {
    pub fn new() -> Self {
        let mut base = BaseScreen::new(ScreenType::WorldOptions);
        base.set_fullscreen(true);

        let mut screen = Self { base };
        screen.build_layout();
        screen
    }

    fn build_layout(&mut self) {
        let mut container = Container::new();
        container.set_rect(Rect::new(0.0, 0.0, 1920.0, 1080.0));

        // Title
        let mut title = Label::new("World Options")
            .font_size(48.0)
            .color([255, 255, 255, 255]);
        title.set_rect(Rect::new(660.0, 50.0, 600.0, 80.0));
        let title_id = self.base.add_widget(Box::new(title));

        // Options panel
        let mut panel = Panel::new()
            .background_color([32, 32, 32, 240])
            .border_color([128, 128, 128, 255])
            .border_width(2.0);
        panel.set_rect(Rect::new(460.0, 160.0, 1000.0, 600.0));
        let panel_id = self.base.add_widget(Box::new(panel));

        // Buttons panel
        let mut btn_panel = Panel::new()
            .background_color([32, 32, 32, 240])
            .border_color([128, 128, 128, 255])
            .border_width(2.0);
        btn_panel.set_rect(Rect::new(460.0, 780.0, 1000.0, 120.0));
        let btn_panel_id = self.base.add_widget(Box::new(btn_panel));

        // Game options
        let options = [
            "Game Mode",
            "Difficulty",
            "Allow Cheats",
            "Game Rules",
            "Open to LAN",
        ];

        let mut buttons = Vec::new();
        for (i, option) in options.iter().enumerate() {
            let y = 200.0 + i as f32 * 100.0;
            let mut btn = Button::new(option.to_string())
                .font_size(28.0)
                .background_color([64, 64, 64, 255])
                .text_color([255, 255, 255, 255]);
            btn.set_rect(Rect::new(560.0, y, 800.0, 70.0));
            let btn_id = self.base.add_widget(Box::new(btn));
            self.base.focus_manager.register_focusable(btn_id, i as i32);
            buttons.push(btn_id);
        }

        // Action buttons
        let mut action_buttons = Vec::new();
        let action_labels = ["Done", "Cancel"];
        for (i, label) in action_labels.iter().enumerate() {
            let x = 560.0 + i as f32 * 480.0;
            let mut btn = Button::new(label.to_string())
                .font_size(24.0)
                .background_color(if i == 0 {
                    [0, 100, 0, 255]
                } else {
                    [100, 0, 0, 255]
                })
                .text_color([255, 255, 255, 255]);
            btn.set_rect(Rect::new(x, 800.0, 440.0, 60.0));
            let btn_id = self.base.add_widget(Box::new(btn));
            self.base
                .focus_manager
                .register_focusable(btn_id, 10 + i as i32);
            action_buttons.push(btn_id);
        }

        // Container
        let mut container = Container::new();
        container.set_rect(Rect::new(0.0, 0.0, 1920.0, 1080.0));
        container.add_child(title_id);
        container.add_child(panel_id);
        container.add_child(btn_panel_id);
        for btn_id in &buttons {
            container.add_child(*btn_id);
        }
        for btn_id in &action_buttons {
            container.add_child(*btn_id);
        }

        let root_id = self.base.add_widget(Box::new(container));
        self.base.set_root_widget(root_id);

        if let Some(first) = buttons.first() {
            let _ = self.base.focus_manager.set_focus(Some(*first));
        }
    }
}

impl Screen for WorldOptionsScreen {
    fn id(&self) -> SceneId {
        self.base.scene_id
    }

    fn name(&self) -> &str {
        "World Options"
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn handle_input(
        &mut self,
        _ui: &mut UiContext,
        action: mclre_input::Action,
        pressed: bool,
    ) -> bool {
        if !pressed {
            return false;
        }
        match action {
            mclre_input::Action::NavigateUp
            | mclre_input::Action::NavigateDown
            | mclre_input::Action::NavigateLeft
            | mclre_input::Action::NavigateRight => {
                let direction = match action {
                    mclre_input::Action::NavigateUp => FocusDirection::Up,
                    mclre_input::Action::NavigateDown => FocusDirection::Down,
                    mclre_input::Action::NavigateLeft => FocusDirection::Left,
                    mclre_input::Action::NavigateRight => FocusDirection::Right,
                    _ => return false,
                };
                let _ = self.base.focus_manager.move_focus(direction);
                true
            }
            mclre_input::Action::NavigateSelect | mclre_input::Action::UseItem => {
                let _ = self.base.focus_manager.set_focus(None);
                true
            }
            _ => false,
        }
    }
}

/// Settings screen.
pub struct SettingsScreen {
    base: BaseScreen,
    categories: Vec<String>,
    selected_category: usize,
}

impl SettingsScreen {
    pub fn new() -> Self {
        let mut base = BaseScreen::new(ScreenType::Settings);
        base.set_fullscreen(true);

        let mut screen = Self {
            base,
            categories: vec![
                "Video".to_string(),
                "Audio".to_string(),
                "Controls".to_string(),
                "Language".to_string(),
                "Accessibility".to_string(),
                "Multiplayer".to_string(),
            ],
            selected_category: 0,
        };

        screen.build_layout();
        screen
    }

    fn build_layout(&mut self) {
        let mut container = Container::new();
        container.set_rect(Rect::new(0.0, 0.0, 1920.0, 1080.0));

        // Title
        let mut title = Label::new("Settings")
            .font_size(48.0)
            .color([255, 255, 255, 255]);
        title.set_rect(Rect::new(660.0, 50.0, 600.0, 80.0));
        let title_id = self.base.add_widget(Box::new(title));

        // Categories panel (left)
        let mut cat_panel = Panel::new()
            .background_color([32, 32, 32, 240])
            .border_color([128, 128, 128, 255])
            .border_width(2.0);
        cat_panel.set_rect(Rect::new(160.0, 160.0, 300.0, 760.0));
        let cat_panel_id = self.base.add_widget(Box::new(cat_panel));

        // Options panel (right)
        let mut opt_panel = Panel::new()
            .background_color([32, 32, 32, 240])
            .border_color([128, 128, 128, 255])
            .border_width(2.0);
        opt_panel.set_rect(Rect::new(500.0, 160.0, 1260.0, 760.0));
        let opt_panel_id = self.base.add_widget(Box::new(opt_panel));

        // Buttons panel
        let mut btn_panel = Panel::new()
            .background_color([32, 32, 32, 240])
            .border_color([128, 128, 128, 255])
            .border_width(2.0);
        btn_panel.set_rect(Rect::new(160.0, 940.0, 1600.0, 100.0));
        let btn_panel_id = self.base.add_widget(Box::new(btn_panel));

        // Category buttons
        let mut cat_buttons = Vec::new();
        for (i, cat) in self.categories.iter().enumerate() {
            let y = 200.0 + i as f32 * 110.0;
            let mut btn = Button::new(cat.clone())
                .font_size(24.0)
                .background_color(if i == self.selected_category {
                    [0, 100, 0, 255]
                } else {
                    [64, 64, 64, 255]
                })
                .text_color([255, 255, 255, 255]);
            btn.set_rect(Rect::new(180.0, y, 260.0, 80.0));
            let btn_id = self.base.add_widget(Box::new(btn));
            self.base.focus_manager.register_focusable(btn_id, i as i32);
            cat_buttons.push(btn_id);
        }

        // Options for selected category (placeholder)
        let mut opt_buttons = Vec::new();
        let options = match self.selected_category {
            0 => vec![
                "Render Distance",
                "Graphics",
                "Smooth Lighting",
                "FOV",
                "Vsync",
                "Fullscreen",
            ],
            1 => vec![
                "Music Volume",
                "Sound Volume",
                "Ambient Volume",
                "Music",
                "Sounds",
            ],
            2 => vec![
                "Key Bindings",
                "Controller",
                "Touch",
                "Mouse Sensitivity",
                "Invert Mouse",
            ],
            3 => vec!["Language", "Force Unicode Font"],
            4 => vec!["Text to Speech", "Narrator", "Auto-Jump", "Show Subtitles"],
            5 => vec![
                "Chat Visibility",
                "Chat Colors",
                "Prompt on Links",
                "Server List",
            ],
            _ => vec![],
        };

        for (i, option) in options.iter().enumerate() {
            let y = 200.0 + i as f32 * 90.0;
            let mut btn = Button::new(option.to_string())
                .font_size(22.0)
                .background_color([64, 64, 64, 255])
                .text_color([255, 255, 255, 255]);
            btn.set_rect(Rect::new(540.0, y, 1180.0, 70.0));
            let btn_id = self.base.add_widget(Box::new(btn));
            self.base
                .focus_manager
                .register_focusable(btn_id, 10 + i as i32);
            opt_buttons.push(btn_id);
        }

        // Action buttons
        let mut action_buttons = Vec::new();
        let action_labels = ["Done", "Cancel"];
        for (i, label) in action_labels.iter().enumerate() {
            let x = 160.0 + i as f32 * 280.0;
            let mut btn = Button::new(label.to_string())
                .font_size(24.0)
                .background_color(if i == 0 {
                    [0, 100, 0, 255]
                } else {
                    [100, 0, 0, 255]
                })
                .text_color([255, 255, 255, 255]);
            btn.set_rect(Rect::new(x, 950.0, 240.0, 60.0));
            let btn_id = self.base.add_widget(Box::new(btn));
            self.base
                .focus_manager
                .register_focusable(btn_id, 30 + i as i32);
            action_buttons.push(btn_id);
        }

        // Container
        let mut container = Container::new();
        container.set_rect(Rect::new(0.0, 0.0, 1920.0, 1080.0));
        container.add_child(title_id);
        container.add_child(cat_panel_id);
        container.add_child(opt_panel_id);
        container.add_child(btn_panel_id);
        for btn_id in &cat_buttons {
            container.add_child(*btn_id);
        }
        for btn_id in &opt_buttons {
            container.add_child(*btn_id);
        }
        for btn_id in &action_buttons {
            container.add_child(*btn_id);
        }

        let root_id = self.base.add_widget(Box::new(container));
        self.base.set_root_widget(root_id);

        if let Some(first) = cat_buttons.first() {
            let _ = self.base.focus_manager.set_focus(Some(*first));
        }
    }
}

impl Screen for SettingsScreen {
    fn id(&self) -> SceneId {
        self.base.scene_id
    }

    fn name(&self) -> &str {
        "Settings"
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn handle_input(
        &mut self,
        _ui: &mut UiContext,
        action: mclre_input::Action,
        pressed: bool,
    ) -> bool {
        if !pressed {
            return false;
        }
        match action {
            mclre_input::Action::NavigateUp
            | mclre_input::Action::NavigateDown
            | mclre_input::Action::NavigateLeft
            | mclre_input::Action::NavigateRight => {
                let direction = match action {
                    mclre_input::Action::NavigateUp => FocusDirection::Up,
                    mclre_input::Action::NavigateDown => FocusDirection::Down,
                    mclre_input::Action::NavigateLeft => FocusDirection::Left,
                    mclre_input::Action::NavigateRight => FocusDirection::Right,
                    _ => return false,
                };
                let _ = self.base.focus_manager.move_focus(direction);
                true
            }
            mclre_input::Action::NavigateSelect | mclre_input::Action::UseItem => {
                let _ = self.base.focus_manager.set_focus(None);
                true
            }
            _ => false,
        }
    }
}

/// Inventory screen.
pub struct InventoryScreen {
    base: BaseScreen,
    player_inventory: Vec<Option<HotbarSlot>>,
    hotbar: Vec<Option<HotbarSlot>>,
    selected_slot: usize,
    crafting_grid: Vec<Option<HotbarSlot>>, // 2x2
    crafting_result: Option<HotbarSlot>,
    armor_slots: Vec<Option<HotbarSlot>>, // 4 slots
    offhand_slot: Option<HotbarSlot>,
}

impl InventoryScreen {
    pub fn new() -> Self {
        let mut base = BaseScreen::new(ScreenType::Inventory);
        base.set_fullscreen(true);
        base.background_color = Color::new(0, 0, 0, 180);

        let mut screen = Self {
            base,
            player_inventory: vec![None; 27], // 3 rows x 9
            hotbar: vec![None; 9],
            selected_slot: 0,
            crafting_grid: vec![None; 4], // 2x2
            crafting_result: None,
            armor_slots: vec![None; 4], // helmet, chestplate, leggings, boots
            offhand_slot: None,
        };

        screen.build_layout();
        screen
    }

    fn build_layout(&mut self) {
        let mut container = Container::new();
        container.set_rect(Rect::new(0.0, 0.0, 1920.0, 1080.0));

        // Title
        let mut title = Label::new("Inventory")
            .font_size(36.0)
            .color([255, 255, 255, 255]);
        title.set_rect(Rect::new(660.0, 50.0, 600.0, 60.0));
        let title_id = self.base.add_widget(Box::new(title));

        // Main inventory panel (centered)
        let panel_x = 335.0;
        let panel_y = 180.0;
        let panel_w = 1250.0;
        let panel_h = 660.0;

        let mut panel = Panel::new()
            .background_color([32, 32, 32, 240])
            .border_color([128, 128, 128, 255])
            .border_width(2.0);
        panel.set_rect(Rect::new(panel_x, panel_y, panel_w, panel_h));
        let panel_id = self.base.add_widget(Box::new(panel));

        // Armor slots (left side)
        let armor_x = panel_x + 20.0;
        let armor_y = panel_y + 20.0;
        for i in 0..4 {
            let y = armor_y + i as f32 * 55.0;
            let mut slot = Panel::new()
                .background_color([48, 48, 48, 255])
                .border_color([128, 128, 128, 255])
                .border_width(1.0);
            slot.set_rect(Rect::new(armor_x, y, 44.0, 44.0));
            let slot_id = self.base.add_widget(Box::new(slot));
            self.base
                .focus_manager
                .register_focusable(slot_id, i as i32);
        }

        // Crafting grid (2x2, top-left of main area)
        let craft_x = armor_x + 80.0;
        let craft_y = armor_y;
        for row in 0..2 {
            for col in 0..2 {
                let x = craft_x + col as f32 * 50.0;
                let y = craft_y + row as f32 * 50.0;
                let mut slot = Panel::new()
                    .background_color([48, 48, 48, 255])
                    .border_color([128, 128, 128, 255])
                    .border_width(1.0);
                slot.set_rect(Rect::new(x, y, 44.0, 44.0));
                let slot_id = self.base.add_widget(Box::new(slot));
                self.base
                    .focus_manager
                    .register_focusable(slot_id, 10 + row * 2 + col);
            }
        }

        // Crafting result (right of crafting grid)
        let result_x = craft_x + 120.0;
        let mut result_slot = Panel::new()
            .background_color([48, 48, 48, 255])
            .border_color([0, 128, 0, 255])
            .border_width(2.0);
        result_slot.set_rect(Rect::new(result_x, craft_y, 44.0, 44.0));
        let result_id = self.base.add_widget(Box::new(result_slot));
        self.base.focus_manager.register_focusable(result_id, 14);

        // Offhand slot (below armor)
        let offhand_x = armor_x;
        let offhand_y = armor_y + 4.0 * 55.0 + 20.0;
        let mut offhand_slot = Panel::new()
            .background_color([48, 48, 48, 255])
            .border_color([128, 0, 128, 255])
            .border_width(1.0);
        offhand_slot.set_rect(Rect::new(offhand_x, offhand_y, 44.0, 44.0));
        let offhand_id = self.base.add_widget(Box::new(offhand_slot));
        self.base.focus_manager.register_focusable(offhand_id, 15);

        // Player inventory (3 rows of 9)
        let inv_x = craft_x;
        let inv_y = craft_y + 120.0;
        for row in 0..3 {
            for col in 0..9 {
                let x = inv_x + col as f32 * 55.0;
                let y = inv_y + row as f32 * 55.0;
                let mut slot = Panel::new()
                    .background_color([48, 48, 48, 255])
                    .border_color([128, 128, 128, 255])
                    .border_width(1.0);
                slot.set_rect(Rect::new(x, y, 48.0, 48.0));
                let slot_id = self.base.add_widget(Box::new(slot));
                self.base
                    .focus_manager
                    .register_focusable(slot_id, 20 + row * 9 + col);
            }
        }

        // Hotbar (bottom row)
        let hotbar_x = inv_x;
        let hotbar_y = inv_y + 3.0 * 55.0 + 20.0;
        for col in 0..9 {
            let x = hotbar_x + col as f32 * 55.0;
            let border_color = if col == self.selected_slot {
                [255, 215, 0, 255]
            } else {
                [128, 128, 128, 255]
            };
            let border_width = if col == self.selected_slot { 3.0 } else { 1.0 };
            let mut slot = Panel::new()
                .background_color([48, 48, 48, 255])
                .border_color(border_color)
                .border_width(border_width);
            slot.set_rect(Rect::new(x, hotbar_y, 48.0, 48.0));
            let slot_id = self.base.add_widget(Box::new(slot));
            self.base
                .focus_manager
                .register_focusable(slot_id, 50 + col as i32);
        }

        // Container
        let mut container = Container::new();
        container.set_rect(Rect::new(0.0, 0.0, 1920.0, 1080.0));
        container.add_child(title_id);
        container.add_child(panel_id);

        let root_id = self.base.add_widget(Box::new(container));
        self.base.set_root_widget(root_id);

        // Focus first inventory slot
        let _ = self
            .base
            .focus_manager
            .set_focus(self.base.focus_manager.focused());
    }
}

impl Screen for InventoryScreen {
    fn id(&self) -> SceneId {
        self.base.scene_id
    }

    fn name(&self) -> &str {
        "Inventory"
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn handle_input(
        &mut self,
        _ui: &mut UiContext,
        action: mclre_input::Action,
        pressed: bool,
    ) -> bool {
        if !pressed {
            return false;
        }
        match action {
            mclre_input::Action::NavigateUp
            | mclre_input::Action::NavigateDown
            | mclre_input::Action::NavigateLeft
            | mclre_input::Action::NavigateRight => {
                let direction = match action {
                    mclre_input::Action::NavigateUp => FocusDirection::Up,
                    mclre_input::Action::NavigateDown => FocusDirection::Down,
                    mclre_input::Action::NavigateLeft => FocusDirection::Left,
                    mclre_input::Action::NavigateRight => FocusDirection::Right,
                    _ => return false,
                };
                let _ = self.base.focus_manager.move_focus(direction);
                true
            }
            mclre_input::Action::UseItem => {
                if let Some(focused) = self.base.focus_manager.focused() {
                    if let Some(widget) = self.base.get_widget(focused) {
                        if let Some(on_click) = widget.on_click_fn() {
                            on_click();
                            return true;
                        }
                    }
                }
                false
            }
            mclre_input::Action::Pause | Action::Inventory => {
                // Close inventory
                true
            }
            Action::Hotbar1
            | Action::Hotbar2
            | Action::Hotbar3
            | Action::Hotbar4
            | Action::Hotbar5
            | Action::Hotbar6
            | Action::Hotbar7
            | Action::Hotbar8
            | Action::Hotbar9 => {
                // Quick select hotbar slot
                let slot = match action {
                    Action::Hotbar1 => 0,
                    Action::Hotbar2 => 1,
                    Action::Hotbar3 => 2,
                    Action::Hotbar4 => 3,
                    Action::Hotbar5 => 4,
                    Action::Hotbar6 => 5,
                    Action::Hotbar7 => 6,
                    Action::Hotbar8 => 7,
                    Action::Hotbar9 => 8,
                    _ => 0,
                };
                self.selected_slot = slot;
                // Would need to rebuild or update hotbar highlights
                true
            }
            _ => false,
        }
    }
}

/// Crafting screen (crafting table).
pub struct CraftingScreen {
    base: BaseScreen,
    crafting_grid: Vec<Option<HotbarSlot>>, // 3x3
    crafting_result: Option<HotbarSlot>,
    player_inventory: Vec<Option<HotbarSlot>>,
    hotbar: Vec<Option<HotbarSlot>>,
}

impl CraftingScreen {
    pub fn new() -> Self {
        let mut base = BaseScreen::new(ScreenType::Crafting);
        base.set_fullscreen(true);
        base.background_color = Color::new(0, 0, 0, 180);

        let mut screen = Self {
            base,
            crafting_grid: vec![None; 9], // 3x3
            crafting_result: None,
            player_inventory: vec![None; 27],
            hotbar: vec![None; 9],
        };

        screen.build_layout();
        screen
    }

    fn build_layout(&mut self) {
        let mut container = Container::new();
        container.set_rect(Rect::new(0.0, 0.0, 1920.0, 1080.0));

        // Title
        let mut title = Label::new("Crafting")
            .font_size(36.0)
            .color([255, 255, 255, 255]);
        title.set_rect(Rect::new(660.0, 50.0, 600.0, 60.0));
        let title_id = self.base.add_widget(Box::new(title));

        // Crafting panel
        let panel_x = 500.0;
        let panel_y = 200.0;
        let panel_w = 920.0;
        let panel_h = 580.0;

        let mut panel = Panel::new()
            .background_color([32, 32, 32, 240])
            .border_color([128, 128, 128, 255])
            .border_width(2.0);
        panel.set_rect(Rect::new(panel_x, panel_y, panel_w, panel_h));
        let panel_id = self.base.add_widget(Box::new(panel));

        // 3x3 Crafting grid
        let grid_x = panel_x + 50.0;
        let grid_y = panel_y + 50.0;
        for row in 0..3 {
            for col in 0..3 {
                let x = grid_x + col as f32 * 60.0;
                let y = grid_y + row as f32 * 60.0;
                let mut slot = Panel::new()
                    .background_color([48, 48, 48, 255])
                    .border_color([128, 128, 128, 255])
                    .border_width(1.0);
                slot.set_rect(Rect::new(x, y, 52.0, 52.0));
                let slot_id = self.base.add_widget(Box::new(slot));
                self.base
                    .focus_manager
                    .register_focusable(slot_id, row * 3 + col);
            }
        }

        // Crafting arrow (visual only)
        let arrow_x = grid_x + 3.0 * 60.0 + 30.0;
        let arrow_y = grid_y + 60.0;
        let mut arrow = Label::new("->").font_size(36.0).color([200, 200, 200, 255]);
        arrow.set_rect(Rect::new(arrow_x, arrow_y, 60.0, 60.0));
        let arrow_id = self.base.add_widget(Box::new(arrow));

        // Crafting result
        let result_x = arrow_x + 80.0;
        let mut result_slot = Panel::new()
            .background_color([48, 48, 48, 255])
            .border_color([0, 128, 0, 255])
            .border_width(2.0);
        result_slot.set_rect(Rect::new(result_x, arrow_y, 52.0, 52.0));
        let result_id = self.base.add_widget(Box::new(result_slot));
        self.base.focus_manager.register_focusable(result_id, 9);

        // Recipe book button (placeholder)
        let mut recipe_btn = Button::new("Recipe Book")
            .font_size(20.0)
            .background_color([64, 64, 64, 255])
            .text_color([255, 255, 255, 255]);
        recipe_btn.set_rect(Rect::new(grid_x, grid_y + 3.0 * 60.0 + 30.0, 200.0, 50.0));
        let recipe_id = self.base.add_widget(Box::new(recipe_btn));
        self.base.focus_manager.register_focusable(recipe_id, 10);

        // Player inventory (3 rows of 9)
        let inv_x = grid_x;
        let inv_y = grid_y + 3.0 * 60.0 + 100.0;
        for row in 0..3 {
            for col in 0..9 {
                let x = inv_x + col as f32 * 55.0;
                let y = inv_y + row as f32 * 55.0;
                let mut slot = Panel::new()
                    .background_color([48, 48, 48, 255])
                    .border_color([128, 128, 128, 255])
                    .border_width(1.0);
                slot.set_rect(Rect::new(x, y, 48.0, 48.0));
                let slot_id = self.base.add_widget(Box::new(slot));
                self.base
                    .focus_manager
                    .register_focusable(slot_id, 20 + row * 9 + col);
            }
        }

        // Hotbar
        let hotbar_x = inv_x;
        let hotbar_y = inv_y + 3.0 * 55.0 + 20.0;
        for col in 0..9 {
            let x = hotbar_x + col as f32 * 55.0;
            let mut slot = Panel::new()
                .background_color([48, 48, 48, 255])
                .border_color([128, 128, 128, 255])
                .border_width(1.0);
            slot.set_rect(Rect::new(x, hotbar_y, 48.0, 48.0));
            let slot_id = self.base.add_widget(Box::new(slot));
            self.base
                .focus_manager
                .register_focusable(slot_id, 50 + col as i32);
        }

        // Container
        let mut container = Container::new();
        container.set_rect(Rect::new(0.0, 0.0, 1920.0, 1080.0));
        container.add_child(title_id);
        container.add_child(panel_id);
        container.add_child(arrow_id);

        let root_id = self.base.add_widget(Box::new(container));
        self.base.set_root_widget(root_id);

        let _ = self
            .base
            .focus_manager
            .set_focus(self.base.focus_manager.focused());
    }
}

impl Screen for CraftingScreen {
    fn id(&self) -> SceneId {
        self.base.scene_id
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn name(&self) -> &str {
        "Crafting"
    }

    fn handle_input(
        &mut self,
        _ui: &mut UiContext,
        action: mclre_input::Action,
        pressed: bool,
    ) -> bool {
        if !pressed {
            return false;
        }
        match action {
            mclre_input::Action::NavigateUp
            | mclre_input::Action::NavigateDown
            | mclre_input::Action::NavigateLeft
            | mclre_input::Action::NavigateRight => {
                let direction = match action {
                    mclre_input::Action::NavigateUp => FocusDirection::Up,
                    mclre_input::Action::NavigateDown => FocusDirection::Down,
                    mclre_input::Action::NavigateLeft => FocusDirection::Left,
                    mclre_input::Action::NavigateRight => FocusDirection::Right,
                    _ => return false,
                };
                let _ = self.base.focus_manager.move_focus(direction);
                true
            }
            mclre_input::Action::NavigateSelect | mclre_input::Action::UseItem => {
                // Close crafting
                true
            }
            _ => false,
        }
    }
}

/// Help & Options screen.
pub struct HelpOptionsScreen {
    base: BaseScreen,
    options: Vec<String>,
}

impl HelpOptionsScreen {
    pub fn new() -> Self {
        let mut base = BaseScreen::new(ScreenType::HelpOptions);
        base.set_fullscreen(true);

        let mut screen = Self {
            base,
            options: vec![
                "How to Play".to_string(),
                "Controls".to_string(),
                "Video Settings".to_string(),
                "Audio Settings".to_string(),
                "Accessibility".to_string(),
                "Feedback".to_string(),
                "Back".to_string(),
            ],
        };

        screen.build_layout();
        screen
    }

    fn build_layout(&mut self) {
        let mut container = Container::new();
        container.set_rect(Rect::new(0.0, 0.0, 1920.0, 1080.0));

        // Title
        let mut title = Label::new("Help & Options")
            .font_size(48.0)
            .color([255, 255, 255, 255]);
        title.set_rect(Rect::new(660.0, 50.0, 600.0, 80.0));
        let title_id = self.base.add_widget(Box::new(title));

        // Options panel
        let mut panel = Panel::new()
            .background_color([32, 32, 32, 240])
            .border_color([128, 128, 128, 255])
            .border_width(2.0);
        panel.set_rect(Rect::new(460.0, 160.0, 1000.0, 720.0));
        let panel_id = self.base.add_widget(Box::new(panel));

        // Buttons panel
        let mut btn_panel = Panel::new()
            .background_color([32, 32, 32, 240])
            .border_color([128, 128, 128, 255])
            .border_width(2.0);
        btn_panel.set_rect(Rect::new(460.0, 900.0, 1000.0, 120.0));
        let btn_panel_id = self.base.add_widget(Box::new(btn_panel));

        // Options buttons
        let mut buttons = Vec::new();
        for (i, option) in self.options.iter().enumerate() {
            let y = 200.0 + i as f32 * 90.0;
            let mut btn = Button::new(option.clone())
                .font_size(28.0)
                .background_color([64, 64, 64, 255])
                .text_color([255, 255, 255, 255]);
            btn.set_rect(Rect::new(560.0, y, 800.0, 70.0));
            let btn_id = self.base.add_widget(Box::new(btn));
            self.base.focus_manager.register_focusable(btn_id, i as i32);
            buttons.push(btn_id);
        }

        // Container
        let mut container = Container::new();
        container.set_rect(Rect::new(0.0, 0.0, 1920.0, 1080.0));
        container.add_child(title_id);
        container.add_child(panel_id);
        container.add_child(btn_panel_id);
        for btn_id in &buttons {
            container.add_child(*btn_id);
        }

        let root_id = self.base.add_widget(Box::new(container));
        self.base.set_root_widget(root_id);

        if let Some(first) = buttons.first() {
            let _ = self.base.focus_manager.set_focus(Some(*first));
        }
    }
}

impl Screen for HelpOptionsScreen {
    fn id(&self) -> SceneId {
        self.base.scene_id
    }

    fn name(&self) -> &str {
        "Help & Options"
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn handle_input(
        &mut self,
        _ui: &mut UiContext,
        action: mclre_input::Action,
        pressed: bool,
    ) -> bool {
        if !pressed {
            return false;
        }
        match action {
            mclre_input::Action::NavigateUp
            | mclre_input::Action::NavigateDown
            | mclre_input::Action::NavigateLeft
            | mclre_input::Action::NavigateRight => {
                let direction = match action {
                    mclre_input::Action::NavigateUp => FocusDirection::Up,
                    mclre_input::Action::NavigateDown => FocusDirection::Down,
                    mclre_input::Action::NavigateLeft => FocusDirection::Left,
                    mclre_input::Action::NavigateRight => FocusDirection::Right,
                    _ => return false,
                };
                let _ = self.base.focus_manager.move_focus(direction);
                true
            }
            mclre_input::Action::NavigateSelect | mclre_input::Action::UseItem => {
                let _ = self.base.focus_manager.set_focus(None);
                true
            }
            _ => false,
        }
    }
}

/// Credits screen.
pub struct CreditsScreen {
    base: BaseScreen,
    scroll_offset: f32,
}

impl CreditsScreen {
    pub fn new() -> Self {
        let mut base = BaseScreen::new(ScreenType::Credits);
        base.set_fullscreen(true);
        base.background_color = Color::new(0, 0, 0, 255);

        let mut screen = Self {
            base,
            scroll_offset: 0.0,
        };

        screen.build_layout();
        screen
    }

    fn build_layout(&mut self) {
        let mut container = Container::new();
        container.set_rect(Rect::new(0.0, 0.0, 1920.0, 1080.0));

        // Title
        let mut title = Label::new("Credits")
            .font_size(48.0)
            .color([255, 215, 0, 255]); // Gold
        title.set_rect(Rect::new(660.0, 50.0, 600.0, 80.0));
        let title_id = self.base.add_widget(Box::new(title));

        // Credits text (placeholder - would scroll)
        let credits = [
            "MINECRAFT: LEGACY CONSOLE EDITION",
            "",
            "Original Game by Mojang Studios",
            "Rust Reimplementation by MCLRE Team",
            "",
            "PROGRAMMING",
            "Lead Developer: [Team]",
            "Engine: [Team]",
            "World Generation: [Team]",
            "Networking: [Team]",
            "UI System: [Team]",
            "Rendering: [Team]",
            "",
            "ART & DESIGN",
            "Textures: [Team]",
            "Models: [Team]",
            "UI Design: [Team]",
            "",
            "AUDIO",
            "Music: C418",
            "Sound Effects: [Team]",
            "",
            "SPECIAL THANKS",
            "Mojang Studios for Minecraft",
            "The Rust Community",
            "Open Source Contributors",
            "",
            "Reference: Legacy Console Edition TU19/CU7",
            "Build Date: Dec 17, 2014",
        ];

        let mut text_widgets = Vec::new();
        for (i, line) in credits.iter().enumerate() {
            let y = 200.0 + i as f32 * 40.0;
            let color = if line.starts_with("MINECRAFT") || line.is_empty() {
                [255, 215, 0, 255]
            } else if line.ends_with(':') {
                [200, 200, 200, 255]
            } else {
                [255, 255, 255, 255]
            };
            let mut label = Label::new(line.to_string()).font_size(24.0).color(color);
            label.set_rect(Rect::new(460.0, y, 1000.0, 35.0));
            let label_id = self.base.add_widget(Box::new(label));
            text_widgets.push(label_id);
        }

        // Container
        let mut container = Container::new();
        container.set_rect(Rect::new(0.0, 0.0, 1920.0, 1080.0));
        container.add_child(title_id);
        for tw_id in &text_widgets {
            container.add_child(*tw_id);
        }

        let root_id = self.base.add_widget(Box::new(container));
        self.base.set_root_widget(root_id);
    }
}

impl Screen for CreditsScreen {
    fn id(&self) -> SceneId {
        self.base.scene_id
    }

    fn name(&self) -> &str {
        "Credits"
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn update(&mut self, _ui: &mut UiContext, dt: f32) {
        // Auto-scroll credits
        self.scroll_offset += dt * 50.0;
        // Would update widget positions based on scroll
    }

    fn handle_input(
        &mut self,
        _ui: &mut UiContext,
        action: mclre_input::Action,
        pressed: bool,
    ) -> bool {
        if !pressed {
            return false;
        }
        match action {
            mclre_input::Action::Pause => {
                // Exit credits
                true
            }
            _ => false,
        }
    }
}

/// Multiplayer screen for LAN game discovery and connection.
pub struct MultiplayerScreen {
    base: BaseScreen,
    discovered_servers: Vec<DiscoveredServer>,
    refresh_timer: f32,
    is_refreshing: bool,
    /// Callback when user wants to connect to a server
    on_connect: Option<Arc<dyn Fn(String, u16) + Send + Sync>>,
    /// Callback to trigger mDNS discovery
    on_refresh: Option<Arc<dyn Fn() + Send + Sync>>,
    /// Callback for direct connect with UPnP
    on_direct_connect: Option<Arc<dyn Fn(String, u16, bool) + Send + Sync>>,
    /// Whether direct connect dialog is open
    show_direct_connect: bool,
    /// Direct connect dialog input fields
    direct_connect_ip: String,
    direct_connect_port: String,
    direct_connect_upnp: bool,
    /// Focus index for direct connect dialog (0=IP, 1=Port, 2=UPnP, 3=Connect, 4=Cancel)
    direct_connect_focus: usize,
}

#[derive(Clone, Debug)]
pub struct DiscoveredServer {
    pub name: String,
    pub address: String,
    pub port: u16,
    pub motd: String,
    pub players: u32,
    pub max_players: u32,
    pub version: String,
}

impl MultiplayerScreen {
    pub fn new() -> Self {
        let mut base = BaseScreen::new(ScreenType::Multiplayer);
        base.set_fullscreen(true);
        base.background_color = Color::new(0, 0, 0, 255);

        Self {
            base,
            discovered_servers: Vec::new(),
            refresh_timer: 0.0,
            is_refreshing: false,
            on_connect: None,
            on_refresh: None,
            on_direct_connect: None,
            show_direct_connect: false,
            direct_connect_ip: String::new(),
            direct_connect_port: "25565".to_string(),
            direct_connect_upnp: false,
            direct_connect_focus: 0, // 0=IP, 1=Port, 2=UPnP, 3=Connect, 4=Cancel
        }
    }

    /// Toggle the direct connect dialog
    pub fn toggle_direct_connect(&mut self) {
        self.show_direct_connect = !self.show_direct_connect;
        if self.show_direct_connect {
            self.direct_connect_focus = 0;
            // Rebuild layout to show dialog
            self.rebuild_layout();
        }
    }

    /// Set callback for when user wants to connect to a server
    pub fn set_on_connect<F>(&mut self, callback: F)
    where
        F: Fn(String, u16) + Send + Sync + 'static,
    {
        self.on_connect = Some(Arc::new(callback));
    }

    /// Set callback to trigger mDNS discovery
    pub fn set_on_refresh<F>(&mut self, callback: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.on_refresh = Some(Arc::new(callback));
    }

    /// Set callback for direct connect with UPnP
    pub fn set_on_direct_connect<F>(&mut self, callback: F)
    where
        F: Fn(String, u16, bool) + Send + Sync + 'static,
    {
        self.on_direct_connect = Some(Arc::new(callback));
    }

    /// Update the discovered server list
    pub fn set_discovered_servers(&mut self, servers: Vec<DiscoveredServer>) {
        self.discovered_servers = servers;
        self.rebuild_layout();
    }

    fn rebuild_layout(&mut self) {
        // Clear existing widgets by rebuilding
        self.base = BaseScreen::new(ScreenType::Multiplayer);
        self.base.set_fullscreen(true);
        self.base.background_color = Color::new(0, 0, 0, 255);
        self.build_layout();
    }

    fn build_layout(&mut self) {
        // Root container
        let mut root_container = Container::new();
        root_container.set_rect(Rect::new(0.0, 0.0, 1920.0, 1080.0));

        // Title
        let mut title = Label::new("Multiplayer")
            .font_size(48.0)
            .color([255, 215, 0, 255]); // Gold
        title.set_rect(Rect::new(660.0, 50.0, 600.0, 80.0));
        let title_id = self.base.add_widget(Box::new(title));
        root_container.add_child(title_id);

        // Server list container
        let mut list_container = Container::new();
        list_container.set_rect(Rect::new(310.0, 180.0, 1300.0, 700.0));

        // If no servers found, show message
        if self.discovered_servers.is_empty() {
            let mut no_servers = Label::new("Scanning for LAN games...")
                .font_size(24.0)
                .color([200, 200, 200, 255]);
            no_servers.set_rect(Rect::new(350.0, 400.0, 800.0, 50.0));
            let no_servers_id = self.base.add_widget(Box::new(no_servers));
            list_container.add_child(no_servers_id);
        } else {
            for (i, server) in self.discovered_servers.iter().enumerate() {
                let y = 20.0 + i as f32 * 80.0;

                // Server entry background
                let mut entry_bg = Panel::new().background_color([40, 40, 40, 200]);
                entry_bg.set_rect(Rect::new(10.0, y, 1280.0, 70.0));
                let entry_bg_id = self.base.add_widget(Box::new(entry_bg));

                // Server name
                let mut name_label = Label::new(&server.name)
                    .font_size(24.0)
                    .color([255, 255, 255, 255]);
                name_label.set_rect(Rect::new(30.0, y + 10.0, 600.0, 30.0));
                let name_id = self.base.add_widget(Box::new(name_label));

                // Server info (MOTD, players, version)
                let info_text = format!(
                    "{} | {} players | v{}",
                    server.motd, server.players, server.version
                );
                let mut info_label = Label::new(&info_text)
                    .font_size(16.0)
                    .color([180, 180, 180, 255]);
                info_label.set_rect(Rect::new(30.0, y + 40.0, 600.0, 25.0));
                let info_id = self.base.add_widget(Box::new(info_label));

                // Connect button
                let server_addr = server.address.clone();
                let server_port = server.port;
                let on_connect = self.on_connect.clone();
                let mut connect_btn = Button::new("Connect").on_click(move || {
                    if let Some(ref cb) = on_connect {
                        cb(server_addr.clone(), server_port);
                    }
                });
                connect_btn.set_rect(Rect::new(1000.0, y + 15.0, 150.0, 40.0));
                let connect_id = self.base.add_widget(Box::new(connect_btn));

                list_container.add_child(entry_bg_id);
                list_container.add_child(name_id);
                list_container.add_child(info_id);
                list_container.add_child(connect_id);
            }
        }

        let list_container_id = list_container.id();
        root_container.add_child(list_container_id);

        // Refresh button
        let on_refresh = self.on_refresh.clone();
        let mut refresh_btn = Button::new("Refresh").on_click(move || {
            info!("Refreshing LAN server list...");
            if let Some(ref cb) = on_refresh {
                cb();
            }
        });
        refresh_btn.set_rect(Rect::new(310.0, 900.0, 150.0, 40.0));
        let refresh_id = self.base.add_widget(Box::new(refresh_btn));
        self.base.focus_manager.register_focusable(refresh_id, 10);
        root_container.add_child(refresh_id);

        // Direct connect button
        let mut direct_connect_btn = Button::new("Direct Connect").on_click(|| {
            info!("Direct Connect button clicked (handled via focus)");
        });
        direct_connect_btn.set_rect(Rect::new(480.0, 900.0, 200.0, 40.0));
        let direct_connect_id = self.base.add_widget(Box::new(direct_connect_btn));
        self.base
            .focus_manager
            .register_focusable(direct_connect_id, 11);
        root_container.add_child(direct_connect_id);

        // Back button
        let mut back_btn = Button::new("Back").on_click(|| {
            info!("Back to main menu");
        });
        back_btn.set_rect(Rect::new(1460.0, 900.0, 150.0, 40.0));
        let back_id = self.base.add_widget(Box::new(back_btn));
        self.base.focus_manager.register_focusable(back_id, 12);
        root_container.add_child(back_id);

        // Direct connect dialog (overlay)
        if self.show_direct_connect {
            let dialog_id = self.build_direct_connect_dialog(&mut root_container);
            root_container.add_child(dialog_id);
        }

        let root_id = self.base.add_widget(Box::new(root_container));
        self.base.set_root_widget(root_id);
    }

    fn build_direct_connect_dialog(&mut self, root_container: &mut Container) -> WidgetId {
        // Dialog background (semi-transparent overlay)
        let mut overlay = Panel::new().background_color([0, 0, 0, 180]);
        overlay.set_rect(Rect::new(0.0, 0.0, 1920.0, 1080.0));
        let overlay_id = self.base.add_widget(Box::new(overlay));
        root_container.add_child(overlay_id);

        // Dialog panel
        let mut dialog = Panel::new()
            .background_color([40, 40, 40, 255])
            .border_color([128, 128, 128, 255])
            .border_width(2.0);
        dialog.set_rect(Rect::new(560.0, 290.0, 800.0, 500.0));
        let dialog_id = self.base.add_widget(Box::new(dialog));
        root_container.add_child(dialog_id);

        // Title
        let mut title = Label::new("Direct Connect")
            .font_size(36.0)
            .color([255, 215, 0, 255]);
        title.set_rect(Rect::new(760.0, 320.0, 400.0, 60.0));
        let title_id = self.base.add_widget(Box::new(title));
        root_container.add_child(title_id);

        // IP Address label
        let mut ip_label = Label::new("Server Address (IP:Port or Hostname)")
            .font_size(20.0)
            .color([200, 200, 200, 255]);
        ip_label.set_rect(Rect::new(600.0, 400.0, 500.0, 30.0));
        let ip_label_id = self.base.add_widget(Box::new(ip_label));
        root_container.add_child(ip_label_id);

        // IP input field (Panel with Label inside for background)
        let mut ip_input_panel = Panel::new().background_color([64, 64, 64, 255]);
        ip_input_panel.set_rect(Rect::new(600.0, 440.0, 500.0, 40.0));
        let ip_input_panel_id = self.base.add_widget(Box::new(ip_input_panel));
        root_container.add_child(ip_input_panel_id);

        let mut ip_input = Label::new(&self.direct_connect_ip)
            .font_size(24.0)
            .color([255, 255, 255, 255]);
        ip_input.set_rect(Rect::new(610.0, 445.0, 480.0, 30.0));
        let ip_input_id = self.base.add_widget(Box::new(ip_input));
        root_container.add_child(ip_input_id);

        // Port label
        let mut port_label = Label::new("Port")
            .font_size(20.0)
            .color([200, 200, 200, 255]);
        port_label.set_rect(Rect::new(600.0, 500.0, 200.0, 30.0));
        let port_label_id = self.base.add_widget(Box::new(port_label));
        root_container.add_child(port_label_id);

        // Port input
        let mut port_input_panel = Panel::new().background_color([64, 64, 64, 255]);
        port_input_panel.set_rect(Rect::new(700.0, 500.0, 150.0, 40.0));
        let port_input_panel_id = self.base.add_widget(Box::new(port_input_panel));
        root_container.add_child(port_input_panel_id);

        let mut port_input = Label::new(&self.direct_connect_port)
            .font_size(24.0)
            .color([255, 255, 255, 255]);
        port_input.set_rect(Rect::new(710.0, 505.0, 130.0, 30.0));
        let port_input_id = self.base.add_widget(Box::new(port_input));
        root_container.add_child(port_input_id);

        // UPnP checkbox label
        let upnp_text = if self.direct_connect_upnp {
            "[X] Use UPnP for port forwarding"
        } else {
            "[ ] Use UPnP for port forwarding"
        };
        let mut upnp_checkbox = Label::new(upnp_text)
            .font_size(20.0)
            .color([200, 200, 200, 255]);
        upnp_checkbox.set_rect(Rect::new(600.0, 560.0, 500.0, 30.0));
        let upnp_id = self.base.add_widget(Box::new(upnp_checkbox));
        root_container.add_child(upnp_id);

        // Connect button
        let on_connect = self.on_direct_connect.clone();
        let ip = self.direct_connect_ip.clone();
        let port = self.direct_connect_port.clone();
        let use_upnp = self.direct_connect_upnp;
        let mut connect_btn = Button::new("Connect").on_click(move || {
            if let Some(ref cb) = on_connect {
                if let Ok(port) = port.parse::<u16>() {
                    cb(ip.clone(), port, use_upnp);
                }
            }
        });
        connect_btn.set_rect(Rect::new(600.0, 620.0, 150.0, 50.0));
        let connect_id = self.base.add_widget(Box::new(connect_btn));
        root_container.add_child(connect_id);

        // Cancel button
        let mut cancel_btn = Button::new("Cancel").on_click(|| {
            info!("Cancel button clicked (handled via focus/Escape)");
        });
        cancel_btn.set_rect(Rect::new(950.0, 620.0, 150.0, 50.0));
        let cancel_id = self.base.add_widget(Box::new(cancel_btn));
        root_container.add_child(cancel_id);

        // Return the overlay ID for cleanup
        overlay_id
    }
}

impl Screen for MultiplayerScreen {
    fn id(&self) -> SceneId {
        self.base.scene_id
    }

    fn name(&self) -> &str {
        "Multiplayer"
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn update(&mut self, _ui: &mut UiContext, dt: f32) {
        self.refresh_timer += dt;

        // Auto-refresh every 5 seconds
        if self.refresh_timer > 5.0 && !self.is_refreshing {
            self.refresh_timer = 0.0;
            self.is_refreshing = true;

            // TODO: Trigger mDNS discovery
            // For now, just simulate finding servers
            // In real implementation, this would call LanDiscovery::discover()

            self.is_refreshing = false;
        }
    }

    fn handle_input(
        &mut self,
        _ui: &mut UiContext,
        action: mclre_input::Action,
        pressed: bool,
    ) -> bool {
        if !pressed {
            return false;
        }

        // Handle direct connect dialog input first
        if self.show_direct_connect {
            match action {
                mclre_input::Action::Pause => {
                    // Escape closes dialog
                    self.show_direct_connect = false;
                    self.rebuild_layout();
                    return true;
                }
                mclre_input::Action::NavigateSelect | mclre_input::Action::UseItem => {
                    // Enter activates focused button
                    match self.direct_connect_focus {
                        3 => {
                            // Connect button
                            if let Some(ref cb) = self.on_direct_connect {
                                if let Ok(port) = self.direct_connect_port.parse::<u16>() {
                                    cb(
                                        self.direct_connect_ip.clone(),
                                        port,
                                        self.direct_connect_upnp,
                                    );
                                }
                            }
                        }
                        4 => {
                            // Cancel button
                            self.show_direct_connect = false;
                            self.rebuild_layout();
                        }
                        _ => {}
                    }
                    return true;
                }
                mclre_input::Action::NavigateNext => {
                    // Tab - next focus
                    self.direct_connect_focus = (self.direct_connect_focus + 1) % 5;
                    return true;
                }
                mclre_input::Action::NavigatePrevious => {
                    // Shift+Tab - previous focus
                    self.direct_connect_focus = (self.direct_connect_focus + 4) % 5;
                    return true;
                }
                mclre_input::Action::NavigateUp | mclre_input::Action::NavigateDown => {
                    // Up/Down also navigate focus
                    if self.direct_connect_focus < 3 {
                        self.direct_connect_focus = (self.direct_connect_focus + 2) % 3; // Cycle IP, Port, UPnP
                    } else {
                        self.direct_connect_focus = (self.direct_connect_focus + 1) % 2 + 3; // Cycle Connect, Cancel
                    }
                    return true;
                }
                _ => return false,
            }
        }

        // Normal multiplayer screen input
        match action {
            mclre_input::Action::Pause => {
                // Back to main menu
                true
            }
            mclre_input::Action::NavigateSelect => {
                // Enter key pressed - handle generically
                false
            }
            mclre_input::Action::UseItem => {
                // UseItem key pressed - handle generically
                false
            }
            _ => false,
        }
    }
}

/// Screen manager for managing game screens.
pub struct ScreenManager {
    screens: HashMap<ScreenType, Box<dyn Screen>>,
    current_screen: Option<ScreenType>,
    screen_stack: Vec<ScreenType>,
    focus_manager: FocusManager,
}

impl ScreenManager {
    pub fn new() -> Self {
        Self {
            screens: HashMap::new(),
            current_screen: None,
            screen_stack: Vec::new(),
            focus_manager: FocusManager::new(),
        }
    }

    pub fn register_screen(&mut self, screen_type: ScreenType, screen: Box<dyn Screen>) {
        self.screens.insert(screen_type, screen);
    }

    pub fn push_screen(&mut self, screen_type: ScreenType, context: &mut UiContext) -> Result<()> {
        if let Some(current) = self.current_screen {
            self.screen_stack.push(current);
        }
        if let Some(screen) = self.screens.get_mut(&screen_type) {
            screen.on_enter(context);
            self.current_screen = Some(screen_type);
            Ok(())
        } else {
            Err(UiError::SceneNotFound(screen_type.as_str().to_string()))
        }
    }

    pub fn pop_screen(&mut self, context: &mut UiContext) -> Result<()> {
        if let Some(current) = self.current_screen {
            if let Some(screen) = self.screens.get_mut(&current) {
                screen.on_exit(context);
            }
        }
        if let Some(previous) = self.screen_stack.pop() {
            if let Some(screen) = self.screens.get_mut(&previous) {
                screen.on_enter(context);
            }
            self.current_screen = Some(previous);
            Ok(())
        } else {
            self.current_screen = None;
            Ok(())
        }
    }

    pub fn get_current(&self) -> Option<ScreenType> {
        self.current_screen
    }

    /// Set the current screen directly (used for entering gameplay from menu).
    pub fn set_current_screen(&mut self, screen_type: Option<ScreenType>) {
        self.current_screen = screen_type;
    }

    pub fn get_screen(&self, screen_type: ScreenType) -> Option<&dyn Screen> {
        self.screens.get(&screen_type).map(|s| s.as_ref())
    }

    pub fn get_screen_mut(&mut self, screen_type: ScreenType) -> Option<&mut (dyn Screen + '_)> {
        match self.screens.get_mut(&screen_type) {
            Some(s) => Some(s.as_mut()),
            None => None,
        }
    }

    pub fn update(&mut self, context: &mut UiContext, dt: f32) {
        if let Some(current) = self.current_screen {
            if let Some(screen) = self.screens.get_mut(&current) {
                screen.update(context, dt);
            }
        }
    }

    pub fn render(&self, renderer: &mut dyn crate::scene::Renderer) {
        if let Some(current) = self.current_screen {
            if let Some(screen) = self.screens.get(&current) {
                screen.render(renderer);
            }
        }
    }

    pub fn handle_input(
        &mut self,
        context: &mut UiContext,
        action: mclre_input::Action,
        pressed: bool,
    ) -> bool {
        if let Some(current) = self.current_screen {
            if let Some(screen) = self.screens.get_mut(&current) {
                return screen.handle_input(context, action, pressed);
            }
        }
        false
    }
}

impl Default for ScreenManager {
    fn default() -> Self {
        Self::new()
    }
}
