//! Input abstraction: keyboard/mouse/gamepad actions, per-player device bindings (LCE-style layouts).

use std::collections::HashMap;

use gilrs::{Button, Gilrs};
use parking_lot::Mutex;
use winit::{
    event::{ElementState, KeyEvent, MouseButton, MouseScrollDelta},
    keyboard::{KeyCode, PhysicalKey},
};

/// User input action (high-level, device-agnostic).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    // Movement
    MoveForward,
    MoveBackward,
    MoveLeft,
    MoveRight,
    Jump,
    Sprint,
    Sneak,

    // Camera
    LookUp,
    LookDown,
    LookLeft,
    LookRight,

    // Interaction
    UseItem,
    Attack,
    DropItem,
    PickBlock,

    // Inventory
    Inventory,
    Hotbar1,
    Hotbar2,
    Hotbar3,
    Hotbar4,
    Hotbar5,
    Hotbar6,
    Hotbar7,
    Hotbar8,
    Hotbar9,

    // Game
    Pause,
    Chat,
    Command,
    Screenshot,
    TogglePerspective,
    ToggleFullscreen,

    // Splitscreen
    Join,  // Join game as additional player (splitscreen)
    Leave, // Leave game (splitscreen)

    // UI Navigation (for gamepad/keyboard UI control)
    NavigateUp,
    NavigateDown,
    NavigateLeft,
    NavigateRight,
    NavigateSelect,
    NavigateBack,
    NavigateNext,
    NavigatePrevious,
}

/// Input state for a single action.
#[derive(Debug, Clone, Copy, Default)]
pub struct ActionState {
    /// Whether the action is currently pressed.
    pub pressed: bool,
    /// Whether the action was just pressed this frame.
    pub just_pressed: bool,
    /// Whether the action was just released this frame.
    pub just_released: bool,
}

impl ActionState {
    /// Updates the action state.
    pub fn update(&mut self, pressed: bool) {
        self.just_pressed = !self.pressed && pressed;
        self.just_released = self.pressed && !pressed;
        self.pressed = pressed;
    }

    /// Resets the just_pressed/just_released flags.
    pub fn reset_transient(&mut self) {
        self.just_pressed = false;
        self.just_released = false;
    }
}

/// Keyboard binding for an action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyboardBinding {
    pub key: KeyCode,
}

/// Mouse binding for an action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseBinding {
    Left,
    Right,
    Middle,
    Button4,
    Button5,
    ScrollUp,
    ScrollDown,
}

/// Gamepad binding for an action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GamepadBinding {
    Button(Button),
    AxisLeftX,
    AxisLeftY,
    AxisRightX,
    AxisRightY,
}

/// Input manager for a single player.
#[derive(Debug)]
pub struct InputManager {
    /// Current action states.
    action_states: HashMap<Action, ActionState>,
    /// Keyboard bindings.
    keyboard_bindings: HashMap<KeyCode, Action>,
    /// Mouse bindings.
    mouse_bindings: HashMap<MouseBinding, Action>,
    /// Gamepad bindings.
    gamepad_bindings: HashMap<GamepadBinding, Action>,
    /// Mouse delta for camera movement.
    mouse_delta: Mutex<(f32, f32)>,
    /// Whether mouse is captured.
    mouse_captured: bool,
    /// Mouse sensitivity.
    mouse_sensitivity: f32,
}

