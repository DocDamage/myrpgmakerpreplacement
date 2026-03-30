//! Quest Pool Management
//!
//! Manages the lifecycle of quests: proposals, active quests, and history.
//! Handles quest activation, completion, and failure.

use super::generator::{Difficulty, QuestProposal, QuestType, Reward};
use dde_core::World;
use serde::{Deserialize, Serialize};

/// Manages the pool of available quests
#[derive(Debug, Clone, Default)]
pub struct QuestPool {
    /// Proposed quests waiting for player/author selection
    proposed_quests: Vec<QuestProposal>,
    /// Active quests
    active_quests: Vec<ActiveQuest>,
    /// Completed quests (for story continuity)
    quest_history: Vec<QuestHistory>,
    /// ID counter for quest IDs
    next_quest_id: QuestId,
    /// Total proposals ever generated
    total_proposals_generated: u64,
}

impl QuestPool {
    /// Create a new quest pool
    pub fn new() -> Self {
        Self {
            proposed_quests: Vec::new(),
            active_quests: Vec::new(),
            quest_history: Vec::new(),
            next_quest_id: QuestId(1),
            total_proposals_generated: 0,
        }
    }

    /// Add generated quests to proposal pool
    pub fn propose_quests(&mut self, proposals: Vec<QuestProposal>) {
        for proposal in proposals {
            self.proposed_quests.push(proposal);
            self.total_proposals_generated += 1;
        }

        // Keep only the most recent proposals if we have too many
        const MAX_PROPOSALS: usize = 10;
        if self.proposed_quests.len() > MAX_PROPOSALS {
            let excess = self.proposed_quests.len() - MAX_PROPOSALS;
            self.proposed_quests.drain(0..excess);
        }
    }

    /// Activate a proposed quest by index
    pub fn activate_quest(&mut self, index: usize) -> Option<ActiveQuest> {
        if index >= self.proposed_quests.len() {
            return None;
        }

        let proposal = self.proposed_quests.remove(index);
        let quest_id = self.next_quest_id();

        let active_quest = ActiveQuest::from_proposal(quest_id, proposal);
        self.active_quests.push(active_quest.clone());

        Some(active_quest)
    }

    /// Reject a proposal by index (removes it from pool)
    pub fn reject_proposal(&mut self, index: usize) -> bool {
        if index < self.proposed_quests.len() {
            self.proposed_quests.remove(index);
            true
        } else {
            false
        }
    }

    /// Clear all proposals
    pub fn clear_proposals(&mut self) {
        self.proposed_quests.clear();
    }

    /// Check for quest completion conditions
    pub fn update_quests(&mut self, world: &World, dt: f32) {
        // Update each active quest
        for quest in &mut self.active_quests {
            quest.update(world, dt);
        }

        // Check for completion
        let mut completed = Vec::new();
        for (idx, quest) in self.active_quests.iter().enumerate() {
            if quest.is_complete() {
                completed.push(idx);
            }
        }

        // Process completions (in reverse order to maintain indices)
        for idx in completed.iter().rev() {
            let quest = self.active_quests.remove(*idx);
            self.quest_history.push(QuestHistory::from_quest(&quest, QuestOutcome::Completed));
        }
    }

    /// Complete a quest manually
    pub fn complete_quest(&mut self, quest_id: QuestId) -> Option<QuestHistory> {
        if let Some(idx) = self.active_quests.iter().position(|q| q.id == quest_id) {
            let quest = self.active_quests.remove(idx);
            let history = QuestHistory::from_quest(&quest, QuestOutcome::Completed);
            self.quest_history.push(history.clone());
            Some(history)
        } else {
            None
        }
    }

