//! Tilemap editing tools

use super::layer::{LayerType, Tile};
use super::map::TileMap;

/// Tool types for editing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ToolType {
    /// Paint single tiles or brush
    #[default]
    Brush,
    /// Erase tiles
    Eraser,
    /// Flood fill
    Fill,
    /// Select area
    Select,
    /// Pick tile
    Eyedropper,
    /// Place collision markers
    Collision,
    /// Place event markers
    Event,
}

impl ToolType {
    /// Get display name
    pub fn name(&self) -> &'static str {
        match self {
            ToolType::Brush => "Brush",
            ToolType::Eraser => "Eraser",
            ToolType::Fill => "Fill",
            ToolType::Select => "Select",
            ToolType::Eyedropper => "Eyedropper",
            ToolType::Collision => "Collision",
            ToolType::Event => "Event",
        }
    }

    /// Get keyboard shortcut
    pub fn shortcut(&self) -> &'static str {
        match self {
            ToolType::Brush => "B",
            ToolType::Eraser => "E",
            ToolType::Fill => "F",
            ToolType::Select => "S",
            ToolType::Eyedropper => "I",
            ToolType::Collision => "C",
            ToolType::Event => "V",
        }
    }
}

/// Brush sizes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BrushSize {
    #[default]
    Small = 1, // 1x1
    Medium = 3, // 3x3
    Large = 5,  // 5x5
}

impl BrushSize {
    /// Get size in tiles
    pub fn size(&self) -> u32 {
        *self as u32
    }

    /// Get all brush sizes
    pub fn all() -> [BrushSize; 3] {
        [BrushSize::Small, BrushSize::Medium, BrushSize::Large]
    }

    /// Get display name
    pub fn name(&self) -> &'static str {
        match self {
            BrushSize::Small => "1x1",
            BrushSize::Medium => "3x3",
            BrushSize::Large => "5x5",
        }
    }
}

/// Tool state
#[derive(Debug, Clone)]
pub struct ToolState {
    /// Current active tool
    pub current_tool: ToolType,
    /// Brush size
    pub brush_size: BrushSize,
    /// Currently selected tile
    pub selected_tile: Tile,
    /// Selected tileset ID
    pub selected_tileset: u32,
    /// Whether we're currently dragging
    pub is_dragging: bool,
    /// Drag start position
    pub drag_start: Option<(u32, u32)>,
    /// Current mouse position in tile coords
    pub current_tile_pos: Option<(u32, u32)>,
    /// Last painted positions (to avoid repainting same tile)
    pub last_painted: Vec<(u32, u32)>,
    /// Undo stack
    pub undo_stack: Vec<EditAction>,
    /// Redo stack
    pub redo_stack: Vec<EditAction>,
    /// Maximum undo history
    pub max_undo: usize,
}

impl Default for ToolState {
    fn default() -> Self {
        Self {
            current_tool: ToolType::Brush,
            brush_size: BrushSize::Small,
            selected_tile: Tile::new(0, 0),
            selected_tileset: 0,
            is_dragging: false,
            drag_start: None,
            current_tile_pos: None,
            last_painted: Vec::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_undo: 50,
        }
    }
}

impl ToolState {
    /// Create new tool state
    pub fn new() -> Self {
        Self::default()
    }

    /// Set current tool
    pub fn set_tool(&mut self, tool: ToolType) {
        self.current_tool = tool;
    }

    /// Set brush size
    pub fn set_brush_size(&mut self, size: BrushSize) {
        self.brush_size = size;
    }

    /// Select a tile
    pub fn select_tile(&mut self, tileset_id: u32, tile_index: u32) {
        self.selected_tileset = tileset_id;
        self.selected_tile = Tile::new(tileset_id, tile_index);
    }

    /// Start drag operation
    pub fn start_drag(&mut self, x: u32, y: u32) {
        self.is_dragging = true;
        self.drag_start = Some((x, y));
        self.last_painted.clear();
    }

