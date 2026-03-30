//! Asset Hot-Reload module tests
//!
//! Tests for AssetHotReloader, AssetType, and related functionality

use crate::asset_hot_reload::{
    AssetChangeEvent, AssetHandler, AssetType, ChangeType, HotReloadConfig, HotReloadError,
    ReloadResult,
};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;

// Mock AssetHandler for testing
type HandlerFn = Box<dyn Fn(&Path) -> ReloadResult + Send + Sync>;
type ValidateFn = Box<dyn Fn(&Path) -> Result<(), String> + Send + Sync>;

pub struct MockAssetHandler {
    can_handle_fn: Box<dyn Fn(AssetType) -> bool + Send + Sync>,
    reload_fn: HandlerFn,
    validate_fn: ValidateFn,
}

impl MockAssetHandler {
    pub fn new<F, G, H>(can_handle: F, reload: G, validate: H) -> Self
    where
        F: Fn(AssetType) -> bool + Send + Sync + 'static,
        G: Fn(&Path) -> ReloadResult + Send + Sync + 'static,
        H: Fn(&Path) -> Result<(), String> + Send + Sync + 'static,
    {
        Self {
            can_handle_fn: Box::new(can_handle),
            reload_fn: Box::new(reload),
            validate_fn: Box::new(validate),
        }
    }

    pub fn always_success(asset_type: AssetType) -> Self {
        Self::new(
            move |t| t == asset_type,
            |_| ReloadResult::Success,
            |_| Ok(()),
        )
    }

    pub fn always_fail(asset_type: AssetType, msg: String) -> Self {
        let msg_clone = msg.clone();
        Self::new(
            move |t| t == asset_type,
            move |_| ReloadResult::Failed(msg.clone()),
            move |_| Err(msg_clone.clone()),
        )
    }
}

impl AssetHandler for MockAssetHandler {
    fn can_handle(&self, asset_type: AssetType) -> bool {
        (self.can_handle_fn)(asset_type)
    }

    fn reload(&self, path: &Path) -> ReloadResult {
        (self.reload_fn)(path)
    }

    fn validate(&self, path: &Path) -> Result<(), String> {
        (self.validate_fn)(path)
    }
}

#[test]
fn test_asset_type_from_extension() {
    // Image formats
    assert_eq!(AssetType::from_extension("png"), Some(AssetType::Texture));
    assert_eq!(AssetType::from_extension("PNG"), Some(AssetType::Texture));
    assert_eq!(AssetType::from_extension("jpg"), Some(AssetType::Texture));
    assert_eq!(AssetType::from_extension("jpeg"), Some(AssetType::Texture));
    assert_eq!(AssetType::from_extension("webp"), Some(AssetType::Texture));

    // Audio formats
    assert_eq!(AssetType::from_extension("ogg"), Some(AssetType::Audio));
    assert_eq!(AssetType::from_extension("mp3"), Some(AssetType::Audio));
    assert_eq!(AssetType::from_extension("wav"), Some(AssetType::Audio));

    // Script formats
    assert_eq!(AssetType::from_extension("lua"), Some(AssetType::LuaScript));

    // Shader formats
    assert_eq!(AssetType::from_extension("wgsl"), Some(AssetType::Shader));
    assert_eq!(AssetType::from_extension("glsl"), Some(AssetType::Shader));
    assert_eq!(AssetType::from_extension("hlsl"), Some(AssetType::Shader));

    // Tileset formats
    assert_eq!(AssetType::from_extension("tmx"), Some(AssetType::Tileset));
    assert_eq!(AssetType::from_extension("tsx"), Some(AssetType::Tileset));
    assert_eq!(
        AssetType::from_extension("tileset"),
        Some(AssetType::Tileset)
    );

    // Unknown extension
    assert_eq!(AssetType::from_extension("unknown"), None);
    assert_eq!(AssetType::from_extension(""), None);
}

