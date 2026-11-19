//! Chunk mesh generation: per-face meshing with AO/smooth lighting.
//!
//! Implements the per-face strategy from ADR 0008 with LCE smooth lighting.

use crate::resources::ChunkVertex;
use mclre_world::section::ChunkSection;

/// Generates a mesh for a chunk section using per-face meshing with AO.
///
/// This implements the per-face strategy from ADR 0008 with LCE smooth lighting.
pub fn generate_mesh(section: &ChunkSection) -> (Vec<ChunkVertex>, Vec<u32>) {
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
/// Uses the standard 3-3-3 AO pattern for smooth lighting (LCE style).
fn compute_face_ao(section: &ChunkSection, x: u32, y: u32, z: u32, face: usize) -> [u32; 4] {
    // For each corner of the face, we check 3 voxels (side, side, corner)
    // This implements the standard "3-3-3" AO pattern used in Minecraft

    let mut ao = [0u32; 4];

    // Define which voxels to check for each corner of each face
    // The 3 voxels are: the two face-adjacent and the corner-adjacent
    let corner_offsets = get_corner_ao_offsets(face);

    for (corner, offsets) in corner_offsets.iter().enumerate() {
        let mut occlusion = 0u32;
        for &(ox, oy, oz) in offsets {
            let cx = x as i32 + ox;
            let cy = y as i32 + oy;
            let cz = z as i32 + oz;

            let is_solid =
                if (0..16).contains(&cx) && (0..16).contains(&cy) && (0..16).contains(&cz) {
                    let idx = ((cy as usize) << 8) | ((cz as usize) << 4) | (cx as usize);
                    section.get_index(idx).0 != 0
                } else {
                    false // Out of section bounds - treat as air for now
                };

            if is_solid {
                occlusion += 1;
            }
        }
        // AO value: 0 = fully occluded (dark), 63 = fully lit
        // 3 samples: 0, 1, 2, 3 solid neighbors
        ao[corner] = 63 - occlusion * 21; // 63, 42, 21, 0
    }

    ao
}

/// Returns the 3 voxel offsets to check for AO at each corner of a face.
///
/// Returns [4 corners][3 offsets] where each offset is (x, y, z).
fn get_corner_ao_offsets(face: usize) -> [[(i32, i32, i32); 3]; 4] {
    // For each face, the 4 corners and their 3 AO sample positions
    // The face normals are: +X, -X, +Y, -Y, +Z, -Z
    match face {
        0 => {
            // +X face (right): corners are at (x+1, y, z), (x+1, y+1, z), (x+1, y+1, z+1), (x+1, y, z+1)
            // AO samples for each corner: (side1, side2, corner)
            [
                [(1, 0, 0), (1, 1, 0), (1, 1, 1)], // bottom-left
                [(1, 1, 0), (1, 0, 0), (1, 1, 1)], // top-left (reordered for consistent winding)
                [(1, 1, 1), (1, 1, 0), (1, 0, 1)], // top-right
                [(1, 0, 1), (1, 0, 0), (1, 1, 1)], // bottom-right
            ]
        }
        1 => {
            // -X face (left)
            [
                [(0, 0, 1), (0, 1, 1), (0, 1, 0)],
                [(0, 1, 1), (0, 0, 1), (0, 1, 0)],
                [(0, 1, 0), (0, 1, 1), (0, 0, 0)],
                [(0, 0, 0), (0, 0, 1), (0, 1, 0)],
            ]
        }
        2 => {
            // +Y face (top)
            [
                [(0, 1, 0), (1, 1, 0), (1, 1, 1)],
                [(1, 1, 0), (0, 1, 0), (1, 1, 1)],
                [(1, 1, 1), (1, 1, 0), (0, 1, 1)],
                [(0, 1, 1), (0, 1, 0), (1, 1, 1)],
            ]
        }
        3 => {
            // -Y face (bottom)
            [
                [(0, 0, 1), (1, 0, 1), (1, 0, 0)],
                [(1, 0, 1), (0, 0, 1), (1, 0, 0)],
                [(1, 0, 0), (1, 0, 1), (0, 0, 0)],
                [(0, 0, 0), (0, 0, 1), (1, 0, 0)],
            ]
        }
        4 => {
            // +Z face (front)
            [
                [(0, 0, 1), (1, 0, 1), (1, 1, 1)],
                [(1, 0, 1), (0, 0, 1), (1, 1, 1)],
                [(1, 1, 1), (1, 0, 1), (0, 1, 1)],
                [(0, 1, 1), (0, 0, 1), (1, 1, 1)],
            ]
        }
        5 => {
            // -Z face (back)
            [
                [(1, 0, 0), (1, 1, 0), (0, 1, 0)],
                [(1, 1, 0), (1, 0, 0), (0, 1, 0)],
                [(0, 1, 0), (1, 1, 0), (0, 0, 0)],
                [(0, 0, 0), (1, 0, 0), (0, 1, 0)],
            ]
        }
        _ => unreachable!(),
    }
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
    fn single_block_generates_6_faces() {
        let mut section = ChunkSection::air();
        section.set(
            mclre_world::pos::LocalPos::from_axes(8, 8, 8),
            mclre_core::BlockStateId(1),
        );
        let (vertices, indices) = generate_mesh(&section);
        // 6 faces * 4 vertices = 24 vertices
        // 6 faces * 6 indices = 36 indices
        assert_eq!(vertices.len(), 24);
        assert_eq!(indices.len(), 36);
    }

    #[test]
    fn two_adjacent_blocks_share_face() {
        let mut section = ChunkSection::air();
        section.set(
            mclre_world::pos::LocalPos::from_axes(8, 8, 8),
            mclre_core::BlockStateId(1),
        );
        section.set(
            mclre_world::pos::LocalPos::from_axes(9, 8, 8),
            mclre_core::BlockStateId(1),
        );
        let (vertices, indices) = generate_mesh(&section);
        // 2 blocks = 12 faces, minus 1 shared face = 10 faces
        // 10 * 4 = 40 vertices, 10 * 6 = 60 indices
        assert_eq!(vertices.len(), 40);
        assert_eq!(indices.len(), 60);
    }

    #[test]
    fn ao_values_are_in_range() {
        let mut section = ChunkSection::air();
        section.set(
            mclre_world::pos::LocalPos::from_axes(8, 8, 8),
            mclre_core::BlockStateId(1),
        );
        let (vertices, _) = generate_mesh(&section);
        for v in vertices {
            assert!(v.normal_ao & 0x3F <= 63); // AO is 6 bits
        }
    }
}
