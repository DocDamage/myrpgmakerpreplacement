#![cfg(feature = "ui")]

//! Integration Tests for Inventory UI System
//!
//! Tests the inventory UI components including:
//! - Category filtering
//! - Sort methods
//! - Action handling
//! - Equipment slot validation

use dde_battle::items::{Item, ItemDatabase, ItemTarget, ItemType};
use dde_battle::ui::inventory::{
    EquipmentComparison, EquipmentSlot, InventoryAction, InventoryUiState, ItemCategory,
    ItemRarity, ItemTooltip, SortMethod,
};
use dde_core::components::{Equipment, Inventory};

/// Create a test inventory with various items
fn create_test_inventory() -> Inventory {
    let mut inventory = Inventory::new();
    inventory.add_item(1, 5, 99); // Potion (Heal)
    inventory.add_item(2, 3, 99); // Hi-Potion (Heal)
    inventory.add_item(3, 2, 99); // Ether (Mana)
    inventory.add_item(4, 1, 99); // Elixir (Elixir)
    inventory.add_item(6, 4, 99); // Grenade (Offensive)
    inventory
}

/// Create test equipment with some items equipped
fn create_test_equipment() -> Equipment {
    Equipment {
        weapon: Some(100),
        armor: Some(101),
        accessory: None,
    }
}

#[test]
fn test_inventory_ui_state_lifecycle() {
    let mut state = InventoryUiState::new();

    // Initial state
    assert!(!state.visible);
    assert_eq!(state.selected_category, ItemCategory::All);
    assert!(state.selected_slot.is_none());

    // Show inventory
    state.show();
    assert!(state.visible);

    // Hide inventory
    state.hide();
    assert!(!state.visible);

    // Toggle
    state.toggle();
    assert!(state.visible);
    state.toggle();
    assert!(!state.visible);
}

#[test]
fn test_category_cycling() {
    let mut state = InventoryUiState::new();

    assert_eq!(state.selected_category, ItemCategory::All);

    // Simulate Tab key cycling through categories
    state.selected_category = ItemCategory::Consumable;
    assert_eq!(state.selected_category, ItemCategory::Consumable);

    state.selected_category = ItemCategory::Equipment;
    assert_eq!(state.selected_category, ItemCategory::Equipment);

    state.selected_category = ItemCategory::KeyItem;
    assert_eq!(state.selected_category, ItemCategory::KeyItem);

    // Wrap around
    state.selected_category = ItemCategory::All;
    assert_eq!(state.selected_category, ItemCategory::All);
}

#[test]
fn test_item_category_filtering() {
    let inventory = create_test_inventory();
    let item_db = ItemDatabase::new();
    let state = InventoryUiState::new();

    // All items
    let all_items = state.get_filtered_items(&inventory, &item_db);
    assert_eq!(all_items.len(), 5);

    // Consumable items (Heal, Mana, Elixir, Phoenix, Remedy)
    let mut consumable_state = InventoryUiState::new();
    consumable_state.selected_category = ItemCategory::Consumable;
    let consumable_items = consumable_state.get_filtered_items(&inventory, &item_db);
    // Potion, Hi-Potion, Ether, Elixir are consumable
    assert_eq!(consumable_items.len(), 4);
}

#[test]
fn test_search_filtering() {
    let inventory = create_test_inventory();
    let item_db = ItemDatabase::new();
    let mut state = InventoryUiState::new();

    // Empty search returns all
    let items = state.get_filtered_items(&inventory, &item_db);
    assert_eq!(items.len(), 5);

    // Search for "pot" (should match Potion, Hi-Potion)
    state.search_query = "pot".to_string();
    let items = state.get_filtered_items(&inventory, &item_db);
    assert_eq!(items.len(), 2);
    assert!(items
        .iter()
        .any(|(_, _, item)| item.name.contains("Potion")));

    // Search for "ether"
    state.search_query = "ether".to_string();
    let items = state.get_filtered_items(&inventory, &item_db);
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].2.name, "Ether");

    // Case insensitive search
    state.search_query = "GRENADE".to_string();
    let items = state.get_filtered_items(&inventory, &item_db);
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].2.name, "Grenade");

    // No matches
    state.search_query = "nonexistent".to_string();
    let items = state.get_filtered_items(&inventory, &item_db);
    assert!(items.is_empty());
}

