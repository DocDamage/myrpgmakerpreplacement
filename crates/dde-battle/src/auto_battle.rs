//! Auto-Battle AI System
//!
//! Provides AI control for party members with configurable strategies.
//! Supports manual, auto, and semi-auto modes with various battle strategies.

use std::collections::HashMap;

use dde_core::{Entity, World};
use dde_core::components::Stats;
use serde::{Deserialize, Serialize};

use crate::skills::{SkillDatabase, SkillType};
use crate::turn_queue::{BattleAction, ActionType};
use crate::items::ItemDatabase;

/// Serializable record of an AI action for history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionRecord {
    /// Turn number when action was taken
    pub turn: u32,
    /// Action type taken
    pub action_type: ActionType,
    /// Target entity (if any) - stored as index for serialization
    pub target_index: Option<usize>,
    /// Score of the action at decision time
    pub score: f32,
}

/// AI control settings for a combatant
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[derive(Default)]
pub enum AutoBattleMode {
    /// Player controls this combatant
    #[default]
    Manual,
    /// AI controls with selected strategy
    Auto,
    /// AI suggests, player confirms
    SemiAuto,
}


/// AI strategy priorities
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(Default)]
pub enum BattleStrategy {
    /// Mix of offense and defense
    #[default]
    Balanced,
    /// Focus on dealing damage
    Aggressive,
    /// Focus on healing/buffing
    Defensive,
    /// Prioritize keeping party alive
    HealerFirst,
    /// Focus fire on weakest enemy
    TargetWeakest,
    /// Conserve MP, use basic attacks
    SaveMP,
}

impl BattleStrategy {
    /// Get all available strategies
    pub fn all() -> &'static [BattleStrategy] {
        &[
            BattleStrategy::Balanced,
            BattleStrategy::Aggressive,
            BattleStrategy::Defensive,
            BattleStrategy::HealerFirst,
            BattleStrategy::TargetWeakest,
            BattleStrategy::SaveMP,
        ]
    }
}


/// AI decision weighting
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct AIDecisionWeights {
    /// Weight for dealing damage
    pub damage_weight: f32,
    /// Weight for healing
    pub healing_weight: f32,
    /// Weight for buffs
    pub buff_weight: f32,
    /// Weight for MP conservation
    pub mp_conservation: f32,
    /// Bonus for targeting low HP enemies
    pub target_low_hp_bonus: f32,
    /// Risk tolerance (0.0 = safe, 1.0 = risky)
    pub risk_tolerance: f32,
}

impl AIDecisionWeights {
    /// Get default weights for a specific strategy
    pub fn for_strategy(strategy: BattleStrategy) -> Self {
        match strategy {
            BattleStrategy::Balanced => Self {
                damage_weight: 1.0,
                healing_weight: 0.8,
                buff_weight: 0.5,
                mp_conservation: 0.5,
                target_low_hp_bonus: 0.3,
                risk_tolerance: 0.5,
            },
            BattleStrategy::Aggressive => Self {
                damage_weight: 1.5,
                healing_weight: 0.3,
                buff_weight: 0.2,
                mp_conservation: 0.2,
                target_low_hp_bonus: 0.5,
                risk_tolerance: 0.8,
            },
            BattleStrategy::Defensive => Self {
                damage_weight: 0.6,
                healing_weight: 1.5,
                buff_weight: 1.0,
                mp_conservation: 0.7,
                target_low_hp_bonus: 0.2,
                risk_tolerance: 0.2,
            },
            BattleStrategy::HealerFirst => Self {
                damage_weight: 0.3,
                healing_weight: 2.0,
                buff_weight: 0.8,
                mp_conservation: 0.6,
                target_low_hp_bonus: 0.0,
                risk_tolerance: 0.3,
            },
            BattleStrategy::TargetWeakest => Self {
                damage_weight: 1.2,
                healing_weight: 0.6,
                buff_weight: 0.4,
                mp_conservation: 0.4,
                target_low_hp_bonus: 1.0,
                risk_tolerance: 0.6,
            },
            BattleStrategy::SaveMP => Self {
                damage_weight: 0.8,
                healing_weight: 0.5,
                buff_weight: 0.3,
                mp_conservation: 1.5,
                target_low_hp_bonus: 0.3,
                risk_tolerance: 0.4,
            },
        }
    }
}

impl Default for AIDecisionWeights {
    fn default() -> Self {
        Self::for_strategy(BattleStrategy::Balanced)
    }
}

/// Auto-battle AI component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoBattleAI {
    /// Current control mode
    pub mode: AutoBattleMode,
    /// Selected battle strategy
    pub strategy: BattleStrategy,
    /// Decision weights for this AI
    pub weights: AIDecisionWeights,
    /// Last action record taken
    #[serde(skip)]
    pub last_action: Option<BattleAction>,
    /// Serializable action history for learning/adaptation
    pub action_history: Vec<ActionRecord>,
}

impl AutoBattleAI {
    /// Create new AI with specified strategy
    pub fn new(strategy: BattleStrategy) -> Self {
        Self {
            mode: AutoBattleMode::Manual,
            strategy,
            weights: AIDecisionWeights::for_strategy(strategy),
            last_action: None,
            action_history: Vec::new(),
        }
    }

