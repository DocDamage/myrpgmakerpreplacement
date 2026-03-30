//! Integration tests for database layer

use dde_db::{Database, SaveSlotInfo, ScreenshotData, ScreenshotFormat};
use std::path::PathBuf;

/// Helper to create a temp database path
fn temp_db_path() -> PathBuf {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let random = rand::random::<u64>();
    std::env::temp_dir().join(format!("test_db_{}_{}.db", timestamp, random))
}

/// Test database creation and initialization
#[test]
fn test_database_creation() {
    let db_path = temp_db_path();
    
    let db = Database::create_new(&db_path, "Test Project").unwrap();
    
    // Verify we can get metadata
    let meta = db.get_project_meta().unwrap();
    assert_eq!(meta.project_name, "Test Project");
    
    // Cleanup
    let _ = std::fs::remove_file(&db_path);
}

/// Test database integrity check
#[test]
fn test_database_integrity() {
    let db_path = temp_db_path();
    
    let db = Database::create_new(&db_path, "Test Project").unwrap();
    
    // Check integrity
    let is_valid = db.integrity_check().unwrap();
    assert!(is_valid);
    
    // Cleanup
    let _ = std::fs::remove_file(&db_path);
}

/// Test project metadata retrieval
#[test]
fn test_project_metadata() {
    let db_path = temp_db_path();
    
    let db = Database::create_new(&db_path, "Test Project").unwrap();
    
    let meta = db.get_project_meta().unwrap();
    assert_eq!(meta.project_name, "Test Project");
    assert_eq!(meta.schema_version, 5); // Current schema version
    assert!(!meta.project_id.is_empty());
    
    // Cleanup
    let _ = std::fs::remove_file(&db_path);
}

/// Test database opening
#[test]
fn test_database_open() {
    let db_path = temp_db_path();
    
    // Create a database first
    {
        let _db = Database::create_new(&db_path, "Test Project").unwrap();
    }
    
    // Now open it
    let db = Database::open(&db_path).unwrap();
    let meta = db.get_project_meta().unwrap();
    assert_eq!(meta.project_name, "Test Project");
    
    // Cleanup
    let _ = std::fs::remove_file(&db_path);
}

/// Test save slot operations
#[test]
fn test_save_slot_operations() {
    let db_path = temp_db_path();
    
    let db = Database::create_new(&db_path, "Test Project").unwrap();
    
    // Initially slot should not exist
    assert!(!db.slot_exists(1));
    
    // Save to slot
    db.save_to_slot(1).unwrap();
    
    // Now it should exist
    assert!(db.slot_exists(1));
    
    // Load from slot should return valid path
    let slot_path = db.load_from_slot(1).unwrap();
    assert!(slot_path.exists());
    
    // Cleanup
    let _ = std::fs::remove_file(&db_path);
    let _ = std::fs::remove_file(&slot_path);
}

/// Test save slot with screenshot
#[test]
fn test_save_slot_with_screenshot() {
    let db_path = temp_db_path();
    
    let db = Database::create_new(&db_path, "Test Project").unwrap();
    
    // Create a screenshot
    let screenshot = ScreenshotData {
        data: vec![1, 2, 3, 4, 5],
        width: 320,
        height: 180,
        format: ScreenshotFormat::Webp,
    };
    
    // Save to slot with screenshot
    db.save_to_slot_with_screenshot(1, 3600000, Some(&screenshot)).unwrap();
    
    // Check slot info
    let slot_info = db.get_slot_info(1).unwrap().unwrap();
    assert!(slot_info.has_screenshot);
    assert_eq!(slot_info.play_time_ms, 3600000);
    
    // Cleanup
    let slot_path = db.load_from_slot(1).unwrap();
    let _ = std::fs::remove_file(&db_path);
    let _ = std::fs::remove_file(&slot_path);
}

