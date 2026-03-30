//! Script Management
//!
//! Manages Lua scripts from the database and their attachment points.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Type of script attachment
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScriptType {
    /// Attached to an event trigger
    Event,
    /// Attached to an entity (runs each tick)
    Entity,
    /// Battle script (environmental hazards, boss phases)
    Battle,
    /// Global hook (on_tick, on_map_enter, etc.)
    Global,
    /// NPC Behavior scripts
    NpcBehavior,
    /// Quest scripts
    Quest,
    /// Battle AI scripts
    BattleAi,
    /// Utility scripts (shared functions)
    Utility,
}

impl ScriptType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ScriptType::Event => "event",
            ScriptType::Entity => "entity",
            ScriptType::Battle => "battle",
            ScriptType::Global => "global",
            ScriptType::NpcBehavior => "npc_behavior",
            ScriptType::Quest => "quest",
            ScriptType::BattleAi => "battle_ai",
            ScriptType::Utility => "utility",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            ScriptType::Event => "Event Script",
            ScriptType::Entity => "Entity Script",
            ScriptType::Battle => "Battle Script",
            ScriptType::Global => "Global Hook",
            ScriptType::NpcBehavior => "NPC Behavior",
            ScriptType::Quest => "Quest Script",
            ScriptType::BattleAi => "Battle AI",
            ScriptType::Utility => "Utility Script",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            ScriptType::Event => "⚡",
            ScriptType::Entity => "👤",
            ScriptType::Battle => "⚔️",
            ScriptType::Global => "🌍",
            ScriptType::NpcBehavior => "🤖",
            ScriptType::Quest => "📜",
            ScriptType::BattleAi => "🧠",
            ScriptType::Utility => "🛠️",
        }
    }

    pub fn all_types() -> &'static [ScriptType] {
        &[
            ScriptType::NpcBehavior,
            ScriptType::Quest,
            ScriptType::BattleAi,
            ScriptType::Event,
            ScriptType::Entity,
            ScriptType::Utility,
            ScriptType::Battle,
            ScriptType::Global,
        ]
    }
}

impl std::str::FromStr for ScriptType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "event" => Ok(ScriptType::Event),
            "entity" => Ok(ScriptType::Entity),
            "battle" => Ok(ScriptType::Battle),
            "global" => Ok(ScriptType::Global),
            "npc_behavior" | "npc" => Ok(ScriptType::NpcBehavior),
            "quest" => Ok(ScriptType::Quest),
            "battle_ai" => Ok(ScriptType::BattleAi),
            "utility" => Ok(ScriptType::Utility),
            _ => Err(format!("Unknown script type: {}", s)),
        }
    }
}

/// Script metadata for the script manager
#[derive(Debug, Clone)]
pub struct ScriptMetadata {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub source: String,
    pub script_type: ScriptType,
    pub file_path: Option<PathBuf>,
    pub dependencies: Vec<String>,
    pub created_at: i64,
    pub modified_at: i64,
    pub compiled: bool,
    pub syntax_valid: bool,
    pub api_valid: bool,
    pub reload_status: ReloadStatus,
    pub folder_path: String,
}

impl Default for ScriptMetadata {
    fn default() -> Self {
        Self {
            id: 0,
            name: String::new(),
            description: None,
            author: None,
            source: String::new(),
            script_type: ScriptType::Utility,
            file_path: None,
            dependencies: Vec::new(),
            created_at: 0,
            modified_at: 0,
            compiled: false,
            syntax_valid: false,
            api_valid: false,
            reload_status: ReloadStatus::Unloaded,
            folder_path: "/".to_string(),
        }
    }
}

/// Reload status for a script
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReloadStatus {
    Unloaded,
    Loading,
    Loaded,
    Modified,
    Error,
    Reloading,
}

impl ReloadStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ReloadStatus::Unloaded => "unloaded",
            ReloadStatus::Loading => "loading",
            ReloadStatus::Loaded => "loaded",
            ReloadStatus::Modified => "modified",
            ReloadStatus::Error => "error",
            ReloadStatus::Reloading => "reloading",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            ReloadStatus::Unloaded => "Unloaded",
            ReloadStatus::Loading => "Loading",
            ReloadStatus::Loaded => "Active",
            ReloadStatus::Modified => "Modified",
            ReloadStatus::Error => "Error",
            ReloadStatus::Reloading => "Reloading",
        }
    }
}

/// Script validation result
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub valid: bool,
    pub syntax_errors: Vec<SyntaxError>,
    pub api_errors: Vec<ApiError>,
    pub warnings: Vec<String>,
}

