//! Entity Lock Manager for collaborative editing
//!
//! Provides a comprehensive interface for managing entity locks including:
//! - Lock list view with filtering and sorting
//! - Force unlock capabilities (admin)
//! - Lock transfer requests
//! - Statistics dashboard
//! - Visual indicators for lock status

use dde_core::Entity;
use dde_sync::lock::{LockInfo, LockManager};
use std::collections::HashMap;
use uuid::Uuid;

/// Lock Manager Panel for the editor
pub struct LockManagerPanel {
    /// Whether the panel is visible
    visible: bool,
    /// Lock manager reference (shared with collaboration panel)
    lock_manager: LockManager,
    /// Current user's client ID
    client_id: Option<Uuid>,
    /// Current username
    username: String,
    /// Whether user has admin privileges
    is_admin: bool,
    /// Filter text for lock list
    filter_text: String,
    /// Sort column for lock list
    sort_column: SortColumn,
    /// Sort direction
    sort_ascending: bool,
    /// Selected lock entry
    selected_lock: Option<Entity>,
    /// Lock transfer request state
    transfer_request: Option<LockTransferRequest>,
    /// Stale lock threshold in milliseconds
    stale_threshold_ms: u64,
    /// Auto-refresh interval
    auto_refresh: bool,
    /// Last refresh timestamp
    last_refresh: u64,
    /// User color cache (client_id -> color)
    user_colors: HashMap<Uuid, egui::Color32>,
    /// Statistics cache
    stats: LockStatistics,
    /// Show stale locks only
    show_stale_only: bool,
    /// Show my locks only
    show_my_locks_only: bool,
    /// Column widths for the table
    column_widths: [f32; 5],
}

/// Sortable columns for the lock list
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortColumn {
    EntityId,
    LockedBy,
    Timestamp,
    LockAge,
    Status,
}

/// Lock transfer request state
#[derive(Debug, Clone)]
pub struct LockTransferRequest {
    pub entity: Entity,
    pub from_user: String,
    pub from_client_id: Uuid,
    pub message: String,
    pub status: TransferStatus,
}

/// Transfer request status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransferStatus {
    Pending,
    Accepted,
    Denied,
    Timeout,
}

/// Lock statistics for the dashboard
#[derive(Debug, Clone, Default)]
pub struct LockStatistics {
    pub total_locks: usize,
    pub my_locks: usize,
    pub stale_locks: usize,
    pub locks_per_user: Vec<(String, usize)>,
    pub oldest_lock_age_ms: u64,
    pub average_lock_age_ms: u64,
}

/// Display row for a lock entry
#[derive(Debug, Clone)]
pub struct LockRow {
    pub entity: Entity,
    pub lock_info: LockInfo,
    pub age_ms: u64,
    pub is_mine: bool,
    pub is_stale: bool,
}

impl LockManagerPanel {
    /// Create a new lock manager panel
    pub fn new() -> Self {
        Self {
            visible: false,
            lock_manager: LockManager::new(),
            client_id: None,
            username: "Anonymous".to_string(),
            is_admin: false,
            filter_text: String::new(),
            sort_column: SortColumn::LockAge,
            sort_ascending: false,
            selected_lock: None,
            transfer_request: None,
            stale_threshold_ms: 5 * 60 * 1000, // 5 minutes
            auto_refresh: true,
            last_refresh: 0,
            user_colors: HashMap::new(),
            stats: LockStatistics::default(),
            show_stale_only: false,
            show_my_locks_only: false,
            column_widths: [120.0, 150.0, 80.0, 100.0, 80.0],
        }
    }

    /// Set the lock manager (shared with collaboration)
    pub fn set_lock_manager(&mut self, manager: LockManager) {
        self.lock_manager = manager;
        self.refresh_stats();
    }

    /// Set the current user's client ID
    pub fn set_client_id(&mut self, client_id: Uuid) {
        self.client_id = Some(client_id);
        self.refresh_stats();
    }

