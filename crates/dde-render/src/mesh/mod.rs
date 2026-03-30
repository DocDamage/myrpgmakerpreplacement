//! Mesh utilities

use bytemuck::{Pod, Zeroable};

/// 2D vertex with color and UV
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub tex_coords: [f32; 2],
    pub color: [f32; 4],
}

impl Vertex {
    pub const DESC: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &[
            wgpu::VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: wgpu::VertexFormat::Float32x3,
            },
            wgpu::VertexAttribute {
                offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                shader_location: 1,
                format: wgpu::VertexFormat::Float32x2,
            },
            wgpu::VertexAttribute {
                offset: std::mem::size_of::<[f32; 5]>() as wgpu::BufferAddress,
                shader_location: 2,
                format: wgpu::VertexFormat::Float32x4,
            },
        ],
    };

    pub fn new(position: [f32; 3], tex_coords: [f32; 2], color: [f32; 4]) -> Self {
        Self {
            position,
            tex_coords,
            color,
        }
    }
}

/// Simple quad mesh
pub struct Quad {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u16>,
}

impl Quad {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        let vertices = vec![
            Vertex::new([x, y, 0.0], [0.0, 0.0], [1.0, 1.0, 1.0, 1.0]),
            Vertex::new([x + width, y, 0.0], [1.0, 0.0], [1.0, 1.0, 1.0, 1.0]),
            Vertex::new(
                [x + width, y + height, 0.0],
                [1.0, 1.0],
                [1.0, 1.0, 1.0, 1.0],
            ),
            Vertex::new([x, y + height, 0.0], [0.0, 1.0], [1.0, 1.0, 1.0, 1.0]),
        ];

        let indices = vec![0, 1, 2, 0, 2, 3];

        Self { vertices, indices }
    }
}
