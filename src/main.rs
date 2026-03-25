//! DocDamage Engine - Main Entry Point
//! 
//! A desktop RPG maker and simulation engine.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use tracing::info;
use winit::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};

use dde_core::{TICK_RATE, World};
use dde_core::events::EventBus;
use dde_core::resources::InputState;
use dde_core::systems::simulation::Simulation;
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
    camera_target: glam::Vec2,
    project_manager: ProjectManager,
}

impl App {
    fn new(seed: u64) -> Self {
        let world = World::new();
        let event_bus = EventBus::new();
        let simulation = Simulation::new(seed);
        let input_state = InputState::default();
        
        Self {
            world,
            event_bus,
            simulation,
            input_state,
            last_frame: Instant::now(),
            fps_counter: FpsCounter::new(),
            camera_target: glam::Vec2::ZERO,
            project_manager: ProjectManager::new(),
        }
    }
    
    fn update(&mut self, dt: Duration) {
        // Update FPS counter
        self.fps_counter.update(dt);
        
        // Run simulation tick(s)
        self.simulation.update(dt, &mut self.world, &self.event_bus);
        
        // Process events
        let events = self.event_bus.drain();
        for event in events {
            info!("Event: {:?}", event);
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
            info!("FPS: {:.1} | Sim Ticks: {}", self.fps, self.frame_count);
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
    info!("  Version: {}", env!("CARGO_PKG_VERSION"));
    info!("═══════════════════════════════════════════");
    
    // Create event loop
    let event_loop = EventLoop::new()?;
    
    // Create window
    let window_attributes = Window::default_attributes()
        .with_title("DocDamage Engine - Week 1 Demo")
        .with_inner_size(LogicalSize::new(1280.0, 720.0));
    
    let window = event_loop.create_window(window_attributes)?;
    
    // Create renderer
    let mut renderer = pollster::block_on(Renderer::new(window));
    
    // Set camera target to center of world
    let world_size = renderer.world_size().unwrap_or(glam::Vec2::new(2048.0, 2048.0));
    let camera_target = world_size / 2.0;
    
    // Create app state
    let mut app = App::new(12345);
    app.camera_target = camera_target;
    app.project_manager = project_manager;
    
    info!("World size: {:.0}x{:.0}", world_size.x, world_size.y);
    info!("Camera target: {:.0}, {:.0}", camera_target.x, camera_target.y);
    info!("Tick rate: {} Hz ({}ms)", 1000 / TICK_RATE.as_millis(), TICK_RATE.as_millis());
    info!("Press ESC or close window to exit");
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
                
                // Update app state (runs simulation ticks)
                app.update(dt);
                
                // Update camera (smooth follow)
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
