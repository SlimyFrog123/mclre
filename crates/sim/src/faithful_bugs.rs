//! Faithful bugs feature flag module.
//!
//! This module provides access to the `faithful_bugs` feature flag which enables
//! documented deviations from reference behavior that are known bugs in the baseline
//! but must be preserved for world/compatibility (SPEC §3.3).

/// Check if the `faithful_bugs` feature is enabled.
#[cfg(feature = "faithful_bugs")]
pub fn is_enabled() -> bool {
    true
}

/// Check if the `faithful_bugs` feature is enabled.
#[cfg(not(feature = "faithful_bugs"))]
pub fn is_enabled() -> bool {
    false
}

/// Returns the faithful value if `faithful_bugs` is enabled, otherwise the corrected value.
#[cfg(feature = "faithful_bugs")]
pub fn choose<T>(faithful: T, _corrected: T) -> T {
    faithful
}

/// Returns the faithful value if `faithful_bugs` is enabled, otherwise the corrected value.
#[cfg(not(feature = "faithful_bugs"))]
pub fn choose<T>(_faithful: T, corrected: T) -> T {
    corrected
}

/// Trait for behaviors that differ based on the faithful_bugs flag.
/// Implementations can provide different logic for faithful vs corrected behavior.
pub trait FaithfulBehavior<T> {
    /// The behavior when faithful_bugs is enabled (replicates baseline bug).
    fn faithful(&self) -> T;

    /// The behavior when faithful_bugs is disabled (corrected behavior).
    fn corrected(&self) -> T;

    /// Returns the appropriate behavior based on the feature flag.
    fn get(&self) -> T {
        if crate::faithful_bugs::is_enabled() {
            self.faithful()
        } else {
            self.corrected()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_faithful_bugs_choose() {
        let result = choose(42, 0);
        #[cfg(feature = "faithful_bugs")]
        assert_eq!(result, 42);
        #[cfg(not(feature = "faithful_bugs"))]
        assert_eq!(result, 0);
    }

    #[test]
    fn test_faithful_behavior_trait() {
        struct TestBehavior;
        impl FaithfulBehavior<i32> for TestBehavior {
            fn faithful(&self) -> i32 { 100 }
            fn corrected(&self) -> i32 { 200 }
        }
        let behavior = TestBehavior;
        #[cfg(feature = "faithful_bugs")]
        assert_eq!(behavior.get(), 100);
        #[cfg(not(feature = "faithful_bugs"))]
        assert_eq!(behavior.get(), 200);
    }
}