    /// Fail a quest
    pub fn fail_quest(&mut self, quest_id: QuestId, reason: String) -> Option<QuestHistory> {
        if let Some(idx) = self.active_quests.iter().position(|q| q.id == quest_id) {
            let quest = self.active_quests.remove(idx);
            let mut history = QuestHistory::from_quest(&quest, QuestOutcome::Failed { reason });
            history.completion_time = Some(std::time::Duration::from_secs(0));
            self.quest_history.push(history.clone());
            Some(history)
        } else {
            None
        }
    }

    /// Abandon a quest (player gives up)
    pub fn abandon_quest(&mut self, quest_id: QuestId) -> Option<QuestHistory> {
        if let Some(idx) = self.active_quests.iter().position(|q| q.id == quest_id) {
            let quest = self.active_quests.remove(idx);
            let history = QuestHistory::from_quest(&quest, QuestOutcome::Abandoned);
            self.quest_history.push(history.clone());
            Some(history)
        } else {
            None
        }
    }

    /// Get current proposals
    pub fn get_proposals(&self) -> &[QuestProposal] {
        &self.proposed_quests
    }

    /// Get all active quests
    pub fn get_active_quests(&self) -> &[ActiveQuest] {
        &self.active_quests
    }

    /// Get mutable reference to active quests
    pub fn get_active_quests_mut(&mut self) -> &mut [ActiveQuest] {
        &mut self.active_quests
    }

    /// Get quest history
    pub fn get_history(&self) -> &[QuestHistory] {
        &self.quest_history
    }

    /// Get specific active quest
    pub fn get_quest(&self, quest_id: QuestId) -> Option<&ActiveQuest> {
        self.active_quests.iter().find(|q| q.id == quest_id)
    }

    /// Get mutable reference to specific quest
    pub fn get_quest_mut(&mut self, quest_id: QuestId) -> Option<&mut ActiveQuest> {
        self.active_quests.iter_mut().find(|q| q.id == quest_id)
    }

    /// Get number of active quests
    pub fn active_quest_count(&self) -> usize {
        self.active_quests.len()
    }

    /// Get number of completed quests in history
    pub fn completed_quest_count(&self) -> u64 {
        self.quest_history
            .iter()
            .filter(|h| matches!(h.outcome, QuestOutcome::Completed))
            .count() as u64
    }

    /// Get number of failed quests in history
    pub fn failed_quest_count(&self) -> u64 {
        self.quest_history
            .iter()
            .filter(|h| matches!(h.outcome, QuestOutcome::Failed { .. }))
            .count() as u64
    }

    /// Get total proposals ever generated
    pub fn total_proposals_generated(&self) -> u64 {
        self.total_proposals_generated
    }

    /// Get active quest by type
    pub fn get_active_by_type(&self, quest_type: QuestType) -> Vec<&ActiveQuest> {
        self.active_quests
            .iter()
            .filter(|q| q.quest_type == quest_type)
            .collect()
    }

    /// Check if player has an active quest with specific NPC
    pub fn has_quest_with_npc(&self, npc_name: &str) -> bool {
        self.active_quests.iter().any(|q| {
            q.involved_npcs
                .iter()
                .any(|npc| npc.eq_ignore_ascii_case(npc_name))
        })
    }

    /// Get quest completion percentage for all active quests
    pub fn overall_completion(&self) -> f32 {
        if self.active_quests.is_empty() {
            0.0
        } else {
            let total: f32 = self.active_quests.iter().map(|q| q.completion_percentage()).sum();
            total / self.active_quests.len() as f32
        }
    }

    /// Generate a new quest ID
    fn next_quest_id(&mut self) -> QuestId {
        let id = self.next_quest_id;
        self.next_quest_id = QuestId(id.0 + 1);
        id
    }

    /// Prune old history to save memory
    pub fn prune_history(&mut self, keep_count: usize) {
        if self.quest_history.len() > keep_count {
            let excess = self.quest_history.len() - keep_count;
            self.quest_history.drain(0..excess);
        }
    }