    /// End drag operation
    pub fn end_drag(&mut self) {
        self.is_dragging = false;
        self.drag_start = None;
        self.last_painted.clear();
    }

    /// Update current tile position
    pub fn set_current_pos(&mut self, pos: Option<(u32, u32)>) {
        self.current_tile_pos = pos;
    }

    /// Check if we can paint at this position (avoid duplicates while dragging)
    pub fn can_paint_at(&self, x: u32, y: u32) -> bool {
        !self.last_painted.contains(&(x, y))
    }

    /// Mark position as painted
    pub fn mark_painted(&mut self, x: u32, y: u32) {
        self.last_painted.push((x, y));
    }

    /// Push action to undo stack
    pub fn push_undo(&mut self, action: EditAction) {
        self.undo_stack.push(action);
        if self.undo_stack.len() > self.max_undo {
            self.undo_stack.remove(0);
        }
        // Clear redo stack on new action
        self.redo_stack.clear();
    }

    /// Undo last action
    pub fn undo(&mut self, map: &mut TileMap) -> bool {
        if let Some(action) = self.undo_stack.pop() {
            let inverse = action.apply_inverse(map);
            self.redo_stack.push(inverse);
            true
        } else {
            false
        }
    }

    /// Redo last undone action
    pub fn redo(&mut self, map: &mut TileMap) -> bool {
        if let Some(action) = self.redo_stack.pop() {
            let inverse = action.apply(map);
            self.undo_stack.push(inverse);
            true
        } else {
            false
        }
    }

    /// Get tiles affected by brush at position
    pub fn get_brush_tiles(
        &self,
        center_x: u32,
        center_y: u32,
        map_width: u32,
        map_height: u32,
    ) -> Vec<(u32, u32)> {
        let size = self.brush_size.size();
        let radius = size / 2;

        let mut tiles = Vec::new();

        for dy in 0..size {
            for dx in 0..size {
                let x = center_x.saturating_sub(radius).saturating_add(dx);
                let y = center_y.saturating_sub(radius).saturating_add(dy);

                if x < map_width && y < map_height {
                    tiles.push((x, y));
                }
            }
        }

        tiles
    }
}

/// An edit action for undo/redo
#[derive(Debug, Clone)]
pub enum EditAction {
    /// Paint a single tile
    PaintTile {
        x: u32,
        y: u32,
        layer: LayerType,
        old_tile: Tile,
        new_tile: Tile,
    },
    /// Paint multiple tiles
    PaintTiles {
        layer: LayerType,
        changes: Vec<TileChange>,
    },
    /// Fill area
    Fill {
        layer: LayerType,
        changes: Vec<TileChange>,
    },
    /// Clear layer
    ClearLayer {
        layer: LayerType,
        old_tiles: Vec<Tile>,
    },
}

/// A single tile change
#[derive(Debug, Clone)]
pub struct TileChange {
    pub x: u32,
    pub y: u32,
    pub old_tile: Tile,
    pub new_tile: Tile,
}

impl EditAction {
    /// Apply the action and return the inverse action
    pub fn apply(&self, map: &mut TileMap) -> EditAction {
        match self {
            EditAction::PaintTile {
                x,
                y,
                layer,
                old_tile,
                new_tile,
            } => {
                if let Some(layer_ref) = map.get_layer_mut(*layer) {
                    layer_ref.set_tile(*x, *y, *new_tile);
                }
                EditAction::PaintTile {
                    x: *x,
                    y: *y,
                    layer: *layer,
                    old_tile: *new_tile,
                    new_tile: *old_tile,
                }
            }
            EditAction::PaintTiles { layer, changes } => {
                let mut new_changes = Vec::new();
                for change in changes {
                    if let Some(layer_ref) = map.get_layer_mut(*layer) {
                        let current = *layer_ref
                            .get_tile(change.x, change.y)
                            .unwrap_or(&Tile::empty());
                        layer_ref.set_tile(change.x, change.y, change.new_tile);
                        new_changes.push(TileChange {
                            x: change.x,
                            y: change.y,
                            old_tile: change.new_tile,
                            new_tile: current,
                        });
                    }
                }
                EditAction::PaintTiles {
                    layer: *layer,
                    changes: new_changes,
                }
            }
            EditAction::Fill { .. } => {
                // Same as PaintTiles
                self.apply(map)
            }
            EditAction::ClearLayer { layer, .. } => {
                if let Some(layer_ref) = map.get_layer_mut(*layer) {
                    let current: Vec<Tile> = layer_ref.tiles.clone();
                    layer_ref.clear();
                    EditAction::ClearLayer {
                        layer: *layer,
                        old_tiles: current,
                    }
                } else {
                    self.clone()
                }
            }
        }
    }

