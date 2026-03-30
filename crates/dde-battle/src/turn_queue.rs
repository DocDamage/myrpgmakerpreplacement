//! Turn queue management for ATB battle system
//!
//! Handles:
//! - ATB gauge updates
//! - Turn ordering
//! - Action queue
//! - Status effect ticks

use dde_core::{Entity, World};
use dde_core::components::battle::AtbGauge;
use serde::{Deserialize, Serialize};

use crate::skills::{Skill, SkillId};
use crate::status::{StatusEffects};

/// Turn queue manager
#[derive(Debug, Clone)]
pub struct TurnQueue {
    /// Combatants in the battle
    combatants: Vec<CombatantInfo>,
    /// Queue of ready combatants (ATB full)
    ready_queue: Vec<Entity>,
    /// Currently active combatant
    active_entity: Option<Entity>,
    /// Current turn number
    turn_number: u32,
    /// Action history
    action_history: Vec<ActionRecord>,
}

/// Combatant info tracked by turn queue
#[derive(Debug, Clone)]
pub struct CombatantInfo {
    pub entity: Entity,
    pub is_player: bool,
    pub is_alive: bool,
    /// Current ATB value (0-100)
    pub atb: f32,
    /// ATB fill rate per tick
    pub atb_rate: f32,
    /// Current status effects
    pub status_effects: Vec<StatusEffectInstance>,
    /// Cooldowns for skills (skill_id -> turns remaining)
    pub cooldowns: std::collections::HashMap<SkillId, u32>,
}

/// Status effect instance
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct StatusEffectInstance {
    pub effect_type: StatusEffectType,
    pub remaining_turns: u32,
    pub potency: i32,
}

/// Status effect types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StatusEffectType {
    Poison,
    Burn,
    Regen,
    Haste,
    Slow,
    Stun,
    Shield,
    AttackUp,
    DefenseUp,
    MagicUp,
    SpeedUp,
    AttackDown,
    DefenseDown,
    MagicDown,
    SpeedDown,
}

/// Battle action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BattleAction {
    pub actor: Entity,
    pub action_type: ActionType,
    pub target: Option<Entity>,
}

/// Type of battle action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionType {
    Attack,
    Skill(SkillId),
    Item(u32), // Item ID
    Defend,
    Flee,
}

/// Action record for history/log
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionRecord {
    pub turn: u32,
    pub actor: Entity,
    pub action: BattleAction,
    pub result_summary: String,
}

impl TurnQueue {
    /// Create new turn queue
    pub fn new() -> Self {
        Self {
            combatants: Vec::new(),
            ready_queue: Vec::new(),
            active_entity: None,
            turn_number: 0,
            action_history: Vec::new(),
        }
    }
    
    /// Add combatant to the queue
    pub fn add_combatant(&mut self, entity: Entity, is_player: bool, atb_rate: f32) {
        self.combatants.push(CombatantInfo {
            entity,
            is_player,
            is_alive: true,
            atb: if is_player { 0.0 } else { 0.0 }, // Players get slight advantage
            atb_rate,
            status_effects: Vec::new(),
            cooldowns: std::collections::HashMap::new(),
        });
    }
    
    /// Remove combatant from queue
    pub fn remove_combatant(&mut self, entity: Entity) {
        self.combatants.retain(|c| c.entity != entity);
        self.ready_queue.retain(|&e| e != entity);
    }
    
    /// Update all ATB gauges
    /// 
    /// Integrates with StatusEffects component for haste/slow/stop modifiers
    pub fn tick(&mut self, world: &mut World) {
        // Don't update if someone is taking their turn
        if self.active_entity.is_some() {
            return;
        }
        
        for combatant in &mut self.combatants {
            if !combatant.is_alive {
                continue;
            }
            
            // Check for stun (prevents ATB gain)
            if combatant.has_status(StatusEffectType::Stun) {
                continue;
            }
            
            // Get status effect ATB modifier from StatusEffects component
            let status_atb_mod = world.query_one::<&StatusEffects>(combatant.entity)
                .ok()
                .and_then(|mut q| q.get().cloned())
                .map(|se| se.atb_modifier())
                .unwrap_or(1.0);
            
            // Legacy status effect support (will be deprecated)
            let legacy_mod = 1.0;
            if combatant.has_status(StatusEffectType::Haste) {
                legacy_mod *= 2.0;
            }
            if combatant.has_status(StatusEffectType::Slow) {
                legacy_mod *= 0.5;
            }
            
            // Calculate final rate combining both systems
            let rate = combatant.atb_rate * status_atb_mod * legacy_mod;
            
            // Hard stop if StatusEffects says so
            if status_atb_mod == 0.0 {
                continue;
            }
            
            // Update ATB
            combatant.atb = (combatant.atb + rate).min(100.0);
            
            // Sync with component
            if let Ok(atb_comp) = world.query_one_mut::<&mut AtbGauge>(combatant.entity) {
                atb_comp.current = combatant.atb;
            }
            
            // Check if ready for action
            if combatant.atb >= 100.0 && !self.ready_queue.contains(&combatant.entity) {
                self.ready_queue.push(combatant.entity);
            }
        }
        
        // Process legacy status effects (damage over time, etc.)
        self.process_status_effects(world);
    }
    
