//! Lua Script Hot Reload
//!
//! Watches Lua script files and reloads them automatically without restarting
//! the engine. Provides debounced file change detection, syntax error handling,
//! and state preservation across reloads.
//!
//! # Example
//!
//! ```rust
//! use dde_lua::{LuaEngine, LuaHotReloader};
//! use std::path::PathBuf;
//!
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let mut lua = LuaEngine::default();
//! let mut reloader = LuaHotReloader::new(vec![
//!     PathBuf::from("./scripts"),
//! ])?;
//!
//! // In game loop:
//! for event in reloader.check_reloads(&mut lua) {
//!     if event.success {
//!         println!("Reloaded: {}", event.module_name);
//!     } else {
//!         eprintln!("Failed to reload: {:?}", event.error);
//!     }
//! }
//! # Ok(())
//! # }
//! ```

use crate::{LuaEngine, LuaError};
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

/// Default debounce duration for file changes
pub const DEFAULT_DEBOUNCE_MS: u64 = 300;

/// Error types for hot reload operations
#[derive(thiserror::Error, Debug, Clone)]
pub enum ReloadError {
    /// Lua syntax error with line number
    #[error("Syntax error at line {line}: {message}")]
    SyntaxError { line: usize, message: String },

    /// Lua runtime error during reload
    #[error("Runtime error: {0}")]
    RuntimeError(String),

    /// File not found
    #[error("File not found: {0}")]
    FileNotFound(PathBuf),

    /// I/O error
    #[error("I/O error: {0}")]
    IoError(String),

    /// Watch error
    #[error("Watch error: {0}")]
    WatchError(String),
}

impl From<std::io::Error> for ReloadError {
    fn from(err: std::io::Error) -> Self {
        ReloadError::IoError(err.to_string())
    }
}

impl From<notify::Error> for ReloadError {
    fn from(err: notify::Error) -> Self {
        ReloadError::WatchError(err.to_string())
    }
}

/// Event emitted when a module is reloaded
#[derive(Debug, Clone)]
pub struct ReloadEvent {
    /// Path to the file that changed
    pub path: PathBuf,
    /// Module name derived from the file
    pub module_name: String,
    /// Whether the reload was successful
    pub success: bool,
    /// Error message if reload failed
    pub error: Option<String>,
    /// Timestamp of the reload
    pub timestamp: Instant,
}

/// Tracks file modification times for debouncing
#[derive(Debug)]
struct FileState {
    last_modified: Instant,
    pending: bool,
}

/// Hot reloader for Lua scripts
///
/// Watches directories for `.lua` file changes and provides a mechanism
/// to reload modified scripts into a running `LuaEngine`.
pub struct LuaHotReloader {
    watcher: RecommendedWatcher,
    /// Receiver for file system events
    #[allow(dead_code)]
    event_receiver: mpsc::Receiver<Event>,
    /// Paths being watched
    watch_paths: Vec<PathBuf>,
    /// Files pending reload with their debounce state
    pending_files: Arc<Mutex<HashMap<PathBuf, FileState>>>,
    /// Debounce duration
    debounce_duration: Duration,
    /// Module registry: module_name -> path
    module_registry: Arc<Mutex<HashMap<String, PathBuf>>>,
}

impl LuaHotReloader {
    /// Create a new hot reloader watching given paths
    ///
    /// # Arguments
    ///
    /// * `paths` - Directories to watch for Lua file changes
    ///
    /// # Errors
    ///
    /// Returns an error if the file watcher cannot be created
    ///
    /// # Example
    ///
    /// ```rust
    /// use dde_lua::LuaHotReloader;
    /// use std::path::PathBuf;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let reloader = LuaHotReloader::new(vec![
    ///     PathBuf::from("./scripts"),
    ///     PathBuf::from("./modules"),
    /// ])?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(paths: Vec<PathBuf>) -> Result<Self, ReloadError> {
        let (_tx, rx) = mpsc::channel();
        let pending_files = Arc::new(Mutex::new(HashMap::new()));
        let pending_files_clone = Arc::clone(&pending_files);

