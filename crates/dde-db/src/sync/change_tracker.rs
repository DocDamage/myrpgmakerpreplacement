//! Change Tracker
//!
//! Tracks changes to entities and components for delta synchronization.

use std::collections::{HashMap, VecDeque};
use std::time::Instant;

/// Type of change
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChangeKind {
    /// Entity/component was created
    Created,
    /// Entity/component was modified
    Modified,
    /// Entity/component was deleted
    Deleted,
}

impl ChangeKind {
    /// Get a human-readable name for the change kind
    pub fn name(&self) -> &'static str {
        match self {
            ChangeKind::Created => "Created",
            ChangeKind::Modified => "Modified",
            ChangeKind::Deleted => "Deleted",
        }
    }
}

/// Component-level change record
#[derive(Debug, Clone)]
pub struct ComponentChange {
    /// Type of component (e.g., "Position", "Stats")
    pub component_type: String,
    /// Kind of change
    pub kind: ChangeKind,
    /// Timestamp of change
    pub timestamp: Instant,
    /// Optional: Hash of component data for comparison
    pub data_hash: Option<u64>,
}

/// Entity-level change record
#[derive(Debug, Clone)]
pub struct EntityChange {
    /// Database entity ID
    pub entity_id: u64,
    /// Kind of change
    pub kind: ChangeKind,
    /// Timestamp of change
    pub timestamp: Instant,
    /// Component-level changes
    pub component_changes: Vec<ComponentChange>,
}

impl EntityChange {
    /// Create a new entity change record
    pub fn new(entity_id: u64, kind: ChangeKind) -> Self {
        Self {
            entity_id,
            kind,
            timestamp: Instant::now(),
            component_changes: Vec::new(),
        }
    }

    /// Add a component change
    pub fn add_component_change(&mut self, component_type: impl Into<String>, kind: ChangeKind) {
        self.component_changes.push(ComponentChange {
            component_type: component_type.into(),
            kind,
            timestamp: Instant::now(),
            data_hash: None,
        });
    }
}

/// Tracks changes for delta synchronization
pub struct ChangeTracker {
    /// Pending entity changes (entity_id -> change)
    pending_changes: HashMap<u64, EntityChange>,
    /// Change history (for debugging/auditing)
    history: VecDeque<EntityChange>,
    /// Maximum history size
    max_history: usize,
    /// Total changes tracked
    total_changes: u64,
}

impl ChangeTracker {
    /// Create a new change tracker
    pub fn new() -> Self {
        Self::with_capacity(1000)
    }

    /// Create a new change tracker with custom history capacity
    pub fn with_capacity(max_history: usize) -> Self {
        Self {
            pending_changes: HashMap::new(),
            history: VecDeque::with_capacity(max_history),
            max_history,
            total_changes: 0,
        }
    }

    /// Track an entity-level change
    pub fn track_entity_change(&mut self, entity_id: u64, kind: ChangeKind) {
        self.total_changes += 1;

        match self.pending_changes.get_mut(&entity_id) {
            Some(existing) => {
                // Merge changes: Created + Modified = Created, Modified + Modified = Modified
                // Deleted overrides everything
                match (existing.kind, kind) {
                    (_, ChangeKind::Deleted) => {
                        existing.kind = ChangeKind::Deleted;
                    }
                    (ChangeKind::Deleted, ChangeKind::Created) => {
                        // Deleted then Created = Modified (recreated)
                        existing.kind = ChangeKind::Modified;
                    }
                    (ChangeKind::Created, ChangeKind::Modified) => {
                        // Created then Modified = Created (with updates)
                        // Keep as Created
                    }
                    (ChangeKind::Modified, ChangeKind::Modified) => {
                        // Already Modified, keep as Modified
                    }
                    (ChangeKind::Created, ChangeKind::Created) => {
                        // Shouldn't happen, but keep as Created
                    }
                    (ChangeKind::Deleted, ChangeKind::Modified) => {
                        // Shouldn't happen, but treat as Modified
                        existing.kind = ChangeKind::Modified;
                    }
                    (ChangeKind::Modified, ChangeKind::Created) => {
                        // Shouldn't happen, but treat as Created
                        existing.kind = ChangeKind::Created;
                    }
                }
                existing.timestamp = Instant::now();
            }
            None => {
                let change = EntityChange::new(entity_id, kind);
                self.pending_changes.insert(entity_id, change);
            }
        }
    }

