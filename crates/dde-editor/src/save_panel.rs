//! Save Manager Panel for Editor
//!
//! Provides UI for:
//! - Save slot management
//! - Password protection
//! - Export/Import
//! - Backup management

use dde_core::save::{SaveConfig, SaveError, SaveManager, SaveMetadata};
use dde_core::GameSave;
use egui::{Color32, RichText, Ui};
use std::path::PathBuf;

/// Save panel tabs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SavePanelTab {
    Slots,
    Settings,
    ImportExport,
}

/// Save panel state
pub struct SavePanel {
    /// Current tab
    current_tab: SavePanelTab,
    /// Save manager
    save_manager: SaveManager,
    /// Selected slot
    selected_slot: Option<u32>,
    /// Password input
    password_input: String,
    /// Show password
    show_password: bool,
    /// Status message
    status_message: Option<(String, bool)>, // (message, is_error)
    /// Confirm delete dialog
    confirm_delete: Option<u32>,
    /// Export path input
    export_path: String,
    /// Import path input
    import_path: String,
    /// New save player name
    new_player_name: String,
    /// New save map name
    new_map_name: String,
}

impl Default for SavePanel {
    fn default() -> Self {
        let config = SaveConfig::default();
        let save_manager = SaveManager::new(config).unwrap_or_else(|_| {
            // Fallback to temp directory if default fails
            SaveManager::new(SaveConfig {
                save_dir: std::env::temp_dir().join("dde_saves"),
                ..Default::default()
            })
            .unwrap()
        });

        Self {
            current_tab: SavePanelTab::Slots,
            save_manager,
            selected_slot: None,
            password_input: String::new(),
            show_password: false,
            status_message: None,
            confirm_delete: None,
            export_path: String::new(),
            import_path: String::new(),
            new_player_name: "Player".to_string(),
            new_map_name: "Map001".to_string(),
        }
    }
}

impl SavePanel {
    /// Create new save panel with custom config
    pub fn with_config(config: SaveConfig) -> Result<Self, SaveError> {
        let save_manager = SaveManager::new(config)?;

        Ok(Self {
            current_tab: SavePanelTab::Slots,
            save_manager,
            selected_slot: None,
            password_input: String::new(),
            show_password: false,
            status_message: None,
            confirm_delete: None,
            export_path: String::new(),
            import_path: String::new(),
            new_player_name: "Player".to_string(),
            new_map_name: "Map001".to_string(),
        })
    }

    /// Refresh save cache
    pub fn refresh(&mut self) {
        if let Err(e) = self.save_manager.refresh_cache() {
            self.set_status(format!("Failed to refresh: {}", e), true);
        }
    }

    /// Set status message
    fn set_status(&mut self, message: impl Into<String>, is_error: bool) {
        self.status_message = Some((message.into(), is_error));
    }

    /// Clear status message
    fn clear_status(&mut self) {
        self.status_message = None;
    }

    /// Draw the save panel
    pub fn draw(&mut self, ui: &mut Ui) {
        // Tab bar
        ui.horizontal(|ui| {
            if ui
                .selectable_label(self.current_tab == SavePanelTab::Slots, "💾 Save Slots")
                .clicked()
            {
                self.current_tab = SavePanelTab::Slots;
                self.refresh();
            }
            if ui
                .selectable_label(self.current_tab == SavePanelTab::Settings, "⚙ Settings")
                .clicked()
            {
                self.current_tab = SavePanelTab::Settings;
            }
            if ui
                .selectable_label(
                    self.current_tab == SavePanelTab::ImportExport,
                    "📁 Import/Export",
                )
                .clicked()
            {
                self.current_tab = SavePanelTab::ImportExport;
            }
        });

        ui.separator();

        // Status message
        if let Some((msg, is_error)) = &self.status_message {
            let color = if *is_error {
                Color32::RED
            } else {
                Color32::GREEN
            };
            ui.label(RichText::new(msg).color(color));
            if ui.button("Clear").clicked() {
                self.clear_status();
            }
            ui.separator();
        }

        // Tab content
        match self.current_tab {
            SavePanelTab::Slots => self.draw_slots_tab(ui),
            SavePanelTab::Settings => self.draw_settings_tab(ui),
            SavePanelTab::ImportExport => self.draw_import_export_tab(ui),
        }

        // Delete confirmation dialog
        if let Some(slot) = self.confirm_delete {
            self.draw_delete_dialog(ui, slot);
        }
    }

