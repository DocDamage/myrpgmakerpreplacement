//! Asset Dependency Graph
//!
//! Tracks dependencies between game assets for:
//! - Automatic dependency resolution during export
//! - Detecting orphaned assets
//! - Safe asset deletion (checking for references)
//! - Circular dependency detection
//! - Batch asset operations

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Asset types in the dependency graph
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize)]
pub enum AssetType {
    #[default]
    Data,
    Texture,
    SpriteSheet,
    Audio,
    Music,
    Map,
    Tileset,
    Script,
    Prefab,
    Animation,
    Shader,
    Font,
}

impl AssetType {
    /// Get file extensions associated with this asset type
    pub fn extensions(&self) -> &'static [&'static str] {
        match self {
            AssetType::Texture => &["png", "jpg", "jpeg", "webp"],
            AssetType::SpriteSheet => &["png", "json"],
            AssetType::Audio => &["wav", "ogg", "mp3"],
            AssetType::Music => &["ogg", "mp3", "wav"],
            AssetType::Map => &["tmx", "json"],
            AssetType::Tileset => &["tsx", "png"],
            AssetType::Script => &["lua", "js"],
            AssetType::Prefab => &["prefab", "json"],
            AssetType::Animation => &["anim", "json"],
            AssetType::Shader => &["wgsl", "glsl", "hlsl"],
            AssetType::Font => &["ttf", "otf"],
            AssetType::Data => &["json", "toml", "yaml", "csv"],
        }
    }

    /// Detect asset type from file extension
    pub fn from_extension(ext: &str) -> Option<Self> {
        let ext = ext.to_lowercase();
        [
            AssetType::Texture,
            AssetType::SpriteSheet,
            AssetType::Audio,
            AssetType::Music,
            AssetType::Map,
            AssetType::Tileset,
            AssetType::Script,
            AssetType::Prefab,
            AssetType::Animation,
            AssetType::Shader,
            AssetType::Font,
            AssetType::Data,
        ].into_iter().find(|&asset_type| asset_type.extensions().contains(&ext.as_str()))
    }
}

/// Unique identifier for an asset in the dependency graph
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize)]
pub struct AssetId {
    /// Asset type
    #[serde(default)]
    pub asset_type: AssetType,
    /// Relative path from project root
    #[serde(default)]
    pub path: PathBuf,
}

impl AssetId {
    /// Create new asset ID
    pub fn new(asset_type: AssetType, path: impl Into<PathBuf>) -> Self {
        Self {
            asset_type,
            path: path.into(),
        }
    }

    /// Create from file path
    pub fn from_path(project_root: &Path, file_path: &Path) -> Option<Self> {
        let rel_path = file_path.strip_prefix(project_root).ok()?;
        let ext = rel_path.extension()?.to_str()?;
        let asset_type = AssetType::from_extension(ext)?;

        Some(Self::new(asset_type, rel_path))
    }

    /// Get full path
    pub fn full_path(&self, project_root: &Path) -> PathBuf {
        project_root.join(&self.path)
    }

    /// Get filename
    pub fn filename(&self) -> Option<&str> {
        self.path.file_name()?.to_str()
    }
}

/// Dependency type (how the asset is used)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum DependencyKind {
    /// Hard dependency - asset is required
    Required,
    /// Soft dependency - asset enhances but not required
    Optional,
    /// Runtime dependency - loaded dynamically
    Runtime,
    /// Reference only - metadata link
    Reference,
}

/// A dependency edge in the graph
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Dependency {
    /// Target asset
    pub target: AssetId,
    /// How this dependency is used
    pub kind: DependencyKind,
    /// Context (e.g., "background_layer", "npc_dialogue")
    pub context: Option<String>,
}

