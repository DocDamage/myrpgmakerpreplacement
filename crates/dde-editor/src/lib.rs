//! DocDamage Engine - Editor Layer
//!
//! Editor mode with egui panels for world editing.

pub mod asset_forge_panels;
pub mod battle_log_viewer;
pub mod battle_panel;
pub mod turn_queue_visual;
pub mod behavior_tree;
pub mod behavior_tree_editor;
pub mod collaboration_panel;
pub mod dialogue_editor;
pub mod commands;
pub mod cutscene_editor;
pub mod director_panel;
pub mod documentation_panel;
pub mod event_bus_monitor;
pub mod export;
pub mod formation_editor;
pub mod formula_editor;
pub mod particle_editor;
pub mod hot_reload_panel;
pub mod item_database_editor;
pub mod live_play;
pub mod lock_manager;
pub mod pathfinding_debug;
pub mod patrol_editor;
pub mod status_effect_editor;
pub mod profiler_panel;
pub mod profiler_panel_enhanced;
pub mod replay_panel;
pub mod save_browser;
pub mod save_panel;
pub mod schedule_editor;
pub mod script_manager;
pub mod sync_panel;
pub mod tilemap;
pub mod timeline;
pub mod visual_script;
pub mod visual_script_editor;

pub use asset_forge_panels::{
    AssetForgePanels, AssetOsPipelinePanel, ClassificationRulesPanel, 
    DependencyGraphPanel, DuplicateScannerPanel,
};
pub use battle_log_viewer::{BattleLogViewer, BattleLogInterface};
pub use battle_panel::BattlePanel;
pub use turn_queue_visual::TurnQueueVisualizer;
pub use behavior_tree_editor::BehaviorTreeVisualEditor;
pub use dialogue_editor::DialogueEditor;
pub use collaboration_panel::{CollaborationExt, CollaborationPanel};
// pub use lock_manager::{LockManagerExt, LockManagerPanel, LockStatistics}; // TODO: Implement lock_manager module
pub use cutscene_editor::CutsceneEditor;
pub use director_panel::DirectorPanel;
pub use documentation_panel::{DocumentationPanel, ExportFormat, GeneratedDocs};
pub use formation_editor::{
    FormationEditor, FormationEditorInterface, PartyMember, EnemyFormation,
    EnemySlot, FormationPreset, PositionProperties, GridPosition,
};
pub use formula_editor::FormulaEditor;
pub use hot_reload_panel::HotReloadPanel;
pub use item_database_editor::ItemDatabaseEditor;
pub use live_play::{CameraState, EditorController, PlayMode};
pub use particle_editor::{ParticleEditor, ParticlePreset, ParticleSystemData};
pub use pathfinding_debug::{PathfindingDebugOverlay, PathfindingDebugPanel, PathfindingDebugExt, SettingMode};
pub use status_effect_editor::{
    StatusEffectEditor, StatusEffectInterface, StatusEffectTemplate, 
    StackBehavior, TestEntityStats
};
pub use profiler_panel::ProfilerPanel;
pub use profiler_panel_enhanced::ProfilerPanelEnhanced;
pub use event_bus_monitor::{EventBusMonitor, EventBusMonitorExt};
pub use replay_panel::ReplayPanel;
pub use save_browser::SaveBrowser;
pub use save_panel::SavePanel;
pub use schedule_editor::{ScheduleEditor, ScheduleEditorExt, NpcInfo};
// pub use patrol_editor::{
//     PatrolEditor, PatrolEditorExt, PatrolPathData, PatrolPathId, 
//     Waypoint, NpcInfo as PatrolNpcInfo, ToolMode, MapViewState, 
//     PreviewState
// }; // TODO: Implement patrol_editor module
pub use script_manager::{ScriptManagerPanel, ScriptManagerBackend, ValidationResult};
pub use sync_panel::SyncPanel;
pub use timeline::*;
pub use visual_script::canvas::CanvasStyle;

use dde_battle::BattleLog;
use dde_core::Entity;
use export::ExportPanel;
use tilemap::TileMapEditor;
use visual_script_editor::VisualScriptEditor;

