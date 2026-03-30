//! Battle Log/Text History for the DocDamage Engine
//!
//! Text log of battle events for UI display and debugging.
//! Provides structured logging with filtering, styling, and builder patterns.

use std::collections::VecDeque;
use dde_core::{Entity, Element};

/// Battle log entry types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogEntryType {
    /// Battle has started
    BattleStart,
    /// A turn has started
    TurnStart,
    /// Generic action occurred
    Action,
    /// Damage was dealt
    Damage,
    /// Healing was applied
    Heal,
    /// An attack missed
    Miss,
    /// A critical hit occurred
    Crit,
    /// A status effect was applied
    StatusApplied,
    /// A status effect was removed
    StatusRemoved,
    /// A status effect ticked
    StatusTick,
    /// An item was used
    ItemUse,
    /// A skill was used
    SkillUse,
    /// An entity attempted to flee
    Flee,
    /// An entity was defeated
    Defeat,
    /// Battle victory
    Victory,
    /// Successfully escaped
    Escape,
    /// System message
    System,
    /// Error occurred
    Error,
}

/// Log severity levels for UI coloring
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogSeverity {
    /// Informational message
    Info,
    /// Success - heals, buffs (Green)
    Success,
    /// Warning - misses, resists (Yellow)
    Warning,
    /// Danger - damage, defeats (Red)
    Danger,
    /// Critical - crits, level ups (Purple)
    Critical,
}

/// Battle log entry
#[derive(Debug, Clone, PartialEq)]
pub struct LogEntry {
    /// Game tick when entry was created
    pub tick: u64,
    /// Turn number
    pub turn: u32,
    /// Type of log entry
    pub entry_type: LogEntryType,
    /// Display message
    pub message: String,
    /// Actor entity (if applicable)
    pub actor: Option<Entity>,
    /// Target entity (if applicable)
    pub target: Option<Entity>,
    /// Timestamp for when the entry was created (for replay/debugging)
    pub timestamp: std::time::SystemTime,
    /// Damage dealt (if applicable)
    pub damage_dealt: Option<u32>,
    /// Healing done (if applicable)
    pub healing_done: Option<u32>,
}

impl LogEntry {
    /// Create a new log entry
    pub fn new(tick: u64, turn: u32, entry_type: LogEntryType, message: impl Into<String>) -> Self {
        Self {
            tick,
            turn,
            entry_type,
            message: message.into(),
            actor: None,
            target: None,
            timestamp: std::time::SystemTime::now(),
            damage_dealt: None,
            healing_done: None,
        }
    }

    /// Set damage dealt value
    pub fn with_damage(mut self, damage: u32) -> Self {
        self.damage_dealt = Some(damage);
        self
    }

    /// Set healing done value
    pub fn with_healing(mut self, healing: u32) -> Self {
        self.healing_done = Some(healing);
        self
    }

    /// Set the actor entity
    pub fn with_actor(mut self, actor: Entity) -> Self {
        self.actor = Some(actor);
        self
    }

    /// Set the target entity
    pub fn with_target(mut self, target: Entity) -> Self {
        self.target = Some(target);
        self
    }

    /// Get the severity level for this entry type
    pub fn severity(&self) -> LogSeverity {
        self.entry_type.severity()
    }

    /// Get the style for this entry type
    pub fn style(&self) -> LogStyle {
        self.entry_type.style()
    }
}

/// Styling for log display
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LogStyle {
    /// Prefix to add to the message
    pub prefix: &'static str,
    /// RGB color tuple
    pub color: (u8, u8, u8),
    /// Whether to display in bold
    pub bold: bool,
    /// Optional icon/emoji
    pub icon: Option<&'static str>,
}

impl Default for LogStyle {
    fn default() -> Self {
        Self {
            prefix: "",
            color: (255, 255, 255),
            bold: false,
            icon: None,
        }
    }
}

impl LogEntryType {
    /// Get the display style for this entry type
    pub fn style(&self) -> LogStyle {
        match self {
            LogEntryType::Damage => LogStyle {
                prefix: "",
                color: (255, 100, 100), // Red
                bold: false,
                icon: Some("⚔️"),
            },
            LogEntryType::Crit => LogStyle {
                prefix: "",
                color: (255, 50, 50),
                bold: true,
                icon: Some("💥"),
            },
            LogEntryType::Heal => LogStyle {
                prefix: "+",
                color: (100, 255, 100), // Green
                bold: false,
                icon: Some("💚"),
            },
            LogEntryType::Miss => LogStyle {
                prefix: "",
                color: (200, 200, 200), // Gray
                bold: false,
                icon: Some("💨"),
            },
            LogEntryType::StatusApplied => LogStyle {
                prefix: "",
                color: (255, 200, 100), // Orange
                bold: false,
                icon: Some("⚡"),
            },
            LogEntryType::StatusRemoved => LogStyle {
                prefix: "",
                color: (100, 200, 255), // Light Blue
                bold: false,
                icon: Some("✨"),
            },
            LogEntryType::Victory => LogStyle {
                prefix: "",
                color: (255, 215, 0), // Gold
                bold: true,
                icon: Some("🏆"),
            },
            LogEntryType::Defeat => LogStyle {
                prefix: "",
                color: (139, 0, 0), // Dark Red
                bold: true,
                icon: Some("💀"),
            },
            LogEntryType::Flee | LogEntryType::Escape => LogStyle {
                prefix: "",
                color: (100, 100, 255), // Blue
                bold: false,
                icon: Some("🏃"),
            },
            LogEntryType::ItemUse => LogStyle {
                prefix: "",
                color: (255, 165, 0), // Orange
                bold: false,
                icon: Some("🎒"),
            },
            LogEntryType::SkillUse => LogStyle {
                prefix: "",
                color: (147, 112, 219), // Medium Purple
                bold: false,
                icon: Some("✨"),
            },
            LogEntryType::TurnStart => LogStyle {
                prefix: "►",
                color: (255, 255, 200), // Light Yellow
                bold: true,
                icon: None,
            },
            LogEntryType::BattleStart => LogStyle {
                prefix: "",
                color: (100, 255, 255), // Cyan
                bold: true,
                icon: Some("⚔️"),
            },
            LogEntryType::Error => LogStyle {
                prefix: "ERROR:",
                color: (255, 0, 0),
                bold: true,
                icon: Some("❌"),
            },
            LogEntryType::System => LogStyle {
                prefix: "",
                color: (200, 200, 200),
                bold: false,
                icon: Some("ℹ️"),
            },
            _ => LogStyle::default(),
        }
    }

