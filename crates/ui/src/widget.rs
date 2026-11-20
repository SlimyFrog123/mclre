//! Widget system for UI toolkit.

use super::{
    AlignH, AlignV, Color, Margin, Padding, Rect, Result, UiError, WidgetId, WidgetState,
    WidgetType,
};
use mclre_assets::Identifier;
use std::sync::Arc;

/// Trait for all widgets.
pub trait Widget {
    fn id(&self) -> WidgetId;
    fn widget_type(&self) -> WidgetType;
    fn state(&self) -> WidgetState;
    fn set_state(&mut self, state: WidgetState);
    fn rect(&self) -> Rect;
    fn set_rect(&mut self, rect: Rect);
    fn margin(&self) -> Margin;
    fn set_margin(&mut self, margin: Margin);
    fn padding(&self) -> Padding;
    fn set_padding(&mut self, padding: Padding);
    fn visible(&self) -> bool;
    fn set_visible(&mut self, visible: bool);
    fn enabled(&self) -> bool;
    fn set_enabled(&mut self, enabled: bool);
    fn z_index(&self) -> i32;
    fn set_z_index(&mut self, z: i32);
    fn children(&self) -> &[WidgetId];
    fn children_mut(&mut self) -> &mut Vec<WidgetId>;
    fn add_child(&mut self, child: WidgetId);
    fn remove_child(&mut self, child: WidgetId) -> bool;
    fn parent(&self) -> Option<WidgetId>;
    fn set_parent(&mut self, parent: Option<WidgetId>);
}

/// Base widget implementation.
#[derive(Debug, Clone)]
pub struct BaseWidget {
    id: WidgetId,
    widget_type: WidgetType,
    state: WidgetState,
    rect: Rect,
    margin: Margin,
    padding: Padding,
    visible: bool,
    enabled: bool,
    z_index: i32,
    children: Vec<WidgetId>,
    parent: Option<WidgetId>,
}

impl BaseWidget {
    pub fn new(widget_type: WidgetType) -> Self {
        Self {
            id: WidgetId::new(),
            widget_type,
            state: WidgetState::Normal,
            rect: Rect::default(),
            margin: Margin::ZERO,
            padding: Padding::ZERO,
            visible: true,
            enabled: true,
            z_index: 0,
            children: Vec::new(),
            parent: None,
        }
    }
}

impl Widget for BaseWidget {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn widget_type(&self) -> WidgetType {
        self.widget_type
    }

    fn state(&self) -> WidgetState {
        self.state
    }

    fn set_state(&mut self, state: WidgetState) {
        self.state = state;
    }

    fn rect(&self) -> Rect {
        self.rect
    }

    fn set_rect(&mut self, rect: Rect) {
        self.rect = rect;
    }

    fn margin(&self) -> Margin {
        self.margin
    }

    fn set_margin(&mut self, margin: Margin) {
        self.margin = margin;
    }

    fn padding(&self) -> Padding {
        self.padding
    }

    fn set_padding(&mut self, padding: Padding) {
        self.padding = padding;
    }

    fn visible(&self) -> bool {
        self.visible
    }

    fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    fn enabled(&self) -> bool {
        self.enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    fn z_index(&self) -> i32 {
        self.z_index
    }

    fn set_z_index(&mut self, z: i32) {
        self.z_index = z;
    }

    fn children(&self) -> &[WidgetId] {
        &self.children
    }

    fn children_mut(&mut self) -> &mut Vec<WidgetId> {
        &mut self.children
    }

    fn add_child(&mut self, child: WidgetId) {
        self.children.push(child);
    }

    fn remove_child(&mut self, child: WidgetId) -> bool {
        if let Some(pos) = self.children.iter().position(|&c| c == child) {
            self.children.remove(pos);
            true
        } else {
            false
        }
    }

    fn parent(&self) -> Option<WidgetId> {
        self.parent
    }

    fn set_parent(&mut self, parent: Option<WidgetId>) {
        self.parent = parent;
    }
}

/// Button widget.
pub struct Button {
    base: BaseWidget,
    label: String,
    on_click: Option<Arc<dyn Fn() + Send + Sync>>,
    background_color: [u8; 4],
    text_color: [u8; 4],
    font_size: f32,
}

impl std::fmt::Debug for Button {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Button")
            .field("base", &self.base)
            .field("label", &self.label)
            .field("background_color", &self.background_color)
            .field("text_color", &self.text_color)
            .field("font_size", &self.font_size)
            .finish_non_exhaustive()
    }
}

