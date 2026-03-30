//! Asset OS Pipeline Panel
//!
//! Editor UI panel for managing the Asset OS pipeline - a workflow system
//! for asset processing, classification, and management.

use egui::{Color32, Context, Ui, Vec2};

/// Asset OS Pipeline Panel
pub struct AssetOsPipelinePanel {
    /// Whether the panel is visible
    visible: bool,
    /// Pipeline status
    status: PipelineStatus,
    /// Processing queue
    queue: Vec<QueuedAsset>,
    /// Selected pipeline stage
    selected_stage: PipelineStage,
    /// Auto-process flag
    auto_process: bool,
    /// Show processed assets
    show_processed: bool,
}

/// Pipeline processing status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipelineStatus {
    /// Idle - waiting for input
    Idle,
    /// Scanning for assets
    Scanning,
    /// Classifying assets
    Classifying,
    /// Processing assets
    Processing,
    /// Error occurred
    Error,
}

impl PipelineStatus {
    /// Get display name
    pub fn name(&self) -> &'static str {
        match self {
            PipelineStatus::Idle => "Idle",
            PipelineStatus::Scanning => "Scanning",
            PipelineStatus::Classifying => "Classifying",
            PipelineStatus::Processing => "Processing",
            PipelineStatus::Error => "Error",
        }
    }

    /// Get color for status
    pub fn color(&self) -> Color32 {
        match self {
            PipelineStatus::Idle => Color32::GRAY,
            PipelineStatus::Scanning => Color32::YELLOW,
            PipelineStatus::Classifying => Color32::BLUE,
            PipelineStatus::Processing => Color32::GREEN,
            PipelineStatus::Error => Color32::RED,
        }
    }
}

/// Pipeline processing stage
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipelineStage {
    /// Import stage
    Import,
    /// Classification stage
    Classification,
    /// Optimization stage
    Optimization,
    /// Export stage
    Export,
}

impl PipelineStage {
    /// Get display name
    pub fn name(&self) -> &'static str {
        match self {
            PipelineStage::Import => "Import",
            PipelineStage::Classification => "Classification",
            PipelineStage::Optimization => "Optimization",
            PipelineStage::Export => "Export",
        }
    }
}

/// Queued asset for processing
#[derive(Debug, Clone)]
pub struct QueuedAsset {
    /// Asset path
    pub path: String,
    /// Asset type
    pub asset_type: String,
    /// Processing status
    pub status: AssetProcessingStatus,
    /// Error message if failed
    pub error: Option<String>,
}

/// Asset processing status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetProcessingStatus {
    /// Waiting to be processed
    Pending,
    /// Currently processing
    Processing,
    /// Successfully processed
    Completed,
    /// Processing failed
    Failed,
}

impl AssetProcessingStatus {
    /// Get display name
    pub fn name(&self) -> &'static str {
        match self {
            AssetProcessingStatus::Pending => "Pending",
            AssetProcessingStatus::Processing => "Processing",
            AssetProcessingStatus::Completed => "Completed",
            AssetProcessingStatus::Failed => "Failed",
        }
    }

    /// Get color for status
    pub fn color(&self) -> Color32 {
        match self {
            AssetProcessingStatus::Pending => Color32::GRAY,
            AssetProcessingStatus::Processing => Color32::YELLOW,
            AssetProcessingStatus::Completed => Color32::GREEN,
            AssetProcessingStatus::Failed => Color32::RED,
        }
    }
}

impl Default for AssetOsPipelinePanel {
    fn default() -> Self {
        Self::new()
    }
}

impl AssetOsPipelinePanel {
    /// Create a new Asset OS Pipeline panel
    pub fn new() -> Self {
        Self {
            visible: false,
            status: PipelineStatus::Idle,
            queue: Vec::new(),
            selected_stage: PipelineStage::Import,
            auto_process: false,
            show_processed: true,
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

    /// Update the panel (call each frame)
    pub fn update(&mut self, _dt: f32) {
        // Process queue if auto-processing is enabled
        if self.auto_process && self.status == PipelineStatus::Idle && !self.queue.is_empty() {
            self.process_next();
        }
    }

    /// Draw the panel
    pub fn draw(&mut self, ctx: &Context) {
        if !self.visible {
            return;
        }

        egui::Window::new("🔄 Asset OS Pipeline")
            .default_size([800.0, 600.0])
            .show(ctx, |ui| {
                self.draw_ui(ui);
            });
    }

    /// Draw the UI content
    fn draw_ui(&mut self, ui: &mut Ui) {
        // Status bar
        ui.horizontal(|ui| {
            ui.label("Status:");
            ui.colored_label(self.status.color(), self.status.name());
            
            ui.separator();
            
            ui.label(format!("Queue: {} items", self.queue.len()));
            
            ui.separator();
            
            ui.checkbox(&mut self.auto_process, "Auto-process");
            
            ui.separator();
            
            if ui.button("▶ Process All").clicked() {
                self.process_all();
            }
            
            if ui.button("🧹 Clear Completed").clicked() {
                self.clear_completed();
            }
        });
        
        ui.separator();
        
        // Pipeline stages
        ui.horizontal(|ui| {
            ui.label("Stage:");
            for stage in [PipelineStage::Import, PipelineStage::Classification, PipelineStage::Optimization, PipelineStage::Export] {
                let selected = self.selected_stage == stage;
                if ui.selectable_label(selected, stage.name()).clicked() {
                    self.selected_stage = stage;
                }
            }
        });
        
        ui.separator();
        
        // Queue display
        ui.checkbox(&mut self.show_processed, "Show processed assets");
        
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.label("Processing Queue:");
            
            let mut to_remove = Vec::new();
            for (i, asset) in self.queue.iter().enumerate() {
                // Filter by status if needed
                if !self.show_processed && asset.status == AssetProcessingStatus::Completed {
                    continue;
                }
                
                ui.horizontal(|ui| {
                    ui.colored_label(asset.status.color(), asset.status.name());
                    ui.label(&asset.asset_type);
                    ui.label(&asset.path);
                    
                    if asset.status == AssetProcessingStatus::Pending {
                        if ui.button("▶").clicked() {
                            self.process_asset(i);
                        }
                    }
                    
                    if ui.button("🗑").clicked() {
                        to_remove.push(i);
                    }
                });
                
                if let Some(ref error) = asset.error {
                    ui.colored_label(Color32::RED, format!("  Error: {}", error));
                }
            }
            
            // Remove marked items
            for i in to_remove.into_iter().rev() {
                self.queue.remove(i);
            }
        });
        
        // Add test assets button (for demo)
        if ui.button("➕ Add Test Assets").clicked() {
            self.add_test_assets();
        }
    }