/// Asset node in the dependency graph
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct AssetNode {
    /// Asset ID
    pub id: AssetId,
    /// Dependencies (this asset depends on these)
    pub dependencies: Vec<Dependency>,
    /// Dependents (these assets depend on this)
    pub dependents: Vec<AssetId>,
    /// File size in bytes
    pub file_size: u64,
    /// Last modified timestamp
    pub last_modified: i64,
    /// Asset metadata
    pub metadata: HashMap<String, String>,
    /// Is marked for deletion
    pub marked_for_deletion: bool,
}

/// Dependency graph errors
#[derive(Debug, Error)]
pub enum DependencyError {
    #[error("Circular dependency detected: {0}")]
    CircularDependency(String),
    #[error("Asset not found: {0:?}")]
    AssetNotFound(AssetId),
    #[error("Asset already exists: {0:?}")]
    AssetExists(AssetId),
    #[error("Cannot delete asset with dependents: {count} references")]
    HasDependents { count: usize },
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Asset dependency graph
#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct DependencyGraph {
    /// All assets in the graph
    nodes: HashMap<AssetId, AssetNode>,
    /// Project root path (not serialized)
    #[serde(skip)]
    project_root: Option<PathBuf>,
}

impl DependencyGraph {
    /// Create new empty graph
    pub fn new() -> Self {
        Self::default()
    }

    /// Set project root
    pub fn set_project_root(&mut self, root: impl Into<PathBuf>) {
        self.project_root = Some(root.into());
    }

    /// Add an asset to the graph
    pub fn add_asset(&mut self, id: AssetId) -> Result<&mut AssetNode, DependencyError> {
        if self.nodes.contains_key(&id) {
            return Err(DependencyError::AssetExists(id));
        }

        let node = AssetNode {
            id: id.clone(),
            ..Default::default()
        };

        self.nodes.insert(id.clone(), node);
        Ok(self.nodes.get_mut(&id).unwrap())
    }

    /// Get asset node
    pub fn get(&self, id: &AssetId) -> Option<&AssetNode> {
        self.nodes.get(id)
    }

    /// Get mutable asset node
    pub fn get_mut(&mut self, id: &AssetId) -> Option<&mut AssetNode> {
        self.nodes.get_mut(id)
    }

    /// Add a dependency edge
    pub fn add_dependency(
        &mut self,
        from: &AssetId,
        to: AssetId,
        kind: DependencyKind,
        context: Option<String>,
    ) -> Result<(), DependencyError> {
        // Ensure both assets exist
        if !self.nodes.contains_key(from) {
            return Err(DependencyError::AssetNotFound(from.clone()));
        }
        if !self.nodes.contains_key(&to) {
            return Err(DependencyError::AssetNotFound(to.clone()));
        }

        // Check for circular dependency
        if self.would_create_cycle(from, &to) {
            return Err(DependencyError::CircularDependency(format!(
                "{:?} -> {:?}",
                from.path, to.path
            )));
        }

        // Add dependency to source
        if let Some(node) = self.nodes.get_mut(from) {
            // Remove existing dependency to same target if any
            node.dependencies.retain(|d| d.target != to);
            node.dependencies.push(Dependency {
                target: to.clone(),
                kind,
                context,
            });
        }

        // Add dependent to target
        if let Some(node) = self.nodes.get_mut(&to) {
            if !node.dependents.contains(from) {
                node.dependents.push(from.clone());
            }
        }

        Ok(())
    }

    /// Remove a dependency edge
    pub fn remove_dependency(&mut self, from: &AssetId, to: &AssetId) {
        if let Some(node) = self.nodes.get_mut(from) {
            node.dependencies.retain(|d| &d.target != to);
        }
        if let Some(node) = self.nodes.get_mut(to) {
            node.dependents.retain(|d| d != from);
        }
    }

    /// Check if adding an edge would create a cycle
    fn would_create_cycle(&self, from: &AssetId, to: &AssetId) -> bool {
        // If 'to' can reach 'from', adding from->to would create a cycle
        self.can_reach(to, from)
    }

