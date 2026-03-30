//! Real-time preview for cutscene timeline
//!
//! Renders the cutscene at the current playhead position for live editing feedback.

use super::editor::TimelineEditor;
use super::keyframes::{CameraValue, EffectType, TrackValue};
use dde_core::{Direction4, Entity, World};
use glam::{Mat4, Vec3};

/// Renders the cutscene at current playhead position
#[derive(Debug, Clone)]
pub struct PreviewRenderer {
    /// Camera override for preview
    pub preview_camera: PreviewCamera,
    /// Whether to show debug overlays
    pub show_debug: bool,
    /// Whether to show safe frame guides
    pub show_safe_frames: bool,
    /// Whether to show rule of thirds guides
    pub show_rule_of_thirds: bool,
    /// Whether to show track info overlay
    pub show_track_info: bool,
    /// Render quality (0.0-1.0)
    pub quality: f32,
    /// Last rendered frame data (for export)
    last_frame: Option<FrameData>,
}

/// Camera state for preview
#[derive(Debug, Clone)]
pub struct PreviewCamera {
    pub position: Vec3,
    pub zoom: f32,
    pub rotation: f32,
    pub shake_amount: f32,
    pub fade_alpha: f32,
}

impl Default for PreviewCamera {
    fn default() -> Self {
        Self {
            position: Vec3::new(0.0, 0.0, 10.0),
            zoom: 1.0,
            rotation: 0.0,
            shake_amount: 0.0,
            fade_alpha: 0.0,
        }
    }
}

impl PreviewCamera {
    /// Create view matrix for this camera
    pub fn view_matrix(&self) -> Mat4 {
        let shake_offset = if self.shake_amount > 0.0 {
            // Random shake based on amount
            let seed = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as f32;
            let dx = (seed * 12.9898).sin() * self.shake_amount;
            let dy = (seed * 78.233).sin() * self.shake_amount;
            Vec3::new(dx, dy, 0.0)
        } else {
            Vec3::ZERO
        };

        let position = self.position + shake_offset;
        
        Mat4::look_at_rh(
            position,
            position - Vec3::Z, // Look down -Z
            Vec3::Y,
        )
    }

    /// Create projection matrix
    pub fn projection_matrix(&self, aspect_ratio: f32) -> Mat4 {
        let base_zoom = self.zoom;
        let fov = 45.0f32.to_radians() / base_zoom;
        Mat4::perspective_rh(fov, aspect_ratio, 0.1, 1000.0)
    }

    /// Get the combined view-projection matrix
    pub fn view_projection_matrix(&self, aspect_ratio: f32) -> Mat4 {
        self.projection_matrix(aspect_ratio) * self.view_matrix()
    }

    /// Apply rotation around Z axis
    pub fn apply_rotation(&mut self) {
        if self.rotation != 0.0 {
            let rotation_rad = self.rotation.to_radians();
            let cos_r = rotation_rad.cos();
            let sin_r = rotation_rad.sin();
            
            let x = self.position.x;
            let y = self.position.y;
            
            self.position.x = x * cos_r - y * sin_r;
            self.position.y = x * sin_r + y * cos_r;
        }
    }
}

/// Data for a single rendered frame
#[derive(Debug, Clone)]
pub struct FrameData {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>, // RGBA8
}

impl FrameData {
    /// Create frame data from dimensions
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            pixels: vec![0; (width * height * 4) as usize],
        }
    }

    /// Save as PNG
    pub fn save_png(&self, path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
        // This would use an image library in real implementation
        // For now, just a placeholder
        let _ = path;
        Ok(())
    }
}

/// Effect state during preview
#[derive(Debug, Clone)]
pub struct PreviewEffectState {
    pub effect_type: EffectType,
    pub intensity: f32,
    pub color: [f32; 4],
}

impl Default for PreviewEffectState {
    fn default() -> Self {
        Self {
            effect_type: EffectType::None,
            intensity: 0.0,
            color: [1.0, 1.0, 1.0, 1.0],
        }
    }
}

/// Entity state during preview
#[derive(Debug, Clone)]
pub struct PreviewEntityState {
    pub position: Vec3,
    pub animation_id: Option<u32>,
    pub visible: bool,
    pub direction: Direction4,
}

impl PreviewRenderer {
    /// Create a new preview renderer
    pub fn new() -> Self {
        Self {
            preview_camera: PreviewCamera::default(),
            show_debug: false,
            show_safe_frames: false,
            show_rule_of_thirds: false,
            show_track_info: true,
            quality: 1.0,
            last_frame: None,
        }
    }

