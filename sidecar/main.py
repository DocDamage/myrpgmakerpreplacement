#!/usr/bin/env python3
"""
DocDamage Engine - AI Sidecar

A FastAPI service that handles LLM requests for the DDE engine.
Supports multiple models: OpenAI (GPT-4), Anthropic (Claude), Google (Gemini), local (Ollama/Llama3)
"""

import os
import hashlib
import json
import sqlite3
from datetime import datetime, timedelta
from typing import Optional, List, Dict, Any
from contextlib import asynccontextmanager

from fastapi import FastAPI, HTTPException, BackgroundTasks
from fastapi.responses import StreamingResponse
from pydantic import BaseModel, Field
import uvicorn

# Model definitions
class GenerationRequest(BaseModel):
    request_id: str
    task_type: str = Field(..., description="dialogue, bark, narrative, balancing, shader")
    model: str = Field(default="openai", description="openai, anthropic, gemini, ollama")
    prompt: str
    context: Optional[Dict[str, Any]] = None
    max_tokens: int = 500
    temperature: float = 0.7
    
class GenerationResponse(BaseModel):
    request_id: str
    content: str
    tokens_used: int
    model: str
    cached: bool = False
    generation_time_ms: int

class BarkRequest(BaseModel):
    npc_name: str
    npc_role: str
    context: str
    mood: str = "neutral"
    location: Optional[str] = None
    
class BarkResponse(BaseModel):
    text: str
    confidence: float

class DialogueRequest(BaseModel):
    npc_id: str
    npc_vibecode: Dict[str, Any]
    player_input: Optional[str] = None
    conversation_history: List[Dict[str, str]] = []
    world_state: Dict[str, Any]

class DialogueResponse(BaseModel):
    text: str
    choices: List[Dict[str, Any]] = []
    emotion: str = "neutral"

# Global state
app = FastAPI(title="DDE AI Sidecar", version="0.1.0")
db_path = os.getenv("DDE_SIDECAR_DB", "sidecar_cache.db")

# Cache database setup
def init_db():
    """Initialize SQLite cache database"""
    conn = sqlite3.connect(db_path)
    cursor = conn.cursor()
    
    cursor.execute("""
        CREATE TABLE IF NOT EXISTS llm_cache (
            prompt_hash TEXT PRIMARY KEY,
            model TEXT NOT NULL,
            response TEXT NOT NULL,
            token_cost INTEGER DEFAULT 0,
            created_at INTEGER NOT NULL,
            expires_at INTEGER NOT NULL
        )
    """)
    
    cursor.execute("""
        CREATE INDEX IF NOT EXISTS idx_cache_model ON llm_cache(model)
    """)
    
    cursor.execute("""
        CREATE INDEX IF NOT EXISTS idx_cache_expires ON llm_cache(expires_at)
    """)
    
    conn.commit()
    conn.close()

def get_cache_key(model: str, prompt: str) -> str:
    """Generate cache key from model and prompt"""
    return hashlib.sha256(f"{model}:{prompt}".encode()).hexdigest()

def get_cached_response(prompt_hash: str) -> Optional[Dict]:
    """Get cached response if not expired"""
    conn = sqlite3.connect(db_path)
    cursor = conn.cursor()
    
    now = int(datetime.utcnow().timestamp() * 1000)
    cursor.execute(
        "SELECT model, response, token_cost FROM llm_cache WHERE prompt_hash = ? AND expires_at > ?",
        (prompt_hash, now)
    )
    
    result = cursor.fetchone()
    conn.close()
    
    if result:
        return {
            "model": result[0],
            "response": result[1],
            "token_cost": result[2]
        }
    return None

def cache_response(prompt_hash: str, model: str, response: str, token_cost: int, ttl_hours: int = 24):
    """Cache a response with TTL"""
    conn = sqlite3.connect(db_path)
    cursor = conn.cursor()
    
    now = int(datetime.utcnow().timestamp() * 1000)
    expires = int((datetime.utcnow() + timedelta(hours=ttl_hours)).timestamp() * 1000)
    
    cursor.execute(
        """INSERT OR REPLACE INTO llm_cache 
           (prompt_hash, model, response, token_cost, created_at, expires_at)
           VALUES (?, ?, ?, ?, ?, ?)""",
        (prompt_hash, model, response, token_cost, now, expires)
    )
    
    conn.commit()
    conn.close()

# Template fallback system
BARK_TEMPLATES = {
    "greeting": [
        "Greetings, traveler.",
        "Well met!",
        "Hello there.",
        "Welcome to these parts.",
    ],
    "danger": [
        "Be careful around here.",
        "Danger lurks nearby.",
        "Watch your step.",
        "Stay alert!",
    ],
    "weather": [
        "Fine weather we're having.",
        "Storm's coming, I can feel it.",
        "Bit chilly today, isn't it?",
        "Perfect day for traveling.",
    ],
    "trade": [
        "Looking to buy or sell?",
        "Got some fine goods here.",
        "Best prices in town!",
        "What can I get for you?",
    ],
    "lore": [
        "They say these ruins are ancient...",
        "Legend speaks of a great treasure.",
        "My grandmother told me stories...",
        "This place has a dark history.",
    ],
}

