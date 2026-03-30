//! Turn Queue System
//!
//! ATB-based turn queue management for battle system.

use dde_core::Entity;
use std::collections::VecDeque;

/// ATB gauge for a combatant
#[derive(Debug, Clone, Copy)]
pub struct AtbGauge {
    /// Current ATB value (0-100)
    pub value: f32,
    /// Speed stat (affects fill rate)
    pub speed: i32,
    /// Maximum ATB value
    pub max: f32,
}

impl AtbGauge {
    /// Create a new ATB gauge with given speed
    pub fn new(speed: i32) -> Self {
        Self {
            value: 0.0,
            speed,
            max: 100.0,
        }
    }

    /// Create from speed stat
    pub fn from_speed(speed: i32) -> Self {
        Self::new(speed)
    }

    /// Reset gauge to 0
    pub fn reset(&mut self) {
        self.value = 0.0;
    }

    /// Fill gauge to max
    pub fn fill(&mut self) {
        self.value = self.max;
    }

    /// Tick the gauge (called each update)
    pub fn tick(&mut self) {
        let increment = self.speed as f32 * 0.1; // Base increment
        self.value = (self.value + increment).min(self.max);
    }

    /// Check if gauge is full (ready to act)
    pub fn is_full(&self) -> bool {
        self.value >= self.max
    }

    /// Get fill percentage (0-100)
    pub fn percentage(&self) -> f32 {
        (self.value / self.max) * 100.0
    }
}

impl Default for AtbGauge {
    fn default() -> Self {
        Self::new(50)
    }
}

/// Turn queue entry
#[derive(Debug, Clone, Copy)]
pub struct TurnEntry {
    /// Entity ID
    pub entity: Entity,
    /// ATB gauge value at time of entry
    pub atb_value: f32,
    /// Entry timestamp
    pub tick: u64,
}

/// Turn queue for managing battle turns
#[derive(Debug, Clone)]
pub struct TurnQueue {
    /// Queue of ready entities (ATB full)
    ready_queue: VecDeque<TurnEntry>,
    /// Current tick count
    tick_count: u64,
    /// All tracked gauges
    gauges: Vec<(Entity, AtbGauge)>,
}

impl TurnQueue {
    /// Create a new turn queue
    pub fn new() -> Self {
        Self {
            ready_queue: VecDeque::new(),
            tick_count: 0,
            gauges: Vec::new(),
        }
    }

    /// Register a combatant with the queue
    pub fn register(&mut self, entity: Entity, speed: i32) {
        if !self.gauges.iter().any(|(e, _)| *e == entity) {
            self.gauges.push((entity, AtbGauge::new(speed)));
        }
    }

    /// Unregister a combatant
    pub fn unregister(&mut self, entity: Entity) {
        self.gauges.retain(|(e, _)| *e != entity);
        self.ready_queue.retain(|e| e.entity != entity);
    }

    /// Reset all gauges
    pub fn reset_all(&mut self) {
        for (_, gauge) in &mut self.gauges {
            gauge.reset();
        }
        self.ready_queue.clear();
    }

    /// Reset specific entity's gauge
    pub fn reset_entity(&mut self, entity: Entity) {
        if let Some((_, gauge)) = self.gauges.iter_mut().find(|(e, _)| *e == entity) {
            gauge.reset();
        }
        self.ready_queue.retain(|e| e.entity != entity);
    }

    /// Update all gauges (call each tick)
    pub fn tick(&mut self) {
        self.tick_count += 1;

        for (entity, gauge) in &mut self.gauges {
            let was_full = gauge.is_full();
            gauge.tick();
            
            // If just became full, add to ready queue
            if !was_full && gauge.is_full() {
                self.ready_queue.push_back(TurnEntry {
                    entity: *entity,
                    atb_value: gauge.value,
                    tick: self.tick_count,
                });
            }
        }
    }

    /// Get the next ready entity
    pub fn next_ready(&mut self) -> Option<Entity> {
        self.ready_queue.pop_front().map(|e| e.entity)
    }

    /// Peek at the next ready entity without removing
    pub fn peek_ready(&self) -> Option<Entity> {
        self.ready_queue.front().map(|e| e.entity)
    }

    /// Check if any entity is ready
    pub fn has_ready(&self) -> bool {
        !self.ready_queue.is_empty()
    }

    /// Get count of ready entities
    pub fn ready_count(&self) -> usize {
        self.ready_queue.len()
    }

    /// Get all ready entities
    pub fn get_ready(&self) -> Vec<Entity> {
        self.ready_queue.iter().map(|e| e.entity).collect()
    }

    /// Get ATB gauge for an entity
    pub fn get_gauge(&self, entity: Entity) -> Option<&AtbGauge> {
        self.gauges.iter().find(|(e, _)| *e == entity).map(|(_, g)| g)
    }

    /// Get mutable ATB gauge for an entity
    pub fn get_gauge_mut(&mut self, entity: Entity) -> Option<&mut AtbGauge> {
        self.gauges.iter_mut().find(|(e, _)| *e == entity).map(|(_, g)| g)
    }

    /// Get all entities and their gauges
    pub fn get_all_gauges(&self) -> &[(Entity, AtbGauge)] {
        &self.gauges
    }

    /// Get current tick count
    pub fn tick_count(&self) -> u64 {
        self.tick_count
    }

    /// Clear the queue
    pub fn clear(&mut self) {
        self.ready_queue.clear();
        self.gauges.clear();
        self.tick_count = 0;
    }

    /// Get number of registered combatants
    pub fn len(&self) -> usize {
        self.gauges.len()
    }

    /// Check if queue is empty
    pub fn is_empty(&self) -> bool {
        self.gauges.is_empty()
    }
}

impl Default for TurnQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_turn_queue_creation() {
        let queue = TurnQueue::new();
        assert!(queue.is_empty());
        assert_eq!(queue.ready_count(), 0);
    }

    #[test]
    fn test_register_unregister() {
        let mut queue = TurnQueue::new();
        let entity = Entity::from_id(1);

        queue.register(entity, 100);
        assert_eq!(queue.len(), 1);

        queue.unregister(entity);
        assert!(queue.is_empty());
    }

    #[test]
    fn test_atb_gauge() {
        let mut gauge = AtbGauge::new(100);
        assert!(!gauge.is_full());
        
        gauge.tick();
        assert!(gauge.value > 0.0);
        
        gauge.fill();
        assert!(gauge.is_full());
        
        gauge.reset();
        assert_eq!(gauge.value, 0.0);
    }

    #[test]
    fn test_turn_queue_tick() {
        let mut queue = TurnQueue::new();
        let entity = Entity::from_id(1);

        queue.register(entity, 1000); // Very fast
        
        // Tick many times
        for _ in 0..20 {
            queue.tick();
            if queue.has_ready() {
                break;
            }
        }

        // Fast entity should become ready
        assert!(queue.has_ready());
        assert_eq!(queue.next_ready(), Some(entity));
    }
}
