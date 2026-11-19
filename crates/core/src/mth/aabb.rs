//! Axis-aligned bounding boxes (`AABB<T>`).
//!
//! An AABB is a pair of [`Vec3`] corners (`min`, `max`). It is the shared
//! bounding-box type used for selection, collision and culling logic, so that
//! containment, intersection and merging are implemented once.

use std::ops::{Mul, Sub};

use crate::mth::Vec3;

/// An axis-aligned bounding box, defined by a minimum and a maximum corner.
///
/// `min` should be componentwise `<= max`; methods that assume a valid box
/// document that requirement. There is no runtime enforcement so constant
/// contexts and zero-sized boxes stay usable.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AABB<T> {
    /// Minimum corner (inclusive).
    pub min: Vec3<T>,
    /// Maximum corner (inclusive).
    pub max: Vec3<T>,
}

impl<T: PartialOrd + Copy> AABB<T> {
    /// Creates a box from corners. Callers must ensure `min <= max` per axis
    /// if they rely on validity-sensitive methods.
    #[must_use]
    pub const fn new(min: Vec3<T>, max: Vec3<T>) -> Self {
        Self { min, max }
    }

    /// Builds a box enclosing `points`. Returns `None` for an empty iterator.
    ///
    /// This is the fallible constructor; use [`AABB::new`] when the corners
    /// are already known.
    pub fn from_points<I>(points: I) -> Option<Self>
    where
        I: IntoIterator<Item = Vec3<T>>,
    {
        let mut iter = points.into_iter();
        let first = iter.next()?;
        let mut min = first;
        let mut max = first;
        for point in iter {
            min = min.min(point);
            max = max.max(point);
        }
        Some(Self::new(min, max))
    }

    /// Whether `point` is inside the box (inclusive on every axis).
    #[must_use]
    pub fn contains(&self, point: Vec3<T>) -> bool {
        self.min.x <= point.x
            && point.x <= self.max.x
            && self.min.y <= point.y
            && point.y <= self.max.y
            && self.min.z <= point.z
            && point.z <= self.max.z
    }

    /// Whether `other` overlaps this box (touching counts as overlapping).
    #[must_use]
    pub fn intersects(&self, other: Self) -> bool {
        self.min.x <= other.max.x
            && other.min.x <= self.max.x
            && self.min.y <= other.max.y
            && other.min.y <= self.max.y
            && self.min.z <= other.max.z
            && other.min.z <= self.max.z
    }

    /// The smallest box containing both `self` and `other`.
    #[must_use]
    pub fn union(self, other: Self) -> Self {
        Self {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }

    /// Returns a box grown to also include `point`.
    #[must_use]
    pub fn expand(self, point: Vec3<T>) -> Self {
        Self {
            min: self.min.min(point),
            max: self.max.max(point),
        }
    }

    /// Whether every component of `min` is `<=` the corresponding component of
    /// `max` (a valid, non-degenerate-or-empty box).
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.min.x <= self.max.x && self.min.y <= self.max.y && self.min.z <= self.max.z
    }

    /// The center of the box, computed in `f64` to avoid integer truncation.
    #[must_use]
    pub fn center_f64(self) -> Vec3<f64>
    where
        T: Into<f64> + Copy,
    {
        let lo = self.min.cast::<f64>();
        let hi = self.max.cast::<f64>();
        (lo + hi) * Vec3::splat(0.5)
    }

    /// The volume `(max - min).x * (max - min).y * (max - min).z` (zero if
    /// the box is degenerate). May overflow for wide integer boxes.
    #[must_use]
    pub fn volume(self) -> T
    where
        T: Sub<Output = T> + Mul<Output = T> + Copy,
    {
        let d = self.max - self.min;
        d.x * d.y * d.z
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contains_and_intersects() {
        let boxed = AABB::new(Vec3::new(0, 0, 0), Vec3::new(4, 4, 4));
        assert!(boxed.contains(Vec3::new(2, 2, 2)));
        assert!(boxed.contains(Vec3::new(0, 0, 0)));
        assert!(boxed.contains(Vec3::new(4, 4, 4)));
        assert!(!boxed.contains(Vec3::new(5, 2, 2)));
        assert!(!boxed.contains(Vec3::new(2, -1, 2)));

        let other = AABB::new(Vec3::new(3, 3, 3), Vec3::new(6, 6, 6));
        assert!(boxed.intersects(other));
        let far = AABB::new(Vec3::new(8, 8, 8), Vec3::new(10, 10, 10));
        assert!(!boxed.intersects(far));
    }

    #[test]
    fn union_and_expand_enclose() {
        let a = AABB::new(Vec3::new(0, 0, 0), Vec3::new(2, 2, 2));
        let b = AABB::new(Vec3::new(1, 1, 1), Vec3::new(3, 3, 3));
        assert_eq!(
            a.union(b),
            AABB::new(Vec3::new(0, 0, 0), Vec3::new(3, 3, 3))
        );
        assert_eq!(
            a.expand(Vec3::new(5, 5, 5)),
            AABB::new(Vec3::new(0, 0, 0), Vec3::new(5, 5, 5))
        );
    }

    #[test]
    fn from_points_handles_empty_and_single() {
        assert!(AABB::<i32>::from_points(std::iter::empty::<Vec3<i32>>()).is_none());
        let single = AABB::from_points([Vec3::new(1, 2, 3)]).unwrap();
        assert_eq!(single.min, Vec3::new(1, 2, 3));
        assert_eq!(single.max, Vec3::new(1, 2, 3));
        let boxed = AABB::from_points([Vec3::new(-1, 0, 2), Vec3::new(3, 4, -5)]).unwrap();
        assert_eq!(boxed.min, Vec3::new(-1, 0, -5));
        assert_eq!(boxed.max, Vec3::new(3, 4, 2));
    }

    #[test]
    fn center_and_volume() {
        let boxed = AABB::new(Vec3::new(0, 0, 0), Vec3::new(4, 4, 4));
        assert_eq!(boxed.center_f64(), Vec3::new(2.0, 2.0, 2.0));
        assert_eq!(boxed.volume(), 64);
        let degenerate = AABB::new(Vec3::new(0, 0, 0), Vec3::new(0, 4, 4));
        assert_eq!(degenerate.volume(), 0);
        assert!(degenerate.is_valid());
        let invalid = AABB::new(Vec3::new(2, 0, 0), Vec3::new(0, 4, 4));
        assert!(!invalid.is_valid());
    }
}
