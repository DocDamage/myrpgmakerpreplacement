//! Performance Profiler
//!
//! Built-in frame profiler for tracking system performance budgets.
//! Toggled with F11 in editor mode.

pub mod advanced;

use std::collections::VecDeque;
use std::time::Instant;

/// Profiler data for a single frame
#[derive(Debug, Clone, Default)]
pub struct FrameProfile {
    /// Frame number
    pub frame: u64,
    /// Total frame time in ms
    pub frame_time_ms: f32,
    /// Time spent in simulation (ms)
    pub simulation_ms: f32,
    /// Time spent in rendering (ms)
    pub render_ms: f32,
    /// Time spent in pathfinding (ms)
    pub pathfinding_ms: f32,
    /// Time spent in audio (ms)
    pub audio_ms: f32,
    /// Time spent in UI (ms)
    pub ui_ms: f32,
    /// Entity count
    pub entity_count: usize,
    /// Active particle count
    pub particle_count: usize,
    /// Pending path requests
    pub path_requests: usize,
    /// Lua memory used (bytes)
    pub lua_memory: usize,
}

/// Performance profiler
pub struct Profiler {
    /// Is profiling enabled
    enabled: bool,
    /// Frame history (circular buffer)
    history: VecDeque<FrameProfile>,
    /// Maximum history size
    max_history: usize,
    /// Current frame being built
    current_frame: FrameProfile,
    /// Frame counter
    frame_count: u64,
    /// Last frame time
    last_frame_time: Instant,
    /// Section timing stack
    section_stack: Vec<(String, Instant)>,
    /// Budgets for each system (ms)
    budgets: SystemBudgets,
}

/// Performance budgets
#[derive(Debug, Clone, Copy)]
pub struct SystemBudgets {
    /// Total frame budget (16.6ms for 60fps)
    pub frame_ms: f32,
    /// Simulation budget
    pub simulation_ms: f32,
    /// Render budget
    pub render_ms: f32,
    /// Pathfinding budget
    pub pathfinding_ms: f32,
    /// Audio budget
    pub audio_ms: f32,
    /// UI budget
    pub ui_ms: f32,
}

impl Default for SystemBudgets {
    fn default() -> Self {
        Self {
            frame_ms: 16.6,
            simulation_ms: 3.0,
            render_ms: 4.0,
            pathfinding_ms: 1.0,
            audio_ms: 1.0,
            ui_ms: 3.0,
        }
    }
}

impl Profiler {
    /// Create a new profiler
    pub fn new() -> Self {
        Self {
            enabled: false,
            history: VecDeque::with_capacity(120),
            max_history: 120,
            current_frame: FrameProfile::default(),
            frame_count: 0,
            last_frame_time: Instant::now(),
            section_stack: Vec::new(),
            budgets: SystemBudgets::default(),
        }
    }

    /// Enable/disable profiling
    pub fn toggle(&mut self) {
        self.enabled = !self.enabled;
        tracing::info!(
            "Profiler {}",
            if self.enabled { "enabled" } else { "disabled" }
        );
    }

    /// Check if profiling is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Start a new frame
    pub fn begin_frame(&mut self) {
        if !self.enabled {
            return;
        }

        self.frame_count += 1;
        self.current_frame = FrameProfile {
            frame: self.frame_count,
            ..Default::default()
        };

        let now = Instant::now();
        let frame_time = now.duration_since(self.last_frame_time);
        self.current_frame.frame_time_ms = frame_time.as_secs_f32() * 1000.0;
        self.last_frame_time = now;
    }

    /// End the current frame
    pub fn end_frame(&mut self) {
        if !self.enabled {
            return;
        }

        // Add to history
        self.history.push_back(self.current_frame.clone());
        if self.history.len() > self.max_history {
            self.history.pop_front();
        }
    }

    /// Start timing a section
    pub fn begin_section(&mut self, name: &str) {
        if !self.enabled {
            return;
        }

        self.section_stack.push((name.to_string(), Instant::now()));
    }