    /// Render one frame of the cutscene at the current playhead position
    pub fn render(&mut self, timeline: &TimelineEditor, _world: &World) -> PreviewFrame {
        let time = timeline.playhead;
        
        // Sample all tracks at current time
        let mut camera_value: Option<CameraValue> = None;
        let mut effect_state = PreviewEffectState::default();
        let mut entity_states: std::collections::HashMap<Entity, PreviewEntityState> = 
            std::collections::HashMap::new();

        for track in &timeline.tracks {
            if track.muted {
                continue;
            }

            if let Some(value) = track.value_at(time) {
                match value {
                    TrackValue::Camera(cam) => {
                        camera_value = Some(cam);
                    }
                    TrackValue::Entity(ent) => {
                        if let Some(entity) = track.target {
                            entity_states.insert(entity, PreviewEntityState {
                                position: ent.position.into(),
                                animation_id: ent.animation_id,
                                visible: ent.visible,
                                direction: ent.direction,
                            });
                        }
                    }
                    TrackValue::Effect(eff) => {
                        effect_state = PreviewEffectState {
                            effect_type: eff.effect_type,
                            intensity: eff.intensity,
                            color: eff.color,
                        };
                    }
                    TrackValue::Audio(_) => {
                        // Audio is handled separately (played back)
                    }
                    TrackValue::Dialogue(_) => {
                        // Dialogue is handled by UI overlay
                    }
                }
            }
        }

        // Update preview camera
        if let Some(cam) = camera_value {
            self.preview_camera.position = cam.position.into();
            self.preview_camera.zoom = cam.zoom;
            self.preview_camera.rotation = cam.rotation;
            self.preview_camera.shake_amount = cam.shake_amount;
            self.preview_camera.fade_alpha = cam.fade_alpha;
        }

        // Build preview frame
        PreviewFrame {
            camera: self.preview_camera.clone(),
            effect: effect_state,
            entities: entity_states,
            time,
            debug_info: if self.show_debug {
                Some(self.build_debug_info(timeline))
            } else {
                None
            },
        }
    }

    /// Scrub to specific time (for dragging playhead)
    pub fn scrub(&mut self, time: f32, timeline: &TimelineEditor) -> PreviewFrame {
        // Create a temporary timeline with the scrubbed time
        let mut scrub_timeline = timeline.clone();
        scrub_timeline.playhead = time;
        self.render(&scrub_timeline, &World::new())
    }

    /// Build debug info for display
    fn build_debug_info(&self, timeline: &TimelineEditor) -> DebugInfo {
        DebugInfo {
            playhead: timeline.playhead,
            duration: timeline.duration,
            active_tracks: timeline.tracks.iter().filter(|t| !t.muted).count(),
            total_tracks: timeline.tracks.len(),
            camera_position: self.preview_camera.position,
            camera_zoom: self.preview_camera.zoom,
        }
    }

    /// Toggle debug overlay
    pub fn toggle_debug(&mut self) {
        self.show_debug = !self.show_debug;
    }

    /// Toggle safe frame guides
    pub fn toggle_safe_frames(&mut self) {
        self.show_safe_frames = !self.show_safe_frames;
    }

    /// Toggle rule of thirds guides
    pub fn toggle_rule_of_thirds(&mut self) {
        self.show_rule_of_thirds = !self.show_rule_of_thirds;
    }

    /// Set render quality
    pub fn set_quality(&mut self, quality: f32) {
        self.quality = quality.clamp(0.1, 1.0);
    }

    /// Capture current frame as image data
    pub fn capture_frame(&self) -> Option<FrameData> {
        self.last_frame.clone()
    }

    /// Export current view to PNG sequence for video export
    pub fn export_frame(&self, path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(frame) = &self.last_frame {
            frame.save_png(path)?;
        }
        Ok(())
    }

    /// Reset camera to default
    pub fn reset_camera(&mut self) {
        self.preview_camera = PreviewCamera::default();
    }

    /// Set camera to match world camera
    pub fn sync_with_world_camera(&mut self, world: &World) {
        // In a real implementation, this would query the world's camera component
        let _ = world;
        // For now, just reset
        self.reset_camera();
    }
}

impl Default for PreviewRenderer {
    fn default() -> Self {
        Self::new()
    }
}

/// A rendered preview frame
#[derive(Debug, Clone)]
pub struct PreviewFrame {
    /// Camera state used for this frame
    pub camera: PreviewCamera,
    /// Current effect state
    pub effect: PreviewEffectState,
    /// Entity states
    pub entities: std::collections::HashMap<Entity, PreviewEntityState>,
    /// Current time
    pub time: f32,
    /// Debug information
    pub debug_info: Option<DebugInfo>,
}

