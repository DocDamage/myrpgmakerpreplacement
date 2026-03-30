//! Screenshot Management for Save Slots
//!
//! Provides thumbnail screenshot capture, compression, and storage
//! for the save slot system. Supports PNG, JPEG, and WebP formats.

use image::{DynamicImage, ImageBuffer, ImageEncoder, ImageFormat, Rgba};
use std::io::Cursor;

use crate::{Database, DbError, Result};

/// Screenshot format options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenshotFormat {
    /// Lossless PNG format - larger file size, perfect quality
    Png,
    /// Lossy JPEG format - smaller file size, good for photos
    Jpeg,
    /// Modern WebP format - best compression, good quality
    Webp,
}

impl ScreenshotFormat {
    /// Get the MIME type for this format
    pub fn mime_type(&self) -> &'static str {
        match self {
            ScreenshotFormat::Png => "image/png",
            ScreenshotFormat::Jpeg => "image/jpeg",
            ScreenshotFormat::Webp => "image/webp",
        }
    }

    /// Get the file extension for this format
    pub fn extension(&self) -> &'static str {
        match self {
            ScreenshotFormat::Png => "png",
            ScreenshotFormat::Jpeg => "jpg",
            ScreenshotFormat::Webp => "webp",
        }
    }
}

/// Configuration for screenshot capture and compression
#[derive(Debug, Clone)]
pub struct ScreenshotConfig {
    /// Target width for the screenshot
    pub width: u32,
    /// Target height for the screenshot
    pub height: u32,
    /// Output format
    pub format: ScreenshotFormat,
    /// Quality setting (0-100), used for JPEG and WebP
    pub quality: u8,
}

impl Default for ScreenshotConfig {
    fn default() -> Self {
        Self {
            width: 320,
            height: 180,
            format: ScreenshotFormat::Webp,
            quality: 85,
        }
    }
}

/// Compressed screenshot data ready for storage
#[derive(Debug, Clone)]
pub struct ScreenshotData {
    /// Raw compressed image bytes
    pub data: Vec<u8>,
    /// Width of the image
    pub width: u32,
    /// Height of the image
    pub height: u32,
    /// Format of the compressed data
    pub format: ScreenshotFormat,
}

impl ScreenshotData {
    /// Get the size of the compressed data in bytes
    pub fn size_bytes(&self) -> usize {
        self.data.len()
    }

    /// Get the size in KB as a formatted string
    pub fn size_kb(&self) -> f64 {
        self.data.len() as f64 / 1024.0
    }
}

/// Manager for capturing and processing screenshots
pub struct ScreenshotManager {
    config: ScreenshotConfig,
}

impl ScreenshotManager {
    /// Create a new screenshot manager with default configuration
    /// (320x180, WebP format, quality 85)
    pub fn new() -> Self {
        Self {
            config: ScreenshotConfig::default(),
        }
    }

    /// Create a new screenshot manager with custom configuration
    pub fn with_config(config: ScreenshotConfig) -> Self {
        Self { config }
    }

    /// Get the current configuration
    pub fn config(&self) -> &ScreenshotConfig {
        &self.config
    }

    /// Capture a screenshot from raw RGBA pixel data
    ///
    /// # Arguments
    /// * `pixels` - Raw RGBA bytes (4 bytes per pixel: R, G, B, A)
    /// * `width` - Width of the source image
    /// * `height` - Height of the source image
    ///
    /// # Errors
    /// Returns an error if the pixel data length doesn't match dimensions
    pub fn capture_from_rgba(
        &self,
        pixels: &[u8],
        width: u32,
        height: u32,
    ) -> Result<ScreenshotData> {
        let expected_len = (width * height * 4) as usize;
        if pixels.len() != expected_len {
            return Err(DbError::InvalidData(format!(
                "Pixel data length mismatch: expected {}, got {}",
                expected_len,
                pixels.len()
            )));
        }

        // Create image buffer from raw pixels
        let image: ImageBuffer<Rgba<u8>, _> = ImageBuffer::from_raw(width, height, pixels.to_vec())
            .ok_or_else(|| {
                DbError::InvalidData("Failed to create image buffer from raw pixels".to_string())
            })?;

        let dynamic_image = DynamicImage::ImageRgba8(image);
        self.compress(dynamic_image)
    }