    /// Get next ready combatant
    pub fn get_next_ready(&mut self) -> Option<Entity> {
        // Sort ready queue by ATB value (highest first)
        self.ready_queue.sort_by_key(|&entity| {
            let atb = self.combatants
                .iter()
                .find(|c| c.entity == entity)
                .map(|c| -(c.atb as i32))
                .unwrap_or(0);
            atb
        });
        
        self.ready_queue.first().copied()
    }
    
    /// Start a combatant's turn
    pub fn start_turn(&mut self, entity: Entity) -> bool {
        if !self.ready_queue.contains(&entity) {
            return false;
        }
        
        self.ready_queue.retain(|&e| e != entity);
        self.active_entity = Some(entity);
        self.turn_number += 1;
        
        // Reduce cooldowns
        if let Some(combatant) = self.combatants.iter_mut().find(|c| c.entity == entity) {
            for cooldown in combatant.cooldowns.values_mut() {
                *cooldown = cooldown.saturating_sub(1);
            }
            combatant.cooldowns.retain(|_, v| *v > 0);
        }
        
        true
    }
    
    /// End current turn
    pub fn end_turn(&mut self) {
        if let Some(entity) = self.active_entity {
            // Reset ATB
            if let Some(combatant) = self.combatants.iter_mut().find(|c| c.entity == entity) {
                combatant.atb = 0.0;
            }
        }
        
        self.active_entity = None;
    }
    
    /// Get active entity
    pub fn active_entity(&self) -> Option<Entity> {
        self.active_entity
    }
    
    /// Get current turn number
    pub fn turn_number(&self) -> u32 {
        self.turn_number
    }
    
    /// Check if entity can use a skill
    pub fn can_use_skill(&self, entity: Entity, skill: &Skill) -> Result<(), String> {
        let combatant = self.combatants
            .iter()
            .find(|c| c.entity == entity)
            .ok_or("Entity not in battle")?;
        
        // Check cooldown
        if let Some(&cooldown) = combatant.cooldowns.get(&skill.id) {
            if cooldown > 0 {
                return Err(format!("Skill on cooldown: {} turns remaining", cooldown));
            }
        }
        
        // Check if stunned
        if combatant.has_status(StatusEffectType::Stun) {
            return Err("Cannot act while stunned".to_string());
        }
        
        Ok(())
    }
    
    /// Apply skill cooldown
    pub fn apply_cooldown(&mut self, entity: Entity, skill_id: SkillId, cooldown: u32) {
        if let Some(combatant) = self.combatants.iter_mut().find(|c| c.entity == entity) {
            if cooldown > 0 {
                combatant.cooldowns.insert(skill_id, cooldown);
            }
        }
    }
    
    /// Apply status effect
    pub fn apply_status(&mut self, entity: Entity, effect: StatusEffectType, duration: u32, potency: i32) {
        if let Some(combatant) = self.combatants.iter_mut().find(|c| c.entity == entity) {
            // Remove existing effect of same type
            combatant.status_effects.retain(|e| e.effect_type != effect);
            
            combatant.status_effects.push(StatusEffectInstance {
                effect_type: effect,
                remaining_turns: duration,
                potency,
            });
        }
    }
    
    /// Remove status effect
    pub fn remove_status(&mut self, entity: Entity, effect: StatusEffectType) {
        if let Some(combatant) = self.combatants.iter_mut().find(|c| c.entity == entity) {
            combatant.status_effects.retain(|e| e.effect_type != effect);
        }
    }
    
    /// Process status effects (DoT, HoT, etc.)
    fn process_status_effects(&mut self, _world: &mut World) {
        for combatant in &mut self.combatants {
            if !combatant.is_alive {
                continue;
            }
            
            let mut damage = 0;
            let mut healing = 0;
            
            for effect in &mut combatant.status_effects {
                match effect.effect_type {
                    StatusEffectType::Poison | StatusEffectType::Burn => {
                        damage += effect.potency;
                    }
                    StatusEffectType::Regen => {
                        healing += effect.potency;
                    }
                    _ => {}
                }
                
                effect.remaining_turns = effect.remaining_turns.saturating_sub(1);
            }
            
            // Remove expired effects
            combatant.status_effects.retain(|e| e.remaining_turns > 0);
            
            // Apply damage/healing (would need HP component access here)
            // For now, just log it
            if damage > 0 {
                tracing::debug!("Entity {:?} takes {} DoT damage", combatant.entity, damage);
            }
            if healing > 0 {
                tracing::debug!("Entity {:?} heals {} from regen", combatant.entity, healing);
            }
        }
    }
    
    /// Mark combatant as defeated
    pub fn defeat_combatant(&mut self, entity: Entity) {
        if let Some(combatant) = self.combatants.iter_mut().find(|c| c.entity == entity) {
            combatant.is_alive = false;
            combatant.atb = 0.0;
        }
        self.ready_queue.retain(|&e| e != entity);
        
        if self.active_entity == Some(entity) {
            self.active_entity = None;
        }
    }
    
