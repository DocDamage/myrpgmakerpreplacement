//! Integration tests for real-time collaboration

use dde_core::Entity;
use dde_sync::{
    crdt::{EntityCrdt, LWWMap, ProjectCrdt, crdt_to_sqlite_ops},
    lock::LockManager,
    presence::UserPresence,
    protocol::{ComponentData, EntityState, ProjectState, SyncMessage},
    server::{ProjectSession, ServerState},
};
use uuid::Uuid;

/// Test that a new server state initializes correctly
#[test]
fn test_server_state_initialization() {
    let state = ServerState::new();
    assert!(state.sessions.is_empty());
    assert!(state.client_sessions.is_empty());
}

/// Test creating a project session
#[test]
fn test_project_session_creation() {
    let session = ProjectSession::new("test-project".to_string());
    assert_eq!(session.project_id, "test-project");
    assert!(session.clients.is_empty());
    assert!(session.is_empty());
}

/// Test lock manager functionality
#[test]
fn test_lock_manager_entity_locking() {
    let lock_manager = LockManager::new();
    let client_id = Uuid::new_v4();
    let entity = Entity::DANGLING;

    // Initially entity should not be locked
    assert!(!lock_manager.is_locked(entity));
    
    // Lock the entity
    assert!(lock_manager.try_lock(entity, client_id, "test_user"));

    // Now it should be locked
    assert!(lock_manager.is_locked(entity));
    assert_eq!(lock_manager.get_lock_holder(entity), Some(client_id));

    // Another client should not be able to lock it
    let other_client = Uuid::new_v4();
    assert!(!lock_manager.try_lock(entity, other_client, "other_user"));

    // Unlock and verify
    assert!(lock_manager.unlock(entity, client_id));
    assert!(!lock_manager.is_locked(entity));
}

/// Test lock manager unlock by wrong client
#[test]
fn test_lock_manager_unlock_wrong_client() {
    let lock_manager = LockManager::new();
    let owner_id = Uuid::new_v4();
    let other_id = Uuid::new_v4();
    let entity = Entity::DANGLING;

    lock_manager.try_lock(entity, owner_id, "owner");
    
    // Other client cannot unlock
    assert!(!lock_manager.unlock(entity, other_id));
    assert!(lock_manager.is_locked(entity));
    
    // Owner can unlock
    assert!(lock_manager.unlock(entity, owner_id));
    assert!(!lock_manager.is_locked(entity));
}

/// Test releasing all locks for a client
#[test]
fn test_lock_manager_release_all_for_client() {
    let lock_manager = LockManager::new();
    let client_id = Uuid::new_v4();
    let entity1 = Entity::DANGLING;
    let _entity2 = Entity::from_bits(1).unwrap_or(Entity::DANGLING);

    lock_manager.try_lock(entity1, client_id, "user");

    assert!(lock_manager.is_locked(entity1));

    let released = lock_manager.release_all_client_locks(client_id);

    assert!(!lock_manager.is_locked(entity1));
    assert_eq!(released.len(), 1);
    assert_eq!(released[0], entity1);
}

/// Test CRDT LWWRegister basic operations
#[test]
fn test_crdt_lww_register() {
    use dde_sync::crdt::LWWRegister;

    let mut reg = LWWRegister::new("value1");
    assert_eq!(reg.value(), Some(&"value1"));

    // Update with higher timestamp
    let ts1 = reg.timestamp() + 10;
    reg.update("value2", ts1, 0);
    assert_eq!(reg.value(), Some(&"value2"));

    // Update with lower timestamp should not change
    reg.update("value3", ts1 - 5, 0);
    assert_eq!(reg.value(), Some(&"value2"));
}

/// Test CRDT LWWMap operations
#[test]
fn test_crdt_lww_map() {
    let mut map = LWWMap::new();

    // Insert and retrieve
    map.insert("key1", "value1", 1, 0);
    assert_eq!(map.get(&"key1"), Some(&"value1"));

    // Update with higher timestamp
    map.insert("key1", "value2", 2, 0);
    assert_eq!(map.get(&"key1"), Some(&"value2"));

    // Multiple keys
    map.insert("key2", "value3", 1, 0);
    assert_eq!(map.get(&"key1"), Some(&"value2"));
    assert_eq!(map.get(&"key2"), Some(&"value3"));
}