impl Default for ValidationResult {
    fn default() -> Self {
        Self {
            valid: true,
            syntax_errors: Vec::new(),
            api_errors: Vec::new(),
            warnings: Vec::new(),
        }
    }
}

/// Syntax error in a script
#[derive(Debug, Clone)]
pub struct SyntaxError {
    pub line: usize,
    pub column: usize,
    pub message: String,
}

/// API usage error
#[derive(Debug, Clone)]
pub struct ApiError {
    pub line: usize,
    pub function: String,
    pub message: String,
}

/// Legacy Script struct for backward compatibility
#[derive(Debug, Clone)]
pub struct Script {
    pub id: i64,
    pub name: String,
    pub source: String,
    pub script_type: ScriptType,
    pub compiled: bool,
}

/// Script manager
pub struct ScriptManager {
    scripts: HashMap<i64, ScriptMetadata>,
    /// Entity ID -> Script IDs
    entity_scripts: HashMap<i64, Vec<i64>>,
    /// Event trigger ID -> Script IDs
    event_scripts: HashMap<i64, Vec<i64>>,
    /// Global hook name -> Script IDs
    global_scripts: HashMap<String, Vec<i64>>,
    /// Folder structure
    folders: Vec<ScriptFolder>,
    /// Last error log
    error_log: Vec<ScriptErrorEntry>,
}

/// Script folder structure
#[derive(Debug, Clone)]
pub struct ScriptFolder {
    pub path: String,
    pub name: String,
    pub parent: Option<String>,
    pub expanded: bool,
}

/// Script error entry for the error log
#[derive(Debug, Clone)]
pub struct ScriptErrorEntry {
    pub timestamp: i64,
    pub script_id: i64,
    pub script_name: String,
    pub error_type: ErrorType,
    pub message: String,
    pub line: Option<usize>,
}

/// Type of error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorType {
    Syntax,
    Runtime,
    Api,
    Load,
}

impl ScriptManager {
    /// Create a new script manager
    pub fn new() -> Self {
        let mut manager = Self {
            scripts: HashMap::new(),
            entity_scripts: HashMap::new(),
            event_scripts: HashMap::new(),
            global_scripts: HashMap::new(),
            folders: Vec::new(),
            error_log: Vec::new(),
        };
        manager.init_default_folders();
        manager
    }

    /// Initialize default folder structure
    fn init_default_folders(&mut self) {
        self.folders = vec![
            ScriptFolder { path: "/npc".to_string(), name: "NPC Behavior".to_string(), parent: Some("/".to_string()), expanded: true },
            ScriptFolder { path: "/quest".to_string(), name: "Quests".to_string(), parent: Some("/".to_string()), expanded: true },
            ScriptFolder { path: "/ai".to_string(), name: "Battle AI".to_string(), parent: Some("/".to_string()), expanded: true },
            ScriptFolder { path: "/events".to_string(), name: "Events".to_string(), parent: Some("/".to_string()), expanded: true },
            ScriptFolder { path: "/entities".to_string(), name: "Entities".to_string(), parent: Some("/".to_string()), expanded: true },
            ScriptFolder { path: "/utility".to_string(), name: "Utilities".to_string(), parent: Some("/".to_string()), expanded: true },
            ScriptFolder { path: "/global".to_string(), name: "Global Hooks".to_string(), parent: Some("/".to_string()), expanded: true },
        ];
    }

    /// Load scripts from database
    pub fn load_from_db(&mut self, db: &dde_db::Database) -> anyhow::Result<()> {
        let conn = db.conn();
        let mut stmt =
            conn.prepare("SELECT script_id, name, source, attachment_type, file_path, created_at, modified_at FROM scripts")?;

        let scripts = stmt.query_map([], |row| {
            let id: i64 = row.get(0)?;
            let name: String = row.get(1)?;
            let source: String = row.get(2)?;
            let attachment: String = row.get(3)?;
            let file_path: Option<String> = row.get(4)?;
            let created_at: i64 = row.get(5)?;
            let modified_at: i64 = row.get(6)?;

            let script_type = attachment
                .parse::<ScriptType>()
                .unwrap_or(ScriptType::Event);

            let folder_path = Self::get_folder_for_type(&script_type);

            Ok(ScriptMetadata {
                id,
                name,
                description: None,
                author: None,
                source,
                script_type,
                file_path: file_path.map(PathBuf::from),
                dependencies: Vec::new(),
                created_at,
                modified_at,
                compiled: false,
                syntax_valid: false,
                api_valid: false,
                reload_status: ReloadStatus::Unloaded,
                folder_path,
            })
        })?;

        for script in scripts {
            let script = script?;
            self.scripts.insert(script.id, script);
        }

        tracing::info!("Loaded {} scripts from database", self.scripts.len());
        Ok(())
    }

