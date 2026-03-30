//! Advanced Performance Profiler
//!
//! Hierarchical profiling with GPU timing, memory tracking, and detailed metrics.

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Advanced profiler with hierarchical timing and memory tracking
pub struct AdvancedProfiler {
    /// Whether profiling is enabled
    enabled: bool,
    /// Root profiling nodes
    root_nodes: Vec<ProfileNode>,
    /// Node stack for hierarchical profiling (stores indices)
    node_stack: Vec<(usize, usize)>, // (root_index, child_index)
    /// Frame history
    frame_history: Vec<FrameMetrics>,
    /// Current frame metrics
    current_frame: FrameMetrics,
    /// Maximum history size
    max_history: usize,
    /// Frame counter
    frame_count: u64,
    /// GPU profiler
    gpu_profiler: Option<GpuProfiler>,
    /// Memory tracker
    memory_tracker: MemoryTracker,
    /// Current section start times
    section_starts: HashMap<String, Instant>,
}

/// A profiling node representing a timed section
#[derive(Debug, Clone)]
pub struct ProfileNode {
    /// Node name
    pub name: String,
    /// Parent node name (None for root)
    pub parent: Option<String>,
    /// Total elapsed time
    pub elapsed: Duration,
    /// Number of calls
    pub call_count: u64,
    /// Child nodes
    pub children: Vec<ProfileNode>,
    /// Start time (when active)
    #[doc(hidden)]
    pub start_time: Option<Instant>,
}

impl ProfileNode {
    /// Create a new profile node
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            parent: None,
            elapsed: Duration::ZERO,
            call_count: 0,
            children: Vec::new(),
            start_time: None,
        }
    }

    /// Create a new child node
    pub fn new_child(name: impl Into<String>, parent: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            parent: Some(parent.into()),
            elapsed: Duration::ZERO,
            call_count: 0,
            children: Vec::new(),
            start_time: None,
        }
    }

    /// Start timing this node
    pub fn start(&mut self) {
        self.start_time = Some(Instant::now());
        self.call_count += 1;
    }

    /// Stop timing this node
    pub fn stop(&mut self) {
        if let Some(start) = self.start_time {
            self.elapsed += start.elapsed();
            self.start_time = None;
        }
    }

    /// Get elapsed time in milliseconds
    pub fn elapsed_ms(&self) -> f64 {
        self.elapsed.as_secs_f64() * 1000.0
    }

    /// Get average time per call in milliseconds
    pub fn average_ms(&self) -> f64 {
        if self.call_count > 0 {
            self.elapsed_ms() / self.call_count as f64
        } else {
            0.0
        }
    }

    /// Reset node statistics
    pub fn reset(&mut self) {
        self.elapsed = Duration::ZERO;
        self.call_count = 0;
        self.start_time = None;
        for child in &mut self.children {
            child.reset();
        }
    }

    /// Find a child node by name
    pub fn find_child(&self, name: &str) -> Option<&ProfileNode> {
        self.children.iter().find(|c| c.name == name)
    }

    /// Find a child node by name (mutable)
    pub fn find_child_mut(&mut self, name: &str) -> Option<&mut ProfileNode> {
        self.children.iter_mut().find(|c| c.name == name)
    }
}

/// Frame-level metrics
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct FrameMetrics {
    /// Frame number
    pub frame: u64,
    /// Total frame time
    pub total_time: Duration,
    /// CPU time
    pub cpu_time: Duration,
    /// GPU time (if available)
    pub gpu_time: Option<Duration>,
    /// Memory usage at end of frame
    pub memory_usage: MemorySnapshot,
    /// Entity count
    pub entity_count: usize,
    /// System-specific metrics
    pub systems: HashMap<String, SystemMetrics>,
}

/// System-level metrics
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct SystemMetrics {
    /// System name
    pub name: String,
    /// Time spent in system
    pub elapsed: Duration,
    /// Number of entities processed
    pub entities_processed: usize,
    /// Custom counters
    pub counters: HashMap<String, u64>,
}

/// GPU profiler stub
#[derive(Debug, Clone)]
pub struct GpuProfiler {
    /// Whether GPU profiling is available
    available: bool,
}

impl GpuProfiler {
    /// Create a new GPU profiler
    pub fn new() -> Self {
        Self { available: false }
    }

    /// Check if GPU profiling is available
    pub fn is_available(&self) -> bool {
        self.available
    }

    /// Begin GPU timing for a section
    pub fn begin_section(&mut self, _name: &str) {}

