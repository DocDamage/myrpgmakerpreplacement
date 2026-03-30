//! Status Effect System for DocDamage Engine
//!
//! Provides comprehensive status effect management including:
//! - Damage over time (Poison, Burn)
//! - Turn control (Freeze, Stun, Sleep)
//! - Healing over time (Regen)
//! - Stat modifiers (Attack/Defense/Speed Up/Down)
//! - ATB modifiers (Haste, Slow)
//! - Status resistance calculations
//! - Visual indicators (icons, colors)
//!
//! # Example Usage
//! ```
//! use dde_battle::status::{StatusEffects, StatusEffect, StatusType};
//! use dde_core::Entity;
//!
//! // Add a poison effect
//! let mut status_effects = StatusEffects::default();
//! status_effects.add(StatusEffect::new(
//!     StatusType::Poison,
//!     5,      // 5 turns
//!     10,     // 10 damage per tick
//!     Some(source_entity)
//! ));
//!
//! // Check for specific status
//! if status_effects.has(StatusType::Poison) {
//!     // Apply poison damage...
//! }
//!
//! // Get stat modifiers
//! let atk_mod = status_effects.atk_modifier(); // e.g., 1.2 for +20%
//! ```

use dde_core::{Entity, World};
use dde_core::components::Stats;
use serde::{Deserialize, Serialize};

/// Status effect types
///
/// Each status effect has different behavior in battle:
/// - DoT effects (Poison, Burn) apply damage each turn
/// - Control effects (Freeze, Stun, Sleep) prevent action
/// - HoT effects (Regen) heal each turn
/// - Buffs (Haste, AttackUp, etc.) improve stats
/// - Debuffs (Slow, AttackDown, etc.) reduce stats
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StatusType {
    /// Poison - Damage over time, persists through turns
    Poison,
    /// Burn - Damage over time, reduces ATK slightly
    Burn,
    /// Bleed - Physical damage over time
    Bleed,
    /// Freeze - Skip turn, takes extra damage from physical attacks
    Freeze,
    /// Stun - Skip turn (shorter duration than freeze)
    Stun,
    /// Sleep - Skip turn until hit (wakes up on damage)
    Sleep,
    /// Paralysis - Chance to skip turn each tick
    Paralysis,
    /// Silence - Cannot use magic skills
    Silence,
    /// Blind - Reduced accuracy
    Blind,
    /// Confusion - Random target selection
    Confusion,
    /// Berserk - Auto-attack enemies only, increased ATK
    Berserk,
    /// Charm - Attack allies instead of enemies
    Charm,
    /// Regen - Heal over time
    Regen,
    /// Refresh - Regenerate MP over time
    Refresh,
    /// Haste - Increased ATB fill speed
    Haste,
    /// Slow - Decreased ATB fill speed
    Slow,
    /// Stop - ATB completely frozen
    Stop,
    /// Attack Up - Increased ATK stat
    AttackUp,
    /// Attack Down - Decreased ATK stat
    AttackDown,
    /// Defense Up - Increased DEF stat (extends DefenseBuff pattern)
    DefenseUp,
    /// Defense Down - Decreased DEF stat
    DefenseDown,
    /// Magic Up - Increased MAG stat
    MagicUp,
    /// Magic Down - Decreased MAG stat
    MagicDown,
    /// Speed Up - Increased SPD stat
    SpeedUp,
    /// Speed Down - Decreased SPD stat
    SpeedDown,
    /// Luck Up - Increased LUCK stat
    LuckUp,
    /// Luck Down - Decreased LUCK stat
    LuckDown,
    /// Evasion Up - Increased evasion chance
    EvasionUp,
    /// Evasion Down - Decreased evasion chance
    EvasionDown,
    /// Accuracy Up - Increased hit chance
    AccuracyUp,
    /// Accuracy Down - Decreased hit chance
    AccuracyDown,
    /// Shield - Absorbs damage (similar to temporary HP)
    Shield,
    /// Reflect - Reflects magic back at caster
    Reflect,
    /// Invincible - Immune to all damage
    Invincible,
    /// Regenerate - Regenerates HP each turn (stronger than Regen)
    Regenerate,
}

impl StatusType {
    /// Check if this status prevents the entity from taking actions
    pub fn prevents_action(&self) -> bool {
        matches!(self, 
            StatusType::Freeze | 
            StatusType::Stun | 
            StatusType::Sleep |
            StatusType::Stop
        )
    }

    /// Check if this status is a damage over time effect
    pub fn is_dot(&self) -> bool {
        matches!(self, StatusType::Poison | StatusType::Burn | StatusType::Bleed)
    }

    /// Check if this status is a heal over time effect
    pub fn is_hot(&self) -> bool {
        matches!(self, StatusType::Regen | StatusType::Regenerate)
    }

    /// Check if this status is a buff (positive effect)
    pub fn is_buff(&self) -> bool {
        matches!(self,
            StatusType::Regen |
            StatusType::Regenerate |
            StatusType::Refresh |
            StatusType::Haste |
            StatusType::AttackUp |
            StatusType::DefenseUp |
            StatusType::MagicUp |
            StatusType::SpeedUp |
            StatusType::LuckUp |
            StatusType::EvasionUp |
            StatusType::AccuracyUp |
            StatusType::Shield |
            StatusType::Reflect |
            StatusType::Invincible
        )
    }

    /// Check if this status is a debuff (negative effect)
    pub fn is_debuff(&self) -> bool {
        matches!(self,
            StatusType::Poison |
            StatusType::Burn |
            StatusType::Bleed |
            StatusType::Freeze |
            StatusType::Stun |
            StatusType::Sleep |
            StatusType::Paralysis |
            StatusType::Silence |
            StatusType::Blind |
            StatusType::Confusion |
            StatusType::Berserk |
            StatusType::Charm |
            StatusType::Slow |
            StatusType::Stop |
            StatusType::AttackDown |
            StatusType::DefenseDown |
            StatusType::MagicDown |
            StatusType::SpeedDown |
            StatusType::LuckDown |
            StatusType::EvasionDown |
            StatusType::AccuracyDown
        )
    }

    /// Check if this status affects ATB rate
    pub fn affects_atb(&self) -> bool {
        matches!(self, StatusType::Haste | StatusType::Slow | StatusType::Stop)
    }