    /// Compress an image to the configured format and size
    ///
    /// Resizes the image to the target dimensions and compresses it
    /// according to the format settings.
    pub fn compress(&self, image: DynamicImage) -> Result<ScreenshotData> {
        // Resize to target dimensions using Lanczos3 filter for quality
        let resized = image.resize_exact(
            self.config.width,
            self.config.height,
            image::imageops::FilterType::Lanczos3,
        );

        let mut buffer = Cursor::new(Vec::new());

        match self.config.format {
            ScreenshotFormat::Png => {
                // PNG uses lossless compression, quality setting is ignored
                resized
                    .write_to(&mut buffer, ImageFormat::Png)
                    .map_err(|e| DbError::InvalidData(format!("PNG encoding failed: {}", e)))?;
            }
            ScreenshotFormat::Jpeg => {
                // Convert to RGB for JPEG (no alpha channel)
                let rgb_image = resized.to_rgb8();

                // JPEG quality: clamp to 0-100
                let quality = self.config.quality.clamp(0, 100);

                // Use jpeg encoder directly for quality control
                let mut encoder =
                    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buffer, quality);
                encoder
                    .encode_image(&rgb_image)
                    .map_err(|e| DbError::InvalidData(format!("JPEG encoding failed: {}", e)))?;
            }
            ScreenshotFormat::Webp => {
                // Convert to RGB for WebP (better compression without alpha)
                let rgb_image = resized.to_rgb8();

                // Encode to WebP lossless for now
                // Note: image crate's WebP encoder doesn't expose quality control directly
                let encoder = image::codecs::webp::WebPEncoder::new_lossless(&mut buffer);
                encoder
                    .encode(
                        rgb_image.as_raw(),
                        self.config.width,
                        self.config.height,
                        image::ExtendedColorType::Rgb8,
                    )
                    .map_err(|e| DbError::InvalidData(format!("WebP encoding failed: {}", e)))?;
            }
        }

        Ok(ScreenshotData {
            data: buffer.into_inner(),
            width: self.config.width,
            height: self.config.height,
            format: self.config.format,
        })
    }

    /// Save a screenshot to a database slot
    ///
    /// # Arguments
    /// * `db` - Database connection
    /// * `slot` - Slot number (1-99)
    /// * `screenshot` - Screenshot data to save
    pub fn save_to_slot(
        &self,
        db: &Database,
        slot: u32,
        screenshot: &ScreenshotData,
    ) -> Result<()> {
        db.save_screenshot_to_slot(slot, screenshot)
    }

    /// Load a screenshot from a database slot
    ///
    /// # Arguments
    /// * `db` - Database connection
    /// * `slot` - Slot number (1-99)
    ///
    /// # Returns
    /// * `Ok(Some(ScreenshotData))` - Screenshot found and loaded
    /// * `Ok(None)` - Slot exists but has no screenshot
    pub fn load_from_slot(&self, db: &Database, slot: u32) -> Result<Option<ScreenshotData>> {
        db.load_screenshot_from_slot(slot)
    }

    /// Generate a placeholder screenshot with a gradient pattern
    ///
    /// Creates a visually distinctive placeholder based on the slot number.
    /// Uses a gradient from one corner to another with slot-specific colors.
    ///
    /// # Arguments
    /// * `slot` - Slot number to generate unique colors for
    pub fn generate_placeholder(&self, slot: u32) -> ScreenshotData {
        let width = self.config.width;
        let height = self.config.height;

        // Generate colors based on slot number for variety
        let hue_start = ((slot * 137) % 360) as f32 / 360.0; // Golden angle for distribution
        let hue_end = (hue_start + 0.3) % 1.0;

        let color_start = hsl_to_rgba(hue_start, 0.6, 0.4);
        let color_end = hsl_to_rgba(hue_end, 0.6, 0.6);

        let mut pixels = Vec::with_capacity((width * height * 4) as usize);

        for y in 0..height {
            for x in 0..width {
                // Calculate gradient factor (diagonal)
                let t = ((x as f32 / width as f32) + (y as f32 / height as f32)) / 2.0;

                // Add a subtle pattern based on slot
                let pattern = if (x + y + slot) % 32 < 16 { 0.95 } else { 1.0 };

                // Interpolate colors
                let r =
                    ((color_start[0] as f32 * (1.0 - t) + color_end[0] as f32 * t) * pattern) as u8;
                let g =
                    ((color_start[1] as f32 * (1.0 - t) + color_end[1] as f32 * t) * pattern) as u8;
                let b =
                    ((color_start[2] as f32 * (1.0 - t) + color_end[2] as f32 * t) * pattern) as u8;
                let a = 255;

                pixels.push(r);
                pixels.push(g);
                pixels.push(b);
                pixels.push(a);
            }
        }

        // Create image and compress it
        let image: ImageBuffer<Rgba<u8>, _> =
            ImageBuffer::from_raw(width, height, pixels.clone()).expect("Valid dimensions");
        let dynamic_image = DynamicImage::ImageRgba8(image);

        // Always compress placeholders - use current config but ensure reasonable quality
        self.compress(dynamic_image).unwrap_or_else(|_| {
            // Generate a minimal PNG as fallback
            let mut png_buffer = std::io::Cursor::new(Vec::new());
            let _ = image::codecs::png::PngEncoder::new(&mut png_buffer).write_image(
                &pixels,
                width,
                height,
                image::ExtendedColorType::Rgba8,
            );
            ScreenshotData {
                data: png_buffer.into_inner(),
                width,
                height,
                format: ScreenshotFormat::Png,
            }
        })
    }
}

