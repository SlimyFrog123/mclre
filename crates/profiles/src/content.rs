//! Per-entry content availability: `since`/`until` profile bounds (SPEC.md 6.1).
//!
//! Content registration is not limited to the newest profile: one codebase
//! serves all profiles, so an entry (block, item, recipe, mob, biome) declares
//! the window of profiles in which it exists. Bounds are compared through
//! [`ProfileOrder`], a monotonic rank assigned to profiles in the version
//! timeline, so comparisons never parse or order version strings.

/// Position of a profile in the version timeline (monotonic; `0` = oldest).
///
/// Ranks are data: an update-by-update train assigns each profile the next
/// rank. They must never be reordered once saves exist.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProfileOrder(u32);

impl ProfileOrder {
    /// The rank of the oldest supported profile.
    pub const FIRST: Self = Self(0);

    /// Creates a rank from its numeric value.
    pub const fn new(rank: u32) -> Self {
        Self(rank)
    }

    /// The numeric rank.
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// The inclusive profile window in which a content entry exists.
///
/// Both bounds are inclusive and optional: `None` means "since the beginning"
/// / "forever". The default (`always()`) applies to content present in every
/// profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ContentBounds {
    since: Option<ProfileOrder>,
    until: Option<ProfileOrder>,
}

impl ContentBounds {
    /// Present in every profile.
    pub const fn always() -> Self {
        Self {
            since: None,
            until: None,
        }
    }

    /// Present from `order` onward (inclusive).
    pub const fn since(order: ProfileOrder) -> Self {
        Self {
            since: Some(order),
            until: None,
        }
    }

    /// Present up to and including `order`.
    pub const fn until(order: ProfileOrder) -> Self {
        Self {
            since: None,
            until: Some(order),
        }
    }

    /// Present from `since` to `until` inclusive.
    ///
    /// Returns `None` when the window is empty (`since > until`), which is a
    /// content-authoring bug rather than a valid entry.
    pub fn between(since: ProfileOrder, until: ProfileOrder) -> Option<Self> {
        if since > until {
            None
        } else {
            Some(Self {
                since: Some(since),
                until: Some(until),
            })
        }
    }

    /// Whether an entry with these bounds exists in the profile at `order`.
    pub fn contains(&self, order: ProfileOrder) -> bool {
        self.since.is_none_or(|start| order >= start) && self.until.is_none_or(|end| order <= end)
    }

    /// The lower bound, if any.
    pub const fn since_order(&self) -> Option<ProfileOrder> {
        self.since
    }

    /// The upper bound, if any.
    pub const fn until_order(&self) -> Option<ProfileOrder> {
        self.until
    }
}

impl Default for ContentBounds {
    fn default() -> Self {
        Self::always()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const OLD: ProfileOrder = ProfileOrder::new(0);
    const MID: ProfileOrder = ProfileOrder::new(1);
    const NEW: ProfileOrder = ProfileOrder::new(2);

    #[test]
    fn always_contains_every_rank() {
        let bounds = ContentBounds::always();
        assert!(bounds.contains(OLD));
        assert!(bounds.contains(MID));
        assert!(bounds.contains(NEW));
        assert_eq!(bounds, ContentBounds::default());
    }

    #[test]
    fn since_is_inclusive() {
        let bounds = ContentBounds::since(MID);
        assert!(!bounds.contains(OLD));
        assert!(bounds.contains(MID));
        assert!(bounds.contains(NEW));
    }

    #[test]
    fn until_is_inclusive() {
        let bounds = ContentBounds::until(MID);
        assert!(bounds.contains(OLD));
        assert!(bounds.contains(MID));
        assert!(!bounds.contains(NEW));
    }

    #[test]
    fn between_checks_both_ends() {
        let bounds = ContentBounds::between(MID, NEW).expect("non-empty window");
        assert!(!bounds.contains(OLD));
        assert!(bounds.contains(MID));
        assert!(bounds.contains(NEW));
        assert_eq!(ContentBounds::between(NEW, MID), None);
    }

    #[test]
    fn single_profile_window() {
        let bounds = ContentBounds::between(MID, MID).expect("non-empty window");
        assert!(bounds.contains(MID));
        assert!(!bounds.contains(OLD));
        assert!(!bounds.contains(NEW));
        assert_eq!(bounds.since_order(), Some(MID));
        assert_eq!(bounds.until_order(), Some(MID));
    }
}
