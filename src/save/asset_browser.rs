//! Asset Browser
//!
//! Browse and manage game assets: tilesets, sprites, audio, scripts, etc.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Asset types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AssetType {
    Tileset,
    Sprite,
    Audio,
    Script,
    Map,
    Dialogue,
    Vibecode,
    Font,
    Shader,
    Unknown,
}

impl AssetType {
    /// Get display name
    pub fn name(&self) -> &'static str {
        match self {
            AssetType::Tileset => "Tileset",
            AssetType::Sprite => "Sprite",
            AssetType::Audio => "Audio",
            AssetType::Script => "Script",
            AssetType::Map => "Map",
            AssetType::Dialogue => "Dialogue",
            AssetType::Vibecode => "Vibecode",
            AssetType::Font => "Font",
            AssetType::Shader => "Shader",
            AssetType::Unknown => "Unknown",
        }
    }

    /// Get file extensions for this asset type
    pub fn extensions(&self) -> &'static [&'static str] {
        match self {
            AssetType::Tileset => &["png", "jpg", "jpeg"],
            AssetType::Sprite => &["png", "jpg", "jpeg"],
            AssetType::Audio => &["ogg", "mp3", "wav"],
            AssetType::Script => &["lua", "js"],
            AssetType::Map => &["json", "tmx"],
            AssetType::Dialogue => &["json"],
            AssetType::Vibecode => &["toml"],
            AssetType::Font => &["ttf", "otf"],
            AssetType::Shader => &["wgsl", "glsl"],
            AssetType::Unknown => &[],
        }
    }

    /// Detect asset type from file extension
    pub fn from_extension(ext: &str) -> Self {
        let ext = ext.to_lowercase();
        for asset_type in [
            AssetType::Tileset,
            AssetType::Sprite,
            AssetType::Audio,
            AssetType::Script,
            AssetType::Map,
            AssetType::Dialogue,
            AssetType::Vibecode,
            AssetType::Font,
            AssetType::Shader,
        ] {
            if asset_type.extensions().contains(&ext.as_str()) {
                return asset_type;
            }
        }
        AssetType::Unknown
    }

    /// Get all asset types
    #[allow(dead_code)]
    pub fn all() -> [AssetType; 9] {
        [
            AssetType::Tileset,
            AssetType::Sprite,
            AssetType::Audio,
            AssetType::Script,
            AssetType::Map,
            AssetType::Dialogue,
            AssetType::Vibecode,
            AssetType::Font,
            AssetType::Shader,
        ]
    }
}

/// Asset metadata
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct AssetInfo {
    /// Asset ID (UUID or filename hash)
    pub id: String,
    /// Asset name
    pub name: String,
    /// Asset type
    pub asset_type: AssetType,
    /// File path
    pub path: PathBuf,
    /// File size in bytes
    pub size: u64,
    /// Last modified timestamp
    pub modified: i64,
    /// Tags
    pub tags: Vec<String>,
    /// Custom metadata
    pub metadata: HashMap<String, String>,
    /// Preview thumbnail (optional)
    pub has_thumbnail: bool,
}

impl AssetInfo {
    /// Format file size for display
    pub fn format_size(&self) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
        let mut size = self.size as f64;
        let mut unit_index = 0;

        while size >= 1024.0 && unit_index < UNITS.len() - 1 {
            size /= 1024.0;
            unit_index += 1;
        }

        format!("{:.1} {}", size, UNITS[unit_index])
    }

    /// Get filename
    #[allow(dead_code)]
    pub fn filename(&self) -> String {
        self.path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string()
    }
}

/// Asset browser
pub struct AssetBrowser {
    /// Asset directories
    asset_dirs: Vec<PathBuf>,
    /// Cached asset list
    assets: Vec<AssetInfo>,
    /// Filter by type
    filter_type: Option<AssetType>,
    /// Search query
    pub search_query: String,
    /// Selected asset
    pub selected_asset: Option<String>,
    /// Last refresh time
    last_refresh: std::time::Instant,
    /// Whether the browser is visible
    pub visible: bool,
}

