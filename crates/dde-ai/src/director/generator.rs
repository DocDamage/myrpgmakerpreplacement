//! Quest Generator
//!
//! Generates quests using AI/LLM based on game context.
//! Falls back to template-based generation when LLM is unavailable.

use super::analyzer::GameContext;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Generates quests using AI based on context
#[derive(Debug, Clone)]
pub struct QuestGenerator {
    /// LLM client for generation
    llm_client: Option<LlmClient>,
    /// Template-based fallback generator
    template_fallback: TemplateQuestGenerator,
    /// Generation statistics
    stats: GeneratorStats,
    /// Whether to use LLM or templates only
    use_llm: bool,
}

impl QuestGenerator {
    /// Create a new quest generator
    pub fn new() -> Self {
        Self {
            llm_client: None,
            template_fallback: TemplateQuestGenerator::new(),
            stats: GeneratorStats::default(),
            use_llm: true,
        }
    }

    /// Create with LLM client
    pub fn with_llm(llm_client: LlmClient) -> Self {
        Self {
            llm_client: Some(llm_client),
            template_fallback: TemplateQuestGenerator::new(),
            stats: GeneratorStats::default(),
            use_llm: true,
        }
    }

    /// Disable LLM, use templates only
    pub fn disable_llm(mut self) -> Self {
        self.use_llm = false;
        self
    }

    /// Generate 3 quest proposals based on context
    pub async fn generate_quests(
        &mut self,
        context: &GameContext,
    ) -> Result<Vec<QuestProposal>, GeneratorError> {
        // Determine what types of quests to generate based on context
        let target_types = self.select_quest_types(context);

        let mut proposals = Vec::with_capacity(3);

        for quest_type in target_types.iter().take(3) {
            match self.generate_quest_of_type(context, *quest_type).await {
                Ok(Some(proposal)) => proposals.push(proposal),
                Ok(None) => {}
                Err(e) => {
                    tracing::warn!("Failed to generate {:?} quest: {}", quest_type, e);
                }
            }
        }

        self.stats.total_generated += proposals.len() as u64;

        if proposals.is_empty() {
            // Fall back to template generation
            proposals = self.template_fallback.generate_proposals(context, 3);
            self.stats.fallback_uses += 1;
        }

        Ok(proposals)
    }

    /// Generate a specific quest type
    pub async fn generate_quest_of_type(
        &mut self,
        context: &GameContext,
        quest_type: QuestType,
    ) -> Result<Option<QuestProposal>, GeneratorError> {
        if self.use_llm && self.llm_client.is_some() {
            match self.generate_with_llm(context, quest_type).await {
                Ok(proposal) => {
                    self.stats.llm_uses += 1;
                    return Ok(Some(proposal));
                }
                Err(e) => {
                    tracing::warn!("LLM generation failed, using template: {}", e);
                    self.stats.llm_failures += 1;
                }
            }
        }

        // Use template fallback
        self.stats.fallback_uses += 1;
        Ok(self.template_fallback.generate_quest(context, quest_type))
    }

    /// Generate quest using LLM
    async fn generate_with_llm(
        &mut self,
        context: &GameContext,
        quest_type: QuestType,
    ) -> Result<QuestProposal, GeneratorError> {
        let prompt = self.build_llm_prompt(context, quest_type);

        // Placeholder for actual LLM call
        // In real implementation, this would call the sidecar
        tracing::debug!("LLM prompt for {:?} quest:\n{}", quest_type, prompt);

        // Simulate LLM response for now
        let mock_response = self.mock_llm_response(context, quest_type);

        // Parse response into proposal
        self.parse_llm_response(&mock_response, quest_type, context)
    }

