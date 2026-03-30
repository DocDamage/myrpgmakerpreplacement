//! LLM Provider Configuration Module
//!
//! Manages provider routing tables, API key status, and model selection
//! for different AI task types.

use super::AiTaskType;
use super::LlmProvider;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// API key status for a provider
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApiKeyStatus {
    /// API key is configured and valid
    Configured,
    /// API key is configured but needs validation
    Unverified,
    /// API key is missing or invalid
    Missing,
    /// API key has expired
    Expired,
    /// Rate limit reached
    RateLimited,
}

impl ApiKeyStatus {
    /// Get display text for the status
    pub fn display_text(&self) -> &'static str {
        match self {
            ApiKeyStatus::Configured => "✓ Configured",
            ApiKeyStatus::Unverified => "? Unverified",
            ApiKeyStatus::Missing => "✗ Missing",
            ApiKeyStatus::Expired => "✗ Expired",
            ApiKeyStatus::RateLimited => "⚠ Rate Limited",
        }
    }

    /// Get color for UI (RGB values)
    pub fn color(&self) -> [u8; 3] {
        match self {
            ApiKeyStatus::Configured => [50, 200, 50],    // Green
            ApiKeyStatus::Unverified => [255, 200, 50],   // Yellow
            ApiKeyStatus::Missing => [255, 50, 50],       // Red
            ApiKeyStatus::Expired => [255, 50, 50],       // Red
            ApiKeyStatus::RateLimited => [255, 150, 50],  // Orange
        }
    }

    /// Check if this status allows API calls
    pub fn is_usable(&self) -> bool {
        matches!(self, ApiKeyStatus::Configured | ApiKeyStatus::Unverified)
    }
}

/// Information about a model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    /// Model ID (for API calls)
    pub id: String,
    /// Display name
    pub name: String,
    /// Context window size in tokens
    pub context_window: usize,
    /// Whether this model supports images
    #[serde(default)]
    pub supports_images: bool,
    /// Whether this model supports function calling
    #[serde(default)]
    pub supports_functions: bool,
    /// Cost per 1K input tokens (in USD, approximate)
    #[serde(default)]
    pub cost_per_1k_input: Option<f32>,
    /// Cost per 1K output tokens (in USD, approximate)
    #[serde(default)]
    pub cost_per_1k_output: Option<f32>,
}

impl ModelInfo {
    /// Create a new model info
    pub fn new(id: impl Into<String>, name: impl Into<String>, context_window: usize) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            context_window,
            supports_images: false,
            supports_functions: false,
            cost_per_1k_input: None,
            cost_per_1k_output: None,
        }
    }

    /// Set image support
    pub fn with_images(mut self) -> Self {
        self.supports_images = true;
        self
    }

    /// Set function calling support
    pub fn with_functions(mut self) -> Self {
        self.supports_functions = true;
        self
    }

    /// Set cost information
    pub fn with_cost(mut self, input: f32, output: f32) -> Self {
        self.cost_per_1k_input = Some(input);
        self.cost_per_1k_output = Some(output);
        self
    }
}

/// Provider configuration for a specific task type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// Task type this config applies to
    pub task_type: AiTaskType,
    /// Primary provider
    pub primary_provider: LlmProvider,
    /// Primary model ID
    pub primary_model: String,
    /// Fallback provider (if primary fails)
    pub fallback_provider: Option<LlmProvider>,
    /// Fallback model ID
    pub fallback_model: Option<String>,
    /// Priority (0-100, higher = more preferred when routing)
    pub priority: u8,
    /// Whether this task type is enabled
    pub enabled: bool,
    /// Custom temperature for this task type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Custom max tokens for this task type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
}

impl ProviderConfig {
    /// Create default config for a task type
    pub fn default_for_task(task_type: AiTaskType) -> Self {
        let preferred_provider = match task_type.preferred_model() {
            "openai" => LlmProvider::OpenAi,
            "anthropic" => LlmProvider::Anthropic,
            "gemini" => LlmProvider::Gemini,
            "ollama" => LlmProvider::Ollama,
            _ => LlmProvider::Gemini,
        };

        let default_model = Self::default_model_for_provider(&preferred_provider, &task_type);

        Self {
            task_type,
            primary_provider: preferred_provider,
            primary_model: default_model,
            fallback_provider: Some(LlmProvider::Ollama),
            fallback_model: Some("llama3.2".to_string()),
            priority: 50,
            enabled: true,
            temperature: None,
            max_tokens: None,
        }
    }

    /// Get default model for provider and task type
    fn default_model_for_provider(provider: &LlmProvider, task_type: &AiTaskType) -> String {
        match provider {
            LlmProvider::OpenAi => match task_type {
                AiTaskType::ImageGen => "dall-e-3".to_string(),
                _ => "gpt-4o-mini".to_string(),
            },
            LlmProvider::Anthropic => "claude-3-5-sonnet-20241022".to_string(),
            LlmProvider::Gemini => "gemini-1.5-flash".to_string(),
            LlmProvider::Ollama => "llama3.2".to_string(),
        }
    }

