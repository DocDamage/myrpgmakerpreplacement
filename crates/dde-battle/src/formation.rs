//! Formation System for the DocDamage Engine
//!
//! Party positioning affects battle mechanics (front row vs back row).
//! - Front Row: More damage dealt and taken
//! - Back Row: Less damage taken, reduced physical accuracy

use dde_core::{Entity, World};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// Position in battle formation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(Default)]
pub enum FormationPosition {
    /// Row 0 - More damage taken/given
    #[default]
    FrontRow,
    /// Row 1 - Less damage, reduced physical accuracy
    BackRow,
    // Future: FlankLeft, FlankRight, etc.
}

impl FormationPosition {
    /// Get modifiers for this position
    pub fn modifiers(&self) -> FormationModifiers {
        match self {
            FormationPosition::FrontRow => FormationModifiers {
                damage_dealt_mult: 1.1,  // +10% damage dealt
                damage_taken_mult: 1.2,  // +20% damage taken
                physical_accuracy_mult: 1.0,
                magic_accuracy_mult: 1.0,
                atb_speed_mult: 1.0,
            },
            FormationPosition::BackRow => FormationModifiers {
                damage_dealt_mult: 0.85, // -15% physical damage
                damage_taken_mult: 0.75, // -25% damage taken
                physical_accuracy_mult: 0.8, // -20% physical accuracy
                magic_accuracy_mult: 1.0,    // Magic unaffected
                atb_speed_mult: 0.95,    // Slightly slower
            },
        }
    }

    /// Description for UI
    pub fn description(&self) -> &'static str {
        match self {
            FormationPosition::FrontRow => "Take and deal more damage",
            FormationPosition::BackRow => "Take less damage, reduced physical accuracy",
        }
    }

    /// Icon for UI
    pub fn icon(&self) -> &'static str {
        match self {
            FormationPosition::FrontRow => "⚔️",
            FormationPosition::BackRow => "🛡️",
        }
    }

    /// Get the row index (0 for front, 1 for back)
    pub fn row_index(&self) -> u8 {
        match self {
            FormationPosition::FrontRow => 0,
            FormationPosition::BackRow => 1,
        }
    }
}


/// Modifiers applied based on formation position
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct FormationModifiers {
    /// Multiplier for damage dealt
    pub damage_dealt_mult: f32,
    /// Multiplier for damage taken
    pub damage_taken_mult: f32,
    /// Multiplier for physical attack accuracy
    pub physical_accuracy_mult: f32,
    /// Multiplier for magic attack accuracy
    pub magic_accuracy_mult: f32,
    /// Multiplier for ATB gauge fill speed
    pub atb_speed_mult: f32,
}

impl FormationModifiers {
    /// Combine two sets of modifiers (multiplicative)
    pub fn combine(&self, other: &FormationModifiers) -> FormationModifiers {
        FormationModifiers {
            damage_dealt_mult: self.damage_dealt_mult * other.damage_dealt_mult,
            damage_taken_mult: self.damage_taken_mult * other.damage_taken_mult,
            physical_accuracy_mult: self.physical_accuracy_mult * other.physical_accuracy_mult,
            magic_accuracy_mult: self.magic_accuracy_mult * other.magic_accuracy_mult,
            atb_speed_mult: self.atb_speed_mult * other.atb_speed_mult,
        }
    }

    /// Get a modifier summary for UI display
    pub fn summary(&self) -> Vec<(&'static str, f32, &'static str)> {
        let mut summary = Vec::new();
        
        if self.damage_dealt_mult != 1.0 {
            summary.push(("Damage Dealt", self.damage_dealt_mult, "%"));
        }
        if self.damage_taken_mult != 1.0 {
            summary.push(("Damage Taken", self.damage_taken_mult, "%"));
        }
        if self.physical_accuracy_mult != 1.0 {
            summary.push(("Physical Acc", self.physical_accuracy_mult, "%"));
        }
        if self.magic_accuracy_mult != 1.0 {
            summary.push(("Magic Acc", self.magic_accuracy_mult, "%"));
        }
        if self.atb_speed_mult != 1.0 {
            summary.push(("ATB Speed", self.atb_speed_mult, "%"));
        }
        
        summary
    }
}

/// Formation slot assignment (ECS Component)
/// Formation slot component for ECS
/// 
/// Note: To use this as an ECS component, manually implement `hecs::Component`
/// or use `world.insert(entity, (FormationSlot::new(...),))`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormationSlot {
    pub position: FormationPosition,
    pub slot_index: u8, // 0-3 within row
}

impl FormationSlot {
    /// Create a new formation slot
    pub fn new(position: FormationPosition, slot_index: u8) -> Self {
        Self {
            position,
            slot_index: slot_index.min(3), // Clamp to valid range
        }
    }

    /// Get modifiers for this slot
    pub fn modifiers(&self) -> FormationModifiers {
        self.position.modifiers()
    }

    /// Check if this slot is in the front row
    pub fn is_front_row(&self) -> bool {
        matches!(self.position, FormationPosition::FrontRow)
    }

    /// Check if this slot is in the back row
    pub fn is_back_row(&self) -> bool {
        matches!(self.position, FormationPosition::BackRow)
    }
}

impl Default for FormationSlot {
    fn default() -> Self {
        Self {
            position: FormationPosition::FrontRow,
            slot_index: 0,
        }
    }
}

/// Formation slot assignment for a specific entity
/// 
/// Note: Entity does not implement Serialize/Deserialize by default.
/// For persistence, convert to/from a unique identifier.
#[derive(Debug, Clone)]
pub struct FormationSlotAssignment {
    pub entity: Entity,
    pub position: FormationPosition,
    pub slot_index: u8,
}

impl FormationSlotAssignment {
    /// Create a new slot assignment
    pub fn new(entity: Entity, position: FormationPosition, slot_index: u8) -> Self {
        Self {
            entity,
            position,
            slot_index: slot_index.min(3),
        }
    }
}

/// Serializable version of FormationSlotAssignment for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableFormationSlot {
    /// Entity identifier as bits (may need mapping when loading)
    pub entity_bits: u64,
    pub position: FormationPosition,
    pub slot_index: u8,
}

impl SerializableFormationSlot {
    /// Create from a slot assignment
    pub fn from_assignment(assignment: &FormationSlotAssignment) -> Self {
        Self {
            entity_bits: assignment.entity.to_bits().get(),
            position: assignment.position,
            slot_index: assignment.slot_index,
        }
    }

    /// Convert back to slot assignment with entity mapping
    /// 
    /// The `entity_map` function should map the stored bits to the actual entity in the world
    pub fn to_assignment<F>(&self, entity_map: F) -> Option<FormationSlotAssignment>
    where
        F: FnOnce(u64) -> Option<Entity>,
    {
        entity_map(self.entity_bits).map(|entity| FormationSlotAssignment {
            entity,
            position: self.position,
            slot_index: self.slot_index,
        })
    }
}

/// Formation layout preset
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[derive(Default)]
pub enum FormationLayout {
    /// 2 front, 2 back
    #[default]
    Balanced,
    /// 3 front, 1 back
    Aggressive,
    /// 1 front, 3 back
    Defensive,
    /// User-defined
    Custom,
}

