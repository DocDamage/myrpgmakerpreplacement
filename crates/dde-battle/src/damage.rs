//! Damage calculation formulas
//!
//! Implements RPG-style damage calculations with:
//! - Attack vs Defense
//! - Elemental affinities
//! - Critical hits
//! - Variance/randomization
//! - Custom formula support via FormulaResource

use dde_core::components::Stats;
use dde_core::resources::formula::{CombatantStats, FormulaContext, FormulaResource};
use dde_core::Element;
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

/// Calculate damage for a skill using default formulas
pub fn calculate_damage<R: Rng>(
    params: &DamageParams<'_>,
    rng: &mut R,
) -> DamageResult {
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
        SkillType::Support => 0.0,                         // No damage
        SkillType::Status => 0.0,                          // No damage
    };

    // Apply skill power multiplier
    let skill_power = params.skill.power as f32 / 100.0;
    let mut damage = base_damage * skill_power;

    // Apply elemental affinity
    let element_mult = get_elemental_multiplier(params.skill.element, &params.defender);
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
        damage: if params.skill.skill_type == SkillType::Heal {
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

/// Damage calculator that uses custom formulas from FormulaResource
#[derive(Debug, Clone)]
pub struct DamageCalculator {
    formula_resource: FormulaResource,
}

impl DamageCalculator {
    /// Create a new damage calculator with the given formula resource
    pub fn new(formula_resource: FormulaResource) -> Self {
        Self { formula_resource }
    }

    /// Create with default formulas
    pub fn default() -> Self {
        Self::new(FormulaResource::default())
    }

    /// Calculate damage using the custom damage formula
    pub fn calculate_damage<R: Rng>(
        &self,
        attacker: &Stats,
        defender: &Stats,
        skill: &Skill,
        rng: &mut R,
    ) -> DamageResult {
        // Check accuracy first
        let hit_roll: f32 = rng.gen();
        if hit_roll > skill.accuracy {
            return DamageResult {
                damage: 0,
                is_crit: false,
                is_miss: true,
                element_effectiveness: 1.0,
            };
        }

        // Build context for formula evaluation
        let ctx = self.build_context(attacker, defender, skill);

        // Evaluate the damage formula
        let formula = &self.formula_resource.damage;
        let base_damage = match self.evaluate_formula(formula, &ctx, rng) {
            Ok(val) => val.max(1.0), // Minimum 1 damage
            Err(e) => {
                tracing::warn!("Damage formula error: {}, using default", e);
                // Fallback to default calculation
                calculate_default_damage(attacker, defender, attacker.level, skill)
            }
        };

        // Apply skill power multiplier
        let skill_power = skill.power as f32 / 100.0;
        let mut damage = base_damage * skill_power;

        // Apply elemental affinity
        let element_mult = get_elemental_multiplier(skill.element, defender);
        damage *= element_mult;

        // Check for critical hit using custom formula
        let crit_chance = self.calculate_crit_chance(attacker, rng);
        let crit_roll: f32 = rng.gen();
        let is_crit = crit_roll < crit_chance;

        if is_crit {
            damage *= 1.5; // Critical hits do 1.5x damage
        }

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

    /// Calculate healing using the custom healing formula
    pub fn calculate_healing<R: Rng>(
        &self,
        healer: &Stats,
        target: &Stats,
        skill: &Skill,
        rng: &mut R,
    ) -> i32 {
        let ctx = self.build_context(healer, target, skill);

        let formula = &self.formula_resource.healing;
        let base_heal = match self.evaluate_formula(formula, &ctx, rng) {
            Ok(val) => val.max(1.0),
            Err(e) => {
                tracing::warn!("Healing formula error: {}, using default", e);
                (healer.mag as f32 * 3.0 + healer.level as f32).max(1.0)
            }
        };

        // Apply skill power multiplier
        let skill_power = skill.power as f32 / 100.0;
        let mut healing = base_heal * skill_power;

        // Apply variance (±10%)
        let variance: f32 = rng.gen_range(0.9..=1.1);
        healing *= variance;

        healing.max(1.0) as i32
    }

    /// Calculate critical hit chance using the custom formula
    pub fn calculate_crit_chance<R: Rng>(&self, attacker: &Stats, rng: &mut R) -> f32 {
        // Create a minimal context for crit calculation
        let ctx = FormulaContext {
            attacker: CombatantStats::from_core_stats(attacker),
            defender: CombatantStats::default(),
            skill_power: 100,
            skill_accuracy: 1.0,
        };

        let formula = &self.formula_resource.critical;
        match self.evaluate_formula(formula, &ctx, rng) {
            Ok(val) => (val as f32).clamp(0.0, 1.0), // Clamp between 0 and 1
            Err(e) => {
                tracing::warn!("Critical hit formula error: {}, using default", e);
                // Default: 5% + luck bonus
                let base_chance = 0.05;
                let luck_bonus = attacker.luck as f32 / 200.0;
                (base_chance + luck_bonus).min(0.5)
            }
        }
    }

    /// Calculate status effect apply chance
    pub fn calculate_status_chance<R: Rng>(
        &self,
        attacker: &Stats,
        defender: &Stats,
        rng: &mut R,
    ) -> f32 {
        // Create a skill context for status calculation
        let ctx = FormulaContext {
            attacker: CombatantStats::from_core_stats(attacker),
            defender: CombatantStats::from_core_stats(defender),
            skill_power: 100,
            skill_accuracy: 1.0,
        };

        let formula = &self.formula_resource.status_apply;
        match self.evaluate_formula(formula, &ctx, rng) {
            Ok(val) => (val as f32).clamp(0.0, 1.0),
            Err(e) => {
                tracing::warn!("Status apply formula error: {}, using default", e);
                // Default: (MAG difference + 50) / 100
                let mag_diff = attacker.mag - defender.mag;
                ((mag_diff + 50) as f32 / 100.0).clamp(0.0, 1.0)
            }
        }
    }

    /// Calculate flee/escape chance
    pub fn calculate_flee_chance<R: Rng>(
        &self,
        attacker: &Stats,
        defender: &Stats,
        rng: &mut R,
    ) -> f32 {
        let ctx = FormulaContext {
            attacker: CombatantStats::from_core_stats(attacker),
            defender: CombatantStats::from_core_stats(defender),
            skill_power: 100,
            skill_accuracy: 1.0,
        };

        let formula = &self.formula_resource.flee;
        match self.evaluate_formula(formula, &ctx, rng) {
            Ok(val) => (val as f32).clamp(0.0, 1.0),
            Err(e) => {
                tracing::warn!("Flee formula error: {}, using default", e);
                // Default: 50% + speed difference / 100
                let spd_diff = attacker.spd - defender.spd;
                (0.5 + spd_diff as f32 / 100.0).clamp(0.0, 1.0)
            }
        }
    }

    /// Build formula context from stats and skill
    fn build_context(&self, attacker: &Stats, defender: &Stats, skill: &Skill) -> FormulaContext {
        FormulaContext {
            attacker: CombatantStats::from_core_stats(attacker),
            defender: CombatantStats::from_core_stats(defender),
            skill_power: skill.power,
            skill_accuracy: skill.accuracy,
        }
    }

    /// Evaluate a formula string with the given context
    fn evaluate_formula<R: Rng>(
        &self,
        formula: &str,
        ctx: &FormulaContext,
        rng: &mut R,
    ) -> Result<f32, String> {
        // Replace variables with values
        let mut expr = formula.to_string();

        // Attacker stats
        expr = expr.replace("attacker.str", &ctx.attacker.str.to_string());
        expr = expr.replace("attacker.def", &ctx.attacker.def.to_string());
        expr = expr.replace("attacker.spd", &ctx.attacker.spd.to_string());
        expr = expr.replace("attacker.mag", &ctx.attacker.mag.to_string());
        expr = expr.replace("attacker.luck", &ctx.attacker.luck.to_string());
        expr = expr.replace("attacker.level", &ctx.attacker.level.to_string());
        expr = expr.replace("attacker.hp", &ctx.attacker.hp.to_string());
        expr = expr.replace("attacker.max_hp", &ctx.attacker.max_hp.to_string());

        // Defender stats
        expr = expr.replace("defender.str", &ctx.defender.str.to_string());
        expr = expr.replace("defender.def", &ctx.defender.def.to_string());
        expr = expr.replace("defender.spd", &ctx.defender.spd.to_string());
        expr = expr.replace("defender.mag", &ctx.defender.mag.to_string());
        expr = expr.replace("defender.luck", &ctx.defender.luck.to_string());
        expr = expr.replace("defender.level", &ctx.defender.level.to_string());
        expr = expr.replace("defender.hp", &ctx.defender.hp.to_string());
        expr = expr.replace("defender.max_hp", &ctx.defender.max_hp.to_string());

        // Skill stats
        expr = expr.replace("skill.power", &ctx.skill_power.to_string());
        expr = expr.replace("skill.accuracy", &ctx.skill_accuracy.to_string());

        // Replace random() calls
        while expr.contains("random()") {
            let val: f64 = rng.gen();
            expr = expr.replacen("random()", &format!("{:.6}", val), 1);
        }

        // Replace random(min, max) calls
        while let Some(start) = expr.find("random(") {
            if let Some(end) = expr[start..].find(')') {
                let full_call = &expr[start..start + end + 1];
                let args = &full_call[7..full_call.len() - 1]; // Remove "random(" and ")"
                let parts: Vec<&str> = args.split(',').map(|s| s.trim()).collect();

                if parts.len() == 2 {
                    if let (Ok(min), Ok(max)) = (parts[0].parse::<f64>(), parts[1].parse::<f64>()) {
                        let val: f64 = rng.gen_range(min..=max);
                        expr = expr.replacen(full_call, &format!("{:.6}", val), 1);
                    } else {
                        return Err(format!("Invalid random() arguments: {}", full_call));
                    }
                } else {
                    return Err(format!("Invalid random() call: {}", full_call));
                }
            } else {
                return Err("Unclosed random() call".to_string());
            }
        }

        // Evaluate the expression
        evaluate_expression(&expr)
    }

    /// Run simulation with current formulas
    pub fn run_simulation<R: Rng>(
        &self,
        attacker: &Stats,
        defender: &Stats,
        skill: &Skill,
        iterations: usize,
        rng: &mut R,
    ) -> SimulationResult {
        let mut values = Vec::with_capacity(iterations);
        let mut crit_count = 0;

        for _ in 0..iterations {
            let result = self.calculate_damage(attacker, defender, skill, rng);
            values.push(result.damage as f64);
            if result.is_crit {
                crit_count += 1;
            }
        }

        SimulationResult::from_values(values, crit_count, iterations)
    }

    /// Get the formula resource
    pub fn formula_resource(&self) -> &FormulaResource {
        &self.formula_resource
    }

    /// Update the formula resource
    pub fn set_formula_resource(&mut self, resource: FormulaResource) {
        self.formula_resource = resource;
    }
}

impl Default for DamageCalculator {
    fn default() -> Self {
        Self::new(FormulaResource::default())
    }
}

/// Simulation results
#[derive(Debug, Clone, Default)]
pub struct SimulationResult {
    pub min_value: f64,
    pub max_value: f64,
    pub avg_value: f64,
    pub median_value: f64,
    pub std_deviation: f64,
    pub crit_rate: f64,
    pub distribution: Vec<(f64, u32)>,
    pub iterations: usize,
}

impl SimulationResult {
    fn from_values(mut values: Vec<f64>, crit_count: usize, iterations: usize) -> Self {
        if values.is_empty() {
            return Self::default();
        }

        values.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let min_value = values[0];
        let max_value = values[values.len() - 1];
        let avg_value = values.iter().sum::<f64>() / values.len() as f64;
        let median_value = if values.len() % 2 == 0 {
            (values[values.len() / 2 - 1] + values[values.len() / 2]) / 2.0
        } else {
            values[values.len() / 2]
        };

        // Standard deviation
        let variance = values
            .iter()
            .map(|v| (*v - avg_value).powi(2))
            .sum::<f64>()
            / values.len() as f64;
        let std_deviation = variance.sqrt();

        // Build distribution histogram (10 bins)
        let distribution = build_histogram(&values, 10);

        Self {
            min_value,
            max_value,
            avg_value,
            median_value,
            std_deviation,
            crit_rate: crit_count as f64 / iterations as f64,
            distribution,
            iterations: values.len(),
        }
    }
}

/// Build histogram distribution
fn build_histogram(values: &[f64], bins: usize) -> Vec<(f64, u32)> {
    if values.is_empty() || bins == 0 {
        return Vec::new();
    }

    let min = values[0];
    let max = values[values.len() - 1];

    if min == max {
        return vec![(min, values.len() as u32)];
    }

    let range = max - min;
    let bin_width = range / bins as f64;
    let mut counts = vec![0u32; bins];

    for value in values {
        let bin = (((value - min) / range) * (bins - 1) as f64).round() as usize;
        let bin = bin.min(bins - 1);
        counts[bin] += 1;
    }

    (0..bins)
        .map(|i| (min + bin_width * i as f64 + bin_width / 2.0, counts[i]))
        .collect()
}

/// Calculate default damage (fallback when formula fails)
fn calculate_default_damage(
    attacker: &Stats,
    defender: &Stats,
    attacker_level: i32,
    skill: &Skill,
) -> f32 {
    match skill.skill_type {
        SkillType::Physical => {
            let attack = attacker.str as f32 * 2.0 + attacker_level as f32;
            let defense = defender.def as f32;
            (attack - defense * 0.5).max(1.0)
        }
        SkillType::Magic => {
            let attack = attacker.mag as f32 * 2.0 + attacker_level as f32;
            let defense = defender.def as f32 * 0.8;
            (attack - defense * 0.5).max(1.0)
        }
        SkillType::Hybrid => {
            let avg_attack = (attacker.str + attacker.mag) as f32;
            let attack = avg_attack + attacker_level as f32;
            let defense = defender.def as f32 * 0.75;
            (attack - defense * 0.5).max(1.0)
        }
        _ => 0.0,
    }
}

/// Evaluate a mathematical expression
fn evaluate_expression(expr: &str) -> Result<f32, String> {
    // Handle min/max/clamp functions first
    if let Some(result) = evaluate_function(expr)? {
        return Ok(result);
    }

    // Parse and evaluate the expression
    let mut chars = expr.chars().peekable();
    parse_expression(&mut chars)
}

/// Evaluate built-in functions
fn evaluate_function(expr: &str) -> Result<Option<f32>, String> {
    let trimmed = expr.trim();

    // min(a, b)
    if trimmed.starts_with("min(") && trimmed.ends_with(')') {
        let args = &trimmed[4..trimmed.len() - 1];
        let parts: Vec<&str> = args.split(',').map(|s| s.trim()).collect();
        if parts.len() == 2 {
            let a = evaluate_expression(parts[0])?;
            let b = evaluate_expression(parts[1])?;
            return Ok(Some(a.min(b)));
        }
    }

    // max(a, b)
    if trimmed.starts_with("max(") && trimmed.ends_with(')') {
        let args = &trimmed[4..trimmed.len() - 1];
        let parts: Vec<&str> = args.split(',').map(|s| s.trim()).collect();
        if parts.len() == 2 {
            let a = evaluate_expression(parts[0])?;
            let b = evaluate_expression(parts[1])?;
            return Ok(Some(a.max(b)));
        }
    }

    // clamp(val, min, max)
    if trimmed.starts_with("clamp(") && trimmed.ends_with(')') {
        let args = &trimmed[6..trimmed.len() - 1];
        let parts: Vec<&str> = args.split(',').map(|s| s.trim()).collect();
        if parts.len() == 3 {
            let val = evaluate_expression(parts[0])?;
            let min = evaluate_expression(parts[1])?;
            let max = evaluate_expression(parts[2])?;
            return Ok(Some(val.clamp(min, max)));
        }
    }

    // abs(val)
    if trimmed.starts_with("abs(") && trimmed.ends_with(')') {
        let args = &trimmed[4..trimmed.len() - 1];
        let val = evaluate_expression(args.trim())?;
        return Ok(Some(val.abs()));
    }

    // sqrt(val)
    if trimmed.starts_with("sqrt(") && trimmed.ends_with(')') {
        let args = &trimmed[5..trimmed.len() - 1];
        let val = evaluate_expression(args.trim())?;
        return Ok(Some(val.sqrt()));
    }

    // pow(base, exp)
    if trimmed.starts_with("pow(") && trimmed.ends_with(')') {
        let args = &trimmed[4..trimmed.len() - 1];
        let parts: Vec<&str> = args.split(',').map(|s| s.trim()).collect();
        if parts.len() == 2 {
            let base = evaluate_expression(parts[0])?;
            let exp = evaluate_expression(parts[1])?;
            return Ok(Some(base.powf(exp)));
        }
    }

    Ok(None)
}

/// Parse and evaluate an expression
fn parse_expression(chars: &mut std::iter::Peekable<std::str::Chars>) -> Result<f32, String> {
    parse_addition_subtraction(chars)
}

fn skip_whitespace(chars: &mut std::iter::Peekable<std::str::Chars>) {
    while let Some(&ch) = chars.peek() {
        if ch.is_whitespace() {
            chars.next();
        } else {
            break;
        }
    }
}

fn parse_addition_subtraction(
    chars: &mut std::iter::Peekable<std::str::Chars>,
) -> Result<f32, String> {
    let mut left = parse_multiplication_division(chars)?;

    loop {
        skip_whitespace(chars);
        match chars.peek() {
            Some(&'+') => {
                chars.next();
                let right = parse_multiplication_division(chars)?;
                left += right;
            }
            Some(&'-') => {
                chars.next();
                let right = parse_multiplication_division(chars)?;
                left -= right;
            }
            _ => break,
        }
    }

    Ok(left)
}

fn parse_multiplication_division(
    chars: &mut std::iter::Peekable<std::str::Chars>,
) -> Result<f32, String> {
    let mut left = parse_power(chars)?;

    loop {
        skip_whitespace(chars);
        match chars.peek() {
            Some(&'*') => {
                chars.next();
                let right = parse_power(chars)?;
                left *= right;
            }
            Some(&'/') => {
                chars.next();
                let right = parse_power(chars)?;
                if right == 0.0 {
                    return Err("Division by zero".to_string());
                }
                left /= right;
            }
            Some(&'%') => {
                chars.next();
                let right = parse_power(chars)?;
                if right == 0.0 {
                    return Err("Modulo by zero".to_string());
                }
                left %= right;
            }
            _ => break,
        }
    }

    Ok(left)
}

fn parse_power(chars: &mut std::iter::Peekable<std::str::Chars>) -> Result<f32, String> {
    let base = parse_unary(chars)?;

    skip_whitespace(chars);
    if let Some(&'^') = chars.peek() {
        chars.next();
        let exp = parse_power(chars)?;
        Ok(base.powf(exp))
    } else {
        Ok(base)
    }
}

fn parse_unary(chars: &mut std::iter::Peekable<std::str::Chars>) -> Result<f32, String> {
    skip_whitespace(chars);
    if let Some(&ch) = chars.peek() {
        match ch {
            '-' => {
                chars.next();
                Ok(-parse_primary(chars)?)
            }
            '+' => {
                chars.next();
                parse_primary(chars)
            }
            _ => parse_primary(chars),
        }
    } else {
        Err("Unexpected end of expression".to_string())
    }
}

fn parse_primary(chars: &mut std::iter::Peekable<std::str::Chars>) -> Result<f32, String> {
    // Skip whitespace
    while let Some(&ch) = chars.peek() {
        if ch.is_whitespace() {
            chars.next();
        } else {
            break;
        }
    }

    if let Some(&ch) = chars.peek() {
        if ch == '(' {
            chars.next();
            let value = parse_expression(chars)?;

            // Expect closing parenthesis
            if let Some(&')') = chars.peek() {
                chars.next();
                Ok(value)
            } else {
                Err("Missing closing parenthesis".to_string())
            }
        } else if ch.is_ascii_digit() || ch == '.' {
            // Parse number
            let mut num_str = String::new();
            while let Some(&c) = chars.peek() {
                if c.is_ascii_digit() || c == '.' {
                    num_str.push(c);
                    chars.next();
                } else {
                    break;
                }
            }
            num_str
                .parse::<f32>()
                .map_err(|e| format!("Invalid number '{}': {}", num_str, e))
        } else {
            Err(format!("Unexpected character: {}", ch))
        }
    } else {
        Err("Unexpected end of expression".to_string())
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

    #[test]
    fn test_damage_calculator_default() {
        let calculator = DamageCalculator::default();
        assert!(!calculator.formula_resource().damage.is_empty());
    }

    #[test]
    fn test_damage_calculator_with_custom_formula() {
        let mut formulas = FormulaResource::default();
        formulas.damage = "attacker.str * 2".to_string();

        let calculator = DamageCalculator::new(formulas);
        let attacker = test_stats();
        let defender = test_stats();
        let skill = basic_attack_skill();

        let mut rng = rand::thread_rng();
        let result = calculator.calculate_damage(&attacker, &defender, &skill, &mut rng);

        // With formula "attacker.str * 2", base damage should be 40
        // Then multiplied by skill power (1.0) and variance
        assert!(result.damage > 0);
    }

    #[test]
    fn test_crit_chance_calculation() {
        let formulas = FormulaResource::default();
        let calculator = DamageCalculator::new(formulas);
        let attacker = test_stats();

        let mut rng = rand::thread_rng();
        let crit_chance = calculator.calculate_crit_chance(&attacker, &mut rng);

        // Default formula: 0.05 + luck/200 = 0.05 + 10/200 = 0.10
        assert!((crit_chance - 0.10).abs() < 0.001);
    }

    #[test]
    fn test_healing_calculation() {
        let formulas = FormulaResource::default();
        let calculator = DamageCalculator::new(formulas);
        let healer = test_stats();
        let target = test_stats();
        let skill = Skill {
            skill_type: SkillType::Heal,
            power: 100,
            ..basic_attack_skill()
        };

        let mut rng = rand::thread_rng();
        let healing = calculator.calculate_healing(&healer, &target, &skill, &mut rng);

        // Default formula: mag * 3 + level * 2 = 15 * 3 + 1 * 2 = 47
        // Then multiplied by skill power (1.0) and variance
        assert!(healing > 0);
    }

    #[test]
    fn test_flee_chance_calculation() {
        let formulas = FormulaResource::default();
        let calculator = DamageCalculator::new(formulas);
        let attacker = test_stats();
        let defender = test_stats();

        let mut rng = rand::thread_rng();
        let flee_chance = calculator.calculate_flee_chance(&attacker, &defender, &mut rng);

        // Default formula: 0.5 + (spd - spd) / 100 = 0.5
        assert!((flee_chance - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_simulation_result() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = SimulationResult::from_values(values, 1, 5);

        assert_eq!(result.min_value, 1.0);
        assert_eq!(result.max_value, 5.0);
        assert_eq!(result.avg_value, 3.0);
        assert_eq!(result.median_value, 3.0);
        assert!(!result.distribution.is_empty());
    }

    #[test]
    fn test_expression_evaluation() {
        assert_eq!(evaluate_expression("2 + 3").unwrap(), 5.0);
        assert_eq!(evaluate_expression("10 - 4").unwrap(), 6.0);
        assert_eq!(evaluate_expression("3 * 4").unwrap(), 12.0);
        assert_eq!(evaluate_expression("12 / 4").unwrap(), 3.0);
        assert_eq!(evaluate_expression("(2 + 3) * 4").unwrap(), 20.0);
        assert_eq!(evaluate_expression("2 + 3 * 4").unwrap(), 14.0);
        assert_eq!(evaluate_expression("-5").unwrap(), -5.0);
        assert_eq!(evaluate_expression("+5").unwrap(), 5.0);
    }

    #[test]
    fn test_function_evaluation() {
        assert_eq!(evaluate_expression("min(5, 10)").unwrap(), 5.0);
        assert_eq!(evaluate_expression("max(5, 10)").unwrap(), 10.0);
        assert_eq!(evaluate_expression("clamp(15, 0, 10)").unwrap(), 10.0);
        assert_eq!(evaluate_expression("clamp(-5, 0, 10)").unwrap(), 0.0);
        assert_eq!(evaluate_expression("clamp(5, 0, 10)").unwrap(), 5.0);
        assert_eq!(evaluate_expression("abs(-5)").unwrap(), 5.0);
        assert_eq!(evaluate_expression("sqrt(16)").unwrap(), 4.0);
        assert_eq!(evaluate_expression("pow(2, 3)").unwrap(), 8.0);
    }
}
