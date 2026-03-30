//! Visual Script Editor Window
//!
//! Main editor window integrating the visual script canvas with:
//! - File operations (New, Open, Save)
//! - Node palette sidebar (draggable node types)
//! - Property panel for selected node
//! - Compile & Test buttons
//! - Zoom controls

use crate::visual_script::{
    compile_to_events, compiler::{graph_from_json, graph_to_json}, CompiledScript, NodeCanvas, NodeGraph, ScriptExecutor,
};

/// Visual script editor state
pub struct VisualScriptEditor {
    /// The node canvas for editing
    pub canvas: NodeCanvas,
    /// Current script name
    pub script_name: String,
    /// Script description
    pub description: String,
    /// Whether the editor is active/visible
    pub active: bool,
    /// Whether there are unsaved changes
    pub dirty: bool,
    /// Current file path (if saved)
    pub file_path: Option<std::path::PathBuf>,
    /// Last compilation result
    pub last_compile_result: Option<Result<CompiledScript, String>>,
    /// Show minimap
    pub show_minimap: bool,
    /// Show grid
    pub show_grid: bool,
    /// Property panel state
    pub property_panel: PropertyPanel,
    /// Node palette filter
    pub palette_filter: String,
    /// Whether to show the node palette
    pub show_palette: bool,
    /// Whether to show the properties panel
    pub show_properties: bool,
    /// Test executor for runtime testing
    pub test_executor: Option<ScriptExecutor>,
    /// Compilation output panel
    pub show_output_panel: bool,
    /// Compilation/output messages
    pub output_messages: Vec<OutputMessage>,
}

/// Output message for the output panel
#[derive(Debug, Clone)]
pub struct OutputMessage {
    pub level: MessageLevel,
    pub text: String,
    pub timestamp: std::time::Instant,
}

/// Message severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageLevel {
    Info,
    Warning,
    Error,
    Success,
}

/// Property panel state
#[derive(Debug, Clone)]
pub struct PropertyPanel {
    /// Currently selected property tab
    pub active_tab: PropertyTab,
    /// Node type filter for search
    pub search_query: String,
}

impl Default for PropertyPanel {
    fn default() -> Self {
        Self {
            active_tab: PropertyTab::Properties,
            search_query: String::new(),
        }
    }
}

/// Property panel tabs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertyTab {
    Properties,
    Palette,
    Compile,
}

impl Default for VisualScriptEditor {
    fn default() -> Self {
        Self::new()
    }
}

impl VisualScriptEditor {
    /// Create a new visual script editor
    pub fn new() -> Self {
        Self {
            canvas: NodeCanvas::new(),
            script_name: "Untitled Script".to_string(),
            description: String::new(),
            active: false,
            dirty: false,
            file_path: None,
            last_compile_result: None,
            show_minimap: true,
            show_grid: true,
            property_panel: PropertyPanel::default(),
            palette_filter: String::new(),
            show_palette: true,
            show_properties: true,
            test_executor: None,
            show_output_panel: false,
            output_messages: Vec::new(),
        }
    }

    /// Create editor with an existing graph
    pub fn with_graph(graph: NodeGraph, name: impl Into<String>) -> Self {
        let mut editor = Self::new();
        editor.canvas.load_graph(graph);
        editor.script_name = name.into();
        editor
    }

    /// Toggle editor visibility
    pub fn toggle(&mut self) {
        self.active = !self.active;
    }

    /// Check if editor is active
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Set editor active state
    pub fn set_active(&mut self, active: bool) {
        self.active = active;
    }

    /// Check if there are unsaved changes
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Create a new script
    pub fn new_script(&mut self) {
        self.canvas = NodeCanvas::new();
        self.script_name = "Untitled Script".to_string();
        self.description = String::new();
        self.file_path = None;
        self.dirty = false;
        self.last_compile_result = None;
        self.output_messages.clear();
        self.log_info("Created new script");
    }

    /// Open a script from file
    pub fn open_script(&mut self, path: impl AsRef<std::path::Path>) -> Result<(), Box<dyn std::error::Error>> {
        let path = path.as_ref();
        let json = std::fs::read_to_string(path)?;
        let graph = graph_from_json(&json)?;
        
        self.canvas.load_graph(graph);
        self.file_path = Some(path.to_path_buf());
        self.dirty = false;
        
        // Try to extract name from filename
        if let Some(stem) = path.file_stem() {
            self.script_name = stem.to_string_lossy().to_string();
        }
        
        self.log_success(format!("Opened script from {:?}", path));
        Ok(())
    }

