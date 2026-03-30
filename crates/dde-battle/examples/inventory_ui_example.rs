#![cfg(feature = "ui")]

//! Inventory UI Example
//!
//! This example demonstrates how to integrate the inventory UI system
//! into your game application.
//!
//! Run with: cargo run --example inventory_ui_example --features ui

use dde_battle::items::ItemDatabase;
use dde_battle::ui::inventory::{EquipmentSlot, InventoryAction, InventoryUiState, SortMethod};
use dde_core::components::{Equipment, Inventory};
use dde_core::World;

fn main() {
    println!("Inventory UI Integration Example");
    println!("================================\n");

    // Initialize the world and systems
    let mut world = World::new();
    let mut inventory_ui = InventoryUiState::new();
    let item_db = ItemDatabase::new();

    // Create a player entity with inventory and equipment
    let player = world.spawn((Inventory::new(), Equipment::default()));

    println!("Player entity created: {:?}", player);

    // Add some items to the player's inventory
    {
        let mut query = world.query_one_mut::<&mut Inventory>(player).unwrap();
        query.add_item(1, 5, 99); // 5 Potions
        query.add_item(2, 2, 99); // 2 Hi-Potions
        query.add_item(3, 3, 99); // 3 Ethers
        query.add_item(6, 1, 99); // 1 Grenade
        println!("Added items to inventory");
    }

    // Example: Show inventory UI
    println!("\n--- Opening Inventory ---");
    inventory_ui.show();
    println!("Inventory visible: {}", inventory_ui.visible);

    // Example: Filter by category
    println!("\n--- Category Filtering ---");
    inventory_ui.selected_category = dde_battle::ui::inventory::ItemCategory::Consumable;
    println!("Selected category: {:?}", inventory_ui.selected_category);

    // Example: Search for items
    println!("\n--- Search ---");
    inventory_ui.search_query = "potion".to_string();
    println!("Search query: '{}'", inventory_ui.search_query);

    // Example: Sort items
    println!("\n--- Sorting ---");
    inventory_ui.sort_method = SortMethod::ByValue;
    println!("Sort method: {:?}", inventory_ui.sort_method);

    // Example: Handle actions
    println!("\n--- Actions ---");

    // Use an item
    let use_action = InventoryAction::UseItem {
        item_id: 1,
        target: Some(player),
    };
    println!("Action: Use Item (id: 1)");

    // Equip an item
    let equip_action = InventoryAction::EquipItem {
        item_id: 100,
        slot: EquipmentSlot::Weapon,
    };
    println!(
        "Action: Equip Item (id: 100) to {:?}",
        EquipmentSlot::Weapon
    );

    // Unequip
    let unequip_action = InventoryAction::UnequipItem {
        slot: EquipmentSlot::Armor,
    };
    println!("Action: Unequip from {:?}", EquipmentSlot::Armor);

    // Drop items
    let drop_action = InventoryAction::DropItem {
        item_id: 1,
        quantity: 2,
    };
    println!("Action: Drop {}x Item {}", 2, 1);

    // Sort
    let sort_action = InventoryAction::SortItems(SortMethod::ByName);
    println!("Action: Sort by Name");

    // Example: Keyboard shortcuts
    println!("\n--- Keyboard Shortcuts ---");
    println!("I - Toggle inventory");
    println!("E - Equip selected item");
    println!("U - Use selected item");
    println!("D - Drop selected item");
    println!("Tab - Switch category");

    // Example: Hide inventory
    println!("\n--- Closing Inventory ---");
    inventory_ui.hide();
    println!("Inventory visible: {}", inventory_ui.visible);

    println!("\n================================");
    println!("Example completed successfully!");
    println!("\nTo use in your game:");
    println!("1. Create InventoryUiState");
    println!("2. Call draw() each frame with egui context");
    println!("3. Handle actions returned by the UI");
    println!("4. Process keyboard input for shortcuts");
}

/// Example of how to draw the inventory UI in your game loop
#[cfg(feature = "ui")]
fn draw_inventory_example(ctx: &egui::Context, inventory_ui: &mut InventoryUiState) {
    // Get inventory and equipment from your ECS
    // let inventory = world.query_one::<&Inventory>(player).unwrap().get().unwrap();
    // let equipment = world.query_one::<&Equipment>(player).unwrap().get().unwrap();
    // let item_db = &game_state.item_database;

    // Draw the inventory UI
    // inventory_ui.draw(ctx, inventory, equipment, item_db);
}

/// Example of handling inventory actions
fn handle_inventory_action(
    action: InventoryAction,
    inventory_ui: &mut InventoryUiState,
    world: &mut World,
    item_db: &ItemDatabase,
) {
    match inventory_ui.handle_action(action, world, item_db) {
        Ok(result) => {
            inventory_ui.last_action_result = Some(result);
            inventory_ui.action_message_timer = 2.0; // Show for 2 seconds
        }
        Err(e) => {
            inventory_ui.last_action_result = Some(
                dde_battle::ui::inventory::InventoryActionResult::Failure(e.to_string()),
            );
            inventory_ui.action_message_timer = 2.0;
        }
    }
}