/// Test save slot deletion
#[test]
fn test_save_slot_deletion() {
    let db_path = temp_db_path();
    
    let db = Database::create_new(&db_path, "Test Project").unwrap();
    
    // Create a slot
    db.save_to_slot(1).unwrap();
    assert!(db.slot_exists(1));
    
    // Delete it
    db.delete_slot(1).unwrap();
    assert!(!db.slot_exists(1));
    
    // Cleanup
    let _ = std::fs::remove_file(&db_path);
}

/// Test list save slots
#[test]
fn test_list_save_slots() {
    let db_path = temp_db_path();
    
    let db = Database::create_new(&db_path, "Test Project").unwrap();
    
    // Create multiple slots
    db.save_to_slot(1).unwrap();
    db.save_to_slot(3).unwrap();
    db.save_to_slot(5).unwrap();
    
    let slots = db.list_slots().unwrap();
    assert_eq!(slots.len(), 3);
    
    // Should be ordered by slot number
    assert_eq!(slots[0].slot_number, 1);
    assert_eq!(slots[1].slot_number, 3);
    assert_eq!(slots[2].slot_number, 5);
    
    // Cleanup
    let _ = std::fs::remove_file(&db_path);
    for slot in [1, 3, 5] {
        let slot_path = format!("{}.slot{:02}.dde", db_path.display(), slot);
        let _ = std::fs::remove_file(&slot_path);
    }
}

/// Test slot info for non-existent slot
#[test]
fn test_nonexistent_slot_info() {
    let db_path = temp_db_path();
    
    let db = Database::create_new(&db_path, "Test Project").unwrap();
    
    let info = db.get_slot_info(99).unwrap();
    assert!(info.is_none());
    
    // Cleanup
    let _ = std::fs::remove_file(&db_path);
}

/// Test invalid slot numbers
#[test]
fn test_invalid_slot_numbers() {
    let db_path = temp_db_path();
    
    let db = Database::create_new(&db_path, "Test Project").unwrap();
    
    // Slot 0 is invalid
    assert!(db.save_to_slot(0).is_err());
    assert!(db.save_to_slot(100).is_err());
    
    // Cleanup
    let _ = std::fs::remove_file(&db_path);
}

/// Test save slot info formatting
#[test]
fn test_save_slot_info_formatting() {
    let info = SaveSlotInfo {
        slot_number: 1,
        saved_at: 0,
        play_time_ms: 3661000, // 1 hour, 1 minute, 1 second
        exists: true,
        has_screenshot: false,
    };
    assert_eq!(info.formatted_play_time(), "01:01:01");
    
    let info_short = SaveSlotInfo {
        slot_number: 1,
        saved_at: 0,
        play_time_ms: 65000, // 1 minute, 5 seconds
        exists: true,
        has_screenshot: false,
    };
    assert_eq!(info_short.formatted_play_time(), "01:05");
    
    // Never saved
    let info_never = SaveSlotInfo::empty(1);
    assert_eq!(info_never.formatted_save_time(), "Never");
}

/// Test screenshot data conversion
#[test]
fn test_screenshot_data_conversion() {
    let screenshot = ScreenshotData {
        data: vec![1, 2, 3, 4, 5],
        width: 320,
        height: 180,
        format: ScreenshotFormat::Png,
    };
    
    assert_eq!(screenshot.width, 320);
    assert_eq!(screenshot.height, 180);
    assert_eq!(screenshot.data, vec![1, 2, 3, 4, 5]);
}

/// Test connection reference
#[test]
fn test_connection_reference() {
    let db_path = temp_db_path();
    
    let db = Database::create_new(&db_path, "Test Project").unwrap();
    
    // Should be able to get connection reference
    let conn = db.conn();
    
    // Can use connection for raw queries
    let result: i32 = conn.query_row("SELECT 1", [], |row| row.get(0)).unwrap();
    assert_eq!(result, 1);
    
    // Cleanup
    let _ = std::fs::remove_file(&db_path);
}
