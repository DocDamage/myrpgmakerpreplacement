//! Formation System
//!
//! Manages party and enemy formations for battle positioning.

use dde_core::Entity;

/// Formation position (front/back row, slot index)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FormationPosition {
    /// Row: true = front, false = back
    pub is_front: bool,
    /// Slot index (0-3)
    pub slot: usize,
}

impl FormationPosition {
    /// Create a new formation position
    pub fn new(is_front: bool, slot: usize) -> Self {
        Self {
            is_front,
            slot: slot.min(3),
        }
    }

    /// Get position in front row
    pub fn front(slot: usize) -> Self {
        Self::new(true, slot)
    }

    /// Get position in back row
    pub fn back(slot: usize) -> Self {
        Self::new(false, slot)
    }
}

/// A slot in the formation
#[derive(Debug, Clone)]
pub struct FormationSlot {
    /// Position
    pub position: FormationPosition,
    /// Entity in this slot (None if empty)
    pub entity: Option<Entity>,
    /// Whether this is a player slot
    pub is_player_slot: bool,
}

impl FormationSlot {
    /// Create a new empty slot
    pub fn new(position: FormationPosition, is_player_slot: bool) -> Self {
        Self {
            position,
            entity: None,
            is_player_slot,
        }
    }

    /// Check if slot is occupied
    pub fn is_occupied(&self) -> bool {
        self.entity.is_some()
    }

    /// Place an entity in this slot
    pub fn place(&mut self, entity: Entity) {
        self.entity = Some(entity);
    }

    /// Clear this slot
    pub fn clear(&mut self) {
        self.entity = None;
    }
}

/// Battle formation
#[derive(Debug, Clone)]
pub struct Formation {
    /// Name of the formation
    pub name: String,
    /// All slots in the formation
    pub slots: Vec<FormationSlot>,
    /// Is this a player formation
    pub is_player_formation: bool,
}

impl Formation {
    /// Create a new formation
    pub fn new(name: impl Into<String>, is_player: bool) -> Self {
        let name = name.into();
        let mut slots = Vec::new();

        // Create 4 front slots and 4 back slots
        for slot in 0..4 {
            slots.push(FormationSlot::new(FormationPosition::front(slot), is_player));
        }
        for slot in 0..4 {
            slots.push(FormationSlot::new(FormationPosition::back(slot), is_player));
        }

        Self {
            name,
            slots,
            is_player_formation: is_player,
        }
    }

    /// Create default player formation
    pub fn default_player() -> Self {
        Self::new("Default Party", true)
    }

    /// Create default enemy formation
    pub fn default_enemy() -> Self {
        Self::new("Enemy Formation", false)
    }

    /// Get slot at position
    pub fn slot_at(&self, position: FormationPosition) -> Option<&FormationSlot> {
        self.slots.iter().find(|s| s.position == position)
    }

    /// Get mutable slot at position
    pub fn slot_at_mut(&mut self, position: FormationPosition) -> Option<&mut FormationSlot> {
        self.slots.iter_mut().find(|s| s.position == position)
    }

    /// Place entity at position
    pub fn place_at(&mut self, position: FormationPosition, entity: Entity) -> bool {
        if let Some(slot) = self.slot_at_mut(position) {
            slot.place(entity);
            true
        } else {
            false
        }
    }

    /// Remove entity from position
    pub fn remove_at(&mut self, position: FormationPosition) -> Option<Entity> {
        if let Some(slot) = self.slot_at_mut(position) {
            let entity = slot.entity;
            slot.clear();
            entity
        } else {
            None
        }
    }

    /// Find which position an entity is in
    pub fn find_entity(&self, entity: Entity) -> Option<FormationPosition> {
        self.slots
            .iter()
            .find(|s| s.entity == Some(entity))
            .map(|s| s.position)
    }

    /// Get all occupied slots
    pub fn occupied_slots(&self) -> Vec<&FormationSlot> {
        self.slots.iter().filter(|s| s.is_occupied()).collect()
    }

    /// Get all entities in the formation
    pub fn entities(&self) -> Vec<Entity> {
        self.slots
            .iter()
            .filter_map(|s| s.entity)
            .collect()
    }

    /// Count of occupied slots
    pub fn count(&self) -> usize {
        self.occupied_slots().len()
    }

    /// Check if formation is empty
    pub fn is_empty(&self) -> bool {
        self.count() == 0
    }

    /// Clear all slots
    pub fn clear(&mut self) {
        for slot in &mut self.slots {
            slot.clear();
        }
    }
}

impl Default for Formation {
    fn default() -> Self {
        Self::default_player()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_formation_creation() {
        let formation = Formation::default_player();
        assert_eq!(formation.slots.len(), 8); // 4 front + 4 back
        assert!(formation.is_player_formation);
    }

    #[test]
    fn test_formation_position() {
        let pos = FormationPosition::front(2);
        assert!(pos.is_front);
        assert_eq!(pos.slot, 2);

        let pos = FormationPosition::back(1);
        assert!(!pos.is_front);
        assert_eq!(pos.slot, 1);
    }

    #[test]
    fn test_slot_management() {
        let mut formation = Formation::default_player();
        let entity = Entity::from_id(1);

        // Place entity
        assert!(formation.place_at(FormationPosition::front(0), entity));
        
        // Check it's there
        assert!(formation.slot_at(FormationPosition::front(0)).unwrap().is_occupied());
        
        // Find entity
        assert_eq!(formation.find_entity(entity), Some(FormationPosition::front(0)));
        
        // Remove entity
        assert_eq!(formation.remove_at(FormationPosition::front(0)), Some(entity));
        assert!(!formation.slot_at(FormationPosition::front(0)).unwrap().is_occupied());
    }
}
