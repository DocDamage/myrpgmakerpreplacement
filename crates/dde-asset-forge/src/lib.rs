//! DocDamage Engine - Asset Forge Integration
//!
//! Embeds the Sprite Generator (Next.js app) into DDE via webview,
//! provides the SpriteGeneratorAdapter for IPC, and implements the
//! Asset OS (inbox, staging, review queue, production library).

pub mod adapter;
pub mod asset_os;
pub mod classification;
pub mod duplicate_detection;

#[cfg(feature = "embedded-server")]
pub mod embedded_server;

#[cfg(feature = "webview")]
pub mod webview;

pub use adapter::{ForgeAction, ForgeState, SpriteGeneratorAdapter};
pub use asset_os::{AssetOs, AssetPipelineStage, AssetReview};
pub use classification::{AssetClassifier, ClassificationResult, ClassificationRule};
pub use duplicate_detection::{DuplicateDetector, HashType};

use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::RwLock;

/// Asset Forge configuration
#[derive(Debug, Clone)]
pub struct AssetForgeConfig {
    /// Path to the sprite_generator Next.js app
    pub sprite_generator_path: PathBuf,
    /// Port for the embedded server (0 = auto-assign)
    pub server_port: u16,
    /// Whether to use an external server instead of embedded
    pub use_external_server: bool,
    /// External server URL (if use_external_server is true)
    pub external_server_url: String,
    /// Asset inbox directory
    pub inbox_path: PathBuf,
    /// Asset staging directory
    pub staging_path: PathBuf,
    /// Asset production directory
    pub production_path: PathBuf,
    /// Enable devtools in webview
    pub enable_devtools: bool,
}

impl Default for AssetForgeConfig {
    fn default() -> Self {
        Self {
            sprite_generator_path: PathBuf::from("sprite_generator"),
            server_port: 0,
            use_external_server: false,
            external_server_url: "http://localhost:3000".to_string(),
            inbox_path: PathBuf::from("assets/inbox"),
            staging_path: PathBuf::from("assets/staging"),
            production_path: PathBuf::from("assets/production"),
            enable_devtools: cfg!(debug_assertions),
        }
    }
}

/// Asset Forge handle - manages the webview and server
pub struct AssetForge {
    config: AssetForgeConfig,
    state: Arc<RwLock<AssetForgeState>>,
    asset_os: AssetOs,
}

#[derive(Debug, Default)]
struct AssetForgeState {
    server_url: Option<String>,
    server_process: Option<std::process::Child>,
}

/// Error types for Asset Forge
#[derive(thiserror::Error, Debug)]
pub enum AssetForgeError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Database error: {0}")]
    Database(#[from] dde_db::DbError),

    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("Webview error: {0}")]
    Webview(String),

    #[error("Server error: {0}")]
    Server(String),

    #[error("IPC error: {0}")]
    Ipc(String),

    #[error("Asset not found: {0}")]
    AssetNotFound(String),

    #[error("Classification error: {0}")]
    Classification(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Base64 decode error: {0}")]
    Base64Decode(String),
}

pub type Result<T> = std::result::Result<T, AssetForgeError>;

impl AssetForge {
    /// Create a new Asset Forge instance
    pub async fn new(config: AssetForgeConfig, database: dde_db::Database) -> Result<Self> {
        // Ensure directories exist
        tokio::fs::create_dir_all(&config.inbox_path).await?;
        tokio::fs::create_dir_all(&config.staging_path).await?;
        tokio::fs::create_dir_all(&config.production_path).await?;

        let asset_os = AssetOs::new(database);

        Ok(Self {
            config,
            state: Arc::new(RwLock::new(AssetForgeState::default())),
            asset_os,
        })
    }

    /// Start the Asset Forge server and open webview
    pub async fn open(&self) -> Result<()> {
        // Start embedded server or use external
        let server_url = if self.config.use_external_server {
            self.config.external_server_url.clone()
        } else {
            self.start_embedded_server().await?
        };

        {
            let mut state = self.state.write().await;
            state.server_url = Some(server_url.clone());
        }

        // Open webview (platform-specific)
        tracing::info!("Opening Asset Forge at {}", server_url);

        #[cfg(feature = "webview")]
        {
            self.open_webview(&server_url).await?;
        }

        Ok(())
    }

