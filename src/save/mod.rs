//! Save/Load System
//!
//! Manages game saves with:
//! - 6 save slots (0-5)
//! - Autosave every 30 seconds
//! - Save/load UI
//! - Screenshot capture

pub mod asset_browser;

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use dde_core::{GameSave, WorldSerializer};

pub use asset_browser::AssetBrowser;

/// Number of save slots
pub const SAVE_SLOT_COUNT: usize = 6;

/// Autosave interval (30 seconds)
pub const AUTOSAVE_INTERVAL: Duration = Duration::from_secs(30);

/// Save manager
pub struct SaveManager {
    /// Save directory path
    save_dir: PathBuf,
    /// Current save slot (None = not saved yet)
    current_slot: Option<u32>,
    /// Last autosave time
    last_autosave: Instant,
    /// Total play time
    play_time: Duration,
    /// Whether autosave is enabled
    autosave_enabled: bool,
    /// Current map name
    current_map: String,
    /// Player name
    player_name: String,
}

#[allow(dead_code)]
impl SaveManager {
    /// Create a new save manager
    pub fn new(save_dir: impl Into<PathBuf>) -> Self {
        let save_dir = save_dir.into();
        std::fs::create_dir_all(&save_dir).ok();

        Self {
            save_dir,
            current_slot: None,
            last_autosave: Instant::now(),
            play_time: Duration::ZERO,
            autosave_enabled: true,
            current_map: "map_001".to_string(),
            player_name: "Player".to_string(),
        }
    }

    /// Create save manager in default location
    pub fn default() -> Self {
        let save_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("DocDamageEngine")
            .join("saves");

        Self::new(save_dir)
    }

    /// Set current map
    pub fn set_current_map(&mut self, map: impl Into<String>) {
        self.current_map = map.into();
    }

    /// Set player name
    pub fn set_player_name(&mut self, name: impl Into<String>) {
        self.player_name = name.into();
    }

    /// Enable/disable autosave
    pub fn set_autosave(&mut self, enabled: bool) {
        self.autosave_enabled = enabled;
    }

    /// Check if autosave is enabled
    pub fn autosave_enabled(&self) -> bool {
        self.autosave_enabled
    }

    /// Update play time and check for autosave
    pub fn update(
        &mut self,
        dt: Duration,
        world: &hecs::World,
        seed: u64,
        tick_count: u64,
    ) -> Option<u32> {
        self.play_time += dt;

        if self.autosave_enabled {
            let now = Instant::now();
            if now.duration_since(self.last_autosave) >= AUTOSAVE_INTERVAL {
                self.last_autosave = now;
                // Autosave to slot 0
                match self.save_to_slot(world, seed, tick_count, 0) {
                    Ok(_) => return Some(0),
                    Err(e) => tracing::error!("Autosave failed: {}", e),
                }
            }
        }

        None
    }

    /// Save to a specific slot
    pub fn save_to_slot(
        &mut self,
        world: &hecs::World,
        seed: u64,
        tick_count: u64,
        slot: u32,
    ) -> Result<GameSave, Box<dyn std::error::Error>> {
        if slot as usize >= SAVE_SLOT_COUNT {
            return Err("Invalid save slot".into());
        }

        let mut save = GameSave::new(slot, &self.player_name, &self.current_map);
        save.play_time_secs = self.play_time.as_secs();
        save.world = WorldSerializer::serialize(world, seed, tick_count);

        let path = self.save_path(slot);
        let json = save.to_json()?;
        std::fs::write(&path, json)?;

        self.current_slot = Some(slot);

        tracing::info!("Saved to slot {}: {:?}", slot, path);
        Ok(save)
    }

    /// Load from a specific slot
    pub fn load_from_slot(
        &mut self,
        world: &mut hecs::World,
        slot: u32,
    ) -> Result<GameSave, Box<dyn std::error::Error>> {
        if slot as usize >= SAVE_SLOT_COUNT {
            return Err("Invalid save slot".into());
        }

        let path = self.save_path(slot);
        let json = std::fs::read_to_string(&path)?;
        let save = GameSave::from_json(&json)?;

        // Restore world state
        WorldSerializer::deserialize(world, &save.world);

        // Restore manager state
        self.play_time = Duration::from_secs(save.play_time_secs);
        self.current_map = save.current_map.clone();
        self.player_name = save.player_name.clone();
        self.current_slot = Some(slot);

        tracing::info!("Loaded from slot {}: {:?}", slot, path);
        Ok(save)
    }

    /// Get save info for a slot (without loading)
    pub fn get_save_info(&self, slot: u32) -> Option<GameSave> {
        if slot as usize >= SAVE_SLOT_COUNT {
            return None;
        }

        let path = self.save_path(slot);
        if !path.exists() {
            return None;
        }

        let json = std::fs::read_to_string(&path).ok()?;
        GameSave::from_json(&json).ok()
    }

