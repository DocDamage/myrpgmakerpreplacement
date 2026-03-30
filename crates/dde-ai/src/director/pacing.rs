//! Pacing Controller
//!
//! Controls the rhythm and escalation of generated content.
//! Manages tension curves, cooldowns, and content type selection.

use dde_core::World;
use serde::{Deserialize, Serialize};

/// Pacing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PacingConfig {
    /// Desired tension curve type
    pub tension_curve: TensionCurve,
    /// Base cooldown between quests (seconds)
    pub quest_cooldown: f32,
    /// Minimum time between quests
    pub min_quest_interval: f32,
    /// Maximum time between quests
    pub max_quest_interval: f32,
    /// Time in "quiet" state before generating new quest
    pub quiet_time_threshold: f32,
    /// Tension decay rate per second
    pub tension_decay_rate: f32,
    /// Tension increase from combat
    pub combat_tension_boost: f32,
    /// Maximum number of recent content to track
    pub recent_content_history: usize,
}

impl Default for PacingConfig {
    fn default() -> Self {
        Self {
            tension_curve: TensionCurve::Sawtooth,
            quest_cooldown: 300.0, // 5 minutes
            min_quest_interval: 120.0,
            max_quest_interval: 600.0,
            quiet_time_threshold: 30.0,
            tension_decay_rate: 0.05,
            combat_tension_boost: 0.3,
            recent_content_history: 5,
        }
    }
}

/// Controls the pacing of generated content
#[derive(Debug, Clone)]
pub struct PacingController {
    /// Current position in tension curve
    tension_position: f32,
    /// Current tension value (0.0 - 1.0)
    current_tension: f32,
    /// Last quest generation time
    last_quest_time: f32,
    /// Time accumulator
    time: f32,
    /// Time in "quiet" state
    quiet_time: f32,
    /// Current player power level (for escalation)
    player_power_level: f32,
    /// Tension curve configuration
    tension_curve: TensionCurve,
    /// Pacing configuration
    config: PacingConfig,
    /// Recent content types generated
    recent_content: Vec<ContentType>,
    /// Combat state tracking
    in_combat: bool,
    /// Dialogue state tracking
    in_dialogue: bool,
}



impl PacingController {
    /// Create a new pacing controller with default settings
    pub fn new() -> Self {
        Self {
            tension_position: 0.0,
            current_tension: 0.0,
            last_quest_time: -600.0, // Start eligible for generation
            time: 0.0,
            quiet_time: 0.0,
            player_power_level: 1.0,
            tension_curve: TensionCurve::Sawtooth,
            config: PacingConfig::default(),
            recent_content: Vec::with_capacity(5),
            in_combat: false,
            in_dialogue: false,
        }
    }

    /// Create with custom configuration
    pub fn with_config(config: PacingConfig) -> Self {
        Self {
            tension_curve: config.tension_curve,
            config,
            ..Self::new()
        }
    }

    /// Update pacing state (called each frame)
    pub fn tick(&mut self, dt: f32) {
        self.time += dt;

        // Update quiet time tracking
        if self.is_quiet_state() {
            self.quiet_time += dt;
        } else {
            self.quiet_time = 0.0;
        }

        // Natural tension decay
        if !self.in_combat {
            self.current_tension =
                (self.current_tension - dt * self.config.tension_decay_rate).max(0.0);
        }

        // Advance tension position based on curve
        self.update_tension_position(dt);
    }

    /// Should we generate a new quest now?
    pub fn should_generate_quest(
        &self,
        _world: &World,
        director_config: &super::DirectorConfig,
    ) -> bool {
        // Check if director is enabled
        if !director_config.enabled {
            return false;
        }

        // Check cooldown
        let time_since_last = self.time_since_last_generation();
        if time_since_last < self.config.min_quest_interval {
            return false;
        }

        // Check max quest interval - force generation if too long
        if time_since_last >= self.config.max_quest_interval {
            return true;
        }

        // Check if player is in "quiet" state
        if !self.is_quiet_state() {
            return false;
        }

        // Check quiet time threshold
        if self.quiet_time < self.config.quiet_time_threshold {
            return false;
        }

        // Check if tension aligns with desired curve
        let desired_tension = self.calculate_desired_tension();
        let tension_diff = (self.current_tension - desired_tension).abs();

        // Generate if tension is significantly below desired (need challenge)
        // or if we're at a good transition point
        if self.current_tension < desired_tension * 0.7 {
            return true;
        }

        // Generate if we've been quiet long enough and tension is aligned
        if self.quiet_time >= self.config.quiet_time_threshold * 2.0 && tension_diff < 0.2 {
            return true;
        }

        false
    }

    /// What type of content should we generate next?
    pub fn desired_content_type(&self, context: &super::analyzer::GameContext) -> ContentType {
        // Check recent content to avoid repetition
        let last_content = self.recent_content.last();

        // After combat -> Exploration or Social
        if self.time_since_combat() < 60.0 {
            if last_content != Some(&ContentType::Exploration) {
                return ContentType::Exploration;
            }
            return ContentType::Social;
        }

        // High tension -> Resolution quest
        if context.tension_level > 0.7 {
            return ContentType::Quest;
        }

        // Low tension -> Challenge quest
        if context.tension_level < 0.3 {
            return ContentType::Quest;
        }

        // Medium tension with quiet time -> World event or discovery
        if self.quiet_time > 60.0 {
            if context.is_exploring() {
                return ContentType::Discovery;
            }
            if context.nearby_npcs.len() > 2 {
                return ContentType::Relationship;
            }
        }

        // Default to quest
        ContentType::Quest
    }

