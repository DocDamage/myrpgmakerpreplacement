# DocDamage Engine - Complete Feature Reference

> **Last Updated:** March 30, 2026  
> **Version:** Week 2 Milestone  
> **Status:** 71 Features Surfaced ✅

---

## Table of Contents

- [Core Engine Features](#core-engine-features)
- [Editor Features](#editor-features)
- [Battle System](#battle-system)
- [AI & Scripting](#ai--scripting)
- [Asset Management](#asset-management)
- [Collaboration](#collaboration)
- [Export & Deployment](#export--deployment)

---

## Core Engine Features

### ECS (Entity Component System)
| Feature | Status | Description |
|---------|--------|-------------|
| Entity Spawning | ✅ | Spawn entities with any component combination |
| Component Queries | ✅ | Fast filtered queries with hecs |
| Component Add/Remove | ✅ | Dynamic component modification |
| World Serialization | ✅ | Save/load entire world state |
| Event Bus | ✅ | Typed event system with crossbeam-channel |
| Deterministic RNG | ✅ | Seeded PRNGs for reproducible gameplay |

### Simulation
| Feature | Status | Description |
|---------|--------|-------------|
| Fixed Timestep | ✅ | 20Hz simulation tick rate |
| Catch-up Protection | ✅ | Prevent spiral of death |
| Pause/Resume | ✅ | Simulation state control |
| Time Scaling | ✅ | Slow-motion and fast-forward |

### Pathfinding
| Feature | Status | Description |
|---------|--------|-------------|
| A* Algorithm | ✅ | Grid-based pathfinding |
| Passability Cache | ✅ | Fast collision lookups |
| Entity Avoidance | ✅ | Dynamic obstacle avoidance |
| Patrol Paths | ✅ | Waypoint-based NPC movement |
| NPC Schedules | ✅ | Time-of-day location changes |
| Debug Visualization | ✅ | Grid overlay with path display |

---

## Editor Features

### Map Editing
| Feature | Status | Description |
|---------|--------|-------------|
| Tilemap Editor | ✅ | Paint tiles with brush tools |
| Auto-tiling | ✅ | Automatic edge/corner matching |
| Layer Management | ✅ | Surface/underground/overhead layers |
| Collision Editing | ✅ | Visual passability editing |
| Entity Placement | ✅ | Drag-and-drop NPCs, items, triggers |

### Visual Scripting
| Feature | Status | Description |
|---------|--------|-------------|
| Node Canvas | ✅ | Drag-and-drop node editor |
| Event Nodes | ✅ | On-enter, on-interact, on-timer |
| Condition Nodes | ✅ | Has component, distance check, line of sight |
| Action Nodes | ✅ | Spawn, move, play animation, show dialogue |
| Variable System | ✅ | Blackboard for script state |
| Live Debugging | ✅ | Step-through execution |

### AI Tools
| Feature | Status | Description |
|---------|--------|-------------|
| Behavior Tree Editor | ✅ | Visual node-based AI designer |
| Behavior Tree Debugger | ✅ | Runtime state visualization |
| Patrol Path Editor | ✅ | Visual waypoint editor |
| NPC Schedule Editor | ✅ | 24-hour timeline editor |

### Effects
| Feature | Status | Description |
|---------|--------|-------------|
| Particle Editor | ✅ | Live preview with 10+ presets |
| Particle Types | ✅ | Rain, snow, fire, smoke, magic, heal |
| Custom Emitters | ✅ | Configure rate, lifetime, velocity, color |

### Debugging
| Feature | Status | Description |
|---------|--------|-------------|
| Profiler Panel | ✅ | Frame time, FPS, memory tracking |
| Advanced Profiler | ✅ | Budget configuration, export to CSV/JSON |
| Event Bus Monitor | ✅ | Real-time event stream |
| Pathfinding Debug | ✅ | Grid overlay with A* visualization |
| Hot Reload Panel | ✅ | Asset and script hot reload management |

---

## Battle System

### Core Battle
| Feature | Status | Description |
|---------|--------|-------------|
| ATB System | ✅ | Active Time Battle with charge bars |
| Turn Queue | ✅ | Visual turn order display |
| Formation Editor | ✅ | 5x3 grid position assignment |
| Formation Presets | ✅ | Balanced, Aggressive, Defensive, etc. |
| Row Bonuses | ✅ | Front/middle/back row modifiers |

### Status Effects
| Feature | Status | Description |
|---------|--------|-------------|
| Status Effect Editor | ✅ | Visual editor for 34+ effect types |
| DoT Effects | ✅ | Poison, Burn, Bleed |
| Crowd Control | ✅ | Stun, Sleep, Silence, Freeze |
| Buffs/Debuffs | ✅ | All stat modifiers |
| Stack Behavior | ✅ | Replace, stack, extend, intensify |
| Resistance System | ✅ | Category-based resistance |

### Items & Skills
| Feature | Status | Description |
|---------|--------|-------------|
| Item Database Editor | ✅ | Complete item creation UI |
| Item Types | ✅ | Heal, Mana, Elixir, Phoenix, Buff, Offensive |
| Effect Configuration | ✅ | Power, target, element, cooldown |
| Price Management | ✅ | Buy/sell prices |

### Formulas
| Feature | Status | Description |
|---------|--------|-------------|
| Formula Editor | ✅ | Damage/healing formula editor |
| Test Simulator | ✅ | 1000-iteration simulation |
| Preset Formulas | ✅ | Standard, Pokemon-style, percentage |
| TOML Persistence | ✅ | Save/load formulas |

### Battle Log
| Feature | Status | Description |
|---------|--------|-------------|
| Battle Log Viewer | ✅ | Complete battle history |
| Statistics | ✅ | Damage/healing per combatant |
| Replay System | ✅ | Step-through battle replay |
| Export | ✅ | Log export functionality |

---

## AI & Scripting

### Lua Scripting
| Feature | Status | Description |
|---------|--------|-------------|
| Lua 5.4 Runtime | ✅ | mlua-based scripting |
| Sandboxed API | ✅ | Curated safe API surface |
| Script Manager | ✅ | File browser for all scripts |
| Hot Reload | ✅ | Automatic script reloading |
| Error Display | ✅ | Compile/runtime error reporting |

### AI Director
| Feature | Status | Description |
|---------|--------|-------------|
| Cache Management | ✅ | LLM response cache with TTL |
| Provider Routing | ✅ | Claude/Gemini/Ollama routing |
| Quest Pool | ✅ | Procedural quest generation |
| Tension Analysis | ✅ | Pacing curve visualization |
| Bark System | ✅ | Contextual NPC dialogue |

### Dialogue
| Feature | Status | Description |
|---------|--------|-------------|
| Dialogue Editor | ✅ | Node-based conversation designer |
| Node Types | ✅ | Text, choice, condition, action, branch |
| Variable Interpolation | ✅ | Dynamic text substitution |
| Preview Mode | ✅ | Test conversations in-editor |

---

## Asset Management

### Asset Forge
| Feature | Status | Description |
|---------|--------|-------------|
| Asset Pipeline | ✅ | Kanban workflow (Inbox→Staging→Review→Production) |
| Classification Rules | ✅ | Auto-tagging based on file patterns |
| Dependency Graph | ✅ | Visual asset dependency viewer |
| Duplicate Detection | ✅ | Hash-based duplicate finder |
| Review Queue | ✅ | Approve/reject workflow |

### Asset Pipeline Stages
| Stage | Description |
|-------|-------------|
| Inbox | New assets awaiting classification |
| Staging | Processing and tagging |
| Review | Approval queue |
| Production | Approved and ready |
| Rejected | Sent back for revision |

### Hot Reload
| Feature | Status | Description |
|---------|--------|-------------|
| Asset Watching | ✅ | File system watcher |
| Texture Reload | ✅ | Runtime texture updates |
| Shader Reload | ✅ | Runtime shader updates |
| Script Reload | ✅ | Lua script hot reload |
| Change Log | ✅ | Recent changes display |

---

## Collaboration

### Real-time Collaboration
| Feature | Status | Description |
|---------|--------|-------------|
| Multi-user Editing | ✅ | Concurrent map editing |
| WebSocket Sync | ✅ | Real-time data synchronization |
| CRDT | ✅ | Conflict-free replicated data types |
| Presence | ✅ | Online user indicators |

### Communication
| Feature | Status | Description |
|---------|--------|-------------|
| Chat System | ✅ | In-editor messaging |
| Cursor Tracking | ✅ | See other users' cursors |
| Selection Highlighting | ✅ | See what others are editing |

### Conflict Resolution
| Feature | Status | Description |
|---------|--------|-------------|
| Entity Locking | ✅ | Lock entities during editing |
| Lock Manager | ✅ | Admin panel for lock management |
| Conflict Resolution | ✅ | Merge conflict handling |
| Operation History | ✅ | Undo/redo with sync |

---

## Export & Deployment

### Export Targets
| Target | Status | Description |
|--------|--------|-------------|
| RPG Maker MZ | ✅ | MZ-compatible JSON export |
| Standalone | ✅ | Self-contained executable |
| WASM | ✅ | Web deployment |

### Export Features
| Feature | Status | Description |
|---------|--------|-------------|
| Asset Packing | ✅ | Bundle all resources |
| Encryption | ✅ | Optional save encryption |
| Compression | ✅ | Compressed output |
| Manifest Generation | ✅ | Export manifest |

---

## Documentation

### Auto-Documentation
| Feature | Status | Description |
|---------|--------|-------------|
| Game Design Doc | ✅ | Export to Markdown/HTML/PDF |
| API Reference | ✅ | Lua API documentation |
| NPC Profiles | ✅ | Character documentation |
| Quest Logs | ✅ | Quest documentation |

---

## Performance

### Benchmarks
| Feature | Status | Description |
|---------|--------|-------------|
| ECS Benchmarks | ✅ | Entity spawn/query performance |
| Simulation Benchmarks | ✅ | Tick processing performance |
| Database Benchmarks | ✅ | SQLite performance |
| Lua Benchmarks | ✅ | Script execution performance |
| Render Benchmarks | ✅ | Camera/graphics performance |
| Audio Benchmarks | ✅ | Audio math performance |

### Optimization Tools
| Feature | Status | Description |
|---------|--------|-------------|
| Profiler Overlay | ✅ | F11 toggle |
| Budget Configuration | ✅ | Per-system time budgets |
| Memory Breakdown | ✅ | RAM/VRAM usage |
| Entity Counts | ✅ | Per-system entity tracking |

---

## Feature Count Summary

| Category | Features | Status |
|----------|----------|--------|
| Core Engine | 10 | ✅ Complete |
| Editor Tools | 25 | ✅ Complete |
| Battle System | 15 | ✅ Complete |
| AI & Scripting | 10 | ✅ Complete |
| Asset Management | 8 | ✅ Complete |
| Collaboration | 6 | ✅ Complete |
| Export | 4 | ✅ Complete |
| Documentation | 4 | ✅ Complete |
| Performance | 6 | ✅ Complete |
| **TOTAL** | **88** | **✅ 71 Surfaced + 17 Enhanced** |

---

## Quick Reference

### Menu Shortcuts
```
Assets
  ├─ Asset Pipeline (Kanban)
  ├─ Classification Rules
  ├─ Dependency Graph
  └─ Find Duplicates

Battle
  ├─ Formation Editor
  ├─ Battle Panel
  ├─ Item Database
  ├─ Status Effects
  └─ Battle Log

Tools
  ├─ Dialogue Editor
  ├─ Script Manager
  ├─ Hot Reload Panel
  └─ Replay Theater

Effects
  └─ Particle Editor

NPC
  ├─ Schedule Editor
  ├─ Patrol Paths
  └─ Manage NPCs

Debug
  ├─ Pathfinding Debug
  ├─ Event Bus Monitor
  └─ Profiler

Collaboration
  ├─ Lock Manager
  └─ Sync Panel
```

### Keyboard Shortcuts
| Key | Action |
|-----|--------|
| F5 | Hot reload assets |
| F11 | Toggle profiler |
| Tab | Toggle editor mode |
| Ctrl+S | Save project |
| Ctrl+Z | Undo |
| Ctrl+Y | Redo |

---

## Changelog

### Week 2 Milestone (March 30, 2026)
- ✅ Surfaced 71 previously hidden backend features
- ✅ Created 18 new editor panels
- ✅ Wired 7 critical systems (Status Effects, Items, Formulas, etc.)
- ✅ Added 4 database migrations
- ✅ Complete menu integration across 8 categories

---

*For detailed API documentation, see the inline code documentation in each module.*
