//! Error handling and recovery for the battle system
//!
//! Provides graceful error recovery that doesn't crash the engine on edge cases.
//! Implements automatic recovery strategies for non-fatal errors.

use thiserror::Error;
use dde_core::{Entity, World};
use crate::turn_queue::{BattleAction, TurnQueue};

/// Battle error types with detailed context
#[derive(Error, Debug, Clone, PartialEq)]
pub enum BattleError {
    /// Entity doesn't exist in the world
    #[error("Invalid entity: {0:?}")]
    InvalidEntity(Entity),
    
    /// Entity exists but is not participating in battle
    #[error("Entity not in battle: {0:?}")]
    EntityNotInBattle(Entity),
    
    /// Action is invalid or malformed
    #[error("Invalid action: {0}")]
    InvalidAction(String),
    
    /// Action not available for entity (wrong turn, stunned, etc.)
    #[error("Action not available: {action} for entity {entity:?}")]
    ActionNotAvailable { entity: Entity, action: String },
    
    /// Skill is on cooldown
    #[error("Skill on cooldown: {skill_id} (remaining: {remaining_turns} turns)")]
    SkillOnCooldown { skill_id: u32, remaining_turns: u32 },
    
    /// Not enough MP to use skill
    #[error("Insufficient MP: needed {needed}, have {have}")]
    InsufficientMp { needed: u32, have: u32 },
    
    /// Target is invalid (dead, out of range, wrong faction, etc.)
    #[error("Invalid target: {reason}")]
    InvalidTarget { reason: String },
    
    /// Battle has already ended
    #[error("Battle already ended")]
    BattleEnded,
    
    /// Internal system error
    #[error("System error: {0}")]
    System(String),
}

/// Battle result outcomes including errors
#[derive(Debug, Clone, PartialEq)]
pub enum BattleResult {
    /// Players won the battle
    Victory,
    /// Players lost the battle
    Defeat,
    /// Players successfully fled
    Fled,
    /// Battle ended in a draw
    Draw,
    /// Battle ended with an error
    Error(BattleError),
}

impl std::fmt::Display for BattleResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BattleResult::Victory => write!(f, "Victory"),
            BattleResult::Defeat => write!(f, "Defeat"),
            BattleResult::Fled => write!(f, "Fled"),
            BattleResult::Draw => write!(f, "Draw"),
            BattleResult::Error(e) => write!(f, "Error: {}", e),
        }
    }
}

impl BattleResult {
    /// Check if this result is a victory
    pub fn is_victory(&self) -> bool {
        matches!(self, BattleResult::Victory)
    }
    
    /// Check if this result is a defeat
    pub fn is_defeat(&self) -> bool {
        matches!(self, BattleResult::Defeat)
    }
    
    /// Check if this result is a successful flee
    pub fn is_fled(&self) -> bool {
        matches!(self, BattleResult::Fled)
    }
    
    /// Check if this result is a draw
    pub fn is_draw(&self) -> bool {
        matches!(self, BattleResult::Draw)
    }
    
    /// Check if this result is an error
    pub fn is_error(&self) -> bool {
        matches!(self, BattleResult::Error(_))
    }
    
    /// Get the error if this is an error result
    pub fn error(&self) -> Option<&BattleError> {
        match self {
            BattleResult::Error(e) => Some(e),
            _ => None,
        }
    }
}

/// Type alias for battle operations that can fail
pub type BattleOutcome<T> = std::result::Result<T, BattleError>;

/// Recovery action to take when an error occurs
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RecoveryAction {
    /// Continue without action - error can be safely ignored
    Ignore,
    /// Log the error but continue execution
    Report,
    /// Clamp a value to a valid range (value, min, max)
    ClampValue(i32, i32, i32),
    /// Use a fallback action or skill
    UseFallback,
    /// Skip this entity's turn
    SkipTurn,
    /// End the battle with the error
    Abort,
}

/// Trait for types that can recover from errors automatically
pub trait Recoverable {
    /// Attempt to recover from error automatically
    fn try_recover(&self) -> RecoveryAction;
    
    /// Check if this error is fatal (cannot be recovered from)
    fn is_fatal(&self) -> bool;
}

