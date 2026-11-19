//! Deterministic math helpers, ported from the reference `Mth`
//! (`docs/reference/rng.md`; source:
//! `Minecraft.World/Mth.{h,cpp}` in the reference tree).
//!
//! Gameplay- and worldgen-affecting code must use these instead of generic
//! `f32`/`f64` intrinsics where the reference uses `Mth`, because the
//! 65536-entry sine table is part of the observable behavior.

// Casts here mirror the reference's float/int mixing (table build, index
// scaling); none are fallible in the ranges used.
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_lossless)]

pub mod aabb;
pub mod vec3;

pub use aabb::AABB;
pub use vec3::Vec3;

/// The reference's `Mth` math surface (subset used so far; extend per system).
///
/// `sin`/`cos` use the exact 65536-entry `f32` lookup table built by the
/// reference (`Mth.cpp:17-24`); `floor`/`ceil` replicate the reference's
/// int-cast + comparison approach (`Mth.cpp:48-90`).
pub struct Mth;

const TABLE_SIZE: usize = 65536;
/// `65536.0 / (PI * 2)` as `f32` (`Mth.h:20`).
const SIN_SCALE: f32 = 65536.0_f32 / (std::f32::consts::PI * 2.0);

/// Lazily-built sine table (`Mth.cpp:17-24`).
fn sine_table() -> &'static [f32] {
    static TABLE: std::sync::OnceLock<Vec<f32>> = std::sync::OnceLock::new();
    TABLE.get_or_init(|| {
        // Heap-allocated (Vec, not a stack array) to keep the 256 KB table
        // off the stack.
        let mut table = vec![0.0_f32; TABLE_SIZE];
        for (i, slot) in table.iter_mut().enumerate() {
            // (float) ::sin(i * PI * 2 / 65536.0f) — f64 sin, then f32 cast.
            *slot = ((i as f64) * std::f64::consts::PI * 2.0 / 65536.0).sin() as f32;
        }
        table
    })
}

impl Mth {
    /// Reference `Mth::sin` (`Mth.cpp:26-30`).
    pub fn sin(x: f32) -> f32 {
        let table = sine_table();
        table[(Self::table_index(x, 0)) & (TABLE_SIZE - 1)]
    }

    /// Reference `Mth::cos` (`Mth.cpp:32-36`).
    pub fn cos(x: f32) -> f32 {
        let table = sine_table();
        table[(Self::table_index(x, TABLE_SIZE / 4)) & (TABLE_SIZE - 1)]
    }

    /// `(int)(x * sinScale [+ quarter-turn offset]) & 65535`.
    fn table_index(x: f32, quarter_offset: usize) -> usize {
        // Reference: (int)(i * sinScale) & 65535 — C `int` cast truncates
        // toward zero. On MSVC/x64 the cast compiles to `cvttss2si`, which
        // yields INT_MIN (0x80000000) for out-of-range/NaN inputs; emulate
        // that so huge angles match the reference platform exactly
        // (Rust's `as` would saturate to i32::MAX instead).
        let scaled = x * SIN_SCALE;
        let int_index = if scaled.is_nan() || scaled < i32::MIN as f32 || scaled > i32::MAX as f32 {
            i32::MIN
        } else {
            scaled as i32
        };
        int_index.wrapping_add(quarter_offset as i32) as usize
    }

    /// Reference `Mth::sqrt` (`Mth.cpp:38-46`).
    pub fn sqrt(x: f32) -> f32 {
        (x as f64).sqrt() as f32
    }

    /// Reference `Mth::floor(float)` (`Mth.cpp:48-52`).
    pub fn floor_f32(v: f32) -> i32 {
        let i = v as i32;
        if v < i as f32 { i - 1 } else { i }
    }

    /// Reference `Mth::lfloor(double)` (`Mth.cpp:54-58`).
    pub fn lfloor(v: f64) -> i64 {
        let i = v as i64;
        if v < i as f64 { i - 1 } else { i }
    }

    /// Reference `Mth::fastFloor(double)` (`Mth.cpp:60-63`).
    ///
    /// Only correct for inputs within ±1024 of zero (by design in the
    /// reference); do not use for unbounded values.
    pub fn fast_floor(x: f64) -> i32 {
        ((x + 1024.0) as i32) - 1024
    }

    /// Reference `Mth::floor(double)` (`Mth.cpp:65-69`).
    pub fn floor_f64(v: f64) -> i32 {
        let i = v as i32;
        if v < i as f64 { i - 1 } else { i }
    }

    /// Reference `Mth::absFloor(double)` (`Mth.cpp:71-74`).
    pub fn abs_floor(v: f64) -> i32 {
        if v >= 0.0 {
            v as i32
        } else {
            (-v + 1.0) as i32
        }
    }