impl FormationLayout {
    /// Get default slot assignments for this layout
    pub fn slots(&self, party: &[Entity]) -> Vec<FormationSlotAssignment> {
        let positions = match self {
            FormationLayout::Balanced => vec![
                (FormationPosition::FrontRow, 0),
                (FormationPosition::FrontRow, 1),
                (FormationPosition::BackRow, 0),
                (FormationPosition::BackRow, 1),
            ],
            FormationLayout::Aggressive => vec![
                (FormationPosition::FrontRow, 0),
                (FormationPosition::FrontRow, 1),
                (FormationPosition::FrontRow, 2),
                (FormationPosition::BackRow, 0),
            ],
            FormationLayout::Defensive => vec![
                (FormationPosition::FrontRow, 0),
                (FormationPosition::BackRow, 0),
                (FormationPosition::BackRow, 1),
                (FormationPosition::BackRow, 2),
            ],
            FormationLayout::Custom => return Vec::new(), // Must be set manually
        };

        party
            .iter()
            .enumerate()
            .filter_map(|(i, &entity)| {
                positions.get(i).map(|&(pos, slot)| FormationSlotAssignment {
                    entity,
                    position: pos,
                    slot_index: slot,
                })
            })
            .collect()
    }

    /// Get the number of front row slots for this layout
    pub fn front_row_count(&self) -> usize {
        match self {
            FormationLayout::Balanced => 2,
            FormationLayout::Aggressive => 3,
            FormationLayout::Defensive => 1,
            FormationLayout::Custom => 0, // Variable
        }
    }

    /// Get the number of back row slots for this layout
    pub fn back_row_count(&self) -> usize {
        match self {
            FormationLayout::Balanced => 2,
            FormationLayout::Aggressive => 1,
            FormationLayout::Defensive => 3,
            FormationLayout::Custom => 0, // Variable
        }
    }

    /// Description for UI
    pub fn description(&self) -> &'static str {
        match self {
            FormationLayout::Balanced => "Balanced formation with equal front and back",
            FormationLayout::Aggressive => "Offensive formation focusing on damage",
            FormationLayout::Defensive => "Defensive formation minimizing damage taken",
            FormationLayout::Custom => "Custom user-defined formation",
        }
    }

    /// Short name for UI
    pub fn name(&self) -> &'static str {
        match self {
            FormationLayout::Balanced => "Balanced",
            FormationLayout::Aggressive => "Aggressive",
            FormationLayout::Defensive => "Defensive",
            FormationLayout::Custom => "Custom",
        }
    }

    /// Icon for UI
    pub fn icon(&self) -> &'static str {
        match self {
            FormationLayout::Balanced => "⚖️",
            FormationLayout::Aggressive => "⚔️",
            FormationLayout::Defensive => "🛡️",
            FormationLayout::Custom => "⚙️",
        }
    }
}


/// Formation layout for a party
/// 
/// Note: This struct contains Entity references and cannot be directly serialized.
/// Use `SerializableFormation` for persistence.
#[derive(Debug, Clone)]
pub struct Formation {
    pub slots: Vec<FormationSlotAssignment>,
    pub default_layout: FormationLayout,
}

impl Formation {
    /// Create a new formation from a layout preset
    pub fn from_layout(layout: FormationLayout, party: &[Entity]) -> Self {
        Self {
            slots: layout.slots(party),
            default_layout: layout,
        }
    }

    /// Create a new empty custom formation
    pub fn new_custom() -> Self {
        Self {
            slots: Vec::new(),
            default_layout: FormationLayout::Custom,
        }
    }

    /// Find slot assignment for an entity
    pub fn find_slot(&self, entity: Entity) -> Option<&FormationSlotAssignment> {
        self.slots.iter().find(|s| s.entity == entity)
    }

    /// Find mutable slot assignment for an entity
    pub fn find_slot_mut(&mut self, entity: Entity) -> Option<&mut FormationSlotAssignment> {
        self.slots.iter_mut().find(|s| s.entity == entity)
    }

    /// Get all entities in a specific position
    pub fn entities_in_position(&self, position: FormationPosition) -> Vec<Entity> {
        self.slots
            .iter()
            .filter(|s| s.position == position)
            .map(|s| s.entity)
            .collect()
    }

    /// Get all entities in the front row
    pub fn front_row_entities(&self) -> Vec<Entity> {
        self.entities_in_position(FormationPosition::FrontRow)
    }

    /// Get all entities in the back row
    pub fn back_row_entities(&self) -> Vec<Entity> {
        self.entities_in_position(FormationPosition::BackRow)
    }

    /// Check if an entity is in this formation
    pub fn contains(&self, entity: Entity) -> bool {
        self.slots.iter().any(|s| s.entity == entity)
    }

    /// Get the number of entities in the formation
    pub fn len(&self) -> usize {
        self.slots.len()
    }

    /// Check if the formation is empty
    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }

    /// Add or update an entity's position
    pub fn assign(&mut self, entity: Entity, position: FormationPosition, slot_index: u8) {
        let slot_index = slot_index.min(3);

        // Remove any existing assignment for this entity
        self.slots.retain(|s| s.entity != entity);

        // Remove any existing occupant of this slot
        self.slots.retain(|s| !(s.position == position && s.slot_index == slot_index));

        // Add new assignment
        self.slots.push(FormationSlotAssignment {
            entity,
            position,
            slot_index,
        });
    }

    /// Remove an entity from the formation
    pub fn remove(&mut self, entity: Entity) {
        self.slots.retain(|s| s.entity != entity);
    }

    /// Swap positions of two entities
    pub fn swap_positions(&mut self, entity1: Entity, entity2: Entity) -> Result<(), FormationError> {
        let slot1 = self
            .find_slot(entity1)
            .ok_or(FormationError::EntityNotInParty)?
            .clone();
        let slot2 = self
            .find_slot(entity2)
            .ok_or(FormationError::EntityNotInParty)?
            .clone();

        // Update positions
        if let Some(s1) = self.find_slot_mut(entity1) {
            s1.position = slot2.position;
            s1.slot_index = slot2.slot_index;
        }
        if let Some(s2) = self.find_slot_mut(entity2) {
            s2.position = slot1.position;
            s2.slot_index = slot1.slot_index;
        }

        Ok(())
    }

    /// Move an entity to a specific slot
    pub fn move_to_slot(
        &mut self,
        entity: Entity,
        position: FormationPosition,
        slot_index: u8,
    ) -> Result<(), FormationError> {
        if !self.contains(entity) {
            return Err(FormationError::EntityNotInParty);
        }

        let slot_index = slot_index.min(3);

        // Check if slot is occupied by someone else
        if let Some(_occupant) = self
            .slots
            .iter()
            .find(|s| s.position == position && s.slot_index == slot_index && s.entity != entity)
        {
            return Err(FormationError::SlotOccupied);
        }

        // Update entity's position
        if let Some(slot) = self.find_slot_mut(entity) {
            slot.position = position;
            slot.slot_index = slot_index;
        }

        Ok(())
    }

    /// Apply a layout preset to this formation
    pub fn apply_layout(&mut self, layout: FormationLayout, party: &[Entity]) {
        self.default_layout = layout;
        if layout != FormationLayout::Custom {
            self.slots = layout.slots(party);
        }
    }

    /// Get all entities in a specific row
    pub fn get_row(&self, position: FormationPosition) -> Vec<Entity> {
        self.entities_in_position(position)
    }

    /// Clear all slot assignments
    pub fn clear(&mut self) {
        self.slots.clear();
    }

    /// Validate the formation (check for conflicts)
    pub fn validate(&self) -> Vec<FormationError> {
        let mut errors = Vec::new();
        let mut seen_slots = std::collections::HashSet::new();

        for slot in &self.slots {
            let key = (slot.position, slot.slot_index);
            if seen_slots.contains(&key) {
                errors.push(FormationError::SlotOccupied);
            }
            seen_slots.insert(key);
        }

        errors
    }
}