    /// Start the embedded Asset Forge server
    async fn start_embedded_server(&self) -> Result<String> {
        // Build and serve the sprite_generator Next.js app
        // For now, we assume it's already built
        let dist_path = self.config.sprite_generator_path.join(".next");

        if !dist_path.exists() {
            return Err(AssetForgeError::Server(format!(
                "Asset Forge not built. Run 'npm run build' in {:?}",
                self.config.sprite_generator_path
            )));
        }

        #[cfg(feature = "embedded-server")]
        {
            let port = embedded_server::start_server(dist_path, self.config.server_port).await?;

            Ok(format!("http://localhost:{}", port))
        }

        #[cfg(not(feature = "embedded-server"))]
        {
            Err(AssetForgeError::Server(
                "embedded-server feature not enabled".to_string(),
            ))
        }
    }

    /// Open the webview window
    #[cfg(feature = "webview")]
    async fn open_webview(&self, url: &str) -> Result<()> {
        webview::open_webview(url, self.config.enable_devtools).await
    }

    /// Get the Asset OS for pipeline operations
    pub fn asset_os(&self) -> &AssetOs {
        &self.asset_os
    }

    /// Close the Asset Forge
    pub async fn close(&self) -> Result<()> {
        let mut state = self.state.write().await;

        if let Some(mut child) = state.server_process.take() {
            let _ = child.kill();
        }

        state.server_url = None;

        tracing::info!("Asset Forge closed");
        Ok(())
    }
}

/// IPC Message format for JS <-> Rust communication
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum IpcMessage {
    /// Ping/pong for connectivity check
    Ping,
    Pong,

    /// Asset export from Forge to DDE
    AssetExport {
        asset_data: AssetExportData,
    },

    /// Asset approval from DDE to Forge
    AssetApproved {
        asset_id: String,
    },

    /// Asset rejection from DDE to Forge
    AssetRejected {
        asset_id: String,
        reason: String,
    },

    /// Request asset from DDE library
    RequestAsset {
        asset_id: String,
    },

    /// Asset data response
    AssetData {
        asset: Option<AssetExportData>,
    },

    /// Sync state between Forge and DDE
    SyncState {
        forge_state: ForgeState,
    },

    /// Error notification
    Error {
        message: String,
    },
}

/// Asset export data format
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AssetExportData {
    pub id: String,
    pub name: String,
    pub asset_type: String,
    pub file_data_base64: String,
    pub file_format: String,
    pub width: u32,
    pub height: u32,
    pub metadata: serde_json::Value,
    pub provenance: serde_json::Value,
}

impl AssetExportData {
    /// Decode base64 file data
    pub fn decode_data(&self) -> Result<Vec<u8>> {
        use base64::{engine::general_purpose::STANDARD, Engine as _};
        STANDARD
            .decode(&self.file_data_base64)
            .map_err(|e| AssetForgeError::Base64Decode(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // AssetForgeConfig Tests
    // =========================================================================

    #[test]
    fn test_asset_forge_config_default() {
        let config = AssetForgeConfig::default();

        // Test paths
        assert_eq!(
            config.sprite_generator_path,
            PathBuf::from("sprite_generator")
        );
        assert_eq!(config.inbox_path, PathBuf::from("assets/inbox"));
        assert_eq!(config.staging_path, PathBuf::from("assets/staging"));
        assert_eq!(config.production_path, PathBuf::from("assets/production"));

        // Test ports
        assert_eq!(config.server_port, 0);

        // Test flags
        assert_eq!(config.use_external_server, false);
        assert_eq!(config.enable_devtools, cfg!(debug_assertions));

        // Test external URL
        assert_eq!(config.external_server_url, "http://localhost:3000");
    }

    #[test]
    fn test_asset_forge_config_custom() {
        let config = AssetForgeConfig {
            sprite_generator_path: PathBuf::from("/custom/sprite_gen"),
            server_port: 8080,
            use_external_server: true,
            external_server_url: "http://192.168.1.100:3000".to_string(),
            inbox_path: PathBuf::from("/custom/inbox"),
            staging_path: PathBuf::from("/custom/staging"),
            production_path: PathBuf::from("/custom/production"),
            enable_devtools: true,
        };

        assert_eq!(
            config.sprite_generator_path,
            PathBuf::from("/custom/sprite_gen")
        );
        assert_eq!(config.server_port, 8080);
        assert_eq!(config.use_external_server, true);
        assert_eq!(config.external_server_url, "http://192.168.1.100:3000");
        assert_eq!(config.inbox_path, PathBuf::from("/custom/inbox"));
        assert_eq!(config.staging_path, PathBuf::from("/custom/staging"));
        assert_eq!(config.production_path, PathBuf::from("/custom/production"));
        assert_eq!(config.enable_devtools, true);
    }

    // =========================================================================
    // IpcMessage Tests
    // =========================================================================

    #[test]
    fn test_ipc_message_ping_serialization() {
        let msg = IpcMessage::Ping;
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("ping"));

        // Test deserialization
        let decoded: IpcMessage = serde_json::from_str(&json).unwrap();
        assert!(matches!(decoded, IpcMessage::Ping));
    }

    #[test]
    fn test_ipc_message_pong_serialization() {
        let msg = IpcMessage::Pong;
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("pong"));

        let decoded: IpcMessage = serde_json::from_str(&json).unwrap();
        assert!(matches!(decoded, IpcMessage::Pong));
    }

    #[test]
    fn test_ipc_message_asset_export_serialization() {
        let asset_data = AssetExportData {
            id: "asset-123".to_string(),
            name: "Test Asset".to_string(),
            asset_type: "character".to_string(),
            file_data_base64: "dGVzdCBkYXRh".to_string(),
            file_format: "png".to_string(),
            width: 256,
            height: 256,
            metadata: serde_json::json!({"key": "value"}),
            provenance: serde_json::json!({"source": "test"}),
        };

        let msg = IpcMessage::AssetExport { asset_data };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("asset_export"));
        assert!(json.contains("asset-123"));
        assert!(json.contains("Test Asset"));
    }

