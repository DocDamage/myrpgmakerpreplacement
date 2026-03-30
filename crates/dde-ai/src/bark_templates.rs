//! Bark Template Management Module
//!
//! Provides a template system for NPC barks (short dialogue lines) with
//! variable substitution and category-based organization.

use super::{BarkRequest, LlmProvider};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A bark template category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BarkCategory {
    Greeting,
    Combat,
    LowHealth,
    Victory,
    Defeat,
    Trading,
    Gossip,
    Quest,
    Weather,
    Night,
    Danger,
    Healing,
    LevelUp,
    Discovery,
    Farewell,
}

impl BarkCategory {
    /// Get all categories
    pub fn all() -> &'static [BarkCategory] {
        &[
            BarkCategory::Greeting,
            BarkCategory::Combat,
            BarkCategory::LowHealth,
            BarkCategory::Victory,
            BarkCategory::Defeat,
            BarkCategory::Trading,
            BarkCategory::Gossip,
            BarkCategory::Quest,
            BarkCategory::Weather,
            BarkCategory::Night,
            BarkCategory::Danger,
            BarkCategory::Healing,
            BarkCategory::LevelUp,
            BarkCategory::Discovery,
            BarkCategory::Farewell,
        ]
    }

    /// Get display name
    pub fn display_name(&self) -> &'static str {
        match self {
            BarkCategory::Greeting => "Greeting",
            BarkCategory::Combat => "Combat",
            BarkCategory::LowHealth => "Low Health",
            BarkCategory::Victory => "Victory",
            BarkCategory::Defeat => "Defeat",
            BarkCategory::Trading => "Trading",
            BarkCategory::Gossip => "Gossip",
            BarkCategory::Quest => "Quest",
            BarkCategory::Weather => "Weather",
            BarkCategory::Night => "Night",
            BarkCategory::Danger => "Danger",
            BarkCategory::Healing => "Healing",
            BarkCategory::LevelUp => "Level Up",
            BarkCategory::Discovery => "Discovery",
            BarkCategory::Farewell => "Farewell",
        }
    }

    /// Get icon
    pub fn icon(&self) -> &'static str {
        match self {
            BarkCategory::Greeting => "👋",
            BarkCategory::Combat => "⚔️",
            BarkCategory::LowHealth => "💔",
            BarkCategory::Victory => "🏆",
            BarkCategory::Defeat => "💀",
            BarkCategory::Trading => "💰",
            BarkCategory::Gossip => "💬",
            BarkCategory::Quest => "📜",
            BarkCategory::Weather => "🌤️",
            BarkCategory::Night => "🌙",
            BarkCategory::Danger => "⚠️",
            BarkCategory::Healing => "💚",
            BarkCategory::LevelUp => "⭐",
            BarkCategory::Discovery => "🔍",
            BarkCategory::Farewell => "👋",
        }
    }

    /// Get description
    pub fn description(&self) -> &'static str {
        match self {
            BarkCategory::Greeting => "NPC greetings when player approaches",
            BarkCategory::Combat => "Lines spoken during combat",
            BarkCategory::LowHealth => "Lines when NPC is near death",
            BarkCategory::Victory => "Celebration after winning",
            BarkCategory::Defeat => "Lines when defeated",
            BarkCategory::Trading => "Merchant/trading related",
            BarkCategory::Gossip => "Idle chatter and rumors",
            BarkCategory::Quest => "Quest-related dialogue",
            BarkCategory::Weather => "Comments about weather",
            BarkCategory::Night => "Nighttime specific lines",
            BarkCategory::Danger => "Warning about threats",
            BarkCategory::Healing => "When being healed",
            BarkCategory::LevelUp => "Player level up reactions",
            BarkCategory::Discovery => "Upon discovering something",
            BarkCategory::Farewell => "Parting words",
        }
    }

    /// Detect category from context string
    pub fn from_context(context: &str) -> Self {
        let ctx = context.to_lowercase();
        if ctx.contains("greet") || ctx.contains("hello") || ctx.contains("hi ") {
            BarkCategory::Greeting
        } else if ctx.contains("combat") || ctx.contains("fight") || ctx.contains("attack") {
            BarkCategory::Combat
        } else if ctx.contains("low health") || ctx.contains("hurt") || ctx.contains("wound") {
            BarkCategory::LowHealth
        } else if ctx.contains("victory") || ctx.contains("win") || ctx.contains("triumph") {
            BarkCategory::Victory
        } else if ctx.contains("defeat") || ctx.contains("lose") || ctx.contains("death") {
            BarkCategory::Defeat
        } else if ctx.contains("trade") || ctx.contains("buy") || ctx.contains("sell") {
            BarkCategory::Trading
        } else if ctx.contains("quest") || ctx.contains("mission") {
            BarkCategory::Quest
        } else if ctx.contains("weather") || ctx.contains("rain") || ctx.contains("storm") {
            BarkCategory::Weather
        } else if ctx.contains("night") || ctx.contains("dark") {
            BarkCategory::Night
        } else if ctx.contains("danger") || ctx.contains("warning") || ctx.contains("careful") {
            BarkCategory::Danger
        } else if ctx.contains("heal") || ctx.contains("cure") {
            BarkCategory::Healing
        } else if ctx.contains("level") || ctx.contains("levelup") {
            BarkCategory::LevelUp
        } else if ctx.contains("discover") || ctx.contains("find") {
            BarkCategory::Discovery
        } else if ctx.contains("farewell") || ctx.contains("goodbye") || ctx.contains("bye") {
            BarkCategory::Farewell
        } else {
            BarkCategory::Gossip
        }
    }
}

