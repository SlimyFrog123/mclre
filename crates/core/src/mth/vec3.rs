//! Three-component vectors (`Vec3<T>`).
//!
//! This is the shared coordinate/vector type used across MCLRE. Coordinate
//! types such as [`crate::pos::BlockPos`] build on `Vec3<i32>` so that
//! addition, subtraction, offset, distance and componentwise min/max logic is
//! written once instead of being re-implemented per position type.
//!
//! Arithmetic is generic over the component type, so the same implementation
//! serves integer block positions (`Vec3<i32>`), section-local coordinates
//! (`Vec3<u8>`) and floating-point geometry (`Vec3<f32>`, `Vec3<f64>`).

use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

/// A 3-component vector.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Vec3<T> {
    /// X component.
    pub x: T,
    /// Y component.
    pub y: T,
    /// Z component.
    pub z: T,
}

impl<T> Vec3<T> {
    /// Creates a vector from components.
    #[must_use]
    pub const fn new(x: T, y: T, z: T) -> Self {
        Self { x, y, z }
    }

    /// A vector with every component set to `value`.
    #[must_use]
    pub const fn splat(value: T) -> Self
    where
        T: Copy,
    {
        Self {
            x: value,
            y: value,
            z: value,
        }
    }

    /// The zero vector (requires `T: Default`).
    #[must_use]
    pub fn zero() -> Self
    where
        T: Default,
    {
        Self::default()
    }

    /// The components as an array `[x, y, z]`.
    #[must_use]
    pub fn to_array(self) -> [T; 3] {
        [self.x, self.y, self.z]
    }

    /// Builds a vector from an array `[x, y, z]`.
    #[must_use]
    pub fn from_array(array: [T; 3]) -> Self
    where
        T: Copy,
    {
        Self {
            x: array[0],
            y: array[1],
            z: array[2],
        }
    }

    /// Maps every component with `f`.
    #[must_use]
    pub fn map<U, F>(self, f: F) -> Vec3<U>
    where
        F: FnMut(T) -> U,
    {
        let mut f = f;
        Vec3 {
            x: f(self.x),
            y: f(self.y),
            z: f(self.z),
        }
    }

    /// Maps two vectors componentwise with `f`.
    #[must_use]
    pub fn map2<U, F>(self, other: Self, mut f: F) -> Vec3<U>
    where
        F: FnMut(T, T) -> U,
    {
        Vec3 {
            x: f(self.x, other.x),
            y: f(self.y, other.y),
            z: f(self.z, other.z),
        }
    }
}

/// Componentwise absolute value.
impl<T> Vec3<T>
where
    T: PartialOrd + Neg<Output = T> + Copy + Default,
{
    /// A vector with the absolute value of every component.
    #[must_use]
    pub fn abs(self) -> Self {
        let zero = T::default();
        self.map(|value| if value < zero { -value } else { value })
    }
}

impl<T: Copy> Vec3<T> {
    /// Casts every component to `U` (e.g. `Vec3<i32>` → `Vec3<f64>`).
    #[must_use]
    pub fn cast<U>(self) -> Vec3<U>
    where
        T: Into<U>,
        U: Copy,
    {
        self.map(std::convert::Into::into)
    }
}

// ----------------------------------------------------------------------------
// Componentwise arithmetic.
// ----------------------------------------------------------------------------

macro_rules! impl_binop {
    ($trait:ident, $method:ident, $assign:ident, $assign_method:ident) => {
        impl<T> $trait for Vec3<T>
        where
            T: Copy + $trait<Output = T>,
        {
            type Output = Self;
            fn $method(self, rhs: Self) -> Self::Output {
                Self {
                    x: self.x.$method(rhs.x),
                    y: self.y.$method(rhs.y),
                    z: self.z.$method(rhs.z),
                }
            }
        }

        impl<T> $assign for Vec3<T>
        where
            T: Copy + $trait<Output = T>,
        {
            fn $assign_method(&mut self, rhs: Self) {
                self.x = self.x.$method(rhs.x);
                self.y = self.y.$method(rhs.y);
                self.z = self.z.$method(rhs.z);
            }
        }
    };
}

