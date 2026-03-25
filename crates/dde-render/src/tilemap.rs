//! Tile map rendering

use glam::{Vec2, Vec3};
use wgpu::util::DeviceExt;

use crate::mesh::Vertex;
use crate::Renderer;

/// Tile map renderer
pub struct TileMapRenderer {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
    tile_size: f32,
    grid_size: (u32, u32),
}

impl TileMapRenderer {
    /// Create a tile map renderer for a grid
    pub fn new(device: &wgpu::Device, grid_width: u32, grid_height: u32, tile_size: f32) -> Self {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        
        // Generate a simple grid of quads
        for y in 0..grid_height {
            for x in 0..grid_width {
                let x_pos = x as f32 * tile_size;
                let y_pos = y as f32 * tile_size;
                let base_index = vertices.len() as u16;
                
                // Create quad vertices (top-down view for now)
                vertices.extend_from_slice(&[
                    Vertex::new(x_pos, y_pos, 0.0, 0.0, 0.0, 0.2, 0.25, 0.3, 1.0), // Top-left
                    Vertex::new(x_pos + tile_size, y_pos, 0.0, 1.0, 0.0, 0.2, 0.25, 0.3, 1.0), // Top-right
                    Vertex::new(x_pos + tile_size, y_pos + tile_size, 0.0, 1.0, 1.0, 0.2, 0.25, 0.3, 1.0), // Bottom-right
                    Vertex::new(x_pos, y_pos + tile_size, 0.0, 0.0, 1.0, 0.2, 0.25, 0.3, 1.0), // Bottom-left
                ]);
                
                // Add indices for two triangles
                indices.extend_from_slice(&[
                    base_index, base_index + 1, base_index + 2,
                    base_index, base_index + 2, base_index + 3,
                ]);
            }
        }
        
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Tile Map Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Tile Map Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        
        Self {
            vertex_buffer,
            index_buffer,
            num_indices: indices.len() as u32,
            tile_size,
            grid_size: (grid_width, grid_height),
        }
    }
    
    /// Render the tile map
    pub fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
    }
    
    /// Get grid dimensions
    pub fn grid_size(&self) -> (u32, u32) {
        self.grid_size
    }
    
    /// Get tile size
    pub fn tile_size(&self) -> f32 {
        self.tile_size
    }
    
    /// Get total size in world units
    pub fn world_size(&self) -> Vec2 {
        Vec2::new(
            self.grid_size.0 as f32 * self.tile_size,
            self.grid_size.1 as f32 * self.tile_size,
        )
    }
}

/// Simple colored tile for testing
pub struct TestTileMap;

impl TestTileMap {
    pub fn generate(renderer: &Renderer) -> TileMapRenderer {
        TileMapRenderer::new(renderer.device(), 64, 64, 32.0)
    }
}
