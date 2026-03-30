//! Particle System Editor
//!
//! Visual editor for creating and editing particle effects with:
//! - Particle emitter configuration (rate, lifetime, velocity, size, color)
//! - Preset library with common effects (Rain, Snow, Fire, Smoke, Magic, Heal, Buff)
//! - Live preview window
//! - Color gradient editor
//! - Texture picker
//! - Save/Load particle systems to files

use dde_core::particles::{
    BlendMode, EmissionShape, Particle, ParticleEmitter, ParticleSystem,
};
use glam::Vec2;
use std::path::PathBuf;

/// Particle effect preset types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParticlePreset {
    Rain,
    Snow,
    Fire,
    Smoke,
    Magic,
    Heal,
    Buff,
    Explosion,
    Sparkles,
    Custom,
}

impl ParticlePreset {
    fn name(&self) -> &'static str {
        match self {
            ParticlePreset::Rain => "🌧️ Rain",
            ParticlePreset::Snow => "❄️ Snow",
            ParticlePreset::Fire => "🔥 Fire",
            ParticlePreset::Smoke => "💨 Smoke",
            ParticlePreset::Magic => "✨ Magic",
            ParticlePreset::Heal => "💚 Heal",
            ParticlePreset::Buff => "⚡ Buff",
            ParticlePreset::Explosion => "💥 Explosion",
            ParticlePreset::Sparkles => "⭐ Sparkles",
            ParticlePreset::Custom => "🎨 Custom",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            ParticlePreset::Rain => "Falling rain drops with gravity",
            ParticlePreset::Snow => "Gentle falling snow",
            ParticlePreset::Fire => "Rising flames with flicker",
            ParticlePreset::Smoke => "Billowing smoke clouds",
            ParticlePreset::Magic => "Magical sparkles and glow",
            ParticlePreset::Heal => "Healing green particles",
            ParticlePreset::Buff => "Power-up energy effect",
            ParticlePreset::Explosion => "Burst explosion particles",
            ParticlePreset::Sparkles => "Shiny sparkle effect",
            ParticlePreset::Custom => "User-defined custom effect",
        }
    }

    fn create_emitter(&self, position: Vec2) -> ParticleEmitter {
        match self {
            ParticlePreset::Rain => Self::rain_preset(position),
            ParticlePreset::Snow => Self::snow_preset(position),
            ParticlePreset::Fire => Self::fire_preset(position),
            ParticlePreset::Smoke => Self::smoke_preset(position),
            ParticlePreset::Magic => Self::magic_preset(position),
            ParticlePreset::Heal => Self::heal_preset(position),
            ParticlePreset::Buff => Self::buff_preset(position),
            ParticlePreset::Explosion => Self::explosion_preset(position),
            ParticlePreset::Sparkles => Self::sparkles_preset(position),
            ParticlePreset::Custom => ParticleEmitter::default(),
        }
    }

    fn rain_preset(position: Vec2) -> ParticleEmitter {
        ParticleEmitter {
            emission_rate: 200.0,
            max_particles: 500,
            lifetime: 0.5..1.2,
            velocity: Vec2::new(-30.0, -300.0)..Vec2::new(30.0, -500.0),
            acceleration: Vec2::new(0.0, -200.0),
            start_scale: 0.3..0.6,
            end_scale: 0.2,
            start_color: [0.7, 0.8, 0.95, 1.0],
            end_color: [0.5, 0.6, 0.8, 1.0],
            start_alpha: 0.7,
            end_alpha: 0.2,
            position,
            shape: EmissionShape::Line {
                length: 400.0,
                angle: 0.0,
            },
            blend_mode: BlendMode::Additive,
            ..Default::default()
        }
    }

    fn snow_preset(position: Vec2) -> ParticleEmitter {
        ParticleEmitter {
            emission_rate: 50.0,
            max_particles: 300,
            lifetime: 3.0..6.0,
            velocity: Vec2::new(-20.0, -30.0)..Vec2::new(20.0, -60.0),
            acceleration: Vec2::new(0.0, -5.0),
            rotation_speed: -30.0..30.0,
            start_scale: 0.2..0.5,
            end_scale: 0.1,
            start_color: [1.0, 1.0, 1.0, 1.0],
            end_color: [0.9, 0.95, 1.0, 1.0],
            start_alpha: 0.9,
            end_alpha: 0.3,
            position,
            shape: EmissionShape::Line {
                length: 400.0,
                angle: 0.0,
            },
            blend_mode: BlendMode::Alpha,
            ..Default::default()
        }
    }

    fn fire_preset(position: Vec2) -> ParticleEmitter {
        ParticleEmitter {
            emission_rate: 80.0,
            max_particles: 200,
            lifetime: 0.5..1.5,
            velocity: Vec2::new(-20.0, 50.0)..Vec2::new(20.0, 150.0),
            acceleration: Vec2::new(0.0, 30.0),
            rotation_speed: -90.0..90.0,
            start_scale: 0.5..1.0,
            end_scale: 0.0,
            start_color: [1.0, 0.9, 0.3, 1.0],
            end_color: [1.0, 0.2, 0.0, 1.0],
            start_alpha: 0.9,
            end_alpha: 0.0,
            position,
            shape: EmissionShape::Circle { radius: 30.0 },
            blend_mode: BlendMode::Additive,
            ..Default::default()
        }
    }

    fn smoke_preset(position: Vec2) -> ParticleEmitter {
        ParticleEmitter {
            emission_rate: 30.0,
            max_particles: 150,
            lifetime: 2.0..4.0,
            velocity: Vec2::new(-10.0, 20.0)..Vec2::new(10.0, 60.0),
            acceleration: Vec2::new(0.0, 10.0),
            rotation_speed: -20.0..20.0,
            start_scale: 0.5..0.8,
            end_scale: 1.5,
            start_color: [0.8, 0.8, 0.8, 1.0],
            end_color: [0.4, 0.4, 0.4, 1.0],
            start_alpha: 0.6,
            end_alpha: 0.0,
            position,
            shape: EmissionShape::Circle { radius: 20.0 },
            blend_mode: BlendMode::Alpha,
            ..Default::default()
        }
    }

    fn magic_preset(position: Vec2) -> ParticleEmitter {
        ParticleEmitter {
            emission_rate: 40.0,
            max_particles: 100,
            lifetime: 1.0..2.5,
            velocity: Vec2::new(-30.0, -30.0)..Vec2::new(30.0, 30.0),
            acceleration: Vec2::new(0.0, -10.0),
            rotation_speed: -180.0..180.0,
            start_scale: 0.3..0.6,
            end_scale: 0.0,
            start_color: [0.8, 0.4, 1.0, 1.0],
            end_color: [0.2, 0.0, 0.8, 1.0],
            start_alpha: 1.0,
            end_alpha: 0.0,
            position,
            shape: EmissionShape::Circle { radius: 25.0 },
            blend_mode: BlendMode::Additive,
            ..Default::default()
        }
    }

    fn heal_preset(position: Vec2) -> ParticleEmitter {
        ParticleEmitter {
            emission_rate: 25.0,
            max_particles: 80,
            lifetime: 1.5..3.0,
            velocity: Vec2::new(-15.0, 30.0)..Vec2::new(15.0, 80.0),
            acceleration: Vec2::new(0.0, -15.0),
            rotation_speed: -45.0..45.0,
            start_scale: 0.2..0.4,
            end_scale: 0.0,
            start_color: [0.6, 1.0, 0.6, 1.0],
            end_color: [0.2, 0.8, 0.4, 1.0],
            start_alpha: 0.9,
            end_alpha: 0.0,
            position,
            shape: EmissionShape::Circle { radius: 20.0 },
            blend_mode: BlendMode::Additive,
            ..Default::default()
        }
    }

    fn buff_preset(position: Vec2) -> ParticleEmitter {
        ParticleEmitter {
            emission_rate: 35.0,
            max_particles: 120,
            lifetime: 1.0..2.0,
            velocity: Vec2::new(-25.0, 40.0)..Vec2::new(25.0, 100.0),
            acceleration: Vec2::new(0.0, -20.0),
            rotation_speed: -120.0..120.0,
            start_scale: 0.3..0.5,
            end_scale: 0.0,
            start_color: [1.0, 0.8, 0.2, 1.0],
            end_color: [1.0, 0.4, 0.0, 1.0],
            start_alpha: 1.0,
            end_alpha: 0.0,
            position,
            shape: EmissionShape::Circle { radius: 15.0 },
            blend_mode: BlendMode::Additive,
            ..Default::default()
        }
    }

    fn explosion_preset(position: Vec2) -> ParticleEmitter {
        ParticleEmitter {
            emission_rate: 0.0, // Burst
            max_particles: 100,
            lifetime: 0.3..0.8,
            velocity: Vec2::new(-200.0, -200.0)..Vec2::new(200.0, 200.0),
            acceleration: Vec2::new(0.0, -50.0),
            rotation_speed: -360.0..360.0,
            start_scale: 0.8..1.2,
            end_scale: 0.0,
            start_color: [1.0, 0.9, 0.3, 1.0],
            end_color: [1.0, 0.1, 0.0, 1.0],
            start_alpha: 1.0,
            end_alpha: 0.0,
            position,
            shape: EmissionShape::Circle { radius: 10.0 },
            blend_mode: BlendMode::Additive,
            active: false,
            ..Default::default()
        }
    }

    fn sparkles_preset(position: Vec2) -> ParticleEmitter {
        ParticleEmitter {
            emission_rate: 60.0,
            max_particles: 150,
            lifetime: 0.5..1.5,
            velocity: Vec2::new(-40.0, -40.0)..Vec2::new(40.0, 40.0),
            acceleration: Vec2::new(0.0, 0.0),
            rotation_speed: -270.0..270.0,
            start_scale: 0.2..0.4,
            end_scale: 0.0,
            start_color: [1.0, 1.0, 0.8, 1.0],
            end_color: [1.0, 0.8, 0.4, 1.0],
            start_alpha: 1.0,
            end_alpha: 0.0,
            position,
            shape: EmissionShape::Circle { radius: 40.0 },
            blend_mode: BlendMode::Additive,
            ..Default::default()
        }
    }

    fn all() -> [ParticlePreset; 10] {
        [
            ParticlePreset::Rain,
            ParticlePreset::Snow,
            ParticlePreset::Fire,
            ParticlePreset::Smoke,
            ParticlePreset::Magic,
            ParticlePreset::Heal,
            ParticlePreset::Buff,
            ParticlePreset::Explosion,
            ParticlePreset::Sparkles,
            ParticlePreset::Custom,
        ]
    }
}

