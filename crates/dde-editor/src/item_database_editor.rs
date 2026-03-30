//! Item Database Editor
//!
//! Editor UI for creating and managing items in the game.
//! Fully wired to the backend dde_db::Database for persistence.

use dde_battle::items::{Item, ItemId, ItemType, ItemTarget};
use dde_db::{Database, DbError};
use serde::{Deserialize, Serialize};

/// Element types for items (offensive/defensive)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ElementType {
    #[default]
    None,
    Fire,
    Ice,
    Lightning,
    Earth,
    Holy,
    Dark,
}

impl ElementType {
    pub fn name(&self) -> &'static str {
        match self {
            ElementType::None => "None",
            ElementType::Fire => "Fire",
            ElementType::Ice => "Ice",
            ElementType::Lightning => "Lightning",
            ElementType::Earth => "Earth",
            ElementType::Holy => "Holy",
            ElementType::Dark => "Dark",
        }
    }

    pub fn all() -> &'static [ElementType] {
        &[
            ElementType::None,
            ElementType::Fire,
            ElementType::Ice,
            ElementType::Lightning,
            ElementType::Earth,
            ElementType::Holy,
            ElementType::Dark,
        ]
    }

    /// Convert to string for database storage
    pub fn as_str(&self) -> &'static str {
        match self {
            ElementType::None => "none",
            ElementType::Fire => "fire",
            ElementType::Ice => "ice",
            ElementType::Lightning => "lightning",
            ElementType::Earth => "earth",
            ElementType::Holy => "holy",
            ElementType::Dark => "dark",
        }
    }

    /// Parse from database string
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "fire" => ElementType::Fire,
            "ice" => ElementType::Ice,
            "lightning" => ElementType::Lightning,
            "earth" => ElementType::Earth,
            "holy" => ElementType::Holy,
            "dark" => ElementType::Dark,
            _ => ElementType::None,
        }
    }
}

/// Effect data stored in database effect_json column
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct ItemEffectData {
    /// Power/value of the effect (heal amount, damage, etc.)
    pub power: i32,
    /// Target type for the item
    pub target_type: String,
    /// Cooldown in turns
    pub cooldown: u32,
    /// Element for offensive items
    #[serde(default)]
    pub element: String,
    /// Buff stat type (for buff items)
    #[serde(default)]
    pub buff_stat: String,
    /// Buff duration in turns
    #[serde(default)]
    pub buff_duration: u32,
    /// Can use in battle
    #[serde(default = "default_true")]
    pub use_in_battle: bool,
    /// Can use in field
    #[serde(default = "default_true")]
    pub use_in_field: bool,
}

fn default_true() -> bool {
    true
}

/// Extended item data for editor (includes fields not in base Item)
#[derive(Debug, Clone)]
pub struct EditableItem {
    /// Base item data (from dde_battle)
    pub base: Item,
    /// Element type for offensive/defensive items
    pub element: ElementType,
    /// Maximum stack size
    pub max_stack: u32,
    /// Buy price in gold
    pub buy_price: u32,
    /// Sell price in gold
    pub sell_price: u32,
    /// Icon texture path
    pub icon_path: String,
    /// Can use in battle
    pub use_in_battle: bool,
    /// Can use in field
    pub use_in_field: bool,
    /// Is consumable (false = key item)
    pub consumable: bool,
    /// For buff items: which stat to buff
    pub buff_stat: BuffStatType,
    /// For buff items: duration in turns
    pub buff_duration: u32,
}

/// Stat types for buffs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum BuffStatType {
    #[default]
    Strength,
    Defense,
    Speed,
    Magic,
}

impl BuffStatType {
    pub fn name(&self) -> &'static str {
        match self {
            BuffStatType::Strength => "Strength",
            BuffStatType::Defense => "Defense",
            BuffStatType::Speed => "Speed",
            BuffStatType::Magic => "Magic",
        }
    }

    pub fn all() -> &'static [BuffStatType] {
        &[
            BuffStatType::Strength,
            BuffStatType::Defense,
            BuffStatType::Speed,
            BuffStatType::Magic,
        ]
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            BuffStatType::Strength => "strength",
            BuffStatType::Defense => "defense",
            BuffStatType::Speed => "speed",
            BuffStatType::Magic => "magic",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "strength" | "str" => BuffStatType::Strength,
            "defense" | "def" => BuffStatType::Defense,
            "speed" | "spd" => BuffStatType::Speed,
            "magic" | "mag" => BuffStatType::Magic,
            _ => BuffStatType::Strength,
        }
    }
}

impl EditableItem {
    /// Create new editable item with defaults
    pub fn new(id: ItemId) -> Self {
        Self {
            base: Item {
                id,
                name: format!("New Item {}", id),
                description: String::new(),
                item_type: ItemType::Heal,
                power: 50,
                target_type: ItemTarget::SingleAlly,
                cooldown: 0,
            },
            element: ElementType::None,
            max_stack: 99,
            buy_price: 100,
            sell_price: 50,
            icon_path: String::new(),
            use_in_battle: true,
            use_in_field: true,
            consumable: true,
            buff_stat: BuffStatType::Strength,
            buff_duration: 3,
        }
    }

    /// Create from existing base item
    pub fn from_base(base: Item) -> Self {
        Self {
            base,
            element: ElementType::None,
            max_stack: 99,
            buy_price: 100,
            sell_price: 50,
            icon_path: String::new(),
            use_in_battle: true,
            use_in_field: true,
            consumable: true,
            buff_stat: BuffStatType::Strength,
            buff_duration: 3,
        }
    }

