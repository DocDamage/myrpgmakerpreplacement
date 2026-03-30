//! Script Management
//!
//! Manages Lua scripts from the database and their attachment points.

use std::collections::HashMap;

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
}

impl ScriptType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ScriptType::Event => "event",
            ScriptType::Entity => "entity",
            ScriptType::Battle => "battle",
            ScriptType::Global => "global",
        }
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
            _ => Err(format!("Unknown script type: {}", s)),
        }
    }
}

/// Script metadata
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
    scripts: HashMap<i64, Script>,
    /// Entity ID -> Script IDs
    entity_scripts: HashMap<i64, Vec<i64>>,
    /// Event trigger ID -> Script IDs
    event_scripts: HashMap<i64, Vec<i64>>,
    /// Global hook name -> Script IDs
    global_scripts: HashMap<String, Vec<i64>>,
}

impl ScriptManager {
    /// Create a new script manager
    pub fn new() -> Self {
        Self {
            scripts: HashMap::new(),
            entity_scripts: HashMap::new(),
            event_scripts: HashMap::new(),
            global_scripts: HashMap::new(),
        }
    }

    /// Load scripts from database
    pub fn load_from_db(&mut self, db: &dde_db::Database) -> anyhow::Result<()> {
        let conn = db.conn();
        let mut stmt =
            conn.prepare("SELECT script_id, name, source, attachment_type FROM scripts")?;

        let scripts = stmt.query_map([], |row| {
            let id: i64 = row.get(0)?;
            let name: String = row.get(1)?;
            let source: String = row.get(2)?;
            let attachment: String = row.get(3)?;

            let script_type = attachment
                .parse::<ScriptType>()
                .unwrap_or(ScriptType::Event);

            Ok(Script {
                id,
                name,
                source,
                script_type,
                compiled: false,
            })
        })?;

        for script in scripts {
            let script = script?;
            self.scripts.insert(script.id, script);
        }

        tracing::info!("Loaded {} scripts from database", self.scripts.len());
        Ok(())
    }

    /// Get a script by ID
    pub fn get(&self, id: i64) -> Option<&Script> {
        self.scripts.get(&id)
    }

    /// Get mutable reference to script
    pub fn get_mut(&mut self, id: i64) -> Option<&mut Script> {
        self.scripts.get_mut(&id)
    }

    /// Add a script
    pub fn add(&mut self, script: Script) {
        self.scripts.insert(script.id, script);
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
    pub fn get_entity_scripts(&self, entity_id: i64) -> Vec<&Script> {
        self.entity_scripts
            .get(&entity_id)
            .map(|ids| ids.iter().filter_map(|id| self.scripts.get(id)).collect())
            .unwrap_or_default()
    }

    /// Get scripts attached to event
    pub fn get_event_scripts(&self, event_id: i64) -> Vec<&Script> {
        self.event_scripts
            .get(&event_id)
            .map(|ids| ids.iter().filter_map(|id| self.scripts.get(id)).collect())
            .unwrap_or_default()
    }

    /// Get global hook scripts
    pub fn get_global_scripts(&self, hook: &str) -> Vec<&Script> {
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
    pub fn get_by_type(&self, script_type: ScriptType) -> Vec<&Script> {
        self.scripts
            .values()
            .filter(|s| s.script_type == script_type)
            .collect()
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
}
