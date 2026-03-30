//! Pathfinding & Navigation
//!
//! A* pathfinding on the tile grid with support for different terrain costs
//! and entity avoidance.

use glam::IVec2;
use std::collections::{BinaryHeap, HashMap, HashSet};

/// Node for A* pathfinding
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Node {
    /// Position on grid
    position: IVec2,
    /// Cost from start (g)
    g_cost: i32,
    /// Estimated cost to goal (h)
    h_cost: i32,
    /// Total cost (f = g + h)
    f_cost: i32,
}

impl Ord for Node {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Reverse order for min-heap
        other.f_cost.cmp(&self.f_cost)
    }
}

impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Pathfinding grid
pub struct PathGrid {
    /// Width of the grid
    width: i32,
    /// Height of the grid
    height: i32,
    /// Passability cache (true = walkable)
    passable: Vec<bool>,
    /// Movement costs (default = 1.0)
    costs: Vec<f32>,
    /// Entities occupying tiles (cost multiplier)
    occupied: Vec<f32>,
}

impl PathGrid {
    /// Create a new pathfinding grid
    pub fn new(width: i32, height: i32) -> Self {
        let size = (width * height) as usize;
        Self {
            width,
            height,
            passable: vec![true; size],
            costs: vec![1.0; size],
            occupied: vec![1.0; size],
        }
    }

    /// Get index for position
    fn index(&self, x: i32, y: i32) -> Option<usize> {
        if x < 0 || x >= self.width || y < 0 || y >= self.height {
            return None;
        }
        Some((y * self.width + x) as usize)
    }

    /// Check if a tile is walkable
    pub fn is_walkable(&self, x: i32, y: i32) -> bool {
        self.index(x, y).map(|i| self.passable[i]).unwrap_or(false)
    }

    /// Set tile passability
    pub fn set_walkable(&mut self, x: i32, y: i32, walkable: bool) {
        if let Some(i) = self.index(x, y) {
            self.passable[i] = walkable;
        }
    }

    /// Set tile movement cost
    pub fn set_cost(&mut self, x: i32, y: i32, cost: f32) {
        if let Some(i) = self.index(x, y) {
            self.costs[i] = cost;
        }
    }

    /// Mark tile as occupied by entity
    pub fn set_occupied(&mut self, x: i32, y: i32, occupied: bool) {
        if let Some(i) = self.index(x, y) {
            self.occupied[i] = if occupied { 5.0 } else { 1.0 };
        }
    }

    /// Get movement cost for a tile
    pub fn get_cost(&self, x: i32, y: i32) -> f32 {
        self.index(x, y)
            .map(|i| self.costs[i] * self.occupied[i])
            .unwrap_or(f32::INFINITY)
    }

    /// A* pathfinding from start to goal
    pub fn find_path(&self, start: IVec2, goal: IVec2) -> Option<Path> {
        // Check bounds
        if !self.is_walkable(start.x, start.y) || !self.is_walkable(goal.x, goal.y) {
            return None;
        }

        // Already at goal
        if start == goal {
            return Some(Path {
                waypoints: vec![start],
                total_cost: 0.0,
            });
        }

        let mut open_set = BinaryHeap::new();
        let mut closed_set = HashSet::new();
        let mut came_from: HashMap<IVec2, IVec2> = HashMap::new();
        let mut g_scores: HashMap<IVec2, i32> = HashMap::new();

        let start_node = Node {
            position: start,
            g_cost: 0,
            h_cost: Self::heuristic(start, goal),
            f_cost: Self::heuristic(start, goal),
        };

        open_set.push(start_node);
        g_scores.insert(start, 0);

        while let Some(current) = open_set.pop() {
            // Reached goal
            if current.position == goal {
                let path = self.reconstruct_path(came_from, goal);
                let total_cost = *g_scores.get(&goal).unwrap_or(&0) as f32;
                return Some(Path {
                    waypoints: path,
                    total_cost,
                });
            }

            // Already processed
            if closed_set.contains(&current.position) {
                continue;
            }

            closed_set.insert(current.position);

            // Check neighbors
            for neighbor in self.neighbors(current.position) {
                if closed_set.contains(&neighbor) {
                    continue;
                }

                let cost = self.get_cost(neighbor.x, neighbor.y);
                if cost.is_infinite() {
                    continue;
                }

                let tentative_g = current.g_cost + cost as i32;

                if tentative_g < *g_scores.get(&neighbor).unwrap_or(&i32::MAX) {
                    came_from.insert(neighbor, current.position);
                    g_scores.insert(neighbor, tentative_g);

                    let h = Self::heuristic(neighbor, goal);
                    open_set.push(Node {
                        position: neighbor,
                        g_cost: tentative_g,
                        h_cost: h,
                        f_cost: tentative_g + h,
                    });
                }
            }
        }

        // No path found
        None
    }