impl InputManager {
    /// Creates a new input manager with default LCE-style bindings.
    pub fn new() -> Self {
        let mut manager = Self {
            action_states: HashMap::new(),
            keyboard_bindings: HashMap::new(),
            mouse_bindings: HashMap::new(),
            gamepad_bindings: HashMap::new(),
            mouse_delta: Mutex::new((0.0, 0.0)),
            mouse_captured: false,
            mouse_sensitivity: 0.002,
        };

        // Initialize all action states
        for action in Self::all_actions() {
            manager.action_states.insert(action, ActionState::default());
        }

        // Default LCE-style keyboard bindings
        manager.bind_keyboard(KeyCode::KeyW, Action::MoveForward);
        manager.bind_keyboard(KeyCode::KeyS, Action::MoveBackward);
        manager.bind_keyboard(KeyCode::KeyA, Action::MoveLeft);
        manager.bind_keyboard(KeyCode::KeyD, Action::MoveRight);
        manager.bind_keyboard(KeyCode::Space, Action::Jump);
        manager.bind_keyboard(KeyCode::ControlLeft, Action::Sneak);
        manager.bind_keyboard(KeyCode::ShiftLeft, Action::Sprint);
        manager.bind_keyboard(KeyCode::KeyE, Action::Inventory);
        manager.bind_keyboard(KeyCode::KeyQ, Action::DropItem);
        manager.bind_keyboard(KeyCode::Escape, Action::Pause);
        manager.bind_keyboard(KeyCode::KeyT, Action::Chat);
        manager.bind_keyboard(KeyCode::Slash, Action::Command);
        manager.bind_keyboard(KeyCode::F2, Action::Screenshot);
        manager.bind_keyboard(KeyCode::F5, Action::TogglePerspective);
        manager.bind_keyboard(KeyCode::F11, Action::ToggleFullscreen);
        manager.bind_keyboard(KeyCode::KeyJ, Action::Join);
        manager.bind_keyboard(KeyCode::KeyL, Action::Leave);

        // Number keys for hotbar
        for (i, key) in [
            KeyCode::Digit1,
            KeyCode::Digit2,
            KeyCode::Digit3,
            KeyCode::Digit4,
            KeyCode::Digit5,
            KeyCode::Digit6,
            KeyCode::Digit7,
            KeyCode::Digit8,
            KeyCode::Digit9,
        ]
        .into_iter()
        .enumerate()
        {
            let action = match i {
                0 => Action::Hotbar1,
                1 => Action::Hotbar2,
                2 => Action::Hotbar3,
                3 => Action::Hotbar4,
                4 => Action::Hotbar5,
                5 => Action::Hotbar6,
                6 => Action::Hotbar7,
                7 => Action::Hotbar8,
                8 => Action::Hotbar9,
                _ => continue,
            };
            manager.bind_keyboard(key, action);
        }

        // Default mouse bindings
        manager.bind_mouse(MouseBinding::Left, Action::Attack);
        manager.bind_mouse(MouseBinding::Right, Action::UseItem);
        manager.bind_mouse(MouseBinding::Middle, Action::PickBlock);
        manager.bind_mouse(MouseBinding::ScrollUp, Action::Hotbar1);
        manager.bind_mouse(MouseBinding::ScrollDown, Action::Hotbar1);

        // Default gamepad bindings (Xbox-style)
        manager.bind_gamepad(GamepadBinding::Button(Button::DPadUp), Action::MoveForward);
        manager.bind_gamepad(
            GamepadBinding::Button(Button::DPadDown),
            Action::MoveBackward,
        );
        manager.bind_gamepad(GamepadBinding::Button(Button::DPadLeft), Action::MoveLeft);
        manager.bind_gamepad(GamepadBinding::Button(Button::DPadRight), Action::MoveRight);
        manager.bind_gamepad(GamepadBinding::Button(Button::South), Action::Jump); // A
        manager.bind_gamepad(GamepadBinding::Button(Button::East), Action::Attack); // B
        manager.bind_gamepad(GamepadBinding::Button(Button::West), Action::UseItem); // X
        manager.bind_gamepad(GamepadBinding::Button(Button::North), Action::Inventory); // Y
        manager.bind_gamepad(GamepadBinding::Button(Button::LeftTrigger), Action::Sneak);
        manager.bind_gamepad(GamepadBinding::Button(Button::RightTrigger), Action::Sprint);
        manager.bind_gamepad(GamepadBinding::Button(Button::LeftThumb), Action::PickBlock);
        manager.bind_gamepad(
            GamepadBinding::Button(Button::RightThumb),
            Action::TogglePerspective,
        );
        manager.bind_gamepad(GamepadBinding::Button(Button::Start), Action::Pause);
        manager.bind_gamepad(GamepadBinding::Button(Button::Select), Action::Chat);
        manager.bind_gamepad(GamepadBinding::Button(Button::Start), Action::Join);
        manager.bind_gamepad(GamepadBinding::Button(Button::Mode), Action::Leave);

        // Gamepad analog sticks for camera
        manager.bind_gamepad(GamepadBinding::AxisRightX, Action::LookLeft);
        manager.bind_gamepad(GamepadBinding::AxisRightY, Action::LookDown);

        // UI Navigation bindings (keyboard - arrow keys)
        manager.bind_keyboard(KeyCode::ArrowUp, Action::NavigateUp);
        manager.bind_keyboard(KeyCode::ArrowDown, Action::NavigateDown);
        manager.bind_keyboard(KeyCode::ArrowLeft, Action::NavigateLeft);
        manager.bind_keyboard(KeyCode::ArrowRight, Action::NavigateRight);
        manager.bind_keyboard(KeyCode::Enter, Action::NavigateSelect);
        manager.bind_keyboard(KeyCode::Escape, Action::NavigateBack);
        manager.bind_keyboard(KeyCode::Tab, Action::NavigateNext);
        manager.bind_keyboard(KeyCode::Backspace, Action::NavigatePrevious);

        // UI Navigation bindings (gamepad - D-pad and face buttons)
        manager.bind_gamepad(GamepadBinding::Button(Button::DPadUp), Action::NavigateUp);
        manager.bind_gamepad(
            GamepadBinding::Button(Button::DPadDown),
            Action::NavigateDown,
        );
        manager.bind_gamepad(
            GamepadBinding::Button(Button::DPadLeft),
            Action::NavigateLeft,
        );
        manager.bind_gamepad(
            GamepadBinding::Button(Button::DPadRight),
            Action::NavigateRight,
        );
        manager.bind_gamepad(
            GamepadBinding::Button(Button::South),
            Action::NavigateSelect,
        ); // A
        manager.bind_gamepad(GamepadBinding::Button(Button::East), Action::NavigateBack); // B
        manager.bind_gamepad(
            GamepadBinding::Button(Button::LeftTrigger2),
            Action::NavigatePrevious,
        ); // LB
        manager.bind_gamepad(
            GamepadBinding::Button(Button::RightTrigger2),
            Action::NavigateNext,
        ); // RB

        manager
    }