    /// End GPU timing for a section
    pub fn end_section(&mut self, _name: &str) {}

    /// Collect GPU times
    pub fn collect_times(&self) -> HashMap<String, Duration> {
        HashMap::new()
    }
}

impl Default for GpuProfiler {
    fn default() -> Self {
        Self::new()
    }
}

/// Memory snapshot
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct MemorySnapshot {
    /// Total allocated memory (bytes)
    pub total_allocated: usize,
    /// Used memory (bytes)
    pub used_memory: usize,
    /// ECS component memory (bytes)
    pub ecs_memory: usize,
    /// Lua memory (bytes)
    pub lua_memory: usize,
    /// Asset memory (bytes)
    pub asset_memory: usize,
    /// Texture memory (bytes)
    pub texture_memory: usize,
}

impl MemorySnapshot {
    /// Get total memory in MB
    pub fn total_mb(&self) -> f64 {
        self.total_allocated as f64 / (1024.0 * 1024.0)
    }

    /// Get used memory in MB
    pub fn used_mb(&self) -> f64 {
        self.used_memory as f64 / (1024.0 * 1024.0)
    }

    /// Format for display
    pub fn format(&self) -> String {
        format!(
            "Total: {:.1} MB | ECS: {:.1} MB | Lua: {:.1} MB | Assets: {:.1} MB",
            self.total_mb(),
            self.ecs_memory as f64 / (1024.0 * 1024.0),
            self.lua_memory as f64 / (1024.0 * 1024.0),
            self.asset_memory as f64 / (1024.0 * 1024.0)
        )
    }
}

/// Memory tracker
#[derive(Debug, Clone)]
pub struct MemoryTracker {
    /// Current memory snapshot
    current: MemorySnapshot,
    /// Peak memory usage
    peak: MemorySnapshot,
    /// Memory history
    history: Vec<MemorySnapshot>,
}

impl MemoryTracker {
    /// Create a new memory tracker
    pub fn new() -> Self {
        Self {
            current: MemorySnapshot::default(),
            peak: MemorySnapshot::default(),
            history: Vec::with_capacity(60),
        }
    }

    /// Update memory snapshot
    pub fn update(&mut self, snapshot: MemorySnapshot) {
        self.current = snapshot.clone();

        // Update peaks
        if snapshot.total_allocated > self.peak.total_allocated {
            self.peak.total_allocated = snapshot.total_allocated;
        }
        if snapshot.used_memory > self.peak.used_memory {
            self.peak.used_memory = snapshot.used_memory;
        }

        // Add to history
        self.history.push(snapshot);
        if self.history.len() > 60 {
            self.history.remove(0);
        }
    }

    /// Get current snapshot
    pub fn current(&self) -> &MemorySnapshot {
        &self.current
    }

    /// Get peak snapshot
    pub fn peak(&self) -> &MemorySnapshot {
        &self.peak
    }

    /// Get memory history
    pub fn history(&self) -> &[MemorySnapshot] {
        &self.history
    }

    /// Calculate memory growth rate (MB/s)
    pub fn growth_rate(&self) -> f64 {
        if self.history.len() < 2 {
            return 0.0;
        }

        let first = &self.history[0];
        let last = &self.history[self.history.len() - 1];
        let delta = last.total_allocated as f64 - first.total_allocated as f64;
        delta / (1024.0 * 1024.0) // Convert to MB
    }
}

impl Default for MemoryTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl AdvancedProfiler {
    /// Create a new advanced profiler
    pub fn new() -> Self {
        Self {
            enabled: false,
            root_nodes: Vec::new(),
            node_stack: Vec::new(),
            frame_history: Vec::with_capacity(120),
            current_frame: FrameMetrics::default(),
            max_history: 120,
            frame_count: 0,
            gpu_profiler: Some(GpuProfiler::new()),
            memory_tracker: MemoryTracker::new(),
            section_starts: HashMap::new(),
        }
    }

    /// Enable profiling
    pub fn enable(&mut self) {
        self.enabled = true;
        tracing::info!("Advanced profiler enabled");
    }

    /// Disable profiling
    pub fn disable(&mut self) {
        self.enabled = false;
        tracing::info!("Advanced profiler disabled");
    }

    /// Toggle profiling
    pub fn toggle(&mut self) {
        if self.enabled {
            self.disable();
        } else {
            self.enable();
        }
    }

    /// Check if enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Begin a new frame
    pub fn begin_frame(&mut self) {
        if !self.enabled {
            return;
        }

        self.frame_count += 1;
        self.current_frame = FrameMetrics {
            frame: self.frame_count,
            ..Default::default()
        };

        // Clear previous frame nodes
        self.root_nodes.clear();
        self.node_stack.clear();
        self.section_starts.clear();
    }

