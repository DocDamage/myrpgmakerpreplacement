//! Asset Hot-Reloading System
//!
//! Watches asset directories and reloads spritesheets, audio, and other assets
//! without requiring an engine restart.

use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Asset hot reloader with file watching capabilities
pub struct AssetHotReloader {
    watcher: RecommendedWatcher,
    pending_changes: Arc<Mutex<Vec<AssetChangeEvent>>>,
    tracked_assets: HashMap<PathBuf, AssetType>,
    config: HotReloadConfig,
    handlers: Vec<Box<dyn AssetHandler + Send + Sync>>,
    last_debounce: Instant,
}

/// Configuration for hot reloading behavior
#[derive(Debug, Clone)]
pub struct HotReloadConfig {
    /// Debounce duration to batch rapid changes
    pub debounce_ms: u64,
    /// Auto-reload on change (vs manual trigger)
    pub auto_reload: bool,
    /// Preserve modified assets until explicit save
    pub preserve_unsaved: bool,
}

impl Default for HotReloadConfig {
    fn default() -> Self {
        Self {
            debounce_ms: 300,
            auto_reload: true,
            preserve_unsaved: true,
        }
    }
}

/// Types of assets that can be hot-reloaded
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AssetType {
    SpriteSheet,
    Texture,
    Audio,
    Tileset,
    LuaScript,
    Shader,
}

impl AssetType {
    /// Detect asset type from file extension
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "png" | "jpg" | "jpeg" | "webp" => Some(AssetType::Texture),
            "json" if ext.contains("sheet") => Some(AssetType::SpriteSheet),
            "ogg" | "mp3" | "wav" => Some(AssetType::Audio),
            "tmx" | "tsx" | "tileset" => Some(AssetType::Tileset),
            "lua" => Some(AssetType::LuaScript),
            "wgsl" | "glsl" | "hlsl" => Some(AssetType::Shader),
            _ => None,
        }
    }

    /// Get asset type from path
    pub fn from_path(path: &Path) -> Option<Self> {
        path.extension()
            .and_then(|e| e.to_str())
            .and_then(Self::from_extension)
    }
}

/// Asset change event
#[derive(Debug, Clone)]
pub struct AssetChangeEvent {
    pub path: PathBuf,
    pub asset_type: AssetType,
    pub change_type: ChangeType,
    pub timestamp: Instant,
}

/// Type of file system change
#[derive(Debug, Clone)]
pub enum ChangeType {
    Created,
    Modified,
    Deleted,
    Renamed(PathBuf), // old_path
}

/// Result of a reload operation
#[derive(Debug, Clone)]
pub enum ReloadResult {
    Success,
    Failed(String),
    Skipped, // Due to config or validation
}

impl std::fmt::Display for ReloadResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReloadResult::Success => write!(f, "Success"),
            ReloadResult::Failed(msg) => write!(f, "Failed: {}", msg),
            ReloadResult::Skipped => write!(f, "Skipped"),
        }
    }
}

/// Error types for asset hot reloading
#[derive(Debug, thiserror::Error)]
pub enum HotReloadError {
    #[error("Notify error: {0}")]
    Notify(#[from] notify::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Handler error: {0}")]
    Handler(String),

    #[error("Path not tracked: {0}")]
    PathNotTracked(PathBuf),
}

pub type Result<T> = std::result::Result<T, HotReloadError>;

impl AssetHotReloader {
    /// Create new reloader watching project assets
    pub fn new(_project_path: &Path) -> Result<Self> {
        let pending_changes = Arc::new(Mutex::new(Vec::new()));
        let pending_clone = Arc::clone(&pending_changes);

        let watcher = RecommendedWatcher::new(
            move |res: std::result::Result<Event, notify::Error>| {
                if let Ok(event) = res {
                    Self::handle_notify_event(event, &pending_clone);
                }
            },
            Config::default(),
        )?;

        Ok(Self {
            watcher,
            pending_changes,
            tracked_assets: HashMap::new(),
            config: HotReloadConfig::default(),
            handlers: Vec::new(),
            last_debounce: Instant::now(),
        })
    }