impl Recoverable for BattleError {
    fn try_recover(&self) -> RecoveryAction {
        match self {
            // Auto-fix: clamp MP to what's available
            BattleError::InsufficientMp { needed, have } => {
                if *have > 0 {
                    // Allow action with available MP (will be less effective)
                    RecoveryAction::ClampValue(*have as i32, 0, *needed as i32)
                } else {
                    // No MP at all, skip turn
                    RecoveryAction::SkipTurn
                }
            }
            
            // Auto-fix: skip cooldown check in debug builds for testing
            BattleError::SkillOnCooldown { remaining_turns, .. } => {
                if cfg!(debug_assertions) && *remaining_turns <= 1 {
                    // In debug, allow skills that are almost off cooldown
                    RecoveryAction::Ignore
                } else {
                    RecoveryAction::SkipTurn
                }
            }
            
            // Auto-fix: use basic attack as fallback
            BattleError::InvalidAction(_) => RecoveryAction::UseFallback,
            
            // Auto-fix: skip turn if action not available
            BattleError::ActionNotAvailable { .. } => RecoveryAction::SkipTurn,
            
            // Auto-fix: report invalid target but continue
            BattleError::InvalidTarget { .. } => RecoveryAction::Report,
            
            // Fatal errors - entity doesn't exist or system failure
            BattleError::InvalidEntity(_) => RecoveryAction::Abort,
            BattleError::System(_) => RecoveryAction::Abort,
            BattleError::BattleEnded => RecoveryAction::Abort,
            
            // Entity not in battle - report but continue
            BattleError::EntityNotInBattle(_) => RecoveryAction::Report,
        }
    }
    
    fn is_fatal(&self) -> bool {
        matches!(self, 
            BattleError::InvalidEntity(_) |
            BattleError::System(_) |
            BattleError::BattleEnded
        )
    }
}

/// Log severity levels for battle log entries
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogSeverity {
    /// Informational message
    Info,
    /// Warning - non-critical issue
    Warning,
    /// Error - action failed but battle continues
    Error,
    /// Critical - battle may need to end
    Critical,
}

/// Battle log entry for errors and events
#[derive(Debug, Clone)]
pub enum LogEntry {
    /// Standard message
    Message(String),
    /// Error entry with severity
    Error {
        message: String,
        severity: LogSeverity,
    },
    /// Action was taken
    Action {
        actor: Entity,
        action: String,
        result: String,
    },
}

/// Battle log for tracking events and errors
#[derive(Debug, Clone, Default)]
pub struct BattleLog {
    entries: Vec<LogEntry>,
    max_entries: usize,
}

impl BattleLog {
    /// Create a new battle log with default capacity
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            max_entries: 100,
        }
    }
    
    /// Create a new battle log with custom capacity
    pub fn with_capacity(max_entries: usize) -> Self {
        Self {
            entries: Vec::new(),
            max_entries,
        }
    }
    
    /// Add an entry to the log
    pub fn add(&mut self, entry: LogEntry) {
        self.entries.push(entry);
        
        // Trim old entries if over capacity
        if self.entries.len() > self.max_entries {
            self.entries.remove(0);
        }
    }
    
    /// Add a simple message
    pub fn add_message(&mut self, message: impl Into<String>) {
        self.add(LogEntry::Message(message.into()));
    }
    
    /// Add an error entry
    pub fn add_error(&mut self, error: &BattleError) {
        self.add(LogEntry::Error {
            message: error.to_string(),
            severity: if error.is_fatal() { 
                LogSeverity::Critical 
            } else { 
                LogSeverity::Warning 
            },
        });
    }
    
    /// Get all log entries
    pub fn entries(&self) -> &[LogEntry] {
        &self.entries
    }
    
    /// Get the most recent entry
    pub fn last(&self) -> Option<&LogEntry> {
        self.entries.last()
    }
    
    /// Clear all entries
    pub fn clear(&mut self) {
        self.entries.clear();
    }
    
    /// Get the number of entries
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    
    /// Check if the log is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    
    /// Get all error entries
    pub fn errors(&self) -> Vec<&LogEntry> {
        self.entries
            .iter()
            .filter(|e| matches!(e, LogEntry::Error { .. }))
            .collect()
    }
    
    /// Get all critical errors
    pub fn critical_errors(&self) -> Vec<&LogEntry> {
        self.entries
            .iter()
            .filter(|e| matches!(e, LogEntry::Error { severity: LogSeverity::Critical, .. }))
            .collect()
    }
}

