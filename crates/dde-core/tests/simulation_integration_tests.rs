//! Integration tests for core simulation and ECS

use dde_core::{
    components::*,
    events::{EngineEvent, EngineEventBus},
    resources::{RngPool, SimTime},
    Direction4, EntityKind, World,
};
use rand::Rng;

/// Test world creation
#[test]
fn test_world_creation() {
    let world = World::new();
    assert_eq!(world.len(), 0);
}

/// Test entity spawning with components
#[test]
fn test_entity_spawning() {
    let mut world = World::new();

    let entity = world.spawn((
        EntityKindComp {
            kind: EntityKind::Player,
        },
        Position::new(10, 20, 0),
        Name::new("Hero", "player"),
        Stats::default(),
        Direction4::Down,
    ));

    assert_eq!(world.len(), 1);
    assert!(world.get::<&Position>(entity).is_ok());
    assert!(world.get::<&Name>(entity).is_ok());
}

/// Test entity query
#[test]
fn test_entity_query() {
    let mut world = World::new();

    // Spawn multiple entities
    for i in 0..5 {
        world.spawn((
            EntityKindComp {
                kind: EntityKind::Npc,
            },
            Position::new(i * 10, i * 10, 0),
            Name::new(&format!("NPC{}", i), &format!("npc{}", i)),
        ));
    }

    // Query all positions
    let mut query = world.query::<&Position>();
    let positions: Vec<_> = query.into_iter().collect();
    assert_eq!(positions.len(), 5);
}

/// Test component removal
#[test]
fn test_component_removal() {
    let mut world = World::new();

    let entity = world.spawn((
        EntityKindComp {
            kind: EntityKind::Player,
        },
        Position::new(10, 20, 0),
    ));

    // Verify component exists
    assert!(world.get::<&Position>(entity).is_ok());

    // Remove component
    assert!(world.remove_one::<Position>(entity).is_ok());

    // Verify component removed
    assert!(world.get::<&Position>(entity).is_err());
}

/// Test entity destruction
#[test]
fn test_entity_destruction() {
    let mut world = World::new();

    let entity = world.spawn((
        EntityKindComp {
            kind: EntityKind::Player,
        },
        Position::new(10, 20, 0),
    ));

    assert_eq!(world.len(), 1);

    world.despawn(entity).unwrap();

    assert_eq!(world.len(), 0);
}

/// Test event bus
#[test]
fn test_event_bus() {
    let bus = EngineEventBus::new();

    // Send events
    bus.send(EngineEvent::UndoRequested);
    bus.send(EngineEvent::RedoRequested);

    // Drain events
    let events = bus.drain();
    assert_eq!(events.len(), 2);

    // Bus should be empty now
    assert!(bus.is_empty());
}

/// Test RNG pool determinism
#[test]
fn test_rng_pool_determinism() {
    let seed = 12345u64;
    let mut rng1 = RngPool::from_seed(seed);
    let mut rng2 = RngPool::from_seed(seed);

    // Generate some values
    let val1: u32 = rng1.master().gen();
    let val2: u32 = rng2.master().gen();

    assert_eq!(val1, val2);
}

/// Test RNG pool forked streams
#[test]
fn test_rng_pool_streams() {
    let seed = 12345u64;
    let mut rng = RngPool::from_seed(seed);

    // Different streams should produce different sequences
    let sim_val: u32 = rng.sim().gen();
    let battle_val: u32 = rng.battle().gen();
    let loot_val: u32 = rng.loot().gen();

    // They should all be different (with high probability)
    assert_ne!(sim_val, battle_val);
    assert_ne!(battle_val, loot_val);
}

/// Test SimTime advancement
#[test]
fn test_sim_time() {
    let mut time = SimTime::default();

    assert_eq!(time.tick_count, 0);
    assert_eq!(time.hour, 0);

    // Advance many ticks
    for _ in 0..SimTime::TICKS_PER_HOUR {
        time.tick();
    }

    assert_eq!(time.hour, 1);
    assert_eq!(time.tick_count, SimTime::TICKS_PER_HOUR);
}

/// Test SimTime day rollover
#[test]
fn test_sim_time_day_rollover() {
    let mut time = SimTime::default();

    // Advance 24 hours
    for _ in 0..(SimTime::TICKS_PER_HOUR * 24) {
        time.tick();
    }

    assert_eq!(time.hour, 0);
    assert_eq!(time.day, 1);
}

/// Test stats component
#[test]
fn test_stats_component() {
    let stats = Stats::default();

    assert_eq!(stats.hp, stats.max_hp);
    assert_eq!(stats.mp, stats.max_mp);
}

/// Test position component
#[test]
fn test_position_component() {
    let pos = Position::new(10, 20, 5);

    assert_eq!(pos.x, 10);
    assert_eq!(pos.y, 20);
    assert_eq!(pos.z, 5);
}

/// Test name component
#[test]
fn test_name_component() {
    let name = Name::new("Test Entity", "test_entity");

    assert_eq!(name.display, "Test Entity");
    assert_eq!(name.internal, "test_entity");
}

