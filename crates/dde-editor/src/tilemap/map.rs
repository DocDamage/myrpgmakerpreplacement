//! Tilemap data structure

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::layer::{LayerType, Tile, TileLayer};

/// A complete tilemap with multiple layers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TileMap {
    /// Map ID
    pub id: String,
    /// Map name
    pub name: String,
    /// Map dimensions in tiles
    pub width: u32,
    pub height: u32,
    /// Tile size in pixels
    pub tile_size: u32,
    /// Layers
    pub layers: Vec<TileLayer>,
    /// Tileset references
    pub tilesets: Vec<TilesetRef>,
    /// Map properties
    pub properties: HashMap<String, String>,
}

/// Reference to a tileset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TilesetRef {
    /// Tileset ID
    pub id: u32,
    /// Tileset name
    pub name: String,
    /// Path to tileset image
    pub image_path: String,
    /// Tile width
    pub tile_width: u32,
    /// Tile height
    pub tile_height: u32,
    /// Spacing between tiles
    pub spacing: u32,
    /// Margin around tiles
    pub margin: u32,
    /// First global tile ID
    pub first_gid: u32,
}

impl TileMap {
    /// Create a new empty tilemap
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        width: u32,
        height: u32,
        tile_size: u32,
    ) -> Self {
        let id = id.into();
        let name = name.into();

        // Create all 5 layers
        let layers = LayerType::all()
            .map(|layer_type| TileLayer::new(layer_type, width, height))
            .to_vec();

        Self {
            id,
            name,
            width,
            height,
            tile_size,
            layers,
            tilesets: Vec::new(),
            properties: HashMap::new(),
        }
    }

    /// Get a layer by type
    pub fn get_layer(&self, layer_type: LayerType) -> Option<&TileLayer> {
        self.layers.iter().find(|l| l.layer_type == layer_type)
    }

    /// Get mutable layer by type
    pub fn get_layer_mut(&mut self, layer_type: LayerType) -> Option<&mut TileLayer> {
        self.layers.iter_mut().find(|l| l.layer_type == layer_type)
    }

    /// Get layer by index
    pub fn get_layer_by_index(&self, index: usize) -> Option<&TileLayer> {
        self.layers.get(index)
    }

    /// Get mutable layer by index
    pub fn get_layer_mut_by_index(&mut self, index: usize) -> Option<&mut TileLayer> {
        self.layers.get_mut(index)
    }

    /// Set layer visibility
    pub fn set_layer_visible(&mut self, layer_type: LayerType, visible: bool) {
        if let Some(layer) = self.get_layer_mut(layer_type) {
            layer.visible = visible;
        }
    }

    /// Set layer opacity
    pub fn set_layer_opacity(&mut self, layer_type: LayerType, opacity: f32) {
        if let Some(layer) = self.get_layer_mut(layer_type) {
            layer.opacity = opacity.clamp(0.0, 1.0);
        }
    }

    /// Lock/unlock layer
    pub fn set_layer_locked(&mut self, layer_type: LayerType, locked: bool) {
        if let Some(layer) = self.get_layer_mut(layer_type) {
            layer.locked = locked;
        }
    }

    /// Get tile at position (from visible layers, top-down)
    pub fn get_tile_at(&self, x: u32, y: u32, layer_type: LayerType) -> Option<&Tile> {
        self.get_layer(layer_type)?.get_tile(x, y)
    }

    /// Set tile at position
    pub fn set_tile_at(&mut self, x: u32, y: u32, layer_type: LayerType, tile: Tile) -> bool {
        if let Some(layer) = self.get_layer_mut(layer_type) {
            if layer.locked {
                return false;
            }
            layer.set_tile(x, y, tile)
        } else {
            false
        }
    }

    /// Clear tile at position
    pub fn clear_tile_at(&mut self, x: u32, y: u32, layer_type: LayerType) -> bool {
        self.set_tile_at(x, y, layer_type, Tile::empty())
    }

    /// Resize the map (clears all data)
    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;

        for layer in &mut self.layers {
            layer.resize(width, height);
        }
    }

    /// Add a tileset reference
    pub fn add_tileset(&mut self, tileset: TilesetRef) {
        self.tilesets.push(tileset);
    }

    /// Get tileset by ID
    pub fn get_tileset(&self, id: u32) -> Option<&TilesetRef> {
        self.tilesets.iter().find(|t| t.id == id)
    }

    /// Convert screen coordinates to tile coordinates
    pub fn screen_to_tile(
        &self,
        screen_x: f32,
        screen_y: f32,
        camera_x: f32,
        camera_y: f32,
        zoom: f32,
    ) -> Option<(u32, u32)> {
        let world_x = (screen_x / zoom) + camera_x;
        let world_y = (screen_y / zoom) + camera_y;

        let tile_x = (world_x / self.tile_size as f32) as i32;
        let tile_y = (world_y / self.tile_size as f32) as i32;

        if tile_x < 0 || tile_y < 0 || tile_x >= self.width as i32 || tile_y >= self.height as i32 {
            return None;
        }

        Some((tile_x as u32, tile_y as u32))
    }

    /// Convert tile coordinates to world position
    pub fn tile_to_world(&self, tile_x: u32, tile_y: u32) -> (f32, f32) {
        (
            tile_x as f32 * self.tile_size as f32,
            tile_y as f32 * self.tile_size as f32,
        )
    }

    /// Get world size in pixels
    pub fn world_size(&self) -> (f32, f32) {
        (
            self.width as f32 * self.tile_size as f32,
            self.height as f32 * self.tile_size as f32,
        )
    }

    /// Check if position is within bounds
    pub fn in_bounds(&self, x: u32, y: u32) -> bool {
        x < self.width && y < self.height
    }

    /// Get collision data as a 2D grid
    pub fn get_collision_grid(&self) -> Vec<Vec<bool>> {
        let mut grid = vec![vec![false; self.height as usize]; self.width as usize];

        if let Some(collision_layer) = self.get_layer(LayerType::Collision) {
            for y in 0..self.height {
                for x in 0..self.width {
                    if let Some(tile) = collision_layer.get_tile(x, y) {
                        // Tile index 1 = blocked
                        grid[x as usize][y as usize] = !tile.empty && tile.tile_index == 1;
                    }
                }
            }
        }

        grid
    }

    /// Export to Tiled JSON format
    pub fn to_tiled_json(&self) -> serde_json::Value {
        let mut layers = Vec::new();

        for (i, layer) in self.layers.iter().enumerate() {
            let data: Vec<u32> = layer
                .tiles
                .iter()
                .map(|t| {
                    if t.empty {
                        0
                    } else {
                        // Calculate global tile ID
                        let gid = t.tile_index + 1;
                        // Add flip/rotation flags if needed
                        let mut final_gid = gid;
                        if t.flip_h {
                            final_gid |= 0x80000000;
                        }
                        if t.flip_v {
                            final_gid |= 0x40000000;
                        }
                        if t.rotation > 0 {
                            final_gid |= 0x20000000;
                        }
                        final_gid
                    }
                })
                .collect();

            layers.push(serde_json::json!({
                "id": i,
                "name": layer.name,
                "type": "tilelayer",
                "width": layer.width,
                "height": layer.height,
                "visible": layer.visible,
                "opacity": layer.opacity,
                "data": data,
            }));
        }

        let tilesets: Vec<serde_json::Value> = self
            .tilesets
            .iter()
            .map(|t| {
                serde_json::json!({
                    "firstgid": t.first_gid,
                    "source": t.name.clone() + ".tsx",
                })
            })
            .collect();

        serde_json::json!({
            "version": 1.10,
            "type": "map",
            "width": self.width,
            "height": self.height,
            "tilewidth": self.tile_size,
            "tileheight": self.tile_size,
            "infinite": false,
            "orientation": "orthogonal",
            "renderorder": "right-down",
            "layers": layers,
            "tilesets": tilesets,
            "properties": self.properties,
        })
    }

    /// Import from Tiled JSON format
    pub fn from_tiled_json(json: &serde_json::Value) -> Option<Self> {
        let width = json.get("width")?.as_u64()? as u32;
        let height = json.get("height")?.as_u64()? as u32;
        let tile_width = json.get("tilewidth")?.as_u64()? as u32;

        let mut map = Self::new("imported", "Imported Map", width, height, tile_width);

        // Parse layers
        if let Some(layers) = json.get("layers")?.as_array() {
            for layer_json in layers {
                let layer_name = layer_json.get("name")?.as_str()?;
                let layer_type = match layer_name {
                    "Terrain" => LayerType::Terrain,
                    "Objects" => LayerType::Objects,
                    "Collision" => LayerType::Collision,
                    "Regions" => LayerType::Regions,
                    "Events" => LayerType::Events,
                    _ => LayerType::Terrain,
                };

                if let Some(data) = layer_json.get("data")?.as_array() {
                    if let Some(layer) = map.get_layer_mut(layer_type) {
                        for (i, gid_val) in data.iter().enumerate() {
                            let gid = gid_val.as_u64()? as u32;
                            if gid > 0 {
                                let x = (i % width as usize) as u32;
                                let y = (i / width as usize) as u32;
                                let tile = Tile::new(0, gid - 1);
                                layer.set_tile(x, y, tile);
                            }
                        }

                        layer.visible = layer_json.get("visible")?.as_bool()?;
                        layer.opacity = layer_json.get("opacity")?.as_f64()? as f32;
                    }
                }
            }
        }

        Some(map)
    }
}

