//! Formula resource for damage calculations
//!
//! Provides customizable formulas for:
//! - Damage calculation
//! - Healing calculation
//! - Critical hit chance
//! - Status effect application
//! - Flee/escape chance

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Formula types for different calculations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FormulaKind {
    /// Damage dealt to target
    Damage,
    /// HP restored to target
    Healing,
    /// Critical hit chance (0.0 - 1.0)
    CriticalHit,
    /// Status effect apply chance (0.0 - 1.0)
    StatusApply,
    /// Flee/escape chance (0.0 - 1.0)
    Flee,
}

impl FormulaKind {
    /// Get display name for the formula type
    pub const fn name(&self) -> &'static str {
        match self {
            FormulaKind::Damage => "Damage",
            FormulaKind::Healing => "Healing",
            FormulaKind::CriticalHit => "Critical Hit",
            FormulaKind::StatusApply => "Status Apply",
            FormulaKind::Flee => "Flee",
        }
    }

    /// Get default formula string for this type
    pub fn default_formula(&self) -> String {
        match self {
            FormulaKind::Damage => {
                "(attacker.str * 4 - defender.def * 2) * skill.power / 100".to_string()
            }
            FormulaKind::Healing => "attacker.mag * 3 + attacker.level * 2".to_string(),
            FormulaKind::CriticalHit => "0.05 + attacker.luck / 200".to_string(),
            FormulaKind::StatusApply => "(attacker.mag - defender.mag + 50) / 100".to_string(),
            FormulaKind::Flee => "0.5 + (attacker.spd - defender.spd) / 100".to_string(),
        }
    }

    /// Get all formula kinds
    pub fn all() -> [FormulaKind; 5] {
        [
            FormulaKind::Damage,
            FormulaKind::Healing,
            FormulaKind::CriticalHit,
            FormulaKind::StatusApply,
            FormulaKind::Flee,
        ]
    }
}

/// Collection of all game formulas
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FormulaResource {
    /// Formula version for migrations
    pub version: String,
    /// Damage formula string
    pub damage: String,
    /// Healing formula string
    pub healing: String,
    /// Critical hit chance formula
    pub critical: String,
    /// Status effect application formula
    pub status_apply: String,
    /// Flee/escape formula
    pub flee: String,
    /// Custom formulas (for extensibility)
    #[serde(default)]
    pub custom: HashMap<String, String>,
}

impl Default for FormulaResource {
    fn default() -> Self {
        Self {
            version: "1.0".to_string(),
            damage: FormulaKind::Damage.default_formula(),
            healing: FormulaKind::Healing.default_formula(),
            critical: FormulaKind::CriticalHit.default_formula(),
            status_apply: FormulaKind::StatusApply.default_formula(),
            flee: FormulaKind::Flee.default_formula(),
            custom: HashMap::new(),
        }
    }
}

impl FormulaResource {
    /// Create a new formula resource with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Get formula by kind
    pub fn get(&self, kind: FormulaKind) -> &str {
        match kind {
            FormulaKind::Damage => &self.damage,
            FormulaKind::Healing => &self.healing,
            FormulaKind::CriticalHit => &self.critical,
            FormulaKind::StatusApply => &self.status_apply,
            FormulaKind::Flee => &self.flee,
        }
    }

    /// Set formula by kind
    pub fn set(&mut self, kind: FormulaKind, formula: String) {
        match kind {
            FormulaKind::Damage => self.damage = formula,
            FormulaKind::Healing => self.healing = formula,
            FormulaKind::CriticalHit => self.critical = formula,
            FormulaKind::StatusApply => self.status_apply = formula,
            FormulaKind::Flee => self.flee = formula,
        }
    }

    /// Get all formulas as a map
    pub fn to_map(&self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.insert("damage".to_string(), self.damage.clone());
        map.insert("healing".to_string(), self.healing.clone());
        map.insert("critical".to_string(), self.critical.clone());
        map.insert("status_apply".to_string(), self.status_apply.clone());
        map.insert("flee".to_string(), self.flee.clone());
        map.extend(self.custom.clone());
        map
    }

    /// Load from a map
    pub fn from_map(map: HashMap<String, String>) -> Self {
        let mut resource = Self::new();
        if let Some(f) = map.get("damage") {
            resource.damage = f.clone();
        }
        if let Some(f) = map.get("healing") {
            resource.healing = f.clone();
        }
        if let Some(f) = map.get("critical") {
            resource.critical = f.clone();
        }
        if let Some(f) = map.get("status_apply") {
            resource.status_apply = f.clone();
        }
        if let Some(f) = map.get("flee") {
            resource.flee = f.clone();
        }
        // Load custom formulas (non-standard keys)
        for (key, value) in map {
            match key.as_str() {
                "damage" | "healing" | "critical" | "status_apply" | "flee" => {}
                _ => {
                    resource.custom.insert(key, value);
                }
            }
        }
        resource
    }

    /// Load from TOML file
    pub fn from_toml(toml_str: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(toml_str)
    }

