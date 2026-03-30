//! Skill system for battles
//!
//! Defines skills, their effects, costs, and targeting.

use dde_core::{Element, Entity};
use serde::{Deserialize, Serialize};

/// Skill ID type
pub type SkillId = u32;

/// Skill definition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Skill {
    pub id: SkillId,
    pub name: String,
    pub description: String,
    pub skill_type: SkillType,
    pub target_type: TargetType,
    pub power: i32,
    pub accuracy: f32, // 0.0 - 1.0
    pub element: Element,
    pub mp_cost: i32,
    pub tp_cost: i32, // TP = Tactical Points (rage/momentum)
    pub effects: Vec<SkillEffect>,
    pub cooldown: u32, // Turns before can use again
    pub animation_id: Option<String>,
    pub icon_id: u32,
}

impl Skill {
    /// Create a basic attack skill
    pub fn basic_attack() -> Self {
        Self {
            id: 1,
            name: "Attack".to_string(),
            description: "A basic attack with your weapon".to_string(),
            skill_type: SkillType::Physical,
            target_type: TargetType::SingleEnemy,
            power: 100,
            accuracy: 0.95,
            element: Element::None,
            mp_cost: 0,
            tp_cost: 0,
            effects: vec![],
            cooldown: 0,
            animation_id: Some("attack".to_string()),
            icon_id: 1,
        }
    }
    
    /// Create a magic skill
    pub fn fireball() -> Self {
        Self {
            id: 2,
            name: "Fireball".to_string(),
            description: "Hurls a ball of fire at the enemy".to_string(),
            skill_type: SkillType::Magic,
            target_type: TargetType::SingleEnemy,
            power: 150,
            accuracy: 1.0,
            element: Element::Fire,
            mp_cost: 10,
            tp_cost: 0,
            effects: vec![SkillEffect::DamageOverTime {
                element: Element::Fire,
                power: 20,
                duration: 3,
            }],
            cooldown: 0,
            animation_id: Some("fireball".to_string()),
            icon_id: 2,
        }
    }
    
    /// Check if skill can target the given target type
    pub fn can_target(&self, target_type: TargetType) -> bool {
        match (self.target_type, target_type) {
            (TargetType::SelfOnly, TargetType::SelfOnly) => true,
            (TargetType::SingleAlly, TargetType::SingleAlly) => true,
            (TargetType::SingleAlly, TargetType::SelfOnly) => true, // Can target self as ally
            (TargetType::AllAllies, TargetType::SingleAlly) => true,
            (TargetType::AllAllies, TargetType::SelfOnly) => true,
            (TargetType::SingleEnemy, TargetType::SingleEnemy) => true,
            (TargetType::AllEnemies, TargetType::SingleEnemy) => true,
            (a, b) => a == b,
        }
    }
}

/// Type of skill
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillType {
    Physical,    // Uses STR
    Magic,       // Uses MAG
    Hybrid,      // Uses both STR and MAG
    Heal,        // Restores HP
    Support,     // Buffs/utility
    Status,      // Inflicts status effects
}

/// Targeting options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TargetType {
    SelfOnly,      // Caster only
    SingleAlly,    // One ally
    AllAllies,     // Whole party
    SingleEnemy,   // One enemy
    AllEnemies,    // All enemies
    RandomEnemy,   // Random enemy
}

/// Skill effect
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum SkillEffect {
    /// Heal target
    Heal { power: i32 },
    /// Restore MP
    RestoreMp { amount: i32 },
    /// Apply buff
    Buff {
        stat: StatType,
        multiplier: f32,
        duration: u32,
    },
    /// Apply debuff
    Debuff {
        stat: StatType,
        multiplier: f32,
        duration: u32,
    },
    /// Damage over time
    DamageOverTime {
        element: Element,
        power: i32,
        duration: u32,
    },
    /// Heal over time
    HealOverTime {
        power: i32,
        duration: u32,
    },
    /// Stun (prevents action)
    Stun { duration: u32 },
    /// Apply shield
    Shield { amount: i32, duration: u32 },
}

/// Stat types for buffs/debuffs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StatType {
    Strength,
    Defense,
    Magic,
    Speed,
    Luck,
}

/// Skill database
#[derive(Debug, Clone, Default)]
pub struct SkillDatabase {
    skills: Vec<Skill>,
    by_id: std::collections::HashMap<SkillId, usize>,
}

