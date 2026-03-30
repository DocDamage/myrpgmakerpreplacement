//! Benchmarks for simulation systems
//!
//! Run with: cargo bench -p dde-core

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use dde_core::{
    components::{*, behavior::MovementSpeed},
    events::EngineEventBus,
    resources::RngPool,
    systems::{simulation::Simulation, MovementSystem, TileCollisionMap},
    Direction4, EntityKind, World,
};
use rand::Rng;
use std::time::Duration;

/// Benchmark simulation tick
fn bench_simulation_tick(c: &mut Criterion) {
    let mut group = c.benchmark_group("simulation_tick");

    for entity_count in [10, 100, 1000, 5000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(entity_count),
            entity_count,
            |b, &count| {
                b.iter_with_setup(
                    || {
                        let mut world = World::new();
                        let mut collision_map = TileCollisionMap::new(64, 64);
                        collision_map.block_edges();

                        // Spawn entities
                        for i in 0..count {
                            world.spawn((
                                EntityKindComp {
                                    kind: EntityKind::Npc,
                                },
                                Position::new((i % 60 + 2) as i32, (i / 60 + 2) as i32, 0),
                                SubPosition::default(),
                                MovementSpeed::from_spd_stat(5),
                                Direction4::Down,
                            ));
                        }

                        let simulation = Simulation::new(12345);
                        let event_bus = EngineEventBus::new();

                        (world, simulation, event_bus, collision_map)
                    },
                    |(mut world, mut simulation, event_bus, collision_map)| {
                        // Simulate one frame (50ms tick)
                        simulation.update(
                            Duration::from_millis(50),
                            &mut world,
                            &event_bus,
                        );
                        MovementSystem::update(&mut world, &collision_map, 0.05);
                        black_box(world.len());
                    },
                );
            },
        );
    }

    group.finish();
}

/// Benchmark movement system
fn bench_movement_system(c: &mut Criterion) {
    let mut group = c.benchmark_group("movement_system");

    for entity_count in [10, 100, 1000, 5000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(entity_count),
            entity_count,
            |b, &count| {
                b.iter_with_setup(
                    || {
                        let mut world = World::new();
                        let mut collision_map = TileCollisionMap::new(128, 128);
                        collision_map.block_edges();

                        for i in 0..count {
                            world.spawn((
                                EntityKindComp {
                                    kind: EntityKind::Npc,
                                },
                                Position::new((i % 100 + 10) as i32, (i / 100 + 10) as i32, 0),
                                SubPosition::default(),
                                MovementSpeed::from_spd_stat(5),
                                Direction4::Down,
                            ));
                        }

                        (world, collision_map)
                    },
                    |(mut world, collision_map)| {
                        MovementSystem::update(&mut world, &collision_map, 0.016);
                        black_box(world.len());
                    },
                );
            },
        );
    }

    group.finish();
}

/// Benchmark RNG operations
fn bench_rng_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("rng_operations");

    group.bench_function("master_gen_u32", |b| {
        let mut rng = RngPool::from_seed(12345);
        b.iter(|| {
            let val: u32 = rng.master().gen();
            black_box(val);
        });
    });

    group.bench_function("sim_gen_u32", |b| {
        let mut rng = RngPool::from_seed(12345);
        b.iter(|| {
            let val: u32 = rng.sim().gen();
            black_box(val);
        });
    });

    group.bench_function("battle_gen_u32", |b| {
        let mut rng = RngPool::from_seed(12345);
        b.iter(|| {
            let val: u32 = rng.battle().gen();
            black_box(val);
        });
    });

    group.bench_function("loot_gen_u32", |b| {
        let mut rng = RngPool::from_seed(12345);
        b.iter(|| {
            let val: u32 = rng.loot().gen();
            black_box(val);
        });
    });

    // Test determinism
    group.bench_function("rng_determinism", |b| {
        b.iter(|| {
            let mut rng1 = RngPool::from_seed(12345);
            let mut rng2 = RngPool::from_seed(12345);
            let v1: u32 = rng1.master().gen();
            let v2: u32 = rng2.master().gen();
            assert_eq!(v1, v2);
            black_box((v1, v2));
        });
    });

    group.finish();
}

/// Benchmark collision detection
fn bench_collision_detection(c: &mut Criterion) {
    let mut group = c.benchmark_group("collision_detection");

    for size in [64, 128, 256, 512].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let mut collision_map = TileCollisionMap::new(size, size);
            collision_map.block_edges();

            // Add some random obstacles
            for x in 10..size - 10 {
                for y in 10..size - 10 {
                    if (x + y) % 7 == 0 {
                        collision_map.set_walkable(x, y, false);
                    }
                }
            }

            b.iter(|| {
                let mut walkable_count = 0;
                for x in 0..size {
                    for y in 0..size {
                        if collision_map.is_walkable(x, y) {
                            walkable_count += 1;
                        }
                    }
                }
                black_box(walkable_count);
            });
        });
    }

    group.finish();
}

/// Benchmark event bus operations
fn bench_event_bus(c: &mut Criterion) {
    use dde_core::events::EngineEvent;
    
    let mut group = c.benchmark_group("event_bus");

    group.bench_function("send_1000_events", |b| {
        b.iter(|| {
            let bus = EngineEventBus::new();
            for i in 0..1000 {
                let event = EngineEvent::SimStatChanged {
                    key: format!("stat_{}", i),
                    old: 0.0,
                    new: 1.0,
                };
                let _ = bus.send(event);
            }
            black_box(bus.drain().len());
        });
    });

    group.bench_function("drain_1000_events", |b| {
        b.iter_with_setup(
            || {
                let bus = EngineEventBus::new();
                for i in 0..1000 {
                    let event = EngineEvent::SimStatChanged {
                        key: format!("stat_{}", i),
                        old: 0.0,
                        new: 1.0,
                    };
                    let _ = bus.send(event);
                }
                bus
            },
            |bus| {
                let events = bus.drain();
                black_box(events.len());
            },
        );
    });

    group.finish();
}

/// Benchmark pathfinding
fn bench_pathfinding(c: &mut Criterion) {
    use dde_core::pathfinding::PathGrid;
    use glam::IVec2;

    let mut group = c.benchmark_group("pathfinding");

    for size in [32, 64, 128].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let mut grid = PathGrid::new(size, size);

            // Block edges
            for x in 0..size {
                grid.set_walkable(x, 0, false);
                grid.set_walkable(x, size - 1, false);
            }
            for y in 0..size {
                grid.set_walkable(0, y, false);
                grid.set_walkable(size - 1, y, false);
            }

            // Add obstacles
            for x in 5..size - 5 {
                if x % 10 == 0 {
                    for y in 0..size {
                        grid.set_walkable(x, y, false);
                    }
                }
            }

            let start = IVec2::new(2, 2);
            let goal = IVec2::new((size - 3) as i32, (size - 3) as i32);

            b.iter(|| {
                let path = grid.find_path(start, goal);
                black_box(path);
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_simulation_tick,
    bench_movement_system,
    bench_rng_operations,
    bench_collision_detection,
    bench_event_bus,
    bench_pathfinding
);
criterion_main!(benches);