    /// Set the control mode (builder pattern)
    #[must_use]
    pub fn with_mode(mut self, mode: AutoBattleMode) -> Self {
        self.mode = mode;
        self
    }

    /// Check if AI is in auto mode
    pub fn is_auto(&self) -> bool {
        matches!(self.mode, AutoBattleMode::Auto)
    }

    /// Check if AI is in semi-auto mode
    pub fn is_semi_auto(&self) -> bool {
        matches!(self.mode, AutoBattleMode::SemiAuto)
    }

    /// Record an action in the history
    /// 
    /// # Arguments
    /// * `action` - The battle action taken
    /// * `turn` - The turn number when the action was taken
    /// * `score` - The AI score for this action
    pub fn record_action(&mut self, action: BattleAction, turn: u32, score: f32) {
        self.last_action = Some(action.clone());
        
        let record = ActionRecord {
            turn,
            action_type: action.action_type,
            target_index: None, // Would need entity to index mapping
            score,
        };
        
        self.action_history.push(record);
        // Keep last 20 actions
        if self.action_history.len() > 20 {
            self.action_history.remove(0);
        }
    }
    
    /// Clear action history
    pub fn clear_history(&mut self) {
        self.action_history.clear();
        self.last_action = None;
    }

    /// Update strategy and recalculate weights
    pub fn set_strategy(&mut self, strategy: BattleStrategy) {
        self.strategy = strategy;
        self.weights = AIDecisionWeights::for_strategy(strategy);
    }

    /// Toggle between auto modes
    pub fn toggle_mode(&mut self) {
        self.mode = match self.mode {
            AutoBattleMode::Manual => AutoBattleMode::Auto,
            AutoBattleMode::Auto => AutoBattleMode::SemiAuto,
            AutoBattleMode::SemiAuto => AutoBattleMode::Manual,
        };
    }
}

impl Default for AutoBattleAI {
    fn default() -> Self {
        Self::new(BattleStrategy::Balanced)
    }
}

/// Evaluated action with score
#[derive(Debug, Clone)]
pub struct ScoredAction {
    /// The action being scored
    pub action: BattleAction,
    /// Calculated score (higher is better)
    pub score: f32,
    /// Reasoning for the score (for debugging)
    pub reasoning: String,
}

/// Auto-battle system
pub struct AutoBattleSystem {
    /// Predefined strategies with their weights
    strategies: HashMap<BattleStrategy, AIDecisionWeights>,
    /// Skill database reference
    skill_db: SkillDatabase,
    /// Item database reference
    item_db: ItemDatabase,
}

impl AutoBattleSystem {
    /// Create new auto-battle system with default strategies
    pub fn new() -> Self {
        let mut strategies = HashMap::new();
        for strategy in [
            BattleStrategy::Balanced,
            BattleStrategy::Aggressive,
            BattleStrategy::Defensive,
            BattleStrategy::HealerFirst,
            BattleStrategy::TargetWeakest,
            BattleStrategy::SaveMP,
        ] {
            strategies.insert(strategy, AIDecisionWeights::for_strategy(strategy));
        }
        Self {
            strategies,
            skill_db: SkillDatabase::new(),
            item_db: ItemDatabase::new(),
        }
    }

    /// Set skill database
    pub fn with_skill_db(mut self, skill_db: SkillDatabase) -> Self {
        self.skill_db = skill_db;
        self
    }

    /// Set item database
    pub fn with_item_db(mut self, item_db: ItemDatabase) -> Self {
        self.item_db = item_db;
        self
    }

    /// Get skill database reference
    pub fn skill_db(&self) -> &SkillDatabase {
        &self.skill_db
    }

    /// Get item database reference
    pub fn item_db(&self) -> &ItemDatabase {
        &self.item_db
    }

    /// Decide action for entity
    pub fn decide_action(
        &self,
        world: &World,
        entity: Entity,
        available_actions: &[BattleAction],
        allies: &[Entity],
        enemies: &[Entity],
    ) -> Option<BattleAction> {
        let ai = world.query_one::<&AutoBattleAI>(entity).ok()?.get()?.clone();

        if !ai.is_auto() {
            return None;
        }

        if available_actions.is_empty() {
            return None;
        }

        let mut scored: Vec<ScoredAction> = available_actions
            .iter()
            .map(|action| self.score_action(world, entity, action, &ai, allies, enemies))
            .collect();

        // Sort by score descending
        scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        // Return highest scoring action
        scored.first().map(|s| s.action.clone())
    }

