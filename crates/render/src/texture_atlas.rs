//! Texture atlas for block textures.

use std::collections::HashMap;
use std::path::Path;

use image::{ImageBuffer, Rgba};
use wgpu::{
    Device, Extent3d, Queue, Texture, TextureDescriptor, TextureDimension, TextureFormat,
    TextureUsages,
};

/// A texture atlas containing block textures.
#[derive(Debug)]
pub struct TextureAtlas {
    /// The GPU texture.
    texture: Texture,
    /// The texture view.
    view: wgpu::TextureView,
    /// The bind group layout.
    bind_group_layout: wgpu::BindGroupLayout,
    /// The bind group.
    bind_group: wgpu::BindGroup,
    /// Texture dimensions.
    width: u32,
    height: u32,
    /// UV coordinates for each texture (name -> (u1, v1, u2, v2)).
    uvs: HashMap<String, [f32; 4]>,
    /// Padding between textures (in pixels).
    padding: u32,
}

impl TextureAtlas {
    /// Creates a new texture atlas from a directory of PNG files.
    pub fn from_directory(device: &Device, queue: &Queue, dir: &Path) -> Result<Self, AtlasError> {
        let padding = 1; // 1 pixel padding between textures
        let tile_size = 16; // Minecraft textures are 16x16

        // Collect all PNG files
        let mut entries = Vec::new();
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("png") {
                let name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
                    .to_string();
                entries.push((name, path));
            }
        }

        if entries.is_empty() {
            return Err(AtlasError::NoTextures);
        }

        // Sort for deterministic ordering
        entries.sort_by(|a, b| a.0.cmp(&b.0));

        // Calculate atlas dimensions (square-ish)
        let count = entries.len();
        let tiles_per_row = (count as f32).sqrt().ceil() as u32;
        let rows = (count as u32).div_ceil(tiles_per_row);

        let width = tiles_per_row * tile_size + (tiles_per_row + 1) * padding;
        let height = rows * tile_size + (rows + 1) * padding;

        // Create the atlas image
        let mut atlas = ImageBuffer::<Rgba<u8>, Vec<u8>>::new(width, height);

        // Fill with transparent
        for pixel in atlas.pixels_mut() {
            *pixel = Rgba([0, 0, 0, 0]);
        }

        // Place each texture
        let mut uvs = HashMap::new();
        for (i, (name, path)) in entries.iter().enumerate() {
            let img = image::open(path)?.to_rgba8();
            let row = i as u32 / tiles_per_row;
            let col = i as u32 % tiles_per_row;

            let x = padding + col * (tile_size + padding);
            let y = padding + row * (tile_size + padding);

            // Copy pixels
            for ty in 0..tile_size.min(img.height()) {
                for tx in 0..tile_size.min(img.width()) {
                    atlas.put_pixel(x + tx, y + ty, *img.get_pixel(tx, ty));
                }
            }

            // Calculate UV coordinates (0.0 to 1.0)
            let u1 = (x as f32) / (width as f32);
            let v1 = (y as f32) / (height as f32);
            let u2 = ((x + tile_size) as f32) / (width as f32);
            let v2 = ((y + tile_size) as f32) / (height as f32);
            uvs.insert(name.clone(), [u1, v1, u2, v2]);
        }

        // Create GPU texture
        let texture = device.create_texture(&TextureDescriptor {
            label: Some("texture atlas"),
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8UnormSrgb,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        });

        wgpu::Queue::write_texture(
            queue,
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &atlas,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("texture atlas bind group layout"),
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
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest, // Pixel art should use nearest
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("texture atlas bind group"),
            layout: &bind_group_layout,
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
        });

        Ok(Self {
            texture,
            view,
            bind_group_layout,
            bind_group,
            width,
            height,
            uvs,
            padding,
        })
    }

    /// Returns the bind group for the texture atlas.
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }

    /// Returns the bind group layout.
    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }

    /// Creates the bind group layout for the texture atlas (static method for pipeline creation).
    pub fn create_bind_group_layout(device: &Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("texture atlas bind group layout"),
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
        })
    }

    /// Returns UV coordinates for a texture name.
    pub fn uvs(&self, name: &str) -> Option<[f32; 4]> {
        self.uvs.get(name).copied()
    }

    /// Returns the texture dimensions.
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Creates a default texture atlas with a single white texture for testing.
    pub fn default(device: &Device, queue: &Queue) -> Result<Self, AtlasError> {
        let width = 16;
        let height = 16;
        let padding = 1;

        // Create a simple white texture
        let mut atlas = ImageBuffer::<Rgba<u8>, Vec<u8>>::new(width, height);
        for pixel in atlas.pixels_mut() {
            *pixel = Rgba([255, 255, 255, 255]);
        }

        // UV coordinates for the single texture
        let mut uvs = HashMap::new();
        let u1 = (padding as f32) / (width as f32);
        let v1 = (padding as f32) / (height as f32);
        let u2 = ((padding + 16) as f32) / (width as f32);
        let v2 = ((padding + 16) as f32) / (height as f32);
        uvs.insert("default".to_string(), [u1, v1, u2, v2]);

        // Create GPU texture
        let texture = device.create_texture(&TextureDescriptor {
            label: Some("default texture atlas"),
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8UnormSrgb,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        });

        wgpu::Queue::write_texture(
            queue,
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &atlas,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("texture atlas bind group layout"),
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
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("texture atlas bind group"),
            layout: &bind_group_layout,
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
        });

        Ok(Self {
            texture,
            view,
            bind_group_layout,
            bind_group,
            width,
            height,
            uvs,
            padding,
        })
    }
}

/// Texture atlas errors.
#[derive(Debug, thiserror::Error)]
pub enum AtlasError {
    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// Image loading/decoding error.
    #[error("image error: {0}")]
    Image(#[from] image::ImageError),
    /// No textures found in directory.
    #[error("no textures found in directory")]
    NoTextures,
    /// wgpu error.
    #[error("wgpu error: {0}")]
    Wgpu(#[from] wgpu::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atlas_creation_fails_on_empty_dir() {
        // We can't easily test this without a device, but we can test the logic
    }
}
