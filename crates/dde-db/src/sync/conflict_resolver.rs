//! Conflict Resolver
//!
//! Detects and resolves conflicts during bidirectional synchronization.

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Conflict resolution strategies
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictStrategy {
    /// Always use database version
    DatabaseWins,
    /// Always use ECS version
    EcsWins,
    /// Use the most recently modified version
    LastWriteWins,
    /// Merge changes if possible
    Merge,
    /// Manual resolution required
    Manual,
}

impl ConflictStrategy {
    /// Get human-readable name
    pub fn name(&self) -> &'static str {
        match self {
            ConflictStrategy::DatabaseWins => "Database Wins",
            ConflictStrategy::EcsWins => "ECS Wins",
            ConflictStrategy::LastWriteWins => "Last Write Wins",
            ConflictStrategy::Merge => "Merge",
            ConflictStrategy::Manual => "Manual",
        }
    }

    /// Get description of the strategy
    pub fn description(&self) -> &'static str {
        match self {
            ConflictStrategy::DatabaseWins => {
                "Always use the database version when conflicts occur"
            }
            ConflictStrategy::EcsWins => "Always use the ECS version when conflicts occur",
            ConflictStrategy::LastWriteWins => "Use whichever version was modified most recently",
            ConflictStrategy::Merge => "Attempt to merge changes from both versions",
            ConflictStrategy::Manual => "Flag conflicts for manual resolution",
        }
    }
}

/// Represents a conflict between database and ECS versions
#[derive(Debug, Clone)]
pub struct Conflict {
    /// Entity ID (database ID)
    pub entity_id: u64,
    /// Component type that conflicted
    pub component_type: String,
    /// Timestamp when conflict was detected
    pub detected_at: Instant,
    /// Database version timestamp
    pub db_timestamp: Option<Instant>,
    /// ECS version timestamp
    pub ecs_timestamp: Option<Instant>,
    /// Database version data (JSON)
    pub db_data: Option<String>,
    /// ECS version data (JSON)
    pub ecs_data: Option<String>,
}

impl Conflict {
    /// Create a new conflict record
    pub fn new(
        entity_id: u64,
        component_type: impl Into<String>,
        db_data: Option<String>,
        ecs_data: Option<String>,
    ) -> Self {
        Self {
            entity_id,
            component_type: component_type.into(),
            detected_at: Instant::now(),
            db_timestamp: None,
            ecs_timestamp: None,
            db_data,
            ecs_data,
        }
    }

    /// Check if this conflict has been resolved
    pub fn is_resolved(&self) -> bool {
        // A conflict is considered resolved if one version is None
        // (meaning one side was deleted) or both are None
        match (&self.db_data, &self.ecs_data) {
            (None, None) => true,
            (Some(_), None) => true,
            (None, Some(_)) => true,
            (Some(_), Some(_)) => false,
        }
    }

    /// Get a summary of the conflict
    pub fn summary(&self) -> String {
        match (&self.db_data, &self.ecs_data) {
            (Some(_), Some(_)) => format!(
                "Entity {} component {} modified in both DB and ECS",
                self.entity_id, self.component_type
            ),
            (Some(_), None) => format!(
                "Entity {} component {} exists in DB but deleted in ECS",
                self.entity_id, self.component_type
            ),
            (None, Some(_)) => format!(
                "Entity {} component {} deleted in DB but exists in ECS",
                self.entity_id, self.component_type
            ),
            (None, None) => format!(
                "Entity {} component {} deleted in both (resolved)",
                self.entity_id, self.component_type
            ),
        }
    }
}

/// Resolution outcome for a conflict
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConflictResolution {
    /// Use database version
    UseDatabase,
    /// Use ECS version
    UseEcs,
    /// Use merged data
    UseMerged(String),
    /// Skip this component
    Skip,
    /// Requires manual resolution
    NeedsManual,
}

/// Resolves conflicts during synchronization
pub struct ConflictResolver {
    /// Resolution strategy
    strategy: ConflictStrategy,
    /// Pending conflicts requiring resolution
    pending_conflicts: Vec<Conflict>,
    /// Resolved conflicts history
    resolved_count: u64,
    /// Conflict timeout (how long to wait for manual resolution)
    timeout: Duration,
}

impl ConflictResolver {
    /// Create a new conflict resolver with default strategy
    pub fn new(strategy: ConflictStrategy) -> Self {
        Self {
            strategy,
            pending_conflicts: Vec::new(),
            resolved_count: 0,
            timeout: Duration::from_secs(300), // 5 minutes default
        }
    }

