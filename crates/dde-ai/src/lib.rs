//! DocDamage Engine - AI Sidecar Client
//!
//! Communicates with Python FastAPI sidecar for LLM requests.
//! Supports multiple models: OpenAI, Anthropic (Claude), Gemini, local (Ollama)

pub mod director;
pub mod documentation;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Re-export director types
pub use director::{
    ActiveQuest, ContentType, DirectorConfig, DirectorError, DirectorStats, DirectorSystem,
    GameContext, PacingConfig, ProposalId, QuestHistory, QuestOutcome, QuestPool, QuestProposal,
    QuestStage, QuestType, TensionCurve, WorldAnalyzer, WorldEvent, WorldStateSnapshot,
};

/// AI task types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AiTaskType {
    Dialogue,
    Bark,
    Narrative,
    Balancing,
    Shader,
    /// Quest generation for AI Director
    QuestGeneration,
}

impl AiTaskType {
    /// Get the preferred model for this task type
    pub fn preferred_model(&self) -> &'static str {
        match self {
            // Claude for code-heavy tasks
            AiTaskType::Shader | AiTaskType::Balancing => "anthropic",
            // Gemini for narrative and creative tasks
            AiTaskType::Narrative | AiTaskType::Dialogue | AiTaskType::QuestGeneration => "gemini",
            // Local for barks (fast, cheap)
            AiTaskType::Bark => "ollama",
        }
    }

    /// Get cache TTL in hours
    pub fn cache_ttl_hours(&self) -> i64 {
        match self {
            // Barks have short TTL (1 hour)
            AiTaskType::Bark => 1,
            // Everything else 24 hours
            _ => 24,
        }
    }
}

/// LLM Provider
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LlmProvider {
    OpenAi,
    Anthropic,
    Gemini,
    Ollama,
}

impl LlmProvider {
    pub fn as_str(&self) -> &'static str {
        match self {
            LlmProvider::OpenAi => "openai",
            LlmProvider::Anthropic => "anthropic",
            LlmProvider::Gemini => "gemini",
            LlmProvider::Ollama => "ollama",
        }
    }
}

impl std::str::FromStr for LlmProvider {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "openai" => Ok(LlmProvider::OpenAi),
            "anthropic" => Ok(LlmProvider::Anthropic),
            "gemini" => Ok(LlmProvider::Gemini),
            "ollama" => Ok(LlmProvider::Ollama),
            _ => Err(format!("Unknown provider: {}", s)),
        }
    }
}

/// AI generation request
#[derive(Debug, Clone, Serialize)]
pub struct GenerationRequest {
    pub request_id: String,
    pub task_type: AiTaskType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    pub prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
}

/// AI generation response
#[derive(Debug, Clone, Deserialize)]
pub struct GenerationResponse {
    pub request_id: String,
    pub content: String,
    pub tokens_used: u32,
    pub model: String,
    #[serde(default)]
    pub cached: bool,
    pub generation_time_ms: u64,
}

/// Bark request
#[derive(Debug, Clone, Serialize)]
pub struct BarkRequest {
    pub npc_name: String,
    pub npc_role: String,
    pub context: String,
    #[serde(default = "default_mood")]
    pub mood: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
}

#[allow(dead_code)]
fn default_mood() -> String {
    "neutral".to_string()
}

/// Bark response
#[derive(Debug, Clone, Deserialize)]
pub struct BarkResponse {
    pub text: String,
    #[serde(default)]
    pub confidence: f32,
}

/// Dialogue request
#[derive(Debug, Clone, Serialize)]
pub struct DialogueRequest {
    pub npc_id: String,
    pub npc_vibecode: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub player_input: Option<String>,
    #[serde(default)]
    pub conversation_history: Vec<DialogueHistoryEntry>,
    #[serde(default)]
    pub world_state: HashMap<String, serde_json::Value>,
}

/// Dialogue history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogueHistoryEntry {
    pub speaker: String,
    pub text: String,
}

/// Dialogue choice
#[derive(Debug, Clone, Deserialize)]
pub struct DialogueChoice {
    pub id: u64,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,
}

