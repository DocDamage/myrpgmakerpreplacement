//! Enhanced Profiler Features
//!
//! Extended profiling capabilities including:
//! - Detailed section breakdowns with min/max/avg tracking
//! - 99th percentile and variance calculations
//! - Bottleneck identification
//! - Optimization suggestions
//! - Memory profiling by category
//! - Entity counting per system
//! - Time-series data management

use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

/// Enhanced profiler with comprehensive metrics
#[derive(Debug, Clone)]
pub struct EnhancedProfiler {
    /// Whether profiling is enabled
    enabled: bool,
    /// Current frame number
    frame_count: u64,
    /// Frame history
    frame_history: VecDeque<EnhancedFrameMetrics>,
    /// Maximum history size (default: 3600 frames = 60 seconds at 60fps)
    max_history: usize,
    /// Section timings for current frame
    section_timings: HashMap<String, SectionTiming>,
    /// Active section stack
    section_stack: Vec<(String, Instant)>,
    /// Memory snapshots history
    memory_history: VecDeque<MemoryBreakdown>,
    /// Entity counts per category
    entity_counts: EntityCounts,
    /// Budget configuration
    budget_config: BudgetConfiguration,
    /// Recording state
    recording_state: RecordingState,
    /// Current frame start time
    frame_start: Option<Instant>,
}

/// Enhanced frame metrics
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct EnhancedFrameMetrics {
    /// Frame number
    pub frame: u64,
    /// Timestamp
    pub timestamp: f64,
    /// Total frame time
    pub total_time_ms: f64,
    /// CPU time
    pub cpu_time_ms: f64,
    /// GPU time (if available)
    pub gpu_time_ms: Option<f64>,
    /// FPS
    pub fps: f32,
    /// Per-section timings
    pub sections: HashMap<String, SectionMetrics>,
    /// Memory at end of frame
    pub memory: MemoryBreakdown,
    /// Entity counts
    pub entities: EntityCounts,
}

/// Section timing information
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct SectionMetrics {
    /// Section name
    pub name: String,
    /// Current time (ms)
    pub current_ms: f64,
    /// Average time (ms)
    pub average_ms: f64,
    /// Minimum time (ms)
    pub min_ms: f64,
    /// Maximum time (ms)
    pub max_ms: f64,
    /// Total calls
    pub call_count: u64,
    /// Budget status
    pub budget_status: BudgetStatus,
    /// Child sections
    pub children: Vec<SectionMetrics>,
}

/// Section timing for internal tracking
#[derive(Debug, Clone)]
struct SectionTiming {
    name: String,
    parent: Option<String>,
    current_elapsed: Duration,
    total_elapsed: Duration,
    min_elapsed: Duration,
    max_elapsed: Duration,
    call_count: u64,
    start_time: Option<Instant>,
    children: Vec<String>,
}

impl SectionTiming {
    fn new(name: String, parent: Option<String>) -> Self {
        Self {
            name,
            parent,
            current_elapsed: Duration::ZERO,
            total_elapsed: Duration::ZERO,
            min_elapsed: Duration::MAX,
            max_elapsed: Duration::ZERO,
            call_count: 0,
            start_time: None,
            children: Vec::new(),
        }
    }

    fn start(&mut self) {
        self.start_time = Some(Instant::now());
        self.call_count += 1;
    }

    fn stop(&mut self) {
        if let Some(start) = self.start_time {
            let elapsed = start.elapsed();
            self.current_elapsed = elapsed;
            self.total_elapsed += elapsed;
            if elapsed < self.min_elapsed {
                self.min_elapsed = elapsed;
            }
            if elapsed > self.max_elapsed {
                self.max_elapsed = elapsed;
            }
            self.start_time = None;
        }
    }

    fn average_ms(&self) -> f64 {
        if self.call_count > 0 {
            self.total_elapsed.as_secs_f64() * 1000.0 / self.call_count as f64
        } else {
            0.0
        }
    }

    fn to_metrics(&self, budget: f64) -> SectionMetrics {
        let current_ms = self.current_elapsed.as_secs_f64() * 1000.0;
        let avg_ms = self.average_ms();
        let min_ms = if self.min_elapsed == Duration::MAX {
            0.0
        } else {
            self.min_elapsed.as_secs_f64() * 1000.0
        };
        let max_ms = self.max_elapsed.as_secs_f64() * 1000.0;

        let ratio = avg_ms / budget;
        let budget_status = if budget <= 0.0 {
            BudgetStatus::Unknown
        } else if ratio <= 0.8 {
            BudgetStatus::Good
        } else if ratio <= 1.0 {
            BudgetStatus::Warning
        } else {
            BudgetStatus::OverBudget
        };

        SectionMetrics {
            name: self.name.clone(),
            current_ms,
            average_ms: avg_ms,
            min_ms,
            max_ms,
            call_count: self.call_count,
            budget_status,
            children: Vec::new(),
        }
    }
}