    /// Check if target can be reached from source (BFS)
    fn can_reach(&self, source: &AssetId, target: &AssetId) -> bool {
        if source == target {
            return true;
        }

        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(source);

        while let Some(current) = queue.pop_front() {
            if let Some(node) = self.nodes.get(current) {
                for dep in &node.dependencies {
                    if &dep.target == target {
                        return true;
                    }
                    if visited.insert(&dep.target) {
                        queue.push_back(&dep.target);
                    }
                }
            }
        }

        false
    }

    /// Get all dependencies of an asset (transitive closure)
    pub fn get_all_dependencies(&self, id: &AssetId) -> Vec<&AssetId> {
        let mut result = Vec::new();
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();

        if let Some(node) = self.nodes.get(id) {
            for dep in &node.dependencies {
                if visited.insert(&dep.target) {
                    queue.push_back(&dep.target);
                }
            }
        }

        while let Some(current) = queue.pop_front() {
            result.push(current);

            if let Some(node) = self.nodes.get(current) {
                for dep in &node.dependencies {
                    if visited.insert(&dep.target) {
                        queue.push_back(&dep.target);
                    }
                }
            }
        }

        result
    }

    /// Get all dependents of an asset (transitive closure)
    pub fn get_all_dependents(&self, id: &AssetId) -> Vec<&AssetId> {
        let mut result = Vec::new();
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();

        if let Some(node) = self.nodes.get(id) {
            for dep in &node.dependents {
                if visited.insert(dep) {
                    queue.push_back(dep);
                }
            }
        }

        while let Some(current) = queue.pop_front() {
            result.push(current);

            if let Some(node) = self.nodes.get(current) {
                for dep in &node.dependents {
                    if visited.insert(dep) {
                        queue.push_back(dep);
                    }
                }
            }
        }

        result
    }

    /// Get immediate dependencies
    pub fn get_dependencies(&self, id: &AssetId) -> Option<&Vec<Dependency>> {
        self.nodes.get(id).map(|n| &n.dependencies)
    }

    /// Get immediate dependents
    pub fn get_dependents(&self, id: &AssetId) -> Option<&Vec<AssetId>> {
        self.nodes.get(id).map(|n| &n.dependents)
    }

    /// Check if asset can be safely deleted
    pub fn can_delete(&self, id: &AssetId) -> Result<(), DependencyError> {
        if let Some(node) = self.nodes.get(id) {
            if node.dependents.is_empty() {
                Ok(())
            } else {
                Err(DependencyError::HasDependents {
                    count: node.dependents.len(),
                })
            }
        } else {
            Err(DependencyError::AssetNotFound(id.clone()))
        }
    }

    /// Remove asset from graph
    pub fn remove_asset(&mut self, id: &AssetId) -> Result<(), DependencyError> {
        self.can_delete(id)?;

        // Remove from dependencies of other assets
        let deps_to_remove: Vec<_> = self
            .nodes
            .values()
            .filter_map(|node| {
                if node.dependencies.iter().any(|d| &d.target == id) {
                    Some(node.id.clone())
                } else {
                    None
                }
            })
            .collect();

        for dep_id in deps_to_remove {
            if let Some(node) = self.nodes.get_mut(&dep_id) {
                node.dependencies.retain(|d| &d.target != id);
            }
        }

        self.nodes.remove(id);
        Ok(())
    }

    /// Find orphaned assets (no dependents, not root assets)
    pub fn find_orphans(&self, root_types: &[AssetType]) -> Vec<&AssetId> {
        self.nodes
            .values()
            .filter(|node| {
                node.dependents.is_empty() && !root_types.contains(&node.id.asset_type)
            })
            .map(|node| &node.id)
            .collect()
    }

    /// Get assets by type
    pub fn get_by_type(&self, asset_type: AssetType) -> Vec<&AssetNode> {
        self.nodes
            .values()
            .filter(|n| n.id.asset_type == asset_type)
            .collect()
    }

