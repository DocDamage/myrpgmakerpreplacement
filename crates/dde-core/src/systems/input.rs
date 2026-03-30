//! Input system

use winit::event::KeyEvent;

use crate::resources::InputState;
use crate::InputAction;

/// Input binding configuration
#[derive(Debug, Clone)]
pub struct InputBindings {
    pub bindings: std::collections::HashMap<InputAction, Vec<String>>,
}

impl Default for InputBindings {
    fn default() -> Self {
        let mut bindings = std::collections::HashMap::new();

        // Default WASD bindings
        bindings.insert(
            InputAction::MoveUp,
            vec!["KeyW".to_string(), "ArrowUp".to_string()],
        );
        bindings.insert(
            InputAction::MoveDown,
            vec!["KeyS".to_string(), "ArrowDown".to_string()],
        );
        bindings.insert(
            InputAction::MoveLeft,
            vec!["KeyA".to_string(), "ArrowLeft".to_string()],
        );
        bindings.insert(
            InputAction::MoveRight,
            vec!["KeyD".to_string(), "ArrowRight".to_string()],
        );
        bindings.insert(
            InputAction::Confirm,
            vec!["Enter".to_string(), "Space".to_string()],
        );
        bindings.insert(InputAction::Cancel, vec!["Escape".to_string()]);
        bindings.insert(InputAction::Menu, vec!["Tab".to_string()]);
        bindings.insert(InputAction::Interact, vec!["KeyE".to_string()]);
        bindings.insert(InputAction::Run, vec!["Shift".to_string()]);

        Self { bindings }
    }
}

/// Input context for different game states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputContext {
    Overworld,
    Dialogue,
    Battle,
    Menu,
    Editor,
}

/// Input system processes raw input into game actions
pub struct InputSystem {
    pub state: InputState,
    pub bindings: InputBindings,
    pub context_stack: Vec<InputContext>,
}

impl InputSystem {
    pub fn new() -> Self {
        Self {
            state: InputState::default(),
            bindings: InputBindings::default(),
            context_stack: vec![InputContext::Overworld],
        }
    }

    /// Get current input context
    pub fn current_context(&self) -> InputContext {
        *self
            .context_stack
            .last()
            .unwrap_or(&InputContext::Overworld)
    }

    /// Push a new context onto the stack
    pub fn push_context(&mut self, context: InputContext) {
        self.context_stack.push(context);
    }

    /// Pop the current context
    pub fn pop_context(&mut self) {
        if self.context_stack.len() > 1 {
            self.context_stack.pop();
        }
    }

    /// Handle keyboard input
    pub fn handle_key_event(&mut self, event: &KeyEvent) {
        let key_str = format!("{:?}", event.logical_key);

        match event.state {
            winit::event::ElementState::Pressed => {
                if !self.state.held.contains(&key_str) {
                    self.state.pressed.insert(key_str.clone());
                }
                self.state.held.insert(key_str);
            }
            winit::event::ElementState::Released => {
                self.state.released.insert(key_str.clone());
                self.state.held.remove(&key_str);
            }
        }
    }

    /// Check if an action is pressed this frame
    pub fn is_action_pressed(&self, action: InputAction) -> bool {
        if let Some(keys) = self.bindings.bindings.get(&action) {
            keys.iter().any(|k| self.state.is_pressed(k))
        } else {
            false
        }
    }

    /// Check if an action is held
    pub fn is_action_held(&self, action: InputAction) -> bool {
        if let Some(keys) = self.bindings.bindings.get(&action) {
            keys.iter().any(|k| self.state.is_held(k))
        } else {
            false
        }
    }

    /// Get movement direction from input
    pub fn get_movement_direction(&self) -> glam::Vec2 {
        let mut dir = glam::Vec2::ZERO;

        if self.is_action_held(InputAction::MoveUp) {
            dir.y -= 1.0;
        }
        if self.is_action_held(InputAction::MoveDown) {
            dir.y += 1.0;
        }
        if self.is_action_held(InputAction::MoveLeft) {
            dir.x -= 1.0;
        }
        if self.is_action_held(InputAction::MoveRight) {
            dir.x += 1.0;
        }

        // Normalize if moving diagonally
        if dir.length_squared() > 0.0 {
            dir = dir.normalize();
        }

        dir
    }

    /// Clear per-frame state (call at end of frame)
    pub fn clear_frame(&mut self) {
        self.state.clear_frame();
    }

    /// Update (called each frame)
    pub fn update(&mut self) {
        // Frame state is cleared at end of frame, not here
    }

    pub fn state(&self) -> &InputState {
        &self.state
    }
}

impl Default for InputSystem {
    fn default() -> Self {
        Self::new()
    }
}
