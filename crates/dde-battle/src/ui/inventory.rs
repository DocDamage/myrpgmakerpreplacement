//! Inventory UI System
//!
//! An egui-based inventory screen with:
//! - Category filtering
//! - Item sorting
//! - Equipment management
//! - Drag and drop support
//! - Tooltips with stat comparisons

use crate::items::{Item, ItemDatabase, ItemEffect, ItemResult, ItemType, StatType};
use dde_core::components::{Equipment, Inventory, ItemSlot, Stats};
use dde_core::{Entity, World};

/// Category filter for inventory items
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ItemCategory {
    #[default]
    All,
    Consumable,
    Equipment,
    KeyItem,
}

impl ItemCategory {
    /// Get display name for the category
    pub fn name(self) -> &'static str {
        match self {
            ItemCategory::All => "All",
            ItemCategory::Consumable => "Consumable",
            ItemCategory::Equipment => "Equipment",
            ItemCategory::KeyItem => "Key",
        }
    }

    /// Get icon unicode character for the category
    pub fn icon(self) -> &'static str {
        match self {
            ItemCategory::All => "☰",
            ItemCategory::Consumable => "🧪",
            ItemCategory::Equipment => "⚔",
            ItemCategory::KeyItem => "🔑",
        }
    }
}

impl std::fmt::Display for ItemCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Item rarity for visual styling
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ItemRarity {
    #[default]
    Common,
    Uncommon,
    Rare,
    Epic,
    Legendary,
}

impl ItemRarity {
    /// Get color for this rarity
    pub fn color(self) -> egui::Color32 {
        match self {
            ItemRarity::Common => egui::Color32::WHITE,
            ItemRarity::Uncommon => egui::Color32::from_rgb(0, 255, 0), // Green
            ItemRarity::Rare => egui::Color32::from_rgb(0, 150, 255),   // Blue
            ItemRarity::Epic => egui::Color32::from_rgb(180, 0, 255),   // Purple
            ItemRarity::Legendary => egui::Color32::from_rgb(255, 165, 0), // Orange
        }
    }
}

/// Equipment slot types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EquipmentSlot {
    #[default]
    Weapon,
    Armor,
    Accessory,
}

impl EquipmentSlot {
    /// Get display name
    pub fn name(self) -> &'static str {
        match self {
            EquipmentSlot::Weapon => "Weapon",
            EquipmentSlot::Armor => "Armor",
            EquipmentSlot::Accessory => "Accessory",
        }
    }

    /// Get icon for the slot
    pub fn icon(self) -> &'static str {
        match self {
            EquipmentSlot::Weapon => "⚔",
            EquipmentSlot::Armor => "🛡",
            EquipmentSlot::Accessory => "💍",
        }
    }
}

/// Sort methods for inventory items
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortMethod {
    #[default]
    ByName,
    ByType,
    ByValue,
    RecentlyAcquired,
}

impl SortMethod {
    /// Get display name
    pub fn name(self) -> &'static str {
        match self {
            SortMethod::ByName => "Name",
            SortMethod::ByType => "Type",
            SortMethod::ByValue => "Value",
            SortMethod::RecentlyAcquired => "Recent",
        }
    }
}

/// Equipment comparison for tooltips
#[derive(Debug, Clone)]
pub struct EquipmentComparison {
    pub current: Option<Item>,
    pub equipped: Option<Item>,
    pub stat_diffs: Vec<(StatType, i32)>,
}

/// Item tooltip data
#[derive(Debug, Clone)]
pub struct ItemTooltip {
    pub item: Item,
    pub comparison: Option<EquipmentComparison>,
}

/// Response from clicking an equipment slot
#[derive(Debug, Clone)]
pub struct EquipmentSlotResponse {
    pub clicked: bool,
    pub right_clicked: bool,
    pub hovered: bool,
    pub dropped_item: Option<u32>,
}

/// Inventory action result
#[derive(Debug, Clone)]
pub enum InventoryActionResult {
    Success(String),
    Failure(String),
}

