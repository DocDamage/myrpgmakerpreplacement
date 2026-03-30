//! UI Components for Battle System
//!
//! Provides egui-based interfaces for:
//! - Inventory management
//! - Battle status displays
//! - Skill selection

pub mod inventory;

pub use inventory::{
    draw_equipment_slot, EquipmentComparison, EquipmentSlot, EquipmentSlotResponse,
    InventoryAction, InventoryActionResult, InventoryUiState, ItemCategory, ItemError, ItemRarity,
    ItemTooltip, SortMethod,
};
