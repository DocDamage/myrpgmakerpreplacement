//! User presence and cursors

use dde_core::Entity;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Information about a connected collaborator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPresence {
    pub client_id: Uuid,
    pub username: String,
    pub color: Color32,
    pub cursor: CursorPosition,
    pub selected_entities: Vec<Entity>,
    pub viewport: Rect,
    pub status: UserStatus,
}

impl UserPresence {
    pub fn new(client_id: Uuid, username: String) -> Self {
        Self {
            client_id,
            username,
            color: generate_user_color(&client_id),
            cursor: CursorPosition::default(),
            selected_entities: Vec::new(),
            viewport: Rect::default(),
            status: UserStatus::Active,
        }
    }

    /// Update the user's status
    pub fn set_status(&mut self, status: UserStatus) {
        self.status = status;
    }

    /// Update cursor position
    pub fn set_cursor(&mut self, position: CursorPosition) {
        self.cursor = position;
    }

    /// Update selected entities
    pub fn set_selection(&mut self, entities: Vec<Entity>) {
        self.selected_entities = entities;
    }

    /// Update viewport
    pub fn set_viewport(&mut self, rect: Rect) {
        self.viewport = rect;
    }

    /// Check if user is currently editing something
    pub fn is_editing(&self) -> bool {
        !self.selected_entities.is_empty()
    }

    /// Get editing description
    pub fn editing_description(&self) -> String {
        match self.selected_entities.len() {
            0 => "Browsing".to_string(),
            1 => format!("Editing entity {:?}", self.selected_entities[0]),
            n => format!("Editing {} entities", n),
        }
    }
}

/// Cursor position on a map
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CursorPosition {
    pub map_id: u32,
    pub x: f32,
    pub y: f32,
}

impl CursorPosition {
    pub fn new(map_id: u32, x: f32, y: f32) -> Self {
        Self { map_id, x, y }
    }

    /// Get distance to another cursor position
    pub fn distance_to(&self, other: &Self) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }

    /// Check if cursor is on the same map
    pub fn is_same_map(&self, other: &Self) -> bool {
        self.map_id == other.map_id
    }

    /// Convert to screen coordinates (simplified)
    pub fn to_screen(&self, camera_x: f32, camera_y: f32, zoom: f32) -> (f32, f32) {
        (
            (self.x - camera_x) * zoom,
            (self.y - camera_y) * zoom,
        )
    }
}

impl Default for CursorPosition {
    fn default() -> Self {
        Self {
            map_id: 0,
            x: 0.0,
            y: 0.0,
        }
    }
}

/// User activity status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum UserStatus {
    Active,
    Idle,
    Away,
}

impl UserStatus {
    /// Get human-readable name
    pub fn name(&self) -> &'static str {
        match self {
            UserStatus::Active => "Active",
            UserStatus::Idle => "Idle",
            UserStatus::Away => "Away",
        }
    }

    /// Get color for this status
    pub fn color(&self) -> Color32 {
        match self {
            UserStatus::Active => Color32::GREEN,
            UserStatus::Idle => Color32::YELLOW,
            UserStatus::Away => Color32::GRAY,
        }
    }
}

