//! 9-slice scaling for UI panels and borders.

use super::Rect;
use mclre_assets::Identifier;

/// 9-slice region for scaling UI elements without distortion.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NineSliceRegion {
    /// Left slice width (in pixels).
    pub left: u32,
    /// Right slice width (in pixels).
    pub right: u32,
    /// Top slice height (in pixels).
    pub top: u32,
    /// Bottom slice height (in pixels).
    pub bottom: u32,
    /// UV coordinates for the texture atlas.
    pub u_min: f32,
    pub v_min: f32,
    pub u_max: f32,
    pub v_max: f32,
}

impl NineSliceRegion {
    /// Creates a new 9-slice region.
    pub fn new(left: u32, right: u32, top: u32, bottom: u32) -> Self {
        Self {
            left,
            right,
            top,
            bottom,
            u_min: 0.0,
            v_min: 0.0,
            u_max: 1.0,
            v_max: 1.0,
        }
    }

    /// Sets the UV coordinates for the texture atlas.
    pub fn with_uv(mut self, u_min: f32, v_min: f32, u_max: f32, v_max: f32) -> Self {
        self.u_min = u_min;
        self.v_min = v_min;
        self.u_max = u_max;
        self.v_max = v_max;
        self
    }

    /// Returns the minimum size required to avoid stretching the corners.
    pub fn min_size(&self) -> (u32, u32) {
        (self.left + self.right, self.top + self.bottom)
    }

    /// Checks if the given size is sufficient for 9-slice rendering.
    pub fn fits_in(&self, width: u32, height: u32) -> bool {
        width >= self.left + self.right && height >= self.top + self.bottom
    }
}

/// 9-slice texture atlas for managing multiple 9-slice regions.
#[derive(Debug)]
pub struct NineSliceAtlas {
    regions: std::collections::HashMap<Identifier, NineSliceRegion>,
    texture_atlas: Option<mclre_render::TextureAtlas>,
}

impl NineSliceAtlas {
    /// Creates a new 9-slice atlas without a texture atlas.
    pub fn new() -> Self {
        Self {
            regions: std::collections::HashMap::new(),
            texture_atlas: None,
        }
    }

    /// Creates a 9-slice atlas from a texture atlas.
    pub fn from_texture_atlas(texture_atlas: mclre_render::TextureAtlas) -> Self {
        Self {
            regions: std::collections::HashMap::new(),
            texture_atlas: Some(texture_atlas),
        }
    }

    /// Sets the texture atlas.
    pub fn set_texture_atlas(&mut self, texture_atlas: mclre_render::TextureAtlas) {
        self.texture_atlas = Some(texture_atlas);
    }

    /// Gets the underlying texture atlas.
    pub fn texture_atlas(&self) -> Option<&mclre_render::TextureAtlas> {
        self.texture_atlas.as_ref()
    }

    /// Adds a 9-slice region.
    pub fn add_region(&mut self, id: Identifier, region: NineSliceRegion) {
        self.regions.insert(id, region);
    }

    /// Gets a 9-slice region by ID.
    pub fn get_region(&self, id: &Identifier) -> Option<NineSliceRegion> {
        self.regions.get(id).copied()
    }

    /// Gets a mutable reference to the texture atlas.
    pub fn texture_atlas_mut(&mut self) -> Option<&mut mclre_render::TextureAtlas> {
        self.texture_atlas.as_mut()
    }
}

/// Computes the 9-slice UV coordinates for rendering.
pub fn compute_nine_slice_uvs(
    region: NineSliceRegion,
    target_width: f32,
    target_height: f32,
) -> [NineSliceQuad; 9] {
    let left_w = region.left as f32;
    let right_w = region.right as f32;
    let top_h = region.top as f32;
    let bottom_h = region.bottom as f32;

    let center_w = target_width - left_w - right_w;
    let center_h = target_height - top_h - bottom_h;

    let u_left = region.u_min + (region.left as f32 / (region.u_max - region.u_min));
    let u_center_end = region.u_max - (region.right as f32 / (region.u_max - region.u_min));
    let v_top = region.v_min + (region.top as f32 / (region.v_max - region.v_min));
    let v_center_end = region.v_max - (region.bottom as f32 / (region.v_max - region.v_min));

    // 9 quads: tl, tc, tr, ml, mc, mr, bl, bc, br
    [
        // Top-left
        NineSliceQuad::new(
            Rect::new(0.0, 0.0, left_w, top_h),
            [region.u_min, region.v_min, u_left, v_top],
        ),
        // Top-center
        NineSliceQuad::new(
            Rect::new(left_w, 0.0, center_w, top_h),
            [u_left, region.v_min, u_center_end, v_top],
        ),
        // Top-right
        NineSliceQuad::new(
            Rect::new(left_w + center_w, 0.0, right_w, top_h),
            [u_center_end, region.v_min, region.u_max, v_top],
        ),
        // Middle-left
        NineSliceQuad::new(
            Rect::new(0.0, top_h, left_w, center_h),
            [region.u_min, v_top, u_left, v_center_end],
        ),
        // Middle-center
        NineSliceQuad::new(
            Rect::new(left_w, top_h, center_w, center_h),
            [u_left, v_top, u_center_end, v_center_end],
        ),
        // Middle-right
        NineSliceQuad::new(
            Rect::new(left_w + center_w, top_h, right_w, center_h),
            [u_center_end, v_top, region.u_max, v_center_end],
        ),
        // Bottom-left
        NineSliceQuad::new(
            Rect::new(0.0, top_h + center_h, left_w, bottom_h),
            [region.u_min, v_center_end, u_left, region.v_max],
        ),
        // Bottom-center
        NineSliceQuad::new(
            Rect::new(left_w, top_h + center_h, center_w, bottom_h),
            [u_left, v_center_end, u_center_end, region.v_max],
        ),
        // Bottom-right
        NineSliceQuad::new(
            Rect::new(left_w + center_w, top_h + center_h, right_w, bottom_h),
            [u_center_end, v_center_end, region.u_max, region.v_max],
        ),
    ]
}

/// A quad for 9-slice rendering.
#[derive(Debug, Clone, Copy)]
pub struct NineSliceQuad {
    pub rect: Rect,
    pub uv: [f32; 4], // u_min, v_min, u_max, v_max
}

impl NineSliceQuad {
    pub fn new(rect: Rect, uv: [f32; 4]) -> Self {
        Self { rect, uv }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nine_slice_region_min_size() {
        let region = NineSliceRegion::new(4, 4, 4, 4);
        assert_eq!(region.min_size(), (8, 8));
    }

    #[test]
    fn nine_slice_fits() {
        let region = NineSliceRegion::new(4, 4, 4, 4);
        assert!(region.fits_in(16, 16));
        assert!(!region.fits_in(7, 16));
        assert!(!region.fits_in(16, 7));
    }

    #[test]
    fn nine_slice_atlas() {
        let mut atlas = NineSliceAtlas::new();
        let id = Identifier::new("minecraft", "panel").unwrap();
        let region = NineSliceRegion::new(4, 4, 4, 4);

        atlas.add_region(id.clone(), region);
        assert_eq!(atlas.get_region(&id), Some(region));
    }
}
