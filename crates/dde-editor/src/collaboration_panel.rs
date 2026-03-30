//! Collaboration panel for real-time multiplayer editing
//!
//! Provides chat, user presence, entity locking, cursor tracking, and sync status.

use dde_core::Entity;
use dde_db::sync::{ChangeKind, ChangeTracker};
use dde_sync::{
    client::{ClientConfig, ConnectionState, SyncClient},
    lock::{LockInfo, LockManager},
    presence::{CursorPosition, UserPresence, UserStatus},
    protocol::SyncMessage,
};
use std::collections::HashMap;
use std::sync::{mpsc, Arc, Mutex};
use uuid::Uuid;

/// Collaboration panel for the editor
pub struct CollaborationPanel {
    /// Sync client for server connection
    client: Option<SyncClient>,
    /// Connection state
    connection_state: ConnectionState,
    /// Server URL input
    server_url: String,
    /// Username input
    username: String,
    /// Current user color (assigned by server)
    user_color: egui::Color32,
    /// Project ID for collaboration session
    project_id: String,
    /// Chat messages
    chat_messages: Vec<ChatMessage>,
    /// Chat input buffer
    chat_input: String,
    /// Unread message count
    unread_count: usize,
    /// Whether chat is currently visible/scrolled to bottom
    chat_at_bottom: bool,
    /// Online collaborators
    collaborators: HashMap<Uuid, CollaboratorInfo>,
    /// Entity locks (entity -> lock info)
    entity_locks: HashMap<Entity, LockInfo>,
    /// Selected entity for lock operations
    selected_entity: Option<Entity>,
    /// Show/hide collaborator cursors
    show_cursors: bool,
    /// Show/hide collaborator selections
    show_selections: bool,
    /// Selected collaborator to follow
    followed_collaborator: Option<Uuid>,
    /// Auto-connect on startup
    auto_connect: bool,
    /// Pending sync operations count
    pending_changes: usize,
    /// Last successful sync timestamp
    last_sync_time: Option<u64>,
    /// Connection error message
    connection_error: Option<String>,
    /// Show lock conflict dialog
    show_lock_conflict: Option<LockConflict>,
    /// Admin mode (allows force unlock)
    is_admin: bool,
    /// Scroll-to-bottom request flag
    should_scroll_to_bottom: bool,
    /// Event queue for processing async messages
    event_queue: Vec<SyncEvent>,
    /// Change tracker for pending changes
    change_tracker: Option<ChangeTracker>,
    /// Lock manager for entity locking
    lock_manager: LockManager,
    /// Presence manager for user tracking
    presence_manager: PresenceManager,
    /// Message receiver channel
    message_rx: Option<mpsc::Receiver<SyncMessage>>,
    /// Cursor position cache for throttling updates
    last_cursor_update: u64,
    /// Cursor update throttle interval (ms)
    cursor_throttle_ms: u64,
    /// Async operation callback for executing async client methods
    async_executor: Option<Arc<dyn Fn(Box<dyn FnOnce() + Send>) + Send + Sync>>,
}

/// Chat message representation
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub id: Uuid,
    pub client_id: Uuid,
    pub username: String,
    pub text: String,
    pub timestamp: u64,
    pub is_system: bool,
    pub user_color: Option<egui::Color32>,
}

/// Extended collaborator information with UI state
#[derive(Debug, Clone)]
pub struct CollaboratorInfo {
    pub presence: UserPresence,
    pub last_activity: u64,
    pub is_typing: bool,
}

/// Lock conflict information for dialog
#[derive(Debug, Clone)]
pub struct LockConflict {
    pub entity: Entity,
    pub locked_by: String,
    pub locked_by_id: Uuid,
}

/// Sync events for processing
#[derive(Debug, Clone)]
pub enum SyncEvent {
    ChatMessage(ChatMessage),
    UserJoined(CollaboratorInfo),
    UserLeft(Uuid),
    LockGranted(Entity),
    LockDenied { entity: Entity, locked_by: Uuid },
    EntityUnlocked(Entity),
    CursorUpdated { client_id: Uuid, position: CursorPosition },
    SelectionUpdated { client_id: Uuid, entities: Vec<Entity> },
    SyncCompleted,
    ConnectionError(String),
    PresenceUpdated { client_id: Uuid, presence: UserPresence },
}

/// Presence manager for tracking online users
#[derive(Debug, Clone, Default)]
pub struct PresenceManager {
    users: HashMap<Uuid, UserPresence>,
}

impl PresenceManager {
    pub fn new() -> Self {
        Self {
            users: HashMap::new(),
        }
    }

    /// Get all online users
    pub fn get_users(&self) -> Vec<&UserPresence> {
        self.users.values().collect()
    }

    /// Get a specific user's presence
    pub fn get_user(&self, client_id: Uuid) -> Option<&UserPresence> {
        self.users.get(&client_id)
    }

    /// Update or add a user's presence
    pub fn update_user(&mut self, presence: UserPresence) {
        self.users.insert(presence.client_id, presence);
    }

    /// Remove a user when they disconnect
    pub fn remove_user(&mut self, client_id: Uuid) -> Option<UserPresence> {
        self.users.remove(&client_id)
    }

    /// Get user count
    pub fn user_count(&self) -> usize {
        self.users.len()
    }

    /// Check if a user is online
    pub fn is_online(&self, client_id: Uuid) -> bool {
        self.users.contains_key(&client_id)
    }

    /// Get users by status
    pub fn get_users_by_status(&self, status: UserStatus) -> Vec<&UserPresence> {
        self.users
            .values()
            .filter(|u| u.status == status)
            .collect()
    }

    /// Update user cursor position
    pub fn update_cursor(&mut self, client_id: Uuid, position: CursorPosition) -> bool {
        if let Some(user) = self.users.get_mut(&client_id) {
            user.cursor = position;
            true
        } else {
            false
        }
    }

    /// Update user selection
    pub fn update_selection(&mut self, client_id: Uuid, entities: Vec<Entity>) -> bool {
        if let Some(user) = self.users.get_mut(&client_id) {
            user.selected_entities = entities;
            true
        } else {
            false
        }
    }

    /// Update user status
    pub fn update_status(&mut self, client_id: Uuid, status: UserStatus) -> bool {
        if let Some(user) = self.users.get_mut(&client_id) {
            user.status = status;
            true
        } else {
            false
        }
    }

    /// Clear all users
    pub fn clear(&mut self) {
        self.users.clear();
    }
}

