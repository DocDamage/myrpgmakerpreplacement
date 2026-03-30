//! Sprite Generator Adapter
//!
//! Implements the trait-based adapter pattern for communication between
//! DDE and the Asset Forge (sprite generator). Uses postMessage IPC.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Actions that can be dispatched from DDE to the Asset Forge
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum ForgeAction {
    /// Initialize the Forge with project context
    Initialize {
        project_id: String,
        project_name: String,
    },

    /// Generate a new character
    GenerateCharacter {
        prompt: String,
        style_profile: Option<String>,
        provider: Option<String>,
    },

    /// Generate a sprite sheet from a hero image
    GenerateSheet {
        hero_asset_id: String,
        animation_type: String,
        direction_count: u32,
    },

    /// Generate a portrait from a hero image
    GeneratePortrait {
        hero_asset_id: String,
        expression: String,
    },

    /// Request background removal
    RemoveBackground { asset_id: String },

    /// Extract frames from an animation
    ExtractFrames { asset_id: String, frame_count: u32 },

    /// Set the active style profile
    SetStyleProfile { profile_id: String },

    /// Load an asset into the Forge for editing
    LoadAsset { asset_id: String },

    /// Export the current workspace
    ExportWorkspace,

    /// Request QA check
    RunQa { asset_id: String },

    /// Get generation cost estimate
    EstimateCost {
        cost_action: String,
        params: serde_json::Value,
    },

    /// Update settings
    UpdateSettings {
        settings: HashMap<String, serde_json::Value>,
    },

    /// Ping for connectivity
    Ping,
}

/// State snapshot from the Asset Forge
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ForgeState {
    /// Current step in the generation workflow (1-6)
    pub current_step: u32,

    /// Completed steps
    pub completed_steps: Vec<u32>,

    /// Currently selected provider
    pub active_provider: String,

    /// Current style profile
    pub style_profile: String,

    /// Assets in the current workspace
    pub workspace_assets: Vec<WorkspaceAsset>,

    /// Generation queue
    pub generation_queue: Vec<GenerationJob>,

    /// Total cost this session (in cents)
    pub session_cost_cents: u32,

    /// Error state if any
    pub error: Option<String>,

    /// Whether the Forge is ready
    pub is_ready: bool,
}

/// Asset in the Forge workspace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceAsset {
    pub id: String,
    pub name: String,
    pub asset_type: String,
    pub preview_url: String,
    pub status: String,
    pub metadata: serde_json::Value,
}

/// Generation job in queue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationJob {
    pub job_id: String,
    pub job_type: String,
    pub status: String,
    pub progress_percent: u32,
    pub estimated_cost_cents: u32,
}

/// Sprite Generator Adapter trait
pub trait SpriteGeneratorAdapter: Send + Sync {
    /// Send an action to the Asset Forge
    fn dispatch(&self, action: ForgeAction) -> crate::Result<()>;

    /// Get a readonly snapshot of the Forge state
    fn get_state(&self) -> crate::Result<ForgeState>;

    /// Register a callback for state changes
    fn on_state_change<F>(&self, callback: F) -> crate::Result<()>
    where
        F: Fn(ForgeState) + Send + Sync + 'static;

    /// Register a callback for asset exports
    fn on_asset_export<F>(&self, callback: F) -> crate::Result<()>
    where
        F: Fn(AssetExportEvent) + Send + Sync + 'static;

    /// Check if the adapter is connected
    fn is_connected(&self) -> bool;
}

/// Asset export event from Forge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetExportEvent {
    pub asset_id: String,
    pub asset_type: String,
    pub file_path: String,
    pub file_hash: String,
    pub metadata: serde_json::Value,
    pub provenance: ProvenanceData,
}

/// Provenance data for generated assets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceData {
    pub source_type: String,
    pub generation_prompt: Option<String>,
    pub generation_model: Option<String>,
    pub generation_provider: Option<String>,
    pub generation_seed: Option<u64>,
    pub generation_cost_cents: Option<u32>,
    pub parent_asset_id: Option<String>,
    pub derivation_type: Option<String>,
    pub style_profile_id: Option<String>,
}
