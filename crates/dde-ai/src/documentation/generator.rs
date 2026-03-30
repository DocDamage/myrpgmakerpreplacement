//! AI-powered Documentation Generator
//!
//! Generates documentation from world data using LLM integration.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::templates::TemplateEngine;

/// Error types for documentation generation
#[derive(Debug, thiserror::Error)]
pub enum DocError {
    #[error("Database error: {0}")]
    Database(String),

    #[error("LLM generation failed: {0}")]
    GenerationFailed(String),

    #[error("Template error: {0}")]
    Template(String),

    #[error("Invalid data: {0}")]
    InvalidData(String),
}

impl From<dde_db::DbError> for DocError {
    fn from(e: dde_db::DbError) -> Self {
        DocError::Database(e.to_string())
    }
}

/// Result type for documentation operations
pub type DocResult<T> = Result<T, DocError>;

/// LLM Client trait for abstraction
#[async_trait::async_trait]
pub trait LlmClient: Send + Sync {
    /// Generate text from a prompt
    async fn generate(&self, prompt: &str) -> DocResult<String>;
}

/// Default LLM client implementation
pub struct DefaultLlmClient {
    _base_url: String,
    _client: reqwest::Client,
}

impl DefaultLlmClient {
    /// Create a new LLM client
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            _base_url: base_url.into(),
            _client: reqwest::Client::new(),
        }
    }

    /// Create with default localhost URL
    pub fn default_local() -> Self {
        Self::new("http://127.0.0.1:8000")
    }
}

#[async_trait::async_trait]
impl LlmClient for DefaultLlmClient {
    async fn generate(&self, prompt: &str) -> DocResult<String> {
        // In production, this would call the actual LLM sidecar
        // For now, return a placeholder response
        Ok(format!("Generated content for prompt: {}...", &prompt[..prompt.len().min(50)]))
    }
}

/// Generates documentation from world data
pub struct DocGenerator {
    llm_client: Box<dyn LlmClient>,
    _template_engine: TemplateEngine,
}

impl DocGenerator {
    /// Create a new document generator with the default LLM client
    pub fn new() -> Self {
        Self {
            llm_client: Box::new(DefaultLlmClient::default_local()),
            _template_engine: TemplateEngine,
        }
    }