    /// Returns all possible actions.
    fn all_actions() -> Vec<Action> {
        vec![
            Action::MoveForward,
            Action::MoveBackward,
            Action::MoveLeft,
            Action::MoveRight,
            Action::Jump,
            Action::Sprint,
            Action::Sneak,
            Action::LookUp,
            Action::LookDown,
            Action::LookLeft,
            Action::LookRight,
            Action::UseItem,
            Action::Attack,
            Action::DropItem,
            Action::PickBlock,
            Action::Inventory,
            Action::Hotbar1,
            Action::Hotbar2,
            Action::Hotbar3,
            Action::Hotbar4,
            Action::Hotbar5,
            Action::Hotbar6,
            Action::Hotbar7,
            Action::Hotbar8,
            Action::Hotbar9,
            Action::Pause,
            Action::Chat,
            Action::Command,
            Action::Screenshot,
            Action::TogglePerspective,
            Action::ToggleFullscreen,
            // UI Navigation
            Action::NavigateUp,
            Action::NavigateDown,
            Action::NavigateLeft,
            Action::NavigateRight,
            Action::NavigateSelect,
            Action::NavigateBack,
            Action::NavigateNext,
            Action::NavigatePrevious,
        ]
    }

    /// Binds a keyboard key to an action.
    pub fn bind_keyboard(&mut self, key: KeyCode, action: Action) {
        self.keyboard_bindings.insert(key, action);
    }

    /// Binds a mouse button to an action.
    pub fn bind_mouse(&mut self, mouse: MouseBinding, action: Action) {
        self.mouse_bindings.insert(mouse, action);
    }

    /// Binds a gamepad input to an action.
    pub fn bind_gamepad(&mut self, gamepad: GamepadBinding, action: Action) {
        self.gamepad_bindings.insert(gamepad, action);
    }

    /// Sets mouse sensitivity.
    pub fn set_mouse_sensitivity(&mut self, sensitivity: f32) {
        self.mouse_sensitivity = sensitivity;
    }

    /// Gets mouse sensitivity.
    pub fn mouse_sensitivity(&self) -> f32 {
        self.mouse_sensitivity
    }

    /// Sets whether the mouse is captured.
    pub fn set_mouse_captured(&mut self, captured: bool) {
        self.mouse_captured = captured;
    }

    /// Returns whether the mouse is captured.
    pub fn mouse_captured(&self) -> bool {
        self.mouse_captured
    }

    /// Gets the current state of an action.
    pub fn action_state(&self, action: Action) -> Option<&ActionState> {
        self.action_states.get(&action)
    }

    /// Checks if an action is currently pressed.
    pub fn is_pressed(&self, action: Action) -> bool {
        self.action_states
            .get(&action)
            .map(|s| s.pressed)
            .unwrap_or(false)
    }

    /// Checks if an action was just pressed this frame.
    pub fn just_pressed(&self, action: Action) -> bool {
        self.action_states
            .get(&action)
            .map(|s| s.just_pressed)
            .unwrap_or(false)
    }

    /// Checks if an action was just released this frame.
    pub fn just_released(&self, action: Action) -> bool {
        self.action_states
            .get(&action)
            .map(|s| s.just_released)
            .unwrap_or(false)
    }

    /// Gets the mouse delta and resets it.
    pub fn take_mouse_delta(&self) -> (f32, f32) {
        let mut delta = self.mouse_delta.lock();
        let result = *delta;
        *delta = (0.0, 0.0);
        result
    }

    /// Processes a keyboard event.
    pub fn process_keyboard(&mut self, event: KeyEvent) {
        if let PhysicalKey::Code(key) = event.physical_key {
            if let Some(action) = self.keyboard_bindings.get(&key) {
                if let Some(state) = self.action_states.get_mut(action) {
                    state.update(event.state == ElementState::Pressed);
                }
            }
        }
    }

