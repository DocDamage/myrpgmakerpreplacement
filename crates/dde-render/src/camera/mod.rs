//! Camera system

use dde_core::components::CameraConfig;
use glam::{Mat4, Vec2};

/// Camera controller
pub struct Camera {
    pub position: Vec2,
    pub target: Vec2,
    pub zoom: f32,
    pub config: CameraConfig,
}

impl Camera {
    pub fn new(config: CameraConfig) -> Self {
        Self {
            position: Vec2::ZERO,
            target: Vec2::ZERO,
            zoom: config.zoom,
            config,
        }
    }

    /// Update camera to follow target
    pub fn update(&mut self, dt: f32) {
        // Smooth follow with exponential decay
        let speed = self.config.follow_speed * dt;
        self.position = self.position.lerp(self.target, speed);
    }

    /// Set target position
    pub fn set_target(&mut self, target: Vec2) {
        self.target = target;
    }

    /// Build view matrix
    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(
            Vec3::new(self.position.x, self.position.y, 1.0),
            Vec3::new(self.position.x, self.position.y, 0.0),
            Vec3::Y,
        )
    }

    /// Build projection matrix (orthographic)
    pub fn projection_matrix(&self, width: f32, height: f32) -> Mat4 {
        let half_width = (width / 2.0) / self.zoom;
        let half_height = (height / 2.0) / self.zoom;

        Mat4::orthographic_rh(
            -half_width,
            half_width,
            -half_height,
            half_height,
            0.1,
            100.0,
        )
    }

    /// Convert screen position to world position
    pub fn screen_to_world(&self, screen: Vec2, window_size: Vec2) -> Vec2 {
        let normalized = (screen / window_size) * 2.0 - Vec2::ONE;

        self.position
            + normalized
                * Vec2::new(
                    window_size.x / (2.0 * self.zoom),
                    -window_size.y / (2.0 * self.zoom),
                )
    }
}

use glam::Vec3;