/// Serializable particle system data for save/load
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ParticleSystemData {
    pub name: String,
    pub description: String,
    pub emitter: ParticleEmitterData,
    pub version: String,
}

impl Default for ParticleSystemData {
    fn default() -> Self {
        Self {
            name: "New Particle Effect".to_string(),
            description: String::new(),
            emitter: ParticleEmitterData::default(),
            version: "1.0".to_string(),
        }
    }
}

/// Serializable emitter data
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ParticleEmitterData {
    pub sprite_id: u32,
    pub emission_rate: f32,
    pub max_particles: usize,
    pub lifetime_min: f32,
    pub lifetime_max: f32,
    pub velocity_min_x: f32,
    pub velocity_min_y: f32,
    pub velocity_max_x: f32,
    pub velocity_max_y: f32,
    pub acceleration_x: f32,
    pub acceleration_y: f32,
    pub rotation_speed_min: f32,
    pub rotation_speed_max: f32,
    pub start_scale_min: f32,
    pub start_scale_max: f32,
    pub end_scale: f32,
    pub start_color: [f32; 4],
    pub end_color: [f32; 4],
    pub start_alpha: f32,
    pub end_alpha: f32,
    pub shape: EmissionShapeData,
    pub blend_mode: BlendModeData,
}

impl Default for ParticleEmitterData {
    fn default() -> Self {
        Self {
            sprite_id: 0,
            emission_rate: 10.0,
            max_particles: 100,
            lifetime_min: 1.0,
            lifetime_max: 2.0,
            velocity_min_x: -10.0,
            velocity_min_y: -50.0,
            velocity_max_x: 10.0,
            velocity_max_y: -100.0,
            acceleration_x: 0.0,
            acceleration_y: -98.0,
            rotation_speed_min: -45.0,
            rotation_speed_max: 45.0,
            start_scale_min: 0.5,
            start_scale_max: 1.0,
            end_scale: 0.0,
            start_color: [1.0, 1.0, 1.0, 1.0],
            end_color: [1.0, 1.0, 1.0, 1.0],
            start_alpha: 1.0,
            end_alpha: 0.0,
            shape: EmissionShapeData::Point,
            blend_mode: BlendModeData::Alpha,
        }
    }
}

