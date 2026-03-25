//! DocDamage Engine - Editor Layer
//! 
//! Editor mode with egui panels for world editing.

/// Editor state
pub struct Editor {
    pub active: bool,
    pub selected_entity: Option<dde_core::Entity>,
}

impl Editor {
    pub fn new() -> Self {
        Self {
            active: false,
            selected_entity: None,
        }
    }
    
    pub fn toggle(&mut self) {
        self.active = !self.active;
    }
    
    pub fn is_active(&self) -> bool {
        self.active
    }
}

impl Default for Editor {
    fn default() -> Self {
        Self::new()
    }
}