    /// Add assets to the queue
    pub fn queue_assets(&mut self, paths: Vec<String>) {
        for path in paths {
            self.queue.push(QueuedAsset {
                path,
                asset_type: "Unknown".to_string(),
                status: AssetProcessingStatus::Pending,
                error: None,
            });
        }
    }

    /// Process the next asset in the queue
    fn process_next(&mut self) {
        if let Some(index) = self.queue.iter().position(|a| a.status == AssetProcessingStatus::Pending) {
            self.process_asset(index);
        }
    }

    /// Process a specific asset
    fn process_asset(&mut self, index: usize) {
        if let Some(asset) = self.queue.get_mut(index) {
            asset.status = AssetProcessingStatus::Processing;
            self.status = PipelineStatus::Processing;
            
            // Simulate processing
            // In a real implementation, this would call into dde-asset-forge
            asset.status = AssetProcessingStatus::Completed;
            self.status = PipelineStatus::Idle;
        }
    }

    /// Process all pending assets
    fn process_all(&mut self) {
        for i in 0..self.queue.len() {
            if self.queue[i].status == AssetProcessingStatus::Pending {
                self.process_asset(i);
            }
        }
    }

    /// Clear completed assets from the queue
    fn clear_completed(&mut self) {
        self.queue.retain(|a| a.status != AssetProcessingStatus::Completed);
    }

    /// Add test assets for demonstration
    fn add_test_assets(&mut self) {
        let test_assets = vec![
            ("assets/characters/hero.png", "Sprite"),
            ("assets/tiles/grass.png", "Tileset"),
            ("assets/audio/bgm_main.mp3", "Audio"),
            ("assets/fonts/main.ttf", "Font"),
            ("assets/scripts/npc_behavior.lua", "Script"),
        ];
        
        for (path, asset_type) in test_assets {
            self.queue.push(QueuedAsset {
                path: path.to_string(),
                asset_type: asset_type.to_string(),
                status: AssetProcessingStatus::Pending,
                error: None,
            });
        }
    }

    /// Get the current pipeline status
    pub fn status(&self) -> PipelineStatus {
        self.status
    }

    /// Get the number of items in the queue
    pub fn queue_len(&self) -> usize {
        self.queue.len()
    }

    /// Get the number of pending items
    pub fn pending_count(&self) -> usize {
        self.queue.iter().filter(|a| a.status == AssetProcessingStatus::Pending).count()
    }

    /// Get the number of completed items
    pub fn completed_count(&self) -> usize {
        self.queue.iter().filter(|a| a.status == AssetProcessingStatus::Completed).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_panel_creation() {
        let panel = AssetOsPipelinePanel::new();
        assert!(!panel.is_visible());
        assert_eq!(panel.status, PipelineStatus::Idle);
    }

    #[test]
    fn test_panel_toggle() {
        let mut panel = AssetOsPipelinePanel::new();
        assert!(!panel.is_visible());
        
        panel.toggle();
        assert!(panel.is_visible());
        
        panel.toggle();
        assert!(!panel.is_visible());
    }

    #[test]
    fn test_queue_assets() {
        let mut panel = AssetOsPipelinePanel::new();
        
        panel.queue_assets(vec![
            "test1.png".to_string(),
            "test2.png".to_string(),
        ]);
        
        assert_eq!(panel.queue_len(), 2);
        assert_eq!(panel.pending_count(), 2);
    }

    #[test]
    fn test_clear_completed() {
        let mut panel = AssetOsPipelinePanel::new();
        
        panel.queue_assets(vec!["test.png".to_string()]);
        panel.process_all();
        
        assert_eq!(panel.completed_count(), 1);
        
        panel.clear_completed();
        
        assert_eq!(panel.queue_len(), 0);
    }

    #[test]
    fn test_pipeline_status() {
        assert_eq!(PipelineStatus::Idle.name(), "Idle");
        assert_eq!(PipelineStatus::Processing.name(), "Processing");
        assert_eq!(PipelineStatus::Error.name(), "Error");
    }

    #[test]
    fn test_pipeline_stages() {
        assert_eq!(PipelineStage::Import.name(), "Import");
        assert_eq!(PipelineStage::Classification.name(), "Classification");
        assert_eq!(PipelineStage::Optimization.name(), "Optimization");
        assert_eq!(PipelineStage::Export.name(), "Export");
    }

    #[test]
    fn test_asset_processing_status() {
        assert_eq!(AssetProcessingStatus::Pending.name(), "Pending");
        assert_eq!(AssetProcessingStatus::Completed.name(), "Completed");
        assert_eq!(AssetProcessingStatus::Failed.name(), "Failed");
    }
}