    /// Get all alive combatants
    pub fn alive_combatants(&self) -> Vec<Entity> {
        self.combatants
            .iter()
            .filter(|c| c.is_alive)
            .map(|c| c.entity)
            .collect()
    }
    
    /// Get all alive player combatants
    pub fn alive_players(&self) -> Vec<Entity> {
        self.combatants
            .iter()
            .filter(|c| c.is_alive && c.is_player)
            .map(|c| c.entity)
            .collect()
    }
    
    /// Get all alive enemy combatants
    pub fn alive_enemies(&self) -> Vec<Entity> {
        self.combatants
            .iter()
            .filter(|c| c.is_alive && !c.is_player)
            .map(|c| c.entity)
            .collect()
    }
    
    /// Check if all players are defeated
    pub fn all_players_defeated(&self) -> bool {
        self.alive_players().is_empty()
    }
    
    /// Check if all enemies are defeated
    pub fn all_enemies_defeated(&self) -> bool {
        self.alive_enemies().is_empty()
    }
    
    /// Get combatant info
    pub fn get_combatant(&self, entity: Entity) -> Option<&CombatantInfo> {
        self.combatants.iter().find(|c| c.entity == entity)
    }
    
    /// Get mutable combatant info
    pub fn get_combatant_mut(&mut self, entity: Entity) -> Option<&mut CombatantInfo> {
        self.combatants.iter_mut().find(|c| c.entity == entity)
    }
    
    /// Record action in history
    pub fn record_action(&mut self, action: BattleAction, result_summary: String) {
        if let Some(actor) = self.active_entity {
            self.action_history.push(ActionRecord {
                turn: self.turn_number,
                actor,
                action,
                result_summary,
            });
        }
    }
    
    /// Get action history
    pub fn action_history(&self) -> &[ActionRecord] {
        &self.action_history
    }
    
    /// Clear the queue (end of battle)
    pub fn clear(&mut self) {
        self.combatants.clear();
        self.ready_queue.clear();
        self.active_entity = None;
        self.turn_number = 0;
        self.action_history.clear();
    }
}

impl CombatantInfo {
    /// Check if combatant has a status effect
    pub fn has_status(&self, effect: StatusEffectType) -> bool {
        self.status_effects.iter().any(|e| e.effect_type == effect)
    }
    
    /// Get status effect potency
    pub fn get_status_potency(&self, effect: StatusEffectType) -> i32 {
        self.status_effects
            .iter()
            .find(|e| e.effect_type == effect)
            .map(|e| e.potency)
            .unwrap_or(0)
    }
}

impl Default for TurnQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    fn create_test_entity() -> (World, Entity) {
        let mut world = World::new();
        let entity = world.spawn(());
        (world, entity)
    }
    
    #[test]
    fn test_turn_queue_basic() {
        let mut queue = TurnQueue::new();
        let (_world, entity) = create_test_entity();
        
        queue.add_combatant(entity, true, 10.0);
        
        assert!(queue.get_combatant(entity).is_some());
        assert_eq!(queue.alive_combatants().len(), 1);
    }
    
    #[test]
    fn test_status_effects() {
        let mut queue = TurnQueue::new();
        let (_world, entity) = create_test_entity();
        
        queue.add_combatant(entity, true, 10.0);
        
        // Apply poison
        queue.apply_status(entity, StatusEffectType::Poison, 3, 10);
        
        let combatant = queue.get_combatant(entity).unwrap();
        assert!(combatant.has_status(StatusEffectType::Poison));
        assert_eq!(combatant.get_status_potency(StatusEffectType::Poison), 10);
        
        // Remove poison
        queue.remove_status(entity, StatusEffectType::Poison);
        
        let combatant = queue.get_combatant(entity).unwrap();
        assert!(!combatant.has_status(StatusEffectType::Poison));
    }
    
    #[test]
    fn test_defeat_combatant() {
        let mut queue = TurnQueue::new();
        let (_world, entity) = create_test_entity();
        
        queue.add_combatant(entity, true, 10.0);
        queue.defeat_combatant(entity);
        
        assert!(queue.all_players_defeated());
        assert!(queue.alive_combatants().is_empty());
    }
    
    #[test]
    fn test_cooldowns() {
        let mut queue = TurnQueue::new();
        let (_world, entity) = create_test_entity();
        
        queue.add_combatant(entity, true, 10.0);
        queue.apply_cooldown(entity, 1, 3);
        
        let combatant = queue.get_combatant(entity).unwrap();
        assert_eq!(combatant.cooldowns.get(&1), Some(&3));
        
        // Simulate turn start to reduce cooldown
        queue.ready_queue.push(entity);
        queue.start_turn(entity);
        
        let combatant = queue.get_combatant(entity).unwrap();
        assert_eq!(combatant.cooldowns.get(&1), Some(&2));
    }
}