    /// Set the current username
    pub fn set_username(&mut self, username: String) {
        self.username = username;
    }

    /// Set admin mode
    pub fn set_admin(&mut self, is_admin: bool) {
        self.is_admin = is_admin;
    }

    /// Set user color for a client
    pub fn set_user_color(&mut self, client_id: Uuid, color: egui::Color32) {
        self.user_colors.insert(client_id, color);
    }

    /// Show the panel
    pub fn show(&mut self) {
        self.visible = true;
        self.refresh_stats();
    }

    /// Hide the panel
    pub fn hide(&mut self) {
        self.visible = false;
    }

    /// Toggle panel visibility
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
        if self.visible {
            self.refresh_stats();
        }
    }

    /// Check if panel is visible
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Update the panel (call each frame)
    pub fn update(&mut self, _dt: f32) {
        if !self.visible || !self.auto_refresh {
            return;
        }

        let now = current_timestamp();
        // Refresh every second
        if now - self.last_refresh > 1000 {
            self.refresh_stats();
            self.last_refresh = now;
        }
    }

    /// Draw the lock manager UI
    pub fn draw(&mut self, ctx: &egui::Context) {
        if !self.visible {
            return;
        }

        egui::Window::new("🔒 Lock Manager")
            .default_size([800.0, 600.0])
            .min_size([600.0, 400.0])
            .show(ctx, |ui| {
                self.draw_content(ui);
            });

        // Draw transfer request dialog if active
        self.draw_transfer_dialog(ctx);
    }

    /// Draw the main content area
    fn draw_content(&mut self, ui: &mut egui::Ui) {
        // Statistics dashboard
        self.draw_statistics_dashboard(ui);
        ui.separator();

        // Toolbar with filters and actions
        self.draw_toolbar(ui);
        ui.separator();

        // Lock list table
        self.draw_lock_table(ui);
        ui.separator();

        // Selected lock details
        if let Some(entity) = self.selected_lock {
            self.draw_lock_details(ui, entity);
        }

        // Transfer request section
        if let Some(ref request) = self.transfer_request {
            self.draw_transfer_request_status(ui, request.clone());
        }
    }

    /// Draw the statistics dashboard
    fn draw_statistics_dashboard(&mut self, ui: &mut egui::Ui) {
        ui.heading("📊 Lock Statistics");

        egui::Grid::new("lock_stats_grid")
            .num_columns(4)
            .spacing([40.0, 4.0])
            .show(ui, |ui| {
                // Row 1
                ui.label(format!("Total Locks: {}", self.stats.total_locks));
                ui.label(format!("My Locks: {}", self.stats.my_locks));
                let stale_color = if self.stats.stale_locks > 0 {
                    egui::Color32::YELLOW
                } else {
                    ui.visuals().text_color()
                };
                ui.colored_label(stale_color, format!("⚠ Stale Locks: {}", self.stats.stale_locks));
                ui.label(format!("Unique Users: {}", self.stats.locks_per_user.len()));
                ui.end_row();

                // Row 2 - Lock ages
                if self.stats.total_locks > 0 {
                    ui.label(format!(
                        "Oldest: {}",
                        format_duration(self.stats.oldest_lock_age_ms)
                    ));
                    ui.label(format!(
                        "Average: {}",
                        format_duration(self.stats.average_lock_age_ms)
                    ));
                }
                ui.end_row();
            });

        // Locks per user bar chart
        if !self.stats.locks_per_user.is_empty() {
            ui.collapsing("👥 Locks per User", |ui| {
                for (username, count) in &self.stats.locks_per_user {
                    ui.horizontal(|ui| {
                        ui.label(format!("{}: ", username));
                        // Simple bar visualization
                        let bar_width = (*count as f32 * 20.0).min(200.0);
                        let (rect, _) = ui.allocate_exact_size(
                            egui::vec2(bar_width, 16.0),
                            egui::Sense::hover(),
                        );
                        ui.painter().rect_filled(
                            rect,
                            2.0,
                            egui::Color32::from_rgb(100, 150, 255),
                        );
                        ui.label(count.to_string());
                    });
                }
            });
        }
    }

    /// Draw the toolbar with filters and actions
    fn draw_toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            // Filter input
            ui.label("🔍 Filter:");
            ui.add(
                egui::TextEdit::singleline(&mut self.filter_text)
                    .hint_text("Filter by entity or user...")
                    .desired_width(200.0),
            );

            ui.separator();

            // View filters
            ui.checkbox(&mut self.show_stale_only, "Stale only");
            ui.checkbox(&mut self.show_my_locks_only, "My locks only");

            ui.separator();

            // Auto-refresh toggle
            ui.checkbox(&mut self.auto_refresh, "Auto-refresh");

            // Manual refresh button
            if ui.button("🔄 Refresh").clicked() {
                self.refresh_stats();
            }

            ui.separator();

            // Admin actions
            if self.is_admin {
                if ui.button("⚡ Force Unlock All Stale").clicked() {
                    self.force_unlock_all_stale();
                }
                if ui.button("🧹 Clear All Locks").clicked() {
                    self.clear_all_locks();
                }
            }
        });

        // Stale threshold configuration (admin only)
        if self.is_admin {
            ui.horizontal(|ui| {
                ui.label("Stale threshold:");
                let mut minutes = (self.stale_threshold_ms / 60000) as f32;
                ui.add(egui::Slider::new(&mut minutes, 1.0..=60.0).text("minutes"));
                self.stale_threshold_ms = (minutes * 60000.0) as u64;
            });
        }
    }

    /// Draw the lock table
    fn draw_lock_table(&mut self, ui: &mut egui::Ui) {
        ui.heading("🔒 Active Locks");

        let rows = self.get_filtered_and_sorted_locks();

        if rows.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label("No locks to display");
            });
            return;
        }

        // Table header
        ui.horizontal(|ui| {
            let headers = ["Entity", "Locked By", "Time", "Age", "Actions"];
            for (i, header) in headers.iter().enumerate() {
                let sort_indicator = if self.sort_column as usize == i {
                    if self.sort_ascending { " ▲" } else { " ▼" }
                } else {
                    ""
                };
                let button_text = format!("{}{}", header, sort_indicator);
                
                if ui.button(&button_text).clicked() {
                    let clicked_column = match i {
                        0 => SortColumn::EntityId,
                        1 => SortColumn::LockedBy,
                        2 => SortColumn::Timestamp,
                        3 => SortColumn::LockAge,
                        4 => SortColumn::Status,
                        _ => SortColumn::EntityId,
                    };
                    
                    if self.sort_column == clicked_column {
                        self.sort_ascending = !self.sort_ascending;
                    } else {
                        self.sort_column = clicked_column;
                        self.sort_ascending = true;
                    }
                }
                ui.allocate_exact_size(egui::vec2(self.column_widths[i], 0.0), egui::Sense::hover());
            }
        });

        ui.separator();

        // Table rows
        egui::ScrollArea::vertical()
            .max_height(300.0)
            .show(ui, |ui| {
                for row in &rows {
                    self.draw_lock_row(ui, row);
                }
            });
    }

    /// Draw a single lock row
    fn draw_lock_row(&mut self, ui: &mut egui::Ui, row: &LockRow) {
        let is_selected = self.selected_lock == Some(row.entity);
        
        // Background for selected row
        if is_selected {
            let rect = ui.available_rect_before_wrap();
            ui.painter().rect_filled(
                rect,
                0.0,
                ui.visuals().selection.bg_fill,
            );
        }

        ui.horizontal(|ui| {
            // Entity ID column
            let entity_text = format!("{:?}", row.entity);
            let response = ui.selectable_label(
                is_selected,
                &entity_text,
            );
            if response.clicked() {
                self.selected_lock = Some(row.entity);
            }
            ui.allocate_exact_size(egui::vec2(self.column_widths[0] - response.rect.width(), 0.0), egui::Sense::hover());

            // Locked By column with color
            let user_color = self.get_user_color(row.lock_info.client_id);
            ui.colored_label(user_color, &row.lock_info.username);
            ui.allocate_exact_size(egui::vec2(self.column_widths[1] - 100.0, 0.0), egui::Sense::hover());

            // Timestamp column
            ui.label(format_timestamp_short(row.lock_info.timestamp));
            ui.allocate_exact_size(egui::vec2(self.column_widths[2], 0.0), egui::Sense::hover());

            // Age column with color coding
            let age_color = if row.is_stale {
                egui::Color32::RED
            } else if row.age_ms > self.stale_threshold_ms / 2 {
                egui::Color32::YELLOW
            } else {
                egui::Color32::GREEN
            };
            ui.colored_label(age_color, format_duration(row.age_ms));
            ui.allocate_exact_size(egui::vec2(self.column_widths[3], 0.0), egui::Sense::hover());

            // Actions column
            self.draw_lock_actions(ui, row);
        });

        ui.separator();
    }

    /// Draw action buttons for a lock row
    fn draw_lock_actions(&mut self, ui: &mut egui::Ui, row: &LockRow) {
        ui.horizontal(|ui| {
            if row.is_mine {
                // Unlock button for my locks
                if ui.small_button("🔓 Unlock").clicked() {
                    self.unlock_entity(row.entity);
                }
            } else {
                // Request transfer button
                if ui.small_button("📨 Request").clicked() {
                    self.initiate_transfer_request(row.entity, &row.lock_info);
                }

                // Force unlock for admins
                if self.is_admin && ui.small_button("⚡ Force").clicked() {
                    self.force_unlock_entity(row.entity);
                }
            }

            // Visual indicator button
            let indicator = if row.is_mine {
                ("🟢", "Locked by you")
            } else if row.is_stale {
                ("🔴", "Stale lock - consider force unlock")
            } else {
                ("🟡", "Locked by another user")
            };
            ui.label(indicator.0).on_hover_text(indicator.1);
        });
    }

    /// Draw lock details for the selected lock
    fn draw_lock_details(&mut self, ui: &mut egui::Ui, entity: Entity) {
        ui.separator();
        ui.heading("📋 Lock Details");

        if let Some(lock_info) = self.lock_manager.get_lock_info(entity) {
            let age = self.lock_manager.get_lock_age(entity).unwrap_or(0);
            let is_mine = self.client_id == Some(lock_info.client_id);

            egui::Grid::new("lock_details_grid")
                .num_columns(2)
                .spacing([20.0, 4.0])
                .show(ui, |ui| {
                    ui.label("Entity:");
                    ui.monospace(format!("{:?}", entity));
                    ui.end_row();

                    ui.label("Locked by:");
                    let user_color = self.get_user_color(lock_info.client_id);
                    ui.colored_label(user_color, &lock_info.username);
                    ui.end_row();

                    ui.label("Client ID:");
                    ui.monospace(lock_info.client_id.to_string());
                    ui.end_row();

                    ui.label("Lock timestamp:");
                    ui.label(format_timestamp(lock_info.timestamp));
                    ui.end_row();

                    ui.label("Lock age:");
                    let age_color = if age > self.stale_threshold_ms {
                        egui::Color32::RED
                    } else if age > self.stale_threshold_ms / 2 {
                        egui::Color32::YELLOW
                    } else {
                        egui::Color32::GREEN
                    };
                    ui.colored_label(age_color, format_duration(age));
                    ui.end_row();

                    ui.label("Status:");
                    if is_mine {
                        ui.colored_label(egui::Color32::GREEN, "🔒 Locked by you");
                    } else {
                        ui.colored_label(egui::Color32::YELLOW, "🔒 Locked by another user");
                    }
                    ui.end_row();
                });

            // Action buttons
            ui.horizontal(|ui| {
                if is_mine {
                    if ui.button("🔓 Unlock").clicked() {
                        self.unlock_entity(entity);
                        self.selected_lock = None;
                    }
                } else {
                    if ui.button("📨 Request Transfer").clicked() {
                        self.initiate_transfer_request(entity, &lock_info);
                    }

                    if self.is_admin {
                        ui.separator();
                        if ui.button("⚡ Force Unlock").clicked() {
                            self.force_unlock_entity(entity);
                            self.selected_lock = None;
                        }
                    }
                }
            });
        } else {
            ui.label("Lock no longer exists");
            self.selected_lock = None;
        }
    }

    /// Draw transfer request dialog
    fn draw_transfer_dialog(&mut self, ctx: &egui::Context) {
        if self.transfer_request.is_none() {
            return;
        }

        let request = self.transfer_request.clone().unwrap();
        let mut should_close = false;

        egui::Window::new("📨 Lock Transfer Request")
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label(format!(
                    "Requesting lock transfer for entity {:?}",
                    request.entity
                ));
                ui.label(format!("Current owner: {}", request.from_user));

                ui.separator();

                ui.label("Message (optional):");
                ui.text_edit_multiline(&mut self.transfer_request.as_mut().unwrap().message);

                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button("Send Request").clicked() {
                        self.send_transfer_request();
                        should_close = true;
                    }
                    if ui.button("Cancel").clicked() {
                        should_close = true;
                    }
                });
            });

        if should_close && self.transfer_request.as_ref().map(|r| r.status) != Some(TransferStatus::Pending) {
            self.transfer_request = None;
        }
    }

    /// Draw transfer request status
    fn draw_transfer_request_status(&mut self, ui: &mut egui::Ui, request: LockTransferRequest) {
        ui.separator();
        ui.heading("📨 Transfer Request");

        let status_text = match request.status {
            TransferStatus::Pending => ("⏳ Pending...", egui::Color32::YELLOW),
            TransferStatus::Accepted => ("✅ Accepted!", egui::Color32::GREEN),
            TransferStatus::Denied => ("❌ Denied", egui::Color32::RED),
            TransferStatus::Timeout => ("⏱️ Timeout", egui::Color32::GRAY),
        };

        ui.horizontal(|ui| {
            ui.label(format!("Entity: {:?}", request.entity));
            ui.colored_label(status_text.1, status_text.0);
        });

        if !request.message.is_empty() {
            ui.label(format!("Message: {}", request.message));
        }

        if request.status != TransferStatus::Pending {
            if ui.button("Clear").clicked() {
                self.transfer_request = None;
            }
        }
    }

    /// Get filtered and sorted lock rows
    fn get_filtered_and_sorted_locks(&self) -> Vec<LockRow> {
        let now = current_timestamp();
        let my_id = self.client_id.unwrap_or_default();

        let mut rows: Vec<LockRow> = self
            .lock_manager
            .get_locked_entities()
            .into_iter()
            .filter_map(|(entity, lock_info)| {
                let age = now.saturating_sub(lock_info.timestamp);
                let is_mine = lock_info.client_id == my_id;
                let is_stale = age > self.stale_threshold_ms;

                // Apply filters
                if self.show_stale_only && !is_stale {
                    return None;
                }
                if self.show_my_locks_only && !is_mine {
                    return None;
                }
                if !self.filter_text.is_empty() {
                    let filter_lower = self.filter_text.to_lowercase();
                    let entity_match = format!("{:?}", entity).to_lowercase().contains(&filter_lower);
                    let user_match = lock_info.username.to_lowercase().contains(&filter_lower);
                    if !entity_match && !user_match {
                        return None;
                    }
                }

                Some(LockRow {
                    entity,
                    lock_info,
                    age_ms: age,
                    is_mine,
                    is_stale,
                })
            })
            .collect();

        // Sort rows
        rows.sort_by(|a, b| {
            let comparison = match self.sort_column {
                SortColumn::EntityId => format!("{:?}", a.entity).cmp(&format!("{:?}", b.entity)),
                SortColumn::LockedBy => a.lock_info.username.cmp(&b.lock_info.username),
                SortColumn::Timestamp => a.lock_info.timestamp.cmp(&b.lock_info.timestamp),
                SortColumn::LockAge => a.age_ms.cmp(&b.age_ms),
                SortColumn::Status => {
                    let status_a = (a.is_mine, a.is_stale);
                    let status_b = (b.is_mine, b.is_stale);
                    status_a.cmp(&status_b)
                }
            };

            if self.sort_ascending {
                comparison
            } else {
                comparison.reverse()
            }
        });

        rows
    }

    /// Get color for a user
    fn get_user_color(&self, client_id: Uuid) -> egui::Color32 {
        self.user_colors
            .get(&client_id)
            .copied()
            .unwrap_or_else(|| generate_user_color(client_id))
    }

    /// Refresh statistics
    fn refresh_stats(&mut self) {
        let locks = self.lock_manager.get_locked_entities();
        let now = current_timestamp();
        let my_id = self.client_id.unwrap_or_default();

        self.stats.total_locks = locks.len();
        self.stats.my_locks = locks
            .iter()
            .filter(|(_, info)| info.client_id == my_id)
            .count();
        self.stats.stale_locks = locks
            .iter()
            .filter(|(_, info)| now - info.timestamp > self.stale_threshold_ms)
            .count();

        // Calculate lock ages
        if !locks.is_empty() {
            let ages: Vec<u64> = locks
                .iter()
                .map(|(_, info)| now.saturating_sub(info.timestamp))
                .collect();
            self.stats.oldest_lock_age_ms = ages.iter().copied().max().unwrap_or(0);
            self.stats.average_lock_age_ms = ages.iter().sum::<u64>() / ages.len() as u64;
        } else {
            self.stats.oldest_lock_age_ms = 0;
            self.stats.average_lock_age_ms = 0;
        }

        // Locks per user
        let mut user_counts: HashMap<String, usize> = HashMap::new();
        for (_, info) in &locks {
            *user_counts.entry(info.username.clone()).or_insert(0) += 1;
        }
        self.stats.locks_per_user = user_counts.into_iter().collect();
        self.stats.locks_per_user.sort_by(|a, b| b.1.cmp(&a.1));
    }

    /// Unlock an entity
    fn unlock_entity(&mut self, entity: Entity) {
        if let Some(client_id) = self.client_id {
            if self.lock_manager.unlock(entity, client_id) {
                self.refresh_stats();
            }
        }
    }

    /// Force unlock an entity (admin only)
    fn force_unlock_entity(&mut self, entity: Entity) {
        if self.is_admin {
            self.lock_manager.force_unlock(entity);
            self.refresh_stats();
        }
    }

    /// Force unlock all stale locks (admin only)
    fn force_unlock_all_stale(&mut self) {
        if !self.is_admin {
            return;
        }

        let stale = self
            .lock_manager
            .cleanup_stale_locks(self.stale_threshold_ms);
        
        for entity in stale {
            tracing::info!("Force unlocked stale lock on entity {:?}", entity);
        }
        
        self.refresh_stats();
    }

    /// Clear all locks (admin only, use with caution!)
    fn clear_all_locks(&mut self) {
        if !self.is_admin {
            return;
        }

        self.lock_manager.clear_all();
        self.refresh_stats();
        self.selected_lock = None;
    }

    /// Initiate a lock transfer request
    fn initiate_transfer_request(&mut self, entity: Entity, lock_info: &LockInfo) {
        self.transfer_request = Some(LockTransferRequest {
            entity,
            from_user: lock_info.username.clone(),
            from_client_id: lock_info.client_id,
            message: String::new(),
            status: TransferStatus::Pending,
        });
    }

    /// Send transfer request to server
    fn send_transfer_request(&mut self) {
        // In a real implementation, this would send a message to the server
        // For now, we just log it
        if let Some(ref request) = self.transfer_request {
            tracing::info!(
                "Sending lock transfer request for entity {:?} to user {}",
                request.entity,
                request.from_user
            );
        }
    }

    /// Handle transfer response
    pub fn handle_transfer_response(&mut self, accepted: bool) {
        if let Some(ref mut request) = self.transfer_request {
            request.status = if accepted {
                TransferStatus::Accepted
            } else {
                TransferStatus::Denied
            };
        }
    }

    /// Get the lock manager reference
    pub fn lock_manager(&self) -> &LockManager {
        &self.lock_manager
    }

    /// Get mutable lock manager reference
    pub fn lock_manager_mut(&mut self) -> &mut LockManager {
        &mut self.lock_manager
    }

    /// Get statistics
    pub fn statistics(&self) -> &LockStatistics {
        &self.stats
    }

    /// Check if an entity is locked
    pub fn is_locked(&self, entity: Entity) -> bool {
        self.lock_manager.is_locked(entity)
    }

    /// Get lock info for an entity
    pub fn get_lock_info(&self, entity: Entity) -> Option<LockInfo> {
        self.lock_manager.get_lock_info(entity)
    }

    /// Get lock age for an entity
    pub fn get_lock_age(&self, entity: Entity) -> Option<u64> {
        self.lock_manager.get_lock_age(entity)
    }

    /// Check if a lock is stale
    pub fn is_lock_stale(&self, entity: Entity) -> bool {
        self.lock_manager
            .get_lock_age(entity)
            .map(|age| age > self.stale_threshold_ms)
            .unwrap_or(false)
    }

    /// Get the visual indicator color for an entity's lock status
    pub fn get_lock_indicator_color(&self, entity: Entity) -> Option<egui::Color32> {
        self.lock_manager.get_lock_info(entity).map(|info| {
            if Some(info.client_id) == self.client_id {
                egui::Color32::GREEN
            } else {
                self.get_user_color(info.client_id)
            }
        })
    }

    /// Draw a visual lock indicator for an entity (for use in other editors)
    pub fn draw_lock_indicator(&self, ui: &mut egui::Ui, entity: Entity, rect: egui::Rect) {
        if let Some(lock_info) = self.lock_manager.get_lock_info(entity) {
            let color = self.get_user_color(lock_info.client_id);
            let is_mine = Some(lock_info.client_id) == self.client_id;

            // Draw colored border
            ui.painter().rect_stroke(
                rect,
                2.0,
                egui::Stroke::new(2.0, color),
            );

            // Draw lock icon
            let icon = if is_mine { "🔒" } else { "🔐" };
            let icon_pos = rect.right_top() - egui::vec2(16.0, -4.0);
            ui.painter().text(
                icon_pos,
                egui::Align2::LEFT_TOP,
                icon,
                egui::FontId::proportional(12.0),
                color,
            );

            // Tooltip
            let tooltip = if is_mine {
                format!("Locked by you")
            } else {
                format!("Locked by {}", lock_info.username)
            };
            ui.label(icon).on_hover_text(tooltip);
        }
    }
}