/// Inventory actions that can be performed
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InventoryAction {
    UseItem {
        item_id: u32,
        target: Option<Entity>,
    },
    EquipItem {
        item_id: u32,
        slot: EquipmentSlot,
    },
    UnequipItem {
        slot: EquipmentSlot,
    },
    DropItem {
        item_id: u32,
        quantity: u32,
    },
    SortItems(SortMethod),
}

/// Errors that can occur during inventory operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ItemError {
    ItemNotFound,
    InvalidTarget,
    CannotEquip,
    InventoryFull,
    NotEnoughQuantity,
    SlotOccupied,
}

impl std::fmt::Display for ItemError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ItemError::ItemNotFound => write!(f, "Item not found"),
            ItemError::InvalidTarget => write!(f, "Invalid target"),
            ItemError::CannotEquip => write!(f, "Cannot equip this item"),
            ItemError::InventoryFull => write!(f, "Inventory is full"),
            ItemError::NotEnoughQuantity => write!(f, "Not enough items"),
            ItemError::SlotOccupied => write!(f, "Slot is occupied"),
        }
    }
}

impl std::error::Error for ItemError {}

/// Inventory UI state
#[derive(Debug)]
pub struct InventoryUiState {
    pub visible: bool,
    pub selected_category: ItemCategory,
    pub selected_slot: Option<usize>,
    pub equip_mode: bool,
    pub tooltip_delay: f32,
    pub sort_method: SortMethod,
    pub search_query: String,
    pub dragged_item: Option<u32>,
    pub last_action_result: Option<InventoryActionResult>,
    pub action_message_timer: f32,
    pub hovered_slot: Option<EquipmentSlot>,
}

impl Default for InventoryUiState {
    fn default() -> Self {
        Self {
            visible: false,
            selected_category: ItemCategory::All,
            selected_slot: None,
            equip_mode: false,
            tooltip_delay: 0.5,
            sort_method: SortMethod::ByName,
            search_query: String::new(),
            dragged_item: None,
            last_action_result: None,
            action_message_timer: 0.0,
            hovered_slot: None,
        }
    }
}

impl InventoryUiState {
    /// Create new inventory UI state
    pub fn new() -> Self {
        Self::default()
    }

    /// Toggle visibility
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    /// Show the inventory
    pub fn show(&mut self) {
        self.visible = true;
    }

    /// Hide the inventory
    pub fn hide(&mut self) {
        self.visible = false;
    }