/// A variable that can be substituted in a template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateVariable {
    /// Variable name (e.g., "npc_name")
    pub name: String,
    /// Display name for UI
    pub display_name: String,
    /// Description of what this variable represents
    pub description: String,
    /// Example value
    pub example: String,
    /// Whether this variable is required
    pub required: bool,
}

impl TemplateVariable {
    /// Create a new template variable
    pub fn new(
        name: impl Into<String>,
        display_name: impl Into<String>,
        description: impl Into<String>,
        example: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            display_name: display_name.into(),
            description: description.into(),
            example: example.into(),
            required: true,
        }
    }

    /// Make this variable optional
    pub fn optional(mut self) -> Self {
        self.required = false;
        self
    }

    /// Wrap variable for template use (e.g., "{{npc_name}}")
    pub fn template_key(&self) -> String {
        format!("{{{{{}}}}}", self.name)
    }
}

/// A bark template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BarkTemplate {
    /// Unique ID
    pub id: String,
    /// Category
    pub category: BarkCategory,
    /// Template text with variables (e.g., "Hello, {{player_name}}!")
    pub template: String,
    /// Description of when this template should be used
    pub description: String,
    /// Variables used in this template
    pub variables: Vec<TemplateVariable>,
    /// Mood tags this template applies to
    pub moods: Vec<String>,
    /// Priority (higher = more likely to be selected)
    pub priority: u8,
    /// Whether this template is enabled
    pub enabled: bool,
    /// Maximum times this can be used per session (0 = unlimited)
    pub max_uses: u32,
    /// Current use count (transient)
    #[serde(skip)]
    pub current_uses: u32,
    /// Creator info
    pub created_by: TemplateSource,
    /// Creation timestamp
    pub created_at: Option<std::time::SystemTime>,
}