impl Default for ScreenshotManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for capturing screenshots from a renderer
///
/// Implement this trait for the renderer to enable screenshot capture
/// from the GPU frame buffer.
pub trait ScreenshotCapture {
    /// Capture the current frame as RGBA bytes
    ///
    /// # Returns
    /// Raw RGBA pixel data (4 bytes per pixel)
    fn capture_frame(&self) -> Result<Vec<u8>>;
}

/// Generate a placeholder screenshot with default settings
///
/// This is a convenience function for when no renderer is available
/// or when screenshot capture fails.
pub fn capture_placeholder(slot: u32) -> ScreenshotData {
    let manager = ScreenshotManager::new();
    manager.generate_placeholder(slot)
}

/// Load a screenshot as an egui texture handle
///
/// # Arguments
/// * `screenshot` - The screenshot data to load
/// * `ctx` - The egui context
/// * `texture_id` - Unique identifier for the texture
///
/// # Returns
/// * `Some(TextureHandle)` - If the image can be decoded
/// * `None` - If decoding fails
#[cfg(feature = "egui")]
pub fn load_as_egui_texture(
    screenshot: &ScreenshotData,
    ctx: &egui::Context,
    texture_id: impl Into<String>,
) -> Option<egui::TextureHandle> {
    use egui::ColorImage;

    // Decode the image based on format
    let dynamic_image = match screenshot.format {
        ScreenshotFormat::Png => {
            image::load_from_memory_with_format(&screenshot.data, ImageFormat::Png).ok()?
        }
        ScreenshotFormat::Jpeg => {
            image::load_from_memory_with_format(&screenshot.data, ImageFormat::Jpeg).ok()?
        }
        ScreenshotFormat::Webp => {
            image::load_from_memory_with_format(&screenshot.data, ImageFormat::WebP).ok()?
        }
    };

    // Convert to RGBA8
    let rgba = dynamic_image.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    let pixels = rgba.into_raw();

    // Create egui color image
    let color_image = ColorImage::from_rgba_unmultiplied(size, &pixels);

    // Load into egui
    Some(ctx.load_texture(texture_id, color_image, egui::TextureOptions::LINEAR))
}