    /// Get quests that can be turned in (completed objectives but not turned in)
    pub fn get_ready_for_turn_in(&self) -> Vec<&ActiveQuest> {
        self.active_quests
            .iter()
            .filter(|q| q.stage == QuestStage::ReadyForTurnIn)
            .collect()
    }

    /// Get quest statistics
    pub fn stats(&self) -> QuestPoolStats {
        QuestPoolStats {
            active_quests: self.active_quest_count(),
            proposals_available: self.proposed_quests.len(),
            total_completed: self.completed_quest_count(),
            total_failed: self.failed_quest_count(),
            total_abandoned: self
                .quest_history
                .iter()
                .filter(|h| matches!(h.outcome, QuestOutcome::Abandoned))
                .count() as u64,
            overall_completion: self.overall_completion(),
        }
    }
}

/// Quest ID type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct QuestId(pub u64);

/// Proposal ID (index into proposals array)
pub type ProposalId = usize;

/// Active quest instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveQuest {
    /// Unique quest ID
    pub id: QuestId,
    /// Quest title
    pub title: String,
    /// Quest description
    pub description: String,
    /// Quest type
    pub quest_type: QuestType,
    /// Current stage
    pub stage: QuestStage,
    /// Objectives to complete
    pub objectives: Vec<QuestObjective>,
    /// Involved NPCs
    pub involved_npcs: Vec<String>,
    /// Rewards
    pub rewards: Vec<Reward>,
    /// Time limit (if any)
    pub time_limit: Option<f32>,
    /// Time elapsed since activation
    pub elapsed_time: f32,
    /// When quest was activated
    pub activated_at: std::time::SystemTime,
    /// Quest metadata
    pub metadata: QuestMetadata,
}

impl ActiveQuest {
    /// Create from a proposal
    pub fn from_proposal(id: QuestId, proposal: QuestProposal) -> Self {
        let quest_type = proposal.quest_type;
        Self {
            id,
            title: proposal.title,
            description: proposal.description,
            quest_type,
            stage: QuestStage::Started,
            objectives: Self::generate_objectives(quest_type),
            involved_npcs: proposal.involved_npcs,
            rewards: proposal.suggested_rewards,
            time_limit: None,
            elapsed_time: 0.0,
            activated_at: std::time::SystemTime::now(),
            metadata: QuestMetadata {
                difficulty: proposal.difficulty_estimate,
                source: QuestSource::Generated,
                confidence: proposal.confidence_score,
                location_hint: proposal.location_hint,
            },
        }
    }

    /// Create a manual quest
    pub fn new_manual(
        id: QuestId,
        title: String,
        description: String,
        quest_type: QuestType,
    ) -> Self {
        Self {
            id,
            title,
            description,
            quest_type,
            stage: QuestStage::Started,
            objectives: Vec::new(),
            involved_npcs: Vec::new(),
            rewards: Vec::new(),
            time_limit: None,
            elapsed_time: 0.0,
            activated_at: std::time::SystemTime::now(),
            metadata: QuestMetadata {
                difficulty: Difficulty::Medium,
                source: QuestSource::Manual,
                confidence: 1.0,
                location_hint: String::new(),
            },
        }
    }

    /// Update quest state
    pub fn update(&mut self, _world: &World, dt: f32) {
        self.elapsed_time += dt;

        // Check time limit
        if let Some(limit) = self.time_limit {
            if self.elapsed_time >= limit {
                self.stage = QuestStage::Failed;
            }
        }

        // Update stage based on objectives
        self.update_stage();
    }

    /// Add an objective
    pub fn add_objective(&mut self, objective: QuestObjective) {
        self.objectives.push(objective);
    }

    /// Update objective progress
    pub fn update_objective(&mut self, objective_id: u32, progress: u32) -> bool {
        if let Some(obj) = self.objectives.iter_mut().find(|o| o.id == objective_id) {
            obj.current = progress.min(obj.required);
            self.update_stage();
            true
        } else {
            false
        }
    }