// BudgetStatus is imported from the parent module (mod.rs)
use super::BudgetStatus;

/// Budget configuration
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct BudgetConfiguration {
    /// Target FPS
    pub target_fps: f32,
    /// Frame time budget (calculated from target_fps)
    pub frame_budget_ms: f64,
    /// Simulation budget (ms)
    pub simulation_budget_ms: f64,
    /// Render budget (ms)
    pub render_budget_ms: f64,
    /// Audio budget (ms)
    pub audio_budget_ms: f64,
    /// Script budget (ms)
    pub script_budget_ms: f64,
    /// UI budget (ms)
    pub ui_budget_ms: f64,
    /// Pathfinding budget (ms)
    pub pathfinding_budget_ms: f64,
    /// Warning threshold (0.0-1.0, default 0.8)
    pub warning_threshold: f64,
    /// Critical threshold (0.0-1.0, default 1.0)
    pub critical_threshold: f64,
}

impl Default for BudgetConfiguration {
    fn default() -> Self {
        Self {
            target_fps: 60.0,
            frame_budget_ms: 16.67,
            simulation_budget_ms: 3.0,
            render_budget_ms: 4.0,
            audio_budget_ms: 1.0,
            script_budget_ms: 2.0,
            ui_budget_ms: 3.0,
            pathfinding_budget_ms: 1.0,
            warning_threshold: 0.8,
            critical_threshold: 1.0,
        }
    }
}

/// Memory breakdown by category
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct MemoryBreakdown {
    /// Total RAM usage (bytes)
    pub total_ram: usize,
    /// ECS World memory (bytes)
    pub ecs_world: usize,
    /// Assets memory (bytes)
    pub assets: usize,
    /// Texture memory (bytes)
    pub textures: usize,
    /// Audio memory (bytes)
    pub audio: usize,
    /// Scripts memory (bytes)
    pub scripts: usize,
    /// Database cache memory (bytes)
    pub db_cache: usize,
    /// Other memory (bytes)
    pub other: usize,
    /// VRAM usage (bytes, if available)
    pub vram: Option<usize>,
}

impl MemoryBreakdown {
    /// Get total in MB
    pub fn total_mb(&self) -> f64 {
        self.total_ram as f64 / (1024.0 * 1024.0)
    }

    /// Get ECS memory in MB
    pub fn ecs_mb(&self) -> f64 {
        self.ecs_world as f64 / (1024.0 * 1024.0)
    }

    /// Get assets memory in MB
    pub fn assets_mb(&self) -> f64 {
        self.assets as f64 / (1024.0 * 1024.0)
    }

    /// Get textures memory in MB
    pub fn textures_mb(&self) -> f64 {
        self.textures as f64 / (1024.0 * 1024.0)
    }

    /// Get audio memory in MB
    pub fn audio_mb(&self) -> f64 {
        self.audio as f64 / (1024.0 * 1024.0)
    }

    /// Get scripts memory in MB
    pub fn scripts_mb(&self) -> f64 {
        self.scripts as f64 / (1024.0 * 1024.0)
    }

    /// Get DB cache memory in MB
    pub fn db_cache_mb(&self) -> f64 {
        self.db_cache as f64 / (1024.0 * 1024.0)
    }

    /// Get VRAM in MB
    pub fn vram_mb(&self) -> Option<f64> {
        self.vram.map(|v| v as f64 / (1024.0 * 1024.0))
    }
}

/// Entity counts by category
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct EntityCounts {
    /// Total entities
    pub total: usize,
    /// NPC entities
    pub npcs: usize,
    /// Item entities
    pub items: usize,
    /// Projectile entities
    pub projectiles: usize,
    /// Effect entities
    pub effects: usize,
    /// Player entities
    pub players: usize,
    /// Static entities
    pub static_entities: usize,
}

/// Recording state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordingState {
    Recording,
    Paused,
}

impl Default for RecordingState {
    fn default() -> Self {
        RecordingState::Recording
    }
}