#[test]
fn test_sort_by_name() {
    let inventory = create_test_inventory();
    let item_db = ItemDatabase::new();
    let mut state = InventoryUiState::new();
    state.sort_method = SortMethod::ByName;

    let items = state.get_filtered_items(&inventory, &item_db);
    assert!(!items.is_empty());

    // Check that items are sorted alphabetically
    for i in 1..items.len() {
        assert!(
            items[i - 1].2.name <= items[i].2.name,
            "Items should be sorted by name: {} before {}",
            items[i - 1].2.name,
            items[i].2.name
        );
    }
}

#[test]
fn test_sort_by_type() {
    let inventory = create_test_inventory();
    let item_db = ItemDatabase::new();
    let mut state = InventoryUiState::new();
    state.sort_method = SortMethod::ByType;

    let items = state.get_filtered_items(&inventory, &item_db);
    assert!(!items.is_empty());

    // Items should be grouped by type
    let type_strings: Vec<_> = items
        .iter()
        .map(|(_, _, item)| format!("{:?}", item.item_type))
        .collect();

    for i in 1..type_strings.len() {
        assert!(
            type_strings[i - 1] <= type_strings[i],
            "Items should be sorted by type"
        );
    }
}

#[test]
fn test_sort_by_value() {
    let inventory = create_test_inventory();
    let item_db = ItemDatabase::new();
    let mut state = InventoryUiState::new();
    state.sort_method = SortMethod::ByValue;

    let items = state.get_filtered_items(&inventory, &item_db);
    assert!(!items.is_empty());

    // Items should be sorted by power descending
    for i in 1..items.len() {
        assert!(
            items[i - 1].2.power >= items[i].2.power,
            "Items should be sorted by power descending: {} ({} power) before {} ({} power)",
            items[i - 1].2.name,
            items[i - 1].2.power,
            items[i].2.name,
            items[i].2.power
        );
    }
}

#[test]
#[cfg(feature = "ui")]
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
    assert_eq!(
        ItemRarity::Epic.color(),
        egui::Color32::from_rgb(180, 0, 255)
    );
    assert_eq!(
        ItemRarity::Legendary.color(),
        egui::Color32::from_rgb(255, 165, 0)
    );
}

#[test]
fn test_equipment_slot_types() {
    assert_eq!(EquipmentSlot::Weapon.name(), "Weapon");
    assert_eq!(EquipmentSlot::Armor.name(), "Armor");
    assert_eq!(EquipmentSlot::Accessory.name(), "Accessory");

    assert_eq!(EquipmentSlot::Weapon.icon(), "⚔");
    assert_eq!(EquipmentSlot::Armor.icon(), "🛡");
    assert_eq!(EquipmentSlot::Accessory.icon(), "💍");
}

#[test]
fn test_item_category_display() {
    assert_eq!(ItemCategory::All.name(), "All");
    assert_eq!(ItemCategory::Consumable.name(), "Consumable");
    assert_eq!(ItemCategory::Equipment.name(), "Equipment");
    assert_eq!(ItemCategory::KeyItem.name(), "Key");

    assert_eq!(ItemCategory::All.icon(), "☰");
    assert_eq!(ItemCategory::Consumable.icon(), "🧪");
    assert_eq!(ItemCategory::Equipment.icon(), "⚔");
    assert_eq!(ItemCategory::KeyItem.icon(), "🔑");
}

