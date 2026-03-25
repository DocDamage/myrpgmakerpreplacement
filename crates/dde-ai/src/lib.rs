//! DocDamage Engine - AI Sidecar Client
//! 
//! Communicates with Python FastAPI sidecar for LLM requests.

use std::collections::HashMap;

/// AI task types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AiTaskType {
    Dialogue,
    Bark,
    Narrative,
    Balancing,
    Shader,
}

/// AI request
#[derive(Debug, Clone)]
pub struct AiRequest {
    pub id: u64,
    pub task_type: AiTaskType,
    pub context: String,
    pub prompt: String,
}

/// AI response
#[derive(Debug, Clone)]
pub struct AiResponse {
    pub request_id: u64,
    pub content: String,
    pub tokens_used: u32,
}

/// AI router
pub struct AiRouter {
    sidecar_url: String,
    pending_requests: HashMap<u64, AiRequest>,
    next_id: u64,
}

impl AiRouter {
    pub fn new(sidecar_url: impl Into<String>) -> Self {
        Self {
            sidecar_url: sidecar_url.into(),
            pending_requests: HashMap::new(),
            next_id: 1,
        }
    }
    
    pub fn send_request(&mut self, task_type: AiTaskType, context: String, prompt: String) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        
        let request = AiRequest {
            id,
            task_type,
            context,
            prompt,
        };
        
        self.pending_requests.insert(id, request);
        
        // TODO: Actually send to sidecar
        tracing::info!("AI request {} queued for {:?}", id, task_type);
        
        id
    }
    
    pub fn poll_responses(&mut self) -> Vec<AiResponse> {
        // TODO: Check for responses from sidecar
        Vec::new()
    }
    
    pub fn is_available(&self) -> bool {
        // TODO: Health check
        true
    }
}
