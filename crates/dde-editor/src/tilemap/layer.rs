//! Tilemap layer system
//!
//! Supports 5 layers as per the blueprint:
//! - Terrain (layer 0): Ground tiles, floors
//! - Objects (layer 1): Trees, buildings, decorations
//! - Collision (layer 2): Walkability, collision data
//! - Regions (layer 3): Trigger zones, areas
//! - Events (layer 4): Event markers, spawn points

use serde::{Deserialize, Serialize};

/// Layer types for the tilemap
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum LayerType {
    /// Ground/terrain layer
    #[default]
    Terrain = 0,
    /// Objects and decorations
    Objects = 1,
    /// Collision data
    Collision = 2,
    /// Region/trigger zones
    Regions = 3,
    /// Event markers
    Events = 4,
}

impl LayerType {
    /// Get all layer types in order
    pub fn all() -> [LayerType; 5] {
        [
            LayerType::Terrain,
            LayerType::Objects,
            LayerType::Collision,
            LayerType::Regions,
            LayerType::Events,
        ]
    }

    /// Get display name
    pub fn name(&self) -> &'static str {
        match self {
            LayerType::Terrain => "Terrain",
            LayerType::Objects => "Objects",
            LayerType::Collision => "Collision",
            LayerType::Regions => "Regions",
            LayerType::Events => "Events",
        }
    }

    /// Check if this layer is visible by default
    pub fn visible_by_default(&self) -> bool {
        match self {
            LayerType::Terrain | LayerType::Objects => true,
            LayerType::Collision | LayerType::Regions | LayerType::Events => false,
        }
    }

    /// Check if this layer supports transparency
    pub fn supports_transparency(&self) -> bool {
        !matches!(self, LayerType::Terrain)
    }
}

/// A single tile in a layer
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Tile {
    /// Tileset ID
    pub tileset_id: u32,
    /// Tile index within the tileset
    pub tile_index: u32,
    /// Whether this tile is empty
    pub empty: bool,
    /// Flip horizontally
    pub flip_h: bool,
    /// Flip vertically
    pub flip_v: bool,
    /// Rotation (0, 90, 180, 270 degrees)
    pub rotation: u8,
}

impl Tile {
    /// Create a new tile
    pub fn new(tileset_id: u32, tile_index: u32) -> Self {
        Self {
            tileset_id,
            tile_index,
            empty: false,
            flip_h: false,
            flip_v: false,
            rotation: 0,
        }
    }

    /// Create an empty tile
    pub fn empty() -> Self {
        Self {
            tileset_id: 0,
            tile_index: 0,
            empty: true,
            flip_h: false,
            flip_v: false,
            rotation: 0,
        }
    }

    /// Check if tile is empty
    pub fn is_empty(&self) -> bool {
        self.empty
    }

    /// Set flip horizontally
    pub fn with_flip_h(mut self, flip: bool) -> Self {
        self.flip_h = flip;
        self
    }

    /// Set flip vertically
    pub fn with_flip_v(mut self, flip: bool) -> Self {
        self.flip_v = flip;
        self
    }

    /// Set rotation
    pub fn with_rotation(mut self, rotation: u8) -> Self {
        self.rotation = rotation % 4;
        self
    }
}

/// A layer in the tilemap
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TileLayer {
    /// Layer type
    pub layer_type: LayerType,
    /// Layer name
    pub name: String,
    /// Layer dimensions
    pub width: u32,
    pub height: u32,
    /// Tile data (row-major order)
    pub tiles: Vec<Tile>,
    /// Whether layer is visible
    pub visible: bool,
    /// Layer opacity (0.0 - 1.0)
    pub opacity: f32,
    /// Locked (cannot edit)
    pub locked: bool,
}

impl TileLayer {
    /// Create a new empty layer
    pub fn new(layer_type: LayerType, width: u32, height: u32) -> Self {
        let tile_count = (width * height) as usize;
        Self {
            layer_type,
            name: layer_type.name().to_string(),
            width,
            height,
            tiles: vec![Tile::empty(); tile_count],
            visible: layer_type.visible_by_default(),
            opacity: 1.0,
            locked: false,
        }
    }