    /// End the current frame
    pub fn end_frame(&mut self) {
        if !self.enabled {
            return;
        }

        // Update frame metrics
        self.current_frame.systems = self.collect_system_metrics();

        // Add to history
        self.frame_history.push(self.current_frame.clone());
        if self.frame_history.len() > self.max_history {
            self.frame_history.remove(0);
        }
    }

    /// Begin a profiling scope
    pub fn begin_scope(&mut self, name: &str) {
        if !self.enabled {
            return;
        }

        // Record start time
        self.section_starts.insert(name.to_string(), Instant::now());

        // Create or find node
        if let Some((root_idx, _)) = self.node_stack.last().copied() {
            // Add as child
            let parent = &mut self.root_nodes[root_idx];
            if let Some(child) = parent.find_child_mut(name) {
                child.start();
            } else {
                let mut node = ProfileNode::new_child(name, &parent.name);
                node.start();
                parent.children.push(node);
            }
            let child_idx = parent.children.len() - 1;
            self.node_stack.push((root_idx, child_idx));
        } else {
            // Add as root
            let mut node = ProfileNode::new(name);
            node.start();
            self.root_nodes.push(node);
            let root_idx = self.root_nodes.len() - 1;
            self.node_stack.push((root_idx, 0));
        }
    }

    /// End a profiling scope
    pub fn end_scope(&mut self, name: &str) {
        if !self.enabled {
            return;
        }

        // Pop from stack
        if let Some((root_idx, child_idx)) = self.node_stack.pop() {
            if child_idx < self.root_nodes[root_idx].children.len() {
                self.root_nodes[root_idx].children[child_idx].stop();
            }
        }

        // Record elapsed
        if let Some(start) = self.section_starts.remove(name) {
            let elapsed = start.elapsed();
            self.current_frame.total_time += elapsed;
        }
    }

    /// Begin GPU section
    pub fn begin_gpu_section(&mut self, name: &str) {
        if let Some(ref mut gpu) = self.gpu_profiler {
            gpu.begin_section(name);
        }
    }

    /// End GPU section
    pub fn end_gpu_section(&mut self, name: &str) {
        if let Some(ref mut gpu) = self.gpu_profiler {
            gpu.end_section(name);
        }
    }

    /// Record memory snapshot
    pub fn record_memory(&mut self, snapshot: MemorySnapshot) {
        if !self.enabled {
            return;
        }

        self.memory_tracker.update(snapshot);
        self.current_frame.memory_usage = self.memory_tracker.current().clone();
    }

    /// Record entity count
    pub fn record_entity_count(&mut self, count: usize) {
        if !self.enabled {
            return;
        }

        self.current_frame.entity_count = count;
    }

    /// Get root nodes
    pub fn root_nodes(&self) -> &[ProfileNode] {
        &self.root_nodes
    }

    /// Get frame history
    pub fn frame_history(&self) -> &[FrameMetrics] {
        &self.frame_history
    }

    /// Get memory tracker
    pub fn memory_tracker(&self) -> &MemoryTracker {
        &self.memory_tracker
    }

    /// Get average frame time
    pub fn average_frame_time(&self, frames: usize) -> Duration {
        let count = frames.min(self.frame_history.len());
        if count == 0 {
            return Duration::ZERO;
        }

        let sum: Duration = self
            .frame_history
            .iter()
            .rev()
            .take(count)
            .map(|f| f.total_time)
            .sum();

        sum / count as u32
    }

    /// Get current FPS
    pub fn fps(&self) -> f64 {
        let avg = self.average_frame_time(60);
        if avg.as_secs_f64() > 0.0 {
            1.0 / avg.as_secs_f64()
        } else {
            0.0
        }
    }

    /// Get 99th percentile frame time
    pub fn percentile_frame_time(&self, percentile: f64) -> Duration {
        if self.frame_history.is_empty() {
            return Duration::ZERO;
        }

        let mut times: Vec<Duration> = self.frame_history.iter().map(|f| f.total_time).collect();
        times.sort();

        let index = ((times.len() as f64 * percentile / 100.0) as usize).min(times.len() - 1);
        times[index]
    }

    /// Collect system metrics from profile nodes
    fn collect_system_metrics(&self) -> HashMap<String, SystemMetrics> {
        let mut systems = HashMap::new();

        for root in &self.root_nodes {
            self.collect_node_metrics(root, &mut systems);
        }

        systems
    }

