//! DDE Lua API
//!
//! Curated API surface exposed to Lua scripts.

use mlua::{Lua, Result as LuaResult, Table};

/// Main DDE API for Lua scripts
pub struct DdeApi;

impl DdeApi {
    /// Create a new API instance
    pub fn new() -> Self {
        Self
    }

    /// Register all API functions with the Lua state
    pub fn register(&self, lua: &Lua) -> LuaResult<()> {
        let globals = lua.globals();
        let dde = lua.create_table()?;

        // World query API
        Self::register_world_queries(lua, &dde)?;

        // World mutations
        Self::register_world_mutations(lua, &dde)?;

        // Entity operations
        Self::register_entity_ops(lua, &dde)?;

        // Battle operations
        Self::register_battle_ops(lua, &dde)?;

        // Utility functions
        Self::register_utils(lua, &dde)?;

        globals.set("dde", dde)?;
        Ok(())
    }

    fn register_world_queries(lua: &Lua, dde: &Table) -> LuaResult<()> {
        // get_tile(x, y, [map_id]) -> tile_info
        let get_tile = lua.create_function(|lua, (x, y): (i32, i32)| {
            let tile = lua.create_table()?;
            tile.set("x", x)?;
            tile.set("y", y)?;
            tile.set("passable", true)?;
            tile.set("biome", "grassland")?;
            Ok(tile)
        })?;
        dde.set("get_tile", get_tile)?;

        // get_entities_in_area(x, y, radius) -> entity_list
        let get_entities_in_area =
            lua.create_function(|lua, (_x, _y, _radius): (i32, i32, f32)| {
                let entities = lua.create_table()?;
                Ok(entities)
            })?;
        dde.set("get_entities_in_area", get_entities_in_area)?;

        // get_stat(name) -> value
        let get_stat = lua.create_function(|_, name: String| {
            tracing::debug!("[Lua API] get_stat('{}')", name);
            Ok(0.5f64)
        })?;
        dde.set("get_stat", get_stat)?;

        // get_flag(name) -> bool
        let get_flag = lua.create_function(|_, name: String| {
            tracing::debug!("[Lua API] get_flag('{}')", name);
            Ok(false)
        })?;
        dde.set("get_flag", get_flag)?;

        Ok(())
    }

    fn register_world_mutations(lua: &Lua, dde: &Table) -> LuaResult<()> {
        // set_stat(name, value)
        let set_stat = lua.create_function(|_, (name, value): (String, f64)| {
            tracing::info!("[Lua API] set_stat('{}', {})", name, value);
            Ok(())
        })?;
        dde.set("set_stat", set_stat)?;

        // set_flag(name, value)
        let set_flag = lua.create_function(|_, (name, value): (String, bool)| {
            tracing::info!("[Lua API] set_flag('{}', {})", name, value);
            Ok(())
        })?;
        dde.set("set_flag", set_flag)?;

        // set_tile_state(x, y, state)
        let set_tile_state = lua.create_function(|_, (x, y, state): (i32, i32, String)| {
            tracing::info!("[Lua API] set_tile_state({}, {}, '{}')", x, y, state);
            Ok(())
        })?;
        dde.set("set_tile_state", set_tile_state)?;

        Ok(())
    }

    fn register_entity_ops(lua: &Lua, dde: &Table) -> LuaResult<()> {
        // spawn_entity(type, x, y) -> entity_id
        let spawn_entity = lua.create_function(|_, (entity_type, x, y): (String, i32, i32)| {
            tracing::info!("[Lua API] spawn_entity('{}', {}, {})", entity_type, x, y);
            Ok(format!("entity_{}_{}_{}", entity_type, x, y))
        })?;
        dde.set("spawn_entity", spawn_entity)?;

        // move_entity(entity_id, x, y)
        let move_entity = lua.create_function(|_, (entity_id, x, y): (String, i32, i32)| {
            tracing::info!("[Lua API] move_entity('{}', {}, {})", entity_id, x, y);
            Ok(())
        })?;
        dde.set("move_entity", move_entity)?;

        // remove_entity(entity_id)
        let remove_entity = lua.create_function(|_, entity_id: String| {
            tracing::info!("[Lua API] remove_entity('{}')", entity_id);
            Ok(())
        })?;
        dde.set("remove_entity", remove_entity)?;

        Ok(())
    }

    fn register_battle_ops(lua: &Lua, dde: &Table) -> LuaResult<()> {
        // damage(target, amount)
        let damage = lua.create_function(|_, (target, amount): (String, i32)| {
            tracing::info!("[Lua API] damage('{}', {})", target, amount);
            Ok(())
        })?;
        dde.set("damage", damage)?;

        // heal(target, amount)
        let heal = lua.create_function(|_, (target, amount): (String, i32)| {
            tracing::info!("[Lua API] heal('{}', {})", target, amount);
            Ok(())
        })?;
        dde.set("heal", heal)?;

        // apply_status(target, status, duration)
        let apply_status =
            lua.create_function(|_, (target, status, duration): (String, String, i32)| {
                tracing::info!(
                    "[Lua API] apply_status('{}', '{}', {})",
                    target,
                    status,
                    duration
                );
                Ok(())
            })?;
        dde.set("apply_status", apply_status)?;

        Ok(())
    }

    fn register_utils(lua: &Lua, dde: &Table) -> LuaResult<()> {
        // random() -> 0.0-1.0
        let random = lua.create_function(|_, (): ()| Ok(rand::random::<f64>()))?;
        dde.set("random", random)?;

        // random_range(min, max) -> value
        let random_range = lua.create_function(|_, (min, max): (f64, f64)| {
            Ok(min + rand::random::<f64>() * (max - min))
        })?;
        dde.set("random_range", random_range)?;

        // random_int(min, max) -> integer
        let random_int = lua.create_function(|_, (min, max): (i32, i32)| {
            use rand::Rng;
            let mut rng = rand::thread_rng();
            Ok(rng.gen_range(min..=max))
        })?;
        dde.set("random_int", random_int)?;

        // Logging
        let log_info = lua.create_function(|_, msg: String| {
            tracing::info!("[Lua] {}", msg);
            Ok(())
        })?;
        dde.set("log_info", log_info)?;

        let log_warn = lua.create_function(|_, msg: String| {
            tracing::warn!("[Lua] {}", msg);
            Ok(())
        })?;
        dde.set("log_warn", log_warn)?;

        let log_error = lua.create_function(|_, msg: String| {
            tracing::error!("[Lua] {}", msg);
            Ok(())
        })?;
        dde.set("log_error", log_error)?;

        Ok(())
    }
}

impl Default for DdeApi {
    fn default() -> Self {
        Self::new()
    }
}