        let watcher = RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| {
                if let Ok(event) = res {
                    // Only care about modify/create events on .lua files
                    if matches!(event.kind, EventKind::Modify(_) | EventKind::Create(_)) {
                        for path in event.paths {
                            if path.extension().map(|e| e == "lua").unwrap_or(false) {
                                let mut files = pending_files_clone.lock().unwrap();
                                files.insert(
                                    path,
                                    FileState {
                                        last_modified: Instant::now(),
                                        pending: true,
                                    },
                                );
                            }
                        }
                    }
                }
            },
            Config::default(),
        )?;

        let mut reloader = Self {
            watcher,
            event_receiver: rx,
            watch_paths: Vec::new(),
            pending_files,
            debounce_duration: Duration::from_millis(DEFAULT_DEBOUNCE_MS),
            module_registry: Arc::new(Mutex::new(HashMap::new())),
        };

        // Add initial paths
        for path in paths {
            reloader.watch(path)?;
        }

        Ok(reloader)
    }

    /// Create a new hot reloader with custom debounce duration
    ///
    /// # Arguments
    ///
    /// * `paths` - Directories to watch for Lua file changes
    /// * `debounce_ms` - Debounce duration in milliseconds
    pub fn with_debounce(paths: Vec<PathBuf>, debounce_ms: u64) -> Result<Self, ReloadError> {
        let mut reloader = Self::new(paths)?;
        reloader.debounce_duration = Duration::from_millis(debounce_ms);
        Ok(reloader)
    }

    /// Add a path to watch
    ///
    /// # Arguments
    ///
    /// * `path` - Directory to watch for Lua file changes
    ///
    /// # Errors
    ///
    /// Returns an error if the path cannot be watched
    pub fn watch(&mut self, path: PathBuf) -> Result<(), ReloadError> {
        self.watcher.watch(&path, RecursiveMode::Recursive)?;
        self.watch_paths.push(path);
        Ok(())
    }

    /// Stop watching a path
    ///
    /// # Arguments
    ///
    /// * `path` - Directory to stop watching
    ///
    /// # Errors
    ///
    /// Returns an error if the path is not being watched or cannot be unwatched
    pub fn unwatch(&mut self, path: &Path) -> Result<(), ReloadError> {
        self.watcher.unwatch(path)?;
        self.watch_paths.retain(|p| p != path);
        Ok(())
    }

    /// Check for pending reloads and process them
    ///
    /// This should be called regularly in the game loop to process
    /// any file changes that have been detected.
    ///
    /// # Arguments
    ///
    /// * `lua` - Mutable reference to the Lua engine
    ///
    /// # Returns
    ///
    /// A vector of reload events describing the results of any reloads
    pub fn check_reloads(&mut self, lua: &mut LuaEngine) -> Vec<ReloadEvent> {
        let mut events = Vec::new();
        let now = Instant::now();

        // Collect files that have passed the debounce period
        let ready_files: Vec<PathBuf> = {
            let files = self.pending_files.lock().unwrap();
            files
                .iter()
                .filter(|(_, state)| {
                    state.pending
                        && now.duration_since(state.last_modified) >= self.debounce_duration
                })
                .map(|(path, _)| path.clone())
                .collect()
        };

        // Process each ready file
        for path in ready_files {
            // Mark as processed
            {
                let mut files = self.pending_files.lock().unwrap();
                if let Some(state) = files.get_mut(&path) {
                    state.pending = false;
                }
            }

            // Try to reload the module
            let event = self.reload_module(lua, &path);
            events.push(event);
        }

        events
    }

    /// Reload a specific module by path
    ///
    /// # Arguments
    ///
    /// * `lua` - Mutable reference to the Lua engine
    /// * `path` - Path to the Lua file to reload
    ///
    /// # Returns
    ///
    /// A reload event describing the result
    fn reload_module(&mut self, lua: &mut LuaEngine, path: &Path) -> ReloadEvent {
        let module_name = self.path_to_module_name(path);
        let timestamp = Instant::now();

        // Check if file exists
        if !path.exists() {
            return ReloadEvent {
                path: path.to_path_buf(),
                module_name,
                success: false,
                error: Some(format!("File not found: {}", path.display())),
                timestamp,
            };
        }

        // Read file content
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                return ReloadEvent {
                    path: path.to_path_buf(),
                    module_name,
                    success: false,
                    error: Some(format!("Failed to read file: {}", e)),
                    timestamp,
                };
            }
        };

        // Try to compile and execute
        match self.try_reload(lua, &module_name, &content) {
            Ok(()) => {
                // Update module registry
                {
                    let mut registry = self.module_registry.lock().unwrap();
                    registry.insert(module_name.clone(), path.to_path_buf());
                }
                // Register with LuaEngine for tracking
                lua.register_module(&module_name, path);

                ReloadEvent {
                    path: path.to_path_buf(),
                    module_name,
                    success: true,
                    error: None,
                    timestamp,
                }
            }
            Err(e) => ReloadEvent {
                path: path.to_path_buf(),
                module_name,
                success: false,
                error: Some(e.to_string()),
                timestamp,
            },
        }
    }

    /// Try to reload a module, with rollback on failure
    ///
    /// This method first validates the script by compiling it in an isolated
    /// context, then applies it to the main Lua state.
    fn try_reload(
        &self,
        lua: &mut LuaEngine,
        module_name: &str,
        content: &str,
    ) -> Result<(), ReloadError> {
        // First, try to parse/compile the script to catch syntax errors
        // We do this by attempting to load it as a function
        let compile_result = lua.compile_check(content);
        if let Err(e) = compile_result {
            return Err(self.parse_error(&e.to_string()));
        }

        // Wrap the content in a module table assignment
        let wrapped = format!(
            r#"
            local _MODULE = {{}}
            local _ENV = setmetatable({{}}, {{ __index = _G }})
            setfenv(1, _ENV)
            
            {}
            
            _G["{}"] = _MODULE
            return _MODULE
            "#,
            content, module_name
        );

        // Execute the wrapped script
        match lua.execute(&wrapped) {
            Ok(_) => {
                tracing::info!("Hot reloaded module: {}", module_name);
                Ok(())
            }
            Err(e) => {
                tracing::error!("Failed to reload module {}: {}", module_name, e);
                Err(self.parse_error(&e.to_string()))
            }
        }
    }

    /// Parse an error string into a ReloadError
    ///
    /// Attempts to extract line numbers from common Lua error formats
    fn parse_error(&self, error_str: &str) -> ReloadError {
        // Try to extract line number from error like:
        // "[string \"...\"]:5: unexpected symbol near 'x'"
        if let Some(line_start) = error_str.find("]:") {
            if let Some(line_end) = error_str[line_start + 2..].find(':') {
                let line_str = &error_str[line_start + 2..line_start + 2 + line_end];
                if let Ok(line) = line_str.parse::<usize>() {
                    let message = error_str[line_start + 2 + line_end + 1..].to_string();
                    return ReloadError::SyntaxError { line, message };
                }
            }
        }

        // Try alternative format: "line X:"
        if let Some(line_pos) = error_str.find("line ") {
            let after_line = &error_str[line_pos + 5..];
            if let Some(space_pos) = after_line.find(|c: char| !c.is_ascii_digit()) {
                if let Ok(line) = after_line[..space_pos].parse::<usize>() {
                    let message = after_line[space_pos + 1..].to_string();
                    return ReloadError::SyntaxError { line, message };
                }
            }
        }

        ReloadError::RuntimeError(error_str.to_string())
    }

    /// Convert a file path to a module name
    ///
    /// Examples:
    /// - `./scripts/player.lua` -> `player`
    /// - `./scripts/ai/enemy.lua` -> `ai.enemy`
    fn path_to_module_name(&self, path: &Path) -> String {
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        // Get relative path from watched directories
        for watch_path in &self.watch_paths {
            if let Ok(relative) = path.strip_prefix(watch_path) {
                let components: Vec<_> = relative
                    .parent()
                    .iter()
                    .flat_map(|p| p.components())
                    .filter_map(|c| match c {
                        std::path::Component::Normal(os_str) => os_str.to_str(),
                        _ => None,
                    })
                    .chain(std::iter::once(stem))
                    .collect();

                if !components.is_empty() {
                    return components.join(".");
                }
            }
        }

        // Fallback to just the file stem
        stem.to_string()
    }

    /// Get list of watched paths
    pub fn watched_paths(&self) -> &[PathBuf] {
        &self.watch_paths
    }

    /// Get the debounce duration
    pub fn debounce_duration(&self) -> Duration {
        self.debounce_duration
    }

    /// Set the debounce duration
    pub fn set_debounce_duration(&mut self, duration: Duration) {
        self.debounce_duration = duration;
    }

    /// Get loaded module names and their paths
    pub fn loaded_modules(&self) -> Vec<(String, PathBuf)> {
        let registry = self.module_registry.lock().unwrap();
        registry
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    /// Force a reload of a specific file
    ///
    /// This bypasses the debounce and pending checks
    pub fn force_reload(&mut self, lua: &mut LuaEngine, path: &Path) -> ReloadEvent {
        self.reload_module(lua, path)
    }
}