/// Helper function to convert HSL to RGBA
fn hsl_to_rgba(h: f32, s: f32, l: f32) -> [u8; 4] {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h * 6.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;

    let (r1, g1, b1) = match (h * 6.0) as i32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };

    [
        ((r1 + m) * 255.0) as u8,
        ((g1 + m) * 255.0) as u8,
        ((b1 + m) * 255.0) as u8,
        255,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_db_path() -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!("dde_screenshot_test_{}.db", uuid::Uuid::new_v4()));
        path
    }

    #[test]
    fn test_screenshot_config_default() {
        let config = ScreenshotConfig::default();
        assert_eq!(config.width, 320);
        assert_eq!(config.height, 180);
        assert!(matches!(config.format, ScreenshotFormat::Webp));
        assert_eq!(config.quality, 85);
    }

    #[test]
    fn test_manager_new() {
        let manager = ScreenshotManager::new();
        assert_eq!(manager.config().width, 320);
        assert_eq!(manager.config().height, 180);
    }

    #[test]
    fn test_manager_with_config() {
        let config = ScreenshotConfig {
            width: 640,
            height: 360,
            format: ScreenshotFormat::Png,
            quality: 90,
        };
        let manager = ScreenshotManager::with_config(config.clone());
        assert_eq!(manager.config().width, 640);
        assert_eq!(manager.config().height, 360);
        assert!(matches!(manager.config().format, ScreenshotFormat::Png));
        assert_eq!(manager.config().quality, 90);
    }

    #[test]
    fn test_capture_from_rgba() {
        let manager = ScreenshotManager::new();
        let width = 320;
        let height = 180;
        let pixels = vec![128u8; (width * height * 4) as usize];

        let result = manager.capture_from_rgba(&pixels, width, height);
        assert!(result.is_ok());

        let screenshot = result.unwrap();
        assert_eq!(screenshot.width, 320);
        assert_eq!(screenshot.height, 180);
        assert!(!screenshot.data.is_empty());
    }

    #[test]
    fn test_capture_from_rgba_wrong_size() {
        let manager = ScreenshotManager::new();
        let pixels = vec![128u8; 100]; // Wrong size

        let result = manager.capture_from_rgba(&pixels, 320, 180);
        assert!(result.is_err());
    }

    #[test]
    fn test_compress_png() {
        let config = ScreenshotConfig {
            width: 320,
            height: 180,
            format: ScreenshotFormat::Png,
            quality: 100,
        };
        let manager = ScreenshotManager::with_config(config);

        // Create a simple gradient image
        let mut img = ImageBuffer::new(640, 360);
        for (x, _, pixel) in img.enumerate_pixels_mut() {
            let value = (x % 256) as u8;
            *pixel = Rgba([value, value, value, 255]);
        }
        let dynamic = DynamicImage::ImageRgba8(img);

        let result = manager.compress(dynamic);
        assert!(result.is_ok());

        let screenshot = result.unwrap();
        assert_eq!(screenshot.width, 320);
        assert_eq!(screenshot.height, 180);
        assert!(matches!(screenshot.format, ScreenshotFormat::Png));
        assert!(!screenshot.data.is_empty());
    }

    #[test]
    fn test_compress_jpeg() {
        let config = ScreenshotConfig {
            width: 320,
            height: 180,
            format: ScreenshotFormat::Jpeg,
            quality: 85,
        };
        let manager = ScreenshotManager::with_config(config);

        let mut img = ImageBuffer::new(640, 360);
        for (x, _, pixel) in img.enumerate_pixels_mut() {
            let value = (x % 256) as u8;
            *pixel = Rgba([value, value, value, 255]);
        }
        let dynamic = DynamicImage::ImageRgba8(img);

        let result = manager.compress(dynamic);
        assert!(result.is_ok());

        let screenshot = result.unwrap();
        assert_eq!(screenshot.width, 320);
        assert_eq!(screenshot.height, 180);
        assert!(matches!(screenshot.format, ScreenshotFormat::Jpeg));
        assert!(!screenshot.data.is_empty());
    }

    #[test]
    fn test_generate_placeholder() {
        let manager = ScreenshotManager::new();

        // Generate placeholders for different slots
        let placeholder1 = manager.generate_placeholder(1);
        let placeholder2 = manager.generate_placeholder(2);
        let placeholder3 = manager.generate_placeholder(99);

        // All should have correct dimensions
        assert_eq!(placeholder1.width, 320);
        assert_eq!(placeholder1.height, 180);
        assert_eq!(placeholder2.width, 320);
        assert_eq!(placeholder2.height, 180);
        assert_eq!(placeholder3.width, 320);
        assert_eq!(placeholder3.height, 180);

        // Data should not be empty
        assert!(!placeholder1.data.is_empty());
        assert!(!placeholder2.data.is_empty());
        assert!(!placeholder3.data.is_empty());

        // Different slots should produce different data
        assert_ne!(placeholder1.data, placeholder2.data);
    }

    #[test]
    fn test_screenshot_data_size() {
        let screenshot = ScreenshotData {
            data: vec![0u8; 10240], // 10 KB
            width: 320,
            height: 180,
            format: ScreenshotFormat::Webp,
        };

        assert_eq!(screenshot.size_bytes(), 10240);
        assert!((screenshot.size_kb() - 10.0).abs() < 0.01);
    }

    #[test]
    fn test_screenshot_format_helpers() {
        assert_eq!(ScreenshotFormat::Png.mime_type(), "image/png");
        assert_eq!(ScreenshotFormat::Jpeg.mime_type(), "image/jpeg");
        assert_eq!(ScreenshotFormat::Webp.mime_type(), "image/webp");

        assert_eq!(ScreenshotFormat::Png.extension(), "png");
        assert_eq!(ScreenshotFormat::Jpeg.extension(), "jpg");
        assert_eq!(ScreenshotFormat::Webp.extension(), "webp");
    }

    #[test]
    fn test_placeholder_varies_by_slot() {
        let manager = ScreenshotManager::new();

        // Generate multiple placeholders
        let p1 = manager.generate_placeholder(1);
        let p2 = manager.generate_placeholder(2);
        let p10 = manager.generate_placeholder(10);

        // All should be valid images (non-empty)
        assert!(!p1.data.is_empty());
        assert!(!p2.data.is_empty());
        assert!(!p10.data.is_empty());

        // Different slots should have different visual data
        // Note: This is probabilistic but highly unlikely to fail
        assert_ne!(p1.data, p2.data);
        assert_ne!(p1.data, p10.data);
        assert_ne!(p2.data, p10.data);
    }

    #[test]
    fn test_capture_placeholder_function() {
        let placeholder = capture_placeholder(42);

        assert_eq!(placeholder.width, 320);
        assert_eq!(placeholder.height, 180);
        assert!(!placeholder.data.is_empty());
    }

    #[test]
    fn test_database_save_and_load_screenshot() {
        use crate::Database;

        let path = test_db_path();
        let db = Database::create_new(&path, "Screenshot Test Project").unwrap();

        // First create the slot (saves the database file)
        db.save_to_slot(1).unwrap();

        // Create a simple screenshot
        let manager = ScreenshotManager::new();
        let placeholder = manager.generate_placeholder(1);

        // Save screenshot to existing slot
        let result = manager.save_to_slot(&db, 1, &placeholder);
        assert!(result.is_ok());

        // Load from slot
        let loaded = manager.load_from_slot(&db, 1).unwrap();
        assert!(loaded.is_some());

        let loaded_screenshot = loaded.unwrap();
        assert_eq!(loaded_screenshot.width, placeholder.width);
        assert_eq!(loaded_screenshot.height, placeholder.height);
        assert_eq!(loaded_screenshot.format as i32, placeholder.format as i32);
        // Data might differ slightly due to re-encoding, but should be similar size
        let size_diff =
            (loaded_screenshot.size_bytes() as i64 - placeholder.size_bytes() as i64).abs();
        assert!(
            size_diff < 1000,
            "Screenshot data size changed significantly after save/load"
        );

        // Cleanup
        drop(db);
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(format!("{}.slot01.dde", path.display()));
    }

    #[test]
    fn test_database_load_nonexistent_screenshot() {
        use crate::Database;

        let path = test_db_path();
        let db = Database::create_new(&path, "Screenshot Test Project").unwrap();

        let manager = ScreenshotManager::new();

        // Try to load from a slot that doesn't have a screenshot
        let loaded = manager.load_from_slot(&db, 99).unwrap();
        assert!(loaded.is_none());

        // Cleanup
        drop(db);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_save_to_slot_with_screenshot() {
        use crate::Database;

        let path = test_db_path();
        let db = Database::create_new(&path, "Screenshot Test Project").unwrap();

        // Create a screenshot
        let manager = ScreenshotManager::new();
        let screenshot = manager.generate_placeholder(1);

        // Save slot with screenshot
        let result = db.save_to_slot_with_screenshot(1, 12345678, Some(&screenshot));
        assert!(result.is_ok());

        // Get slot info
        let slot_info = db.get_slot_info(1).unwrap();
        assert!(slot_info.is_some());

        let info = slot_info.unwrap();
        assert_eq!(info.slot_number, 1);
        assert_eq!(info.play_time_ms, 12345678);
        assert!(info.exists);
        assert!(info.has_screenshot);

        // Cleanup
        drop(db);
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(format!("{}.slot01.dde", path.display()));
    }

    #[test]
    fn test_compression_target_size() {
        // Test that screenshots are under 50KB target
        let manager = ScreenshotManager::new();

        // Generate placeholder
        let screenshot = manager.generate_placeholder(1);

        // Should be under 50KB
        assert!(
            screenshot.size_kb() < 50.0,
            "Screenshot size {}KB exceeds 50KB target",
            screenshot.size_kb()
        );

        // Create a more complex test image
        let mut pixels = Vec::with_capacity((320 * 180 * 4) as usize);
        for y in 0..180u32 {
            for x in 0..320u32 {
                // Create a pattern with lots of variation
                let r = ((x * 255) / 320) as u8;
                let g = ((y * 255) / 180) as u8;
                let b = (((x + y) * 255) / 500) as u8;
                pixels.push(r);
                pixels.push(g);
                pixels.push(b);
                pixels.push(255);
            }
        }

        let complex_screenshot = manager.capture_from_rgba(&pixels, 320, 180).unwrap();

        // Even complex images should be reasonable size
        assert!(
            complex_screenshot.size_kb() < 100.0,
            "Complex screenshot size {}KB exceeds 100KB",
            complex_screenshot.size_kb()
        );
    }
}
