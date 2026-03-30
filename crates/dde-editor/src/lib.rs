//! DocDamage Engine - Editor Layer
//!
//! Editor mode with egui panels for world editing.

pub mod battle_panel;
pub mod behavior_tree;
pub mod commands;
pub mod collaboration_panel;
pub mod cutscene_editor;
pub mod director_panel;
pub mod documentation_panel;
pub mod export;
pub mod hot_reload_panel;
pub mod live_play;
pub mod profiler_panel;
pub mod replay_panel;
pub mod save_panel;
pub mod sync_panel;
pub mod tilemap;
pub mod timeline;
pub mod visual_script;
pub mod visual_script_editor;

pub use battle_panel::BattlePanel;
pub use collaboration_panel::{CollaborationPanel, CollaborationExt};
pub use cutscene_editor::CutsceneEditor;
pub use director_panel::DirectorPanel;
pub use documentation_panel::{DocumentationPanel, ExportFormat, GeneratedDocs};
pub use hot_reload_panel::HotReloadPanel;
pub use live_play::{CameraState, EditorController, PlayMode};
pub use profiler_panel::ProfilerPanel;
pub use replay_panel::ReplayPanel;
pub use save_panel::SavePanel;
pub use sync_panel::SyncPanel;
pub use timeline::*;
pub use visual_script::canvas::CanvasStyle;

use export::ExportPanel;
use tilemap::TileMapEditor;
use visual_script_editor::VisualScriptEditor;

/// Editor state
pub struct Editor {
    pub active: bool,
    pub selected_entity: Option<dde_core::Entity>,
    /// Tilemap editor
    pub tilemap_editor: TileMapEditor,
    /// Export panel
    pub export_panel: ExportPanel,
    /// Collaboration panel
    pub collaboration_panel: CollaborationPanel,
    /// AI Director panel
    pub director_panel: DirectorPanel,
    /// Cutscene editor
    pub cutscene_editor: CutsceneEditor,
    /// Visual script editor
    pub visual_script_editor: VisualScriptEditor,
    /// Auto-documentation panel
    pub documentation_panel: DocumentationPanel,
    /// Hot-reload panel
    pub hot_reload_panel: HotReloadPanel,
    /// Save manager panel
    pub save_panel: SavePanel,
    /// Replay theater panel
    pub replay_panel: ReplayPanel,
}

impl Editor {
    pub fn new() -> Self {
        Self {
            active: false,
            selected_entity: None,
            tilemap_editor: TileMapEditor::new(),
            export_panel: ExportPanel::new(),
            collaboration_panel: CollaborationPanel::new(),
            director_panel: DirectorPanel::new(),
            cutscene_editor: CutsceneEditor::new(),
            visual_script_editor: VisualScriptEditor::new(),
            documentation_panel: DocumentationPanel::new(),
            hot_reload_panel: HotReloadPanel::new(),
            save_panel: SavePanel::default(),
            replay_panel: ReplayPanel::default(),
        }
    }

    pub fn toggle(&mut self) {
        self.active = !self.active;
        self.tilemap_editor.set_active(self.active);
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Draw the editor UI
    pub fn draw(&mut self, ctx: &egui::Context, project_id: &str) {
        if !self.active {
            return;
        }

        // Draw tilemap editor
        self.tilemap_editor.draw(ctx);

        // Draw export panel
        self.export_panel.draw(ctx);

        // Draw collaboration panel
        self.collaboration_panel.draw_ui(ctx, project_id);

        // Draw documentation panel
        self.documentation_panel.draw(ctx);

        // Draw director panel (will only show if visible)
        // Note: DirectorSystem would need to be passed in from the application level
    }

    /// Draw the director panel with the director system
    pub fn draw_director_panel(&mut self, ctx: &egui::Context, director: Option<&mut dde_ai::DirectorSystem>) {
        self.director_panel.draw(ctx, director);
    }

    /// Draw the documentation panel
    pub fn draw_documentation_panel(&mut self, ctx: &egui::Context) {
        self.documentation_panel.draw(ctx);
    }

    /// Check if editor has unsaved changes
    pub fn has_unsaved_changes(&self) -> bool {
        self.tilemap_editor.is_dirty() || self.visual_script_editor.is_dirty()
    }
}

impl Default for Editor {
    fn default() -> Self {
        Self::new()
    }
}