    /// Get default folder path for a script type
    fn get_folder_for_type(script_type: &ScriptType) -> String {
        match script_type {
            ScriptType::NpcBehavior => "/npc".to_string(),
            ScriptType::Quest => "/quest".to_string(),
            ScriptType::BattleAi => "/ai".to_string(),
            ScriptType::Event => "/events".to_string(),
            ScriptType::Entity => "/entities".to_string(),
            ScriptType::Utility => "/utility".to_string(),
            ScriptType::Global => "/global".to_string(),
            ScriptType::Battle => "/ai".to_string(),
        }
    }

    /// Get a script by ID
    pub fn get(&self, id: i64) -> Option<&ScriptMetadata> {
        self.scripts.get(&id)
    }

    /// Get mutable reference to script
    pub fn get_mut(&mut self, id: i64) -> Option<&mut ScriptMetadata> {
        self.scripts.get_mut(&id)
    }

    /// Add a script
    pub fn add(&mut self, script: ScriptMetadata) {
        self.scripts.insert(script.id, script);
    }

    /// Remove a script
    pub fn remove(&mut self, id: i64) -> Option<ScriptMetadata> {
        self.scripts.remove(&id)
    }

    /// Duplicate a script
    pub fn duplicate(&mut self, id: i64, new_name: &str) -> Option<ScriptMetadata> {
        let script = self.get(id)?.clone();
        let mut new_script = script;
        new_script.id = self.generate_new_id();
        new_script.name = new_name.to_string();
        new_script.created_at = chrono::Utc::now().timestamp();
        new_script.modified_at = new_script.created_at;
        new_script.reload_status = ReloadStatus::Unloaded;
        self.add(new_script.clone());
        Some(new_script)
    }

    /// Generate a new unique ID
    fn generate_new_id(&self) -> i64 {
        self.scripts.keys().max().copied().unwrap_or(0) + 1
    }

    /// Attach script to entity
    pub fn attach_to_entity(&mut self, script_id: i64, entity_id: i64) {
        self.entity_scripts
            .entry(entity_id)
            .or_default()
            .push(script_id);
    }

    /// Attach script to event
    pub fn attach_to_event(&mut self, script_id: i64, event_id: i64) {
        self.event_scripts
            .entry(event_id)
            .or_default()
            .push(script_id);
    }

    /// Attach script to global hook
    pub fn attach_to_global(&mut self, script_id: i64, hook: &str) {
        self.global_scripts
            .entry(hook.to_string())
            .or_default()
            .push(script_id);
    }

    /// Get scripts attached to entity
    pub fn get_entity_scripts(&self, entity_id: i64) -> Vec<&ScriptMetadata> {
        self.entity_scripts
            .get(&entity_id)
            .map(|ids| ids.iter().filter_map(|id| self.scripts.get(id)).collect())
            .unwrap_or_default()
    }

    /// Get scripts attached to event
    pub fn get_event_scripts(&self, event_id: i64) -> Vec<&ScriptMetadata> {
        self.event_scripts
            .get(&event_id)
            .map(|ids| ids.iter().filter_map(|id| self.scripts.get(id)).collect())
            .unwrap_or_default()
    }

    /// Get global hook scripts
    pub fn get_global_scripts(&self, hook: &str) -> Vec<&ScriptMetadata> {
        self.global_scripts
            .get(hook)
            .map(|ids| ids.iter().filter_map(|id| self.scripts.get(id)).collect())
            .unwrap_or_default()
    }

    /// Get all script IDs
    pub fn script_ids(&self) -> Vec<i64> {
        self.scripts.keys().copied().collect()
    }

    /// Count scripts
    pub fn count(&self) -> usize {
        self.scripts.len()
    }

    /// Get scripts by type
    pub fn get_by_type(&self, script_type: ScriptType) -> Vec<&ScriptMetadata> {
        self.scripts
            .values()
            .filter(|s| s.script_type == script_type)
            .collect()
    }

    /// Get all scripts in a folder
    pub fn get_in_folder(&self, folder_path: &str) -> Vec<&ScriptMetadata> {
        self.scripts
            .values()
            .filter(|s| s.folder_path == folder_path)
            .collect()
    }