    /// Get default duration for this status type (in turns)
    pub fn default_duration(&self) -> u32 {
        match self {
            StatusType::Poison => 5,
            StatusType::Burn => 4,
            StatusType::Bleed => 3,
            StatusType::Freeze => 2,
            StatusType::Stun => 1,
            StatusType::Sleep => 3,
            StatusType::Paralysis => 3,
            StatusType::Silence => 4,
            StatusType::Blind => 4,
            StatusType::Confusion => 3,
            StatusType::Berserk => 3,
            StatusType::Charm => 2,
            StatusType::Regen => 5,
            StatusType::Refresh => 5,
            StatusType::Haste => 4,
            StatusType::Slow => 4,
            StatusType::Stop => 2,
            StatusType::Shield => 3,
            StatusType::Reflect => 3,
            StatusType::Invincible => 1,
            StatusType::Regenerate => 4,
            // Stat modifiers
            _ => 3,
        }
    }

    /// Get default potency for this status type
    /// For DoT/HoT: damage/heal amount per tick
    /// For stat modifiers: percentage modifier (e.g., 20 for +20%)
    pub fn default_potency(&self) -> u32 {
        match self {
            StatusType::Poison => 10,
            StatusType::Burn => 15,
            StatusType::Bleed => 12,
            StatusType::Regen => 15,
            StatusType::Regenerate => 25,
            StatusType::Refresh => 5,
            StatusType::AttackUp | StatusType::AttackDown => 25,
            StatusType::DefenseUp | StatusType::DefenseDown => 25,
            StatusType::MagicUp | StatusType::MagicDown => 25,
            StatusType::SpeedUp | StatusType::SpeedDown => 25,
            StatusType::LuckUp | StatusType::LuckDown => 25,
            StatusType::Shield => 50,
            // Percentage-based for ATB
            StatusType::Haste => 100, // Double speed
            StatusType::Slow => 50,   // Half speed
            _ => 0,
        }
    }

    /// Get the category of this status for resistance calculations
    pub fn category(&self) -> StatusCategory {
        match self {
            StatusType::Poison | StatusType::Bleed => StatusCategory::Physical,
            StatusType::Burn | StatusType::Freeze | StatusType::Paralysis => StatusCategory::Elemental,
            StatusType::Silence | StatusType::Blind | StatusType::Confusion | 
            StatusType::Charm | StatusType::Sleep => StatusCategory::Mental,
            _ => StatusCategory::Magical,
        }
    }
}

/// Status categories for resistance calculations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StatusCategory {
    Physical,   // Poison, Bleed
    Elemental,  // Burn, Freeze, Paralysis
    Mental,     // Sleep, Confusion, Charm
    Magical,    // Other magical effects
}

/// A single status effect instance
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StatusEffect {
    /// Type of status effect
    pub status_type: StatusType,
    /// Remaining duration in turns
    pub duration_turns: u32,
    /// Potency (damage/heal amount or % modifier)
    pub potency: u32,
    /// Entity that applied this effect
    pub source: Option<Entity>,
    /// Turn counter for effects that tick each turn
    pub tick_counter: u32,
    /// Whether this effect can be dispelled
    pub dispellable: bool,
    /// Custom data for extensibility (stored as JSON string)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_data: Option<String>,
}

impl StatusEffect {
    /// Create a new status effect with default values
    pub fn new(status_type: StatusType, duration: u32, potency: u32, source: Option<Entity>) -> Self {
        Self {
            status_type,
            duration_turns: duration,
            potency,
            source,
            tick_counter: 0,
            dispellable: true,
            custom_data: None,
        }
    }

    /// Create with default duration and potency for the status type
    pub fn with_defaults(status_type: StatusType, source: Option<Entity>) -> Self {
        Self::new(
            status_type,
            status_type.default_duration(),
            status_type.default_potency(),
            source,
        )
    }

    /// Set dispellable flag
    pub fn dispellable(mut self, dispellable: bool) -> Self {
        self.dispellable = dispellable;
        self
    }

    /// Tick the effect (reduce duration)
    /// Returns true if the effect is still active
    pub fn tick(&mut self) -> bool {
        self.duration_turns = self.duration_turns.saturating_sub(1);
        self.tick_counter += 1;
        self.is_active()
    }

    /// Check if effect is still active
    pub fn is_active(&self) -> bool {
        self.duration_turns > 0
    }

    /// Calculate damage/heal value for this tick
    pub fn calculate_tick_value(&self, max_hp: i32) -> i32 {
        match self.status_type {
            StatusType::Poison | StatusType::Burn | StatusType::Bleed => {
                // DoT deals fixed damage based on potency
                self.potency as i32
            }
            StatusType::Regen => {
                // HoT heals based on potency
                self.potency as i32
            }
            StatusType::Regenerate => {
                // Stronger HoT
                self.potency as i32
            }
            _ => 0,
        }
    }

    /// Get the stat modifier multiplier for this effect
    /// Returns None if this status doesn't modify stats
    pub fn stat_modifier(&self) -> Option<f32> {
        let multiplier = match self.status_type {
            // Buffs
            StatusType::AttackUp |
            StatusType::DefenseUp |
            StatusType::MagicUp |
            StatusType::SpeedUp |
            StatusType::LuckUp |
            StatusType::EvasionUp |
            StatusType::AccuracyUp => 1.0 + (self.potency as f32 / 100.0),
            // Debuffs
            StatusType::AttackDown |
            StatusType::DefenseDown |
            StatusType::MagicDown |
            StatusType::SpeedDown |
            StatusType::LuckDown |
            StatusType::EvasionDown |
            StatusType::AccuracyDown |
            StatusType::Burn => 1.0 - (self.potency as f32 / 100.0).min(0.9),
            // Special
            StatusType::Berserk => 1.5, // +50% ATK when berserk
            _ => return None,
        };
        Some(multiplier)
    }

    /// Get ATB modifier for this effect
    pub fn atb_modifier(&self) -> f32 {
        match self.status_type {
            StatusType::Haste => 2.0,  // Double speed
            StatusType::Slow => 0.5,   // Half speed
            StatusType::Stop => 0.0,   // No ATB gain
            _ => 1.0,
        }
    }
}

impl Default for StatusEffect {
    fn default() -> Self {
        Self {
            status_type: StatusType::Poison,
            duration_turns: 0,
            potency: 0,
            source: None,
            tick_counter: 0,
            dispellable: true,
            custom_data: None,
        }
    }
}

/// Component to hold all status effects on an entity
///
/// This component replaces and extends the simpler DefenseBuff pattern
/// to support full RPG status effects.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StatusEffects {
    /// Active status effects
    pub effects: Vec<StatusEffect>,
    /// Internal flag to track if effects were modified
    #[serde(skip)]
    dirty: bool,
}

