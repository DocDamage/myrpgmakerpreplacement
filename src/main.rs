//! DocDamage Engine - Main Entry Point
//!
//! A desktop RPG maker and simulation engine.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use glam::Vec2;
use tracing::info;
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

use dde_core::events::EngineEventBus;
use dde_core::resources::InputState;
use dde_core::systems::{
    dialogue::DialogueManager, simulation::Simulation, BarkSystem, InputSystem, MovementSystem,
    NpcBark, PlayerController, TileCollisionMap,
};
use dde_core::{World, TICK_RATE};
use dde_render::Renderer;

use save::{AssetBrowser, SaveMenu, SaveMenuMode};

mod project;
use project::ProjectManager;

mod save;
use save::SaveManager;

/// Application state
struct App {
    world: World,
    event_bus: EngineEventBus,
    simulation: Simulation,
    #[allow(dead_code)]
    input_state: InputState,
    last_frame: Instant,
    fps_counter: FpsCounter,
    camera_target: Vec2,
    project_manager: ProjectManager,
    input_system: InputSystem,
    player_controller: PlayerController,
    tile_collision: TileCollisionMap,
    bark_system: BarkSystem,
    #[allow(dead_code)]
    dialogue_manager: DialogueManager,
    game_time: f32,
    save_manager: SaveManager,
    save_menu: SaveMenu,
    asset_browser: AssetBrowser,
    // Week 8: Asset Forge integration - kept for future UI state tracking
    #[allow(dead_code)]
    asset_forge_open: bool,
    // Renderer
    renderer: Option<Renderer>,
    // Egui context for UI
    egui_ctx: egui::Context,
    egui_scale_factor: f32,
    // Pending save action to handle
    pending_save_action: SaveMenuResult,
    // Pending asset browser action
    pending_asset_selection: AssetBrowserResult,
}

