//! Math utilities for world generation matching LCE reference
//!
//! Re-exports Mth from mclre-core with worldgen-specific helpers.

pub use mclre_core::Mth;

/// World generation math helpers
pub mod worldgen_math {
    use mclre_core::GameRandom;

    /// Seed mixing function matching reference
    pub fn mix_seed(seed: i64, seed_mixup: i64) -> i64 {
        let mut result = seed;
        result = result.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        result = result.wrapping_add(seed_mixup);
        result = result.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        result = result.wrapping_add(seed_mixup);
        result = result.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        result = result.wrapping_add(seed_mixup);
        result
    }

    /// Chunk seed derivation matching reference
    /// `random->setSeed(xOffs * 341873128712l + zOffs * 132897987541l);`
    pub fn chunk_seed(x: i32, z: i32) -> i64 {
        x as i64 * 341873128712 + z as i64 * 132897987541
    }

    /// Post-process seed derivation matching reference
    /// `pprandom->setSeed(level->getSeed());`
    /// `__int64 xScale = pprandom->nextLong() / 2 * 2 + 1;`
    /// `__int64 zScale = pprandom->nextLong() / 2 * 2 + 1;`
    /// `pprandom->setSeed(((xt * xScale) + (zt * zScale)) ^ level->getSeed());`
    pub fn post_process_seed(seed: i64, chunk_x: i32, chunk_z: i32) -> i64 {
        let mut random = GameRandom::with_seed(seed);
        let x_scale = random.next_long() / 2 * 2 + 1;
        let z_scale = random.next_long() / 2 * 2 + 1;
        ((chunk_x as i64 * x_scale) + (chunk_z as i64 * z_scale)) ^ seed
    }
}