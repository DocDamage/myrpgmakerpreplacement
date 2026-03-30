//! Export Manifest
//!
//! Tracks exported assets and provides import hints.

use crate::{ExportTarget, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Export manifest tracking all exported assets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportManifest {
    pub version: String,
    pub project_name: String,
    #[serde(rename = "target")]
    pub target: ExportTarget,
    #[serde(rename = "exported_at")]
    pub exported_at: String,
    pub assets: HashMap<String, PathBuf>,
    pub hints: HashMap<String, String>,
}

impl ExportManifest {
    pub fn new(project_name: &str, target: ExportTarget) -> Self {
        Self {
            version: "1.0".to_string(),
            project_name: project_name.to_string(),
            target,
            exported_at: chrono::Utc::now().to_rfc3339(),
            assets: HashMap::new(),
            hints: HashMap::new(),
        }
    }

    pub fn add_asset(&mut self, key: &str, path: PathBuf) {
        self.assets.insert(key.to_string(), path);
    }

    pub fn add_hint(&mut self, key: &str, value: &str) {
        self.hints.insert(key.to_string(), value.to_string());
    }

    pub fn write_to_file(&self, path: &Path) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn read_from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let manifest = serde_json::from_str(&content)?;
        Ok(manifest)
    }
}

impl Default for ExportManifest {
    fn default() -> Self {
        Self::new("Untitled", ExportTarget::MzAssets)
    }
}