impl Default for Formation {
    fn default() -> Self {
        Self {
            slots: Vec::new(),
            default_layout: FormationLayout::Balanced,
        }
    }
}

/// Serializable formation for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableFormation {
    pub slots: Vec<SerializableFormationSlot>,
    pub default_layout: FormationLayout,
}

impl SerializableFormation {
    /// Create a serializable formation from a Formation
    pub fn from_formation(formation: &Formation) -> Self {
        Self {
            slots: formation.slots.iter().map(SerializableFormationSlot::from_assignment).collect(),
            default_layout: formation.default_layout,
        }
    }

    /// Convert back to a Formation using an entity mapping function
    /// 
    /// The `entity_map` function should convert stored entity bits to actual entities
    pub fn to_formation<F>(self, mut entity_map: F) -> Formation
    where
        F: FnMut(u64) -> Option<Entity>,
    {
        let slots = self.slots
            .into_iter()
            .filter_map(|s| s.to_assignment(&mut entity_map))
            .collect();
        
        Formation {
            slots,
            default_layout: self.default_layout,
        }
    }

    /// Convert back to a Formation with a simple bits-to-entity mapping
    /// 
    /// Note: This assumes entity bits can be directly converted.
    /// In practice, entity mappings may need to be more complex across save/load cycles.
    pub fn to_formation_simple(self) -> Formation {
        // Entity::from_bits returns Option<Entity>
        // We only proceed if bits are non-zero
        self.to_formation(|bits| {
            if bits == 0 {
                None
            } else {
                Entity::from_bits(bits)
            }
        })
    }
}

/// Formation error types
#[derive(Debug, Error, Clone, PartialEq)]
pub enum FormationError {
    #[error("Slot already occupied")]
    SlotOccupied,

    #[error("Entity not in party")]
    EntityNotInParty,

    #[error("Invalid position")]
    InvalidPosition,

    #[error("Formation not found")]
    FormationNotFound,
}

/// Formation system manager
#[derive(Debug, Clone)]
pub struct FormationSystem {
    formations: HashMap<Entity, Formation>, // Party leader -> formation
}

impl FormationSystem {
    /// Create a new formation system
    pub fn new() -> Self {
        Self {
            formations: HashMap::new(),
        }
    }

    /// Set formation for a party
    pub fn set_formation(&mut self, party_leader: Entity, formation: Formation) {
        self.formations.insert(party_leader, formation);
    }

    /// Get formation for a party
    pub fn get_formation(&self, party_leader: Entity) -> Option<&Formation> {
        self.formations.get(&party_leader)
    }

    /// Get mutable formation for a party
    pub fn get_formation_mut(&mut self, party_leader: Entity) -> Option<&mut Formation> {
        self.formations.get_mut(&party_leader)
    }

    /// Remove formation for a party
    pub fn remove_formation(&mut self, party_leader: Entity) -> Option<Formation> {
        self.formations.remove(&party_leader)
    }

    /// Check if a party has a formation
    pub fn has_formation(&self, party_leader: Entity) -> bool {
        self.formations.contains_key(&party_leader)
    }

    /// Get position of entity in formation (searches all formations)
    pub fn get_position(&self, entity: Entity) -> Option<FormationPosition> {
        for formation in self.formations.values() {
            if let Some(slot) = formation.find_slot(entity) {
                return Some(slot.position);
            }
        }
        None
    }

    /// Get formation slot for an entity (searches all formations)
    pub fn get_slot(&self, entity: Entity) -> Option<&FormationSlotAssignment> {
        for formation in self.formations.values() {
            if let Some(slot) = formation.find_slot(entity) {
                return Some(slot);
            }
        }
        None
    }

    /// Find which party leader's formation contains this entity
    pub fn find_party_leader(&self, entity: Entity) -> Option<Entity> {
        for (&leader, formation) in &self.formations {
            if formation.contains(entity) {
                return Some(leader);
            }
        }
        None
    }

    /// Assign entity to position
    pub fn assign_position(
        &mut self,
        party_leader: Entity,
        entity: Entity,
        position: FormationPosition,
        slot_index: u8,
    ) -> Result<(), FormationError> {
        let formation = self
            .formations
            .get_mut(&party_leader)
            .ok_or(FormationError::FormationNotFound)?;

        formation.move_to_slot(entity, position, slot_index)
    }

    /// Swap positions of two entities
    pub fn swap_positions(&mut self, entity1: Entity, entity2: Entity) -> Result<(), FormationError> {
        // Find which formations contain these entities
        let leader1 = self.find_party_leader(entity1);
        let leader2 = self.find_party_leader(entity2);

        match (leader1, leader2) {
            (Some(leader), None) | (None, Some(leader)) => {
                // Both in same formation
                let formation = self
                    .formations
                    .get_mut(&leader)
                    .ok_or(FormationError::FormationNotFound)?;
                formation.swap_positions(entity1, entity2)
            }
            (Some(leader1), Some(leader2)) if leader1 == leader2 => {
                // Same formation
                let formation = self
                    .formations
                    .get_mut(&leader1)
                    .ok_or(FormationError::FormationNotFound)?;
                formation.swap_positions(entity1, entity2)
            }
            _ => Err(FormationError::EntityNotInParty),
        }
    }

    /// Get all entities in a row
    pub fn get_row(&self, party_leader: Entity, row: FormationPosition) -> Vec<Entity> {
        self.formations
            .get(&party_leader)
            .map(|f| f.get_row(row))
            .unwrap_or_default()
    }

    /// Apply layout preset to a party
    pub fn apply_layout(
        &mut self,
        party_leader: Entity,
        layout: FormationLayout,
        party: &[Entity],
    ) -> Result<(), FormationError> {
        let formation = self
            .formations
            .get_mut(&party_leader)
            .ok_or(FormationError::FormationNotFound)?;

        formation.apply_layout(layout, party);
        Ok(())
    }

    /// Create and set a new formation from a layout
    pub fn create_formation(
        &mut self,
        party_leader: Entity,
        layout: FormationLayout,
        party: &[Entity],
    ) -> &Formation {
        let formation = Formation::from_layout(layout, party);
        self.formations.insert(party_leader, formation);
        self.formations.get(&party_leader).unwrap()
    }

    /// Clear all formations
    pub fn clear(&mut self) {
        self.formations.clear();
    }

    /// Get the number of formations
    pub fn len(&self) -> usize {
        self.formations.len()
    }

    /// Check if there are no formations
    pub fn is_empty(&self) -> bool {
        self.formations.is_empty()
    }

    /// Get all party leaders
    pub fn party_leaders(&self) -> impl Iterator<Item = &Entity> {
        self.formations.keys()
    }

    /// Get formation modifiers for an entity
    pub fn get_modifiers(&self, entity: Entity) -> FormationModifiers {
        self.get_slot(entity)
            .map(|slot| slot.position.modifiers())
            .unwrap_or_default()
    }

    /// Update ECS components for all entities in a formation
    pub fn sync_to_world(&self, party_leader: Entity, world: &mut World) -> Result<(), FormationError> {
        let formation = self
            .get_formation(party_leader)
            .ok_or(FormationError::FormationNotFound)?;

        for slot in &formation.slots {
            let component = FormationSlot::new(slot.position, slot.slot_index);
            
            // Try to update existing component or insert new one
            if let Ok(existing) = world.query_one_mut::<&mut FormationSlot>(slot.entity) {
                *existing = component;
            } else {
                let _ = world.insert(slot.entity, (component,));
            }
        }

        Ok(())
    }