/// Error recovery context for tracking recovery attempts
#[derive(Debug, Clone)]
pub struct RecoveryContext {
    /// Number of recovery attempts made
    pub attempts: u32,
    /// Maximum allowed recovery attempts before aborting
    pub max_attempts: u32,
    /// Errors that have been recovered from
    pub recovered_errors: Vec<BattleError>,
}

impl RecoveryContext {
    /// Create a new recovery context
    pub fn new() -> Self {
        Self {
            attempts: 0,
            max_attempts: 5,
            recovered_errors: Vec::new(),
        }
    }
    
    /// Create with custom max attempts
    pub fn with_max_attempts(max_attempts: u32) -> Self {
        Self {
            attempts: 0,
            max_attempts,
            recovered_errors: Vec::new(),
        }
    }
    
    /// Record a recovery attempt
    pub fn record_attempt(&mut self, error: BattleError) {
        self.attempts += 1;
        self.recovered_errors.push(error);
    }
    
    /// Check if recovery attempts have been exhausted
    pub fn should_abort(&self) -> bool {
        self.attempts >= self.max_attempts
    }
    
    /// Reset the attempt counter (e.g., after successful action)
    pub fn reset(&mut self) {
        self.attempts = 0;
    }
}

impl Default for RecoveryContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Extension trait for BattleSystem to support error recovery
/// 
/// This trait is implemented in the main BattleSystem to provide
/// error handling and recovery capabilities.
pub trait BattleErrorHandler {
    /// Execute action with automatic error recovery
    fn execute_action_safe(
        &mut self, 
        world: &mut World, 
        action: BattleAction
    ) -> BattleResult;
    
    /// Validate action before execution
    fn validate_action(
        &self, 
        world: &World, 
        action: &BattleAction
    ) -> Vec<BattleError>;
    
    /// Get the battle log
    fn battle_log(&self) -> &BattleLog;
    
    /// Get mutable battle log
    fn battle_log_mut(&mut self) -> &mut BattleLog;
    
    /// Skip the current turn
    fn skip_current_turn(&mut self);
    
    /// End the battle
    fn end_battle(&mut self);
    
    /// Retry action with clamped MP
    fn retry_with_clamped_mp(
        &mut self,
        world: &mut World,
        action: BattleAction,
        available_mp: u32,
    ) -> BattleResult;
    
    /// Get the turn queue
    fn turn_queue(&self) -> &TurnQueue;
    
    /// Get mutable turn queue
    fn turn_queue_mut(&mut self) -> &mut TurnQueue;
}

#[cfg(test)]
pub mod test_helpers {
    use super::*;
    use dde_core::World;
    use crate::turn_queue::{ActionType, BattleAction};
    use crate::skills::{SkillId, SkillResult};
    
    /// Create a test world with a single entity
    pub fn create_test_entity() -> (World, Entity) {
        let mut world = World::new();
        let entity = world.spawn(());
        (world, entity)
    }
    
    /// Create a scenario that triggers specific error
    pub fn trigger_error_scenario(error_type: &str) -> (World, Entity, BattleAction) {
        let (mut world, entity) = create_test_entity();
        
        let action = match error_type {
            "invalid_entity" => {
                // Use a non-existent entity by spawning and then despawning
                let fake_entity = world.spawn(());
                world.despawn(fake_entity).ok();
                BattleAction {
                    actor: fake_entity,
                    action_type: ActionType::Attack,
                    target: None,
                }
            }
            "insufficient_mp" => {
                BattleAction {
                    actor: entity,
                    action_type: ActionType::Skill(2), // Fireball
                    target: None,
                }
            }
            "invalid_action" => {
                BattleAction {
                    actor: entity,
                    action_type: ActionType::Skill(999), // Non-existent skill
                    target: None,
                }
            }
            _ => {
                BattleAction {
                    actor: entity,
                    action_type: ActionType::Attack,
                    target: None,
                }
            }
        };
        
        (world, entity, action)
    }
    
    /// Assert that error recovery worked
    pub fn assert_recovered<T>(result: &BattleOutcome<T>) {
        assert!(result.is_ok(), "Expected recovery but got: {:?}", result.as_ref().err());
    }
    
    /// Assert that result is a specific error
    pub fn assert_error<T: std::fmt::Debug>(result: &BattleOutcome<T>, expected: BattleError) {
        assert!(result.is_err(), "Expected error {:?} but got Ok", expected);
        match result {
            Err(ref e) if *e == expected => {},
            _ => panic!("Expected {:?} but got {:?}", expected, result),
        }
    }
    
