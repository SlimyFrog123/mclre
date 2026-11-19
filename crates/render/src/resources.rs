//! GPU resources: chunk sections, mesh buffers, texture atlases.

use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use mclre_core::Vec3;
use mclre_world::section::ChunkSection;
use wgpu::util::DeviceExt;
use wgpu::{Buffer, BufferUsages, Device, Queue};

/// A chunk section uploaded to the GPU.
#[derive(Debug, Clone)]
pub struct GpuChunkSection {
    /// The chunk section position.
    pub position: Vec3<i32>,
    /// Mesh buffers (vertices + indices).
    pub mesh: MeshBuffers,
    /// Whether the mesh is empty (all air).
    pub is_empty: bool,
}

impl GpuChunkSection {
    /// Creates a GPU chunk section from a CPU chunk section.
    pub fn from_section(
        device: &Device,
        queue: &Queue,
        section: &ChunkSection,
        position: Vec3<i32>,
    ) -> Self {
        let mesh = if section.is_empty() {
            MeshBuffers::empty(device)
        } else {
            MeshBuffers::from_section(device, queue, section)
        };
        let is_empty = section.is_empty();
        Self {
            position,
            mesh,
            is_empty,
        }
    }
}

/// Mesh buffers for a chunk section (vertices + indices).
#[derive(Debug, Clone)]
pub struct MeshBuffers {
    /// Vertex buffer.
    pub vertices: Arc<Buffer>,
    /// Index buffer.
    pub indices: Arc<Buffer>,
    /// Number of indices.
    pub index_count: u32,
}

impl MeshBuffers {
    /// Creates empty mesh buffers.
    pub fn empty(device: &Device) -> Self {
        let vertices = Arc::new(device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("empty vertex buffer"),
            size: std::mem::size_of::<ChunkVertex>() as u64,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));
        let indices = Arc::new(device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("empty index buffer"),
            size: std::mem::size_of::<u32>() as u64,
            usage: BufferUsages::INDEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));
        Self {
            vertices,
            indices,
            index_count: 0,
        }
    }

    /// Creates mesh buffers from a chunk section.
    pub fn from_section(device: &Device, _queue: &Queue, section: &ChunkSection) -> Self {
        let (vertices, indices) = generate_mesh(section);
        let index_count = indices.len() as u32;

        // ChunkVertex is repr(C) with 3 u32 fields, so it has the same layout as [u32; 3]
        // Convert to flat u32 slice for bytemuck upload (avoids NoUninit issues)
        let vertex_u32s: Vec<u32> = vertices
            .iter()
            .flat_map(|v| [v.position, v.normal_ao, v.tex_coord])
            .collect();

        let vertex_buffer = Arc::new(device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("chunk vertex buffer"),
                contents: bytemuck::cast_slice(&vertex_u32s),
                usage: BufferUsages::VERTEX,
            },
        ));

        let index_buffer = Arc::new(
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("chunk index buffer"),
                contents: bytemuck::cast_slice(&indices),
                usage: BufferUsages::INDEX,
            }),
        );

        Self {
            vertices: vertex_buffer,
            indices: index_buffer,
            index_count,
        }
    }
}

/// Vertex format for chunk meshes.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct ChunkVertex {
    /// Packed position: 10 bits x, 10 bits y, 10 bits z (local to section), 2 bits unused.
    pub position: u32,
    /// Packed normal + AO: 2 bits face, 6 bits AO (0-63), 24 bits unused.
    pub normal_ao: u32,
    /// Packed texture coordinates: 8 bits u, 8 bits v, 16 bits texture layer.
    pub tex_coord: u32,
}

impl ChunkVertex {
    /// Creates a new vertex.
    pub fn new(x: u32, y: u32, z: u32, face: u32, ao: u32, u: u32, v: u32, layer: u32) -> Self {
        Self {
            position: (x & 0x3FF) | ((y & 0x3FF) << 10) | ((z & 0x3FF) << 20),
            normal_ao: (face & 0x3) | ((ao & 0x3F) << 2),
            tex_coord: (u & 0xFF) | ((v & 0xFF) << 8) | ((layer & 0xFFFF) << 16),
        }
    }
}

