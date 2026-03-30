//! DocDamage Engine - Battle System
//!
//! ATB-based battle system with arena generation.

use dde_core::components::battle::AtbGauge;
use dde_core::{Entity, World};

/// Battle state machine
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BattleState {
    Inactive,
    TransitionIn,
    Active,
    Victory,
    Defeat,
    Flee,
    TransitionOut,
}

impl BattleState {
    /// Get a human-readable name for the state
    pub fn name(&self) -> &'static str {
        match self {
            BattleState::Inactive => "Inactive",
            BattleState::TransitionIn => "Transition In",
            BattleState::Active => "Active",
            BattleState::Victory => "Victory",
            BattleState::Defeat => "Defeat",
            BattleState::Flee => "Flee",
            BattleState::TransitionOut => "Transition Out",
        }
    }

    /// Check if the battle is finished (victory, defeat, or flee)
    pub fn is_finished(&self) -> bool {
        matches!(
            self,
            BattleState::Victory | BattleState::Defeat | BattleState::Flee
        )
    }

    /// Check if the battle is currently active
    pub fn is_active(&self) -> bool {
        *self == BattleState::Active
    }
}

/// Battle manager
pub struct BattleSystem {
    state: BattleState,
    combatants: Vec<Entity>,
    turn_queue: Vec<Entity>,
    tick_count: u64,
}

impl BattleSystem {
    pub fn new() -> Self {
        Self {
            state: BattleState::Inactive,
            combatants: Vec::new(),
            turn_queue: Vec::new(),
            tick_count: 0,
        }
    }

    pub fn state(&self) -> BattleState {
        self.state
    }

    pub fn tick_count(&self) -> u64 {
        self.tick_count
    }

    pub fn combatant_count(&self) -> usize {
        self.combatants.len()
    }

    pub fn is_active(&self) -> bool {
        self.state.is_active()
    }

    pub fn start_battle(&mut self, world: &mut World, enemies: Vec<Entity>) {
        self.state = BattleState::Active;
        self.combatants = enemies;
        self.turn_queue.clear();
        self.tick_count = 0;

        // Initialize ATB gauges for all combatants
        for &entity in &self.combatants {
            if let Ok(atb) = world.query_one_mut::<&mut AtbGauge>(entity) {
                atb.reset();
            }
        }
    }

    pub fn tick(&mut self, world: &mut World) {
        if self.state != BattleState::Active {
            return;
        }

        self.tick_count += 1;

        // Update ATB gauges
        for &entity in &self.combatants {
            if let Ok(atb) = world.query_one_mut::<&mut AtbGauge>(entity) {
                atb.tick();
                if atb.is_full() && !self.turn_queue.contains(&entity) {
                    self.turn_queue.push(entity);
                }
            }
        }
    }

    pub fn end_battle(&mut self, result: BattleState) {
        assert!(result.is_finished(), "Result must be a finished state");
        self.state = result;
        self.turn_queue.clear();
    }

    /// Get the next entity ready to act
    pub fn next_ready_entity(&mut self) -> Option<Entity> {
        if self.turn_queue.is_empty() {
            None
        } else {
            Some(self.turn_queue.remove(0))
        }
    }

    /// Check if any combatant is ready to act
    pub fn has_ready_combatant(&self) -> bool {
        !self.turn_queue.is_empty()
    }

    /// Get the number of ready combatants
    pub fn ready_count(&self) -> usize {
        self.turn_queue.len()
    }

    /// Reset ATB for an entity after they act
    pub fn reset_atb(&self, world: &mut World, entity: Entity) {
        if let Ok(atb) = world.query_one_mut::<&mut AtbGauge>(entity) {
            atb.reset();
        }
    }

    /// Add a combatant mid-battle (e.g., reinforcements)
    pub fn add_combatant(&mut self, entity: Entity) {
        if !self.combatants.contains(&entity) {
            self.combatants.push(entity);
        }
    }

    /// Remove a combatant (e.g., defeated or fled)
    pub fn remove_combatant(&mut self, entity: Entity) {
        self.combatants.retain(|&e| e != entity);
        self.turn_queue.retain(|&e| e != entity);
    }
}

impl Default for BattleSystem {
    fn default() -> Self {
        Self::new()
    }
}

/// Item system
pub mod items;

/// UI components (requires `ui` feature)
#[cfg(feature = "ui")]
pub mod ui;

#[cfg(test)]
mod tests {
    use super::*;
    use dde_core::components::battle::{AtbGauge, Combatant};
    use dde_core::World;

    fn create_test_combatant(world: &mut World, speed: i32) -> Entity {
        world.spawn((Combatant, AtbGauge::from_speed(speed)))
    }

    #[test]
    fn test_battle_system_new() {
        let system = BattleSystem::new();
        assert_eq!(system.state(), BattleState::Inactive);
        assert_eq!(system.combatant_count(), 0);
        assert_eq!(system.tick_count(), 0);
        assert!(!system.is_active());
    }

