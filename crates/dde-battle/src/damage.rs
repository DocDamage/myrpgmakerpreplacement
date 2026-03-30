//! Damage calculation formulas
//!
//! Implements RPG-style damage calculations with:
//! - Attack vs Defense
//! - Elemental affinities
//! - Critical hits
//! - Variance/randomization

use dde_core::Element;
use dde_core::components::Stats;
use rand::Rng;

use crate::skills::{Skill, SkillType};

/// Damage calculation parameters
#[derive(Debug, Clone)]
pub struct DamageParams<'a> {
    /// Attacker's stats
    pub attacker: Stats,
    /// Defender's stats
    pub defender: Stats,
    /// Skill being used
    pub skill: &'a Skill,
    /// Attacker level
    pub attacker_level: i32,
    /// Defender level
    pub defender_level: i32,
    /// Critical hit chance bonus (0.0 - 1.0)
    pub crit_bonus: f32,
    /// Damage multiplier (from buffs/debuffs)
    pub damage_multiplier: f32,
}

/// Damage calculation result
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DamageResult {
    /// Final damage amount
    pub damage: i32,
    /// Whether it was a critical hit
    pub is_crit: bool,
    /// Whether the attack missed
    pub is_miss: bool,
    /// Elemental effectiveness (1.0 = normal, >1.0 = strong, <1.0 = weak)
    pub element_effectiveness: f32,
}

/// Calculate damage for a skill
pub fn calculate_damage<R: Rng>(
    params: &DamageParams<'_>,
    rng: &mut R,
) -> DamageResult {
    let skill = params.skill;
    // Check accuracy first
    let hit_roll: f32 = rng.gen();
    if hit_roll > params.skill.accuracy {
        return DamageResult {
            damage: 0,
            is_crit: false,
            is_miss: true,
            element_effectiveness: 1.0,
        };
    }
    
    // Calculate base damage based on skill type
    let base_damage = match params.skill.skill_type {
        SkillType::Physical => calculate_physical_damage(params),
        SkillType::Magic => calculate_magic_damage(params),
        SkillType::Hybrid => calculate_hybrid_damage(params),
        SkillType::Heal => -calculate_heal_amount(params), // Negative = healing
        SkillType::Support => 0.0, // No damage
        SkillType::Status => 0.0, // No damage
    };
    
    // Apply skill power multiplier
    let skill_power = params.skill.power as f32 / 100.0;
    let mut damage = base_damage * skill_power;
    
    // Apply elemental affinity
    let element_mult = get_elemental_multiplier(skill.element, &params.defender);
    damage *= element_mult;
    
    // Check for critical hit
    let crit_chance = calculate_crit_chance(params) + params.crit_bonus;
    let crit_roll: f32 = rng.gen();
    let is_crit = crit_roll < crit_chance;
    
    if is_crit {
        damage *= 1.5; // Critical hits do 1.5x damage
    }
    
    // Apply damage multiplier (buffs/debuffs)
    damage *= params.damage_multiplier;
    
    // Apply variance (±10% randomization)
    let variance: f32 = rng.gen_range(0.9..=1.1);
    damage *= variance;
    
    // Final damage
    let final_damage = damage.max(1.0) as i32;
    
    DamageResult {
        damage: if skill.skill_type == SkillType::Heal {
            -final_damage // Negative for healing
        } else {
            final_damage
        },
        is_crit,
        is_miss: false,
        element_effectiveness: element_mult,
    }
}

/// Calculate physical damage
fn calculate_physical_damage(params: &DamageParams) -> f32 {
    // Formula: (STR * 2 + Level) - (DEF)
    let attack = params.attacker.str as f32 * 2.0 + params.attacker_level as f32;
    let defense = params.defender.def as f32;
    
    (attack - defense * 0.5).max(1.0)
}

/// Calculate magic damage
fn calculate_magic_damage(params: &DamageParams) -> f32 {
    // Formula: (MAG * 2 + Level) - (MAG_DEF)
    // For simplicity, using DEF as magic defense in basic implementation
    let attack = params.attacker.mag as f32 * 2.0 + params.attacker_level as f32;
    let defense = params.defender.def as f32 * 0.8; // Magic typically penetrates armor
    
    (attack - defense * 0.5).max(1.0)
}

/// Calculate hybrid damage (uses average of STR and MAG)
fn calculate_hybrid_damage(params: &DamageParams) -> f32 {
    let avg_attack = (params.attacker.str + params.attacker.mag) as f32;
    let attack = avg_attack + params.attacker_level as f32;
    let defense = params.defender.def as f32 * 0.75;
    
    (attack - defense * 0.5).max(1.0)
}

/// Calculate heal amount
fn calculate_heal_amount(params: &DamageParams) -> f32 {
    // Formula: MAG * 3 + Level + Skill Power
    let heal = params.attacker.mag as f32 * 3.0 + params.attacker_level as f32;
    heal.max(1.0)
}

/// Calculate critical hit chance
fn calculate_crit_chance(params: &DamageParams) -> f32 {
    // Base 5% + luck bonus
    let base_chance = 0.05;
    let luck_bonus = params.attacker.luck as f32 / 200.0; // 0.5% per luck point
    (base_chance + luck_bonus).min(0.5) // Cap at 50%
}