impl Default for TileMap {
    fn default() -> Self {
        Self::new("untitled", "Untitled Map", 64, 64, 32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tilemap_creation() {
        let map = TileMap::new("test", "Test Map", 32, 32, 32);

        assert_eq!(map.width, 32);
        assert_eq!(map.height, 32);
        assert_eq!(map.tile_size, 32);
        assert_eq!(map.layers.len(), 5);
    }

    #[test]
    fn test_layer_access() {
        let mut map = TileMap::new("test", "Test", 10, 10, 32);

        // Test layer access
        let terrain = map.get_layer(LayerType::Terrain).unwrap();
        assert_eq!(terrain.layer_type, LayerType::Terrain);

        // Test setting tile
        let tile = Tile::new(1, 5);
        assert!(map.set_tile_at(5, 5, LayerType::Terrain, tile));

        let retrieved = map.get_tile_at(5, 5, LayerType::Terrain).unwrap();
        assert_eq!(retrieved.tileset_id, 1);
        assert_eq!(retrieved.tile_index, 5);
    }

    #[test]
    fn test_screen_to_tile() {
        let map = TileMap::new("test", "Test", 64, 64, 32);

        // Screen (0, 0) with no camera offset at zoom 1.0
        let result = map.screen_to_tile(0.0, 0.0, 0.0, 0.0, 1.0);
        assert_eq!(result, Some((0, 0)));

        // Screen (32, 32) should be tile (1, 1)
        let result = map.screen_to_tile(32.0, 32.0, 0.0, 0.0, 1.0);
        assert_eq!(result, Some((1, 1)));

        // Out of bounds
        let result = map.screen_to_tile(10000.0, 10000.0, 0.0, 0.0, 1.0);
        assert_eq!(result, None);
    }

    #[test]
    fn test_json_export() {
        let map = TileMap::new("test", "Test Map", 2, 2, 32);

        let json = map.to_tiled_json();

        assert_eq!(json["width"], 2);
        assert_eq!(json["height"], 2);
        assert_eq!(json["tilewidth"], 32);
        assert!(json["layers"].as_array().unwrap().len() > 0);
    }
}