    /// Track a component-level change
    pub fn track_component_change(
        &mut self,
        entity_id: u64,
        component_type: impl Into<String>,
        kind: ChangeKind,
    ) {
        let component_type = component_type.into();
        self.total_changes += 1;

        match self.pending_changes.get_mut(&entity_id) {
            Some(entity_change) => {
                // Check if we already have a change for this component
                if let Some(existing) = entity_change
                    .component_changes
                    .iter_mut()
                    .find(|c| c.component_type == component_type)
                {
                    // Merge component changes
                    match (existing.kind, kind) {
                        (_, ChangeKind::Deleted) => existing.kind = ChangeKind::Deleted,
                        (ChangeKind::Deleted, ChangeKind::Created) => {
                            existing.kind = ChangeKind::Modified
                        }
                        (ChangeKind::Created, ChangeKind::Modified) => {
                            // Keep as Created
                        }
                        _ => existing.kind = kind,
                    }
                    existing.timestamp = Instant::now();
                } else {
                    entity_change.add_component_change(component_type, kind);
                }

                // If entity was not marked as modified, mark it now
                if entity_change.kind == ChangeKind::Created {
                    // Keep as Created
                } else {
                    entity_change.kind = ChangeKind::Modified;
                }
            }
            None => {
                let mut change = EntityChange::new(entity_id, ChangeKind::Modified);
                change.add_component_change(component_type, kind);
                self.pending_changes.insert(entity_id, change);
            }
        }
    }

    /// Get pending change for an entity
    pub fn get_pending(&self, entity_id: u64) -> Option<&EntityChange> {
        self.pending_changes.get(&entity_id)
    }

    /// Get all pending changes
    pub fn get_all_pending(&self) -> Vec<&EntityChange> {
        self.pending_changes.values().collect()
    }

    /// Get pending changes for specific entities
    pub fn get_pending_for(&self, entity_ids: &[u64]) -> Vec<&EntityChange> {
        entity_ids
            .iter()
            .filter_map(|id| self.pending_changes.get(id))
            .collect()
    }

    /// Get count of pending changes
    pub fn pending_count(&self) -> usize {
        self.pending_changes.len()
    }

    /// Get the timestamp of the oldest pending change
    pub fn oldest_pending_timestamp(&self) -> Option<Instant> {
        self.pending_changes.values().map(|c| c.timestamp).min()
    }

    /// Check if there are stale changes older than the given duration
    pub fn has_stale_changes(&self, max_age: std::time::Duration) -> bool {
        self.oldest_pending_timestamp()
            .map(|ts| ts.elapsed() > max_age)
            .unwrap_or(false)
    }

    /// Check if there are pending changes
    pub fn has_pending(&self) -> bool {
        !self.pending_changes.is_empty()
    }

    /// Clear a specific pending change
    pub fn clear_pending(&mut self, entity_id: u64) {
        if let Some(change) = self.pending_changes.remove(&entity_id) {
            if self.history.len() >= self.max_history {
                self.history.pop_front();
            }
            self.history.push_back(change);
        }
    }

    /// Clear all pending changes
    pub fn clear(&mut self) {
        let changes: Vec<EntityChange> = self.pending_changes.drain().map(|(_, c)| c).collect();
        for change in changes {
            if self.history.len() >= self.max_history {
                self.history.pop_front();
            }
            self.history.push_back(change);
        }
    }

    /// Move pending changes to history and clear
    pub fn flush(&mut self) -> Vec<EntityChange> {
        let changes: Vec<EntityChange> = self.pending_changes.drain().map(|(_, c)| c).collect();
        // Add to history without borrowing self mutably while iterating
        let history_entries = changes.clone();
        for change in history_entries {
            if self.history.len() >= self.max_history {
                self.history.pop_front();
            }
            self.history.push_back(change);
        }
        changes
    }

    /// Get change history
    pub fn history(&self) -> &VecDeque<EntityChange> {
        &self.history
    }

    /// Get recent changes (last N)
    pub fn recent_changes(&self, count: usize) -> Vec<&EntityChange> {
        self.history.iter().rev().take(count).collect()
    }

    /// Get total changes tracked
    pub fn total_changes(&self) -> u64 {
        self.total_changes
    }

    /// Get changes by kind
    pub fn count_by_kind(&self) -> HashMap<ChangeKind, usize> {
        let mut counts = HashMap::new();
        for change in self.pending_changes.values() {
            *counts.entry(change.kind).or_insert(0) += 1;
        }
        counts
    }
}

impl Default for ChangeTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Utility struct for batching changes
pub struct ChangeBatch {
    changes: Vec<EntityChange>,
}

impl ChangeBatch {
    /// Create a new empty batch
    pub fn new() -> Self {
        Self { changes: Vec::new() }
    }

    /// Add a change to the batch
    pub fn add(&mut self, change: EntityChange) {
        self.changes.push(change);
    }

    /// Get all changes
    pub fn changes(&self) -> &[EntityChange] {
        &self.changes
    }

    /// Get count of changes
    pub fn len(&self) -> usize {
        self.changes.len()
    }

