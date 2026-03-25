//! Resources - shared game state that doesn't belong to entities

use std::collections::HashMap;

use rand::SeedableRng;
use rand::rngs::StdRng;

/// Simulation time tracking
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SimTime {
    /// Total ticks elapsed
    pub tick_count: u64,
    /// In-game time (hour 0-23)
    pub hour: u8,
    /// In-game day
    pub day: u32,
}

impl SimTime {
    /// Ticks per in-game hour
    pub const TICKS_PER_HOUR: u64 = 600; // 10 minutes of real time = 1 game hour
    
    /// Advance by one tick
    pub fn tick(&mut self) {
        self.tick_count += 1;
        
        // Update in-game time
        if self.tick_count % Self::TICKS_PER_HOUR == 0 {
            self.hour += 1;
            if self.hour >= 24 {
                self.hour = 0;
                self.day += 1;
            }
        }
    }
}

/// Global simulation stats
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SimulationStats {
    pub stats: HashMap<String, SimStat>,
}

/// Individual simulation stat
#[derive(Debug, Clone, PartialEq)]
pub struct SimStat {
    /// Normalized value 0.0-1.0
    pub value: f64,
    /// Raw un-normalized value
    pub raw_value: f64,
    /// Display value
    pub display_value: String,
    /// Min value
    pub min_value: f64,
    /// Max value
    pub max_value: f64,
    /// Per-tick decay rate
    pub decay_rate: f64,
}

impl SimStat {
    pub fn new(min: f64, max: f64, initial: f64) -> Self {
        Self {
            value: (initial - min) / (max - min),
            raw_value: initial,
            display_value: format!("{:.1}", initial),
            min_value: min,
            max_value: max,
            decay_rate: 0.0,
        }
    }
    
    pub fn with_decay(mut self, rate: f64) -> Self {
        self.decay_rate = rate;
        self
    }
    
    /// Set raw value and update normalized
    pub fn set_raw(&mut self, value: f64) {
        self.raw_value = value.clamp(self.min_value, self.max_value);
        self.value = (self.raw_value - self.min_value) / (self.max_value - self.min_value);
        self.display_value = format!("{:.1}", self.raw_value);
    }
    
    /// Apply decay
    pub fn tick(&mut self) {
        if self.decay_rate != 0.0 {
            self.set_raw(self.raw_value - self.decay_rate);
        }
    }
}

/// Random number generators for deterministic simulation
pub struct RngPool {
    master: StdRng,
    sim: StdRng,
    battle: StdRng,
    loot: StdRng,
}

impl RngPool {
    /// Salt values for forked RNGs
    const SIM_SALT: u64 = 0x53494D5F53414C54; // "SIM_SALT"
    const BATTLE_SALT: u64 = 0x4241545F53414C54; // "BAT_SALT"
    const LOOT_SALT: u64 = 0x4C4F4F5453414C54; // "LOOTSALT"
    
    /// Create new RNG pool from seed
    pub fn from_seed(seed: u64) -> Self {
        let master = StdRng::seed_from_u64(seed);
        let sim = StdRng::seed_from_u64(seed ^ Self::SIM_SALT);
        let battle = StdRng::seed_from_u64(seed ^ Self::BATTLE_SALT);
        let loot = StdRng::seed_from_u64(seed ^ Self::LOOT_SALT);
        
        Self {
            master,
            sim,
            battle,
            loot,
        }
    }
    
    pub fn master(&mut self) -> &mut StdRng {
        &mut self.master
    }
    
    pub fn sim(&mut self) -> &mut StdRng {
        &mut self.sim
    }
    
    pub fn battle(&mut self) -> &mut StdRng {
        &mut self.battle
    }
    
    pub fn loot(&mut self) -> &mut StdRng {
        &mut self.loot
    }
}

/// Game configuration
#[derive(Debug, Clone, PartialEq)]
pub struct GameConfig {
    pub window_title: String,
    pub window_width: u32,
    pub window_height: u32,
    pub vsync: bool,
    pub fullscreen: bool,
    pub target_fps: u32,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            window_title: "DocDamage Engine".to_string(),
            window_width: 1280,
            window_height: 720,
            vsync: true,
            fullscreen: false,
            target_fps: 60,
        }
    }
}

/// Input state
#[derive(Debug, Clone, PartialEq, Default)]
pub struct InputState {
    /// Currently held keys/actions
    pub held: std::collections::HashSet<String>,
    /// Keys pressed this frame
    pub pressed: std::collections::HashSet<String>,
    /// Keys released this frame
    pub released: std::collections::HashSet<String>,
    /// Mouse position in screen coordinates
    pub mouse_pos: (f32, f32),
    /// Mouse position in world coordinates
    pub mouse_world_pos: (f32, f32),
    /// Mouse button states
    pub mouse_held: [bool; 3],
    pub mouse_pressed: [bool; 3],
    pub mouse_released: [bool; 3],
    /// Scroll delta
    pub scroll_delta: f32,
}

impl InputState {
    pub fn clear_frame(&mut self) {
        self.pressed.clear();
        self.released.clear();
        self.mouse_pressed = [false; 3];
        self.mouse_released = [false; 3];
        self.scroll_delta = 0.0;
    }
    
    pub fn is_held(&self, action: &str) -> bool {
        self.held.contains(action)
    }
    
    pub fn is_pressed(&self, action: &str) -> bool {
        self.pressed.contains(action)
    }
    
    pub fn is_released(&self, action: &str) -> bool {
        self.released.contains(action)
    }
}

/// Asset database (references to loaded assets)
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AssetDB {
    pub tilesets: HashMap<u32, TilesetInfo>,
    pub spritesheets: HashMap<u32, SpriteSheetInfo>,
    pub audio: HashMap<String, AudioInfo>,
}

/// Tileset info
#[derive(Debug, Clone, PartialEq)]
pub struct TilesetInfo {
    pub id: u32,
    pub name: String,
    pub tile_width: u32,
    pub tile_height: u32,
    pub columns: u32,
    pub rows: u32,
}

/// Sprite sheet info
#[derive(Debug, Clone, PartialEq)]
pub struct SpriteSheetInfo {
    pub id: u32,
    pub name: String,
    pub frame_width: u32,
    pub frame_height: u32,
    pub frames: u32,
}

/// Audio info
#[derive(Debug, Clone, PartialEq)]
pub struct AudioInfo {
    pub id: String,
    pub duration_ms: u32,
    pub is_streaming: bool,
}
