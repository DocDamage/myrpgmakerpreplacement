# DocDamage Engine (DDE)

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

## Features

- **ECS Architecture**: Built with `hecs` for composition-over-inheritance design
- **Deterministic Simulation**: Seeded PRNGs for reproducible worlds
- **SQLite World State**: Single-file project format with WAL mode
- **Event-Driven**: Typed event bus for decoupled subsystem communication
- **Fixed Timestep**: 20Hz simulation tick rate
- **Data-Driven**: TOML configuration for formulas, rules, and tuning

## Building

```bash
cargo build --release
```

## Running

```bash
cargo run
```

## Development Roadmap

See [PROJECT_STATUS.md](PROJECT_STATUS.md) for detailed status and 10-week prototype roadmap.

## License

MIT OR Apache-2.0