    /// Build LLM prompt for quest generation
    fn build_llm_prompt(&self, context: &GameContext, quest_type: QuestType) -> String {
        format!(
            "Generate a {} quest for an RPG game based on the following context:\n\n\
             {}\n\n\
             Create a quest with:\n\
             1. A compelling title (max 40 characters)\n\
             2. A brief description (2-3 sentences)\n\
             3. Suggested rewards (gold, XP, items)\n\
             4. Estimated difficulty (Easy, Medium, Hard, Legendary)\n\
             5. Any NPCs involved\n\n\
             Respond in this JSON format:\n\
             {{\n\
               \"title\": \"...\",\n\
               \"description\": \"...\",\n\
               \"rewards\": [{{\"type\": \"gold\", \"amount\": 100}}],\n\
               \"difficulty\": \"Medium\",\n\
               \"involved_npcs\": [\"NPC Name\"]\n\
             }}",
            quest_type.name(),
            context.to_prompt_context()
        )
    }

    /// Parse LLM response into quest proposal
    fn parse_llm_response(
        &self,
        response: &str,
        quest_type: QuestType,
        context: &GameContext,
    ) -> Result<QuestProposal, GeneratorError> {
        // Try to parse as JSON
        match serde_json::from_str::<serde_json::Value>(response) {
            Ok(json) => {
                let title = json["title"]
                    .as_str()
                    .unwrap_or("Mysterious Quest")
                    .to_string();
                let description = json["description"]
                    .as_str()
                    .unwrap_or("Something needs to be done.")
                    .to_string();

                let rewards = self.parse_rewards(&json["rewards"]);
                let difficulty = self.parse_difficulty(json["difficulty"].as_str());

                let involved_npcs: Vec<String> = json["involved_npcs"]
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();

                Ok(QuestProposal {
                    title,
                    description,
                    quest_type,
                    suggested_rewards: rewards,
                    involved_npcs: if involved_npcs.is_empty() {
                        vec!["new".to_string()]
                    } else {
                        involved_npcs
                    },
                    location_hint: self.generate_location_hint(context, quest_type),
                    difficulty_estimate: difficulty,
                    confidence_score: 0.85,
                })
            }
            Err(e) => Err(GeneratorError::ParseError(e.to_string())),
        }
    }

    /// Parse rewards from JSON
    fn parse_rewards(&self, rewards_json: &serde_json::Value) -> Vec<Reward> {
        let mut rewards = Vec::new();

        if let Some(arr) = rewards_json.as_array() {
            for reward in arr {
                let reward_type = match reward["type"].as_str() {
                    Some("gold") => RewardType::Gold,
                    Some("xp") => RewardType::XP,
                    Some("item") => RewardType::Item(reward["item_id"].as_u64().unwrap_or(1) as u32),
                    Some("reputation") => {
                        RewardType::Reputation(reward["faction_id"].as_u64().unwrap_or(1) as u32)
                    }
                    _ => RewardType::Gold,
                };

                let amount = reward["amount"].as_u64().unwrap_or(100) as u32;

                rewards.push(Reward {
                    reward_type,
                    amount,
                });
            }
        }

        if rewards.is_empty() {
            // Default reward
            rewards.push(Reward {
                reward_type: RewardType::Gold,
                amount: 100,
            });
        }

        rewards
    }

    /// Parse difficulty string
    fn parse_difficulty(&self, difficulty: Option<&str>) -> Difficulty {
        match difficulty {
            Some("Easy") => Difficulty::Easy,
            Some("Medium") => Difficulty::Medium,
            Some("Hard") => Difficulty::Hard,
            Some("Legendary") => Difficulty::Legendary,
            _ => Difficulty::Medium,
        }
    }

