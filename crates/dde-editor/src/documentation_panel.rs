//! Auto-Documentation Panel
//!
//! Editor UI for generating and exporting AI-powered documentation.
//! Generates World Bibles, Character Profiles, Quest Logs, and Store Descriptions.

use dde_ai::documentation::{
    generator::{
        CharacterProfile, DocGenerator, QuestLog, StoreDescription, WorldBible, WorldDataProvider,
    },
    exporters::{export_markdown, export_pdf, export_wiki},
};

/// Export format options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Markdown,
    Pdf,
    Wiki,
}

/// Generated documents bundle
pub struct GeneratedDocs {
    /// World bible content
    pub world_bible: WorldBible,
    /// Character profiles
    pub characters: Vec<CharacterProfile>,
    /// Quest log
    pub quest_log: QuestLog,
    /// Store description
    pub store_description: StoreDescription,
    /// Markdown preview
    pub markdown_preview: String,
}

/// Auto-Documentation Panel
pub struct DocumentationPanel {
    _generator: DocGenerator,
    generated_docs: Option<GeneratedDocs>,
    export_format: ExportFormat,
    generating: bool,
    progress: f32,
    visible: bool,
    selected_tab: DocTab,
    status_message: Option<String>,
}

/// Documentation panel tabs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DocTab {
    Generate,
    Preview,
    Export,
}

impl DocumentationPanel {
    /// Create a new documentation panel
    pub fn new() -> Self {
        Self {
            _generator: DocGenerator::new(),
            generated_docs: None,
            export_format: ExportFormat::Markdown,
            generating: false,
            progress: 0.0,
            visible: false,
            selected_tab: DocTab::Generate,
            status_message: None,
        }
    }

    /// Show the panel
    pub fn show(&mut self) {
        self.visible = true;
    }

    /// Hide the panel
    pub fn hide(&mut self) {
        self.visible = false;
    }

    /// Toggle visibility
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    /// Check if visible
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Draw the documentation panel UI
    pub fn draw(&mut self, ctx: &egui::Context) {
        if !self.visible {
            return;
        }

        let mut visible = self.visible;
        egui::Window::new("📚 Auto-Documentation")
            .open(&mut visible)
            .resizable(true)
            .default_size([700.0, 500.0])
            .show(ctx, |ui| {
                self.draw_panel_content(ui);
            });
        self.visible = visible;
    }