    /// Get all assets
    pub fn all_assets(&self) -> &HashMap<AssetId, AssetNode> {
        &self.nodes
    }

    /// Calculate total size of asset and all dependencies
    pub fn get_bundle_size(&self, id: &AssetId) -> u64 {
        let mut total = 0u64;
        let mut visited = HashSet::new();

        if let Some(node) = self.nodes.get(id) {
            total += node.file_size;
            visited.insert(id);

            for dep_id in self.get_all_dependencies(id) {
                if visited.insert(dep_id) {
                    if let Some(dep_node) = self.nodes.get(dep_id) {
                        total += dep_node.file_size;
                    }
                }
            }
        }

        total
    }

    /// Topological sort of all assets (dependencies first)
    pub fn topological_sort(&self) -> Vec<&AssetId> {
        let mut result = Vec::new();
        let mut visited = HashSet::new();
        let mut temp_mark = HashSet::new();

        for id in self.nodes.keys() {
            if !visited.contains(id) {
                self.visit_topological(id, &mut visited, &mut temp_mark, &mut result);
            }
        }

        // The DFS post-order visit already produces dependencies-first order
        result
    }

    fn visit_topological<'a>(
        &'a self,
        id: &'a AssetId,
        visited: &mut HashSet<&'a AssetId>,
        temp_mark: &mut HashSet<&'a AssetId>,
        result: &mut Vec<&'a AssetId>,
    ) {
        if temp_mark.contains(id) {
            // Circular dependency detected
            return;
        }

        if visited.contains(id) {
            return;
        }

        temp_mark.insert(id);

        if let Some(node) = self.nodes.get(id) {
            for dep in &node.dependencies {
                self.visit_topological(&dep.target, visited, temp_mark, result);
            }
        }

        temp_mark.remove(id);
        visited.insert(id);
        result.push(id);
    }

    /// Get export bundle (asset + all dependencies)
    pub fn get_export_bundle<'a>(&'a self, id: &'a AssetId) -> Vec<&'a AssetId> {
        let mut bundle = vec![id];
        bundle.extend(self.get_all_dependencies(id));
        bundle
    }

    /// Find missing dependencies (referenced but not in graph)
    pub fn find_missing_dependencies(&self) -> Vec<(AssetId, AssetId)> {
        let mut missing = Vec::new();

        for (id, node) in &self.nodes {
            for dep in &node.dependencies {
                if !self.nodes.contains_key(&dep.target) {
                    missing.push((id.clone(), dep.target.clone()));
                }
            }
        }

        missing
    }

    /// Validate entire graph
    pub fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();

        // Check for circular dependencies
        for id in self.nodes.keys() {
            if self.can_reach(id, id) {
                errors.push(format!("Circular dependency involving {:?}", id.path));
            }
        }

        // Check for missing dependencies
        for (from, to) in self.find_missing_dependencies() {
            errors.push(format!(
                "Missing dependency: {:?} depends on {:?}",
                from.path, to.path
            ));
        }

        errors
    }

    /// Serialize to JSON
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Deserialize from JSON
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Get asset count
    pub fn asset_count(&self) -> usize {
        self.nodes.len()
    }

    /// Get total size of all assets
    pub fn total_size(&self) -> u64 {
        self.nodes.values().map(|n| n.file_size).sum()
    }

    /// Clear all assets
    pub fn clear(&mut self) {
        self.nodes.clear();
    }
}

/// Dependency analysis result
#[derive(Debug, Default)]
pub struct DependencyAnalysis {
    /// Total number of assets
    pub total_assets: usize,
    /// Number of orphaned assets
    pub orphaned_count: usize,
    /// Number of circular dependencies
    pub circular_count: usize,
    /// Total size in bytes
    pub total_size: u64,
    /// Assets by type
    pub by_type: HashMap<AssetType, usize>,
    /// Most referenced assets
    pub most_referenced: Vec<(AssetId, usize)>,
}