    /// Generate location hint based on context
    fn generate_location_hint(&self, context: &GameContext, quest_type: QuestType) -> String {
        let (biome, direction) = match context.world_state.biome {
            super::analyzer::Biome::Forest => ("forest", "deep within the woods"),
            super::analyzer::Biome::Desert => ("desert", "in the shifting sands"),
            super::analyzer::Biome::Mountain => ("mountains", "among the peaks"),
            super::analyzer::Biome::Dungeon => ("dungeon", "in the depths"),
            super::analyzer::Biome::Swamp => ("swamp", "in the marshlands"),
            _ => ("area", "nearby"),
        };

        match quest_type {
            QuestType::Combat => format!("Enemy stronghold {}", direction),
            QuestType::Exploration => format!("Unexplored {} region", biome),
            QuestType::Social => "Local settlement".to_string(),
            QuestType::Mystery => format!("Ancient ruins in the {}", biome),
            QuestType::Escort => "Safe path between locations".to_string(),
            QuestType::Delivery => "Various locations".to_string(),
            QuestType::Revenge => "Where the target can be found".to_string(),
        }
    }

    /// Select appropriate quest types based on context
    fn select_quest_types(&self, context: &GameContext) -> Vec<QuestType> {
        let mut types = Vec::with_capacity(3);

        // Based on tension level
        if context.tension_level > 0.7 {
            // High tension - offer resolution
            types.push(QuestType::Combat);
            types.push(QuestType::Escort);
        } else if context.tension_level < 0.3 {
            // Low tension - offer challenge
            types.push(QuestType::Exploration);
            types.push(QuestType::Mystery);
        } else {
            // Medium tension - variety
            types.push(QuestType::Social);
            types.push(QuestType::Delivery);
        }

        // Based on recent events
        if context
            .recent_events
            .iter()
            .any(|e| matches!(e, super::analyzer::WorldEvent::CombatEncounter))
        {
            types.push(QuestType::Revenge);
        }

        // Based on nearby NPCs
        if context.nearby_npcs.len() > 2 {
            types.push(QuestType::Social);
        }

        // Ensure we have at least 3 types
        while types.len() < 3 {
            types.push(QuestType::Exploration);
        }

        // Remove duplicates while preserving order
        let mut seen = std::collections::HashSet::new();
        types.retain(|t| seen.insert(*t));

        types.truncate(3);
        types
    }

    /// Mock LLM response for testing/development
    fn mock_llm_response(&self, _context: &GameContext, quest_type: QuestType) -> String {
        let templates: HashMap<QuestType, serde_json::Value> = [
            (QuestType::Combat, serde_json::json!({
                "title": "Clear the Threat",
                "description": "Dangerous enemies have been spotted nearby. Eliminate them to ensure the safety of the region.",
                "rewards": [{"type": "gold", "amount": 200}, {"type": "xp", "amount": 150}],
                "difficulty": "Medium",
                "involved_npcs": ["Village Elder"]
            })),
            (QuestType::Exploration, serde_json::json!({
                "title": "Lost Ruins",
                "description": "Ancient ruins have been discovered in the wilderness. Explore them and uncover their secrets.",
                "rewards": [{"type": "gold", "amount": 150}, {"type": "xp", "amount": 200}],
                "difficulty": "Easy",
                "involved_npcs": ["Explorer's Guild Rep"]
            })),
            (QuestType::Social, serde_json::json!({
                "title": "Diplomatic Mission",
                "description": "Tensions are rising between factions. Negotiate a peaceful resolution.",
                "rewards": [{"type": "reputation", "amount": 50, "faction_id": 1}, {"type": "gold", "amount": 300}],
                "difficulty": "Hard",
                "involved_npcs": ["Faction Leader", "Mediator"]
            })),
            (QuestType::Mystery, serde_json::json!({
                "title": "The Missing Merchant",
                "description": "A merchant has vanished under mysterious circumstances. Investigate and find the truth.",
                "rewards": [{"type": "gold", "amount": 250}, {"type": "item", "item_id": 42, "amount": 1}],
                "difficulty": "Medium",
                "involved_npcs": ["Merchant's Wife", "Town Guard"]
            })),
            (QuestType::Escort, serde_json::json!({
                "title": "Safe Passage",
                "description": "Escort a vulnerable traveler through dangerous territory to their destination.",
                "rewards": [{"type": "gold", "amount": 180}, {"type": "xp", "amount": 120}],
                "difficulty": "Medium",
                "involved_npcs": ["Traveler"]
            })),
            (QuestType::Delivery, serde_json::json!({
                "title": "Urgent Delivery",
                "description": "Deliver an important package to a remote location before time runs out.",
                "rewards": [{"type": "gold", "amount": 100}, {"type": "xp", "amount": 80}],
                "difficulty": "Easy",
                "involved_npcs": ["Courier Master", "Recipient"]
            })),
            (QuestType::Revenge, serde_json::json!({
                "title": "Retribution",
                "description": "Those who wronged you must pay. Hunt them down and settle the score.",
                "rewards": [{"type": "gold", "amount": 400}, {"type": "xp", "amount": 300}],
                "difficulty": "Hard",
                "involved_npcs": ["Victim"]
            })),
        ].into_iter().collect();

        templates
            .get(&quest_type)
            .cloned()
            .map(|v| v.to_string())
            .unwrap_or_else(|| {
                serde_json::json!({
                    "title": "Adventure Awaits",
                    "description": "A new opportunity has arisen. Seize it!",
                    "rewards": [{"type": "gold", "amount": 100}],
                    "difficulty": "Medium",
                    "involved_npcs": []
                })
                .to_string()
            })
    }