/// Dialogue response
#[derive(Debug, Clone, Deserialize)]
pub struct DialogueResponse {
    pub text: String,
    #[serde(default)]
    pub choices: Vec<DialogueChoice>,
    #[serde(default = "default_emotion")]
    pub emotion: String,
}

fn default_emotion() -> String {
    "neutral".to_string()
}

/// Sidecar client error
#[derive(Debug, thiserror::Error)]
pub enum SidecarError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Sidecar unavailable: {0}")]
    Unavailable(String),

    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    #[error("Timeout")]
    Timeout,
}

/// AI Sidecar Client
pub struct AiSidecarClient {
    http_client: reqwest::Client,
    base_url: String,
    available: bool,
}

impl AiSidecarClient {
    /// Create new sidecar client
    pub fn new(base_url: impl Into<String>) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            http_client,
            base_url: base_url.into(),
            available: false,
        }
    }

    /// Create with default localhost URL
    pub fn default_local() -> Self {
        Self::new("http://127.0.0.1:8000")
    }

    /// Check if sidecar is available
    pub async fn check_health(&mut self) -> Result<bool, SidecarError> {
        match self
            .http_client
            .get(format!("{}/health", self.base_url))
            .send()
            .await
        {
            Ok(response) => {
                let healthy = response.status().is_success();
                self.available = healthy;
                Ok(healthy)
            }
            Err(e) => {
                self.available = false;
                Err(SidecarError::Http(e))
            }
        }
    }

    /// Check if sidecar was last known to be available
    pub fn is_available(&self) -> bool {
        self.available
    }

    /// Generate content
    pub async fn generate(
        &self,
        request: GenerationRequest,
    ) -> Result<GenerationResponse, SidecarError> {
        if !self.available {
            return Err(SidecarError::Unavailable("Sidecar not available".into()));
        }

        let response = self
            .http_client
            .post(format!("{}/generate", self.base_url))
            .json(&request)
            .send()
            .await
            .map_err(SidecarError::Http)?;

        if !response.status().is_success() {
            return Err(SidecarError::InvalidResponse(format!(
                "HTTP {}",
                response.status()
            )));
        }

        let result = response
            .json::<GenerationResponse>()
            .await
            .map_err(SidecarError::Http)?;

        Ok(result)
    }

    /// Generate a bark (short NPC line)
    pub async fn generate_bark(&self, request: BarkRequest) -> Result<BarkResponse, SidecarError> {
        if !self.available {
            // Fallback to templates
            return Ok(BarkResponse {
                text: get_template_bark(&request.context, &request.mood),
                confidence: 0.5,
            });
        }

        let response = self
            .http_client
            .post(format!("{}/bark", self.base_url))
            .json(&request)
            .send()
            .await;

        match response {
            Ok(response) if response.status().is_success() => {
                let result = response
                    .json::<BarkResponse>()
                    .await
                    .map_err(SidecarError::Http)?;
                Ok(result)
            }
            _ => {
                // Fallback to templates on error
                Ok(BarkResponse {
                    text: get_template_bark(&request.context, &request.mood),
                    confidence: 0.5,
                })
            }
        }
    }

    /// Generate dialogue response
    pub async fn generate_dialogue(
        &self,
        request: DialogueRequest,
    ) -> Result<DialogueResponse, SidecarError> {
        if !self.available {
            return Ok(DialogueResponse {
                text: "...".to_string(),
                choices: vec![],
                emotion: "neutral".to_string(),
            });
        }

        let response = self
            .http_client
            .post(format!("{}/dialogue", self.base_url))
            .json(&request)
            .send()
            .await
            .map_err(SidecarError::Http)?;

        if !response.status().is_success() {
            return Err(SidecarError::InvalidResponse(format!(
                "HTTP {}",
                response.status()
            )));
        }

        let result = response
            .json::<DialogueResponse>()
            .await
            .map_err(SidecarError::Http)?;

        Ok(result)
    }

    /// Get cache statistics
    pub async fn get_cache_stats(&self) -> Result<CacheStats, SidecarError> {
        if !self.available {
            return Err(SidecarError::Unavailable("Sidecar not available".into()));
        }

        let response = self
            .http_client
            .get(format!("{}/cache/stats", self.base_url))
            .send()
            .await
            .map_err(SidecarError::Http)?;

        let stats = response
            .json::<CacheStats>()
            .await
            .map_err(SidecarError::Http)?;

        Ok(stats)
    }

    /// Clear the cache
    pub async fn clear_cache(&self) -> Result<(), SidecarError> {
        if !self.available {
            return Err(SidecarError::Unavailable("Sidecar not available".into()));
        }

        self.http_client
            .delete(format!("{}/cache/clear", self.base_url))
            .send()
            .await
            .map_err(SidecarError::Http)?;

        Ok(())
    }
}