impl From<&ParticleEmitter> for ParticleEmitterData {
    fn from(emitter: &ParticleEmitter) -> Self {
        Self {
            sprite_id: emitter.sprite_id,
            emission_rate: emitter.emission_rate,
            max_particles: emitter.max_particles,
            lifetime_min: emitter.lifetime.start,
            lifetime_max: emitter.lifetime.end,
            velocity_min_x: emitter.velocity.start.x,
            velocity_min_y: emitter.velocity.start.y,
            velocity_max_x: emitter.velocity.end.x,
            velocity_max_y: emitter.velocity.end.y,
            acceleration_x: emitter.acceleration.x,
            acceleration_y: emitter.acceleration.y,
            rotation_speed_min: emitter.rotation_speed.start,
            rotation_speed_max: emitter.rotation_speed.end,
            start_scale_min: emitter.start_scale.start,
            start_scale_max: emitter.start_scale.end,
            end_scale: emitter.end_scale,
            start_color: emitter.start_color,
            end_color: emitter.end_color,
            start_alpha: emitter.start_alpha,
            end_alpha: emitter.end_alpha,
            shape: EmissionShapeData::from(emitter.shape),
            blend_mode: BlendModeData::from(emitter.blend_mode),
        }
    }
}

impl From<&ParticleEmitterData> for ParticleEmitter {
    fn from(data: &ParticleEmitterData) -> Self {
        Self {
            sprite_id: data.sprite_id,
            emission_rate: data.emission_rate,
            max_particles: data.max_particles,
            lifetime: data.lifetime_min..data.lifetime_max,
            velocity: Vec2::new(data.velocity_min_x, data.velocity_min_y)
                ..Vec2::new(data.velocity_max_x, data.velocity_max_y),
            acceleration: Vec2::new(data.acceleration_x, data.acceleration_y),
            rotation_speed: data.rotation_speed_min..data.rotation_speed_max,
            start_scale: data.start_scale_min..data.start_scale_max,
            end_scale: data.end_scale,
            start_color: data.start_color,
            end_color: data.end_color,
            start_alpha: data.start_alpha,
            end_alpha: data.end_alpha,
            active: true,
            position: Vec2::ZERO,
            shape: EmissionShape::from(&data.shape),
            blend_mode: BlendMode::from(&data.blend_mode),
        }
    }
}

/// Serializable emission shape
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum EmissionShapeData {
    Point,
    Circle { radius: f32 },
    Rectangle { width: f32, height: f32 },
    Line { length: f32, angle: f32 },
}

impl From<EmissionShape> for EmissionShapeData {
    fn from(shape: EmissionShape) -> Self {
        match shape {
            EmissionShape::Point => EmissionShapeData::Point,
            EmissionShape::Circle { radius } => EmissionShapeData::Circle { radius },
            EmissionShape::Rectangle { width, height } => {
                EmissionShapeData::Rectangle { width, height }
            }
            EmissionShape::Line { length, angle } => EmissionShapeData::Line { length, angle },
        }
    }
}

impl From<&EmissionShapeData> for EmissionShape {
    fn from(data: &EmissionShapeData) -> Self {
        match *data {
            EmissionShapeData::Point => EmissionShape::Point,
            EmissionShapeData::Circle { radius } => EmissionShape::Circle { radius },
            EmissionShapeData::Rectangle { width, height } => {
                EmissionShape::Rectangle { width, height }
            }
            EmissionShapeData::Line { length, angle } => EmissionShape::Line { length, angle },
        }
    }
}

/// Serializable blend mode
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum BlendModeData {
    Alpha,
    Additive,
    Multiply,
}

impl From<BlendMode> for BlendModeData {
    fn from(mode: BlendMode) -> Self {
        match mode {
            BlendMode::Alpha => BlendModeData::Alpha,
            BlendMode::Additive => BlendModeData::Additive,
            BlendMode::Multiply => BlendModeData::Multiply,
        }
    }
}

impl From<&BlendModeData> for BlendMode {
    fn from(data: &BlendModeData) -> Self {
        match *data {
            BlendModeData::Alpha => BlendMode::Alpha,
            BlendModeData::Additive => BlendMode::Additive,
            BlendModeData::Multiply => BlendMode::Multiply,
        }
    }
}

/// Particle editor state
pub struct ParticleEditor {
    /// Whether the editor is visible
    visible: bool,
    /// Currently selected preset
    selected_preset: ParticlePreset,
    /// Current emitter configuration
    emitter: ParticleEmitter,
    /// Particle system for preview
    particle_system: ParticleSystem,
    /// Editor state
    state: EditorState,
    /// Saved presets library
    saved_presets: Vec<ParticleSystemData>,
    /// Currently selected saved preset
    selected_saved_preset: Option<usize>,
    /// File path for save/load
    save_directory: PathBuf,
    /// Preview state
    preview: PreviewState,
    /// Last update time for animation
    last_update: std::time::Instant,
    /// Available texture options
    texture_options: Vec<TextureOption>,
    /// Selected texture
    selected_texture: usize,
}

/// Editor state
#[derive(Debug, Clone, Default)]
struct EditorState {
    /// Current tab
    selected_tab: EditorTab,
    /// Whether changes are unsaved
    dirty: bool,
    /// Show advanced options
    show_advanced: bool,
    /// Last saved timestamp
    last_saved: Option<std::time::SystemTime>,
    /// Status message
    status_message: Option<(String, f32)>,
}

/// Editor tabs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum EditorTab {
    #[default]
    Emitter,
    Color,
    Texture,
}

/// Preview state
#[derive(Debug, Clone)]
struct PreviewState {
    /// Whether preview is playing
    playing: bool,
    /// Preview scale
    scale: f32,
    /// Show particle count
    show_stats: bool,
    /// Background color
    background_color: [f32; 3],
    /// Simulation speed
    simulation_speed: f32,
}

impl Default for PreviewState {
    fn default() -> Self {
        Self {
            playing: true,
            scale: 1.0,
            show_stats: true,
            background_color: [0.1, 0.1, 0.12],
            simulation_speed: 1.0,
        }
    }
}

/// Texture option
#[derive(Debug, Clone)]
struct TextureOption {
    id: u32,
    name: String,
    path: String,
    category: TextureCategory,
}

/// Texture category
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TextureCategory {
    Basic,
    Nature,
    Magic,
    UI,
}

impl TextureCategory {
    fn name(&self) -> &'static str {
        match self {
            TextureCategory::Basic => "Basic",
            TextureCategory::Nature => "Nature",
            TextureCategory::Magic => "Magic",
            TextureCategory::UI => "UI",
        }
    }
}