    /// Convert ItemType to database string
    fn item_type_to_db(item_type: ItemType) -> &'static str {
        match item_type {
            ItemType::Heal => "heal",
            ItemType::Mana => "mana",
            ItemType::Elixir => "elixir",
            ItemType::Phoenix => "phoenix",
            ItemType::Buff => "buff",
            ItemType::Offensive => "offensive",
            ItemType::Remedy => "remedy",
        }
    }

    /// Parse ItemType from database string
    fn item_type_from_db(s: &str) -> ItemType {
        match s.to_lowercase().as_str() {
            "heal" => ItemType::Heal,
            "mana" => ItemType::Mana,
            "elixir" => ItemType::Elixir,
            "phoenix" => ItemType::Phoenix,
            "buff" => ItemType::Buff,
            "offensive" => ItemType::Offensive,
            "remedy" => ItemType::Remedy,
            _ => ItemType::Heal,
        }
    }

    /// Convert ItemTarget to database string
    fn target_type_to_db(target: ItemTarget) -> &'static str {
        match target {
            ItemTarget::SingleAlly => "single_ally",
            ItemTarget::AllAllies => "all_allies",
            ItemTarget::SingleEnemy => "single_enemy",
            ItemTarget::AllEnemies => "all_enemies",
            ItemTarget::SelfOnly => "self",
        }
    }

    /// Parse ItemTarget from database string
    fn target_type_from_db(s: &str) -> ItemTarget {
        match s.to_lowercase().as_str() {
            "single_ally" => ItemTarget::SingleAlly,
            "all_allies" => ItemTarget::AllAllies,
            "single_enemy" => ItemTarget::SingleEnemy,
            "all_enemies" => ItemTarget::AllEnemies,
            "self" => ItemTarget::SelfOnly,
            _ => ItemTarget::SingleAlly,
        }
    }

    /// Serialize effect data to JSON for database storage
    fn serialize_effect(&self) -> String {
        let effect = ItemEffectData {
            power: self.base.power,
            target_type: Self::target_type_to_db(self.base.target_type).to_string(),
            cooldown: self.base.cooldown,
            element: self.element.as_str().to_string(),
            buff_stat: self.buff_stat.as_str().to_string(),
            buff_duration: self.buff_duration,
            use_in_battle: self.use_in_battle,
            use_in_field: self.use_in_field,
        };
        serde_json::to_string(&effect).unwrap_or_else(|_| "{}".to_string())
    }

    /// Deserialize effect data from database JSON
    fn deserialize_effect(&mut self, json: &str) {
        if let Ok(effect) = serde_json::from_str::<ItemEffectData>(json) {
            self.base.power = effect.power;
            self.base.target_type = Self::target_type_from_db(&effect.target_type);
            self.base.cooldown = effect.cooldown;
            self.element = ElementType::from_str(&effect.element);
            self.buff_stat = BuffStatType::from_str(&effect.buff_stat);
            self.buff_duration = effect.buff_duration;
            self.use_in_battle = effect.use_in_battle;
            self.use_in_field = effect.use_in_field;
        }
    }

    /// Convert to database row values
    fn to_db_row(&self) -> (ItemId, String, String, String, String, u32, u32, u32, bool, Option<u32>, String) {
        (
            self.base.id,
            self.base.name.clone(),
            self.base.description.clone(),
            Self::item_type_to_db(self.base.item_type).to_string(),
            self.serialize_effect(),
            self.buy_price,
            self.sell_price,
            self.max_stack,
            self.consumable,
            self.icon_path.clone().parse().ok(), // Try to parse as asset ID
            "common".to_string(), // rarity - could be extended
        )
    }

    /// Create from database row
    fn from_db_row(
        id: ItemId,
        name: String,
        description: String,
        item_type: String,
        effect_json: String,
        buy_price: u32,
        sell_price: u32,
        max_stack: u32,
        consumable: bool,
        icon_asset_id: Option<u32>,
    ) -> Self {
        let mut item = Self {
            base: Item {
                id,
                name: name.clone(),
                description: description.clone(),
                item_type: Self::item_type_from_db(&item_type),
                power: 0,
                target_type: ItemTarget::SingleAlly,
                cooldown: 0,
            },
            element: ElementType::None,
            max_stack,
            buy_price,
            sell_price,
            icon_path: icon_asset_id.map(|id| format!("assets/{}", id)).unwrap_or_default(),
            use_in_battle: true,
            use_in_field: true,
            consumable,
            buff_stat: BuffStatType::Strength,
            buff_duration: 3,
        };
        item.deserialize_effect(&effect_json);
        item
    }
}

/// Filter type for item list
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ItemFilter {
    #[default]
    All,
    Heal,
    Mana,
    Elixir,
    Phoenix,
    Buff,
    Offensive,
    Remedy,
}

impl ItemFilter {
    pub fn name(&self) -> &'static str {
        match self {
            ItemFilter::All => "All Items",
            ItemFilter::Heal => "Healing",
            ItemFilter::Mana => "Mana",
            ItemFilter::Elixir => "Elixirs",
            ItemFilter::Phoenix => "Phoenix",
            ItemFilter::Buff => "Buffs",
            ItemFilter::Offensive => "Offensive",
            ItemFilter::Remedy => "Remedies",
        }
    }

    pub fn matches(&self, item: &EditableItem) -> bool {
        match self {
            ItemFilter::All => true,
            ItemFilter::Heal => item.base.item_type == ItemType::Heal,
            ItemFilter::Mana => item.base.item_type == ItemType::Mana,
            ItemFilter::Elixir => item.base.item_type == ItemType::Elixir,
            ItemFilter::Phoenix => item.base.item_type == ItemType::Phoenix,
            ItemFilter::Buff => item.base.item_type == ItemType::Buff,
            ItemFilter::Offensive => item.base.item_type == ItemType::Offensive,
            ItemFilter::Remedy => item.base.item_type == ItemType::Remedy,
        }
    }

    pub fn all() -> &'static [ItemFilter] {
        &[
            ItemFilter::All,
            ItemFilter::Heal,
            ItemFilter::Mana,
            ItemFilter::Elixir,
            ItemFilter::Phoenix,
            ItemFilter::Buff,
            ItemFilter::Offensive,
            ItemFilter::Remedy,
        ]
    }
}

/// Item database editor panel
pub struct ItemDatabaseEditor {
    /// Whether panel is visible
    visible: bool,
    /// Items in the database
    items: Vec<EditableItem>,
    /// Currently selected item ID
    selected_item_id: Option<ItemId>,
    /// Search query
    search_query: String,
    /// Current filter
    filter: ItemFilter,
    /// Next item ID to assign
    next_id: ItemId,
    /// Show delete confirmation
    show_delete_confirm: bool,
    /// Item ID pending deletion
    pending_delete_id: Option<ItemId>,
    /// Database connection (optional - editor works without it for testing)
    db: Option<Database>,
    /// Unsaved changes flag
    has_unsaved_changes: bool,
    /// Status message for user feedback
    status_message: Option<(String, f64)>, // (message, time_remaining_seconds)
}