    /// Update escalation based on player power
    pub fn update_escalation(&mut self, player_power: f32) {
        self.player_power_level = player_power;

        // Adjust tension curve based on player power
        // More powerful players get more challenging curves
        if player_power > 50.0 {
            self.tension_curve = TensionCurve::Sawtooth; // More intense
        } else if player_power > 25.0 {
            self.tension_curve = TensionCurve::Wave; // Moderate
        } else {
            self.tension_curve = TensionCurve::Flat; // Gentler for new players
        }
    }

    /// Record that content was generated
    pub fn record_generation(&mut self) {
        self.last_quest_time = self.time;
        self.recent_content.push(ContentType::Quest);

        // Trim recent content history
        while self.recent_content.len() > self.config.recent_content_history {
            self.recent_content.remove(0);
        }

        // Reduce tension after generating content
        self.current_tension *= 0.7;
    }

    /// Record combat started
    pub fn record_combat_start(&mut self) {
        self.in_combat = true;
        self.current_tension = (self.current_tension + self.config.combat_tension_boost).min(1.0);
        self.quiet_time = 0.0;
    }

    /// Record combat ended
    pub fn record_combat_end(&mut self) {
        self.in_combat = false;
    }

    /// Record dialogue started
    pub fn record_dialogue_start(&mut self) {
        self.in_dialogue = true;
    }

    /// Record dialogue ended
    pub fn record_dialogue_end(&mut self) {
        self.in_dialogue = false;
    }

    /// Get current tension level
    pub fn current_tension(&self) -> f32 {
        self.current_tension
    }

    /// Get time since last generation
    pub fn time_since_last_generation(&self) -> f32 {
        (self.time - self.last_quest_time).max(0.0)
    }

    /// Get time since last combat
    pub fn time_since_combat(&self) -> f32 {
        // This would need to be tracked separately with actual combat timestamps
        // For now, estimate based on tension decay
        if self.current_tension > 0.5 {
            0.0
        } else {
            (0.5 - self.current_tension) / self.config.tension_decay_rate
        }
    }

    /// Check if player is in "quiet" state
    fn is_quiet_state(&self) -> bool {
        !self.in_combat && !self.in_dialogue
    }

    /// Update tension position based on curve
    fn update_tension_position(&mut self, dt: f32) {
        // Advance position through the curve
        // Full cycle takes approximately 10 minutes
        let cycle_duration = 600.0;
        self.tension_position += dt / cycle_duration;

        // Wrap around
        if self.tension_position >= 1.0 {
            self.tension_position -= 1.0;
        }
    }

    /// Calculate desired tension at current position
    fn calculate_desired_tension(&self) -> f32 {
        match self.tension_curve {
            TensionCurve::Flat => 0.3,
            TensionCurve::Wave => {
                // Sine wave between 0.2 and 0.6
                (self.tension_position * std::f32::consts::TAU).sin() * 0.2 + 0.4
            }
            TensionCurve::Sawtooth => {
                // Rising tension with sharp drops
                if self.tension_position < 0.7 {
                    self.tension_position / 0.7 * 0.8
                } else {
                    0.8 - (self.tension_position - 0.7) / 0.3 * 0.6
                }
            }
            TensionCurve::Escalating => {
                // Gradually increasing baseline
                let base = (self.tension_position * 0.5).min(0.5);
                base + (self.tension_position * std::f32::consts::TAU * 2.0).sin() * 0.2
            }
        }
    }

    /// Set tension directly (for external events)
    pub fn set_tension(&mut self, tension: f32) {
        self.current_tension = tension.clamp(0.0, 1.0);
    }

    /// Boost tension (for dramatic events)
    pub fn boost_tension(&mut self, amount: f32) {
        self.current_tension = (self.current_tension + amount).min(1.0);
    }

    /// Get pacing statistics
    pub fn stats(&self) -> PacingStats {
        PacingStats {
            current_tension: self.current_tension,
            desired_tension: self.calculate_desired_tension(),
            time_since_last_generation: self.time_since_last_generation(),
            quiet_time: self.quiet_time,
            player_power_level: self.player_power_level,
            is_quiet_state: self.is_quiet_state(),
        }
    }
}

impl Default for PacingController {
    fn default() -> Self {
        Self::new()
    }
}

/// Tension curve types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TensionCurve {
    /// Flat, low tension (beginner friendly)
    Flat,
    /// Gentle wave (balanced experience)
    Wave,
    /// Rising with sharp drops (dramatic)
    Sawtooth,
    /// Gradually increasing (escalation)
    Escalating,
}

