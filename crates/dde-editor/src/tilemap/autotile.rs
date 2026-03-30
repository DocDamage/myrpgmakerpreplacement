//! Autotiling System
//!
//! Smart tile placement with automatic edge/connection detection.
//! Supports 16-tile and 47-tile autotile sets.

/// Autotile rule set type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutotileType {
    /// Simple 4-tile set (corners only)
    Simple4,
    /// Standard 16-tile set ( Wang tiles )
    Wang16,
    /// Extended 47-tile set (blob pattern)
    Blob47,
}

impl AutotileType {
    /// Get the number of tiles in this set
    pub fn tile_count(&self) -> usize {
        match self {
            AutotileType::Simple4 => 4,
            AutotileType::Wang16 => 16,
            AutotileType::Blob47 => 47,
        }
    }

    /// Get tile index based on neighbor mask
    pub fn get_tile_index(&self, mask: NeighborMask) -> u32 {
        match self {
            AutotileType::Simple4 => mask.to_simple4(),
            AutotileType::Wang16 => mask.to_wang16(),
            AutotileType::Blob47 => mask.to_blob47(),
        }
    }
}

/// Neighbor mask representing 8 surrounding tiles
/// Bits: 0=N, 1=NE, 2=E, 3=SE, 4=S, 5=SW, 6=W, 7=NW
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NeighborMask(pub u8);

impl NeighborMask {
    /// Create mask from cardinal directions
    pub fn from_cardinals(n: bool, e: bool, s: bool, w: bool) -> Self {
        let mut mask = 0u8;
        if n {
            mask |= 1 << 0;
        }
        if e {
            mask |= 1 << 2;
        }
        if s {
            mask |= 1 << 4;
        }
        if w {
            mask |= 1 << 6;
        }
        Self(mask)
    }

    /// Create mask from all 8 directions
    pub fn from_all(
        n: bool,
        ne: bool,
        e: bool,
        se: bool,
        s: bool,
        sw: bool,
        w: bool,
        nw: bool,
    ) -> Self {
        let mut mask = 0u8;
        if n {
            mask |= 1 << 0;
        }
        if ne {
            mask |= 1 << 1;
        }
        if e {
            mask |= 1 << 2;
        }
        if se {
            mask |= 1 << 3;
        }
        if s {
            mask |= 1 << 4;
        }
        if sw {
            mask |= 1 << 5;
        }
        if w {
            mask |= 1 << 6;
        }
        if nw {
            mask |= 1 << 7;
        }
        Self(mask)
    }

    /// Check if north neighbor is set
    pub fn north(&self) -> bool {
        self.0 & (1 << 0) != 0
    }

    /// Check if east neighbor is set
    pub fn east(&self) -> bool {
        self.0 & (1 << 2) != 0
    }

    /// Check if south neighbor is set
    pub fn south(&self) -> bool {
        self.0 & (1 << 4) != 0
    }

    /// Check if west neighbor is set
    pub fn west(&self) -> bool {
        self.0 & (1 << 6) != 0
    }

    /// Convert to Simple4 index (0-3)
    /// Returns corner index based on N/E presence
    pub fn to_simple4(&self) -> u32 {
        let n = self.north() as u32;
        let e = self.east() as u32;
        (n << 1) | e
    }

    /// Convert to Wang16 index (0-15)
    /// Classic Wang tile indexing: N|E|S|W as 4-bit number
    pub fn to_wang16(&self) -> u32 {
        let n = self.north() as u32;
        let e = self.east() as u32;
        let s = self.south() as u32;
        let w = self.west() as u32;
        (n << 3) | (w << 2) | (s << 1) | e
    }

    /// Convert to Blob47 index (0-46)
    /// Uses precomputed lookup table for blob pattern
    pub fn to_blob47(&self) -> u32 {
        BLOB47_TABLE[self.0 as usize]
    }
}