#[test]
fn test_inventory_action_variants() {
    // Test that all action variants can be created
    let use_action = InventoryAction::UseItem {
        item_id: 1,
        target: None,
    };

    let equip_action = InventoryAction::EquipItem {
        item_id: 1,
        slot: EquipmentSlot::Weapon,
    };

    let unequip_action = InventoryAction::UnequipItem {
        slot: EquipmentSlot::Armor,
    };

    let drop_action = InventoryAction::DropItem {
        item_id: 1,
        quantity: 1,
    };

    let sort_action = InventoryAction::SortItems(SortMethod::ByName);

    // Verify they are different variants
    assert!(matches!(use_action, InventoryAction::UseItem { .. }));
    assert!(matches!(equip_action, InventoryAction::EquipItem { .. }));
    assert!(matches!(
        unequip_action,
        InventoryAction::UnequipItem { .. }
    ));
    assert!(matches!(drop_action, InventoryAction::DropItem { .. }));
    assert!(matches!(sort_action, InventoryAction::SortItems(_)));
}

#[test]
fn test_sort_method_names() {
    assert_eq!(SortMethod::ByName.name(), "Name");
    assert_eq!(SortMethod::ByType.name(), "Type");
    assert_eq!(SortMethod::ByValue.name(), "Value");
    assert_eq!(SortMethod::RecentlyAcquired.name(), "Recent");
}

#[test]
fn test_equipment_struct() {
    let equipment = create_test_equipment();

    assert!(equipment.weapon.is_some());
    assert!(equipment.armor.is_some());
    assert!(equipment.accessory.is_none());

    assert_eq!(equipment.weapon, Some(100));
    assert_eq!(equipment.armor, Some(101));
}

#[test]
fn test_item_tooltip_creation() {
    let item = Item {
        id: 1,
        name: "Test Item".to_string(),
        description: "A test item".to_string(),
        item_type: ItemType::Heal,
        power: 50,
        target_type: ItemTarget::SingleAlly,
        cooldown: 0,
    };

    let tooltip = ItemTooltip {
        item: item.clone(),
        comparison: None,
    };

    assert_eq!(tooltip.item.name, "Test Item");
    assert!(tooltip.comparison.is_none());

    // With comparison
    let comparison = EquipmentComparison {
        current: Some(item.clone()),
        equipped: None,
        stat_diffs: vec![(dde_battle::items::StatType::Str, 5)],
    };

    let tooltip_with_comparison = ItemTooltip {
        item: item.clone(),
        comparison: Some(comparison),
    };

    assert!(tooltip_with_comparison.comparison.is_some());
    let comp = tooltip_with_comparison.comparison.unwrap();
    assert_eq!(comp.stat_diffs.len(), 1);
    assert_eq!(comp.stat_diffs[0].1, 5);
}

#[test]
fn test_inventory_with_equipment_slots() {
    let inventory = create_test_inventory();
    let equipment = create_test_equipment();

    // Verify inventory has items
    assert!(!inventory.items.is_empty());

    // Verify equipment has some items equipped
    assert!(equipment.weapon.is_some());
    assert!(equipment.armor.is_some());

    // Create UI state
    let mut state = InventoryUiState::new();
    state.show();
    assert!(state.visible);
}

#[test]
fn test_category_and_search_combined() {
    let inventory = create_test_inventory();
    let item_db = ItemDatabase::new();
    let mut state = InventoryUiState::new();

    // Set category to Consumable
    state.selected_category = ItemCategory::Consumable;

    // Search for "potion" within consumables
    state.search_query = "potion".to_string();
    let items = state.get_filtered_items(&inventory, &item_db);

    // Should find Potion and Hi-Potion
    assert_eq!(items.len(), 2);
    assert!(items
        .iter()
        .all(|(_, _, item)| { item.name.to_lowercase().contains("potion") }));
}

#[test]
fn test_recently_acquired_sort_preserves_order() {
    let mut inventory = Inventory::new();
    let item_db = ItemDatabase::new();
    let mut state = InventoryUiState::new();
    state.sort_method = SortMethod::RecentlyAcquired;

    // Add items in specific order
    inventory.add_item(1, 1, 99);
    inventory.add_item(2, 1, 99);
    inventory.add_item(3, 1, 99);

    let items = state.get_filtered_items(&inventory, &item_db);

    // Should preserve insertion order
    assert_eq!(items.len(), 3);
    // Original order should be maintained
    assert_eq!(items[0].2.id, 1);
    assert_eq!(items[1].2.id, 2);
    assert_eq!(items[2].2.id, 3);
}