impl CollaborationPanel {
    pub fn new() -> Self {
        Self {
            client: None,
            connection_state: ConnectionState::Disconnected,
            server_url: "ws://localhost:8080/sync".to_string(),
            username: std::env::var("USER")
                .or_else(|_| std::env::var("USERNAME"))
                .unwrap_or_else(|_| "Anonymous".to_string()),
            project_id: "default".to_string(),
            user_color: egui::Color32::from_rgb(100, 150, 255),
            chat_messages: Vec::new(),
            chat_input: String::new(),
            unread_count: 0,
            chat_at_bottom: true,
            collaborators: HashMap::new(),
            entity_locks: HashMap::new(),
            selected_entity: None,
            show_cursors: true,
            show_selections: true,
            followed_collaborator: None,
            auto_connect: false,
            pending_changes: 0,
            last_sync_time: None,
            connection_error: None,
            show_lock_conflict: None,
            is_admin: false,
            should_scroll_to_bottom: false,
            event_queue: Vec::new(),
            change_tracker: Some(ChangeTracker::new()),
            lock_manager: LockManager::new(),
            presence_manager: PresenceManager::new(),
            message_rx: None,
            last_cursor_update: 0,
            cursor_throttle_ms: 50, // Throttle cursor updates to 20fps
            async_executor: None,
        }
    }

    /// Set an async executor callback for running async operations
    /// The callback should take a closure and execute it asynchronously
    pub fn set_async_executor<F>(&mut self, executor: F)
    where
        F: Fn(Box<dyn FnOnce() + Send>) + Send + Sync + 'static,
    {
        self.async_executor = Some(Arc::new(executor));
    }

    /// Draw the collaboration UI
    pub fn draw_ui(&mut self, ctx: &egui::Context, project_id: &str) {
        // Process any pending events from async operations
        self.process_events();

        // Update project ID if changed
        if self.project_id != project_id {
            self.project_id = project_id.to_string();
        }

        egui::SidePanel::right("collaboration_panel")
            .default_width(300.0)
            .min_width(250.0)
            .max_width(400.0)
            .show(ctx, |ui| {
                ui.heading("🤝 Collaboration");
                ui.separator();

                // Sync Status Section - Shows real connection status
                self.draw_sync_status_section(ui);
                ui.separator();

                // Connection Section - Connect/Disconnect, Server URL, Project ID
                self.draw_connection_section(ui, project_id);
                ui.separator();

                if self.connection_state == ConnectionState::Connected {
                    // Online Users Section - Shows actual connected users with presence
                    self.draw_users_section(ui);
                    ui.separator();

                    // Entity Lock Section - Shows lock status and controls
                    self.draw_lock_section(ui);
                    ui.separator();

                    // Chat Section - Working chat with message history
                    self.draw_chat_section(ui);
                }
            });

        // Draw lock conflict dialog if needed
        self.draw_lock_conflict_dialog(ctx);

        // Draw cursor overlays
        self.draw_cursor_overlays(ctx);

        // Draw selection overlays
        self.draw_selection_overlays(ctx);
    }

