//! DocDamage Engine - Main Entry Point
//! 
//! A desktop RPG maker and simulation engine.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use glam::Vec2;
use tracing::info;
use winit::{
    dpi::LogicalSize,
    event::{Event, WindowEvent, KeyEvent},
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};

use dde_core::{TICK_RATE, World};
use dde_core::events::EventBus;
use dde_core::resources::InputState;
use dde_core::systems::{
    InputSystem, InputContext,
    MovementSystem, TileCollisionMap,
    PlayerController,
    simulation::Simulation,
};
use dde_render::Renderer;

mod project;
use project::ProjectManager;

/// Application state
struct App {
    world: World,
    event_bus: EventBus,
    simulation: Simulation,
    input_state: InputState,
    last_frame: Instant,
    fps_counter: FpsCounter,
    camera_target: Vec2,
    project_manager: ProjectManager,
    input_system: InputSystem,
    player_controller: PlayerController,
    tile_collision: TileCollisionMap,
}

impl App {
    fn new(seed: u64) -> Self {
        let world = World::new();
        let event_bus = EventBus::new();
        let simulation = Simulation::new(seed);
        let input_state = InputState::default();
        let input_system = InputSystem::new();
        let player_controller = PlayerController::new();
        
        // Create collision map (64x64 tiles, edges blocked)
        let mut tile_collision = TileCollisionMap::new(64, 64);
        tile_collision.block_edges();
        // Add some random obstacles
        for i in 10..20 {
            tile_collision.set_walkable(i, 15, false);
            tile_collision.set_walkable(i, 16, false);
        }
        
        Self {
            world,
            event_bus,
            simulation,
            input_state,
            last_frame: Instant::now(),
            fps_counter: FpsCounter::new(),
            camera_target: Vec2::new(32.0 * 32.0, 32.0 * 32.0), // Center of world
            project_manager: ProjectManager::new(),
            input_system,
            player_controller,
            tile_collision,
        }
    }
    
    fn update(&mut self, dt: Duration) {
        // Update FPS counter
        self.fps_counter.update(dt);
        
        // Handle player movement from input
        if self.player_controller.exists() {
            let move_dir = self.input_system.get_movement_direction();
            
            if move_dir.length_squared() > 0.0 {
                let run = self.input_system.is_action_held(dde_core::InputAction::Run);
                let speed = if run {
                    self.player_controller.move_speed * self.player_controller.run_multiplier
                } else {
                    self.player_controller.move_speed
                };
                
                // Move the player with collision
                if let Some(entity) = self.player_controller.entity {
                    MovementSystem::move_entity(
                        &mut self.world,
                        entity,
                        move_dir,
                        speed * 32.0, // Convert to pixels per second (32px tiles)
                        &self.tile_collision,
                        dt.as_secs_f32(),
                    );
                }
            }
        }
        
        // Update other entity movement
        MovementSystem::update(&mut self.world, &self.tile_collision, dt.as_secs_f32());
        
        // Run simulation tick(s)
        self.simulation.update(dt, &mut self.world, &self.event_bus);
        
        // Update camera target to follow player
        if let Some(player_pos) = self.player_controller.world_position(&self.world) {
            self.camera_target = player_pos * 32.0; // Convert to world pixels
        }
        
        // Process events
        let events = self.event_bus.drain();
        for _event in events {
            // Handle events
        }
        
        // Clear input frame state
        self.input_system.clear_frame();
    }
    
    /// Initialize the game world
    fn init_world(&mut self) {
        // Spawn player at center
        let player_x = 32;
        let player_y = 32;
        self.player_controller.spawn_player(&mut self.world, player_x, player_y);
        info!("Player spawned at ({}, {})", player_x, player_y);
        
        // Spawn some NPCs
        for i in 0..5 {
            let x = 20 + i * 8;
            let y = 25;
            let entity = self.world.spawn((
                dde_core::components::EntityKindComp { 
                    kind: dde_core::EntityKind::Npc 
                },
                dde_core::components::Name::new(
                    format!("NPC {}", i + 1), 
                    format!("npc_{}", i)
                ),
                dde_core::components::Position::new(x, y, 0),
                dde_core::components::SubPosition::default(),
                dde_core::components::behavior::MovementSpeed::from_spd_stat(5),
                dde_core::Direction4::Down,
            ));
            info!("Spawned NPC {} at ({}, {})", i + 1, x, y);
            let _ = entity;
        }
    }
}

/// FPS counter for display
struct FpsCounter {
    frame_count: u32,
    elapsed: Duration,
    fps: f32,
}

impl FpsCounter {
    fn new() -> Self {
        Self {
            frame_count: 0,
            elapsed: Duration::ZERO,
            fps: 0.0,
        }
    }
    
