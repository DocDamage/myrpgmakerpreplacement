//! DocDamage Engine - Rendering Layer
//!
//! wgpu-based rendering pipeline for 2D tile-based RPG with egui support.

use winit::window::Window;

pub mod asset_hot_reload;
pub mod camera;
pub mod mesh;
pub mod pipeline;
pub mod shader;
pub mod texture;
pub mod tilemap;
pub mod ui;

pub use asset_hot_reload::AssetHotReloader;

use camera::Camera;
use glam::{Mat4, Vec2};
use pipeline::SpritePipeline;
use tilemap::TileMapRenderer;
use ui::UiRenderer;

/// Renderer state
pub struct Renderer {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    sprite_pipeline: SpritePipeline,
    camera: Camera,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    tile_map: Option<TileMapRenderer>,
    /// UI renderer for egui
    ui_renderer: UiRenderer,
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniform {
    view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    fn new() -> Self {
        Self {
            view_proj: Mat4::IDENTITY.to_cols_array_2d(),
        }
    }

    fn update_view_proj(&mut self, camera: &Camera, width: f32, height: f32) {
        let view = camera.view_matrix();
        let proj = camera.projection_matrix(width, height);
        self.view_proj = (proj * view).to_cols_array_2d();
    }
}

impl Renderer {
    /// Create a new renderer for the given window
    pub async fn new(window: Window) -> Self {
        let size = window.inner_size();

        // Instance
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        // Surface
        let surface = instance.create_surface(window).unwrap();

        // Adapter
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        // Device and queue
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    label: None,
                },
                None,
            )
            .await
            .unwrap();

        // Surface configuration
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &config);

        // Create sprite pipeline
        let sprite_pipeline = SpritePipeline::new(&device, &config);

        // Create camera
        let _camera = Camera::new(dde_core::components::CameraConfig::default());

        // Create camera uniform buffer
        let camera_uniform = CameraUniform::new();
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("camera_bind_group_layout"),
            });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
            label: Some("camera_bind_group"),
        });

        // Create test tile map (64x64 grid, 32px tiles)
        let tile_map = TileMapRenderer::new(&device, 64, 64, 32.0);

        // Center camera on tile map
        let mut camera = Camera::new(dde_core::components::CameraConfig::default());
        let world_size = tile_map.world_size();
        camera.set_target(Vec2::new(world_size.x / 2.0, world_size.y / 2.0));

        // Create UI renderer
        let ui_renderer = UiRenderer::new(&device, config.format);

        tracing::info!("Renderer initialized: {}x{}", size.width, size.height);
        tracing::info!("Tile map: {}x{} tiles", 64, 64);

        Self {
            surface,
            device,
            queue,
            config,
            size,
            sprite_pipeline,
            camera,
            camera_buffer,
            camera_bind_group,
            tile_map: Some(tile_map),
            ui_renderer,
        }
    }

    /// Resize the renderer
    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    /// Update camera
    pub fn update_camera(&mut self, target: Vec2, dt: f32) {
        self.camera.set_target(target);
        self.camera.update(dt);

        // Update uniform buffer
        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update_view_proj(
            &self.camera,
            self.size.width as f32,
            self.size.height as f32,
        );
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[camera_uniform]),
        );
    }

    /// Render a frame
    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        // Main render pass
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.05,
                            g: 0.05,
                            b: 0.08,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            // Set pipeline and bind groups
            render_pass.set_pipeline(&self.sprite_pipeline.pipeline);
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);

            // Render tile map
            if let Some(ref tile_map) = self.tile_map {
                tile_map.render(&mut render_pass);
            }
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    /// Render a frame with UI
    pub fn render_with_ui(
        &mut self,
        ctx: &egui::Context,
        scale_factor: f32,
        run_ui: impl FnOnce(&egui::Context),
    ) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        // Main render pass
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.05,
                            g: 0.05,
                            b: 0.08,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            // Set pipeline and bind groups
            render_pass.set_pipeline(&self.sprite_pipeline.pipeline);
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);

            // Render tile map
            if let Some(ref tile_map) = self.tile_map {
                tile_map.render(&mut render_pass);
            }
        }

        // UI render pass
        self.ui_renderer.render(
            crate::ui::RenderParams {
                device: &self.device,
                queue: &self.queue,
                encoder: &mut encoder,
                view: &view,
                size: self.size,
                scale_factor,
            },
            ctx,
            run_ui,
        );

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    /// Get device reference
    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    /// Get queue reference
    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    /// Get size
    pub fn size(&self) -> winit::dpi::PhysicalSize<u32> {
        self.size
    }

    /// Get camera reference
    pub fn camera(&self) -> &Camera {
        &self.camera
    }

    /// Get camera mutable reference
    pub fn camera_mut(&mut self) -> &mut Camera {
        &mut self.camera
    }

    /// Get tile map world size
    pub fn world_size(&self) -> Option<Vec2> {
        self.tile_map.as_ref().map(|tm| tm.world_size())
    }
}

use wgpu::util::DeviceExt;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod unit_tests {
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
    fn test_quad_creation() {
        // Test Quad::new creates correct vertices and indices
        let quad = Quad::new(0.0, 0.0, 32.0, 32.0);

        // Should have 4 vertices
        assert_eq!(quad.vertices.len(), 4);

        // Should have 6 indices (2 triangles)
        assert_eq!(quad.indices.len(), 6);

        // Check correct triangle winding
        assert_eq!(quad.indices, vec![0, 1, 2, 0, 2, 3]);
    }
}