def get_template_bark(context: str, mood: str) -> str:
    """Get a template-based bark when AI is unavailable"""
    import random
    
    # Map context/mood to template category
    category = "greeting"  # default
    
    if any(word in context.lower() for word in ["danger", "enemy", "monster", "fight"]):
        category = "danger"
    elif any(word in context.lower() for word in ["weather", "rain", "storm", "sun"]):
        category = "weather"
    elif any(word in context.lower() for word in ["buy", "sell", "trade", "gold", "price"]):
        category = "trade"
    elif any(word in context.lower() for word in ["story", "legend", "history", "ancient"]):
        category = "lore"
    
    templates = BARK_TEMPLATES.get(category, BARK_TEMPLATES["greeting"])
    return random.choice(templates)

@app.on_event("startup")
async def startup():
    """Initialize on startup"""
    init_db()
    print(f"DDE AI Sidecar started")
    print(f"Cache database: {db_path}")

@app.get("/health")
async def health_check():
    """Health check endpoint"""
    return {"status": "healthy", "service": "dde-ai-sidecar", "version": "0.1.0"}

@app.post("/generate", response_model=GenerationResponse)
async def generate(request: GenerationRequest, background_tasks: BackgroundTasks):
    """Generate content using specified LLM"""
    import time
    
    start_time = time.time()
    
    # Check cache first
    cache_key = get_cache_key(request.model, request.prompt)
    cached = get_cached_response(cache_key)
    
    if cached:
        return GenerationResponse(
            request_id=request.request_id,
            content=cached["response"],
            tokens_used=cached["token_cost"],
            model=cached["model"],
            cached=True,
            generation_time_ms=int((time.time() - start_time) * 1000)
        )
    
    # TODO: Implement actual LLM calls
    # For now, return a placeholder response
    content = f"[Generated content for {request.task_type} task using {request.model}]"
    tokens_used = len(content.split())
    
    # Cache the response
    background_tasks.add_task(
        cache_response, 
        cache_key, 
        request.model, 
        content, 
        tokens_used,
        24 if request.task_type != "bark" else 1  # Shorter TTL for barks
    )
    
    return GenerationResponse(
        request_id=request.request_id,
        content=content,
        tokens_used=tokens_used,
        model=request.model,
        cached=False,
        generation_time_ms=int((time.time() - start_time) * 1000)
    )

@app.post("/bark", response_model=BarkResponse)
async def generate_bark(request: BarkRequest):
    """Generate a short NPC bark (ambient dialogue)"""
    import time
    import random
    
    start_time = time.time()
    
    # Try AI generation first (placeholder)
    # If AI fails or is unavailable, use templates
    
    text = get_template_bark(request.context, request.mood)
    
    # Ensure it's short (< 80 chars as per blueprint)
    if len(text) > 80:
        text = text[:77] + "..."
    
    generation_time = (time.time() - start_time) * 1000
    
    return BarkResponse(
        text=text,
        confidence=0.8 if generation_time < 80 else 0.5  # Fast = higher confidence
    )

@app.post("/dialogue", response_model=DialogueResponse)
async def generate_dialogue(request: DialogueRequest):
    """Generate dialogue response for an NPC"""
    
    # Parse Vibecode
    vibecode = request.npc_vibecode
    personality = vibecode.get("identity", {}).get("personality", ["neutral"])
    speech_style = vibecode.get("identity", {}).get("speech_style", "normal")
    
    # Generate response based on NPC personality
    name = vibecode.get("identity", {}).get("name", "NPC")
    
    # Simple template-based response for now
    if request.player_input:
        text = f"You say '{request.player_input}'? Interesting..."
    else:
        text = f"Greetings, traveler. I am {name}."
    
    # Add personality flavor
    if "noble" in personality:
        text = f"*adjusts collar* {text}"
    elif "friendly" in personality:
        text = f"*smiles warmly* {text}"
    elif "gruff" in personality:
        text = f"*grunts* {text}"
    
    return DialogueResponse(
        text=text,
        choices=[
            {"id": 1, "text": "Tell me more.", "condition": None},
            {"id": 2, "text": "Goodbye.", "condition": None},
        ],
        emotion="neutral"
    )

@app.get("/cache/stats")
async def get_cache_stats():
    """Get cache statistics"""
    conn = sqlite3.connect(db_path)
    cursor = conn.cursor()
    
    cursor.execute("SELECT COUNT(*), SUM(token_cost) FROM llm_cache")
    total_entries, total_tokens = cursor.fetchone()
    
    now = int(datetime.utcnow().timestamp() * 1000)
    cursor.execute("SELECT COUNT(*) FROM llm_cache WHERE expires_at > ?", (now,))
    valid_entries = cursor.fetchone()[0]
    
    cursor.execute("SELECT model, COUNT(*) FROM llm_cache GROUP BY model")
    by_model = {row[0]: row[1] for row in cursor.fetchall()}
    
    conn.close()
    
    return {
        "total_entries": total_entries or 0,
        "valid_entries": valid_entries or 0,
        "expired_entries": (total_entries or 0) - (valid_entries or 0),
        "total_tokens": total_tokens or 0,
        "by_model": by_model
    }

@app.delete("/cache/clear")
async def clear_cache():
    """Clear all cached responses"""
    conn = sqlite3.connect(db_path)
    cursor = conn.cursor()
    cursor.execute("DELETE FROM llm_cache")
    conn.commit()
    conn.close()
    return {"status": "cleared"}

if __name__ == "__main__":
    # Run with: python main.py
    # Or with reload: python main.py --reload
    uvicorn.run(
        "main:app",
        host="127.0.0.1",
        port=8000,
        reload=True,
        log_level="info"
    )
