//! Mesh module tests
//!
//! Tests for Vertex and Quad primitives

use crate::mesh::{Quad, Vertex};

#[test]
fn test_vertex_creation() {
    // Test Vertex::new with all parameters
    let vertex = Vertex::new([1.0, 2.0, 3.0], [0.5, 0.5], [1.0, 0.0, 0.0, 1.0]);

    assert_eq!(vertex.position, [1.0, 2.0, 3.0]);
    assert_eq!(vertex.tex_coords, [0.5, 0.5]);
    assert_eq!(vertex.color, [1.0, 0.0, 0.0, 1.0]);
}

#[test]
fn test_vertex_creation_various_values() {
    // Test with zero values
    let zero_vertex = Vertex::new([0.0, 0.0, 0.0], [0.0, 0.0], [0.0, 0.0, 0.0, 0.0]);
    assert_eq!(zero_vertex.position, [0.0, 0.0, 0.0]);
    assert_eq!(zero_vertex.tex_coords, [0.0, 0.0]);
    assert_eq!(zero_vertex.color, [0.0, 0.0, 0.0, 0.0]);

    // Test with negative values
    let neg_vertex = Vertex::new([-1.0, -2.0, -3.0], [-0.5, -0.5], [0.5, 0.5, 0.5, 0.5]);
    assert_eq!(neg_vertex.position, [-1.0, -2.0, -3.0]);
    assert_eq!(neg_vertex.tex_coords, [-0.5, -0.5]);
    assert_eq!(neg_vertex.color, [0.5, 0.5, 0.5, 0.5]);
}

#[test]
fn test_vertex_buffer_layout() {
    // Verify vertex buffer layout is correctly defined
    let layout = Vertex::DESC;

    // Check array stride matches size of Vertex struct
    assert_eq!(layout.array_stride, std::mem::size_of::<Vertex>() as u64);

    // Check step mode is Vertex
    assert!(matches!(layout.step_mode, wgpu::VertexStepMode::Vertex));

    // Should have 3 attributes: position (3 floats), tex_coords (2 floats), color (4 floats)
    assert_eq!(layout.attributes.len(), 3);
}

#[test]
fn test_quad_creation() {
    // Test Quad::new creates correct vertices
    let quad = Quad::new(10.0, 20.0, 100.0, 50.0);

    // Should have 4 vertices
    assert_eq!(quad.vertices.len(), 4);

    // Should have 6 indices (2 triangles)
    assert_eq!(quad.indices.len(), 6);
    assert_eq!(quad.indices, vec![0, 1, 2, 0, 2, 3]);

    // Check vertex positions (counter-clockwise winding)
    // Bottom-left
    assert_eq!(quad.vertices[0].position, [10.0, 20.0, 0.0]);
    assert_eq!(quad.vertices[0].tex_coords, [0.0, 0.0]);

    // Bottom-right
    assert_eq!(quad.vertices[1].position, [110.0, 20.0, 0.0]);
    assert_eq!(quad.vertices[1].tex_coords, [1.0, 0.0]);

    // Top-right
    assert_eq!(quad.vertices[2].position, [110.0, 70.0, 0.0]);
    assert_eq!(quad.vertices[2].tex_coords, [1.0, 1.0]);

    // Top-left
    assert_eq!(quad.vertices[3].position, [10.0, 70.0, 0.0]);
    assert_eq!(quad.vertices[3].tex_coords, [0.0, 1.0]);
}

#[test]
fn test_quad_creation_at_origin() {
    let quad = Quad::new(0.0, 0.0, 32.0, 32.0);

    assert_eq!(quad.vertices[0].position, [0.0, 0.0, 0.0]);
    assert_eq!(quad.vertices[1].position, [32.0, 0.0, 0.0]);
    assert_eq!(quad.vertices[2].position, [32.0, 32.0, 0.0]);
    assert_eq!(quad.vertices[3].position, [0.0, 32.0, 0.0]);
}

#[test]
fn test_quad_vertex_colors() {
    // Quads should have white vertices by default (for tinting in shader)
    let quad = Quad::new(0.0, 0.0, 10.0, 10.0);

    for vertex in &quad.vertices {
        assert_eq!(vertex.color, [1.0, 1.0, 1.0, 1.0]);
    }
}

#[test]
fn test_vertex_pod_zerocopy() {
    // Test that Vertex is properly marked as Pod and Zeroable
    use bytemuck::{Pod, Zeroable};

    fn assert_pod<T: Pod>() {}
    fn assert_zerocopy<T: Zeroable>() {}

    assert_pod::<Vertex>();
    assert_zerocopy::<Vertex>();
}

#[test]
fn test_vertex_copy_clone() {
    let vertex = Vertex::new([1.0, 2.0, 3.0], [0.5, 0.5], [1.0, 1.0, 1.0, 1.0]);

    // Test Copy
    let copied = vertex;
    assert_eq!(copied.position, vertex.position);

    // Test Clone
    let cloned = vertex.clone();
    assert_eq!(cloned.position, vertex.position);
    assert_eq!(cloned.tex_coords, vertex.tex_coords);
    assert_eq!(cloned.color, vertex.color);
}

#[test]
fn test_vertex_debug() {
    let vertex = Vertex::new([1.0, 2.0, 3.0], [0.5, 0.5], [1.0, 0.0, 0.0, 1.0]);
    let debug_str = format!("{:?}", vertex);

    // Debug should contain position info
    assert!(debug_str.contains("position"));
    assert!(debug_str.contains("tex_coords"));
    assert!(debug_str.contains("color"));
}