    /// Get the current strategy
    pub fn strategy(&self) -> ConflictStrategy {
        self.strategy
    }

    /// Set the resolution strategy
    pub fn set_strategy(&mut self, strategy: ConflictStrategy) {
        self.strategy = strategy;
    }

    /// Set conflict timeout
    pub fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = timeout;
    }

    /// Detect conflict between database and ECS versions
    pub fn detect_conflict(
        &self,
        entity_id: u64,
        component_type: &str,
        db_data: Option<&str>,
        ecs_data: Option<&str>,
    ) -> Option<Conflict> {
        // No conflict if both are identical
        if db_data == ecs_data {
            return None;
        }

        // No conflict if both are None (both deleted)
        if db_data.is_none() && ecs_data.is_none() {
            return None;
        }

        Some(Conflict::new(
            entity_id,
            component_type,
            db_data.map(String::from),
            ecs_data.map(String::from),
        ))
    }

    /// Resolve a conflict using the current strategy
    pub fn resolve(&mut self, conflict: &Conflict) -> ConflictResolution {
        let resolution = match self.strategy {
            ConflictStrategy::DatabaseWins => ConflictResolution::UseDatabase,
            ConflictStrategy::EcsWins => ConflictResolution::UseEcs,
            ConflictStrategy::LastWriteWins => {
                // Compare timestamps if available
                match (conflict.db_timestamp, conflict.ecs_timestamp) {
                    (Some(db_time), Some(ecs_time)) => {
                        if db_time > ecs_time {
                            ConflictResolution::UseDatabase
                        } else {
                            ConflictResolution::UseEcs
                        }
                    }
                    (Some(_), None) => ConflictResolution::UseDatabase,
                    (None, Some(_)) => ConflictResolution::UseEcs,
                    (None, None) => ConflictResolution::UseEcs, // Default to ECS if no timestamps
                }
            }
            ConflictStrategy::Merge => {
                // Attempt to merge
                if let Some(merged) = self.try_merge(conflict) {
                    ConflictResolution::UseMerged(merged)
                } else {
                    // Fall back to last write wins if merge fails
                    ConflictResolution::UseEcs
                }
            }
            ConflictStrategy::Manual => {
                // Add to pending and return needs manual
                self.pending_conflicts.push(conflict.clone());
                ConflictResolution::NeedsManual
            }
        };

        if resolution != ConflictResolution::NeedsManual {
            self.resolved_count += 1;
        }

        resolution
    }

    /// Try to merge conflicting data
    fn try_merge(&self, conflict: &Conflict) -> Option<String> {
        // This is a simplified merge - in practice, you'd want component-specific merging
        match (&conflict.db_data, &conflict.ecs_data) {
            (Some(db), Some(ecs)) => {
                // Try JSON merge
                if let Ok(db_json) = serde_json::from_str::<serde_json::Value>(db) {
                    if let Ok(ecs_json) = serde_json::from_str::<serde_json::Value>(ecs) {
                        if let Some(merged) = merge_json_values(db_json, ecs_json) {
                            return serde_json::to_string(&merged).ok();
                        }
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// Get pending conflicts
    pub fn pending_conflicts(&self) -> &[Conflict] {
        &self.pending_conflicts
    }

    /// Get count of pending conflicts
    pub fn pending_count(&self) -> usize {
        self.pending_conflicts.len()
    }

    /// Get count of resolved conflicts
    pub fn resolved_count(&self) -> u64 {
        self.resolved_count
    }

    /// Resolve a specific pending conflict manually
    pub fn resolve_manual(
        &mut self,
        entity_id: u64,
        component_type: &str,
        resolution: ConflictResolution,
    ) -> bool {
        let idx = self
            .pending_conflicts
            .iter()
            .position(|c| c.entity_id == entity_id && c.component_type == component_type);

        if let Some(idx) = idx {
            self.pending_conflicts.remove(idx);
            if resolution != ConflictResolution::NeedsManual {
                self.resolved_count += 1;
            }
            true
        } else {
            false
        }
    }

    /// Clear all pending conflicts
    pub fn clear_pending(&mut self) {
        self.pending_conflicts.clear();
    }

    /// Get conflicts that have timed out
    pub fn timed_out_conflicts(&self) -> Vec<&Conflict> {
        let now = Instant::now();
        self.pending_conflicts
            .iter()
            .filter(|c| now.duration_since(c.detected_at) > self.timeout)
            .collect()
    }

    /// Auto-resolve timed out conflicts using current strategy
    pub fn resolve_timed_out(&mut self) -> Vec<(Conflict, ConflictResolution)> {
        let timed_out: Vec<Conflict> = self
            .pending_conflicts
            .iter()
            .filter(|c| Instant::now().duration_since(c.detected_at) > self.timeout)
            .cloned()
            .collect();

        let mut results = Vec::new();
        for conflict in timed_out {
            // Temporarily change strategy from Manual to resolve
            let original_strategy = self.strategy;
            if self.strategy == ConflictStrategy::Manual {
                self.strategy = ConflictStrategy::LastWriteWins;
            }

            let resolution = self.resolve(&conflict);
            results.push((conflict, resolution));

            self.strategy = original_strategy;
        }

        results
    }
}

impl Default for ConflictResolver {
    fn default() -> Self {
        Self::new(ConflictStrategy::LastWriteWins)
    }
}

/// Merge two JSON values, preferring non-null values from both
fn merge_json_values(db: serde_json::Value, ecs: serde_json::Value) -> Option<serde_json::Value> {
    match (db, ecs) {
        (serde_json::Value::Object(mut db_map), serde_json::Value::Object(ecs_map)) => {
            for (key, ecs_value) in ecs_map {
                match db_map.get(&key) {
                    Some(db_value) if db_value.is_object() && ecs_value.is_object() => {
                        // Recursively merge nested objects
                        if let Some(merged) = merge_json_values(db_value.clone(), ecs_value.clone())
                        {
                            db_map.insert(key, merged);
                        }
                    }
                    _ => {
                        // Use ECS value if db value is null, otherwise keep db value
                        if db_map.get(&key).is_none() || db_map[&key].is_null() {
                            db_map.insert(key, ecs_value);
                        }
                    }
                }
            }
            Some(serde_json::Value::Object(db_map))
        }
        (serde_json::Value::Array(mut db_arr), serde_json::Value::Array(ecs_arr)) => {
            // For arrays, append unique ECS items to db items
            for ecs_item in ecs_arr {
                if !db_arr.contains(&ecs_item) {
                    db_arr.push(ecs_item);
                }
            }
            Some(serde_json::Value::Array(db_arr))
        }
        // If types don't match, prefer ECS for primitives
        (_, ecs) => Some(ecs),
    }
}

/// Batch conflict resolution result
#[derive(Debug, Clone, Default)]
pub struct BatchResolutionResult {
    /// Successfully resolved
    pub resolved: Vec<(Conflict, ConflictResolution)>,
    /// Requires manual resolution
    pub needs_manual: Vec<Conflict>,
    /// Resolution failed
    pub failed: Vec<(Conflict, String)>,
}

impl BatchResolutionResult {
    /// Check if all conflicts were resolved
    pub fn is_complete(&self) -> bool {
        self.needs_manual.is_empty() && self.failed.is_empty()
    }

    /// Get total number of conflicts processed
    pub fn total(&self) -> usize {
        self.resolved.len() + self.needs_manual.len() + self.failed.len()
    }
}

/// Conflict resolution helper for components
pub struct ComponentConflictResolver {
    /// Component-specific strategies
    component_strategies: HashMap<String, ConflictStrategy>,
    /// Default strategy
    default_strategy: ConflictStrategy,
}

impl ComponentConflictResolver {
    /// Create a new resolver with default strategy
    pub fn new(default_strategy: ConflictStrategy) -> Self {
        Self {
            component_strategies: HashMap::new(),
            default_strategy,
        }
    }

    /// Set strategy for a specific component type
    pub fn set_component_strategy(
        &mut self,
        component_type: impl Into<String>,
        strategy: ConflictStrategy,
    ) {
        self.component_strategies
            .insert(component_type.into(), strategy);
    }

    /// Get strategy for component type
    pub fn get_strategy(&self, component_type: &str) -> ConflictStrategy {
        self.component_strategies
            .get(component_type)
            .copied()
            .unwrap_or(self.default_strategy)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conflict_detection() {
        let resolver = ConflictResolver::new(ConflictStrategy::LastWriteWins);

        // Same data = no conflict
        assert!(resolver
            .detect_conflict(1, "Position", Some("{\"x\":1}"), Some("{\"x\":1}"))
            .is_none());

        // Different data = conflict
        assert!(resolver
            .detect_conflict(1, "Position", Some("{\"x\":1}"), Some("{\"x\":2}"))
            .is_some());

        // One side missing = conflict
        assert!(resolver
            .detect_conflict(1, "Position", Some("{\"x\":1}"), None)
            .is_some());
        assert!(resolver
            .detect_conflict(1, "Position", None, Some("{\"x\":1}"))
            .is_some());

        // Both missing = no conflict
        assert!(resolver
            .detect_conflict(1, "Position", None, None)
            .is_none());
    }

    #[test]
    fn test_conflict_resolution_strategies() {
        // Database wins
        let mut resolver = ConflictResolver::new(ConflictStrategy::DatabaseWins);
        let conflict = Conflict::new(
            1,
            "Position",
            Some("db".to_string()),
            Some("ecs".to_string()),
        );
        assert_eq!(resolver.resolve(&conflict), ConflictResolution::UseDatabase);

        // ECS wins
        resolver.set_strategy(ConflictStrategy::EcsWins);
        assert_eq!(resolver.resolve(&conflict), ConflictResolution::UseEcs);
    }

    #[test]
    fn test_manual_resolution() {
        let mut resolver = ConflictResolver::new(ConflictStrategy::Manual);
        let conflict = Conflict::new(
            1,
            "Position",
            Some("db".to_string()),
            Some("ecs".to_string()),
        );

        assert_eq!(resolver.resolve(&conflict), ConflictResolution::NeedsManual);
        assert_eq!(resolver.pending_count(), 1);

        // Resolve manually
        let resolved = resolver.resolve_manual(1, "Position", ConflictResolution::UseDatabase);
        assert!(resolved);
        assert_eq!(resolver.pending_count(), 0);
        assert_eq!(resolver.resolved_count(), 1);
    }

    #[test]
    fn test_json_merge() {
        let db = r#"{"x": 1, "y": 2, "nested": {"a": 1}}"#;
        let ecs = r#"{"y": 3, "z": 4, "nested": {"b": 2}}"#;

        let conflict = Conflict::new(1, "Position", Some(db.to_string()), Some(ecs.to_string()));
        let resolver = ConflictResolver::new(ConflictStrategy::Merge);

        if let Some(merged) = resolver.try_merge(&conflict) {
            let parsed: serde_json::Value = serde_json::from_str(&merged).unwrap();
            assert_eq!(parsed["x"], 1); // From DB
            assert_eq!(parsed["y"], 2); // From DB (not overwritten)
            assert_eq!(parsed["z"], 4); // From ECS
            assert_eq!(parsed["nested"]["a"], 1); // From DB
            assert_eq!(parsed["nested"]["b"], 2); // From ECS
        } else {
            panic!("Merge should succeed");
        }
    }

    #[test]
    fn test_conflict_summary() {
        let conflict = Conflict::new(
            1,
            "Position",
            Some("db".to_string()),
            Some("ecs".to_string()),
        );
        assert!(conflict.summary().contains("modified in both"));

        let conflict2 = Conflict::new(1, "Position", Some("db".to_string()), None);
        assert!(conflict2
            .summary()
            .contains("exists in DB but deleted in ECS"));

        let conflict3 = Conflict::new(1, "Position", None, Some("ecs".to_string()));
        assert!(conflict3
            .summary()
            .contains("deleted in DB but exists in ECS"));
    }

    #[test]
    fn test_component_specific_strategies() {
        let mut resolver = ComponentConflictResolver::new(ConflictStrategy::LastWriteWins);
        resolver.set_component_strategy("Position", ConflictStrategy::EcsWins);
        resolver.set_component_strategy("Stats", ConflictStrategy::DatabaseWins);

        assert_eq!(resolver.get_strategy("Position"), ConflictStrategy::EcsWins);
        assert_eq!(
            resolver.get_strategy("Stats"),
            ConflictStrategy::DatabaseWins
        );
        assert_eq!(
            resolver.get_strategy("Other"),
            ConflictStrategy::LastWriteWins
        );
    }

    #[test]
    fn test_batch_resolution_result() {
        let mut result = BatchResolutionResult::default();
        assert!(result.is_complete());

        result.resolved.push((
            Conflict::new(1, "Position", None, None),
            ConflictResolution::UseEcs,
        ));
        assert!(result.is_complete());

        result
            .needs_manual
            .push(Conflict::new(2, "Stats", None, None));
        assert!(!result.is_complete());

        assert_eq!(result.total(), 2);
    }
}