    /// Draw panel content
    fn draw_panel_content(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("Auto-Documentation Generator");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("✕").clicked() {
                    self.visible = false;
                }
            });
        });

        ui.separator();

        // Tab bar
        ui.horizontal(|ui| {
            self.tab_button(ui, "📝 Generate", DocTab::Generate);
            self.tab_button(ui, "👁 Preview", DocTab::Preview);
            self.tab_button(ui, "💾 Export", DocTab::Export);
        });

        ui.separator();

        // Status message
        if let Some(ref msg) = self.status_message {
            ui.colored_label(egui::Color32::GREEN, msg);
            ui.separator();
        }

        // Progress bar
        if self.generating {
            ui.add(
                egui::ProgressBar::new(self.progress)
                    .text(format!("Generating... {:.0}%", self.progress * 100.0)),
            );
            ui.separator();
        }

        // Tab content
        match self.selected_tab {
            DocTab::Generate => self.draw_generate_tab(ui),
            DocTab::Preview => self.draw_preview_tab(ui),
            DocTab::Export => self.draw_export_tab(ui),
        }
    }

    /// Draw tab button
    fn tab_button(&mut self, ui: &mut egui::Ui, label: &str, tab: DocTab) {
        let selected = self.selected_tab == tab;
        if ui.selectable_label(selected, label).clicked() {
            self.selected_tab = tab;
        }
    }

    /// Draw generate tab
    fn draw_generate_tab(&mut self, ui: &mut egui::Ui) {
        ui.label("Generate comprehensive documentation for your RPG project using AI.");
        ui.add_space(20.0);

        ui.vertical_centered(|ui| {
            ui.add_space(30.0);

            if self.generating {
                ui.spinner();
                ui.label("Generating documentation...");
                ui.label("This may take a few moments depending on project size.");
            } else {
                let button_text = if self.generated_docs.is_some() {
                    "🔄 Regenerate Documentation"
                } else {
                    "📝 Generate World Bible"
                };

                if ui.button(egui::RichText::new(button_text).size(18.0)).clicked() {
                    // Note: In actual implementation, this would trigger async generation
                    // For now, we simulate the progress
                    self.start_generation();
                }
            }

            ui.add_space(20.0);

            ui.label(
                "This will generate:\n\
                • World Bible (lore, timeline, geography, factions)\n\
                • Character Profiles (for all NPCs)\n\
                • Quest Log (with story arcs)\n\
                • Store Description (marketing copy)",
            );
        });
    }

    /// Draw preview tab
    fn draw_preview_tab(&mut self, ui: &mut egui::Ui) {
        if let Some(ref docs) = self.generated_docs {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.collapsing("World Bible", |ui| {
                    ui.monospace(&docs.markdown_preview);
                });

                ui.collapsing("Character Profiles", |ui| {
                    for character in &docs.characters {
                        ui.collapsing(&character.name, |ui| {
                            ui.label(format!("Description: {}", character.physical_description));
                            ui.label(format!(
                                "Personality: {}",
                                character.personality.join(", ")
                            ));
                        });
                    }
                });

                ui.collapsing("Quest Log", |ui| {
                    for arc in &docs.quest_log.story_arcs {
                        ui.collapsing(&arc.name, |ui| {
                            ui.label(&arc.description);
                            ui.label(&arc.narrative);
                        });
                    }
                });

                ui.collapsing("Store Description", |ui| {
                    ui.label(&docs.store_description.short_description);
                    ui.separator();
                    ui.label(&docs.store_description.full_description);
                    ui.separator();
                    ui.label(format!(
                        "Target Audience: {}",
                        docs.store_description.target_audience
                    ));
                });
            });
        } else {
            ui.vertical_centered(|ui| {
                ui.add_space(50.0);
                ui.label(egui::RichText::new("No documentation generated yet").weak());
                ui.label("Generate documentation first to see a preview.");
            });
        }
    }

    /// Draw export tab
    fn draw_export_tab(&mut self, ui: &mut egui::Ui) {
        if self.generated_docs.is_none() {
            ui.vertical_centered(|ui| {
                ui.add_space(50.0);
                ui.label(egui::RichText::new("No documentation to export").weak());
                ui.label("Generate documentation first before exporting.");
            });
            return;
        }

        ui.label("Choose export format:");
        ui.add_space(10.0);

        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.export_format, ExportFormat::Markdown, "📝 Markdown");
            ui.selectable_value(&mut self.export_format, ExportFormat::Pdf, "📄 PDF");
            ui.selectable_value(&mut self.export_format, ExportFormat::Wiki, "🌐 Wiki");
        });

        ui.add_space(20.0);

        // Format description
        match self.export_format {
            ExportFormat::Markdown => {
                ui.label("Export as Markdown files (.md)");
                ui.label("Best for: GitHub, Obsidian, Notion, general editing");
            }
            ExportFormat::Pdf => {
                ui.label("Export as PDF document (.pdf)");
                ui.label("Best for: Printing, sharing, professional documentation");
            }
            ExportFormat::Wiki => {
                ui.label("Export as MediaWiki format (.wiki)");
                ui.label("Best for: Fandom wikis, MediaWiki sites");
            }
        }

        ui.add_space(20.0);

        if ui.button("💾 Export Documentation").clicked() {
            self.export_docs();
            self.status_message = Some(format!(
                "Documentation exported as {:?}",
                self.export_format
            ));
        }
    }

    /// Start the generation process
    fn start_generation(&mut self) {
        self.generating = true;
        self.progress = 0.0;
        self.status_message = Some("Starting documentation generation...".to_string());

        // In actual implementation, this would spawn an async task
        // For now, we simulate progress
    }

    /// Update generation progress (call from update loop)
    pub fn update_generation(&mut self, dt: f32) {
        if !self.generating {
            return;
        }

        // Simulate progress
        self.progress += dt * 0.2; // 5 seconds total

        if self.progress >= 1.0 {
            self.progress = 1.0;
            self.generating = false;
            // In actual implementation, this would be set when async completes
            self.status_message = Some("Documentation generation complete!".to_string());
        }
    }

    /// Set generated docs (called when async generation completes)
    pub fn set_generated_docs(&mut self, docs: GeneratedDocs) {
        self.generated_docs = Some(docs);
        self.generating = false;
        self.progress = 1.0;
        self.status_message = Some("Documentation generated successfully!".to_string());
    }

    /// Export documents
    fn export_docs(&self) {
        if let Some(ref docs) = self.generated_docs {
            let _ = match self.export_format {
                ExportFormat::Markdown => {
                    let content = export_markdown(&docs.world_bible, &docs.characters);
                    // In actual implementation, save to file
                    tracing::info!("Exporting {} bytes as Markdown", content.len());
                    content
                }
                ExportFormat::Pdf => {
                    let content = export_pdf(&docs.world_bible, &docs.characters);
                    tracing::info!("Exporting {} bytes as PDF", content.len());
                    String::from_utf8_lossy(&content).to_string()
                }
                ExportFormat::Wiki => {
                    let content = export_wiki(&docs.world_bible, &docs.characters);
                    tracing::info!("Exporting {} bytes as Wiki", content.len());
                    content
                }
            };
        }
    }

    /// Check if currently generating
    pub fn is_generating(&self) -> bool {
        self.generating
    }

    /// Get generation progress (0.0 - 1.0)
    pub fn progress(&self) -> f32 {
        self.progress
    }
}

