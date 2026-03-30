//! Texture management with hot-reload support

use image::GenericImageView;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

/// Texture with hot-reload support
#[derive(Debug)]
pub struct Texture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub bind_group: wgpu::BindGroup,
    pub path: PathBuf,
    pub last_modified: SystemTime,
    pub dimensions: (u32, u32),
}

impl Texture {
    /// Load texture from file with hot-reload tracking
    pub fn from_file(
        path: &Path,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Option<Self> {
        let img = image::open(path).ok()?;
        let rgba = img.to_rgba8();
        let dimensions = img.dimensions();

        let texture_size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            label: Some(&format!("texture_{}", path.display())),
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::ImageCopyTexture {
                aspect: wgpu::TextureAspect::All,
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            &rgba,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * dimensions.0),
                rows_per_image: Some(dimensions.1),
            },
            texture_size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
            label: Some(&format!("bind_group_{}", path.display())),
        });

        let last_modified = std::fs::metadata(path)
            .and_then(|m| m.modified())
            .unwrap_or_else(|_| SystemTime::now());

        Some(Self {
            texture,
            view,
            sampler,
            bind_group,
            path: path.to_path_buf(),
            last_modified,
            dimensions,
        })
    }

    /// Hot-swap texture data while preserving bind group
    pub fn hot_swap(&mut self, new_image: image::DynamicImage, queue: &wgpu::Queue) {
        let rgba = new_image.to_rgba8();
        let new_dimensions = new_image.dimensions();

        // If dimensions changed, we need to recreate the texture
        if new_dimensions != self.dimensions {
            tracing::warn!(
                "Texture dimensions changed from {:?} to {:?}, recreation required",
                self.dimensions,
                new_dimensions
            );
            // Note: Full recreation would require device access
            // For now, just update the data if dimensions match
            return;
        }

        queue.write_texture(
            wgpu::ImageCopyTexture {
                aspect: wgpu::TextureAspect::All,
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            &rgba,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * self.dimensions.0),
                rows_per_image: Some(self.dimensions.1),
            },
            wgpu::Extent3d {
                width: self.dimensions.0,
                height: self.dimensions.1,
                depth_or_array_layers: 1,
            },
        );

        self.last_modified = SystemTime::now();
        tracing::info!("Hot-swapped texture: {:?}", self.path);
    }
}

/// Texture manager with hot-reload support
pub struct TextureManager {
    textures: Arc<Mutex<HashMap<PathBuf, Texture>>>,
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    bind_group_layout: wgpu::BindGroupLayout,
}

impl TextureManager {
    /// Create a new texture manager
    pub fn new(device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>) -> Self {
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: Some("texture_bind_group_layout"),
        });

        Self {
            textures: Arc::new(Mutex::new(HashMap::new())),
            device,
            queue,
            bind_group_layout,
        }
    }

    /// Load a texture from file
    pub fn load(&self, path: &Path) -> Option<TextureHandle> {
        let texture = Texture::from_file(path, &self.device, &self.queue, &self.bind_group_layout)?;

        let mut textures = self.textures.lock().ok()?;
        textures.insert(path.to_path_buf(), texture);

        Some(TextureHandle {
            path: path.to_path_buf(),
        })
    }

    /// Hot-reload a texture
    pub fn hot_reload(&self, path: &Path) -> bool {
        let Ok(img) = image::open(path) else {
            tracing::error!("Failed to open image for hot-reload: {:?}", path);
            return false;
        };

        let mut textures = match self.textures.lock() {
            Ok(t) => t,
            Err(_) => return false,
        };

        if let Some(texture) = textures.get_mut(path) {
            texture.hot_swap(img, &self.queue);
            true
        } else {
            // Texture not loaded yet, load it
            drop(textures);
            self.load(path).is_some()
        }
    }

    /// Check if a texture exists at path
    pub fn contains(&self, path: &Path) -> bool {
        let textures = match self.textures.lock() {
            Ok(t) => t,
            Err(_) => return false,
        };
        textures.contains_key(path)
    }

    /// Get texture dimensions
    pub fn get_dimensions(&self, path: &Path) -> Option<(u32, u32)> {
        let textures = self.textures.lock().ok()?;
        textures.get(path).map(|t| t.dimensions)
    }

    /// Get texture last modified time
    pub fn get_last_modified(&self, path: &Path) -> Option<SystemTime> {
        let textures = self.textures.lock().ok()?;
        textures.get(path).map(|t| t.last_modified)
    }

    /// Get bind group for a texture
    /// Note: Returns the bind group id/handle which can be used with the render pipeline
    pub fn with_bind_group<F, R>(&self, path: &Path, f: F) -> Option<R>
    where
        F: FnOnce(&wgpu::BindGroup) -> R,
    {
        let textures = self.textures.lock().ok()?;
        textures.get(path).map(|t| f(&t.bind_group))
    }

    /// Remove a texture
    pub fn remove(&self, path: &Path) -> bool {
        let mut textures = match self.textures.lock() {
            Ok(t) => t,
            Err(_) => return false,
        };
        textures.remove(path).is_some()
    }

    /// Get all loaded texture paths
    pub fn loaded_paths(&self) -> Vec<PathBuf> {
        let textures = match self.textures.lock() {
            Ok(t) => t,
            Err(_) => return vec![],
        };
        textures.keys().cloned().collect()
    }

    /// Get count of loaded textures
    pub fn count(&self) -> usize {
        self.textures.lock().map(|t| t.len()).unwrap_or(0)
    }

    /// Clear all textures
    pub fn clear(&self) {
        if let Ok(mut textures) = self.textures.lock() {
            textures.clear();
        }
    }
}

/// Handle to a loaded texture
#[derive(Debug, Clone)]
pub struct TextureHandle {
    pub path: PathBuf,
}

impl TextureHandle {
    /// Get the texture path
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Error type for texture operations
#[derive(Debug, thiserror::Error)]
pub enum TextureError {
    #[error("Texture not found: {0}")]
    NotFound(PathBuf),
    #[error("Failed to load image: {0}")]
    ImageLoad(#[from] image::ImageError),
    #[error("Lock poisoned")]
    LockPoisoned,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_texture_handle() {
        let handle = TextureHandle {
            path: PathBuf::from("test.png"),
        };
        assert_eq!(handle.path().to_str(), Some("test.png"));
    }
}