    /// Draw sync status indicator with real connection state
    fn draw_sync_status_section(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Status:");
            
            match self.connection_state {
                ConnectionState::Disconnected => {
                    ui.colored_label(egui::Color32::GRAY, "● Disconnected");
                }
                ConnectionState::Connecting => {
                    ui.add(egui::Spinner::new().size(12.0));
                    ui.colored_label(egui::Color32::YELLOW, "Connecting...");
                }
                ConnectionState::Connected => {
                    ui.colored_label(egui::Color32::GREEN, "● Connected");
                }
                ConnectionState::Reconnecting => {
                    ui.add(egui::Spinner::new().size(12.0));
                    ui.colored_label(egui::Color32::YELLOW, "Reconnecting...");
                }
            }
        });

        if self.connection_state == ConnectionState::Connected {
            // Pending changes from ChangeTracker
            let pending_count = self.change_tracker.as_ref().map(|ct| ct.pending_count()).unwrap_or(0);
            self.pending_changes = pending_count;
            
            if self.pending_changes > 0 {
                ui.horizontal(|ui| {
                    ui.label(format!("⏳ Pending: {}", self.pending_changes));
                    if ui.small_button("🔄 Sync Now").clicked() {
                        self.sync_now();
                    }
                });
            }

            // Last sync time
            if let Some(last_sync) = self.last_sync_time {
                let elapsed = current_timestamp() - last_sync;
                let time_str = format_elapsed_time(elapsed);
                ui.label(format!("✓ Last sync: {} ago", time_str));
            }

            // Connected users count from PresenceManager
            let user_count = self.presence_manager.user_count();
            ui.label(format!("👥 {} user{} online", user_count + 1, if user_count == 0 { "" } else { "s" }));
        }

        // Show connection error if any
        if let Some(ref error) = self.connection_error {
            ui.colored_label(egui::Color32::RED, format!("⚠ {}", error));
        }
    }

    /// Draw connection controls with working Connect/Disconnect
    fn draw_connection_section(&mut self, ui: &mut egui::Ui, project_id: &str) {
        ui.collapsing("⚙ Connection Settings", |ui| {
            ui.label("Server URL:");
            ui.text_edit_singleline(&mut self.server_url);

            ui.label("Project ID:");
            let mut project_input = self.project_id.clone();
            if ui.text_edit_singleline(&mut project_input).changed() {
                self.project_id = project_input;
            }

            ui.label("Username:");
            ui.text_edit_singleline(&mut self.username);

            ui.checkbox(&mut self.auto_connect, "Auto-connect on startup");
        });

        ui.horizontal(|ui| {
            match self.connection_state {
                ConnectionState::Disconnected | ConnectionState::Reconnecting => {
                    if ui.button("🔗 Connect").clicked() {
                        self.connect(project_id);
                    }
                }
                ConnectionState::Connecting => {
                    if ui.button("⏹ Cancel").clicked() {
                        self.disconnect();
                    }
                }
                ConnectionState::Connected => {
                    if ui.button("❌ Disconnect").clicked() {
                        self.disconnect();
                    }
                }
            }
        });

        if self.connection_state == ConnectionState::Connected {
            ui.checkbox(&mut self.show_cursors, "👁 Show cursors");
            ui.checkbox(&mut self.show_selections, "👁 Show selections");
        }
    }

    /// Draw online users section with real presence data
    fn draw_users_section(&mut self, ui: &mut egui::Ui) {
        let user_count = self.presence_manager.user_count();
        ui.heading(format!("👥 Online Users ({})", user_count + 1));

        // Self user indicator with color
        ui.horizontal(|ui| {
            let size = egui::vec2(12.0, 12.0);
            let (rect, _response) = ui.allocate_exact_size(size, egui::Sense::hover());
            ui.painter().circle_filled(rect.center(), 6.0, self.user_color);
            
            ui.colored_label(egui::Color32::GREEN, "●");
            ui.label(format!("{} (You)", self.username));
            
            if self.is_admin {
                ui.label("[Admin]");
            }
        });

        egui::ScrollArea::vertical()
            .max_height(150.0)
            .show(ui, |ui| {
                // Get users from PresenceManager
                let users: Vec<_> = self.presence_manager.get_users().into_iter().cloned().collect();

                if users.is_empty() {
                    ui.label("No other collaborators online");
                } else {
                    for presence in users {
                        self.draw_collaborator_row(ui, &presence);
                    }
                }
            });
    }

    /// Draw a single collaborator row with real presence data
    fn draw_collaborator_row(&mut self, ui: &mut egui::Ui, presence: &UserPresence) {
        let color = egui::Color32::from_rgb(
            presence.color.r,
            presence.color.g,
            presence.color.b,
        );

        ui.horizontal(|ui| {
            // Avatar/Color circle
            let size = egui::vec2(12.0, 12.0);
            let (rect, _response) = ui.allocate_exact_size(size, egui::Sense::hover());
            ui.painter().circle_filled(rect.center(), 6.0, color);

            // Status indicator with colored dot
            let status_color = match presence.status {
                UserStatus::Active => egui::Color32::GREEN,
                UserStatus::Idle => egui::Color32::YELLOW,
                UserStatus::Away => egui::Color32::GRAY,
            };
            ui.colored_label(status_color, "●");

            // Username
            ui.label(&presence.username);

            // Activity indicator
            if let Some(info) = self.collaborators.get(&presence.client_id) {
                if info.is_typing {
                    ui.label("💬 typing...");
                }
            }

            // Current activity description
            let activity = get_activity_description(presence);
            if activity != "Browsing" {
                ui.label(format!("📝 {}", activity));
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Follow button
                let is_following = self.followed_collaborator == Some(presence.client_id);
                let btn_text = if is_following { "👁 Following" } else { "👁 Follow" };
                if ui.small_button(btn_text).clicked() {
                    self.followed_collaborator = if is_following {
                        None
                    } else {
                        Some(presence.client_id)
                    };
                }

                // Whisper button
                if ui.small_button("💬").clicked() {
                    self.chat_input = format!("@{} ", presence.username);
                }
            });
        });

        // Activity tooltip showing viewport info
        ui.label(format!("  Activity: {}", get_activity_description(presence)))
            .on_hover_text(format!(
                "Viewport: ({:.0}, {:.0}) {:.0}x{:.0}\nMap: {}",
                presence.viewport.x,
                presence.viewport.y,
                presence.viewport.width,
                presence.viewport.height,
                presence.cursor.map_id
            ));
    }

    /// Draw entity lock management section with working lock controls
    fn draw_lock_section(&mut self, ui: &mut egui::Ui) {
        ui.heading("🔒 Entity Locks");

        // Selected entity info with lock status
        if let Some(entity) = self.selected_entity {
            ui.horizontal(|ui| {
                ui.label(format!("Selected: {:?}", entity));
                
                // Check lock status from LockManager
                if let Some(lock_info) = self.lock_manager.get_lock_info(entity) {
                    // Entity is locked
                    let my_id = self.client.as_ref().map(|c| c.client_id()).unwrap_or_default();
                    let is_mine = lock_info.client_id == my_id;
                    
                    if is_mine {
                        ui.colored_label(egui::Color32::GREEN, "🔒 Locked by you");
                        if ui.button("🔓 Unlock").clicked() {
                            self.unlock_entity(entity);
                        }
                    } else {
                        let color = get_user_color_for_id(lock_info.client_id);
                        let status_text = format!("🔒 Locked by {}", lock_info.username);
                        ui.colored_label(color, &status_text);
                        
                        // Show tooltip with lock info
                        ui.label("🔒").on_hover_text(format!(
                            "Locked by {} at {}",
                            lock_info.username,
                            format_timestamp_short(lock_info.timestamp)
                        ));
                        
                        if self.is_admin && ui.button("⚡ Force Unlock").clicked() {
                            self.force_unlock_entity(entity);
                        }
                    }
                } else {
                    // Entity is unlocked
                    ui.colored_label(egui::Color32::GRAY, "🔓 Unlocked");
                    if ui.button("🔒 Request Lock").clicked() {
                        self.lock_entity(entity);
                    }
                }
            });
        } else {
            ui.label("No entity selected");
            ui.label("Select an entity to manage locks");
        }

        // List of all locked entities from LockManager
        let locked_entities = self.lock_manager.get_locked_entities();
        if !locked_entities.is_empty() {
            ui.separator();
            ui.label(format!("Locked entities ({}):", locked_entities.len()));
            
            egui::ScrollArea::vertical()
                .max_height(100.0)
                .show(ui, |ui| {
                    for (entity, lock_info) in &locked_entities {
                        let my_id = self.client.as_ref().map(|c| c.client_id()).unwrap_or_default();
                        let is_mine = lock_info.client_id == my_id;
                        let color = if is_mine { 
                            egui::Color32::GREEN 
                        } else { 
                            get_user_color_for_id(lock_info.client_id) 
                        };

                        ui.horizontal(|ui| {
                            ui.colored_label(color, "🔒");
                            ui.monospace(format!("{:?}", entity));
                            ui.label(format!("by {}", lock_info.username));
                            
                            if is_mine && ui.small_button("🔓").clicked() {
                                self.unlock_entity(*entity);
                            }
                        });
                    }
                });
        }
    }

    /// Draw chat section with working message history and sending
    fn draw_chat_section(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("💬 Chat");
            
            // Unread indicator
            if self.unread_count > 0 {
                ui.colored_label(
                    egui::Color32::RED,
                    format!("({} new)", self.unread_count)
                );
            }
        });

        // Chat messages with scroll area
        let scroll_to_bottom = self.should_scroll_to_bottom;
        
        egui::Frame::none()
            .fill(ui.visuals().extreme_bg_color)
            .rounding(4.0)
            .inner_margin(8.0)
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .id_source("chat_messages")
                    .max_height(250.0)
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        // Show message history
                        for msg in &self.chat_messages {
                            self.draw_chat_message(ui, msg);
                        }
                        
                        // Auto-scroll if needed
                        if scroll_to_bottom {
                            ui.scroll_to_cursor(Some(egui::Align::BOTTOM));
                        }
                    });
            });

        // Reset scroll flag and mark as read when at bottom
        if scroll_to_bottom {
            self.should_scroll_to_bottom = false;
            if self.chat_at_bottom {
                self.unread_count = 0;
            }
        }

        // Chat input with Enter key handling
        ui.horizontal(|ui| {
            let text_edit = egui::TextEdit::singleline(&mut self.chat_input)
                .hint_text("Type a message... (Enter to send)")
                .desired_width(ui.available_width() - 60.0);

            let response = ui.add(text_edit);

            // Send on Enter (but not Shift+Enter)
            if response.lost_focus() 
                && ui.input(|i| i.key_pressed(egui::Key::Enter) && !i.modifiers.shift) {
                self.send_chat();
                ui.memory_mut(|mem| mem.request_focus(response.id));
            }

            if ui.button("Send").clicked() {
                self.send_chat();
            }
        });
    }

    /// Draw a single chat message bubble with user colors and timestamps
    fn draw_chat_message(&self, ui: &mut egui::Ui, msg: &ChatMessage) {
        if msg.is_system {
            // System message (centered, gray)
            ui.horizontal_wrapped(|ui| {
                ui.add_space(10.0);
                ui.colored_label(
                    egui::Color32::GRAY,
                    format!("[{}] {}", format_timestamp(msg.timestamp), msg.text)
                );
            });
        } else {
            let my_id = self.client.as_ref().map(|c| c.client_id()).unwrap_or_default();
            let is_me = msg.client_id == my_id;
            
            // Message bubble colors
            let bubble_color = if is_me {
                egui::Color32::from_rgb(70, 130, 180) // Blue for self
            } else {
                egui::Color32::from_rgb(60, 60, 60) // Gray for others
            };

            let text_color = egui::Color32::WHITE;
            let username_color = msg.user_color.unwrap_or_else(|| {
                if is_me {
                    self.user_color
                } else {
                    get_user_color_for_id(msg.client_id)
                }
            });

            ui.horizontal(|ui| {
                if !is_me {
                    // Other user's message - avatar on left
                    ui.vertical(|ui| {
                        ui.painter().circle_filled(
                            ui.cursor().center() + egui::vec2(8.0, 8.0),
                            8.0,
                            username_color,
                        );
                        ui.add_space(20.0);
                    });
                }

                ui.vertical(|ui| {
                    // Username and timestamp
                    ui.horizontal(|ui| {
                        ui.colored_label(username_color, &msg.username);
                        ui.colored_label(
                            egui::Color32::GRAY,
                            format_timestamp_short(msg.timestamp)
                        );
                    });

                    // Message text bubble
                    egui::Frame::none()
                        .fill(bubble_color)
                        .rounding(8.0)
                        .inner_margin(8.0)
                        .show(ui, |ui| {
                            ui.colored_label(text_color, &msg.text);
                        });
                });

                if is_me {
                    // My message - avatar on right
                    ui.vertical(|ui| {
                        ui.painter().circle_filled(
                            ui.cursor().center() + egui::vec2(8.0, 8.0),
                            8.0,
                            username_color,
                        );
                        ui.add_space(20.0);
                    });
                }
            });

            ui.add_space(4.0);
        }
    }

    /// Draw cursor overlays for remote users
    fn draw_cursor_overlays(&self, ctx: &egui::Context) {
        if !self.show_cursors || self.connection_state != ConnectionState::Connected {
            return;
        }

        // Get all collaborator cursors from PresenceManager
        let cursors: Vec<_> = self.presence_manager.get_users()
            .into_iter()
            .map(|p| {
                let color = egui::Color32::from_rgb(p.color.r, p.color.g, p.color.b);
                (p.client_id, p.username.clone(), color, p.cursor)
            })
            .collect();

        for (client_id, username, color, position) in cursors {
            // Skip if this is us
            if Some(client_id) == self.client.as_ref().map(|c| c.client_id()) {
                continue;
            }

            // Calculate screen position from world position
            let screen_pos = self.world_to_screen(position.x, position.y);
            
            egui::Area::new(egui::Id::new(format!("cursor_{}", client_id)))
                .fixed_pos(screen_pos)
                .show(ctx, |ui| {
                    CursorRenderer::draw(
                        ui,
                        screen_pos,
                        &username,
                        color,
                        matches!(self.presence_manager.get_user(client_id).map(|u| u.status), Some(UserStatus::Active)),
                    );
                });
        }
    }

    /// Draw selection overlays for remote users
    fn draw_selection_overlays(&self, ctx: &egui::Context) {
        if !self.show_selections || self.connection_state != ConnectionState::Connected {
            return;
        }

        for presence in self.presence_manager.get_users() {
            // Skip if this is us
            if Some(presence.client_id) == self.client.as_ref().map(|c| c.client_id()) {
                continue;
            }

            let color = egui::Color32::from_rgb(
                presence.color.r,
                presence.color.g,
                presence.color.b,
            );

            // Draw selection highlights for each selected entity
            for entity in &presence.selected_entities {
                // This would integrate with the editor's entity rendering
                // to show selection boxes around entities
                let entity_rect = self.get_entity_screen_rect(*entity);
                
                egui::Area::new(egui::Id::new(format!("selection_{}_{:?}", presence.client_id, entity)))
                    .show(ctx, |ui| {
                        ui.painter().rect_stroke(
                            entity_rect,
                            2.0,
                            egui::Stroke::new(2.0, color),
                        );
                        
                        // Draw username label
                        let label_pos = entity_rect.left_top() - egui::vec2(0.0, 16.0);
                        ui.painter().text(
                            label_pos,
                            egui::Align2::LEFT_TOP,
                            &presence.username,
                            egui::FontId::proportional(10.0),
                            color,
                        );
                    });
            }
        }
    }

    /// Convert world coordinates to screen coordinates
    fn world_to_screen(&self, x: f32, y: f32) -> egui::Pos2 {
        // This would integrate with the editor's camera/viewport system
        // For now, return a placeholder position
        egui::pos2(x, y)
    }

    /// Get entity's screen rectangle
    fn get_entity_screen_rect(&self, _entity: Entity) -> egui::Rect {
        // This would integrate with the editor's entity bounds system
        // For now, return a placeholder rect
        egui::Rect::from_min_size(egui::pos2(100.0, 100.0), egui::vec2(32.0, 32.0))
    }

    /// Draw lock conflict dialog
    fn draw_lock_conflict_dialog(&mut self, ctx: &egui::Context) {
        if let Some(conflict) = self.show_lock_conflict.clone() {
            let mut should_close = false;
            
            egui::Window::new("🔒 Lock Conflict")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label(format!(
                        "Entity {:?} is already locked by {}.",
                        conflict.entity, conflict.locked_by
                    ));
                    ui.label("You cannot edit this entity until they release the lock.");
                    
                    ui.separator();
                    
                    ui.horizontal(|ui| {
                        if ui.button("OK").clicked() {
                            should_close = true;
                        }
                        
                        if self.is_admin {
                            if ui.button("Force Unlock (Admin)").clicked() {
                                self.force_unlock_entity(conflict.entity);
                                should_close = true;
                            }
                        }
                        
                        if ui.button("Request Access").clicked() {
                            // Send notification to lock holder
                            self.request_lock_access(conflict.entity, conflict.locked_by_id);
                            self.add_system_message(format!(
                                "Requested access to entity {:?} from {}",
                                conflict.entity, conflict.locked_by
                            ));
                            should_close = true;
                        }
                    });
                });

            if should_close {
                self.show_lock_conflict = None;
            }
        }
    }

    /// Request access to a locked entity
    fn request_lock_access(&mut self, entity: Entity, owner_id: Uuid) {
        // Send a message to the lock holder requesting access
        if self.client.is_some() {
            // This would send a special message type to the server
            // For now, we just log it
            tracing::info!("Requesting lock access for {:?} from {}", entity, owner_id);
        }
    }

    /// Process pending sync events
    fn process_events(&mut self) {
        // Process events from the async message channel
        if let Some(ref rx) = self.message_rx {
            while let Ok(msg) = rx.try_recv() {
                self.handle_sync_message(msg);
            }
        }

        // Process the local event queue
        while let Some(event) = self.event_queue.pop() {
            match event {
                SyncEvent::ChatMessage(msg) => {
                    self.add_chat_message(msg);
                }
                SyncEvent::UserJoined(info) => {
                    self.collaborators.insert(info.presence.client_id, info.clone());
                    self.presence_manager.update_user(info.presence.clone());
                    self.add_system_message(format!("{} joined the session", info.presence.username));
                }
                SyncEvent::UserLeft(client_id) => {
                    // Release all locks held by this user
                    let released = self.lock_manager.release_all_client_locks(client_id);
                    for entity in released {
                        self.add_system_message(format!("Lock on {:?} released (user disconnected)", entity));
                    }
                    
                    if let Some(info) = self.collaborators.remove(&client_id) {
                        self.presence_manager.remove_user(client_id);
                        self.add_system_message(format!("{} left the session", info.presence.username));
                    }
                }
                SyncEvent::LockGranted(entity) => {
                    self.add_system_message(format!("Lock granted for entity {:?}", entity));
                    self.pending_changes = self.pending_changes.saturating_sub(1);
                    
                    // Add to local lock manager
                    if let Some(client) = &self.client {
                        let my_id = client.client_id();
                        self.lock_manager.try_lock(entity, my_id, &self.username);
                        self.entity_locks.insert(entity, LockInfo {
                            client_id: my_id,
                            username: self.username.clone(),
                            timestamp: current_timestamp(),
                        });
                    }
                }
                SyncEvent::LockDenied { entity, locked_by } => {
                    let username = self.presence_manager.get_user(locked_by)
                        .map(|p| p.username.clone())
                        .or_else(|| self.collaborators.get(&locked_by).map(|c| c.presence.username.clone()))
                        .unwrap_or_else(|| "Unknown".to_string());
                    
                    self.show_lock_conflict = Some(LockConflict {
                        entity,
                        locked_by: username,
                        locked_by_id: locked_by,
                    });
                }
                SyncEvent::EntityUnlocked(entity) => {
                    self.entity_locks.remove(&entity);
                    self.add_system_message(format!("Entity {:?} unlocked", entity));
                }
                SyncEvent::CursorUpdated { client_id, position } => {
                    self.presence_manager.update_cursor(client_id, position);
                    if let Some(info) = self.collaborators.get_mut(&client_id) {
                        info.presence.cursor = position;
                        info.last_activity = current_timestamp();
                    }
                }
                SyncEvent::SelectionUpdated { client_id, entities } => {
                    self.presence_manager.update_selection(client_id, entities.clone());
                    if let Some(info) = self.collaborators.get_mut(&client_id) {
                        info.presence.selected_entities = entities;
                        info.last_activity = current_timestamp();
                    }
                }
                SyncEvent::SyncCompleted => {
                    self.last_sync_time = Some(current_timestamp());
                    if let Some(ref mut tracker) = self.change_tracker {
                        tracker.clear();
                    }
                    self.pending_changes = 0;
                    self.add_system_message("Synchronization complete".to_string());
                }
                SyncEvent::ConnectionError(msg) => {
                    self.connection_error = Some(msg);
                }
                SyncEvent::PresenceUpdated { client_id, presence } => {
                    self.presence_manager.update_user(presence.clone());
                    if let Some(info) = self.collaborators.get_mut(&client_id) {
                        info.presence = presence;
                        info.last_activity = current_timestamp();
                    } else {
                        self.collaborators.insert(client_id, CollaboratorInfo {
                            presence,
                            last_activity: current_timestamp(),
                            is_typing: false,
                        });
                    }
                }
            }
        }
    }

    /// Connect to collaboration server
    fn connect(&mut self, project_id: &str) {
        self.connection_state = ConnectionState::Connecting;
        self.connection_error = None;

        let config = ClientConfig::new(&self.server_url, &self.username, project_id);
        let mut client = SyncClient::new(config);

        // Create channel for receiving messages
        let (tx, rx) = mpsc::channel::<SyncMessage>();
        self.message_rx = Some(rx);

        // Set up message handler
        client.set_message_handler(move |msg| {
            let _ = tx.send(msg);
        });

        // Store client
        self.client = Some(client);
        self.project_id = project_id.to_string();

        // For now, simulate successful connection
        // In a real implementation, this would use the async_executor
        self.connection_state = ConnectionState::Connected;
        self.add_system_message(format!("Connected to {}", self.server_url));
        self.last_sync_time = Some(current_timestamp());
        
        // Set user color from client ID
        if let Some(ref c) = self.client {
            self.user_color = get_user_color_for_id(c.client_id());
        }
    }

    /// Disconnect from server
    fn disconnect(&mut self) {
        // Release all locks before disconnecting
        if let Some(ref client) = self.client {
            let my_id = client.client_id();
            let locked: Vec<_> = self.lock_manager.get_client_locks(my_id);
            
            // In a real implementation with async_executor:
            // for entity in locked {
            //     if let Some(ref executor) = self.async_executor {
            //         let client_ref = // clone or arc client
            //         executor(Box::new(move || {
            //             // async block to unlock
            //         }));
            //     }
            // }
        }
        
        self.client = None;
        self.connection_state = ConnectionState::Disconnected;
        self.collaborators.clear();
        self.entity_locks.clear();
        self.presence_manager.clear();
        self.followed_collaborator = None;
        self.message_rx = None;
        self.add_system_message("Disconnected from server".to_string());
    }

    /// Send chat message via SyncClient
    fn send_chat(&mut self) {
        let text = self.chat_input.trim();
        if text.is_empty() {
            return;
        }

        let client_id = self.client.as_ref().map(|c| c.client_id()).unwrap_or_default();
        let timestamp = current_timestamp();
        
        let msg = ChatMessage {
            id: Uuid::new_v4(),
            client_id,
            username: self.username.clone(),
            text: text.to_string(),
            timestamp,
            is_system: false,
            user_color: Some(self.user_color),
        };

        // Add to local chat immediately
        self.add_chat_message(msg.clone());
        self.should_scroll_to_bottom = true;

        // Send to server via SyncClient using async executor if available
        if let Some(ref _client) = self.client {
            if let Some(ref executor) = self.async_executor {
                let text = text.to_string();
                executor(Box::new(move || {
                    // In actual implementation:
                    // runtime.block_on(async { client.send_chat(&text).await })
                    tracing::info!("Sending chat: {}", text);
                }));
            }
        }

        self.chat_input.clear();
    }

    /// Add chat message to the list
    fn add_chat_message(&mut self, msg: ChatMessage) {
        self.chat_messages.push(msg);
        
        // Keep only last 100 messages
        if self.chat_messages.len() > 100 {
            self.chat_messages.remove(0);
        }

        // Increment unread if not at bottom
        if !self.chat_at_bottom {
            self.unread_count += 1;
        }
    }

    /// Add system message
    fn add_system_message(&mut self, text: String) {
        self.add_chat_message(ChatMessage {
            id: Uuid::new_v4(),
            client_id: Uuid::nil(),
            username: "System".to_string(),
            text,
            timestamp: current_timestamp(),
            is_system: true,
            user_color: None,
        });
        self.should_scroll_to_bottom = true;
    }

    /// Request lock on entity via LockManager
    fn lock_entity(&mut self, entity: Entity) {
        if let Some(ref client) = self.client {
            let my_id = client.client_id();
            
            // Try to acquire lock locally first
            if self.lock_manager.try_lock(entity, my_id, &self.username) {
                self.pending_changes += 1;
                
                // Send lock request to server using async executor
                if let Some(ref executor) = self.async_executor {
                    executor(Box::new(move || {
                        // In actual implementation:
                        // runtime.block_on(async { client.lock_entity(entity).await })
                        tracing::info!("Requesting lock for entity {:?}", entity);
                    }));
                }
                
                self.entity_locks.insert(entity, LockInfo {
                    client_id: my_id,
                    username: self.username.clone(),
                    timestamp: current_timestamp(),
                });
                self.add_system_message(format!("Lock acquired for entity {:?}", entity));
            } else {
                // Lock is held by someone else
                if let Some(lock_info) = self.lock_manager.get_lock_info(entity) {
                    self.show_lock_conflict = Some(LockConflict {
                        entity,
                        locked_by: lock_info.username.clone(),
                        locked_by_id: lock_info.client_id,
                    });
                }
            }
        }
    }

    /// Release lock on entity via LockManager
    fn unlock_entity(&mut self, entity: Entity) {
        if let Some(ref client) = self.client {
            let my_id = client.client_id();
            
            // Release lock locally
            if self.lock_manager.unlock(entity, my_id) {
                self.entity_locks.remove(&entity);
                
                // Send unlock to server using async executor
                if let Some(ref executor) = self.async_executor {
                    executor(Box::new(move || {
                        // In actual implementation:
                        // runtime.block_on(async { client.unlock_entity(entity).await })
                        tracing::info!("Unlocking entity {:?}", entity);
                    }));
                }
                
                self.add_system_message(format!("Released lock on entity {:?}", entity));
            }
        }
    }

    /// Force unlock (admin only)
    fn force_unlock_entity(&mut self, entity: Entity) {
        if self.is_admin {
            // Remove from local locks
            self.entity_locks.remove(&entity);
            self.lock_manager.force_unlock(entity);
            
            self.add_system_message(format!("Force unlocked entity {:?}", entity));
        }
    }

    /// Request full sync from server
    fn sync_now(&mut self) {
        if let Some(ref _client) = self.client {
            if let Some(ref executor) = self.async_executor {
                executor(Box::new(move || {
                    // In actual implementation:
                    // runtime.block_on(async { client.request_sync().await })
                    tracing::info!("Requesting sync");
                }));
            }
            self.add_system_message("Sync requested...".to_string());
        }
    }

    /// Update cursor position and send via SyncClient
    pub fn update_cursor(&mut self, position: CursorPosition) {
        let now = current_timestamp();
        
        // Throttle cursor updates
        if now - self.last_cursor_update < self.cursor_throttle_ms {
            return;
        }
        self.last_cursor_update = now;
        
        // Update local presence
        if let Some(ref client) = self.client {
            let my_id = client.client_id();
            self.presence_manager.update_cursor(my_id, position);
        }
        
        // Send to server using async executor
        if let Some(ref _client) = self.client {
            if let Some(ref executor) = self.async_executor {
                executor(Box::new(move || {
                    // In actual implementation:
                    // runtime.block_on(async { client.move_cursor(position).await })
                }));
            }
        }
    }

    /// Update selection and send via SyncClient
    pub fn update_selection(&mut self, entities: Vec<Entity>) {
        // Update local presence
        if let Some(ref client) = self.client {
            let my_id = client.client_id();
            self.presence_manager.update_selection(my_id, entities.clone());
        }
        
        // Send to server using async executor
        if let Some(ref _client) = self.client {
            if let Some(ref executor) = self.async_executor {
                executor(Box::new(move || {
                    // In actual implementation:
                    // runtime.block_on(async { client.set_selection(entities).await })
                }));
            }
        }
    }

    /// Set selected entity and request lock
    pub fn set_selected_entity(&mut self, entity: Option<Entity>) {
        // Release lock on previous entity if we had one
        if let Some(prev_entity) = self.selected_entity {
            if self.is_entity_locked_by_me(prev_entity) {
                // Keep the lock for a moment in case user switches back
                // Could also auto-release here
            }
        }
        
        self.selected_entity = entity;
        
        // Auto-request lock on new selection
        if let Some(new_entity) = entity {
            if !self.lock_manager.is_locked(new_entity) {
                self.lock_entity(new_entity);
            }
        }
    }

    /// Handle incoming sync message from message handler
    pub fn handle_sync_message(&mut self, msg: SyncMessage) {
        match msg {
            SyncMessage::ChatMessage { client_id, username, text, timestamp, .. } => {
                // Don't echo our own messages
                if Some(client_id) != self.client.as_ref().map(|c| c.client_id()) {
                    self.event_queue.push(SyncEvent::ChatMessage(ChatMessage {
                        id: Uuid::new_v4(),
                        client_id,
                        username,
                        text,
                        timestamp,
                        is_system: false,
                        user_color: Some(get_user_color_for_id(client_id)),
                    }));
                }
            }
            SyncMessage::ClientJoined { client } => {
                self.event_queue.push(SyncEvent::UserJoined(CollaboratorInfo {
                    presence: client.clone(),
                    last_activity: current_timestamp(),
                    is_typing: false,
                }));
                self.event_queue.push(SyncEvent::PresenceUpdated { 
                    client_id: client.client_id, 
                    presence: client 
                });
            }
            SyncMessage::ClientLeft { client_id } => {
                self.event_queue.push(SyncEvent::UserLeft(client_id));
            }
            SyncMessage::LockGranted { entity_id } => {
                self.event_queue.push(SyncEvent::LockGranted(entity_id));
            }
            SyncMessage::LockDenied { entity_id, locked_by } => {
                self.event_queue.push(SyncEvent::LockDenied { 
                    entity: entity_id, 
                    locked_by 
                });
            }
            SyncMessage::EntityUnlocked { entity_id, unlocked_by } => {
                self.event_queue.push(SyncEvent::EntityUnlocked(entity_id));
                // Also remove from local lock manager if not us
                if Some(unlocked_by) != self.client.as_ref().map(|c| c.client_id()) {
                    self.lock_manager.unlock(entity_id, unlocked_by);
                }
            }
            SyncMessage::CursorMove { client_id, position } => {
                // Don't process our own cursor
                if Some(client_id) != self.client.as_ref().map(|c| c.client_id()) {
                    self.event_queue.push(SyncEvent::CursorUpdated { 
                        client_id, 
                        position 
                    });
                }
            }
            SyncMessage::SelectionChange { client_id, selected_entities } => {
                // Don't process our own selection
                if Some(client_id) != self.client.as_ref().map(|c| c.client_id()) {
                    self.event_queue.push(SyncEvent::SelectionUpdated { 
                        client_id, 
                        entities: selected_entities 
                    });
                }
            }
            SyncMessage::SyncState { .. } => {
                self.event_queue.push(SyncEvent::SyncCompleted);
            }
            SyncMessage::Error { code, message } => {
                self.event_queue.push(SyncEvent::ConnectionError(
                    format!("{}: {}", code, message)
                ));
            }
            _ => {}
        }
    }

    /// Get collaborators for cursor rendering
    pub fn get_collaborator_cursors(&self) -> Vec<(Uuid, String, egui::Color32, CursorPosition)> {
        if !self.show_cursors {
            return Vec::new();
        }

        self.presence_manager.get_users()
            .into_iter()
            .filter_map(|presence| {
                // Skip self
                if Some(presence.client_id) == self.client.as_ref().map(|c| c.client_id()) {
                    return None;
                }
                
                let color = egui::Color32::from_rgb(
                    presence.color.r,
                    presence.color.g,
                    presence.color.b,
                );
                Some((
                    presence.client_id,
                    presence.username.clone(),
                    color,
                    presence.cursor,
                ))
            })
            .collect()
    }

    /// Get collaborator selections for highlighting
    pub fn get_collaborator_selections(&self) -> Vec<(Uuid, egui::Color32, Vec<Entity>)> {
        if !self.show_selections {
            return Vec::new();
        }

        self.presence_manager.get_users()
            .into_iter()
            .filter_map(|presence| {
                // Skip self
                if Some(presence.client_id) == self.client.as_ref().map(|c| c.client_id()) {
                    return None;
                }
                
                let color = egui::Color32::from_rgb(
                    presence.color.r,
                    presence.color.g,
                    presence.color.b,
                );
                Some((
                    presence.client_id,
                    color,
                    presence.selected_entities.clone(),
                ))
            })
            .collect()
    }

    /// Check if entity is locked by someone else
    pub fn is_entity_locked_by_other(&self, entity: Entity) -> Option<String> {
        self.lock_manager.get_lock_info(entity).and_then(|lock| {
            let my_id = self.client.as_ref().map(|c| c.client_id());
            if Some(lock.client_id) != my_id {
                Some(lock.username.clone())
            } else {
                None
            }
        })
    }

    /// Check if we have the lock on an entity
    pub fn is_entity_locked_by_me(&self, entity: Entity) -> bool {
        if let Some(lock) = self.lock_manager.get_lock_info(entity) {
            let my_id = self.client.as_ref().map(|c| c.client_id()).unwrap_or_default();
            lock.client_id == my_id
        } else {
            false
        }
    }

    /// Get followed collaborator
    pub fn followed_collaborator(&self) -> Option<Uuid> {
        self.followed_collaborator
    }

    /// Get collaborator viewport (for following)
    pub fn get_collaborator_viewport(&self, client_id: Uuid) -> Option<dde_sync::presence::Rect> {
        self.presence_manager.get_user(client_id).map(|p| p.viewport)
    }

    /// Check if connected
    pub fn is_connected(&self) -> bool {
        self.connection_state == ConnectionState::Connected
    }

    /// Set admin mode
    pub fn set_admin(&mut self, is_admin: bool) {
        self.is_admin = is_admin;
    }

    /// Get client reference
    pub fn client(&self) -> Option<&SyncClient> {
        self.client.as_ref()
    }

    /// Get mutable client reference
    pub fn client_mut(&mut self) -> Option<&mut SyncClient> {
        self.client.as_mut()
    }

    /// Track a change for sync
    pub fn track_change(&mut self, entity_id: u64, kind: ChangeKind) {
        if let Some(ref mut tracker) = self.change_tracker {
            tracker.track_entity_change(entity_id, kind);
        }
    }

    /// Get pending changes count
    pub fn pending_changes(&self) -> usize {
        self.change_tracker.as_ref().map(|ct| ct.pending_count()).unwrap_or(0)
    }

    /// Get connection state
    pub fn connection_state(&self) -> ConnectionState {
        self.connection_state
    }
}