    /// End timing a section
    pub fn end_section(&mut self, name: &str) {
        if !self.enabled {
            return;
        }

        if let Some((section_name, start)) = self.section_stack.pop() {
            if section_name == name {
                let elapsed = start.elapsed().as_secs_f32() * 1000.0;

                // Store in appropriate field
                match name {
                    "simulation" => self.current_frame.simulation_ms = elapsed,
                    "render" => self.current_frame.render_ms = elapsed,
                    "pathfinding" => self.current_frame.pathfinding_ms = elapsed,
                    "audio" => self.current_frame.audio_ms = elapsed,
                    "ui" => self.current_frame.ui_ms = elapsed,
                    _ => {}
                }
            }
        }
    }

    /// Record entity count
    pub fn record_entities(&mut self, count: usize) {
        if self.enabled {
            self.current_frame.entity_count = count;
        }
    }

    /// Record particle count
    pub fn record_particles(&mut self, count: usize) {
        if self.enabled {
            self.current_frame.particle_count = count;
        }
    }

    /// Record path requests
    pub fn record_path_requests(&mut self, count: usize) {
        if self.enabled {
            self.current_frame.path_requests = count;
        }
    }

    /// Record Lua memory
    pub fn record_lua_memory(&mut self, bytes: usize) {
        if self.enabled {
            self.current_frame.lua_memory = bytes;
        }
    }

    /// Get average frame time over last N frames
    pub fn average_frame_time(&self, frames: usize) -> f32 {
        let count = frames.min(self.history.len());
        if count == 0 {
            return 0.0;
        }

        let sum: f32 = self
            .history
            .iter()
            .rev()
            .take(count)
            .map(|f| f.frame_time_ms)
            .sum();

        sum / count as f32
    }

    /// Get current FPS
    pub fn fps(&self) -> f32 {
        let avg = self.average_frame_time(60);
        if avg > 0.0 {
            1000.0 / avg
        } else {
            0.0
        }
    }

    /// Get budget compliance for a system
    pub fn budget_status(&self, system: &str) -> BudgetStatus {
        if self.history.is_empty() {
            return BudgetStatus::Unknown;
        }

        let recent: Vec<_> = self.history.iter().rev().take(10).collect();

        let (actual, budget) = match system {
            "frame" => (
                recent.iter().map(|f| f.frame_time_ms).sum::<f32>() / recent.len() as f32,
                self.budgets.frame_ms,
            ),
            "simulation" => (
                recent.iter().map(|f| f.simulation_ms).sum::<f32>() / recent.len() as f32,
                self.budgets.simulation_ms,
            ),
            "render" => (
                recent.iter().map(|f| f.render_ms).sum::<f32>() / recent.len() as f32,
                self.budgets.render_ms,
            ),
            "pathfinding" => (
                recent.iter().map(|f| f.pathfinding_ms).sum::<f32>() / recent.len() as f32,
                self.budgets.pathfinding_ms,
            ),
            "audio" => (
                recent.iter().map(|f| f.audio_ms).sum::<f32>() / recent.len() as f32,
                self.budgets.audio_ms,
            ),
            "ui" => (
                recent.iter().map(|f| f.ui_ms).sum::<f32>() / recent.len() as f32,
                self.budgets.ui_ms,
            ),
            _ => return BudgetStatus::Unknown,
        };

        let ratio = actual / budget;
        if ratio <= 0.8 {
            BudgetStatus::Good
        } else if ratio <= 1.0 {
            BudgetStatus::Warning
        } else {
            BudgetStatus::OverBudget
        }
    }

    /// Get all budget statuses
    pub fn all_budget_statuses(&self) -> Vec<(&str, BudgetStatus)> {
        vec![
            ("frame", self.budget_status("frame")),
            ("simulation", self.budget_status("simulation")),
            ("render", self.budget_status("render")),
            ("pathfinding", self.budget_status("pathfinding")),
            ("audio", self.budget_status("audio")),
            ("ui", self.budget_status("ui")),
        ]
    }

