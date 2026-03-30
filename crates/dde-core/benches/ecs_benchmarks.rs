//! Benchmarks for ECS (Entity Component System) operations
//!
//! Run with: cargo bench -p dde-core

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use dde_core::{
    components::*,
    Direction4, EntityKind, World,
};

/// Benchmark entity spawning
fn bench_entity_spawn(c: &mut Criterion) {
    let mut group = c.benchmark_group("entity_spawn");

    for count in [10, 100, 1000, 10000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, &count| {
            b.iter(|| {
                let mut world = World::new();
                for i in 0..count {
                    world.spawn((
                        EntityKindComp {
                            kind: EntityKind::Npc,
                        },
                        Position::new(i as i32, i as i32, 0),
                        Name::new(format!("NPC{}", i), format!("npc_{}", i)),
                    ));
                }
                black_box(world.len());
            });
        });
    }

    group.finish();
}

/// Benchmark entity spawning with many components
fn bench_entity_spawn_complex(c: &mut Criterion) {
    let mut group = c.benchmark_group("entity_spawn_complex");

    for count in [10, 100, 1000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, &count| {
            b.iter(|| {
                let mut world = World::new();
                for i in 0..count {
                    world.spawn((
                        EntityKindComp {
                            kind: EntityKind::Player,
                        },
                        Position::new(i as i32, i as i32, 0),
                        SubPosition::default(),
                        Name::new(format!("Player{}", i), format!("player_{}", i)),
                        Stats::default(),
                        Direction4::Down,
                    ));
                }
                black_box(world.len());
            });
        });
    }

    group.finish();
}

/// Benchmark entity queries
fn bench_entity_query(c: &mut Criterion) {
    let mut group = c.benchmark_group("entity_query");

    for count in [100, 1000, 10000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, &count| {
            // Setup: create world with entities
            let mut world = World::new();
            for i in 0..count {
                world.spawn((
                    EntityKindComp {
                        kind: EntityKind::Npc,
                    },
                    Position::new(i as i32, i as i32, 0),
                    Name::new(format!("NPC{}", i), format!("npc_{}", i)),
                ));
            }

            b.iter(|| {
                let sum: i32 = world
                    .query::<&Position>()
                    .into_iter()
                    .map(|(_, pos)| pos.x + pos.y)
                    .sum();
                black_box(sum);
            });
        });
    }

    group.finish();
}

/// Benchmark multi-component queries
fn bench_query_multiple_components(c: &mut Criterion) {
    let mut group = c.benchmark_group("query_multiple_components");

    for count in [100, 1000, 10000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, &count| {
            // Setup
            let mut world = World::new();
            for i in 0..count {
                world.spawn((
                    EntityKindComp {
                        kind: EntityKind::Npc,
                    },
                    Position::new(i as i32, i as i32, 0),
                    Name::new(format!("NPC{}", i), format!("npc_{}", i)),
                ));
            }

            b.iter(|| {
                let count = world.query::<(&Position, &Name)>().into_iter().count();
                black_box(count);
            });
        });
    }

    group.finish();
}

/// Benchmark entity destruction
fn bench_entity_despawn(c: &mut Criterion) {
    let mut group = c.benchmark_group("entity_despawn");

    for count in [100, 1000, 5000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, &count| {
            b.iter(|| {
                let mut world = World::new();
                let entities: Vec<_> = (0..count)
                    .map(|i| {
                        world.spawn((
                            EntityKindComp {
                                kind: EntityKind::Npc,
                            },
                            Position::new(i as i32, i as i32, 0),
                        ))
                    })
                    .collect();

                for entity in entities {
                    world.despawn(entity).unwrap();
                }
                black_box(world.len());
            });
        });
    }

    group.finish();
}

/// Benchmark world clear
fn bench_world_clear(c: &mut Criterion) {
    let mut group = c.benchmark_group("world_clear");

    for count in [100, 1000, 10000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, &count| {
            b.iter_with_setup(
                || {
                    let mut world = World::new();
                    for i in 0..count {
                        world.spawn((
                            EntityKindComp {
                                kind: EntityKind::Npc,
                            },
                            Position::new(i as i32, i as i32, 0),
                        ));
                    }
                    world
                },
                |mut world| {
                    world.clear();
                    black_box(world.len());
                },
            );
        });
    }

    group.finish();
}

/// Benchmark component removal
fn bench_component_removal(c: &mut Criterion) {
    let mut group = c.benchmark_group("component_removal");

    for count in [100, 1000, 5000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, &count| {
            b.iter_with_setup(
                || {
                    let mut world = World::new();
                    let entities: Vec<_> = (0..count)
                        .map(|i| {
                            world.spawn((
                                EntityKindComp {
                                    kind: EntityKind::Npc,
                                },
                                Position::new(i as i32, i as i32, 0),
                            ))
                        })
                        .collect();
                    (world, entities)
                },
                |(mut world, entities)| {
                    for entity in entities {
                        let _ = world.remove_one::<Position>(entity);
                    }
                    black_box(world.len());
                },
            );
        });
    }

    group.finish();
}

/// Benchmark random access patterns
fn bench_random_access(c: &mut Criterion) {
    let mut group = c.benchmark_group("random_access");

    for count in [100, 1000, 10000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, &count| {
            // Setup
            let mut world = World::new();
            let entities: Vec<_> = (0..count)
                .map(|i| {
                    world.spawn((
                        EntityKindComp {
                            kind: EntityKind::Npc,
                        },
                        Position::new(i as i32, i as i32, 0),
                        Stats::default(),
                    ))
                })
                .collect();

            b.iter(|| {
                // Access entities randomly
                let mut sum = 0i32;
                for i in (0..count).step_by(7) {
                    if let Ok(pos) = world.get::<&Position>(entities[i]) {
                        sum += pos.x;
                    }
                }
                black_box(sum);
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_entity_spawn,
    bench_entity_spawn_complex,
    bench_entity_query,
    bench_query_multiple_components,
    bench_entity_despawn,
    bench_world_clear,
    bench_component_removal,
    bench_random_access
);
criterion_main!(benches);
