//! Retained-mode data-driven UI toolkit recreating the LCE look: scenes, 9-slice, focus navigation.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use mclre_assets::Identifier;
use mclre_core::Vec3;
use mclre_input::Action;
use mclre_render::Texture;
use thiserror::Error;

pub mod focus;
pub mod layout;
pub mod nine_slice;
pub mod player_ui;
pub mod scene;
pub mod screens;
pub mod widget;

pub use focus::{FocusDirection, FocusManager, Focusable};
pub use layout::{
    AbsoluteContainer, FlexContainer, GridContainer, LayoutBuilder, LayoutConstraints,
    LayoutEngine, LayoutNode,
};
pub use nine_slice::{NineSliceAtlas, NineSliceQuad, NineSliceRegion, compute_nine_slice_uvs};
pub use player_ui::PlayerUIContext;
pub use scene::{Renderer, Scene, SceneId, SceneManager, UiContext, WidgetRegistry};
pub use screens::{
    BaseScreen, HotbarSlot, HudScreen, MainMenuScreen, PauseScreen, Screen, ScreenManager,
    ScreenType, TitleScreen,
};
pub use widget::{BaseWidget, Button, Container, Image, Label, Panel, ScrollView, Widget};

/// Errors from UI operations.
#[derive(Debug, thiserror::Error)]
pub enum UiError {
    #[error("Widget not found: {0}")]
    WidgetNotFound(String),
    #[error("Layout error: {0}")]
    Layout(String),
    #[error("Focus error: {0}")]
    Focus(String),
    #[error("Asset error: {0}")]
    Asset(#[from] mclre_assets::AssetError),
    #[error("Render error: {0}")]
    Render(String),
    #[error("Scene not found: {0}")]
    SceneNotFound(String),
}

pub type Result<T> = std::result::Result<T, UiError>;

/// Unique identifier for a widget.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WidgetId(pub u64);

impl WidgetId {
    pub fn new() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        Self(COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

impl Default for WidgetId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for WidgetId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "WidgetId({})", self.0)
    }
}

/// Rectangle for UI layout.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn contains(&self, point: (f32, f32)) -> bool {
        point.0 >= self.x
            && point.0 < self.x + self.width
            && point.1 >= self.y
            && point.1 < self.y + self.height
    }

    pub fn center(&self) -> (f32, f32) {
        (self.x + self.width * 0.5, self.y + self.height * 0.5)
    }

    pub fn intersects(&self, other: &Rect) -> bool {
        self.x < other.x + other.width
            && self.x + self.width > other.x
            && self.y < other.y + other.height
            && self.y + self.height > other.y
    }
}

impl Default for Rect {
    fn default() -> Self {
        Self::new(0.0, 0.0, 0.0, 0.0)
    }
}

/// Color in RGBA format (0-255).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const WHITE: Color = Color {
        r: 255,
        g: 255,
        b: 255,
        a: 255,
    };
    pub const BLACK: Color = Color {
        r: 0,
        g: 0,
        b: 0,
        a: 255,
    };
    pub const TRANSPARENT: Color = Color {
        r: 0,
        g: 0,
        b: 0,
        a: 0,
    };
    pub const RED: Color = Color {
        r: 255,
        g: 0,
        b: 0,
        a: 255,
    };
    pub const GREEN: Color = Color {
        r: 0,
        g: 255,
        b: 0,
        a: 255,
    };
    pub const BLUE: Color = Color {
        r: 0,
        g: 0,
        b: 255,
        a: 255,
    };

    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub fn with_alpha(self, a: u8) -> Self {
        Self { a, ..self }
    }
}

/// Horizontal alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AlignH {
    #[default]
    Left,
    Center,
    Right,
}

/// Vertical alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AlignV {
    #[default]
    Top,
    Center,
    Bottom,
}

/// Margin for layout.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Margin {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

impl Margin {
    pub const ZERO: Margin = Margin {
        left: 0.0,
        right: 0.0,
        top: 0.0,
        bottom: 0.0,
    };
    pub const fn uniform(all: f32) -> Self {
        Self {
            left: all,
            right: all,
            top: all,
            bottom: all,
        }
    }
    pub fn horizontal(h: f32) -> Self {
        Self {
            left: h,
            right: h,
            top: 0.0,
            bottom: 0.0,
        }
    }
    pub fn vertical(v: f32) -> Self {
        Self {
            left: 0.0,
            right: 0.0,
            top: v,
            bottom: v,
        }
    }
    pub fn new(left: f32, right: f32, top: f32, bottom: f32) -> Self {
        Self {
            left,
            right,
            top,
            bottom,
        }
    }
    pub fn total_width(&self) -> f32 {
        self.left + self.right
    }
    pub fn total_height(&self) -> f32 {
        self.top + self.bottom
    }
}

/// Padding for layout.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Padding {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

impl Padding {
    pub const ZERO: Padding = Padding {
        left: 0.0,
        right: 0.0,
        top: 0.0,
        bottom: 0.0,
    };
    pub const fn uniform(all: f32) -> Self {
        Self {
            left: all,
            right: all,
            top: all,
            bottom: all,
        }
    }
    pub fn horizontal(h: f32) -> Self {
        Self {
            left: h,
            right: h,
            top: 0.0,
            bottom: 0.0,
        }
    }
    pub fn vertical(v: f32) -> Self {
        Self {
            left: 0.0,
            right: 0.0,
            top: v,
            bottom: v,
        }
    }
    pub fn new(left: f32, right: f32, top: f32, bottom: f32) -> Self {
        Self {
            left,
            right,
            top,
            bottom,
        }
    }
    pub fn total_width(&self) -> f32 {
        self.left + self.right
    }
    pub fn total_height(&self) -> f32 {
        self.top + self.bottom
    }
}

/// Type of widget.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WidgetType {
    Container,
    Button,
    Label,
    Image,
    Panel,
    TextInput,
    Slider,
    ScrollView,
    ListView,
    GridView,
    Custom,
}

/// State of a widget.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WidgetState {
    Normal,
    Hovered,
    Focused,
    Pressed,
    Disabled,
    Hidden,
}