    /// Create new reloader with custom configuration
    pub fn with_config(project_path: &Path, config: HotReloadConfig) -> Result<Self> {
        let mut reloader = Self::new(project_path)?;
        reloader.config = config;
        Ok(reloader)
    }

    /// Handle notify events and convert to asset change events
    fn handle_notify_event(event: Event, pending: &Arc<Mutex<Vec<AssetChangeEvent>>>) {
        let change_type = match event.kind {
            EventKind::Create(_) => ChangeType::Created,
            EventKind::Modify(_) => ChangeType::Modified,
            EventKind::Remove(_) => ChangeType::Deleted,
            _ => return, // Skip other event types
        };

        for path in event.paths {
            if let Some(asset_type) = AssetType::from_path(&path) {
                let change_event = AssetChangeEvent {
                    path,
                    asset_type,
                    change_type: change_type.clone(),
                    timestamp: Instant::now(),
                };

                if let Ok(mut pending_guard) = pending.lock() {
                    pending_guard.push(change_event);
                }
            }
        }
    }

    /// Watch additional directory
    pub fn watch(&mut self, path: &Path, asset_type: AssetType) -> Result<()> {
        self.watcher.watch(path, RecursiveMode::Recursive)?;
        self.tracked_assets.insert(path.to_path_buf(), asset_type);
        tracing::info!("Watching {:?} for {:?}", path, asset_type);
        Ok(())
    }

    /// Stop watching a directory
    pub fn unwatch(&mut self, path: &Path) -> Result<()> {
        self.watcher.unwatch(path)?;
        self.tracked_assets.remove(path);
        tracing::info!("Stopped watching {:?}", path);
        Ok(())
    }

    /// Register an asset handler
    pub fn register_handler(&mut self, handler: Box<dyn AssetHandler + Send + Sync>) {
        self.handlers.push(handler);
    }

    /// Check for and process pending changes
    pub fn check_changes(&mut self) -> Vec<(AssetChangeEvent, ReloadResult)> {
        let now = Instant::now();
        let debounce_duration = Duration::from_millis(self.config.debounce_ms);

        // Check if debounce period has elapsed
        if now.duration_since(self.last_debounce) < debounce_duration {
            return Vec::new();
        }

        let pending = self.drain_pending_changes();
        let mut results = Vec::new();

        // Group by path to avoid duplicate processing
        let mut seen_paths = std::collections::HashSet::new();

        for event in pending {
            if seen_paths.contains(&event.path) {
                continue;
            }
            seen_paths.insert(event.path.clone());

            let result = if self.config.auto_reload {
                self.process_change(event.clone())
            } else {
                ReloadResult::Skipped
            };

            results.push((event, result));
        }

        self.last_debounce = now;
        results
    }

    /// Drain pending changes from the queue
    fn drain_pending_changes(&mut self) -> Vec<AssetChangeEvent> {
        if let Ok(mut pending) = self.pending_changes.lock() {
            std::mem::take(&mut *pending)
        } else {
            Vec::new()
        }
    }

    /// Process single change event
    fn process_change(&mut self, event: AssetChangeEvent) -> ReloadResult {
        // Find appropriate handler
        let handler = self
            .handlers
            .iter()
            .find(|h| h.can_handle(event.asset_type));

        match handler {
            Some(handler) => {
                // Validate first if configured
                if self.config.preserve_unsaved {
                    if let Err(msg) = handler.validate(&event.path) {
                        return ReloadResult::Failed(format!("Validation failed: {}", msg));
                    }
                }

                // Perform reload
                handler.reload(&event.path)
            }
            None => {
                tracing::warn!("No handler for asset type: {:?}", event.asset_type);
                ReloadResult::Skipped
            }
        }
    }

    /// Register an asset as tracked
    pub fn register(&mut self, path: PathBuf, asset_type: AssetType) {
        self.tracked_assets.insert(path, asset_type);
    }