    #[test]
    fn test_ipc_message_asset_approved_serialization() {
        let msg = IpcMessage::AssetApproved {
            asset_id: "asset-456".to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("asset_approved"));
        assert!(json.contains("asset-456"));

        let decoded: IpcMessage = serde_json::from_str(&json).unwrap();
        match decoded {
            IpcMessage::AssetApproved { asset_id } => assert_eq!(asset_id, "asset-456"),
            _ => panic!("Expected AssetApproved variant"),
        }
    }

    #[test]
    fn test_ipc_message_asset_rejected_serialization() {
        let msg = IpcMessage::AssetRejected {
            asset_id: "asset-789".to_string(),
            reason: "Invalid format".to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("asset_rejected"));
        assert!(json.contains("asset-789"));
        assert!(json.contains("Invalid format"));
    }

    #[test]
    fn test_ipc_message_request_asset_serialization() {
        let msg = IpcMessage::RequestAsset {
            asset_id: "asset-000".to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("request_asset"));
        assert!(json.contains("asset-000"));
    }

    #[test]
    fn test_ipc_message_asset_data_serialization() {
        let asset_data = AssetExportData {
            id: "asset-111".to_string(),
            name: "Response Asset".to_string(),
            asset_type: "portrait".to_string(),
            file_data_base64: "cmVzcG9uc2U=".to_string(),
            file_format: "jpg".to_string(),
            width: 128,
            height: 128,
            metadata: serde_json::Value::Null,
            provenance: serde_json::Value::Null,
        };

        let msg = IpcMessage::AssetData {
            asset: Some(asset_data),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("asset_data"));
        assert!(json.contains("asset-111"));
    }

