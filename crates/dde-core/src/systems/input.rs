//! Input system

use crate::resources::InputState;

/// Input system processes raw input into game actions
pub struct InputSystem {
    pub state: InputState,
}

impl InputSystem {
    pub fn new() -> Self {
        Self {
            state: InputState::default(),
        }
    }
    
    pub fn update(&mut self) {
        // Clear per-frame state
        self.state.clear_frame();
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