impl BarkTemplate {
    /// Create a new template
    pub fn new(id: impl Into<String>, category: BarkCategory, template: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            category,
            template: template.into(),
            description: String::new(),
            variables: Vec::new(),
            moods: vec!["neutral".to_string()],
            priority: 50,
            enabled: true,
            max_uses: 0,
            current_uses: 0,
            created_by: TemplateSource::System,
            created_at: Some(std::time::SystemTime::now()),
        }
    }

    /// Set description
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Add a variable
    pub fn with_variable(mut self, var: TemplateVariable) -> Self {
        self.variables.push(var);
        self
    }

    /// Set moods
    pub fn with_moods(mut self, moods: Vec<String>) -> Self {
        self.moods = moods;
        self
    }

    /// Set priority
    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority.clamp(0, 100);
        self
    }

    /// Set max uses
    pub fn with_max_uses(mut self, max: u32) -> Self {
        self.max_uses = max;
        self
    }

    /// Check if this template can be used
    pub fn is_available(&self) -> bool {
        if !self.enabled {
            return false;
        }
        if self.max_uses > 0 && self.current_uses >= self.max_uses {
            return false;
        }
        true
    }

    /// Record a use
    pub fn record_use(&mut self) {
        self.current_uses += 1;
    }

    /// Reset use count
    pub fn reset_uses(&mut self) {
        self.current_uses = 0;
    }

    /// Extract variables from template text
    pub fn extract_variables(&self) -> Vec<String> {
        let mut vars = Vec::new();
        let template = &self.template;

        let mut chars = template.chars().peekable();
        while let Some(ch) = chars.next() {
            if ch == '{' && chars.peek() == Some(&'{') {
                chars.next(); // Skip second '{'
                let mut var_name = String::new();
                while let Some(ch) = chars.next() {
                    if ch == '}' && chars.peek() == Some(&'}') {
                        chars.next(); // Skip second '}'
                        break;
                    }
                    var_name.push(ch);
                }
                if !var_name.is_empty() && !vars.contains(&var_name) {
                    vars.push(var_name);
                }
            }
        }

        vars
    }

    /// Render the template with variable values
    pub fn render(&self, values: &HashMap<String, String>) -> String {
        let mut result = self.template.clone();

        for var in &self.variables {
            let key = var.template_key();
            let value = values.get(&var.name).unwrap_or(&var.example);
            result = result.replace(&key, value);
        }

        result
    }

    /// Render with sample data
    pub fn render_sample(&self) -> String {
        let mut values = HashMap::new();
        for var in &self.variables {
            values.insert(var.name.clone(), var.example.clone());
        }
        self.render(&values)
    }

    /// Validate the template
    pub fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();

        if self.template.is_empty() {
            errors.push("Template text cannot be empty".to_string());
        }

        if self.template.len() > 200 {
            errors.push("Template is too long (max 200 chars)".to_string());
        }

        // Check for unclosed variables
        let open_count = self.template.matches("{{").count();
        let close_count = self.template.matches("}}").count();
        if open_count != close_count {
            errors.push(format!(
                "Mismatched braces: {} open, {} close",
                open_count, close_count
            ));
        }

        // Check that all extracted variables are defined
        let extracted = self.extract_variables();
        let defined: Vec<_> = self.variables.iter().map(|v| v.name.clone()).collect();
        for var in &extracted {
            if !defined.contains(var) {
                errors.push(format!("Variable '{}' used but not defined", var));
            }
        }

        errors
    }
}

/// Source of a template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TemplateSource {
    System,
    User(String),
    Generated(LlmProvider),
}

/// Manager for bark templates
#[derive(Debug, Clone)]
pub struct BarkTemplateManager {
    /// Templates by ID
    templates: HashMap<String, BarkTemplate>,
    /// Templates organized by category
    by_category: HashMap<BarkCategory, Vec<String>>,
    /// Global enabled flag
    pub enabled: bool,
    /// Whether to prefer templates over LLM generation
    pub prefer_templates: bool,
}

impl BarkTemplateManager {
    /// Create a new template manager with default templates
    pub fn new() -> Self {
        let mut manager = Self {
            templates: HashMap::new(),
            by_category: HashMap::new(),
            enabled: true,
            prefer_templates: true,
        };

        manager.load_default_templates();
        manager
    }

