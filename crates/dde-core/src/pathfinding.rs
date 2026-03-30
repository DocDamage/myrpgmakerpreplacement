//! Pathfinding & Navigation
//!
//! A* pathfinding on the tile grid with support for different terrain costs
//! and entity avoidance.

use glam::IVec2;
use serde::{Deserialize, Serialize};
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
    pub(crate) width: i32,
    /// Height of the grid
    pub(crate) height: i32,
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

    /// Get grid width
    pub fn width(&self) -> i32 {
        self.width
    }

    /// Get grid height
    pub fn height(&self) -> i32 {
        self.height
    }

    /// Check if coordinates are within grid bounds
    pub fn in_bounds(&self, x: i32, y: i32) -> bool {
        x >= 0 && x < self.width && y >= 0 && y < self.height
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

    /// Get all walkable positions in the grid
    pub fn walkable_positions(&self) -> Vec<IVec2> {
        let mut positions = Vec::new();
        for y in 0..self.height {
            for x in 0..self.width {
                if self.is_walkable(x, y) {
                    positions.push(IVec2::new(x, y));
                }
            }
        }
        positions
    }

    /// Get all unwalkable positions in the grid
    pub fn unwalkable_positions(&self) -> Vec<IVec2> {
        let mut positions = Vec::new();
        for y in 0..self.height {
            for x in 0..self.width {
                if !self.is_walkable(x, y) {
                    positions.push(IVec2::new(x, y));
                }
            }
        }
        positions
    }

    /// Get positions with higher movement costs
    pub fn high_cost_positions(&self, threshold: f32) -> Vec<(IVec2, f32)> {
        let mut positions = Vec::new();
        for y in 0..self.height {
            for x in 0..self.width {
                let cost = self.get_cost(x, y);
                if cost > threshold && cost.is_finite() {
                    positions.push((IVec2::new(x, y), cost));
                }
            }
        }
        positions
    }
}

/// Computed path
#[derive(Debug, Clone, PartialEq)]
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

/// Type of schedule entry
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ScheduleEntryType {
    /// Work at a location
    Work,
    /// Sleep at a location
    Sleep,
    /// Eat at a location
    Eat,
    /// Patrol a path
    Patrol,
    /// Idle at a location
    Idle,
    /// Custom activity
    Custom,
}

impl ScheduleEntryType {
    /// Get display name for UI
    pub fn display_name(&self) -> &'static str {
        match self {
            ScheduleEntryType::Work => "Work",
            ScheduleEntryType::Sleep => "Sleep",
            ScheduleEntryType::Eat => "Eat",
            ScheduleEntryType::Patrol => "Patrol",
            ScheduleEntryType::Idle => "Idle",
            ScheduleEntryType::Custom => "Custom",
        }
    }

    /// Get icon/emoji for UI
    pub fn icon(&self) -> &'static str {
        match self {
            ScheduleEntryType::Work => "💼",
            ScheduleEntryType::Sleep => "🛏️",
            ScheduleEntryType::Eat => "🍽️",
            ScheduleEntryType::Patrol => "👮",
            ScheduleEntryType::Idle => "☕",
            ScheduleEntryType::Custom => "⚙️",
        }
    }

    /// Get color for UI (RGB)
    pub fn color(&self) -> [u8; 3] {
        match self {
            ScheduleEntryType::Work => [100, 150, 255],    // Blue
            ScheduleEntryType::Sleep => [100, 100, 200],   // Dark Blue
            ScheduleEntryType::Eat => [255, 180, 100],     // Orange
            ScheduleEntryType::Patrol => [255, 100, 100],  // Red
            ScheduleEntryType::Idle => [150, 255, 150],    // Green
            ScheduleEntryType::Custom => [200, 200, 200],  // Gray
        }
    }
}

impl Default for ScheduleEntryType {
    fn default() -> Self {
        ScheduleEntryType::Idle
    }
}

/// Location reference for schedule entries
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ScheduleLocation {
    /// X coordinate on map
    pub x: i32,
    /// Y coordinate on map
    pub y: i32,
    /// Map ID
    pub map_id: u32,
}

impl ScheduleLocation {
    /// Create a new location
    pub fn new(x: i32, y: i32, map_id: u32) -> Self {
        Self { x, y, map_id }
    }

    /// Get position as IVec2
    pub fn position(&self) -> IVec2 {
        IVec2::new(self.x, self.y)
    }

    /// Create from IVec2 and map_id
    pub fn from_position(pos: IVec2, map_id: u32) -> Self {
        Self::new(pos.x, pos.y, map_id)
    }
}

impl Default for ScheduleLocation {
    fn default() -> Self {
        Self { x: 0, y: 0, map_id: 1 }
    }
}