impl Default for AssetBrowser {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
impl AssetBrowser {
    /// Create a new asset browser
    pub fn new() -> Self {
        Self {
            asset_dirs: Vec::new(),
            assets: Vec::new(),
            filter_type: None,
            search_query: String::new(),
            selected_asset: None,
            last_refresh: std::time::Instant::now(),
            visible: false,
        }
    }

    /// Add an asset directory
    pub fn add_directory(&mut self, path: impl Into<PathBuf>) {
        self.asset_dirs.push(path.into());
        self.refresh();
    }

    /// Remove an asset directory
    pub fn remove_directory(&mut self, path: &Path) {
        self.asset_dirs.retain(|p| p != path);
        self.refresh();
    }

    /// Set type filter
    pub fn set_filter(&mut self, asset_type: Option<AssetType>) {
        self.filter_type = asset_type;
    }

    /// Set search query
    pub fn set_search(&mut self, query: impl Into<String>) {
        self.search_query = query.into();
    }

    /// Refresh the asset list
    pub fn refresh(&mut self) {
        self.assets.clear();

        let dirs: Vec<PathBuf> = self.asset_dirs.clone();
        for dir in &dirs {
            self.scan_directory(dir, dir);
        }

        self.assets.sort_by(|a, b| a.name.cmp(&b.name));
        self.last_refresh = std::time::Instant::now();
    }

    /// Scan a directory for assets
    fn scan_directory(&mut self, _base_path: &Path, current_path: &Path) {
        let Ok(entries) = std::fs::read_dir(current_path) else {
            return;
        };

        for entry in entries.flatten() {
            let path = entry.path();

            if path.is_dir() {
                // Recurse into subdirectories
                self.scan_directory(_base_path, &path);
            } else {
                // Check if this is an asset file
                if let Some(info) = self.file_to_asset(&path) {
                    self.assets.push(info);
                }
            }
        }
    }

    /// Convert a file path to asset info
    fn file_to_asset(&self, path: &Path) -> Option<AssetInfo> {
        let ext = path.extension()?.to_str()?;
        let asset_type = AssetType::from_extension(ext);

        if asset_type == AssetType::Unknown {
            return None;
        }

        let metadata = std::fs::metadata(path).ok()?;
        let name = path.file_stem()?.to_str()?.to_string();

        let id = format!("{}_{}", asset_type.name(), name);

        Some(AssetInfo {
            id,
            name,
            asset_type,
            path: path.to_path_buf(),
            size: metadata.len(),
            modified: metadata
                .modified()
                .ok()?
                .duration_since(std::time::UNIX_EPOCH)
                .ok()?
                .as_secs() as i64,
            tags: Vec::new(),
            metadata: HashMap::new(),
            has_thumbnail: false, // TODO: Generate thumbnails
        })
    }

    /// Get filtered assets
    pub fn filtered_assets(&self) -> Vec<&AssetInfo> {
        self.assets
            .iter()
            .filter(|asset| {
                // Type filter
                if let Some(filter) = self.filter_type {
                    if asset.asset_type != filter {
                        return false;
                    }
                }

                // Search filter
                if !self.search_query.is_empty() {
                    let query = self.search_query.to_lowercase();
                    if !asset.name.to_lowercase().contains(&query) {
                        return false;
                    }
                }

                true
            })
            .collect()
    }

    /// Get asset by ID
    pub fn get_asset(&self, id: &str) -> Option<&AssetInfo> {
        self.assets.iter().find(|a| a.id == id)
    }

    /// Get all assets of a specific type
    pub fn get_assets_by_type(&self, asset_type: AssetType) -> Vec<&AssetInfo> {
        self.assets
            .iter()
            .filter(|a| a.asset_type == asset_type)
            .collect()
    }