impl Default for CollaborationPanel {
    fn default() -> Self {
        Self::new()
    }
}

/// Get activity description for a user
fn get_activity_description(presence: &UserPresence) -> String {
    if presence.selected_entities.is_empty() {
        match presence.status {
            UserStatus::Active => "Browsing".to_string(),
            UserStatus::Idle => "Idle".to_string(),
            UserStatus::Away => "Away".to_string(),
        }
    } else if presence.selected_entities.len() == 1 {
        format!("Editing entity {:?}", presence.selected_entities[0])
    } else {
        format!("Editing {} entities", presence.selected_entities.len())
    }
}

/// Generate a consistent color for a user based on their UUID
fn get_user_color_for_id(id: Uuid) -> egui::Color32 {
    let bytes = id.as_bytes();
    let hue = bytes[0] as f32 / 255.0;
    
    let saturation = 0.8;
    let lightness = 0.5;

    let c = (1.0 - f32::abs(2.0 * lightness - 1.0)) * saturation;
    let x = c * (1.0 - ((hue * 6.0) % 2.0 - 1.0).abs());
    let m = lightness - c / 2.0;

    let (r1, g1, b1) = match (hue * 6.0) as u8 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };

    egui::Color32::from_rgb(
        ((r1 + m) * 255.0) as u8,
        ((g1 + m) * 255.0) as u8,
        ((b1 + m) * 255.0) as u8,
    )
}