impl SkillDatabase {
    /// Create new skill database with default skills
    pub fn new() -> Self {
        let mut db = Self::default();
        db.register(Skill::basic_attack());
        db.register(Skill::fireball());
        db.register(Skill {
            id: 3,
            name: "Heal".to_string(),
            description: "Restores HP to one ally".to_string(),
            skill_type: SkillType::Heal,
            target_type: TargetType::SingleAlly,
            power: 200,
            accuracy: 1.0,
            element: Element::Holy,
            mp_cost: 8,
            tp_cost: 0,
            effects: vec![],
            cooldown: 0,
            animation_id: Some("heal".to_string()),
            icon_id: 3,
        });
        db.register(Skill {
            id: 4,
            name: "Thunder Strike".to_string(),
            description: "Calls down lightning on all enemies".to_string(),
            skill_type: SkillType::Magic,
            target_type: TargetType::AllEnemies,
            power: 120,
            accuracy: 0.9,
            element: Element::Lightning,
            mp_cost: 20,
            tp_cost: 0,
            effects: vec![SkillEffect::Stun { duration: 1 }],
            cooldown: 3,
            animation_id: Some("thunder".to_string()),
            icon_id: 4,
        });
        db.register(Skill {
            id: 5,
            name: "Power Attack".to_string(),
            description: "A powerful strike that deals 1.5x damage".to_string(),
            skill_type: SkillType::Physical,
            target_type: TargetType::SingleEnemy,
            power: 150,
            accuracy: 0.85,
            element: Element::None,
            mp_cost: 5,
            tp_cost: 10,
            effects: vec![],
            cooldown: 1,
            animation_id: Some("power_attack".to_string()),
            icon_id: 5,
        });
        db.register(Skill {
            id: 6,
            name: "Protect".to_string(),
            description: "Raises an ally's defense".to_string(),
            skill_type: SkillType::Support,
            target_type: TargetType::SingleAlly,
            power: 0,
            accuracy: 1.0,
            element: Element::None,
            mp_cost: 6,
            tp_cost: 0,
            effects: vec![SkillEffect::Buff {
                stat: StatType::Defense,
                multiplier: 1.5,
                duration: 3,
            }],
            cooldown: 0,
            animation_id: Some("buff".to_string()),
            icon_id: 6,
        });
        db
    }
    
    /// Register a skill
    pub fn register(&mut self, skill: Skill) {
        let index = self.skills.len();
        self.by_id.insert(skill.id, index);
        self.skills.push(skill);
    }
    
    /// Get skill by ID
    pub fn get(&self, id: SkillId) -> Option<&Skill> {
        self.by_id.get(&id).and_then(|&idx| self.skills.get(idx))
    }
    
    /// Get all skills
    pub fn all(&self) -> &[Skill] {
        &self.skills
    }
    
    /// Get skills by type
    pub fn by_type(&self, skill_type: SkillType) -> Vec<&Skill> {
        self.skills
            .iter()
            .filter(|s| s.skill_type == skill_type)
            .collect()
    }
}

/// Skill execution result
#[derive(Debug, Clone)]
pub struct SkillResult {
    pub success: bool,
    pub damage: i32,
    pub healing: i32,
    pub mp_restored: i32,
    pub is_crit: bool,
    pub effects_applied: Vec<AppliedEffect>,
    pub messages: Vec<String>,
}

impl SkillResult {
    pub fn failure(reason: &str) -> Self {
        Self {
            success: false,
            damage: 0,
            healing: 0,
            mp_restored: 0,
            is_crit: false,
            effects_applied: vec![],
            messages: vec![reason.to_string()],
        }
    }
    
    pub fn success() -> Self {
        Self {
            success: true,
            damage: 0,
            healing: 0,
            mp_restored: 0,
            is_crit: false,
            effects_applied: vec![],
            messages: vec![],
        }
    }
}

/// Applied effect info
#[derive(Debug, Clone)]
pub struct AppliedEffect {
    pub effect: SkillEffect,
    pub target: Entity,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_skill_database() {
        let db = SkillDatabase::new();
        
        // Check default skills exist
        assert!(db.get(1).is_some()); // Attack
        assert!(db.get(2).is_some()); // Fireball
        assert!(db.get(3).is_some()); // Heal
        
        // Check skill properties
        let attack = db.get(1).unwrap();
        assert_eq!(attack.name, "Attack");
        assert_eq!(attack.skill_type, SkillType::Physical);
        
        let fireball = db.get(2).unwrap();
        assert_eq!(fireball.element, Element::Fire);
    }
    
    #[test]
    fn test_target_type_compatibility() {
        let single_ally = Skill {
            id: 1,
            name: "Test".to_string(),
            description: "Test".to_string(),
            skill_type: SkillType::Heal,
            target_type: TargetType::SingleAlly,
            power: 100,
            accuracy: 1.0,
            element: Element::None,
            mp_cost: 0,
            tp_cost: 0,
            effects: vec![],
            cooldown: 0,
            animation_id: None,
            icon_id: 0,
        };
        
        assert!(single_ally.can_target(TargetType::SelfOnly));
        assert!(single_ally.can_target(TargetType::SingleAlly));
        assert!(!single_ally.can_target(TargetType::SingleEnemy));
    }
}