    /// Get tile at position
    pub fn get_tile(&self, x: u32, y: u32) -> Option<&Tile> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let index = (y * self.width + x) as usize;
        self.tiles.get(index)
    }

    /// Get mutable tile at position
    pub fn get_tile_mut(&mut self, x: u32, y: u32) -> Option<&mut Tile> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let index = (y * self.width + x) as usize;
        self.tiles.get_mut(index)
    }

    /// Set tile at position
    pub fn set_tile(&mut self, x: u32, y: u32, tile: Tile) -> bool {
        if let Some(t) = self.get_tile_mut(x, y) {
            *t = tile;
            true
        } else {
            false
        }
    }

    /// Clear tile at position
    pub fn clear_tile(&mut self, x: u32, y: u32) -> bool {
        self.set_tile(x, y, Tile::empty())
    }

    /// Fill entire layer with a tile
    pub fn fill(&mut self, tile: Tile) {
        for t in &mut self.tiles {
            *t = tile;
        }
    }

    /// Clear entire layer
    pub fn clear(&mut self) {
        self.fill(Tile::empty());
    }

    /// Resize layer (clears data)
    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        self.tiles = vec![Tile::empty(); (width * height) as usize];
    }

    /// Get tile index from coordinates
    pub fn coords_to_index(&self, x: u32, y: u32) -> Option<usize> {
        if x >= self.width || y >= self.height {
            return None;
        }
        Some((y * self.width + x) as usize)
    }

    /// Get coordinates from tile index
    pub fn index_to_coords(&self, index: usize) -> Option<(u32, u32)> {
        if index >= self.tiles.len() {
            return None;
        }
        let x = (index % self.width as usize) as u32;
        let y = (index / self.width as usize) as u32;
        Some((x, y))
    }
}

/// Collision tile values
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollisionValue {
    /// Walkable
    Walkable = 0,
    /// Blocked
    Blocked = 1,
    /// Water (swimmable)
    Water = 2,
    /// Ledge (can jump down)
    Ledge = 3,
}

impl CollisionValue {
    pub fn from_tile(tile: &Tile) -> Self {
        if tile.empty {
            CollisionValue::Walkable
        } else {
            match tile.tile_index {
                0 => CollisionValue::Walkable,
                1 => CollisionValue::Blocked,
                2 => CollisionValue::Water,
                3 => CollisionValue::Ledge,
                _ => CollisionValue::Blocked,
            }
        }
    }

    pub fn to_tile(&self) -> Tile {
        Tile::new(0, *self as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layer_creation() {
        let layer = TileLayer::new(LayerType::Terrain, 10, 10);
        assert_eq!(layer.width, 10);
        assert_eq!(layer.height, 10);
        assert_eq!(layer.tiles.len(), 100);
        assert!(layer.visible);
    }

    #[test]
    fn test_tile_operations() {
        let mut layer = TileLayer::new(LayerType::Objects, 5, 5);

        let tile = Tile::new(1, 5);
        layer.set_tile(2, 2, tile);

        let retrieved = layer.get_tile(2, 2).unwrap();
        assert_eq!(retrieved.tileset_id, 1);
        assert_eq!(retrieved.tile_index, 5);

        layer.clear_tile(2, 2);
        assert!(layer.get_tile(2, 2).unwrap().is_empty());
    }

    #[test]
    fn test_out_of_bounds() {
        let mut layer = TileLayer::new(LayerType::Terrain, 5, 5);

        assert!(layer.get_tile(10, 10).is_none());
        assert!(!layer.set_tile(10, 10, Tile::new(1, 1)));
    }

    #[test]
    fn test_fill() {
        let mut layer = TileLayer::new(LayerType::Terrain, 3, 3);
        let tile = Tile::new(1, 2);

        layer.fill(tile);

        for y in 0..3 {
            for x in 0..3 {
                let t = layer.get_tile(x, y).unwrap();
                assert_eq!(t.tileset_id, 1);
                assert_eq!(t.tile_index, 2);
            }
        }
    }

    #[test]
    fn test_coords_conversion() {
        let layer = TileLayer::new(LayerType::Terrain, 10, 10);

        assert_eq!(layer.coords_to_index(5, 5), Some(55));
        assert_eq!(layer.index_to_coords(55), Some((5, 5)));

        assert_eq!(layer.coords_to_index(15, 5), None);
    }
}
