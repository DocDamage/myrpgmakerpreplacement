# DocDamage Engine (DDE) - Development Status

## Project Overview

This is the **DocDamage Engine (DDE)** - a desktop RPG maker and simulation engine built in Rust, as specified in the Unified Master Blueprint v4.0.

### Architecture

The engine follows a modular workspace architecture with the following crates:

| Crate | Purpose | Status |
|-------|---------|--------|
| `dde-core` | ECS, components, events, resources, systems | ✅ Foundation complete |
| `dde-db` | SQLite persistence, migrations, queries | ✅ Foundation complete |
| `dde-render` | wgpu rendering pipeline | ✅ Foundation complete |
| `dde-editor` | egui-based editor mode | ✅ Foundation complete |
| `dde-battle` | ATB battle system | ✅ Foundation complete |
| `dde-audio` | kira-based stem mixer | ✅ Foundation complete |
| `dde-ai` | Python FastAPI sidecar client | ✅ Foundation complete |
| `dde-export` | MZ export, packaging | ✅ Foundation complete |
| `dde-asset-forge` | Asset Forge integration, webview, Asset OS | ✅ Foundation complete |
| `dde-lua` | Lua scripting with mlua, sandboxed API | ✅ Foundation complete |

## Week 1 & 2 Deliverables ✅ COMPLETE

### Week 1

| Deliverable | Status | Notes |
|-------------|--------|-------|
| wgpu window + render loop | ✅ | 1280x720 window, clear color, FPS counter |
| Isometric tile renderer | ✅ | 64×64 grid, 32px tiles, vertex buffers |
| SQLite schema v1 | ✅ | All blueprint tables, WAL mode, integrity checks |
| Fixed 20Hz sim tick | ✅ | Decoupled from render, catch-up protection |
| Event bus skeleton | ✅ | crossbeam-channel, typed events |
| Basic camera | ✅ | Smooth follow, centered on world |
| Project file handling | ✅ | .dde SQLite files, CLI interface |

### Week 2

| Deliverable | Status | Notes |
|-------------|--------|-------|
| ECS integration | ✅ | hecs entities, components, queries |
| Player entity | ✅ | Spawn at center, keyboard movement |
| Tile collision | ✅ | Walkable checks, blocked edges |
| NPC entities | ✅ | 5 NPCs spawned at init |
| Camera follow | ✅ | Exponential decay, follows player |
| Input system | ✅ | WASD/arrows, Shift to run |

### Usage

```bash
# Create new project
cargo run -- new "My RPG"

# Open existing project
cargo run -- open path/to/project.dde

# Run with demo project
cargo run
```

### Controls
- **WASD / Arrow Keys** - Move
- **Shift** - Run (2x speed)
- **ESC** - Exit
- Window can be resized

---

## Implemented Features

### Core Architecture (dde-core)

- ✅ ECS foundation with `hecs`
- ✅ Component definitions:
  - Position, SubPosition, WorldState, Biome
  - Entity metadata (Name, Stats, Inventory, Equipment)
  - Animation components (Sprite, AnimationState, RenderLayer)
  - Battle components (AtbGauge, Combatant, DamageInfo)
  - Behavior components (LogicPrompt, PatrolPath, Schedule, MovementSpeed)
  - Audio components (AudioEmitter)
  - Render components (ColorTint)
- ✅ Event bus with `crossbeam-channel`
- ✅ Engine event types (world, sim, interaction, battle, audio, AI, editor, scene, quest)
- ✅ Resources:
  - SimTime (tick-based time tracking)
  - SimulationStats (data-driven stat system)
  - RngPool (deterministic seeded random)
  - InputState
- ✅ Core types: Direction4, WorldState, BiomeKind, MapType, EntityKind, Element, GameState
- ✅ Systems:
  - Simulation (fixed 20Hz timestep)
  - InputSystem (WASD/arrows, Shift, ESC)
  - MovementSystem (collision detection)
  - PlayerController (spawn, move, world position)
  - AnimationSystem (placeholder)

### Database Layer (dde-db)

- ✅ SQLite connection management with pragmas (WAL mode, etc.)
- ✅ Complete schema migration v1 with all tables from blueprint:
  - project_meta, tilesets, maps, tiles
  - entities, simulation_stats, factions, faction_relations
  - dialogue_trees, dialogue_nodes, dialogue_choices
  - encounter_tables, enemy_groups, enemy_templates, skills, items
  - quests, event_triggers, game_flags
  - timelines, timeline_clips, llm_cache
  - assets, asset_tags, asset_provenance, scripts, savepoints
- ✅ Database wrapper with integrity checks
- ✅ Model and query stubs

### Rendering (dde-render)

- ✅ wgpu renderer with surface configuration
- ✅ TileMapRenderer with vertex/index buffers
- ✅ Camera system with smooth follow, projection, view matrix
- ✅ Vertex/mesh definitions
- ✅ WGSL shader foundations
- ✅ Sprite pipeline with camera uniform buffer
- ✅ Texture loading placeholder