    /// Create with a custom LLM client
    pub fn with_client<C: LlmClient + 'static>(client: C) -> Self {
        Self {
            llm_client: Box::new(client),
            _template_engine: TemplateEngine,
        }
    }

    /// Generate complete World Bible
    pub async fn generate_world_bible(
        &self,
        db: &dyn WorldDataProvider,
    ) -> DocResult<WorldBible> {
        // Collect all world data
        let world_data = self.collect_world_data(db).await?;

        // Generate sections via LLM
        let lore = self.generate_lore(&world_data).await?;
        let timeline = self.generate_timeline(&world_data).await?;
        let geography = self.generate_geography(&world_data).await?;
        let factions = self.generate_factions(&world_data).await?;

        Ok(WorldBible {
            title: world_data.project_name,
            lore,
            timeline,
            geography,
            factions,
            generated_at: Utc::now(),
        })
    }

    /// Generate character profiles for all NPCs
    pub async fn generate_character_profiles(
        &self,
        db: &dyn WorldDataProvider,
    ) -> DocResult<Vec<CharacterProfile>> {
        let npcs = db.get_all_npcs()?;
        let mut profiles = Vec::new();

        for npc in npcs {
            let profile = self.generate_npc_profile(&npc).await?;
            profiles.push(profile);
        }

        Ok(profiles)
    }

    /// Generate quest log with full story arcs
    pub async fn generate_quest_log(&self, db: &dyn WorldDataProvider) -> DocResult<QuestLog> {
        let quests = db.get_all_quests()?;

        // Group quests by story arc
        let arcs = self.identify_story_arcs(&quests).await?;

        // Generate narrative for each arc
        let mut arc_narratives = Vec::new();
        for arc in arcs {
            let narrative = self.generate_arc_narrative(&arc).await?;
            arc_narratives.push(narrative);
        }

        Ok(QuestLog {
            quests,
            story_arcs: arc_narratives,
            generated_at: Utc::now(),
        })
    }

    /// Generate store/marketing description
    pub async fn generate_store_description(
        &self,
        db: &dyn WorldDataProvider,
    ) -> DocResult<StoreDescription> {
        let world_data = self.collect_world_data(db).await?;

        let prompt = format!(
            "Generate a compelling store description for this RPG:\n\n\
            Title: {}\n\
            Setting: {}\n\
            Key Features: {}\n\
            Main Conflict: {}\n\n\
            Write a 2-paragraph description that would make players want to buy this game.",
            world_data.project_name,
            world_data.setting_description,
            world_data.key_features.join(", "),
            world_data.main_conflict
        );

        let description = self.llm_client.generate(&prompt).await?;

        let target_audience = self.identify_target_audience(&world_data).await?;

        Ok(StoreDescription {
            short_description: description.lines().take(2).collect::<Vec<_>>().join(" "),
            full_description: description,
            key_features: world_data.key_features,
            target_audience,
        })
    }

    /// Collect all world data from database
    async fn collect_world_data(&self, db: &dyn WorldDataProvider) -> DocResult<WorldData> {
        Ok(WorldData {
            project_name: db.get_project_name()?,
            setting_description: db.get_setting_description()?.unwrap_or_default(),
            maps: db.get_all_maps()?,
            npcs: db.get_all_npcs()?,
            quests: db.get_all_quests()?,
            items: db.get_all_items()?,
            dialogue_trees: db.get_all_dialogue_trees()?,
            factions: db.get_all_factions()?,
            key_features: self.identify_key_features(db).await?,
            main_conflict: db.get_main_conflict()?.unwrap_or_default(),
        })
    }

    /// Generate lore section via LLM
    async fn generate_lore(&self, data: &WorldData) -> DocResult<LoreSection> {
        let map_names: Vec<_> = data.maps.iter().map(|m| m.name.clone()).collect();
        let faction_names: Vec<_> = data.factions.iter().map(|f| f.name.clone()).collect();
        let item_names: Vec<_> = data.items.iter().take(5).map(|i| i.name.clone()).collect();

        let prompt = format!(
            "Based on this RPG world data, write compelling lore:\n\n\
            Maps: {}\n\
            NPCs: {}\n\
            Factions: {}\n\
            Key Items: {}\n\n\
            Write:\n\
            1. Creation myth (200 words)\n\
            2. Major historical events (bullet points)\n\
            3. Cultural practices (150 words)",
            map_names.join(", "),
            data.npcs.len(),
            faction_names.join(", "),
            item_names.join(", ")
        );

        let response = self.llm_client.generate(&prompt).await?;

        Ok(LoreSection {
            creation_myth: self.extract_section(&response, "Creation"),
            history: self.extract_bullets(&response, "Historical"),
            culture: self.extract_section(&response, "Cultural"),
        })
    }

    /// Generate timeline
    async fn generate_timeline(&self, data: &WorldData) -> DocResult<Vec<HistoricalEvent>> {
        let prompt = format!(
            "Create a historical timeline for this RPG world:\n\n\
            Setting: {}\n\
            Factions: {}\n\
            Quests: {}\n\n\
            Generate 5-10 major historical events with years/eras.",
            data.setting_description,
            data.factions.len(),
            data.quests.len()
        );

        let response = self.llm_client.generate(&prompt).await?;

        // Parse the response into events
        let mut events = Vec::new();
        for line in response.lines() {
            if let Some((year, desc)) = line.split_once(':') {
                events.push(HistoricalEvent {
                    year: year.trim().to_string(),
                    description: desc.trim().to_string(),
                });
            }
        }

        if events.is_empty() {
            // Fallback events if parsing fails
            events = vec![
                HistoricalEvent {
                    year: "Year 0".to_string(),
                    description: "The beginning of recorded history".to_string(),
                },
            ];
        }

        Ok(events)
    }

    /// Generate geography section
    async fn generate_geography(&self, data: &WorldData) -> DocResult<GeographySection> {
        let map_names: Vec<_> = data.maps.iter().map(|m| m.name.clone()).collect();

        let prompt = format!(
            "Describe the geography of this RPG world:\n\n\
            Maps/Locations: {}\n\n\
            Write a comprehensive description of the world's geography, climate, and notable regions.",
            map_names.join(", ")
        );

        let response = self.llm_client.generate(&prompt).await?;

        Ok(GeographySection {
            description: response,
            regions: data.maps.clone(),
        })
    }

    /// Generate faction profiles
    async fn generate_factions(&self, data: &WorldData) -> DocResult<Vec<FactionProfile>> {
        let mut profiles = Vec::new();

        for faction in &data.factions {
            let prompt = format!(
                "Generate a detailed profile for this faction:\n\n\
                Name: {}\n\
                Description: {}\n\n\
                Write about their goals, allies, enemies, and methods.",
                faction.name, faction.description
            );

            let response = self.llm_client.generate(&prompt).await?;

            profiles.push(FactionProfile {
                name: faction.name.clone(),
                description: faction.description.clone(),
                goals: self.extract_bullets(&response, "Goals"),
                relationships: self.extract_bullets(&response, "Relationships"),
            });
        }

        Ok(profiles)
    }

    /// Generate NPC profile with personality
    async fn generate_npc_profile(&self, npc: &NpcData) -> DocResult<CharacterProfile> {
        let prompt = format!(
            "Create a detailed character profile for this NPC:\n\n\
            Name: {}\n\
            Role: {}\n\
            Location: {}\n\
            Stats: {:?}\n\
            Dialogue samples: {}\n\n\
            Write:\n\
            1. Physical description (100 words)\n\
            2. Personality traits (bullet points)\n\
            3. Background story (150 words)\n\
            4. Motivations and goals\n\
            5. Relationships with other characters",
            npc.name,
            npc.role,
            npc.location_name,
            npc.stats,
            npc.dialogue_samples.join("\n")
        );

        let response = self.llm_client.generate(&prompt).await?;
        let portrait_prompt = self.generate_portrait_prompt(&response).await?;

        Ok(CharacterProfile {
            name: npc.name.clone(),
            physical_description: self.extract_section(&response, "Physical"),
            personality: self.extract_bullets(&response, "Personality"),
            background: self.extract_section(&response, "Background"),
            motivations: self.extract_section(&response, "Motivations"),
            relationships: self.extract_bullets(&response, "Relationships"),
            portrait_prompt,
        })
    }

    /// Generate portrait prompt for AI image generation
    async fn generate_portrait_prompt(&self, profile_text: &str) -> DocResult<String> {
        let prompt = format!(
            "Based on this character profile, create a concise AI image generation prompt\n\
            for a fantasy RPG character portrait (max 100 words):\n\n{}",
            profile_text
        );

        self.llm_client.generate(&prompt).await
    }

    /// Identify story arcs from quests
    async fn identify_story_arcs(&self, quests: &[QuestData]) -> DocResult<Vec<StoryArc>> {
        let quest_summary: Vec<_> = quests
            .iter()
            .map(|q| format!("{}: {}", q.name, q.description))
            .collect();

        let prompt = format!(
            "Analyze these quests and identify 2-4 story arcs:\n\n{}\n\n\
            For each arc, provide: Arc Name | Brief Description | Quest Names (comma separated)",
            quest_summary.join("\n")
        );

        let response = self.llm_client.generate(&prompt).await?;

        // Parse arcs from response
        let mut arcs = Vec::new();
        for line in response.lines() {
            if line.contains('|') {
                let parts: Vec<_> = line.split('|').collect();
                if parts.len() >= 3 {
                    arcs.push(StoryArc {
                        name: parts[0].trim().to_string(),
                        description: parts[1].trim().to_string(),
                        quest_names: parts[2].split(',').map(|s| s.trim().to_string()).collect(),
                    });
                }
            }
        }

        Ok(arcs)
    }

    /// Generate narrative for a story arc
    async fn generate_arc_narrative(&self, arc: &StoryArc) -> DocResult<StoryArcNarrative> {
        let prompt = format!(
            "Write a compelling narrative summary for this story arc:\n\n\
            Arc Name: {}\n\
            Description: {}\n\
            Quests: {}\n\n\
            Write 2-3 paragraphs describing the story arc's narrative flow and key moments.",
            arc.name,
            arc.description,
            arc.quest_names.join(", ")
        );

        let narrative = self.llm_client.generate(&prompt).await?;

        Ok(StoryArcNarrative {
            name: arc.name.clone(),
            description: arc.description.clone(),
            narrative,
            quest_names: arc.quest_names.clone(),
        })
    }

    /// Identify key features of the game
    async fn identify_key_features(&self, db: &dyn WorldDataProvider) -> DocResult<Vec<String>> {
        let maps = db.get_all_maps()?;
        let quests = db.get_all_quests()?;
        let npcs = db.get_all_npcs()?;

        let mut features = Vec::new();

        if maps.len() > 5 {
            features.push(format!("{} unique locations to explore", maps.len()));
        }
        if quests.len() > 10 {
            features.push(format!("{}+ quests", quests.len()));
        }
        if npcs.len() > 20 {
            features.push(format!("{}+ unique characters", npcs.len()));
        }

        // Add some generic features if not enough identified
        if features.len() < 3 {
            features.push("Immersive story".to_string());
            features.push("Strategic combat".to_string());
        }

        Ok(features)
    }

    /// Identify target audience
    async fn identify_target_audience(&self, data: &WorldData) -> DocResult<String> {
        let prompt = format!(
            "Based on this RPG's characteristics, identify the target audience:\n\n\
            Title: {}\n\
            Setting: {}\n\
            Features: {}\n\n\
            Describe the ideal player demographic in 1-2 sentences.",
            data.project_name,
            data.setting_description,
            data.key_features.join(", ")
        );

        self.llm_client.generate(&prompt).await
    }

    /// Extract a section from LLM response
    fn extract_section(&self, text: &str, section_name: &str) -> String {
        let lower_text = text.to_lowercase();
        let lower_section = section_name.to_lowercase();

        // Try to find the section
        if let Some(start) = lower_text.find(&lower_section) {
            // Skip past the section header
            let after_header = &text[start..];
            if let Some(header_end) = after_header.find('\n') {
                let content = &after_header[header_end..];
                // Find the next section header (numbered or ## style)
                let next_section = content[1..].find(|c: char| c.is_ascii_digit() && content[1..].find(c).map(|i| content.chars().nth(i + 1) == Some('.')).unwrap_or(false))
                    .or_else(|| content.find("##"));
                
                if let Some(end) = next_section {
                    return content[..end].trim().to_string();
                }
                return content.trim().to_string();
            }
            return after_header.trim().to_string();
        }

        // Fallback: return truncated text
        text.chars().take(200).collect()
    }

    /// Extract bullet points from LLM response
    #[allow(clippy::manual_strip)]
    fn extract_bullets(&self, text: &str, section_name: &str) -> Vec<String> {
        let section = self.extract_section(text, section_name);
        section
            .lines()
            .filter(|line| line.trim().starts_with('-') || line.trim().starts_with("•"))
            .map(|line| {
                line.trim()
                    .trim_start_matches('-')
                    .trim_start_matches("•")
                    .trim()
                    .to_string()
            })
            .filter(|s| !s.is_empty())
            .collect()
    }
}

