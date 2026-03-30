//! Vibecode - TOML-based entity behavior scripting
//!
//! Vibecode allows defining NPC personality, dialogue style, and behavior triggers
//! using a simple TOML format stored in the `logic_prompt` field of entities.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Parsed Vibecode configuration for an entity
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Vibecode {
    /// Entity identity and personality
    #[serde(default)]
    pub identity: Identity,

    /// Memory configuration
    #[serde(default)]
    pub memory: Memory,

    /// Mood and emotional state
    #[serde(default)]
    pub mood: Mood,

    /// Dialogue configuration
    #[serde(default)]
    pub dialogue: Dialogue,

    /// Behavior triggers
    #[serde(default)]
    pub triggers: Vec<Trigger>,

    /// Custom properties
    #[serde(flatten)]
    pub custom: HashMap<String, serde_json::Value>,
}

/// Entity identity
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Identity {
    /// Display name
    #[serde(default)]
    pub name: String,

    /// Role in the world (merchant, guard, villager, etc.)
    #[serde(default)]
    pub role: String,

    /// Personality traits (e.g., ["friendly", "nervous", "noble"])
    #[serde(default)]
    pub personality: Vec<String>,

    /// Speech style (normal, formal, slang, archaic, etc.)
    #[serde(default = "default_speech_style")]
    pub speech_style: String,

    /// Background/backstory
    #[serde(default)]
    pub background: String,

    /// Voice/accent description
    #[serde(default)]
    pub voice: String,
}

fn default_speech_style() -> String {
    "normal".to_string()
}

/// Memory configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Memory {
    /// Short-term memory (recent events, current conversation)
    #[serde(default)]
    pub short_term: Vec<String>,

    /// Long-term memory (backstory, important facts)
    #[serde(default)]
    pub long_term: Vec<String>,

    /// Maximum short-term entries to retain
    #[serde(default = "default_short_term_limit")]
    pub short_term_limit: usize,

    /// Topics the entity knows about
    #[serde(default)]
    pub knowledge_topics: Vec<String>,
}

fn default_short_term_limit() -> usize {
    10
}

/// Mood and emotional state
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Mood {
    /// Current/default mood
    #[serde(default = "default_mood")]
    pub current: String,

    /// Mood triggers - conditions that change mood
    #[serde(default)]
    pub triggers: Vec<MoodTrigger>,

    /// Mood intensity (0.0 to 1.0)
    #[serde(default = "default_intensity")]
    pub intensity: f32,
}

fn default_mood() -> String {
    "neutral".to_string()
}

fn default_intensity() -> f32 {
    0.5
}

/// Mood trigger
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoodTrigger {
    /// Condition to check
    pub condition: String,
    /// Mood to switch to when condition is met
    pub mood: String,
    /// Optional duration in seconds
    #[serde(default)]
    pub duration_secs: Option<u32>,
}

/// Dialogue configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Dialogue {
    /// Dialogue style description
    #[serde(default)]
    pub style: String,

    /// Preferred topics
    #[serde(default)]
    pub topics: Vec<String>,

    /// Topics to avoid
    #[serde(default)]
    pub avoid_topics: Vec<String>,

    /// Greeting variants
    #[serde(default)]
    pub greetings: Vec<String>,

    /// Farewell variants
    #[serde(default)]
    pub farewells: Vec<String>,

    /// Maximum response length (words)
    #[serde(default = "default_max_length")]
    pub max_response_length: usize,
}

fn default_max_length() -> usize {
    50
}

/// Behavior trigger
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trigger {
    /// Trigger name
    pub name: String,
    /// Condition to activate
    pub condition: String,
    /// Action to take when triggered
    pub action: TriggerAction,
    /// Cooldown in seconds
    #[serde(default)]
    pub cooldown_secs: u32,
    /// Probability (0.0 to 1.0) of triggering
    #[serde(default = "default_probability")]
    pub probability: f32,
}

fn default_probability() -> f32 {
    1.0
}