/// Get elemental damage multiplier
pub fn get_elemental_multiplier(element: Element, _defender: &Stats) -> f32 {
    // This is a simplified version - in a full game, you'd have elemental affinities
    // per enemy type stored in a component
    match element {
        Element::None => 1.0,
        // Default multipliers when no special affinity
        _ => 1.0,
    }
}

/// Apply buff/debuff multiplier to a stat
pub fn apply_stat_modifier(base_stat: i32, multiplier: f32) -> i32 {
    (base_stat as f32 * multiplier) as i32
}

/// Calculate evasion chance
pub fn calculate_evasion(attacker_spd: i32, defender_spd: i32) -> f32 {
    // Base 5% evasion, plus speed difference
    let speed_diff = defender_spd - attacker_spd;
    let evasion = 0.05 + (speed_diff as f32 / 200.0);
    evasion.clamp(0.0, 0.5) // Cap between 0% and 50%
}

/// Damage preview for UI
#[derive(Debug, Clone)]
pub struct DamagePreview {
    pub min_damage: i32,
    pub max_damage: i32,
    pub crit_damage: i32,
    pub hit_chance: f32,
    pub crit_chance: f32,
    pub element_effectiveness: f32,
}

/// Generate damage preview for UI
pub fn preview_damage(params: &DamageParams<'_>) -> DamagePreview {
    let skill = params.skill;
    let base = match skill.skill_type {
        SkillType::Physical => calculate_physical_damage(params),
        SkillType::Magic => calculate_magic_damage(params),
        SkillType::Hybrid => calculate_hybrid_damage(params),
        _ => 0.0,
    };
    
    let skill_power = skill.power as f32 / 100.0;
    let element_mult = get_elemental_multiplier(skill.element, &params.defender);
    
    let min_base = base * skill_power * element_mult * 0.9; // -10% variance
    let max_base = base * skill_power * element_mult * 1.1; // +10% variance
    let crit_base = max_base * 1.5; // Crit multiplier
    
    let crit_chance = calculate_crit_chance(params) + params.crit_bonus;
    
    DamagePreview {
        min_damage: (min_base * params.damage_multiplier).max(1.0) as i32,
        max_damage: (max_base * params.damage_multiplier).max(1.0) as i32,
        crit_damage: (crit_base * params.damage_multiplier).max(1.0) as i32,
        hit_chance: params.skill.accuracy,
        crit_chance,
        element_effectiveness: element_mult,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skills::{Skill, SkillType, TargetType};
    
    fn test_stats() -> Stats {
        Stats {
            hp: 100,
            max_hp: 100,
            mp: 50,
            max_mp: 50,
            str: 20,
            def: 15,
            spd: 10,
            mag: 15,
            luck: 10,
            level: 1,
            exp: 0,
        }
    }
    
    fn basic_attack_skill() -> Skill {
        Skill {
            id: 1,
            name: "Attack".to_string(),
            description: "Basic attack".to_string(),
            skill_type: SkillType::Physical,
            target_type: TargetType::SingleEnemy,
            power: 100,
            accuracy: 1.0,
            element: Element::None,
            mp_cost: 0,
            tp_cost: 0,
            effects: vec![],
            cooldown: 0,
            animation_id: None,
            icon_id: 1,
        }
    }
    
    #[test]
    fn test_physical_damage() {
        let attacker = test_stats();
        let defender = test_stats();
        let skill = basic_attack_skill();
        
        let params = DamageParams {
            attacker,
            defender,
            skill: &skill,
            attacker_level: 10,
            defender_level: 10,
            crit_bonus: 0.0,
            damage_multiplier: 1.0,
        };
        
        let mut rng = rand::thread_rng();
        let result = calculate_damage(&params, &mut rng);
        
        // Should deal damage (not miss)
        assert!(!result.is_miss);
        assert!(result.damage > 0);
    }
    
    #[test]
    fn test_damage_variance() {
        let attacker = test_stats();
        let defender = test_stats();
        let skill = basic_attack_skill();
        
        let params = DamageParams {
            attacker,
            defender,
            skill: &skill,
            attacker_level: 10,
            defender_level: 10,
            crit_bonus: 0.0,
            damage_multiplier: 1.0,
        };
        
        let preview = preview_damage(&params);
        
        assert!(preview.min_damage <= preview.max_damage);
        assert!(preview.crit_damage > preview.max_damage);
    }
    
    #[test]
    fn test_accuracy_miss() {
        let attacker = test_stats();
        let defender = test_stats();
        let skill = Skill {
            accuracy: 0.0, // Never hits
            ..basic_attack_skill()
        };
        
        let params = DamageParams {
            attacker,
            defender,
            skill: &skill,
            attacker_level: 10,
            defender_level: 10,
            crit_bonus: 0.0,
            damage_multiplier: 1.0,
        };
        
        let mut rng = rand::thread_rng();
        let result = calculate_damage(&params, &mut rng);
        
        assert!(result.is_miss);
        assert_eq!(result.damage, 0);
    }
    
    #[test]
    fn test_stat_modifier() {
        assert_eq!(apply_stat_modifier(100, 1.5), 150);
        assert_eq!(apply_stat_modifier(100, 0.5), 50);
        assert_eq!(apply_stat_modifier(100, 1.0), 100);
    }
}