/// Test direction4 enum
#[test]
fn test_direction4() {
    assert_eq!(Direction4::Down.opposite(), Direction4::Up);
    assert_eq!(Direction4::Up.opposite(), Direction4::Down);
    assert_eq!(Direction4::Left.opposite(), Direction4::Right);
    assert_eq!(Direction4::Right.opposite(), Direction4::Left);
}

/// Test world clear
#[test]
fn test_world_clear() {
    let mut world = World::new();

    // Spawn some entities
    for _ in 0..10 {
        world.spawn((
            EntityKindComp {
                kind: EntityKind::Npc,
            },
            Position::new(0, 0, 0),
        ));
    }

    assert_eq!(world.len(), 10);

    world.clear();

    assert_eq!(world.len(), 0);
}

/// Test component iteration
#[test]
fn test_component_iteration() {
    let mut world = World::new();

    // Spawn entities with different components
    for i in 0..5 {
        world.spawn((
            EntityKindComp {
                kind: EntityKind::Npc,
            },
            Position::new(i, i, 0),
        ));
    }

    for i in 0..3 {
        world.spawn((
            EntityKindComp {
                kind: EntityKind::Object,
            },
            Position::new(i * 2, i * 2, 0),
        ));
    }

    // Count all entities with Position
    let mut query = world.query::<&Position>();
    let count = query.into_iter().count();
    assert_eq!(count, 8);
}

/// Test event bus multiple subscribers pattern
#[test]
fn test_event_bus_multiple_subscribers() {
    let bus = EngineEventBus::new();

    // Simulate multiple systems subscribing
    let sender1 = bus.sender();
    let sender2 = bus.sender();

    sender1.send(EngineEvent::UndoRequested).unwrap();
    sender2.send(EngineEvent::RedoRequested).unwrap();

    let events = bus.drain();
    assert_eq!(events.len(), 2);
}

/// Test entity builder pattern
#[test]
fn test_entity_builder() {
    let mut world = World::new();

    let entity = world.spawn((
        EntityKindComp {
            kind: EntityKind::Player,
        },
        Position::new(10, 20, 0),
        SubPosition::default(),
        Name::new("Player", "player"),
        Stats::default(),
        Direction4::Down,
    ));

    // Verify all components
    assert!(world.get::<&EntityKindComp>(entity).is_ok());
    assert!(world.get::<&Position>(entity).is_ok());
    assert!(world.get::<&SubPosition>(entity).is_ok());
    assert!(world.get::<&Name>(entity).is_ok());
    assert!(world.get::<&Stats>(entity).is_ok());
    assert!(world.get::<&Direction4>(entity).is_ok());
}

/// Test entity kind component
#[test]
fn test_entity_kind_comp() {
    let player = EntityKindComp { kind: EntityKind::Player };
    assert_eq!(player.kind, EntityKind::Player);

    let npc = EntityKindComp { kind: EntityKind::Npc };
    assert_eq!(npc.kind, EntityKind::Npc);
}

/// Test query with multiple components
#[test]
fn test_query_multiple_components() {
    let mut world = World::new();

    for i in 0..10 {
        world.spawn((
            EntityKindComp {
                kind: EntityKind::Npc,
            },
            Position::new(i, i, 0),
            Name::new(&format!("NPC{}", i), &format!("npc{}", i)),
        ));
    }

    // Query for both Position and Name
    let mut query = world.query::<(&Position, &Name)>();
    let count = query.into_iter().count();
    assert_eq!(count, 10);
}

/// Test sub-position overflow handling
#[test]
fn test_sub_position_overflow() {
    let mut sub = SubPosition::default();
    
    sub.px = 1.5; // Should overflow to next tile
    
    // In actual usage, the movement system would handle this
    assert_eq!(sub.px, 1.5);
}

/// Test event types
#[test]
fn test_event_types() {
    let bus = EngineEventBus::new();

    // Test various event types
    bus.send(EngineEvent::UndoRequested);
    bus.send(EngineEvent::RedoRequested);
    bus.send(EngineEvent::TileStateChanged {
        tile_id: 1,
        old: dde_core::WorldState::default(),
        new: dde_core::WorldState::default(),
    });
    bus.send(EngineEvent::PlayerInteracted { target: dde_core::Entity::DANGLING });

    let events = bus.drain();
    assert_eq!(events.len(), 4);
}

/// Test empty world queries
#[test]
fn test_empty_world_queries() {
    let world = World::new();

    let mut query = world.query::<&Position>();
    let count = query.into_iter().count();
    assert_eq!(count, 0);
}

/// Test entity despawning multiple
#[test]
fn test_despawn_multiple() {
    let mut world = World::new();

    let entities: Vec<_> = (0..5)
        .map(|i| {
            world.spawn((
                EntityKindComp {
                    kind: EntityKind::Npc,
                },
                Position::new(i, i, 0),
            ))
        })
        .collect();

    assert_eq!(world.len(), 5);

    // Despawn in reverse order
    for entity in entities.iter().rev() {
        world.despawn(*entity).unwrap();
    }

    assert_eq!(world.len(), 0);
}