    /// Score a single action
    fn score_action(
        &self,
        world: &World,
        actor: Entity,
        action: &BattleAction,
        ai: &AutoBattleAI,
        allies: &[Entity],
        _enemies: &[Entity],
    ) -> ScoredAction {
        let mut score = 0.0;
        let mut reasoning_parts: Vec<String> = Vec::new();

        match &action.action_type {
            ActionType::Attack => {
                score += ai.weights.damage_weight * 10.0;
                reasoning_parts.push(format!("base attack (+{:.1})", ai.weights.damage_weight * 10.0));

                // Bonus for targeting low HP enemies
                if let Some(target) = action.target {
                    if let Ok(mut query) = world.query_one::<&Stats>(target) {
                        if let Some(stats) = query.get() {
                            let hp_percent = stats.hp as f32 / stats.max_hp.max(1) as f32;
                            if hp_percent < 0.3 {
                                let bonus = ai.weights.target_low_hp_bonus * 20.0;
                                score += bonus;
                                reasoning_parts.push(format!("target low HP (+{:.1})", bonus));
                            }
                        }
                    }
                }
            }

            ActionType::Skill(skill_id) => {
                if let Some(skill) = self.skill_db.get(*skill_id) {
                    match skill.skill_type {
                        SkillType::Physical | SkillType::Magic | SkillType::Hybrid => {
                            score += ai.weights.damage_weight * 15.0;
                            
                            // Penalty for MP cost
                            let mp_penalty = skill.mp_cost as f32 * (1.0 - ai.weights.mp_conservation);
                            score -= mp_penalty;
                            
                            reasoning_parts.push(format!(
                                "skill damage (+{:.1}, MP cost {:.1})",
                                ai.weights.damage_weight * 15.0,
                                mp_penalty
                            ));

                            // Bonus for targeting low HP enemies
                            if let Some(target) = action.target {
                                if let Ok(mut query) = world.query_one::<&Stats>(target) {
                                    if let Some(stats) = query.get() {
                                        let hp_percent = stats.hp as f32 / stats.max_hp.max(1) as f32;
                                        if hp_percent < 0.3 {
                                            let bonus = ai.weights.target_low_hp_bonus * 20.0;
                                            score += bonus;
                                            reasoning_parts.push(format!("target low HP (+{:.1})", bonus));
                                        }
                                    }
                                }
                            }
                        }
                        SkillType::Heal => {
                            // Only heal if ally is damaged
                            if let Some(target) = action.target {
                                if let Ok(mut query) = world.query_one::<&Stats>(target) {
                                    if let Some(stats) = query.get() {
                                        if stats.hp < stats.max_hp {
                                            let hp_percent = stats.hp as f32 / stats.max_hp.max(1) as f32;
                                            let heal_multiplier = 1.0 + (1.0 - hp_percent); // More damaged = higher priority
                                            let heal_score = ai.weights.healing_weight * 20.0 * heal_multiplier;
                                            score += heal_score;
                                            reasoning_parts.push(format!(
                                                "heal ally at {:.0}% HP (+{:.1})",
                                                hp_percent * 100.0,
                                                heal_score
                                            ));
                                        } else {
                                            score -= 15.0; // Don't heal full HP
                                            reasoning_parts.push("target full HP (-15.0)".to_string());
                                        }
                                    }
                                }
                            }

                            // Penalty for MP cost
                            let mp_penalty = skill.mp_cost as f32 * (1.0 - ai.weights.mp_conservation);
                            score -= mp_penalty;
                        }
                        SkillType::Support => {
                            score += ai.weights.buff_weight * 8.0;
                            reasoning_parts.push(format!("buff ally (+{:.1})", ai.weights.buff_weight * 8.0));

                            // Penalty for MP cost
                            let mp_penalty = skill.mp_cost as f32 * (1.0 - ai.weights.mp_conservation);
                            score -= mp_penalty;
                        }
                        SkillType::Status => {
                            // Status effects are situational
                            score += ai.weights.damage_weight * 5.0;
                            reasoning_parts.push("status effect (+5.0)".to_string());

                            // Penalty for MP cost
                            let mp_penalty = skill.mp_cost as f32 * (1.0 - ai.weights.mp_conservation);
                            score -= mp_penalty;
                        }
                    }
                } else {
                    score -= 100.0; // Invalid skill
                    reasoning_parts.push("invalid skill (-100.0)".to_string());
                }
            }

            ActionType::Item(_item_id) => {
                // Items are valuable - moderate score
                score += 12.0;
                reasoning_parts.push("item use (+12.0)".to_string());

                // Bonus for healing items when needed
                if let Some(target) = action.target {
                    if let Ok(mut query) = world.query_one::<&Stats>(target) {
                        if let Some(stats) = query.get() {
                            if stats.hp < stats.max_hp / 2 {
                                score += 10.0;
                                reasoning_parts.push("heal needed (+10.0)".to_string());
                            }
                        }
                    }
                }
            }

            ActionType::Defend => {
                // Defend if low HP
                if let Ok(mut query) = world.query_one::<&Stats>(actor) {
                    if let Some(stats) = query.get() {
                        let hp_percent = stats.hp as f32 / stats.max_hp.max(1) as f32;
                        if hp_percent < 0.3 {
                            let defend_score = ai.weights.risk_tolerance * 15.0;
                            score += defend_score;
                            reasoning_parts.push(format!("low HP defend (+{:.1})", defend_score));
                        }
                    }
                }
                score += 5.0;
                reasoning_parts.push("base defend (+5.0)".to_string());
            }

            ActionType::Flee => {
                // Flee if party is weak
                if !allies.is_empty() {
                    let party_hp: f32 = allies
                        .iter()
                        .filter_map(|e| world.query_one::<&Stats>(*e).ok()?.get().copied())
                        .map(|s: Stats| s.hp as f32 / s.max_hp.max(1) as f32)
                        .sum();
                    let avg_party_hp = party_hp / allies.len() as f32;

                    if avg_party_hp < 0.2 {
                        score += 25.0;
                        reasoning_parts.push(format!("party weak ({:.0}% HP), flee (+25.0)", avg_party_hp * 100.0));
                    } else if avg_party_hp < 0.5 {
                        score += 5.0;
                        reasoning_parts.push("party damaged, consider flee (+5.0)".to_string());
                    } else {
                        score -= 10.0; // Don't flee when healthy
                        reasoning_parts.push("party healthy, don't flee (-10.0)".to_string());
                    }
                }
            }
        }

        // Add randomness based on risk tolerance
        // Higher risk tolerance = more randomness
        let variance = (1.0 - ai.weights.risk_tolerance) * 5.0;
        let random_factor = rand::random::<f32>() * variance - variance / 2.0;
        score += random_factor;

        if random_factor.abs() > 0.1 {
            reasoning_parts.push(format!("random factor ({:+.1})", random_factor));
        }

        ScoredAction {
            action: action.clone(),
            score,
            reasoning: reasoning_parts.join(", "),
        }
    }