/// Bottleneck identification result
#[derive(Debug, Clone)]
pub struct Bottleneck {
    /// System/section name
    pub name: String,
    /// Severity (0.0-1.0, higher is worse)
    pub severity: f64,
    /// Average time over budget
    pub over_budget_ms: f64,
    /// Percentage of total frame time
    pub percentage_of_frame: f64,
}

/// Optimization suggestion
#[derive(Debug, Clone)]
pub struct OptimizationSuggestion {
    /// Title
    pub title: String,
    /// Description
    pub description: String,
    /// Priority (0.0-1.0)
    pub priority: f64,
    /// Category
    pub category: SuggestionCategory,
}

/// Suggestion category
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuggestionCategory {
    Performance,
    Memory,
    Rendering,
    Audio,
    Script,
    ECS,
}

impl EnhancedProfiler {
    /// Create a new enhanced profiler
    pub fn new() -> Self {
        Self {
            enabled: false,
            frame_count: 0,
            frame_history: VecDeque::with_capacity(3600),
            max_history: 3600,
            section_timings: HashMap::new(),
            section_stack: Vec::new(),
            memory_history: VecDeque::with_capacity(3600),
            entity_counts: EntityCounts::default(),
            budget_config: BudgetConfiguration::default(),
            recording_state: RecordingState::Recording,
            frame_start: None,
        }
    }

    /// Enable profiling
    pub fn enable(&mut self) {
        self.enabled = true;
        tracing::info!("Enhanced profiler enabled");
    }