#[test]
fn test_asset_type_from_path() {
    use std::path::Path;

    assert_eq!(
        AssetType::from_path(Path::new("assets/player.png")),
        Some(AssetType::Texture)
    );
    assert_eq!(
        AssetType::from_path(Path::new("assets/bgm.ogg")),
        Some(AssetType::Audio)
    );
    assert_eq!(
        AssetType::from_path(Path::new("scripts/game.lua")),
        Some(AssetType::LuaScript)
    );
    assert_eq!(
        AssetType::from_path(Path::new("shaders/sprite.wgsl")),
        Some(AssetType::Shader)
    );

    // No extension
    assert_eq!(AssetType::from_path(Path::new("README")), None);

    // Hidden file with extension
    assert_eq!(AssetType::from_path(Path::new(".gitignore")), None);
}

#[test]
fn test_hot_reload_config_default() {
    let config = HotReloadConfig::default();
    assert_eq!(config.debounce_ms, 300);
    assert!(config.auto_reload);
    assert!(config.preserve_unsaved);
}

#[test]
fn test_hot_reload_config_clone() {
    let config = HotReloadConfig::default();
    let cloned = config.clone();

    assert_eq!(config.debounce_ms, cloned.debounce_ms);
    assert_eq!(config.auto_reload, cloned.auto_reload);
    assert_eq!(config.preserve_unsaved, cloned.preserve_unsaved);
}

#[test]
fn test_asset_change_event_creation() {
    let path = PathBuf::from("assets/test.png");
    let event = AssetChangeEvent {
        path: path.clone(),
        asset_type: AssetType::Texture,
        change_type: ChangeType::Modified,
        timestamp: Instant::now(),
    };

    assert_eq!(event.path, path);
    assert_eq!(event.asset_type, AssetType::Texture);
    assert!(matches!(event.change_type, ChangeType::Modified));
}

#[test]
fn test_change_type_variants() {
    let created = ChangeType::Created;
    let modified = ChangeType::Modified;
    let deleted = ChangeType::Deleted;
    let renamed = ChangeType::Renamed(PathBuf::from("old.png"));

    assert!(matches!(created, ChangeType::Created));
    assert!(matches!(modified, ChangeType::Modified));
    assert!(matches!(deleted, ChangeType::Deleted));
    assert!(matches!(renamed, ChangeType::Renamed(_)));

    // Test that renamed contains the old path
    if let ChangeType::Renamed(old_path) = renamed {
        assert_eq!(old_path, PathBuf::from("old.png"));
    }
}

#[test]
fn test_reload_result_display() {
    assert_eq!(ReloadResult::Success.to_string(), "Success");
    assert_eq!(
        ReloadResult::Failed("error message".to_string()).to_string(),
        "Failed: error message"
    );
    assert_eq!(ReloadResult::Skipped.to_string(), "Skipped");
}

#[test]
fn test_reload_result_clone() {
    let success = ReloadResult::Success;
    let failed = ReloadResult::Failed("error".to_string());
    let skipped = ReloadResult::Skipped;

    assert_eq!(success.clone().to_string(), success.to_string());
    assert_eq!(failed.clone().to_string(), failed.to_string());
    assert_eq!(skipped.clone().to_string(), skipped.to_string());
}

#[test]
fn test_reload_result_debug() {
    let success = ReloadResult::Success;
    let debug_str = format!("{:?}", success);
    assert!(debug_str.contains("Success"));
}

#[test]
fn test_asset_type_equality() {
    // Test that AssetType can be compared for equality
    let texture1 = AssetType::Texture;
    let texture2 = AssetType::Texture;
    let audio = AssetType::Audio;

    assert_eq!(texture1, texture2);
    assert_ne!(texture1, audio);
}

#[test]
fn test_asset_type_clone_copy() {
    // AssetType should be Copy
    let texture = AssetType::Texture;
    let copied = texture;
    assert_eq!(texture, copied);

    // AssetType should be Clone
    let cloned = texture.clone();
    assert_eq!(texture, cloned);
}