impl ParticleEditor {
    /// Create a new particle editor
    pub fn new() -> Self {
        let texture_options = Self::create_default_textures();
        let mut editor = Self {
            visible: false,
            selected_preset: ParticlePreset::Fire,
            emitter: ParticlePreset::Fire.create_emitter(Vec2::ZERO),
            particle_system: ParticleSystem::new(5000),
            state: EditorState::default(),
            saved_presets: Vec::new(),
            selected_saved_preset: None,
            save_directory: PathBuf::from("assets/particles"),
            preview: PreviewState::default(),
            last_update: std::time::Instant::now(),
            texture_options,
            selected_texture: 0,
        };

        editor.load_default_presets();
        editor
    }

    /// Create default texture options
    fn create_default_textures() -> Vec<TextureOption> {
        vec![
            TextureOption {
                id: 0,
                name: "Default Circle".to_string(),
                path: "particles/circle.png".to_string(),
                category: TextureCategory::Basic,
            },
            TextureOption {
                id: 1,
                name: "Soft Glow".to_string(),
                path: "particles/glow.png".to_string(),
                category: TextureCategory::Basic,
            },
            TextureOption {
                id: 2,
                name: "Sparkle".to_string(),
                path: "particles/sparkle.png".to_string(),
                category: TextureCategory::Magic,
            },
            TextureOption {
                id: 3,
                name: "Star".to_string(),
                path: "particles/star.png".to_string(),
                category: TextureCategory::Magic,
            },
            TextureOption {
                id: 4,
                name: "Rain Drop".to_string(),
                path: "particles/rain.png".to_string(),
                category: TextureCategory::Nature,
            },
            TextureOption {
                id: 5,
                name: "Snowflake".to_string(),
                path: "particles/snow.png".to_string(),
                category: TextureCategory::Nature,
            },
            TextureOption {
                id: 6,
                name: "Smoke".to_string(),
                path: "particles/smoke.png".to_string(),
                category: TextureCategory::Nature,
            },
            TextureOption {
                id: 7,
                name: "Fire".to_string(),
                path: "particles/fire.png".to_string(),
                category: TextureCategory::Nature,
            },
            TextureOption {
                id: 8,
                name: "Heart".to_string(),
                path: "particles/heart.png".to_string(),
                category: TextureCategory::UI,
            },
            TextureOption {
                id: 9,
                name: "Plus".to_string(),
                path: "particles/plus.png".to_string(),
                category: TextureCategory::UI,
            },
        ]
    }

    /// Load default presets
    fn load_default_presets(&mut self) {
        for preset in ParticlePreset::all() {
            let emitter = preset.create_emitter(Vec2::ZERO);
            let data = ParticleSystemData {
                name: preset.name().to_string(),
                description: preset.description().to_string(),
                emitter: ParticleEmitterData::from(&emitter),
                version: "1.0".to_string(),
            };
            self.saved_presets.push(data);
        }
    }

    /// Show the editor
    pub fn show(&mut self) {
        self.visible = true;
    }

    /// Hide the editor
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

    /// Get current emitter
    pub fn get_emitter(&self) -> &ParticleEmitter {
        &self.emitter
    }

    /// Get mutable emitter
    pub fn get_emitter_mut(&mut self) -> &mut ParticleEmitter {
        &mut self.emitter
    }

    /// Apply a preset
    pub fn apply_preset(&mut self, preset: ParticlePreset) {
        self.selected_preset = preset;
        self.emitter = preset.create_emitter(Vec2::ZERO);
        self.particle_system.clear();
        self.state.dirty = true;
    }

    /// Update the particle system (call each frame)
    pub fn update(&mut self, dt: f32) {
        if !self.preview.playing {
            return;
        }

        let scaled_dt = dt * self.preview.simulation_speed;

        // Spawn new particles based on emission rate
        if self.emitter.active && self.emitter.emission_rate > 0.0 {
            let particles_to_spawn = (self.emitter.emission_rate * scaled_dt) as usize;
            if particles_to_spawn > 0 {
                let mut rng = rand::thread_rng();
                self.particle_system.spawn(&self.emitter, particles_to_spawn, &mut rng);
            }
        }

        // Update existing particles
        self.particle_system.update(scaled_dt, self.emitter.acceleration);
    }

    /// Trigger a burst emission (for explosion-type effects)
    pub fn trigger_burst(&mut self, count: usize) {
        let mut rng = rand::thread_rng();
        self.particle_system.spawn(&self.emitter, count, &mut rng);
    }

    /// Save current particle system to file
    pub fn save_to_file(&mut self, name: &str) -> Result<(), String> {
        let data = ParticleSystemData {
            name: name.to_string(),
            description: self.saved_presets
                .get(self.selected_saved_preset.unwrap_or(0))
                .map(|p| p.description.clone())
                .unwrap_or_default(),
            emitter: ParticleEmitterData::from(&self.emitter),
            version: "1.0".to_string(),
        };

        std::fs::create_dir_all(&self.save_directory).map_err(|e| e.to_string())?;

        let filename = format!("{}.json", sanitize_filename(name));
        let path = self.save_directory.join(filename);

        let json = serde_json::to_string_pretty(&data).map_err(|e| e.to_string())?;
        std::fs::write(&path, json).map_err(|e| e.to_string())?;

        self.state.last_saved = Some(std::time::SystemTime::now());
        self.state.dirty = false;
        self.set_status_message(&format!("Saved to {}", path.display()));

        Ok(())
    }

    /// Load particle system from file
    pub fn load_from_file(&mut self, path: &PathBuf) -> Result<(), String> {
        let json = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        let data: ParticleSystemData = serde_json::from_str(&json).map_err(|e| e.to_string())?;

        self.emitter = ParticleEmitter::from(&data.emitter);
        self.particle_system.clear();
        self.state.dirty = false;
        self.set_status_message(&format!("Loaded {}", data.name));

        Ok(())
    }