    /// Generate available actions for an entity
    pub fn generate_available_actions(
        &self,
        _world: &World,
        entity: Entity,
        allies: &[Entity],
        enemies: &[Entity],
    ) -> Vec<BattleAction> {
        let mut actions = Vec::new();

        // Basic attack on each enemy
        for &target in enemies {
            actions.push(BattleAction {
                actor: entity,
                action_type: ActionType::Attack,
                target: Some(target),
            });
        }

        // Skills (would need skill list from entity)
        // For now, add common skills
        for skill_id in [2, 3, 4, 5, 6] { // fireball, heal, thunder, power attack, protect
            if let Some(skill) = self.skill_db.get(skill_id) {
                let targets = match skill.target_type {
                    crate::skills::TargetType::SelfOnly => vec![entity],
                    crate::skills::TargetType::SingleAlly | crate::skills::TargetType::AllAllies => {
                        allies.to_vec()
                    }
                    crate::skills::TargetType::SingleEnemy | crate::skills::TargetType::AllEnemies | crate::skills::TargetType::RandomEnemy => {
                        enemies.to_vec()
                    }
                };

                for target in targets {
                    actions.push(BattleAction {
                        actor: entity,
                        action_type: ActionType::Skill(skill_id),
                        target: Some(target),
                    });
                }
            }
        }

        // Defend
        actions.push(BattleAction {
            actor: entity,
            action_type: ActionType::Defend,
            target: None,
        });

        // Flee
        actions.push(BattleAction {
            actor: entity,
            action_type: ActionType::Flee,
            target: None,
        });

        actions
    }
}

impl Default for AutoBattleSystem {
    fn default() -> Self {
        Self::new()
    }
}

/// Party-wide AI management
#[derive(Debug, Clone, Default)]
pub struct PartyAI {
    /// Global mode applied to all party members
    pub global_mode: AutoBattleMode,
    /// Individual AI settings per entity
    pub individual_settings: HashMap<Entity, AutoBattleAI>,
}

impl PartyAI {
    /// Create new PartyAI
    pub fn new() -> Self {
        Self {
            global_mode: AutoBattleMode::Manual,
            individual_settings: HashMap::new(),
        }
    }

    /// Set AI for a specific entity
    pub fn set_ai(&mut self, entity: Entity, ai: AutoBattleAI) {
        self.individual_settings.insert(entity, ai);
    }

    /// Get AI for a specific entity
    pub fn get_ai(&self, entity: Entity) -> Option<&AutoBattleAI> {
        self.individual_settings.get(&entity)
    }

    /// Get mutable AI for a specific entity
    pub fn get_ai_mut(&mut self, entity: Entity) -> Option<&mut AutoBattleAI> {
        self.individual_settings.get_mut(&entity)
    }

    /// Toggle auto-battle for entire party
    pub fn toggle_auto(&mut self, mode: AutoBattleMode) {
        self.global_mode = mode;
        for ai in self.individual_settings.values_mut() {
            ai.mode = mode;
        }
    }

    /// Set strategy for specific entity
    pub fn set_strategy(&mut self, entity: Entity, strategy: BattleStrategy) {
        if let Some(ai) = self.individual_settings.get_mut(&entity) {
            ai.set_strategy(strategy);
        } else {
            let mut ai = AutoBattleAI::new(strategy);
            ai.mode = self.global_mode;
            self.individual_settings.insert(entity, ai);
        }
    }

    /// Check if any party member is in auto mode
    pub fn has_auto_members(&self) -> bool {
        self.individual_settings
            .values()
            .any(|ai| ai.is_auto())
    }

    /// Check if any party member is in semi-auto mode
    pub fn has_semi_auto_members(&self) -> bool {
        self.individual_settings
            .values()
            .any(|ai| ai.is_semi_auto())
    }

