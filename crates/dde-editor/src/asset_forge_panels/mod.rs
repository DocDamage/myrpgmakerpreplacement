//! Asset Forge UI Panels
//!
//! Editor UI panels for the Asset Forge integration:
//! - Classification Rules Editor
//! - Dependency Graph Viewer
//! - Duplicate Scanner
//! - Asset OS Pipeline

pub mod asset_os_panel;
pub mod classification_rules_panel;
pub mod dependency_graph_panel;
pub mod duplicate_scanner_panel;

pub use asset_os_panel::AssetOsPipelinePanel;
pub use classification_rules_panel::{
    ClassificationRuleDef, ClassificationRulesPanel, AssetTypeOption, PatternTestResultDisplay,
};
pub use dependency_graph_panel::DependencyGraphPanel;
pub use duplicate_scanner_panel::DuplicateScannerPanel;

use egui::Context;

/// Combined Asset Forge panels manager
pub struct AssetForgePanels {
    /// Classification rules panel
    pub classification: ClassificationRulesPanel,
    /// Dependency graph panel
    pub dependency_graph: DependencyGraphPanel,
    /// Duplicate scanner panel
    pub duplicate_scanner: DuplicateScannerPanel,
    /// Asset OS Pipeline panel
    pub asset_os_pipeline: AssetOsPipelinePanel,
}

impl Default for AssetForgePanels {
    fn default() -> Self {
        Self::new()
    }
}

impl AssetForgePanels {
    /// Create new Asset Forge panels
    pub fn new() -> Self {
        Self {
            classification: ClassificationRulesPanel::new(),
            dependency_graph: DependencyGraphPanel::new(),
            duplicate_scanner: DuplicateScannerPanel::new(),
            asset_os_pipeline: AssetOsPipelinePanel::new(),
        }
    }

    /// Update all panels (call each frame)
    pub fn update(&mut self, dt: f32) {
        self.classification.update(dt);
        self.duplicate_scanner.update(dt);
        self.asset_os_pipeline.update(dt);
        // Dependency graph panel doesn't need update for now
    }

    /// Draw all visible panels
    pub fn draw(&mut self, ctx: &Context) {
        self.classification.draw(ctx);
        self.dependency_graph.draw(ctx);
        self.duplicate_scanner.draw(ctx);
        self.asset_os_pipeline.draw(ctx);
    }

    /// Show classification rules panel
    pub fn show_classification(&mut self) {
        self.classification.show();
    }

    /// Show dependency graph panel
    pub fn show_dependency_graph(&mut self) {
        self.dependency_graph.show();
    }

    /// Show duplicate scanner panel
    pub fn show_duplicate_scanner(&mut self) {
        self.duplicate_scanner.show();
    }

    /// Show asset OS pipeline panel
    pub fn show_asset_os_pipeline(&mut self) {
        self.asset_os_pipeline.show();
    }

    /// Hide all panels
    pub fn hide_all(&mut self) {
        self.classification.hide();
        self.dependency_graph.hide();
        self.duplicate_scanner.hide();
        self.asset_os_pipeline.hide();
    }

    /// Check if any panel is visible
    pub fn any_visible(&self) -> bool {
        self.classification.is_visible()
            || self.dependency_graph.is_visible()
            || self.duplicate_scanner.is_visible()
            || self.asset_os_pipeline.is_visible()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_panels_creation() {
        let panels = AssetForgePanels::new();
        assert!(!panels.any_visible());
    }

    #[test]
    fn test_show_panels() {
        let mut panels = AssetForgePanels::new();
        
        panels.show_classification();
        assert!(panels.classification.is_visible());
        
        panels.show_dependency_graph();
        assert!(panels.dependency_graph.is_visible());
        
        panels.show_duplicate_scanner();
        assert!(panels.duplicate_scanner.is_visible());
        
        assert!(panels.any_visible());
    }

    #[test]
    fn test_hide_all() {
        let mut panels = AssetForgePanels::new();
        
        panels.show_classification();
        panels.show_dependency_graph();
        panels.show_duplicate_scanner();
        panels.show_asset_os_pipeline();
        
        panels.hide_all();
        
        assert!(!panels.any_visible());
    }

    #[test]
    fn test_asset_os_pipeline_panel() {
        let mut panels = AssetForgePanels::new();
        assert!(!panels.asset_os_pipeline.is_visible());
        
        panels.show_asset_os_pipeline();
        assert!(panels.asset_os_pipeline.is_visible());
        
        panels.hide_all();
        assert!(!panels.asset_os_pipeline.is_visible());
    }
}
