//! SQLite-ECS Sync Layer
//!
//! Bidirectional synchronization between SQLite database and ECS world.
//! Tracks changes, resolves conflicts, and provides batch operations.

use std::collections::HashSet;
use std::time::{Duration, Instant};

pub mod change_tracker;
pub mod conflict_resolver;
pub mod entity_mapper;

pub use change_tracker::{ChangeKind, ChangeTracker, ComponentChange, EntityChange};
pub use conflict_resolver::{ConflictResolution, ConflictResolver, ConflictStrategy};
pub use entity_mapper::EntityMapper;

/// Sync layer error types
#[derive(thiserror::Error, Debug)]
pub enum SyncError {
    #[error("Database error: {0}")]
    Database(#[from] crate::DbError),

    #[error("Entity mapping not found: {0}")]
    EntityNotFound(u64),

    #[error("Component serialization failed: {0}")]
    Serialization(String),

    #[error("Conflict detected: {0}")]
    Conflict(String),

    #[error("Sync operation timed out")]
    Timeout,

    #[error("Batch operation failed: {0}")]
    BatchFailed(String),
}

pub type Result<T> = std::result::Result<T, SyncError>;

/// Direction of sync operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncDirection {
    /// ECS -> Database (save to persistent storage)
    EcsToDb,
    /// Database -> ECS (load from persistent storage)
    DbToEcs,
    /// Bidirectional (merge both directions)
    Bidirectional,
}

/// Sync operation configuration
#[derive(Debug, Clone)]
pub struct SyncConfig {
    /// Conflict resolution strategy
    pub conflict_strategy: ConflictStrategy,
    /// Auto-sync interval (None = manual only)
    pub auto_sync_interval: Option<Duration>,
    /// Batch size for bulk operations
    pub batch_size: usize,
    /// Enable change tracking
    pub track_changes: bool,
    /// Sync only modified entities (delta sync)
    pub delta_sync: bool,
    /// Maximum sync operations per second
    pub rate_limit: u32,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            conflict_strategy: ConflictStrategy::LastWriteWins,
            auto_sync_interval: Some(Duration::from_secs(30)),
            batch_size: 100,
            track_changes: true,
            delta_sync: true,
            rate_limit: 10,
        }
    }
}

/// Statistics about sync operations
#[derive(Debug, Clone, Default)]
pub struct SyncStats {
    /// Total number of entities synced
    pub entities_synced: usize,
    /// Total number of components synced
    pub components_synced: usize,
    /// Number of conflicts detected
    pub conflicts_detected: usize,
    /// Number of conflicts resolved
    pub conflicts_resolved: usize,
    /// Number of errors
    pub errors: usize,
    /// Last sync duration
    pub last_sync_duration: Duration,
    /// Last sync timestamp
    pub last_sync_time: Option<Instant>,
}

impl SyncStats {
    /// Reset statistics
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Check if last sync was successful
    pub fn is_healthy(&self) -> bool {
        self.errors == 0 || self.errors < self.entities_synced.max(1) / 10
    }
}

/// SQLite-ECS sync layer
pub struct EcsSyncLayer {
    /// Entity ID mapper
    entity_mapper: EntityMapper,
    /// Change tracker
    change_tracker: ChangeTracker,
    /// Configuration
    config: SyncConfig,
    /// Statistics
    stats: SyncStats,
    /// Last auto-sync time
    last_auto_sync: Instant,
    /// Sync operation in progress flag
    sync_in_progress: bool,
    /// Modified entity IDs (for delta sync)
    modified_entities: HashSet<u64>,
}

impl EcsSyncLayer {
    /// Create a new sync layer with default configuration
    pub fn new() -> Self {
        Self::with_config(SyncConfig::default())
    }

    /// Create a new sync layer with custom configuration
    pub fn with_config(config: SyncConfig) -> Self {
        Self {
            entity_mapper: EntityMapper::new(),
            change_tracker: ChangeTracker::new(),
            config,
            stats: SyncStats::default(),
            last_auto_sync: Instant::now(),
            sync_in_progress: false,
            modified_entities: HashSet::new(),
        }
    }

    /// Get configuration reference
    pub fn config(&self) -> &SyncConfig {
        &self.config
    }

    /// Get mutable configuration reference
    pub fn config_mut(&mut self) -> &mut SyncConfig {
        &mut self.config
    }

    /// Get statistics
    pub fn stats(&self) -> &SyncStats {
        &self.stats
    }

    /// Get entity mapper
    pub fn entity_mapper(&self) -> &EntityMapper {
        &self.entity_mapper
    }