    /// Get the severity level for this entry type
    pub fn severity(&self) -> LogSeverity {
        match self {
            LogEntryType::Crit | LogEntryType::Victory => LogSeverity::Critical,
            LogEntryType::Damage | LogEntryType::Defeat => LogSeverity::Danger,
            LogEntryType::Heal | LogEntryType::StatusRemoved => LogSeverity::Success,
            LogEntryType::Miss | LogEntryType::Flee => LogSeverity::Warning,
            LogEntryType::Error => LogSeverity::Danger,
            _ => LogSeverity::Info,
        }
    }
}

/// Battle log with scrolling history
#[derive(Debug, Clone)]
pub struct BattleLog {
    entries: VecDeque<LogEntry>,
    max_entries: usize,
    current_turn: u32,
    current_tick: u64,
}

impl BattleLog {
    /// Create new log with max history size
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: VecDeque::with_capacity(max_entries),
            max_entries,
            current_turn: 0,
            current_tick: 0,
        }
    }

    /// Add entry to log
    pub fn add(&mut self, entry: LogEntry) {
        if self.entries.len() >= self.max_entries {
            self.entries.pop_front();
        }
        self.entries.push_back(entry);
    }

    /// Add a simple message entry
    pub fn add_message(&mut self, message: impl Into<String>) {
        self.add(LogEntry::new(
            self.current_tick,
            self.current_turn,
            LogEntryType::System,
            message,
        ));
    }

    /// Add an error entry
    pub fn add_error(&mut self, message: impl Into<String>) {
        self.add(LogEntry::new(
            self.current_tick,
            self.current_turn,
            LogEntryType::Error,
            message,
        ));
    }

    /// Builder pattern for entries
    pub fn entry(&self) -> LogBuilder {
        LogBuilder::new(self.current_tick, self.current_turn)
    }

    /// Get recent entries (most recent first)
    pub fn recent(&self, count: usize) -> Vec<&LogEntry> {
        self.entries.iter().rev().take(count).collect()
    }

    /// Get entries by type
    pub fn filter_by_type(&self, entry_type: LogEntryType) -> Vec<&LogEntry> {
        self.entries
            .iter()
            .filter(|e| e.entry_type == entry_type)
            .collect()
    }

    /// Get entries for specific entity
    pub fn filter_by_entity(&self, entity: Entity) -> Vec<&LogEntry> {
        self.entries
            .iter()
            .filter(|e| e.actor == Some(entity) || e.target == Some(entity))
            .collect()
    }

    /// Get entries by severity
    pub fn filter_by_severity(&self, severity: LogSeverity) -> Vec<&LogEntry> {
        self.entries
            .iter()
            .filter(|e| e.severity() == severity)
            .collect()
    }

    /// Get entries from current turn only
    pub fn current_turn_entries(&self) -> Vec<&LogEntry> {
        self.entries.iter().filter(|e| e.turn == self.current_turn).collect()
    }

    /// Get entries from a specific turn range
    pub fn entries_in_turn_range(&self, start: u32, end: u32) -> Vec<&LogEntry> {
        self.entries
            .iter()
            .filter(|e| e.turn >= start && e.turn <= end)
            .collect()
    }

    /// Clear all entries
    pub fn clear(&mut self) {
        self.entries.clear();
        self.current_turn = 0;
        self.current_tick = 0;
    }

    /// Advance turn counter
    pub fn next_turn(&mut self) {
        self.current_turn += 1;
    }

    /// Advance tick counter
    pub fn tick(&mut self) {
        self.current_tick += 1;
    }

    /// Get current turn number
    pub fn current_turn(&self) -> u32 {
        self.current_turn
    }

    /// Get current tick
    pub fn current_tick(&self) -> u64 {
        self.current_tick
    }

    /// Set current turn (for initialization/loading)
    pub fn set_turn(&mut self, turn: u32) {
        self.current_turn = turn;
    }

    /// Set current tick (for initialization/loading)
    pub fn set_tick(&mut self, tick: u64) {
        self.current_tick = tick;
    }

    /// Export to string for debugging/save
    pub fn to_formatted_string(&self) -> String {
        self.entries
            .iter()
            .map(|e| format!("[Turn {}] {}", e.turn, e.message))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Export with full details for debugging
    pub fn to_detailed_string(&self) -> String {
        self.entries
            .iter()
            .map(|e| {
                let actor = e.actor.map(|a| format!("{:?}", a)).unwrap_or_else(|| "-".to_string());
                let target = e.target.map(|t| format!("{:?}", t)).unwrap_or_else(|| "-".to_string());
                format!(
                    "[Tick {} | Turn {} | {:?}] {} (actor: {}, target: {})",
                    e.tick, e.turn, e.entry_type, e.message, actor, target
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Get entry count
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if log is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get all entries as a slice
    pub fn entries(&self) -> &VecDeque<LogEntry> {
        &self.entries
    }

    /// Get the most recent entry
    pub fn last(&self) -> Option<&LogEntry> {
        self.entries.back()
    }

    /// Get the first entry
    pub fn first(&self) -> Option<&LogEntry> {
        self.entries.front()
    }

    /// Get iterator over entries
    pub fn iter(&self) -> impl Iterator<Item = &LogEntry> {
        self.entries.iter()
    }

    /// Get mutable iterator over entries
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut LogEntry> {
        self.entries.iter_mut()
    }

    /// Get the max entries capacity
    pub fn max_entries(&self) -> usize {
        self.max_entries
    }

    /// Change max entries (may truncate existing)
    pub fn set_max_entries(&mut self, max: usize) {
        self.max_entries = max;
        while self.entries.len() > self.max_entries {
            self.entries.pop_front();
        }
    }

    /// Get all unique combatants (entities that appear as actor or target)
    pub fn get_combatants(&self) -> Vec<Entity> {
        let mut combatants = std::collections::HashSet::new();
        for entry in &self.entries {
            if let Some(actor) = entry.actor {
                combatants.insert(actor);
            }
            if let Some(target) = entry.target {
                combatants.insert(target);
            }
        }
        combatants.into_iter().collect()
    }

    /// Get total damage dealt by a combatant
    pub fn get_total_damage_dealt(&self, entity: Entity) -> u32 {
        self.entries
            .iter()
            .filter(|e| e.actor == Some(entity))
            .filter(|e| matches!(e.entry_type, LogEntryType::Damage | LogEntryType::Crit))
            .filter_map(|e| e.damage_dealt)
            .sum()
    }

    /// Get total damage taken by a combatant
    pub fn get_total_damage_taken(&self, entity: Entity) -> u32 {
        self.entries
            .iter()
            .filter(|e| e.target == Some(entity))
            .filter(|e| matches!(e.entry_type, LogEntryType::Damage | LogEntryType::Crit))
            .filter_map(|e| e.damage_dealt)
            .sum()
    }

    /// Get total healing done by a combatant
    pub fn get_total_healing_done(&self, entity: Entity) -> u32 {
        self.entries
            .iter()
            .filter(|e| e.actor == Some(entity))
            .filter(|e| e.entry_type == LogEntryType::Heal)
            .filter_map(|e| e.healing_done)
            .sum()
    }

    /// Get total healing received by a combatant
    pub fn get_total_healing_received(&self, entity: Entity) -> u32 {
        self.entries
            .iter()
            .filter(|e| e.target == Some(entity))
            .filter(|e| e.entry_type == LogEntryType::Heal)
            .filter_map(|e| e.healing_done)
            .sum()
    }

    /// Get action count by type for a combatant
    pub fn get_action_count(&self, entity: Entity, entry_type: LogEntryType) -> usize {
        self.entries
            .iter()
            .filter(|e| e.actor == Some(entity) && e.entry_type == entry_type)
            .count()
    }

    /// Get all turns that have entries
    pub fn get_turns_with_entries(&self) -> Vec<u32> {
        let mut turns: Vec<u32> = self.entries
            .iter()
            .map(|e| e.turn)
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        turns.sort_unstable();
        turns
    }

    /// Get entries for a specific turn
    pub fn entries_for_turn(&self, turn: u32) -> Vec<&LogEntry> {
        self.entries.iter().filter(|e| e.turn == turn).collect()
    }

    /// Get entry at specific index
    pub fn get(&self, index: usize) -> Option<&LogEntry> {
        self.entries.get(index)
    }

    /// Get entry index by turn and tick (for replay navigation)
    pub fn find_entry_index(&self, turn: u32, tick: u64) -> Option<usize> {
        self.entries
            .iter()
            .position(|e| e.turn == turn && e.tick == tick)
    }

    /// Get the highest turn number in the log
    pub fn max_turn(&self) -> u32 {
        self.entries.iter().map(|e| e.turn).max().unwrap_or(0)
    }

    /// Get statistics summary for all combatants
    pub fn get_statistics_summary(&self) -> CombatantStatistics {
        CombatantStatistics::from_log(self)
    }
}

/// Statistics for a single combatant
#[derive(Debug, Clone, Default)]
pub struct CombatantStats {
    /// Total damage dealt
    pub damage_dealt: u32,
    /// Total damage taken
    pub damage_taken: u32,
    /// Total healing done
    pub healing_done: u32,
    /// Total healing received
    pub healing_received: u32,
    /// Number of attacks made
    pub attacks_made: usize,
    /// Number of critical hits
    pub crits_made: usize,
    /// Number of heals performed
    pub heals_performed: usize,
    /// Number of times missed
    pub misses: usize,
    /// Number of status effects applied
    pub statuses_applied: usize,
    /// Number of status effects received
    pub statuses_received: usize,
}

/// Statistics summary for all combatants
#[derive(Debug, Clone)]
pub struct CombatantStatistics {
    /// Stats per combatant entity
    pub stats: std::collections::HashMap<Entity, CombatantStats>,
}

impl CombatantStatistics {
    /// Calculate statistics from a battle log
    pub fn from_log(log: &BattleLog) -> Self {
        let mut stats: std::collections::HashMap<Entity, CombatantStats> = std::collections::HashMap::new();

        for entry in log.entries() {
            // Process actor stats
            if let Some(actor) = entry.actor {
                let actor_stats = stats.entry(actor).or_default();
                
                match entry.entry_type {
                    LogEntryType::Damage => {
                        actor_stats.attacks_made += 1;
                        if let Some(dmg) = entry.damage_dealt {
                            actor_stats.damage_dealt += dmg;
                        }
                    }
                    LogEntryType::Crit => {
                        actor_stats.attacks_made += 1;
                        actor_stats.crits_made += 1;
                        if let Some(dmg) = entry.damage_dealt {
                            actor_stats.damage_dealt += dmg;
                        }
                    }
                    LogEntryType::Heal => {
                        actor_stats.heals_performed += 1;
                        if let Some(heal) = entry.healing_done {
                            actor_stats.healing_done += heal;
                        }
                    }
                    LogEntryType::Miss => {
                        actor_stats.attacks_made += 1;
                        actor_stats.misses += 1;
                    }
                    LogEntryType::StatusApplied => {
                        actor_stats.statuses_applied += 1;
                    }
                    _ => {}
                }
            }

            // Process target stats
            if let Some(target) = entry.target {
                let target_stats = stats.entry(target).or_default();
                
                match entry.entry_type {
                    LogEntryType::Damage | LogEntryType::Crit => {
                        if let Some(dmg) = entry.damage_dealt {
                            target_stats.damage_taken += dmg;
                        }
                    }
                    LogEntryType::Heal => {
                        if let Some(heal) = entry.healing_done {
                            target_stats.healing_received += heal;
                        }
                    }
                    LogEntryType::StatusApplied => {
                        target_stats.statuses_received += 1;
                    }
                    _ => {}
                }
            }
        }

        Self { stats }
    }

    /// Get stats for a specific combatant
    pub fn get(&self, entity: Entity) -> Option<&CombatantStats> {
        self.stats.get(&entity)
    }

    /// Get all combatant entities
    pub fn combatants(&self) -> Vec<Entity> {
        self.stats.keys().copied().collect()
    }

    /// Get total damage dealt across all combatants
    pub fn total_damage_dealt(&self) -> u32 {
        self.stats.values().map(|s| s.damage_dealt).sum()
    }

    /// Get total healing done across all combatants
    pub fn total_healing_done(&self) -> u32 {
        self.stats.values().map(|s| s.healing_done).sum()
    }
}

impl Default for BattleLog {
    fn default() -> Self {
        Self::new(100) // Default 100 entry history
    }
}

/// Log builder for fluent entry creation
#[derive(Debug, Clone)]
pub struct LogBuilder {
    entry: LogEntry,
}

impl LogBuilder {
    /// Create a new log builder
    fn new(tick: u64, turn: u32) -> Self {
        Self {
            entry: LogEntry {
                tick,
                turn,
                entry_type: LogEntryType::System,
                message: String::new(),
                actor: None,
                target: None,
                timestamp: std::time::SystemTime::now(),
                damage_dealt: None,
                healing_done: None,
            },
        }
    }

    /// Set battle start message
    pub fn battle_start(mut self) -> Self {
        self.entry.entry_type = LogEntryType::BattleStart;
        self.entry.message = "Battle started!".to_string();
        self
    }

    /// Set turn start message
    pub fn turn_start(mut self, entity: Entity, name: &str) -> Self {
        self.entry.entry_type = LogEntryType::TurnStart;
        self.entry.actor = Some(entity);
        self.entry.message = format!("{}'s turn!", name);
        self
    }

    /// Set damage message
    pub fn damage(
        mut self,
        attacker: Entity,
        defender: Entity,
        attacker_name: &str,
        defender_name: &str,
        amount: u32,
        element: Element,
        critical: bool,
    ) -> Self {
        self.entry.entry_type = if critical {
            LogEntryType::Crit
        } else {
            LogEntryType::Damage
        };
        self.entry.actor = Some(attacker);
        self.entry.target = Some(defender);
        self.entry.damage_dealt = Some(amount);

        let crit_str = if critical { " Critical hit!" } else { "" };
        self.entry.message = format!(
            "{} deals {} {} damage to {}!{}",
            attacker_name,
            amount,
            element.name(),
            defender_name,
            crit_str
        );
        self
    }

    /// Set heal message
    pub fn heal(
        mut self,
        healer: Option<Entity>,
        target: Entity,
        healer_name: Option<&str>,
        target_name: &str,
        amount: u32,
    ) -> Self {
        self.entry.entry_type = LogEntryType::Heal;
        self.entry.actor = healer;
        self.entry.target = Some(target);
        self.entry.healing_done = Some(amount);

        self.entry.message = match healer_name {
            Some(name) => format!("{} heals {} for {} HP!", name, target_name, amount),
            None => format!("{} recovers {} HP!", target_name, amount),
        };
        self
    }

    /// Set skill use message
    pub fn skill_use(
        mut self,
        user: Entity,
        target: Option<Entity>,
        user_name: &str,
        skill_name: &str,
    ) -> Self {
        self.entry.entry_type = LogEntryType::SkillUse;
        self.entry.actor = Some(user);
        self.entry.target = target;
        self.entry.message = format!("{} uses {}!", user_name, skill_name);
        self
    }

    /// Set item use message
    pub fn item_use(
        mut self,
        user: Entity,
        target: Option<Entity>,
        user_name: &str,
        item_name: &str,
    ) -> Self {
        self.entry.entry_type = LogEntryType::ItemUse;
        self.entry.actor = Some(user);
        self.entry.target = target;
        self.entry.message = format!("{} uses {}!", user_name, item_name);
        self
    }

    /// Set status effect message
    pub fn status(
        mut self,
        target: Entity,
        target_name: &str,
        status_name: &str,
        applied: bool,
    ) -> Self {
        self.entry.entry_type = if applied {
            LogEntryType::StatusApplied
        } else {
            LogEntryType::StatusRemoved
        };
        self.entry.target = Some(target);
        self.entry.message = if applied {
            format!("{} is affected by {}!", target_name, status_name)
        } else {
            format!("{} is no longer {}!", target_name, status_name)
        };
        self
    }

    /// Set status tick message (for DOT/HOT effects)
    pub fn status_tick(
        mut self,
        target: Entity,
        target_name: &str,
        status_name: &str,
        damage: Option<u32>,
        healing: Option<u32>,
    ) -> Self {
        self.entry.entry_type = LogEntryType::StatusTick;
        self.entry.target = Some(target);
        
        self.entry.message = if let Some(dmg) = damage {
            format!("{} takes {} damage from {}!", target_name, dmg, status_name)
        } else if let Some(heal) = healing {
            format!("{} recovers {} HP from {}!", target_name, heal, status_name)
        } else {
            format!("{}'s {} ticked!", target_name, status_name)
        };
        self
    }

    /// Set miss message
    pub fn miss(
        mut self,
        attacker: Entity,
        defender: Entity,
        attacker_name: &str,
        defender_name: &str,
    ) -> Self {
        self.entry.entry_type = LogEntryType::Miss;
        self.entry.actor = Some(attacker);
        self.entry.target = Some(defender);
        self.entry.message = format!("{} misses {}!", attacker_name, defender_name);
        self
    }

    /// Set defeat message
    pub fn defeat(mut self, entity: Entity, name: &str) -> Self {
        self.entry.entry_type = LogEntryType::Defeat;
        self.entry.target = Some(entity);
        self.entry.message = format!("{} is defeated!", name);
        self
    }

    /// Set victory message
    pub fn victory(mut self) -> Self {
        self.entry.entry_type = LogEntryType::Victory;
        self.entry.message = "Victory!".to_string();
        self
    }

    /// Set flee message
    pub fn flee(mut self, name: &str, success: bool) -> Self {
        self.entry.entry_type = LogEntryType::Flee;
        self.entry.message = if success {
            format!("{} escaped successfully!", name)
        } else {
            format!("{} failed to escape!", name)
        };
        self
    }

    /// Set escape message (for whole party)
    pub fn escape(mut self, success: bool) -> Self {
        self.entry.entry_type = LogEntryType::Escape;
        self.entry.message = if success {
            "The party escaped successfully!".to_string()
        } else {
            "Escape failed!".to_string()
        };
        self
    }

    /// Set action message
    pub fn action(mut self, actor: Entity, name: &str, action: &str) -> Self {
        self.entry.entry_type = LogEntryType::Action;
        self.entry.actor = Some(actor);
        self.entry.message = format!("{} {}!", name, action);
        self
    }

    /// Set system message
    pub fn system(mut self, message: impl Into<String>) -> Self {
        self.entry.entry_type = LogEntryType::System;
        self.entry.message = message.into();
        self
    }

    /// Set error message
    pub fn error(mut self, message: impl Into<String>) -> Self {
        self.entry.entry_type = LogEntryType::Error;
        self.entry.message = message.into();
        self
    }

    /// Set custom message with type
    pub fn message(mut self, entry_type: LogEntryType, message: impl Into<String>) -> Self {
        self.entry.entry_type = entry_type;
        self.entry.message = message.into();
        self
    }

    /// Set the actor entity
    pub fn with_actor(mut self, actor: Entity) -> Self {
        self.entry.actor = Some(actor);
        self
    }

    /// Set the target entity
    pub fn with_target(mut self, target: Entity) -> Self {
        self.entry.target = Some(target);
        self
    }

    /// Set both actor and target
    pub fn with_entities(mut self, actor: Entity, target: Entity) -> Self {
        self.entry.actor = Some(actor);
        self.entry.target = Some(target);
        self
    }

    /// Build the log entry
    pub fn build(self) -> LogEntry {
        self.entry
    }
}

/// Extension trait for Element to get display name
pub trait ElementExt {
    fn name(&self) -> &'static str;
}

impl ElementExt for Element {
    fn name(&self) -> &'static str {
        match self {
            Element::None => "Physical",
            Element::Fire => "Fire",
            Element::Ice => "Ice",
            Element::Lightning => "Lightning",
            Element::Holy => "Holy",
            Element::Dark => "Dark",
        }
    }
}

#[cfg(feature = "ui")]
/// UI integration for egui
pub mod ui {
    use super::*;

    /// Battle log UI component
    pub struct BattleLogUi {
        scroll_to_bottom: bool,
        filter: Option<LogEntryType>,
    }

    impl BattleLogUi {
        /// Create a new battle log UI
        pub fn new() -> Self {
            Self {
                scroll_to_bottom: true,
                filter: None,
            }
        }

        /// Create with initial filter
        pub fn with_filter(filter: LogEntryType) -> Self {
            Self {
                scroll_to_bottom: true,
                filter: Some(filter),
            }
        }

        /// Set whether to auto-scroll to bottom
        pub fn set_auto_scroll(&mut self, enabled: bool) {
            self.scroll_to_bottom = enabled;
        }

        /// Set filter (None to show all)
        pub fn set_filter(&mut self, filter: Option<LogEntryType>) {
            self.filter = filter;
        }

        /// Clear filter
        pub fn clear_filter(&mut self) {
            self.filter = None;
        }

        /// Draw the battle log UI
        pub fn draw(&mut self, ui: &mut egui::Ui, log: &BattleLog) {
            egui::ScrollArea::vertical()
                .max_height(200.0)
                .show(ui, |ui| {
                    for entry in log.entries().iter() {
                        if let Some(filter) = self.filter {
                            if entry.entry_type != filter {
                                continue;
                            }
                        }

                        let style = entry.entry_type.style();
                        let color = egui::Color32::from_rgb(
                            style.color.0,
                            style.color.1,
                            style.color.2,
                        );

                        let text = if let Some(icon) = style.icon {
                            if style.prefix.is_empty() {
                                format!("{} {}", icon, entry.message)
                            } else {
                                format!("{} {} {}", icon, style.prefix, entry.message)
                            }
                        } else if !style.prefix.is_empty() {
                            format!("{} {}", style.prefix, entry.message)
                        } else {
                            entry.message.clone()
                        };

                        let label = if style.bold {
                            egui::RichText::new(text).color(color).strong()
                        } else {
                            egui::RichText::new(text).color(color)
                        };

                        ui.label(label);
                    }

                    if self.scroll_to_bottom {
                        ui.scroll_to_cursor(Some(egui::Align::BOTTOM));
                    }
                });
        }

        /// Draw with custom max height
        pub fn draw_sized(&mut self, ui: &mut egui::Ui, log: &BattleLog, max_height: f32) {
            egui::ScrollArea::vertical()
                .max_height(max_height)
                .show(ui, |ui| {
                    for entry in log.entries().iter() {
                        if let Some(filter) = self.filter {
                            if entry.entry_type != filter {
                                continue;
                            }
                        }

                        let style = entry.entry_type.style();
                        let color = egui::Color32::from_rgb(
                            style.color.0,
                            style.color.1,
                            style.color.2,
                        );

                        let text = if let Some(icon) = style.icon {
                            format!("{} {}", icon, entry.message)
                        } else {
                            entry.message.clone()
                        };

                        let label = if style.bold {
                            egui::RichText::new(text).color(color).strong()
                        } else {
                            egui::RichText::new(text).color(color)
                        };

                        ui.label(label);
                    }

                    if self.scroll_to_bottom {
                        ui.scroll_to_cursor(Some(egui::Align::BOTTOM));
                    }
                });
        }
    }

    impl Default for BattleLogUi {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dde_core::World;

    fn create_test_world_with_entities(count: usize) -> (World, Vec<Entity>) {
        let mut world = World::new();
        let mut entities = Vec::new();
        for _ in 0..count {
            entities.push(world.spawn(()));
        }
        (world, entities)
    }

    #[test]
    fn test_log_entry_creation() {
        let entry = LogEntry::new(1, 1, LogEntryType::System, "Test message");
        assert_eq!(entry.tick, 1);
        assert_eq!(entry.turn, 1);
        assert_eq!(entry.entry_type, LogEntryType::System);
        assert_eq!(entry.message, "Test message");
        assert!(entry.actor.is_none());
        assert!(entry.target.is_none());
    }

    #[test]
    fn test_log_entry_with_entities() {
        let (mut world, entities) = create_test_world_with_entities(2);
        let actor = entities[0];
        let target = entities[1];
        
        let entry = LogEntry::new(1, 1, LogEntryType::Damage, "Attack")
            .with_actor(actor)
            .with_target(target);
        
        assert_eq!(entry.actor, Some(actor));
        assert_eq!(entry.target, Some(target));
    }

    #[test]
    fn test_battle_log_basic() {
        let mut log = BattleLog::new(10);
        assert!(log.is_empty());
        assert_eq!(log.len(), 0);

        log.add(LogEntry::new(1, 1, LogEntryType::System, "Entry 1"));
        assert!(!log.is_empty());
        assert_eq!(log.len(), 1);

        log.clear();
        assert!(log.is_empty());
    }

    #[test]
    fn test_battle_log_max_size() {
        let mut log = BattleLog::new(3);
        
        log.add(LogEntry::new(1, 1, LogEntryType::System, "Entry 1"));
        log.add(LogEntry::new(2, 1, LogEntryType::System, "Entry 2"));
        log.add(LogEntry::new(3, 1, LogEntryType::System, "Entry 3"));
        log.add(LogEntry::new(4, 1, LogEntryType::System, "Entry 4"));
        
        assert_eq!(log.len(), 3);
        // First entry should have been removed
        assert!(log.entries().front().unwrap().message.contains("Entry 2"));
    }

    #[test]
    fn test_builder_pattern() {
        let log = BattleLog::new(10);
        let entry = log.entry()
            .system("System message")
            .build();
        
        assert_eq!(entry.entry_type, LogEntryType::System);
        assert_eq!(entry.message, "System message");
    }

    #[test]
    fn test_builder_damage() {
        let log = BattleLog::new(10);
        let (mut world, entities) = create_test_world_with_entities(2);
        let attacker = entities[0];
        let defender = entities[1];
        
        let entry = log.entry()
            .damage(
                attacker,
                defender,
                "Hero",
                "Goblin",
                50,
                Element::Fire,
                false,
            )
            .build();
        
        assert_eq!(entry.entry_type, LogEntryType::Damage);
        assert_eq!(entry.actor, Some(attacker));
        assert_eq!(entry.target, Some(defender));
        assert!(entry.message.contains("Hero"));
        assert!(entry.message.contains("Goblin"));
        assert!(entry.message.contains("50"));
        assert!(entry.message.contains("Fire"));
    }

    #[test]
    fn test_builder_critical_damage() {
        let log = BattleLog::new(10);
        let (mut world, entities) = create_test_world_with_entities(2);
        let attacker = entities[0];
        let defender = entities[1];
        
        let entry = log.entry()
            .damage(
                attacker,
                defender,
                "Hero",
                "Goblin",
                100,
                Element::None,
                true,
            )
            .build();
        
        assert_eq!(entry.entry_type, LogEntryType::Crit);
        assert!(entry.message.contains("Critical hit"));
    }

    #[test]
    fn test_builder_heal() {
        let log = BattleLog::new(10);
        let (mut world, entities) = create_test_world_with_entities(2);
        let healer = entities[0];
        let target = entities[1];
        
        let entry = log.entry()
            .heal(Some(healer), target, Some("Cleric"), "Hero", 30)
            .build();
        
        assert_eq!(entry.entry_type, LogEntryType::Heal);
        assert!(entry.message.contains("Cleric heals Hero"));
        assert!(entry.message.contains("30"));
    }

    #[test]
    fn test_builder_heal_no_healer() {
        let log = BattleLog::new(10);
        let (mut world, entities) = create_test_world_with_entities(1);
        let target = entities[0];
        
        let entry = log.entry()
            .heal(None, target, None, "Hero", 50)
            .build();
        
        assert!(entry.message.contains("Hero recovers"));
        assert!(!entry.message.contains("heals"));
    }

    #[test]
    fn test_builder_skill_use() {
        let log = BattleLog::new(10);
        let (mut world, entities) = create_test_world_with_entities(2);
        let user = entities[0];
        let target = entities[1];
        
        let entry = log.entry()
            .skill_use(user, Some(target), "Hero", "Fireball")
            .build();
        
        assert_eq!(entry.entry_type, LogEntryType::SkillUse);
        assert_eq!(entry.actor, Some(user));
        assert_eq!(entry.target, Some(target));
        assert!(entry.message.contains("Fireball"));
    }

    #[test]
    fn test_builder_item_use() {
        let log = BattleLog::new(10);
        let (mut world, entities) = create_test_world_with_entities(1);
        let user = entities[0];
        
        let entry = log.entry()
            .item_use(user, None, "Hero", "Potion")
            .build();
        
        assert_eq!(entry.entry_type, LogEntryType::ItemUse);
        assert!(entry.message.contains("Potion"));
    }

    #[test]
    fn test_builder_status_applied() {
        let log = BattleLog::new(10);
        let (mut world, entities) = create_test_world_with_entities(1);
        let target = entities[0];
        
        let entry = log.entry()
            .status(target, "Hero", "Poison", true)
            .build();
        
        assert_eq!(entry.entry_type, LogEntryType::StatusApplied);
        assert!(entry.message.contains("is affected by Poison"));
    }

    #[test]
    fn test_builder_status_removed() {
        let log = BattleLog::new(10);
        let (mut world, entities) = create_test_world_with_entities(1);
        let target = entities[0];
        
        let entry = log.entry()
            .status(target, "Hero", "Poison", false)
            .build();
        
        assert_eq!(entry.entry_type, LogEntryType::StatusRemoved);
        assert!(entry.message.contains("is no longer Poison"));
    }

    #[test]
    fn test_builder_miss() {
        let log = BattleLog::new(10);
        let (mut world, entities) = create_test_world_with_entities(2);
        let attacker = entities[0];
        let defender = entities[1];
        
        let entry = log.entry()
            .miss(attacker, defender, "Hero", "Goblin")
            .build();
        
        assert_eq!(entry.entry_type, LogEntryType::Miss);
        assert!(entry.message.contains("misses"));
    }

    #[test]
    fn test_builder_defeat() {
        let log = BattleLog::new(10);
        let (mut world, entities) = create_test_world_with_entities(1);
        let entity = entities[0];
        
        let entry = log.entry()
            .defeat(entity, "Goblin")
            .build();
        
        assert_eq!(entry.entry_type, LogEntryType::Defeat);
        assert!(entry.message.contains("defeated"));
    }

    #[test]
    fn test_builder_victory() {
        let log = BattleLog::new(10);
        
        let entry = log.entry()
            .victory()
            .build();
        
        assert_eq!(entry.entry_type, LogEntryType::Victory);
        assert_eq!(entry.message, "Victory!");
    }

    #[test]
    fn test_builder_flee() {
        let log = BattleLog::new(10);
        
        let success_entry = log.entry()
            .flee("Hero", true)
            .build();
        
        assert_eq!(success_entry.entry_type, LogEntryType::Flee);
        assert!(success_entry.message.contains("escaped"));
        
        let fail_entry = log.entry()
            .flee("Hero", false)
            .build();
        
        assert!(fail_entry.message.contains("failed"));
    }

    #[test]
    fn test_builder_turn_start() {
        let log = BattleLog::new(10);
        let (mut world, entities) = create_test_world_with_entities(1);
        let entity = entities[0];
        
        let entry = log.entry()
            .turn_start(entity, "Hero")
            .build();
        
        assert_eq!(entry.entry_type, LogEntryType::TurnStart);
        assert!(entry.message.contains("Hero's turn"));
    }

    #[test]
    fn test_filter_by_type() {
        let mut log = BattleLog::new(10);
        let (mut world, entities) = create_test_world_with_entities(3);
        let entity1 = entities[0];
        let entity2 = entities[1];
        
        log.add(log.entry().damage(
            entity1, entity2,
            "A", "B", 10, Element::None, false
        ).build());
        log.add(log.entry().heal(None, entity1, None, "A", 20).build());
        log.add(log.entry().damage(
            entity1, entity2,
            "A", "B", 15, Element::None, false
        ).build());
        
        let damage_entries = log.filter_by_type(LogEntryType::Damage);
        assert_eq!(damage_entries.len(), 2);
        
        let heal_entries = log.filter_by_type(LogEntryType::Heal);
        assert_eq!(heal_entries.len(), 1);
    }

    #[test]
    fn test_filter_by_entity() {
        let mut log = BattleLog::new(10);
        let (mut world, entities) = create_test_world_with_entities(3);
        let entity1 = entities[0];
        let entity2 = entities[1];
        let entity3 = entities[2];
        
        log.add(log.entry().damage(
            entity1, entity2, "A", "B", 10, Element::None, false
        ).build());
        log.add(log.entry().heal(None, entity1, None, "A", 20).build());
        log.add(log.entry().damage(
            entity2, entity3, "B", "C", 15, Element::None, false
        ).build());
        
        let entity1_entries = log.filter_by_entity(entity1);
        assert_eq!(entity1_entries.len(), 2); // As actor in damage, as target in heal
        
        let entity3_entries = log.filter_by_entity(entity3);
        assert_eq!(entity3_entries.len(), 1);
    }

    #[test]
    fn test_filter_by_severity() {
        let mut log = BattleLog::new(10);
        let (mut world, entities) = create_test_world_with_entities(2);
        let entity1 = entities[0];
        let entity2 = entities[1];
        
        log.add(log.entry().victory().build()); // Critical
        log.add(log.entry().damage(
            entity1, entity2, "A", "B", 10, Element::None, false
        ).build()); // Danger
        log.add(log.entry().heal(None, entity1, None, "A", 20).build()); // Success
        log.add(log.entry().system("Info").build()); // Info
        
        assert_eq!(log.filter_by_severity(LogSeverity::Critical).len(), 1);
        assert_eq!(log.filter_by_severity(LogSeverity::Danger).len(), 1);
        assert_eq!(log.filter_by_severity(LogSeverity::Success).len(), 1);
        assert_eq!(log.filter_by_severity(LogSeverity::Info).len(), 1);
    }

    #[test]
    fn test_recent_entries() {
        let mut log = BattleLog::new(10);
        
        for i in 0..5 {
            log.add(log.entry().system(format!("Entry {}", i)).build());
        }
        
        let recent = log.recent(3);
        assert_eq!(recent.len(), 3);
        // Most recent first
        assert!(recent[0].message.contains("Entry 4"));
        assert!(recent[1].message.contains("Entry 3"));
        assert!(recent[2].message.contains("Entry 2"));
    }

    #[test]
    fn test_turn_tick_management() {
        let mut log = BattleLog::new(10);
        
        assert_eq!(log.current_turn(), 0);
        assert_eq!(log.current_tick(), 0);
        
        log.tick();
        assert_eq!(log.current_tick(), 1);
        
        log.next_turn();
        assert_eq!(log.current_turn(), 1);
        
        log.set_turn(5);
        log.set_tick(100);
        assert_eq!(log.current_turn(), 5);
        assert_eq!(log.current_tick(), 100);
    }

    #[test]
    fn test_current_turn_entries() {
        let mut log = BattleLog::new(10);
        
        log.add(log.entry().system("Turn 0").build());
        log.next_turn();
        log.add(log.entry().system("Turn 1a").build());
        log.add(log.entry().system("Turn 1b").build());
        
        let current = log.current_turn_entries();
        assert_eq!(current.len(), 2);
    }

    #[test]
    fn test_to_formatted_string() {
        let mut log = BattleLog::new(10);
        log.add(log.entry().system("Test 1").build());
        log.add(log.entry().system("Test 2").build());
        
        let output = log.to_formatted_string();
        assert!(output.contains("[Turn 0]"));
        assert!(output.contains("Test 1"));
        assert!(output.contains("Test 2"));
    }

    #[test]
    fn test_to_detailed_string() {
        let mut log = BattleLog::new(10);
        log.add(log.entry().system("Test").build());
        
        let output = log.to_detailed_string();
        assert!(output.contains("Tick"));
        assert!(output.contains("Turn"));
        assert!(output.contains("System"));
    }

    #[test]
    fn test_log_styles() {
        let damage_style = LogEntryType::Damage.style();
        assert_eq!(damage_style.color, (255, 100, 100));
        assert!(!damage_style.bold);
        
        let crit_style = LogEntryType::Crit.style();
        assert!(crit_style.bold);
        
        let heal_style = LogEntryType::Heal.style();
        assert_eq!(heal_style.color, (100, 255, 100));
    }

    #[test]
    fn test_log_severities() {
        assert_eq!(LogEntryType::Victory.severity(), LogSeverity::Critical);
        assert_eq!(LogEntryType::Crit.severity(), LogSeverity::Critical);
        assert_eq!(LogEntryType::Damage.severity(), LogSeverity::Danger);
        assert_eq!(LogEntryType::Heal.severity(), LogSeverity::Success);
        assert_eq!(LogEntryType::Miss.severity(), LogSeverity::Warning);
        assert_eq!(LogEntryType::System.severity(), LogSeverity::Info);
    }

    #[test]
    fn test_element_names() {
        use super::ElementExt;
        
        assert_eq!(Element::Fire.name(), "Fire");
        assert_eq!(Element::Ice.name(), "Ice");
        assert_eq!(Element::Lightning.name(), "Lightning");
        assert_eq!(Element::Holy.name(), "Holy");
        assert_eq!(Element::None.name(), "Physical");
    }

    #[test]
    fn test_builder_status_tick_damage() {
        let log = BattleLog::new(10);
        let (mut world, entities) = create_test_world_with_entities(1);
        let target = entities[0];
        
        let entry = log.entry()
            .status_tick(target, "Hero", "Poison", Some(10), None)
            .build();
        
        assert_eq!(entry.entry_type, LogEntryType::StatusTick);
        assert!(entry.message.contains("takes"));
        assert!(entry.message.contains("damage"));
    }

    #[test]
    fn test_builder_status_tick_heal() {
        let log = BattleLog::new(10);
        let (mut world, entities) = create_test_world_with_entities(1);
        let target = entities[0];
        
        let entry = log.entry()
            .status_tick(target, "Hero", "Regen", None, Some(10))
            .build();
        
        assert!(entry.message.contains("recovers"));
        assert!(entry.message.contains("HP"));
    }

    #[test]
    fn test_builder_escape() {
        let log = BattleLog::new(10);
        
        let success = log.entry().escape(true).build();
        assert_eq!(success.entry_type, LogEntryType::Escape);
        assert!(success.message.contains("escaped"));
        
        let fail = log.entry().escape(false).build();
        assert!(fail.message.contains("failed"));
    }

    #[test]
    fn test_add_message_and_error() {
        let mut log = BattleLog::new(10);
        
        log.add_message("Test message");
        assert_eq!(log.last().unwrap().entry_type, LogEntryType::System);
        
        log.add_error("Test error");
        assert_eq!(log.last().unwrap().entry_type, LogEntryType::Error);
    }

    #[test]
    fn test_set_max_entries() {
        let mut log = BattleLog::new(10);
        
        for i in 0..10 {
            log.add(log.entry().system(format!("Entry {}", i)).build());
        }
        
        assert_eq!(log.len(), 10);
        
        log.set_max_entries(5);
        assert_eq!(log.len(), 5);
        assert_eq!(log.max_entries(), 5);
    }

    #[test]
    fn test_default_battle_log() {
        let log = BattleLog::default();
        assert_eq!(log.max_entries(), 100);
        assert!(log.is_empty());
    }

    #[test]
    fn test_entries_in_turn_range() {
        let mut log = BattleLog::new(10);
        
        log.set_turn(1);
        log.add(log.entry().system("Turn 1").build());
        log.next_turn();
        log.add(log.entry().system("Turn 2").build());
        log.next_turn();
        log.add(log.entry().system("Turn 3").build());
        log.next_turn();
        log.add(log.entry().system("Turn 4").build());
        
        let range = log.entries_in_turn_range(2, 3);
        assert_eq!(range.len(), 2);
    }

    #[test]
    fn test_first_and_last() {
        let mut log = BattleLog::new(10);
        
        assert!(log.first().is_none());
        assert!(log.last().is_none());
        
        log.add(log.entry().system("First").build());
        log.add(log.entry().system("Last").build());
        
        assert_eq!(log.first().unwrap().message, "First");
        assert_eq!(log.last().unwrap().message, "Last");
    }

    #[test]
    fn test_builder_action() {
        let log = BattleLog::new(10);
        let (mut world, entities) = create_test_world_with_entities(1);
        let actor = entities[0];
        
        let entry = log.entry()
            .action(actor, "Hero", "defends")
            .build();
        
        assert_eq!(entry.entry_type, LogEntryType::Action);
        assert!(entry.message.contains("defends"));
    }

    #[test]
    fn test_builder_battle_start() {
        let log = BattleLog::new(10);
        
        let entry = log.entry()
            .battle_start()
            .build();
        
        assert_eq!(entry.entry_type, LogEntryType::BattleStart);
        assert!(entry.message.contains("Battle started"));
    }
}