    /// Save script to file
    pub fn save_script(&mut self, path: Option<impl AsRef<std::path::Path>>) -> Result<(), Box<dyn std::error::Error>> {
        let path = if let Some(p) = path {
            p.as_ref().to_path_buf()
        } else if let Some(ref p) = self.file_path {
            p.clone()
        } else {
            return Err("No file path specified".into());
        };

        let json = graph_to_json(self.canvas.graph())?;
        std::fs::write(&path, json)?;
        
        self.file_path = Some(path.clone());
        self.dirty = false;
        
        self.log_success(format!("Saved script to {:?}", path));
        Ok(())
    }

    /// Compile the current script
    pub fn compile(&mut self) {
        self.log_info("Compiling script...");
        
        match compile_to_events(self.canvas.graph()) {
            Ok(script) => {
                let event_count = script.events.len();
                let warning_count = script.warnings.len();
                
                // Log warnings
                for warning in &script.warnings {
                    self.log_warning(warning);
                }
                
                self.last_compile_result = Some(Ok(script));
                self.log_success(format!(
                    "Compilation successful: {} events, {} warnings",
                    event_count, warning_count
                ));
            }
            Err(e) => {
                self.last_compile_result = Some(Err(e.to_string()));
                self.log_error(format!("Compilation failed: {}", e));
            }
        }
        
        self.show_output_panel = true;
    }

    /// Test run the script
    pub fn test_run(&mut self) {
        if self.last_compile_result.is_none() {
            self.compile();
        }
        
        if let Some(Ok(ref script)) = self.last_compile_result {
            self.log_info("Starting test execution...");
            let executor = ScriptExecutor::new();
            // Note: We'd need a World reference here for actual execution
            // For now, just log that we're ready to execute
            self.test_executor = Some(executor);
            self.log_success("Test execution prepared (requires world context)");
        } else {
            self.log_error("Cannot test: script has compilation errors");
        }
        
        self.show_output_panel = true;
    }

    /// Export script to Lua
    pub fn export_to_lua(&mut self) -> Result<String, Box<dyn std::error::Error>> {
        if self.last_compile_result.is_none() {
            self.compile();
        }
        
        let script = self.last_compile_result
            .as_ref()
            .ok_or("No compilation result")?
            .as_ref()
            .map_err(|e| format!("Compilation error: {}", e))?;
        
        let lua = self.generate_lua(script)?;
        self.log_success("Exported script to Lua");
        Ok(lua)
    }

    /// Generate Lua code from compiled script
    fn generate_lua(&self, script: &CompiledScript) -> Result<String, Box<dyn std::error::Error>> {
        let mut lua = String::new();
        
        lua.push_str("-- Generated by DocDamage Engine Visual Scripting\n");
        lua.push_str(&format!("-- Script: {}\n\n", self.script_name));
        
        lua.push_str("function runScript(context)\n");
        lua.push_str("    -- Script execution\n");
        
        for (i, event) in script.events.iter().enumerate() {
            lua.push_str(&format!("    -- Event {}: {:?}\n", i, std::mem::discriminant(event)));
            // Add event-specific Lua generation here
            match event {
                super::visual_script::GameEvent::ShowDialogue { text, speaker, .. } => {
                    lua.push_str(&format!(
                        "    context:showDialogue(\"{}\", \"{}\")\n",
                        speaker.replace('"', "\\\""),
                        text.replace('"', "\\\"")
                    ));
                }
                super::visual_script::GameEvent::Delay { seconds } => {
                    lua.push_str(&format!("    context:delay({})\n", seconds));
                }
                super::visual_script::GameEvent::GiveItem { item_id, quantity } => {
                    lua.push_str(&format!("    context:giveItem({}, {})\n", item_id, quantity));
                }
                _ => {
                    lua.push_str(&format!("    -- TODO: Implement {:?}\n", event));
                }
            }
        }
        
        lua.push_str("end\n");
        
        Ok(lua)
    }

    /// Draw the editor
    pub fn draw(&mut self, ctx: &egui::Context) {
        if !self.active {
            return;
        }

        // Main editor window
        egui::Window::new(format!("Visual Script Editor - {}", self.script_name))
            .default_size([1400.0, 900.0])
            .show(ctx, |ui| {
                self.draw_menu_bar(ui);
                self.draw_toolbar(ui);
                
                ui.separator();
                
                // Main content area
                egui::SidePanel::left("node_palette")
                    .default_width(220.0)
                    .resizable(true)
                    .show_inside(ui, |ui| {
                        self.draw_node_palette(ui);
                    });

                egui::SidePanel::right("properties_panel")
                    .default_width(280.0)
                    .resizable(true)
                    .show_inside(ui, |ui| {
                        self.draw_properties_panel(ui);
                    });

                if self.show_output_panel {
                    egui::TopBottomPanel::bottom("output_panel")
                        .default_height(150.0)
                        .resizable(true)
                        .show_inside(ui, |ui| {
                            self.draw_output_panel(ui);
                        });
                }

                egui::CentralPanel::default().show_inside(ui, |ui| {
                    self.draw_canvas(ui);
                });
            });
    }

