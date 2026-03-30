# DocDamage Engine (DDE)

[![Rust CI](https://github.com/DocDamage/myrpgmakerpreplacement/actions/workflows/rust.yml/badge.svg)](https://github.com/DocDamage/myrpgmakerpreplacement/actions/workflows/rust.yml)

A desktop RPG maker and simulation engine built in Rust.

## Overview

DocDamage Engine is an opinionated RPG construction kit where the simulation layer (world state, NPC behavior, calamity propagation) is a first-class citizen alongside the map editor and battle system.

## Architecture

The engine follows a modular workspace architecture:

| Crate | Purpose |
|-------|---------|
| `dde-core` | ECS, components, events, resources, systems |
| `dde-db` | SQLite persistence with complete schema migrations |
| `dde-render` | wgpu rendering pipeline |
| `dde-editor` | egui-based editor mode |
| `dde-battle` | ATB battle system |
| `dde-audio` | kira-based stem mixer |
| `dde-ai` | Python FastAPI sidecar client |
| `dde-export` | RPG Maker MZ export compatibility |
| `dde-sync` | Real-time collaboration via WebSocket |
| `dde-lua` | Lua scripting runtime |
| `dde-asset-forge` | Asset management and generation |

## Features

- **ECS Architecture**: Built with `hecs` for composition-over-inheritance design
- **Deterministic Simulation**: Seeded PRNGs for reproducible worlds
- **SQLite World State**: Single-file project format with WAL mode
- **Event-Driven**: Typed event bus for decoupled subsystem communication
- **Fixed Timestep**: 20Hz simulation tick rate
- **Data-Driven**: TOML configuration for formulas, rules, and tuning
- **Visual Scripting**: Node-based event scripting
- **AI Game Director**: LLM-powered procedural content generation
- **Real-time Collaboration**: Multiplayer editing with CRDT sync
- **Replay System**: Deterministic gameplay recording and playback

## Quick Start

### Prerequisites

- [Rust](https://rustup.rs/) (latest stable)
- (Linux only) `libasound2-dev` and `libudev-dev`

### Building

```bash
# Clone the repository
git clone https://github.com/DocDamage/myrpgmakerpreplacement.git
cd myrpgmakerpreplacement

# Build in release mode
cargo build --release

# Or use the just task runner (if installed)
just build-release
```

### Running

```bash
cargo run --release
```

### Development

```bash
# Run all tests
cargo test --workspace --all-features

# Run CI checks locally
just ci
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for detailed development guidelines.

## Project Status

See [PROJECT_STATUS.md](PROJECT_STATUS.md) for detailed status and 10-week prototype roadmap.

## Technical Debt Status

**~5% Technical Debt** - The codebase is production-ready with:
- ✅ 0 compilation errors
- ✅ 469+ tests passing
- ✅ All features working
- ⚠️ ~70 style warnings (acceptable, being addressed incrementally)

## Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for:
- Development setup
- Code style guidelines
- Testing requirements
- Pull request process

## License

MIT OR Apache-2.0
