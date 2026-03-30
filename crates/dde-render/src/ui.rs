//! UI Rendering Layer
//!
//! Egui integration for rendering debug UI, save menus, etc.

use egui_wgpu::{Renderer as EguiRenderer, ScreenDescriptor};

/// Parameters for UI rendering
pub struct RenderParams<'a> {
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
    pub encoder: &'a mut wgpu::CommandEncoder,
    pub view: &'a wgpu::TextureView,
    pub size: winit::dpi::PhysicalSize<u32>,
    pub scale_factor: f32,
}

/// UI Renderer that wraps egui
pub struct UiRenderer {
    egui_renderer: EguiRenderer,
}

impl UiRenderer {
    /// Create a new UI renderer
    pub fn new(device: &wgpu::Device, output_format: wgpu::TextureFormat) -> Self {
        let egui_renderer = EguiRenderer::new(device, output_format, None, 1);

        Self { egui_renderer }
    }

    /// Render egui to the given encoder
    pub fn render(
        &mut self,
        params: RenderParams<'_>,
        egui_ctx: &egui::Context,
        run_ui: impl FnOnce(&egui::Context),
    ) {
        // Run the UI
        let raw_input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::Vec2::new(params.size.width as f32, params.size.height as f32),
            )),
            ..Default::default()
        };

        let full_output = egui_ctx.run(raw_input, run_ui);

        let clipped_primitives =
            egui_ctx.tessellate(full_output.shapes, full_output.pixels_per_point);

        // Upload textures and render
        for (id, image_delta) in &full_output.textures_delta.set {
            self.egui_renderer
                .update_texture(params.device, params.queue, *id, image_delta);
        }

        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [params.size.width, params.size.height],
            pixels_per_point: params.scale_factor,
        };

        self.egui_renderer.update_buffers(
            params.device,
            params.queue,
            params.encoder,
            &clipped_primitives,
            &screen_descriptor,
        );

        {
            let mut render_pass = params
                .encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("UI Render Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: params.view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load, // Load the previous content (game render)
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });

            self.egui_renderer
                .render(&mut render_pass, &clipped_primitives, &screen_descriptor);
        }

        // Free textures
        for id in &full_output.textures_delta.free {
            self.egui_renderer.free_texture(id);
        }
    }
}