    /// Draw save slots tab
    fn draw_slots_tab(&mut self, ui: &mut Ui) {
        ui.heading("Save Slots");
        ui.add_space(10.0);

        // New save section
        ui.collapsing("➕ Create New Save", |ui| {
            ui.horizontal(|ui| {
                ui.label("Player Name:");
                ui.text_edit_singleline(&mut self.new_player_name);
            });
            ui.horizontal(|ui| {
                ui.label("Map:");
                ui.text_edit_singleline(&mut self.new_map_name);
            });
            ui.horizontal(|ui| {
                ui.label("Password (optional):");
                if self.show_password {
                    ui.text_edit_singleline(&mut self.password_input);
                } else {
                    ui.add(egui::TextEdit::singleline(&mut self.password_input).password(true));
                }
                ui.checkbox(&mut self.show_password, "Show");
            });

            if ui.button("💾 Create Save").clicked() {
                if let Some(slot) = self.save_manager.next_available_slot() {
                    let save = GameSave::new(slot, &self.new_player_name, &self.new_map_name);
                    let password = if self.password_input.is_empty() {
                        None
                    } else {
                        Some(self.password_input.as_str())
                    };

                    match self.save_manager.save(slot, &save, password) {
                        Ok(()) => {
                            self.set_status(format!("Saved to slot {}", slot), false);
                            self.password_input.clear();
                        }
                        Err(e) => self.set_status(format!("Save failed: {}", e), true),
                    }
                } else {
                    self.set_status("No available save slots!", true);
                }
            }
        });

        ui.add_space(10.0);
        ui.separator();
        ui.add_space(10.0);

        // Save slots grid
        let metadata: Vec<_> = self
            .save_manager
            .get_all_metadata()
            .into_iter()
            .cloned()
            .collect();

        if metadata.is_empty() {
            ui.label("No save files found.");
        } else {
            // Collect data needed before the closure
            let selected_slot = self.selected_slot;

            egui::Grid::new("save_slots_grid")
                .num_columns(4)
                .spacing([20.0, 10.0])
                .show(ui, |ui| {
                    ui.label(RichText::new("Slot").strong());
                    ui.label(RichText::new("Player").strong());
                    ui.label(RichText::new("Map").strong());
                    ui.label(RichText::new("Actions").strong());
                    ui.end_row();

                    for meta in &metadata {
                        Self::draw_save_slot_row(
                            ui,
                            meta,
                            selected_slot,
                            &mut self.selected_slot,
                            &mut self.confirm_delete,
                        );
                        ui.end_row();
                    }
                });
        }
    }

    /// Draw a single save slot row
    fn draw_save_slot_row(
        ui: &mut Ui,
        meta: &SaveMetadata,
        selected_slot: Option<u32>,
        selected_slot_ref: &mut Option<u32>,
        confirm_delete: &mut Option<u32>,
    ) {
        let is_selected = selected_slot == Some(meta.slot);

        // Slot number
        let slot_text = if meta.is_encrypted {
            format!("🔒 {}", meta.slot)
        } else {
            format!("{}", meta.slot)
        };

        if ui.selectable_label(is_selected, slot_text).clicked() {
            *selected_slot_ref = Some(meta.slot);
        }

        // Player name
        ui.label(&meta.player_name);

        // Map
        ui.label(&meta.current_map);

        // Actions
        ui.horizontal(|ui| {
            if ui.button("📂 Load").clicked() {
                *selected_slot_ref = Some(meta.slot);
            }

            if ui.button("🗑 Delete").clicked() {
                *confirm_delete = Some(meta.slot);
            }
        });

        // Show details if selected
        if is_selected {
            ui.end_row();
            ui.label("");
            ui.vertical(|ui| {
                ui.label(format!("Play Time: {}", meta.formatted_play_time()));
                ui.label(format!("Saved: {}", meta.formatted_date()));
                ui.label(format!("Size: {} bytes", meta.file_size));

                if meta.is_encrypted {
                    ui.label("🔒 Encrypted (password required to load)");
                }
            });
            ui.label("");
            ui.label("");
        }
    }

    /// Draw settings tab
    fn draw_settings_tab(&mut self, ui: &mut Ui) {
        ui.heading("Save Settings");
        ui.add_space(10.0);

        if ui.button("🔄 Refresh Save Cache").clicked() {
            self.refresh();
            self.set_status("Save cache refreshed", false);
        }

        ui.add_space(10.0);

        // Total size
        match self.save_manager.total_size() {
            Ok(size) => {
                let size_mb = size as f64 / 1_048_576.0;
                ui.label(format!("Total save data: {:.2} MB", size_mb));
            }
            Err(e) => {
                ui.label(format!("Error calculating size: {}", e));
            }
        }

        ui.add_space(10.0);
        ui.separator();
        ui.add_space(10.0);

        // Backup management
        ui.heading("Backup Management");
        ui.add_space(5.0);

        if let Some(slot) = self.selected_slot {
            let backups = self.save_manager.get_backups(slot);
            if backups.is_empty() {
                ui.label(format!("No backups for slot {}", slot));
            } else {
                ui.label(format!("Slot {} has {} backup(s)", slot, backups.len()));
                for backup_num in backups {
                    if ui
                        .button(format!("Restore backup {} for slot {}", backup_num, slot))
                        .clicked()
                    {
                        match self.save_manager.restore_backup(slot, backup_num) {
                            Ok(()) => self.set_status(
                                format!("Restored backup {} for slot {}", backup_num, slot),
                                false,
                            ),
                            Err(e) => self.set_status(format!("Restore failed: {}", e), true),
                        }
                    }
                }
            }
        } else {
            ui.label("Select a save slot to manage backups");
        }
    }