/// Events generated by status effect processing
#[derive(Debug, Clone, PartialEq)]
pub enum StatusEvent {
    /// Status effect was applied to target
    Applied {
        target: Entity,
        effect: StatusEffect,
    },
    /// Status effect ticked (DoT/HoT)
    Tick {
        target: Entity,
        status: StatusType,
        value: i32,
        /// True if healing, false if damage
        is_healing: bool,
    },
    /// Status effect expired naturally
    Expired {
        target: Entity,
        status: StatusType,
    },
    /// Status effect was removed (dispelled/cured)
    Removed {
        target: Entity,
        status: StatusType,
        /// True if dispelled by magic/item
        was_dispelled: bool,
    },
    /// Status effect was resisted
    Resisted {
        target: Entity,
        status: StatusType,
        /// Source entity that tried to apply
        source: Option<Entity>,
    },
    /// Entity woke up from sleep (due to damage)
    WokeUp {
        target: Entity,
        caused_by: Entity,
    },
    /// Status effect was refreshed (extended duration)
    Refreshed {
        target: Entity,
        status: StatusType,
        new_duration: u32,
    },
}

impl StatusEffects {
    /// Create empty status effects container
    pub fn new() -> Self {
        Self {
            effects: Vec::new(),
            dirty: false,
        }
    }

    /// Add a status effect
    ///
    /// If an effect of the same type already exists, it will be replaced
    /// and a Refreshed event will be generated instead of Applied.
    pub fn add(&mut self, effect: StatusEffect) -> Option<StatusEvent> {
        // Check if effect already exists
        if let Some(existing) = self.effects.iter_mut()
            .find(|e| e.status_type == effect.status_type) {
            let old_duration = existing.duration_turns;
            *existing = effect.clone();
            return Some(StatusEvent::Refreshed {
                target: Entity::DANGLING, // Will be set by caller
                status: effect.status_type,
                new_duration: effect.duration_turns,
            });
        }

        self.effects.push(effect);
        self.dirty = true;
        None
    }

    /// Remove a specific status effect by type
    /// Returns true if an effect was removed
    pub fn remove(&mut self, status_type: StatusType) -> bool {
        let initial_len = self.effects.len();
        self.effects.retain(|e| e.status_type != status_type);
        let removed = self.effects.len() < initial_len;
        if removed {
            self.dirty = true;
        }
        removed
    }

    /// Remove all effects that match a predicate
    pub fn remove_if<F>(&mut self, predicate: F) -> Vec<StatusType>
    where
        F: Fn(&StatusEffect) -> bool,
    {
        let mut removed = Vec::new();
        self.effects.retain(|e| {
            if predicate(e) {
                removed.push(e.status_type);
                false
            } else {
                true
            }
        });
        if !removed.is_empty() {
            self.dirty = true;
        }
        removed
    }

    /// Remove all dispellable effects (for Dispel magic)
    pub fn dispel_all(&mut self) -> Vec<StatusType> {
        self.remove_if(|e| e.dispellable)
    }

    /// Remove all debuffs
    pub fn cure_debuffs(&mut self) -> Vec<StatusType> {
        self.remove_if(|e| e.status_type.is_debuff())
    }

    /// Remove all buffs (for enemy dispel)
    pub fn dispel_buffs(&mut self) -> Vec<StatusType> {
        self.remove_if(|e| e.status_type.is_buff())
    }

    /// Check if entity has a specific status effect
    pub fn has(&self, status_type: StatusType) -> bool {
        self.effects.iter().any(|e| e.status_type == status_type && e.is_active())
    }

    /// Check if entity has any of the given status effects
    pub fn has_any(&self, types: &[StatusType]) -> bool {
        types.iter().any(|&t| self.has(t))
    }

    /// Check if entity has all of the given status effects
    pub fn has_all(&self, types: &[StatusType]) -> bool {
        types.iter().all(|&t| self.has(t))
    }

    /// Get a specific status effect
    pub fn get(&self, status_type: StatusType) -> Option<&StatusEffect> {
        self.effects.iter().find(|e| e.status_type == status_type && e.is_active())
    }

    /// Get mutable reference to a specific status effect
    pub fn get_mut(&mut self, status_type: StatusType) -> Option<&mut StatusEffect> {
        self.effects.iter_mut().find(|e| e.status_type == status_type)
    }

    /// Get all active effects
    pub fn active_effects(&self) -> impl Iterator<Item = &StatusEffect> {
        self.effects.iter().filter(|e| e.is_active())
    }

    /// Get all effects of a specific category
    pub fn effects_by_category(&self, category: StatusCategory) -> impl Iterator<Item = &StatusEffect> {
        self.effects.iter().filter(move |e| e.status_type.category() == category)
    }

    /// Tick all status effects for turn progression
    /// Returns a list of expired effects and events
    pub fn tick_turn(&mut self, entity: Entity) -> (Vec<StatusType>, Vec<StatusEvent>) {
        let mut expired = Vec::new();
        let mut events = Vec::new();

        self.effects.retain(|e| {
            if e.duration_turns <= 1 {
                expired.push(e.status_type);
                events.push(StatusEvent::Expired {
                    target: entity,
                    status: e.status_type,
                });
                false
            } else {
                true
            }
        });

        // Reduce duration for remaining effects
        for effect in &mut self.effects {
            effect.duration_turns -= 1;
        }

        if !expired.is_empty() {
            self.dirty = true;
        }

        (expired, events)
    }

    /// Process DoT/HoT ticks
    /// Returns events for each tick effect
    pub fn process_ticks(&self, entity: Entity, max_hp: i32) -> Vec<StatusEvent> {
        let mut events = Vec::new();

        for effect in self.effects.iter().filter(|e| e.is_active()) {
            let value = effect.calculate_tick_value(max_hp);
            
            if value != 0 {
                let is_healing = effect.status_type.is_hot();
                events.push(StatusEvent::Tick {
                    target: entity,
                    status: effect.status_type,
                    value: value.abs(),
                    is_healing,
                });
            }
        }

        events
    }

    /// Clear all status effects
    pub fn clear(&mut self) {
        self.effects.clear();
        self.dirty = true;
    }

    /// Check if entity is prevented from taking actions
    pub fn is_action_prevented(&self) -> bool {
        self.effects.iter().any(|e| {
            e.is_active() && e.status_type.prevents_action()
        })
    }

    /// Get the preventing status if action is prevented
    pub fn action_prevention_reason(&self) -> Option<StatusType> {
        self.effects.iter()
            .find(|e| e.is_active() && e.status_type.prevents_action())
            .map(|e| e.status_type)
    }

    /// Calculate attack modifier from all active effects
    /// Returns multiplier (e.g., 1.2 for +20%)
    pub fn atk_modifier(&self) -> f32 {
        let mut modifier = 1.0f32;
        
        for effect in self.effects.iter().filter(|e| e.is_active()) {
            if let Some(m) = effect.stat_modifier() {
                match effect.status_type {
                    StatusType::AttackUp | StatusType::AttackDown | StatusType::Berserk | StatusType::Burn => {
                        modifier *= m;
                    }
                    _ => {}
                }
            }
        }

        modifier.max(0.1) // Minimum 10% attack
    }