    /// Load default system templates
    fn load_default_templates(&mut self) {
        // Greeting templates
        self.add_template(
            BarkTemplate::new(
                "greeting_1",
                BarkCategory::Greeting,
                "Greetings, {{player_name}}! Welcome to {{location}}.",
            )
            .with_description("Standard greeting")
            .with_variable(TemplateVariable::new(
                "player_name",
                "Player Name",
                "Name of the player character",
                "Adventurer",
            ))
            .with_variable(TemplateVariable::new(
                "location",
                "Location",
                "Current location name",
                "town",
            )),
        );

        self.add_template(
            BarkTemplate::new("greeting_2", BarkCategory::Greeting, "Well met, traveler!")
                .with_description("Simple greeting")
                .with_priority(40),
        );

        self.add_template(
            BarkTemplate::new(
                "greeting_3",
                BarkCategory::Greeting,
                "{{time_of_day}} to you! What brings you to these parts?",
            )
            .with_description("Time-based greeting")
            .with_variable(TemplateVariable::new(
                "time_of_day",
                "Time of Day",
                "Morning/Afternoon/Evening",
                "Good morning",
            )),
        );

        // Combat templates
        self.add_template(
            BarkTemplate::new("combat_1", BarkCategory::Combat, "Have at thee!")
                .with_description("Combat challenge")
                .with_moods(vec!["aggressive".to_string()]),
        );

        self.add_template(
            BarkTemplate::new("combat_2", BarkCategory::Combat, "You'll not take me alive!")
                .with_description("Defiant combat bark")
                .with_moods(vec!["defiant".to_string(), "brave".to_string()]),
        );

        self.add_template(
            BarkTemplate::new(
                "combat_3",
                BarkCategory::Combat,
                "For {{faction_name}}!",
            )
            .with_description("Faction battle cry")
            .with_variable(TemplateVariable::new(
                "faction_name",
                "Faction",
                "NPC's faction",
                "the Kingdom",
            )),
        );

        // Low Health templates
        self.add_template(
            BarkTemplate::new("lowhealth_1", BarkCategory::LowHealth, "I... I need a healer...")
                .with_description("Critical health")
                .with_priority(80),
        );

        self.add_template(
            BarkTemplate::new("lowhealth_2", BarkCategory::LowHealth, "Medic! I'm hit!")
                .with_description("Call for help")
                .with_priority(70),
        );

        // Victory templates
        self.add_template(
            BarkTemplate::new("victory_1", BarkCategory::Victory, "Victory is ours!")
                .with_description("Celebration")
                .with_moods(vec!["triumphant".to_string()]),
        );

        self.add_template(
            BarkTemplate::new(
                "victory_2",
                BarkCategory::Victory,
                "Another foe vanquished! {{player_name}} fights well!",
            )
            .with_description("Acknowledge player")
            .with_variable(TemplateVariable::new(
                "player_name",
                "Player Name",
                "Player's name",
                "Champion",
            )),
        );

        // Trading templates
        self.add_template(
            BarkTemplate::new(
                "trade_1",
                BarkCategory::Trading,
                "Looking to buy or sell? I've got the best prices in {{location}}!",
            )
            .with_description("Merchant pitch")
            .with_variable(TemplateVariable::new(
                "location",
                "Location",
                "Current location",
                "town",
            )),
        );

        // Danger templates
        self.add_template(
            BarkTemplate::new("danger_1", BarkCategory::Danger, "Be careful around here!")
                .with_description("General warning"),
        );

        self.add_template(
            BarkTemplate::new(
                "danger_2",
                BarkCategory::Danger,
                "I heard there's a {{monster_name}} lurking nearby...",
            )
            .with_description("Specific threat warning")
            .with_variable(TemplateVariable::new(
                "monster_name",
                "Monster",
                "Name of threat",
                "dragon",
            )),
        );

        // Gossip templates
        self.add_template(
            BarkTemplate::new(
                "gossip_1",
                BarkCategory::Gossip,
                "Did you hear about the {{topic}}? Quite scandalous!",
            )
            .with_description("Rumor")
            .with_variable(TemplateVariable::new(
                "topic",
                "Topic",
                "Subject of gossip",
                "mayor's son",
            )),
        );

        // Weather templates
        self.add_template(
            BarkTemplate::new("weather_1", BarkCategory::Weather, "Fine weather we're having.")
                .with_description("Good weather"),
        );

        self.add_template(
            BarkTemplate::new(
                "weather_2",
                BarkCategory::Weather,
                "Storm's coming, I can feel it.",
            )
            .with_description("Storm warning"),
        );

        // Discovery templates
        self.add_template(
            BarkTemplate::new(
                "discovery_1",
                BarkCategory::Discovery,
                "By the gods! A {{item_name}}!",
            )
            .with_description("Rare find")
            .with_variable(TemplateVariable::new(
                "item_name",
                "Item",
                "Discovered item",
                "legendary sword",
            )),
        );

        // Farewell templates
        self.add_template(
            BarkTemplate::new("farewell_1", BarkCategory::Farewell, "Safe travels!")
                .with_description("Goodbye"),
        );

        self.add_template(
            BarkTemplate::new(
                "farewell_2",
                BarkCategory::Farewell,
                "May your path be clear, {{player_name}}.",
            )
            .with_description("Blessing farewell")
            .with_variable(TemplateVariable::new(
                "player_name",
                "Player Name",
                "Player's name",
                "traveler",
            )),
        );
    }