    /// Build formation from existing ECS components
    pub fn sync_from_world(
        &mut self,
        party_leader: Entity,
        party: &[Entity],
        world: &World,
    ) -> Result<(), FormationError> {
        let mut slots = Vec::new();

        for &entity in party {
            if let Ok(mut query) = world.query_one::<&FormationSlot>(entity) {
                if let Some(slot) = query.get() {
                    slots.push(FormationSlotAssignment {
                        entity,
                        position: slot.position,
                        slot_index: slot.slot_index,
                    });
                }
            }
        }

        let formation = Formation {
            slots,
            default_layout: FormationLayout::Custom,
        };

        self.set_formation(party_leader, formation);
        Ok(())
    }
}

impl Default for FormationSystem {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper functions for damage calculation integration
pub mod damage_integration {
    use super::*;
    use dde_core::World;

    /// Calculate damage with formation modifiers
    pub fn calculate_damage_with_formation(
        world: &World,
        base_damage: f32,
        attacker: Entity,
        defender: Entity,
    ) -> f32 {
        let mut damage = base_damage;

        // Apply attacker formation bonus
        if let Ok(mut query) = world.query_one::<&FormationSlot>(attacker) {
            if let Some(slot) = query.get() {
                let mods = slot.modifiers();
                damage *= mods.damage_dealt_mult;
            }
        }

        // Apply defender formation reduction
        if let Ok(mut query) = world.query_one::<&FormationSlot>(defender) {
            if let Some(slot) = query.get() {
                let mods = slot.modifiers();
                damage *= mods.damage_taken_mult;
            }
        }

        damage.max(1.0)
    }

    /// Apply formation damage dealt modifier only
    pub fn apply_attacker_formation(world: &World, base_damage: f32, attacker: Entity) -> f32 {
        if let Ok(mut query) = world.query_one::<&FormationSlot>(attacker) {
            if let Some(slot) = query.get() {
                return (base_damage * slot.modifiers().damage_dealt_mult).max(1.0);
            }
        }
        base_damage
    }

    /// Apply formation damage taken modifier only
    pub fn apply_defender_formation(world: &World, base_damage: f32, defender: Entity) -> f32 {
        if let Ok(mut query) = world.query_one::<&FormationSlot>(defender) {
            if let Some(slot) = query.get() {
                return (base_damage * slot.modifiers().damage_taken_mult).max(1.0);
            }
        }
        base_damage
    }

    /// Get accuracy with formation modifier
    pub fn get_accuracy_with_formation(
        world: &World,
        base_accuracy: f32,
        attacker: Entity,
        is_physical: bool,
    ) -> f32 {
        if let Ok(mut query) = world.query_one::<&FormationSlot>(attacker) {
            if let Some(slot) = query.get() {
                let mods = slot.modifiers();
                if is_physical {
                    return base_accuracy * mods.physical_accuracy_mult;
                } else {
                    return base_accuracy * mods.magic_accuracy_mult;
                }
            }
        }
        base_accuracy
    }

    /// Get ATB speed with formation modifier
    pub fn get_atb_speed_with_formation(world: &World, base_speed: f32, entity: Entity) -> f32 {
        if let Ok(mut query) = world.query_one::<&FormationSlot>(entity) {
            if let Some(slot) = query.get() {
                return base_speed * slot.modifiers().atb_speed_mult;
            }
        }
        base_speed
    }
}

/// UI-related types and helpers (requires egui feature)
#[cfg(feature = "ui")]
pub mod ui {
    use super::*;

    /// Response from drawing a formation slot
    #[derive(Debug, Clone, Copy, Default, PartialEq)]
    pub struct SlotResponse {
        /// Slot was clicked
        pub clicked: bool,
        /// Slot is being dragged
        pub dragged: bool,
        /// Something was dropped on this slot
        pub dropped: bool,
        /// Mouse is hovering over slot
        pub hovered: bool,
    }

    impl SlotResponse {
        /// Check if any interaction occurred
        pub fn has_interaction(&self) -> bool {
            self.clicked || self.dragged || self.dropped || self.hovered
        }
    }

    /// Formation editor UI component
    #[derive(Debug, Clone)]
    pub struct FormationEditor {
        pub selected_layout: FormationLayout,
        pub dragging: Option<Entity>,
        pub hovered_slot: Option<(FormationPosition, u8)>,
    }

    impl FormationEditor {
        /// Create a new formation editor
        pub fn new() -> Self {
            Self {
                selected_layout: FormationLayout::Balanced,
                dragging: None,
                hovered_slot: None,
            }
        }

        /// Create with a specific default layout
        pub fn with_layout(layout: FormationLayout) -> Self {
            Self {
                selected_layout: layout,
                dragging: None,
                hovered_slot: None,
            }
        }

        /// Draw the formation grid
        pub fn draw(
            &mut self,
            ui: &mut egui::Ui,
            formation: &mut Formation,
            entity_names: &std::collections::HashMap<Entity, String>,
        ) -> Vec<(Entity, FormationPosition, u8)> {
            let mut changes = Vec::new();

            ui.vertical(|ui| {
                // Draw layout selector
                self.draw_layout_selector(ui, formation);

                ui.separator();

                // Draw back row (top)
                ui.label("Back Row:");
                ui.horizontal(|ui| {
                    for i in 0..=3 {
                        let response = self.draw_slot(
                            ui,
                            formation,
                            FormationPosition::BackRow,
                            i,
                            entity_names,
                        );
                        if response.dropped && self.dragging.is_some() {
                            changes.push((self.dragging.unwrap(), FormationPosition::BackRow, i));
                            self.dragging = None;
                        }
                    }
                });

                ui.add_space(16.0);

                // Draw front row (bottom)
                ui.label("Front Row:");
                ui.horizontal(|ui| {
                    for i in 0..=3 {
                        let response = self.draw_slot(
                            ui,
                            formation,
                            FormationPosition::FrontRow,
                            i,
                            entity_names,
                        );
                        if response.dropped && self.dragging.is_some() {
                            changes.push((self.dragging.unwrap(), FormationPosition::FrontRow, i));
                            self.dragging = None;
                        }
                    }
                });

                ui.separator();

                // Draw legend
                Self::draw_legend(ui);
            });

            changes
        }

        /// Draw a single formation slot
        fn draw_slot(
            &mut self,
            ui: &mut egui::Ui,
            formation: &Formation,
            position: FormationPosition,
            slot_index: u8,
            entity_names: &std::collections::HashMap<Entity, String>,
        ) -> SlotResponse {
            let slot_size = egui::vec2(80.0, 60.0);
            let (rect, response) = ui.allocate_exact_size(slot_size, egui::Sense::click_and_drag());

            let entity = formation
                .slots
                .iter()
                .find(|s| s.position == position && s.slot_index == slot_index)
                .map(|s| s.entity);

            let is_hovered = response.hovered();
            let is_dragged = response.dragged();
            let is_dropped = response.dropped();
            let is_clicked = response.clicked();

            // Draw slot background
            let bg_color = if entity.is_some() {
                if self.dragging == entity {
                    ui.visuals().widgets.inactive.bg_fill
                } else {
                    ui.visuals().widgets.active.bg_fill
                }
            } else {
                ui.visuals().widgets.noninteractive.bg_fill
            };

            ui.painter().rect_filled(rect, 4.0, bg_color);
            ui.painter().rect_stroke(
                rect,
                4.0,
                egui::Stroke::new(1.0, ui.visuals().widgets.noninteractive.fg_stroke.color),
            );

            // Draw slot content
            if let Some(e) = entity {
                let name = entity_names.get(&e).map(|s| s.as_str()).unwrap_or("???");
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    format!("{}\n{}", position.icon(), name),
                    egui::FontId::proportional(12.0),
                    ui.visuals().text_color(),
                );

                // Handle drag start
                if is_dragged && self.dragging.is_none() {
                    self.dragging = Some(e);
                }
            } else {
                // Draw empty slot indicator
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    if is_hovered { "+" } else { "·" },
                    egui::FontId::proportional(20.0),
                    ui.visuals().weak_text_color(),
                );
            }

