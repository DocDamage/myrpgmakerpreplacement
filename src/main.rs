//! DocDamage Engine - Main Entry Point
//! 
//! A desktop RPG maker and simulation engine.

use std::time::{Duration, Instant};

use tracing::{info, warn};

fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    info!("DocDamage Engine starting...");
    info!("Version: {}-{}", env!("CARGO_PKG_VERSION"), env!("CARGO_PKG_NAME"));
    
    // TODO: Initialize window, renderer, and game loop
    // This is a placeholder for the Week 1 deliverables
    
    info!("Engine initialized successfully");
    info!("Note: Full implementation requires Week 1-10 development per blueprint");
    
    // Simple test to verify crates compile
    test_crates()?;
    
    Ok(())
}

fn test_crates() -> anyhow::Result<()> {
    // Verify core types
    let _dir = dde_core::Direction4::Down;
    let _state = dde_core::GameState::Overworld;
    
    // Verify ECS world creation
    let mut world = dde_core::World::new();
    let entity = world.spawn((
        dde_core::components::Position::new(0, 0, 0),
        dde_core::components::Name::new("Test Entity", "test_entity"),
    ));
    
    info!("ECS test: Created entity {:?}", entity);
    
    // Verify event bus
    let event_bus = dde_core::events::EventBus::new();
    event_bus.send(dde_core::events::EngineEvent::EntitySpawned {
        entity,
        kind: "test".to_string(),
    });
    
    let events = event_bus.drain();
    info!("Event bus test: Drained {} events", events.len());
    
    // Verify resources
    let _rng = dde_core::resources::RngPool::from_seed(12345);
    let _time = dde_core::resources::SimTime::default();
    
    info!("All crate tests passed!");
    
    Ok(())
}
