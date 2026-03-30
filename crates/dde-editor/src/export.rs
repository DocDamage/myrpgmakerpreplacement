//! Export UI Panel
//!
//! Editor integration for RPG Maker MZ and standalone exports.

use dde_export::{
    ActorDefinition, AssetSources, ClassDefinition, DatabaseConfig, ExportOptions, ExportSystem,
    ExportTarget, SystemConfig,
};
use std::path::PathBuf;

/// Export panel state
pub struct ExportPanel {
    pub visible: bool,
    pub project_name: String,
    pub output_path: String,
    pub selected_target: ExportTarget,
    pub status_message: Option<String>,
    pub status_error: bool,
}

impl ExportPanel {
    pub fn new() -> Self {
        Self {
            visible: false,
            project_name: "MyRPG".to_string(),
            output_path: "./export".to_string(),
            selected_target: ExportTarget::MzAssets,
            status_message: None,
            status_error: false,
        }
    }

    pub fn show(&mut self) {
        self.visible = true;
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Draw the export panel UI
    pub fn draw(&mut self, ctx: &egui::Context) {
        if !self.visible {
            return;
        }

        let mut visible = self.visible;
        egui::Window::new("Export Project")
            .open(&mut visible)
            .resizable(true)
            .default_size([400.0, 300.0])
            .show(ctx, |ui| {
                self.draw_export_ui(ui);
            });
        self.visible = visible;
    }

    fn draw_export_ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("Export Settings");
        ui.separator();

        // Project name
        ui.horizontal(|ui| {
            ui.label("Project Name:");
            ui.text_edit_singleline(&mut self.project_name);
        });

        // Output path
        ui.horizontal(|ui| {
            ui.label("Output Path:");
            ui.text_edit_singleline(&mut self.output_path);
        });

        ui.separator();

        // Export target selection
        ui.label("Export Target:");
        ui.horizontal(|ui| {
            ui.radio_value(
                &mut self.selected_target,
                ExportTarget::MzAssets,
                "MZ Assets Only",
            );
        });
        ui.horizontal(|ui| {
            ui.radio_value(
                &mut self.selected_target,
                ExportTarget::MzPartial,
                "MZ Partial Project",
            );
        });
        ui.horizontal(|ui| {
            ui.radio_value(
                &mut self.selected_target,
                ExportTarget::MzFull,
                "MZ Full Project",
            );
        });
        ui.horizontal(|ui| {
            ui.radio_value(
                &mut self.selected_target,
                ExportTarget::Standalone,
                "Standalone Runtime",
            );
        });

        ui.separator();

        // Export button
        ui.horizontal(|ui| {
            if ui.button("📦 Export").clicked() {
                self.perform_export();
            }

            if ui.button("Cancel").clicked() {
                self.hide();
            }
        });

        // Status message
        if let Some(ref msg) = self.status_message {
            ui.separator();
            if self.status_error {
                ui.colored_label(egui::Color32::RED, msg);
            } else {
                ui.colored_label(egui::Color32::GREEN, msg);
            }
        }

        // Help text
        ui.separator();
        ui.collapsing("Help", |ui| match self.selected_target {
            ExportTarget::MzAssets => {
                ui.label("Exports only image assets in MZ format:");
                ui.label("• Character sprites with $ prefix");
                ui.label("• Facesets (4x2 grid)");
                ui.label("• Tilesets, Parallaxes, Enemies");
                ui.label("Copy the img/ folder to your MZ project.");
            }
            ExportTarget::MzPartial => {
                ui.label("Exports assets + basic database files:");
                ui.label("• All image assets");
                ui.label("• Actors.json, Classes.json");
                ui.label("• System.json with game title");
                ui.label("Copy both img/ and data/ folders.");
            }
            ExportTarget::MzFull => {
                ui.label("Exports complete MZ project structure:");
                ui.label("• All assets and database files");
                ui.label("• Audio folder structure");
                ui.label("• Placeholder maps");
                ui.label("Ready to open in RPG Maker MZ.");
            }
            ExportTarget::Standalone => {
                ui.label("Exports for DDE standalone runtime:");
                ui.label("• Optimized asset bundle");
                ui.label("• game.json configuration");
                ui.label("• Run with DDE runtime executable.");
            }
        });
    }

    fn perform_export(&mut self) {
        let output_path = PathBuf::from(&self.output_path);

        // Create default database config
        let database = DatabaseConfig {
            actors: vec![ActorDefinition {
                name: self.project_name.clone(),
                ..Default::default()
            }],
            classes: vec![ClassDefinition::default()],
            system: Some(SystemConfig {
                game_title: self.project_name.clone(),
                ..Default::default()
            }),
            ..Default::default()
        };

        let options = ExportOptions {
            target: self.selected_target,
            output_path,
            project_name: self.project_name.clone(),
            include_assets: true,
            encrypt_assets: false,
            asset_sources: AssetSources::default(),
            database,
            overwrite_existing: true,
        };

        let mut exporter = ExportSystem::new(options);

        match exporter.export() {
            Ok(result) => {
                let file_count = result.files_created.len();
                self.status_message = Some(format!(
                    "✓ Export successful! Created {} files in {:?}",
                    file_count, result.output_path
                ));
                self.status_error = false;
            }
            Err(e) => {
                self.status_message = Some(format!("✗ Export failed: {}", e));
                self.status_error = true;
            }
        }
    }
}

impl Default for ExportPanel {
    fn default() -> Self {
        Self::new()
    }
}
