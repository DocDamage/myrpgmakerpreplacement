//! Lock Manager Panel Example
//!
//! This example demonstrates how to use the Lock Manager panel for managing
//! entity locks in a collaborative editing environment.
//!
//! Run with: cargo run --example lock_manager_example

use dde_core::Entity;
use dde_editor::lock_manager::{LockManagerPanel, LockManagerExt, SortColumn, TransferStatus};
use dde_sync::lock::LockManager;
use uuid::Uuid;

fn main() {
    println!("🔒 Lock Manager Panel Example");
    println!("==============================\n");

    // Create a new lock manager panel
    let mut panel = LockManagerPanel::new();
    
    // Set up the panel
    let my_client_id = Uuid::new_v4();
    panel.set_client_id(my_client_id);
    panel.set_username("Developer".to_string());
    panel.set_admin(true);
    
    println!("✅ Lock Manager Panel created");
    println!("   Client ID: {}", my_client_id);
    println!("   Username: Developer");
    println!("   Admin: true\n");

    // Get the underlying lock manager and simulate some locks
    let lock_manager = panel.lock_manager_mut();
    
    // Simulate locking some entities
    let entity1 = Entity::from_raw(1);
    let entity2 = Entity::from_raw(2);
    let entity3 = Entity::from_raw(3);
    
    // Lock entities as different users
    lock_manager.try_lock(entity1, my_client_id, "Developer");
    lock_manager.try_lock(entity2, Uuid::new_v4(), "Alice");
    lock_manager.try_lock(entity3, Uuid::new_v4(), "Bob");
    
    println!("🔒 Simulated locks created:");
    println!("   Entity {:?} - locked by Developer (you)", entity1);
    println!("   Entity {:?} - locked by Alice", entity2);
    println!("   Entity {:?} - locked by Bob\n", entity3);

    // Show statistics
    let stats = panel.statistics();
    println!("📊 Lock Statistics:");
    println!("   Total locks: {}", stats.total_locks);
    println!("   Your locks: {}", stats.my_locks);
    println!("   Stale locks: {}", stats.stale_locks);
    println!("   Unique users: {}\n", stats.locks_per_user.len());

    // Demonstrate checking lock status
    println!("🔍 Lock Status Checks:");
    println!("   Is Entity {:?} locked? {}", entity1, panel.is_locked(entity1));
    println!("   Is Entity {:?} locked? {}", entity2, panel.is_locked(entity2));
    println!("   Is Entity 999 locked? {}\n", panel.is_locked(Entity::from_raw(999)));

    // Demonstrate getting lock info
    if let Some(info) = panel.get_lock_info(entity1) {
        println!("📋 Lock Info for Entity {:?}:", entity1);
        println!("   Locked by: {}", info.username);
        println!("   Client ID: {}", info.client_id);
    }

    // Demonstrate visual indicator colors
    println!("\n🎨 Lock Indicator Colors:");
    if let Some(color) = panel.get_lock_indicator_color(entity1) {
        println!("   Entity {:?}: RGB({}, {}, {}) - Green (yours)", 
            entity1, color.r(), color.g(), color.b());
    }
    if let Some(color) = panel.get_lock_indicator_color(entity2) {
        println!("   Entity {:?}: RGB({}, {}, {}) - User color (Alice)", 
            entity2, color.r(), color.g(), color.b());
    }

    // Demonstrate sorting options
    println!("\n📊 Available Sort Columns:");
    let columns = [
        SortColumn::EntityId,
        SortColumn::LockedBy,
        SortColumn::Timestamp,
        SortColumn::LockAge,
        SortColumn::Status,
    ];
    for col in &columns {
        println!("   - {:?}", col);
    }

    // Demonstrate transfer request
    println!("\n📨 Lock Transfer Request Example:");
    let request = dde_editor::lock_manager::LockTransferRequest {
        entity: entity2,
        from_user: "Alice".to_string(),
        from_client_id: Uuid::new_v4(),
        message: "Need to edit this entity".to_string(),
        status: TransferStatus::Pending,
    };
    println!("   Entity: {:?}", request.entity);
    println!("   From: {}", request.from_user);
    println!("   Message: {}", request.message);
    println!("   Status: {:?}", request.status);

    // Demonstrate admin operations
    println!("\n⚡ Admin Operations (available since is_admin=true):");
    println!("   - Force unlock any entity");
    println!("   - Force unlock all stale locks");
    println!("   - Clear all locks");
    println!("   - Configure stale threshold");

    // Demonstrate filtering options
    println!("\n🔍 Filter Options:");
    println!("   - Filter by entity ID or username");
    println!("   - Show stale locks only");
    println!("   - Show my locks only");
    println!("   - Auto-refresh statistics");

    // Demonstrate menu integration
    println!("\n📋 Menu Integration:");
    println!("   Add to your egui menu:");
    println!("   ui.menu_button(\"Collaboration\", |ui| {{");
    println!("       if ui.button(\"🔒 Lock Manager...\").clicked() {{");
    println!("           editor.lock_manager.toggle();");
    println!("           ui.close_menu();");
    println!("       }}");
    println!("   }});");

    // Show how to draw in the UI loop
    println!("\n🖼️  UI Integration:");
    println!("   In your main draw loop:");
    println!("   editor.lock_manager.draw(ctx);");
    println!("   editor.update_lock_manager(dt);  // Call each frame");

    // Demonstrate visual indicator usage
    println!("\n🔒 Visual Lock Indicators in Other Editors:");
    println!("   // In your entity editor:");
    println!("   let entity_rect = egui::Rect::from_min_size(pos, size);");
    println!("   editor.draw_entity_lock_indicator(ui, entity, entity_rect);");

    println!("\n✅ Example completed!");
    println!("\nThe Lock Manager panel provides:");
    println!("   - Comprehensive lock list with sorting and filtering");
    println!("   - Statistics dashboard with per-user breakdown");
    println!("   - Force unlock capabilities (admin)");
    println!("   - Lock transfer request system");
    println!("   - Visual indicators for entity lock status");
    println!("   - Stale lock detection and cleanup");
}

#[cfg(test)]
mod example_tests {
    use super::*;

    #[test]
    fn test_lock_manager_setup() {
        let mut panel = LockManagerPanel::new();
        let client_id = Uuid::new_v4();
        
        panel.set_client_id(client_id);
        panel.set_username("Test".to_string());
        panel.set_admin(true);
        
        assert!(!panel.is_visible());
    }

    #[test]
    fn test_lock_operations() {
        let mut panel = LockManagerPanel::new();
        let client_id = Uuid::new_v4();
        let entity = Entity::from_raw(1);
        
        panel.set_client_id(client_id);
        
        // Lock an entity through the underlying manager
        panel.lock_manager_mut().try_lock(entity, client_id, "Test");
        
        assert!(panel.is_locked(entity));
        assert!(panel.get_lock_info(entity).is_some());
    }
}