    /// Load all presets from save directory
    pub fn load_saved_presets(&mut self) {
        self.saved_presets.clear();
        self.load_default_presets();

        if let Ok(entries) = std::fs::read_dir(&self.save_directory) {
            for entry in entries.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    if metadata.is_file() {
                        let path = entry.path();
                        if path.extension().map(|e| e == "json").unwrap_or(false) {
                            if let Ok(json) = std::fs::read_to_string(&path) {
                                if let Ok(data) = serde_json::from_str::<ParticleSystemData>(&json) {
                                    self.saved_presets.push(data);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Set status message with timeout
    fn set_status_message(&mut self, msg: &str) {
        self.state.status_message = Some((msg.to_string(), 3.0));
    }

    /// Draw the particle editor UI
    pub fn draw(&mut self, ctx: &egui::Context) {
        if !self.visible {
            return;
        }

        // Update animation
        let now = std::time::Instant::now();
        let dt = now.duration_since(self.last_update).as_secs_f32();
        self.last_update = now;
        self.update(dt);

        // Update status message timer
        if let Some((_, ref mut time)) = self.state.status_message {
            *time -= ctx.input(|i| i.stable_dt);
            if *time <= 0.0 {
                self.state.status_message = None;
            }
        }

        let mut visible = self.visible;
        egui::Window::new("✨ Particle Editor")
            .open(&mut visible)
            .resizable(true)
            .default_size([1200.0, 800.0])
            .show(ctx, |ui| {
                self.draw_content(ui);
            });
        self.visible = visible;

        // Request continuous repaint for animation
        if self.preview.playing {
            ctx.request_repaint();
        }
    }

    /// Draw editor content
    fn draw_content(&mut self, ui: &mut egui::Ui) {
        // Menu bar
        self.draw_menu_bar(ui);

        ui.separator();

        // Status message
        if let Some((ref msg, _)) = self.state.status_message {
            ui.colored_label(egui::Color32::GREEN, msg);
            ui.separator();
        }

        // Dirty indicator
        if self.state.dirty {
            ui.horizontal(|ui| {
                ui.colored_label(egui::Color32::YELLOW, "● Unsaved Changes");
                if ui.button("Save").clicked() {
                    if let Err(e) = self.save_to_file(&format!("custom_{}", uuid::Uuid::new_v4())) {
                        self.set_status_message(&format!("Error: {}", e));
                    }
                }
            });
            ui.separator();
        }

        // Main layout: Left (Presets) | Center (Preview) | Right (Properties)
        egui::SidePanel::left("particle_presets")
            .default_width(250.0)
            .resizable(true)
            .show_inside(ui, |ui| {
                self.draw_preset_panel(ui);
            });

        egui::SidePanel::right("particle_properties")
            .default_width(300.0)
            .resizable(true)
            .show_inside(ui, |ui| {
                self.draw_properties_panel(ui);
            });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            self.draw_preview_panel(ui);
        });
    }

    /// Draw menu bar
    fn draw_menu_bar(&mut self, ui: &mut egui::Ui) {
        egui::menu::bar(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("New Effect").clicked() {
                    self.apply_preset(ParticlePreset::Custom);
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Save...").clicked() {
                    if let Err(e) = self.save_to_file("custom_effect") {
                        self.set_status_message(&format!("Error: {}", e));
                    }
                    ui.close_menu();
                }
                if ui.button("Load...").clicked() {
                    self.load_saved_presets();
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Reset to Defaults").clicked() {
                    self.load_default_presets();
                    self.apply_preset(ParticlePreset::Fire);
                    ui.close_menu();
                }
            });

            ui.menu_button("Presets", |ui| {
                for preset in ParticlePreset::all() {
                    if ui.button(preset.name()).clicked() {
                        self.apply_preset(preset);
                        ui.close_menu();
                    }
                }
            });

            ui.menu_button("View", |ui| {
                ui.checkbox(&mut self.preview.show_stats, "Show Statistics");
                ui.checkbox(&mut self.state.show_advanced, "Show Advanced Options");
            });

            ui.menu_button("Actions", |ui| {
                if ui.button("Trigger Burst").clicked() {
                    self.trigger_burst(50);
                    ui.close_menu();
                }
                if ui.button("Clear Particles").clicked() {
                    self.particle_system.clear();
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Reset Emitter").clicked() {
                    self.emitter = self.selected_preset.create_emitter(Vec2::ZERO);
                    self.state.dirty = true;
                    ui.close_menu();
                }
            });
        });
    }

    /// Draw preset panel (left sidebar)
    fn draw_preset_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Presets");
        ui.separator();

        // Built-in presets
        ui.label("Built-in:");
        egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
            for preset in ParticlePreset::all() {
                let is_selected = self.selected_preset == preset;
                let response = ui.selectable_label(is_selected, preset.name());
                if response.clicked() && !is_selected {
                    self.apply_preset(preset);
                }
                response.on_hover_text(preset.description());
            }
        });

        ui.separator();

        // Saved presets
        ui.label("Saved:");
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (idx, preset) in self.saved_presets.iter().enumerate() {
                let is_selected = self.selected_saved_preset == Some(idx);
                if ui.selectable_label(is_selected, &preset.name).clicked() {
                    self.selected_saved_preset = Some(idx);
                    self.emitter = ParticleEmitter::from(&preset.emitter);
                    self.particle_system.clear();
                    self.state.dirty = false;
                }
            }
        });

        ui.separator();

        // Quick actions
        if ui.button("🔄 Refresh List").clicked() {
            self.load_saved_presets();
        }
    }

    /// Draw properties panel (right sidebar)
    fn draw_properties_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Properties");
        ui.separator();

        // Tab selection
        ui.horizontal(|ui| {
            for (tab, name) in [
                (EditorTab::Emitter, "🔧 Emitter"),
                (EditorTab::Color, "🎨 Color"),
                (EditorTab::Texture, "🖼️ Texture"),
            ] {
                let selected = self.state.selected_tab == tab;
                if ui.selectable_label(selected, name).clicked() {
                    self.state.selected_tab = tab;
                }
            }
        });

        ui.separator();

        match self.state.selected_tab {
            EditorTab::Emitter => self.draw_emitter_properties(ui),
            EditorTab::Color => self.draw_color_properties(ui),
            EditorTab::Texture => self.draw_texture_properties(ui),
        }
    }

    /// Draw emitter properties
    fn draw_emitter_properties(&mut self, ui: &mut egui::Ui) {
        ui.label("Emission");
        ui.separator();

        // Active toggle
        ui.checkbox(&mut self.emitter.active, "Active");

        // Emission rate
        ui.horizontal(|ui| {
            ui.label("Rate:");
            ui.add(
                egui::DragValue::new(&mut self.emitter.emission_rate)
                    .speed(1.0)
                    .suffix(" /sec"),
            );
        });

        // Max particles
        ui.horizontal(|ui| {
            ui.label("Max:");
            ui.add(
                egui::DragValue::new(&mut self.emitter.max_particles)
                    .speed(10)
                    .clamp_range(1..=10000),
            );
        });

        ui.separator();
        ui.label("Lifetime");

        // Lifetime range
        ui.horizontal(|ui| {
            ui.label("Min:");
            ui.add(
                egui::DragValue::new(&mut self.emitter.lifetime.start)
                    .speed(0.1)
                    .clamp_range(0.01..=60.0),
            );
            ui.label("Max:");
            ui.add(
                egui::DragValue::new(&mut self.emitter.lifetime.end)
                    .speed(0.1)
                    .clamp_range(self.emitter.lifetime.start..=60.0),
            );
        });

        ui.separator();
        ui.label("Velocity");

        // Velocity min
        ui.horizontal(|ui| {
            ui.label("Min X:");
            ui.add(egui::DragValue::new(&mut self.emitter.velocity.start.x).speed(1.0));
            ui.label("Y:");
            ui.add(egui::DragValue::new(&mut self.emitter.velocity.start.y).speed(1.0));
        });

        // Velocity max
        ui.horizontal(|ui| {
            ui.label("Max X:");
            ui.add(egui::DragValue::new(&mut self.emitter.velocity.end.x).speed(1.0));
            ui.label("Y:");
            ui.add(egui::DragValue::new(&mut self.emitter.velocity.end.y).speed(1.0));
        });

        ui.separator();
        ui.label("Acceleration");

        // Acceleration
        ui.horizontal(|ui| {
            ui.label("X:");
            ui.add(egui::DragValue::new(&mut self.emitter.acceleration.x).speed(1.0));
            ui.label("Y:");
            ui.add(egui::DragValue::new(&mut self.emitter.acceleration.y).speed(1.0));
        });

        ui.separator();
        ui.label("Scale");

        // Scale range
        ui.horizontal(|ui| {
            ui.label("Start Min:");
            ui.add(
                egui::DragValue::new(&mut self.emitter.start_scale.start)
                    .speed(0.05)
                    .clamp_range(0.0..=10.0),
            );
        });
        ui.horizontal(|ui| {
            ui.label("Start Max:");
            ui.add(
                egui::DragValue::new(&mut self.emitter.start_scale.end)
                    .speed(0.05)
                    .clamp_range(0.0..=10.0),
            );
        });
        ui.horizontal(|ui| {
            ui.label("End Scale:");
            ui.add(
                egui::DragValue::new(&mut self.emitter.end_scale)
                    .speed(0.05)
                    .clamp_range(0.0..=10.0),
            );
        });

        if self.state.show_advanced {
            ui.separator();
            ui.label("Advanced");

            // Rotation speed
            ui.horizontal(|ui| {
                ui.label("Rot Min:");
                ui.add(egui::DragValue::new(&mut self.emitter.rotation_speed.start).speed(1.0));
            });
            ui.horizontal(|ui| {
                ui.label("Rot Max:");
                ui.add(egui::DragValue::new(&mut self.emitter.rotation_speed.end).speed(1.0));
            });

            // Emission shape
            ui.label("Shape:");
            ui.horizontal(|ui| {
                if ui.button("Point").clicked() {
                    self.emitter.shape = EmissionShape::Point;
                }
                if ui.button("Circle").clicked() {
                    self.emitter.shape = EmissionShape::Circle { radius: 30.0 };
                }
                if ui.button("Line").clicked() {
                    self.emitter.shape = EmissionShape::Line {
                        length: 200.0,
                        angle: 0.0,
                    };
                }
            });

            // Shape-specific parameters
            match &mut self.emitter.shape {
                EmissionShape::Circle { radius } => {
                    ui.horizontal(|ui| {
                        ui.label("Radius:");
                        ui.add(egui::DragValue::new(radius).speed(1.0).clamp_range(0.0..=500.0));
                    });
                }
                EmissionShape::Line { length, angle } => {
                    ui.horizontal(|ui| {
                        ui.label("Length:");
                        ui.add(egui::DragValue::new(length).speed(1.0).clamp_range(0.0..=1000.0));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Angle:");
                        ui.add(
                            egui::DragValue::new(angle)
                                .speed(0.1)
                                .clamp_range(0.0..=std::f32::consts::TAU),
                        );
                    });
                }
                _ => {}
            }

            // Blend mode
            ui.label("Blend Mode:");
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.emitter.blend_mode, BlendMode::Alpha, "Alpha");
                ui.selectable_value(&mut self.emitter.blend_mode, BlendMode::Additive, "Additive");
                ui.selectable_value(&mut self.emitter.blend_mode, BlendMode::Multiply, "Multiply");
            });
        }

        self.state.dirty = true;
    }

    /// Draw color properties
    fn draw_color_properties(&mut self, ui: &mut egui::Ui) {
        ui.label("Start Color");
        ui.separator();

        // Start color picker
        let mut start_color = self.emitter.start_color;
        if ui.color_edit_button_rgba_unmultiplied(&mut start_color).changed() {
            self.emitter.start_color = start_color;
            self.state.dirty = true;
        }

        // RGBA sliders for start color
        ui.horizontal(|ui| {
            ui.label("R:");
            ui.add(egui::Slider::new(&mut self.emitter.start_color[0], 0.0..=1.0));
        });
        ui.horizontal(|ui| {
            ui.label("G:");
            ui.add(egui::Slider::new(&mut self.emitter.start_color[1], 0.0..=1.0));
        });
        ui.horizontal(|ui| {
            ui.label("B:");
            ui.add(egui::Slider::new(&mut self.emitter.start_color[2], 0.0..=1.0));
        });

        ui.separator();
        ui.label("Start Alpha:");
        ui.add(egui::Slider::new(&mut self.emitter.start_alpha, 0.0..=1.0));

        ui.separator();
        ui.label("End Color");
        ui.separator();

        // End color picker
        let mut end_color = self.emitter.end_color;
        if ui.color_edit_button_rgba_unmultiplied(&mut end_color).changed() {
            self.emitter.end_color = end_color;
            self.state.dirty = true;
        }

        // RGBA sliders for end color
        ui.horizontal(|ui| {
            ui.label("R:");
            ui.add(egui::Slider::new(&mut self.emitter.end_color[0], 0.0..=1.0));
        });
        ui.horizontal(|ui| {
            ui.label("G:");
            ui.add(egui::Slider::new(&mut self.emitter.end_color[1], 0.0..=1.0));
        });
        ui.horizontal(|ui| {
            ui.label("B:");
            ui.add(egui::Slider::new(&mut self.emitter.end_color[2], 0.0..=1.0));
        });

        ui.separator();
        ui.label("End Alpha:");
        ui.add(egui::Slider::new(&mut self.emitter.end_alpha, 0.0..=1.0));

        // Gradient preview
        ui.separator();
        ui.label("Gradient Preview");

        let gradient_rect = ui.available_rect_before_wrap();
        let height = 30.0;
        let rect = egui::Rect::from_min_size(
            gradient_rect.min,
            egui::vec2(gradient_rect.width(), height),
        );

        let painter = ui.painter_at(rect);
        let steps = 20;
        let step_width = rect.width() / steps as f32;

        for i in 0..steps {
            let t = i as f32 / (steps - 1) as f32;
            let color = egui::Color32::from_rgba_premultiplied(
                ((self.emitter.start_color[0] + (self.emitter.end_color[0] - self.emitter.start_color[0]) * t) * 255.0) as u8,
                ((self.emitter.start_color[1] + (self.emitter.end_color[1] - self.emitter.start_color[1]) * t) * 255.0) as u8,
                ((self.emitter.start_color[2] + (self.emitter.end_color[2] - self.emitter.start_color[2]) * t) * 255.0) as u8,
                ((self.emitter.start_alpha + (self.emitter.end_alpha - self.emitter.start_alpha) * t) * 255.0) as u8,
            );

            let bar_rect = egui::Rect::from_min_size(
                egui::pos2(rect.min.x + i as f32 * step_width, rect.min.y),
                egui::vec2(step_width, height),
            );
            painter.rect_filled(bar_rect, 0.0, color);
        }

        ui.allocate_space(egui::vec2(0.0, height + 10.0));

        self.state.dirty = true;
    }

    /// Draw texture properties
    fn draw_texture_properties(&mut self, ui: &mut egui::Ui) {
        ui.label("Texture Selection");
        ui.separator();

        // Category filter
        let mut selected_category: Option<TextureCategory> = None;
        ui.horizontal(|ui| {
            for category in [TextureCategory::Basic, TextureCategory::Nature, TextureCategory::Magic, TextureCategory::UI] {
                if ui.button(category.name()).clicked() {
                    selected_category = Some(category);
                }
            }
        });

        ui.separator();

        // Texture list
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (idx, texture) in self.texture_options.iter().enumerate() {
                if let Some(category) = selected_category {
                    if texture.category != category {
                        continue;
                    }
                }

                let is_selected = self.selected_texture == idx;
                let response = ui.selectable_label(
                    is_selected,
                    format!("{} {}", self.get_texture_icon(&texture.category), texture.name),
                );

                if response.clicked() {
                    self.selected_texture = idx;
                    self.emitter.sprite_id = texture.id;
                    self.state.dirty = true;
                }

                response.on_hover_text(&texture.path);
            }
        });

        ui.separator();

        // Current texture info
        if let Some(texture) = self.texture_options.get(self.selected_texture) {
            ui.label(format!("Selected: {}", texture.name));
            ui.label(format!("ID: {}", texture.id));
            ui.label(format!("Path: {}", texture.path));
        }
    }

    /// Get icon for texture category
    fn get_texture_icon(&self, category: &TextureCategory) -> &'static str {
        match category {
            TextureCategory::Basic => "🔵",
            TextureCategory::Nature => "🌿",
            TextureCategory::Magic => "✨",
            TextureCategory::UI => "🎯",
        }
    }

    /// Draw preview panel (center)
    fn draw_preview_panel(&mut self, ui: &mut egui::Ui) {
        // Preview controls
        ui.horizontal(|ui| {
            let play_text = if self.preview.playing { "⏸ Pause" } else { "▶ Play" };
            if ui.button(play_text).clicked() {
                self.preview.playing = !self.preview.playing;
            }
            if ui.button("⏹ Stop").clicked() {
                self.particle_system.clear();
                self.preview.playing = false;
            }
            if ui.button("🔄 Burst").clicked() {
                self.trigger_burst(50);
            }

            ui.separator();

            ui.label("Speed:");
            ui.add(
                egui::Slider::new(&mut self.preview.simulation_speed, 0.1..=3.0)
                    .show_value(true),
            );

            ui.separator();

            ui.checkbox(&mut self.preview.show_stats, "Stats");
        });

        ui.separator();

        // Preview area
        let available = ui.available_rect_before_wrap();
        let preview_rect = egui::Rect::from_min_size(
            available.min,
            egui::vec2(available.width(), available.height() - 60.0),
        );

        let painter = ui.painter_at(preview_rect);

        // Background
        let bg_color = egui::Color32::from_rgb(
            (self.preview.background_color[0] * 255.0) as u8,
            (self.preview.background_color[1] * 255.0) as u8,
            (self.preview.background_color[2] * 255.0) as u8,
        );
        painter.rect_filled(preview_rect, 4.0, bg_color);

        // Draw grid
        self.draw_grid(&painter, preview_rect);

        // Draw particles
        self.draw_particles(&painter, preview_rect);

        // Draw emitter position
        let center = preview_rect.center();
        painter.circle_stroke(center, 5.0, egui::Stroke::new(2.0, egui::Color32::YELLOW));

        // Statistics overlay
        if self.preview.show_stats {
            let stats_rect = egui::Rect::from_min_size(
                egui::pos2(preview_rect.min.x + 10.0, preview_rect.min.y + 10.0),
                egui::vec2(150.0, 80.0),
            );

            painter.rect_filled(
                stats_rect,
                4.0,
                egui::Color32::from_rgba_premultiplied(0, 0, 0, 180),
            );

            let particle_count = self.particle_system.count();
            let max_count = self.particle_system.particles().len();
            let emission_rate = self.emitter.emission_rate;

            painter.text(
                stats_rect.min + egui::vec2(10.0, 20.0),
                egui::Align2::LEFT_TOP,
                format!("Particles: {}/{}", particle_count, max_count),
                egui::FontId::monospace(12.0),
                egui::Color32::WHITE,
            );

            painter.text(
                stats_rect.min + egui::vec2(10.0, 40.0),
                egui::Align2::LEFT_TOP,
                format!("Emission: {:.1}/s", emission_rate),
                egui::FontId::monospace(12.0),
                egui::Color32::WHITE,
            );

            painter.text(
                stats_rect.min + egui::vec2(10.0, 60.0),
                egui::Align2::LEFT_TOP,
                format!("Preset: {}", self.selected_preset.name()),
                egui::FontId::monospace(12.0),
                egui::Color32::WHITE,
            );
        }

        // Background color selector
        ui.horizontal(|ui| {
            ui.label("Background:");
            ui.color_edit_button_rgb(&mut self.preview.background_color);
        });
    }

    /// Draw grid in preview
    fn draw_grid(&self, painter: &egui::Painter, rect: egui::Rect) {
        let grid_size = 50.0 * self.preview.scale;
        let grid_color = egui::Color32::from_rgba_premultiplied(255, 255, 255, 20);

        let offset_x = rect.min.x % grid_size;
        let offset_y = rect.min.y % grid_size;

        // Vertical lines
        let mut x = rect.min.x + offset_x;
        while x < rect.max.x {
            painter.line_segment(
                [egui::pos2(x, rect.min.y), egui::pos2(x, rect.max.y)],
                egui::Stroke::new(1.0, grid_color),
            );
            x += grid_size;
        }

        // Horizontal lines
        let mut y = rect.min.y + offset_y;
        while y < rect.max.y {
            painter.line_segment(
                [egui::pos2(rect.min.x, y), egui::pos2(rect.max.x, y)],
                egui::Stroke::new(1.0, grid_color),
            );
            y += grid_size;
        }

        // Center crosshair
        let center = rect.center();
        let crosshair_color = egui::Color32::from_rgba_premultiplied(255, 255, 255, 60);
        painter.line_segment(
            [egui::pos2(center.x - 20.0, center.y), egui::pos2(center.x + 20.0, center.y)],
            egui::Stroke::new(1.0, crosshair_color),
        );
        painter.line_segment(
            [egui::pos2(center.x, center.y - 20.0), egui::pos2(center.x, center.y + 20.0)],
            egui::Stroke::new(1.0, crosshair_color),
        );
    }

    /// Draw particles in preview
    fn draw_particles(&self, painter: &egui::Painter, rect: egui::Rect) {
        let center = rect.center();
        let particles = self.particle_system.particles();

        for particle in particles {
            if !particle.alive {
                continue;
            }

            // Convert particle position to screen space
            let screen_pos = egui::pos2(
                center.x + particle.position.x * self.preview.scale,
                center.y - particle.position.y * self.preview.scale, // Flip Y for screen coords
            );

            // Skip if outside preview rect
            if !rect.contains(screen_pos) {
                continue;
            }

            // Calculate color with alpha
            let color = egui::Color32::from_rgba_premultiplied(
                (particle.color[0] * 255.0) as u8,
                (particle.color[1] * 255.0) as u8,
                (particle.color[2] * 255.0) as u8,
                (particle.color[3] * 255.0) as u8,
            );

            // Draw particle as circle
            let size = particle.scale * 10.0 * self.preview.scale;
            if size > 1.0 {
                painter.circle_filled(screen_pos, size.max(1.0), color);
            }
        }
    }

    /// Export current emitter as code
    pub fn export_as_code(&self) -> String {
        format!(
            r#"// Generated Particle Emitter
ParticleEmitter {{
    emission_rate: {:.1},
    max_particles: {},
    lifetime: {:.2}..{:.2},
    velocity: Vec2::new({:.1}, {:.1})..Vec2::new({:.1}, {:.1}),
    acceleration: Vec2::new({:.1}, {:.1}),
    rotation_speed: {:.1}..{:.1},
    start_scale: {:.2}..{:.2},
    end_scale: {:.2},
    start_color: [{:.2}, {:.2}, {:.2}, {:.2}],
    end_color: [{:.2}, {:.2}, {:.2}, {:.2}],
    start_alpha: {:.2},
    end_alpha: {:.2},
    blend_mode: BlendMode::{:?},
    ..Default::default()
}}"#,
            self.emitter.emission_rate,
            self.emitter.max_particles,
            self.emitter.lifetime.start,
            self.emitter.lifetime.end,
            self.emitter.velocity.start.x,
            self.emitter.velocity.start.y,
            self.emitter.velocity.end.x,
            self.emitter.velocity.end.y,
            self.emitter.acceleration.x,
            self.emitter.acceleration.y,
            self.emitter.rotation_speed.start,
            self.emitter.rotation_speed.end,
            self.emitter.start_scale.start,
            self.emitter.start_scale.end,
            self.emitter.end_scale,
            self.emitter.start_color[0],
            self.emitter.start_color[1],
            self.emitter.start_color[2],
            self.emitter.start_color[3],
            self.emitter.end_color[0],
            self.emitter.end_color[1],
            self.emitter.end_color[2],
            self.emitter.end_color[3],
            self.emitter.start_alpha,
            self.emitter.end_alpha,
            self.emitter.blend_mode,
        )
    }
}