/// RGBA color representation
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Color32 {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color32 {
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Predefined colors for users
    pub const RED: Self = Self::new(255, 0, 0, 255);
    pub const GREEN: Self = Self::new(0, 255, 0, 255);
    pub const BLUE: Self = Self::new(0, 0, 255, 255);
    pub const YELLOW: Self = Self::new(255, 255, 0, 255);
    pub const CYAN: Self = Self::new(0, 255, 255, 255);
    pub const MAGENTA: Self = Self::new(255, 0, 255, 255);
    pub const ORANGE: Self = Self::new(255, 165, 0, 255);
    pub const PURPLE: Self = Self::new(128, 0, 128, 255);
    pub const PINK: Self = Self::new(255, 192, 203, 255);
    pub const LIME: Self = Self::new(50, 205, 50, 255);
    pub const WHITE: Self = Self::new(255, 255, 255, 255);
    pub const BLACK: Self = Self::new(0, 0, 0, 255);
    pub const GRAY: Self = Self::new(128, 128, 128, 255);

    /// Get color as RGB array for rendering
    pub fn to_rgb(&self) -> [u8; 3] {
        [self.r, self.g, self.b]
    }

    /// Get color as RGBA array for rendering
    pub fn to_rgba(&self) -> [u8; 4] {
        [self.r, self.g, self.b, self.a]
    }

    /// Get color as a hex string
    pub fn to_hex(&self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }

    /// Create from HSL values
    pub fn from_hsl(h: f32, s: f32, l: f32) -> Self {
        let c = (1.0 - f32::abs(2.0 * l - 1.0)) * s;
        let x = c * (1.0 - ((h * 6.0) % 2.0 - 1.0).abs());
        let m = l - c / 2.0;

        let (r1, g1, b1) = match (h * 6.0) as u8 {
            0 => (c, x, 0.0),
            1 => (x, c, 0.0),
            2 => (0.0, c, x),
            3 => (0.0, x, c),
            4 => (x, 0.0, c),
            _ => (c, 0.0, x),
        };

        Self::new(
            ((r1 + m) * 255.0) as u8,
            ((g1 + m) * 255.0) as u8,
            ((b1 + m) * 255.0) as u8,
            255,
        )
    }
}

impl Default for Color32 {
    fn default() -> Self {
        Self::new(128, 128, 128, 255)
    }
}

/// Rectangle for viewport representation
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn contains(&self, point: (f32, f32)) -> bool {
        point.0 >= self.x
            && point.0 <= self.x + self.width
            && point.1 >= self.y
            && point.1 <= self.y + self.height
    }

    pub fn center(&self) -> (f32, f32) {
        (self.x + self.width / 2.0, self.y + self.height / 2.0)
    }

    pub fn intersects(&self, other: &Self) -> bool {
        self.x < other.x + other.width
            && self.x + self.width > other.x
            && self.y < other.y + other.height
            && self.y + self.height > other.y
    }

    /// Get the area of the rectangle
    pub fn area(&self) -> f32 {
        self.width * self.height
    }

    /// Check if rectangle is empty (zero area)
    pub fn is_empty(&self) -> bool {
        self.width <= 0.0 || self.height <= 0.0
    }

    /// Expand rectangle by padding on all sides
    pub fn expand(&self, padding: f32) -> Self {
        Self::new(
            self.x - padding,
            self.y - padding,
            self.width + padding * 2.0,
            self.height + padding * 2.0,
        )
    }

    /// Shrink rectangle by padding on all sides
    pub fn shrink(&self, padding: f32) -> Self {
        Self::new(
            self.x + padding,
            self.y + padding,
            (self.width - padding * 2.0).max(0.0),
            (self.height - padding * 2.0).max(0.0),
        )
    }
}

impl Default for Rect {
    fn default() -> Self {
        Self::new(0.0, 0.0, 100.0, 100.0)
    }
}

/// Generate a consistent color for a user based on their UUID
fn generate_user_color(uuid: &Uuid) -> Color32 {
    let bytes = uuid.as_bytes();
    let hue = bytes[0] as f32 / 255.0;

    // Convert HSL to RGB for vibrant colors
    let saturation = 0.8;
    let lightness = 0.5;

    Color32::from_hsl(hue, saturation, lightness)
}

/// Render information for a user cursor
pub struct CursorRenderInfo {
    pub position: CursorPosition,
    pub color: Color32,
    pub username: String,
    pub is_active: bool,
}

/// Collects all users' cursors for rendering
pub fn collect_user_cursors(users: &[UserPresence], current_user: Uuid) -> Vec<CursorRenderInfo> {
    users
        .iter()
        .filter(|u| u.client_id != current_user)
        .map(|u| CursorRenderInfo {
            position: u.cursor,
            color: u.color,
            username: u.username.clone(),
            is_active: matches!(u.status, UserStatus::Active),
        })
        .collect()
}

/// Get predefined colors for users (for consistent assignment)
pub const USER_COLORS: &[Color32] = &[
    Color32::RED,
    Color32::GREEN,
    Color32::BLUE,
    Color32::YELLOW,
    Color32::CYAN,
    Color32::MAGENTA,
    Color32::ORANGE,
    Color32::PURPLE,
    Color32::PINK,
    Color32::LIME,
];

/// Get a color for a user index (for UI assignment)
pub fn get_user_color(index: usize) -> Color32 {
    USER_COLORS[index % USER_COLORS.len()]
}

/// Get a color for a user UUID (consistent)
pub fn get_color_for_user(uuid: &Uuid) -> Color32 {
    generate_user_color(uuid)
}