    /// Create a mock battle error for testing
    pub fn mock_error(error_type: &str) -> BattleError {
        // Use DANGLING entity for testing - this is a valid Entity value for tests
        let dummy_entity = Entity::DANGLING;
        
        match error_type {
            "invalid_entity" => BattleError::InvalidEntity(dummy_entity),
            "entity_not_in_battle" => BattleError::EntityNotInBattle(dummy_entity),
            "invalid_action" => BattleError::InvalidAction("test action".to_string()),
            "action_not_available" => BattleError::ActionNotAvailable {
                entity: dummy_entity,
                action: "test".to_string(),
            },
            "skill_on_cooldown" => BattleError::SkillOnCooldown {
                skill_id: 1,
                remaining_turns: 2,
            },
            "insufficient_mp" => BattleError::InsufficientMp {
                needed: 10,
                have: 5,
            },
            "invalid_target" => BattleError::InvalidTarget {
                reason: "target is dead".to_string(),
            },
            "battle_ended" => BattleError::BattleEnded,
            "system" => BattleError::System("test error".to_string()),
            _ => BattleError::System("unknown test error".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_helpers::*;
    
    #[test]
    fn test_error_creation_and_display() {
        let mut world = World::new();
        let entity = world.spawn(());
        
        let err1 = BattleError::InvalidEntity(entity);
        assert!(err1.to_string().contains("Invalid entity:"));
        
        let err2 = BattleError::InsufficientMp { needed: 10, have: 5 };
        assert_eq!(err2.to_string(), "Insufficient MP: needed 10, have 5");
        
        let err3 = BattleError::SkillOnCooldown { skill_id: 5, remaining_turns: 3 };
        assert_eq!(err3.to_string(), "Skill on cooldown: 5 (remaining: 3 turns)");
        
        let err4 = BattleError::InvalidTarget { reason: "out of range".to_string() };
        assert_eq!(err4.to_string(), "Invalid target: out of range");
    }
    
    #[test]
    fn test_battle_result_variants() {
        assert!(BattleResult::Victory.is_victory());
        assert!(BattleResult::Defeat.is_defeat());
        assert!(BattleResult::Fled.is_fled());
        assert!(BattleResult::Draw.is_draw());
        
        let err_result = BattleResult::Error(BattleError::BattleEnded);
        assert!(err_result.is_error());
        assert!(err_result.error().is_some());
    }
    
    #[test]
    fn test_fatal_error_detection() {
        assert!(BattleError::InvalidEntity(Entity::DANGLING).is_fatal());
        assert!(BattleError::System("test".to_string()).is_fatal());
        assert!(BattleError::BattleEnded.is_fatal());
        
        assert!(!BattleError::InsufficientMp { needed: 10, have: 5 }.is_fatal());
        assert!(!BattleError::SkillOnCooldown { skill_id: 1, remaining_turns: 2 }.is_fatal());
        assert!(!BattleError::InvalidTarget { reason: "test".to_string() }.is_fatal());
    }
    
    #[test]
    fn test_recovery_strategies() {
        // Fatal errors should abort
        let fatal = BattleError::InvalidEntity(Entity::DANGLING);
        assert_eq!(fatal.try_recover(), RecoveryAction::Abort);
        
        // Insufficient MP with some MP should clamp
        let low_mp = BattleError::InsufficientMp { needed: 10, have: 5 };
        assert_eq!(low_mp.try_recover(), RecoveryAction::ClampValue(5, 0, 10));
        
        // Insufficient MP with zero MP should skip turn
        let zero_mp = BattleError::InsufficientMp { needed: 10, have: 0 };
        assert_eq!(zero_mp.try_recover(), RecoveryAction::SkipTurn);
        
        // Invalid action should use fallback
        let invalid_action = BattleError::InvalidAction("test".to_string());
        assert_eq!(invalid_action.try_recover(), RecoveryAction::UseFallback);
        
        // Invalid target should report
        let invalid_target = BattleError::InvalidTarget { reason: "dead".to_string() };
        assert_eq!(invalid_target.try_recover(), RecoveryAction::Report);
    }
    
    #[test]
    fn test_recovery_action_equality() {
        assert_eq!(RecoveryAction::Ignore, RecoveryAction::Ignore);
        assert_eq!(RecoveryAction::Report, RecoveryAction::Report);
        assert_eq!(RecoveryAction::SkipTurn, RecoveryAction::SkipTurn);
        assert_eq!(RecoveryAction::Abort, RecoveryAction::Abort);
        assert_eq!(RecoveryAction::ClampValue(1, 2, 3), RecoveryAction::ClampValue(1, 2, 3));
        assert_ne!(RecoveryAction::ClampValue(1, 2, 3), RecoveryAction::ClampValue(1, 2, 4));
    }
    
    #[test]
    fn test_battle_log() {
        let mut log = BattleLog::new();
        
        // Add some entries
        log.add_message("Battle started");
        log.add_error(&BattleError::InvalidTarget { reason: "test".to_string() });
        
        assert_eq!(log.len(), 2);
        assert!(!log.is_empty());
        
        // Check last entry is an error
        match log.last() {
            Some(LogEntry::Error { severity, .. }) => {
                assert_eq!(*severity, LogSeverity::Warning);
            }
            _ => panic!("Expected error entry"),
        }
        
        // Check errors filter
        assert_eq!(log.errors().len(), 1);
        
        // Add critical error
        log.add_error(&BattleError::System("fatal".to_string()));
        assert_eq!(log.critical_errors().len(), 1);
        
        // Clear log
        log.clear();
        assert!(log.is_empty());
    }
    
    #[test]
    fn test_battle_log_capacity() {
        let mut log = BattleLog::with_capacity(3);
        
        log.add_message("1");
        log.add_message("2");
        log.add_message("3");
        log.add_message("4"); // Should cause "1" to be removed
        
        assert_eq!(log.len(), 3);
        
        // First entry should be "2"
        match &log.entries()[0] {
            LogEntry::Message(m) => assert_eq!(m, "2"),
            _ => panic!("Expected message entry"),
        }
    }
    
    #[test]
    fn test_recovery_context() {
        let mut ctx = RecoveryContext::new();
        
        assert!(!ctx.should_abort());
        assert_eq!(ctx.attempts, 0);
        
        // Record some attempts
        ctx.record_attempt(BattleError::BattleEnded);
        ctx.record_attempt(BattleError::System("test".to_string()));
        
        assert_eq!(ctx.attempts, 2);
        assert_eq!(ctx.recovered_errors.len(), 2);
        
        // Reset
        ctx.reset();
        assert_eq!(ctx.attempts, 0);
        
        // Test abort threshold
        let mut ctx = RecoveryContext::with_max_attempts(2);
        ctx.record_attempt(BattleError::InvalidEntity(Entity::DANGLING));
        assert!(!ctx.should_abort());
        ctx.record_attempt(BattleError::InvalidEntity(Entity::DANGLING));
        assert!(ctx.should_abort());
    }
    
    #[test]
    fn test_mock_errors() {
        let invalid_entity = mock_error("invalid_entity");
        assert!(matches!(invalid_entity, BattleError::InvalidEntity(_)));
        
        let insufficient_mp = mock_error("insufficient_mp");
        assert!(matches!(insufficient_mp, BattleError::InsufficientMp { .. }));
        
        let cooldown = mock_error("skill_on_cooldown");
        assert!(matches!(cooldown, BattleError::SkillOnCooldown { .. }));
        
        let battle_ended = mock_error("battle_ended");
        assert!(matches!(battle_ended, BattleError::BattleEnded));
    }
    
    #[test]
    fn test_log_severity_ordering() {
        // Just verify they can be compared for equality
        assert_eq!(LogSeverity::Info, LogSeverity::Info);
        assert_eq!(LogSeverity::Warning, LogSeverity::Warning);
        assert_eq!(LogSeverity::Error, LogSeverity::Error);
        assert_eq!(LogSeverity::Critical, LogSeverity::Critical);
        
        assert_ne!(LogSeverity::Info, LogSeverity::Critical);
    }
    
    #[test]
    fn test_battle_error_clone() {
        let err = BattleError::InsufficientMp { needed: 10, have: 5 };
        let cloned = err.clone();
        assert_eq!(err, cloned);
    }
    
    #[test]
    fn test_battle_result_clone() {
        let result = BattleResult::Error(BattleError::BattleEnded);
        let cloned = result.clone();
        assert_eq!(result, cloned);
    }
}