    /// Reference `Mth::abs(float)` (`Mth.cpp:76-79`).
    pub fn abs_f32(v: f32) -> f32 {
        if v >= 0.0 { v } else { -v }
    }

    /// Reference `Mth::abs(int)` (`Mth.cpp:81-84`).
    pub fn abs_i32(v: i32) -> i32 {
        if v >= 0 { v } else { -v }
    }

    /// Reference `Mth::ceil(float)` (`Mth.cpp:86-90`).
    pub fn ceil_f32(v: f32) -> i32 {
        let i = v as i32;
        if v > i as f32 { i + 1 } else { i }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::float_cmp)] // compared values are exact by construction

    use super::*;

    /// Common reference-visible angles: quarter/half turns and a sweep.
    #[test]
    fn sine_table_matches_reference_construction() {
        let table = sine_table();
        // sin(0) = 0, sin(pi/2) ~ 1 (i = 16384), sin(pi) ~ 0 (i = 32768).
        assert_eq!(table[0], 0.0);
        assert!((table[16384] - 1.0).abs() < 1e-6);
        assert!(table[32768].abs() < 1e-6);
        assert_eq!(table.len(), TABLE_SIZE);
    }

    #[test]
    fn cos_is_quarter_table_offset_of_sin() {
        // cos(x) = table[index(x) + 16384]: the offset is exact in *table
        // units*. Comparing with sin(x + pi/2) would re-round in f32, so use
        // a one-table-step tolerance instead of bit equality.
        assert!((Mth::cos(0.0) - 1.0).abs() < 1e-6);
        assert!(Mth::cos(std::f32::consts::FRAC_PI_2).abs() < 1e-6);
        for k in 0..64 {
            let x = k as f32 * 0.7;
            let via_sin = Mth::sin(x + std::f32::consts::FRAC_PI_2);
            let tolerance = 2.0 * (std::f64::consts::PI / 65536.0) as f32;
            assert!(
                (Mth::cos(x) - via_sin).abs() <= tolerance,
                "cos({x})={}; sin(x+pi/2)={via_sin}",
                Mth::cos(x)
            );
        }
    }

    #[test]
    fn sin_matches_real_sin_for_small_angles() {
        for k in 0..100 {
            let x = k as f32 * 0.123;
            let want = x.sin();
            let got = Mth::sin(x);
            // Table quantization: within one table step.
            let tolerance = 2.0 * (std::f64::consts::PI / 65536.0) as f32;
            assert!((got - want).abs() <= tolerance, "sin({x}): {got} vs {want}");
        }
    }

    #[test]
    fn sin_handles_negative_angles_by_wraparound() {
        // sin is periodic with the & 65535 mask; the f32 re-rounding of
        // (-1.0 + 2*pi) can shift the index by one step, so allow a step of
        // tolerance rather than bit equality.
        let a = Mth::sin(-1.0);
        let b = Mth::sin(-1.0 + 2.0 * std::f32::consts::PI);
        let tolerance = 2.0 * (std::f64::consts::PI / 65536.0) as f32;
        assert!((a - b).abs() <= tolerance, "sin(-1)={a} vs sin(-1+2pi)={b}");
    }

    #[test]
    fn sin_of_huge_angle_matches_reference_int_min_cast() {
        // 1e6 * sinScale exceeds i32::MAX; the reference's MSVC/x64
        // `(int)` cast yields INT_MIN -> index 0 -> sin == table[0] == 0.
        let big = 1e6_f32;
        assert_eq!(Mth::sin(big).to_bits(), 0.0_f32.to_bits());
    }

    #[test]
    fn floors_match_reference_semantics() {
        assert_eq!(Mth::floor_f32(-0.5), -1);
        assert_eq!(Mth::floor_f32(3.9), 3);
        assert_eq!(Mth::floor_f32(4.0), 4);
        assert_eq!(Mth::lfloor(-0.5), -1);
        assert_eq!(Mth::lfloor(3.9), 3);
        assert_eq!(Mth::fast_floor(-0.5), -1);
        assert_eq!(Mth::fast_floor(1023.5), 1023);
        assert_eq!(Mth::floor_f64(-0.5), -1);
        assert_eq!(Mth::abs_floor(-1.2), 2); // reference: (int)(-(-1.2) + 1) = (int)2.2 = 2
        assert_eq!(Mth::abs_floor(1.2), 1);
        assert_eq!(Mth::ceil_f32(3.1), 4);
        assert_eq!(Mth::ceil_f32(4.0), 4);
        assert_eq!(Mth::ceil_f32(-0.5), 0);
        assert_eq!(Mth::abs_i32(-5), 5);
        assert_eq!(Mth::abs_f32(-5.0), 5.0);
        assert_eq!(Mth::sqrt(4.0), 2.0);
    }
}