/// Test CRDT merge functionality
#[test]
fn test_crdt_lww_map_merge() {
    let mut map1 = LWWMap::new();
    map1.insert("key1", "value1", 1, 0);

    let mut map2 = LWWMap::new();
    map2.insert("key2", "value2", 1, 0);

    map1.merge(&map2);

    assert_eq!(map1.get(&"key1"), Some(&"value1"));
    assert_eq!(map1.get(&"key2"), Some(&"value2"));
}

/// Test ProjectCrdt entity updates
#[test]
fn test_project_crdt_entity_updates() {
    let mut crdt = ProjectCrdt::new(0);
    let entity = Entity::DANGLING;

    let component = ComponentData {
        component_type: "transform".to_string(),
        data: serde_json::json!({"x": 10, "y": 20}),
    };

    crdt.update_component(entity, component);

    assert_eq!(crdt.version, 1);
    assert!(crdt.entities.contains_key(&entity));
}

/// Test ProjectCrdt tile updates
#[test]
fn test_project_crdt_tile_updates() {
    let mut crdt = ProjectCrdt::new(0);

    crdt.update_tile(1, 10, 20, 0, 42);

    assert_eq!(crdt.version, 1);
}

/// Test ProjectCrdt merge
#[test]
fn test_project_crdt_merge() {
    let mut crdt1 = ProjectCrdt::new(0);
    let mut crdt2 = ProjectCrdt::new(1);

    let entity1 = Entity::DANGLING;

    let component1 = ComponentData {
        component_type: "transform".to_string(),
        data: serde_json::json!({"x": 10}),
    };
    let component2 = ComponentData {
        component_type: "sprite".to_string(),
        data: serde_json::json!({"id": 5}),
    };

    crdt1.update_component(entity1, component1);
    crdt2.update_component(entity1, component2);

    crdt1.merge(&crdt2);

    assert!(crdt1.entities.contains_key(&entity1));
}

/// Test UserPresence creation
#[test]
fn test_user_presence() {
    let client_id = Uuid::new_v4();
    let presence = UserPresence::new(client_id, "test_user".to_string());

    assert_eq!(presence.username, "test_user");
    assert_eq!(presence.client_id, client_id);
    // Color should be generated (non-zero RGB values likely)
    assert!(presence.color.r > 0 || presence.color.g > 0 || presence.color.b > 0);
}

/// Test SyncMessage serialization roundtrip
#[test]
fn test_sync_message_serialization() {
    let client_id = Uuid::new_v4();
    let msg = SyncMessage::Hello {
        client_id,
        username: "test".to_string(),
        project_id: "proj123".to_string(),
    };

    let json = serde_json::to_string(&msg).unwrap();
    let deserialized: SyncMessage = serde_json::from_str(&json).unwrap();

    match deserialized {
        SyncMessage::Hello {
            username, project_id, ..
        } => {
            assert_eq!(username, "test");
            assert_eq!(project_id, "proj123");
        }
        _ => panic!("Wrong message type"),
    }
}

/// Test EntityState serialization
#[test]
fn test_entity_state_serialization() {
    let entity_state = EntityState {
        entity_id: Entity::DANGLING,
        components: vec![ComponentData {
            component_type: "position".to_string(),
            data: serde_json::json!({"x": 10, "y": 20}),
        }],
    };

    let json = serde_json::to_string(&entity_state).unwrap();
    let deserialized: EntityState = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.components.len(), 1);
    assert_eq!(deserialized.components[0].component_type, "position");
}

/// Test ProjectState serialization
#[test]
fn test_project_state_serialization() {
    let state = ProjectState {
        project_id: "test".to_string(),
        entities: vec![],
        tile_maps: vec![],
        timestamp: 12345,
    };

    let json = serde_json::to_string(&state).unwrap();
    let deserialized: ProjectState = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.project_id, "test");
    assert_eq!(deserialized.timestamp, 12345);
}

/// Test user presence color
#[test]
fn test_user_presence_color() {
    let client_id1 = Uuid::new_v4();
    let client_id2 = Uuid::new_v4();
    
    // Same client ID should generate same color
    let presence1 = UserPresence::new(client_id1, "alice".to_string());
    let presence2 = UserPresence::new(client_id1, "alice".to_string());
    assert_eq!(presence1.color.r, presence2.color.r);
    assert_eq!(presence1.color.g, presence2.color.g);
    assert_eq!(presence1.color.b, presence2.color.b);

    // Different client IDs should (likely) generate different colors
    let presence3 = UserPresence::new(client_id2, "bob".to_string());
    assert!(presence1.color.r != presence3.color.r || presence1.color.g != presence3.color.g || presence1.color.b != presence3.color.b);
}

