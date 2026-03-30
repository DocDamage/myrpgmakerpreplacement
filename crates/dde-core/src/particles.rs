//! Particle & VFX System
//!
//! Particle effects for weather (rain, snow), environmental FX (fire, smoke),
//! battle spells (explosions, beams), and calamity visuals.

use glam::Vec2;

/// Particle emitter component
#[derive(Debug, Clone)]
pub struct ParticleEmitter {
    /// Particle texture/sprite ID
    pub sprite_id: u32,
    /// Particles per second
    pub emission_rate: f32,
    /// Maximum particles in pool
    pub max_particles: usize,
    /// Lifetime range in seconds
    pub lifetime: std::ops::Range<f32>,
    /// Initial velocity range
    pub velocity: std::ops::Range<Vec2>,
    /// Acceleration (gravity, wind)
    pub acceleration: Vec2,
    /// Rotation speed range (degrees/sec)
    pub rotation_speed: std::ops::Range<f32>,
    /// Start scale range
    pub start_scale: std::ops::Range<f32>,
    /// End scale (lerps over lifetime)
    pub end_scale: f32,
    /// Start color (RGBA)
    pub start_color: [f32; 4],
    /// End color (RGBA)
    pub end_color: [f32; 4],
    /// Start alpha
    pub start_alpha: f32,
    /// End alpha
    pub end_alpha: f32,
    /// Whether emitter is active
    pub active: bool,
    /// Emitter position
    pub position: Vec2,
    /// Emission shape
    pub shape: EmissionShape,
    /// Blend mode
    pub blend_mode: BlendMode,
}

/// Emission shape for particle spawn
#[derive(Debug, Clone, Copy)]
pub enum EmissionShape {
    /// Point emission
    Point,
    /// Circle with radius
    Circle { radius: f32 },
    /// Rectangle with width/height
    Rectangle { width: f32, height: f32 },
    /// Line with length
    Line { length: f32, angle: f32 },
}

/// Blend mode for particles
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendMode {
    /// Normal alpha blending
    Alpha,
    /// Additive blending (for glowing particles)
    Additive,
    /// Multiply blending
    Multiply,
}

impl Default for ParticleEmitter {
    fn default() -> Self {
        Self {
            sprite_id: 0,
            emission_rate: 10.0,
            max_particles: 100,
            lifetime: 1.0..2.0,
            velocity: Vec2::new(-10.0, -50.0)..Vec2::new(10.0, -100.0),
            acceleration: Vec2::new(0.0, -98.0), // Gravity
            rotation_speed: -45.0..45.0,
            start_scale: 0.5..1.0,
            end_scale: 0.0,
            start_color: [1.0, 1.0, 1.0, 1.0],
            end_color: [1.0, 1.0, 1.0, 1.0],
            start_alpha: 1.0,
            end_alpha: 0.0,
            active: true,
            position: Vec2::ZERO,
            shape: EmissionShape::Point,
            blend_mode: BlendMode::Alpha,
        }
    }
}

/// Individual particle
#[derive(Debug, Clone, Copy)]
pub struct Particle {
    /// Current position
    pub position: Vec2,
    /// Current velocity
    pub velocity: Vec2,
    /// Remaining lifetime in seconds
    pub lifetime: f32,
    /// Maximum lifetime (for ratio calculations)
    pub max_lifetime: f32,
    /// Current scale
    pub scale: f32,
    /// Start scale
    pub start_scale: f32,
    /// End scale
    pub end_scale: f32,
    /// Current rotation in degrees
    pub rotation: f32,
    /// Rotation speed (degrees/sec)
    pub rotation_speed: f32,
    /// Current color (RGBA)
    pub color: [f32; 4],
    /// Start color
    pub start_color: [f32; 4],
    /// End color
    pub end_color: [f32; 4],
    /// Start alpha
    pub start_alpha: f32,
    /// End alpha
    pub end_alpha: f32,
    /// Whether particle is alive
    pub alive: bool,
}

