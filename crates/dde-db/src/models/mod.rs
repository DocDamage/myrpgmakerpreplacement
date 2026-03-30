//! Database Models
//!
//! Data structures for entities stored in the database.

use serde::{Deserialize, Serialize};

use crate::screenshot::{ScreenshotData, ScreenshotFormat};

/// Dialogue tree model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogueTreeModel {
    pub tree_id: u32,
    pub tree_name: String,
    pub root_node_id: String,
}

/// Dialogue node model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogueNodeModel {
    pub node_id: String,
    pub tree_id: u32,
    pub node_type: String,
    pub speaker: Option<String>,
    pub text: String,
    pub next_node_id: Option<String>,
    pub emotion: Option<String>,
    pub conditions_json: Option<String>,
    pub effects_json: Option<String>,
}

/// Dialogue choice model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogueChoiceModel {
    pub choice_id: u32,
    pub node_id: String,
    pub tree_id: u32,
    pub choice_text: String,
    pub next_node_id: Option<String>,
    pub conditions_json: Option<String>,
    pub effects_json: Option<String>,
    pub sort_order: i32,
}

/// Tile model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tile {
    pub tile_id: u64,
    pub map_id: u32,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub tileset_id: u32,
    pub tile_index: u32,
    pub world_state: i32,
    pub biome: String,
    pub passable: bool,
    pub event_trigger_id: Option<u32>,
}

/// Entity model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityModel {
    pub entity_id: u64,
    pub entity_type: String,
    pub name: String,
    pub map_id: u32,
    pub x: i32,
    pub y: i32,
    pub sprite_sheet_id: Option<u32>,
    pub direction: i32,
    pub logic_prompt: Option<String>,
    pub dialogue_tree_id: Option<u32>,
    pub stats_json: String,
    pub equipment_json: Option<String>,
    pub inventory_json: String,
    pub patrol_path_json: Option<String>,
    pub schedule_json: Option<String>,
    pub faction_id: Option<u32>,
    pub is_interactable: bool,
    pub is_collidable: bool,
    pub respawn_ticks: i32,
}

/// Information about a save slot
#[derive(Debug, Clone)]
pub struct SaveSlotInfo {
    /// Slot number (1-99)
    pub slot_number: u32,
    /// Unix timestamp when the slot was saved
    pub saved_at: i64,
    /// Total play time in milliseconds
    pub play_time_ms: u64,
    /// Whether the slot file exists
    pub exists: bool,
    /// Whether the slot has a screenshot
    pub has_screenshot: bool,
}

impl SaveSlotInfo {
    /// Create a new save slot info for a non-existent slot
    pub fn empty(slot_number: u32) -> Self {
        Self {
            slot_number,
            saved_at: 0,
            play_time_ms: 0,
            exists: false,
            has_screenshot: false,
        }
    }

    /// Format play time as HH:MM:SS
    pub fn formatted_play_time(&self) -> String {
        let total_seconds = self.play_time_ms / 1000;
        let hours = total_seconds / 3600;
        let minutes = (total_seconds % 3600) / 60;
        let seconds = total_seconds % 60;

        if hours > 0 {
            format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
        } else {
            format!("{:02}:{:02}", minutes, seconds)
        }
    }

    /// Format the saved timestamp as a human-readable string
    pub fn formatted_save_time(&self) -> String {
        use chrono::{Local, TimeZone};

        if self.saved_at == 0 {
            return "Never".to_string();
        }

        match Local.timestamp_opt(self.saved_at, 0) {
            chrono::LocalResult::Single(dt) => dt.format("%Y-%m-%d %H:%M").to_string(),
            _ => "Unknown".to_string(),
        }
    }
}

/// Row structure for saving screenshot data to database
#[derive(Debug, Clone)]
pub(crate) struct ScreenshotRow {
    /// Slot number for the screenshot (stored for future API use)
    #[allow(dead_code)]
    pub slot_number: i32,
    pub width: i32,
    pub height: i32,
    pub format: String,
    pub data: Vec<u8>,
}

impl From<&ScreenshotData> for ScreenshotRow {
    fn from(screenshot: &ScreenshotData) -> Self {
        Self {
            slot_number: 0, // Set by caller
            width: screenshot.width as i32,
            height: screenshot.height as i32,
            format: format_to_string(screenshot.format),
            data: screenshot.data.clone(),
        }
    }
}

impl TryFrom<ScreenshotRow> for ScreenshotData {
    type Error = crate::DbError;