impl Default for DocumentationPanel {
    fn default() -> Self {
        Self::new()
    }
}

/// Extension trait for Database to implement WorldDataProvider
pub mod db_adapter {
    use super::*;
    use dde_ai::documentation::generator::{
        DialogueTreeData, DocResult, FactionData, ItemData, MapData, NpcData, QuestData,
    };
    use dde_db::Database;

    /// Adapter to make Database implement WorldDataProvider
    pub struct DatabaseAdapter<'a> {
        db: &'a Database,
    }

    impl<'a> DatabaseAdapter<'a> {
        /// Create a new adapter
        pub fn new(db: &'a Database) -> Self {
            Self { db }
        }
    }

    impl<'a> WorldDataProvider for DatabaseAdapter<'a> {
        fn get_project_name(&self) -> DocResult<String> {
            let meta = self.db.get_project_meta()?;
            Ok(meta.project_name)
        }

        fn get_setting_description(&self) -> DocResult<Option<String>> {
            // In actual implementation, this would query a settings table
            Ok(Some("A fantasy world".to_string()))
        }

        fn get_main_conflict(&self) -> DocResult<Option<String>> {
            // In actual implementation, this would query story settings
            Ok(None)
        }

        fn get_all_maps(&self) -> DocResult<Vec<MapData>> {
            // In actual implementation, query the maps table
            // For now, return empty
            Ok(vec![])
        }

        fn get_all_npcs(&self) -> DocResult<Vec<NpcData>> {
            // In actual implementation, query the entities table for NPCs
            Ok(vec![])
        }

        fn get_all_quests(&self) -> DocResult<Vec<QuestData>> {
            // In actual implementation, query the quests table
            Ok(vec![])
        }

        fn get_all_items(&self) -> DocResult<Vec<ItemData>> {
            // In actual implementation, query the items table
            Ok(vec![])
        }

        fn get_all_dialogue_trees(&self) -> DocResult<Vec<DialogueTreeData>> {
            // In actual implementation, query the dialogue trees
            Ok(vec![])
        }

        fn get_all_factions(&self) -> DocResult<Vec<FactionData>> {
            // In actual implementation, query the factions table
            Ok(vec![])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_panel_creation() {
        let panel = DocumentationPanel::new();
        assert!(!panel.is_visible());
        assert_eq!(panel.progress(), 0.0);
        assert!(!panel.is_generating());
    }

    #[test]
    fn test_panel_toggle() {
        let mut panel = DocumentationPanel::new();
        assert!(!panel.is_visible());

        panel.toggle();
        assert!(panel.is_visible());

        panel.toggle();
        assert!(!panel.is_visible());
    }

    #[test]
    fn test_export_format_default() {
        let panel = DocumentationPanel::new();
        assert_eq!(panel.export_format, ExportFormat::Markdown);
    }
}
