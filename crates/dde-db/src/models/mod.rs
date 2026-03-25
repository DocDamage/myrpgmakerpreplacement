//! Database models

use serde::{Deserialize, Serialize};

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