/// Blob47 lookup table
/// Maps 8-bit neighbor mask to 47-tile set index
const BLOB47_TABLE: [u32; 256] = {
    let mut table = [0u32; 256];

    // This is a simplified blob table
    // Full implementation would have all 47 variations
    // See: https://www.cr31.co.uk/stagecast/wang/blob.html

    let mut i = 0u32;
    while i < 256 {
        let mask = i as u8;

        // Count connected neighbors
        let cardinal_count = (mask & 1) + ((mask >> 2) & 1) + ((mask >> 4) & 1) + ((mask >> 6) & 1);

        // Simple mapping based on connection count
        table[i as usize] = match cardinal_count {
            0 => 0,  // Isolated
            1 => 1,  // End piece
            2 => 2,  // Corner or straight
            3 => 10, // T-junction
            4 => 22, // Cross
            _ => 0,
        };

        i += 1;
    }

    table
};

/// Autotile configuration for a terrain type
#[derive(Debug, Clone)]
pub struct AutotileConfig {
    /// Name of this autotile set
    pub name: String,
    /// Type of autotile
    pub autotile_type: AutotileType,
    /// Base tile ID in tileset
    pub base_tile_id: u32,
    /// Tileset this autotile belongs to
    pub tileset_id: String,
}

impl AutotileConfig {
    /// Create a new autotile configuration
    pub fn new(name: impl Into<String>, autotile_type: AutotileType, base_tile_id: u32) -> Self {
        Self {
            name: name.into(),
            autotile_type,
            base_tile_id,
            tileset_id: String::new(),
        }
    }

    /// Get the actual tile index for a neighbor mask
    pub fn get_tile(&self, mask: NeighborMask) -> u32 {
        self.base_tile_id + self.autotile_type.get_tile_index(mask)
    }
}

/// Smart terrain brush that uses autotiling
#[derive(Debug, Clone)]
pub struct SmartBrush {
    /// Current autotile configuration
    pub config: AutotileConfig,
    /// Whether to update neighboring tiles
    pub update_neighbors: bool,
}

impl SmartBrush {
    /// Create a new smart brush
    pub fn new(config: AutotileConfig) -> Self {
        Self {
            config,
            update_neighbors: true,
        }
    }

    /// Calculate neighbor mask at position
    pub fn calculate_mask<F>(&self, x: i32, y: i32, is_same_terrain: F) -> NeighborMask
    where
        F: Fn(i32, i32) -> bool,
    {
        NeighborMask::from_all(
            is_same_terrain(x, y - 1),     // N
            is_same_terrain(x + 1, y - 1), // NE
            is_same_terrain(x + 1, y),     // E
            is_same_terrain(x + 1, y + 1), // SE
            is_same_terrain(x, y + 1),     // S
            is_same_terrain(x - 1, y + 1), // SW
            is_same_terrain(x - 1, y),     // W
            is_same_terrain(x - 1, y - 1), // NW
        )
    }

    /// Get tile index for position
    pub fn get_tile_at<F>(&self, x: i32, y: i32, is_same_terrain: F) -> u32
    where
        F: Fn(i32, i32) -> bool,
    {
        let mask = self.calculate_mask(x, y, is_same_terrain);
        self.config.get_tile(mask)
    }
}

/// Terrain type definitions
#[derive(Debug, Clone)]
pub struct TerrainType {
    pub id: String,
    pub name: String,
    pub autotile_config: Option<AutotileConfig>,
    /// Fallback tile if not using autotile
    pub fallback_tile: u32,
}

impl TerrainType {
    /// Create a new terrain type with autotile
    pub fn with_autotile(
        id: impl Into<String>,
        name: impl Into<String>,
        config: AutotileConfig,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            autotile_config: Some(config),
            fallback_tile: 0,
        }
    }

    /// Create a simple terrain type without autotile
    pub fn simple(id: impl Into<String>, name: impl Into<String>, tile: u32) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            autotile_config: None,
            fallback_tile: tile,
        }
    }

    /// Get tile for position
    pub fn get_tile<F>(&self, x: i32, y: i32, is_same_terrain: F) -> u32
    where
        F: Fn(i32, i32) -> bool,
    {
        match &self.autotile_config {
            Some(config) => {
                let brush = SmartBrush::new(config.clone());
                brush.get_tile_at(x, y, is_same_terrain)
            }
            None => self.fallback_tile,
        }
    }
}