    fn update(&mut self, dt: Duration) {
        self.frame_count += 1;
        self.elapsed += dt;
        
        if self.elapsed >= Duration::from_secs(1) {
            self.fps = self.frame_count as f32 / self.elapsed.as_secs_f32();
            info!("FPS: {:.1} | Sim Ticks: {} | Tick Count: {}", 
                self.fps, self.frame_count, 0); // TODO: Get actual tick count
            self.frame_count = 0;
            self.elapsed = Duration::ZERO;
        }
    }
}

fn print_usage() {
    println!("DocDamage Engine (DDE)");
    println!("Usage:");
    println!("  dde                      # Create temporary demo project");
    println!("  dde new <name>           # Create new project");
    println!("  dde open <path>          # Open existing project");
    println!();
    println!("Controls:");
    println!("  WASD / Arrow Keys        # Move");
    println!("  Shift                    # Run");
    println!("  ESC                      # Exit");
    println!();
}

fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();
    
    let mut project_manager = ProjectManager::new();
    
    match args.len() {
        1 => {
            // No arguments - create demo project
            info!("No project specified, creating demo project...");
            let demo_path = project::default_project_path().join("demo.dde");
            project_manager.create_new(&demo_path, "Demo Project")?;
        }
        2 => {
            if args[1] == "--help" || args[1] == "-h" {
                print_usage();
                return Ok(());
            }
            // Single argument - treat as project file to open
            let path = PathBuf::from(&args[1]);
            project_manager.open(&path)?;
        }
        3 => {
            match args[1].as_str() {
                "new" => {
                    let name = &args[2];
                    let path = project::default_project_path()
                        .join(project::project_filename(name));
                    project_manager.create_new(&path, name)?;
                }
                "open" => {
                    let path = PathBuf::from(&args[2]);
                    project_manager.open(&path)?;
                }
                _ => {
                    print_usage();
                    anyhow::bail!("Unknown command: {}", args[1]);
                }
            }
        }
        _ => {
            print_usage();
            anyhow::bail!("Too many arguments");
        }
    }
    
    info!("═══════════════════════════════════════════");
    info!("  DocDamage Engine (DDE)");
    info!("  Week 2: Player, Movement, Camera");
    info!("  Version: {}", env!("CARGO_PKG_VERSION"));
    info!("═══════════════════════════════════════════");
    
    // Create event loop
    let event_loop = EventLoop::new()?;
    
    // Create window
    let window_attributes = Window::default_attributes()
        .with_title("DocDamage Engine - Week 2 Demo")
        .with_inner_size(LogicalSize::new(1280.0, 720.0));
    
    let window = event_loop.create_window(window_attributes)?;
    
    // Create renderer
    let mut renderer = pollster::block_on(Renderer::new(window));
    
    // Create app state
    let mut app = App::new(12345);
    app.project_manager = project_manager;
    
    // Initialize world (spawn player, NPCs)
    app.init_world();
    
    // Initial camera position
    let world_size = renderer.world_size().unwrap_or(Vec2::new(2048.0, 2048.0));
    info!("World size: {:.0}x{:.0}", world_size.x, world_size.y);
    info!("Tick rate: {} Hz ({}ms)", 1000 / TICK_RATE.as_millis(), TICK_RATE.as_millis());
    info!("Controls: WASD to move, Shift to run, ESC to exit");
    info!("═══════════════════════════════════════════");
    
    // Run event loop
    event_loop.run(move |event, elwt| {
        elwt.set_control_flow(ControlFlow::Poll);
        
        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => {
                    info!("Shutting down...");
                    app.project_manager.close();
                    elwt.exit();
                }
                WindowEvent::Resized(physical_size) => {
                    renderer.resize(physical_size);
                }
                WindowEvent::KeyboardInput { event, .. } => {
                    app.input_system.handle_key_event(&event);
                    
                    if event.logical_key == winit::keyboard::Key::Named(winit::keyboard::NamedKey::Escape) {
                        info!("ESC pressed - shutting down...");
                        app.project_manager.close();
                        elwt.exit();
                    }
                }
                _ => {}
            },
            Event::AboutToWait => {
                // Calculate delta time
                let now = Instant::now();
                let dt = now - app.last_frame;
                app.last_frame = now;
                
                // Update app state
                app.update(dt);
                
                // Update camera (smooth follow player)
                renderer.update_camera(app.camera_target, dt.as_secs_f32());
                
                // Render
                match renderer.render() {
                    Ok(_) => {}
                    Err(wgpu::SurfaceError::Lost) => renderer.resize(renderer.size()),
                    Err(wgpu::SurfaceError::OutOfMemory) => {
                        tracing::error!("Out of memory");
                        elwt.exit();
                    }
                    Err(e) => tracing::error!("Render error: {:?}", e),
                }
            }
            Event::LoopExiting => {
                info!("Goodbye!");
            }
            _ => {}
        }
    })?;
    
    Ok(())
}