    /// Complete an objective
    pub fn complete_objective(&mut self, objective_id: u32) -> bool {
        if let Some(obj) = self.objectives.iter_mut().find(|o| o.id == objective_id) {
            obj.current = obj.required;
            obj.completed = true;
            self.update_stage();
            true
        } else {
            false
        }
    }

    /// Check if quest is complete
    pub fn is_complete(&self) -> bool {
        self.stage == QuestStage::Completed
    }

    /// Check if quest has failed
    pub fn is_failed(&self) -> bool {
        self.stage == QuestStage::Failed
    }

    /// Get completion percentage
    pub fn completion_percentage(&self) -> f32 {
        if self.objectives.is_empty() {
            return 0.0;
        }

        let total: f32 = self
            .objectives
            .iter()
            .map(|o| o.progress_percentage())
            .sum();
        total / self.objectives.len() as f32
    }

    /// Get remaining objectives
    pub fn remaining_objectives(&self) -> Vec<&QuestObjective> {
        self.objectives.iter().filter(|o| !o.completed).collect()
    }

    /// Advance to next stage
    pub fn advance_stage(&mut self) {
        self.stage = match self.stage {
            QuestStage::NotStarted => QuestStage::Started,
            QuestStage::Started => QuestStage::InProgress,
            QuestStage::InProgress => QuestStage::AlmostComplete,
            QuestStage::AlmostComplete => QuestStage::ReadyForTurnIn,
            QuestStage::ReadyForTurnIn => QuestStage::Completed,
            QuestStage::Completed => QuestStage::Completed,
            QuestStage::Failed => QuestStage::Failed,
        };
    }

    /// Generate objectives from quest type
    fn generate_objectives(quest_type: QuestType) -> Vec<QuestObjective> {
        match quest_type {
            QuestType::Combat => vec![
                QuestObjective::new(1, "Defeat the enemies", 1),
                QuestObjective::new(2, "Return to quest giver", 1),
            ],
            QuestType::Exploration => vec![
                QuestObjective::new(1, "Find the location", 1),
                QuestObjective::new(2, "Explore the area", 1),
                QuestObjective::new(3, "Report back", 1),
            ],
            QuestType::Social => vec![
                QuestObjective::new(1, "Speak with the NPC", 1),
                QuestObjective::new(2, "Negotiate agreement", 1),
            ],
            QuestType::Mystery => vec![
                QuestObjective::new(1, "Gather clues", 3),
                QuestObjective::new(2, "Solve the mystery", 1),
                QuestObjective::new(3, "Confront the culprit", 1),
            ],
            QuestType::Escort => vec![
                QuestObjective::new(1, "Meet the escort target", 1),
                QuestObjective::new(2, "Escort to destination", 1),
                QuestObjective::new(3, "Ensure target survives", 1),
            ],
            QuestType::Delivery => vec![
                QuestObjective::new(1, "Pick up the item", 1),
                QuestObjective::new(2, "Deliver to recipient", 1),
            ],
            QuestType::Revenge => vec![
                QuestObjective::new(1, "Track down the target", 1),
                QuestObjective::new(2, "Defeat them", 1),
            ],
        }
    }

    /// Update stage based on objectives
    fn update_stage(&mut self) {
        if self.stage == QuestStage::Failed || self.stage == QuestStage::Completed {
            return;
        }

        let completed_count = self.objectives.iter().filter(|o| o.completed).count();
        let total_count = self.objectives.len();

        if completed_count == 0 {
            self.stage = QuestStage::Started;
        } else if completed_count < total_count / 2 {
            self.stage = QuestStage::InProgress;
        } else if completed_count < total_count {
            self.stage = QuestStage::AlmostComplete;
        } else if completed_count == total_count {
            if self.stage != QuestStage::ReadyForTurnIn {
                self.stage = QuestStage::ReadyForTurnIn;
            }
        }
    }
}