impl Clone for Button {
    fn clone(&self) -> Self {
        Self {
            base: self.base.clone(),
            label: self.label.clone(),
            on_click: self.on_click.clone(),
            background_color: self.background_color,
            text_color: self.text_color,
            font_size: self.font_size,
        }
    }
}

impl Button {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            base: BaseWidget::new(WidgetType::Button),
            label: label.into(),
            on_click: None,
            background_color: [64, 64, 64, 255],
            text_color: [255, 255, 255, 255],
            font_size: 14.0,
        }
    }

    pub fn on_click<F>(mut self, f: F) -> Self
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.on_click = Some(Arc::new(f));
        self
    }

    pub fn background_color(mut self, color: [u8; 4]) -> Self {
        self.background_color = color;
        self
    }

    pub fn text_color(mut self, color: [u8; 4]) -> Self {
        self.text_color = color;
        self
    }

    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    pub fn on_click_fn(&self) -> Option<&Arc<dyn Fn() + Send + Sync>> {
        self.on_click.as_ref()
    }
}

impl std::ops::Deref for Button {
    type Target = BaseWidget;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl std::ops::DerefMut for Button {
    fn deref_mut(&mut self) -> &mut BaseWidget {
        &mut self.base
    }
}

/// Label widget for text display.
#[derive(Debug, Clone)]
pub struct Label {
    base: BaseWidget,
    text: String,
    color: [u8; 4],
    font_size: f32,
    align_h: AlignH,
    align_v: AlignV,
    wrap: bool,
}

impl Label {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            base: BaseWidget::new(WidgetType::Label),
            text: text.into(),
            color: [255, 255, 255, 255],
            font_size: 14.0,
            align_h: AlignH::Left,
            align_v: AlignV::Top,
            wrap: false,
        }
    }

    pub fn color(mut self, color: [u8; 4]) -> Self {
        self.color = color;
        self
    }

    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    pub fn align_h(mut self, align: AlignH) -> Self {
        self.align_h = align;
        self
    }

    pub fn align_v(mut self, align: AlignV) -> Self {
        self.align_v = align;
        self
    }

    pub fn wrap(mut self, wrap: bool) -> Self {
        self.wrap = wrap;
        self
    }

    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
    }
}

impl std::ops::Deref for Label {
    type Target = BaseWidget;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl std::ops::DerefMut for Label {
    fn deref_mut(&mut self) -> &mut BaseWidget {
        &mut self.base
    }
}

/// Image widget for displaying textures.
#[derive(Debug, Clone)]
pub struct Image {
    base: BaseWidget,
    texture_id: Option<Identifier>,
    uv_rect: Option<[f32; 4]>,
    tint: [u8; 4],
    keep_aspect: bool,
}

impl Image {
    pub fn new(texture_id: impl Into<Identifier>) -> Self {
        Self {
            base: BaseWidget::new(WidgetType::Image),
            texture_id: Some(texture_id.into()),
            uv_rect: None,
            tint: [255, 255, 255, 255],
            keep_aspect: true,
        }
    }

    pub fn uv_rect(mut self, uv: [f32; 4]) -> Self {
        self.uv_rect = Some(uv);
        self
    }

    pub fn tint(mut self, color: [u8; 4]) -> Self {
        self.tint = color;
        self
    }

    pub fn keep_aspect(mut self, keep: bool) -> Self {
        self.keep_aspect = keep;
        self
    }
}

impl std::ops::Deref for Image {
    type Target = BaseWidget;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl std::ops::DerefMut for Image {
    fn deref_mut(&mut self) -> &mut BaseWidget {
        &mut self.base
    }
}

/// Panel widget with 9-slice background.
#[derive(Debug, Clone)]
pub struct Panel {
    base: BaseWidget,
    nine_slice: Option<crate::nine_slice::NineSliceRegion>,
    background_color: [u8; 4],
    border_color: [u8; 4],
    border_width: f32,
}

impl Panel {
    pub fn new() -> Self {
        Self {
            base: BaseWidget::new(WidgetType::Panel),
            nine_slice: None,
            background_color: [32, 32, 32, 255],
            border_color: [128, 128, 128, 255],
            border_width: 1.0,
        }
    }

    pub fn nine_slice(mut self, region: crate::nine_slice::NineSliceRegion) -> Self {
        self.nine_slice = Some(region);
        self
    }

    pub fn background_color(mut self, color: [u8; 4]) -> Self {
        self.background_color = color;
        self
    }

    pub fn border_color(mut self, color: [u8; 4]) -> Self {
        self.border_color = color;
        self
    }