    /// Processes a mouse button event.
    pub fn process_mouse_button(&mut self, button: MouseButton, state: ElementState) {
        let binding = match button {
            MouseButton::Left => MouseBinding::Left,
            MouseButton::Right => MouseBinding::Right,
            MouseButton::Middle => MouseBinding::Middle,
            MouseButton::Back => MouseBinding::Button4,
            MouseButton::Forward => MouseBinding::Button5,
            _ => return,
        };

        if let Some(action) = self.mouse_bindings.get(&binding) {
            if let Some(action_state) = self.action_states.get_mut(action) {
                action_state.update(state == ElementState::Pressed);
            }
        }
    }

    /// Processes a mouse scroll event.
    pub fn process_mouse_scroll(&mut self, delta: MouseScrollDelta) {
        let action = match delta {
            MouseScrollDelta::LineDelta(_, y) if y > 0.0 => Action::Hotbar1,
            MouseScrollDelta::LineDelta(_, y) if y < 0.0 => Action::Hotbar1,
            MouseScrollDelta::PixelDelta(pos) if pos.y > 0.0 => Action::Hotbar1,
            MouseScrollDelta::PixelDelta(pos) if pos.y < 0.0 => Action::Hotbar1,
            _ => return,
        };

        if let Some(state) = self.action_states.get_mut(&action) {
            state.update(true);
            // Note: In practice, you'd cycle hotbar here
        }
    }

    /// Processes mouse movement.
    pub fn process_mouse_motion(&mut self, delta_x: f32, delta_y: f32) {
        if !self.mouse_captured {
            return;
        }
        let mut delta = self.mouse_delta.lock();
        delta.0 += delta_x * self.mouse_sensitivity;
        delta.1 += delta_y * self.mouse_sensitivity;
    }

    /// Updates gamepad state for a specific gamepad (call once per frame).
    pub fn update_gamepad(&mut self, gilrs: &Gilrs, gamepad_id: gilrs::GamepadId) {
        let gamepad = gilrs.gamepad(gamepad_id);

        // Process buttons
        for (binding, action) in &self.gamepad_bindings {
            if let GamepadBinding::Button(button) = binding {
                let pressed = gamepad.is_pressed(*button);
                if let Some(state) = self.action_states.get_mut(action) {
                    state.update(pressed);
                }
            }
        }

        // Process axes
        for (binding, action) in &self.gamepad_bindings {
            match binding {
                GamepadBinding::AxisLeftX => {
                    let value = gamepad.value(gilrs::Axis::LeftStickX);
                    if let Some(state) = self.action_states.get_mut(action) {
                        state.update(value < -0.5);
                    }
                }
                GamepadBinding::AxisLeftY => {
                    let value = gamepad.value(gilrs::Axis::LeftStickY);
                    if let Some(state) = self.action_states.get_mut(action) {
                        state.update(value > 0.5);
                    }
                }
                GamepadBinding::AxisRightX => {
                    let value = gamepad.value(gilrs::Axis::RightStickX);
                    // Update look left/right based on axis
                    if value < -0.1 {
                        self.action_states
                            .get_mut(&Action::LookLeft)
                            .map(|s| s.update(true));
                    } else if value > 0.1 {
                        self.action_states
                            .get_mut(&Action::LookRight)
                            .map(|s| s.update(true));
                    }
                }
                GamepadBinding::AxisRightY => {
                    let value = gamepad.value(gilrs::Axis::RightStickY);
                    if value < -0.1 {
                        self.action_states
                            .get_mut(&Action::LookUp)
                            .map(|s| s.update(true));
                    } else if value > 0.1 {
                        self.action_states
                            .get_mut(&Action::LookDown)
                            .map(|s| s.update(true));
                    }
                }
                _ => {}
            }
        }
    }

    /// Resets transient states (call at end of frame).
    pub fn end_frame(&mut self) {
        for state in self.action_states.values_mut() {
            state.reset_transient();
        }
    }
}

impl Default for InputManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Events that can be sent from the input system to the game.
#[derive(Debug, Clone)]
pub enum InputEvent {
    ActionPressed(Action),
    ActionReleased(Action),
    MouseMoved(f32, f32),
    MouseCaptured(bool),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_manager_default_bindings() {
        let manager = InputManager::new();
        assert!(manager.is_pressed(Action::MoveForward) == false);
        assert!(manager.keyboard_bindings.contains_key(&KeyCode::KeyW));
        assert!(manager.mouse_bindings.contains_key(&MouseBinding::Left));
    }

    #[test]
    fn action_state_transitions() {
        let mut state = ActionState::default();
        assert!(!state.pressed);
        assert!(!state.just_pressed);

        state.update(true);
        assert!(state.pressed);
        assert!(state.just_pressed);
        assert!(!state.just_released);

        state.update(true);
        assert!(state.pressed);
        assert!(!state.just_pressed);

        state.update(false);
        assert!(!state.pressed);
        assert!(state.just_released);

        state.update(false);
        assert!(!state.just_released);
    }
}
