//! GPU resources: chunk sections, mesh buffers, texture atlases.

use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use mclre_core::{BlockStateModel, Vec3};
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
        block_state_model: Option<&BlockStateModel>,
    ) -> Self {
        let mesh = if section.is_empty() {
            MeshBuffers::empty(device)
        } else {
            MeshBuffers::from_section(device, queue, section, block_state_model)
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
    pub fn from_section(
        device: &Device,
        _queue: &Queue,
        section: &ChunkSection,
        block_state_model: Option<&BlockStateModel>,
    ) -> Self {
        let (vertices, indices) = crate::chunk_mesh::generate_mesh(section, block_state_model);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk_mesh::generate_mesh;
    use mclre_world::section::ChunkSection;

    #[test]
    fn empty_section_generates_empty_mesh() {
        let section = ChunkSection::air();
        let (vertices, indices) = generate_mesh(&section, None);
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
        let (vertices, indices) = generate_mesh(&section, None);
        // 6 faces * 4 vertices = 24 vertices, 6 faces * 6 indices = 36 indices
        // But some faces may be internal...
        assert!(!vertices.is_empty());
        assert!(!indices.is_empty());
    }
}