/// Cache statistics
#[derive(Debug, Clone, Deserialize)]
pub struct CacheStats {
    pub total_entries: u64,
    pub valid_entries: u64,
    pub expired_entries: u64,
    pub total_tokens: u64,
    pub by_model: HashMap<String, u64>,
}

/// Template-based bark fallback
fn get_template_bark(context: &str, mood: &str) -> String {
    use std::sync::OnceLock;

    static TEMPLATES: OnceLock<HashMap<&str, Vec<&str>>> = OnceLock::new();

    let templates = TEMPLATES.get_or_init(|| {
        let mut map = HashMap::new();
        map.insert(
            "greeting",
            vec![
                "Greetings, traveler.",
                "Well met!",
                "Hello there.",
                "Welcome to these parts.",
            ],
        );
        map.insert(
            "danger",
            vec![
                "Be careful around here.",
                "Danger lurks nearby.",
                "Watch your step.",
                "Stay alert!",
            ],
        );
        map.insert(
            "weather",
            vec![
                "Fine weather we're having.",
                "Storm's coming, I can feel it.",
                "Bit chilly today, isn't it?",
                "Perfect day for traveling.",
            ],
        );
        map.insert(
            "trade",
            vec![
                "Looking to buy or sell?",
                "Got some fine goods here.",
                "Best prices in town!",
                "What can I get for you?",
            ],
        );
        map.insert(
            "lore",
            vec![
                "They say these ruins are ancient...",
                "Legend speaks of a great treasure.",
                "My grandmother told me stories...",
                "This place has a dark history.",
            ],
        );
        map
    });

    // Determine category
    let category = if context.to_lowercase().contains("danger")
        || context.to_lowercase().contains("enemy")
        || context.to_lowercase().contains("monster")
        || context.to_lowercase().contains("fight")
    {
        "danger"
    } else if context.to_lowercase().contains("weather")
        || context.to_lowercase().contains("rain")
        || context.to_lowercase().contains("storm")
    {
        "weather"
    } else if context.to_lowercase().contains("buy")
        || context.to_lowercase().contains("sell")
        || context.to_lowercase().contains("trade")
        || context.to_lowercase().contains("gold")
    {
        "trade"
    } else if context.to_lowercase().contains("story")
        || context.to_lowercase().contains("legend")
        || context.to_lowercase().contains("history")
        || context.to_lowercase().contains("ancient")
    {
        "lore"
    } else {
        "greeting"
    };

    // Select random template
    let category_templates = templates.get(category).unwrap_or(&templates["greeting"]);
    let idx = (context.len() + mood.len()) % category_templates.len();
    let text = category_templates[idx];

    // Ensure short (< 80 chars)
    if text.len() > 80 {
        format!("{}...", &text[..77])
    } else {
        text.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_bark() {
        let bark = get_template_bark("danger", "neutral");
        assert!(!bark.is_empty());
        assert!(bark.len() <= 80);
    }

    #[test]
    fn test_task_type_model_routing() {
        assert_eq!(AiTaskType::Bark.preferred_model(), "ollama");
        assert_eq!(AiTaskType::Dialogue.preferred_model(), "gemini");
        assert_eq!(AiTaskType::Shader.preferred_model(), "anthropic");
    }
}