    /// Add a template
    pub fn add_template(&mut self, template: BarkTemplate) {
        let category = template.category;
        let id = template.id.clone();

        self.templates.insert(id.clone(), template);

        self.by_category
            .entry(category)
            .or_default()
            .push(id);
    }

    /// Remove a template
    pub fn remove_template(&mut self, id: &str) -> Option<BarkTemplate> {
        let template = self.templates.remove(id)?;

        if let Some(ids) = self.by_category.get_mut(&template.category) {
            ids.retain(|tid| tid != id);
        }

        Some(template)
    }

    /// Get a template by ID
    pub fn get_template(&self, id: &str) -> Option<&BarkTemplate> {
        self.templates.get(id)
    }

    /// Get mutable reference to a template
    pub fn get_template_mut(&mut self, id: &str) -> Option<&mut BarkTemplate> {
        self.templates.get_mut(id)
    }

    /// Get all templates
    pub fn all_templates(&self) -> &HashMap<String, BarkTemplate> {
        &self.templates
    }

    /// Get templates by category
    pub fn get_by_category(&self, category: BarkCategory) -> Vec<&BarkTemplate> {
        self.by_category
            .get(&category)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.templates.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all categories
    pub fn categories(&self) -> Vec<BarkCategory> {
        self.by_category.keys().copied().collect()
    }

    /// Get a random available template for a category and mood
    pub fn get_random_template(&self, context: &str, mood: &str) -> Option<&BarkTemplate> {
        if !self.enabled {
            return None;
        }

        let category = BarkCategory::from_context(context);
        let templates: Vec<_> = self
            .get_by_category(category)
            .into_iter()
            .filter(|t| t.is_available() && (mood.is_empty() || t.moods.contains(&mood.to_string())))
            .collect();

        if templates.is_empty() {
            return None;
        }

        // Weight by priority
        let total_priority: u32 = templates.iter().map(|t| t.priority as u32).sum();
        let mut rand_val = (context.len() as u32 + mood.len() as u32) % total_priority.max(1);

        for template in &templates {
            if rand_val < template.priority as u32 {
                return Some(*template);
            }
            rand_val -= template.priority as u32;
        }

        templates.first().copied()
    }

    /// Render a template with a bark request
    pub fn render_template(&self, template: &BarkTemplate, request: &BarkRequest) -> String {
        let mut values = HashMap::new();

        for var in &template.variables {
            let value = match var.name.as_str() {
                "npc_name" => request.npc_name.clone(),
                "player_name" => "Adventurer".to_string(), // Could be passed in request
                "location" => request.location.clone().unwrap_or_else(|| "here".to_string()),
                "time_of_day" => {
                    // Simple time-based greeting
                    "Greetings".to_string()
                }
                _ => var.example.clone(),
            };
            values.insert(var.name.clone(), value);
        }

        template.render(&values)
    }

    /// Update a template
    pub fn update_template(&mut self, id: &str, f: impl FnOnce(&mut BarkTemplate)) -> bool {
        if let Some(template) = self.templates.get_mut(id) {
            let old_category = template.category;
            f(template);
            let new_category = template.category;

            // Reorganize if category changed
            if old_category != new_category {
                if let Some(ids) = self.by_category.get_mut(&old_category) {
                    ids.retain(|tid| tid != id);
                }
                self.by_category
                    .entry(new_category)
                    .or_default()
                    .push(id.to_string());
            }

            true
        } else {
            false
        }
    }

    /// Reset all use counts
    pub fn reset_all_uses(&mut self) {
        for template in self.templates.values_mut() {
            template.reset_uses();
        }
    }

    /// Get template count
    pub fn template_count(&self) -> usize {
        self.templates.len()
    }

    /// Get count by category
    pub fn count_by_category(&self, category: BarkCategory) -> usize {
        self.by_category
            .get(&category)
            .map(|ids| ids.len())
            .unwrap_or(0)
    }

    /// Export templates to JSON
    pub fn export_to_json(&self) -> Result<String, serde_json::Error> {
        let templates: Vec<_> = self.templates.values().collect();
        serde_json::to_string_pretty(&templates)
    }

    /// Import templates from JSON
    pub fn import_from_json(&mut self, json: &str) -> Result<(), serde_json::Error> {
        let templates: Vec<BarkTemplate> = serde_json::from_str(json)?;
        for template in templates {
            self.add_template(template);
        }
        Ok(())
    }

    /// Validate all templates
    pub fn validate_all(&self) -> HashMap<String, Vec<String>> {
        let mut errors = HashMap::new();

        for (id, template) in &self.templates {
            let template_errors = template.validate();
            if !template_errors.is_empty() {
                errors.insert(id.clone(), template_errors);
            }
        }

        errors
    }
}

impl Default for BarkTemplateManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_category_detection() {
        assert_eq!(
            BarkCategory::from_context("greeting the player"),
            BarkCategory::Greeting
        );
        assert_eq!(
            BarkCategory::from_context("combat fight attack"),
            BarkCategory::Combat
        );
        assert_eq!(
            BarkCategory::from_context("buy sell trade"),
            BarkCategory::Trading
        );
    }