impl ItemDatabaseEditor {
    /// Create new item database editor with default items
    pub fn new() -> Self {
        let mut editor = Self {
            visible: false,
            items: Vec::new(),
            selected_item_id: None,
            search_query: String::new(),
            filter: ItemFilter::All,
            next_id: 1,
            show_delete_confirm: false,
            pending_delete_id: None,
            db: None,
            has_unsaved_changes: false,
            status_message: None,
        };
        editor.init_default_items();
        editor
    }

    /// Create new editor with database connection
    pub fn with_database(db: Database) -> Self {
        let mut editor = Self {
            visible: false,
            items: Vec::new(),
            selected_item_id: None,
            search_query: String::new(),
            filter: ItemFilter::All,
            next_id: 1,
            show_delete_confirm: false,
            pending_delete_id: None,
            db: Some(db),
            has_unsaved_changes: false,
            status_message: None,
        };
        // Try to load from database, fall back to defaults if empty
        if let Err(e) = editor.load_items_from_db() {
            tracing::warn!("Failed to load items from database: {}", e);
            editor.init_default_items();
        } else if editor.items.is_empty() {
            editor.init_default_items();
            // Save defaults to database
            let _ = editor.save_all_items_to_db();
        }
        editor
    }

    /// Initialize with default RPG items
    fn init_default_items(&mut self) {
        let defaults = vec![
            EditableItem {
                base: Item {
                    id: 1,
                    name: "Potion".to_string(),
                    description: "Restores 50 HP".to_string(),
                    item_type: ItemType::Heal,
                    power: 50,
                    target_type: ItemTarget::SingleAlly,
                    cooldown: 0,
                },
                element: ElementType::None,
                max_stack: 99,
                buy_price: 50,
                sell_price: 25,
                icon_path: "items/potion.png".to_string(),
                use_in_battle: true,
                use_in_field: true,
                consumable: true,
                buff_stat: BuffStatType::Strength,
                buff_duration: 3,
            },
            EditableItem {
                base: Item {
                    id: 2,
                    name: "Hi-Potion".to_string(),
                    description: "Restores 150 HP".to_string(),
                    item_type: ItemType::Heal,
                    power: 150,
                    target_type: ItemTarget::SingleAlly,
                    cooldown: 0,
                },
                element: ElementType::None,
                max_stack: 99,
                buy_price: 150,
                sell_price: 75,
                icon_path: "items/hi_potion.png".to_string(),
                use_in_battle: true,
                use_in_field: true,
                consumable: true,
                buff_stat: BuffStatType::Strength,
                buff_duration: 3,
            },
            EditableItem {
                base: Item {
                    id: 3,
                    name: "Ether".to_string(),
                    description: "Restores 30 MP".to_string(),
                    item_type: ItemType::Mana,
                    power: 30,
                    target_type: ItemTarget::SingleAlly,
                    cooldown: 0,
                },
                element: ElementType::None,
                max_stack: 99,
                buy_price: 150,
                sell_price: 75,
                icon_path: "items/ether.png".to_string(),
                use_in_battle: true,
                use_in_field: true,
                consumable: true,
                buff_stat: BuffStatType::Strength,
                buff_duration: 3,
            },
            EditableItem {
                base: Item {
                    id: 4,
                    name: "Elixir".to_string(),
                    description: "Fully restores HP and MP".to_string(),
                    item_type: ItemType::Elixir,
                    power: 999,
                    target_type: ItemTarget::SingleAlly,
                    cooldown: 3,
                },
                element: ElementType::None,
                max_stack: 10,
                buy_price: 500,
                sell_price: 250,
                icon_path: "items/elixir.png".to_string(),
                use_in_battle: true,
                use_in_field: true,
                consumable: true,
                buff_stat: BuffStatType::Strength,
                buff_duration: 3,
            },
            EditableItem {
                base: Item {
                    id: 5,
                    name: "Phoenix Down".to_string(),
                    description: "Revives a fallen ally with 25% HP".to_string(),
                    item_type: ItemType::Phoenix,
                    power: 25,
                    target_type: ItemTarget::SingleAlly,
                    cooldown: 2,
                },
                element: ElementType::None,
                max_stack: 50,
                buy_price: 300,
                sell_price: 150,
                icon_path: "items/phoenix_down.png".to_string(),
                use_in_battle: true,
                use_in_field: false,
                consumable: true,
                buff_stat: BuffStatType::Strength,
                buff_duration: 3,
            },
            EditableItem {
                base: Item {
                    id: 6,
                    name: "Grenade".to_string(),
                    description: "Deals 100 fire damage to one enemy".to_string(),
                    item_type: ItemType::Offensive,
                    power: 100,
                    target_type: ItemTarget::SingleEnemy,
                    cooldown: 0,
                },
                element: ElementType::Fire,
                max_stack: 99,
                buy_price: 100,
                sell_price: 50,
                icon_path: "items/grenade.png".to_string(),
                use_in_battle: true,
                use_in_field: false,
                consumable: true,
                buff_stat: BuffStatType::Strength,
                buff_duration: 3,
            },
            EditableItem {
                base: Item {
                    id: 7,
                    name: "Remedy".to_string(),
                    description: "Cures all status ailments".to_string(),
                    item_type: ItemType::Remedy,
                    power: 0,
                    target_type: ItemTarget::SingleAlly,
                    cooldown: 0,
                },
                element: ElementType::None,
                max_stack: 99,
                buy_price: 200,
                sell_price: 100,
                icon_path: "items/remedy.png".to_string(),
                use_in_battle: true,
                use_in_field: true,
                consumable: true,
                buff_stat: BuffStatType::Strength,
                buff_duration: 3,
            },
            EditableItem {
                base: Item {
                    id: 8,
                    name: "Strength Tonic".to_string(),
                    description: "Boosts STR by 20% for 3 turns".to_string(),
                    item_type: ItemType::Buff,
                    power: 20,
                    target_type: ItemTarget::SingleAlly,
                    cooldown: 5,
                },
                element: ElementType::None,
                max_stack: 50,
                buy_price: 250,
                sell_price: 125,
                icon_path: "items/strength_tonic.png".to_string(),
                use_in_battle: true,
                use_in_field: false,
                consumable: true,
                buff_stat: BuffStatType::Strength,
                buff_duration: 3,
            },
        ];

        self.next_id = 9;
        self.items = defaults;
        self.has_unsaved_changes = true;
    }