    /// Draw the full inventory window
    pub fn draw(
        &mut self,
        ctx: &egui::Context,
        inventory: &Inventory,
        equipment: &Equipment,
        item_db: &ItemDatabase,
    ) {
        if !self.visible {
            return;
        }

        // Update action message timer
        if self.action_message_timer > 0.0 {
            self.action_message_timer -= ctx.input(|i| i.stable_dt);
            if self.action_message_timer <= 0.0 {
                self.last_action_result = None;
            }
        }

        // Handle keyboard shortcuts
        self.handle_keyboard_input(ctx);

        // Main inventory window
        egui::Window::new("Inventory")
            .id(egui::Id::new("inventory_window"))
            .resizable(true)
            .default_size([800.0, 500.0])
            .frame(Self::rpg_window_frame())
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    // Left panel: Category tabs + Item grid
                    ui.vertical(|ui| {
                        self.draw_category_tabs(ui);
                        ui.separator();

                        // Search bar
                        ui.horizontal(|ui| {
                            ui.label("🔍");
                            ui.text_edit_singleline(&mut self.search_query);
                            if ui.button("×").clicked() {
                                self.search_query.clear();
                            }
                        });

                        ui.separator();

                        // Get filtered items
                        let filtered_items = self.get_filtered_items(inventory, item_db);
                        self.draw_item_grid(ui, &filtered_items, item_db);
                    });

                    ui.separator();

                    // Right panel: Equipment + Details
                    ui.vertical(|ui| {
                        self.draw_equipment_slots(ui, equipment, item_db);
                        ui.separator();

                        if let Some(slot_idx) = self.selected_slot {
                            if let Some(item_slot) = inventory.items.get(slot_idx) {
                                if let Some(item) = item_db.get(item_slot.item_id) {
                                    self.draw_item_details(ui, item, item_slot.quantity);
                                }
                            }
                        } else {
                            ui.label("Select an item to view details");
                        }
                    });
                });

                // Action message overlay
                if let Some(ref result) = self.last_action_result {
                    let (text, color) = match result {
                        InventoryActionResult::Success(msg) => (msg.as_str(), egui::Color32::GREEN),
                        InventoryActionResult::Failure(msg) => (msg.as_str(), egui::Color32::RED),
                    };

                    let rect = ui.max_rect();
                    let _response = egui::Area::new(egui::Id::new("action_message"))
                        .fixed_pos(egui::pos2(rect.center().x - 100.0, rect.max.y - 40.0))
                        .show(ctx, |ui| {
                            ui.colored_label(color, text);
                        });
                }
            });

        // Drag overlay
        if let Some(item_id) = self.dragged_item {
            if let Some(item) = item_db.get(item_id) {
                Self::draw_drag_overlay(ctx, item);
            }
        }
    }

    /// Draw category tabs
    fn draw_category_tabs(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            for category in [
                ItemCategory::All,
                ItemCategory::Consumable,
                ItemCategory::Equipment,
                ItemCategory::KeyItem,
            ] {
                let is_selected = self.selected_category == category;
                let button = if is_selected {
                    egui::Button::new(
                        egui::RichText::new(format!("{} {}", category.icon(), category.name()))
                            .color(egui::Color32::YELLOW)
                            .strong(),
                    )
                } else {
                    egui::Button::new(format!("{} {}", category.icon(), category.name()))
                };

                if ui.add(button).clicked() {
                    self.selected_category = category;
                    self.selected_slot = None;
                }
            }
        });
    }

    /// Draw the item grid
    fn draw_item_grid(
        &mut self,
        ui: &mut egui::Ui,
        items: &[(usize, ItemSlot, &Item)],
        _item_db: &ItemDatabase,
    ) {
        let available_width = ui.available_width();
        let item_size = 64.0;
        let spacing = 4.0;
        let _items_per_row = ((available_width + spacing) / (item_size + spacing)) as usize;
        let _items_per_row = _items_per_row.max(1);

        egui::ScrollArea::vertical()
            .max_height(300.0)
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(spacing, spacing);

                    for (inventory_idx, item_slot, item) in items {
                        let is_selected = self.selected_slot == Some(*inventory_idx);
                        let rarity = Self::determine_rarity(item);

                        let response = self.draw_item_cell(
                            ui,
                            item,
                            item_slot.quantity,
                            rarity,
                            is_selected,
                            *inventory_idx,
                        );

                        if response.clicked() {
                            self.selected_slot = Some(*inventory_idx);
                        }

                        if response.double_clicked() {
                            // Quick use on double click
                            self.selected_slot = Some(*inventory_idx);
                        }

                        // Drag start
                        if response.drag_started() {
                            self.dragged_item = Some(item.id);
                        }

                        // Drag end
                        if response.drag_stopped() {
                            self.dragged_item = None;
                        }

                        // Tooltip on hover
                        if response.hovered() && self.dragged_item.is_none() {
                            response.on_hover_ui(|ui| {
                                let tooltip = ItemTooltip {
                                    item: (*item).clone(),
                                    comparison: None,
                                };
                                self.draw_tooltip(ui, &tooltip);
                            });
                        }
                    }
                });
            });
    }

    /// Draw a single item cell
    fn draw_item_cell(
        &mut self,
        ui: &mut egui::Ui,
        item: &Item,
        quantity: u32,
        rarity: ItemRarity,
        selected: bool,
        _index: usize,
    ) -> egui::Response {
        let size = egui::vec2(64.0, 64.0);
        let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click_and_drag());

        if ui.is_rect_visible(rect) {
            let painter = ui.painter();

            // Background
            let bg_color = if selected {
                egui::Color32::from_rgb(60, 60, 80)
            } else if response.hovered() {
                egui::Color32::from_rgb(50, 50, 60)
            } else {
                egui::Color32::from_rgb(40, 40, 45)
            };

            painter.rect_filled(rect, 4.0, bg_color);

            // Border
            let border_color = if selected {
                egui::Color32::YELLOW
            } else {
                rarity.color().gamma_multiply(0.5)
            };
            painter.rect_stroke(rect, 4.0, egui::Stroke::new(2.0, border_color));

            // Item icon (placeholder - would be texture in real implementation)
            let icon_text = Self::item_type_icon(&item.item_type);
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                icon_text,
                egui::FontId::proportional(24.0),
                rarity.color(),
            );

            // Quantity badge
            if quantity > 1 {
                let qty_text = format!("{}", quantity);
                let qty_size = ui
                    .fonts(|f| {
                        f.layout_no_wrap(
                            qty_text.clone(),
                            egui::FontId::proportional(12.0),
                            egui::Color32::WHITE,
                        )
                    })
                    .size();
                let qty_rect = egui::Rect::from_min_size(
                    egui::pos2(rect.max.x - qty_size.x - 4.0, rect.max.y - 16.0),
                    qty_size + egui::vec2(4.0, 2.0),
                );
                painter.rect_filled(qty_rect, 2.0, egui::Color32::from_rgb(0, 0, 0));
                painter.text(
                    qty_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    qty_text,
                    egui::FontId::proportional(12.0),
                    egui::Color32::WHITE,
                );
            }
        }

        response
    }

    /// Draw item details panel
    fn draw_item_details(&mut self, ui: &mut egui::Ui, item: &Item, quantity: u32) {
        let rarity = Self::determine_rarity(item);

        ui.vertical(|ui| {
            // Item name with rarity color
            ui.colored_label(
                rarity.color().gamma_multiply(1.5),
                egui::RichText::new(&item.name).size(18.0).strong(),
            );

            ui.add_space(4.0);

            // Item type and quantity
            ui.horizontal(|ui| {
                ui.label(format!("Type: {:?}", item.item_type));
                ui.label(format!("Qty: {}", quantity));
            });

            ui.separator();

            // Description
            ui.label(&item.description);

            ui.separator();

            // Stats
            ui.label(format!("Power: {}", item.power));
            ui.label(format!("Cooldown: {} turns", item.cooldown));
            ui.label(format!("Target: {:?}", item.target_type));

            ui.add_space(8.0);

            // Action buttons
            ui.horizontal(|ui| {
                if ui.button("Use (U)").clicked() {
                    // Would emit UseItem action
                }

                if Self::is_equippable(item) && ui.button("Equip (E)").clicked() {
                    self.equip_mode = true;
                }

                if ui.button("Drop (D)").clicked() {
                    // Would emit DropItem action
                }
            });
        });
    }

    /// Draw equipment slots
    fn draw_equipment_slots(
        &mut self,
        ui: &mut egui::Ui,
        equipment: &Equipment,
        item_db: &ItemDatabase,
    ) {
        ui.label("Equipment");
        ui.add_space(4.0);

        let slots = [
            (EquipmentSlot::Weapon, equipment.weapon),
            (EquipmentSlot::Armor, equipment.armor),
            (EquipmentSlot::Accessory, equipment.accessory),
        ];

        for (slot, equipped_id) in slots {
            let equipped_item = equipped_id.and_then(|id| item_db.get(id));
            let is_hovered = self.hovered_slot == Some(slot);

            let response = draw_equipment_slot(ui, slot, equipped_item, is_hovered);

            if response.clicked {
                // Handle slot click (unequip if occupied)
            }

            if response.right_clicked {
                // Context menu
            }

            if response.hovered {
                self.hovered_slot = Some(slot);
            }

            // Handle drop
            if let Some(_dropped_id) = response.dropped_item {
                // Would emit EquipItem action
                self.dragged_item = None;
            }

            ui.add_space(4.0);
        }
    }

    /// Draw tooltip with optional comparison
    fn draw_tooltip(&self, ui: &mut egui::Ui, tooltip: &ItemTooltip) {
        let item = &tooltip.item;
        let rarity = Self::determine_rarity(item);

        ui.vertical(|ui| {
            // Name
            ui.colored_label(
                rarity.color(),
                egui::RichText::new(&item.name).size(16.0).strong(),
            );

            ui.separator();

            // Description
            ui.label(egui::RichText::new(&item.description).italics());

            ui.separator();

            // Stats
            ui.label(format!("Power: {}", item.power));
            ui.label(format!("Cooldown: {} turns", item.cooldown));

            // Comparison if available
            if let Some(ref comparison) = tooltip.comparison {
                ui.separator();
                ui.label("Comparison:");

                for (stat, diff) in &comparison.stat_diffs {
                    let color = if *diff > 0 {
                        egui::Color32::GREEN
                    } else if *diff < 0 {
                        egui::Color32::RED
                    } else {
                        egui::Color32::GRAY
                    };

                    let sign = if *diff > 0 { "+" } else { "" };
                    ui.colored_label(color, format!("{:?}: {}{}", stat, sign, diff));
                }
            }
        });
    }

    /// Handle an inventory action
    pub fn handle_action(
        &mut self,
        action: InventoryAction,
        world: &mut World,
        item_db: &ItemDatabase,
    ) -> Result<InventoryActionResult, ItemError> {
        match action {
            InventoryAction::UseItem { item_id, target } => {
                self.handle_use_item(item_id, target, world, item_db)
            }
            InventoryAction::EquipItem { item_id, slot } => {
                self.handle_equip_item(item_id, slot, world)
            }
            InventoryAction::UnequipItem { slot } => self.handle_unequip_item(slot, world),
            InventoryAction::DropItem { item_id, quantity } => {
                self.handle_drop_item(item_id, quantity, world)
            }
            InventoryAction::SortItems(method) => {
                self.sort_method = method;
                Ok(InventoryActionResult::Success("Items sorted".to_string()))
            }
        }
    }

    /// Handle keyboard input
    fn handle_keyboard_input(&mut self, ctx: &egui::Context) {
        ctx.input(|i| {
            // Toggle inventory with 'I'
            if i.key_pressed(egui::Key::I) {
                self.toggle();
            }

            if !self.visible {
                return;
            }

            // Category switching with Tab
            if i.key_pressed(egui::Key::Tab) {
                self.selected_category = match self.selected_category {
                    ItemCategory::All => ItemCategory::Consumable,
                    ItemCategory::Consumable => ItemCategory::Equipment,
                    ItemCategory::Equipment => ItemCategory::KeyItem,
                    ItemCategory::KeyItem => ItemCategory::All,
                };
                self.selected_slot = None;
            }

            // Only process item shortcuts if something is selected
            if let Some(_slot_idx) = self.selected_slot {
                // Use with 'U'
                if i.key_pressed(egui::Key::U) {
                    // Would emit UseItem action
                }

                // Equip with 'E'
                if i.key_pressed(egui::Key::E) {
                    self.equip_mode = true;
                }

                // Drop with 'D'
                if i.key_pressed(egui::Key::D) {
                    // Would emit DropItem action
                }
            }
        });
    }

    /// Get filtered and sorted items
    pub fn get_filtered_items<'a>(
        &self,
        inventory: &'a Inventory,
        item_db: &'a ItemDatabase,
    ) -> Vec<(usize, ItemSlot, &'a Item)> {
        let mut result: Vec<_> = inventory
            .items
            .iter()
            .enumerate()
            .filter_map(|(idx, slot)| {
                item_db.get(slot.item_id).and_then(|item| {
                    // Apply category filter
                    let matches_category = match self.selected_category {
                        ItemCategory::All => true,
                        ItemCategory::Consumable => matches!(
                            item.item_type,
                            ItemType::Heal
                                | ItemType::Mana
                                | ItemType::Elixir
                                | ItemType::Phoenix
                                | ItemType::Remedy
                        ),
                        ItemCategory::Equipment => matches!(item.item_type, ItemType::Buff),
                        ItemCategory::KeyItem => matches!(item.item_type, ItemType::Offensive),
                    };

                    // Apply search filter
                    let matches_search = self.search_query.is_empty()
                        || item
                            .name
                            .to_lowercase()
                            .contains(&self.search_query.to_lowercase())
                        || item
                            .description
                            .to_lowercase()
                            .contains(&self.search_query.to_lowercase());

                    if matches_category && matches_search {
                        Some((idx, *slot, item))
                    } else {
                        None
                    }
                })
            })
            .collect();

        // Apply sorting
        match self.sort_method {
            SortMethod::ByName => {
                result.sort_by(|a, b| a.2.name.cmp(&b.2.name));
            }
            SortMethod::ByType => {
                result.sort_by(|a, b| {
                    format!("{:?}", a.2.item_type).cmp(&format!("{:?}", b.2.item_type))
                });
            }
            SortMethod::ByValue => {
                result.sort_by(|a, b| a.2.power.cmp(&b.2.power).reverse());
            }
            SortMethod::RecentlyAcquired => {
                // Keep original order (assumes items are added at end)
            }
        }

        result
    }

    /// Determine item rarity (placeholder logic - would be data-driven)
    fn determine_rarity(item: &Item) -> ItemRarity {
        match item.id {
            1..=3 => ItemRarity::Common,
            4..=6 => ItemRarity::Uncommon,
            7..=9 => ItemRarity::Rare,
            10..=12 => ItemRarity::Epic,
            _ => ItemRarity::Legendary,
        }
    }

    /// Get icon for item type
    fn item_type_icon(item_type: &ItemType) -> &'static str {
        match item_type {
            ItemType::Heal => "💚",
            ItemType::Mana => "💙",
            ItemType::Elixir => "✨",
            ItemType::Phoenix => "🔥",
            ItemType::Buff => "💪",
            ItemType::Offensive => "💣",
            ItemType::Remedy => "🩹",
        }
    }

    /// Check if item can be equipped
    fn is_equippable(item: &Item) -> bool {
        matches!(item.item_type, ItemType::Buff)
    }

    /// Draw drag overlay
    fn draw_drag_overlay(ctx: &egui::Context, item: &Item) {
        if let Some(pointer_pos) = ctx.pointer_interact_pos() {
            egui::Area::new(egui::Id::new("drag_overlay"))
                .fixed_pos(pointer_pos - egui::vec2(32.0, 32.0))
                .interactable(false)
                .show(ctx, |ui| {
                    let size = egui::vec2(64.0, 64.0);
                    let (rect, _response) = ui.allocate_exact_size(size, egui::Sense::hover());

                    let painter = ui.painter();
                    let rarity = Self::determine_rarity(item);

                    painter.rect_filled(rect, 4.0, rarity.color().gamma_multiply(0.7));
                    painter.rect_stroke(rect, 4.0, egui::Stroke::new(2.0, rarity.color()));

                    painter.text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        Self::item_type_icon(&item.item_type),
                        egui::FontId::proportional(24.0),
                        egui::Color32::WHITE,
                    );
                });
        }
    }

    /// RPG-style window frame
    fn rpg_window_frame() -> egui::Frame {
        egui::Frame::window(&egui::Style::default())
            .fill(egui::Color32::from_rgb(20, 20, 25))
            .stroke(egui::Stroke::new(2.0, egui::Color32::from_rgb(80, 80, 100)))
            .rounding(egui::Rounding::same(8.0))
            .inner_margin(egui::Margin::same(12.0))
    }

    // Action handlers
    fn handle_use_item(
        &mut self,
        item_id: u32,
        target: Option<Entity>,
        world: &mut World,
        item_db: &ItemDatabase,
    ) -> Result<InventoryActionResult, ItemError> {
        // Verify item exists
        let item = item_db.get(item_id).ok_or(ItemError::ItemNotFound)?;

        // Get target stats if specified
        let result = if let Some(target_entity) = target {
            if let Ok(mut query) = world.query_one_mut::<&mut Stats>(target_entity) {
                item_db.use_item(item_id, &Stats::default(), Some(&mut query))
            } else {
                return Err(ItemError::InvalidTarget);
            }
        } else {
            // Self-target
            // This is a simplified version - in real implementation would get the player entity
            ItemResult {
                success: true,
                hp_restored: item.power,
                mp_restored: 0,
                damage_dealt: 0,
                message: format!("Used {}", item.name),
                effects_applied: vec![ItemEffect::Heal(item.power)],
            }
        };

        if result.success {
            Ok(InventoryActionResult::Success(result.message))
        } else {
            Ok(InventoryActionResult::Failure(result.message))
        }
    }

    fn handle_equip_item(
        &mut self,
        _item_id: u32,
        slot: EquipmentSlot,
        _world: &mut World,
    ) -> Result<InventoryActionResult, ItemError> {
        // Simplified - in real implementation would modify Equipment component
        Ok(InventoryActionResult::Success(format!(
            "Equipped to {:?}",
            slot
        )))
    }

    fn handle_unequip_item(
        &mut self,
        slot: EquipmentSlot,
        _world: &mut World,
    ) -> Result<InventoryActionResult, ItemError> {
        Ok(InventoryActionResult::Success(format!(
            "Unequipped from {:?}",
            slot
        )))
    }

    fn handle_drop_item(
        &mut self,
        item_id: u32,
        quantity: u32,
        _world: &mut World,
    ) -> Result<InventoryActionResult, ItemError> {
        Ok(InventoryActionResult::Success(format!(
            "Dropped {}x item {}",
            quantity, item_id
        )))
    }
}

