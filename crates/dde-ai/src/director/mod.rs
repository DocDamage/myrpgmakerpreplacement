//! AI Game Director - Procedural Quest and Event Generation
//!
//! The Director system analyzes world state and player behavior to dynamically
//! generate quests, events, and narrative content that responds to the player's
//! journey.
//!
//! # Architecture
//!
//! - **WorldAnalyzer**: Collects and analyzes world state to determine context
//! - **QuestGenerator**: Uses LLM to generate contextually appropriate quests
//! - **PacingController**: Manages the rhythm and escalation of generated content
//! - **QuestPool**: Manages active, proposed, and historical quests

pub mod analyzer;
pub mod generator;
pub mod pacing;
pub mod quest_pool;

pub use analyzer::{
    GameContext, NpcInfo, QuestStatus, WorldAnalyzer, WorldEvent, WorldStateSnapshot,
};
pub use generator::{
    Difficulty, QuestGenerator, QuestProposal, QuestType, Reward, RewardType,
    TemplateQuestGenerator,
};
pub use pacing::{ContentType, PacingController, TensionCurve};
pub use quest_pool::{ActiveQuest, ProposalId, QuestHistory, QuestOutcome, QuestPool, QuestStage};

/// The main AI Director that orchestrates content generation
pub struct DirectorSystem {
    /// World state analyzer
    pub analyzer: WorldAnalyzer,
    /// Quest generator with LLM integration
    pub generator: QuestGenerator,
    /// Pacing and escalation controller
    pub pacing: PacingController,
    /// Quest pool management
    pub quest_pool: QuestPool,
    /// Whether the director is enabled
    pub enabled: bool,
    /// Director configuration
    pub config: DirectorConfig,
}

impl DirectorSystem {
    /// Create a new director system with default configuration
    pub fn new() -> Self {
        Self {
            analyzer: WorldAnalyzer::new(),
            generator: QuestGenerator::new(),
            pacing: PacingController::new(),
            quest_pool: QuestPool::new(),
            enabled: true,
            config: DirectorConfig::default(),
        }
    }

    /// Create with custom configuration
    pub fn with_config(config: DirectorConfig) -> Self {
        Self {
            analyzer: WorldAnalyzer::new(),
            generator: QuestGenerator::new(),
            pacing: PacingController::with_config(config.pacing.clone()),
            quest_pool: QuestPool::new(),
            enabled: config.enabled,
            config,
        }
    }

    /// Main director tick - called each simulation frame
    pub async fn tick(&mut self, world: &dde_core::World, dt: f32) {
        if !self.enabled {
            return;
        }

        // Update pacing state
        self.pacing.tick(dt);

        // Check if we should generate new content
        if self.pacing.should_generate_quest(world, &self.config) {
            // Find player entity (for now, use first entity with Player kind)
            if let Some(player) = self.find_player_entity(world) {
                // Analyze world state
                let context = self.analyzer.analyze(world, player);

                // Generate quest proposals
                match self.generator.generate_quests(&context).await {
                    Ok(proposals) => {
                        // Add to proposal pool
                        self.quest_pool.propose_quests(proposals);

                        // Update last generation time
                        self.pacing.record_generation();
                    }
                    Err(e) => {
                        tracing::warn!("Quest generation failed: {}", e);
                    }
                }
            }
        }

        // Update active quests
        self.quest_pool.update_quests(world, dt);
    }

    /// Find the player entity in the world
    fn find_player_entity(&self, world: &dde_core::World) -> Option<dde_core::Entity> {
        use dde_core::components::EntityKindComp;
        use dde_core::EntityKind;

        world
            .query::<&EntityKindComp>()
            .iter()
            .find(|(_, kind)| kind.kind == EntityKind::Player)
            .map(|(entity, _)| entity)
    }

    /// Get current quest proposals for UI display
    pub fn get_proposals(&self) -> &[QuestProposal] {
        self.quest_pool.get_proposals()
    }

    /// Accept a quest proposal by index
    pub fn accept_proposal(&mut self, index: usize) -> Option<ActiveQuest> {
        self.quest_pool.activate_quest(index)
    }

    /// Reject a quest proposal by index
    pub fn reject_proposal(&mut self, index: usize) -> bool {
        self.quest_pool.reject_proposal(index)
    }

    /// Get all active quests
    pub fn get_active_quests(&self) -> &[ActiveQuest] {
        self.quest_pool.get_active_quests()
    }

    /// Get quest history
    pub fn get_quest_history(&self) -> &[QuestHistory] {
        self.quest_pool.get_history()
    }