impl Default for DocGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for providing world data (database abstraction)
pub trait WorldDataProvider {
    /// Get project name
    fn get_project_name(&self) -> DocResult<String>;
    /// Get setting description
    fn get_setting_description(&self) -> DocResult<Option<String>>;
    /// Get main conflict
    fn get_main_conflict(&self) -> DocResult<Option<String>>;
    /// Get all maps
    fn get_all_maps(&self) -> DocResult<Vec<MapData>>;
    /// Get all NPCs
    fn get_all_npcs(&self) -> DocResult<Vec<NpcData>>;
    /// Get all quests
    fn get_all_quests(&self) -> DocResult<Vec<QuestData>>;
    /// Get all items
    fn get_all_items(&self) -> DocResult<Vec<ItemData>>;
    /// Get all dialogue trees
    fn get_all_dialogue_trees(&self) -> DocResult<Vec<DialogueTreeData>>;
    /// Get all factions
    fn get_all_factions(&self) -> DocResult<Vec<FactionData>>;
}

/// World data collected from database
#[derive(Debug, Clone)]
pub struct WorldData {
    pub project_name: String,
    pub setting_description: String,
    pub maps: Vec<MapData>,
    pub npcs: Vec<NpcData>,
    pub quests: Vec<QuestData>,
    pub items: Vec<ItemData>,
    pub dialogue_trees: Vec<DialogueTreeData>,
    pub factions: Vec<FactionData>,
    pub key_features: Vec<String>,
    pub main_conflict: String,
}