    /// Apply the inverse action
    pub fn apply_inverse(&self, map: &mut TileMap) -> EditAction {
        match self {
            EditAction::PaintTile {
                x,
                y,
                layer,
                old_tile,
                ..
            } => {
                if let Some(layer_ref) = map.get_layer_mut(*layer) {
                    layer_ref.set_tile(*x, *y, *old_tile);
                }
                self.clone()
            }
            EditAction::PaintTiles { layer, changes } => {
                for change in changes {
                    if let Some(layer_ref) = map.get_layer_mut(*layer) {
                        layer_ref.set_tile(change.x, change.y, change.old_tile);
                    }
                }
                self.clone()
            }
            EditAction::Fill { layer, changes } => {
                for change in changes {
                    if let Some(layer_ref) = map.get_layer_mut(*layer) {
                        layer_ref.set_tile(change.x, change.y, change.old_tile);
                    }
                }
                self.clone()
            }
            EditAction::ClearLayer { layer, .. } => {
                if let Some(layer_ref) = map.get_layer_mut(*layer) {
                    layer_ref.clear();
                }
                self.clone()
            }
        }
    }
}

/// Tool operations
pub struct ToolOps;

impl ToolOps {
    /// Paint with brush at position
    pub fn brush(
        map: &mut TileMap,
        tools: &mut ToolState,
        x: u32,
        y: u32,
        layer: LayerType,
    ) -> Option<EditAction> {
        let tiles = tools.get_brush_tiles(x, y, map.width, map.height);
        let mut changes = Vec::new();

        for (tx, ty) in tiles {
            // Skip if already painted this drag
            if tools.is_dragging && !tools.can_paint_at(tx, ty) {
                continue;
            }

            if let Some(layer_ref) = map.get_layer(layer) {
                let old_tile = *layer_ref.get_tile(tx, ty).unwrap_or(&Tile::empty());
                let new_tile = tools.selected_tile;

                changes.push(TileChange {
                    x: tx,
                    y: ty,
                    old_tile,
                    new_tile,
                });

                if let Some(layer_ref) = map.get_layer_mut(layer) {
                    layer_ref.set_tile(tx, ty, new_tile);
                }

                tools.mark_painted(tx, ty);
            }
        }

        if changes.is_empty() {
            None
        } else {
            Some(EditAction::PaintTiles { layer, changes })
        }
    }

    /// Erase at position
    pub fn erase(
        map: &mut TileMap,
        tools: &mut ToolState,
        x: u32,
        y: u32,
        layer: LayerType,
    ) -> Option<EditAction> {
        let tiles = tools.get_brush_tiles(x, y, map.width, map.height);
        let mut changes = Vec::new();

        for (tx, ty) in tiles {
            if tools.is_dragging && !tools.can_paint_at(tx, ty) {
                continue;
            }

            if let Some(layer_ref) = map.get_layer(layer) {
                let old_tile = *layer_ref.get_tile(tx, ty).unwrap_or(&Tile::empty());

                if !old_tile.is_empty() {
                    changes.push(TileChange {
                        x: tx,
                        y: ty,
                        old_tile,
                        new_tile: Tile::empty(),
                    });

                    if let Some(layer_ref) = map.get_layer_mut(layer) {
                        layer_ref.set_tile(tx, ty, Tile::empty());
                    }

                    tools.mark_painted(tx, ty);
                }
            }
        }

        if changes.is_empty() {
            None
        } else {
            Some(EditAction::PaintTiles { layer, changes })
        }
    }