impl TensionCurve {
    /// Get display name
    pub fn name(&self) -> &'static str {
        match self {
            TensionCurve::Flat => "Flat (Relaxed)",
            TensionCurve::Wave => "Wave (Balanced)",
            TensionCurve::Sawtooth => "Sawtooth (Dramatic)",
            TensionCurve::Escalating => "Escalating (Intense)",
        }
    }

    /// Get description
    pub fn description(&self) -> &'static str {
        match self {
            TensionCurve::Flat => "Consistent low challenge, good for beginners",
            TensionCurve::Wave => "Natural ebb and flow of tension",
            TensionCurve::Sawtooth => "Building tension with dramatic releases",
            TensionCurve::Escalating => "Ever-increasing stakes and challenges",
        }
    }
}

/// Content types the director can generate
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ContentType {
    /// Standard quest
    Quest,
    /// World event (calamity, celebration, invasion)
    WorldEvent,
    /// New location revealed
    Discovery,
    /// NPC interaction opportunity
    Relationship,
    /// Exploration content
    Exploration,
    /// Social interaction
    Social,
}

impl ContentType {
    /// Get display name
    pub fn name(&self) -> &'static str {
        match self {
            ContentType::Quest => "Quest",
            ContentType::WorldEvent => "World Event",
            ContentType::Discovery => "Discovery",
            ContentType::Relationship => "Relationship",
            ContentType::Exploration => "Exploration",
            ContentType::Social => "Social",
        }
    }

    /// Get icon
    pub fn icon(&self) -> &'static str {
        match self {
            ContentType::Quest => "📜",
            ContentType::WorldEvent => "🌍",
            ContentType::Discovery => "🔍",
            ContentType::Relationship => "👥",
            ContentType::Exploration => "🗺️",
            ContentType::Social => "💬",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            ContentType::Quest => "A quest to complete",
            ContentType::WorldEvent => "A world event",
            ContentType::Discovery => "A new discovery",
            ContentType::Relationship => "An NPC relationship event",
            ContentType::Exploration => "An exploration opportunity",
            ContentType::Social => "A social interaction",
        }
    }
}

/// Pacing statistics for UI/debugging
#[derive(Debug, Clone, Copy)]
pub struct PacingStats {
    /// Current tension level (0.0 - 1.0)
    pub current_tension: f32,
    /// Desired tension from curve
    pub desired_tension: f32,
    /// Time since last quest generation
    pub time_since_last_generation: f32,
    /// Time in quiet state
    pub quiet_time: f32,
    /// Current player power level
    pub player_power_level: f32,
    /// Whether currently in quiet state
    pub is_quiet_state: bool,
}

/// Serialization helper for tension curve
impl From<String> for TensionCurve {
    fn from(s: String) -> Self {
        match s.as_str() {
            "flat" => TensionCurve::Flat,
            "wave" => TensionCurve::Wave,
            "sawtooth" => TensionCurve::Sawtooth,
            "escalating" => TensionCurve::Escalating,
            _ => TensionCurve::Wave,
        }
    }
}

impl From<TensionCurve> for String {
    fn from(curve: TensionCurve) -> Self {
        match curve {
            TensionCurve::Flat => "flat".to_string(),
            TensionCurve::Wave => "wave".to_string(),
            TensionCurve::Sawtooth => "sawtooth".to_string(),
            TensionCurve::Escalating => "escalating".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pacing_creation() {
        let pacing = PacingController::new();
        assert_eq!(pacing.current_tension(), 0.0);
        assert!(pacing.time_since_last_generation() > 500.0); // Started at -600
    }

    #[test]
    fn test_tension_decay() {
        let mut pacing = PacingController::new();
        pacing.set_tension(0.5);
        
        pacing.tick(1.0);
        
        // Tension should have decayed
        assert!(pacing.current_tension() < 0.5);
    }

    #[test]
    fn test_tension_curves() {
        let flat = TensionCurve::Flat;
        assert_eq!(flat.name(), "Flat (Relaxed)");

        let wave = TensionCurve::Wave;
        assert!(!wave.description().is_empty());
    }

    #[test]
    fn test_content_types() {
        assert_eq!(ContentType::Quest.name(), "Quest");
        assert_eq!(ContentType::WorldEvent.icon(), "🌍");
    }

    #[test]
    fn test_escalation() {
        let mut pacing = PacingController::new();
        
        pacing.update_escalation(10.0);
        assert_eq!(pacing.stats().player_power_level, 10.0);
        
        pacing.update_escalation(60.0);
        // Should have changed to more intense curve
        assert_eq!(pacing.tension_curve, TensionCurve::Sawtooth);
    }

    #[test]
    fn test_quiet_state() {
        let mut pacing = PacingController::new();
        
        // Initially quiet
        assert!(pacing.is_quiet_state());
        
        // Start combat
        pacing.record_combat_start();
        assert!(!pacing.is_quiet_state());
        
        // End combat
        pacing.record_combat_end();
        assert!(pacing.is_quiet_state());
    }

    #[test]
    fn test_generation_recording() {
        let mut pacing = PacingController::new();
        
        let initial_time = pacing.time_since_last_generation();
        pacing.record_generation();
        
        assert!(pacing.time_since_last_generation() < initial_time);
    }
}
