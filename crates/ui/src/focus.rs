//! Focus management for UI toolkit.

use super::{Result, UiError, WidgetId};

/// Focus direction for navigation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum FocusDirection {
    #[default]
    Up,
    Down,
    Left,
    Right,
    Next,
    Previous,
}

/// Trait for widgets that can receive focus.
pub trait Focusable {
    fn can_focus(&self) -> bool;
    fn on_focus_gained(&mut self);
    fn on_focus_lost(&mut self);
    fn focus_order(&self) -> i32; // For tab order
}

/// Focus manager for handling keyboard/gamepad navigation.
#[derive(Debug, Default)]
pub struct FocusManager {
    focused_widget: Option<WidgetId>,
    focus_order: Vec<WidgetId>,
    focusable_widgets: std::collections::HashSet<WidgetId>,
    tab_order: Vec<WidgetId>,
}

impl FocusManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a widget as focusable.
    pub fn register_focusable(&mut self, id: WidgetId, order: i32) {
        self.focusable_widgets.insert(id);
        // Insert in tab order
        let pos = self
            .tab_order
            .binary_search_by_key(&order, |&id| {
                // This is a simplified version - in reality you'd store order separately
                0
            })
            .unwrap_or_else(|pos| pos);
        self.tab_order.insert(pos, id);
    }

    /// Unregisters a focusable widget.
    pub fn unregister_focusable(&mut self, id: &WidgetId) {
        self.focusable_widgets.remove(id);
        self.tab_order.retain(|&item| item != *id);
        if self.focused_widget == Some(*id) {
            self.focused_widget = None;
        }
    }

    /// Sets focus to a specific widget.
    pub fn set_focus(&mut self, id: Option<WidgetId>) -> Result<()> {
        if let Some(id) = id {
            if !self.focusable_widgets.contains(&id) {
                return Err(UiError::Focus("Widget is not focusable".into()));
            }
        }
        if let Some(current) = self.focused_widget {
            // Notify old widget of focus loss
            // In a real implementation, you'd call on_focus_lost
        }
        self.focused_widget = id;
        if let Some(id) = id {
            // Notify new widget of focus gain
            // In a real implementation, you'd call on_focus_gained
        }
        Ok(())
    }

    /// Gets the currently focused widget.
    pub fn focused(&self) -> Option<WidgetId> {
        self.focused_widget
    }

    /// Moves focus in a direction.
    pub fn move_focus(&mut self, direction: FocusDirection) -> Result<()> {
        // Simplified implementation - in a real version, this would
        // calculate the nearest focusable widget in the given direction
        match direction {
            FocusDirection::Next | FocusDirection::Right | FocusDirection::Down => {
                self.focus_next()
            }
            FocusDirection::Previous | FocusDirection::Left | FocusDirection::Up => {
                self.focus_previous()
            }
        }
    }

    /// Moves focus to the next focusable widget.
    pub fn focus_next(&mut self) -> Result<()> {
        let current_idx = self
            .focused_widget
            .and_then(|id| {
                self.tab_order
                    .iter()
                    .position(|&id| id == self.focused_widget.unwrap())
            })
            .unwrap_or(0);

        let next_idx = (current_idx + 1) % self.tab_order.len().max(1);
        if let Some(&next_id) = self.tab_order.get(next_idx) {
            self.set_focus(Some(next_id))
        } else {
            Ok(())
        }
    }

    /// Moves focus to the previous focusable widget.
    pub fn focus_previous(&mut self) -> Result<()> {
        let current_idx = self
            .focused_widget
            .and_then(|id| {
                self.tab_order
                    .iter()
                    .position(|&id| id == self.focused_widget.unwrap())
            })
            .unwrap_or(0);

        let prev_idx = if current_idx == 0 {
            self.tab_order.len().saturating_sub(1)
        } else {
            current_idx - 1
        };

        if let Some(&prev_id) = self.tab_order.get(prev_idx) {
            self.set_focus(Some(prev_id))
        } else {
            Ok(())
        }
    }

    /// Clears focus.
    pub fn clear_focus(&mut self) {
        self.focused_widget = None;
    }

    /// Checks if a widget has focus.
    pub fn has_focus(&self, id: WidgetId) -> bool {
        self.focused_widget == Some(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn focus_manager_basic() {
        let mut fm = FocusManager::new();
        let id1 = WidgetId::new();
        let id2 = WidgetId::new();

        fm.register_focusable(id1, 0);
        fm.register_focusable(id2, 1);

        assert!(fm.set_focus(Some(id1)).is_ok());
        assert_eq!(fm.focused(), Some(id1));

        assert!(fm.focus_next().is_ok());
        assert_eq!(fm.focused(), Some(id2));
    }

    #[test]
    fn focus_manager_unregister() {
        let mut fm = FocusManager::new();
        let id = WidgetId::new();

        fm.register_focusable(id, 0);
        fm.set_focus(Some(id)).unwrap();
        assert_eq!(fm.focused(), Some(id));

        fm.unregister_focusable(&id);
        assert_eq!(fm.focused(), None);
        assert!(!fm.focusable_widgets.contains(&id));
    }
}