// Note: uuid::Uuid not currently used

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
    /// Item database editor
    pub item_database_editor: ItemDatabaseEditor,
    /// Save manager panel
    pub save_panel: SavePanel,
    /// Save/Backup browser panel
    pub save_browser: SaveBrowser,
    /// Replay theater panel
    pub replay_panel: ReplayPanel,
    /// Status effect editor panel
    pub status_effect_editor: StatusEffectEditor,
    /// Formula editor panel
    pub formula_editor: FormulaEditor,
    /// Behavior Tree visual editor
    pub behavior_tree_editor: BehaviorTreeVisualEditor,
    /// Dialogue Tree Editor
    pub dialogue_editor: DialogueEditor,
    /// Asset Forge panels (classification, dependency graph, duplicate scanner)
    pub asset_forge_panels: AssetForgePanels,
    /// Formation editor
    pub formation_editor: FormationEditor,
    /// Particle system editor
    pub particle_editor: ParticleEditor,
    /// Script Manager browser panel
    pub script_manager: ScriptManagerPanel,
    /// NPC Schedule editor
    pub schedule_editor: ScheduleEditor,
    /// Patrol Path editor
    pub patrol_editor: PatrolEditor,
    /// Pathfinding debug overlay
    pub pathfinding_debug: PathfindingDebugPanel,
    /// Battle testing panel
    pub battle_panel: BattlePanel,
    /// Event Bus Monitor panel
    pub event_bus_monitor: EventBusMonitor,
    /// Lock Manager panel
    pub lock_manager: LockManagerPanel,
    /// Battle Log Viewer for debugging
    pub battle_log_viewer: BattleLogViewer,
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
            item_database_editor: ItemDatabaseEditor::new(),
            save_panel: SavePanel::default(),
            save_browser: SaveBrowser::default(),
            replay_panel: ReplayPanel::default(),
            status_effect_editor: StatusEffectEditor::new(),
            formula_editor: FormulaEditor::new(),
            behavior_tree_editor: BehaviorTreeVisualEditor::new(),
            dialogue_editor: DialogueEditor::new(),
            asset_forge_panels: AssetForgePanels::new(),
            formation_editor: FormationEditor::new(),
            particle_editor: ParticleEditor::new(),
            script_manager: ScriptManagerPanel::new(),
            pathfinding_debug: PathfindingDebugPanel::new(),
            battle_panel: BattlePanel::new(),
            battle_log_viewer: BattleLogViewer::new(),
            event_bus_monitor: EventBusMonitor::new(),
            lock_manager: LockManagerPanel::new(),
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

        // Draw Asset Forge panels (will only show if visible)
        self.asset_forge_panels.draw(ctx);

        // Draw formation editor
        self.formation_editor.draw(ctx);

        // Draw particle editor
        self.particle_editor.draw(ctx);

        // Draw schedule editor
        self.schedule_editor.draw(ctx);

        // Draw patrol editor
        self.patrol_editor.draw(ctx);

        // Draw pathfinding debug overlay
        self.pathfinding_debug.draw(ctx);

        // Draw event bus monitor
        self.event_bus_monitor.draw(ctx);

        // Draw lock manager
        self.lock_manager.draw(ctx);
    }

    /// Draw the director panel with the director system
    pub fn draw_director_panel(
        &mut self,
        ctx: &egui::Context,
        director: Option<&mut dde_ai::DirectorSystem>,
    ) {
        self.director_panel.draw(ctx, director);
    }

    /// Draw the documentation panel
    pub fn draw_documentation_panel(&mut self, ctx: &egui::Context) {
        self.documentation_panel.draw(ctx);
    }

    /// Check if editor has unsaved changes
    pub fn has_unsaved_changes(&self) -> bool {
        self.tilemap_editor.is_dirty()
            || self.visual_script_editor.is_dirty()
            || self.status_effect_editor.is_dirty()
            || self.formation_editor.has_unsaved_changes()
    }

    /// Draw the status effect editor
    pub fn draw_status_effect_editor(
        &mut self,
        ctx: &egui::Context,
        interface: &mut dyn StatusEffectInterface,
    ) {
        self.status_effect_editor.draw(ctx, interface);
    }

    /// Toggle the status effect editor visibility
    pub fn toggle_status_effect_editor(&mut self) {
        self.status_effect_editor.toggle();
    }

    /// Check if status effect editor is visible
    pub fn is_status_effect_editor_visible(&self) -> bool {
        self.status_effect_editor.is_visible()
    }

    /// Draw the item database editor
    pub fn draw_item_database_editor(&mut self, ctx: &egui::Context) {
        self.item_database_editor.draw(ctx);
    }

    /// Toggle the item database editor visibility
    pub fn toggle_item_database_editor(&mut self) {
        self.item_database_editor.toggle();
    }

    /// Check if item database editor is visible
    pub fn is_item_database_editor_visible(&self) -> bool {
        self.item_database_editor.is_visible()
    }

    /// Draw the formula editor
    pub fn draw_formula_editor(&mut self, ctx: &egui::Context) {
        self.formula_editor.draw(ctx);
    }

    /// Toggle the formula editor visibility
    pub fn toggle_formula_editor(&mut self) {
        self.formula_editor.toggle();
    }

    /// Check if formula editor is visible
    pub fn is_formula_editor_visible(&self) -> bool {
        self.formula_editor.is_visible()
    }

    /// Draw the behavior tree editor
    pub fn draw_behavior_tree_editor(&mut self, ctx: &egui::Context) {
        egui::Window::new("Behavior Tree Editor")
            .default_size([1200.0, 800.0])
            .show(ctx, |ui| {
                self.behavior_tree_editor.draw_ui(ui, None);
            });
    }

    /// Draw the dialogue editor
    pub fn draw_dialogue_editor(&mut self, ctx: &egui::Context) {
        self.dialogue_editor.draw(ctx);
    }

    /// Toggle the dialogue editor visibility
    pub fn toggle_dialogue_editor(&mut self) {
        // The dialogue editor is shown via a window, so we track visibility separately if needed
        // For now, it's always drawn when requested
    }

    /// Create a new dialogue tree
    pub fn new_dialogue_tree(&mut self) {
        self.dialogue_editor.new_tree();
    }

    /// Load a dialogue tree into the editor
    pub fn load_dialogue_tree(&mut self, tree: dde_core::systems::dialogue::DialogueTree) {
        self.dialogue_editor.load_tree(tree);
    }

    /// Get reference to dialogue editor
    pub fn dialogue_editor(&self) -> &DialogueEditor {
        &self.dialogue_editor
    }

    /// Get mutable reference to dialogue editor
    pub fn dialogue_editor_mut(&mut self) -> &mut DialogueEditor {
        &mut self.dialogue_editor
    }

    /// Draw the save/backup browser window
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // In your main draw loop:
    /// if editor.save_browser.is_visible() {
    ///     editor.draw_save_browser(ctx);
    /// }
    ///
    /// // Handle save/load requests:
    /// if let Some(slot) = editor.save_browser.take_load_request() {
    ///     game_state.load_from_slot(slot);
    ///     editor.save_browser.set_loaded_slot(Some(slot));
    /// }
    ///
    /// if let Some(slot) = editor.save_browser.take_save_request() {
    ///     let save_data = game_state.create_save_data(slot);
    ///     editor.save_browser.execute_save(slot, &save_data).ok();
    /// }
    /// ```
    pub fn draw_save_browser(&mut self, ctx: &egui::Context) {
        self.save_browser.draw_window(ctx);
    }

    /// Toggle the save browser visibility
    pub fn toggle_save_browser(&mut self) {
        self.save_browser.toggle();
    }

    /// Check if save browser is visible
    pub fn is_save_browser_visible(&self) -> bool {
        self.save_browser.is_visible()
    }

    // =========================================================================
    // Asset Forge Panel Methods
    // =========================================================================

    /// Draw the Asset menu with Asset Forge panel options
    /// 
    /// Call this from your main menu bar to add the Assets menu:
    /// 
    /// ```rust,ignore
    /// ui.menu_button("Assets", |ui| {
    ///     editor.draw_assets_menu(ui);
    /// });
    /// ```
    pub fn draw_assets_menu(&mut self, ui: &mut egui::Ui) {
        if ui.button("🔄 Asset Pipeline...").clicked() {
            self.asset_forge_panels.show_asset_os_pipeline();
            ui.close_menu();
        }
        
        ui.separator();
        
        if ui.button("📋 Classification Rules...").clicked() {
            self.asset_forge_panels.show_classification();
            ui.close_menu();
        }
        
        if ui.button("🔗 Dependency Graph...").clicked() {
            self.asset_forge_panels.show_dependency_graph();
            ui.close_menu();
        }
        
        if ui.button("🔍 Find Duplicates...").clicked() {
            self.asset_forge_panels.show_duplicate_scanner();
            ui.close_menu();
        }
        
        ui.separator();
        
        if ui.button("Hide All Asset Panels").clicked() {
            self.asset_forge_panels.hide_all();
            ui.close_menu();
        }
    }

    /// Show the classification rules panel
    pub fn show_classification_rules(&mut self) {
        self.asset_forge_panels.show_classification();
    }

    /// Show the dependency graph panel
    pub fn show_dependency_graph(&mut self) {
        self.asset_forge_panels.show_dependency_graph();
    }

    /// Show the duplicate scanner panel
    pub fn show_duplicate_scanner(&mut self) {
        self.asset_forge_panels.show_duplicate_scanner();
    }

    /// Toggle classification rules panel
    pub fn toggle_classification_rules(&mut self) {
        self.asset_forge_panels.classification.toggle();
    }

    /// Toggle dependency graph panel
    pub fn toggle_dependency_graph(&mut self) {
        self.asset_forge_panels.dependency_graph.toggle();
    }

    /// Toggle duplicate scanner panel
    pub fn toggle_duplicate_scanner(&mut self) {
        self.asset_forge_panels.duplicate_scanner.toggle();
    }

    /// Show the Asset OS Pipeline panel
    pub fn show_asset_os_pipeline(&mut self) {
        self.asset_forge_panels.show_asset_os_pipeline();
    }

    /// Toggle Asset OS Pipeline panel
    pub fn toggle_asset_os_pipeline(&mut self) {
        self.asset_forge_panels.asset_os_pipeline.toggle();
    }

    /// Check if classification rules panel is visible
    pub fn is_classification_rules_visible(&self) -> bool {
        self.asset_forge_panels.classification.is_visible()
    }

    /// Check if dependency graph panel is visible
    pub fn is_dependency_graph_visible(&self) -> bool {
        self.asset_forge_panels.dependency_graph.is_visible()
    }

    /// Check if duplicate scanner panel is visible
    pub fn is_duplicate_scanner_visible(&self) -> bool {
        self.asset_forge_panels.duplicate_scanner.is_visible()
    }

    /// Check if Asset OS Pipeline panel is visible
    pub fn is_asset_os_pipeline_visible(&self) -> bool {
        self.asset_forge_panels.asset_os_pipeline.is_visible()
    }

    /// Update Asset Forge panels (call each frame with delta time)
    pub fn update_asset_forge_panels(&mut self, dt: f32) {
        self.asset_forge_panels.update(dt);
    }

    /// Get mutable reference to Asset Forge panels
    pub fn asset_forge_panels_mut(&mut self) -> &mut AssetForgePanels {
        &mut self.asset_forge_panels
    }

    /// Get reference to Asset Forge panels
    pub fn asset_forge_panels(&self) -> &AssetForgePanels {
        &self.asset_forge_panels
    }

    // =========================================================================
    // Formation Editor Methods
    // =========================================================================

    /// Draw the Battle menu with Formation Editor option
    /// 
    /// Call this from your main menu bar to add the Battle menu:
    /// 
    /// ```rust,ignore
    /// ui.menu_button("Battle", |ui| {
    ///     editor.draw_battle_menu(ui);
    /// });
    /// ```
    pub fn draw_battle_menu(&mut self, ui: &mut egui::Ui) {
        if ui.button("🛡️ Formation Editor...").clicked() {
            self.formation_editor.show();
            ui.close_menu();
        }

        ui.separator();

        if ui.button("⚔️ Battle Panel...").clicked() {
            // Battle panel would be toggled here
            ui.close_menu();
        }

        if ui.button("📦 Item Database...").clicked() {
            self.item_database_editor.show();
            ui.close_menu();
        }

        if ui.button("✨ Status Effects...").clicked() {
            self.status_effect_editor.show();
            ui.close_menu();
        }

        if ui.button("📜 Battle Log...").clicked() {
            self.battle_log_viewer.show();
            ui.close_menu();
        }
    }

    /// Draw the formation editor window
    pub fn draw_formation_editor(&mut self, ctx: &egui::Context) {
        self.formation_editor.draw(ctx);
    }

    /// Toggle the formation editor visibility
    pub fn toggle_formation_editor(&mut self) {
        self.formation_editor.toggle();
    }

    /// Show the formation editor
    pub fn show_formation_editor(&mut self) {
        self.formation_editor.show();
    }

    /// Hide the formation editor
    pub fn hide_formation_editor(&mut self) {
        self.formation_editor.hide();
    }

    /// Check if formation editor is visible
    pub fn is_formation_editor_visible(&self) -> bool {
        self.formation_editor.is_visible()
    }

    /// Get mutable reference to formation editor
    pub fn formation_editor_mut(&mut self) -> &mut FormationEditor {
        &mut self.formation_editor
    }

    /// Get reference to formation editor
    pub fn formation_editor(&self) -> &FormationEditor {
        &self.formation_editor
    }

    // =========================================================================
    // Battle Panel Methods
    // =========================================================================

    /// Draw the battle panel
    /// 
    /// # Example
    ///
    /// ```rust,ignore
    /// // In your main draw loop:
    /// if editor.battle_panel.is_visible() {
    ///     editor.draw_battle_panel(ctx, &mut battle_interface);
    /// }
    /// ```
    pub fn draw_battle_panel(&mut self, ctx: &egui::Context, interface: &mut dyn battle_panel::BattleInterface) {
        self.battle_panel.draw(ctx, interface);
    }

    /// Update battle panel (call each frame)
    pub fn update_battle_panel(&mut self, dt: f32) {
        self.battle_panel.update_turn_queue(dt);
    }

    /// Update turn queue data from battle system
    // pub fn update_battle_turn_queue(&mut self, queue: &dde_battle::turn_queue::TurnQueue, world: &dde_core::World) {
    //     self.battle_panel.update_turn_queue_data(queue, world);
    // }

    /// Toggle the battle panel visibility
    pub fn toggle_battle_panel(&mut self) {
        self.battle_panel.toggle();
    }

    /// Show the battle panel
    pub fn show_battle_panel(&mut self) {
        self.battle_panel.show();
    }

    /// Hide the battle panel
    pub fn hide_battle_panel(&mut self) {
        self.battle_panel.hide();
    }

    /// Check if battle panel is visible
    pub fn is_battle_panel_visible(&self) -> bool {
        self.battle_panel.is_visible()
    }

    /// Get mutable reference to battle panel
    pub fn battle_panel_mut(&mut self) -> &mut BattlePanel {
        &mut self.battle_panel
    }

    /// Get reference to battle panel
    pub fn battle_panel(&self) -> &BattlePanel {
        &self.battle_panel
    }

    /// Toggle the turn queue visualizer (standalone window)
    pub fn toggle_turn_queue_visualizer(&mut self) {
        self.battle_panel.toggle_turn_queue_visualizer();
    }

    /// Draw the standalone turn queue visualizer window
    pub fn draw_turn_queue_visualizer(&mut self, ctx: &egui::Context, selected_entity: &mut Option<dde_core::Entity>) {
        self.battle_panel.draw_turn_queue_visualizer(ctx, selected_entity);
    }

    // =========================================================================
    // Battle Log Viewer Methods
    // =========================================================================

    /// Draw the battle log viewer with a custom interface
    /// 
    /// # Example
    /// 
    /// ```rust,ignore
    /// // In your main draw loop:
    /// editor.draw_battle_log_viewer(ctx, &my_battle_interface);
    /// ```
    pub fn draw_battle_log_viewer(&mut self, ctx: &egui::Context, interface: &dyn BattleLogInterface) {
        self.battle_log_viewer.draw(ctx, interface);
    }

    /// Toggle the battle log viewer visibility
    pub fn toggle_battle_log_viewer(&mut self) {
        self.battle_log_viewer.toggle();
    }

    /// Show the battle log viewer
    pub fn show_battle_log_viewer(&mut self) {
        self.battle_log_viewer.show();
    }

    /// Hide the battle log viewer
    pub fn hide_battle_log_viewer(&mut self) {
        self.battle_log_viewer.hide();
    }

    /// Check if battle log viewer is visible
    pub fn is_battle_log_viewer_visible(&self) -> bool {
        self.battle_log_viewer.is_visible()
    }

    /// Get mutable reference to battle log viewer
    pub fn battle_log_viewer_mut(&mut self) -> &mut BattleLogViewer {
        &mut self.battle_log_viewer
    }

    /// Get reference to battle log viewer
    pub fn battle_log_viewer(&self) -> &BattleLogViewer {
        &self.battle_log_viewer
    }

    /// Update battle log viewer replay state (call each frame)
    pub fn update_battle_log_viewer(&mut self, dt: f32, interface: &dyn BattleLogInterface) {
        self.battle_log_viewer.update(dt, interface);
    }

    /// Set formation editor party members from interface
    pub fn update_formation_party(&mut self, interface: &dyn formation_editor::FormationEditorInterface) {
        let members = interface.get_party_members();
        self.formation_editor.set_party_members(members);
    }

    // =========================================================================
    // Script Manager Methods
    // =========================================================================

    /// Draw the Tools menu with Script Manager option
    /// 
    /// Call this from your main menu bar to add the Tools menu:
    /// 
    /// ```rust,ignore
    /// ui.menu_button("Tools", |ui| {
    ///     editor.draw_tools_menu(ui);
    /// });
    /// ```
    pub fn draw_tools_menu(&mut self, ui: &mut egui::Ui) {
        if ui.button("🗨 Dialogue Editor...").clicked() {
            self.dialogue_editor.new_tree();
            ui.close_menu();
        }

        if ui.button("📜 Script Manager...").clicked() {
            self.script_manager.show();
            ui.close_menu();
        }

        ui.separator();

        if ui.button("🔄 Hot Reload Panel...").clicked() {
            self.hot_reload_panel.show();
            ui.close_menu();
        }

        if ui.button("🎬 Replay Theater...").clicked() {
            self.replay_panel.show();
            ui.close_menu();
        }

        if ui.button("🌐 Sync Panel...").clicked() {
            self.sync_panel.show();
            ui.close_menu();
        }
    }

    /// Draw the script manager panel
    /// 
    /// # Example
    ///
    /// ```rust,ignore
    /// // In your main draw loop:
    /// if editor.script_manager.is_visible() {
    ///     editor.draw_script_manager(ctx, script_backend);
    /// }
    /// ```
    pub fn draw_script_manager(&mut self, ctx: &egui::Context, backend: &mut dyn ScriptManagerBackend) {
        self.script_manager.draw(ctx, backend);
    }

    /// Update the script manager panel
    /// 
    /// Call this each frame to update the panel state
    pub fn update_script_manager(&mut self, dt: f32, backend: &mut dyn ScriptManagerBackend) {
        self.script_manager.update(dt, backend);
    }

    /// Toggle the script manager visibility
    pub fn toggle_script_manager(&mut self) {
        self.script_manager.toggle();
    }

    /// Show the script manager
    pub fn show_script_manager(&mut self) {
        self.script_manager.show();
    }

    /// Hide the script manager
    pub fn hide_script_manager(&mut self) {
        self.script_manager.hide();
    }

    /// Check if script manager is visible
    pub fn is_script_manager_visible(&self) -> bool {
        self.script_manager.is_visible()
    }

    /// Get mutable reference to script manager
    pub fn script_manager_mut(&mut self) -> &mut ScriptManagerPanel {
        &mut self.script_manager
    }

    /// Get reference to script manager
    pub fn script_manager(&self) -> &ScriptManagerPanel {
        &self.script_manager
    }

    /// Set external editor path for script manager
    pub fn set_script_editor(&mut self, path: Option<std::path::PathBuf>) {
        self.script_manager.set_external_editor(path);
    }

    // =========================================================================
    // Particle Editor Methods
    // =========================================================================

    /// Draw the Effects menu with Particle Editor option
    ///
    /// Call this from your main menu bar to add the Effects menu:
    ///
    /// ```rust,ignore
    /// ui.menu_button("Effects", |ui| {
    ///     editor.draw_effects_menu(ui);
    /// });
    /// ```
    pub fn draw_effects_menu(&mut self, ui: &mut egui::Ui) {
        if ui.button("✨ Particle Editor...").clicked() {
            self.particle_editor.show();
            ui.close_menu();
        }
        
        ui.separator();
        
        if ui.button("Hide All Effect Panels").clicked() {
            self.particle_editor.hide();
            ui.close_menu();
        }
    }

    /// Draw the particle editor
    pub fn draw_particle_editor(&mut self, ctx: &egui::Context) {
        self.particle_editor.draw(ctx);
    }

    /// Toggle the particle editor visibility
    pub fn toggle_particle_editor(&mut self) {
        self.particle_editor.toggle();
    }

    /// Show the particle editor
    pub fn show_particle_editor(&mut self) {
        self.particle_editor.show();
    }

    /// Hide the particle editor
    pub fn hide_particle_editor(&mut self) {
        self.particle_editor.hide();
    }

    /// Check if particle editor is visible
    pub fn is_particle_editor_visible(&self) -> bool {
        self.particle_editor.is_visible()
    }

    /// Update particle editor (call each frame with delta time)
    pub fn update_particle_editor(&mut self, dt: f32) {
        self.particle_editor.update(dt);
    }

    /// Get mutable reference to particle editor
    pub fn particle_editor_mut(&mut self) -> &mut ParticleEditor {
        &mut self.particle_editor
    }

    /// Get reference to particle editor
    pub fn particle_editor(&self) -> &ParticleEditor {
        &self.particle_editor
    }

    // =========================================================================
    // NPC Schedule Editor Methods
    // =========================================================================

    /// Draw the NPC menu with Schedule Editor and Patrol Path Editor options
    /// 
    /// Call this from your main menu bar to add the NPC menu:
    /// 
    /// ```rust,ignore
    /// ui.menu_button("NPC", |ui| {
    ///     editor.draw_npc_menu(ui);
    /// });
    /// ```
    pub fn draw_npc_menu(&mut self, ui: &mut egui::Ui) {
        if ui.button("🗓️ Schedule Editor...").clicked() {
            self.schedule_editor.show();
            ui.close_menu();
        }

        if ui.button("🚶 Patrol Paths...").clicked() {
            self.patrol_editor.show();
            ui.close_menu();
        }

        ui.separator();

        if ui.button("👥 Manage NPCs...").clicked() {
            ui.close_menu();
        }
    }

    /// Show the schedule editor
    pub fn show_schedule_editor(&mut self) {
        self.schedule_editor.show();
    }

    /// Hide the schedule editor
    pub fn hide_schedule_editor(&mut self) {
        self.schedule_editor.hide();
    }

    /// Toggle the schedule editor visibility
    pub fn toggle_schedule_editor(&mut self) {
        self.schedule_editor.toggle();
    }

    /// Check if schedule editor is visible
    pub fn is_schedule_editor_visible(&self) -> bool {
        self.schedule_editor.is_visible()
    }

    /// Get mutable reference to schedule editor
    pub fn schedule_editor_mut(&mut self) -> &mut ScheduleEditor {
        &mut self.schedule_editor
    }

    /// Get reference to schedule editor
    pub fn schedule_editor(&self) -> &ScheduleEditor {
        &self.schedule_editor
    }

    /// Add an NPC to the schedule editor
    pub fn add_npc_to_schedule(&mut self, npc: NpcInfo) {
        self.schedule_editor.add_npc(npc);
    }

    /// Get NPC schedule (immutable)
    pub fn get_npc_schedule(&self, npc_id: u64) -> Option<&dde_core::pathfinding::NpcSchedule> {
        self.schedule_editor.get_npc_schedule(npc_id)
    }

    /// Set NPC schedule
    pub fn set_npc_schedule(&mut self, npc_id: u64, schedule: dde_core::pathfinding::NpcSchedule) {
        self.schedule_editor.set_npc_schedule(npc_id, schedule);
    }

    // =========================================================================
    // Pathfinding Debug Methods
    // =========================================================================

    /// Toggle the pathfinding debug overlay
    pub fn toggle_pathfinding_debug(&mut self) {
        self.pathfinding_debug.toggle();
    }

    /// Show the pathfinding debug overlay
    pub fn show_pathfinding_debug(&mut self) {
        self.pathfinding_debug.show();
    }

    /// Hide the pathfinding debug overlay
    pub fn hide_pathfinding_debug(&mut self) {
        self.pathfinding_debug.hide();
    }

    /// Check if pathfinding debug is visible
    pub fn is_pathfinding_debug_visible(&self) -> bool {
        self.pathfinding_debug.is_visible()
    }

    /// Get mutable reference to pathfinding debug panel
    pub fn pathfinding_debug_mut(&mut self) -> &mut PathfindingDebugPanel {
        &mut self.pathfinding_debug
    }

    /// Get reference to pathfinding debug panel
    pub fn pathfinding_debug(&self) -> &PathfindingDebugPanel {
        &self.pathfinding_debug
    }

    /// Draw the Debug menu with Pathfinding Debug option
    /// 
    /// Call this from your main menu bar to add the Debug menu:
    /// 
    /// ```rust,ignore
    /// ui.menu_button("Debug", |ui| {
    ///     editor.draw_debug_menu(ui);
    /// });
    /// ```
    pub fn draw_debug_menu(&mut self, ui: &mut egui::Ui) {
        let is_visible = self.pathfinding_debug.is_visible();
        if ui.selectable_label(is_visible, "🔍 Pathfinding Debug...").clicked() {
            self.pathfinding_debug.toggle();
            ui.close_menu();
        }

        ui.separator();

        if ui.button("📊 Profiler...").clicked() {
            ui.close_menu();
        }

        if ui.button("🌡️ Hot Reload Panel...").clicked() {
            self.hot_reload_panel.show();
            ui.close_menu();
        }

        ui.separator();

        // Event Bus Monitor
        let is_event_monitor_visible = self.event_bus_monitor.is_visible();
        if ui.selectable_label(is_event_monitor_visible, "📡 Event Bus Monitor...").clicked() {
            self.event_bus_monitor.toggle();
            ui.close_menu();
        }
    }

    // =========================================================================
    // Event Bus Monitor Methods
    // =========================================================================

    /// Toggle the event bus monitor
    pub fn toggle_event_bus_monitor(&mut self) {
        self.event_bus_monitor.toggle();
    }

    /// Show the event bus monitor
    pub fn show_event_bus_monitor(&mut self) {
        self.event_bus_monitor.show();
    }

    /// Hide the event bus monitor
    pub fn hide_event_bus_monitor(&mut self) {
        self.event_bus_monitor.hide();
    }

    /// Check if event bus monitor is visible
    pub fn is_event_bus_monitor_visible(&self) -> bool {
        self.event_bus_monitor.is_visible()
    }

    /// Get mutable reference to event bus monitor
    pub fn event_bus_monitor_mut(&mut self) -> &mut EventBusMonitor {
        &mut self.event_bus_monitor
    }

    /// Get reference to event bus monitor
    pub fn event_bus_monitor(&self) -> &EventBusMonitor {
        &self.event_bus_monitor
    }

    /// Subscribe the event bus monitor to an event bus
    pub fn subscribe_event_monitor_to_bus(&mut self, bus: &dde_core::events::EventBus) {
        self.event_bus_monitor.subscribe_to_bus(bus);
    }

    /// Update the event bus monitor (call each frame)
    pub fn update_event_bus_monitor(&mut self, dt: f32) {
        self.event_bus_monitor.update(dt);
    }

    /// Sync pathfinding debug grid from tilemap collision data
    pub fn sync_pathfinding_from_tilemap(&mut self) {
        use tilemap::LayerType;
        
        let map = &self.tilemap_editor.map;
        let width = map.width;
        let height = map.height;
        
        // Resize the grid
        self.pathfinding_debug.resize_to_map(width, height);
        
        // Sync collision data from collision layer
        if let Some(collision_layer) = map.get_layer(LayerType::Collision) {
            for y in 0..height {
                for x in 0..width {
                    if let Some(tile) = collision_layer.get_tile(x, y) {
                        // If there's a tile in collision layer, mark as unwalkable
                        self.pathfinding_debug.set_tile_from_collision(x as i32, y as i32, !tile.empty);
                    }
                }
            }
        }
    }

    // =========================================================================
    // Patrol Path Editor Methods
    // =========================================================================

    /// Show the patrol editor
    pub fn show_patrol_editor(&mut self) {
        self.patrol_editor.show();
    }

    /// Hide the patrol editor
    pub fn hide_patrol_editor(&mut self) {
        self.patrol_editor.hide();
    }

    /// Toggle the patrol editor visibility
    pub fn toggle_patrol_editor(&mut self) {
        self.patrol_editor.toggle();
    }

    /// Check if patrol editor is visible
    pub fn is_patrol_editor_visible(&self) -> bool {
        self.patrol_editor.is_visible()
    }

    /// Get mutable reference to patrol editor
    pub fn patrol_editor_mut(&mut self) -> &mut PatrolEditor {
        &mut self.patrol_editor
    }

    /// Get reference to patrol editor
    pub fn patrol_editor(&self) -> &PatrolEditor {
        &self.patrol_editor
    }

    /// Create a new patrol path
    pub fn create_patrol_path(&mut self, name: Option<String>) -> crate::patrol_editor::PatrolPathId {
        self.patrol_editor.create_path(name)
    }

    /// Delete a patrol path
    pub fn delete_patrol_path(&mut self, id: crate::patrol_editor::PatrolPathId) {
        self.patrol_editor.delete_path(id);
    }

    /// Add an NPC to the patrol editor
    pub fn add_npc_to_patrol_editor(&mut self, npc: crate::patrol_editor::NpcInfo) {
        self.patrol_editor.add_npc(npc);
    }

    /// Assign a patrol path to an NPC
    pub fn assign_patrol_to_npc(&mut self, path_id: crate::patrol_editor::PatrolPathId, npc_id: u64) -> bool {
        self.patrol_editor.assign_path_to_npc(path_id, npc_id)
    }

    /// Unassign a patrol path from an NPC
    pub fn unassign_patrol_from_npc(&mut self, npc_id: u64) -> bool {
        self.patrol_editor.unassign_path_from_npc(npc_id)
    }

    // =========================================================================
    // Lock Manager Methods
    // =========================================================================

    /// Draw the Collaboration menu with Lock Manager option
    /// 
    /// Call this from your main menu bar to add the Collaboration menu:
    /// 
    /// ```rust,ignore
    /// ui.menu_button("Collaboration", |ui| {
    ///     editor.draw_collaboration_menu(ui);
    /// });
    /// ```
    pub fn draw_collaboration_menu(&mut self, ui: &mut egui::Ui) {
        if ui.button("🔒 Lock Manager...").clicked() {
            self.lock_manager.toggle();
            ui.close_menu();
        }

        ui.separator();

        if ui.button("🤝 Show Collaboration Panel").clicked() {
            // The collaboration panel is drawn via draw_ui
            ui.close_menu();
        }

        ui.separator();

        if ui.button("🌐 Sync Panel...").clicked() {
            self.sync_panel.show();
            ui.close_menu();
        }
    }

    /// Draw the lock manager window
    pub fn draw_lock_manager(&mut self, ctx: &egui::Context) {
        self.lock_manager.draw(ctx);
    }

    /// Toggle the lock manager visibility
    pub fn toggle_lock_manager(&mut self) {
        self.lock_manager.toggle();
    }

    /// Show the lock manager
    pub fn show_lock_manager(&mut self) {
        self.lock_manager.show();
    }

    /// Hide the lock manager
    pub fn hide_lock_manager(&mut self) {
        self.lock_manager.hide();
    }

    /// Check if lock manager is visible
    pub fn is_lock_manager_visible(&self) -> bool {
        self.lock_manager.is_visible()
    }

    /// Get mutable reference to lock manager
    pub fn lock_manager_mut(&mut self) -> &mut LockManagerPanel {
        &mut self.lock_manager
    }

    /// Get reference to lock manager
    pub fn lock_manager(&self) -> &LockManagerPanel {
        &self.lock_manager
    }

    /// Update lock manager (call each frame with delta time)
    pub fn update_lock_manager(&mut self, dt: f32) {
        self.lock_manager.update(dt);
    }

    /// Set lock manager admin mode
    pub fn set_lock_manager_admin(&mut self, is_admin: bool) {
        self.lock_manager.set_admin(is_admin);
    }

    /// Set lock manager client ID
    pub fn set_lock_manager_client_id(&mut self, client_id: Uuid) {
        self.lock_manager.set_client_id(client_id);
    }

    /// Draw a lock indicator for an entity (for use in other editors)
    pub fn draw_entity_lock_indicator(&self, ui: &mut egui::Ui, entity: Entity, rect: egui::Rect) {
        self.lock_manager.draw_lock_indicator(ui, entity, rect);
    }

    /// Check if an entity is locked
    pub fn is_entity_locked(&self, entity: Entity) -> bool {
        self.lock_manager.is_locked(entity)
    }

    /// Get lock info for an entity
    pub fn get_entity_lock_info(&self, entity: Entity) -> Option<dde_sync::lock::LockInfo> {
        self.lock_manager.get_lock_info(entity)
    }

    /// Get the color for a lock indicator
    pub fn get_lock_indicator_color(&self, entity: Entity) -> Option<egui::Color32> {
        self.lock_manager.get_lock_indicator_color(entity)
    }
}

impl Default for Editor {
    fn default() -> Self {
        Self::new()
    }
}

/// Null implementation of BattleLogInterface for when no battle is active
pub struct NullBattleLogInterface;

impl BattleLogInterface for NullBattleLogInterface {
    fn get_battle_log(&self) -> Option<&BattleLog> {
        None
    }

    fn get_combatant_name(&self, entity: Entity) -> String {
        format!("Entity{}", entity.id())
    }

    fn is_battle_active(&self) -> bool {
        false
    }

    fn current_turn(&self) -> u32 {
        0
    }
}