    /// Check if batch is empty
    pub fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }

    /// Clear the batch
    pub fn clear(&mut self) {
        self.changes.clear();
    }

    /// Consume the batch and return changes
    pub fn into_vec(self) -> Vec<EntityChange> {
        self.changes
    }
}

impl Default for ChangeBatch {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_track_entity_change() {
        let mut tracker = ChangeTracker::new();
        
        tracker.track_entity_change(1, ChangeKind::Created);
        assert_eq!(tracker.pending_count(), 1);
        
        let change = tracker.get_pending(1).unwrap();
        assert_eq!(change.kind, ChangeKind::Created);
        assert_eq!(change.entity_id, 1);
    }

    #[test]
    fn test_track_component_change() {
        let mut tracker = ChangeTracker::new();
        
        tracker.track_component_change(1, "Position", ChangeKind::Modified);
        
        let change = tracker.get_pending(1).unwrap();
        assert_eq!(change.kind, ChangeKind::Modified);
        assert_eq!(change.component_changes.len(), 1);
        assert_eq!(change.component_changes[0].component_type, "Position");
    }

    #[test]
    fn test_change_merging() {
        let mut tracker = ChangeTracker::new();
        
        // Create then modify = Create (with updates)
        tracker.track_entity_change(1, ChangeKind::Created);
        tracker.track_entity_change(1, ChangeKind::Modified);
        
        let change = tracker.get_pending(1).unwrap();
        assert_eq!(change.kind, ChangeKind::Created);
        
        // Modify then delete = Delete
        tracker.track_entity_change(2, ChangeKind::Modified);
        tracker.track_entity_change(2, ChangeKind::Deleted);
        
        let change = tracker.get_pending(2).unwrap();
        assert_eq!(change.kind, ChangeKind::Deleted);
    }

    #[test]
    fn test_component_change_merging() {
        let mut tracker = ChangeTracker::new();
        
        tracker.track_component_change(1, "Position", ChangeKind::Created);
        tracker.track_component_change(1, "Stats", ChangeKind::Created);
        tracker.track_component_change(1, "Position", ChangeKind::Modified);
        
        let change = tracker.get_pending(1).unwrap();
        assert_eq!(change.component_changes.len(), 2);
        
        // Position should stay as Created (Created + Modified = Created with updates)
        let pos_change = change
            .component_changes
            .iter()
            .find(|c| c.component_type == "Position")
            .unwrap();
        assert_eq!(pos_change.kind, ChangeKind::Created);
    }

    #[test]
    fn test_clear_pending() {
        let mut tracker = ChangeTracker::new();
        
        tracker.track_entity_change(1, ChangeKind::Created);
        tracker.track_entity_change(2, ChangeKind::Modified);
        
        assert_eq!(tracker.pending_count(), 2);
        
        tracker.clear_pending(1);
        assert_eq!(tracker.pending_count(), 1);
        assert!(tracker.get_pending(1).is_none());
        assert!(tracker.get_pending(2).is_some());
        
        // Check history
        assert_eq!(tracker.history().len(), 1);
    }

    #[test]
    fn test_flush() {
        let mut tracker = ChangeTracker::new();
        
        tracker.track_entity_change(1, ChangeKind::Created);
        tracker.track_entity_change(2, ChangeKind::Modified);
        
        let flushed = tracker.flush();
        assert_eq!(flushed.len(), 2);
        assert_eq!(tracker.pending_count(), 0);
        assert_eq!(tracker.history().len(), 2);
    }

    #[test]
    fn test_count_by_kind() {
        let mut tracker = ChangeTracker::new();
        
        tracker.track_entity_change(1, ChangeKind::Created);
        tracker.track_entity_change(2, ChangeKind::Modified);
        tracker.track_entity_change(3, ChangeKind::Deleted);
        tracker.track_entity_change(4, ChangeKind::Modified);
        
        let counts = tracker.count_by_kind();
        assert_eq!(counts.get(&ChangeKind::Created), Some(&1));
        assert_eq!(counts.get(&ChangeKind::Modified), Some(&2));
        assert_eq!(counts.get(&ChangeKind::Deleted), Some(&1));
    }

    #[test]
    fn test_history_limit() {
        let mut tracker = ChangeTracker::with_capacity(3);
        
        tracker.track_entity_change(1, ChangeKind::Created);
        tracker.flush();
        
        tracker.track_entity_change(2, ChangeKind::Created);
        tracker.flush();
        
        tracker.track_entity_change(3, ChangeKind::Created);
        tracker.flush();
        
        tracker.track_entity_change(4, ChangeKind::Created);
        tracker.flush();
        
        // History should only have 3 most recent
        assert_eq!(tracker.history().len(), 3);
    }
}