### Other Crates

- ✅ `dde-editor`: Editor state management
- ✅ `dde-battle`: Battle state machine, ATB gauge
- ✅ `dde-audio`: kira AudioManager integration
- ✅ `dde-ai`: AI router with request/response handling
- ✅ `dde-export`: Export system with MZ compatibility stubs

## Performance Benchmarks (Week 1 Milestone) ✅ COMPLETE

All critical paths now have Criterion-based performance benchmarks:

| Crate | Benchmark File | Coverage |
|-------|---------------|----------|
| `dde-core` | `ecs_benchmarks.rs` | Entity spawn/query, component ops, world serialization |
| `dde-core` | `simulation_benchmarks.rs` | Tick processing, movement, RNG, collision, event bus, pathfinding |
| `dde-db` | `database_benchmarks.rs` | DB creation, save slots, metadata, screenshots |
| `dde-render` | `camera_benchmarks.rs` | Camera matrices, movement, coordinate transforms, sprite batching |
| `dde-lua` | `lua_benchmarks.rs` | Script execution, API calls, function calls, sandbox overhead |
| `dde-audio` | `audio_math_benchmarks.rs` | dB conversions, volume mixing, sample rate, buffer ops, pan/pitch |

Run benchmarks:
```bash
cargo bench -p dde-core
cargo bench -p dde-db
cargo bench -p dde-render
cargo bench -p dde-lua
cargo bench -p dde-audio
```

## Next Steps (10-Week Prototype Roadmap)

### Week 1: Performance & Optimization ✅ COMPLETE
- [x] Criterion benchmark suite for all critical paths
- [x] Performance baselines established
- [x] Identified optimization opportunities

### Week 2: ECS, Player, Overworld
- [ ] Migrate tiles/entities to ECS components
- [ ] Player movement with collision
- [ ] NPC rendering
- [ ] Camera following
- [ ] Mode 7 shader
- [ ] Input context system

### Week 3: AI Sidecar, Vibecode, Dialogue
- [ ] Python FastAPI sidecar
- [ ] LLM router (Claude, Gemini, Llama3)
- [ ] Vibecode TOML parser
- [ ] Dialogue UI
- [ ] Template fallback system

### Week 4: Editor Tools (Milestone: Walking & Talking)
- [ ] Editor mode toggle
- [ ] egui panels
- [ ] Biome brush + WFC auto-tiler
- [ ] Smart Plop
- [ ] Calamity slider
- [ ] Undo/redo with SQLite savepoints

### Week 5: Audio Engine
- [ ] 8-track stem mixer
- [ ] TOML driver system
- [ ] Map-based stem sets
- [ ] SFX triggers
- [ ] MIDI/hum-to-pattern

### Week 6: Battle System Core
- [ ] Battle state machine
- [ ] Arena generation from tile snapshot
- [ ] ATB speed-bar
- [ ] Battle menu UI
- [ ] Enemy AI scripts
- [ ] Status effects

### Week 7: Inventory, Equipment, Quests
- [ ] Inventory UI
- [ ] Equipment system
- [ ] Shop UI
- [ ] Quest lifecycle tracking
- [ ] Event trigger system

### Week 8: Asset Forge Integration ✅ COMPLETE
- [x] Webview embedding (wry-based, external browser fallback)
- [x] SpriteGeneratorAdapter trait with postMessage IPC
- [x] Asset OS (inbox, staging, review queue, production)
- [x] Classification engine (deterministic rules + confidence scoring)
- [x] Duplicate detection (SHA-256 + perceptual hashing)
- [x] Database schema v2 with asset pipeline tables

### Week 9: Editor Linkage, Lua, Polish ✅ COMPLETE
- [x] Lua scripting with mlua (curated API, sandboxed)
- [x] Particle system (rain, snow, ash, spell effects)
- [x] Animation priority system foundation
- [x] A* pathfinding with passability cache
- [x] NPC patrol and schedule behaviors
- [x] Performance profiler overlay (F11)

### Week 10: MZ Export, Packaging
- [ ] MZ character sheet repacker
- [ ] MZ faceset/tileset repacker
- [ ] MZ JSON emitters
- [ ] Standalone player runtime
- [ ] Cross-platform builds

## Existing Assets

The `sprite_generator` directory contains a fully functional Next.js-based asset generation tool with:

- Character generation (hero, portrait)
- Sprite sheet derivation (walk, jump, attack, idle)
- 8-direction turnaround support
- Background removal
- Frame extraction and animation editing
- Animation QA and auto-fix
- Style lock / consistency system
- Asset review mode
- World asset generation (enemy, projectile, tileset, background)
- Provider routing (OpenAI, Gemini, Fal, etc.)
- Project import/export
- Sandbox preview

This will be integrated into the engine as the Asset Forge (Week 8).

## Building

```bash
cd dde-engine
cargo build
cargo run
```

## License

MIT OR Apache-2.0