    pub fn border_width(mut self, width: f32) -> Self {
        self.border_width = width;
        self
    }
}

impl Default for Panel {
    fn default() -> Self {
        Self::new()
    }
}

impl std::ops::Deref for Panel {
    type Target = BaseWidget;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl std::ops::DerefMut for Panel {
    fn deref_mut(&mut self) -> &mut BaseWidget {
        &mut self.base
    }
}

/// Container widget for grouping children.
#[derive(Debug, Clone)]
pub struct Container {
    base: BaseWidget,
    layout: Option<crate::layout::LayoutNode>,
}

impl Container {
    pub fn new() -> Self {
        Self {
            base: BaseWidget::new(WidgetType::Container),
            layout: None,
        }
    }

    pub fn layout(mut self, layout: crate::layout::LayoutNode) -> Self {
        self.layout = Some(layout);
        self
    }
}

impl Default for Container {
    fn default() -> Self {
        Self::new()
    }
}

impl std::ops::Deref for Container {
    type Target = BaseWidget;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl std::ops::DerefMut for Container {
    fn deref_mut(&mut self) -> &mut BaseWidget {
        &mut self.base
    }
}

/// Scroll view for scrollable content.
#[derive(Debug, Clone)]
pub struct ScrollView {
    base: BaseWidget,
    content: WidgetId,
    scroll_x: f32,
    scroll_y: f32,
    scroll_speed: f32,
    horizontal_scroll: bool,
    vertical_scroll: bool,
}

impl ScrollView {
    pub fn new(content: WidgetId) -> Self {
        Self {
            base: BaseWidget::new(WidgetType::Container),
            content,
            scroll_x: 0.0,
            scroll_y: 0.0,
            scroll_speed: 1.0,
            horizontal_scroll: true,
            vertical_scroll: true,
        }
    }

    pub fn scroll_speed(mut self, speed: f32) -> Self {
        self.scroll_speed = speed;
        self
    }

    pub fn horizontal_scroll(mut self, enabled: bool) -> Self {
        self.horizontal_scroll = enabled;
        self
    }

    pub fn vertical_scroll(mut self, enabled: bool) -> Self {
        self.vertical_scroll = enabled;
        self
    }
}

impl std::ops::Deref for ScrollView {
    type Target = BaseWidget;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl std::ops::DerefMut for ScrollView {
    fn deref_mut(&mut self) -> &mut BaseWidget {
        &mut self.base
    }
}

/// Widget builder for fluent construction.
pub struct WidgetBuilder {
    widget_type: WidgetType,
    id: Option<WidgetId>,
    rect: Option<Rect>,
    margin: Margin,
    padding: Padding,
    visible: bool,
    enabled: bool,
    z_index: i32,
}

impl WidgetBuilder {
    pub fn new(widget_type: WidgetType) -> Self {
        Self {
            widget_type,
            id: None,
            rect: None,
            margin: Margin::ZERO,
            padding: Padding::ZERO,
            visible: true,
            enabled: true,
            z_index: 0,
        }
    }

    pub fn id(mut self, id: WidgetId) -> Self {
        self.id = Some(id);
        self
    }

    pub fn rect(mut self, rect: Rect) -> Self {
        self.rect = Some(rect);
        self
    }

    pub fn margin(mut self, margin: Margin) -> Self {
        self.margin = margin;
        self
    }

    pub fn padding(mut self, padding: Padding) -> Self {
        self.padding = padding;
        self
    }

    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn z_index(mut self, z: i32) -> Self {
        self.z_index = z;
        self
    }

    pub fn build<W: Widget + Default>(self) -> W {
        let mut widget = W::default();
        if let Some(id) = self.id {
            widget.set_rect(self.rect.unwrap_or_default());
            widget.set_margin(self.margin);
            widget.set_padding(self.padding);
            widget.set_visible(self.visible);
            widget.set_enabled(self.enabled);
            widget.set_z_index(self.z_index);
        }
        widget
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_widget_creation() {
        let widget = BaseWidget::new(WidgetType::Container);
        assert_eq!(widget.widget_type(), WidgetType::Container);
        assert_eq!(widget.state(), WidgetState::Normal);
        assert!(widget.visible());
        assert!(widget.enabled());
    }

    #[test]
    fn button_creation() {
        let button = Button::new("Click me");
        assert_eq!(button.widget_type(), WidgetType::Button);
        assert_eq!(button.label, "Click me");
    }

    #[test]
    fn label_creation() {
        let label = Label::new("Hello World");
        assert_eq!(label.text, "Hello World");
        assert_eq!(label.font_size, 14.0);
    }

    #[test]
    fn panel_default() {
        let panel = Panel::default();
        assert_eq!(panel.background_color, [32, 32, 32, 255]);
        assert_eq!(panel.border_width, 1.0);
    }
}