    /// Move script to a different folder
    pub fn move_to_folder(&mut self, script_id: i64, folder_path: &str) -> bool {
        if let Some(script) = self.scripts.get_mut(&script_id) {
            script.folder_path = folder_path.to_string();
            script.modified_at = chrono::Utc::now().timestamp();
            true
        } else {
            false
        }
    }

    /// Get all folders
    pub fn folders(&self) -> &[ScriptFolder] {
        &self.folders
    }

    /// Get mutable folders
    pub fn folders_mut(&mut self) -> &mut Vec<ScriptFolder> {
        &mut self.folders
    }

    /// Add a new folder
    pub fn add_folder(&mut self, name: &str, parent: &str) -> String {
        let path = format!("{}/{}", parent.trim_end_matches('/'), name);
        self.folders.push(ScriptFolder {
            path: path.clone(),
            name: name.to_string(),
            parent: Some(parent.to_string()),
            expanded: true,
        });
        path
    }

    /// Remove a folder (and optionally move scripts to parent)
    pub fn remove_folder(&mut self, path: &str, move_scripts_to_parent: bool) {
        if let Some(parent) = self.folders.iter().find(|f| f.path == path).and_then(|f| f.parent.clone()) {
            if move_scripts_to_parent {
                for script in self.scripts.values_mut() {
                    if script.folder_path == path {
                        script.folder_path = parent.clone();
                    }
                }
            }
        }
        self.folders.retain(|f| f.path != path);
    }

    /// Get child folders
    pub fn get_child_folders(&self, parent_path: &str) -> Vec<&ScriptFolder> {
        self.folders
            .iter()
            .filter(|f| f.parent.as_ref() == Some(&parent_path.to_string()))
            .collect()
    }

    /// Add error to log
    pub fn add_error(&mut self, script_id: i64, script_name: &str, error_type: ErrorType, message: &str, line: Option<usize>) {
        self.error_log.push(ScriptErrorEntry {
            timestamp: chrono::Utc::now().timestamp(),
            script_id,
            script_name: script_name.to_string(),
            error_type,
            message: message.to_string(),
            line,
        });
        
        // Keep only last 1000 errors
        if self.error_log.len() > 1000 {
            self.error_log.remove(0);
        }
    }

    /// Get error log
    pub fn error_log(&self) -> &[ScriptErrorEntry] {
        &self.error_log
    }

    /// Clear error log
    pub fn clear_error_log(&mut self) {
        self.error_log.clear();
    }

    /// Get all scripts
    pub fn all_scripts(&self) -> &HashMap<i64, ScriptMetadata> {
        &self.scripts
    }

    /// Get mutable all scripts
    pub fn all_scripts_mut(&mut self) -> &mut HashMap<i64, ScriptMetadata> {
        &mut self.scripts
    }

    /// Search scripts by name
    pub fn search(&self, query: &str) -> Vec<&ScriptMetadata> {
        let query_lower = query.to_lowercase();
        self.scripts
            .values()
            .filter(|s| {
                s.name.to_lowercase().contains(&query_lower)
                    || s.description.as_ref().map(|d| d.to_lowercase().contains(&query_lower)).unwrap_or(false)
            })
            .collect()
    }

    /// Update script reload status
    pub fn update_reload_status(&mut self, script_id: i64, status: ReloadStatus) {
        if let Some(script) = self.scripts.get_mut(&script_id) {
            script.reload_status = status;
        }
    }

    /// Set script validation results
    pub fn set_validation(&mut self, script_id: i64, valid: bool) {
        if let Some(script) = self.scripts.get_mut(&script_id) {
            script.syntax_valid = valid;
            script.api_valid = valid;
        }
    }
}

impl Default for ScriptManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Global hook names
pub mod hooks {
    pub const ON_TICK: &str = "on_tick";
    pub const ON_BATTLE_START: &str = "on_battle_start";
    pub const ON_BATTLE_END: &str = "on_battle_end";
    pub const ON_MAP_ENTER: &str = "on_map_enter";
    pub const ON_MAP_EXIT: &str = "on_map_exit";
    pub const ON_GAME_START: &str = "on_game_start";
    pub const ON_GAME_LOAD: &str = "on_game_load";
    pub const ON_QUEST_START: &str = "on_quest_start";
    pub const ON_QUEST_COMPLETE: &str = "on_quest_complete";
}

/// Script template for creating new scripts
pub struct ScriptTemplate {
    pub script_type: ScriptType,
    pub name: String,
    pub description: String,
    pub default_code: String,
}