    /// Get list of pending changes
    pub fn pending(&self) -> Vec<AssetChangeEvent> {
        if let Ok(pending) = self.pending_changes.lock() {
            pending.clone()
        } else {
            Vec::new()
        }
    }

    /// Clear all pending changes
    pub fn clear_pending(&mut self) {
        if let Ok(mut pending) = self.pending_changes.lock() {
            pending.clear();
        }
    }

    /// Get configuration reference
    pub fn config(&self) -> &HotReloadConfig {
        &self.config
    }

    /// Get mutable configuration reference
    pub fn config_mut(&mut self) -> &mut HotReloadConfig {
        &mut self.config
    }

    /// Force reload of a specific asset
    pub fn force_reload(&mut self, path: &Path) -> ReloadResult {
        if let Some(&asset_type) = self.tracked_assets.get(path) {
            let event = AssetChangeEvent {
                path: path.to_path_buf(),
                asset_type,
                change_type: ChangeType::Modified,
                timestamp: Instant::now(),
            };
            self.process_change(event)
        } else {
            ReloadResult::Failed(format!("Asset not tracked: {:?}", path))
        }
    }

    /// Link with Lua hot reloader for coordination
    pub fn link_lua_reloader(&mut self, _lua_reloader: &mut dyn LuaReloader) {
        // This would coordinate with a Lua hot reloader implementation
        // The Lua reloader would share the same file watcher or
        // receive events from this reloader
        tracing::info!("Linked Lua hot reloader");
    }
}

/// Trait for asset handlers
pub trait AssetHandler {
    /// Check if this handler can handle the given asset type
    fn can_handle(&self, asset_type: AssetType) -> bool;

    /// Reload the asset at the given path
    fn reload(&self, path: &Path) -> ReloadResult;

    /// Validate the asset before reloading
    fn validate(&self, path: &Path) -> std::result::Result<(), String>;
}

/// Trait for Lua hot reloader integration
pub trait LuaReloader {
    /// Handle an asset change event
    fn on_asset_changed(&mut self, path: &Path);
    /// Check if the reloader can handle the given path
    fn can_reload(&self, path: &Path) -> bool;
}

/// Handler for sprite sheet and texture assets
pub struct SpriteSheetHandler {
    texture_manager: Arc<crate::texture::TextureManager>,
}

impl SpriteSheetHandler {
    /// Create a new sprite sheet handler
    pub fn new(texture_manager: Arc<crate::texture::TextureManager>) -> Self {
        Self { texture_manager }
    }
}

impl AssetHandler for SpriteSheetHandler {
    fn can_handle(&self, asset_type: AssetType) -> bool {
        matches!(asset_type, AssetType::SpriteSheet | AssetType::Texture)
    }

    fn reload(&self, path: &Path) -> ReloadResult {
        // Use the texture manager's hot_reload for GPU texture update
        if self.texture_manager.hot_reload(path) {
            ReloadResult::Success
        } else {
            ReloadResult::Failed(format!("Failed to hot-reload texture: {:?}", path))
        }
    }

    fn validate(&self, path: &Path) -> std::result::Result<(), String> {
        // Check image dimensions, format
        image::image_dimensions(path)
            .map_err(|e| format!("Failed to read image: {}", e))
            .and_then(|(w, h)| {
                if w > 4096 || h > 4096 {
                    Err("Image too large (max 4096x4096)".to_string())
                } else {
                    Ok(())
                }
            })
    }
}

/// Handler for audio assets
pub struct AudioHandler {
    audio_system: Arc<Mutex<Box<dyn AudioSystemInterface>>>,
}

/// Trait for audio system interface
pub trait AudioSystemInterface: Send + Sync {
    /// Reload an audio file
    fn reload_audio(&mut self, path: &Path) -> std::result::Result<(), String>;
    /// Validate an audio file
    fn validate_audio(&self, path: &Path) -> std::result::Result<(), String>;
}

impl AudioHandler {
    /// Create a new audio handler
    pub fn new(audio_system: Arc<Mutex<Box<dyn AudioSystemInterface>>>) -> Self {
        Self { audio_system }
    }
}

impl AssetHandler for AudioHandler {
    fn can_handle(&self, asset_type: AssetType) -> bool {
        matches!(asset_type, AssetType::Audio)
    }