    /// Draw the menu bar
    fn draw_menu_bar(&mut self, ui: &mut egui::Ui) {
        egui::menu::bar(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("New").clicked() {
                    self.new_script();
                    ui.close_menu();
                }
                if ui.button("Open...").clicked() {
                    // Would open file dialog
                    self.log_info("Open file dialog would appear here");
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Save").clicked() {
                    if let Err(e) = self.save_script(None::<&std::path::Path>) {
                        self.log_error(format!("Save failed: {}", e));
                    }
                    ui.close_menu();
                }
                if ui.button("Save As...").clicked() {
                    // Would open save dialog
                    self.log_info("Save As dialog would appear here");
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Export to Lua...").clicked() {
                    match self.export_to_lua() {
                        Ok(lua) => {
                            // Would show save dialog for Lua
                            self.log_info(format!("Generated {} bytes of Lua", lua.len()));
                        }
                        Err(e) => self.log_error(format!("Export failed: {}", e)),
                    }
                    ui.close_menu();
                }
            });

            ui.menu_button("Edit", |ui| {
                if ui.button("Undo (Ctrl+Z)").clicked() {
                    ui.close_menu();
                }
                if ui.button("Redo (Ctrl+Y)").clicked() {
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Delete Selected").clicked() {
                    ui.close_menu();
                }
                if ui.button("Duplicate (Ctrl+D)").clicked() {
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Select All (Ctrl+A)").clicked() {
                    ui.close_menu();
                }
            });

            ui.menu_button("View", |ui| {
                ui.checkbox(&mut self.show_grid, "Show Grid");
                ui.checkbox(&mut self.show_minimap, "Show Minimap");
                ui.checkbox(&mut self.show_palette, "Show Node Palette");
                ui.checkbox(&mut self.show_properties, "Show Properties");
                ui.checkbox(&mut self.show_output_panel, "Show Output");
                ui.separator();
                if ui.button("Frame All").clicked() {
                    // self.canvas.frame_all(); // Need rect
                    ui.close_menu();
                }
            });

