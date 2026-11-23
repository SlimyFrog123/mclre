//! View and camera uniforms.

use bytemuck::{Pod, Zeroable};
use mclre_core::Vec3;

/// View-projection uniform for shaders (4x4 matrix, column-major, flat).
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct ViewUniform {
    /// Column-major 4x4 view-projection matrix (16 floats).
    pub view_proj: [f32; 16],
}

impl ViewUniform {
    /// Creates a view uniform from a view.
    pub fn from(view: &View) -> Self {
        Self {
            view_proj: view.view_proj_matrix_flat(),
        }
    }
}

/// Camera view with position, orientation, and projection.
#[derive(Debug, Clone, Copy)]
pub struct View {
    /// Camera position in world space.
    pub position: Vec3<f32>,
    /// Camera yaw (horizontal rotation, radians).
    pub yaw: f32,
    /// Camera pitch (vertical rotation, radians).
    pub pitch: f32,
    /// Field of view (radians).
    pub fov: f32,
    /// Aspect ratio (width / height).
    pub aspect: f32,
    /// Near clipping plane.
    pub near: f32,
    /// Far clipping plane.
    pub far: f32,
}

impl View {
    /// Creates a new view.
    pub fn new(
        position: Vec3<f32>,
        yaw: f32,
        pitch: f32,
        fov: f32,
        aspect: f32,
        near: f32,
        far: f32,
    ) -> Self {
        Self {
            position,
            yaw,
            pitch,
            fov,
            aspect,
            near,
            far,
        }
    }

    /// Returns the forward direction vector.
    pub fn forward(&self) -> Vec3<f32> {
        let cos_pitch = self.pitch.cos();
        Vec3::new(
            self.yaw.sin() * cos_pitch,
            self.pitch.sin(),
            -self.yaw.cos() * cos_pitch,
        )
    }

    /// Returns the right direction vector.
    pub fn right(&self) -> Vec3<f32> {
        Vec3::new(self.yaw.cos(), 0.0, self.yaw.sin())
    }

    /// Returns the up direction vector.
    pub fn up(&self) -> Vec3<f32> {
        Vec3::new(0.0, 1.0, 0.0)
    }

    /// Computes the view-projection matrix.
    pub fn view_proj_matrix(&self) -> [[f32; 4]; 4] {
        let forward = self.forward();
        let right = self.right();
        let up = self.up();

        // View matrix (row-major, will be transposed to column-major for GPU).
        // This is the transpose of the standard column-major lookAt matrix.
        // Column-major lookAt:
        // col0: [right.x, right.y, right.z, 0]
        // col1: [up.x, up.y, up.z, 0]
        // col2: [-forward.x, -forward.y, -forward.z, 0]
        // col3: [-dot(right, eye), -dot(up, eye), dot(forward, eye), 1]
        // Transposed to row-major:
        let view = [
            [right.x, right.y, right.z, -right.dot(self.position)],
            [up.x, up.y, up.z, -up.dot(self.position)],
            [
                -forward.x,
                -forward.y,
                -forward.z,
                forward.dot(self.position),
            ],
            [0.0, 0.0, 0.0, 1.0],
        ];

        // Projection matrix (perspective) for Vulkan/DX12 (NDC Z in [0, 1]).
        // Row-major matrix; will be transposed to column-major for GPU.
        // Column-major Vulkan projection:
        // col0: [f/aspect, 0, 0, 0]
        // col1: [0, f, 0, 0]
        // col2: [0, 0, -far/(far-near), -1]
        // col3: [0, 0, -far*near/(far-near), 0]
        // Transposed to row-major:
        let f = 1.0 / (self.fov / 2.0).tan();
        let proj = [
            [f / self.aspect, 0.0, 0.0, 0.0],
            [0.0, f, 0.0, 0.0],
            [0.0, 0.0, -self.far / (self.far - self.near), -1.0],
            [
                0.0,
                0.0,
                -(self.far * self.near) / (self.far - self.near),
                0.0,
            ],
        ];

        // Multiply proj * view
        mul_mat4(&proj, &view)
    }

    /// Computes the view-projection matrix as a flat array (column-major).
    pub fn view_proj_matrix_flat(&self) -> [f32; 16] {
        let mat = self.view_proj_matrix();
        let mut flat = [0.0f32; 16];
        for row in 0..4 {
            for col in 0..4 {
                flat[col * 4 + row] = mat[row][col]; // Column-major: col * 4 + row
            }
        }
        flat
    }
}

/// Multiplies two 4x4 matrices (column-major).
fn mul_mat4(a: &[[f32; 4]; 4], b: &[[f32; 4]; 4]) -> [[f32; 4]; 4] {
    let mut out = [[0.0; 4]; 4];
    for row in 0..4 {
        for col in 0..4 {
            out[row][col] = a[row][0] * b[0][col]
                + a[row][1] * b[1][col]
                + a[row][2] * b[2][col]
                + a[row][3] * b[3][col];
        }
    }
    out
}

/// Viewport rectangle for split-screen rendering.
#[derive(Debug, Clone, Copy)]
pub struct Viewport {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use mclre_core::Vec3;

    #[test]
    fn view_forward_is_unit() {
        let view = View::new(Vec3::new(0.0, 0.0, 0.0), 0.0, 0.0, 1.0, 1.0, 0.1, 100.0);
        let f = view.forward();
        let len = (f.x * f.x + f.y * f.y + f.z * f.z).sqrt();
        assert!((len - 1.0).abs() < 1e-5);
    }

    #[test]
    fn view_matrix_is_valid() {
        let view = View::new(
            Vec3::new(0.0, 1.5, 3.0),
            0.5,
            -0.3,
            std::f32::consts::FRAC_PI_2,
            16.0 / 9.0,
            0.1,
            100.0,
        );
        let mat = view.view_proj_matrix();
        // Check it's a valid 4x4 matrix
        assert_eq!(mat.len(), 4);
        for row in mat {
            assert_eq!(row.len(), 4);
        }
    }

    #[test]
    fn viewport_calculation() {
        let viewports = crate::Renderer::calculate_viewports(2, 1920.0, 1080.0);
        assert_eq!(viewports.len(), 2);
        assert_eq!(viewports[0].width, 1920.0);
        assert_eq!(viewports[0].height, 540.0);
        assert_eq!(viewports[1].y, 540.0);
    }
}