    /// Calculate defense modifier from all active effects
    /// Returns multiplier (e.g., 0.8 for -20%)
    pub fn def_modifier(&self) -> f32 {
        let mut modifier = 1.0f32;
        
        for effect in self.effects.iter().filter(|e| e.is_active()) {
            if let Some(m) = effect.stat_modifier() {
                match effect.status_type {
                    StatusType::DefenseUp | StatusType::DefenseDown => {
                        modifier *= m;
                    }
                    _ => {}
                }
            }
        }

        modifier.max(0.1) // Minimum 10% defense
    }

    /// Calculate magic modifier from all active effects
    pub fn mag_modifier(&self) -> f32 {
        let mut modifier = 1.0f32;
        
        for effect in self.effects.iter().filter(|e| e.is_active()) {
            if let Some(m) = effect.stat_modifier() {
                match effect.status_type {
                    StatusType::MagicUp | StatusType::MagicDown => {
                        modifier *= m;
                    }
                    _ => {}
                }
            }
        }

        modifier.max(0.1)
    }

    /// Calculate speed modifier from all active effects
    pub fn spd_modifier(&self) -> f32 {
        let mut modifier = 1.0f32;
        
        for effect in self.effects.iter().filter(|e| e.is_active()) {
            if let Some(m) = effect.stat_modifier() {
                match effect.status_type {
                    StatusType::SpeedUp | StatusType::SpeedDown => {
                        modifier *= m;
                    }
                    _ => {}
                }
            }
        }

        modifier.max(0.1)
    }

    /// Calculate luck modifier from all active effects
    pub fn luck_modifier(&self) -> f32 {
        let mut modifier = 1.0f32;
        
        for effect in self.effects.iter().filter(|e| e.is_active()) {
            if let Some(m) = effect.stat_modifier() {
                match effect.status_type {
                    StatusType::LuckUp | StatusType::LuckDown => {
                        modifier *= m;
                    }
                    _ => {}
                }
            }
        }

        modifier.max(0.1)
    }

    /// Calculate ATB fill rate modifier
    /// Returns multiplier for ATB rate
    pub fn atb_modifier(&self) -> f32 {
        let mut modifier = 1.0f32;
        
        for effect in self.effects.iter().filter(|e| e.is_active()) {
            modifier *= effect.atb_modifier();
        }

        modifier.max(0.0)
    }

    /// Calculate evasion modifier
    pub fn evasion_modifier(&self) -> f32 {
        let mut modifier = 1.0f32;
        
        for effect in self.effects.iter().filter(|e| e.is_active()) {
            match effect.status_type {
                StatusType::EvasionUp | StatusType::EvasionDown => {
                    if let Some(m) = effect.stat_modifier() {
                        modifier *= m;
                    }
                }
                StatusType::Blind => {
                    modifier *= 0.5; // Blind halves evasion
                }
                _ => {}
            }
        }

        modifier.max(0.0)
    }

    /// Calculate accuracy modifier
    pub fn accuracy_modifier(&self) -> f32 {
        let mut modifier = 1.0f32;
        
        for effect in self.effects.iter().filter(|e| e.is_active()) {
            match effect.status_type {
                StatusType::AccuracyUp | StatusType::AccuracyDown => {
                    if let Some(m) = effect.stat_modifier() {
                        modifier *= m;
                    }
                }
                StatusType::Blind => {
                    modifier *= 0.5; // Blind halves accuracy
                }
                _ => {}
            }
        }

        modifier.max(0.1)
    }

    /// Check if entity can use magic
    pub fn can_use_magic(&self) -> bool {
        !self.has(StatusType::Silence)
    }

    /// Check if entity is invincible
    pub fn is_invincible(&self) -> bool {
        self.has(StatusType::Invincible)
    }

    /// Check if entity has a shield
    pub fn get_shield(&self) -> Option<u32> {
        self.get(StatusType::Shield).map(|e| e.potency)
    }

    /// Consume shield damage and return remaining shield amount
    pub fn consume_shield(&mut self, damage: u32) -> u32 {
        if let Some(effect) = self.get_mut(StatusType::Shield) {
            if effect.potency > damage {
                effect.potency -= damage;
                0 // All damage absorbed
            } else {
                let remaining_damage = damage - effect.potency;
                effect.potency = 0;
                effect.duration_turns = 0; // Shield broken
                self.remove(StatusType::Shield);
                remaining_damage
            }
        } else {
            damage
        }
    }

    /// Check if dirty flag is set
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Clear dirty flag
    pub fn clear_dirty(&mut self) {
        self.dirty = false;
    }

    /// Count of active effects
    pub fn count(&self) -> usize {
        self.effects.iter().filter(|e| e.is_active()).count()
    }

    /// Check if there are any active effects
    pub fn is_empty(&self) -> bool {
        self.count() == 0
    }
}

/// Resistance data for status effects
/// 
/// Stored in Stats component extension or as separate component
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct StatusResistances {
    /// Physical status resistance (Poison, Bleed)
    pub physical: f32,
    /// Elemental status resistance (Burn, Freeze)
    pub elemental: f32,
    /// Mental status resistance (Sleep, Confusion)
    pub mental: f32,
    /// Magical status resistance (other magical effects)
    pub magical: f32,
}

impl StatusResistances {
    /// Create with all resistances at same value
    pub fn uniform(resistance: f32) -> Self {
        Self {
            physical: resistance,
            elemental: resistance,
            mental: resistance,
            magical: resistance,
        }
    }

    /// Get resistance for a specific status category
    pub fn get(&self, category: StatusCategory) -> f32 {
        match category {
            StatusCategory::Physical => self.physical,
            StatusCategory::Elemental => self.elemental,
            StatusCategory::Mental => self.mental,
            StatusCategory::Magical => self.magical,
        }
    }

    /// Set resistance for a specific category
    pub fn set(&mut self, category: StatusCategory, value: f32) {
        let clamped = value.clamp(0.0, 1.0);
        match category {
            StatusCategory::Physical => self.physical = clamped,
            StatusCategory::Elemental => self.elemental = clamped,
            StatusCategory::Mental => self.mental = clamped,
            StatusCategory::Magical => self.magical = clamped,
        }
    }
}

/// Extension trait for Stats to add status resistance
pub trait StatusResistanceExt {
    /// Get resistance value for a specific status type (0.0-1.0)
    fn status_resistance(&self, status: StatusType) -> f32;
    /// Get base resistance based on luck stat
    fn base_status_resistance(&self) -> f32;
}