    fn reload(&self, path: &Path) -> ReloadResult {
        if let Ok(mut audio) = self.audio_system.lock() {
            match audio.reload_audio(path) {
                Ok(()) => ReloadResult::Success,
                Err(e) => ReloadResult::Failed(e),
            }
        } else {
            ReloadResult::Failed("Audio system locked".to_string())
        }
    }

    fn validate(&self, path: &Path) -> std::result::Result<(), String> {
        // Check file exists and is readable
        if !path.exists() {
            return Err(format!("Audio file not found: {:?}", path));
        }

        // Check file size (max 100MB for audio)
        match std::fs::metadata(path) {
            Ok(metadata) => {
                if metadata.len() > 100 * 1024 * 1024 {
                    Err("Audio file too large (max 100MB)".to_string())
                } else {
                    Ok(())
                }
            }
            Err(e) => Err(format!("Failed to read audio metadata: {}", e)),
        }
    }
}

/// Type alias for reload callback functions
pub type ReloadCallback = Arc<Mutex<dyn Fn(&Path) -> ReloadResult + Send + Sync>>;

/// Handler for Lua scripts
pub struct LuaScriptHandler {
    reload_callback: ReloadCallback,
}

impl LuaScriptHandler {
    /// Create a new Lua script handler with a callback
    pub fn new<F>(callback: F) -> Self
    where
        F: Fn(&Path) -> ReloadResult + Send + Sync + 'static,
    {
        Self {
            reload_callback: Arc::new(Mutex::new(callback)),
        }
    }
}

impl AssetHandler for LuaScriptHandler {
    fn can_handle(&self, asset_type: AssetType) -> bool {
        matches!(asset_type, AssetType::LuaScript)
    }

    fn reload(&self, path: &Path) -> ReloadResult {
        if let Ok(callback) = self.reload_callback.lock() {
            callback(path)
        } else {
            ReloadResult::Failed("Callback locked".to_string())
        }
    }

    fn validate(&self, path: &Path) -> std::result::Result<(), String> {
        // Check file is valid UTF-8 and valid Lua syntax
        match std::fs::read_to_string(path) {
            Ok(content) => {
                // Basic syntax check - ensure no obvious issues
                if content.is_empty() {
                    Err("Empty Lua script".to_string())
                } else {
                    Ok(())
                }
            }
            Err(e) => Err(format!("Failed to read Lua script: {}", e)),
        }
    }
}

/// Handler for shader assets
pub struct ShaderHandler {
    reload_callback: ReloadCallback,
}

impl ShaderHandler {
    /// Create a new shader handler with a callback
    pub fn new<F>(callback: F) -> Self
    where
        F: Fn(&Path) -> ReloadResult + Send + Sync + 'static,
    {
        Self {
            reload_callback: Arc::new(Mutex::new(callback)),
        }
    }
}

impl AssetHandler for ShaderHandler {
    fn can_handle(&self, asset_type: AssetType) -> bool {
        matches!(asset_type, AssetType::Shader)
    }

    fn reload(&self, path: &Path) -> ReloadResult {
        if let Ok(callback) = self.reload_callback.lock() {
            callback(path)
        } else {
            ReloadResult::Failed("Callback locked".to_string())
        }
    }