    /// Show the panel
    pub fn show(&mut self) {
        self.visible = true;
    }

    /// Hide the panel
    pub fn hide(&mut self) {
        self.visible = false;
    }

    /// Toggle visibility
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    /// Check if visible
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Set database connection
    pub fn set_database(&mut self, db: Database) {
        self.db = Some(db);
    }

    /// Check if database is connected
    pub fn is_database_connected(&self) -> bool {
        self.db.is_some()
    }

    /// ============================================================
    /// DATABASE INTEGRATION METHODS
    /// ============================================================

    /// Load all items from the database
    /// 
    /// # Errors
    /// Returns DbError if no database is connected or query fails
    pub fn load_items(&mut self, db: &Database) -> Result<(), DbError> {
        self.db = Some(db.clone_connection()?);
        self.load_items_from_db()
    }

    /// Internal: Load items from connected database
    fn load_items_from_db(&mut self) -> Result<(), DbError> {
        let db = self.db.as_ref().ok_or_else(|| {
            DbError::InvalidData("No database connection".to_string())
        })?;

        let mut stmt = db.conn().prepare(
            "SELECT item_id, name, description, item_type, effect_json, 
                    price_buy, price_sell, max_stack, stackable, icon_asset_id
             FROM items
             ORDER BY item_id"
        )?;

        let items: Vec<EditableItem> = stmt
            .query_map([], |row| {
                let id: u32 = row.get(0)?;
                let name: String = row.get(1)?;
                let description: String = row.get(2).unwrap_or_default();
                let item_type: String = row.get(3)?;
                let effect_json: String = row.get(4).unwrap_or_default();
                let buy_price: u32 = row.get(5).unwrap_or(0) as u32;
                let sell_price: u32 = row.get(6).unwrap_or(0) as u32;
                let max_stack: u32 = row.get(7).unwrap_or(99) as u32;
                let consumable: bool = row.get(8).unwrap_or(true);
                let icon_asset_id: Option<u32> = row.get(9)?;

                Ok(EditableItem::from_db_row(
                    id, name, description, item_type, effect_json,
                    buy_price, sell_price, max_stack, consumable, icon_asset_id,
                ))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        // Update next_id to be higher than any loaded item
        self.next_id = items.iter().map(|i| i.base.id + 1).max().unwrap_or(1);
        self.items = items;
        self.has_unsaved_changes = false;
        
        tracing::info!("Loaded {} items from database", self.items.len());
        Ok(())
    }

    /// Save a single item to the database
    ///
    /// Inserts new item or updates existing one based on item_id
    /// 
    /// # Arguments
    /// * `db` - Database connection
    /// * `item` - Item to save
    /// 
    /// # Errors
    /// Returns DbError if save fails
    pub fn save_item(&self, db: &Database, item: &EditableItem) -> Result<(), DbError> {
        let effect_json = item.serialize_effect();
        let item_type_str = EditableItem::item_type_to_db(item.base.item_type);
        let icon_asset_id = item.icon_path.parse::<u32>().ok();

        db.conn().execute(
            "INSERT OR REPLACE INTO items 
             (item_id, name, description, item_type, effect_json,
              price_buy, price_sell, max_stack, stackable, icon_asset_id, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            (
                item.base.id as i64,
                &item.base.name,
                &item.base.description,
                item_type_str,
                &effect_json,
                item.buy_price as i64,
                item.sell_price as i64,
                item.max_stack as i64,
                item.consumable,
                icon_asset_id.map(|id| id as i64),
                chrono::Utc::now().timestamp_millis(),
            ),
        )?;

        tracing::info!("Saved item '{}' (ID: {}) to database", item.base.name, item.base.id);
        Ok(())
    }

    /// Delete an item from the database
    ///
    /// # Arguments
    /// * `db` - Database connection
    /// * `item_id` - ID of item to delete
    /// 
    /// # Returns
    /// * `Ok(true)` if item was deleted
    /// * `Ok(false)` if item didn't exist
    /// 
    /// # Errors
    /// Returns DbError if delete fails
    pub fn delete_item(&self, db: &Database, item_id: ItemId) -> Result<bool, DbError> {
        let rows = db.conn().execute(
            "DELETE FROM items WHERE item_id = ?1",
            [item_id as i64],
        )?;

        if rows > 0 {
            tracing::info!("Deleted item ID: {} from database", item_id);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Save all current items to database
    fn save_all_items_to_db(&mut self) -> Result<(), DbError> {
        let db = self.db.as_ref().ok_or_else(|| {
            DbError::InvalidData("No database connection".to_string())
        })?;

        for item in &self.items {
            self.save_item(db, item)?;
        }

        self.has_unsaved_changes = false;
        self.set_status_message(format!("Saved {} items to database", self.items.len()));
        Ok(())
    }

    /// ============================================================
    /// ITEM MANAGEMENT METHODS
    /// ============================================================

    /// Create a new item
    pub fn create_new_item(&mut self) {
        let id = self.next_id;
        self.next_id += 1;
        
        let mut new_item = EditableItem::new(id);
        new_item.base.name = format!("New Item {}", id);
        
        self.items.push(new_item);
        self.selected_item_id = Some(id);
        self.has_unsaved_changes = true;
        
        // Auto-save to database if connected
        if let Some(db) = &self.db {
            if let Err(e) = self.save_item(db, self.items.last().unwrap()) {
                tracing::warn!("Failed to auto-save new item: {}", e);
            }
        }
    }

    /// Duplicate the selected item
    fn duplicate_selected(&mut self) {
        if let Some(selected_id) = self.selected_item_id {
            if let Some(source) = self.items.iter().find(|i| i.base.id == selected_id).cloned() {
                let id = self.next_id;
                self.next_id += 1;
                
                let source_name = source.base.name.clone();
                let mut new_item = source;
                new_item.base.id = id;
                new_item.base.name = format!("{} (Copy)", source_name);
                
                self.items.push(new_item);
                self.selected_item_id = Some(id);
                self.has_unsaved_changes = true;
                
                // Auto-save to database if connected
                if let Some(db) = &self.db {
                    if let Err(e) = self.save_item(db, self.items.last().unwrap()) {
                        tracing::warn!("Failed to auto-save duplicated item: {}", e);
                    }
                }
            }
        }
    }

    /// Delete an item by ID (with confirmation flow)
    fn delete_item_internal(&mut self, id: ItemId) {
        // Delete from database first if connected
        if let Some(db) = &self.db {
            if let Err(e) = self.delete_item(db, id) {
                tracing::error!("Failed to delete item from database: {}", e);
                self.set_status_message(format!("Error deleting item: {}", e));
                return;
            }
        }

        // Delete from local list
        if let Some(pos) = self.items.iter().position(|i| i.base.id == id) {
            self.items.remove(pos);
            if self.selected_item_id == Some(id) {
                self.selected_item_id = None;
            }
            self.has_unsaved_changes = true;
            self.set_status_message("Item deleted".to_string());
        }
        
        self.show_delete_confirm = false;
        self.pending_delete_id = None;
    }

    /// Mark current changes as saved
    fn mark_saved(&mut self) {
        self.has_unsaved_changes = false;
    }

    /// Set status message with timeout
    fn set_status_message(&mut self, message: String) {
        self.status_message = Some((message, 3.0)); // 3 second display
    }

    /// Update status message timer
    fn update(&mut self, dt: f32) {
        if let Some((_, ref mut time)) = self.status_message {
            *time -= dt as f64;
            if *time <= 0.0 {
                self.status_message = None;
            }
        }
    }

    /// Get filtered and sorted items
    fn filtered_items(&self) -> Vec<&EditableItem> {
        let mut filtered: Vec<&EditableItem> = self
            .items
            .iter()
            .filter(|item| {
                self.filter.matches(item)
                    && (self.search_query.is_empty()
                        || item.base.name.to_lowercase().contains(&self.search_query.to_lowercase()))
            })
            .collect();
        
        filtered.sort_by(|a, b| a.base.name.cmp(&b.base.name));
        filtered
    }

    /// Draw the item database editor
    pub fn draw(&mut self, ctx: &egui::Context) {
        if !self.visible {
            return;
        }

        // Update timers
        self.update(ctx.input(|i| i.stable_dt));

        let mut visible = self.visible;
        egui::Window::new("📦 Item Database")
            .open(&mut visible)
            .resizable(true)
            .default_size([800.0, 600.0])
            .show(ctx, |ui| {
                self.draw_panel_content(ui, ctx);
            });
        self.visible = visible;
    }

    /// Draw panel content
    fn draw_panel_content(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        // Toolbar
        ui.horizontal(|ui| {
            ui.heading("Item Database");
            ui.separator();
            
            if ui.button("➕ New Item").clicked() {
                self.create_new_item();
            }
            
            if ui.button("📋 Duplicate").clicked() {
                self.duplicate_selected();
            }

            // Save button (only if database connected and unsaved changes)
            if self.db.is_some() {
                if self.has_unsaved_changes {
                    if ui.button("💾 Save All").clicked() {
                        if let Err(e) = self.save_all_items_to_db() {
                            tracing::error!("Failed to save items: {}", e);
                            self.set_status_message(format!("Save failed: {}", e));
                        }
                    }
                } else {
                    ui.add_enabled(false, egui::Button::new("✓ Saved"));
                }
            }
            
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Status message
                if let Some((ref msg, _)) = self.status_message {
                    ui.label(egui::RichText::new(msg).color(egui::Color32::GREEN));
                }
                
                // Database connection indicator
                if self.db.is_some() {
                    ui.label(egui::RichText::new("🟢 DB").small());
                } else {
                    ui.label(egui::RichText::new("🔴 No DB").small().color(egui::Color32::YELLOW));
                }
                
                ui.label(format!("{} items", self.items.len()));
            });
        });

        ui.separator();

        // Search and filter row
        ui.horizontal(|ui| {
            ui.label("🔍");
            ui.text_edit_singleline(&mut self.search_query);
            
            ui.separator();
            
            ui.label("Filter:");
            egui::ComboBox::from_id_source("item_filter")
                .selected_text(self.filter.name())
                .show_ui(ui, |ui| {
                    for filter in ItemFilter::all() {
                        if ui.selectable_label(self.filter == *filter, filter.name()).clicked() {
                            self.filter = *filter;
                        }
                    }
                });
        });

        ui.separator();

        // Two-pane layout
        egui::SidePanel::left("item_list_panel")
            .resizable(true)
            .default_width(250.0)
            .show_inside(ui, |ui| {
                self.draw_item_list(ui);
            });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            self.draw_item_editor(ui);
        });

        // Draw delete confirmation dialog if needed
        self.draw_delete_confirmation(ctx);
    }

    /// Draw delete confirmation dialog
    fn draw_delete_confirmation(&mut self, ctx: &egui::Context) {
        if !self.show_delete_confirm {
            return;
        }

        let mut confirm_open = true;
        egui::Window::new("Confirm Delete")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(ctx, |ui| {
                ui.label("Are you sure you want to delete this item?");
                ui.label("This action cannot be undone.");
                ui.add_space(10.0);
                
                ui.horizontal(|ui| {
                    if ui.button("Delete").clicked() {
                        if let Some(id) = self.pending_delete_id {
                            self.delete_item_internal(id);
                        }
                        confirm_open = false;
                    }
                    
                    if ui.button("Cancel").clicked() {
                        self.show_delete_confirm = false;
                        self.pending_delete_id = None;
                        confirm_open = false;
                    }
                });
            });

        if !confirm_open {
            self.show_delete_confirm = false;
        }
    }

    /// Draw the item list (left pane)
    fn draw_item_list(&mut self, ui: &mut egui::Ui) {
        ui.heading("Items");
        ui.separator();

        // Collect data needed for rendering to avoid borrow issues
        let items_data: Vec<(ItemId, String, ItemType)> = self
            .items
            .iter()
            .filter(|item| {
                self.filter.matches(item)
                    && (self.search_query.is_empty()
                        || item.base.name.to_lowercase().contains(&self.search_query.to_lowercase()))
            })
            .map(|item| (item.base.id, item.base.name.clone(), item.base.item_type))
            .collect();
        
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (item_id, item_name, item_type) in items_data {
                let is_selected = self.selected_item_id == Some(item_id);
                let type_icon = item_type_icon(item_type);
                
                let response = ui.selectable_label(
                    is_selected,
                    format!("{} {}", type_icon, item_name),
                );
                
                if response.clicked() {
                    self.selected_item_id = Some(item_id);
                }
                
                // Right-click context menu
                let mut duplicate_requested = false;
                let mut delete_requested = false;
                
                response.context_menu(|ui| {
                    if ui.button("Duplicate").clicked() {
                        duplicate_requested = true;
                        ui.close_menu();
                    }
                    
                    if ui.button("Delete").clicked() {
                        delete_requested = true;
                        ui.close_menu();
                    }
                });
                
                // Handle menu actions outside the closure to avoid borrow issues
                if duplicate_requested {
                    if let Some(source) = self.items.iter().find(|i| i.base.id == item_id).cloned() {
                        let id = self.next_id;
                        self.next_id += 1;
                        let source_name = source.base.name.clone();
                        let mut new_item = source;
                        new_item.base.id = id;
                        new_item.base.name = format!("{} (Copy)", source_name);
                        self.items.push(new_item);
                        self.selected_item_id = Some(id);
                        self.has_unsaved_changes = true;
                        
                        // Auto-save duplicated item
                        if let Some(db) = &self.db {
                            if let Err(e) = self.save_item(db, self.items.last().unwrap()) {
                                tracing::warn!("Failed to auto-save duplicated item: {}", e);
                            }
                        }
                    }
                }
                
                if delete_requested {
                    self.show_delete_confirm = true;
                    self.pending_delete_id = Some(item_id);
                }
            }
        });
    }

    /// Draw the item editor (right pane)
    fn draw_item_editor(&mut self, ui: &mut egui::Ui) {
        let Some(selected_id) = self.selected_item_id else {
            ui.vertical_centered(|ui| {
                ui.add_space(100.0);
                ui.label("Select an item to edit");
            });
            return;
        };

        let Some(item_index) = self.items.iter().position(|i| i.base.id == selected_id) else {
            return;
        };

        // Clone item data to avoid borrow issues during UI rendering
        let mut item_clone = self.items[item_index].clone();
        let mut item_modified = false;

        // Header with name and delete button
        ui.horizontal(|ui| {
            ui.heading("Edit Item");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("🗑️ Delete").clicked() {
                    self.show_delete_confirm = true;
                    self.pending_delete_id = Some(selected_id);
                }
            });
        });

        ui.separator();

        // Scrollable editor area
        egui::ScrollArea::vertical().show(ui, |ui| {
            // Basic Info Section
            ui.collapsing("Basic Info", |ui| {
                ui.horizontal(|ui| {
                    ui.label("Name:");
                    if ui.text_edit_singleline(&mut item_clone.base.name).changed() {
                        item_modified = true;
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Description:");
                });
                if ui.add(
                    egui::TextEdit::multiline(&mut item_clone.base.description)
                        .desired_rows(3)
                        .desired_width(ui.available_width()),
                ).changed() {
                    item_modified = true;
                }

                ui.horizontal(|ui| {
                    ui.label("Icon Path:");
                    if ui.text_edit_singleline(&mut item_clone.icon_path).changed() {
                        item_modified = true;
                    }
                });
            });

            ui.add_space(10.0);

            // Item Type Section
            ui.collapsing("Type & Effects", |ui| {
                // Item Type dropdown
                ui.horizontal(|ui| {
                    ui.label("Item Type:");
                    egui::ComboBox::from_id_source("item_type")
                        .selected_text(item_type_name(item_clone.base.item_type))
                        .show_ui(ui, |ui| {
                            for &item_type in all_item_types() {
                                if ui.selectable_label(
                                    item_clone.base.item_type == item_type,
                                    item_type_name(item_type),
                                ).clicked() {
                                    item_clone.base.item_type = item_type;
                                    item_modified = true;
                                }
                            }
                        });
                });

                // Effect value with context-aware label
                let effect_label = match item_clone.base.item_type {
                    ItemType::Heal => "Heal Amount:",
                    ItemType::Mana => "MP Restore:",
                    ItemType::Elixir => "Restore Power:",
                    ItemType::Phoenix => "Revive HP %:",
                    ItemType::Buff => "Buff Power:",
                    ItemType::Offensive => "Damage:",
                    ItemType::Remedy => "Cure Power:",
                };

                ui.horizontal(|ui| {
                    ui.label(effect_label);
                    if ui.add(egui::Slider::new(&mut item_clone.base.power, 0..=999)).changed() {
                        item_modified = true;
                    }
                });

                // Buff-specific options
                if item_clone.base.item_type == ItemType::Buff {
                    ui.horizontal(|ui| {
                        ui.label("Buff Stat:");
                        egui::ComboBox::from_id_source("buff_stat")
                            .selected_text(item_clone.buff_stat.name())
                            .show_ui(ui, |ui| {
                                for stat in BuffStatType::all() {
                                    if ui.selectable_label(
                                        item_clone.buff_stat == *stat,
                                        stat.name(),
                                    ).clicked() {
                                        item_clone.buff_stat = *stat;
                                        item_modified = true;
                                    }
                                }
                            });
                    });

                    ui.horizontal(|ui| {
                        ui.label("Duration (turns):");
                        if ui.add(egui::Slider::new(&mut item_clone.buff_duration, 1..=10)).changed() {
                            item_modified = true;
                        }
                    });
                }

                // Element (for offensive items mainly)
                if item_clone.base.item_type == ItemType::Offensive {
                    ui.horizontal(|ui| {
                        ui.label("Element:");
                        egui::ComboBox::from_id_source("element")
                            .selected_text(item_clone.element.name())
                            .show_ui(ui, |ui| {
                                for element in ElementType::all() {
                                    if ui.selectable_label(
                                        item_clone.element == *element,
                                        element.name(),
                                    ).clicked() {
                                        item_clone.element = *element;
                                        item_modified = true;
                                    }
                                }
                            });
                    });
                }
            });

            ui.add_space(10.0);

            // Target & Usage Section
            ui.collapsing("Target & Usage", |ui| {
                // Target Type
                ui.horizontal(|ui| {
                    ui.label("Target:");
                    egui::ComboBox::from_id_source("target_type")
                        .selected_text(target_type_name(item_clone.base.target_type))
                        .show_ui(ui, |ui| {
                            for &target in all_target_types() {
                                if ui.selectable_label(
                                    item_clone.base.target_type == target,
                                    target_type_name(target),
                                ).clicked() {
                                    item_clone.base.target_type = target;
                                    item_modified = true;
                                }
                            }
                        });
                });

                // Cooldown
                ui.horizontal(|ui| {
                    ui.label("Cooldown (turns):");
                    if ui.add(egui::Slider::new(&mut item_clone.base.cooldown, 0..=10)).changed() {
                        item_modified = true;
                    }
                });

                // Usage toggles
                ui.horizontal(|ui| {
                    if ui.checkbox(&mut item_clone.use_in_battle, "Use in Battle").changed() {
                        item_modified = true;
                    }
                    if ui.checkbox(&mut item_clone.use_in_field, "Use in Field").changed() {
                        item_modified = true;
                    }
                });

                // Consumable toggle
                ui.horizontal(|ui| {
                    if ui.checkbox(&mut item_clone.consumable, "Consumable").changed() {
                        item_modified = true;
                    }
                    if !item_clone.consumable {
                        ui.label("(Key Item)");
                    }
                });

                // Max stack
                if item_clone.consumable {
                    ui.horizontal(|ui| {
                        ui.label("Max Stack:");
                        if ui.add(egui::Slider::new(&mut item_clone.max_stack, 1..=999)).changed() {
                            item_modified = true;
                        }
                    });
                }
            });

            ui.add_space(10.0);

            // Economy Section
            ui.collapsing("Economy", |ui| {
                ui.horizontal(|ui| {
                    ui.label("Buy Price:");
                    if ui.add(egui::DragValue::new(&mut item_clone.buy_price).range(0..=99999)).changed() {
                        item_modified = true;
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Sell Price:");
                    if ui.add(egui::DragValue::new(&mut item_clone.sell_price).range(0..=99999)).changed() {
                        item_modified = true;
                    }
                });

                // Auto-calculate sell price at 50% if buy price changed significantly
                let expected_sell = item_clone.buy_price / 2;
                if item_clone.sell_price == 0 && item_clone.buy_price > 0 {
                    item_clone.sell_price = expected_sell;
                }
            });

            ui.add_space(10.0);

            // Effect Preview Section
            ui.collapsing("Effect Preview", |ui| {
                self.draw_effect_preview(ui, &item_clone);
            });

            ui.add_space(10.0);

            // Item ID (read-only)
            ui.horizontal(|ui| {
                ui.label("Item ID:");
                ui.label(item_clone.base.id.to_string());
                ui.label(egui::RichText::new("(read-only)").weak());
            });
        });

        // Apply changes if modified
        if item_modified {
            self.items[item_index] = item_clone.clone();
            self.has_unsaved_changes = true;
            
            // Auto-save to database if connected
            if let Some(db) = &self.db {
                if let Err(e) = self.save_item(db, &item_clone) {
                    tracing::warn!("Failed to auto-save item: {}", e);
                }
            }
        }
    }

    /// Draw effect preview for the selected item
    fn draw_effect_preview(&self, ui: &mut egui::Ui, item: &EditableItem) {
        ui.group(|ui| {
            ui.set_width(ui.available_width());
            
            match item.base.item_type {
                ItemType::Heal => {
                    ui.label(format!("💚 Restores {} HP to {}", 
                        item.base.power,
                        target_type_name(item.base.target_type)));
                    ui.label("Effective on: Allies with HP < Max HP");
                }
                ItemType::Mana => {
                    ui.label(format!("💙 Restores {} MP to {}", 
                        item.base.power,
                        target_type_name(item.base.target_type)));
                    ui.label("Effective on: Allies with MP < Max MP");
                }
                ItemType::Elixir => {
                    ui.label(format!("💚💙 Fully restores HP and MP to {}", 
                        target_type_name(item.base.target_type)));
                }
                ItemType::Phoenix => {
                    ui.label(format!("🔥 Revives KO'd ally with {}% HP", item.base.power));
                    ui.label(format!("Target: {}", target_type_name(item.base.target_type)));
                    ui.label("Note: Only works on defeated allies");
                }
                ItemType::Buff => {
                    let stat_name = item.buff_stat.name();
                    ui.label(format!("⬆️ Boosts {} by {}% for {} turns", 
                        stat_name, item.base.power, item.buff_duration));
                    ui.label(format!("Target: {}", target_type_name(item.base.target_type)));
                }
                ItemType::Offensive => {
                    let element_str = if item.element != ElementType::None {
                        format!(" [{}]", item.element.name())
                    } else {
                        String::new()
                    };
                    ui.label(format!("⚔️ Deals {}{} damage to {}", 
                        item.base.power,
                        element_str,
                        target_type_name(item.base.target_type)));
                }
                ItemType::Remedy => {
                    ui.label(format!("✨ Cures all status ailments from {}", 
                        target_type_name(item.base.target_type)));
                }
            }

            if item.base.cooldown > 0 {
                ui.add_space(5.0);
                ui.label(format!("⏱️ Cooldown: {} turns after use", item.base.cooldown));
            }
        });
    }

    /// Get all items as base Item references
    pub fn get_all_base_items(&self) -> Vec<&Item> {
        self.items.iter().map(|e| &e.base).collect()
    }

    /// Get a specific item by ID
    pub fn get_item(&self, id: ItemId) -> Option<&EditableItem> {
        self.items.iter().find(|i| i.base.id == id)
    }

    /// Get mutable reference to item
    pub fn get_item_mut(&mut self, id: ItemId) -> Option<&mut EditableItem> {
        self.items.iter_mut().find(|i| i.base.id == id)
    }

    /// Import items from the battle item database
    pub fn import_from_database(&mut self, db: &dde_battle::items::ItemDatabase) {
        self.items.clear();
        for item in db.all() {
            self.items.push(EditableItem::from_base(item.clone()));
            self.next_id = self.next_id.max(item.id + 1);
        }
        self.has_unsaved_changes = true;
    }

    /// Export all items to the battle item database
    pub fn export_to_database(&self, db: &mut dde_battle::items::ItemDatabase) {
        for editable in &self.items {
            db.add(editable.base.clone());
        }
    }
}

