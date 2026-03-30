//! Script Manager Browser Panel
//!
//! Comprehensive UI for managing Lua scripts with:
//! - File browser for all script types
//! - Script metadata display (name, description, author, dependencies, last modified)
//! - Actions: Create, Edit, Duplicate, Delete, Organize in folders
//! - Hot reload integration with status display
//! - Script validation (syntax check, API usage check)
//!
//! UI Layout:
//! - Left: Folder tree
//! - Center: File list with metadata
//! - Right: Preview/details
//! - Bottom: Error log

use dde_lua::scripts::{
    ErrorType, ReloadStatus, ScriptFolder, ScriptMetadata, ScriptTemplate, ScriptType,
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

// ============================================================================
// Data Structures
// ============================================================================

/// Script Manager Browser panel
pub struct ScriptManagerPanel {
    /// Panel visibility
    visible: bool,
    /// Currently selected folder
    selected_folder: String,
    /// Currently selected script ID
    selected_script: Option<i64>,
    /// Folder expansion state
    folder_expanded: HashMap<String, bool>,
    /// Script filter/search text
    filter_text: String,
    /// Type filter
    type_filter: Option<ScriptType>,
    /// Sort column
    sort_by: SortColumn,
    /// Sort ascending/descending
    sort_ascending: bool,
    /// View mode (list/icons)
    view_mode: ViewMode,
    /// New script dialog state
    new_script_dialog: Option<NewScriptDialog>,
    /// Delete confirmation dialog
    delete_confirm: Option<i64>,
    /// Rename dialog state
    rename_dialog: Option<RenameDialog>,
    /// Error log expanded
    error_log_expanded: bool,
    /// Status message
    status_message: Option<(String, StatusType, Instant)>,
    /// Last update time
    last_update: Instant,
    /// Script content preview (cached)
    preview_cache: HashMap<i64, String>,
    /// Selected scripts for multi-select
    selected_scripts: Vec<i64>,
    /// Drag and drop state
    drag_state: Option<DragState>,
    /// Context menu position
    context_menu_pos: Option<egui::Pos2>,
    /// External editor path
    external_editor: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SortColumn {
    Name,
    Type,
    Modified,
    Status,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ViewMode {
    List,
    Icons,
    Details,
}

#[derive(Debug, Clone)]
struct NewScriptDialog {
    name: String,
    description: String,
    script_type: ScriptType,
    folder: String,
    template_idx: usize,
}

#[derive(Debug, Clone)]
struct RenameDialog {
    script_id: i64,
    new_name: String,
}

#[derive(Debug, Clone)]
struct DragState {
    script_id: i64,
    source_folder: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StatusType {
    Info,
    Success,
    Warning,
    Error,
}

impl StatusType {
    fn color(&self) -> egui::Color32 {
        match self {
            StatusType::Info => egui::Color32::LIGHT_BLUE,
            StatusType::Success => egui::Color32::GREEN,
            StatusType::Warning => egui::Color32::YELLOW,
            StatusType::Error => egui::Color32::RED,
        }
    }
}

// ============================================================================
// Trait Interfaces for Backend Integration
// ============================================================================

/// Interface for script manager backend operations
pub trait ScriptManagerBackend {
    /// Get all scripts
    fn get_scripts(&self) -> Vec<&ScriptMetadata>;
    /// Get scripts in a specific folder
    fn get_scripts_in_folder(&self, folder: &str) -> Vec<&ScriptMetadata>;
    /// Get script by ID
    fn get_script(&self, id: i64) -> Option<&ScriptMetadata>;
    /// Get mutable script
    fn get_script_mut(&mut self, id: i64) -> Option<&mut ScriptMetadata>;
    /// Create new script from template
    fn create_script(&mut self, template: &ScriptTemplate, folder: &str) -> Result<ScriptMetadata, String>;
    /// Delete a script
    fn delete_script(&mut self, id: i64) -> Result<(), String>;
    /// Duplicate a script
    fn duplicate_script(&mut self, id: i64, new_name: &str) -> Result<ScriptMetadata, String>;
    /// Rename a script
    fn rename_script(&mut self, id: i64, new_name: &str) -> Result<(), String>;
    /// Move script to folder
    fn move_script(&mut self, id: i64, folder: &str) -> Result<(), String>;
    /// Validate script
    fn validate_script(&mut self, id: i64) -> ValidationResult;
    /// Reload script
    fn reload_script(&mut self, id: i64) -> Result<(), String>;
    /// Reload all modified scripts
    fn reload_all(&mut self) -> Vec<(i64, Result<(), String>)>;
    /// Get all folders
    fn get_folders(&self) -> Vec<&ScriptFolder>;
    /// Create new folder
    fn create_folder(&mut self, name: &str, parent: &str) -> Result<String, String>;
    /// Delete folder
    fn delete_folder(&mut self, path: &str, move_scripts_to_parent: bool) -> Result<(), String>;
    /// Toggle folder expansion
    fn toggle_folder(&mut self, path: &str);
    /// Get error log
    fn get_error_log(&self) -> Vec<ScriptErrorEntry>;
    /// Clear error log
    fn clear_error_log(&mut self);
    /// Open script in external editor
    fn open_in_external_editor(&self, id: i64) -> Result<(), String>;
    /// Get script source content
    fn get_script_source(&self, id: i64) -> Option<String>;
}

/// Validation result
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<String>,
}

/// Validation error
#[derive(Debug, Clone)]
pub struct ValidationError {
    pub line: usize,
    pub message: String,
    pub error_type: ErrorType,
}

/// Error entry for display
#[derive(Debug, Clone)]
pub struct ScriptErrorEntry {
    pub timestamp: i64,
    pub script_id: i64,
    pub script_name: String,
    pub error_type: ErrorType,
    pub message: String,
    pub line: Option<usize>,
}

// ============================================================================
// Implementation
// ============================================================================

impl Default for ScriptManagerPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl ScriptManagerPanel {
    /// Create a new script manager panel
    pub fn new() -> Self {
        let mut folder_expanded = HashMap::new();
        folder_expanded.insert("/".to_string(), true);
        folder_expanded.insert("/npc".to_string(), true);
        folder_expanded.insert("/quest".to_string(), true);
        folder_expanded.insert("/ai".to_string(), true);
        folder_expanded.insert("/events".to_string(), true);
        folder_expanded.insert("/entities".to_string(), true);
        folder_expanded.insert("/utility".to_string(), true);
        folder_expanded.insert("/global".to_string(), true);

        Self {
            visible: false,
            selected_folder: "/".to_string(),
            selected_script: None,
            folder_expanded,
            filter_text: String::new(),
            type_filter: None,
            sort_by: SortColumn::Name,
            sort_ascending: true,
            view_mode: ViewMode::List,
            new_script_dialog: None,
            delete_confirm: None,
            rename_dialog: None,
            error_log_expanded: false,
            status_message: None,
            last_update: Instant::now(),
            preview_cache: HashMap::new(),
            selected_scripts: Vec::new(),
            drag_state: None,
            context_menu_pos: None,
            external_editor: None,
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

    /// Set external editor path
    pub fn set_external_editor(&mut self, path: Option<PathBuf>) {
        self.external_editor = path;
    }

    /// Update panel state
    pub fn update(&mut self, _dt: f32, backend: &mut dyn ScriptManagerBackend) {
        // Clear old status messages
        if let Some((_, _, timestamp)) = &self.status_message {
            if timestamp.elapsed() > Duration::from_secs(5) {
                self.status_message = None;
            }
        }

        // Update preview cache periodically
        let now = Instant::now();
        if now.duration_since(self.last_update) > Duration::from_millis(500) {
            if let Some(script_id) = self.selected_script {
                if !self.preview_cache.contains_key(&script_id) {
                    if let Some(source) = backend.get_script_source(script_id) {
                        self.preview_cache.insert(script_id, source);
                    }
                }
            }
            self.last_update = now;
        }
    }

    /// Show status message
    fn show_status(&mut self, message: &str, status_type: StatusType) {
        self.status_message = Some((message.to_string(), status_type, Instant::now()));
    }

    /// Draw the script manager panel
    pub fn draw(&mut self, ctx: &egui::Context, backend: &mut dyn ScriptManagerBackend) {
        if !self.visible {
            return;
        }

        let mut visible = self.visible;
        egui::Window::new("📜 Script Manager")
            .open(&mut visible)
            .resizable(true)
            .default_size([1200.0, 800.0])
            .min_size([800.0, 500.0])
            .show(ctx, |ui| {
                self.draw_panel_content(ui, backend);
            });
        self.visible = visible;
    }

    fn draw_panel_content(&mut self, ui: &mut egui::Ui, backend: &mut dyn ScriptManagerBackend) {
        // Toolbar
        self.draw_toolbar(ui, backend);
        ui.separator();

        // Main content area with 3 panels
        egui::SidePanel::left("script_folders")
            .resizable(true)
            .default_width(200.0)
            .show_inside(ui, |ui| {
                self.draw_folder_tree(ui, backend);
            });

        egui::SidePanel::right("script_details")
            .resizable(true)
            .default_width(300.0)
            .show_inside(ui, |ui| {
                self.draw_details_panel(ui, backend);
            });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            self.draw_file_list(ui, backend);
        });

        // Bottom error log panel
        if self.error_log_expanded {
            egui::TopBottomPanel::bottom("error_log")
                .resizable(true)
                .default_height(150.0)
                .show_inside(ui, |ui| {
                    self.draw_error_log(ui, backend);
                });
        }

        // Draw dialogs
        if let Some(dialog) = self.new_script_dialog.clone() {
            self.draw_new_script_dialog(ui.ctx(), backend, dialog);
        }

        if let Some(script_id) = self.delete_confirm {
            self.draw_delete_dialog(ui.ctx(), backend, script_id);
        }

        if let Some(dialog) = self.rename_dialog.clone() {
            self.draw_rename_dialog(ui.ctx(), backend, dialog);
        }

        // Status message
        if let Some((msg, status_type, _)) = &self.status_message {
            ui.separator();
            ui.colored_label(status_type.color(), msg.as_str());
        }
    }

    // ========================================================================
    // Toolbar
    // ========================================================================

    fn draw_toolbar(&mut self, ui: &mut egui::Ui, backend: &mut dyn ScriptManagerBackend) {
        ui.horizontal(|ui| {
            ui.heading("Script Manager");
            ui.separator();

            // New script button
            if ui.button("➕ New Script").clicked() {
                self.new_script_dialog = Some(NewScriptDialog {
                    name: "New Script".to_string(),
                    description: String::new(),
                    script_type: ScriptType::Utility,
                    folder: self.selected_folder.clone(),
                    template_idx: 0,
                });
            }

            // New folder button
            if ui.button("📁 New Folder").clicked() {
                let folder_name = format!("NewFolder{}", backend.get_folders().len());
                match backend.create_folder(&folder_name, &self.selected_folder) {
                    Ok(path) => {
                        self.folder_expanded.insert(path.clone(), true);
                        self.show_status(&format!("Created folder: {}", folder_name), StatusType::Success);
                    }
                    Err(e) => self.show_status(&format!("Error: {}", e), StatusType::Error),
                }
            }

            ui.separator();

            // View mode buttons
            ui.label("View:");
            if ui.selectable_label(self.view_mode == ViewMode::List, "☰").clicked() {
                self.view_mode = ViewMode::List;
            }
            if ui.selectable_label(self.view_mode == ViewMode::Icons, "⊞").clicked() {
                self.view_mode = ViewMode::Icons;
            }
            if ui.selectable_label(self.view_mode == ViewMode::Details, "☷").clicked() {
                self.view_mode = ViewMode::Details;
            }

            ui.separator();

            // Reload all button
            if ui.button("🔄 Reload All").clicked() {
                let results = backend.reload_all();
                let success = results.iter().filter(|(_, r)| r.is_ok()).count();
                let total = results.len();
                if success == total {
                    self.show_status(&format!("Reloaded {}/{} scripts", success, total), StatusType::Success);
                } else {
                    self.show_status(&format!("Reloaded {}/{} scripts (some failed)", success, total), StatusType::Warning);
                }
            }

            ui.separator();

            // Error log toggle
            let error_count = backend.get_error_log().len();
            let error_text = if error_count > 0 {
                format!("⚠️ Errors ({})", error_count)
            } else {
                "📋 Error Log".to_string()
            };
            if ui.selectable_label(self.error_log_expanded, error_text).clicked() {
                self.error_log_expanded = !self.error_log_expanded;
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Search box
                ui.label("🔍");
                ui.add(egui::TextEdit::singleline(&mut self.filter_text).hint_text("Search..."));
            });
        });

        // Second row - type filter
        ui.horizontal(|ui| {
            ui.label("Filter by type:");
            if ui.selectable_label(self.type_filter.is_none(), "All").clicked() {
                self.type_filter = None;
            }
            for script_type in ScriptType::all_types() {
                let selected = self.type_filter == Some(*script_type);
                if ui.selectable_label(selected, format!("{} {}", script_type.icon(), script_type.display_name())).clicked() {
                    self.type_filter = if selected { None } else { Some(*script_type) };
                }
            }
        });
    }

    // ========================================================================
    // Folder Tree (Left Panel)
    // ========================================================================

    fn draw_folder_tree(&mut self, ui: &mut egui::Ui, backend: &mut dyn ScriptManagerBackend) {
        ui.heading("📁 Folders");
        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            self.draw_folder_node(ui, backend, "/", "Scripts");
        });
    }

    fn draw_folder_node(&mut self, ui: &mut egui::Ui, backend: &mut dyn ScriptManagerBackend, path: &str, name: &str) {
        let is_selected = self.selected_folder == path;
        let expanded = *self.folder_expanded.get(path).unwrap_or(&true);
        
        let folder_icon = if expanded { "📂" } else { "📁" };
        let label_text = format!("{} {}", folder_icon, name);

        let response = ui.selectable_label(is_selected, label_text);

        if response.clicked() {
            self.selected_folder = path.to_string();
            self.selected_script = None;
            self.selected_scripts.clear();
        }

        // Context menu for folder
        response.context_menu(|ui| {
            if ui.button("📁 New Folder").clicked() {
                let folder_name = format!("NewFolder{}", backend.get_folders().len());
                if let Err(e) = backend.create_folder(&folder_name, path) {
                    self.show_status(&format!("Error: {}", e), StatusType::Error);
                }
                ui.close_menu();
            }
            if path != "/" {
                ui.separator();
                if ui.button("🗑️ Delete Folder").clicked() {
                    if let Err(e) = backend.delete_folder(path, true) {
                        self.show_status(&format!("Error: {}", e), StatusType::Error);
                    }
                    ui.close_menu();
                }
            }
        });

        // Expand/collapse on double click
        if response.double_clicked() {
            let new_expanded = !expanded;
            self.folder_expanded.insert(path.to_string(), new_expanded);
        }

        // Draw children if expanded
        if expanded {
            ui.indent(path, |ui| {
                // Get child folders from backend
                let children: Vec<_> = backend.get_folders()
                    .into_iter()
                    .filter(|f| f.parent.as_ref() == Some(&path.to_string()))
                    .cloned()
                    .collect();
                
                for child in children {
                    self.draw_folder_node(ui, backend, &child.path, &child.name);
                }

                // Also show scripts count in this folder
                let script_count = backend.get_scripts_in_folder(path).len();
                if script_count > 0 {
                    ui.label(format!("   ({}) items", script_count))
                        .on_hover_text(format!("{} scripts in this folder", script_count));
                }
            });
        }
    }

    // ========================================================================
    // File List (Center Panel)
    // ========================================================================

    fn draw_file_list(&mut self, ui: &mut egui::Ui, backend: &mut dyn ScriptManagerBackend) {
        // Get scripts to display
        let mut scripts: Vec<_> = if self.filter_text.is_empty() {
            backend.get_scripts_in_folder(&self.selected_folder)
        } else {
            backend.get_scripts()
                .into_iter()
                .filter(|s| {
                    let query = self.filter_text.to_lowercase();
                    s.name.to_lowercase().contains(&query)
                })
                .collect()
        };

        // Apply type filter
        if let Some(script_type) = self.type_filter {
            scripts.retain(|s| s.script_type == script_type);
        }

        // Sort scripts
        scripts.sort_by(|a, b| {
            let cmp = match self.sort_by {
                SortColumn::Name => a.name.cmp(&b.name),
                SortColumn::Type => a.script_type.as_str().cmp(b.script_type.as_str()),
                SortColumn::Modified => a.modified_at.cmp(&b.modified_at),
                SortColumn::Status => a.reload_status.as_str().cmp(b.reload_status.as_str()),
            };
            if self.sort_ascending { cmp } else { cmp.reverse() }
        });

        // Draw header with sort buttons
        ui.horizontal(|ui| {
            ui.label(format!("Scripts in: {}", self.selected_folder));
            ui.separator();
            ui.label(format!("({} items)", scripts.len()));
        });
        ui.separator();

        // Column headers for list view
        if self.view_mode == ViewMode::List || self.view_mode == ViewMode::Details {
            ui.horizontal(|ui| {
                let name_header = if self.sort_by == SortColumn::Name {
                    if self.sort_ascending { "Name ▲" } else { "Name ▼" }
                } else { "Name" };
                if ui.button(name_header).clicked() {
                    self.sort_by = SortColumn::Name;
                    self.sort_ascending = !self.sort_ascending;
                }
                ui.add_space(150.0);

                let type_header = if self.sort_by == SortColumn::Type {
                    if self.sort_ascending { "Type ▲" } else { "Type ▼" }
                } else { "Type" };
                if ui.button(type_header).clicked() {
                    self.sort_by = SortColumn::Type;
                    self.sort_ascending = !self.sort_ascending;
                }
                ui.add_space(100.0);

                if self.view_mode == ViewMode::Details {
                    let status_header = if self.sort_by == SortColumn::Status {
                        if self.sort_ascending { "Status ▲" } else { "Status ▼" }
                    } else { "Status" };
                    if ui.button(status_header).clicked() {
                        self.sort_by = SortColumn::Status;
                        self.sort_ascending = !self.sort_ascending;
                    }
                    ui.add_space(80.0);

                    if ui.button("Modified").clicked() {
                        self.sort_by = SortColumn::Modified;
                        self.sort_ascending = !self.sort_ascending;
                    }
                }
            });
            ui.separator();
        }

        // Script list
        egui::ScrollArea::vertical().show(ui, |ui| {
            match self.view_mode {
                ViewMode::List => self.draw_list_view(ui, backend, &scripts),
                ViewMode::Icons => self.draw_icon_view(ui, backend, &scripts),
                ViewMode::Details => self.draw_details_view(ui, backend, &scripts),
            }
        });
    }

    fn draw_list_view(&mut self, ui: &mut egui::Ui, backend: &mut dyn ScriptManagerBackend, scripts: &[&ScriptMetadata]) {
        for script in scripts {
            let is_selected = self.selected_script == Some(script.id);
            let response = ui.selectable_label(is_selected, 
                format!("{} {}", script.script_type.icon(), script.name));

            if response.clicked() {
                self.selected_script = Some(script.id);
                if !ui.input(|i| i.modifiers.ctrl) {
                    self.selected_scripts.clear();
                }
                if !self.selected_scripts.contains(&script.id) {
                    self.selected_scripts.push(script.id);
                }
            }

            if response.double_clicked() {
                self.open_in_external_editor(backend, script.id);
            }

            // Context menu
            response.context_menu(|ui| {
                self.draw_script_context_menu(ui, backend, script.id);
            });

            // Drag start
            if response.drag_started() {
                self.drag_state = Some(DragState {
                    script_id: script.id,
                    source_folder: script.folder_path.clone(),
                });
            }
        }
    }

    fn draw_icon_view(&mut self, ui: &mut egui::Ui, backend: &mut dyn ScriptManagerBackend, scripts: &[&ScriptMetadata]) {
        let icon_size = 80.0;
        let icons_per_row = (ui.available_width() / (icon_size + 10.0)) as usize;
        let icons_per_row = icons_per_row.max(1);

        egui::Grid::new("icon_grid")
            .num_columns(icons_per_row)
            .spacing([10.0, 10.0])
            .show(ui, |ui| {
                for (i, script) in scripts.iter().enumerate() {
                    if i > 0 && i % icons_per_row == 0 {
                        ui.end_row();
                    }

                    let is_selected = self.selected_script == Some(script.id);
                    
                    ui.vertical(|ui| {
                        let button_size = egui::vec2(icon_size, icon_size);
                        let response = ui.add_sized(button_size, 
                            egui::SelectableLabel::new(is_selected, 
                                egui::RichText::new(script.script_type.icon()).size(32.0)));

                        if response.clicked() {
                            self.selected_script = Some(script.id);
                        }

                        ui.label(&script.name);

                        // Status indicator
                        let status_color = match script.reload_status {
                            ReloadStatus::Loaded => egui::Color32::GREEN,
                            ReloadStatus::Error => egui::Color32::RED,
                            ReloadStatus::Modified => egui::Color32::YELLOW,
                            _ => egui::Color32::GRAY,
                        };
                        ui.colored_label(status_color, "●");
                    });
                }
            });
    }

    fn draw_details_view(&mut self, ui: &mut egui::Ui, backend: &mut dyn ScriptManagerBackend, scripts: &[&ScriptMetadata]) {
        for script in scripts {
            let is_selected = self.selected_script == Some(script.id);
            
            let response = ui.horizontal(|ui| {
                let response = ui.selectable_label(is_selected, 
                    format!("{} {}", script.script_type.icon(), script.name));
                ui.add_space(150.0);
                ui.label(script.script_type.display_name());
                ui.add_space(50.0);

                // Status
                let (status_text, status_color) = match script.reload_status {
                    ReloadStatus::Loaded => ("✓ Active", egui::Color32::GREEN),
                    ReloadStatus::Error => ("✗ Error", egui::Color32::RED),
                    ReloadStatus::Modified => ("~ Modified", egui::Color32::YELLOW),
                    ReloadStatus::Loading => ("⟳ Loading", egui::Color32::LIGHT_BLUE),
                    ReloadStatus::Reloading => ("⟳ Reloading", egui::Color32::LIGHT_BLUE),
                    ReloadStatus::Unloaded => ("○ Unloaded", egui::Color32::GRAY),
                };
                ui.colored_label(status_color, status_text);
                ui.add_space(30.0);

                // Modified date
                let modified = chrono::DateTime::from_timestamp(script.modified_at, 0)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                    .unwrap_or_else(|| "Unknown".to_string());
                ui.label(modified);

                response
            });

            if response.inner.clicked() {
                self.selected_script = Some(script.id);
            }

            if response.inner.double_clicked() {
                self.open_in_external_editor(backend, script.id);
            }

            // Context menu
            response.response.context_menu(|ui| {
                self.draw_script_context_menu(ui, backend, script.id);
            });
        }
    }

    fn draw_script_context_menu(&mut self, ui: &mut egui::Ui, backend: &mut dyn ScriptManagerBackend, script_id: i64) {
        if let Some(script) = backend.get_script(script_id) {
            ui.label(format!("{}", script.name));
            ui.separator();

            if ui.button("✏️ Edit").clicked() {
                self.open_in_external_editor(backend, script_id);
                ui.close_menu();
            }

            if ui.button("📋 Duplicate").clicked() {
                let new_name = format!("{}_Copy", script.name);
                match backend.duplicate_script(script_id, &new_name) {
                    Ok(new_script) => {
                        self.selected_script = Some(new_script.id);
                        self.show_status(&format!("Duplicated: {}", new_script.name), StatusType::Success);
                    }
                    Err(e) => self.show_status(&format!("Error: {}", e), StatusType::Error),
                }
                ui.close_menu();
            }

            ui.separator();

            if ui.button("🔄 Reload").clicked() {
                match backend.reload_script(script_id) {
                    Ok(_) => self.show_status("Script reloaded", StatusType::Success),
                    Err(e) => self.show_status(&format!("Reload failed: {}", e), StatusType::Error),
                }
                ui.close_menu();
            }

            if ui.button("✓ Validate").clicked() {
                let result = backend.validate_script(script_id);
                if result.valid {
                    self.show_status("Script is valid", StatusType::Success);
                } else {
                    self.show_status(&format!("Validation failed: {} errors", result.errors.len()), StatusType::Warning);
                }
                ui.close_menu();
            }

            ui.separator();

            if ui.button("✏️ Rename").clicked() {
                self.rename_dialog = Some(RenameDialog {
                    script_id,
                    new_name: script.name.clone(),
                });
                ui.close_menu();
            }

            if ui.button("🗑️ Delete").clicked() {
                self.delete_confirm = Some(script_id);
                ui.close_menu();
            }
        }
    }

    fn open_in_external_editor(&self, backend: &mut dyn ScriptManagerBackend, script_id: i64) {
        if let Err(e) = backend.open_in_external_editor(script_id) {
            // Can't show status here since we don't have mutable self
            tracing::error!("Failed to open external editor: {}", e);
        }
    }

    // ========================================================================
    // Details Panel (Right Panel)
    // ========================================================================

    fn draw_details_panel(&mut self, ui: &mut egui::Ui, backend: &mut dyn ScriptManagerBackend) {
        if let Some(script_id) = self.selected_script {
            if let Some(script) = backend.get_script(script_id) {
                self.draw_script_details(ui, backend, script);
            } else {
                ui.label("Script not found");
            }
        } else {
            ui.vertical_centered(|ui| {
                ui.add_space(50.0);
                ui.label(egui::RichText::new("Select a script").weak().size(16.0));
                ui.label("to view details");
            });
        }
    }

    fn draw_script_details(&mut self, ui: &mut egui::Ui, backend: &mut dyn ScriptManagerBackend, script: &ScriptMetadata) {
        // Header with icon and name
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(script.script_type.icon()).size(24.0));
            ui.heading(&script.name);
        });
        ui.separator();

        // Status badge
        let (status_text, status_color) = match script.reload_status {
            ReloadStatus::Loaded => ("● Active", egui::Color32::GREEN),
            ReloadStatus::Error => ("● Error", egui::Color32::RED),
            ReloadStatus::Modified => ("● Modified", egui::Color32::YELLOW),
            ReloadStatus::Loading => ("● Loading", egui::Color32::LIGHT_BLUE),
            ReloadStatus::Reloading => ("● Reloading", egui::Color32::LIGHT_BLUE),
            ReloadStatus::Unloaded => ("● Unloaded", egui::Color32::GRAY),
        };
        ui.colored_label(status_color, status_text);
        ui.add_space(10.0);

        // Metadata
        egui::Grid::new("metadata_grid")
            .num_columns(2)
            .spacing([10.0, 5.0])
            .show(ui, |ui| {
                ui.label("Type:");
                ui.label(script.script_type.display_name());
                ui.end_row();

                ui.label("ID:");
                ui.label(script.id.to_string());
                ui.end_row();

                ui.label("Folder:");
                ui.label(&script.folder_path);
                ui.end_row();

                if let Some(author) = &script.author {
                    ui.label("Author:");
                    ui.label(author);
                    ui.end_row();
                }

                if let Some(desc) = &script.description {
                    ui.label("Description:");
                    ui.label(desc);
                    ui.end_row();
                }

                ui.label("Modified:");
                let modified = chrono::DateTime::from_timestamp(script.modified_at, 0)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                    .unwrap_or_else(|| "Unknown".to_string());
                ui.label(modified);
                ui.end_row();

                ui.label("Validation:");
                let (valid_text, valid_color) = if script.syntax_valid && script.api_valid {
                    ("✓ Valid", egui::Color32::GREEN)
                } else if script.syntax_valid {
                    ("⚠ API Issues", egui::Color32::YELLOW)
                } else {
                    ("✗ Syntax Errors", egui::Color32::RED)
                };
                ui.colored_label(valid_color, valid_text);
                ui.end_row();
            });

        ui.separator();

        // Dependencies
        ui.label("Dependencies:");
        if script.dependencies.is_empty() {
            ui.label(egui::RichText::new("None").weak());
        } else {
            for dep in &script.dependencies {
                ui.label(format!("  • {}", dep));
            }
        }

        ui.separator();

        // Quick actions
        ui.label("Actions:");
        ui.horizontal(|ui| {
            if ui.button("✏️ Edit").clicked() {
                self.open_in_external_editor(backend, script.id);
            }
            if ui.button("🔄 Reload").clicked() {
                if let Err(e) = backend.reload_script(script.id) {
                    self.show_status(&format!("Reload failed: {}", e), StatusType::Error);
                } else {
                    self.show_status("Script reloaded", StatusType::Success);
                }
            }
            if ui.button("✓ Validate").clicked() {
                let result = backend.validate_script(script.id);
                if result.valid {
                    self.show_status("Script is valid", StatusType::Success);
                } else {
                    self.show_status(&format!("{} errors found", result.errors.len()), StatusType::Warning);
                }
            }
        });

        ui.separator();

        // Source code preview
        ui.label("Preview:");
        if let Some(source) = self.preview_cache.get(&script.id) {
            let preview = if source.len() > 500 {
                format!("{}...", &source[..500])
            } else {
                source.clone()
            };
            egui::ScrollArea::vertical()
                .max_height(200.0)
                .show(ui, |ui| {
                    ui.add(egui::TextEdit::multiline(&mut preview.clone())
                        .code_editor()
                        .desired_rows(10)
                        .interactive(false));
                });
        } else {
            ui.label("Loading preview...");
            if let Some(source) = backend.get_script_source(script.id) {
                self.preview_cache.insert(script.id, source);
            }
        }
    }

    // ========================================================================
    // Error Log (Bottom Panel)
    // ========================================================================

    fn draw_error_log(&mut self, ui: &mut egui::Ui, backend: &mut dyn ScriptManagerBackend) {
        ui.horizontal(|ui| {
            ui.heading("Error Log");
            ui.separator();
            if ui.button("🗑️ Clear").clicked() {
                backend.clear_error_log();
                self.show_status("Error log cleared", StatusType::Success);
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("✕").clicked() {
                    self.error_log_expanded = false;
                }
            });
        });
        ui.separator();

        let errors = backend.get_error_log();
        if errors.is_empty() {
            ui.label(egui::RichText::new("No errors").weak());
        } else {
            egui::ScrollArea::vertical().show(ui, |ui| {
                for error in errors.iter().rev().take(50) {
                    self.draw_error_entry(ui, error);
                }
            });
        }
    }

    fn draw_error_entry(&mut self, ui: &mut egui::Ui, error: &ScriptErrorEntry) {
        let (icon, color) = match error.error_type {
            ErrorType::Syntax => ("🔴", egui::Color32::RED),
            ErrorType::Runtime => ("🟠", egui::Color32::from_rgb(255, 165, 0)),
            ErrorType::Api => ("🟡", egui::Color32::YELLOW),
            ErrorType::Load => ("⚪", egui::Color32::LIGHT_GRAY),
        };

        ui.horizontal(|ui| {
            ui.colored_label(color, icon);
            ui.label(&error.script_name);
            if let Some(line) = error.line {
                ui.label(format!("line {}", line));
            }
            ui.label(&error.message);
            
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let time = chrono::DateTime::from_timestamp(error.timestamp, 0)
                    .map(|dt| dt.format("%H:%M:%S").to_string())
                    .unwrap_or_default();
                ui.label(egui::RichText::new(time).weak().size(10.0));
            });
        });
    }

    // ========================================================================
    // Dialogs
    // ========================================================================

    fn draw_new_script_dialog(&mut self, ctx: &egui::Context, backend: &mut dyn ScriptManagerBackend, mut dialog: NewScriptDialog) {
        let mut open = true;
        egui::Window::new("➕ New Script")
            .open(&mut open)
            .resizable(false)
            .collapsible(false)
            .show(ctx, |ui| {
                ui.label("Script Name:");
                ui.text_edit_singleline(&mut dialog.name);
                
                ui.add_space(10.0);
                
                ui.label("Description:");
                ui.text_edit_multiline(&mut dialog.description);
                
                ui.add_space(10.0);
                
                ui.label("Type:");
                egui::ComboBox::from_id_source("script_type")
                    .selected_text(dialog.script_type.display_name())
                    .show_ui(ui, |ui| {
                        for script_type in ScriptType::all_types() {
                            ui.selectable_value(&mut dialog.script_type, *script_type, 
                                format!("{} {}", script_type.icon(), script_type.display_name()));
                        }
                    });
                
                ui.add_space(10.0);
                
                ui.label("Template:");
                let templates = ScriptTemplate::all_templates();
                let template_names: Vec<_> = templates.iter()
                    .map(|t| format!("{} - {}", t.name, t.description))
                    .collect();
                egui::ComboBox::from_id_source("script_template")
                    .selected_text(&template_names.get(dialog.template_idx).map(|s| s.as_str()).unwrap_or("Custom"))
                    .show_ui(ui, |ui| {
                        for (i, template) in templates.iter().enumerate() {
                            if ui.selectable_label(dialog.template_idx == i, 
                                format!("{} - {}", template.name, template.description)).clicked() {
                                dialog.template_idx = i;
                                dialog.script_type = template.script_type;
                            }
                        }
                    });

                ui.add_space(20.0);
                
                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        self.new_script_dialog = None;
                    }
                    
                    if ui.button("Create").clicked() && !dialog.name.is_empty() {
                        let template = &templates[dialog.template_idx];
                        let mut script_template = template.clone();
                        script_template.name = dialog.name.clone();
                        script_template.description = dialog.description.clone();
                        script_template.script_type = dialog.script_type;
                        
                        match backend.create_script(&script_template, &dialog.folder) {
                            Ok(script) => {
                                self.selected_script = Some(script.id);
                                self.show_status(&format!("Created script: {}", script.name), StatusType::Success);
                                self.new_script_dialog = None;
                            }
                            Err(e) => self.show_status(&format!("Error: {}", e), StatusType::Error),
                        }
                    }
                });
            });

        if !open {
            self.new_script_dialog = None;
        }
    }

    fn draw_delete_dialog(&mut self, ctx: &egui::Context, backend: &mut dyn ScriptManagerBackend, script_id: i64) {
        let mut open = true;
        egui::Window::new("🗑️ Delete Script")
            .open(&mut open)
            .resizable(false)
            .collapsible(false)
            .show(ctx, |ui| {
                if let Some(script) = backend.get_script(script_id) {
                    ui.label(format!("Are you sure you want to delete '{}' ?", script.name));
                    ui.label("This action cannot be undone.");
                    
                    ui.add_space(20.0);
                    
                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            self.delete_confirm = None;
                        }
                        
                        if ui.button("🗑️ Delete").clicked() {
                            match backend.delete_script(script_id) {
                                Ok(_) => {
                                    if self.selected_script == Some(script_id) {
                                        self.selected_script = None;
                                    }
                                    self.selected_scripts.retain(|&id| id != script_id);
                                    self.show_status("Script deleted", StatusType::Success);
                                    self.delete_confirm = None;
                                }
                                Err(e) => self.show_status(&format!("Error: {}", e), StatusType::Error),
                            }
                        }
                    });
                } else {
                    ui.label("Script not found");
                    if ui.button("OK").clicked() {
                        self.delete_confirm = None;
                    }
                }
            });

        if !open {
            self.delete_confirm = None;
        }
    }

    fn draw_rename_dialog(&mut self, ctx: &egui::Context, backend: &mut dyn ScriptManagerBackend, mut dialog: RenameDialog) {
        let mut open = true;
        egui::Window::new("✏️ Rename Script")
            .open(&mut open)
            .resizable(false)
            .collapsible(false)
            .show(ctx, |ui| {
                ui.label("New Name:");
                ui.text_edit_singleline(&mut dialog.new_name);
                
                ui.add_space(20.0);
                
                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        self.rename_dialog = None;
                    }
                    
                    if ui.button("Rename").clicked() && !dialog.new_name.is_empty() {
                        match backend.rename_script(dialog.script_id, &dialog.new_name) {
                            Ok(_) => {
                                self.show_status("Script renamed", StatusType::Success);
                                self.rename_dialog = None;
                            }
                            Err(e) => self.show_status(&format!("Error: {}", e), StatusType::Error),
                        }
                    }
                });
            });

        if !open {
            self.rename_dialog = None;
        }
    }
}

// Implement Clone for NewScriptDialog manually
impl Clone for NewScriptDialog {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            description: self.description.clone(),
            script_type: self.script_type,
            folder: self.folder.clone(),
            template_idx: self.template_idx,
        }
    }
}

// Implement Clone for RenameDialog manually  
impl Clone for RenameDialog {
    fn clone(&self) -> Self {
        Self {
            script_id: self.script_id,
            new_name: self.new_name.clone(),
        }
    }
}

// Implement Clone for ScriptTemplate
impl Clone for ScriptTemplate {
    fn clone(&self) -> Self {
        Self {
            script_type: self.script_type,
            name: self.name.clone(),
            description: self.description.clone(),
            default_code: self.default_code.clone(),
        }
    }
}

// Implement Copy for ScriptType if not already done
impl Copy for ScriptType {}