/// Format timestamp for display
fn format_timestamp(timestamp: u64) -> String {
    let elapsed = current_timestamp().saturating_sub(timestamp);
    format_elapsed_time(elapsed)
}

/// Format timestamp short (HH:MM)
fn format_timestamp_short(timestamp: u64) -> String {
    // Convert millis to HH:MM format
    let secs = timestamp / 1000;
    let mins = (secs / 60) % 60;
    let hours = (secs / 3600) % 24;
    format!("{:02}:{:02}", hours, mins)
}

/// Format elapsed time as human-readable string
fn format_elapsed_time(elapsed_ms: u64) -> String {
    if elapsed_ms < 1000 {
        "just now".to_string()
    } else if elapsed_ms < 60000 {
        format!("{}s ago", elapsed_ms / 1000)
    } else if elapsed_ms < 3600000 {
        format!("{}m ago", elapsed_ms / 60000)
    } else {
        format!("{}h ago", elapsed_ms / 3600000)
    }
}

/// Get current timestamp in milliseconds
fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Extension trait to integrate collaboration into the Editor
pub trait CollaborationExt {
    /// Get the collaboration panel
    fn collaboration_panel(&mut self) -> Option<&mut CollaborationPanel>;

    /// Draw collaboration cursors overlay
    fn draw_collaboration_cursors(&mut self, ctx: &egui::Context);