impl Particle {
    /// Create a new particle from emitter settings
    pub fn new(emitter: &ParticleEmitter, rng: &mut impl rand::Rng) -> Self {
        let t: f32 = rng.gen();
        let max_lifetime =
            emitter.lifetime.start + t * (emitter.lifetime.end - emitter.lifetime.start);

        let vx = rng.gen_range(emitter.velocity.start.x..emitter.velocity.end.x);
        let vy = rng.gen_range(emitter.velocity.start.y..emitter.velocity.end.y);

        let start_scale = rng.gen_range(emitter.start_scale.start..emitter.start_scale.end);
        let rotation_speed =
            rng.gen_range(emitter.rotation_speed.start..emitter.rotation_speed.end);

        // Calculate spawn position based on shape
        let position = match emitter.shape {
            EmissionShape::Point => emitter.position,
            EmissionShape::Circle { radius } => {
                let angle = rng.gen_range(0.0..std::f32::consts::TAU);
                let r = rng.gen_range(0.0..radius);
                emitter.position + Vec2::new(angle.cos() * r, angle.sin() * r)
            }
            EmissionShape::Rectangle { width, height } => {
                let x = rng.gen_range(-width / 2.0..width / 2.0);
                let y = rng.gen_range(-height / 2.0..height / 2.0);
                emitter.position + Vec2::new(x, y)
            }
            EmissionShape::Line { length, angle } => {
                let t = rng.gen_range(-0.5..0.5);
                let dx = angle.cos() * length * t;
                let dy = angle.sin() * length * t;
                emitter.position + Vec2::new(dx, dy)
            }
        };

        Self {
            position,
            velocity: Vec2::new(vx, vy),
            lifetime: max_lifetime,
            max_lifetime,
            scale: start_scale,
            start_scale,
            end_scale: emitter.end_scale,
            rotation: rng.gen_range(0.0..360.0),
            rotation_speed,
            color: emitter.start_color,
            start_color: emitter.start_color,
            end_color: emitter.end_color,
            start_alpha: emitter.start_alpha,
            end_alpha: emitter.end_alpha,
            alive: true,
        }
    }

    /// Update particle state
    pub fn update(&mut self, dt: f32, acceleration: Vec2) {
        if !self.alive {
            return;
        }

        // Update lifetime
        self.lifetime -= dt;
        if self.lifetime <= 0.0 {
            self.alive = false;
            return;
        }

        // Update physics
        self.velocity += acceleration * dt;
        self.position += self.velocity * dt;

        // Update rotation
        self.rotation += self.rotation_speed * dt;

        // Calculate life ratio (0.0 = just spawned, 1.0 = dead)
        let t = 1.0 - (self.lifetime / self.max_lifetime);

        // Lerp scale
        self.scale = self.start_scale + (self.end_scale - self.start_scale) * t;

        // Lerp color
        for i in 0..4 {
            self.color[i] = self.start_color[i] + (self.end_color[i] - self.start_color[i]) * t;
        }

        // Lerp alpha
        let alpha = self.start_alpha + (self.end_alpha - self.start_alpha) * t;
        self.color[3] = alpha;
    }
}

/// Particle system managing all emitters and particles
pub struct ParticleSystem {
    /// Active particles
    particles: Vec<Particle>,
    /// Particle pool for reuse
    pool: Vec<Particle>,
    /// Global particle count
    total_particles: usize,
    /// Max particles system-wide
    max_particles: usize,
}

impl ParticleSystem {
    /// Create a new particle system
    pub fn new(max_particles: usize) -> Self {
        Self {
            particles: Vec::with_capacity(max_particles / 4),
            pool: Vec::with_capacity(max_particles / 4),
            total_particles: 0,
            max_particles,
        }
    }

    /// Spawn particles from an emitter
    pub fn spawn(&mut self, emitter: &ParticleEmitter, count: usize, rng: &mut impl rand::Rng) {
        if !emitter.active {
            return;
        }

        let count = count.min(self.max_particles - self.total_particles);

        for _ in 0..count {
            let particle = if let Some(mut p) = self.pool.pop() {
                p = Particle::new(emitter, rng);
                p
            } else {
                Particle::new(emitter, rng)
            };

            self.particles.push(particle);
            self.total_particles += 1;
        }
    }

    /// Update all particles
    pub fn update(&mut self, dt: f32, gravity: Vec2) {
        let mut dead_count = 0;

        for particle in &mut self.particles {
            particle.update(dt, gravity);
            if !particle.alive {
                dead_count += 1;
            }
        }

        // Remove dead particles and return to pool
        if dead_count > 0 {
            self.particles.retain(|p| {
                if p.alive {
                    true
                } else {
                    self.total_particles -= 1;
                    // Return to pool if not too large
                    if self.pool.len() < self.max_particles / 4 {
                        self.pool.push(*p);
                    }
                    false
                }
            });
        }
    }

    /// Get all active particles
    pub fn particles(&self) -> &[Particle] {
        &self.particles
    }

    /// Get particle count
    pub fn count(&self) -> usize {
        self.particles.len()
    }

    /// Clear all particles
    pub fn clear(&mut self) {
        self.particles.clear();
        self.total_particles = 0;
    }
}

impl Default for ParticleSystem {
    fn default() -> Self {
        Self::new(10000)
    }
}

/// Preset weather emitters
pub mod weather {
    use super::*;

    /// Rain emitter preset
    pub fn rain(position: Vec2, width: f32) -> ParticleEmitter {
        ParticleEmitter {
            sprite_id: 1, // rain drop
            emission_rate: 100.0,
            max_particles: 500,
            lifetime: 0.5..1.0,
            velocity: Vec2::new(-20.0, -200.0)..Vec2::new(20.0, -400.0),
            acceleration: Vec2::new(0.0, -100.0),
            start_scale: 0.5..1.0,
            end_scale: 0.5,
            start_color: [0.6, 0.7, 0.9, 1.0],
            end_color: [0.6, 0.7, 0.9, 1.0],
            start_alpha: 0.6,
            end_alpha: 0.3,
            position,
            shape: EmissionShape::Line {
                length: width,
                angle: 0.0,
            },
            blend_mode: BlendMode::Additive,
            ..Default::default()
        }
    }