    /// Get all entities in auto mode
    pub fn get_auto_entities(&self) -> Vec<Entity> {
        self.individual_settings
            .iter()
            .filter(|(_, ai)| ai.is_auto())
            .map(|(entity, _)| *entity)
            .collect()
    }

    /// Get all auto actions for a turn
    pub fn get_auto_actions(
        &self,
        world: &World,
        ai_system: &AutoBattleSystem,
        party: &[Entity],
        enemies: &[Entity],
    ) -> Vec<(Entity, BattleAction)> {
        let mut actions = Vec::new();

        for &entity in party {
            if let Some(ai) = self.individual_settings.get(&entity) {
                if ai.is_auto() {
                    let available = ai_system.generate_available_actions(world, entity, party, enemies);
                    
                    if let Some(action) = ai_system.decide_action(
                        world,
                        entity,
                        &available,
                        party,
                        enemies,
                    ) {
                        actions.push((entity, action));
                    }
                }
            }
        }

        actions
    }

    /// Remove entity from party AI
    pub fn remove_entity(&mut self, entity: Entity) {
        self.individual_settings.remove(&entity);
    }

    /// Clear all party AI settings
    pub fn clear(&mut self) {
        self.individual_settings.clear();
        self.global_mode = AutoBattleMode::Manual;
    }

    /// Get count of members in each mode
    pub fn mode_counts(&self) -> (usize, usize, usize) {
        let mut manual = 0;
        let mut auto = 0;
        let mut semi_auto = 0;

        for ai in self.individual_settings.values() {
            match ai.mode {
                AutoBattleMode::Manual => manual += 1,
                AutoBattleMode::Auto => auto += 1,
                AutoBattleMode::SemiAuto => semi_auto += 1,
            }
        }

        (manual, auto, semi_auto)
    }
}

/// UI component for auto-battle controls
#[cfg(feature = "ui")]
pub struct AutoBattleUi {
    /// Whether to show strategy preview
    show_strategy_preview: bool,
}

#[cfg(feature = "ui")]
impl AutoBattleUi {
    /// Create new UI component
    pub fn new() -> Self {
        Self {
            show_strategy_preview: false,
        }
    }

    /// Draw auto-battle toggle and strategy selector
    pub fn draw(&mut self, ui: &mut egui::Ui, ai: &mut AutoBattleAI) {
        ui.horizontal(|ui| {
            // Mode toggle button
            let mode_text = match ai.mode {
                AutoBattleMode::Manual => "🎮 Manual",
                AutoBattleMode::Auto => "🤖 Auto",
                AutoBattleMode::SemiAuto => "👤🤖 Semi-Auto",
            };

            if ui.button(mode_text).clicked() {
                ai.toggle_mode();
            }

            // Strategy selector (only in auto/semi-auto)
            if ai.mode != AutoBattleMode::Manual {
                ui.add_space(8.0);
                egui::ComboBox::from_id_source("strategy_selector")
                    .selected_text(format!("{:?}", ai.strategy))
                    .width(120.0)
                    .show_ui(ui, |ui| {
                        for strategy in BattleStrategy::all() {
                            let selected = ai.strategy == *strategy;
                            let response = ui.selectable_label(
                                selected,
                                format!("{:?}", strategy)
                            );
                            if response.clicked() && !selected {
                                ai.set_strategy(*strategy);
                            }
                        }
                    });
            }
        });

        // Show weight breakdown button
        if ui.small_button("⚙ Weights").clicked() {
            self.show_strategy_preview = !self.show_strategy_preview;
        }

        // Show strategy preview
        if self.show_strategy_preview {
            ui.separator();
            self.draw_weights(ui, &ai.weights);
        }
    }

    /// Draw weight visualization
    fn draw_weights(&self, ui: &mut egui::Ui, weights: &AIDecisionWeights) {
        ui.group(|ui| {
            ui.label(egui::RichText::new("AI Weights").strong());
            ui.separator();
            
            ui.horizontal(|ui| {
                ui.label("Damage:");
                ui.add(egui::ProgressBar::new(
                    (weights.damage_weight / 2.0).clamp(0.0, 1.0)
                ).text(format!("{:.1}", weights.damage_weight)));
            });
            
            ui.horizontal(|ui| {
                ui.label("Healing:");
                ui.add(egui::ProgressBar::new(
                    (weights.healing_weight / 2.0).clamp(0.0, 1.0)
                ).text(format!("{:.1}", weights.healing_weight)));
            });
            
            ui.horizontal(|ui| {
                ui.label("Buffs:");
                ui.add(egui::ProgressBar::new(
                    (weights.buff_weight / 2.0).clamp(0.0, 1.0)
                ).text(format!("{:.1}", weights.buff_weight)));
            });
            
            ui.horizontal(|ui| {
                ui.label("MP Save:");
                ui.add(egui::ProgressBar::new(
                    (weights.mp_conservation / 2.0).clamp(0.0, 1.0)
                ).text(format!("{:.1}", weights.mp_conservation)));
            });
            
            ui.horizontal(|ui| {
                ui.label("Risk:");
                ui.add(egui::ProgressBar::new(weights.risk_tolerance)
                    .text(format!("{:.0}%", weights.risk_tolerance * 100.0)));
            });
        });
    }

