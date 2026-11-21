//! Math utilities for world generation matching LCE reference

use std::f64::consts::PI;

/// 65536-entry sine lookup table (matches reference exactly)
static mut SIN_TABLE: Option<[f32; 65536]> = None;

/// Mth class matching LCE reference exactly
pub struct Mth;

impl Mth {
    pub const BIG_ENOUGH_INT: i32 = 1024;
    pub const BIG_ENOUGH_FLOAT: f32 = 1024.0;
    pub const DEGRAD: f32 = std::f32::consts::PI / 180.0;
    pub const RADDEG: f32 = 180.0 / std::f32::consts::PI;
    pub const SIN_SCALE: f32 = 65536.0 / (std::f32::consts::PI * 2.0);

    /// 65536-entry sine lookup table (matches reference exactly)
    fn init() {
        unsafe {
            if SIN_TABLE.is_none() {
                let mut table = [0.0f32; 65536];
                for i in 0..65536 {
                    table[i] = (i as f32 * std::f32::consts::PI * 2.0 / 65536.0).sin();
                }
                SIN_TABLE = Some(table);
            }
        }
    }

    pub fn sin(f: f32) -> f32 {
        unsafe {
            if SIN_TABLE.is_none() { Self::init(); }
            SIN_TABLE.unwrap()[(f * Self::SIN_SCALE) as i32 & 65535]
        }
    }

    pub fn cos(f: f32) -> f32 {
        unsafe {
            if SIN_TABLE.is_none() { Self::init(); }
            SIN_TABLE.unwrap()[(f * Self::SIN_SCALE + 16384.0) as i32 & 65535]
        }
    }

    pub fn sqrt_f32(x: f32) -> f32 { x.sqrt() }
    pub fn sqrt_f64(x: f64) -> f64 { x.sqrt() }

    pub fn floor_f32(v: f32) -> i32 { let i = v as i32; if v < i as f32 { i - 1 } else { i } }
    pub fn floor_f64(v: f64) -> i64 { let i = v as i64; if v < i as f64 { i - 1 } else { i } }
    pub fn lfloor(v: f64) -> i64 { Self::floor_f64(v) }
    pub fn fast_floor(x: f64) -> i32 { (x + Self::BIG_ENOUGH_FLOAT as f64) as i32 - Self::BIG_ENOUGH_INT }
    pub fn abs_floor(v: f64) -> i32 { (if v >= 0.0 { v } else { -v + 1.0 }) as i32 }
    pub fn abs_f32(v: f32) -> f32 { if v >= 0.0 { v } else { -v } }
    pub fn abs_i32(v: i32) -> i32 { if v >= 0 { v } else { -v } }
    pub fn ceil_f32(v: f32) -> i32 { let i = v as i32; if v > i as f32 { i + 1 } else { i } }

    pub fn clamp_i32(value: i32, min: i32, max: i32) -> i32 { if value < min { min } else if value > max { max } else { value } }
    pub fn clamp_f32(value: f32, min: f32, max: f32) -> f32 { if value < min { min } else if value > max { max } else { value } }
    pub fn asb_max(a: f64, b: f64) -> f64 { let a = if a < 0.0 { -a } else { a }; let b = if b < 0.0 { -b } else { b }; if a > b { a } else { b } }
    pub fn int_floor_div(a: i32, b: i32) -> i32 { if a < 0 { -((-a - 1) / b) - 1 } else { a / b } }

    pub fn next_int(random: &mut mclre_core::GameRandom, min: i32, max: i32) -> i32 { if min >= max { return min; } random.next_int_bound(max - min + 1) + min }
    pub fn next_float(random: &mut mclre_core::GameRandom, min: f32, max: f32) -> f32 { if min >= max { return min; } random.next_float() * (max - min) + min }
    pub fn next_double(random: &mut mclre_core::GameRandom, min: f64, max: f64) -> f64 { if min >= max { return min; } random.next_double() * (max - min) + min }

    pub fn wrap_degrees_f32(mut input: f32) -> f32 { while input >= 180.0 { input -= 360.0; } while input < -180.0 { input += 360.0; } input }
    pub fn wrap_degrees_f64(mut input: f64) -> f64 { while input >= 180.0 { input -= 360.0; } while input < -180.0 { input += 360.0; } input }
}