impl App {
    fn new(seed: u64) -> Self {
        let world = World::new();
        let event_bus = EngineEventBus::new();
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
            bark_system: BarkSystem::new(),
            dialogue_manager: DialogueManager::new(),
            game_time: 0.0,
            save_manager: SaveManager::default(),
            save_menu: SaveMenu::new(),
            asset_browser: AssetBrowser::new(),
            asset_forge_open: false,
            renderer: None,
            egui_ctx: egui::Context::default(),
            egui_scale_factor: 1.0,
            pending_save_action: SaveMenuResult::None,
            pending_asset_selection: AssetBrowserResult::None,
        }
    }

    /// Open the Asset Forge
    fn open_asset_forge(&mut self) {
        info!("Opening Asset Forge...");
        info!("(Week 8: Asset Forge opens in browser at http://localhost:3000)");
        info!("Make sure the sprite_generator Next.js app is running: cd sprite_generator && npm run dev");

        // Open browser to Asset Forge
        if let Err(e) = open::that("http://localhost:3000") {
            info!("Failed to open browser: {}", e);
        } else {
            info!("Asset Forge opened in browser");
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

        // Update game time
        self.game_time += dt.as_secs_f32();

        // Update bark system
        let player_pos = self
            .player_controller
            .world_position(&self.world)
            .unwrap_or(Vec2::ZERO);
        self.bark_system.update(
            &mut self.world,
            dt.as_secs_f32(),
            self.game_time,
            player_pos,
        );

        // Run simulation tick(s)
        self.simulation.update(dt, &mut self.world, &self.event_bus);

        // Update save system (handles autosave)
        self.save_manager.update(
            dt,
            &self.world,
            self.simulation.seed(),
            self.simulation.tick_count(),
        );

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
        self.player_controller
            .spawn_player(&mut self.world, player_x, player_y);
        info!("Player spawned at ({}, {})", player_x, player_y);

        // Spawn some NPCs
        for i in 0..5 {
            let x = 20 + i * 8;
            let y = 25;
            let entity = self.world.spawn((
                dde_core::components::EntityKindComp {
                    kind: dde_core::EntityKind::Npc,
                },
                dde_core::components::Name::new(format!("NPC {}", i + 1), format!("npc_{}", i)),
                dde_core::components::Position::new(x, y, 0),
                dde_core::components::SubPosition::default(),
                dde_core::components::behavior::MovementSpeed::from_spd_stat(5),
                dde_core::Direction4::Down,
                NpcBark::new()
                    .with_cooldown(10.0 + i as f32 * 2.0) // Stagger cooldowns
                    .with_proximity(4.0),
            ));
            info!("Spawned NPC {} at ({}, {})", i + 1, x, y);
            let _ = entity;
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Create window
        let window_attributes = Window::default_attributes()
            .with_title("DocDamage Engine - Week 9 Demo")
            .with_inner_size(LogicalSize::new(1280.0, 720.0));

        let window = event_loop
            .create_window(window_attributes)
            .expect("Failed to create window");

        // Create renderer
        let renderer = pollster::block_on(Renderer::new(window));
        self.renderer = Some(renderer);

        // Initialize world (spawn player, NPCs)
        self.init_world();

        // Initial camera position
        if let Some(renderer) = &self.renderer {
            let world_size = renderer.world_size().unwrap_or(Vec2::new(2048.0, 2048.0));
            info!("World size: {:.0}x{:.0}", world_size.x, world_size.y);
        }
        info!(
            "Tick rate: {} Hz ({}ms)",
            1000 / TICK_RATE.as_millis(),
            TICK_RATE.as_millis()
        );
        info!("Controls: WASD to move, Shift to run, ESC to exit");
        info!("Save/Load: Ctrl+S to save, Ctrl+L to load");
        info!("Asset Forge: Ctrl+A to open");
        info!("═══════════════════════════════════════════");
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                info!("Shutting down...");
                self.project_manager.close();
                event_loop.exit();
            }
            WindowEvent::Resized(physical_size) => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(physical_size);
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                self.input_system.handle_key_event(&event);

                // Handle save/load shortcuts
                if let winit::keyboard::Key::Character(ch) = &event.logical_key {
                    if event.state == winit::event::ElementState::Pressed {
                        let ctrl_held =
                            self.input_system.is_action_held(dde_core::InputAction::Run);

                        if ctrl_held {
                            match ch.as_str() {
                                "s" => {
                                    // Open save menu
                                    self.save_menu.show(SaveMenuMode::Save);
                                    info!("Save menu opened");
                                }
                                "l" => {
                                    // Open load menu
                                    self.save_menu.show(SaveMenuMode::Load);
                                    info!("Load menu opened");
                                }
                                "a" => {
                                    // Open Asset Forge
                                    self.open_asset_forge();
                                }
                                "b" => {
                                    // Toggle asset browser
                                    self.asset_browser.toggle();
                                    info!("Asset browser toggled");
                                }
                                _ => {}
                            }
                        }
                    }
                }

                if event.logical_key
                    == winit::keyboard::Key::Named(winit::keyboard::NamedKey::Escape)
                {
                    info!("ESC pressed - shutting down...");
                    self.project_manager.close();
                    event_loop.exit();
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        // Calculate delta time
        let now = Instant::now();
        let dt = now - self.last_frame;
        self.last_frame = now;

        // Update app state
        self.update(dt);

        // Handle save/load actions from previous frame's UI
        self.handle_pending_actions();

        // Update camera (smooth follow player)
        if let Some(renderer) = &mut self.renderer {
            renderer.update_camera(self.camera_target, dt.as_secs_f32());

            // Collect UI state before borrowing
            let save_menu_visible = self.save_menu.is_visible();
            let asset_browser_visible = self.asset_browser.is_visible();
            let scale_factor = self.egui_scale_factor;
            let ctx = self.egui_ctx.clone();

            // We'll store results to handle after render
            let mut save_menu_result = SaveMenuResult::None;
            let mut asset_browser_result = AssetBrowserResult::None;

            // Render with UI
            let render_result = if save_menu_visible || asset_browser_visible {
                // For save menu and asset browser, we need to handle results
                if save_menu_visible {
                    save_menu_result =
                        draw_save_menu(&ctx, &mut self.save_menu, &mut self.save_manager);
                }
                if asset_browser_visible {
                    asset_browser_result = draw_asset_browser(&ctx, &mut self.asset_browser);
                }

                renderer.render_with_ui(&ctx, scale_factor, |_ui_ctx| {
                    // UI was already drawn above to capture results
                })
            } else {
                renderer.render()
            };

            // Store results for next frame handling (to avoid borrow issues)
            self.pending_save_action = save_menu_result;
            self.pending_asset_selection = asset_browser_result;

            match render_result {
                Ok(_) => {}
                Err(wgpu::SurfaceError::Lost) => {
                    let size = renderer.size();
                    renderer.resize(size);
                }
                Err(wgpu::SurfaceError::OutOfMemory) => {
                    tracing::error!("Out of memory");
                    _event_loop.exit();
                }
                Err(e) => tracing::error!("Render error: {:?}", e),
            }
        }
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        info!("Goodbye!");
    }
}

impl App {
    /// Handle any pending save/load actions and asset selections
    fn handle_pending_actions(&mut self) {
        // Handle save/load actions
        match self.pending_save_action {
            SaveMenuResult::Save(slot) => {
                info!("Saving to slot {}...", slot);
                match self.save_manager.save_to_slot(
                    &self.world,
                    self.simulation.seed(),
                    self.simulation.tick_count(),
                    slot,
                ) {
                    Ok(_) => info!("Saved successfully to slot {}!", slot),
                    Err(e) => info!("Save failed: {}", e),
                }
                self.pending_save_action = SaveMenuResult::None;
            }
            SaveMenuResult::Load(slot) => {
                info!("Loading from slot {}...", slot);
                match self.save_manager.load_from_slot(&mut self.world, slot) {
                    Ok(_) => {
                        info!("Loaded successfully from slot {}!", slot);
                        // Re-find player entity after load
                        self.player_controller.find_player(&self.world);
                    }
                    Err(e) => info!("Load failed: {}", e),
                }
                self.pending_save_action = SaveMenuResult::None;
            }
            SaveMenuResult::None => {}
        }

        // Handle asset browser selections
        match &self.pending_asset_selection {
            AssetBrowserResult::Selected(id, asset_type) => {
                info!("Asset selected: {} ({:?})", id, asset_type);
                // TODO: Load/use the selected asset based on type
                // For now, just log it - actual integration depends on asset type
                self.pending_asset_selection = AssetBrowserResult::None;
            }
            AssetBrowserResult::None => {}
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
            info!(
                "FPS: {:.1} | Sim Ticks: {} | Tick Count: {}",
                self.fps, self.frame_count, 0
            ); // TODO: Get actual tick count
            self.frame_count = 0;
            self.elapsed = Duration::ZERO;
        }
    }
}

/// Save menu action result
#[derive(Debug, Clone, Copy, PartialEq)]
enum SaveMenuResult {
    None,
    Save(u32),
    Load(u32),
}

/// Draw the save menu UI
fn draw_save_menu(
    ctx: &egui::Context,
    menu: &mut SaveMenu,
    save_manager: &mut SaveManager,
) -> SaveMenuResult {
    let mut result = SaveMenuResult::None;
    let window_title = match menu.mode {
        SaveMenuMode::Save => "Save Game",
        SaveMenuMode::Load => "Load Game",
    };

    egui::Window::new(window_title)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .show(ctx, |ui| {
            ui.label("Select a save slot:");
            ui.add_space(10.0);

            let saves = save_manager.get_all_saves();

            for (slot, save_opt) in saves.iter().take(6) {
                let button_text = match save_opt {
                    Some(save) => {
                        let play_time = format!(
                            "{}h {}m",
                            save.play_time_secs / 3600,
                            (save.play_time_secs % 3600) / 60
                        );
                        format!("Slot {}: {} - {}", slot + 1, save.player_name, play_time)
                    }
                    None => format!("Slot {}: [Empty]", slot + 1),
                };

                if ui.button(&button_text).clicked() {
                    match menu.mode {
                        SaveMenuMode::Save => {
                            result = SaveMenuResult::Save(*slot);
                        }
                        SaveMenuMode::Load => {
                            if save_opt.is_some() {
                                result = SaveMenuResult::Load(*slot);
                            }
                        }
                    }
                    menu.hide();
                }
            }

            ui.add_space(10.0);
            if ui.button("Cancel").clicked() {
                menu.hide();
            }
        });

    result
}

/// Asset browser action result
#[derive(Debug, Clone)]
enum AssetBrowserResult {
    None,
    Selected(String, save::asset_browser::AssetType),
}

/// Draw the asset browser UI
fn draw_asset_browser(ctx: &egui::Context, browser: &mut AssetBrowser) -> AssetBrowserResult {
    let mut result = AssetBrowserResult::None;
    let mut selected_id: Option<String> = None;
    let mut selected_type: Option<save::asset_browser::AssetType> = None;

    egui::Window::new("Asset Browser")
        .default_size([400.0, 500.0])
        .collapsible(true)
        .show(ctx, |ui| {
            // Search bar
            ui.horizontal(|ui| {
                ui.label("Search:");
                let mut query = browser.search_query.clone();
                if ui.text_edit_singleline(&mut query).changed() {
                    browser.set_search(query);
                }
            });

            ui.add_space(5.0);

            // Type filter buttons
            ui.horizontal(|ui| {
                if ui.button("All").clicked() {
                    browser.set_filter(None);
                }
                use save::asset_browser::AssetType;
                for asset_type in [AssetType::Sprite, AssetType::Audio, AssetType::Script] {
                    if ui.button(asset_type.name()).clicked() {
                        browser.set_filter(Some(asset_type));
                    }
                }
            });

            ui.separator();

            // Asset list - collect data first to avoid borrow issues
            let assets_to_show: Vec<_> = browser
                .filtered_assets()
                .iter()
                .map(|a| (a.asset_type, a.name.clone(), a.format_size(), a.id.clone()))
                .collect();

            egui::ScrollArea::vertical().show(ui, |ui| {
                for (asset_type, name, size, id) in assets_to_show {
                    ui.horizontal(|ui| {
                        ui.label(asset_type.name());
                        ui.label(&name);
                        ui.label(size);
                        if ui.button("Select").clicked() {
                            selected_id = Some(id);
                            selected_type = Some(asset_type);
                            info!("Selected asset: {}", name);
                        }
                    });
                }
            });

            ui.separator();

            if ui.button("Close").clicked() {
                browser.hide();
            }
        });

    // Apply selection and return result
    if let (Some(id), Some(asset_type)) = (selected_id, selected_type) {
        browser.selected_asset = Some(id.clone());
        result = AssetBrowserResult::Selected(id, asset_type);
    }

    result
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
    println!("  Ctrl+S                   # Open Save Menu");
    println!("  Ctrl+L                   # Open Load Menu");
    println!("  Ctrl+B                   # Toggle Asset Browser");
    println!("  Ctrl+A                   # Open Asset Forge");
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
        3 => match args[1].as_str() {
            "new" => {
                let name = &args[2];
                let path = project::default_project_path().join(project::project_filename(name));
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
        },
        _ => {
            print_usage();
            anyhow::bail!("Too many arguments");
        }
    }

    info!("═══════════════════════════════════════════");
    info!("  DocDamage Engine (DDE)");
    info!("  Week 9: Lua, Pathfinding, Particles");
    info!("  Version: {}", env!("CARGO_PKG_VERSION"));
    info!("═══════════════════════════════════════════");

    // Create event loop
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);

    // Create app state
    let mut app = App::new(12345);
    app.project_manager = project_manager;

    // Run event loop
    event_loop.run_app(&mut app)?;

    Ok(())
}