    /// Draw import/export tab
    fn draw_import_export_tab(&mut self, ui: &mut Ui) {
        ui.heading("Import / Export");
        ui.add_space(10.0);

        // Export section
        ui.group(|ui| {
            ui.heading("Export Save");
            ui.add_space(5.0);

            ui.horizontal(|ui| {
                ui.label("Slot to export:");
                if let Some(slot) = self.selected_slot {
                    ui.label(format!("{}", slot));
                } else {
                    ui.label("None selected");
                }
            });

            ui.horizontal(|ui| {
                ui.label("Export path:");
                ui.text_edit_singleline(&mut self.export_path);
            });

            if ui.button("📤 Export").clicked() {
                if let Some(slot) = self.selected_slot {
                    if self.export_path.is_empty() {
                        self.set_status("Please enter an export path", true);
                    } else {
                        let path = PathBuf::from(&self.export_path);
                        let password = if self.password_input.is_empty() {
                            None
                        } else {
                            Some(self.password_input.as_str())
                        };

                        match self.save_manager.export(slot, &path, password) {
                            Ok(()) => self
                                .set_status(format!("Exported slot {} to {:?}", slot, path), false),
                            Err(e) => self.set_status(format!("Export failed: {}", e), true),
                        }
                    }
                } else {
                    self.set_status("Please select a save slot first", true);
                }
            }
        });

        ui.add_space(10.0);

        // Import section
        ui.group(|ui| {
            ui.heading("Import Save");
            ui.add_space(5.0);

            ui.horizontal(|ui| {
                ui.label("Import path:");
                ui.text_edit_singleline(&mut self.import_path);
            });

            ui.horizontal(|ui| {
                ui.label("Target slot:");
                if let Some(slot) = self.save_manager.next_available_slot() {
                    ui.label(format!("{} (first available)", slot));
                } else {
                    ui.label("No slots available");
                }
            });

            ui.horizontal(|ui| {
                ui.label("Password:");
                if self.show_password {
                    ui.text_edit_singleline(&mut self.password_input);
                } else {
                    ui.add(egui::TextEdit::singleline(&mut self.password_input).password(true));
                }
                ui.checkbox(&mut self.show_password, "Show");
            });

            if ui.button("📥 Import").clicked() {
                if self.import_path.is_empty() {
                    self.set_status("Please enter an import path", true);
                } else if let Some(slot) = self.save_manager.next_available_slot() {
                    let path = PathBuf::from(&self.import_path);
                    let password = if self.password_input.is_empty() {
                        None
                    } else {
                        Some(self.password_input.as_str())
                    };

                    match self.save_manager.import(slot, &path, password) {
                        Ok(()) => {
                            self.set_status(format!("Imported to slot {}", slot), false);
                            self.import_path.clear();
                            self.password_input.clear();
                        }
                        Err(e) => self.set_status(format!("Import failed: {}", e), true),
                    }
                } else {
                    self.set_status("No available save slots!", true);
                }
            }
        });
    }

    /// Draw delete confirmation dialog
    fn draw_delete_dialog(&mut self, ui: &mut Ui, slot: u32) {
        egui::Window::new("Confirm Delete")
            .collapsible(false)
            .resizable(false)
            .show(ui.ctx(), |ui| {
                ui.label(format!(
                    "Are you sure you want to delete save slot {}?",
                    slot
                ));
                ui.label("This action cannot be undone!");

                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        self.confirm_delete = None;
                    }

                    if ui
                        .button(RichText::new("Delete").color(Color32::RED))
                        .clicked()
                    {
                        match self.save_manager.delete(slot) {
                            Ok(()) => {
                                self.set_status(format!("Deleted slot {}", slot), false);
                                if self.selected_slot == Some(slot) {
                                    self.selected_slot = None;
                                }
                            }
                            Err(e) => self.set_status(format!("Delete failed: {}", e), true),
                        }
                        self.confirm_delete = None;
                    }
                });
            });
    }

    /// Get selected slot for external loading
    pub fn selected_slot(&self) -> Option<u32> {
        self.selected_slot
    }

    /// Get password for selected slot
    pub fn password(&self) -> &str {
        &self.password_input
    }

    /// Get mutable reference to save manager
    pub fn save_manager(&mut self) -> &mut SaveManager {
        &mut self.save_manager
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_save_panel_default() {
        let panel = SavePanel::default();
        assert_eq!(panel.current_tab, SavePanelTab::Slots);
        assert!(panel.selected_slot.is_none());
        assert!(panel.password_input.is_empty());
    }

    #[test]
    fn test_save_panel_status() {
        let mut panel = SavePanel::default();

        panel.set_status("Test message", false);
        assert!(panel.status_message.is_some());
        assert_eq!(panel.status_message.as_ref().unwrap().0, "Test message");
        assert!(!panel.status_message.as_ref().unwrap().1);

        panel.clear_status();
        assert!(panel.status_message.is_none());
    }
}
