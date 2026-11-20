//! Layout system for UI toolkit.

use super::{AlignH, AlignV, Margin, Padding, Rect, Result, UiError, WidgetId};
use std::collections::HashMap;

/// Layout engine for computing widget positions and sizes.
pub struct LayoutEngine {
    constraints: LayoutConstraints,
}

impl LayoutEngine {
    pub fn new(constraints: LayoutConstraints) -> Self {
        Self { constraints }
    }

    /// Computes layout for a widget tree.
    pub fn compute_layout(&self, widget: &mut dyn Layoutable, available: Rect) -> Result<Rect> {
        widget.layout(self, available)
    }
}

/// Constraints for layout computation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayoutConstraints {
    pub max_width: f32,
    pub max_height: f32,
    pub min_width: f32,
    pub min_height: f32,
}

impl Default for LayoutConstraints {
    fn default() -> Self {
        Self {
            max_width: f32::INFINITY,
            max_height: f32::INFINITY,
            min_width: 0.0,
            min_height: 0.0,
        }
    }
}

/// Trait for widgets that can be laid out.
pub trait Layoutable {
    /// Computes the desired size of this widget given constraints.
    fn desired_size(&self, constraints: LayoutConstraints) -> (f32, f32);

    /// Performs layout and returns the final rect.
    fn layout(&mut self, engine: &LayoutEngine, available: Rect) -> Result<Rect>;

    /// Returns the widget's ID.
    fn id(&self) -> crate::WidgetId;

    /// Returns the computed rect after layout.
    fn rect(&self) -> crate::Rect;
}

/// Flex direction for flexbox layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FlexDirection {
    #[default]
    Row,
    Column,
    RowReverse,
    ColumnReverse,
}

/// Justification for flexbox layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum JustifyContent {
    #[default]
    FlexStart,
    FlexEnd,
    Center,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

/// Alignment for flexbox cross axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AlignItems {
    #[default]
    Stretch,
    FlexStart,
    FlexEnd,
    Center,
    Baseline,
}

/// Flex container for flexible box layout.
#[derive(Debug, Clone)]
pub struct FlexContainer {
    pub direction: FlexDirection,
    pub justify: JustifyContent,
    pub align_items: AlignItems,
    pub gap: f32,
    pub children: Vec<WidgetId>,
    pub padding: super::Padding,
    pub margin: super::Margin,
}

impl Default for FlexContainer {
    fn default() -> Self {
        Self {
            direction: FlexDirection::Row,
            justify: JustifyContent::FlexStart,
            align_items: AlignItems::Stretch,
            gap: 0.0,
            children: Vec::new(),
            padding: super::Padding::ZERO,
            margin: super::Margin::ZERO,
        }
    }
}

/// Grid container for grid layout.
#[derive(Debug, Clone)]
pub struct GridContainer {
    pub columns: u32,
    pub rows: u32,
    pub column_gap: f32,
    pub row_gap: f32,
    pub children: Vec<(WidgetId, u32, u32)>, // (widget_id, col, row)
    pub padding: super::Padding,
    pub margin: super::Margin,
}

impl Default for GridContainer {
    fn default() -> Self {
        Self {
            columns: 1,
            rows: 1,
            column_gap: 0.0,
            row_gap: 0.0,
            children: Vec::new(),
            padding: super::Padding::ZERO,
            margin: super::Margin::ZERO,
        }
    }
}

/// Absolute positioned container.
#[derive(Debug, Clone, Default)]
pub struct AbsoluteContainer {
    pub children: Vec<(WidgetId, f32, f32)>, // (widget_id, x, y)
    pub padding: super::Padding,
    pub margin: super::Margin,
}

/// Layout builder for constructing layouts fluently.
pub struct LayoutBuilder {
    root: Option<LayoutNode>,
}

impl LayoutBuilder {
    pub fn new() -> Self {
        Self { root: None }
    }

    pub fn flex(self, container: FlexContainer) -> Self {
        Self {
            root: Some(LayoutNode::Flex(container)),
        }
    }

    pub fn grid(self, container: GridContainer) -> Self {
        Self {
            root: Some(LayoutNode::Grid(container)),
        }
    }

    pub fn absolute(self, container: AbsoluteContainer) -> Self {
        Self {
            root: Some(LayoutNode::Absolute(container)),
        }
    }

    pub fn build(self) -> Option<LayoutNode> {
        self.root
    }
}

/// Layout node types.
#[derive(Debug, Clone)]
pub enum LayoutNode {
    Flex(FlexContainer),
    Grid(GridContainer),
    Absolute(AbsoluteContainer),
    Widget(WidgetId),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flex_container_default() {
        let c = FlexContainer::default();
        assert_eq!(c.direction, FlexDirection::Row);
        assert_eq!(c.justify, JustifyContent::FlexStart);
        assert_eq!(c.align_items, AlignItems::Stretch);
    }

    #[test]
    fn grid_container_default() {
        let g = GridContainer::default();
        assert_eq!(g.columns, 1);
        assert_eq!(g.rows, 1);
    }

    #[test]
    fn layout_builder() {
        let layout = LayoutBuilder::new().flex(FlexContainer::default()).build();
        assert!(layout.is_some());
    }
}