    /// Snow emitter preset
    pub fn snow(position: Vec2, width: f32) -> ParticleEmitter {
        ParticleEmitter {
            sprite_id: 2, // snow flake
            emission_rate: 30.0,
            max_particles: 300,
            lifetime: 3.0..5.0,
            velocity: Vec2::new(-10.0, -20.0)..Vec2::new(10.0, -40.0),
            acceleration: Vec2::new(0.0, -10.0),
            start_scale: 0.3..0.6,
            end_scale: 0.3,
            start_color: [1.0, 1.0, 1.0, 1.0],
            end_color: [1.0, 1.0, 1.0, 1.0],
            start_alpha: 0.8,
            end_alpha: 0.4,
            position,
            shape: EmissionShape::Line {
                length: width,
                angle: 0.0,
            },
            blend_mode: BlendMode::Alpha,
            ..Default::default()
        }
    }

    /// Ash/calamity emitter preset
    pub fn ash(position: Vec2, width: f32) -> ParticleEmitter {
        ParticleEmitter {
            sprite_id: 3, // ash particle
            emission_rate: 50.0,
            max_particles: 400,
            lifetime: 4.0..8.0,
            velocity: Vec2::new(-5.0, 10.0)..Vec2::new(5.0, 30.0),
            acceleration: Vec2::new(0.0, 5.0),
            start_scale: 0.2..0.5,
            end_scale: 0.1,
            start_color: [0.8, 0.3, 0.2, 1.0],
            end_color: [0.4, 0.2, 0.1, 1.0],
            start_alpha: 0.7,
            end_alpha: 0.2,
            position,
            shape: EmissionShape::Line {
                length: width,
                angle: 0.0,
            },
            blend_mode: BlendMode::Alpha,
            ..Default::default()
        }
    }

    /// Sandstorm emitter preset
    pub fn sandstorm(position: Vec2, height: f32) -> ParticleEmitter {
        ParticleEmitter {
            sprite_id: 4, // sand particle
            emission_rate: 200.0,
            max_particles: 800,
            lifetime: 1.0..2.0,
            velocity: Vec2::new(100.0, -10.0)..Vec2::new(200.0, 10.0),
            acceleration: Vec2::new(50.0, 0.0),
            start_scale: 0.3..0.8,
            end_scale: 0.3,
            start_color: [0.9, 0.7, 0.4, 1.0],
            end_color: [0.8, 0.6, 0.3, 1.0],
            start_alpha: 0.5,
            end_alpha: 0.2,
            position,
            shape: EmissionShape::Line {
                length: height,
                angle: std::f32::consts::FRAC_PI_2,
            },
            blend_mode: BlendMode::Alpha,
            ..Default::default()
        }
    }
}

/// Preset spell/battle effects
pub mod spells {
    use super::*;

    /// Fire explosion effect
    pub fn fire_explosion(position: Vec2) -> ParticleEmitter {
        ParticleEmitter {
            sprite_id: 10,      // fire particle
            emission_rate: 0.0, // Burst
            max_particles: 50,
            lifetime: 0.3..0.8,
            velocity: Vec2::new(-50.0, -50.0)..Vec2::new(50.0, 50.0),
            acceleration: Vec2::new(0.0, -30.0),
            start_scale: 0.5..1.5,
            end_scale: 0.0,
            start_color: [1.0, 0.8, 0.2, 1.0],
            end_color: [1.0, 0.2, 0.0, 1.0],
            start_alpha: 1.0,
            end_alpha: 0.0,
            position,
            shape: EmissionShape::Circle { radius: 20.0 },
            blend_mode: BlendMode::Additive,
            active: false, // Burst emitter
            ..Default::default()
        }
    }

    /// Healing sparkles effect
    pub fn heal_sparkles(position: Vec2) -> ParticleEmitter {
        ParticleEmitter {
            sprite_id: 11, // sparkle
            emission_rate: 20.0,
            max_particles: 30,
            lifetime: 1.0..2.0,
            velocity: Vec2::new(-10.0, 20.0)..Vec2::new(10.0, 50.0),
            acceleration: Vec2::new(0.0, -20.0),
            start_scale: 0.2..0.4,
            end_scale: 0.0,
            start_color: [0.8, 1.0, 0.8, 1.0],
            end_color: [0.4, 1.0, 0.4, 1.0],
            start_alpha: 1.0,
            end_alpha: 0.0,
            position,
            shape: EmissionShape::Circle { radius: 15.0 },
            blend_mode: BlendMode::Additive,
            ..Default::default()
        }
    }
}