impl StatusResistanceExt for Stats {
    fn status_resistance(&self, status: StatusType) -> f32 {
        // Base resistance from luck (0.5% per luck point, max 25%)
        let base_resist = self.base_status_resistance();
        
        // Category-based resistance could be stored in a separate component
        // For now, use base resistance
        let category_multiplier = match status.category() {
            StatusCategory::Physical => 1.0,
            StatusCategory::Elemental => 1.0,
            StatusCategory::Mental => 0.8, // Mental statuses slightly harder to resist
            StatusCategory::Magical => 1.0,
        };

        (base_resist * category_multiplier).min(0.95) // Cap at 95%
    }

    fn base_status_resistance(&self) -> f32 {
        (self.luck as f32 * 0.005).min(0.25)
    }
}

/// Visual indicator functions for UI rendering

/// Get icon/emoji for a status effect
pub fn get_status_icon(status: StatusType) -> &'static str {
    match status {
        StatusType::Poison => "☠️",
        StatusType::Burn => "🔥",
        StatusType::Bleed => "🩸",
        StatusType::Freeze => "❄️",
        StatusType::Stun => "⚡",
        StatusType::Sleep => "💤",
        StatusType::Paralysis => "⚡",
        StatusType::Silence => "🤐",
        StatusType::Blind => "🕶️",
        StatusType::Confusion => "😵",
        StatusType::Berserk => "😤",
        StatusType::Charm => "💖",
        StatusType::Regen => "✨",
        StatusType::Refresh => "💧",
        StatusType::Haste => "⚡",
        StatusType::Slow => "🐌",
        StatusType::Stop => "⏹️",
        StatusType::AttackUp => "⚔️",
        StatusType::AttackDown => "💔",
        StatusType::DefenseUp => "🛡️",
        StatusType::DefenseDown => "🕳️",
        StatusType::MagicUp => "🔮",
        StatusType::MagicDown => "📉",
        StatusType::SpeedUp => "💨",
        StatusType::SpeedDown => "🦥",
        StatusType::LuckUp => "🍀",
        StatusType::LuckDown => "🌧️",
        StatusType::EvasionUp => "👻",
        StatusType::EvasionDown => "🎯",
        StatusType::AccuracyUp => "🎯",
        StatusType::AccuracyDown => "🌫️",
        StatusType::Shield => "🛡️",
        StatusType::Reflect => "🔁",
        StatusType::Invincible => "⭐",
        StatusType::Regenerate => "🌟",
    }
}

/// Get color for a status effect (RGB tuple)
pub fn get_status_color(status: StatusType) -> (u8, u8, u8) {
    match status {
        // Negative effects - Reds/Purples
        StatusType::Poison => (128, 0, 128),      // Purple
        StatusType::Burn => (255, 69, 0),         // Red-Orange
        StatusType::Bleed => (178, 34, 34),       // Dark Red
        StatusType::Freeze => (173, 216, 230),    // Light Blue
        StatusType::Stun => (255, 255, 0),        // Yellow
        StatusType::Sleep => (147, 112, 219),     // Medium Purple
        StatusType::Paralysis => (255, 215, 0),   // Gold
        StatusType::Silence => (105, 105, 105),   // Dim Gray
        StatusType::Blind => (0, 0, 0),           // Black
        StatusType::Confusion => (255, 0, 255),   // Magenta
        StatusType::Berserk => (220, 20, 60),     // Crimson
        StatusType::Charm => (255, 105, 180),     // Hot Pink
        
        // Positive effects - Greens/Blues/Golds
        StatusType::Regen => (50, 205, 50),       // Lime Green
        StatusType::Regenerate => (0, 255, 127),  // Spring Green
        StatusType::Refresh => (0, 191, 255),     // Deep Sky Blue
        StatusType::Haste => (255, 215, 0),       // Gold
        StatusType::Shield => (135, 206, 235),    // Sky Blue
        StatusType::Reflect => (192, 192, 192),   // Silver
        StatusType::Invincible => (255, 215, 0),  // Gold
        
        // Buffs - Blues
        StatusType::AttackUp => (255, 99, 71),    // Tomato
        StatusType::DefenseUp => (65, 105, 225),  // Royal Blue
        StatusType::MagicUp => (138, 43, 226),    // Blue Violet
        StatusType::SpeedUp => (0, 206, 209),     // Dark Turquoise
        StatusType::LuckUp => (50, 205, 50),      // Lime Green
        StatusType::EvasionUp => (147, 112, 219), // Medium Purple
        StatusType::AccuracyUp => (255, 165, 0),  // Orange
        
        // Debuffs - Browns/Oranges
        StatusType::AttackDown => (139, 69, 19),  // Saddle Brown
        StatusType::DefenseDown => (160, 82, 45), // Sienna
        StatusType::MagicDown => (205, 92, 92),   // Indian Red
        StatusType::SpeedDown => (210, 105, 30),  // Chocolate
        StatusType::LuckDown => (112, 128, 144),  // Slate Gray
        StatusType::EvasionDown => (128, 128, 0), // Olive
        StatusType::AccuracyDown => (218, 165, 32), // Goldenrod
        
        // Other
        StatusType::Slow => (139, 0, 0),          // Dark Red
        StatusType::Stop => (0, 0, 139),          // Dark Blue
    }
}

/// Get status name for display
pub fn get_status_name(status: StatusType) -> &'static str {
    match status {
        StatusType::Poison => "Poison",
        StatusType::Burn => "Burn",
        StatusType::Bleed => "Bleed",
        StatusType::Freeze => "Freeze",
        StatusType::Stun => "Stun",
        StatusType::Sleep => "Sleep",
        StatusType::Paralysis => "Paralysis",
        StatusType::Silence => "Silence",
        StatusType::Blind => "Blind",
        StatusType::Confusion => "Confusion",
        StatusType::Berserk => "Berserk",
        StatusType::Charm => "Charm",
        StatusType::Regen => "Regen",
        StatusType::Refresh => "Refresh",
        StatusType::Haste => "Haste",
        StatusType::Slow => "Slow",
        StatusType::Stop => "Stop",
        StatusType::AttackUp => "Attack Up",
        StatusType::AttackDown => "Attack Down",
        StatusType::DefenseUp => "Defense Up",
        StatusType::DefenseDown => "Defense Down",
        StatusType::MagicUp => "Magic Up",
        StatusType::MagicDown => "Magic Down",
        StatusType::SpeedUp => "Speed Up",
        StatusType::SpeedDown => "Speed Down",
        StatusType::LuckUp => "Luck Up",
        StatusType::LuckDown => "Luck Down",
        StatusType::EvasionUp => "Evasion Up",
        StatusType::EvasionDown => "Evasion Down",
        StatusType::AccuracyUp => "Accuracy Up",
        StatusType::AccuracyDown => "Accuracy Down",
        StatusType::Shield => "Shield",
        StatusType::Reflect => "Reflect",
        StatusType::Invincible => "Invincible",
        StatusType::Regenerate => "Regenerate",
    }
}