/// NPC schedule entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleEntry {
    /// Entry ID (unique within schedule)
    pub id: u64,
    /// Start time (0-24 hours)
    pub start_time: f32,
    /// End time (0-24 hours)
    pub end_time: f32,
    /// Entry type
    pub entry_type: ScheduleEntryType,
    /// Location
    pub location: ScheduleLocation,
    /// Activity description
    pub activity: String,
    /// Dialogue to trigger when entry starts (optional)
    pub dialogue_trigger: Option<String>,
    /// Patrol path waypoints (only for Patrol type)
    pub patrol_waypoints: Vec<IVec2>,
    /// Whether NPC can be interrupted during this activity
    pub interruptible: bool,
}

impl ScheduleEntry {
    /// Create a new schedule entry
    pub fn new(id: u64, start_time: f32, end_time: f32, entry_type: ScheduleEntryType) -> Self {
        Self {
            id,
            start_time,
            end_time,
            entry_type,
            location: ScheduleLocation::default(),
            activity: entry_type.display_name().to_string(),
            dialogue_trigger: None,
            patrol_waypoints: Vec::new(),
            interruptible: true,
        }
    }

    /// Create a work entry
    pub fn work(id: u64, start: f32, end: f32, location: ScheduleLocation) -> Self {
        let mut entry = Self::new(id, start, end, ScheduleEntryType::Work);
        entry.location = location;
        entry
    }

    /// Create a sleep entry
    pub fn sleep(id: u64, start: f32, end: f32, location: ScheduleLocation) -> Self {
        let mut entry = Self::new(id, start, end, ScheduleEntryType::Sleep);
        entry.location = location;
        entry.interruptible = false;
        entry
    }

    /// Create an eat entry
    pub fn eat(id: u64, start: f32, end: f32, location: ScheduleLocation) -> Self {
        let mut entry = Self::new(id, start, end, ScheduleEntryType::Eat);
        entry.location = location;
        entry
    }

    /// Create an idle entry
    pub fn idle(id: u64, start: f32, end: f32, location: ScheduleLocation) -> Self {
        let mut entry = Self::new(id, start, end, ScheduleEntryType::Idle);
        entry.location = location;
        entry
    }

    /// Create a patrol entry
    pub fn patrol(id: u64, start: f32, end: f32, waypoints: Vec<IVec2>) -> Self {
        let mut entry = Self::new(id, start, end, ScheduleEntryType::Patrol);
        if let Some(first) = waypoints.first() {
            entry.location = ScheduleLocation::new(first.x, first.y, 1);
        }
        entry.patrol_waypoints = waypoints;
        entry
    }

    /// Get duration in hours
    pub fn duration(&self) -> f32 {
        if self.end_time >= self.start_time {
            self.end_time - self.start_time
        } else {
            // Wraps around midnight (e.g., sleep from 22:00 to 06:00)
            (24.0 - self.start_time) + self.end_time
        }
    }

    /// Check if a given time falls within this entry
    pub fn contains_time(&self, time: f32) -> bool {
        let time = time % 24.0;
        if self.end_time >= self.start_time {
            time >= self.start_time && time < self.end_time
        } else {
            // Wraps around midnight
            time >= self.start_time || time < self.end_time
        }
    }

    /// Set dialogue trigger
    pub fn with_dialogue(mut self, dialogue: impl Into<String>) -> Self {
        self.dialogue_trigger = Some(dialogue.into());
        self
    }

    /// Set activity description
    pub fn with_activity(mut self, activity: impl Into<String>) -> Self {
        self.activity = activity.into();
        self
    }
}

/// Day of the week
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Weekday {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

impl Default for Weekday {
    fn default() -> Self {
        Weekday::Monday
    }
}

impl Weekday {
    /// Get all weekdays
    pub fn all() -> [Weekday; 7] {
        [
            Weekday::Monday,
            Weekday::Tuesday,
            Weekday::Wednesday,
            Weekday::Thursday,
            Weekday::Friday,
            Weekday::Saturday,
            Weekday::Sunday,
        ]
    }

    /// Check if weekend
    pub fn is_weekend(&self) -> bool {
        matches!(self, Weekday::Saturday | Weekday::Sunday)
    }

    /// Check if weekday
    pub fn is_weekday(&self) -> bool {
        !self.is_weekend()
    }

    /// Get display name
    pub fn display_name(&self) -> &'static str {
        match self {
            Weekday::Monday => "Monday",
            Weekday::Tuesday => "Tuesday",
            Weekday::Wednesday => "Wednesday",
            Weekday::Thursday => "Thursday",
            Weekday::Friday => "Friday",
            Weekday::Saturday => "Saturday",
            Weekday::Sunday => "Sunday",
        }
    }

    /// Get short name
    pub fn short_name(&self) -> &'static str {
        match self {
            Weekday::Monday => "Mon",
            Weekday::Tuesday => "Tue",
            Weekday::Wednesday => "Wed",
            Weekday::Thursday => "Thu",
            Weekday::Friday => "Fri",
            Weekday::Saturday => "Sat",
            Weekday::Sunday => "Sun",
        }
    }
}