/// Quest stage
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum QuestStage {
    NotStarted,
    Started,
    InProgress,
    AlmostComplete,
    ReadyForTurnIn,
    Completed,
    Failed,
}

impl QuestStage {
    /// Get display name
    pub fn name(&self) -> &'static str {
        match self {
            QuestStage::NotStarted => "Not Started",
            QuestStage::Started => "Started",
            QuestStage::InProgress => "In Progress",
            QuestStage::AlmostComplete => "Almost Complete",
            QuestStage::ReadyForTurnIn => "Ready for Turn-in",
            QuestStage::Completed => "Completed",
            QuestStage::Failed => "Failed",
        }
    }

    /// Get color for UI
    pub fn color(&self) -> [u8; 3] {
        match self {
            QuestStage::NotStarted => [128, 128, 128],
            QuestStage::Started => [100, 150, 255],
            QuestStage::InProgress => [255, 200, 50],
            QuestStage::AlmostComplete => [100, 255, 100],
            QuestStage::ReadyForTurnIn => [50, 255, 50],
            QuestStage::Completed => [0, 200, 0],
            QuestStage::Failed => [255, 50, 50],
        }
    }

    /// Check if this is a terminal stage
    pub fn is_terminal(&self) -> bool {
        matches!(self, QuestStage::Completed | QuestStage::Failed)
    }
}

/// Quest objective
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestObjective {
    /// Objective ID
    pub id: u32,
    /// Description
    pub description: String,
    /// Required amount
    pub required: u32,
    /// Current progress
    pub current: u32,
    /// Whether completed
    pub completed: bool,
    /// Optional target location
    pub target_location: Option<(u32, i32, i32)>, // map_id, x, y
    /// Optional target entity
    pub target_entity: Option<String>,
}

impl QuestObjective {
    /// Create a new objective
    pub fn new(id: u32, description: impl Into<String>, required: u32) -> Self {
        Self {
            id,
            description: description.into(),
            required,
            current: 0,
            completed: false,
            target_location: None,
            target_entity: None,
        }
    }

    /// Set target location
    pub fn with_location(mut self, map_id: u32, x: i32, y: i32) -> Self {
        self.target_location = Some((map_id, x, y));
        self
    }

    /// Set target entity
    pub fn with_entity(mut self, entity: impl Into<String>) -> Self {
        self.target_entity = Some(entity.into());
        self
    }

    /// Get progress percentage
    pub fn progress_percentage(&self) -> f32 {
        if self.required == 0 {
            0.0
        } else {
            (self.current as f32 / self.required as f32).min(1.0)
        }
    }

    /// Check if objective is complete
    pub fn is_complete(&self) -> bool {
        self.current >= self.required || self.completed
    }

    /// Increment progress
    pub fn increment(&mut self, amount: u32) {
        self.current = (self.current + amount).min(self.required);
        if self.current >= self.required {
            self.completed = true;
        }
    }
}

/// Quest metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestMetadata {
    /// Difficulty level
    pub difficulty: Difficulty,
    /// Source of quest
    pub source: QuestSource,
    /// AI confidence (0.0 - 1.0)
    pub confidence: f32,
    /// Location hint
    pub location_hint: String,
}

/// Quest source
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum QuestSource {
    Generated,
    Manual,
    Scripted,
    Imported,
}

impl QuestSource {
    pub fn name(&self) -> &'static str {
        match self {
            QuestSource::Generated => "AI Generated",
            QuestSource::Manual => "Manual",
            QuestSource::Scripted => "Scripted",
            QuestSource::Imported => "Imported",
        }
    }
}

/// Quest history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestHistory {
    /// Quest ID (may be reused)
    pub id: QuestId,
    /// Quest title
    pub title: String,
    /// Quest type
    pub quest_type: QuestType,
    /// Outcome
    pub outcome: QuestOutcome,
    /// Completion time
    pub completion_time: Option<std::time::Duration>,
    /// Timestamp when completed
    pub completed_at: std::time::SystemTime,
    /// Rewards earned
    pub rewards_earned: Vec<Reward>,
}

