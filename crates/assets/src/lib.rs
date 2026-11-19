//! Asset loading: virtual filesystem layering, content/resource packs, atlas building, localization.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use image::{DynamicImage, ImageBuffer, Rgba};
use mclre_core::Identifier;
use thiserror::Error;

/// Errors from asset loading.
#[derive(Debug, Error)]
pub enum AssetError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Image error: {0}")]
    Image(#[from] image::ImageError),
    #[error("Asset not found: {0}")]
    NotFound(String),
    #[error("Invalid asset path: {0}")]
    InvalidPath(String),
    #[error("Texture atlas error: {0}")]
    Atlas(String),
}

/// Result type for asset operations.
pub type Result<T> = std::result::Result<T, AssetError>;

/// A texture in the asset system.
#[derive(Debug, Clone)]
pub struct Texture {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>, // RGBA8
}

impl Texture {
    /// Creates a texture from an image buffer.
    pub fn from_image(img: ImageBuffer<Rgba<u8>, Vec<u8>>) -> Self {
        let (width, height) = img.dimensions();
        Self {
            width,
            height,
            data: img.into_raw(),
        }
    }

    /// Creates a texture from raw RGBA data.
    pub fn from_rgba(width: u32, height: u32, data: Vec<u8>) -> Self {
        Self {
            width,
            height,
            data,
        }
    }

    /// Returns the raw RGBA data.
    pub fn as_rgba(&self) -> &[u8] {
        &self.data
    }

    /// Returns the texture dimensions.
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}

/// A texture atlas for batching multiple textures.
#[derive(Debug)]
pub struct TextureAtlas {
    texture: Texture,
    regions: HashMap<Identifier, AtlasRegion>,
    width: u32,
    height: u32,
}

/// A region within a texture atlas.
#[derive(Debug, Clone, Copy)]
pub struct AtlasRegion {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub u_min: f32,
    pub v_min: f32,
    pub u_max: f32,
    pub v_max: f32,
}

impl TextureAtlas {
    /// Creates a new empty texture atlas.
    pub fn new(width: u32, height: u32) -> Self {
        let texture = Texture::from_rgba(width, height, vec![0; (width * height * 4) as usize]);
        Self {
            texture,
            regions: HashMap::new(),
            width,
            height,
        }
    }

    /// Adds a texture to the atlas.
    pub fn add_texture(&mut self, id: Identifier, texture: &Texture) -> Result<AtlasRegion> {
        // Simple row-based packing for now
        let tex_width = texture.width;
        let tex_height = texture.height;

        if tex_width > self.width || tex_height > self.height {
            return Err(AssetError::Atlas("Texture too large for atlas".into()));
        }

        // Find a place for the texture (simple row-based packing)
        let mut x = 0;
        let mut y = 0;
        let mut row_height = 0;

        // Find first available spot (simplified)
        for existing in self.regions.values() {
            if existing.y == y && existing.x + existing.width + tex_width <= self.width {
                x = existing.x + existing.width;
            } else if existing.y + existing.height > y {
                y = existing.y + existing.height;
                x = 0;
                row_height = 0;
            }
            row_height = row_height.max(existing.height);
        }

        if x + tex_width > self.width {
            x = 0;
            y += row_height;
            row_height = tex_height;
        }

        if y + tex_height > self.height {
            return Err(AssetError::Atlas("Atlas full".into()));
        }

        // Copy texture data into atlas
        self.copy_texture(texture, x, y);

        let region = AtlasRegion {
            x,
            y,
            width: tex_width,
            height: tex_height,
            u_min: x as f32 / self.width as f32,
            v_min: y as f32 / self.height as f32,
            u_max: (x + tex_width) as f32 / self.width as f32,
            v_max: (y + tex_height) as f32 / self.height as f32,
        };

        self.regions.insert(id, region);
        Ok(region)
    }

    /// Copies texture data into the atlas at the given position.
    fn copy_texture(&mut self, texture: &Texture, x: u32, y: u32) {
        let src = texture.as_rgba();
        let dst = &mut self.texture.data;

        for ty in 0..texture.height {
            for tx in 0..texture.width {
                let src_idx = (((ty * texture.width) + tx) * 4) as usize;
                let dst_idx = (((y + ty) * self.width) + (x + tx)) * 4;

                dst[dst_idx as usize] = src[src_idx];
                dst[dst_idx as usize + 1] = src[src_idx + 1];
                dst[dst_idx as usize + 2] = src[src_idx + 2];
                dst[dst_idx as usize + 3] = src[src_idx + 3];
            }
        }
    }

    /// Gets a region by identifier.
    pub fn get_region(&self, id: &Identifier) -> Option<AtlasRegion> {
        self.regions.get(id).copied()
    }

    /// Returns the atlas texture.
    pub fn texture(&self) -> &Texture {
        &self.texture
    }
}

/// Virtual filesystem for assets with layering support.
#[derive(Debug, Default)]
pub struct AssetVfs {
    roots: Vec<PathBuf>,
    overrides: HashMap<String, Vec<u8>>, // In-memory overrides
}

impl AssetVfs {
    /// Creates a new virtual filesystem.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a root directory to search.
    pub fn add_root(&mut self, path: PathBuf) {
        self.roots.push(path);
    }

    /// Adds an in-memory override.
    pub fn add_override(&mut self, path: String, data: Vec<u8>) {
        self.overrides.insert(path, data);
    }