    /// Flood fill at position
    pub fn fill(
        map: &mut TileMap,
        tools: &ToolState,
        x: u32,
        y: u32,
        layer: LayerType,
    ) -> Option<EditAction> {
        let target_tile = map.get_tile_at(x, y, layer)?;
        let fill_tile = tools.selected_tile;

        // Don't fill if clicking on same tile
        if target_tile.tileset_id == fill_tile.tileset_id
            && target_tile.tile_index == fill_tile.tile_index
        {
            return None;
        }

        let mut changes = Vec::new();
        let mut visited = vec![vec![false; map.height as usize]; map.width as usize];
        let mut stack = vec![(x, y)];

        while let Some((cx, cy)) = stack.pop() {
            if cx >= map.width || cy >= map.height {
                continue;
            }
            if visited[cx as usize][cy as usize] {
                continue;
            }

            let current = map.get_tile_at(cx, cy, layer)?;
            if current.tileset_id != target_tile.tileset_id
                || current.tile_index != target_tile.tile_index
            {
                continue;
            }

            visited[cx as usize][cy as usize] = true;

            changes.push(TileChange {
                x: cx,
                y: cy,
                old_tile: *current,
                new_tile: fill_tile,
            });

            // Push neighbors
            if cx > 0 {
                stack.push((cx - 1, cy));
            }
            if cx < map.width - 1 {
                stack.push((cx + 1, cy));
            }
            if cy > 0 {
                stack.push((cx, cy - 1));
            }
            if cy < map.height - 1 {
                stack.push((cx, cy + 1));
            }
        }

        // Apply changes
        for change in &changes {
            if let Some(layer_ref) = map.get_layer_mut(layer) {
                layer_ref.set_tile(change.x, change.y, change.new_tile);
            }
        }

        if changes.is_empty() {
            None
        } else {
            Some(EditAction::Fill { layer, changes })
        }
    }

    /// Eyedropper - pick tile at position
    pub fn eyedropper(
        map: &TileMap,
        tools: &mut ToolState,
        x: u32,
        y: u32,
        layer: LayerType,
    ) -> bool {
        if let Some(tile) = map.get_tile_at(x, y, layer) {
            if !tile.is_empty() {
                tools.select_tile(tile.tileset_id, tile.tile_index);
                tools.current_tool = ToolType::Brush;
                return true;
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_brush_sizes() {
        assert_eq!(BrushSize::Small.size(), 1);
        assert_eq!(BrushSize::Medium.size(), 3);
        assert_eq!(BrushSize::Large.size(), 5);
    }

    #[test]
    fn test_get_brush_tiles() {
        let tools = ToolState {
            brush_size: BrushSize::Medium,
            ..Default::default()
        };

        let tiles = tools.get_brush_tiles(5, 5, 100, 100);
        assert_eq!(tiles.len(), 9); // 3x3
        assert!(tiles.contains(&(5, 5))); // Center
        assert!(tiles.contains(&(4, 4))); // Corner
    }

    #[test]
    fn test_paint_undo() {
        let mut map = TileMap::new("test", "Test", 10, 10, 32);
        let mut tools = ToolState::new();
        tools.select_tile(1, 5);

        // Paint a tile
        if let Some(action) = ToolOps::brush(&mut map, &mut tools, 5, 5, LayerType::Terrain) {
            tools.push_undo(action);
        }

        // Verify tile was painted
        let tile = map.get_tile_at(5, 5, LayerType::Terrain).unwrap();
        assert_eq!(tile.tile_index, 5);

        // Undo
        tools.undo(&mut map);

        // Verify tile was restored
        let tile = map.get_tile_at(5, 5, LayerType::Terrain).unwrap();
        assert!(tile.is_empty());
    }
}