    /// Manhattan distance heuristic
    fn heuristic(a: IVec2, b: IVec2) -> i32 {
        (a.x - b.x).abs() + (a.y - b.y).abs()
    }

    /// Get walkable neighbors
    fn neighbors(&self, pos: IVec2) -> Vec<IVec2> {
        let dirs = [
            IVec2::new(0, 1),  // Down
            IVec2::new(0, -1), // Up
            IVec2::new(-1, 0), // Left
            IVec2::new(1, 0),  // Right
        ];

        dirs.iter()
            .filter_map(|&d| {
                let neighbor = pos + d;
                if self.is_walkable(neighbor.x, neighbor.y) {
                    Some(neighbor)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Reconstruct path from came_from map
    fn reconstruct_path(&self, came_from: HashMap<IVec2, IVec2>, goal: IVec2) -> Vec<IVec2> {
        let mut path = vec![goal];
        let mut current = goal;

        while let Some(&prev) = came_from.get(&current) {
            path.push(prev);
            current = prev;
        }

        path.reverse();
        path
    }

    /// Clear all occupancy marks
    pub fn clear_occupied(&mut self) {
        for val in &mut self.occupied {
            *val = 1.0;
        }
    }
}

/// Computed path
#[derive(Debug, Clone)]
pub struct Path {
    /// Waypoints from start to goal
    pub waypoints: Vec<IVec2>,
    /// Total path cost
    pub total_cost: f32,
}

impl Path {
    /// Check if path is empty
    pub fn is_empty(&self) -> bool {
        self.waypoints.is_empty()
    }

    /// Get path length
    pub fn len(&self) -> usize {
        self.waypoints.len()
    }

    /// Get next waypoint
    pub fn next(&self, current_idx: usize) -> Option<&IVec2> {
        self.waypoints.get(current_idx + 1)
    }

    /// Check if we've reached the end
    pub fn is_complete(&self, current_idx: usize) -> bool {
        current_idx + 1 >= self.waypoints.len()
    }
}

/// Path request for queuing
#[derive(Debug, Clone, Copy)]
pub struct PathRequest {
    pub entity_id: hecs::Entity,
    pub start: IVec2,
    pub goal: IVec2,
    pub retry_count: u32,
}

/// Pathfinding system
pub struct PathfindingSystem {
    grid: PathGrid,
    pending_requests: Vec<PathRequest>,
    completed_paths: HashMap<hecs::Entity, Path>,
    max_paths_per_tick: usize,
}

impl PathfindingSystem {
    /// Create new pathfinding system
    pub fn new(grid_width: i32, grid_height: i32) -> Self {
        Self {
            grid: PathGrid::new(grid_width, grid_height),
            pending_requests: Vec::new(),
            completed_paths: HashMap::new(),
            max_paths_per_tick: 10,
        }
    }

    /// Request a path
    pub fn request_path(&mut self, entity_id: hecs::Entity, start: IVec2, goal: IVec2) {
        self.pending_requests.push(PathRequest {
            entity_id,
            start,
            goal,
            retry_count: 0,
        });
    }

    /// Update pathfinding - process pending requests
    pub fn update(&mut self) {
        let count = self.pending_requests.len().min(self.max_paths_per_tick);

        for _ in 0..count {
            if let Some(request) = self.pending_requests.pop() {
                if let Some(path) = self.grid.find_path(request.start, request.goal) {
                    self.completed_paths.insert(request.entity_id, path);
                }
            }
        }
    }

    /// Get completed path for entity
    pub fn get_path(&mut self, entity_id: hecs::Entity) -> Option<Path> {
        self.completed_paths.remove(&entity_id)
    }

    /// Check if path is ready
    pub fn has_path(&self, entity_id: hecs::Entity) -> bool {
        self.completed_paths.contains_key(&entity_id)
    }

    /// Get mutable access to grid
    pub fn grid_mut(&mut self) -> &mut PathGrid {
        &mut self.grid
    }

    /// Get grid reference
    pub fn grid(&self) -> &PathGrid {
        &self.grid
    }

    /// Clear all pending requests
    pub fn clear_pending(&mut self) {
        self.pending_requests.clear();
    }

    /// Clear completed paths
    pub fn clear_completed(&mut self) {
        self.completed_paths.clear();
    }

    /// Get pending request count
    pub fn pending_count(&self) -> usize {
        self.pending_requests.len()
    }

    /// Get completed path count
    pub fn completed_count(&self) -> usize {
        self.completed_paths.len()
    }
}

/// NPC patrol behavior
#[derive(Debug, Clone)]
pub struct PatrolPath {
    /// Waypoints to patrol between
    pub waypoints: Vec<IVec2>,
    /// Current waypoint index
    current_idx: usize,
    /// Whether to reverse at end or loop
    pub loop_type: PatrolLoopType,
    /// Direction (1 = forward, -1 = backward)
    direction: i32,
    /// Wait time at each waypoint (seconds)
    pub wait_time: f32,
    /// Current wait timer
    wait_timer: f32,
}

/// How the patrol loops
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatrolLoopType {
    /// Loop back to start
    Loop,
    /// Reverse direction at ends
    PingPong,
    /// Stop at end
    Once,
}

impl PatrolPath {
    /// Create a new patrol path
    pub fn new(waypoints: Vec<IVec2>) -> Self {
        Self {
            waypoints,
            current_idx: 0,
            loop_type: PatrolLoopType::PingPong,
            direction: 1,
            wait_time: 1.0,
            wait_timer: 0.0,
        }
    }

    /// Get current target waypoint
    pub fn current_target(&self) -> Option<&IVec2> {
        self.waypoints.get(self.current_idx)
    }

    /// Move to next waypoint
    pub fn advance(&mut self) {
        match self.loop_type {
            PatrolLoopType::Loop => {
                self.current_idx = (self.current_idx + 1) % self.waypoints.len();
            }
            PatrolLoopType::PingPong => {
                let next = self.current_idx as i32 + self.direction;
                if next < 0 || next >= self.waypoints.len() as i32 {
                    self.direction *= -1;
                }
                self.current_idx = (self.current_idx as i32 + self.direction) as usize;
            }
            PatrolLoopType::Once => {
                if self.current_idx + 1 < self.waypoints.len() {
                    self.current_idx += 1;
                }
            }
        }
    }

    /// Update wait timer
    pub fn update_wait(&mut self, dt: f32) -> bool {
        if self.wait_timer > 0.0 {
            self.wait_timer -= dt;
            false
        } else {
            true
        }
    }

    /// Start waiting at waypoint
    pub fn start_wait(&mut self) {
        self.wait_timer = self.wait_time;
    }

    /// Check if currently waiting
    pub fn is_waiting(&self) -> bool {
        self.wait_timer > 0.0
    }
}

/// NPC schedule entry
#[derive(Debug, Clone)]
pub struct ScheduleEntry {
    /// Time of day (0-24)
    pub time: f32,
    /// Target position
    pub position: IVec2,
    /// Activity at this location
    pub activity: String,
}

/// NPC daily schedule
#[derive(Debug, Clone)]
pub struct NpcSchedule {
    /// Schedule entries sorted by time
    pub entries: Vec<ScheduleEntry>,
    /// Current entry index
    current_idx: usize,
}

impl NpcSchedule {
    /// Create a new schedule
    pub fn new(entries: Vec<ScheduleEntry>) -> Self {
        let mut entries = entries;
        entries.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap());

        Self {
            entries,
            current_idx: 0,
        }
    }

    /// Get current schedule entry
    pub fn current(&self) -> Option<&ScheduleEntry> {
        self.entries.get(self.current_idx)
    }

    /// Get next schedule entry
    pub fn next(&self) -> Option<&ScheduleEntry> {
        let next_idx = (self.current_idx + 1) % self.entries.len();
        self.entries.get(next_idx)
    }

    /// Update based on time of day
    pub fn update(&mut self, time_of_day: f32) {
        // Find the current schedule entry
        for (i, entry) in self.entries.iter().enumerate().rev() {
            if time_of_day >= entry.time {
                self.current_idx = i;
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pathfinding_basic() {
        let mut grid = PathGrid::new(10, 10);

        // Block some tiles
        grid.set_walkable(3, 3, false);
        grid.set_walkable(3, 4, false);
        grid.set_walkable(3, 5, false);

        let path = grid.find_path(IVec2::new(0, 0), IVec2::new(5, 5));
        assert!(path.is_some());

        let path = path.unwrap();
        assert!(!path.is_empty());
        assert_eq!(path.waypoints[0], IVec2::new(0, 0));
        assert_eq!(path.waypoints[path.len() - 1], IVec2::new(5, 5));
    }

    #[test]
    fn test_pathfinding_blocked() {
        let mut grid = PathGrid::new(5, 5);

        // Create wall
        for y in 0..5 {
            grid.set_walkable(2, y, false);
        }

        let path = grid.find_path(IVec2::new(0, 2), IVec2::new(4, 2));
        assert!(path.is_none());
    }

    #[test]
    fn test_patrol_ping_pong() {
        let waypoints = vec![IVec2::new(0, 0), IVec2::new(5, 0), IVec2::new(5, 5)];

        let mut patrol = PatrolPath::new(waypoints);
        patrol.loop_type = PatrolLoopType::PingPong;

        assert_eq!(patrol.current_target(), Some(&IVec2::new(0, 0)));

        patrol.advance();
        assert_eq!(patrol.current_target(), Some(&IVec2::new(5, 0)));

        patrol.advance();
        assert_eq!(patrol.current_target(), Some(&IVec2::new(5, 5)));

        patrol.advance(); // Should reverse
        assert_eq!(patrol.current_target(), Some(&IVec2::new(5, 0)));
    }
}