impl QuestHistory {
    /// Create from active quest
    pub fn from_quest(quest: &ActiveQuest, outcome: QuestOutcome) -> Self {
        Self {
            id: quest.id,
            title: quest.title.clone(),
            quest_type: quest.quest_type,
            outcome,
            completion_time: Some(std::time::Duration::from_secs_f32(quest.elapsed_time)),
            completed_at: std::time::SystemTime::now(),
            rewards_earned: quest.rewards.clone(),
        }
    }
}

/// Quest outcome
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QuestOutcome {
    Completed,
    Failed { reason: String },
    Abandoned,
}

impl QuestOutcome {
    pub fn name(&self) -> &'static str {
        match self {
            QuestOutcome::Completed => "Completed",
            QuestOutcome::Failed { .. } => "Failed",
            QuestOutcome::Abandoned => "Abandoned",
        }
    }
}

/// Quest pool statistics
#[derive(Debug, Clone, Copy)]
pub struct QuestPoolStats {
    pub active_quests: usize,
    pub proposals_available: usize,
    pub total_completed: u64,
    pub total_failed: u64,
    pub total_abandoned: u64,
    pub overall_completion: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::director::generator::{QuestProposal, RewardType};

    fn create_test_proposal() -> QuestProposal {
        QuestProposal {
            title: "Test Quest".to_string(),
            description: "A test quest for testing".to_string(),
            quest_type: QuestType::Combat,
            suggested_rewards: vec![Reward {
                reward_type: RewardType::Gold,
                amount: 100,
            }],
            involved_npcs: vec!["Test NPC".to_string()],
            location_hint: "Test Location".to_string(),
            difficulty_estimate: Difficulty::Easy,
            confidence_score: 0.9,
        }
    }

    #[test]
    fn test_pool_creation() {
        let pool = QuestPool::new();
        assert_eq!(pool.active_quest_count(), 0);
        assert_eq!(pool.get_proposals().len(), 0);
    }

    #[test]
    fn test_propose_and_activate() {
        let mut pool = QuestPool::new();
        
        pool.propose_quests(vec![create_test_proposal()]);
        assert_eq!(pool.get_proposals().len(), 1);
        
        let quest = pool.activate_quest(0);
        assert!(quest.is_some());
        assert_eq!(pool.get_proposals().len(), 0);
        assert_eq!(pool.active_quest_count(), 1);
    }

    #[test]
    fn test_quest_completion() {
        let mut pool = QuestPool::new();
        
        pool.propose_quests(vec![create_test_proposal()]);
        let quest = pool.activate_quest(0).unwrap();
        let quest_id = quest.id;
        
        pool.complete_quest(quest_id);
        assert_eq!(pool.active_quest_count(), 0);
        assert_eq!(pool.completed_quest_count(), 1);
    }

    #[test]
    fn test_objective_progress() {
        let mut obj = QuestObjective::new(1, "Test objective", 5);
        assert_eq!(obj.progress_percentage(), 0.0);
        
        obj.increment(2);
        assert_eq!(obj.current, 2);
        assert!(!obj.is_complete());
        
        obj.increment(5);
        assert_eq!(obj.current, 5);
        assert!(obj.is_complete());
    }

    #[test]
    fn test_quest_stages() {
        assert_eq!(QuestStage::InProgress.name(), "In Progress");
        assert!(QuestStage::Completed.is_terminal());
        assert!(!QuestStage::InProgress.is_terminal());
    }

    #[test]
    fn test_reject_proposal() {
        let mut pool = QuestPool::new();
        pool.propose_quests(vec![create_test_proposal()]);
        
        assert!(pool.reject_proposal(0));
        assert_eq!(pool.get_proposals().len(), 0);
        assert!(!pool.reject_proposal(0)); // Already empty
    }
}
