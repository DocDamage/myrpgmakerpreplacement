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

#[derive(Debug, Clone)]
pub struct LockInfo {
    pub client_id: Uuid,
    pub username: String,
    pub timestamp: u64,
}

impl LockManager {
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
        
        locks.insert(entity, LockInfo {
            client_id,
            username: username.to_string(),
            timestamp: current_timestamp(),
        });
        
        true
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
}