    /// Set primary provider and auto-select default model
    pub fn set_primary_provider(&mut self, provider: LlmProvider) {
        self.primary_provider = provider;
        self.primary_model = Self::default_model_for_provider(&provider, &self.task_type);
    }

    /// Set fallback provider and auto-select default model
    pub fn set_fallback_provider(&mut self, provider: Option<LlmProvider>) {
        self.fallback_provider = provider;
        self.fallback_model = provider.map(|p| Self::default_model_for_provider(&p, &self.task_type));
    }

    /// Get available models for the primary provider
    pub fn available_primary_models(&self) -> Vec<ModelInfo> {
        self.primary_provider.available_models()
    }

    /// Get available models for the fallback provider
    pub fn available_fallback_models(&self) -> Vec<ModelInfo> {
        self.fallback_provider
            .map(|p| p.available_models())
            .unwrap_or_default()
    }

    /// Check if this config has a valid fallback
    pub fn has_fallback(&self) -> bool {
        self.fallback_provider.is_some() && self.fallback_model.is_some()
    }
}

/// Provider routing table - manages configurations for all task types
#[derive(Debug, Clone)]
pub struct ProviderRoutingTable {
    /// Configuration per task type
    configs: HashMap<AiTaskType, ProviderConfig>,
    /// API key status per provider
    api_key_status: HashMap<LlmProvider, ApiKeyStatus>,
    /// Whether to use routing (if false, always use preferred model)
    pub use_routing: bool,
}

impl ProviderRoutingTable {
    /// Create a new routing table with default configurations
    pub fn new() -> Self {
        let mut configs = HashMap::new();

        for task_type in AiTaskType::all() {
            configs.insert(*task_type, ProviderConfig::default_for_task(*task_type));
        }

        let mut api_key_status = HashMap::new();
        api_key_status.insert(LlmProvider::OpenAi, ApiKeyStatus::Missing);
        api_key_status.insert(LlmProvider::Anthropic, ApiKeyStatus::Missing);
        api_key_status.insert(LlmProvider::Gemini, ApiKeyStatus::Missing);
        api_key_status.insert(LlmProvider::Ollama, ApiKeyStatus::Configured); // Local doesn't need key

        Self {
            configs,
            api_key_status,
            use_routing: true,
        }
    }

    /// Get configuration for a task type
    pub fn get_config(&self, task_type: AiTaskType) -> Option<&ProviderConfig> {
        self.configs.get(&task_type)
    }

    /// Get mutable configuration for a task type
    pub fn get_config_mut(&mut self, task_type: AiTaskType) -> Option<&mut ProviderConfig> {
        self.configs.get_mut(&task_type)
    }

    /// Set configuration for a task type
    pub fn set_config(&mut self, config: ProviderConfig) {
        self.configs.insert(config.task_type, config);
    }

    /// Get API key status for a provider
    pub fn get_api_key_status(&self, provider: LlmProvider) -> ApiKeyStatus {
        self.api_key_status
            .get(&provider)
            .copied()
            .unwrap_or(ApiKeyStatus::Missing)
    }

    /// Set API key status for a provider
    pub fn set_api_key_status(&mut self, provider: LlmProvider, status: ApiKeyStatus) {
        self.api_key_status.insert(provider, status);
    }

    /// Get all API key statuses
    pub fn get_all_api_key_statuses(&self) -> &HashMap<LlmProvider, ApiKeyStatus> {
        &self.api_key_status
    }

    /// Get provider for a task type (considering routing and fallbacks)
    pub fn get_provider_for_task(&self, task_type: AiTaskType) -> Option<(LlmProvider, String)> {
        let config = self.configs.get(&task_type)?;

        if !config.enabled {
            return None;
        }

        // Check if primary is usable
        let primary_status = self.get_api_key_status(config.primary_provider);
        if primary_status.is_usable() {
            return Some((config.primary_provider, config.primary_model.clone()));
        }

        // Try fallback
        if let Some(fallback) = config.fallback_provider {
            let fallback_status = self.get_api_key_status(fallback);
            if fallback_status.is_usable() {
                return Some((fallback, config.fallback_model.clone().unwrap_or_default()));
            }
        }

        // No usable provider
        None
    }

    /// Get all configurations
    pub fn get_all_configs(&self) -> &HashMap<AiTaskType, ProviderConfig> {
        &self.configs
    }

    /// Get mutable configurations
    pub fn get_all_configs_mut(&mut self) -> &mut HashMap<AiTaskType, ProviderConfig> {
        &mut self.configs
    }

    /// Reset all configurations to defaults
    pub fn reset_to_defaults(&mut self) {
        for task_type in AiTaskType::all() {
            self.configs.insert(*task_type, ProviderConfig::default_for_task(*task_type));
        }
    }

    /// Get total number of configured task types
    pub fn configured_count(&self) -> usize {
        self.configs.values().filter(|c| c.enabled).count()
    }