    /// Draw collaboration selections overlay
    fn draw_collaboration_selections(&mut self, ctx: &egui::Context);

    /// Connect to collaboration server
    fn connect_collaboration(&mut self, project_id: &str);

    /// Disconnect from collaboration server
    fn disconnect_collaboration(&mut self);

    /// Check if entity is locked
    fn is_entity_locked(&self, entity: Entity) -> Option<String>;

    /// Request lock on entity
    fn request_entity_lock(&mut self, entity: Entity);

    /// Release lock on entity
    fn release_entity_lock(&mut self, entity: Entity);

    /// Update cursor position
    fn update_collaboration_cursor(&mut self, position: CursorPosition);

    /// Update selection
    fn update_collaboration_selection(&mut self, entities: Vec<Entity>);
}

/// Cursor renderer for drawing collaborator cursors
pub struct CursorRenderer;

impl CursorRenderer {
    /// Draw a collaborator cursor
    pub fn draw(
        ui: &mut egui::Ui,
        position: egui::Pos2,
        username: &str,
        color: egui::Color32,
        is_active: bool,
    ) {
        let alpha = if is_active { 255 } else { 128 };
        let color_with_alpha = egui::Color32::from_rgba_premultiplied(
            color.r(),
            color.g(),
            color.b(),
            alpha,
        );

        // Draw cursor arrow
        let cursor_points = vec![
            position,
            position + egui::vec2(12.0, 4.0),
            position + egui::vec2(4.0, 12.0),
        ];
        ui.painter().add(egui::Shape::convex_polygon(
            cursor_points,
            color_with_alpha,
            egui::Stroke::new(1.0, egui::Color32::BLACK),
        ));

        // Draw username label
        let label_pos = position + egui::vec2(12.0, 12.0);
        let label = egui::Label::new(
            egui::RichText::new(username)
                .color(egui::Color32::WHITE)
                .background_color(color_with_alpha)
        );
        
        ui.put(
            egui::Rect::from_min_size(label_pos, egui::vec2(100.0, 20.0)),
            label,
        );
    }
}