/// Weekly schedule for an NPC
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcSchedule {
    /// Schedule entries by weekday
    pub schedules: HashMap<Weekday, Vec<ScheduleEntry>>,
    /// Current weekday
    #[serde(skip)]
    current_day: Weekday,
    /// Current entry index for the current day
    #[serde(skip)]
    current_idx: usize,
    /// Whether to use the same schedule for all weekdays
    pub uniform_schedule: bool,
    /// Uniform schedule (used when uniform_schedule is true)
    pub uniform_entries: Vec<ScheduleEntry>,
}

impl NpcSchedule {
    /// Create a new empty schedule
    pub fn new() -> Self {
        let mut schedules = HashMap::new();
        for day in Weekday::all() {
            schedules.insert(day, Vec::new());
        }

        Self {
            schedules,
            current_day: Weekday::Monday,
            current_idx: 0,
            uniform_schedule: true,
            uniform_entries: Vec::new(),
        }
    }

    /// Create a new uniform schedule
    pub fn uniform(entries: Vec<ScheduleEntry>) -> Self {
        let mut schedule = Self::new();
        schedule.uniform_schedule = true;
        schedule.uniform_entries = entries;
        schedule
    }

    /// Get entries for a specific day
    pub fn entries_for_day(&self, day: Weekday) -> &[ScheduleEntry] {
        if self.uniform_schedule {
            &self.uniform_entries
        } else {
            self.schedules.get(&day).map(|v| v.as_slice()).unwrap_or(&[])
        }
    }

    /// Get mutable entries for a specific day
    pub fn entries_for_day_mut(&mut self, day: Weekday) -> &mut Vec<ScheduleEntry> {
        if self.uniform_schedule {
            &mut self.uniform_entries
        } else {
            // Use entry API to avoid borrow checker issues
            self.schedules.entry(day).or_default()
        }
    }

    /// Get current day's entries
    pub fn current_entries(&self) -> &[ScheduleEntry] {
        self.entries_for_day(self.current_day)
    }

    /// Get current schedule entry
    pub fn current(&self) -> Option<&ScheduleEntry> {
        self.current_entries().get(self.current_idx)
    }

    /// Get next schedule entry for current day
    pub fn next(&self) -> Option<&ScheduleEntry> {
        let entries = self.current_entries();
        if entries.is_empty() {
            return None;
        }
        let next_idx = (self.current_idx + 1) % entries.len();
        entries.get(next_idx)
    }

    /// Update based on time of day
    pub fn update(&mut self, time_of_day: f32) {
        let entries = if self.uniform_schedule {
            &self.uniform_entries
        } else {
            self.schedules.get(&self.current_day).unwrap_or(&self.uniform_entries)
        };

        // Find the current schedule entry based on time
        for (i, entry) in entries.iter().enumerate().rev() {
            if entry.contains_time(time_of_day) {
                self.current_idx = i;
                return;
            }
        }
        
        // If no entry found, default to first entry or maintain current
        if !entries.is_empty() {
            self.current_idx = 0;
        }
    }

    /// Set current day
    pub fn set_day(&mut self, day: Weekday) {
        self.current_day = day;
        self.current_idx = 0;
    }

    /// Get current day
    pub fn current_day(&self) -> Weekday {
        self.current_day
    }

    /// Add an entry to a specific day
    pub fn add_entry(&mut self, day: Weekday, entry: ScheduleEntry) {
        if self.uniform_schedule {
            self.uniform_entries.push(entry);
            self.uniform_entries.sort_by(|a, b| a.start_time.partial_cmp(&b.start_time).unwrap());
        } else {
            let entries = self.schedules.entry(day).or_default();
            entries.push(entry);
            entries.sort_by(|a, b| a.start_time.partial_cmp(&b.start_time).unwrap());
        }
    }

    /// Remove an entry from a specific day
    pub fn remove_entry(&mut self, day: Weekday, entry_id: u64) -> Option<ScheduleEntry> {
        let entries = if self.uniform_schedule {
            &mut self.uniform_entries
        } else {
            self.schedules.get_mut(&day)?
        };
        
        if let Some(pos) = entries.iter().position(|e| e.id == entry_id) {
            Some(entries.remove(pos))
        } else {
            None
        }
    }

