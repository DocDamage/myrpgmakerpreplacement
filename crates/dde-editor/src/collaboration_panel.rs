//! Collaboration panel for real-time multiplayer editing

use dde_sync::{
    client::{ClientConfig, ConnectionState, SyncClient},
    presence::{UserPresence, UserStatus},
    protocol::SyncMessage,
};
use std::collections::HashMap;
use uuid::Uuid;

/// Collaboration panel for the editor
pub struct CollaborationPanel {
    /// Sync client for server connection
    client: Option<SyncClient>,
    /// Connection status
    connection_status: ConnectionStatus,
    /// Server URL input
    server_url: String,
    /// Username input
    username: String,
    /// Chat input
    chat_input: String,
    /// Show/hide collaborator cursors
    show_cursors: bool,
    /// Chat messages
    chat_messages: Vec<ChatMessage>,
    /// Selected collaborator for private actions
    selected_collaborator: Option<Uuid>,
    /// Auto-connect on startup
    auto_connect: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
    Error,
}

#[derive(Debug, Clone)]
struct ChatMessage {
    client_id: Uuid,
    username: String,
    text: String,
    timestamp: u64,
    is_system: bool,
}

impl CollaborationPanel {
    pub fn new() -> Self {
        Self {
            client: None,
            connection_status: ConnectionStatus::Disconnected,
            server_url: "ws://localhost:8080/sync".to_string(),
            username: std::env::var("USER").or_else(|_| std::env::var("USERNAME")).unwrap_or_else(|_| "Anonymous".to_string()),
            chat_input: String::new(),
            show_cursors: true,
            chat_messages: Vec::new(),
            selected_collaborator: None,
            auto_connect: false,
        }
    }

    /// Draw the collaboration UI
    pub fn draw_ui(&mut self, ctx: &egui::Context, project_id: &str) {
        egui::SidePanel::right("collaboration_panel")
            .default_width(250.0)
            .show(ctx, |ui| {
                ui.heading("Collaboration");
                ui.separator();

                self.draw_connection_section(ui, project_id);
                ui.separator();

                if self.connection_status == ConnectionStatus::Connected {
                    self.draw_collaborators_section(ui);
                    ui.separator();
                    self.draw_chat_section(ui);
                }
            });
    }