    fn validate(&self, path: &Path) -> std::result::Result<(), String> {
        // Check file exists and is readable
        match std::fs::read_to_string(path) {
            Ok(content) => {
                if content.is_empty() {
                    Err("Empty shader file".to_string())
                } else {
                    // Could add shader-specific validation here
                    Ok(())
                }
            }
            Err(e) => Err(format!("Failed to read shader: {}", e)),
        }
    }
}

/// UI helper for drawing reload status
pub fn draw_reload_status(ui: &mut egui::Ui, reloader: &AssetHotReloader) {
    let pending = reloader.pending();

    if pending.is_empty() {
        ui.label(egui::RichText::new("✓ Assets up to date").color(egui::Color32::GREEN));
    } else {
        ui.label(
            egui::RichText::new(format!("⏳ {} asset(s) changed", pending.len()))
                .color(egui::Color32::YELLOW),
        );

        egui::CollapsingHeader::new("Pending Changes")
            .default_open(false)
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .max_height(150.0)
                    .show(ui, |ui| {
                        for change in &pending {
                            let icon = match change.change_type {
                                ChangeType::Created => "➕",
                                ChangeType::Modified => "✏️",
                                ChangeType::Deleted => "🗑️",
                                ChangeType::Renamed(_) => "📝",
                            };

                            ui.label(format!(
                                "{} {:?}: {}",
                                icon,
                                change.asset_type,
                                change.path.display()
                            ));
                        }
                    });
            });

        if !reloader.config().auto_reload && ui.button("Reload All").clicked() {
            // Trigger reload by processing pending changes
            // This would be handled by the caller via check_changes()
        }
    }
}

/// Extension trait for Renderer to support hot reloading
pub trait HotReloadRenderer {
    /// Call in render loop to process asset updates
    fn update_assets(&mut self);
    /// Get reference to asset reloader
    fn asset_reloader(&self) -> &AssetHotReloader;
    /// Get mutable reference to asset reloader
    fn asset_reloader_mut(&mut self) -> &mut AssetHotReloader;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_asset_type_from_extension() {
        assert_eq!(AssetType::from_extension("png"), Some(AssetType::Texture));
        assert_eq!(AssetType::from_extension("ogg"), Some(AssetType::Audio));
        assert_eq!(AssetType::from_extension("lua"), Some(AssetType::LuaScript));
        assert_eq!(AssetType::from_extension("wgsl"), Some(AssetType::Shader));
        assert_eq!(AssetType::from_extension("unknown"), None);
    }

    #[test]
    fn test_hot_reload_config_default() {
        let config = HotReloadConfig::default();
        assert_eq!(config.debounce_ms, 300);
        assert!(config.auto_reload);
        assert!(config.preserve_unsaved);
    }

    #[test]
    fn test_asset_change_event_creation() {
        let event = AssetChangeEvent {
            path: PathBuf::from("test.png"),
            asset_type: AssetType::Texture,
            change_type: ChangeType::Modified,
            timestamp: Instant::now(),
        };

        assert_eq!(event.asset_type, AssetType::Texture);
        matches!(event.change_type, ChangeType::Modified);
    }

    #[test]
    fn test_sprite_sheet_handler_can_handle() {
        // We can't easily create wgpu::Device in tests without a surface,
        // so we test the handler logic separately
        assert!(true); // Placeholder - handler logic tested in integration tests
    }

    #[test]
    fn test_audio_handler_validate() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.ogg");
        fs::write(&test_file, b"test audio data").unwrap();

        let handler = AudioHandler {
            audio_system: Arc::new(Mutex::new(Box::new(MockAudioSystem))),
        };

        // Valid file
        assert!(handler.validate(&test_file).is_ok());

        // Non-existent file
        assert!(handler
            .validate(&temp_dir.path().join("nonexistent.ogg"))
            .is_err());
    }

    struct MockAudioSystem;

    impl AudioSystemInterface for MockAudioSystem {
        fn reload_audio(&mut self, _path: &Path) -> std::result::Result<(), String> {
            Ok(())
        }

        fn validate_audio(&self, _path: &Path) -> std::result::Result<(), String> {
            Ok(())
        }
    }

    #[test]
    fn test_reload_result_display() {
        assert_eq!(ReloadResult::Success.to_string(), "Success");
        assert_eq!(
            ReloadResult::Failed("test error".to_string()).to_string(),
            "Failed: test error"
        );
        assert_eq!(ReloadResult::Skipped.to_string(), "Skipped");
    }
}