    #[test]
    fn test_template_creation() {
        let template = BarkTemplate::new(
            "test_1",
            BarkCategory::Greeting,
            "Hello, {{player_name}}!",
        )
        .with_variable(TemplateVariable::new(
            "player_name",
            "Player",
            "Player name",
            "Adventurer",
        ));

        assert_eq!(template.id, "test_1");
        assert_eq!(template.variables.len(), 1);
    }

    #[test]
    fn test_template_render() {
        let template = BarkTemplate::new("test", BarkCategory::Greeting, "Hello, {{name}}!")
            .with_variable(TemplateVariable::new("name", "Name", "Name", "World"));

        let mut values = HashMap::new();
        values.insert("name".to_string(), "Alice".to_string());

        assert_eq!(template.render(&values), "Hello, Alice!");
    }

    #[test]
    fn test_template_extract_variables() {
        let template =
            BarkTemplate::new("test", BarkCategory::Greeting, "{{greeting}}, {{name}}!")
                .with_variable(TemplateVariable::new("greeting", "Greeting", "Greeting", "Hello"))
                .with_variable(TemplateVariable::new("name", "Name", "Name", "World"));

        let vars = template.extract_variables();
        assert!(vars.contains(&"greeting".to_string()));
        assert!(vars.contains(&"name".to_string()));
    }

    #[test]
    fn test_template_validation() {
        let good_template = BarkTemplate::new("good", BarkCategory::Greeting, "Hello!");
        assert!(good_template.validate().is_empty());

        let bad_template = BarkTemplate::new("bad", BarkCategory::Greeting, "Hello, {{name}}!");
        let errors = bad_template.validate();
        assert!(!errors.is_empty()); // Variable not defined
    }

    #[test]
    fn test_template_manager() {
        let manager = BarkTemplateManager::new();
        assert!(!manager.all_templates().is_empty());
        assert!(manager.categories().contains(&BarkCategory::Greeting));
    }

    #[test]
    fn test_add_remove_template() {
        let mut manager = BarkTemplateManager::new();
        let initial_count = manager.template_count();

        let template = BarkTemplate::new("test_add", BarkCategory::Greeting, "Test!");
        manager.add_template(template);

        assert_eq!(manager.template_count(), initial_count + 1);

        let removed = manager.remove_template("test_add");
        assert!(removed.is_some());
        assert_eq!(manager.template_count(), initial_count);
    }

    #[test]
    fn test_template_usage_limits() {
        let mut template = BarkTemplate::new("limited", BarkCategory::Greeting, "Hi!")
            .with_max_uses(2);

        assert!(template.is_available());
        template.record_use();
        assert!(template.is_available());
        template.record_use();
        assert!(!template.is_available());
    }
}