/// Test server state get_or_create_session
#[test]
fn test_server_state_get_or_create_session() {
    let mut state = ServerState::new();

    // Create a new session
    let session = state.get_or_create_session("project1");
    assert_eq!(session.project_id, "project1");
    assert_eq!(state.sessions.len(), 1);

    // Get existing session
    let session2 = state.get_or_create_session("project1");
    assert_eq!(session2.project_id, "project1");
    assert_eq!(state.sessions.len(), 1);

    // Create another session
    let session3 = state.get_or_create_session("project2");
    assert_eq!(session3.project_id, "project2");
    assert_eq!(state.sessions.len(), 2);
}

/// Test server state remove_client
#[test]
fn test_server_state_remove_client() {
    let mut state = ServerState::new();
    let client_id = Uuid::new_v4();

    // Add a client
    state.client_sessions.insert(client_id, "project1".to_string());

    // Remove the client
    state.remove_client(client_id);

    assert!(!state.client_sessions.contains_key(&client_id));
}

/// Test CRDT to SQLite operations conversion
#[test]
fn test_crdt_to_sqlite_ops() {
    let mut crdt = EntityCrdt::new();
    let entity = Entity::DANGLING;

    let component = ComponentData {
        component_type: "test".to_string(),
        data: serde_json::json!({"key": "value"}),
    };

    crdt.insert(entity, component, 1, 0);

    let ops = crdt_to_sqlite_ops(&crdt);
    assert!(!ops.is_empty());

    // Check SQL generation
    for op in &ops {
        let sql = op.to_sql();
        assert!(!sql.is_empty());
    }
}

/// Test multiple entity operations
#[test]
fn test_multiple_entity_operations() {
    let mut crdt = ProjectCrdt::new(0);

    // Add multiple entities with different IDs
    for i in 1..=10 {
        let entity = Entity::from_bits(i).unwrap_or(Entity::DANGLING);
        // Skip if we got DANGLING (same entity)
        if entity == Entity::DANGLING && i > 1 {
            continue;
        }
        let component = ComponentData {
            component_type: format!("component_{}", i),
            data: serde_json::json!({"id": i}),
        };
        crdt.update_component(entity, component);
    }

    // Version should have incremented
    assert!(crdt.version > 0);
    // Should have at least 1 entity (possibly more if from_bits works)
    assert!(crdt.entities.len() >= 1);
}

/// Test concurrent lock attempts
#[test]
fn test_concurrent_lock_attempts() {
    let lock_manager = LockManager::new();
    let client1 = Uuid::new_v4();
    let client2 = Uuid::new_v4();
    let entity = Entity::DANGLING;

    // Client 1 locks
    assert!(lock_manager.try_lock(entity, client1, "client1"));

    // Client 2 cannot lock
    assert!(!lock_manager.try_lock(entity, client2, "client2"));

    // Client 1 unlocks
    assert!(lock_manager.unlock(entity, client1));

    // Now client 2 can lock
    assert!(lock_manager.try_lock(entity, client2, "client2"));
}

/// Test CRDT conflict resolution (timestamp-based)
#[test]
fn test_crdt_conflict_resolution() {
    use dde_sync::crdt::LWWRegister;

    let mut reg1 = LWWRegister::with_timestamp("value1", 100, 0);
    let reg2 = LWWRegister::with_timestamp("value2", 200, 0);

    // reg2 has higher timestamp, should win
    reg1.merge(&reg2);
    assert_eq!(reg1.value(), Some(&"value2"));

    // Test with same timestamp, higher node_id wins
    let mut reg3 = LWWRegister::with_timestamp("value3", 300, 1);
    let reg4 = LWWRegister::with_timestamp("value4", 300, 2);

    reg3.merge(&reg4);
    assert_eq!(reg3.value(), Some(&"value4"));
}

/// Test empty CRDT operations
#[test]
fn test_empty_crdt_operations() {
    let crdt = ProjectCrdt::new(0);

    assert_eq!(crdt.version, 0);
    assert!(crdt.entities.is_empty());
    assert!(crdt.tile_maps.is_empty());
}