/// Trigger action
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TriggerAction {
    /// Speak a bark
    Bark {
        /// Bark text template or category
        text: String,
    },
    /// Start a dialogue
    Dialogue {
        /// Dialogue tree to start
        tree_id: String,
    },
    /// Move to a location
    Move {
        /// Target position or marker
        target: String,
    },
    /// Play an animation
    Animate {
        /// Animation name
        animation: String,
    },
    /// Emit an event
    Event {
        /// Event type
        event_type: String,
        /// Event data
        #[serde(default)]
        data: HashMap<String, serde_json::Value>,
    },
}

impl Vibecode {
    /// Parse Vibecode from TOML string
    pub fn from_toml(toml_str: &str) -> Result<Self, VibecodeError> {
        toml::from_str(toml_str).map_err(|e| VibecodeError::Parse(e.to_string()))
    }

    /// Parse Vibecode from JSON string (for database storage)
    pub fn from_json(json_str: &str) -> Result<Self, VibecodeError> {
        serde_json::from_str(json_str).map_err(|e| VibecodeError::Parse(e.to_string()))
    }

    /// Serialize to TOML string
    pub fn to_toml(&self) -> Result<String, VibecodeError> {
        toml::to_string_pretty(self).map_err(|e| VibecodeError::Serialize(e.to_string()))
    }

    /// Serialize to JSON string (for database storage)
    pub fn to_json(&self) -> Result<String, VibecodeError> {
        serde_json::to_string(self).map_err(|e| VibecodeError::Serialize(e.to_string()))
    }