/// Get description for a status effect
pub fn get_status_description(status: StatusType) -> &'static str {
    match status {
        StatusType::Poison => "Takes damage each turn",
        StatusType::Burn => "Takes fire damage each turn, reduced ATK",
        StatusType::Bleed => "Takes physical damage each turn",
        StatusType::Freeze => "Cannot act, takes extra damage from physical attacks",
        StatusType::Stun => "Cannot act for a short time",
        StatusType::Sleep => "Cannot act until hit",
        StatusType::Paralysis => "Chance to be unable to act",
        StatusType::Silence => "Cannot use magic",
        StatusType::Blind => "Reduced accuracy and evasion",
        StatusType::Confusion => "May attack random targets",
        StatusType::Berserk => "Auto-attacks with increased power",
        StatusType::Charm => "Attacks allies instead of enemies",
        StatusType::Regen => "Recovers HP each turn",
        StatusType::Refresh => "Recovers MP each turn",
        StatusType::Haste => "ATB fills faster",
        StatusType::Slow => "ATB fills slower",
        StatusType::Stop => "ATB completely frozen",
        StatusType::AttackUp => "Increased attack power",
        StatusType::AttackDown => "Decreased attack power",
        StatusType::DefenseUp => "Increased defense",
        StatusType::DefenseDown => "Decreased defense",
        StatusType::MagicUp => "Increased magic power",
        StatusType::MagicDown => "Decreased magic power",
        StatusType::SpeedUp => "Increased speed",
        StatusType::SpeedDown => "Decreased speed",
        StatusType::LuckUp => "Increased luck",
        StatusType::LuckDown => "Decreased luck",
        StatusType::EvasionUp => "Increased evasion",
        StatusType::EvasionDown => "Decreased evasion",
        StatusType::AccuracyUp => "Increased accuracy",
        StatusType::AccuracyDown => "Decreased accuracy",
        StatusType::Shield => "Absorbs damage",
        StatusType::Reflect => "Reflects magic back",
        StatusType::Invincible => "Immune to all damage",
        StatusType::Regenerate => "Recovers large amounts of HP each turn",
    }
}

/// System to process status effects each tick
/// 
/// This should be called regularly during battle update.
/// Returns all status events that occurred.
pub fn process_status_effects(world: &mut World, _dt: f32) -> Vec<StatusEvent> {
    let mut all_events = Vec::new();
    
    // Collect entities with StatusEffects first
    let entities: Vec<Entity> = world.query_mut::<&StatusEffects>()
        .into_iter()
        .map(|(e, _)| e)
        .collect();

    for entity in entities {
        // Get max hp
        let max_hp = world.query_one::<&Stats>(entity)
            .ok()
            .and_then(|q| q.get().map(|s| s.max_hp))
            .unwrap_or(100);
        
        // Process DoT/HoT ticks
        let tick_events = if let Ok(query) = world.query_one_mut::<&mut StatusEffects>(entity) {
            query.process_ticks(entity, max_hp)
        } else {
            Vec::new()
        };
        
        // Apply damage/healing from ticks
        for event in &tick_events {
            if let StatusEvent::Tick { value, is_healing, .. } = event {
                if let Ok(stats_ref) = world.query_one_mut::<&mut Stats>(entity) {
                    if *is_healing {
                        stats_ref.heal(*value);
                    } else {
                        stats_ref.take_damage(*value);
                    }
                }
            }
        }
        
        all_events.extend(tick_events);
    }

    all_events
}

/// Attempt to apply a status effect with resistance check
/// 
/// Returns the result of the application attempt
pub fn try_apply_status(
    world: &mut World,
    target: Entity,
    effect: StatusEffect,
    source: Option<Entity>,
) -> Result<StatusEvent, StatusEvent> {
    // Get target stats for resistance check
    let resistance = world.query_one::<&Stats>(target)
        .ok()
        .and_then(|mut q| q.get().map(|s: &Stats| s.status_resistance(effect.status_type)))
        .unwrap_or(0.0);

    // Roll for resistance
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let roll: f32 = rng.gen();

    if roll < resistance {
        // Resisted
        let event = StatusEvent::Resisted {
            target,
            status: effect.status_type,
            source,
        };
        return Err(event);
    }

    // Apply the effect
    if let Ok(query) = world.query_one_mut::<&mut StatusEffects>(target) {
        query.add(effect.clone());
    } else {
        // Insert new StatusEffects component
        let mut new_effects = StatusEffects::new();
        new_effects.add(effect.clone());
        let _ = world.insert(target, (new_effects,));
    }

    let event = StatusEvent::Applied { target, effect };
    Ok(event)
}

/// Wake up sleeping entity (called when taking damage)
pub fn wake_up_entity(world: &mut World, entity: Entity, caused_by: Entity) -> Option<StatusEvent> {
    if let Ok(query) = world.query_one_mut::<&mut StatusEffects>(entity) {
        if query.has(StatusType::Sleep) {
            query.remove(StatusType::Sleep);
            return Some(StatusEvent::WokeUp {
                target: entity,
                caused_by,
            });
        }
    }
    None
}

/// Check if entity can act on their turn
pub fn can_entity_act(world: &World, entity: Entity) -> bool {
    if let Ok(query) = world.query_one::<&StatusEffects>(entity) {
        if let Some(statuses) = query.get() {
            return !statuses.is_action_prevented();
        }
    }
    true
}