            SlotResponse {
                clicked: is_clicked,
                dragged: is_dragged,
                dropped: is_dropped,
                hovered: is_hovered,
            }
        }

        /// Draw layout selector
        fn draw_layout_selector(&mut self, ui: &mut egui::Ui, formation: &mut Formation) {
            ui.label("Formation Layout:");
            ui.horizontal(|ui| {
                for layout in [
                    FormationLayout::Balanced,
                    FormationLayout::Aggressive,
                    FormationLayout::Defensive,
                ] {
                    let selected = formation.default_layout == layout;
                    let button = egui::Button::new(format!("{} {}", layout.icon(), layout.name()))
                        .selected(selected);

                    if ui.add(button).on_hover_text(layout.description()).clicked() {
                        self.selected_layout = layout;
                    }
                }
            });
        }

        /// Draw formation modifiers legend
        pub fn draw_legend(ui: &mut egui::Ui) {
            ui.label("Position Effects:");
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(format!("{} Front Row:", FormationPosition::FrontRow.icon()));
                    ui.label("  +10% damage dealt");
                    ui.label("  +20% damage taken");
                });
                ui.vertical(|ui| {
                    ui.label(format!("{} Back Row:", FormationPosition::BackRow.icon()));
                    ui.label("  -15% damage dealt");
                    ui.label("  -25% damage taken");
                    ui.label("  -20% physical accuracy");
                });
            });
        }

        /// Check if currently dragging an entity
        pub fn is_dragging(&self) -> bool {
            self.dragging.is_some()
        }

        /// Get the entity being dragged
        pub fn dragged_make_entity(&self) -> Option<Entity> {
            self.dragging
        }

        /// Cancel current drag operation
        pub fn cancel_drag(&mut self) {
            self.dragging = None;
        }
    }

    impl Default for FormationEditor {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create a test entity from bits
    /// Create test entities in a world
    fn test_entities(world: &mut World) -> Vec<Entity> {
        vec![
            world.spawn(()),
            world.spawn(()),
            world.spawn(()),
            world.spawn(()),
        ]
    }

    #[test]
    fn test_formation_position_modifiers() {
        let front = FormationPosition::FrontRow.modifiers();
        assert_eq!(front.damage_dealt_mult, 1.1);
        assert_eq!(front.damage_taken_mult, 1.2);
        assert_eq!(front.physical_accuracy_mult, 1.0);

        let back = FormationPosition::BackRow.modifiers();
        assert_eq!(back.damage_dealt_mult, 0.85);
        assert_eq!(back.damage_taken_mult, 0.75);
        assert_eq!(back.physical_accuracy_mult, 0.8);
    }

    #[test]
    fn test_formation_position_ui() {
        assert!(!FormationPosition::FrontRow.description().is_empty());
        assert!(!FormationPosition::BackRow.description().is_empty());
        assert!(!FormationPosition::FrontRow.icon().is_empty());
        assert!(!FormationPosition::BackRow.icon().is_empty());
    }

    #[test]
    fn test_formation_layout_slots() {
        let mut world = World::new();
        let entities = test_entities(&mut world);

        // Test balanced layout
        let balanced = FormationLayout::Balanced.slots(&entities);
        assert_eq!(balanced.len(), 4);
        assert_eq!(balanced[0].position, FormationPosition::FrontRow);
        assert_eq!(balanced[1].position, FormationPosition::FrontRow);
        assert_eq!(balanced[2].position, FormationPosition::BackRow);
        assert_eq!(balanced[3].position, FormationPosition::BackRow);

        // Test aggressive layout
        let aggressive = FormationLayout::Aggressive.slots(&entities);
        assert_eq!(aggressive.len(), 4);
        assert_eq!(aggressive[0].position, FormationPosition::FrontRow);
        assert_eq!(aggressive[1].position, FormationPosition::FrontRow);
        assert_eq!(aggressive[2].position, FormationPosition::FrontRow);
        assert_eq!(aggressive[3].position, FormationPosition::BackRow);

        // Test defensive layout
        let defensive = FormationLayout::Defensive.slots(&entities);
        assert_eq!(defensive.len(), 4);
        assert_eq!(defensive[0].position, FormationPosition::FrontRow);
        assert_eq!(defensive[1].position, FormationPosition::BackRow);
        assert_eq!(defensive[2].position, FormationPosition::BackRow);
        assert_eq!(defensive[3].position, FormationPosition::BackRow);
    }

    #[test]
    fn test_formation_layout_counts() {
        assert_eq!(FormationLayout::Balanced.front_row_count(), 2);
        assert_eq!(FormationLayout::Balanced.back_row_count(), 2);
        assert_eq!(FormationLayout::Aggressive.front_row_count(), 3);
        assert_eq!(FormationLayout::Aggressive.back_row_count(), 1);
        assert_eq!(FormationLayout::Defensive.front_row_count(), 1);
        assert_eq!(FormationLayout::Defensive.back_row_count(), 3);
    }

    #[test]
    fn test_formation_from_layout() {
        let mut world = World::new();
        let entities = test_entities(&mut world);
        let formation = Formation::from_layout(FormationLayout::Aggressive, &entities);

        assert_eq!(formation.slots.len(), 4);
        assert_eq!(formation.default_layout, FormationLayout::Aggressive);
        assert_eq!(formation.front_row_entities().len(), 3);
        assert_eq!(formation.back_row_entities().len(), 1);
    }

    #[test]
    fn test_formation_find_slot() {
        let mut world = World::new();
        let entities = test_entities(&mut world);
        let formation = Formation::from_layout(FormationLayout::Balanced, &entities);

        assert!(formation.find_slot(entities[0]).is_some());
        assert!(formation.find_slot(world.spawn(())).is_none());
    }

    #[test]
    fn test_formation_contains() {
        let mut world = World::new();
        let entities = test_entities(&mut world);
        let formation = Formation::from_layout(FormationLayout::Balanced, &entities);

        assert!(formation.contains(entities[0]));
        assert!(!formation.contains(world.spawn(())));
    }

    #[test]
    fn test_formation_swap_positions() {
        let mut world = World::new();
        let entities = test_entities(&mut world);
        let mut formation = Formation::from_layout(FormationLayout::Balanced, &entities);

        // Get initial positions
        let slot0 = formation.find_slot(entities[0]).unwrap().clone();
        let slot2 = formation.find_slot(entities[2]).unwrap().clone();

        // Swap entities[0] (front) with entities[2] (back)
        formation.swap_positions(entities[0], entities[2]).unwrap();

        // Check positions swapped
        assert_eq!(
            formation.find_slot(entities[0]).unwrap().position,
            slot2.position
        );
        assert_eq!(
            formation.find_slot(entities[2]).unwrap().position,
            slot0.position
        );
    }

    #[test]
    fn test_formation_swap_positions_error() {
        let mut world = World::new();
        let entities = test_entities(&mut world);
        let mut formation = Formation::from_layout(FormationLayout::Balanced, &entities);

        // Try to swap with entity not in party
        let result = formation.swap_positions(entities[0], world.spawn(()));
        assert!(matches!(result, Err(FormationError::EntityNotInParty)));
    }

    #[test]
    fn test_formation_move_to_slot() {
        let mut world = World::new();
        let entities = test_entities(&mut world);
        let mut formation = Formation::from_layout(FormationLayout::Balanced, &entities);

        // Move entity from front to back
        formation
            .move_to_slot(entities[0], FormationPosition::BackRow, 2)
            .unwrap();

        assert_eq!(
            formation.find_slot(entities[0]).unwrap().position,
            FormationPosition::BackRow
        );
        assert_eq!(formation.find_slot(entities[0]).unwrap().slot_index, 2);
    }

    #[test]
    fn test_formation_move_to_slot_occupied() {
        let mut world = World::new();
        let entities = test_entities(&mut world);
        let mut formation = Formation::from_layout(FormationLayout::Balanced, &entities);

        // entities[0] and entities[1] are both in front row
        // Try to move entities[0] to entities[1]'s slot
        let slot1 = formation.find_slot(entities[1]).unwrap();
        let result = formation.move_to_slot(entities[0], slot1.position, slot1.slot_index);

        assert!(matches!(result, Err(FormationError::SlotOccupied)));
    }

    #[test]
    fn test_formation_system_new() {
        let system = FormationSystem::new();
        assert!(system.is_empty());
    }

    #[test]
    fn test_formation_system_set_and_get() {
        let mut world = World::new();
        let mut system = FormationSystem::new();
        let leader = world.spawn(());
        let entities = test_entities(&mut world);
        let formation = Formation::from_layout(FormationLayout::Defensive, &entities);

        system.set_formation(leader, formation);

        assert!(system.has_formation(leader));
        assert_eq!(system.len(), 1);

        let retrieved = system.get_formation(leader).unwrap();
        assert_eq!(retrieved.default_layout, FormationLayout::Defensive);
    }

    #[test]
    fn test_formation_system_get_position() {
        let mut world = World::new();
        let mut system = FormationSystem::new();
        let leader = world.spawn(());
        let entities = test_entities(&mut world);

        system.create_formation(leader, FormationLayout::Aggressive, &entities);

        // entities[0] should be in front row
        let pos = system.get_position(entities[0]).unwrap();
        assert_eq!(pos, FormationPosition::FrontRow);

        // entities[3] should be in back row
        let pos = system.get_position(entities[3]).unwrap();
        assert_eq!(pos, FormationPosition::BackRow);

        // Unknown entity should return None
        assert!(system.get_position(world.spawn(())).is_none());
    }

    #[test]
    fn test_formation_system_find_party_leader() {
        let mut world = World::new();
        let mut system = FormationSystem::new();
        let leader = world.spawn(());
        let entities = test_entities(&mut world);

        system.create_formation(leader, FormationLayout::Balanced, &entities);

        assert_eq!(system.find_party_leader(entities[0]), Some(leader));
        assert_eq!(system.find_party_leader(world.spawn(())), None);
    }

    #[test]
    fn test_formation_system_swap_positions() {
        let mut world = World::new();
        let mut system = FormationSystem::new();
        let leader = world.spawn(());
        let entities = test_entities(&mut world);

        system.create_formation(leader, FormationLayout::Balanced, &entities);

        // Swap positions
        system.swap_positions(entities[0], entities[2]).unwrap();

        // Check in the formation
        let formation = system.get_formation(leader).unwrap();
        assert_eq!(
            formation.find_slot(entities[0]).unwrap().position,
            FormationPosition::BackRow
        );
        assert_eq!(
            formation.find_slot(entities[2]).unwrap().position,
            FormationPosition::FrontRow
        );
    }

    #[test]
    fn test_formation_system_get_row() {
        let mut world = World::new();
        let mut system = FormationSystem::new();
        let leader = world.spawn(());
        let entities = test_entities(&mut world);

        system.create_formation(leader, FormationLayout::Aggressive, &entities);

        let front = system.get_row(leader, FormationPosition::FrontRow);
        assert_eq!(front.len(), 3);

        let back = system.get_row(leader, FormationPosition::BackRow);
        assert_eq!(back.len(), 1);
    }

    #[test]
    fn test_formation_system_apply_layout() {
        let mut world = World::new();
        let mut system = FormationSystem::new();
        let leader = world.spawn(());
        let entities = test_entities(&mut world);

        system.create_formation(leader, FormationLayout::Balanced, &entities);

        // Apply aggressive layout
        system.apply_layout(leader, FormationLayout::Aggressive, &entities).unwrap();

        let formation = system.get_formation(leader).unwrap();
        assert_eq!(formation.default_layout, FormationLayout::Aggressive);
        assert_eq!(formation.front_row_entities().len(), 3);
    }

    #[test]
    fn test_formation_system_get_modifiers() {
        let mut world = World::new();
        let mut system = FormationSystem::new();
        let leader = world.spawn(());
        let entities = test_entities(&mut world);

        system.create_formation(leader, FormationLayout::Aggressive, &entities);

        let mods = system.get_modifiers(entities[0]);
        assert_eq!(mods.damage_dealt_mult, 1.1);
        assert_eq!(mods.damage_taken_mult, 1.2);
    }

    #[test]
    fn test_formation_system_remove_formation() {
        let mut world = World::new();
        let mut system = FormationSystem::new();
        let leader = world.spawn(());
        let entities = test_entities(&mut world);

        system.create_formation(leader, FormationLayout::Balanced, &entities);
        assert!(system.has_formation(leader));

        system.remove_formation(leader);
        assert!(!system.has_formation(leader));
        assert!(system.is_empty());
    }

    #[test]
    fn test_formation_modifiers_combine() {
        let mods1 = FormationModifiers {
            damage_dealt_mult: 1.5,
            damage_taken_mult: 0.8,
            ..Default::default()
        };
        let mods2 = FormationModifiers {
            damage_dealt_mult: 1.2,
            damage_taken_mult: 0.9,
            ..Default::default()
        };

        let combined = mods1.combine(&mods2);
        assert!((combined.damage_dealt_mult - 1.8).abs() < 0.0001); // 1.5 * 1.2
        assert!((combined.damage_taken_mult - 0.72).abs() < 0.0001); // 0.8 * 0.9
    }

    #[test]
    fn test_formation_modifiers_summary() {
        let mods = FormationPosition::FrontRow.modifiers();
        let summary = mods.summary();
        assert!(!summary.is_empty());

        // Should include damage dealt and damage taken
        let has_damage_dealt = summary.iter().any(|(name, _, _)| *name == "Damage Dealt");
        let has_damage_taken = summary.iter().any(|(name, _, _)| *name == "Damage Taken");
        assert!(has_damage_dealt);
        assert!(has_damage_taken);
    }

    #[test]
    fn test_formation_error_display() {
        assert_eq!(
            FormationError::SlotOccupied.to_string(),
            "Slot already occupied"
        );
        assert_eq!(
            FormationError::EntityNotInParty.to_string(),
            "Entity not in party"
        );
        assert_eq!(
            FormationError::InvalidPosition.to_string(),
            "Invalid position"
        );
        assert_eq!(
            FormationError::FormationNotFound.to_string(),
            "Formation not found"
        );
    }

    #[test]
    fn test_formation_validate() {
        let mut world = World::new();
        let entities = test_entities(&mut world);
        let mut formation = Formation::from_layout(FormationLayout::Balanced, &entities);

        // Should be valid initially
        assert!(formation.validate().is_empty());

        // Manually create a conflict by adding duplicate slot
        formation.slots.push(FormationSlotAssignment {
            entity: world.spawn(()),
            position: FormationPosition::FrontRow,
            slot_index: 0, // Same as entities[0]
        });

        // Should have an error now
        let errors = formation.validate();
        assert!(!errors.is_empty());
        assert!(errors.contains(&FormationError::SlotOccupied));
    }

    #[test]
    fn test_formation_slot_helpers() {
        let front_slot = FormationSlot::new(FormationPosition::FrontRow, 0);
        assert!(front_slot.is_front_row());
        assert!(!front_slot.is_back_row());

        let back_slot = FormationSlot::new(FormationPosition::BackRow, 1);
        assert!(!back_slot.is_front_row());
        assert!(back_slot.is_back_row());

        assert_eq!(back_slot.slot_index, 1);
    }

    #[test]
    fn test_formation_slot_index_clamping() {
        let slot = FormationSlot::new(FormationPosition::FrontRow, 10);
        assert_eq!(slot.slot_index, 3); // Should be clamped to 3

        let slot = FormationSlot::new(FormationPosition::BackRow, 255);
        assert_eq!(slot.slot_index, 3); // Should be clamped to 3
    }

    #[test]
    fn test_formation_layout_ui() {
        assert!(!FormationLayout::Balanced.description().is_empty());
        assert!(!FormationLayout::Balanced.name().is_empty());
        assert!(!FormationLayout::Balanced.icon().is_empty());
    }

    #[test]
    fn test_formation_assign() {
        let mut world = World::new();
        let entities = test_entities(&mut world);
        let mut formation = Formation::new_custom();

        formation.assign(entities[0], FormationPosition::FrontRow, 0);
        assert_eq!(formation.len(), 1);
        assert!(formation.contains(entities[0]));

        // Re-assign same entity to different position
        formation.assign(entities[0], FormationPosition::BackRow, 1);
        assert_eq!(formation.len(), 1);
        assert_eq!(
            formation.find_slot(entities[0]).unwrap().position,
            FormationPosition::BackRow
        );
    }

    #[test]
    fn test_formation_remove() {
        let mut world = World::new();
        let entities = test_entities(&mut world);
        let mut formation = Formation::from_layout(FormationLayout::Balanced, &entities);

        assert_eq!(formation.len(), 4);
        formation.remove(entities[0]);
        assert_eq!(formation.len(), 3);
        assert!(!formation.contains(entities[0]));
    }

    #[test]
    fn test_formation_system_clear() {
        let mut world = World::new();
        let mut system = FormationSystem::new();
        let leader1 = world.spawn(());
        let leader2 = world.spawn(());
        let entities = test_entities(&mut world);

        system.create_formation(leader1, FormationLayout::Balanced, &entities);
        system.create_formation(leader2, FormationLayout::Defensive, &entities);

        assert_eq!(system.len(), 2);
        system.clear();
        assert!(system.is_empty());
    }

    #[test]
    fn test_formation_position_row_index() {
        assert_eq!(FormationPosition::FrontRow.row_index(), 0);
        assert_eq!(FormationPosition::BackRow.row_index(), 1);
    }

    #[test]
    fn test_formation_system_party_leaders() {
        let mut world = World::new();
        let mut system = FormationSystem::new();
        let leader1 = world.spawn(());
        let leader2 = world.spawn(());
        let entities = test_entities(&mut world);

        system.create_formation(leader1, FormationLayout::Balanced, &entities);
        system.create_formation(leader2, FormationLayout::Defensive, &entities);

        let leaders: Vec<_> = system.party_leaders().copied().collect();
        assert_eq!(leaders.len(), 2);
        assert!(leaders.contains(&leader1));
        assert!(leaders.contains(&leader2));
    }

    #[test]
    #[cfg(feature = "ui")]
    fn test_slot_response_has_interaction() {
        let response = SlotResponse::default();
        assert!(!response.has_interaction());

        let response = SlotResponse {
            clicked: true,
            ..Default::default()
        };
        assert!(response.has_interaction());

        let response = SlotResponse {
            hovered: true,
            ..Default::default()
        };
        assert!(response.has_interaction());
    }

    #[test]
    fn test_formation_system_error_not_found() {
        let mut world = World::new();
        let mut system = FormationSystem::new();
        let leader = world.spawn(());
        let entity = world.spawn(());

        // Try operations on non-existent formation
        assert!(system.get_formation(leader).is_none());
        assert!(
            matches!(
                system.apply_layout(leader, FormationLayout::Balanced, &[]),
                Err(FormationError::FormationNotFound)
            )
        );
        assert!(
            matches!(
                system.assign_position(leader, entity, FormationPosition::FrontRow, 0),
                Err(FormationError::FormationNotFound)
            )
        );
    }

    #[test]
    fn test_formation_custom_layout_empty() {
        let mut world = World::new();
        let entities = test_entities(&mut world);
        let custom = FormationLayout::Custom.slots(&entities);
        assert!(custom.is_empty());
    }

    #[test]
    fn test_formation_slot_assignment_new() {
        let mut world = World::new();
        let entity = world.spawn(());
        let assignment = FormationSlotAssignment::new(entity, FormationPosition::FrontRow, 2);
        
        assert_eq!(assignment.entity, entity);
        assert_eq!(assignment.position, FormationPosition::FrontRow);
        assert_eq!(assignment.slot_index, 2);
    }

    #[test]
    fn test_formation_slot_assignment_clamping() {
        let mut world = World::new();
        let entity = world.spawn(());
        let assignment = FormationSlotAssignment::new(entity, FormationPosition::BackRow, 255);
        
        assert_eq!(assignment.slot_index, 3);
    }

    #[test]
    fn test_formation_partial_party() {
        let mut world = World::new();
        let entities = vec![world.spawn(()), world.spawn(())];
        let formation = Formation::from_layout(FormationLayout::Balanced, &entities);
        
        // Should only create slots for available entities
        assert_eq!(formation.len(), 2);
    }

    #[test]
    fn test_formation_empty_party() {
        let formation = Formation::from_layout(FormationLayout::Balanced, &[]);
        assert!(formation.is_empty());
    }

    #[test]
    fn test_formation_slot_default() {
        let slot = FormationSlot::default();
        assert_eq!(slot.position, FormationPosition::FrontRow);
        assert_eq!(slot.slot_index, 0);
    }

    #[test]
    fn test_formation_position_default() {
        assert_eq!(FormationPosition::default(), FormationPosition::FrontRow);
    }

    #[test]
    fn test_formation_layout_default() {
        assert_eq!(FormationLayout::default(), FormationLayout::Balanced);
    }

    #[test]
    fn test_formation_system_default() {
        let system = FormationSystem::default();
        assert!(system.is_empty());
    }

    #[test]
    fn test_formation_default() {
        let formation = Formation::default();
        assert!(formation.is_empty());
        assert_eq!(formation.default_layout, FormationLayout::Balanced);
    }

    #[test]
    fn test_formation_modifiers_default() {
        let mods = FormationModifiers::default();
        assert_eq!(mods.damage_dealt_mult, 0.0);
        assert_eq!(mods.damage_taken_mult, 0.0);
        assert_eq!(mods.physical_accuracy_mult, 0.0);
        assert_eq!(mods.magic_accuracy_mult, 0.0);
        assert_eq!(mods.atb_speed_mult, 0.0);
    }

    #[test]
    fn test_formation_modifiers_default_summary() {
        let mods = FormationModifiers::default();
        let summary = mods.summary();
        // Default values (0.0) should still appear in summary
        assert!(!summary.is_empty());
    }

    #[test]
    fn test_formation_clear() {
        let mut world = World::new();
        let entities = test_entities(&mut world);
        let mut formation = Formation::from_layout(FormationLayout::Balanced, &entities);
        
        assert_eq!(formation.len(), 4);
        formation.clear();
        assert!(formation.is_empty());
    }

    #[test]
    fn test_formation_get_row() {
        let mut world = World::new();
        let entities = test_entities(&mut world);
        let formation = Formation::from_layout(FormationLayout::Aggressive, &entities);
        
        let front = formation.get_row(FormationPosition::FrontRow);
        let back = formation.get_row(FormationPosition::BackRow);
        
        assert_eq!(front.len(), 3);
        assert_eq!(back.len(), 1);
    }

    #[test]
    fn test_formation_entities_in_position() {
        let mut world = World::new();
        let entities = test_entities(&mut world);
        let formation = Formation::from_layout(FormationLayout::Defensive, &entities);
        
        let front = formation.entities_in_position(FormationPosition::FrontRow);
        let back = formation.entities_in_position(FormationPosition::BackRow);
        
        assert_eq!(front.len(), 1);
        assert_eq!(back.len(), 3);
    }

    #[test]
    fn test_formation_custom_new() {
        let formation = Formation::new_custom();
        assert!(formation.is_empty());
        assert_eq!(formation.default_layout, FormationLayout::Custom);
    }

    #[test]
    #[cfg(feature = "ui")]
    fn test_slot_response_default() {
        let response = SlotResponse::default();
        assert!(!response.clicked);
        assert!(!response.dragged);
        assert!(!response.dropped);
        assert!(!response.hovered);
    }

    #[test]
    fn test_formation_layout_custom_counts() {
        assert_eq!(FormationLayout::Custom.front_row_count(), 0);
        assert_eq!(FormationLayout::Custom.back_row_count(), 0);
    }

    // Tests for damage_integration module
    #[test]
    fn test_damage_integration_calculate() {
        let mut world = World::new();
        let attacker = world.spawn((FormationSlot::new(FormationPosition::FrontRow, 0),));
        let defender = world.spawn((FormationSlot::new(FormationPosition::BackRow, 0),));

        let damage = damage_integration::calculate_damage_with_formation(&world, 100.0, attacker, defender);
        
        // Front row attacker: 1.1x, Back row defender: 0.75x
        // Expected: 100 * 1.1 * 0.75 = 82.5
        assert_eq!(damage, 82.5);
    }

    #[test]
    fn test_damage_integration_apply_attacker() {
        let mut world = World::new();
        let attacker = world.spawn((FormationSlot::new(FormationPosition::FrontRow, 0),));

        let damage = damage_integration::apply_attacker_formation(&world, 100.0, attacker);
        assert_eq!(damage, 110.0); // 100 * 1.1
    }

    #[test]
    fn test_damage_integration_apply_defender() {
        let mut world = World::new();
        let defender = world.spawn((FormationSlot::new(FormationPosition::BackRow, 0),));

        let damage = damage_integration::apply_defender_formation(&world, 100.0, defender);
        assert_eq!(damage, 75.0); // 100 * 0.75
    }

    #[test]
    fn test_damage_integration_no_formation() {
        let mut world = World::new();
        let entity = world.spawn(());

        // No FormationSlot component, should return base values
        let damage = damage_integration::apply_attacker_formation(&world, 100.0, entity);
        assert_eq!(damage, 100.0);

        let damage = damage_integration::apply_defender_formation(&world, 100.0, entity);
        assert_eq!(damage, 100.0);
    }

    #[test]
    fn test_damage_integration_accuracy_physical() {
        let mut world = World::new();
        let attacker = world.spawn((FormationSlot::new(FormationPosition::BackRow, 0),));

        let accuracy = damage_integration::get_accuracy_with_formation(&world, 1.0, attacker, true);
        assert_eq!(accuracy, 0.8); // Back row physical accuracy is 0.8
    }

    #[test]
    fn test_damage_integration_accuracy_magic() {
        let mut world = World::new();
        let attacker = world.spawn((FormationSlot::new(FormationPosition::BackRow, 0),));

        let accuracy = damage_integration::get_accuracy_with_formation(&world, 1.0, attacker, false);
        assert_eq!(accuracy, 1.0); // Back row magic accuracy is 1.0
    }

    #[test]
    fn test_damage_integration_atb_speed() {
        let mut world = World::new();
        let entity = world.spawn((FormationSlot::new(FormationPosition::BackRow, 0),));

        let speed = damage_integration::get_atb_speed_with_formation(&world, 100.0, entity);
        assert_eq!(speed, 95.0); // Back row ATB speed is 0.95
    }

    #[test]
    fn test_damage_integration_minimum_one() {
        let mut world = World::new();
        let attacker = world.spawn((FormationSlot::new(FormationPosition::BackRow, 0),));

        // Even with very low base damage, result should be at least 1.0
        let damage = damage_integration::apply_attacker_formation(&world, 0.5, attacker);
        assert_eq!(damage, 1.0);
    }

    #[test]
    fn test_formation_system_sync_to_world() {
        let mut world = World::new();
        let mut system = FormationSystem::new();
        let leader = world.spawn(());
        let entities = test_entities(&mut world);

        // Spawn entities in world
        let world_entities: Vec<_> = entities.iter().map(|_| world.spawn(())).collect();

        // Create formation
        system.create_formation(leader, FormationLayout::Balanced, &world_entities);
        
        // Sync to world
        system.sync_to_world(leader, &mut world).unwrap();

        // Verify components were added
        for entity in &world_entities {
            let slot = world.query_one::<&FormationSlot>(*entity).ok().and_then(|mut q| q.get().cloned());
            assert!(slot.is_some());
        }
    }

    #[test]
    fn test_formation_system_sync_from_world() {
        let mut world = World::new();
        let mut system = FormationSystem::new();
        let leader = world.spawn(());

        // Spawn entities with formation slots
        let entity1 = world.spawn((FormationSlot::new(FormationPosition::FrontRow, 0),));
        let entity2 = world.spawn((FormationSlot::new(FormationPosition::BackRow, 1),));
        let party = vec![entity1, entity2];

        // Sync from world
        system.sync_from_world(leader, &party, &world).unwrap();

        // Verify formation was created
        let formation = system.get_formation(leader).unwrap();
        assert_eq!(formation.len(), 2);
        assert_eq!(formation.find_slot(entity1).unwrap().position, FormationPosition::FrontRow);
        assert_eq!(formation.find_slot(entity2).unwrap().position, FormationPosition::BackRow);
    }

    #[test]
    fn test_formation_system_sync_error_not_found() {
        let mut world = World::new();
        let mut system = FormationSystem::new();
        let leader = world.spawn(());
        let mut world = World::new();

        // Sync to non-existent formation should error
        let result = system.sync_to_world(leader, &mut world);
        assert!(matches!(result, Err(FormationError::FormationNotFound)));
    }
}