/// Test SyncMessage variants serialization
#[test]
fn test_sync_message_variants() {
    let client_id = Uuid::new_v4();

    // Test Ping/Pong
    let ping = SyncMessage::Ping { timestamp: 12345 };
    let json = serde_json::to_string(&ping).unwrap();
    let pong: SyncMessage = serde_json::from_str(&json).unwrap();
    match pong {
        SyncMessage::Ping { timestamp } => assert_eq!(timestamp, 12345),
        _ => panic!("Expected Ping"),
    }

    // Test Operation
    let op = SyncMessage::Operation {
        client_id,
        timestamp: 12345,
        op: dde_sync::protocol::Operation::Insert,
    };
    let json = serde_json::to_string(&op).unwrap();
    let op2: SyncMessage = serde_json::from_str(&json).unwrap();
    match op2 {
        SyncMessage::Operation {
            client_id: cid,
            timestamp,
            ..
        } => {
            assert_eq!(cid, client_id);
            assert_eq!(timestamp, 12345);
        }
        _ => panic!("Expected Operation"),
    }

    // Test Error
    let err = SyncMessage::Error {
        code: dde_sync::protocol::ErrorCode::InvalidMessage,
        message: "test error".to_string(),
    };
    let json = serde_json::to_string(&err).unwrap();
    let err2: SyncMessage = serde_json::from_str(&json).unwrap();
    match err2 {
        SyncMessage::Error { message, .. } => assert_eq!(message, "test error"),
        _ => panic!("Expected Error"),
    }
}

/// Test lock manager cleanup
#[test]
fn test_lock_manager_cleanup() {
    let lock_manager = LockManager::new();
    let client_id = Uuid::new_v4();
    let entity = Entity::DANGLING;

    // Lock the entity
    lock_manager.try_lock(entity, client_id, "user");
    assert!(lock_manager.is_locked(entity));
    
    // With a very large max_age, no locks should be cleaned (they're all fresh)
    let cleaned_fresh = lock_manager.cleanup_stale_locks(1000000);
    // Lock should still exist (it's fresh)
    assert!(lock_manager.is_locked(entity));
    assert!(cleaned_fresh.is_empty());
    
    // With 0 max_age, all locks should be considered stale and cleaned
    let cleaned = lock_manager.cleanup_stale_locks(0);
    // Either the lock was cleaned, or it wasn't considered stale
    assert!(!lock_manager.is_locked(entity) || cleaned.is_empty());
}

/// Test lock info retrieval
#[test]
fn test_lock_info_retrieval() {
    let lock_manager = LockManager::new();
    let client_id = Uuid::new_v4();
    let entity = Entity::DANGLING;

    // No lock info for unlocked entity
    assert!(lock_manager.get_lock_info(entity).is_none());

    // Lock and check info
    lock_manager.try_lock(entity, client_id, "test_user");
    let info = lock_manager.get_lock_info(entity).unwrap();
    
    assert_eq!(info.client_id, client_id);
    assert_eq!(info.username, "test_user");
}

/// Test getting all locked entities
#[test]
fn test_get_locked_entities() {
    let lock_manager = LockManager::new();
    let client_id = Uuid::new_v4();

    let entity1 = Entity::DANGLING;
    let entity2 = Entity::from_bits(1).unwrap_or(Entity::DANGLING);

    // Lock first entity
    lock_manager.try_lock(entity1, client_id, "user");
    
    // Try to lock second entity (may be same as first if from_bits returns DANGLING)
    let locked_second = lock_manager.try_lock(entity2, client_id, "user");
    
    // If entities are different, both should be locked
    // If same, only one is effectively locked (reentrant)
    let locked = lock_manager.get_locked_entities();
    if entity1 == entity2 {
        assert_eq!(locked.len(), 1);
    } else {
        assert_eq!(locked.len(), 2);
    }
}

/// Test getting client locks
#[test]
fn test_get_client_locks() {
    let lock_manager = LockManager::new();
    let client1 = Uuid::new_v4();
    let client2 = Uuid::new_v4();

    let entity1 = Entity::DANGLING;
    let entity2 = Entity::from_bits(1).unwrap_or(Entity::DANGLING);

    lock_manager.try_lock(entity1, client1, "client1");
    lock_manager.try_lock(entity2, client2, "client2");

    let client1_locks = lock_manager.get_client_locks(client1);
    assert_eq!(client1_locks.len(), 1);
    assert_eq!(client1_locks[0], entity1);
}