/// Get turn skip reason if entity cannot act
pub fn get_turn_skip_reason(world: &World, entity: Entity) -> Option<&'static str> {
    if let Ok(mut query) = world.query_one::<&StatusEffects>(entity) {
        if let Some(statuses) = query.get() {
            return match statuses.action_prevention_reason() {
                Some(StatusType::Freeze) => Some("Frozen solid!"),
                Some(StatusType::Stun) => Some("Stunned!"),
                Some(StatusType::Sleep) => Some("Fast asleep..."),
                Some(StatusType::Stop) => Some("Time is stopped!"),
                _ => None,
            };
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_world() -> World {
        World::new()
    }

    fn create_test_entity(world: &mut World, hp: i32) -> Entity {
        world.spawn((
            Stats {
                hp,
                max_hp: hp,
                mp: 50,
                max_mp: 50,
                str: 20,
                def: 15,
                spd: 10,
                mag: 15,
                luck: 10,
                level: 1,
                exp: 0,
            },
            StatusEffects::new(),
        ))
    }

    #[test]
    fn test_status_effect_creation() {
        let effect = StatusEffect::new(StatusType::Poison, 5, 10, None);
        assert_eq!(effect.status_type, StatusType::Poison);
        assert_eq!(effect.duration_turns, 5);
        assert_eq!(effect.potency, 10);
        assert!(effect.is_active());
    }

    #[test]
    fn test_status_effect_tick() {
        let mut effect = StatusEffect::new(StatusType::Poison, 3, 10, None);
        assert!(effect.tick());
        assert_eq!(effect.duration_turns, 2);
        assert!(effect.tick());
        assert_eq!(effect.duration_turns, 1);
        assert!(!effect.tick()); // Expires
        assert!(!effect.is_active());
    }

    #[test]
    fn test_status_effect_defaults() {
        let effect = StatusEffect::with_defaults(StatusType::Poison, None);
        assert_eq!(effect.duration_turns, StatusType::Poison.default_duration());
        assert_eq!(effect.potency, StatusType::Poison.default_potency());
    }

    #[test]
    fn test_status_effects_add_remove() {
        let mut effects = StatusEffects::new();
        let effect = StatusEffect::new(StatusType::Poison, 5, 10, None);
        
        effects.add(effect);
        assert!(effects.has(StatusType::Poison));
        assert_eq!(effects.count(), 1);
        
        assert!(effects.remove(StatusType::Poison));
        assert!(!effects.has(StatusType::Poison));
        assert_eq!(effects.count(), 0);
        
        // Removing non-existent returns false
        assert!(!effects.remove(StatusType::Burn));
    }

    #[test]
    fn test_status_effects_get() {
        let mut effects = StatusEffects::new();
        let effect = StatusEffect::new(StatusType::Poison, 5, 10, None);
        effects.add(effect);
        
        let retrieved = effects.get(StatusType::Poison);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().potency, 10);
        
        assert!(effects.get(StatusType::Burn).is_none());
    }

    #[test]
    fn test_status_effects_tick_turn() {
        let mut effects = StatusEffects::new();
        let entity = Entity::DANGLING;
        
        effects.add(StatusEffect::new(StatusType::Poison, 2, 10, None));
        effects.add(StatusEffect::new(StatusType::Regen, 3, 5, None));
        
        let (expired, _) = effects.tick_turn(entity);
        assert!(expired.is_empty());
        
        let (expired, _) = effects.tick_turn(entity);
        assert_eq!(expired.len(), 1);
        assert!(expired.contains(&StatusType::Poison));
        
        let (expired, _) = effects.tick_turn(entity);
        assert_eq!(expired.len(), 1);
        assert!(expired.contains(&StatusType::Regen));
        
        assert!(effects.is_empty());
    }

    #[test]
    fn test_atk_modifier() {
        let mut effects = StatusEffects::new();
        assert_eq!(effects.atk_modifier(), 1.0);
        
        // Attack Up (+25% -> 1.0 + 25/100 = 1.25)
        effects.add(StatusEffect::new(StatusType::AttackUp, 3, 25, None));
        assert!((effects.atk_modifier() - 1.25).abs() < 0.01);
        
        // Add another modifier (e.g., Berserk +50% ATK) - multiplicative
        effects.add(StatusEffect::new(StatusType::Berserk, 3, 0, None));
        assert!((effects.atk_modifier() - 1.875).abs() < 0.01, "Expected 1.875, got {}", effects.atk_modifier());
    }

    #[test]
    fn test_def_modifier() {
        let mut effects = StatusEffects::new();
        assert_eq!(effects.def_modifier(), 1.0);
        
        // Defense Up (+25%)
        effects.add(StatusEffect::new(StatusType::DefenseUp, 3, 25, None));
        assert!((effects.def_modifier() - 1.25).abs() < 0.01);
        
        // Defense Down (-25%)
        effects.add(StatusEffect::new(StatusType::DefenseDown, 3, 25, None));
        assert!((effects.def_modifier() - 0.9375).abs() < 0.01);
    }

    #[test]
    fn test_atb_modifier() {
        let mut effects = StatusEffects::new();
        assert_eq!(effects.atb_modifier(), 1.0);
        
        effects.add(StatusEffect::new(StatusType::Haste, 3, 100, None));
        assert_eq!(effects.atb_modifier(), 2.0);
        
        // Haste + Slow = normal speed
        effects.add(StatusEffect::new(StatusType::Slow, 3, 50, None));
        assert_eq!(effects.atb_modifier(), 1.0);
        
        // Stop overrides everything
        effects.add(StatusEffect::new(StatusType::Stop, 1, 0, None));
        assert_eq!(effects.atb_modifier(), 0.0);
    }

    #[test]
    fn test_action_prevention() {
        let mut effects = StatusEffects::new();
        assert!(!effects.is_action_prevented());
        
        effects.add(StatusEffect::new(StatusType::Stun, 2, 0, None));
        assert!(effects.is_action_prevented());
        assert_eq!(effects.action_prevention_reason(), Some(StatusType::Stun));
        
        effects.remove(StatusType::Stun);
        assert!(!effects.is_action_prevented());
        
        effects.add(StatusEffect::new(StatusType::Sleep, 3, 0, None));
        assert!(effects.is_action_prevented());
    }

    #[test]
    fn test_status_type_checks() {
        assert!(StatusType::Poison.is_debuff());
        assert!(!StatusType::Poison.is_buff());
        assert!(StatusType::Poison.is_dot());
        assert!(!StatusType::Poison.is_hot());
        
        assert!(StatusType::Regen.is_buff());
        assert!(!StatusType::Regen.is_debuff());
        assert!(!StatusType::Regen.is_dot());
        assert!(StatusType::Regen.is_hot());
        
        assert!(StatusType::Stun.prevents_action());
        assert!(StatusType::Freeze.prevents_action());
        assert!(!StatusType::Poison.prevents_action());
        
        assert!(StatusType::Haste.affects_atb());
        assert!(StatusType::Slow.affects_atb());
    }

    #[test]
    fn test_shield() {
        let mut effects = StatusEffects::new();
        effects.add(StatusEffect::new(StatusType::Shield, 3, 50, None));
        
        assert_eq!(effects.get_shield(), Some(50));
        
        // Consume 20 damage
        let remaining = effects.consume_shield(20);
        assert_eq!(remaining, 0);
        assert_eq!(effects.get_shield(), Some(30));
        
        // Consume remaining + 10 more
        let remaining = effects.consume_shield(40);
        assert_eq!(remaining, 10);
        assert!(effects.get_shield().is_none());
    }

    #[test]
    fn test_can_use_magic() {
        let mut effects = StatusEffects::new();
        assert!(effects.can_use_magic());
        
        effects.add(StatusEffect::new(StatusType::Silence, 3, 0, None));
        assert!(!effects.can_use_magic());
    }

    #[test]
    fn test_invincible() {
        let mut effects = StatusEffects::new();
        assert!(!effects.is_invincible());
        
        effects.add(StatusEffect::new(StatusType::Invincible, 1, 0, None));
        assert!(effects.is_invincible());
    }

    #[test]
    fn test_cure_debuffs() {
        let mut effects = StatusEffects::new();
        effects.add(StatusEffect::new(StatusType::Poison, 5, 10, None));
        effects.add(StatusEffect::new(StatusType::AttackUp, 3, 25, None));
        effects.add(StatusEffect::new(StatusType::Burn, 4, 15, None));
        
        let cured = effects.cure_debuffs();
        assert_eq!(cured.len(), 2);
        assert!(cured.contains(&StatusType::Poison));
        assert!(cured.contains(&StatusType::Burn));
        assert!(effects.has(StatusType::AttackUp));
    }

    #[test]
    fn test_dispel_buffs() {
        let mut effects = StatusEffects::new();
        effects.add(StatusEffect::new(StatusType::Poison, 5, 10, None));
        effects.add(StatusEffect::new(StatusType::AttackUp, 3, 25, None));
        effects.add(StatusEffect::new(StatusType::Regen, 5, 15, None));
        
        let dispelled = effects.dispel_buffs();
        assert_eq!(dispelled.len(), 2);
        assert!(effects.has(StatusType::Poison));
    }

    #[test]
    fn test_status_resistance() {
        let stats = Stats {
            luck: 20,
            ..Default::default()
        };
        
        // 20 luck = 10% base resistance
        assert!((stats.base_status_resistance() - 0.10).abs() < 0.01);
        
        // Specific status resistance
        let resist = stats.status_resistance(StatusType::Poison);
        assert!(resist > 0.0 && resist < 0.5);
    }

    #[test]
    fn test_status_icons() {
        assert_eq!(get_status_icon(StatusType::Poison), "☠️");
        assert_eq!(get_status_icon(StatusType::Burn), "🔥");
        assert_eq!(get_status_icon(StatusType::Regen), "✨");
    }

    #[test]
    fn test_status_colors() {
        let (r, g, b) = get_status_color(StatusType::Poison);
        assert!(r > 0 || g > 0 || b > 0);
        
        let (r, g, b) = get_status_color(StatusType::Regen);
        assert!(g > r); // Green-ish
    }

    #[test]
    fn test_status_names() {
        assert_eq!(get_status_name(StatusType::Poison), "Poison");
        assert_eq!(get_status_name(StatusType::AttackUp), "Attack Up");
    }

    #[test]
    fn test_dot_calculation() {
        let poison = StatusEffect::new(StatusType::Poison, 5, 10, None);
        assert_eq!(poison.calculate_tick_value(100), 10);
        
        let burn = StatusEffect::new(StatusType::Burn, 4, 15, None);
        assert_eq!(burn.calculate_tick_value(100), 15);
        
        let regen = StatusEffect::new(StatusType::Regen, 5, 20, None);
        assert_eq!(regen.calculate_tick_value(100), 20);
    }

    #[test]
    fn test_has_any_all() {
        let mut effects = StatusEffects::new();
        effects.add(StatusEffect::new(StatusType::Poison, 5, 10, None));
        effects.add(StatusEffect::new(StatusType::Burn, 4, 15, None));
        
        assert!(effects.has_any(&[StatusType::Poison, StatusType::Regen]));
        assert!(effects.has_all(&[StatusType::Poison, StatusType::Burn]));
        assert!(!effects.has_all(&[StatusType::Poison, StatusType::Regen]));
    }

    #[test]
    fn test_status_categories() {
        assert_eq!(StatusType::Poison.category(), StatusCategory::Physical);
        assert_eq!(StatusType::Burn.category(), StatusCategory::Elemental);
        assert_eq!(StatusType::Sleep.category(), StatusCategory::Mental);
    }

    #[test]
    fn test_status_resistances() {
        let resist = StatusResistances::uniform(0.5);
        assert_eq!(resist.physical, 0.5);
        assert_eq!(resist.get(StatusCategory::Elemental), 0.5);
        
        let mut resist = StatusResistances::default();
        resist.set(StatusCategory::Physical, 0.75);
        assert_eq!(resist.physical, 0.75);
    }

    #[test]
    fn test_effect_refreshing() {
        let mut effects = StatusEffects::new();
        let entity = Entity::DANGLING;
        
        // Add initial effect
        effects.add(StatusEffect::new(StatusType::Poison, 3, 10, None));
        
        // Refresh with new effect
        let refreshed = effects.add(StatusEffect::new(StatusType::Poison, 5, 15, None));
        assert!(refreshed.is_some());
        
        // Should have the new values
        let effect = effects.get(StatusType::Poison).unwrap();
        assert_eq!(effect.duration_turns, 5);
        assert_eq!(effect.potency, 15);
    }

    #[test]
    fn test_dispellable() {
        let effect = StatusEffect::new(StatusType::Poison, 5, 10, None)
            .dispellable(false);
        assert!(!effect.dispellable);
        
        let mut effects = StatusEffects::new();
        effects.add(effect);
        effects.add(StatusEffect::new(StatusType::Regen, 5, 15, None));
        
        let dispelled = effects.dispel_all();
        assert_eq!(dispelled.len(), 1); // Only Regen is dispellable
        assert!(effects.has(StatusType::Poison));
    }

    #[test]
    fn test_berzerk_modifier() {
        let mut effects = StatusEffects::new();
        effects.add(StatusEffect::new(StatusType::Berserk, 3, 0, None));
        assert!((effects.atk_modifier() - 1.5).abs() < 0.01);
    }

    #[test]
    fn test_blind_modifiers() {
        let mut effects = StatusEffects::new();
        // Blind has hardcoded 50% reduction for accuracy and evasion in the modifier methods
        effects.add(StatusEffect::new(StatusType::Blind, 3, 0, None));
        
        // Blind halves accuracy and evasion (hardcoded 0.5 multiplier in accuracy_modifier/evasion_modifier)
        assert!((effects.accuracy_modifier() - 0.5).abs() < 0.01, 
            "Expected 0.5, got {}", effects.accuracy_modifier());
        assert!((effects.evasion_modifier() - 0.5).abs() < 0.01,
            "Expected 0.5, got {}", effects.evasion_modifier());
    }

    #[test]
    fn test_burn_reduces_atk() {
        let mut effects = StatusEffects::new();
        // Burn reduces ATK by potency percentage (default potency is 15)
        // 1.0 - min(15/100, 0.9) = 0.85
        effects.add(StatusEffect::new(StatusType::Burn, 4, 15, None));
        
        // Burn affects ATK modifier via stat_modifier()
        let modifier = effects.atk_modifier();
        assert!((modifier - 0.85).abs() < 0.01, 
            "Expected 0.85, got {}", modifier);
    }
}