    /// Get current tension level (0.0 - 1.0)
    pub fn current_tension(&self) -> f32 {
        self.pacing.current_tension()
    }

    /// Enable the director
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// Disable the director
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// Check if director is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Force immediate quest generation
    pub async fn force_generate(&mut self, world: &dde_core::World) -> Vec<QuestProposal> {
        if let Some(player) = self.find_player_entity(world) {
            let context = self.analyzer.analyze(world, player);
            match self.generator.generate_quests(&context).await {
                Ok(proposals) => {
                    self.quest_pool.propose_quests(proposals.clone());
                    proposals
                }
                Err(e) => {
                    tracing::warn!("Forced quest generation failed: {}", e);
                    vec![]
                }
            }
        } else {
            vec![]
        }
    }

    /// Complete a quest
    pub fn complete_quest(
        &mut self,
        quest_id: crate::director::quest_pool::QuestId,
    ) -> Option<QuestHistory> {
        self.quest_pool.complete_quest(quest_id)
    }

    /// Fail a quest
    pub fn fail_quest(
        &mut self,
        quest_id: crate::director::quest_pool::QuestId,
        reason: String,
    ) -> Option<QuestHistory> {
        self.quest_pool.fail_quest(quest_id, reason)
    }

    /// Get director statistics
    pub fn stats(&self) -> DirectorStats {
        DirectorStats {
            total_proposals: self.quest_pool.total_proposals_generated(),
            active_quests: self.quest_pool.active_quest_count(),
            completed_quests: self.quest_pool.completed_quest_count(),
            failed_quests: self.quest_pool.failed_quest_count(),
            current_tension: self.current_tension(),
            time_since_last_generation: self.pacing.time_since_last_generation(),
        }
    }
}

impl Default for DirectorSystem {
    fn default() -> Self {
        Self::new()
    }
}

pub use pacing::PacingConfig;

/// Director configuration
#[derive(Debug, Clone)]
pub struct DirectorConfig {
    /// Whether the director is enabled
    pub enabled: bool,
    /// Base cooldown between quest generations (seconds)
    pub quest_cooldown: f32,
    /// Maximum number of active quests
    pub max_active_quests: usize,
    /// Maximum number of proposals to keep
    pub max_proposals: usize,
    /// Enable LLM generation (false = templates only)
    pub use_llm: bool,
    /// Pacing configuration
    pub pacing: PacingConfig,
    /// Minimum player level for quest generation
    pub min_player_level: u32,
    /// Faction IDs to track
    pub tracked_factions: Vec<u32>,
}

impl Default for DirectorConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            quest_cooldown: 300.0, // 5 minutes
            max_active_quests: 5,
            max_proposals: 3,
            use_llm: true,
            pacing: PacingConfig::default(),
            min_player_level: 1,
            tracked_factions: vec![1, 2, 3], // Default factions
        }
    }
}

/// Director runtime statistics
#[derive(Debug, Clone, Copy, Default)]
pub struct DirectorStats {
    /// Total proposals ever generated
    pub total_proposals: u64,
    /// Currently active quests
    pub active_quests: usize,
    /// Total completed quests
    pub completed_quests: u64,
    /// Total failed quests
    pub failed_quests: u64,
    /// Current tension level (0.0 - 1.0)
    pub current_tension: f32,
    /// Time since last quest generation (seconds)
    pub time_since_last_generation: f32,
}

/// Error types for director operations
#[derive(Debug, thiserror::Error)]
pub enum DirectorError {
    #[error("Quest generation failed: {0}")]
    GenerationFailed(String),

    #[error("Invalid proposal index: {0}")]
    InvalidProposalIndex(usize),

    #[error("Quest not found: {0}")]
    QuestNotFound(u64),

    #[error("Max active quests reached ({0})")]
    MaxActiveQuestsReached(usize),

    #[error("Director is disabled")]
    DirectorDisabled,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_director_creation() {
        let director = DirectorSystem::new();
        assert!(director.is_enabled());
        assert_eq!(director.get_proposals().len(), 0);
        assert_eq!(director.get_active_quests().len(), 0);
    }

    #[test]
    fn test_director_config() {
        let config = DirectorConfig {
            enabled: false,
            quest_cooldown: 60.0,
            max_active_quests: 3,
            ..Default::default()
        };
        let director = DirectorSystem::with_config(config);
        assert!(!director.is_enabled());
    }

    #[test]
    fn test_director_stats() {
        let director = DirectorSystem::new();
        let stats = director.stats();
        assert_eq!(stats.active_quests, 0);
        assert_eq!(stats.completed_quests, 0);
        assert_eq!(stats.failed_quests, 0);
    }
}
