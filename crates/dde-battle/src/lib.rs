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

/// Battle manager
pub struct BattleSystem {
    state: BattleState,
    combatants: Vec<Entity>,
    #[allow(dead_code)]
    turn_queue: Vec<Entity>,
}

impl BattleSystem {
    pub fn new() -> Self {
        Self {
            state: BattleState::Inactive,
            combatants: Vec::new(),
            turn_queue: Vec::new(),
        }
    }

    pub fn state(&self) -> BattleState {
        self.state
    }

    pub fn start_battle(&mut self, _world: &mut World, enemies: Vec<Entity>) {
        // TODO: Implement battle start
        self.state = BattleState::TransitionIn;
        self.combatants = enemies;
    }

    pub fn tick(&mut self, world: &mut World) {
        if self.state != BattleState::Active {
            return;
        }

        // Update ATB gauges
        for &entity in &self.combatants {
            if let Ok(atb) = world.query_one_mut::<&mut AtbGauge>(entity) {
                atb.tick();
            }
        }

        // Check for full gauges
        // TODO: Implement turn processing
    }

    pub fn end_battle(&mut self, result: BattleState) {
        self.state = result;
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