    #[test]
    fn test_ipc_message_asset_data_none_serialization() {
        let msg = IpcMessage::AssetData { asset: None };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("asset_data"));
        assert!(json.contains("null"));
    }

    #[test]
    fn test_ipc_message_sync_state_serialization() {
        let forge_state = ForgeState::default();
        let msg = IpcMessage::SyncState { forge_state };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("sync_state"));
        assert!(json.contains("current_step"));
    }

    #[test]
    fn test_ipc_message_error_serialization() {
        let msg = IpcMessage::Error {
            message: "Something went wrong".to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("error"));
        assert!(json.contains("Something went wrong"));

        let decoded: IpcMessage = serde_json::from_str(&json).unwrap();
        match decoded {
            IpcMessage::Error { message } => assert_eq!(message, "Something went wrong"),
            _ => panic!("Expected Error variant"),
        }
    }

    // =========================================================================
    // AssetExportData Tests
    // =========================================================================

    #[test]
    fn test_asset_export_data_creation() {
        let data = AssetExportData {
            id: "test-id".to_string(),
            name: "Test Name".to_string(),
            asset_type: "character".to_string(),
            file_data_base64: "SGVsbG8gV29ybGQ=".to_string(),
            file_format: "png".to_string(),
            width: 256,
            height: 512,
            metadata: serde_json::json!({"author": "tester"}),
            provenance: serde_json::json!({"source": "generator"}),
        };

        assert_eq!(data.id, "test-id");
        assert_eq!(data.name, "Test Name");
        assert_eq!(data.asset_type, "character");
        assert_eq!(data.file_format, "png");
        assert_eq!(data.width, 256);
        assert_eq!(data.height, 512);
    }

    #[test]
    fn test_asset_export_data_decode_valid_base64() {
        let data = AssetExportData {
            id: "test".to_string(),
            name: "Test".to_string(),
            asset_type: "sprite".to_string(),
            file_data_base64: "SGVsbG8gV29ybGQ=".to_string(), // "Hello World"
            file_format: "png".to_string(),
            width: 100,
            height: 100,
            metadata: serde_json::Value::Null,
            provenance: serde_json::Value::Null,
        };

        let decoded = data.decode_data().unwrap();
        assert_eq!(decoded, b"Hello World");
    }

    #[test]
    fn test_asset_export_data_decode_empty_base64() {
        let data = AssetExportData {
            id: "test".to_string(),
            name: "Test".to_string(),
            asset_type: "sprite".to_string(),
            file_data_base64: "".to_string(),
            file_format: "png".to_string(),
            width: 100,
            height: 100,
            metadata: serde_json::Value::Null,
            provenance: serde_json::Value::Null,
        };

        let decoded = data.decode_data().unwrap();
        assert!(decoded.is_empty());
    }

    #[test]
    fn test_asset_export_data_decode_invalid_base64() {
        let data = AssetExportData {
            id: "test".to_string(),
            name: "Test".to_string(),
            asset_type: "sprite".to_string(),
            file_data_base64: "!!!invalid!!!base64!!!".to_string(),
            file_format: "png".to_string(),
            width: 100,
            height: 100,
            metadata: serde_json::Value::Null,
            provenance: serde_json::Value::Null,
        };

        let result = data.decode_data();
        assert!(result.is_err());

        match result {
            Err(AssetForgeError::Base64Decode(_)) => (), // Expected
            _ => panic!("Expected Base64Decode error"),
        }
    }

    #[test]
    fn test_asset_export_data_decode_binary_data() {
        // Test with PNG file header bytes (base64 encoded)
        // PNG magic bytes: 0x89 0x50 0x4E 0x47 0x0D 0x0A 0x1A 0x0A
        let data = AssetExportData {
            id: "test".to_string(),
            name: "Test".to_string(),
            asset_type: "sprite".to_string(),
            file_data_base64: "iVBORw0KGgo=".to_string(), // PNG header
            file_format: "png".to_string(),
            width: 100,
            height: 100,
            metadata: serde_json::Value::Null,
            provenance: serde_json::Value::Null,
        };

        let decoded = data.decode_data().unwrap();
        assert_eq!(
            &decoded[..8],
            &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]
        );
    }

    // =========================================================================
    // ForgeState Tests
    // =========================================================================

    #[test]
    fn test_forge_state_default() {
        let state = ForgeState::default();

        assert_eq!(state.current_step, 0);
        assert!(state.completed_steps.is_empty());
        assert_eq!(state.active_provider, "");
        assert_eq!(state.style_profile, "");
        assert!(state.workspace_assets.is_empty());
        assert!(state.generation_queue.is_empty());
        assert_eq!(state.session_cost_cents, 0);
        assert_eq!(state.error, None);
        assert_eq!(state.is_ready, false);
    }

    #[test]
    fn test_forge_state_transitions() {
        let mut state = ForgeState::default();

        // Simulate workflow progression
        state.current_step = 1;
        state.is_ready = true;

        assert_eq!(state.current_step, 1);
        assert!(state.is_ready);

        // Mark step as complete
        state.completed_steps.push(1);
        assert_eq!(state.completed_steps, vec![1]);

        // Move to next step
        state.current_step = 2;
        assert_eq!(state.current_step, 2);

        // Add cost
        state.session_cost_cents = 50;
        assert_eq!(state.session_cost_cents, 50);

        // Set provider and style
        state.active_provider = "gemini".to_string();
        state.style_profile = "fantasy".to_string();
        assert_eq!(state.active_provider, "gemini");
        assert_eq!(state.style_profile, "fantasy");

        // Set error state
        state.error = Some("Connection lost".to_string());
        assert_eq!(state.error, Some("Connection lost".to_string()));

        // Clear error
        state.error = None;
        assert!(state.error.is_none());
    }

    #[test]
    fn test_forge_state_serialization() {
        let mut state = ForgeState::default();
        state.current_step = 3;
        state.active_provider = "openai".to_string();
        state.is_ready = true;

        let json = serde_json::to_string(&state).unwrap();
        assert!(json.contains("current_step"));
        assert!(json.contains("openai"));

        let deserialized: ForgeState = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.current_step, 3);
        assert_eq!(deserialized.active_provider, "openai");
        assert!(deserialized.is_ready);
    }

    // =========================================================================
    // AssetPipelineStage Tests
    // =========================================================================

    #[test]
    fn test_asset_pipeline_stage_variants() {
        let stages = vec![
            AssetPipelineStage::Inbox,
            AssetPipelineStage::Staging,
            AssetPipelineStage::Review,
            AssetPipelineStage::Approved,
            AssetPipelineStage::Rejected,
        ];

        // Ensure all variants are distinct
        for i in 0..stages.len() {
            for j in (i + 1)..stages.len() {
                assert_ne!(stages[i], stages[j]);
            }
        }
    }

    #[test]
    fn test_asset_pipeline_stage_display_name() {
        assert_eq!(AssetPipelineStage::Inbox.display_name(), "Inbox");
        assert_eq!(AssetPipelineStage::Staging.display_name(), "Staging");
        assert_eq!(AssetPipelineStage::Review.display_name(), "Review Queue");
        assert_eq!(AssetPipelineStage::Approved.display_name(), "Production");
        assert_eq!(AssetPipelineStage::Rejected.display_name(), "Rejected");
    }

    #[test]
    fn test_asset_pipeline_stage_as_str() {
        assert_eq!(AssetPipelineStage::Inbox.as_str(), "inbox");
        assert_eq!(AssetPipelineStage::Staging.as_str(), "staging");
        assert_eq!(AssetPipelineStage::Review.as_str(), "review");
        assert_eq!(AssetPipelineStage::Approved.as_str(), "approved");
        assert_eq!(AssetPipelineStage::Rejected.as_str(), "rejected");
    }

    #[test]
    fn test_asset_pipeline_stage_from_str_valid() {
        assert_eq!(
            "inbox".parse::<AssetPipelineStage>().unwrap(),
            AssetPipelineStage::Inbox
        );
        assert_eq!(
            "staging".parse::<AssetPipelineStage>().unwrap(),
            AssetPipelineStage::Staging
        );
        assert_eq!(
            "review".parse::<AssetPipelineStage>().unwrap(),
            AssetPipelineStage::Review
        );
        assert_eq!(
            "approved".parse::<AssetPipelineStage>().unwrap(),
            AssetPipelineStage::Approved
        );
        assert_eq!(
            "rejected".parse::<AssetPipelineStage>().unwrap(),
            AssetPipelineStage::Rejected
        );
    }

    #[test]
    fn test_asset_pipeline_stage_from_str_invalid() {
        let result = "invalid_stage".parse::<AssetPipelineStage>();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown stage"));
    }

    #[test]
    fn test_asset_pipeline_stage_serialization() {
        let stage = AssetPipelineStage::Staging;
        let json = serde_json::to_string(&stage).unwrap();
        assert_eq!(json, "\"staging\"");

        let deserialized: AssetPipelineStage = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, AssetPipelineStage::Staging);
    }

    #[test]
    fn test_asset_pipeline_stage_roundtrip() {
        let stages = vec![
            AssetPipelineStage::Inbox,
            AssetPipelineStage::Staging,
            AssetPipelineStage::Review,
            AssetPipelineStage::Approved,
            AssetPipelineStage::Rejected,
        ];

        for stage in stages {
            let json = serde_json::to_string(&stage).unwrap();
            let deserialized: AssetPipelineStage = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized, stage);

            let str_val = stage.as_str();
            let from_str: AssetPipelineStage = str_val.parse().unwrap();
            assert_eq!(from_str, stage);
        }
    }
}