impl Default for ItemDatabaseEditor {
    fn default() -> Self {
        Self::new()
    }
}

/// Get icon for item type
fn item_type_icon(item_type: ItemType) -> &'static str {
    match item_type {
        ItemType::Heal => "💚",
        ItemType::Mana => "💙",
        ItemType::Elixir => "🧪",
        ItemType::Phoenix => "🔥",
        ItemType::Buff => "⬆️",
        ItemType::Offensive => "⚔️",
        ItemType::Remedy => "✨",
    }
}

/// Get display name for item type
fn item_type_name(item_type: ItemType) -> &'static str {
    match item_type {
        ItemType::Heal => "Heal (Restore HP)",
        ItemType::Mana => "Mana (Restore MP)",
        ItemType::Elixir => "Elixir (HP/MP)",
        ItemType::Phoenix => "Phoenix (Revive)",
        ItemType::Buff => "Buff (Stat Boost)",
        ItemType::Offensive => "Offensive (Damage)",
        ItemType::Remedy => "Remedy (Cure Status)",
    }
}

/// Get all item types
fn all_item_types() -> &'static [ItemType] {
    &[
        ItemType::Heal,
        ItemType::Mana,
        ItemType::Elixir,
        ItemType::Phoenix,
        ItemType::Buff,
        ItemType::Offensive,
        ItemType::Remedy,
    ]
}