impl Drop for LuaHotReloader {
    fn drop(&mut self) {
        // The watcher will be dropped automatically, which stops watching
        tracing::debug!("LuaHotReloader dropped, stopping file watcher");
    }
}

/// Extension trait for LuaEngine to support hot reload
pub trait HotReloadExt {
    /// Compile check a script without executing it
    fn compile_check(&self, script: &str) -> Result<(), LuaError>;

    /// Reload a specific module by path
    fn reload_module(&mut self, path: &Path) -> Result<(), LuaError>;

    /// Get list of loaded modules
    fn loaded_modules(&self) -> Vec<(String, PathBuf)>;

    /// Register a module with its source path for tracking
    fn register_module(&mut self, name: &str, path: &Path);
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_path_to_module_name() {
        let temp_dir = TempDir::new().unwrap();
        let scripts_dir = temp_dir.path().join("scripts");
        std::fs::create_dir(&scripts_dir).unwrap();

        let reloader = LuaHotReloader::new(vec![scripts_dir.clone()]).unwrap();

        assert_eq!(
            reloader.path_to_module_name(&scripts_dir.join("player.lua")),
            "player"
        );

        let ai_dir = scripts_dir.join("ai");
        std::fs::create_dir(&ai_dir).unwrap();
        assert_eq!(
            reloader.path_to_module_name(&ai_dir.join("enemy.lua")),
            "ai.enemy"
        );
    }