    /// Draw party-wide AI controls
    pub fn draw_party_controls(&mut self, ui: &mut egui::Ui, party_ai: &mut PartyAI) {
        ui.heading("Party AI");
        ui.separator();

        let (manual, auto, semi) = party_ai.mode_counts();
        ui.label(format!("Manual: {} | Auto: {} | Semi: {}", manual, auto, semi));

        ui.horizontal(|ui| {
            if ui.button("Set All Manual").clicked() {
                party_ai.toggle_auto(AutoBattleMode::Manual);
            }
            if ui.button("Set All Auto").clicked() {
                party_ai.toggle_auto(AutoBattleMode::Auto);
            }
            if ui.button("Set All Semi").clicked() {
                party_ai.toggle_auto(AutoBattleMode::SemiAuto);
            }
        });
    }
}

#[cfg(feature = "ui")]
impl Default for AutoBattleUi {
    fn default() -> Self {
        Self::new()
    }
}

/// Extension trait for BattleSystem to integrate auto-battle
pub trait AutoBattleIntegration {
    /// Process auto-battle decisions before player input
    fn process_auto_actions(&mut self, world: &mut World, party_ai: &PartyAI);
    
    /// Get allies of an entity
    fn get_allies(&self, world: &World, entity: Entity) -> Vec<Entity>;
    
    /// Get enemies of an entity
    fn get_enemies(&self, world: &World, entity: Entity) -> Vec<Entity>;
    
    /// Get available actions for an entity
    fn get_available_actions(&self, world: &World, entity: Entity) -> Vec<BattleAction>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use dde_core::World;

    fn create_test_world_with_stats() -> (World, Entity, Entity, Entity) {
        let mut world = World::new();
        
        // Create player with auto battle AI
        let player = world.spawn((
            Stats {
                hp: 100,
                max_hp: 100,
                mp: 50,
                max_mp: 50,
                str: 10,
                def: 10,
                spd: 10,
                mag: 10,
                luck: 10,
                level: 5,
                exp: 0,
            },
            AutoBattleAI::new(BattleStrategy::Balanced).with_mode(AutoBattleMode::Auto),
        ));

        // Create ally
        let ally = world.spawn((Stats {
            hp: 80,
            max_hp: 100,
            mp: 40,
            max_mp: 50,
            str: 8,
            def: 8,
            spd: 8,
            mag: 8,
            luck: 8,
            level: 5,
            exp: 0,
        },));

        // Create enemy
        let enemy = world.spawn((Stats {
            hp: 60,
            max_hp: 80,
            mp: 0,
            max_mp: 0,
            str: 12,
            def: 8,
            spd: 7,
            mag: 5,
            luck: 5,
            level: 4,
            exp: 0,
        },));

        (world, player, ally, enemy)
    }

    #[test]
    fn test_auto_battle_ai_creation() {
        let ai = AutoBattleAI::new(BattleStrategy::Aggressive);
        assert!(!ai.is_auto());
        assert_eq!(ai.strategy, BattleStrategy::Aggressive);
        assert!(ai.action_history.is_empty());
    }

    #[test]
    fn test_auto_battle_ai_with_mode() {
        let ai = AutoBattleAI::new(BattleStrategy::Balanced).with_mode(AutoBattleMode::Auto);
        assert!(ai.is_auto());
        assert!(!ai.is_semi_auto());
    }

    #[test]
    fn test_auto_battle_ai_toggle() {
        let mut ai = AutoBattleAI::new(BattleStrategy::Balanced);
        assert!(!ai.is_auto());
        
        ai.toggle_mode();
        assert!(ai.is_auto());
        
        ai.toggle_mode();
        assert!(ai.is_semi_auto());
        
        ai.toggle_mode();
        assert!(!ai.is_auto() && !ai.is_semi_auto());
    }

    #[test]
    fn test_auto_battle_ai_record_action() {
        let mut ai = AutoBattleAI::new(BattleStrategy::Balanced);
        let action = BattleAction {
            actor: Entity::DANGLING,
            action_type: ActionType::Attack,
            target: None,
        };
        
        ai.record_action(action.clone(), 1, 10.0);
        assert_eq!(ai.action_history.len(), 1);
        assert!(ai.last_action.is_some());
        assert_eq!(ai.action_history[0].turn, 1);
        assert_eq!(ai.action_history[0].score, 10.0);
        
        // Test history limit (20 actions)
        for i in 2..27 {
            ai.record_action(action.clone(), i, 10.0);
        }
        assert_eq!(ai.action_history.len(), 20);
    }

    #[test]
    fn test_decision_weights_for_strategy() {
        let aggressive = AIDecisionWeights::for_strategy(BattleStrategy::Aggressive);
        assert!(aggressive.damage_weight > aggressive.healing_weight);
        assert!(aggressive.risk_tolerance > 0.5);

        let defensive = AIDecisionWeights::for_strategy(BattleStrategy::Defensive);
        assert!(defensive.healing_weight > defensive.damage_weight);
        assert!(defensive.risk_tolerance < 0.5);

        let healer = AIDecisionWeights::for_strategy(BattleStrategy::HealerFirst);
        assert!(healer.healing_weight >= 2.0);
    }

