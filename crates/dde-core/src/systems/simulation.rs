//! Simulation systems

use crate::components::animation::AnimationDef;
use crate::events::EngineEventBus;
use crate::resources::{RngPool, SimTime, SimulationStats};
use crate::systems::animation::AnimationSystem;
use crate::systems::{BarkSystem, TileCollisionMap};
use crate::{World, TICK_RATE};

/// Fixed timestep simulation
pub struct Simulation {
    accumulator: std::time::Duration,
    tick_count: u64,
    time: SimTime,
    stats: SimulationStats,
    rng: RngPool,
    /// Animation definitions for the animation system
    animation_defs: Vec<AnimationDef>,
    /// Tile collision map for movement system
    collision_map: Option<TileCollisionMap>,
}

impl Simulation {
    pub fn new(seed: u64) -> Self {
        Self {
            accumulator: std::time::Duration::ZERO,
            tick_count: 0,
            time: SimTime::default(),
            stats: SimulationStats::default(),
            rng: RngPool::from_seed(seed),
            animation_defs: Vec::new(),
            collision_map: None,
        }
    }

    /// Set animation definitions
    pub fn set_animation_defs(&mut self, defs: Vec<AnimationDef>) {
        self.animation_defs = defs;
    }

    /// Set collision map for movement
    pub fn set_collision_map(&mut self, map: TileCollisionMap) {
        self.collision_map = Some(map);
    }

    /// Accumulate time and run simulation ticks
    pub fn update(
        &mut self,
        delta: std::time::Duration,
        world: &mut World,
        event_bus: &EngineEventBus,
    ) {
        self.accumulator += delta;

        let mut ticks_this_frame = 0;

        while self.accumulator >= TICK_RATE {
            self.tick(world, event_bus);
            self.accumulator -= TICK_RATE;
            ticks_this_frame += 1;

            // Prevent death spiral
            if ticks_this_frame >= 10 {
                tracing::warn!(
                    "Simulation fell behind, dropping {} ticks",
                    self.accumulator.as_millis() / 50
                );
                self.accumulator = std::time::Duration::ZERO;
                break;
            }
        }
    }

    /// Single simulation tick - runs all simulation systems
    fn tick(&mut self, world: &mut World, event_bus: &EngineEventBus) {
        self.tick_count += 1;
        self.time.tick();

        // Update simulation stats
        for (_, stat) in self.stats.stats.iter_mut() {
            stat.tick();
        }

        // Run animation system (50ms per tick = 20 ticks/second)
        if !self.animation_defs.is_empty() {
            AnimationSystem::update(world, TICK_RATE.as_millis() as f32, &self.animation_defs);
        }

        // Run movement system if collision map is available
        if let Some(ref collision_map) = self.collision_map {
            use crate::systems::MovementSystem;
            MovementSystem::update(world, collision_map, TICK_RATE.as_secs_f32());
        }

        // Run bark system for NPC ambient dialogue
        // Find player position for proximity checks
        let player_pos = world
            .query_mut::<(&crate::components::Position, &crate::systems::Player)>()
            .into_iter()
            .next()
            .map(|(_, (pos, _))| glam::Vec2::new(pos.x as f32, pos.y as f32));

        // Calculate current time from ticks (50ms per tick)
        let current_time = self.tick_count as f32 * TICK_RATE.as_secs_f32();

        if let Some(pos) = player_pos {
            let bark_system = BarkSystem::new();
            bark_system.update(world, TICK_RATE.as_secs_f32(), current_time, pos);
        } else {
            // Just update bark display times without proximity triggers
            use crate::systems::NpcBark;
            for (_, bark) in world.query_mut::<&mut NpcBark>() {
                bark.update(TICK_RATE.as_secs_f32());
            }
        }

        // Emit simulation tick event for other systems to respond
        use crate::events::EngineEvent;
        event_bus.emit(EngineEvent::SimulationTick {
            tick_count: self.tick_count,
        });
    }

    pub fn tick_count(&self) -> u64 {
        self.tick_count
    }

    pub fn time(&self) -> &SimTime {
        &self.time
    }

    pub fn rng(&mut self) -> &mut RngPool {
        &mut self.rng
    }

    pub fn seed(&self) -> u64 {
        self.rng.seed()
    }
}