impl DependencyAnalysis {
    /// Analyze a dependency graph
    pub fn analyze(graph: &DependencyGraph) -> Self {
        let total_assets = graph.asset_count();
        let orphaned = graph.find_orphans(&[AssetType::Map, AssetType::Script]);
        let total_size = graph.total_size();

        let mut by_type: HashMap<AssetType, usize> = HashMap::new();
        let mut reference_counts: Vec<(AssetId, usize)> = Vec::new();

        for (id, node) in graph.all_assets() {
            *by_type.entry(id.asset_type).or_insert(0) += 1;
            reference_counts.push((id.clone(), node.dependents.len()));
        }

        reference_counts.sort_by(|a, b| b.1.cmp(&a.1));
        let most_referenced = reference_counts.into_iter().take(10).collect();

        let circular_count = graph
            .all_assets()
            .keys()
            .filter(|id| graph.can_reach(id, id))
            .count();

        Self {
            total_assets,
            orphaned_count: orphaned.len(),
            circular_count,
            total_size,
            by_type,
            most_referenced,
        }
    }

    /// Get human-readable size
    pub fn formatted_size(&self) -> String {
        format_size(self.total_size)
    }
}

/// Format bytes to human-readable string
fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    format!("{:.2} {}", size, UNITS[unit_index])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_graph() -> DependencyGraph {
        let mut graph = DependencyGraph::new();

        // Create assets
        let map = AssetId::new(AssetType::Map, "maps/level1.tmx");
        let tileset = AssetId::new(AssetType::Tileset, "tilesets/terrain.tsx");
        let texture = AssetId::new(AssetType::Texture, "textures/terrain.png");
        let player = AssetId::new(AssetType::Prefab, "prefabs/player.json");
        let sprite = AssetId::new(AssetType::SpriteSheet, "sprites/player.png");

        // Add to graph
        graph.add_asset(map.clone()).unwrap();
        graph.add_asset(tileset.clone()).unwrap();
        graph.add_asset(texture.clone()).unwrap();
        graph.add_asset(player.clone()).unwrap();
        graph.add_asset(sprite.clone()).unwrap();

        // Set file sizes
        graph.get_mut(&map).unwrap().file_size = 1000;
        graph.get_mut(&tileset).unwrap().file_size = 500;
        graph.get_mut(&texture).unwrap().file_size = 10000;
        graph.get_mut(&player).unwrap().file_size = 200;
        graph.get_mut(&sprite).unwrap().file_size = 5000;

        // Create dependencies
        graph
            .add_dependency(&map, tileset.clone(), DependencyKind::Required, None)
            .unwrap();
        graph
            .add_dependency(&tileset, texture.clone(), DependencyKind::Required, None)
            .unwrap();
        graph
            .add_dependency(&player, sprite.clone(), DependencyKind::Required, None)
            .unwrap();

        graph
    }

    #[test]
    fn test_add_asset() {
        let mut graph = DependencyGraph::new();
        let id = AssetId::new(AssetType::Texture, "test.png");

        graph.add_asset(id.clone()).unwrap();
        assert!(graph.get(&id).is_some());

        // Duplicate should fail
        assert!(graph.add_asset(id.clone()).is_err());
    }

    #[test]
    fn test_add_dependency() {
        let mut graph = DependencyGraph::new();

        let a = AssetId::new(AssetType::Map, "a.tmx");
        let b = AssetId::new(AssetType::Texture, "b.png");

        graph.add_asset(a.clone()).unwrap();
        graph.add_asset(b.clone()).unwrap();

        graph
            .add_dependency(&a, b.clone(), DependencyKind::Required, None)
            .unwrap();

        let node_a = graph.get(&a).unwrap();
        assert_eq!(node_a.dependencies.len(), 1);
        assert_eq!(node_a.dependencies[0].target, b);

        let node_b = graph.get(&b).unwrap();
        assert_eq!(node_b.dependents.len(), 1);
        assert_eq!(node_b.dependents[0], a);
    }

    #[test]
    fn test_circular_dependency_detection() {
        let mut graph = DependencyGraph::new();

        let a = AssetId::new(AssetType::Script, "a.lua");
        let b = AssetId::new(AssetType::Script, "b.lua");
        let c = AssetId::new(AssetType::Script, "c.lua");

        graph.add_asset(a.clone()).unwrap();
        graph.add_asset(b.clone()).unwrap();
        graph.add_asset(c.clone()).unwrap();

        graph
            .add_dependency(&a, b.clone(), DependencyKind::Required, None)
            .unwrap();
        graph
            .add_dependency(&b, c.clone(), DependencyKind::Required, None)
            .unwrap();

        // This should fail - would create cycle
        assert!(graph
            .add_dependency(&c, a.clone(), DependencyKind::Required, None)
            .is_err());
    }

    #[test]
    fn test_get_all_dependencies() {
        let graph = create_test_graph();
        let map = AssetId::new(AssetType::Map, "maps/level1.tmx");

        let deps = graph.get_all_dependencies(&map);
        assert_eq!(deps.len(), 2); // tileset and texture
    }

    #[test]
    fn test_find_orphans() {
        let graph = create_test_graph();
        let orphans = graph.find_orphans(&[AssetType::Map]);

        // Player prefab is not a map and has no dependents, so it's orphaned
        assert_eq!(orphans.len(), 1);
        assert_eq!(orphans[0].asset_type, AssetType::Prefab);
    }

    #[test]
    fn test_get_bundle_size() {
        let graph = create_test_graph();
        let map = AssetId::new(AssetType::Map, "maps/level1.tmx");

        // map (1000) + tileset (500) + texture (10000)
        assert_eq!(graph.get_bundle_size(&map), 11500);
    }

    #[test]
    fn test_topological_sort() {
        let graph = create_test_graph();
        let sorted = graph.topological_sort();
        
        // Debug: print all items
        println!("Sorted order:");
        for (i, id) in sorted.iter().enumerate() {
            println!("  {}: {:?}", i, id.path);
        }

        // In topological sort, dependencies come first
        // So texture should come before tileset (which depends on texture)
        let texture_path: PathBuf = "textures/terrain.png".into();
        let tileset_path: PathBuf = "tilesets/terrain.tsx".into();
        
        let texture_pos = sorted.iter().position(|id| id.path == texture_path);
        let tileset_pos = sorted.iter().position(|id| id.path == tileset_path);

        assert!(texture_pos.is_some(), "Texture not found in sorted list");
        assert!(tileset_pos.is_some(), "Tileset not found in sorted list");
        
        // Dependencies first means texture comes before tileset
        assert!(texture_pos.unwrap() < tileset_pos.unwrap(), 
            "Texture (dependency at {:?}) should come before tileset (dependent at {:?})", 
            texture_pos, tileset_pos);
    }

    #[test]
    fn test_can_delete() {
        let graph = create_test_graph();
        let texture = AssetId::new(AssetType::Texture, "textures/terrain.png");
        let player = AssetId::new(AssetType::Prefab, "prefabs/player.json");

        // Texture has dependents (tileset), cannot delete
        assert!(graph.can_delete(&texture).is_err());

        // Player has no dependents, can delete
        assert!(graph.can_delete(&player).is_ok());
    }

    #[test]
    fn test_export_bundle() {
        let graph = create_test_graph();
        let map = AssetId::new(AssetType::Map, "maps/level1.tmx");

        let bundle = graph.get_export_bundle(&map);
        assert_eq!(bundle.len(), 3); // map + tileset + texture
    }

    #[test]
    fn test_analysis() {
        let graph = create_test_graph();
        let analysis = DependencyAnalysis::analyze(&graph);

        assert_eq!(analysis.total_assets, 5);
        assert_eq!(analysis.orphaned_count, 1);
        assert_eq!(analysis.by_type.get(&AssetType::Map), Some(&1));
    }
}