/// Terrain set with multiple terrain types
#[derive(Debug, Clone, Default)]
pub struct TerrainSet {
    terrains: Vec<TerrainType>,
    current_terrain: usize,
}

impl TerrainSet {
    /// Create a new terrain set
    pub fn new() -> Self {
        Self {
            terrains: Vec::new(),
            current_terrain: 0,
        }
    }

    /// Add a terrain type
    pub fn add(&mut self, terrain: TerrainType) {
        self.terrains.push(terrain);
    }

    /// Get current terrain
    pub fn current(&self) -> Option<&TerrainType> {
        self.terrains.get(self.current_terrain)
    }

    /// Set current terrain by index
    pub fn set_current(&mut self, index: usize) {
        if index < self.terrains.len() {
            self.current_terrain = index;
        }
    }

    /// Get all terrains
    pub fn all(&self) -> &[TerrainType] {
        &self.terrains
    }

    /// Get terrain by ID
    pub fn get(&self, id: &str) -> Option<&TerrainType> {
        self.terrains.iter().find(|t| t.id == id)
    }

    /// Create default terrain set
    pub fn default_set() -> Self {
        let mut set = Self::new();

        // Grass with Wang16 autotile
        set.add(TerrainType::with_autotile(
            "grass",
            "Grass",
            AutotileConfig::new("grass", AutotileType::Wang16, 0),
        ));

        // Water with Blob47 autotile
        set.add(TerrainType::with_autotile(
            "water",
            "Water",
            AutotileConfig::new("water", AutotileType::Blob47, 16),
        ));

        // Dirt with Wang16 autotile
        set.add(TerrainType::with_autotile(
            "dirt",
            "Dirt",
            AutotileConfig::new("dirt", AutotileType::Wang16, 63),
        ));

        // Simple stone (no autotile)
        set.add(TerrainType::simple("stone", "Stone", 79));

        set
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_neighbor_mask() {
        let mask = NeighborMask::from_cardinals(true, true, false, false);
        assert!(mask.north());
        assert!(mask.east());
        assert!(!mask.south());
        assert!(!mask.west());
    }

    #[test]
    fn test_wang16_conversion() {
        // All neighbors present = 15 (binary 1111)
        let mask = NeighborMask::from_cardinals(true, true, true, true);
        assert_eq!(mask.to_wang16(), 15);

        // Only north = 8 (binary 1000)
        let mask = NeighborMask::from_cardinals(true, false, false, false);
        assert_eq!(mask.to_wang16(), 8);

        // Only east = 1 (binary 0001)
        let mask = NeighborMask::from_cardinals(false, true, false, false);
        assert_eq!(mask.to_wang16(), 1);
    }

    #[test]
    fn test_autotile_config() {
        let config = AutotileConfig::new("test", AutotileType::Wang16, 10);

        let mask = NeighborMask::from_cardinals(true, false, true, false);
        let tile = config.get_tile(mask);

        // Base (10) + wang16 index for N|S (10 in binary)
        assert_eq!(tile, 10 + 10);
    }

    #[test]
    fn test_autotile_type_count() {
        assert_eq!(AutotileType::Simple4.tile_count(), 4);
        assert_eq!(AutotileType::Wang16.tile_count(), 16);
        assert_eq!(AutotileType::Blob47.tile_count(), 47);
    }

    #[test]
    fn test_smart_brush() {
        let config = AutotileConfig::new("grass", AutotileType::Wang16, 0);
        let brush = SmartBrush::new(config);

        // Create a simple terrain checker
        let is_grass = |x: i32, y: i32| x >= 0 && x < 2 && y >= 0 && y < 2;

        // Center of 2x2 grass patch
        let tile = brush.get_tile_at(0, 0, is_grass);
        assert!(tile < 16); // Should be a valid Wang16 tile
    }

    #[test]
    fn test_terrain_set() {
        let set = TerrainSet::default_set();

        assert_eq!(set.all().len(), 4);
        assert!(set.get("grass").is_some());
        assert!(set.get("water").is_some());
        assert!(set.get("nonexistent").is_none());
    }
}