/// Debug information for preview
#[derive(Debug, Clone)]
pub struct DebugInfo {
    pub playhead: f32,
    pub duration: f32,
    pub active_tracks: usize,
    pub total_tracks: usize,
    pub camera_position: Vec3,
    pub camera_zoom: f32,
}

/// Video exporter for PNG sequence export
pub struct VideoExporter {
    pub output_dir: std::path::PathBuf,
    pub frame_rate: f32,
    pub width: u32,
    pub height: u32,
    pub frame_count: u32,
}

impl VideoExporter {
    /// Create new video exporter
    pub fn new(output_dir: impl Into<std::path::PathBuf>) -> Self {
        Self {
            output_dir: output_dir.into(),
            frame_rate: 30.0,
            width: 1920,
            height: 1080,
            frame_count: 0,
        }
    }

    /// Set frame rate
    pub fn with_frame_rate(mut self, fps: f32) -> Self {
        self.frame_rate = fps;
        self
    }

    /// Set resolution
    pub fn with_resolution(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Export timeline to PNG sequence
    pub fn export(&mut self, timeline: &TimelineEditor, world: &World) -> Result<u32, Box<dyn std::error::Error>> {
        std::fs::create_dir_all(&self.output_dir)?;

        let mut renderer = PreviewRenderer::new();
        let duration = timeline.duration;
        let frame_time = 1.0 / self.frame_rate;
        let total_frames = (duration * self.frame_rate) as u32;

        for frame in 0..total_frames {
            let time = frame as f32 * frame_time;
            let mut frame_timeline = timeline.clone();
            frame_timeline.playhead = time;
            
            renderer.render(&frame_timeline, world);
            
            if let Some(frame_data) = renderer.capture_frame() {
                let filename = format!("frame_{:06}.png", frame);
                let path = self.output_dir.join(filename);
                frame_data.save_png(&path)?;
            }
        }

        self.frame_count = total_frames;
        Ok(total_frames)
    }

    /// Get FFmpeg command to convert PNG sequence to video
    pub fn ffmpeg_command(&self, output_video: &str) -> String {
        format!(
            "ffmpeg -framerate {} -i {}/frame_%06d.png -c:v libx264 -pix_fmt yuv420p {}",
            self.frame_rate,
            self.output_dir.display(),
            output_video
        )
    }
}

/// Preview playback controller
pub struct PreviewPlayback {
    /// Target frame rate for preview
    pub target_fps: f32,
    /// Whether to skip frames to maintain timing
    pub skip_frames: bool,
    /// Playback speed multiplier
    pub speed: f32,
    /// Whether to play audio during preview
    pub play_audio: bool,
}

impl PreviewPlayback {
    /// Create default playback settings
    pub fn new() -> Self {
        Self {
            target_fps: 30.0,
            skip_frames: true,
            speed: 1.0,
            play_audio: true,
        }
    }

    /// Set playback speed
    pub fn with_speed(mut self, speed: f32) -> Self {
        self.speed = speed.clamp(0.1, 4.0);
        self
    }

    /// Calculate actual delta time for preview
    pub fn calculate_dt(&self, real_dt: f32) -> f32 {
        real_dt * self.speed
    }
}

impl Default for PreviewPlayback {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preview_camera_defaults() {
        let camera = PreviewCamera::default();
        assert_eq!(camera.zoom, 1.0);
        assert_eq!(camera.rotation, 0.0);
    }

    #[test]
    fn test_preview_camera_view_matrix() {
        let camera = PreviewCamera::default();
        let matrix = camera.view_matrix();
        // View matrix should be valid (not NaN)
        assert!(!matrix.x_axis.x.is_nan());
    }

    #[test]
    fn test_preview_renderer_new() {
        let renderer = PreviewRenderer::new();
        assert!(!renderer.show_debug);
        assert_eq!(renderer.quality, 1.0);
    }

    #[test]
    fn test_video_exporter_settings() {
        let exporter = VideoExporter::new("/tmp/export")
            .with_frame_rate(60.0)
            .with_resolution(1920, 1080);
        
        assert_eq!(exporter.frame_rate, 60.0);
        assert_eq!(exporter.width, 1920);
        assert_eq!(exporter.height, 1080);
    }

    #[test]
    fn test_preview_playback_speed() {
        let playback = PreviewPlayback::new().with_speed(2.0);
        assert_eq!(playback.speed, 2.0);
        
        let dt = playback.calculate_dt(0.016);
        assert!((dt - 0.032).abs() < 0.001);
    }
}