    /// Save to TOML string
    pub fn to_toml(&self) -> Result<String, toml::ser::Error> {
        toml::to_string_pretty(self)
    }

    /// Validate all formulas
    pub fn validate_all(&self) -> Vec<(FormulaKind, Vec<String>)> {
        FormulaKind::all()
            .into_iter()
            .filter_map(|kind| {
                let formula = self.get(kind);
                let errors = validate_formula(formula);
                if errors.is_empty() {
                    None
                } else {
                    Some((kind, errors))
                }
            })
            .collect()
    }
}

/// Valid variable names for formulas
pub const VALID_VARIABLES: &[&str] = &[
    // Attacker stats
    "attacker.str",
    "attacker.def",
    "attacker.spd",
    "attacker.mag",
    "attacker.luck",
    "attacker.level",
    "attacker.hp",
    "attacker.max_hp",
    // Defender stats
    "defender.str",
    "defender.def",
    "defender.spd",
    "defender.mag",
    "defender.luck",
    "defender.level",
    "defender.hp",
    "defender.max_hp",
    // Skill stats
    "skill.power",
    "skill.accuracy",
];

/// Valid function names for formulas
pub const VALID_FUNCTIONS: &[&str] = &["random", "min", "max", "clamp", "abs", "sqrt", "pow"];

/// Validate a formula string
pub fn validate_formula(formula: &str) -> Vec<String> {
    let mut errors = Vec::new();

    // Check for empty formula
    if formula.trim().is_empty() {
        errors.push("Formula is empty".to_string());
        return errors;
    }

    // Check for balanced parentheses
    let open_count = formula.chars().filter(|&c| c == '(').count();
    let close_count = formula.chars().filter(|&c| c == ')').count();
    if open_count != close_count {
        errors.push(format!(
            "Unbalanced parentheses: {} open, {} close",
            open_count, close_count
        ));
    }

    // Extract and validate variable names
    let var_regex = regex_lite::Regex::new(r"(attacker|defender|skill)\.[a-z_]+").unwrap();
    for cap in var_regex.find_iter(formula) {
        let var = cap.as_str();
        if !VALID_VARIABLES.contains(&var) {
            errors.push(format!("Unknown variable: {}", var));
        }
    }

    // Check for invalid characters
    let valid_chars: std::collections::HashSet<char> =
        "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789._+-*/()=<>!%, "
            .chars()
            .collect();

    for (i, ch) in formula.chars().enumerate() {
        if !valid_chars.contains(&ch) {
            errors.push(format!("Invalid character '{}' at position {}", ch, i));
        }
    }

    errors
}

/// Formula evaluation context
#[derive(Debug, Clone, Default)]
pub struct FormulaContext {
    /// Attacker stats
    pub attacker: CombatantStats,
    /// Defender stats
    pub defender: CombatantStats,
    /// Skill power
    pub skill_power: i32,
    /// Skill accuracy
    pub skill_accuracy: f32,
}

/// Combatant stats for formula evaluation
#[derive(Debug, Clone, Copy, Default)]
pub struct CombatantStats {
    pub str: i32,
    pub def: i32,
    pub spd: i32,
    pub mag: i32,
    pub luck: i32,
    pub level: i32,
    pub hp: i32,
    pub max_hp: i32,
}

impl CombatantStats {
    /// Convert from core Stats component
    pub fn from_core_stats(stats: &crate::components::Stats) -> Self {
        Self {
            str: stats.str,
            def: stats.def,
            spd: stats.spd,
            mag: stats.mag,
            luck: stats.luck,
            level: stats.level,
            hp: stats.hp,
            max_hp: stats.max_hp,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_formula_resource_default() {
        let resource = FormulaResource::default();
        assert!(!resource.damage.is_empty());
        assert!(!resource.healing.is_empty());
        assert!(!resource.critical.is_empty());
        assert_eq!(resource.version, "1.0");
    }

    #[test]
    fn test_formula_resource_get_set() {
        let mut resource = FormulaResource::new();
        resource.set(FormulaKind::Damage, "attacker.str * 2".to_string());
        assert_eq!(resource.get(FormulaKind::Damage), "attacker.str * 2");
    }

    #[test]
    fn test_formula_serialization() {
        let resource = FormulaResource::default();
        let toml = resource.to_toml().unwrap();
        let deserialized = FormulaResource::from_toml(&toml).unwrap();
        assert_eq!(resource, deserialized);
    }

    #[test]
    fn test_formula_validation_empty() {
        let errors = validate_formula("");
        assert!(!errors.is_empty());
        assert!(errors[0].contains("empty"));
    }

    #[test]
    fn test_formula_validation_unbalanced_parens() {
        let errors = validate_formula("(attacker.str * 2");
        assert!(!errors.is_empty());
        assert!(errors[0].contains("parentheses"));
    }

    #[test]
    fn test_formula_context_default() {
        let ctx = FormulaContext::default();
        assert_eq!(ctx.skill_power, 0);
        assert_eq!(ctx.skill_accuracy, 0.0);
    }
}