    /// Show the browser
    pub fn show(&mut self) {
        self.visible = true;
        // Refresh if it's been a while
        if self.last_refresh.elapsed().as_secs() > 30 {
            self.refresh();
        }
    }

    /// Hide the browser
    pub fn hide(&mut self) {
        self.visible = false;
    }

    /// Toggle visibility
    pub fn toggle(&mut self) {
        if self.visible {
            self.hide();
        } else {
            self.show();
        }
    }

    /// Check if browser is visible
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Draw the asset browser UI (stub - egui integration needed)
    pub fn draw(&mut self, _ctx: &egui::Context) -> Option<AssetAction> {
        // TODO: Implement when egui renderer integration is available
        None
    }

    /// Draw the browser UI (stub)
    fn draw_browser_ui(&mut self, _ui: &mut egui::Ui) -> Option<AssetAction> {
        // TODO: Implement when egui renderer integration is available
        None
    }

    /// Create default asset directories
    pub fn create_default_directories(&self, base_path: &Path) -> std::io::Result<()> {
        let dirs = [
            "tilesets",
            "sprites",
            "audio/bgm",
            "audio/sfx",
            "scripts",
            "maps",
            "dialogue",
            "vibecode",
            "fonts",
            "shaders",
        ];

        for dir in &dirs {
            std::fs::create_dir_all(base_path.join(dir))?;
        }

        Ok(())
    }
}

/// Asset actions from the browser
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum AssetAction {
    Open(String),
    Properties(String),
    Delete(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_asset_type_from_extension() {
        assert_eq!(AssetType::from_extension("png"), AssetType::Tileset);
        assert_eq!(AssetType::from_extension("lua"), AssetType::Script);
        assert_eq!(AssetType::from_extension("ogg"), AssetType::Audio);
        assert_eq!(AssetType::from_extension("unknown"), AssetType::Unknown);
    }

    #[test]
    fn test_asset_info_format_size() {
        let asset = AssetInfo {
            id: "test".to_string(),
            name: "Test".to_string(),
            asset_type: AssetType::Sprite,
            path: PathBuf::from("test.png"),
            size: 1536, // 1.5 KB
            modified: 0,
            tags: vec![],
            metadata: HashMap::new(),
            has_thumbnail: false,
        };

        assert_eq!(asset.format_size(), "1.5 KB");
    }

    #[test]
    fn test_asset_browser_filter() {
        let mut browser = AssetBrowser::new();

        // Manually add some test assets
        browser.assets.push(AssetInfo {
            id: "sprite_1".to_string(),
            name: "Hero".to_string(),
            asset_type: AssetType::Sprite,
            path: PathBuf::from("hero.png"),
            size: 1000,
            modified: 0,
            tags: vec![],
            metadata: HashMap::new(),
            has_thumbnail: false,
        });

        browser.assets.push(AssetInfo {
            id: "sprite_2".to_string(),
            name: "Enemy".to_string(),
            asset_type: AssetType::Sprite,
            path: PathBuf::from("enemy.png"),
            size: 1000,
            modified: 0,
            tags: vec![],
            metadata: HashMap::new(),
            has_thumbnail: false,
        });

        browser.assets.push(AssetInfo {
            id: "audio_1".to_string(),
            name: "Battle".to_string(),
            asset_type: AssetType::Audio,
            path: PathBuf::from("battle.ogg"),
            size: 1000,
            modified: 0,
            tags: vec![],
            metadata: HashMap::new(),
            has_thumbnail: false,
        });

        // Test filtering
        browser.set_filter(Some(AssetType::Sprite));
        let sprites = browser.filtered_assets();
        assert_eq!(sprites.len(), 2);

        // Test search
        browser.set_filter(None);
        browser.set_search("Hero");
        let search_results = browser.filtered_assets();
        assert_eq!(search_results.len(), 1);
        assert_eq!(search_results[0].name, "Hero");
    }
}
