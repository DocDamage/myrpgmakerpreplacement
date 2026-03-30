//! Battle system components

use serde::{Deserialize, Serialize};

use crate::Element;

/// Battle position (slot in battle formation)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct BattlePosition {
    pub slot: u8,
}

/// ATB (Active Time Battle) gauge
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct AtbGauge {
    /// Current value 0.0 - 100.0
    pub current: f32,
    /// Fill rate per tick
    pub rate: f32,
}

impl AtbGauge {
    /// Base rate for ATB fill
    pub const BASE_RATE: f32 = 5.0;

    /// Create new ATB gauge from speed stat
    #[must_use]
    pub fn from_speed(spd: i32) -> Self {
        let rate = Self::BASE_RATE * (1.0 + spd as f32 / 100.0);
        Self { current: 0.0, rate }
    }

    /// Check if gauge is full
    #[must_use]
    pub fn is_full(&self) -> bool {
        self.current >= 100.0
    }

    /// Reset gauge after action
    pub fn reset(&mut self) {
        self.current = 0.0;
    }

    /// Tick the gauge
    pub fn tick(&mut self) {
        self.current = (self.current + self.rate).min(100.0);
    }

    /// Apply haste effect (doubles rate)
    pub fn apply_haste(&mut self) {
        self.rate *= 2.0;
    }

    /// Apply slow effect (halves rate)
    pub fn apply_slow(&mut self) {
        self.rate *= 0.5;
    }

    /// Stop gauge (for stun effect)
    pub fn stop(&mut self) {
        self.rate = 0.0;
    }
}

/// Battle combatant flag
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Combatant;

/// Enemy AI behavior
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum AiBehavior {
    #[default]
    Aggressive,
    Defensive,
    Healer,
    Support,
    BossPhase,
}

/// Enemy template ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct EnemyTemplate {
    pub template_id: u32,
}

/// Skill reference
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SkillRef {
    pub skill_id: u32,
}

/// Damage info for combat calculations
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DamageInfo {
    pub amount: i32,
    pub element: Element,
    pub is_crit: bool,
    pub source: crate::Entity,
}

/// Loot table entry
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LootEntry {
    pub item_id: u32,
    pub drop_rate: f32,
    pub min_qty: u32,
    pub max_qty: u32,
}

/// Loot table component
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct LootTable {
    pub entries: Vec<LootEntry>,
    pub exp_reward: u32,
    pub gold_reward: u32,
}

/// Battle group membership
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct BattleGroup {
    pub group_id: u32,
    pub is_player_group: bool,
}

/// Level component for experience and leveling
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Level {
    pub level: i32,
    pub exp: i32,
    pub exp_to_next: i32,
}

impl Level {
    /// Create new level component
    pub fn new(level: i32) -> Self {
        Self {
            level,
            exp: 0,
            exp_to_next: Self::exp_for_level(level + 1),
        }
    }

    /// Calculate EXP needed for a level
    pub fn exp_for_level(level: i32) -> i32 {
        // Simple formula: level^2 * 100
        level * level * 100
    }

    /// Add EXP and check for level up
    pub fn add_exp(&mut self, amount: i32) -> bool {
        self.exp += amount;
        if self.exp >= self.exp_to_next {
            self.level_up();
            true
        } else {
            false
        }
    }

    /// Level up
    fn level_up(&mut self) {
        self.level += 1;
        self.exp -= self.exp_to_next;
        self.exp_to_next = Self::exp_for_level(self.level + 1);
    }
}

/// Defense buff component (applied by defend action)
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct DefenseBuff {
    /// Defense multiplier (e.g., 0.5 = half damage)
    pub defense_mult: f32,
    /// Turns remaining
    pub turns_remaining: u32,
}

impl DefenseBuff {
    /// Create new defense buff
    pub fn new(defense_mult: f32, turns: u32) -> Self {
        Self {
            defense_mult,
            turns_remaining: turns,
        }
    }

    /// Tick the buff (reduce turns)
    pub fn tick(&mut self) {
        self.turns_remaining = self.turns_remaining.saturating_sub(1);
    }

    /// Check if buff is active
    pub fn is_active(&self) -> bool {
        self.turns_remaining > 0
    }
}