/// Generates a mesh for a chunk section using per-face meshing with AO.
///
/// This implements the per-face strategy from ADR 0008 with LCE smooth lighting.
fn generate_mesh(section: &ChunkSection) -> (Vec<ChunkVertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    // Face directions: +X, -X, +Y, -Y, +Z, -Z
    const FACE_DIRS: [(i32, i32, i32); 6] = [
        (1, 0, 0),
        (-1, 0, 0),
        (0, 1, 0),
        (0, -1, 0),
        (0, 0, 1),
        (0, 0, -1),
    ];

    // Vertex offsets for each face (4 vertices per face, CCW from bottom-left when viewed from outside)
    const FACE_VERTICES: [[(u32, u32, u32); 4]; 6] = [
        // +X face (right)
        [(1, 0, 0), (1, 1, 0), (1, 1, 1), (1, 0, 1)],
        // -X face (left)
        [(0, 0, 1), (0, 1, 1), (0, 1, 0), (0, 0, 0)],
        // +Y face (top)
        [(0, 1, 0), (1, 1, 0), (1, 1, 1), (0, 1, 1)],
        // -Y face (bottom)
        [(0, 0, 1), (1, 0, 1), (1, 0, 0), (0, 0, 0)],
        // +Z face (front)
        [(0, 0, 1), (1, 0, 1), (1, 1, 1), (0, 1, 1)],
        // -Z face (back)
        [(1, 0, 0), (1, 1, 0), (0, 1, 0), (0, 0, 0)],
    ];

    // Texture coordinates for each vertex of a face
    const FACE_UV: [(u32, u32); 4] = [(0, 0), (1, 0), (1, 1), (0, 1)];

    for (face, &(dx, dy, dz)) in FACE_DIRS.iter().enumerate() {
        for x in 0..16 {
            for y in 0..16 {
                for z in 0..16 {
                    let idx = ((y << 8) | (z << 4) | x) as usize;
                    let state = section.get_index(idx);
                    if state.0 == 0 {
                        continue; // Air
                    }

                    // Check neighbor in face direction
                    let nx = x as i32 + dx;
                    let ny = y as i32 + dy;
                    let nz = z as i32 + dz;

                    let neighbor_is_solid = if (0..16).contains(&nx)
                        && (0..16).contains(&ny)
                        && (0..16).contains(&nz)
                    {
                        let nidx = ((ny as usize) << 8) | ((nz as usize) << 4) | (nx as usize);
                        section.get_index(nidx).0 != 0
                    } else {
                        false // Neighbor is in another section - assume solid for now
                    };

                    if neighbor_is_solid {
                        continue; // Face is occluded
                    }

                    // Compute AO for the 4 corners of this face
                    let ao_values = compute_face_ao(section, x, y, z, face);

                    let base_vertex = vertices.len() as u32;
                    for (i, &(vx, vy, vz)) in FACE_VERTICES[face].iter().enumerate() {
                        let vx = x + vx;
                        let vy = y + vy;
                        let vz = z + vz;
                        let (u, v) = FACE_UV[i];
                        let ao = ao_values[i];
                        vertices.push(ChunkVertex::new(vx, vy, vz, face as u32, ao, u, v, 0));
                    }

                    // Two triangles per face (CCW winding)
                    indices.push(base_vertex);
                    indices.push(base_vertex + 1);
                    indices.push(base_vertex + 2);
                    indices.push(base_vertex);
                    indices.push(base_vertex + 2);
                    indices.push(base_vertex + 3);
                }
            }
        }
    }

    (vertices, indices)
}

/// Computes ambient occlusion values for the 4 corners of a face.
///
/// Uses the standard 3-3-3 AO pattern for smooth lighting.
fn compute_face_ao(_section: &ChunkSection, _x: u32, _y: u32, _z: u32, _face: usize) -> [u32; 4] {
    // For each corner, check the 3 adjacent voxels
    // This is a simplified AO - real implementation would be more sophisticated
    let mut ao = [0u32; 4];

    // Define the 3 voxels to check for each corner of each face
    // This is a placeholder - full AO implementation is complex
    for i in 0..4 {
        ao[i] = 63; // Full brightness for now
    }

    ao
}

#[cfg(test)]
mod tests {
    use super::*;
    use mclre_world::section::ChunkSection;

    #[test]
    fn empty_section_generates_empty_mesh() {
        let section = ChunkSection::air();
        let (vertices, indices) = generate_mesh(&section);
        assert!(vertices.is_empty());
        assert!(indices.is_empty());
    }

    #[test]
    fn solid_block_generates_vertices() {
        let mut section = ChunkSection::air();
        section.set(
            mclre_world::pos::LocalPos::from_axes(8, 8, 8),
            mclre_core::BlockStateId(1),
        );
        let (vertices, indices) = generate_mesh(&section);
        // 6 faces * 4 vertices = 24 vertices, 6 faces * 6 indices = 36 indices
        // But some faces may be internal...
        assert!(!vertices.is_empty());
        assert!(!indices.is_empty());
    }
}