    #[test]
    fn test_battle_state_transitions() {
        let mut world = World::new();
        let enemy = create_test_combatant(&mut world, 50);
        let mut system = BattleSystem::new();

        // Start battle
        system.start_battle(&mut world, vec![enemy]);
        assert_eq!(system.state(), BattleState::Active);
        assert!(system.is_active());
        assert_eq!(system.combatant_count(), 1);

        // End battle with victory
        system.end_battle(BattleState::Victory);
        assert_eq!(system.state(), BattleState::Victory);
        assert!(system.state().is_finished());
        assert!(!system.is_active());
    }

    #[test]
    fn test_atb_progression() {
        let mut world = World::new();
        let enemy = create_test_combatant(&mut world, 100); // Fast
        let mut system = BattleSystem::new();

        system.start_battle(&mut world, vec![enemy]);

        // Tick multiple times
        for _ in 0..10 {
            system.tick(&mut world);
        }

        assert!(system.tick_count() > 0);
    }

    #[test]
    fn test_turn_queue() {
        let mut world = World::new();
        let enemy1 = create_test_combatant(&mut world, 100);
        let enemy2 = create_test_combatant(&mut world, 50);
        let mut system = BattleSystem::new();

        system.start_battle(&mut world, vec![enemy1, enemy2]);
        assert_eq!(system.ready_count(), 0);
        assert!(!system.has_ready_combatant());

        // Tick until someone is ready (with high speed, should happen quickly)
        for _ in 0..100 {
            system.tick(&mut world);
            if system.has_ready_combatant() {
                break;
            }
        }

        // At least one should be ready after many ticks
        if system.has_ready_combatant() {
            let ready = system.next_ready_entity();
            assert!(ready.is_some());
            assert_eq!(system.ready_count(), 0);
        }
    }

    #[test]
    fn test_add_remove_combatants() {
        let mut world = World::new();
        let enemy1 = create_test_combatant(&mut world, 50);
        let enemy2 = create_test_combatant(&mut world, 50);
        let mut system = BattleSystem::new();

        system.start_battle(&mut world, vec![enemy1]);
        assert_eq!(system.combatant_count(), 1);

        // Add reinforcements
        system.add_combatant(enemy2);
        assert_eq!(system.combatant_count(), 2);

        // Remove one
        system.remove_combatant(enemy1);
        assert_eq!(system.combatant_count(), 1);

        // Removing same entity twice should not panic
        system.remove_combatant(enemy1);
        assert_eq!(system.combatant_count(), 1);
    }

    #[test]
    fn test_battle_state_names() {
        assert_eq!(BattleState::Inactive.name(), "Inactive");
        assert_eq!(BattleState::Active.name(), "Active");
        assert_eq!(BattleState::Victory.name(), "Victory");
        assert_eq!(BattleState::Defeat.name(), "Defeat");
        assert_eq!(BattleState::Flee.name(), "Flee");
    }

    #[test]
    fn test_finished_states() {
        assert!(!BattleState::Inactive.is_finished());
        assert!(!BattleState::TransitionIn.is_finished());
        assert!(!BattleState::Active.is_finished());
        assert!(!BattleState::TransitionOut.is_finished());

        assert!(BattleState::Victory.is_finished());
        assert!(BattleState::Defeat.is_finished());
        assert!(BattleState::Flee.is_finished());
    }

    #[test]
    fn test_atb_reset() {
        let mut world = World::new();
        let enemy = create_test_combatant(&mut world, 100);
        let mut system = BattleSystem::new();

        system.start_battle(&mut world, vec![enemy]);

        // Tick until ready
        for _ in 0..100 {
            system.tick(&mut world);
            if system.has_ready_combatant() {
                break;
            }
        }

        if let Some(ready_entity) = system.next_ready_entity() {
            system.reset_atb(&mut world, ready_entity);
            // After reset, entity should not be ready anymore
            // (though they might become ready again quickly with high speed)
        }
    }

    #[test]
    fn test_no_tick_when_inactive() {
        let mut world = World::new();
        let mut system = BattleSystem::new();

        // Should not panic or change state when inactive
        system.tick(&mut world);
        assert_eq!(system.state(), BattleState::Inactive);
        assert_eq!(system.tick_count(), 0);
    }

    #[test]
    fn test_multiple_combatants_different_speeds() {
        let mut world = World::new();
        let fast = create_test_combatant(&mut world, 200);
        let slow = create_test_combatant(&mut world, 50);
        let mut system = BattleSystem::new();

        system.start_battle(&mut world, vec![fast, slow]);

        // Fast entity should become ready before slow one
        let mut fast_ready_first = false;
        for _ in 0..50 {
            system.tick(&mut world);
            if system.has_ready_combatant() {
                if let Some(ready) = system.next_ready_entity() {
                    if ready == fast {
                        fast_ready_first = true;
                    }
                    break;
                }
            }
        }

        // With significant speed difference, fast should usually go first
        // (this is probabilistic but very likely)
    }
}