impl_binop!(Add, add, AddAssign, add_assign);
impl_binop!(Sub, sub, SubAssign, sub_assign);
impl_binop!(Mul, mul, MulAssign, mul_assign);
impl_binop!(Div, div, DivAssign, div_assign);

/// Scalar division: Vec3 / scalar.
impl<T> Div<T> for Vec3<T>
where
    T: Copy + Div<Output = T>,
{
    type Output = Self;
    fn div(self, rhs: T) -> Self::Output {
        Self {
            x: self.x / rhs,
            y: self.y / rhs,
            z: self.z / rhs,
        }
    }
}

/// Scalar division assignment: Vec3 /= scalar.
impl<T> DivAssign<T> for Vec3<T>
where
    T: Copy + DivAssign,
{
    fn div_assign(&mut self, rhs: T) {
        self.x /= rhs;
        self.y /= rhs;
        self.z /= rhs;
    }
}

/// Scalar multiplication: Vec3 * scalar.
impl<T> Mul<T> for Vec3<T>
where
    T: Copy + Mul<Output = T>,
{
    type Output = Self;
    fn mul(self, rhs: T) -> Self::Output {
        Self {
            x: self.x * rhs,
            y: self.y * rhs,
            z: self.z * rhs,
        }
    }
}

/// Scalar multiplication assignment: Vec3 *= scalar.
impl<T> MulAssign<T> for Vec3<T>
where
    T: Copy + MulAssign,
{
    fn mul_assign(&mut self, rhs: T) {
        self.x *= rhs;
        self.y *= rhs;
        self.z *= rhs;
    }
}

/// Componentwise negation.
impl<T> Neg for Vec3<T>
where
    T: Neg<Output = T>,
{
    type Output = Self;
    fn neg(self) -> Self::Output {
        Self {
            x: -self.x,
            y: -self.y,
            z: -self.z,
        }
    }
}

// ----------------------------------------------------------------------------
// Numeric operations (integer and floating-point components).
// ----------------------------------------------------------------------------