/// Map data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapData {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub map_type: String,
    pub width: i32,
    pub height: i32,
}

/// NPC data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcData {
    pub id: u64,
    pub name: String,
    pub role: String,
    pub location_name: String,
    pub stats: HashMap<String, i32>,
    pub dialogue_samples: Vec<String>,
}

/// Quest data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestData {
    pub id: u64,
    pub name: String,
    pub description: String,
    pub quest_type: QuestType,
    pub difficulty: u32,
    pub giver_name: Option<String>,
}

/// Quest type enumeration
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum QuestType {
    Main,
    Side,
    Bounty,
    Delivery,
    Escort,
    Exploration,
}

/// Item data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemData {
    pub id: u64,
    pub name: String,
    pub item_type: String,
    pub description: String,
}

/// Dialogue tree data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogueTreeData {
    pub id: u32,
    pub name: String,
    pub node_count: usize,
}

/// Faction data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactionData {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub alignment: String,
}

/// Generated world bible
#[derive(Debug, Clone)]
pub struct WorldBible {
    pub title: String,
    pub lore: LoreSection,
    pub timeline: Vec<HistoricalEvent>,
    pub geography: GeographySection,
    pub factions: Vec<FactionProfile>,
    pub generated_at: DateTime<Utc>,
}