            ui.menu_button("Build", |ui| {
                if ui.button("Compile (F5)").clicked() {
                    self.compile();
                    ui.close_menu();
                }
                if ui.button("Test Run (F6)").clicked() {
                    self.test_run();
                    ui.close_menu();
                }
            });
        });
    }

    /// Draw the toolbar
    fn draw_toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            // File operations
            if ui.button("➕ New").clicked() {
                self.new_script();
            }
            if ui.button("💾 Save").clicked() {
                if let Err(e) = self.save_script(None::<&std::path::Path>) {
                    self.log_error(format!("Save failed: {}", e));
                }
            }
            
            ui.separator();
            
            // Compile buttons
            if ui.button("🔨 Compile").on_hover_text("Compile script (F5)").clicked() {
                self.compile();
            }
            if ui.button("▶️ Test").on_hover_text("Test run script (F6)").clicked() {
                self.test_run();
            }
            
            ui.separator();
            
            // Zoom controls
            if ui.button("➖").on_hover_text("Zoom out").clicked() {
                self.canvas.zoom_out();
            }
            ui.label(format!("{:.0}%", self.canvas.zoom * 100.0));
            if ui.button("➕").on_hover_text("Zoom in").clicked() {
                self.canvas.zoom_in();
            }
            if ui.button("⟲").on_hover_text("Reset zoom").clicked() {
                self.canvas.reset_zoom();
            }
            
            ui.separator();
            
            // Status indicators
            if self.dirty {
                ui.label(egui::RichText::new("● Modified").color(egui::Color32::YELLOW));
            } else {
                ui.label(egui::RichText::new("✓ Saved").color(egui::Color32::GREEN));
            }
            
            if let Some(ref result) = self.last_compile_result {
                match result {
                    Ok(_) => {
                        ui.label(egui::RichText::new("✓ Compiled").color(egui::Color32::GREEN));
                    }
                    Err(_) => {
                        ui.label(egui::RichText::new("✗ Errors").color(egui::Color32::RED));
                    }
                }
            }
        });
    }

    /// Draw the node palette sidebar
    fn draw_node_palette(&mut self, ui: &mut egui::Ui) {
        ui.heading("Nodes");
        ui.add_space(8.0);
        
        // Search filter
        ui.add(egui::TextEdit::singleline(&mut self.palette_filter).hint_text("🔍 Search nodes..."));
        ui.add_space(8.0);
        
        egui::ScrollArea::vertical().show(ui, |ui| {
            let categories = crate::visual_script::get_node_categories();
            
            for category in categories {
                // Filter by search
                let matches_search = self.palette_filter.is_empty()
                    || category.name.to_lowercase().contains(&self.palette_filter.to_lowercase())
                    || category.node_types.iter().any(|nt| {
                        nt.name.to_lowercase().contains(&self.palette_filter.to_lowercase())
                    });
                
                if !matches_search {
                    continue;
                }
                
                ui.collapsing(
                    egui::RichText::new(category.name).color(category.color).strong(),
                    |ui| {
                        for node_type in category.node_types {
                            // Filter individual nodes
                            if !self.palette_filter.is_empty()
                                && !node_type.name.to_lowercase().contains(&self.palette_filter.to_lowercase())
                                && !category.name.to_lowercase().contains(&self.palette_filter.to_lowercase())
                            {
                                continue;
                            }
                            
                            let button = ui.add(
                                egui::Button::new(node_type.name)
                                    .fill(ui.visuals().widgets.inactive.bg_fill)
                                    .frame(false)
                            );
                            
                            let response = button.on_hover_text(node_type.description);
                            
                            if response.clicked() {
                                // Add node at center of canvas
                                let canvas_center = [0.0, 0.0]; // Would need to calculate actual center
                                let node = node_type.create_node(canvas_center);
                                self.canvas.graph_mut().add_node(node);
                                self.dirty = true;
                                self.log_info(format!("Added {} node", node_type.name));
                            }
                        }
                    },
                );
            }
        });
    }

    /// Draw the properties panel
    fn draw_properties_panel(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.property_panel.active_tab, PropertyTab::Properties, "Properties");
            ui.selectable_value(&mut self.property_panel.active_tab, PropertyTab::Palette, "Palette");
            ui.selectable_value(&mut self.property_panel.active_tab, PropertyTab::Compile, "Compile");
        });
        
        ui.separator();
        
        match self.property_panel.active_tab {
            PropertyTab::Properties => {
                self.draw_node_properties(ui);
            }
            PropertyTab::Palette => {
                // Already shown in left panel, show mini version or different view
                ui.label("Use the left panel for node palette");
            }
            PropertyTab::Compile => {
                self.draw_compile_info(ui);
            }
        }
    }

    /// Draw node properties
    fn draw_node_properties(&mut self, ui: &mut egui::Ui) {
        if let Some(node) = self.canvas.first_selected_node() {
            ui.heading("Node Properties");
            ui.add_space(8.0);
            
            ui.label(format!("Type: {:?}", node.node_type));
            ui.label(format!("ID: {:?}", node.id));
            ui.add_space(8.0);
            
            // Position
            ui.horizontal(|ui| {
                ui.label("Position:");
                let mut pos = node.position;
                ui.add(egui::DragValue::new(&mut pos[0]).speed(1.0).prefix("X: "));
                ui.add(egui::DragValue::new(&mut pos[1]).speed(1.0).prefix("Y: "));
            });
            
            ui.add_space(8.0);
            
            // Comment
            ui.label("Comment:");
            // Would need mutable access to edit comment
        } else {
            ui.heading("Script Properties");
            ui.add_space(8.0);
            
            ui.label("Name:");
            if ui.text_edit_singleline(&mut self.script_name).changed() {
                self.dirty = true;
            }
            
            ui.add_space(8.0);
            
            ui.label("Description:");
            if ui.text_edit_multiline(&mut self.description).changed() {
                self.dirty = true;
            }
            
            ui.add_space(16.0);
            
            // Graph statistics
            ui.heading("Statistics");
            ui.label(format!("Nodes: {}", self.canvas.graph().nodes.len()));
            ui.label(format!("Connections: {}", self.canvas.graph().connections.len()));
            ui.label(format!("Event Nodes: {}", self.canvas.graph().get_event_nodes().len()));
        }
    }

    /// Draw compile information
    fn draw_compile_info(&mut self, ui: &mut egui::Ui) {
        ui.heading("Compilation");
        ui.add_space(8.0);
        
        if let Some(ref result) = self.last_compile_result {
            match result {
                Ok(script) => {
                    ui.label(egui::RichText::new("✓ Compilation Successful").color(egui::Color32::GREEN));
                    ui.add_space(8.0);
                    ui.label(format!("Events: {}", script.events.len()));
                    ui.label(format!("Warnings: {}", script.warnings.len()));
                    
                    if !script.warnings.is_empty() {
                        ui.add_space(8.0);
                        ui.label("Warnings:");
                        for warning in &script.warnings {
                            ui.label(egui::RichText::new(format!("⚠ {}", warning)).color(egui::Color32::YELLOW));
                        }
                    }
                }
                Err(e) => {
                    ui.label(egui::RichText::new("✗ Compilation Failed").color(egui::Color32::RED));
                    ui.add_space(8.0);
                    ui.label(egui::RichText::new(e).color(egui::Color32::RED));
                }
            }
        } else {
            ui.label("Click Compile to check for errors");
        }
        
        ui.add_space(16.0);
        
        if ui.button("🔨 Compile Now").clicked() {
            self.compile();
        }
    }

    /// Draw the output panel
    fn draw_output_panel(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("Output");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Clear").clicked() {
                    self.output_messages.clear();
                }
                if ui.button("✕").clicked() {
                    self.show_output_panel = false;
                }
            });
        });
        
        ui.separator();
        
        egui::ScrollArea::vertical()
            .stick_to_bottom(true)
            .show(ui, |ui| {
                for msg in &self.output_messages {
                    let color = match msg.level {
                        MessageLevel::Info => ui.visuals().text_color(),
                        MessageLevel::Warning => egui::Color32::YELLOW,
                        MessageLevel::Error => egui::Color32::RED,
                        MessageLevel::Success => egui::Color32::GREEN,
                    };
                    
                    let prefix = match msg.level {
                        MessageLevel::Info => "ℹ ",
                        MessageLevel::Warning => "⚠ ",
                        MessageLevel::Error => "✗ ",
                        MessageLevel::Success => "✓ ",
                    };
                    
                    ui.label(egui::RichText::new(format!("{}{}", prefix, msg.text)).color(color));
                }
            });
    }

    /// Draw the canvas area
    fn draw_canvas(&mut self, ui: &mut egui::Ui) {
        // Apply settings
        self.canvas.show_grid = self.show_grid;
        self.canvas.show_minimap = self.show_minimap;
        
        // Draw the canvas
        let response = self.canvas.draw(ui);
        
        // Check for changes
        if response.changed() {
            self.dirty = true;
        }
    }

    /// Log an info message
    fn log_info(&mut self, text: impl Into<String>) {
        self.output_messages.push(OutputMessage {
            level: MessageLevel::Info,
            text: text.into(),
            timestamp: std::time::Instant::now(),
        });
    }

    /// Log a warning message
    fn log_warning(&mut self, text: impl Into<String>) {
        self.output_messages.push(OutputMessage {
            level: MessageLevel::Warning,
            text: text.into(),
            timestamp: std::time::Instant::now(),
        });
    }

    /// Log an error message
    fn log_error(&mut self, text: impl Into<String>) {
        self.output_messages.push(OutputMessage {
            level: MessageLevel::Error,
            text: text.into(),
            timestamp: std::time::Instant::now(),
        });
    }

    /// Log a success message
    fn log_success(&mut self, text: impl Into<String>) {
        self.output_messages.push(OutputMessage {
            level: MessageLevel::Success,
            text: text.into(),
            timestamp: std::time::Instant::now(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_visual_script_editor_creation() {
        let editor = VisualScriptEditor::new();
        assert!(!editor.active);
        assert!(!editor.is_dirty());
        assert_eq!(editor.script_name, "Untitled Script");
    }

    #[test]
    fn test_new_script() {
        let mut editor = VisualScriptEditor::new();
        editor.dirty = true;
        editor.script_name = "Old Name".to_string();
        
        editor.new_script();
        
        assert!(!editor.is_dirty());
        assert_eq!(editor.script_name, "Untitled Script");
    }

    #[test]
    fn test_log_messages() {
        let mut editor = VisualScriptEditor::new();
        
        editor.log_info("Test info");
        editor.log_warning("Test warning");
        editor.log_error("Test error");
        editor.log_success("Test success");
        
        assert_eq!(editor.output_messages.len(), 4);
        assert_eq!(editor.output_messages[0].level, MessageLevel::Info);
        assert_eq!(editor.output_messages[1].level, MessageLevel::Warning);
        assert_eq!(editor.output_messages[2].level, MessageLevel::Error);
        assert_eq!(editor.output_messages[3].level, MessageLevel::Success);
    }
}