    /// Disable profiling
    pub fn disable(&mut self) {
        self.enabled = false;
        tracing::info!("Enhanced profiler disabled");
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

    /// Pause recording
    pub fn pause(&mut self) {
        self.recording_state = RecordingState::Paused;
    }

    /// Resume recording
    pub fn resume(&mut self) {
        self.recording_state = RecordingState::Recording;
    }

    /// Toggle pause
    pub fn toggle_pause(&mut self) {
        self.recording_state = match self.recording_state {
            RecordingState::Recording => RecordingState::Paused,
            RecordingState::Paused => RecordingState::Recording,
        };
    }

    /// Check if recording
    pub fn is_recording(&self) -> bool {
        matches!(self.recording_state, RecordingState::Recording)
    }

    /// Begin a new frame
    pub fn begin_frame(&mut self) {
        if !self.enabled {
            return;
        }

        self.frame_count += 1;
        self.frame_start = Some(Instant::now());
        self.section_timings.clear();
        self.section_stack.clear();
    }

    /// End the current frame
    pub fn end_frame(&mut self) {
        if !self.enabled {
            return;
        }

        if !self.is_recording() {
            return;
        }

        let total_time = self.frame_start.map(|s| s.elapsed()).unwrap_or_default();
        let total_time_ms = total_time.as_secs_f64() * 1000.0;
        let fps = if total_time_ms > 0.0 {
            1000.0 / total_time_ms as f32
        } else {
            0.0
        };

        // Collect section metrics
        let mut sections = HashMap::new();
        for (name, timing) in &self.section_timings {
            let budget = self.get_budget_for_section(name);
            sections.insert(name.clone(), timing.to_metrics(budget));
        }

        // Get latest memory
        let memory = self.memory_history.back().cloned().unwrap_or_default();

        let frame = EnhancedFrameMetrics {
            frame: self.frame_count,
            timestamp: self.frame_count as f64 / 60.0, // Approximate timestamp at 60fps
            total_time_ms,
            cpu_time_ms: total_time_ms,
            gpu_time_ms: None,
            fps,
            sections,
            memory,
            entities: self.entity_counts.clone(),
        };

        self.frame_history.push_back(frame);
        if self.frame_history.len() > self.max_history {
            self.frame_history.pop_front();
        }
    }

    /// Begin timing a section
    pub fn begin_section(&mut self, name: &str) {
        if !self.enabled {
            return;
        }

        let parent = self.section_stack.last().map(|(n, _)| n.clone());

        let timing = self
            .section_timings
            .entry(name.to_string())
            .or_insert_with(|| SectionTiming::new(name.to_string(), parent.clone()));

        timing.start();
        self.section_stack.push((name.to_string(), Instant::now()));

        // Track parent-child relationship
        if let Some(parent_name) = parent {
            if let Some(parent_timing) = self.section_timings.get_mut(&parent_name) {
                if !parent_timing.children.contains(&name.to_string()) {
                    parent_timing.children.push(name.to_string());
                }
            }
        }
    }

    /// End timing a section
    pub fn end_section(&mut self, name: &str) {
        if !self.enabled {
            return;
        }

        // Pop from stack
        if let Some((popped_name, _)) = self.section_stack.pop() {
            if popped_name != name {
                // Mismatched sections - push back
                self.section_stack.push((popped_name, Instant::now()));
            }
        }

        // Stop timing
        if let Some(timing) = self.section_timings.get_mut(name) {
            timing.stop();
        }
    }

    /// Record memory breakdown
    pub fn record_memory(&mut self, memory: MemoryBreakdown) {
        if !self.enabled {
            return;
        }

        self.memory_history.push_back(memory);
        if self.memory_history.len() > self.max_history {
            self.memory_history.pop_front();
        }
    }

    /// Record entity counts
    pub fn record_entities(&mut self, counts: EntityCounts) {
        if !self.enabled {
            return;
        }

        self.entity_counts = counts;
    }

    /// Get budget for a section
    fn get_budget_for_section(&self, section: &str) -> f64 {
        match section {
            "simulation" | "tick" => self.budget_config.simulation_budget_ms,
            "render" | "rendering" => self.budget_config.render_budget_ms,
            "audio" => self.budget_config.audio_budget_ms,
            "script" | "scripts" | "lua" => self.budget_config.script_budget_ms,
            "ui" => self.budget_config.ui_budget_ms,
            "pathfinding" => self.budget_config.pathfinding_budget_ms,
            "frame" | "total" => self.budget_config.frame_budget_ms,
            _ => self.budget_config.frame_budget_ms, // Default to frame budget
        }
    }

    /// Get budget configuration
    pub fn budget_config(&self) -> &BudgetConfiguration {
        &self.budget_config
    }

    /// Set budget configuration
    pub fn set_budget_config(&mut self, config: BudgetConfiguration) {
        self.budget_config = config;
    }

    /// Get frame history
    pub fn frame_history(&self) -> &VecDeque<EnhancedFrameMetrics> {
        &self.frame_history
    }

    /// Get memory history
    pub fn memory_history(&self) -> &VecDeque<MemoryBreakdown> {
        &self.memory_history
    }

    /// Get current entity counts
    pub fn entity_counts(&self) -> &EntityCounts {
        &self.entity_counts
    }

    /// Get section timings
    pub fn section_timings(&self) -> &HashMap<String, SectionTiming> {
        &self.section_timings
    }

    /// Calculate 99th percentile frame time
    pub fn percentile_frame_time(&self, percentile: f64) -> f64 {
        if self.frame_history.is_empty() {
            return 0.0;
        }

        let mut times: Vec<f64> = self
            .frame_history
            .iter()
            .map(|f| f.total_time_ms)
            .collect();
        times.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let index = ((times.len() as f64 * percentile / 100.0) as usize).min(times.len() - 1);
        times[index]
    }

    /// Calculate frame time variance
    pub fn frame_time_variance(&self) -> f64 {
        if self.frame_history.len() < 2 {
            return 0.0;
        }

        let times: Vec<f64> = self
            .frame_history
            .iter()
            .map(|f| f.total_time_ms)
            .collect();

        let mean = times.iter().sum::<f64>() / times.len() as f64;
        let variance = times.iter().map(|t| (t - mean).powi(2)).sum::<f64>() / times.len() as f64;

        variance
    }

    /// Calculate frame time standard deviation
    pub fn frame_time_std_dev(&self) -> f64 {
        self.frame_time_variance().sqrt()
    }

    /// Identify bottlenecks
    pub fn identify_bottlenecks(&self) -> Vec<Bottleneck> {
        let mut bottlenecks = Vec::new();

        if self.frame_history.is_empty() {
            return bottlenecks;
        }

        // Get average frame time
        let avg_frame_time: f64 = self
            .frame_history
            .iter()
            .map(|f| f.total_time_ms)
            .sum::<f64>()
            / self.frame_history.len() as f64;

        // Check each section
        for (name, timing) in &self.section_timings {
            let avg_time = timing.average_ms();
            let budget = self.get_budget_for_section(name);

            if avg_time > budget && budget > 0.0 {
                let over_budget = avg_time - budget;
                let severity = (over_budget / budget).min(2.0) / 2.0; // Cap at 1.0
                let percentage = if avg_frame_time > 0.0 {
                    avg_time / avg_frame_time
                } else {
                    0.0
                };

                bottlenecks.push(Bottleneck {
                    name: name.clone(),
                    severity,
                    over_budget_ms: over_budget,
                    percentage_of_frame: percentage,
                });
            }
        }

        // Sort by severity (descending)
        bottlenecks.sort_by(|a, b| b.severity.partial_cmp(&a.severity).unwrap());

        bottlenecks
    }

    /// Generate optimization suggestions
    pub fn optimization_suggestions(&self) -> Vec<OptimizationSuggestion> {
        let mut suggestions = Vec::new();
        let bottlenecks = self.identify_bottlenecks();

        for bottleneck in bottlenecks.iter().take(5) {
            let suggestion = match bottleneck.name.as_str() {
                "render" | "rendering" => OptimizationSuggestion {
                    title: "High Render Time".to_string(),
                    description: format!(
                        "Rendering is taking {:.1}ms over budget. Consider reducing draw calls, \
                         using LOD models, or optimizing shaders.",
                        bottleneck.over_budget_ms
                    ),
                    priority: bottleneck.severity,
                    category: SuggestionCategory::Rendering,
                },
                "simulation" | "tick" => OptimizationSuggestion {
                    title: "Simulation Over Budget".to_string(),
                    description: format!(
                        "Simulation tick is taking {:.1}ms over budget. Consider reducing \
                         entity count, optimizing systems, or using spatial partitioning.",
                        bottleneck.over_budget_ms
                    ),
                    priority: bottleneck.severity,
                    category: SuggestionCategory::ECS,
                },
                "pathfinding" => OptimizationSuggestion {
                    title: "Pathfinding Performance Issue".to_string(),
                    description: format!(
                        "Pathfinding is taking {:.1}ms over budget. Consider using simpler \
                         algorithms, caching paths, or reducing path request frequency.",
                        bottleneck.over_budget_ms
                    ),
                    priority: bottleneck.severity,
                    category: SuggestionCategory::Performance,
                },
                "audio" => OptimizationSuggestion {
                    title: "Audio Processing Overload".to_string(),
                    description: format!(
                        "Audio processing is taking {:.1}ms over budget. Consider reducing \
                         simultaneous sounds, using compressed audio formats, or streaming music.",
                        bottleneck.over_budget_ms
                    ),
                    priority: bottleneck.severity,
                    category: SuggestionCategory::Audio,
                },
                "script" | "scripts" | "lua" => OptimizationSuggestion {
                    title: "Script Execution Over Budget".to_string(),
                    description: format!(
                        "Script execution is taking {:.1}ms over budget. Consider optimizing \
                         Lua code, reducing update frequency, or using native implementations.",
                        bottleneck.over_budget_ms
                    ),
                    priority: bottleneck.severity,
                    category: SuggestionCategory::Script,
                },
                _ => OptimizationSuggestion {
                    title: format!("{} Over Budget", bottleneck.name),
                    description: format!(
                        "{} is taking {:.1}ms over budget.",
                        bottleneck.name, bottleneck.over_budget_ms
                    ),
                    priority: bottleneck.severity,
                    category: SuggestionCategory::Performance,
                },
            };

            suggestions.push(suggestion);
        }

        // Memory suggestions
        if let Some(latest_memory) = self.memory_history.back() {
            let total_mb = latest_memory.total_mb();
            if total_mb > 512.0 {
                suggestions.push(OptimizationSuggestion {
                    title: "High Memory Usage".to_string(),
                    description: format!(
                        "Application is using {:.0}MB RAM. Consider reducing texture sizes, \
                         unloading unused assets, or optimizing ECS memory.",
                        total_mb
                    ),
                    priority: (total_mb / 1024.0).min(1.0),
                    category: SuggestionCategory::Memory,
                });
            }
        }

        // Sort by priority
        suggestions.sort_by(|a, b| b.priority.partial_cmp(&a.priority).unwrap());

        suggestions
    }

    /// Export to JSON
    pub fn export_json(&self, range: ExportRange) -> String {
        let frames: Vec<_> = match range {
            ExportRange::LastN(n) => self.frame_history.iter().rev().take(n).cloned().collect(),
            ExportRange::All => self.frame_history.iter().cloned().collect(),
            ExportRange::TimeRange(start, end) => self
                .frame_history
                .iter()
                .filter(|f| f.timestamp >= start && f.timestamp <= end)
                .cloned()
                .collect(),
        };

        let export_data = ExportData {
            frames,
            budget_config: self.budget_config,
            export_timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs_f64(),
        };

        serde_json::to_string_pretty(&export_data).unwrap_or_default()
    }

    /// Export to CSV
    pub fn export_csv(&self, range: ExportRange) -> String {
        let frames: Vec<_> = match range {
            ExportRange::LastN(n) => self.frame_history.iter().rev().take(n).collect(),
            ExportRange::All => self.frame_history.iter().collect(),
            ExportRange::TimeRange(start, end) => self
                .frame_history
                .iter()
                .filter(|f| f.timestamp >= start && f.timestamp <= end)
                .collect(),
        };

        let mut csv = String::new();
        csv.push_str(
            "frame,timestamp,total_time_ms,fps,entities,total_memory_mb,ecs_memory_mb\n",
        );

        for frame in frames {
            csv.push_str(&format!(
                "{},{:.3},{:.3},{:.1},{},{:.2},{:.2}\n",
                frame.frame,
                frame.timestamp,
                frame.total_time_ms,
                frame.fps,
                frame.entities.total,
                frame.memory.total_mb(),
                frame.memory.ecs_mb()
            ));
        }

        csv
    }

    /// Get time series data for graphs
    pub fn get_time_series(&self, metric: TimeSeriesMetric, frames: usize) -> Vec<(f64, f64)> {
        let count = frames.min(self.frame_history.len());
        if count == 0 {
            return Vec::new();
        }

        self.frame_history
            .iter()
            .rev()
            .take(count)
            .map(|f| {
                let value = match metric {
                    TimeSeriesMetric::FrameTime => f.total_time_ms,
                    TimeSeriesMetric::FPS => f.fps as f64,
                    TimeSeriesMetric::Memory => f.memory.total_mb(),
                    TimeSeriesMetric::Entities => f.entities.total as f64,
                };
                (f.timestamp, value)
            })
            .collect()
    }
}

impl Default for EnhancedProfiler {
    fn default() -> Self {
        Self::new()
    }
}

/// Export range selection
#[derive(Debug, Clone, Copy)]
pub enum ExportRange {
    /// Last N frames
    LastN(usize),
    /// All frames
    All,
    /// Time range (start, end) in seconds
    TimeRange(f64, f64),
}

/// Time series metric type
#[derive(Debug, Clone, Copy)]
pub enum TimeSeriesMetric {
    FrameTime,
    FPS,
    Memory,
    Entities,
}

/// Export data structure
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExportData {
    pub frames: Vec<EnhancedFrameMetrics>,
    pub budget_config: BudgetConfiguration,
    pub export_timestamp: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enhanced_profiler_creation() {
        let profiler = EnhancedProfiler::new();
        assert!(!profiler.is_enabled());
        assert_eq!(profiler.frame_count, 0);
    }

    #[test]
    fn test_profiler_toggle() {
        let mut profiler = EnhancedProfiler::new();
        assert!(!profiler.is_enabled());

        profiler.toggle();
        assert!(profiler.is_enabled());

        profiler.toggle();
        assert!(!profiler.is_enabled());
    }

    #[test]
    fn test_recording_control() {
        let mut profiler = EnhancedProfiler::new();
        assert!(profiler.is_recording());

        profiler.pause();
        assert!(!profiler.is_recording());

        profiler.resume();
        assert!(profiler.is_recording());
    }

    #[test]
    fn test_section_timing() {
        let mut timing = SectionTiming::new("test".to_string(), None);
        assert_eq!(timing.call_count, 0);

        timing.start();
        std::thread::sleep(Duration::from_millis(1));
        timing.stop();

        assert_eq!(timing.call_count, 1);
        assert!(timing.current_elapsed > Duration::ZERO);
    }

    #[test]
    fn test_budget_configuration() {
        let config = BudgetConfiguration::default();
        assert_eq!(config.target_fps, 60.0);
        assert_eq!(config.warning_threshold, 0.8);
        assert_eq!(config.critical_threshold, 1.0);
    }

    #[test]
    fn test_memory_breakdown() {
        let memory = MemoryBreakdown {
            total_ram: 1024 * 1024 * 100, // 100 MB
            ecs_world: 1024 * 1024 * 30,
            assets: 1024 * 1024 * 40,
            ..Default::default()
        };

        assert_eq!(memory.total_mb(), 100.0);
        assert_eq!(memory.ecs_mb(), 30.0);
        assert_eq!(memory.assets_mb(), 40.0);
    }

    #[test]
    fn test_budget_status() {
        assert_eq!(BudgetStatus::Good.color(), [0.2, 1.0, 0.2]);
        assert_eq!(BudgetStatus::Warning.color(), [1.0, 1.0, 0.2]);
        assert_eq!(BudgetStatus::Critical.color(), [1.0, 0.2, 0.2]);
    }
}
