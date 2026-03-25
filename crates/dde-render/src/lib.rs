//! DocDamage Engine - Rendering Layer
//! 
//! wgpu-based rendering pipeline for 2D tile-based RPG.

pub mod camera;
pub mod mesh;
pub mod pipeline;
pub mod shader;
pub mod texture;

/// Renderer placeholder
pub struct Renderer;

impl Renderer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}