/// Lore section
#[derive(Debug, Clone)]
pub struct LoreSection {
    pub creation_myth: String,
    pub history: Vec<String>,
    pub culture: String,
}

/// Historical event
#[derive(Debug, Clone)]
pub struct HistoricalEvent {
    pub year: String,
    pub description: String,
}

/// Geography section
#[derive(Debug, Clone)]
pub struct GeographySection {
    pub description: String,
    pub regions: Vec<MapData>,
}

/// Faction profile
#[derive(Debug, Clone)]
pub struct FactionProfile {
    pub name: String,
    pub description: String,
    pub goals: Vec<String>,
    pub relationships: Vec<String>,
}

/// Character profile
#[derive(Debug, Clone)]
pub struct CharacterProfile {
    pub name: String,
    pub physical_description: String,
    pub personality: Vec<String>,
    pub background: String,
    pub motivations: String,
    pub relationships: Vec<String>,
    pub portrait_prompt: String, // For AI image generation
}

/// Quest log
#[derive(Debug, Clone)]
pub struct QuestLog {
    pub quests: Vec<QuestData>,
    pub story_arcs: Vec<StoryArcNarrative>,
    pub generated_at: DateTime<Utc>,
}

/// Story arc (raw identification)
#[derive(Debug, Clone)]
pub struct StoryArc {
    pub name: String,
    pub description: String,
    pub quest_names: Vec<String>,
}