    /// Create a simple Vibecode with just identity
    pub fn simple(name: impl Into<String>, role: impl Into<String>) -> Self {
        Self {
            identity: Identity {
                name: name.into(),
                role: role.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    /// Add a personality trait
    pub fn with_personality(mut self, trait_name: impl Into<String>) -> Self {
        self.identity.personality.push(trait_name.into());
        self
    }

    /// Add a greeting
    pub fn with_greeting(mut self, greeting: impl Into<String>) -> Self {
        self.dialogue.greetings.push(greeting.into());
        self
    }

    /// Get a random greeting
    pub fn get_greeting(&self) -> Option<&str> {
        use rand::seq::SliceRandom;
        use rand::thread_rng;

        self.dialogue
            .greetings
            .choose(&mut thread_rng())
            .map(|s| s.as_str())
    }

    /// Get a random farewell
    pub fn get_farewell(&self) -> Option<&str> {
        use rand::seq::SliceRandom;
        use rand::thread_rng;

        self.dialogue
            .farewells
            .choose(&mut thread_rng())
            .map(|s| s.as_str())
    }

    /// Get current mood
    pub fn get_mood(&self) -> &str {
        &self.mood.current
    }

    /// Check if NPC knows about a topic
    pub fn knows_topic(&self, topic: &str) -> bool {
        self.memory
            .knowledge_topics
            .iter()
            .any(|t| t.to_lowercase() == topic.to_lowercase())
    }

    /// Add to short-term memory
    pub fn remember(&mut self, fact: impl Into<String>) {
        let fact = fact.into();
        if self.memory.short_term.len() >= self.memory.short_term_limit {
            self.memory.short_term.remove(0);
        }
        self.memory.short_term.push(fact);
    }

    /// Clear short-term memory
    pub fn forget_short_term(&mut self) {
        self.memory.short_term.clear();
    }

    /// Get personality summary for prompts
    pub fn get_personality_prompt(&self) -> String {
        let traits = if self.identity.personality.is_empty() {
            "a normal person".to_string()
        } else {
            self.identity.personality.join(", ")
        };

        format!(
            "You are {}, a {}. You are {}. You speak in a {} style.{}",
            self.identity.name,
            self.identity.role,
            traits,
            self.identity.speech_style,
            if self.identity.background.is_empty() {
                String::new()
            } else {
                format!(" Background: {}", self.identity.background)
            }
        )
    }
}

/// Vibecode error types
#[derive(Debug, thiserror::Error)]
pub enum VibecodeError {
    #[error("Failed to parse Vibecode: {0}")]
    Parse(String),

    #[error("Failed to serialize Vibecode: {0}")]
    Serialize(String),

    #[error("Invalid field: {0}")]
    InvalidField(String),
}

/// Preset Vibecode templates
pub mod presets {
    use super::*;

    /// Create a merchant NPC
    pub fn merchant(name: impl Into<String>) -> Vibecode {
        Vibecode::simple(name, "merchant")
            .with_personality("friendly")
            .with_personality("business-minded")
            .with_greeting("Welcome! Take a look at my wares.")
            .with_greeting("Looking to buy or sell?")
            .with_greeting("Best prices in town, I guarantee it!")
    }

    /// Create a guard NPC
    pub fn guard(name: impl Into<String>) -> Vibecode {
        Vibecode::simple(name, "guard")
            .with_personality("gruff")
            .with_personality("loyal")
            .with_greeting("Move along, citizen.")
            .with_greeting("Keep the peace.")
            .with_greeting("Nothing to see here.")
    }

    /// Create a villager NPC
    pub fn villager(name: impl Into<String>) -> Vibecode {
        Vibecode::simple(name, "villager")
            .with_personality("simple")
            .with_personality("friendly")
            .with_greeting("Lovely weather we're having.")
            .with_greeting("Hello there, traveler.")
            .with_greeting("Good day to you!")
    }

    /// Create a mysterious stranger
    pub fn stranger(name: impl Into<String>) -> Vibecode {
        Vibecode::simple(name, "stranger")
            .with_personality("mysterious")
            .with_personality("secretive")
            .with_greeting("...")
            .with_greeting("I've been watching you.")
            .with_greeting("We shouldn't be seen talking.")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXAMPLE_VIBECODE: &str = r#"
[identity]
name = "Elena"
role = "innkeeper"
personality = ["warm", "motherly", "gossipy"]
speech_style = "friendly"
background = "Has run the inn for 20 years"

[memory]
knowledge_topics = ["local rumors", "travelers", "weather"]

[mood]
current = "cheerful"

[dialogue]
style = "casual and warm"
greetings = ["Welcome! Come warm yourself by the fire!", "Rooms available if you need rest."]
"#;

    #[test]
    fn test_parse_vibecode() {
        let vibecode = Vibecode::from_toml(EXAMPLE_VIBECODE).unwrap();

        assert_eq!(vibecode.identity.name, "Elena");
        assert_eq!(vibecode.identity.role, "innkeeper");
        assert!(vibecode.identity.personality.contains(&"warm".to_string()));
        assert_eq!(vibecode.identity.speech_style, "friendly");
    }

    #[test]
    fn test_simple_builder() {
        let vibecode = Vibecode::simple("Bob", "blacksmith")
            .with_personality("gruff")
            .with_greeting("Need something forged?");

        assert_eq!(vibecode.identity.name, "Bob");
        assert_eq!(vibecode.identity.role, "blacksmith");
        assert!(vibecode.identity.personality.contains(&"gruff".to_string()));
        assert_eq!(vibecode.dialogue.greetings.len(), 1);
    }

    #[test]
    fn test_presets() {
        let merchant = presets::merchant("Grom");
        assert_eq!(merchant.identity.role, "merchant");
        assert!(!merchant.dialogue.greetings.is_empty());

        let guard = presets::guard("Captain");
        assert_eq!(guard.identity.role, "guard");
    }

    #[test]
    fn test_memory() {
        let mut vibecode = Vibecode::simple("Test", "test");
        vibecode.memory.short_term_limit = 2;

        vibecode.remember("Event 1");
        vibecode.remember("Event 2");
        vibecode.remember("Event 3");

        assert_eq!(vibecode.memory.short_term.len(), 2);
        assert!(!vibecode.memory.short_term.contains(&"Event 1".to_string()));
    }

    #[test]
    fn test_personality_prompt() {
        let vibecode = Vibecode::simple("Alice", "wizard")
            .with_personality("wise")
            .with_personality("eccentric");

        let prompt = vibecode.get_personality_prompt();
        assert!(prompt.contains("Alice"));
        assert!(prompt.contains("wizard"));
        assert!(prompt.contains("wise"));
    }
}