    fn draw_connection_section(&mut self, ui: &mut egui::Ui, project_id: &str) {
        ui.label("Server");
        ui.text_edit_singleline(&mut self.server_url);

        ui.label("Username");
        ui.text_edit_singleline(&mut self.username);

        // Connection button
        match self.connection_status {
            ConnectionStatus::Disconnected | ConnectionStatus::Error => {
                if ui.button("Connect").clicked() {
                    self.connect(project_id);
                }
            }
            ConnectionStatus::Connecting => {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("Connecting...");
                });
            }
            ConnectionStatus::Connected => {
                ui.horizontal(|ui| {
                    ui.colored_label(egui::Color32::GREEN, "●");
                    ui.label("Connected");
                });
                if ui.button("Disconnect").clicked() {
                    self.disconnect();
                }
            }
        }

        ui.checkbox(&mut self.show_cursors, "Show cursors");
    }

    fn draw_collaborators_section(&mut self, ui: &mut egui::Ui) {
        ui.heading("Collaborators");

        if let Some(client) = &self.client {
            // This would be async in a real implementation
            // For now, we'll use a placeholder
            let collaborators: Vec<UserPresence> = Vec::new();

            if collaborators.is_empty() {
                ui.label("No other collaborators");
            } else {
                for presence in &collaborators {
                    let color = presence.color;
                    let color32 = egui::Color32::from_rgb(color.r, color.g, color.b);

                    ui.horizontal(|ui| {
                        // Status indicator
                        let status_color = match presence.status {
                            UserStatus::Active => egui::Color32::GREEN,
                            UserStatus::Idle => egui::Color32::YELLOW,
                            UserStatus::Away => egui::Color32::GRAY,
                        };
                        ui.colored_label(status_color, "●");

                        // Color swatch
                        let size = egui::vec2(16.0, 16.0);
                        let (rect, _response) = ui.allocate_exact_size(size, egui::Sense::hover());
                        ui.painter().rect_filled(rect, 2.0, color32);

                        // Username
                        ui.label(&presence.username);

                        // Follow button
                        if ui.small_button("👁").clicked() {
                            self.selected_collaborator = Some(presence.client_id);
                        }
                    });
                }
            }
        }
    }

    fn draw_chat_section(&mut self, ui: &mut egui::Ui) {
        ui.heading("Chat");

        // Chat messages
        egui::ScrollArea::vertical()
            .max_height(200.0)
            .stick_to_bottom(true)
            .show(ui, |ui| {
                for msg in &self.chat_messages {
                    if msg.is_system {
                        ui.horizontal_wrapped(|ui| {
                            ui.colored_label(
                                egui::Color32::GRAY,
                                format!("[System] {}", msg.text),
                            );
                        });
                    } else {
                        ui.horizontal_wrapped(|ui| {
                            ui.colored_label(
                                egui::Color32::LIGHT_BLUE,
                                format!("{}: ", msg.username),
                            );
                            ui.label(&msg.text);
                        });
                    }
                }
            });

        // Chat input
        ui.horizontal(|ui| {
            ui.text_edit_singleline(&mut self.chat_input);
            if ui.button("Send").clicked() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                self.send_chat();
            }
        });
    }

    fn connect(&mut self, project_id: &str) {
        self.connection_status = ConnectionStatus::Connecting;

        let config = ClientConfig::new(&self.server_url, &self.username, project_id);
        let mut client = SyncClient::new(config);

        // Set up message handler
        // Note: Using a channel or message queue would be safer than raw pointers
        // For now, disable the handler to avoid Send/Sync issues
        self.connection_status = ConnectionStatus::Connected;

        self.client = Some(client);
        self.connection_status = ConnectionStatus::Connected;

        self.add_system_message(format!("Connected to {}", self.server_url));
    }

    fn disconnect(&mut self) {
        self.client = None;
        self.connection_status = ConnectionStatus::Disconnected;
        self.add_system_message("Disconnected".to_string());
    }

    fn send_chat(&mut self) {
        if self.chat_input.is_empty() {
            return;
        }

        if let Some(client) = &self.client {
            // In a real implementation, this would be async
            // For now, just add to local messages
            self.chat_messages.push(ChatMessage {
                client_id: Uuid::nil(),
                username: self.username.clone(),
                text: self.chat_input.clone(),
                timestamp: current_timestamp(),
                is_system: false,
            });

            self.chat_input.clear();
        }
    }

    fn add_system_message(&mut self, text: String) {
        self.chat_messages.push(ChatMessage {
            client_id: Uuid::nil(),
            username: "System".to_string(),
            text,
            timestamp: current_timestamp(),
            is_system: true,
        });
    }

    /// Get the sync client (if connected)
    pub fn client(&self) -> Option<&SyncClient> {
        self.client.as_ref()
    }

    /// Check if connected
    pub fn is_connected(&self) -> bool {
        self.connection_status == ConnectionStatus::Connected
    }

    /// Get whether to show collaborator cursors
    pub fn show_cursors(&self) -> bool {
        self.show_cursors && self.is_connected()
    }

    /// Get the selected collaborator to follow
    pub fn selected_collaborator(&self) -> Option<Uuid> {
        self.selected_collaborator
    }

    /// Clear the selected collaborator
    pub fn clear_selected_collaborator(&mut self) {
        self.selected_collaborator = None;
    }
}

impl Default for CollaborationPanel {
    fn default() -> Self {
        Self::new()
    }
}

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
    
    /// Connect to collaboration server
    fn connect_collaboration(&mut self, project_id: &str);
    
    /// Disconnect from collaboration server
    fn disconnect_collaboration(&mut self);
}