    /// Copy schedule from one day to another
    pub fn copy_day(&mut self, from: Weekday, to: Weekday) {
        if self.uniform_schedule || from == to {
            return;
        }
        
        if let Some(entries) = self.schedules.get(&from).cloned() {
            self.schedules.insert(to, entries);
        }
    }

    /// Apply weekend/weekday pattern
    pub fn set_weekend_pattern(&mut self, weekday_schedule: Vec<ScheduleEntry>, weekend_schedule: Vec<ScheduleEntry>) {
        self.uniform_schedule = false;
        for day in Weekday::all() {
            if day.is_weekend() {
                self.schedules.insert(day, weekend_schedule.clone());
            } else {
                self.schedules.insert(day, weekday_schedule.clone());
            }
        }
    }

    /// Check for schedule conflicts (overlapping entries)
    pub fn check_conflicts(&self, day: Weekday) -> Vec<(u64, u64)> {
        let mut conflicts = Vec::new();
        let entries = self.entries_for_day(day);
        
        for (i, entry1) in entries.iter().enumerate() {
            for entry2 in entries.iter().skip(i + 1) {
                if Self::entries_overlap(entry1, entry2) {
                    conflicts.push((entry1.id, entry2.id));
                }
            }
        }
        
        conflicts
    }

    /// Check if two entries overlap
    fn entries_overlap(a: &ScheduleEntry, b: &ScheduleEntry) -> bool {
        a.contains_time(b.start_time) || b.contains_time(a.start_time)
    }

    /// Get next entry after the given time
    pub fn get_entry_at(&self, day: Weekday, time: f32) -> Option<&ScheduleEntry> {
        self.entries_for_day(day)
            .iter()
            .find(|e| e.contains_time(time))
    }

    /// Validate the entire schedule
    pub fn validate(&self) -> Vec<ScheduleValidationError> {
        let mut errors = Vec::new();
        
        let days_to_check = if self.uniform_schedule {
            vec![Weekday::Monday]
        } else {
            Weekday::all().to_vec()
        };
        
        for day in days_to_check {
            let entries = self.entries_for_day(day);
            
            // Check for gaps (optional - some NPCs might have gaps)
            // Check for overlaps
            for (i, entry1) in entries.iter().enumerate() {
                for entry2 in entries.iter().skip(i + 1) {
                    if Self::entries_overlap(entry1, entry2) {
                        errors.push(ScheduleValidationError::Overlap {
                            day,
                            entry1_id: entry1.id,
                            entry2_id: entry2.id,
                        });
                    }
                }
                
                // Validate entry times
                if entry1.start_time < 0.0 || entry1.start_time > 24.0 {
                    errors.push(ScheduleValidationError::InvalidTime {
                        day,
                        entry_id: entry1.id,
                        field: "start_time",
                    });
                }
                if entry1.end_time < 0.0 || entry1.end_time > 24.0 {
                    errors.push(ScheduleValidationError::InvalidTime {
                        day,
                        entry_id: entry1.id,
                        field: "end_time",
                    });
                }
            }
        }
        
        errors
    }
}

impl Default for NpcSchedule {
    fn default() -> Self {
        Self::new()
    }
}

/// Schedule validation error
#[derive(Debug, Clone)]
pub enum ScheduleValidationError {
    /// Two entries overlap
    Overlap { day: Weekday, entry1_id: u64, entry2_id: u64 },
    /// Invalid time value
    InvalidTime { day: Weekday, entry_id: u64, field: &'static str },
    /// Missing location for entry type that requires it
    MissingLocation { day: Weekday, entry_id: u64 },
    /// Missing patrol waypoints for patrol entry
    MissingPatrolWaypoints { day: Weekday, entry_id: u64 },
}

impl std::fmt::Display for ScheduleValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScheduleValidationError::Overlap { day, entry1_id, entry2_id } => {
                write!(f, "Overlap on {} between entries {} and {}", day.display_name(), entry1_id, entry2_id)
            }
            ScheduleValidationError::InvalidTime { day, entry_id, field } => {
                write!(f, "Invalid {} for entry {} on {}", field, entry_id, day.display_name())
            }
            ScheduleValidationError::MissingLocation { day, entry_id } => {
                write!(f, "Missing location for entry {} on {}", entry_id, day.display_name())
            }
            ScheduleValidationError::MissingPatrolWaypoints { day, entry_id } => {
                write!(f, "Missing patrol waypoints for entry {} on {}", entry_id, day.display_name())
            }
        }
    }
}

impl std::error::Error for ScheduleValidationError {}

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
