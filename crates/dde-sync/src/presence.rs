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
}

/// Cursor position on a map
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CursorPosition {
    pub map_id: u32,
    pub x: f32,
    pub y: f32,
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
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum UserStatus {
    Active,
    Idle,
    Away,
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
        Self { x, y, width, height }
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

    Color32::new(
        ((r1 + m) * 255.0) as u8,
        ((g1 + m) * 255.0) as u8,
        ((b1 + m) * 255.0) as u8,
        255,
    )
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
}