    /// Get generation statistics
    pub fn stats(&self) -> &GeneratorStats {
        &self.stats
    }
}

impl Default for QuestGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// LLM client placeholder
#[derive(Debug, Clone)]
pub struct LlmClient;

impl LlmClient {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LlmClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Generator statistics
#[derive(Debug, Clone, Copy, Default)]
pub struct GeneratorStats {
    pub total_generated: u64,
    pub llm_uses: u64,
    pub llm_failures: u64,
    pub fallback_uses: u64,
}

/// Generator errors
#[derive(Debug, thiserror::Error)]
pub enum GeneratorError {
    #[error("LLM unavailable")]
    LlmUnavailable,

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Generation failed: {0}")]
    GenerationFailed(String),
}

/// Quest proposal from generator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestProposal {
    /// Quest title
    pub title: String,
    /// Quest description
    pub description: String,
    /// Type of quest
    pub quest_type: QuestType,
    /// Suggested rewards
    pub suggested_rewards: Vec<Reward>,
    /// Involved NPC names (or "new" for generated)
    pub involved_npcs: Vec<String>,
    /// Location hint
    pub location_hint: String,
    /// Estimated difficulty
    pub difficulty_estimate: Difficulty,
    /// LLM confidence score (0.0 - 1.0)
    pub confidence_score: f32,
}

/// Quest types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum QuestType {
    /// Defeat enemies
    Combat,
    /// Find location/item
    Exploration,
    /// Talk to NPCs, negotiate
    Social,
    /// Investigate, solve puzzle
    Mystery,
    /// Protect NPC
    Escort,
    /// Transport item
    Delivery,
    /// Respond to player actions
    Revenge,
}

impl QuestType {
    /// Get display name
    pub fn name(&self) -> &'static str {
        match self {
            QuestType::Combat => "Combat",
            QuestType::Exploration => "Exploration",
            QuestType::Social => "Social",
            QuestType::Mystery => "Mystery",
            QuestType::Escort => "Escort",
            QuestType::Delivery => "Delivery",
            QuestType::Revenge => "Revenge",
        }
    }

    /// Get description
    pub fn description(&self) -> &'static str {
        match self {
            QuestType::Combat => "Defeat enemies and clear threats",
            QuestType::Exploration => "Discover new locations and secrets",
            QuestType::Social => "Interact with NPCs and negotiate",
            QuestType::Mystery => "Investigate and solve puzzles",
            QuestType::Escort => "Protect NPCs on their journey",
            QuestType::Delivery => "Transport items to destinations",
            QuestType::Revenge => "Respond to past events",
        }
    }

    /// Get icon identifier
    pub fn icon(&self) -> &'static str {
        match self {
            QuestType::Combat => "⚔️",
            QuestType::Exploration => "🗺️",
            QuestType::Social => "💬",
            QuestType::Mystery => "🔍",
            QuestType::Escort => "🛡️",
            QuestType::Delivery => "📦",
            QuestType::Revenge => "⚖️",
        }
    }
}