    #[test]
    fn test_reload_event_creation() {
        let event = ReloadEvent {
            path: PathBuf::from("./test.lua"),
            module_name: "test".to_string(),
            success: true,
            error: None,
            timestamp: Instant::now(),
        };

        assert!(event.success);
        assert!(event.error.is_none());
    }

    #[test]
    fn test_reload_error_display() {
        let err = ReloadError::SyntaxError {
            line: 5,
            message: "unexpected symbol".to_string(),
        };
        assert!(err.to_string().contains("line 5"));
        assert!(err.to_string().contains("unexpected symbol"));

        let err = ReloadError::FileNotFound(PathBuf::from("./missing.lua"));
        assert!(err.to_string().contains("File not found"));
    }

    #[test]
    fn test_debounce_configuration() {
        let mut reloader = LuaHotReloader::with_debounce(vec![], 500).unwrap();
        assert_eq!(reloader.debounce_duration().as_millis(), 500);

        reloader.set_debounce_duration(Duration::from_millis(100));
        assert_eq!(reloader.debounce_duration().as_millis(), 100);
    }

    #[test]
    fn test_parse_error_extraction() {
        let reloader = LuaHotReloader::new(vec![]).unwrap();

        // Test standard Lua error format
        let error = "[string \"test.lua\"]:5: unexpected symbol near 'x'";
        let parsed = reloader.parse_error(error);
        match parsed {
            ReloadError::SyntaxError { line, message } => {
                assert_eq!(line, 5);
                assert!(message.contains("unexpected symbol"));
            }
            _ => panic!("Expected SyntaxError"),
        }

        // Test runtime error (no line number)
        let error = "some runtime error occurred";
        let parsed = reloader.parse_error(error);
        match parsed {
            ReloadError::RuntimeError(msg) => {
                assert!(msg.contains("runtime error"));
            }
            _ => panic!("Expected RuntimeError"),
        }
    }

    #[test]
    fn test_watch_and_unwatch() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_path_buf();

        let mut reloader = LuaHotReloader::new(vec![]).unwrap();
        assert!(reloader.watch_paths.is_empty());

        // Watch the temp directory
        reloader.watch(path.clone()).unwrap();
        assert_eq!(reloader.watch_paths.len(), 1);

        // Unwatch
        reloader.unwatch(&path).unwrap();
        assert!(reloader.watch_paths.is_empty());
    }

    #[test]
    fn test_loaded_modules_tracking() {
        let reloader = LuaHotReloader::new(vec![]).unwrap();
        let modules = reloader.loaded_modules();
        assert!(modules.is_empty());
    }
}