impl<T> Vec3<T>
where
    T: Copy + PartialOrd + Neg<Output = T> + Add<Output = T> + Sub<Output = T> + Mul<Output = T>,
{
    /// The dot product `self · other`.
    #[must_use]
    pub fn dot(self, other: Self) -> T {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    /// The cross product `self × other`.
    #[must_use]
    pub fn cross(self, other: Self) -> Self {
        Self {
            x: self.y * other.z - self.z * other.y,
            y: self.z * other.x - self.x * other.z,
            z: self.x * other.y - self.y * other.x,
        }
    }

    /// The squared length `|self|²`.
    #[must_use]
    pub fn length_squared(self) -> T {
        self.dot(self)
    }
}

impl<T: PartialOrd + Copy> Vec3<T> {
    /// Componentwise minimum.
    #[must_use]
    pub fn min(self, other: Self) -> Self {
        Self {
            x: if self.x < other.x { self.x } else { other.x },
            y: if self.y < other.y { self.y } else { other.y },
            z: if self.z < other.z { self.z } else { other.z },
        }
    }

    /// Componentwise maximum.
    #[must_use]
    pub fn max(self, other: Self) -> Self {
        Self {
            x: if self.x > other.x { self.x } else { other.x },
            y: if self.y > other.y { self.y } else { other.y },
            z: if self.z > other.z { self.z } else { other.z },
        }
    }

    /// Clamps every component to `[lo, hi]` (inclusive).
    #[must_use]
    pub fn clamp(self, lo: Self, hi: Self) -> Self {
        self.min(hi).max(lo)
    }
}

impl Vec3<i32> {
    /// The Manhattan length `|x| + |y| + |z|`.
    #[must_use]
    pub fn manhattan_length(self) -> i32 {
        self.abs().x + self.abs().y + self.abs().z
    }
}

impl Vec3<f32> {
    /// The Euclidean length `|self|`.
    #[must_use]
    pub fn length(self) -> f32 {
        self.length_squared().sqrt()
    }

    /// Returns a normalized version of the vector (length 1).
    /// Returns zero vector if length is zero.
    #[must_use]
    pub fn normalize(self) -> Self {
        let len = self.length();
        if len == 0.0 { Self::zero() } else { self / len }
    }

    /// The Euclidean distance between `self` and `other`.
    #[must_use]
    pub fn distance(self, other: Self) -> f32 {
        (self - other).length()
    }

    /// The squared Euclidean distance between `self` and `other`.
    #[must_use]
    pub fn distance_squared(self, other: Self) -> f32 {
        (self - other).length_squared()
    }

    /// Linear interpolation: `self + (other - self) * t`.
    #[must_use]
    pub fn lerp(self, other: Self, t: f32) -> Self {
        self.map2(other, |a, b| a + (b - a) * t)
    }
}

impl Vec3<f64> {
    /// The Euclidean length `|self|`.
    #[must_use]
    pub fn length(self) -> f64 {
        self.length_squared().sqrt()
    }

    /// Returns a normalized version of the vector (length 1).
    /// Returns zero vector if length is zero.
    #[must_use]
    pub fn normalize(self) -> Self {
        let len = self.length();
        if len == 0.0 { Self::zero() } else { self / len }
    }

    /// The Euclidean distance between `self` and `other`.
    #[must_use]
    pub fn distance(self, other: Self) -> f64 {
        (self - other).length()
    }

    /// The squared Euclidean distance between `self` and `other`.
    #[must_use]
    pub fn distance_squared(self, other: Self) -> f64 {
        (self - other).length_squared()
    }

    /// Linear interpolation: `self + (other - self) * t`.
    #[must_use]
    pub fn lerp(self, other: Self, t: f64) -> Self {
        self.map2(other, |a, b| a + (b - a) * t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_and_sub_are_componentwise() {
        let a = Vec3::new(1, 2, 3);
        let b = Vec3::new(4, 5, 6);
        assert_eq!(a + b, Vec3::new(5, 7, 9));
        assert_eq!(b - a, Vec3::new(3, 3, 3));
        let mut v = a;
        v += b;
        assert_eq!(v, Vec3::new(5, 7, 9));
    }

    #[test]
    fn neg_and_mul_and_div() {
        assert_eq!(-Vec3::new(1i32, -2, 3), Vec3::new(-1, 2, -3));
        assert_eq!(Vec3::new(2i32, 4, 6) * Vec3::splat(2), Vec3::new(4, 8, 12));
        assert_eq!(Vec3::new(4i32, 8, 12) / Vec3::splat(2), Vec3::new(2, 4, 6));
    }

    #[test]
    fn dot_cross_and_length() {
        let a = Vec3::new(1, 2, 3);
        let b = Vec3::new(4, 5, 6);
        assert_eq!(a.dot(b), 32);
        assert_eq!(a.cross(b), Vec3::new(-3, 6, -3));
        assert_eq!(Vec3::new(3.0_f32, 4.0, 0.0).length(), 5.0_f32);
        assert_eq!(
            Vec3::new(0.0_f32, 0.0, 0.0).distance(Vec3::new(3.0, 4.0, 12.0)),
            13.0_f32
        );
    }

    #[test]
    fn min_max_and_clamp() {
        let a = Vec3::new(1, 5, 3);
        let b = Vec3::new(4, 2, 6);
        assert_eq!(a.min(b), Vec3::new(1, 2, 3));
        assert_eq!(a.max(b), Vec3::new(4, 5, 6));
        assert_eq!(
            Vec3::new(-1, 7, 4).clamp(Vec3::new(0, 0, 0), Vec3::new(5, 5, 5)),
            Vec3::new(0, 5, 4)
        );
    }

    #[test]
    fn array_round_trip() {
        let v = Vec3::new(1, 2, 3);
        assert_eq!(Vec3::from_array(v.to_array()), v);
    }

    #[test]
    fn cast_changes_component_type() {
        let v = Vec3::new(1, 2, 3).cast::<f64>();
        assert_eq!(v, Vec3::new(1.0, 2.0, 3.0));
    }

    #[test]
    fn i32_manhattan_length() {
        assert_eq!(Vec3::new(-3, 4, -12).manhattan_length(), 19);
    }
}