    /// Reads a file from the virtual filesystem.
    pub fn read_file(&self, path: &str) -> Result<Vec<u8>> {
        // Check overrides first
        if let Some(data) = self.overrides.get(path) {
            return Ok(data.clone());
        }

        // Search roots
        for root in &self.roots {
            let full_path = root.join(path);
            if full_path.exists() {
                return std::fs::read(full_path).map_err(AssetError::Io);
            }
        }

        Err(AssetError::NotFound(path.to_string()))
    }

    /// Checks if a file exists.
    pub fn exists(&self, path: &str) -> bool {
        if self.overrides.contains_key(path) {
            return true;
        }
        for root in &self.roots {
            if root.join(path).exists() {
                return true;
            }
        }
        false
    }

    /// Lists files under a directory.
    pub fn list_dir(&self, path: &str) -> Result<Vec<String>> {
        let mut files = Vec::new();

        for root in &self.roots {
            let full_path = root.join(path);
            if full_path.exists() && full_path.is_dir() {
                for entry in std::fs::read_dir(full_path).map_err(AssetError::Io)? {
                    let entry = entry.map_err(AssetError::Io)?;
                    let name = entry.file_name().to_string_lossy().to_string();
                    if !files.contains(&name) {
                        files.push(name);
                    }
                }
            }
        }

        // Add overrides
        for key in self.overrides.keys() {
            if key.starts_with(path) {
                let rest = &key[path.len()..];
                if !rest.contains('/') && !files.contains(&rest.to_string()) {
                    files.push(rest.to_string());
                }
            }
        }

        files.sort();
        files.dedup();
        Ok(files)
    }
}

/// Asset manager for loading and caching assets.
pub struct AssetManager {
    vfs: AssetVfs,
    texture_cache: HashMap<Identifier, Arc<Texture>>,
    atlas: TextureAtlas,
}

impl AssetManager {
    /// Creates a new asset manager.
    pub fn new() -> Self {
        Self {
            vfs: AssetVfs::new(),
            texture_cache: HashMap::new(),
            atlas: TextureAtlas::new(2048, 2048), // Default 2048x2048 atlas
        }
    }

    /// Adds a root directory to the VFS.
    pub fn add_root(&mut self, path: PathBuf) {
        self.vfs.add_root(path);
    }

    /// Loads a texture from the VFS.
    pub fn load_texture(&mut self, id: Identifier, path: &str) -> Result<Arc<Texture>> {
        if let Some(texture) = self.texture_cache.get(&id) {
            return Ok(texture.clone());
        }

        let data = self.vfs.read_file(path)?;
        let img = image::load_from_memory(&data)
            .map_err(AssetError::Image)?
            .to_rgba8();
        let texture = Texture::from_image(img);
        let texture = Arc::new(texture);
        self.texture_cache.insert(id, texture.clone());
        Ok(texture)
    }

    /// Loads a texture into the atlas.
    pub fn load_into_atlas(&mut self, id: Identifier, path: &str) -> Result<AtlasRegion> {
        let texture = self.load_texture(id, path)?;
        self.atlas.add_texture(id, &texture)
    }

    /// Gets a texture region from the atlas.
    pub fn get_atlas_region(&self, id: &Identifier) -> Option<AtlasRegion> {
        self.atlas.get_region(id)
    }

    /// Gets the atlas texture.
    pub fn atlas_texture(&self) -> &Texture {
        self.atlas.texture()
    }

    /// Gets a cached texture.
    pub fn get_texture(&self, id: &Identifier) -> Option<Arc<Texture>> {
        self.texture_cache.get(id).cloned()
    }
}

impl Default for AssetManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn vfs_reads_files() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("test.txt");
        std::fs::write(&file_path, b"hello").unwrap();

        let mut vfs = AssetVfs::new();
        vfs.add_root(temp.path().to_path_buf());

        let data = vfs.read_file("test.txt").unwrap();
        assert_eq!(data, b"hello");
    }

    #[test]
    fn vfs_overrides_work() {
        let mut vfs = AssetVfs::new();
        vfs.add_override("test.txt".to_string(), b"override".to_vec());

        let data = vfs.read_file("test.txt").unwrap();
        assert_eq!(data, b"override");
    }

    #[test]
    fn texture_atlas_adds_textures() {
        let mut atlas = TextureAtlas::new(256, 256);
        let id = Identifier::new("minecraft", "test").expect("valid id");
        let texture = Texture::from_rgba(16, 16, vec![255; 16 * 16 * 4]);
        let region = atlas.add_texture(id.clone(), &texture).unwrap();

        assert_eq!(region.width, 16);
        assert_eq!(region.height, 16);
        assert!(atlas.get_region(&id).is_some());
    }

    #[test]
    fn asset_manager_loads_textures() {
        let temp = TempDir::new().unwrap();
        let img = ImageBuffer::from_fn(16, 16, |_, _| Rgba([255u8, 0u8, 0u8, 255u8]));
        img.save(temp.path().join("test.png")).unwrap();

        let mut manager = AssetManager::new();
        manager.add_root(temp.path().to_path_buf());

        let id = Identifier::new("minecraft", "test").expect("valid id");
        let texture = manager.load_texture(id.clone(), "test.png").unwrap();

        assert_eq!(texture.width, 16);
        assert_eq!(texture.height, 16);
    }
}