/// Presence manager for tracking multiple users
#[derive(Debug, Clone)]
pub struct PresenceManager {
    users: std::collections::HashMap<Uuid, UserPresence>,
}

impl PresenceManager {
    pub fn new() -> Self {
        Self {
            users: std::collections::HashMap::new(),
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

    /// Get mutable reference to a user
    pub fn get_user_mut(&mut self, client_id: Uuid) -> Option<&mut UserPresence> {
        self.users.get_mut(&client_id)
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

    /// Update user viewport
    pub fn update_viewport(&mut self, client_id: Uuid, viewport: Rect) -> bool {
        if let Some(user) = self.users.get_mut(&client_id) {
            user.viewport = viewport;
            true
        } else {
            false
        }
    }

    /// Clear all users
    pub fn clear(&mut self) {
        self.users.clear();
    }

    /// Get the number of active users
    pub fn active_count(&self) -> usize {
        self.users
            .values()
            .filter(|u| u.status == UserStatus::Active)
            .count()
    }

    /// Get the number of users currently editing
    pub fn editing_count(&self) -> usize {
        self.users.values().filter(|u| u.is_editing()).count()
    }
}

impl Default for PresenceManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color32_new() {
        let color = Color32::new(255, 128, 64, 255);
        assert_eq!(color.r, 255);
        assert_eq!(color.g, 128);
        assert_eq!(color.b, 64);
        assert_eq!(color.a, 255);
    }

    #[test]
    fn test_color_to_rgb() {
        let color = Color32::RED;
        assert_eq!(color.to_rgb(), [255, 0, 0]);
    }

    #[test]
    fn test_color_to_hex() {
        let color = Color32::RED;
        assert_eq!(color.to_hex(), "#ff0000");
    }

    #[test]
    fn test_rect_contains() {
        let rect = Rect::new(0.0, 0.0, 100.0, 100.0);
        assert!(rect.contains((50.0, 50.0)));
        assert!(!rect.contains((150.0, 50.0)));
    }

    #[test]
    fn test_rect_intersects() {
        let rect1 = Rect::new(0.0, 0.0, 100.0, 100.0);
        let rect2 = Rect::new(50.0, 50.0, 100.0, 100.0);
        let rect3 = Rect::new(200.0, 200.0, 100.0, 100.0);

        assert!(rect1.intersects(&rect2));
        assert!(!rect1.intersects(&rect3));
    }

    #[test]
    fn test_generate_user_color() {
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();

        let color1 = generate_user_color(&uuid1);
        let color2 = generate_user_color(&uuid2);

        // Same UUID should generate same color
        assert_eq!(generate_user_color(&uuid1).to_hex(), color1.to_hex());

        // Different UUIDs likely generate different colors
        // (though collision is theoretically possible)
    }

    #[test]
    fn test_user_presence_new() {
        let client_id = Uuid::new_v4();
        let presence = UserPresence::new(client_id, "TestUser".to_string());

        assert_eq!(presence.client_id, client_id);
        assert_eq!(presence.username, "TestUser");
        assert!(matches!(presence.status, UserStatus::Active));
    }

    #[test]
    fn test_user_presence_is_editing() {
        let client_id = Uuid::new_v4();
        let mut presence = UserPresence::new(client_id, "TestUser".to_string());

        assert!(!presence.is_editing());

        presence.selected_entities = vec![Entity::DANGLING];
        assert!(presence.is_editing());
    }

    #[test]
    fn test_user_presence_editing_description() {
        let client_id = Uuid::new_v4();
        let mut presence = UserPresence::new(client_id, "TestUser".to_string());

        assert_eq!(presence.editing_description(), "Browsing");

        presence.selected_entities = vec![Entity::DANGLING];
        assert!(presence.editing_description().contains("Editing"));
    }

    #[test]
    fn test_cursor_position_distance() {
        let pos1 = CursorPosition::new(0, 0.0, 0.0);
        let pos2 = CursorPosition::new(0, 3.0, 4.0);

        assert_eq!(pos1.distance_to(&pos2), 5.0);
    }

    #[test]
    fn test_cursor_position_same_map() {
        let pos1 = CursorPosition::new(1, 0.0, 0.0);
        let pos2 = CursorPosition::new(1, 10.0, 10.0);
        let pos3 = CursorPosition::new(2, 0.0, 0.0);

        assert!(pos1.is_same_map(&pos2));
        assert!(!pos1.is_same_map(&pos3));
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
    }

    #[test]
    fn test_presence_manager_update_cursor() {
        let mut manager = PresenceManager::new();
        let presence = UserPresence::new(Uuid::new_v4(), "TestUser".to_string());
        manager.update_user(presence);

        let new_cursor = CursorPosition::new(1, 100.0, 200.0);
        assert!(manager.update_cursor(presence.client_id, new_cursor));

        let user = manager.get_user(presence.client_id).unwrap();
        assert_eq!(user.cursor.x, 100.0);
        assert_eq!(user.cursor.y, 200.0);
        assert_eq!(user.cursor.map_id, 1);
    }

    #[test]
    fn test_presence_manager_by_status() {
        let mut manager = PresenceManager::new();
        
        let mut presence1 = UserPresence::new(Uuid::new_v4(), "User1".to_string());
        presence1.status = UserStatus::Active;
        
        let mut presence2 = UserPresence::new(Uuid::new_v4(), "User2".to_string());
        presence2.status = UserStatus::Idle;
        
        let mut presence3 = UserPresence::new(Uuid::new_v4(), "User3".to_string());
        presence3.status = UserStatus::Away;

        manager.update_user(presence1);
        manager.update_user(presence2);
        manager.update_user(presence3);

        assert_eq!(manager.get_users_by_status(UserStatus::Active).len(), 1);
        assert_eq!(manager.get_users_by_status(UserStatus::Idle).len(), 1);
        assert_eq!(manager.get_users_by_status(UserStatus::Away).len(), 1);
        assert_eq!(manager.active_count(), 1);
    }

    #[test]
    fn test_presence_manager_editing_count() {
        let mut manager = PresenceManager::new();
        
        let mut presence1 = UserPresence::new(Uuid::new_v4(), "User1".to_string());
        presence1.selected_entities = vec![Entity::DANGLING];
        
        let presence2 = UserPresence::new(Uuid::new_v4(), "User2".to_string());

        manager.update_user(presence1);
        manager.update_user(presence2);

        assert_eq!(manager.editing_count(), 1);
    }

    #[test]
    fn test_get_user_color() {
        let color0 = get_user_color(0);
        let color1 = get_user_color(1);
        let color10 = get_user_color(10);

        assert_eq!(color0, Color32::RED);
        assert_eq!(color1, Color32::GREEN);
        // Index 10 wraps around to index 0
        assert_eq!(color10, Color32::RED);
    }

    #[test]
    fn test_color_from_hsl() {
        // Red at hue 0
        let red = Color32::from_hsl(0.0, 1.0, 0.5);
        assert!(red.r > 200);
        assert!(red.g < 50);
        assert!(red.b < 50);

        // Green at hue 1/3
        let green = Color32::from_hsl(1.0 / 3.0, 1.0, 0.5);
        assert!(green.r < 50);
        assert!(green.g > 200);
        assert!(green.b < 50);

        // Blue at hue 2/3
        let blue = Color32::from_hsl(2.0 / 3.0, 1.0, 0.5);
        assert!(blue.r < 50);
        assert!(blue.g < 50);
        assert!(blue.b > 200);
    }

    #[test]
    fn test_rect_area() {
        let rect = Rect::new(0.0, 0.0, 100.0, 50.0);
        assert_eq!(rect.area(), 5000.0);
    }

    #[test]
    fn test_rect_expand_shrink() {
        let rect = Rect::new(10.0, 10.0, 80.0, 80.0);
        
        let expanded = rect.expand(10.0);
        assert_eq!(expanded.x, 0.0);
        assert_eq!(expanded.y, 0.0);
        assert_eq!(expanded.width, 100.0);
        assert_eq!(expanded.height, 100.0);

        let shrunk = rect.shrink(10.0);
        assert_eq!(shrunk.x, 20.0);
        assert_eq!(shrunk.y, 20.0);
        assert_eq!(shrunk.width, 60.0);
        assert_eq!(shrunk.height, 60.0);
    }

    #[test]
    fn test_user_status() {
        assert_eq!(UserStatus::Active.name(), "Active");
        assert_eq!(UserStatus::Idle.name(), "Idle");
        assert_eq!(UserStatus::Away.name(), "Away");

        assert_eq!(UserStatus::Active.color(), Color32::GREEN);
        assert_eq!(UserStatus::Idle.color(), Color32::YELLOW);
        assert_eq!(UserStatus::Away.color(), Color32::GRAY);
    }
}