    fn collect_node_metrics(
        &self,
        node: &ProfileNode,
        systems: &mut HashMap<String, SystemMetrics>,
    ) {
        systems.insert(
            node.name.clone(),
            SystemMetrics {
                name: node.name.clone(),
                elapsed: node.elapsed,
                entities_processed: 0,
                counters: HashMap::new(),
            },
        );

        for child in &node.children {
            self.collect_node_metrics(child, systems);
        }
    }

    /// Export profiling data to JSON
    pub fn export_json(&self) -> String {
        let data = ProfilingData {
            frames: self.frame_history.clone(),
            memory_peaks: self.memory_tracker.peak().clone(),
            total_frames: self.frame_count,
        };

        serde_json::to_string_pretty(&data).unwrap_or_default()
    }
}

impl Default for AdvancedProfiler {
    fn default() -> Self {
        Self::new()
    }
}

/// Serializable profiling data
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProfilingData {
    pub frames: Vec<FrameMetrics>,
    pub memory_peaks: MemorySnapshot,
    pub total_frames: u64,
}

/// RAII guard for profiling scopes
pub struct ScopeGuard<'a> {
    profiler: &'a mut AdvancedProfiler,
    name: String,
}

impl<'a> ScopeGuard<'a> {
    /// Create a new scope guard
    pub fn new(profiler: &'a mut AdvancedProfiler, name: &str) -> Self {
        profiler.begin_scope(name);
        Self {
            profiler,
            name: name.to_string(),
        }
    }
}

impl<'a> Drop for ScopeGuard<'a> {
    fn drop(&mut self) {
        self.profiler.end_scope(&self.name);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_advanced_profiler_creation() {
        let profiler = AdvancedProfiler::new();
        assert!(!profiler.is_enabled());
    }

    #[test]
    fn test_profiler_toggle() {
        let mut profiler = AdvancedProfiler::new();
        assert!(!profiler.is_enabled());

        profiler.toggle();
        assert!(profiler.is_enabled());

        profiler.toggle();
        assert!(!profiler.is_enabled());
    }

    #[test]
    fn test_profile_node() {
        let mut node = ProfileNode::new("test");
        assert_eq!(node.elapsed_ms(), 0.0);
        assert_eq!(node.call_count, 0);

        node.start();
        std::thread::sleep(Duration::from_millis(1));
        node.stop();

        assert!(node.elapsed_ms() > 0.0);
        assert_eq!(node.call_count, 1);
    }

    #[test]
    fn test_memory_snapshot() {
        let snapshot = MemorySnapshot {
            total_allocated: 1024 * 1024 * 100, // 100 MB
            used_memory: 1024 * 1024 * 80,
            ecs_memory: 1024 * 1024 * 30,
            lua_memory: 1024 * 1024 * 10,
            asset_memory: 1024 * 1024 * 40,
            texture_memory: 1024 * 1024 * 20,
        };

        assert_eq!(snapshot.total_mb(), 100.0);
        assert_eq!(snapshot.used_mb(), 80.0);
    }

    #[test]
    fn test_memory_tracker() {
        let mut tracker = MemoryTracker::new();

        let snapshot = MemorySnapshot {
            total_allocated: 1024 * 1024 * 100,
            ..Default::default()
        };

        tracker.update(snapshot);
        assert_eq!(tracker.current().total_mb(), 100.0);
        assert_eq!(tracker.peak().total_mb(), 100.0);

        // Update with higher memory
        let snapshot2 = MemorySnapshot {
            total_allocated: 1024 * 1024 * 150,
            ..Default::default()
        };
        tracker.update(snapshot2);
        assert_eq!(tracker.peak().total_mb(), 150.0);
    }

    #[test]
    fn test_system_metrics() {
        let metrics = SystemMetrics {
            name: "physics".to_string(),
            elapsed: Duration::from_millis(5),
            entities_processed: 100,
            counters: {
                let mut c = HashMap::new();
                c.insert("collisions".to_string(), 50);
                c
            },
        };

        assert_eq!(metrics.name, "physics");
        assert_eq!(metrics.entities_processed, 100);
    }

    #[test]
    fn test_frame_metrics() {
        let frame = FrameMetrics {
            frame: 1,
            total_time: Duration::from_millis(16),
            cpu_time: Duration::from_millis(10),
            gpu_time: Some(Duration::from_millis(8)),
            ..Default::default()
        };

        assert_eq!(frame.frame, 1);
        assert!(frame.gpu_time.is_some());
    }
}