    #[test]
    fn test_auto_battle_system_new() {
        let system = AutoBattleSystem::new();
        assert!(system.strategies.contains_key(&BattleStrategy::Balanced));
        assert!(system.strategies.contains_key(&BattleStrategy::Aggressive));
        assert!(system.strategies.contains_key(&BattleStrategy::Defensive));
    }

    #[test]
    fn test_decide_action_not_auto() {
        let (mut world, player, _ally, enemy) = create_test_world_with_stats();
        let system = AutoBattleSystem::new();
        
        // Change player to Manual mode (default from create_test_world_with_stats is Auto)
        if let Ok(mut query) = world.query_one_mut::<&mut AutoBattleAI>(player) {
            query.mode = AutoBattleMode::Manual;
        }
        
        // Entity is in Manual mode, so should return None
        let actions = vec![BattleAction {
            actor: player,
            action_type: ActionType::Attack,
            target: Some(enemy),
        }];
        
        let result = system.decide_action(&world, player, &actions, &[player], &[enemy]);
        assert!(result.is_none());
    }

    #[test]
    fn test_decide_action_auto_mode() {
        let (mut world, player, _ally, enemy) = create_test_world_with_stats();
        let system = AutoBattleSystem::new();
        
        // Change to auto mode
        if let Ok(mut query) = world.query_one_mut::<&mut AutoBattleAI>(player) {
            query.mode = AutoBattleMode::Auto;
        }
        
        let actions = vec![
            BattleAction {
                actor: player,
                action_type: ActionType::Attack,
                target: Some(enemy),
            },
            BattleAction {
                actor: player,
                action_type: ActionType::Defend,
                target: None,
            },
        ];
        
        let result = system.decide_action(&world, player, &actions, &[player], &[enemy]);
        assert!(result.is_some());
    }

    #[test]
    fn test_party_ai_new() {
        let party_ai = PartyAI::new();
        assert_eq!(party_ai.global_mode, AutoBattleMode::Manual);
        assert!(party_ai.individual_settings.is_empty());
    }

    #[test]
    fn test_party_ai_set_ai() {
        let mut party_ai = PartyAI::new();
        let entity = Entity::DANGLING;
        let ai = AutoBattleAI::new(BattleStrategy::Aggressive).with_mode(AutoBattleMode::Auto);
        
        party_ai.set_ai(entity, ai);
        assert!(party_ai.get_ai(entity).is_some());
        assert!(party_ai.has_auto_members());
    }

    #[test]
    fn test_party_ai_toggle_auto() {
        let mut party_ai = PartyAI::new();
        let entity = Entity::DANGLING;
        
        party_ai.set_ai(entity, AutoBattleAI::new(BattleStrategy::Balanced));
        assert!(!party_ai.has_auto_members());
        
        party_ai.toggle_auto(AutoBattleMode::Auto);
        assert!(party_ai.has_auto_members());
        
        if let Some(ai) = party_ai.get_ai(entity) {
            assert!(ai.is_auto());
        }
    }

    #[test]
    fn test_party_ai_set_strategy() {
        let mut party_ai = PartyAI::new();
        let entity = Entity::DANGLING;
        
        party_ai.set_strategy(entity, BattleStrategy::Defensive);
        
        if let Some(ai) = party_ai.get_ai(entity) {
            assert_eq!(ai.strategy, BattleStrategy::Defensive);
        }
    }

    #[test]
    fn test_party_ai_mode_counts() {
        let mut party_ai = PartyAI::new();
        let mut world = World::new();
        
        // Create unique entities using World
        let e1 = world.spawn(());
        let e2 = world.spawn(());
        let e3 = world.spawn(());
        
        party_ai.set_ai(e1, AutoBattleAI::new(BattleStrategy::Balanced).with_mode(AutoBattleMode::Manual));
        party_ai.set_ai(e2, AutoBattleAI::new(BattleStrategy::Balanced).with_mode(AutoBattleMode::Auto));
        party_ai.set_ai(e3, AutoBattleAI::new(BattleStrategy::Balanced).with_mode(AutoBattleMode::SemiAuto));
        
        let (manual, auto, semi) = party_ai.mode_counts();
        assert_eq!(manual, 1);
        assert_eq!(auto, 1);
        assert_eq!(semi, 1);
    }

    #[test]
    fn test_party_ai_remove_entity() {
        let mut party_ai = PartyAI::new();
        let entity = Entity::DANGLING;
        
        party_ai.set_ai(entity, AutoBattleAI::new(BattleStrategy::Balanced));
        assert!(party_ai.get_ai(entity).is_some());
        
        party_ai.remove_entity(entity);
        assert!(party_ai.get_ai(entity).is_none());
    }

    #[test]
    fn test_scored_action() {
        let action = BattleAction {
            actor: Entity::DANGLING,
            action_type: ActionType::Attack,
            target: None,
        };
        
        let scored = ScoredAction {
            action: action.clone(),
            score: 10.5,
            reasoning: "test reasoning".to_string(),
        };
        
        assert_eq!(scored.score, 10.5);
        assert_eq!(scored.reasoning, "test reasoning");
    }