impl Default for ParticleEditor {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper function to sanitize filename
fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect::<String>()
        .to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_particle_preset_creation() {
        let fire = ParticlePreset::Fire.create_emitter(Vec2::ZERO);
        assert_eq!(fire.blend_mode, BlendMode::Additive);
        assert!(fire.emission_rate > 0.0);

        let rain = ParticlePreset::Rain.create_emitter(Vec2::ZERO);
        assert!(rain.velocity.start.y < 0.0); // Rain falls down
    }

    #[test]
    fn test_emitter_serialization() {
        let emitter = ParticlePreset::Magic.create_emitter(Vec2::ZERO);
        let data = ParticleEmitterData::from(&emitter);
        let reconstructed = ParticleEmitter::from(&data);

        assert_eq!(emitter.emission_rate, reconstructed.emission_rate);
        assert_eq!(emitter.max_particles, reconstructed.max_particles);
        assert_eq!(emitter.blend_mode, reconstructed.blend_mode);
    }

    #[test]
    fn test_particle_system_data_serialization() {
        let data = ParticleSystemData::default();
        let json = serde_json::to_string(&data).unwrap();
        let deserialized: ParticleSystemData = serde_json::from_str(&json).unwrap();

        assert_eq!(data.name, deserialized.name);
        assert_eq!(data.version, deserialized.version);
    }

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("Hello World"), "hello_world");
        assert_eq!(sanitize_filename("Test-File_123"), "test-file_123");
        assert_eq!(sanitize_filename("A@B#C"), "a_b_c");
    }

    #[test]
    fn test_editor_export_code() {
        let editor = ParticleEditor::new();
        let code = editor.export_as_code();
        assert!(code.contains("ParticleEmitter"));
        assert!(code.contains("emission_rate"));
        assert!(code.contains("max_particles"));
    }
}