/// Draw an equipment slot
pub fn draw_equipment_slot(
    ui: &mut egui::Ui,
    slot: EquipmentSlot,
    equipped: Option<&Item>,
    selected: bool,
) -> EquipmentSlotResponse {
    let size = egui::vec2(64.0, 64.0);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click_and_drag());

    let clicked = response.clicked();
    let right_clicked = response.secondary_clicked();
    let hovered = response.hovered();
    let dropped_item = None;

    // Check for drop
    if response.drag_stopped() {
        // In a real implementation, we'd check what was dragged
        // For now, this is handled by the caller
    }

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();

        // Background
        let bg_color = if selected {
            egui::Color32::from_rgb(60, 60, 80)
        } else if hovered {
            egui::Color32::from_rgb(50, 50, 60)
        } else {
            egui::Color32::from_rgb(35, 35, 40)
        };

        painter.rect_filled(rect, 4.0, bg_color);

        // Border
        let border_color = if selected {
            egui::Color32::YELLOW
        } else {
            egui::Color32::from_rgb(80, 80, 100)
        };
        painter.rect_stroke(rect, 4.0, egui::Stroke::new(2.0, border_color));

        // Content
        if let Some(item) = equipped {
            // Draw equipped item
            let rarity = InventoryUiState::determine_rarity(item);
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                InventoryUiState::item_type_icon(&item.item_type),
                egui::FontId::proportional(24.0),
                rarity.color(),
            );
        } else {
            // Draw slot silhouette
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                slot.icon(),
                egui::FontId::proportional(32.0),
                egui::Color32::from_rgb(60, 60, 70),
            );
        }

        // Slot label
        painter.text(
            rect.center_bottom() + egui::vec2(0.0, -4.0),
            egui::Align2::CENTER_BOTTOM,
            slot.name(),
            egui::FontId::proportional(10.0),
            egui::Color32::GRAY,
        );
    }

    // Check for drops from other UI elements
    if response.hovered() {
        // This would integrate with a drag-and-drop system
        // For now, we assume the caller handles this
    }

    EquipmentSlotResponse {
        clicked,
        right_clicked,
        hovered,
        dropped_item,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::items::{Item, ItemTarget, ItemType};

    fn create_test_item(id: u32, name: &str, item_type: ItemType) -> Item {
        Item {
            id,
            name: name.to_string(),
            description: "Test item".to_string(),
            item_type,
            power: 10,
            target_type: ItemTarget::SingleAlly,
            cooldown: 0,
        }
    }

    #[test]
    fn test_inventory_ui_state_default() {
        let state = InventoryUiState::default();
        assert!(!state.visible);
        assert_eq!(state.selected_category, ItemCategory::All);
        assert_eq!(state.selected_slot, None);
        assert!(!state.equip_mode);
    }

    #[test]
    fn test_inventory_ui_toggle() {
        let mut state = InventoryUiState::new();
        assert!(!state.visible);

        state.toggle();
        assert!(state.visible);

        state.toggle();
        assert!(!state.visible);
    }

    #[test]
    fn test_category_filtering() {
        let mut inventory = Inventory::new();
        inventory.add_item(1, 5, 99); // Heal item
        inventory.add_item(6, 3, 99); // Offensive item
        inventory.add_item(3, 2, 99); // Mana item

        let item_db = ItemDatabase::new();
        let state = InventoryUiState::default();

        // All items
        let all_items = state.get_filtered_items(&inventory, &item_db);
        assert!(!all_items.is_empty());

        // Consumable items (Heal, Mana, Elixir, Phoenix, Remedy)
        let mut consumable_state = InventoryUiState::default();
        consumable_state.selected_category = ItemCategory::Consumable;
        let consumable_items = consumable_state.get_filtered_items(&inventory, &item_db);
        assert!(!consumable_items.is_empty());
    }

    #[test]
    fn test_sort_methods() {
        let mut inventory = Inventory::new();
        inventory.add_item(1, 1, 99); // Potion
        inventory.add_item(2, 1, 99); // Hi-Potion
        inventory.add_item(3, 1, 99); // Ether

        let item_db = ItemDatabase::new();

        // Sort by name
        let mut state = InventoryUiState::default();
        state.sort_method = SortMethod::ByName;
        let items = state.get_filtered_items(&inventory, &item_db);
        assert!(!items.is_empty());

        // Sort by type
        state.sort_method = SortMethod::ByType;
        let items = state.get_filtered_items(&inventory, &item_db);
        assert!(!items.is_empty());

        // Sort by value
        state.sort_method = SortMethod::ByValue;
        let items = state.get_filtered_items(&inventory, &item_db);
        assert!(!items.is_empty());
    }

    #[test]
    fn test_item_rarity_colors() {
        assert_eq!(ItemRarity::Common.color(), egui::Color32::WHITE);
        assert_eq!(
            ItemRarity::Uncommon.color(),
            egui::Color32::from_rgb(0, 255, 0)
        );
        assert_eq!(
            ItemRarity::Rare.color(),
            egui::Color32::from_rgb(0, 150, 255)
        );
    }

    #[test]
    fn test_equipment_slot_types() {
        assert_eq!(EquipmentSlot::Weapon.name(), "Weapon");
        assert_eq!(EquipmentSlot::Armor.name(), "Armor");
        assert_eq!(EquipmentSlot::Accessory.name(), "Accessory");
    }

    #[test]
    fn test_item_category_display() {
        assert_eq!(ItemCategory::All.name(), "All");
        assert_eq!(ItemCategory::Consumable.name(), "Consumable");
        assert_eq!(ItemCategory::Equipment.name(), "Equipment");
        assert_eq!(ItemCategory::KeyItem.name(), "Key");
    }

    #[test]
    fn test_search_filtering() {
        let mut inventory = Inventory::new();
        inventory.add_item(1, 5, 99); // Potion
        inventory.add_item(3, 2, 99); // Ether

        let item_db = ItemDatabase::new();
        let mut state = InventoryUiState::default();

        // No search - all items
        let items = state.get_filtered_items(&inventory, &item_db);
        assert_eq!(items.len(), 2);

        // Search for "pot"
        state.search_query = "pot".to_string();
        let items = state.get_filtered_items(&inventory, &item_db);
        assert_eq!(items.len(), 1);
        assert!(items[0].2.name.to_lowercase().contains("pot"));
    }

    #[test]
    fn test_item_error_display() {
        assert_eq!(ItemError::ItemNotFound.to_string(), "Item not found");
        assert_eq!(ItemError::InvalidTarget.to_string(), "Invalid target");
        assert_eq!(ItemError::CannotEquip.to_string(), "Cannot equip this item");
    }

    #[test]
    fn test_inventory_action_result() {
        let success = InventoryActionResult::Success("Item used".to_string());
        let failure = InventoryActionResult::Failure("Not enough MP".to_string());

        match success {
            InventoryActionResult::Success(msg) => assert_eq!(msg, "Item used"),
            _ => panic!("Expected Success"),
        }

        match failure {
            InventoryActionResult::Failure(msg) => assert_eq!(msg, "Not enough MP"),
            _ => panic!("Expected Failure"),
        }
    }
}