    /// Get change tracker
    pub fn change_tracker(&self) -> &ChangeTracker {
        &self.change_tracker
    }

    /// Check if auto-sync is due
    pub fn is_auto_sync_due(&self) -> bool {
        match self.config.auto_sync_interval {
            Some(interval) => self.last_auto_sync.elapsed() >= interval,
            None => false,
        }
    }

    /// Mark entity as modified (for delta sync)
    pub fn mark_modified(&mut self, ecs_entity_id: u64) {
        self.modified_entities.insert(ecs_entity_id);
        if let Some(db_id) = self.entity_mapper.to_db(ecs_entity_id) {
            self.change_tracker.track_entity_change(db_id, ChangeKind::Modified);
        }
    }

    /// Track component change
    pub fn track_component_change(
        &mut self,
        ecs_entity_id: u64,
        component_type: &str,
        change: ChangeKind,
    ) {
        if let Some(db_id) = self.entity_mapper.to_db(ecs_entity_id) {
            self.change_tracker.track_component_change(db_id, component_type, change);
        }
        self.modified_entities.insert(ecs_entity_id);
    }

    /// Register entity mapping
    pub fn register_entity(&mut self, ecs_id: u64, db_id: u64) {
        self.entity_mapper.register(ecs_id, db_id);
    }

    /// Remove entity mapping
    pub fn unregister_entity(&mut self, ecs_id: u64) {
        self.entity_mapper.unregister_ecs(ecs_id);
    }

    /// Get pending changes count
    pub fn pending_changes_count(&self) -> usize {
        self.change_tracker.pending_count()
    }

    /// Check if sync is in progress
    pub fn is_syncing(&self) -> bool {
        self.sync_in_progress
    }

    /// Reset the sync layer state
    pub fn reset(&mut self) {
        self.entity_mapper.clear();
        self.change_tracker.clear();
        self.stats.reset();
        self.modified_entities.clear();
        self.sync_in_progress = false;
    }

    /// Start a sync operation
    pub fn begin_sync(&mut self) -> Result<()> {
        if self.sync_in_progress {
            return Err(SyncError::Conflict("Sync already in progress".to_string()));
        }
        self.sync_in_progress = true;
        Ok(())
    }

    /// End a sync operation
    pub fn end_sync(&mut self, duration: Duration) {
        self.sync_in_progress = false;
        self.stats.last_sync_duration = duration;
        self.stats.last_sync_time = Some(Instant::now());
        self.last_auto_sync = Instant::now();
        
        // Clear modified entities on successful sync
        if self.config.delta_sync {
            self.modified_entities.clear();
        }
    }

    /// Get entity IDs to sync (all or just modified)
    pub fn entities_to_sync(&self) -> Vec<u64> {
        if self.config.delta_sync {
            self.modified_entities.iter().copied().collect()
        } else {
            // Return all mapped entities
            self.entity_mapper.all_ecs_ids()
        }
    }
}

impl Default for EcsSyncLayer {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for ECS world adapters
/// 
/// Implement this trait to connect your ECS implementation to the sync layer.
pub trait EcsWorldAdapter {
    /// Get all entity IDs in the world
    fn get_all_entities(&self) -> Vec<u64>;
    
    /// Get component data for an entity as JSON
    fn get_component_json(&self, entity_id: u64, component_type: &str) -> Option<String>;
    
    /// Set component data for an entity from JSON
    fn set_component_json(&mut self, entity_id: u64, component_type: &str, json: &str) -> Result<()>;
    
    /// Check if entity has component
    fn has_component(&self, entity_id: u64, component_type: &str) -> bool;
    
    /// Get all component types for an entity
    fn get_component_types(&self, entity_id: u64) -> Vec<String>;
    
    /// Create a new entity
    fn create_entity(&mut self) -> u64;
    
    /// Destroy an entity
    fn destroy_entity(&mut self, entity_id: u64) -> Result<()>;
    
    /// Get entity position (for spatial indexing)
    fn get_position(&self, entity_id: u64) -> Option<(i32, i32, i32)>;
}

/// Batch sync operation builder
pub struct BatchSyncBuilder {
    entity_ids: Vec<u64>,
    component_types: Vec<String>,
    direction: SyncDirection,
}

impl BatchSyncBuilder {
    /// Create a new batch sync builder
    pub fn new() -> Self {
        Self {
            entity_ids: Vec::new(),
            component_types: Vec::new(),
            direction: SyncDirection::Bidirectional,
        }
    }

    /// Add entity to sync
    pub fn with_entity(mut self, entity_id: u64) -> Self {
        self.entity_ids.push(entity_id);
        self
    }

