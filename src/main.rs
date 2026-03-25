//! DocDamage Engine - Main Entry Point
//! 
//! A desktop RPG maker and simulation engine.

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

/// Application state
struct App {
    world: World,
    event_bus: EventBus,
    simulation: Simulation,
    input_state: InputState,
    last_frame: Instant,
    fps_counter: FpsCounter,
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
            info!("FPS: {:.1}", self.fps);
            self.frame_count = 0;
            self.elapsed = Duration::ZERO;
        }
    }
    
    fn fps(&self) -> f32 {
        self.fps
    }
}

fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    info!("DocDamage Engine starting...");
    info!("Version: {}-{}", env!("CARGO_PKG_VERSION"), env!("CARGO_PKG_NAME"));
    
    // Create event loop
    let event_loop = EventLoop::new()?;
    
    // Create window
    let window_attributes = Window::default_attributes()
        .with_title("DocDamage Engine")
        .with_inner_size(LogicalSize::new(1280.0, 720.0));
    
    let window = event_loop.create_window(window_attributes)?;
    
    info!("Window created: 1280x720");
    
    // Create renderer
    let mut renderer = pollster::block_on(Renderer::new(window));
    
    // Create app state
    let mut app = App::new(12345); // Fixed seed for now
    
    info!("Engine initialized successfully");
    info!("Tick rate: {} Hz", 1000 / TICK_RATE.as_millis());
    
    // Run event loop
    event_loop.run(move |event, elwt| {
        elwt.set_control_flow(ControlFlow::Poll);
        
        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => {
                    info!("Window close requested");
                    elwt.exit();
                }
                WindowEvent::Resized(physical_size) => {
                    renderer.resize(physical_size);
                    info!("Window resized: {}x{}", physical_size.width, physical_size.height);
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
                
                // Update camera (follow origin for now)
                renderer.update_camera(glam::Vec2::ZERO, dt.as_secs_f32());
                
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
                info!("Engine shutting down...");
            }
            _ => {}
        }
    })?;
    
    Ok(())
}