impl Default for LockManagerPanel {
    fn default() -> Self {
        Self::new()
    }
}

/// Generate a consistent color for a user based on their UUID
fn generate_user_color(id: Uuid) -> egui::Color32 {
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

/// Format a duration in milliseconds to a human-readable string
fn format_duration(ms: u64) -> String {
    if ms < 1000 {
        format!("{}ms", ms)
    } else if ms < 60000 {
        format!("{}s", ms / 1000)
    } else if ms < 3600000 {
        format!("{}m {}s", ms / 60000, (ms % 60000) / 1000)
    } else {
        format!("{}h {}m", ms / 3600000, (ms % 3600000) / 60000)
    }
}

/// Format a timestamp for display
fn format_timestamp(timestamp: u64) -> String {
    let elapsed = current_timestamp().saturating_sub(timestamp);
    format_duration(elapsed)
}

/// Format timestamp short (HH:MM)
fn format_timestamp_short(timestamp: u64) -> String {
    let secs = timestamp / 1000;
    let mins = (secs / 60) % 60;
    let hours = (secs / 3600) % 24;
    format!("{:02}:{:02}", hours, mins)
}

/// Get current timestamp in milliseconds
fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Extension trait to integrate lock manager into the Editor
pub trait LockManagerExt {
    /// Get the lock manager panel
    fn lock_manager_panel(&mut self) -> Option<&mut LockManagerPanel>;

    /// Draw lock manager window
    fn draw_lock_manager(&mut self, ctx: &egui::Context);

    /// Toggle lock manager visibility
    fn toggle_lock_manager(&mut self);

    /// Check if lock manager is visible
    fn is_lock_manager_visible(&self) -> bool;

    /// Draw a lock indicator on an entity
    fn draw_entity_lock_indicator(&self, ui: &mut egui::Ui, entity: Entity, rect: egui::Rect);

    /// Check if an entity is locked
    fn is_entity_locked(&self, entity: Entity) -> bool;

    /// Get lock info for an entity
    fn get_entity_lock_info(&self, entity: Entity) -> Option<LockInfo>;

    /// Get the color for a lock (for visual indicators)
    fn get_lock_color(&self, entity: Entity) -> Option<egui::Color32>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lock_manager_panel_new() {
        let panel = LockManagerPanel::new();
        assert!(!panel.is_visible());
        assert_eq!(panel.statistics().total_locks, 0);
    }

    #[test]
    fn test_sort_column_variants() {
        let columns = [
            SortColumn::EntityId,
            SortColumn::LockedBy,
            SortColumn::Timestamp,
            SortColumn::LockAge,
            SortColumn::Status,
        ];
        
        for col in &columns {
            let _ = format!("{:?}", col);
        }
    }

    #[test]
    fn test_transfer_status_variants() {
        assert_ne!(TransferStatus::Pending, TransferStatus::Accepted);
        assert_ne!(TransferStatus::Denied, TransferStatus::Timeout);
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(500), "500ms");
        assert_eq!(format_duration(5000), "5s");
        assert_eq!(format_duration(65000), "1m 5s");
        assert_eq!(format_duration(3661000), "1h 1m");
    }

    #[test]
    fn test_generate_user_color() {
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        
        let color1 = generate_user_color(id1);
        let color2 = generate_user_color(id2);
        
        // Same ID should produce same color
        assert_eq!(generate_user_color(id1), color1);
        
        // Different IDs likely produce different colors
        // (collision is theoretically possible but unlikely)
    }

    #[test]
    fn test_lock_statistics_default() {
        let stats = LockStatistics::default();
        assert_eq!(stats.total_locks, 0);
        assert_eq!(stats.my_locks, 0);
        assert_eq!(stats.stale_locks, 0);
        assert!(stats.locks_per_user.is_empty());
    }

    #[test]
    fn test_lock_row_creation() {
        let row = LockRow {
            entity: Entity::DANGLING,
            lock_info: LockInfo {
                client_id: Uuid::new_v4(),
                username: "Test".to_string(),
                timestamp: current_timestamp(),
            },
            age_ms: 1000,
            is_mine: true,
            is_stale: false,
        };
        
        assert!(row.is_mine);
        assert!(!row.is_stale);
    }

    #[test]
    fn test_transfer_request_creation() {
        let request = LockTransferRequest {
            entity: Entity::DANGLING,
            from_user: "Alice".to_string(),
            from_client_id: Uuid::new_v4(),
            message: "Please transfer".to_string(),
            status: TransferStatus::Pending,
        };
        
        assert_eq!(request.from_user, "Alice");
        assert_eq!(request.status, TransferStatus::Pending);
    }
}
