//! Simulation systems

use crate::{World, TICK_RATE};
use crate::events::EventBus;
use crate::resources::{RngPool, SimTime, SimulationStats};

/// Fixed timestep simulation
pub struct Simulation {
    accumulator: std::time::Duration,
    tick_count: u64,
    time: SimTime,
    stats: SimulationStats,
    rng: RngPool,
}

impl Simulation {
    pub fn new(seed: u64) -> Self {
        Self {
            accumulator: std::time::Duration::ZERO,
            tick_count: 0,
            time: SimTime::default(),
            stats: SimulationStats::default(),
            rng: RngPool::from_seed(seed),
        }
    }
    
    /// Accumulate time and run simulation ticks
    pub fn update(&mut self, delta: std::time::Duration, world: &mut World, event_bus: &EventBus) {
        self.accumulator += delta;
        
        let mut ticks_this_frame = 0;
        
        while self.accumulator >= TICK_RATE {
            self.tick(world, event_bus);
            self.accumulator -= TICK_RATE;
            ticks_this_frame += 1;
            
            // Prevent death spiral
            if ticks_this_frame >= 10 {
                tracing::warn!("Simulation fell behind, dropping {} ticks", 
                    self.accumulator.as_millis() / 50);
                self.accumulator = std::time::Duration::ZERO;
                break;
            }
        }
    }
    
    /// Single simulation tick
    fn tick(&mut self, _world: &mut World, _event_bus: &EventBus) {
        self.tick_count += 1;
        self.time.tick();
        
        // Update simulation stats
        for (_, stat) in self.stats.stats.iter_mut() {
            stat.tick();
        }
        
        // TODO: Run simulation systems (AI, physics, etc.)
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
}
