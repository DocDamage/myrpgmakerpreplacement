# DDE AI Sidecar

A Python FastAPI service that provides LLM capabilities for the DocDamage Engine.

## Features

- **Multi-Model Support**: OpenAI GPT-4, Anthropic Claude, Google Gemini, local Ollama/Llama3
- **Smart Routing**: Different models for different tasks:
  - Claude: Code-heavy tasks (shaders, balancing)
  - Gemini: Narrative content (dialogue, lore)
  - Llama3 (local): Fast barks and ambient dialogue
- **Caching**: SQLite-based response caching with prompt hash deduplication
- **Fallback**: Template-based responses when AI is unavailable

## Quick Start

```bash
# Install dependencies
pip install -r requirements.txt

# Run the server
python main.py

# Or with auto-reload for development
python main.py --reload
```

The server will start on `http://127.0.0.1:8000`.

## API Endpoints

### Health Check
```bash
GET /health
```

### Generate Content
```bash
POST /generate
{
  "request_id": "uuid",
  "task_type": "dialogue|bark|narrative|balancing|shader",
  "model": "openai|anthropic|gemini|ollama",
  "prompt": "Generate a greeting for a merchant NPC",
  "max_tokens": 500,
  "temperature": 0.7
}
```

### Generate Bark (NPC Ambient Dialogue)
```bash
POST /bark
{
  "npc_name": "Grom",
  "npc_role": "merchant",
  "context": "Player approaching shop",
  "mood": "cheerful"
}
```

### Generate Dialogue
```bash
POST /dialogue
{
  "npc_id": "npc_123",
  "npc_vibecode": { ... },
  "player_input": "What do you sell?",
  "conversation_history": [],
  "world_state": {}
}
```

### Cache Management
```bash
GET /cache/stats
DELETE /cache/clear
```

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `DDE_SIDECAR_DB` | Path to SQLite cache database | `sidecar_cache.db` |
| `OPENAI_API_KEY` | OpenAI API key | - |
| `ANTHROPIC_API_KEY` | Anthropic API key | - |
| `GEMINI_API_KEY` | Google Gemini API key | - |

## Architecture

```
┌─────────────┐     HTTP      ┌─────────────────┐
│  DDE Engine │ ◄────────────► │  Python Sidecar │
│   (Rust)    │               │   (FastAPI)     │
└─────────────┘               └────────┬────────┘
                                       │
                    ┌──────────────────┼──────────────────┐
                    ▼                  ▼                  ▼
              ┌──────────┐      ┌──────────┐      ┌──────────┐
              │  OpenAI  │      │Anthropic │      │  Ollama  │
              │  (GPT-4) │      │ (Claude) │      │ (Local)  │
              └──────────┘      └──────────┘      └──────────┘
```

## Integration with DDE Engine

The Rust client in `crates/dde-ai` communicates with this sidecar:

```rust
use dde_ai::AiSidecarClient;

let mut client = AiSidecarClient::default_local();

// Check if sidecar is available
if client.check_health().await? {
    // Generate a bark
    let response = client.generate_bark(BarkRequest {
        npc_name: "Grom".to_string(),
        npc_role: "merchant".to_string(),
        context: "shop greeting".to_string(),
        mood: "friendly".to_string(),
        ..Default::default()
    }).await?;
    
    println!("Bark: {}", response.text);
}
```

## Bark Templates

When AI is unavailable, the sidecar falls back to templates:

- **Greeting**: "Greetings, traveler.", "Well met!", etc.
- **Danger**: "Be careful around here.", "Stay alert!", etc.
- **Weather**: "Fine weather we're having.", etc.
- **Trade**: "Looking to buy or sell?", etc.
- **Lore**: "They say these ruins are ancient...", etc.

## Development

```bash
# Run tests
pytest

# Format code
black main.py

# Type checking
mypy main.py
```

## License

MIT OR Apache-2.0