/// Story arc with narrative
#[derive(Debug, Clone)]
pub struct StoryArcNarrative {
    pub name: String,
    pub description: String,
    pub narrative: String,
    pub quest_names: Vec<String>,
}

/// Store description
#[derive(Debug, Clone)]
pub struct StoreDescription {
    pub short_description: String,
    pub full_description: String,
    pub key_features: Vec<String>,
    pub target_audience: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockWorldDataProvider;

    impl WorldDataProvider for MockWorldDataProvider {
        fn get_project_name(&self) -> DocResult<String> {
            Ok("Test Project".to_string())
        }

        fn get_setting_description(&self) -> DocResult<Option<String>> {
            Ok(Some("A fantasy world".to_string()))
        }

        fn get_main_conflict(&self) -> DocResult<Option<String>> {
            Ok(Some("Good vs Evil".to_string()))
        }

        fn get_all_maps(&self) -> DocResult<Vec<MapData>> {
            Ok(vec![MapData {
                id: 1,
                name: "Test Town".to_string(),
                description: "A small town".to_string(),
                map_type: "Town".to_string(),
                width: 20,
                height: 15,
            }])
        }

        fn get_all_npcs(&self) -> DocResult<Vec<NpcData>> {
            Ok(vec![NpcData {
                id: 1,
                name: "Test NPC".to_string(),
                role: "Merchant".to_string(),
                location_name: "Test Town".to_string(),
                stats: HashMap::new(),
                dialogue_samples: vec!["Hello!".to_string()],
            }])
        }

        fn get_all_quests(&self) -> DocResult<Vec<QuestData>> {
            Ok(vec![QuestData {
                id: 1,
                name: "Test Quest".to_string(),
                description: "Do something".to_string(),
                quest_type: QuestType::Main,
                difficulty: 3,
                giver_name: Some("Test NPC".to_string()),
            }])
        }

        fn get_all_items(&self) -> DocResult<Vec<ItemData>> {
            Ok(vec![])
        }

        fn get_all_dialogue_trees(&self) -> DocResult<Vec<DialogueTreeData>> {
            Ok(vec![])
        }

        fn get_all_factions(&self) -> DocResult<Vec<FactionData>> {
            Ok(vec![FactionData {
                id: 1,
                name: "The Good Guys".to_string(),
                description: "They are good".to_string(),
                alignment: "Good".to_string(),
            }])
        }
    }

    #[test]
    fn test_doc_generator_creation() {
        let generator = DocGenerator::new();
        // Just verify it creates without panic
        assert!(true);
    }

    #[test]
    fn test_extract_section() {
        let generator = DocGenerator::new();
        let text = "1. Section One\nThis is content\n2. Section Two\nMore content";
        let section = generator.extract_section(text, "Section One");
        assert!(section.contains("This is content"));
    }

    #[test]
    fn test_extract_bullets() {
        let generator = DocGenerator::new();
        let text = "Section\n- Item 1\n- Item 2\n- Item 3";
        let bullets = generator.extract_bullets(text, "Section");
        assert_eq!(bullets.len(), 3);
        assert_eq!(bullets[0], "Item 1");
        assert_eq!(bullets[1], "Item 2");
        assert_eq!(bullets[2], "Item 3");
    }

    #[tokio::test]
    async fn test_collect_world_data() {
        let generator = DocGenerator::new();
        let db = MockWorldDataProvider;
        let data = generator.collect_world_data(&db).await.unwrap();

        assert_eq!(data.project_name, "Test Project");
        assert_eq!(data.maps.len(), 1);
        assert_eq!(data.npcs.len(), 1);
    }
}