/// Difficulty levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Difficulty {
    Easy,
    Medium,
    Hard,
    Legendary,
}

impl Difficulty {
    /// Get recommended player level offset
    pub fn level_offset(&self) -> i32 {
        match self {
            Difficulty::Easy => -2,
            Difficulty::Medium => 0,
            Difficulty::Hard => 3,
            Difficulty::Legendary => 5,
        }
    }

    /// Get reward multiplier
    pub fn reward_multiplier(&self) -> f32 {
        match self {
            Difficulty::Easy => 0.75,
            Difficulty::Medium => 1.0,
            Difficulty::Hard => 1.5,
            Difficulty::Legendary => 2.5,
        }
    }

    /// Get display name
    pub fn name(&self) -> &'static str {
        match self {
            Difficulty::Easy => "Easy",
            Difficulty::Medium => "Medium",
            Difficulty::Hard => "Hard",
            Difficulty::Legendary => "Legendary",
        }
    }

    /// Get color for UI
    pub fn color(&self) -> [u8; 3] {
        match self {
            Difficulty::Easy => [0, 200, 0],
            Difficulty::Medium => [200, 200, 0],
            Difficulty::Hard => [200, 100, 0],
            Difficulty::Legendary => [200, 0, 0],
        }
    }
}

/// Quest reward
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reward {
    /// Type of reward
    pub reward_type: RewardType,
    /// Amount/value
    pub amount: u32,
}

/// Reward types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RewardType {
    Gold,
    XP,
    Item(u32),       // item_id
    Reputation(u32), // faction_id
}

/// Template-based quest generator for fallback
#[derive(Debug, Clone, Default)]
pub struct TemplateQuestGenerator;

impl TemplateQuestGenerator {
    pub fn new() -> Self {
        Self
    }

    /// Generate multiple proposals
    pub fn generate_proposals(
        &self,
        context: &GameContext,
        count: usize,
    ) -> Vec<QuestProposal> {
        let types = [
            QuestType::Combat,
            QuestType::Exploration,
            QuestType::Social,
        ];

        types
            .iter()
            .take(count)
            .filter_map(|t| self.generate_quest(context, *t))
            .collect()
    }

    /// Generate a single quest from templates
    pub fn generate_quest(&self, context: &GameContext, quest_type: QuestType) -> Option<QuestProposal> {
        let templates = self.get_templates_for_type(quest_type);
        
        if templates.is_empty() {
            return None;
        }

        // Select template based on context
        let index = (context.player_level as usize + context.player_location.1 as usize) % templates.len();
        let template = &templates[index];

        Some(QuestProposal {
            title: template.title.to_string(),
            description: template.description.to_string(),
            quest_type,
            suggested_rewards: template.rewards.clone(),
            involved_npcs: vec!["new".to_string()],
            location_hint: "Nearby".to_string(),
            difficulty_estimate: self.calculate_difficulty(context),
            confidence_score: 0.5,
        })
    }