/// Get display name for target type
fn target_type_name(target: ItemTarget) -> &'static str {
    match target {
        ItemTarget::SelfOnly => "Self",
        ItemTarget::SingleAlly => "Single Ally",
        ItemTarget::AllAllies => "All Allies",
        ItemTarget::SingleEnemy => "Single Enemy",
        ItemTarget::AllEnemies => "All Enemies",
    }
}

/// Get all target types
fn all_target_types() -> &'static [ItemTarget] {
    &[
        ItemTarget::SelfOnly,
        ItemTarget::SingleAlly,
        ItemTarget::AllAllies,
        ItemTarget::SingleEnemy,
        ItemTarget::AllEnemies,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_editor_creation() {
        let editor = ItemDatabaseEditor::new();
        assert!(!editor.is_visible());
        assert!(!editor.items.is_empty()); // Should have defaults
    }

    #[test]
    fn test_create_new_item() {
        let mut editor = ItemDatabaseEditor::new();
        let initial_count = editor.items.len();
        
        editor.create_new_item();
        
        assert_eq!(editor.items.len(), initial_count + 1);
        assert!(editor.selected_item_id.is_some());
    }

    #[test]
    fn test_duplicate_item() {
        let mut editor = ItemDatabaseEditor::new();
        editor.selected_item_id = Some(1);
        let initial_count = editor.items.len();
        
        // Manually trigger duplication
        if let Some(selected_id) = editor.selected_item_id {
            if let Some(source) = editor.items.iter().find(|i| i.base.id == selected_id).cloned() {
                let id = editor.next_id;
                editor.next_id += 1;
                let mut new_item = source;
                new_item.base.id = id;
                new_item.base.name = format!("{} (Copy)", source.base.name);
                editor.items.push(new_item);
                editor.selected_item_id = Some(id);
            }
        }
        
        assert_eq!(editor.items.len(), initial_count + 1);
    }

    #[test]
    fn test_delete_item() {
        let mut editor = ItemDatabaseEditor::new();
        editor.create_new_item();
        let id = editor.selected_item_id.unwrap();
        
        editor.delete_item_internal(id);
        
        assert!(editor.get_item(id).is_none());
    }

    #[test]
    fn test_item_filter() {
        assert!(ItemFilter::Heal.matches(&EditableItem {
            base: Item { item_type: ItemType::Heal, ..Default::test_default() },
            ..Default::test_default()
        }));
        
        assert!(!ItemFilter::Offensive.matches(&EditableItem {
            base: Item { item_type: ItemType::Heal, ..Default::test_default() },
            ..Default::test_default()
        }));
    }

    #[test]
    fn test_element_type_names() {
        assert_eq!(ElementType::Fire.name(), "Fire");
        assert_eq!(ElementType::Ice.name(), "Ice");
        assert_eq!(ElementType::None.name(), "None");
    }

    #[test]
    fn test_element_type_serde() {
        assert_eq!(ElementType::Fire.as_str(), "fire");
        assert_eq!(ElementType::from_str("fire"), ElementType::Fire);
        assert_eq!(ElementType::from_str("FIRE"), ElementType::Fire);
        assert_eq!(ElementType::from_str("unknown"), ElementType::None);
    }

    #[test]
    fn test_buff_stat_serde() {
        assert_eq!(BuffStatType::Strength.as_str(), "strength");
        assert_eq!(BuffStatType::from_str("strength"), BuffStatType::Strength);
        assert_eq!(BuffStatType::from_str("STR"), BuffStatType::Strength);
        assert_eq!(BuffStatType::from_str("unknown"), BuffStatType::Strength);
    }

    #[test]
    fn test_item_effect_serialization() {
        let item = EditableItem::new(1);
        let json = item.serialize_effect();
        assert!(!json.is_empty());
        
        let mut item2 = EditableItem::new(2);
        item2.deserialize_effect(&json);
        
        assert_eq!(item.base.power, item2.base.power);
        assert_eq!(item.base.cooldown, item2.base.cooldown);
        assert_eq!(item.use_in_battle, item2.use_in_battle);
        assert_eq!(item.use_in_field, item2.use_in_field);
    }

    #[test]
    fn test_database_type_conversions() {
        // Test ItemType conversions
        assert_eq!(EditableItem::item_type_to_db(ItemType::Heal), "heal");
        assert_eq!(EditableItem::item_type_from_db("heal"), ItemType::Heal);
        assert_eq!(EditableItem::item_type_from_db("HEAL"), ItemType::Heal);
        assert_eq!(EditableItem::item_type_from_db("unknown"), ItemType::Heal);

        // Test ItemTarget conversions
        assert_eq!(EditableItem::target_type_to_db(ItemTarget::SingleAlly), "single_ally");
        assert_eq!(EditableItem::target_type_from_db("single_ally"), ItemTarget::SingleAlly);
        assert_eq!(EditableItem::target_type_from_db("SINGLE_ALLY"), ItemTarget::SingleAlly);
        assert_eq!(EditableItem::target_type_from_db("unknown"), ItemTarget::SingleAlly);
    }

    // Helper trait for tests
    trait TestDefault {
        fn test_default() -> Self;
    }

    impl TestDefault for EditableItem {
        fn test_default() -> Self {
            EditableItem::new(999)
        }
    }

    impl TestDefault for Item {
        fn test_default() -> Self {
            Item {
                id: 1,
                name: "Test".to_string(),
                description: "Test item".to_string(),
                item_type: ItemType::Heal,
                power: 50,
                target_type: ItemTarget::SingleAlly,
                cooldown: 0,
            }
        }
    }
}
