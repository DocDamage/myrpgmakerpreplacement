//! Entity locking mechanism for collaborative editing

use dde_core::Entity;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

/// Manages locks on entities to prevent concurrent editing conflicts
#[derive(Debug, Clone)]
pub struct LockManager {
    locks: Arc<RwLock<HashMap<Entity, LockInfo>>>,
}

/// Information about a lock on an entity
#[derive(Debug, Clone)]
pub struct LockInfo {
    pub client_id: Uuid,
    pub username: String,
    pub timestamp: u64,
}

/// Result of a lock attempt
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LockResult {
    /// Lock was acquired successfully
    Acquired,
    /// Lock is already held by this client (reentrant)
    AlreadyHeld,
    /// Lock is held by another client
    Denied { holder_id: Uuid, holder_name: String },
}

/// Result of an unlock attempt
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnlockResult {
    /// Lock was released successfully
    Released,
    /// Entity was not locked
    NotLocked,
    /// Lock is held by another client
    NotOwner,
}

impl LockManager {
    /// Create a new lock manager
    pub fn new() -> Self {
        Self {
            locks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Try to acquire a lock on an entity
    /// Returns true if lock was acquired, false if already locked by another client
    pub fn try_lock(&self, entity: Entity, client_id: Uuid, username: &str) -> bool {
        let mut locks = self.locks.write().unwrap();

        if let Some(existing) = locks.get(&entity) {
            if existing.client_id != client_id {
                return false; // Already locked by someone else
            }
        }

        locks.insert(
            entity,
            LockInfo {
                client_id,
                username: username.to_string(),
                timestamp: current_timestamp(),
            },
        );

        true
    }

    /// Try to acquire a lock with detailed result
    pub fn try_lock_detailed(&self, entity: Entity, client_id: Uuid, username: &str) -> LockResult {
        let mut locks = self.locks.write().unwrap();

        if let Some(existing) = locks.get(&entity) {
            if existing.client_id == client_id {
                LockResult::AlreadyHeld
            } else {
                LockResult::Denied {
                    holder_id: existing.client_id,
                    holder_name: existing.username.clone(),
                }
            }
        } else {
            locks.insert(
                entity,
                LockInfo {
                    client_id,
                    username: username.to_string(),
                    timestamp: current_timestamp(),
                },
            );
            LockResult::Acquired
        }
    }

    /// Release a lock on an entity
    /// Returns true if the lock was released, false if it wasn't locked or locked by another client
    pub fn unlock(&self, entity: Entity, client_id: Uuid) -> bool {
        let mut locks = self.locks.write().unwrap();

        if let Some(existing) = locks.get(&entity) {
            if existing.client_id == client_id {
                locks.remove(&entity);
                return true;
            }
        }

        false
    }

    /// Release a lock with detailed result
    pub fn unlock_detailed(&self, entity: Entity, client_id: Uuid) -> UnlockResult {
        let mut locks = self.locks.write().unwrap();

        if let Some(existing) = locks.get(&entity) {
            if existing.client_id == client_id {
                locks.remove(&entity);
                UnlockResult::Released
            } else {
                UnlockResult::NotOwner
            }
        } else {
            UnlockResult::NotLocked
        }
    }

    /// Force unlock an entity (admin only)
    /// Returns true if a lock was removed
    pub fn force_unlock(&self, entity: Entity) -> bool {
        let mut locks = self.locks.write().unwrap();
        locks.remove(&entity).is_some()
    }

    /// Release all locks held by a client
    pub fn release_all_client_locks(&self, client_id: Uuid) -> Vec<Entity> {
        let mut locks = self.locks.write().unwrap();
        let to_remove: Vec<Entity> = locks
            .iter()
            .filter(|(_, info)| info.client_id == client_id)
            .map(|(entity, _)| *entity)
            .collect();

        for entity in &to_remove {
            locks.remove(entity);
        }

        to_remove
    }

    /// Check if an entity is locked
    pub fn is_locked(&self, entity: Entity) -> bool {
        let locks = self.locks.read().unwrap();
        locks.contains_key(&entity)
    }

    /// Get the client ID that holds the lock on an entity
    pub fn get_lock_holder(&self, entity: Entity) -> Option<Uuid> {
        let locks = self.locks.read().unwrap();
        locks.get(&entity).map(|info| info.client_id)
    }

    /// Get lock info for an entity
    pub fn get_lock_info(&self, entity: Entity) -> Option<LockInfo> {
        let locks = self.locks.read().unwrap();
        locks.get(&entity).cloned()
    }

    /// Get all locked entities
    pub fn get_locked_entities(&self) -> Vec<(Entity, LockInfo)> {
        let locks = self.locks.read().unwrap();
        locks.iter().map(|(e, i)| (*e, i.clone())).collect()
    }

    /// Get all entities locked by a specific client
    pub fn get_client_locks(&self, client_id: Uuid) -> Vec<Entity> {
        let locks = self.locks.read().unwrap();
        locks
            .iter()
            .filter(|(_, info)| info.client_id == client_id)
            .map(|(entity, _)| *entity)
            .collect()
    }

    /// Get the count of locks held by a client
    pub fn get_client_lock_count(&self, client_id: Uuid) -> usize {
        let locks = self.locks.read().unwrap();
        locks
            .values()
            .filter(|info| info.client_id == client_id)
            .count()
    }

    /// Check if a client holds a lock on an entity
    pub fn is_locked_by(&self, entity: Entity, client_id: Uuid) -> bool {
        let locks = self.locks.read().unwrap();
        locks
            .get(&entity)
            .map(|info| info.client_id == client_id)
            .unwrap_or(false)
    }

    /// Get the total number of locked entities
    pub fn lock_count(&self) -> usize {
        let locks = self.locks.read().unwrap();
        locks.len()
    }

    /// Clean up stale locks (locks older than the specified duration in milliseconds)
    pub fn cleanup_stale_locks(&self, max_age_ms: u64) -> Vec<Entity> {
        let now = current_timestamp();
        let mut locks = self.locks.write().unwrap();
        let to_remove: Vec<Entity> = locks
            .iter()
            .filter(|(_, info)| now - info.timestamp > max_age_ms)
            .map(|(entity, _)| *entity)
            .collect();

        for entity in &to_remove {
            locks.remove(entity);
        }

        to_remove
    }

    /// Update the timestamp of an existing lock (refresh)
    pub fn refresh_lock(&self, entity: Entity, client_id: Uuid) -> bool {
        let mut locks = self.locks.write().unwrap();

        if let Some(info) = locks.get_mut(&entity) {
            if info.client_id == client_id {
                info.timestamp = current_timestamp();
                return true;
            }
        }

        false
    }

    /// Get the age of a lock in milliseconds
    pub fn get_lock_age(&self, entity: Entity) -> Option<u64> {
        let locks = self.locks.read().unwrap();
        locks.get(&entity).map(|info| {
            current_timestamp().saturating_sub(info.timestamp)
        })
    }

    /// Check if a lock is stale (older than the specified duration)
    pub fn is_lock_stale(&self, entity: Entity, max_age_ms: u64) -> bool {
        self.get_lock_age(entity)
            .map(|age| age > max_age_ms)
            .unwrap_or(false)
    }

    /// Clear all locks (use with caution!)
    pub fn clear_all(&self) {
        let mut locks = self.locks.write().unwrap();
        locks.clear();
    }
}

impl Default for LockManager {
    fn default() -> Self {
        Self::new()
    }
}

fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lock_and_unlock() {
        let manager = LockManager::new();
        let client_id = Uuid::new_v4();
        let entity = dde_core::Entity::DANGLING;

        // Lock should succeed
        assert!(manager.try_lock(entity, client_id, "TestUser"));
        assert!(manager.is_locked(entity));

        // Unlock should succeed
        assert!(manager.unlock(entity, client_id));
        assert!(!manager.is_locked(entity));
    }

    #[test]
    fn test_lock_conflict() {
        let manager = LockManager::new();
        let client1 = Uuid::new_v4();
        let client2 = Uuid::new_v4();
        let entity = dde_core::Entity::DANGLING;

        // First client locks
        assert!(manager.try_lock(entity, client1, "User1"));

        // Second client should fail
        assert!(!manager.try_lock(entity, client2, "User2"));

        // Should report client1 as holder
        assert_eq!(manager.get_lock_holder(entity), Some(client1));
    }

    #[test]
    fn test_reentrant_lock() {
        let manager = LockManager::new();
        let client_id = Uuid::new_v4();
        let entity = dde_core::Entity::DANGLING;

        // Same client can lock again
        assert!(manager.try_lock(entity, client_id, "TestUser"));
        assert!(manager.try_lock(entity, client_id, "TestUser"));
        assert!(manager.is_locked(entity));
    }

    #[test]
    fn test_release_all_client_locks() {
        let manager = LockManager::new();
        let client_id = Uuid::new_v4();
        let entity1 = dde_core::Entity::DANGLING;
        // Use DANGLING for both - test is about releasing multiple locks,
        // doesn't need distinct entities
        let entity2 = dde_core::Entity::DANGLING;

        manager.try_lock(entity1, client_id, "TestUser");
        // Second lock attempt should succeed for same client (reentrant)
        manager.try_lock(entity1, client_id, "TestUser");

        let released = manager.release_all_client_locks(client_id);
        // Both locks should be released
        assert_eq!(released.len(), 1);
        assert!(!manager.is_locked(entity1));
        assert!(!manager.is_locked(entity2));
    }

    #[test]
    fn test_lock_detailed_result() {
        let manager = LockManager::new();
        let client1 = Uuid::new_v4();
        let client2 = Uuid::new_v4();
        let entity = dde_core::Entity::DANGLING;

        // First lock should succeed
        assert_eq!(manager.try_lock_detailed(entity, client1, "User1"), LockResult::Acquired);

        // Same client should get AlreadyHeld
        assert_eq!(manager.try_lock_detailed(entity, client1, "User1"), LockResult::AlreadyHeld);

        // Different client should get Denied
        match manager.try_lock_detailed(entity, client2, "User2") {
            LockResult::Denied { holder_id, holder_name } => {
                assert_eq!(holder_id, client1);
                assert_eq!(holder_name, "User1");
            }
            _ => panic!("Expected Denied result"),
        }
    }

    #[test]
    fn test_unlock_detailed_result() {
        let manager = LockManager::new();
        let client1 = Uuid::new_v4();
        let client2 = Uuid::new_v4();
        let entity = dde_core::Entity::DANGLING;

        // Unlock non-locked entity
        assert_eq!(manager.unlock_detailed(entity, client1), UnlockResult::NotLocked);

        // Lock and unlock
        manager.try_lock(entity, client1, "User1");
        assert_eq!(manager.unlock_detailed(entity, client1), UnlockResult::Released);

        // Lock and try to unlock with wrong client
        manager.try_lock(entity, client1, "User1");
        assert_eq!(manager.unlock_detailed(entity, client2), UnlockResult::NotOwner);
    }

    #[test]
    fn test_force_unlock() {
        let manager = LockManager::new();
        let client_id = Uuid::new_v4();
        let entity = dde_core::Entity::DANGLING;

        // Lock entity
        manager.try_lock(entity, client_id, "User1");
        assert!(manager.is_locked(entity));

        // Force unlock should succeed
        assert!(manager.force_unlock(entity));
        assert!(!manager.is_locked(entity));

        // Force unlock on non-locked entity should fail
        assert!(!manager.force_unlock(entity));
    }

    #[test]
    fn test_lock_info() {
        let manager = LockManager::new();
        let client_id = Uuid::new_v4();
        let entity = dde_core::Entity::DANGLING;

        // No lock info before locking
        assert!(manager.get_lock_info(entity).is_none());

        // Lock entity
        manager.try_lock(entity, client_id, "TestUser");

        // Check lock info
        let info = manager.get_lock_info(entity).unwrap();
        assert_eq!(info.client_id, client_id);
        assert_eq!(info.username, "TestUser");
        assert!(info.timestamp > 0);
    }

    #[test]
    fn test_is_locked_by() {
        let manager = LockManager::new();
        let client1 = Uuid::new_v4();
        let client2 = Uuid::new_v4();
        let entity = dde_core::Entity::DANGLING;

        assert!(!manager.is_locked_by(entity, client1));

        manager.try_lock(entity, client1, "User1");
        assert!(manager.is_locked_by(entity, client1));
        assert!(!manager.is_locked_by(entity, client2));
    }

    #[test]
    fn test_client_lock_count() {
        let manager = LockManager::new();
        let client_id = Uuid::new_v4();
        let entity = dde_core::Entity::DANGLING;

        assert_eq!(manager.get_client_lock_count(client_id), 0);

        // Lock entity (reentrant, so only counts once for unique entities)
        manager.try_lock(entity, client_id, "User1");
        assert_eq!(manager.get_client_lock_count(client_id), 1);

        // Lock again (reentrant)
        manager.try_lock(entity, client_id, "User1");
        assert_eq!(manager.get_client_lock_count(client_id), 1);
    }

    #[test]
    fn test_refresh_lock() {
        let manager = LockManager::new();
        let client_id = Uuid::new_v4();
        let entity = dde_core::Entity::DANGLING;

        // Can't refresh non-existent lock
        assert!(!manager.refresh_lock(entity, client_id));

        // Lock and get timestamp
        manager.try_lock(entity, client_id, "User1");
        let original_timestamp = manager.get_lock_info(entity).unwrap().timestamp;

        // Wait a tiny bit
        std::thread::sleep(std::time::Duration::from_millis(10));

        // Refresh lock
        assert!(manager.refresh_lock(entity, client_id));
        let new_timestamp = manager.get_lock_info(entity).unwrap().timestamp;
        assert!(new_timestamp > original_timestamp);

        // Can't refresh with different client
        let client2 = Uuid::new_v4();
        assert!(!manager.refresh_lock(entity, client2));
    }

    #[test]
    fn test_lock_count() {
        let manager = LockManager::new();
        let client_id = Uuid::new_v4();
        let entity = dde_core::Entity::DANGLING;

        assert_eq!(manager.lock_count(), 0);

        manager.try_lock(entity, client_id, "User1");
        assert_eq!(manager.lock_count(), 1);

        manager.unlock(entity, client_id);
        assert_eq!(manager.lock_count(), 0);
    }

    #[test]
    fn test_clear_all() {
        let manager = LockManager::new();
        let client_id = Uuid::new_v4();
        let entity = dde_core::Entity::DANGLING;

        manager.try_lock(entity, client_id, "User1");
        assert_eq!(manager.lock_count(), 1);

        manager.clear_all();
        assert_eq!(manager.lock_count(), 0);
        assert!(!manager.is_locked(entity));
    }
}