    fn try_from(row: ScreenshotRow) -> Result<Self, Self::Error> {
        Ok(ScreenshotData {
            data: row.data,
            width: row.width as u32,
            height: row.height as u32,
            format: string_to_format(&row.format)?,
        })
    }
}

/// Convert format enum to string for storage
fn format_to_string(format: ScreenshotFormat) -> String {
    match format {
        ScreenshotFormat::Png => "png".to_string(),
        ScreenshotFormat::Jpeg => "jpeg".to_string(),
        ScreenshotFormat::Webp => "webp".to_string(),
    }
}

/// Convert string back to format enum
fn string_to_format(s: &str) -> Result<ScreenshotFormat, crate::DbError> {
    match s.to_lowercase().as_str() {
        "png" => Ok(ScreenshotFormat::Png),
        "jpeg" | "jpg" => Ok(ScreenshotFormat::Jpeg),
        "webp" => Ok(ScreenshotFormat::Webp),
        _ => Err(crate::DbError::InvalidData(format!(
            "Unknown screenshot format: {}",
            s
        ))),
    }
}

/// Map model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Map {
    pub map_id: u32,
    pub name: String,
    pub map_type: String,
    pub width: i32,
    pub height: i32,
    pub parent_map_id: Option<u32>,
    pub entry_x: i32,
    pub entry_y: i32,
    pub bgm_id: Option<String>,
    pub ambient_id: Option<String>,
    pub encounter_rate: f64,
    pub encounter_table_id: Option<u32>,
    pub mode7_enabled: bool,
    pub camera_bounds_json: Option<String>,
}

/// Classification rule model for database storage
#[derive(Debug, Clone)]
pub struct ClassificationRuleModel {
    pub id: String,
    pub name: String,
    pub file_pattern: String,
    pub asset_type: String,
    pub auto_tags_json: String,
    pub priority: i32,
    pub enabled: bool,
    pub exact_dimensions: Option<(u32, u32)>,
    pub min_width: Option<u32>,
    pub max_width: Option<u32>,
    pub min_height: Option<u32>,
    pub max_height: Option<u32>,
    pub confidence: f64,
}

impl ClassificationRuleModel {
    /// Parse auto_tags from JSON
    pub fn auto_tags(&self) -> Vec<String> {
        serde_json::from_str(&self.auto_tags_json).unwrap_or_default()
    }

    /// Set auto_tags as JSON
    pub fn set_auto_tags(&mut self, tags: &[String]) {
        self.auto_tags_json = serde_json::to_string(tags).unwrap_or_else(|_| "[]".to_string());
    }
}

/// Classification statistics model
#[derive(Debug, Clone)]
pub struct ClassificationStatsModel {
    pub rule_id: String,
    pub times_matched: i64,
    pub times_applied: i64,
    pub times_overridden: i64,
    pub avg_confidence: Option<f64>,
    pub last_matched_at: Option<i64>,
}

/// Classification queue item
#[derive(Debug, Clone)]
pub struct ClassificationQueueItem {
    pub queue_id: i64,
    pub file_path: String,
    pub file_name: String,
    pub file_size: Option<i64>,
    pub dimensions: Option<(u32, u32)>,
    pub queued_at: i64,
    pub retry_count: i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_save_slot_info_empty() {
        let info = SaveSlotInfo::empty(5);
        assert_eq!(info.slot_number, 5);
        assert_eq!(info.saved_at, 0);
        assert_eq!(info.play_time_ms, 0);
        assert!(!info.exists);
        assert!(!info.has_screenshot);
    }

    #[test]
    fn test_formatted_play_time() {
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
    }

    #[test]
    fn test_formatted_save_time() {
        let info = SaveSlotInfo {
            slot_number: 1,
            saved_at: 0,
            play_time_ms: 0,
            exists: true,
            has_screenshot: false,
        };
        assert_eq!(info.formatted_save_time(), "Never");
    }

    #[test]
    fn test_screenshot_row_conversion() {
        let screenshot = ScreenshotData {
            data: vec![1, 2, 3, 4, 5],
            width: 320,
            height: 180,
            format: ScreenshotFormat::Webp,
        };

        let row = ScreenshotRow::from(&screenshot);
        assert_eq!(row.width, 320);
        assert_eq!(row.height, 180);
        assert_eq!(row.format, "webp");
        assert_eq!(row.data, vec![1, 2, 3, 4, 5]);
    }
}