#[test]
fn test_asset_type_debug() {
    let texture = AssetType::Texture;
    let debug_str = format!("{:?}", texture);
    assert!(debug_str.contains("Texture"));
}

#[test]
fn test_mock_asset_handler_can_handle() {
    let handler = MockAssetHandler::always_success(AssetType::Texture);

    assert!(handler.can_handle(AssetType::Texture));
    assert!(!handler.can_handle(AssetType::Audio));
    assert!(!handler.can_handle(AssetType::LuaScript));
}

#[test]
fn test_mock_asset_handler_reload() {
    let handler = MockAssetHandler::always_success(AssetType::Texture);
    let path = Path::new("test.png");

    let result = handler.reload(path);
    assert!(matches!(result, ReloadResult::Success));
}

#[test]
fn test_mock_asset_handler_validate() {
    let handler = MockAssetHandler::always_success(AssetType::Texture);
    let path = Path::new("test.png");

    let result = handler.validate(path);
    assert!(result.is_ok());
}

#[test]
fn test_mock_asset_handler_fail() {
    let handler = MockAssetHandler::always_fail(AssetType::Texture, "test failure".to_string());
    let path = Path::new("test.png");

    let result = handler.reload(path);
    assert!(matches!(result, ReloadResult::Failed(_)));

    let validate_result = handler.validate(path);
    assert!(validate_result.is_err());
}

#[test]
fn test_custom_mock_handler() {
    let reloaded = Arc::new(Mutex::new(false));
    let reloaded_clone = Arc::clone(&reloaded);

    let handler = MockAssetHandler::new(
        |t| matches!(t, AssetType::LuaScript),
        move |path| {
            if path.extension().unwrap() == "lua" {
                *reloaded_clone.lock().unwrap() = true;
                ReloadResult::Success
            } else {
                ReloadResult::Failed("Not a Lua file".to_string())
            }
        },
        |_| Ok(()),
    );

    assert!(handler.can_handle(AssetType::LuaScript));
    assert!(!handler.can_handle(AssetType::Texture));

    let result = handler.reload(Path::new("script.lua"));
    assert!(matches!(result, ReloadResult::Success));
    assert!(*reloaded.lock().unwrap());
}

#[test]
fn test_hot_reload_error_types() {
    // Test that error types exist and can be created
    let notify_error = HotReloadError::Notify(notify::Error::new(notify::ErrorKind::Generic(
        "test".into(),
    )));

    assert!(matches!(notify_error, HotReloadError::Notify(_)));
}

#[test]
fn test_change_type_clone() {
    let created = ChangeType::Created;
    let cloned = created.clone();
    assert!(matches!(cloned, ChangeType::Created));

    let renamed = ChangeType::Renamed(PathBuf::from("old.txt"));
    let renamed_clone = renamed.clone();
    if let ChangeType::Renamed(path) = renamed_clone {
        assert_eq!(path, PathBuf::from("old.txt"));
    }
}

#[test]
fn test_change_type_debug() {
    let created = ChangeType::Created;
    let debug_str = format!("{:?}", created);
    assert!(debug_str.contains("Created"));
}

#[test]
fn test_asset_change_event_clone() {
    let event = AssetChangeEvent {
        path: PathBuf::from("test.png"),
        asset_type: AssetType::Texture,
        change_type: ChangeType::Modified,
        timestamp: Instant::now(),
    };

    let cloned = event.clone();
    assert_eq!(event.path, cloned.path);
    assert_eq!(event.asset_type, cloned.asset_type);
}

#[test]
fn test_asset_change_event_debug() {
    let event = AssetChangeEvent {
        path: PathBuf::from("test.png"),
        asset_type: AssetType::Texture,
        change_type: ChangeType::Modified,
        timestamp: Instant::now(),
    };

    let debug_str = format!("{:?}", event);
    assert!(debug_str.contains("AssetChangeEvent"));
    assert!(debug_str.contains("Texture"));
}
