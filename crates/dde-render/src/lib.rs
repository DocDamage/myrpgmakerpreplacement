//! DocDamage Engine - Rendering Layer
//! 
//! wgpu-based rendering pipeline for 2D tile-based RPG.

use winit::window::Window;

pub mod camera;
pub mod mesh;
pub mod pipeline;
pub mod shader;
pub mod texture;

use camera::Camera;
use glam::{Mat4, Vec2, Vec3};
use mesh::{Quad, Vertex};
use pipeline::SpritePipeline;
use texture::Texture;

/// Renderer state
pub struct Renderer {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pub size: winit::dpi::PhysicalSize<u32>,
    sprite_pipeline: SpritePipeline,
    camera: Camera,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
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
        let camera = Camera::new(dde_core::components::CameraConfig::default());
        
        // Create camera uniform buffer
        let camera_uniform = CameraUniform::new();
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        
        let camera_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
            label: Some("camera_bind_group_layout"),
        });
        
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                },
            ],
            label: Some("camera_bind_group"),
        });
        
        tracing::info!("Renderer initialized: {}x{}", size.width, size.height);
        
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
        camera_uniform.update_view_proj(&self.camera, self.size.width as f32, self.size.height as f32);
        self.queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[camera_uniform]));
    }
    
    /// Render a frame
    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });
        
        // Clear render pass
        {
            let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.1,
                            b: 0.15,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        }
        
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
}

use wgpu::util::DeviceExt;