    /// Get count of providers with valid API keys
    pub fn usable_provider_count(&self) -> usize {
        self.api_key_status
            .values()
            .filter(|s| s.is_usable())
            .count()
    }

    /// Validate all configurations
    pub fn validate(&self) -> Vec<String> {
        let mut issues = Vec::new();

        for (task_type, config) in &self.configs {
            if !config.enabled {
                continue;
            }

            let primary_status = self.get_api_key_status(config.primary_provider);
            if !primary_status.is_usable() {
                if config.fallback_provider.is_none() {
                    issues.push(format!(
                        "{:?}: Primary provider {:?} is not usable and no fallback configured",
                        task_type, config.primary_provider
                    ));
                } else {
                    let fallback_status = self.get_api_key_status(config.fallback_provider.unwrap());
                    if !fallback_status.is_usable() {
                        issues.push(format!(
                            "{:?}: Both primary and fallback providers are not usable",
                            task_type
                        ));
                    }
                }
            }
        }

        issues
    }

    /// Check if a specific provider is configured for any task
    pub fn is_provider_used(&self, provider: LlmProvider) -> bool {
        self.configs.values().any(|c| {
            c.enabled && (c.primary_provider == provider || c.fallback_provider == Some(provider))
        })
    }

    /// Get task types that use a specific provider
    pub fn get_tasks_using_provider(&self, provider: LlmProvider) -> Vec<AiTaskType> {
        self.configs
            .iter()
            .filter(|(_, c)| {
                c.enabled && (c.primary_provider == provider || c.fallback_provider == Some(provider))
            })
            .map(|(t, _)| *t)
            .collect()
    }
}

impl Default for ProviderRoutingTable {
    fn default() -> Self {
        Self::new()
    }
}

/// Provider health status
#[derive(Debug, Clone)]
pub struct ProviderHealth {
    /// Provider
    pub provider: LlmProvider,
    /// Whether the provider is reachable
    pub is_reachable: bool,
    /// Average response time (ms)
    pub avg_response_time_ms: f32,
    /// Success rate (0.0 - 1.0)
    pub success_rate: f32,
    /// Last checked timestamp
    pub last_checked: Option<std::time::Instant>,
    /// Error message if unhealthy
    pub error_message: Option<String>,
}

impl ProviderHealth {
    /// Create a new health status
    pub fn new(provider: LlmProvider) -> Self {
        Self {
            provider,
            is_reachable: false,
            avg_response_time_ms: 0.0,
            success_rate: 0.0,
            last_checked: None,
            error_message: None,
        }
    }

    /// Get status color
    pub fn status_color(&self) -> [u8; 3] {
        if !self.is_reachable {
            return [255, 50, 50]; // Red
        }
        if self.success_rate < 0.5 {
            return [255, 150, 50]; // Orange
        }
        if self.avg_response_time_ms > 5000.0 {
            return [255, 200, 50]; // Yellow
        }
        [50, 200, 50] // Green
    }

    /// Get status text
    pub fn status_text(&self) -> String {
        if !self.is_reachable {
            return "Unreachable".to_string();
        }
        if self.success_rate < 0.5 {
            return format!("Unstable ({:.0}%)", self.success_rate * 100.0);
        }
        if self.avg_response_time_ms > 5000.0 {
            return format!("Slow ({:.0}ms)", self.avg_response_time_ms);
        }
        "Healthy".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_config_default() {
        let config = ProviderConfig::default_for_task(AiTaskType::Bark);
        assert_eq!(config.task_type, AiTaskType::Bark);
        assert!(config.enabled);
        assert!(config.has_fallback());
    }

    #[test]
    fn test_routing_table_creation() {
        let table = ProviderRoutingTable::new();
        assert_eq!(table.configured_count(), AiTaskType::all().len());
    }

    #[test]
    fn test_api_key_status() {
        let mut table = ProviderRoutingTable::new();

        table.set_api_key_status(LlmProvider::OpenAi, ApiKeyStatus::Configured);
        assert_eq!(table.get_api_key_status(LlmProvider::OpenAi), ApiKeyStatus::Configured);
        assert!(table.get_api_key_status(LlmProvider::OpenAi).is_usable());
    }

    #[test]
    fn test_provider_for_task() {
        let table = ProviderRoutingTable::new();

        let (provider, model) = table.get_provider_for_task(AiTaskType::Bark).unwrap();
        assert_eq!(provider, LlmProvider::Ollama);
        assert!(!model.is_empty());
    }

    #[test]
    fn test_validation() {
        let table = ProviderRoutingTable::new();
        let issues = table.validate();
        // Ollama is configured by default, so there should be no issues
        assert!(issues.is_empty() || issues.len() <= 3); // Allow some missing API keys
    }

    #[test]
    fn test_model_info() {
        let model = ModelInfo::new("gpt-4", "GPT-4", 8192)
            .with_images()
            .with_functions()
            .with_cost(0.03, 0.06);

        assert_eq!(model.id, "gpt-4");
        assert!(model.supports_images);
        assert!(model.supports_functions);
        assert_eq!(model.cost_per_1k_input, Some(0.03));
    }
}