    #[test]
    fn test_battle_strategy_all() {
        let all = BattleStrategy::all();
        assert_eq!(all.len(), 6);
        assert!(all.contains(&BattleStrategy::Balanced));
        assert!(all.contains(&BattleStrategy::Aggressive));
        assert!(all.contains(&BattleStrategy::Defensive));
        assert!(all.contains(&BattleStrategy::HealerFirst));
        assert!(all.contains(&BattleStrategy::TargetWeakest));
        assert!(all.contains(&BattleStrategy::SaveMP));
    }

    #[test]
    fn test_generate_available_actions() {
        let (world, player, ally, enemy) = create_test_world_with_stats();
        let system = AutoBattleSystem::new();
        
        let actions = system.generate_available_actions(&world, player, &[player, ally], &[enemy]);
        
        // Should have: attack on enemy + skills + defend + flee
        assert!(!actions.is_empty());
        
        // Check that we have an attack action
        let has_attack = actions.iter().any(|a| matches!(a.action_type, ActionType::Attack));
        assert!(has_attack);
        
        // Check that we have a defend action
        let has_defend = actions.iter().any(|a| matches!(a.action_type, ActionType::Defend));
        assert!(has_defend);
        
        // Check that we have a flee action
        let has_flee = actions.iter().any(|a| matches!(a.action_type, ActionType::Flee));
        assert!(has_flee);
    }

    #[test]
    fn test_score_action_attack() {
        let (world, player, _ally, enemy) = create_test_world_with_stats();
        let system = AutoBattleSystem::new();
        let ai = AutoBattleAI::new(BattleStrategy::Aggressive);
        
        let action = BattleAction {
            actor: player,
            action_type: ActionType::Attack,
            target: Some(enemy),
        };
        
        let scored = system.score_action(&world, player, &action, &ai, &[player], &[enemy]);
        
        // Aggressive strategy should give attack a good score
        assert!(scored.score > 0.0);
        assert!(scored.reasoning.contains("attack"));
    }

    #[test]
    fn test_score_action_defend_low_hp() {
        let (mut world, player, _ally, enemy) = create_test_world_with_stats();
        let system = AutoBattleSystem::new();
        let ai = AutoBattleAI::new(BattleStrategy::Defensive);
        
        // Reduce player HP to trigger defend bonus
        if let Ok(mut query) = world.query_one_mut::<&mut Stats>(player) {
            query.hp = 10; // Low HP
        }
        
        let action = BattleAction {
            actor: player,
            action_type: ActionType::Defend,
            target: None,
        };
        
        let scored = system.score_action(&world, player, &action, &ai, &[player], &[enemy]);
        
        // Should have a defend score
        assert!(scored.reasoning.contains("defend"));
    }

    #[test]
    fn test_score_action_heal_full_hp() {
        let (mut world, player, ally, _enemy) = create_test_world_with_stats();
        let system = AutoBattleSystem::new();
        let ai = AutoBattleAI::new(BattleStrategy::HealerFirst);
        
        // Set ally to full HP so the "full HP" penalty is triggered
        if let Ok(mut query) = world.query_one_mut::<&mut Stats>(ally) {
            query.hp = query.max_hp;
        }
        
        let action = BattleAction {
            actor: player,
            action_type: ActionType::Skill(3), // Heal skill
            target: Some(ally),
        };
        
        let scored = system.score_action(&world, player, &action, &ai, &[player, ally], &[]);
        
        // Healing full HP should be penalized
        assert!(scored.reasoning.contains("full HP"));
    }

    #[test]
    fn test_score_action_flee_weak_party() {
        let (mut world, player, _ally, enemy) = create_test_world_with_stats();
        let system = AutoBattleSystem::new();
        let ai = AutoBattleAI::new(BattleStrategy::Balanced);
        
        // Reduce player HP to trigger flee bonus
        if let Ok(mut query) = world.query_one_mut::<&mut Stats>(player) {
            query.hp = 5; // Very low HP
            query.max_hp = 100;
        }
        
        let action = BattleAction {
            actor: player,
            action_type: ActionType::Flee,
            target: None,
        };
        
        let scored = system.score_action(&world, player, &action, &ai, &[player], &[enemy]);
        
        // Weak party should have flee bonus
        assert!(scored.score > 0.0);
    }

    #[test]
    fn test_healer_first_strategy_prioritizes_healing() {
        let healer = AIDecisionWeights::for_strategy(BattleStrategy::HealerFirst);
        let aggressive = AIDecisionWeights::for_strategy(BattleStrategy::Aggressive);
        
        assert!(healer.healing_weight > aggressive.healing_weight);
        assert!(healer.damage_weight < aggressive.damage_weight);
    }

    #[test]
    fn test_save_mp_strategy_conserves_mp() {
        let save_mp = AIDecisionWeights::for_strategy(BattleStrategy::SaveMP);
        let aggressive = AIDecisionWeights::for_strategy(BattleStrategy::Aggressive);
        
        assert!(save_mp.mp_conservation > aggressive.mp_conservation);
    }
}
