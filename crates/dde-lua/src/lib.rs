//! DocDamage Engine - Lua Scripting System
//!
//! Embeds Lua 5.4 via mlua for game scripting with a curated,
//! sandboxed API surface.

use mlua::{Function, Lua, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub mod api;
pub mod hot_reload;
pub mod sandbox;
pub mod scripts;

pub use api::DdeApi;
pub use hot_reload::{HotReloadExt, LuaHotReloader, ReloadError, ReloadEvent};
pub use sandbox::{SandboxConfig, SandboxLimits};
pub use scripts::{ScriptManager, ScriptType};

/// Lua scripting engine
pub struct LuaEngine {
    lua: Lua,
    #[allow(dead_code)]
    config: SandboxConfig,
    script_count: usize,
    /// Registry of loaded modules: name -> path
    module_registry: HashMap<String, PathBuf>,
}

/// Error types for Lua scripting
#[derive(thiserror::Error, Debug)]
pub enum LuaError {
    #[error("Lua runtime error: {0}")]
    Runtime(#[from] mlua::Error),

    #[error("Script timeout: exceeded {0}ms limit")]
    Timeout(u64),

    #[error("Memory limit exceeded: {0} bytes")]
    MemoryLimit(usize),

    #[error("Script not found: {0}")]
    ScriptNotFound(String),

    #[error("API error: {0}")]
    Api(String),
}

pub type Result<T> = std::result::Result<T, LuaError>;

impl LuaEngine {
    /// Create a new Lua engine with sandboxed configuration
    pub fn new(config: SandboxConfig) -> Result<Self> {
        let lua = Lua::new();

        // Set memory limit if configured
        if let Some(memory_limit) = config.memory_limit {
            // Note: mlua doesn't directly expose memory limits,
            // but we can track allocations through custom allocators in future
            tracing::info!("Lua memory limit: {} bytes", memory_limit);
        }

        let mut engine = Self {
            lua,
            config,
            script_count: 0,
            module_registry: HashMap::new(),
        };

        // Initialize the DDE API
        engine.init_api()?;

        Ok(engine)
    }

    /// Initialize the curated DDE API
    fn init_api(&mut self) -> Result<()> {
        let globals = self.lua.globals();

        // Create the dde table
        let dde_table = self.lua.create_table()?;

        // World query API
        let get_tile = self
            .lua
            .create_function(|_, (x, y): (i32, i32)| Ok(format!("tile_at_{}_{}", x, y)))?;
        dde_table.set("get_tile", get_tile)?;

        let get_entity = self
            .lua
            .create_function(|_, entity_id: String| Ok(format!("entity_{}", entity_id)))?;
        dde_table.set("get_entity", get_entity)?;

        let get_stat = self.lua.create_function(|_, _stat_name: String| {
            // Placeholder - would query from simulation stats
            Ok(0.5f64)
        })?;
        dde_table.set("get_stat", get_stat)?;

        let get_flag = self.lua.create_function(|_, _flag_name: String| {
            // Placeholder - would query from game flags
            Ok(false)
        })?;
        dde_table.set("get_flag", get_flag)?;

        // World mutation API (queued)
        let set_stat = self
            .lua
            .create_function(|_, (name, value): (String, f64)| {
                tracing::info!("[Lua] set_stat('{}', {})", name, value);
                Ok(())
            })?;
        dde_table.set("set_stat", set_stat)?;

        let set_flag = self
            .lua
            .create_function(|_, (name, value): (String, bool)| {
                tracing::info!("[Lua] set_flag('{}', {})", name, value);
                Ok(())
            })?;
        dde_table.set("set_flag", set_flag)?;

        let set_tile_state =
            self.lua
                .create_function(|_, (tile_id, state): (String, String)| {
                    tracing::info!("[Lua] set_tile_state('{}', '{}')", tile_id, state);
                    Ok(())
                })?;
        dde_table.set("set_tile_state", set_tile_state)?;

        // Entity manipulation
        let move_entity =
            self.lua
                .create_function(|_, (entity_id, x, y): (String, i32, i32)| {
                    tracing::info!("[Lua] move_entity('{}', {}, {})", entity_id, x, y);
                    Ok(())
                })?;
        dde_table.set("move_entity", move_entity)?;

        let spawn_entity =
            self.lua
                .create_function(|_, (entity_type, x, y): (String, i32, i32)| {
                    tracing::info!("[Lua] spawn_entity('{}', {}, {})", entity_type, x, y);
                    Ok("new_entity_id".to_string())
                })?;
        dde_table.set("spawn_entity", spawn_entity)?;

        // Battle API
        let damage = self
            .lua
            .create_function(|_, (target, amount): (String, i32)| {
                tracing::info!("[Lua] damage('{}', {})", target, amount);
                Ok(())
            })?;
        dde_table.set("damage", damage)?;

        let heal = self
            .lua
            .create_function(|_, (target, amount): (String, i32)| {
                tracing::info!("[Lua] heal('{}', {})", target, amount);
                Ok(())
            })?;
        dde_table.set("heal", heal)?;

        let apply_status =
            self.lua
                .create_function(|_, (target, status, duration): (String, String, i32)| {
                    tracing::info!(
                        "[Lua] apply_status('{}', '{}', {})",
                        target,
                        status,
                        duration
                    );
                    Ok(())
                })?;
        dde_table.set("apply_status", apply_status)?;

        // Random number generation (deterministic if seeded)
        let random = self
            .lua
            .create_function(|_, (): ()| Ok(rand::random::<f64>()))?;
        dde_table.set("random", random)?;

        let random_range = self.lua.create_function(|_, (min, max): (f64, f64)| {
            let val = min + rand::random::<f64>() * (max - min);
            Ok(val)
        })?;
        dde_table.set("random_range", random_range)?;

        // Logging
        let log_info = self.lua.create_function(|_, msg: String| {
            tracing::info!("[Lua] {}", msg);
            Ok(())
        })?;
        dde_table.set("log_info", log_info)?;

        let log_warn = self.lua.create_function(|_, msg: String| {
            tracing::warn!("[Lua] {}", msg);
            Ok(())
        })?;
        dde_table.set("log_warn", log_warn)?;

        let log_error = self.lua.create_function(|_, msg: String| {
            tracing::error!("[Lua] {}", msg);
            Ok(())
        })?;
        dde_table.set("log_error", log_error)?;

        // Set the global dde table
        globals.set("dde", dde_table)?;

        // Remove dangerous globals
        globals.set("dofile", Value::Nil)?;
        globals.set("loadfile", Value::Nil)?;
        globals.set("require", Value::Nil)?;
        globals.set("package", Value::Nil)?;
        globals.set("io", Value::Nil)?;
        globals.set("os", Value::Nil)?;

        Ok(())
    }

    /// Execute a Lua script string
    pub fn execute(&mut self, script: &str) -> Result<Value> {
        self.script_count += 1;
        let result = self.lua.load(script).eval()?;
        Ok(result)
    }

    /// Execute a script with timeout (in milliseconds)
    pub fn execute_with_timeout(&mut self, script: &str, timeout_ms: u64) -> Result<Value> {
        // Note: mlua doesn't have built-in timeout support
        // In production, this would need a custom hook or separate thread
        tracing::debug!("Executing script with {}ms timeout", timeout_ms);
        self.execute(script)
    }

    /// Call a Lua function by name
    pub fn call_function(&self, name: &str, args: Vec<Value>) -> Result<Value> {
        let globals = self.lua.globals();
        let func: Function = globals.get(name)?;

        // Handle different argument counts properly for mlua 0.10
        let result = match args.len() {
            0 => func.call(())?,
            1 => func.call(&args[0])?,
            2 => func.call((&args[0], &args[1]))?,
            3 => func.call((&args[0], &args[1], &args[2]))?,
            4 => func.call((&args[0], &args[1], &args[2], &args[3]))?,
            _ => return Err(LuaError::Api("Too many arguments (max 4)".to_string())),
        };
        Ok(result)
    }

    /// Get script statistics
    pub fn stats(&self) -> EngineStats {
        EngineStats {
            script_count: self.script_count,
            memory_used: 0, // Would track actual Lua memory in production
        }
    }

    /// Reset the engine (clear all state)
    pub fn reset(&mut self) -> Result<()> {
        self.lua = Lua::new();
        self.script_count = 0;
        self.module_registry.clear();
        self.init_api()?;
        Ok(())
    }

    /// Reload a specific module by path
    ///
    /// This re-executes the script file and updates the module in the Lua state.
    /// The module is wrapped in a sandboxed environment.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the Lua file to reload
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or the script has syntax/runtime errors
    pub fn reload_module(&mut self, path: &Path) -> Result<()> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| LuaError::Api(format!("Failed to read file: {}", e)))?;

        // Get module name from path
        let module_name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        // Wrap the content in a module table assignment
        let wrapped = format!(
            r#"
            local _MODULE = {{}}
            _G["{}"] = _MODULE
            {}
            "#,
            module_name, content
        );

        self.execute(&wrapped)?;

        // Register the module
        self.register_module(module_name, path);

        tracing::info!("Reloaded module: {} from {}", module_name, path.display());
        Ok(())
    }

    /// Get list of loaded modules
    ///
    /// Returns a vector of tuples containing the module name and its source path.
    pub fn loaded_modules(&self) -> Vec<(String, PathBuf)> {
        self.module_registry
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    /// Register a module with its source path for tracking
    ///
    /// # Arguments
    ///
    /// * `name` - The module name
    /// * `path` - The source file path
    pub fn register_module(&mut self, name: &str, path: &Path) {
        self.module_registry
            .insert(name.to_string(), path.to_path_buf());
    }

    /// Compile check a script without executing it
    ///
    /// This parses the script to verify syntax without running any code.
    ///
    /// # Arguments
    ///
    /// * `script` - The Lua script to check
    ///
    /// # Errors
    ///
    /// Returns an error if the script has syntax errors
    pub fn compile_check(&self, script: &str) -> Result<()> {
        // Use mlua's load to parse without executing
        self.lua
            .load(script)
            .into_function()
            .map_err(LuaError::Runtime)?;
        Ok(())
    }

    /// Get the Lua state (for advanced usage)
    pub fn lua_state(&self) -> &Lua {
        &self.lua
    }

    /// Get mutable access to the Lua state (for advanced usage)
    pub fn lua_state_mut(&mut self) -> &mut Lua {
        &mut self.lua
    }
}

/// Engine statistics
#[derive(Debug, Clone)]
pub struct EngineStats {
    pub script_count: usize,
    pub memory_used: usize,
}

impl Default for LuaEngine {
    fn default() -> Self {
        Self::new(SandboxConfig::default()).expect("Failed to create Lua engine")
    }
}

impl HotReloadExt for LuaEngine {
    fn compile_check(&self, script: &str) -> Result<()> {
        self.compile_check(script)
    }

    fn reload_module(&mut self, path: &Path) -> Result<()> {
        self.reload_module(path)
    }

    fn loaded_modules(&self) -> Vec<(String, PathBuf)> {
        self.loaded_modules()
    }

    fn register_module(&mut self, name: &str, path: &Path) {
        self.register_module(name, path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_execution() {
        let mut engine = LuaEngine::default();
        let result = engine.execute("return 2 + 2").unwrap();
        let num: i32 = result.as_i32().unwrap();
        assert_eq!(num, 4);
    }

    #[test]
    fn test_dde_api() {
        let mut engine = LuaEngine::default();

        // Test get_stat
        let result = engine
            .execute("return dde.get_stat('danger_level')")
            .unwrap();
        let val: f64 = result.as_f64().unwrap();
        assert_eq!(val, 0.5);

        // Test set_stat (should not panic)
        engine.execute("dde.set_stat('test', 0.8)").unwrap();

        // Test random
        let result = engine.execute("return dde.random()").unwrap();
        let val: f64 = result.as_f64().unwrap();
        assert!(val >= 0.0 && val < 1.0);
    }

    #[test]
    fn test_dangerous_globals_removed() {
        let mut engine = LuaEngine::default();

        // io should be nil
        let result = engine.execute("return io == nil").unwrap();
        assert!(result.as_boolean().unwrap());

        // os should be nil
        let result = engine.execute("return os == nil").unwrap();
        assert!(result.as_boolean().unwrap());
    }

    #[test]
    fn test_script_function() {
        let mut engine = LuaEngine::default();

        engine
            .execute(
                r#"
            function on_trigger(entity_id)
                dde.log_info("Triggered by " .. entity_id)
                return true
            end
        "#,
            )
            .unwrap();

        let result = engine
            .call_function(
                "on_trigger",
                vec![Value::String(engine.lua.create_string("player_1").unwrap())],
            )
            .unwrap();

        assert!(result.as_boolean().unwrap());
    }
}