    /// Add multiple entities to sync
    pub fn with_entities(mut self, entity_ids: Vec<u64>) -> Self {
        self.entity_ids.extend(entity_ids);
        self
    }

    /// Add component type to sync
    pub fn with_component(mut self, component_type: impl Into<String>) -> Self {
        self.component_types.push(component_type.into());
        self
    }

    /// Add multiple component types to sync
    pub fn with_components(mut self, component_types: Vec<impl Into<String>>) -> Self {
        self.component_types
            .extend(component_types.into_iter().map(|c| c.into()));
        self
    }

    /// Set sync direction
    pub fn with_direction(mut self, direction: SyncDirection) -> Self {
        self.direction = direction;
        self
    }
}

impl Default for BatchSyncBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    struct MockEcsWorld {
        entities: HashMap<u64, HashMap<String, String>>,
        next_id: u64,
    }

    impl MockEcsWorld {
        fn new() -> Self {
            Self {
                entities: HashMap::new(),
                next_id: 1,
            }
        }
    }

    impl EcsWorldAdapter for MockEcsWorld {
        fn get_all_entities(&self) -> Vec<u64> {
            self.entities.keys().copied().collect()
        }

        fn get_component_json(&self, entity_id: u64, component_type: &str) -> Option<String> {
            self.entities
                .get(&entity_id)
                .and_then(|comps| comps.get(component_type).cloned())
        }

        fn set_component_json(
            &mut self,
            entity_id: u64,
            component_type: &str,
            json: &str,
        ) -> Result<()> {
            self.entities
                .entry(entity_id)
                .or_default()
                .insert(component_type.to_string(), json.to_string());
            Ok(())
        }

        fn has_component(&self, entity_id: u64, component_type: &str) -> bool {
            self.entities
                .get(&entity_id)
                .map(|comps| comps.contains_key(component_type))
                .unwrap_or(false)
        }

        fn get_component_types(&self, entity_id: u64) -> Vec<String> {
            self.entities
                .get(&entity_id)
                .map(|comps| comps.keys().cloned().collect())
                .unwrap_or_default()
        }

        fn create_entity(&mut self) -> u64 {
            let id = self.next_id;
            self.next_id += 1;
            self.entities.insert(id, HashMap::new());
            id
        }

        fn destroy_entity(&mut self, entity_id: u64) -> Result<()> {
            self.entities.remove(&entity_id);
            Ok(())
        }

        fn get_position(&self, _entity_id: u64) -> Option<(i32, i32, i32)> {
            Some((0, 0, 0))
        }
    }

    #[test]
    fn test_sync_layer_creation() {
        let sync = EcsSyncLayer::new();
        assert!(!sync.is_syncing());
        assert_eq!(sync.pending_changes_count(), 0);
    }

    #[test]
    fn test_sync_layer_config() {
        let mut sync = EcsSyncLayer::new();
        assert!(sync.config().track_changes);
        
        sync.config_mut().track_changes = false;
        assert!(!sync.config().track_changes);
    }

    #[test]
    fn test_entity_mapping() {
        let mut sync = EcsSyncLayer::new();
        
        sync.register_entity(100, 1);
        sync.register_entity(200, 2);
        
        assert_eq!(sync.entity_mapper.to_db(100), Some(1));
        assert_eq!(sync.entity_mapper.to_ecs(1), Some(100));
        
        sync.unregister_entity(100);
        assert_eq!(sync.entity_mapper.to_db(100), None);
    }

    #[test]
    fn test_mark_modified() {
        let mut sync = EcsSyncLayer::new();
        sync.register_entity(100, 1);
        
        sync.mark_modified(100);
        
        let to_sync = sync.entities_to_sync();
        assert!(to_sync.contains(&100));
    }

    #[test]
    fn test_sync_stats() {
        let mut stats = SyncStats::default();
        assert!(stats.is_healthy());
        
        stats.entities_synced = 100;
        stats.errors = 5;
        assert!(stats.is_healthy()); // 5% error rate is acceptable
        
        stats.errors = 50;
        assert!(!stats.is_healthy()); // 50% error rate is not acceptable
        
        stats.reset();
        assert_eq!(stats.entities_synced, 0);
    }

    #[test]
    fn test_batch_sync_builder() {
        let builder = BatchSyncBuilder::new()
            .with_entity(1)
            .with_entity(2)
            .with_component("Position")
            .with_component("Stats")
            .with_direction(SyncDirection::EcsToDb);

        // Just verify it builds without panic
        assert_eq!(builder.entity_ids.len(), 2);
        assert_eq!(builder.component_types.len(), 2);
        assert_eq!(builder.direction, SyncDirection::EcsToDb);
    }
}