    /// Get all save slot info
    pub fn get_all_saves(&self) -> Vec<(u32, Option<GameSave>)> {
        (0..SAVE_SLOT_COUNT as u32)
            .map(|slot| (slot, self.get_save_info(slot)))
            .collect()
    }

    /// Delete a save
    pub fn delete_save(&mut self, slot: u32) -> Result<(), Box<dyn std::error::Error>> {
        if slot as usize >= SAVE_SLOT_COUNT {
            return Err("Invalid save slot".into());
        }

        let path = self.save_path(slot);
        if path.exists() {
            std::fs::remove_file(&path)?;
            tracing::info!("Deleted save slot {}", slot);
        }

        if self.current_slot == Some(slot) {
            self.current_slot = None;
        }

        Ok(())
    }

    /// Get path for a save slot
    fn save_path(&self, slot: u32) -> PathBuf {
        self.save_dir.join(format!("save_{:02}.json", slot))
    }

    /// Get current save slot
    pub fn current_slot(&self) -> Option<u32> {
        self.current_slot
    }

    /// Get play time
    pub fn play_time(&self) -> Duration {
        self.play_time
    }

    /// Format play time as string
    pub fn format_play_time(&self) -> String {
        let hours = self.play_time.as_secs() / 3600;
        let minutes = (self.play_time.as_secs() % 3600) / 60;
        let secs = self.play_time.as_secs() % 60;
        format!("{:02}:{:02}:{:02}", hours, minutes, secs)
    }

    /// Check if slot exists
    pub fn slot_exists(&self, slot: u32) -> bool {
        self.save_path(slot).exists()
    }

    /// Get save directory
    pub fn save_dir(&self) -> &Path {
        &self.save_dir
    }
}

impl Default for SaveManager {
    fn default() -> Self {
        Self::default()
    }
}

/// Save menu UI
pub struct SaveMenu {
    /// Whether the menu is visible
    pub visible: bool,
    /// Selected slot for confirmation
    pub selected_slot: Option<u32>,
    /// Confirmation dialog state
    pub confirming_overwrite: bool,
    /// Mode: Save or Load
    pub mode: SaveMenuMode,
}

impl Default for SaveMenu {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
impl SaveMenu {
    /// Create a new save menu
    pub fn new() -> Self {
        Self {
            visible: false,
            selected_slot: None,
            confirming_overwrite: false,
            mode: SaveMenuMode::Save,
        }
    }

    /// Show the menu
    pub fn show(&mut self, mode: SaveMenuMode) {
        self.visible = true;
        self.mode = mode;
        self.selected_slot = None;
        self.confirming_overwrite = false;
    }

    /// Hide the menu
    pub fn hide(&mut self) {
        self.visible = false;
        self.selected_slot = None;
        self.confirming_overwrite = false;
    }

    /// Toggle visibility
    pub fn toggle(&mut self, mode: SaveMenuMode) {
        if self.visible && self.mode == mode {
            self.hide();
        } else {
            self.show(mode);
        }
    }

    /// Check if menu is visible
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Draw the save menu UI (stub - egui integration needed)
    pub fn draw(
        &mut self,
        _ctx: &egui::Context,
        _save_manager: &mut SaveManager,
    ) -> SaveMenuAction {
        // TODO: Implement when egui renderer integration is available
        SaveMenuAction::None
    }

    /// Draw the slot list (stub)
    fn draw_slot_list(
        &mut self,
        _ui: &mut egui::Ui,
        _save_manager: &SaveManager,
    ) -> SaveMenuAction {
        // TODO: Implement when egui renderer integration is available
        SaveMenuAction::None
    }
}

/// Save menu mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveMenuMode {
    Save,
    Load,
}

/// Save menu action result
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
pub enum SaveMenuAction {
    None,
    Save(u32),
    Load(u32),
    Delete(u32),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_save_manager_creation() {
        let temp_dir = std::env::temp_dir().join("dde_test_saves");
        let manager = SaveManager::new(&temp_dir);

        assert_eq!(manager.current_slot(), None);
        assert!(manager.autosave_enabled());

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_save_path() {
        let temp_dir = std::env::temp_dir().join("dde_test_saves");
        let manager = SaveManager::new(&temp_dir);

        let path = temp_dir.join("save_01.json");
        assert_eq!(manager.save_path(1), path);

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_play_time_formatting() {
        let temp_dir = std::env::temp_dir().join("dde_test_saves");
        let mut manager = SaveManager::new(&temp_dir);

        manager.play_time = Duration::from_secs(3661); // 1h 1m 1s
        assert_eq!(manager.format_play_time(), "01:01:01");

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