impl ScriptTemplate {
    /// Get templates for all script types
    pub fn all_templates() -> Vec<ScriptTemplate> {
        vec![
            ScriptTemplate::npc_behavior(),
            ScriptTemplate::quest(),
            ScriptTemplate::battle_ai(),
            ScriptTemplate::event(),
            ScriptTemplate::utility(),
        ]
    }

    pub fn npc_behavior() -> Self {
        Self {
            script_type: ScriptType::NpcBehavior,
            name: "New NPC Behavior".to_string(),
            description: "NPC behavior script with patrol and interaction logic".to_string(),
            default_code: r#"-- NPC Behavior Script
local npc = {}

function npc.on_spawn(entity_id)
    dde.log_info("NPC spawned: " .. entity_id)
end

function npc.on_interact(entity_id, player_id)
    dde.log_info("NPC interacted: " .. entity_id)
    -- Add dialogue or action here
end

function npc.on_tick(entity_id, delta_time)
    -- Update logic here
end

return npc
"#.to_string(),
        }
    }

    pub fn quest() -> Self {
        Self {
            script_type: ScriptType::Quest,
            name: "New Quest".to_string(),
            description: "Quest script with objectives and rewards".to_string(),
            default_code: r#"-- Quest Script
local quest = {
    id = "quest_001",
    name = "Sample Quest",
    description = "Complete the objectives",
    objectives = {},
    rewards = {}
}

function quest.on_start(player_id)
    dde.log_info("Quest started: " .. quest.name)
end

function quest.on_objective_complete(objective_id)
    dde.log_info("Objective completed: " .. objective_id)
end

function quest.on_complete(player_id)
    dde.log_info("Quest completed!")
    -- Grant rewards
end

return quest
"#.to_string(),
        }
    }

    pub fn battle_ai() -> Self {
        Self {
            script_type: ScriptType::BattleAi,
            name: "New Battle AI".to_string(),
            description: "AI behavior for battle encounters".to_string(),
            default_code: r#"-- Battle AI Script
local ai = {}

function ai.evaluate(battle_state, entity_id)
    -- Return action score (0-100)
    return 50
end

function ai.select_action(battle_state, entity_id)
    -- Return action: { type = "attack"|"skill"|"item"|"flee", target = entity_id, skill_id = "" }
    return { type = "attack", target = battle_state.enemies[1] }
end

function ai.on_turn_start(entity_id)
    dde.log_info("AI turn start: " .. entity_id)
end

return ai
"#.to_string(),
        }
    }

    pub fn event() -> Self {
        Self {
            script_type: ScriptType::Event,
            name: "New Event".to_string(),
            description: "Map event trigger script".to_string(),
            default_code: r#"-- Event Script
local event = {}

function event.on_trigger(trigger_id, entity_id)
    dde.log_info("Event triggered: " .. trigger_id)
    -- Add event logic here
    return true
end

function event.on_exit(trigger_id, entity_id)
    dde.log_info("Event exited: " .. trigger_id)
end

return event
"#.to_string(),
        }
    }

    pub fn utility() -> Self {
        Self {
            script_type: ScriptType::Utility,
            name: "New Utility".to_string(),
            description: "Reusable utility functions".to_string(),
            default_code: r#"-- Utility Script
local utils = {}

-- Add your utility functions here
function utils.example_function(param)
    return param * 2
end

return utils
"#.to_string(),
        }
    }
}

/// Interface for script manager operations
pub trait ScriptManagerInterface {
    fn get_scripts(&self) -> Vec<&ScriptMetadata>;
    fn get_script(&self, id: i64) -> Option<&ScriptMetadata>;
    fn create_script(&mut self, template: &ScriptTemplate, folder: &str) -> Result<ScriptMetadata, String>;
    fn delete_script(&mut self, id: i64) -> Result<(), String>;
    fn duplicate_script(&mut self, id: i64, new_name: &str) -> Result<ScriptMetadata, String>;
    fn move_script(&mut self, id: i64, folder: &str) -> Result<(), String>;
    fn validate_script(&self, id: i64) -> ValidationResult;
    fn reload_script(&mut self, id: i64) -> Result<(), String>;
    fn open_in_external_editor(&self, id: i64) -> Result<(), String>;
    fn get_folders(&self) -> Vec<&ScriptFolder>;
    fn create_folder(&mut self, name: &str, parent: &str) -> Result<String, String>;
    fn delete_folder(&mut self, path: &str) -> Result<(), String>;
    fn get_error_log(&self) -> Vec<&ScriptErrorEntry>;
    fn clear_error_log(&mut self);
}
