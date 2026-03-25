//! AI/Behavior components

use serde::{Deserialize, Serialize};

/// Vibecode logic prompt component (TOML directives)
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct LogicPrompt {
    pub directives: String,
}

/// Dialogue tree reference
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct DialogueTreeRef {
    pub tree_id: Option<u32>,
}

/// AI state for behavior
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum AiState {
    #[default]
    Idle,
    Patrol,
    Chase,
    Flee,
    Schedule,
    Combat,
    Dead,
}

/// Patrol path component
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PatrolPath {
    pub waypoints: Vec<Waypoint>,
    pub current_index: usize,
    pub loop_mode: LoopMode,
    pub wait_ticks_remaining: u32,
}

/// Waypoint in a patrol path
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Waypoint {
    pub x: i32,
    pub y: i32,
    pub wait_ticks: u32,
}

/// Loop mode for patrol paths
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum LoopMode {
    #[default]
    Loop,
    PingPong,
    Once,
}

/// Schedule entry for time-of-day behavior
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScheduleEntry {
    pub hour: u8,
    pub x: i32,
    pub y: i32,
    pub behavior: ScheduledBehavior,
}

/// Scheduled behavior types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScheduledBehavior {
    Idle,
    Work,
    Sleep,
    Patrol,
    GoToLocation,
}

/// Schedule component
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Schedule {
    pub entries: Vec<ScheduleEntry>,
}

/// Target entity for chase/follow behavior
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TargetEntity {
    pub target: crate::Entity,
}

impl TargetEntity {
    pub fn new(target: crate::Entity) -> Self {
        Self { target }
    }
}

/// Detection radius for AI
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct DetectionRadius {
    pub radius: f32,
}

/// Movement speed component
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct MovementSpeed {
    /// Tiles per second
    pub speed: f32,
}

impl MovementSpeed {
    pub fn from_spd_stat(spd: i32) -> Self {
        // Base speed + spd bonus
        let speed = 2.0 + (spd as f32 / 50.0);
        Self { speed }
    }
    
    pub fn run_speed(&self) -> f32 {
        self.speed * 2.0
    }
}

/// Pathfinding target (current movement target)
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PathfindingTarget {
    pub x: i32,
    pub y: i32,
}

/// Current path (waypoints to follow)
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CurrentPath {
    pub waypoints: Vec<(i32, i32)>,
    pub current_index: usize,
    pub failed_attempts: u32,
}

impl CurrentPath {
    pub fn is_complete(&self) -> bool {
        self.current_index >= self.waypoints.len()
    }
    
    pub fn next_waypoint(&self) -> Option<(i32, i32)> {
        self.waypoints.get(self.current_index).copied()
    }
    
    pub fn advance(&mut self) {
        self.current_index += 1;
    }
    
    pub fn clear(&mut self) {
        self.waypoints.clear();
        self.current_index = 0;
        self.failed_attempts = 0;
    }
    
    pub fn record_failure(&mut self) {
        self.failed_attempts += 1;
    }
    
    pub fn should_give_up(&self) -> bool {
        self.failed_attempts >= 3
    }
}