    fn get_templates_for_type(&self, quest_type: QuestType) -> Vec<QuestTemplate> {
        match quest_type {
            QuestType::Combat => vec![
                QuestTemplate {
                    title: "Clear the Monsters",
                    description: "Dangerous creatures threaten the area. Defeat them to restore safety.",
                    rewards: vec![
                        Reward { reward_type: RewardType::Gold, amount: 150 },
                        Reward { reward_type: RewardType::XP, amount: 100 },
                    ],
                },
                QuestTemplate {
                    title: "Bandit Hunt",
                    description: "Bandits have been raiding travelers. Track them down and eliminate the threat.",
                    rewards: vec![
                        Reward { reward_type: RewardType::Gold, amount: 200 },
                        Reward { reward_type: RewardType::XP, amount: 150 },
                    ],
                },
            ],
            QuestType::Exploration => vec![
                QuestTemplate {
                    title: "Explore the Unknown",
                    description: "A mysterious location has been discovered. Explore it and report your findings.",
                    rewards: vec![
                        Reward { reward_type: RewardType::Gold, amount: 100 },
                        Reward { reward_type: RewardType::XP, amount: 200 },
                    ],
                },
                QuestTemplate {
                    title: "Lost Treasure",
                    description: "Rumors speak of hidden treasure in the area. Find it before others do.",
                    rewards: vec![
                        Reward { reward_type: RewardType::Gold, amount: 300 },
                        Reward { reward_type: RewardType::XP, amount: 150 },
                    ],
                },
            ],
            QuestType::Social => vec![
                QuestTemplate {
                    title: "Deliver the Message",
                    description: "An important message needs to be delivered. Ensure it reaches its destination.",
                    rewards: vec![
                        Reward { reward_type: RewardType::Gold, amount: 75 },
                        Reward { reward_type: RewardType::Reputation(1), amount: 10 },
                    ],
                },
                QuestTemplate {
                    title: "Mediate the Dispute",
                    description: "Two parties are in conflict. Help them find a peaceful resolution.",
                    rewards: vec![
                        Reward { reward_type: RewardType::Gold, amount: 150 },
                        Reward { reward_type: RewardType::Reputation(1), amount: 20 },
                    ],
                },
            ],
            _ => vec![QuestTemplate {
                title: "Adventure Calls",
                description: "A new opportunity awaits. Embark on this adventure!",
                rewards: vec![Reward { reward_type: RewardType::Gold, amount: 100 }],
            }],
        }
    }

    fn calculate_difficulty(&self, context: &GameContext) -> Difficulty {
        if context.player_health_percent < 0.3 {
            Difficulty::Easy
        } else if context.tension_level > 0.7 {
            Difficulty::Hard
        } else {
            Difficulty::Medium
        }
    }
}

struct QuestTemplate {
    title: &'static str,
    description: &'static str,
    rewards: Vec<Reward>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::director::analyzer::{Biome, Weather, WorldStateSnapshot};
    use std::collections::HashMap;

    fn create_test_context() -> GameContext {
        GameContext {
            player_location: (1, 10, 10),
            player_level: 5,
            player_power: 25.0,
            recent_events: vec![],
            faction_standings: HashMap::new(),
            active_quests: vec![],
            world_state: WorldStateSnapshot {
                time_of_day: 12,
                weather: Weather::Clear,
                calamity_level: 2,
                biome: Biome::Forest,
                danger_level: 0.3,
            },
            nearby_npcs: vec![],
            tension_level: 0.4,
            time_since_combat: 60.0,
            inventory_items: 10,
            player_health_percent: 0.8,
        }
    }

    #[test]
    fn test_generator_creation() {
        let generator = QuestGenerator::new();
        assert_eq!(generator.stats().total_generated, 0);
    }

    #[test]
    fn test_template_generator() {
        let template_gen = TemplateQuestGenerator::new();
        let context = create_test_context();
        
        let proposal = template_gen.generate_quest(&context, QuestType::Combat);
        assert!(proposal.is_some());
        
        let proposal = proposal.unwrap();
        assert!(!proposal.title.is_empty());
        assert!(!proposal.description.is_empty());
    }

    #[test]
    fn test_quest_type_properties() {
        assert_eq!(QuestType::Combat.name(), "Combat");
        assert_eq!(QuestType::Exploration.icon(), "🗺️");
        assert!(!QuestType::Mystery.description().is_empty());
    }

    #[test]
    fn test_difficulty_properties() {
        assert_eq!(Difficulty::Easy.reward_multiplier(), 0.75);
        assert_eq!(Difficulty::Hard.level_offset(), 3);
        assert_eq!(Difficulty::Legendary.name(), "Legendary");
    }
}