    /// Get history reference
    pub fn history(&self) -> &VecDeque<FrameProfile> {
        &self.history
    }

    /// Get current frame
    pub fn current_frame(&self) -> &FrameProfile {
        &self.current_frame
    }

    /// Get budgets
    pub fn budgets(&self) -> &SystemBudgets {
        &self.budgets
    }

    /// Set budgets
    pub fn set_budgets(&mut self, budgets: SystemBudgets) {
        self.budgets = budgets;
    }

    /// Export profile data to CSV
    pub fn export_csv(&self, path: &std::path::Path) -> std::io::Result<()> {
        use std::io::Write;

        let mut file = std::fs::File::create(path)?;

        // Header
        writeln!(file, "frame,frame_time_ms,simulation_ms,render_ms,pathfinding_ms,audio_ms,ui_ms,entity_count,particle_count")?;

        // Data
        for frame in &self.history {
            writeln!(
                file,
                "{},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{},{}",
                frame.frame,
                frame.frame_time_ms,
                frame.simulation_ms,
                frame.render_ms,
                frame.pathfinding_ms,
                frame.audio_ms,
                frame.ui_ms,
                frame.entity_count,
                frame.particle_count
            )?;
        }

        Ok(())
    }
}

impl Default for Profiler {
    fn default() -> Self {
        Self::new()
    }
}

/// Budget compliance status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BudgetStatus {
    /// Within budget (< 80%)
    Good,
    /// Near budget (80-100%)
    Warning,
    /// Over budget (> 100%)
    OverBudget,
    /// Unknown/no data
    Unknown,
}

impl BudgetStatus {
    /// Get color for display (RGB)
    pub fn color(&self) -> [f32; 3] {
        match self {
            BudgetStatus::Good => [0.2, 1.0, 0.2],       // Green
            BudgetStatus::Warning => [1.0, 1.0, 0.2],    // Yellow
            BudgetStatus::OverBudget => [1.0, 0.2, 0.2], // Red
            BudgetStatus::Unknown => [0.5, 0.5, 0.5],    // Gray
        }
    }

    /// Get display text
    pub fn text(&self) -> &'static str {
        match self {
            BudgetStatus::Good => "OK",
            BudgetStatus::Warning => "WARN",
            BudgetStatus::OverBudget => "OVER",
            BudgetStatus::Unknown => "---",
        }
    }
}

/// Simple timing guard for RAII profiling
pub struct ProfileGuard<'a> {
    profiler: &'a mut Profiler,
    name: String,
}

impl<'a> ProfileGuard<'a> {
    /// Create a new profile guard
    pub fn new(profiler: &'a mut Profiler, name: &str) -> Self {
        profiler.begin_section(name);
        Self {
            profiler,
            name: name.to_string(),
        }
    }
}

impl<'a> Drop for ProfileGuard<'a> {
    fn drop(&mut self) {
        self.profiler.end_section(&self.name);
    }
}

/// Macro for easy profiling
#[macro_export]
macro_rules! profile_scope {
    ($profiler:expr, $name:expr) => {
        let _guard = $crate::profiler::ProfileGuard::new($profiler, $name);
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profiler_basic() {
        let mut profiler = Profiler::new();
        profiler.toggle();

        profiler.begin_frame();
        profiler.record_entities(100);
        profiler.record_particles(500);
        profiler.end_frame();

        assert_eq!(profiler.current_frame().entity_count, 100);
        assert_eq!(profiler.current_frame().particle_count, 500);
    }

    #[test]
    fn test_budget_status() {
        let profiler = Profiler::new();

        // No data = Unknown
        assert_eq!(profiler.budget_status("frame"), BudgetStatus::Unknown);
    }
}