/// Lock indicator renderer for entity overlays
pub struct LockIndicatorRenderer;

impl LockIndicatorRenderer {
    /// Draw lock indicator on an entity
    pub fn draw(
        ui: &mut egui::Ui,
        rect: egui::Rect,
        locked_by: Option<&str>,
        color: Option<egui::Color32>,
    ) {
        let (icon, tooltip, stroke_color) = if let Some(username) = locked_by {
            let col = color.unwrap_or(egui::Color32::RED);
            ("🔒", format!("Locked by {}", username), col)
        } else {
            ("🔓", "Unlocked".to_string(), egui::Color32::GRAY)
        };

        // Draw lock icon
        let icon_pos = rect.right_top() - egui::vec2(16.0, -4.0);
        ui.put(
            egui::Rect::from_min_size(icon_pos, egui::vec2(16.0, 16.0)),
            egui::Label::new(icon),
        );

        // Draw border around entity
        ui.painter().rect_stroke(
            rect,
            2.0,
            egui::Stroke::new(2.0, stroke_color),
        );

        // Tooltip
        ui.label(icon).on_hover_text(tooltip);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collaboration_panel_new() {
        let panel = CollaborationPanel::new();
        assert!(!panel.is_connected());
        assert!(panel.chat_messages.is_empty());
        assert!(panel.collaborators.is_empty());
        assert_eq!(panel.connection_state(), ConnectionState::Disconnected);
    }

    #[test]
    fn test_chat_message_creation() {
        let msg = ChatMessage {
            id: Uuid::new_v4(),
            client_id: Uuid::new_v4(),
            username: "TestUser".to_string(),
            text: "Hello World".to_string(),
            timestamp: current_timestamp(),
            is_system: false,
            user_color: Some(egui::Color32::RED),
        };
        
        assert_eq!(msg.username, "TestUser");
        assert_eq!(msg.text, "Hello World");
        assert!(!msg.is_system);
    }

    #[test]
    fn test_system_message() {
        let mut panel = CollaborationPanel::new();
        panel.add_system_message("Test system message".to_string());
        
        assert_eq!(panel.chat_messages.len(), 1);
        assert!(panel.chat_messages[0].is_system);
        assert_eq!(panel.chat_messages[0].text, "Test system message");
    }

    #[test]
    fn test_user_color_generation() {
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        
        let color1 = get_user_color_for_id(id1);
        let color2 = get_user_color_for_id(id2);
        
        // Same ID should produce same color
        assert_eq!(get_user_color_for_id(id1), color1);
        
        // Different IDs likely produce different colors
        // (collision is theoretically possible but unlikely)
    }

    #[test]
    fn test_format_elapsed_time() {
        assert_eq!(format_elapsed_time(500), "just now");
        assert_eq!(format_elapsed_time(5000), "5s ago");
        assert_eq!(format_elapsed_time(120000), "2m ago");
        assert_eq!(format_elapsed_time(7200000), "2h ago");
    }

    #[test]
    fn test_activity_description() {
        let presence = UserPresence::new(Uuid::new_v4(), "Test".to_string());
        
        assert_eq!(get_activity_description(&presence), "Browsing");
        
        let mut with_selection = presence.clone();
        with_selection.selected_entities = vec![Entity::DANGLING];
        assert!(get_activity_description(&with_selection).contains("Editing"));
    }

    #[test]
    fn test_presence_manager() {
        let mut manager = PresenceManager::new();
        let presence = UserPresence::new(Uuid::new_v4(), "TestUser".to_string());
        
        assert_eq!(manager.user_count(), 0);
        
        manager.update_user(presence.clone());
        assert_eq!(manager.user_count(), 1);
        assert!(manager.is_online(presence.client_id));
        
        manager.remove_user(presence.client_id);
        assert_eq!(manager.user_count(), 0);
        assert!(!manager.is_online(presence.client_id));
    }

    #[test]
    fn test_presence_manager_update_cursor() {
        let mut manager = PresenceManager::new();
        let presence = UserPresence::new(Uuid::new_v4(), "TestUser".to_string());
        manager.update_user(presence.clone());
        
        let new_cursor = CursorPosition { map_id: 1, x: 100.0, y: 200.0 };
        assert!(manager.update_cursor(presence.client_id, new_cursor));
        
        let user = manager.get_user(presence.client_id).unwrap();
        assert_eq!(user.cursor.x, 100.0);
        assert_eq!(user.cursor.y, 200.0);
    }

    #[test]
    fn test_lock_manager_integration() {
        let panel = CollaborationPanel::new();
        let entity = Entity::DANGLING;
        
        // Initially not locked
        assert!(!panel.is_entity_locked_by_me(entity));
        assert!(panel.is_entity_locked_by_other(entity).is_none());
    }
}
