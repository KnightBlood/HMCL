//! A range of versions.
//!
//! The launcher uses the ranges to tell whether a version of Minecraft
//! supports a feature, for example whether the version is between `1.13`
//! and `1.20`.

use std::fmt;

/// The kind of a range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// The range holds no version at all.
    Empty,
    /// The range holds every version.
    All,
    /// The range holds the versions between two versions.
    Bounded,
}

/// A range of versions.
///
/// A range has a lowest and a highest version; a range without a lowest or
/// without a highest version is open on that side.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionRange<T> {
    /// The kind of the range.
    kind: Kind,
    /// The lowest version of the range, if it has one.
    minimum: Option<T>,
    /// The highest version of the range, if it has one.
    maximum: Option<T>,
}

impl<T> VersionRange<T> {
    /// Creates a range that holds no version at all.
    pub fn empty() -> Self {
        Self {
            kind: Kind::Empty,
            minimum: None,
            maximum: None,
        }
    }

    /// Creates a range that holds every version.
    pub fn all() -> Self {
        Self {
            kind: Kind::All,
            minimum: None,
            maximum: None,
        }
    }

    /// Creates a range that holds the versions between the two versions.
    pub fn between(minimum: T, maximum: T) -> Self {
        Self {
            kind: Kind::Bounded,
            minimum: Some(minimum),
            maximum: Some(maximum),
        }
    }

    /// Creates a range that holds the versions that are not older than the
    /// given version.
    pub fn at_least(minimum: T) -> Self {
        Self {
            kind: Kind::Bounded,
            minimum: Some(minimum),
            maximum: None,
        }
    }

    /// Creates a range that holds the versions that are not newer than the
    /// given version.
    pub fn at_most(maximum: T) -> Self {
        Self {
            kind: Kind::Bounded,
            minimum: None,
            maximum: Some(maximum),
        }
    }

    /// Returns whether the range holds no version at all.
    pub fn is_empty(&self) -> bool {
        self.kind == Kind::Empty
    }

    /// Returns whether the range holds every version.
    pub fn is_all(&self) -> bool {
        self.kind == Kind::All
    }

    /// Returns the lowest version of the range, if it has one.
    pub fn minimum(&self) -> Option<&T> {
        self.minimum.as_ref()
    }

    /// Returns the highest version of the range, if it has one.
    pub fn maximum(&self) -> Option<&T> {
        self.maximum.as_ref()
    }
}

impl<T: Clone> VersionRange<T> {
    /// Creates a range that holds only the given version.
    pub fn is(version: T) -> Self {
        Self {
            kind: Kind::Bounded,
            minimum: Some(version.clone()),
            maximum: Some(version),
        }
    }
}

impl<T: Ord + Clone> VersionRange<T> {
    /// Returns whether the range holds the given version.
    pub fn contains(&self, version: &T) -> bool {
        match self.kind {
            Kind::Empty => false,
            Kind::All => true,
            Kind::Bounded => {
                self.minimum
                    .as_ref()
                    .is_none_or(|minimum| minimum <= version)
                    && self
                        .maximum
                        .as_ref()
                        .is_none_or(|maximum| maximum >= version)
            }
        }
    }

    /// Returns whether the two ranges have a version in common.
    pub fn is_overlapped_by(&self, that: &Self) -> bool {
        if self.is_empty() || that.is_empty() {
            return false;
        }

        if self.is_all() || that.is_all() {
            return true;
        }

        match (&self.minimum, &self.maximum) {
            // The range is open on both sides, so it holds every version.
            (None, None) => true,
            (None, Some(maximum)) => that
                .minimum
                .as_ref()
                .is_none_or(|minimum| minimum <= maximum),
            (Some(minimum), None) => that
                .maximum
                .as_ref()
                .is_none_or(|maximum| maximum >= minimum),
            (Some(minimum), Some(maximum)) => {
                that.contains(minimum)
                    || that.contains(maximum)
                    || that
                        .minimum
                        .as_ref()
                        .is_some_and(|minimum| self.contains(minimum))
            }
        }
    }

    /// Returns the versions the two ranges have in common.
    pub fn intersection_with(&self, that: &Self) -> Self {
        if self.is_all() {
            return that.clone();
        }
        if that.is_all() {
            return self.clone();
        }

        if !self.is_overlapped_by(that) {
            return Self::empty();
        }

        let minimum = match (&self.minimum, &that.minimum) {
            (None, other) => other.clone(),
            (Some(_), None) => self.minimum.clone(),
            (Some(left), Some(right)) => Some(if left >= right {
                left.clone()
            } else {
                right.clone()
            }),
        };

        let maximum = match (&self.maximum, &that.maximum) {
            (None, other) => other.clone(),
            (Some(_), None) => self.maximum.clone(),
            (Some(left), Some(right)) => Some(if left <= right {
                left.clone()
            } else {
                right.clone()
            }),
        };

        Self {
            kind: Kind::Bounded,
            minimum,
            maximum,
        }
    }
}

impl<T: fmt::Display> fmt::Display for VersionRange<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            Kind::Empty => formatter.write_str("EMPTY"),
            Kind::All => formatter.write_str("ALL"),
            Kind::Bounded => match (&self.minimum, &self.maximum) {
                (None, Some(maximum)) => write!(formatter, "At most {maximum}"),
                (Some(minimum), None) => write!(formatter, "At least {minimum}"),
                (Some(minimum), Some(maximum)) => write!(formatter, "[{minimum}..{maximum}]"),
                (None, None) => formatter.write_str("ALL"),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_empty_range_holds_nothing() {
        let range = VersionRange::<i32>::empty();

        assert!(range.is_empty());
        assert!(!range.is_all());
        assert!(!range.contains(&1));
        assert_eq!(range.to_string(), "EMPTY");
    }

    #[test]
    fn the_full_range_holds_everything() {
        let range = VersionRange::<i32>::all();

        assert!(range.is_all());
        assert!(!range.is_empty());
        assert!(range.contains(&1));
        assert!(range.contains(&100));
        assert_eq!(range.to_string(), "ALL");
    }

    #[test]
    fn the_bounded_range_holds_the_versions_between_the_two() {
        let range = VersionRange::between(10, 20);

        assert!(range.contains(&10));
        assert!(range.contains(&15));
        assert!(range.contains(&20));
        assert!(!range.contains(&9));
        assert!(!range.contains(&21));
        assert_eq!(range.to_string(), "[10..20]");
    }

    #[test]
    fn the_open_ranges_hold_the_versions_on_one_side() {
        let range = VersionRange::at_least(10);
        assert!(range.contains(&10));
        assert!(range.contains(&1000));
        assert!(!range.contains(&9));
        assert_eq!(range.to_string(), "At least 10");

        let range = VersionRange::at_most(10);
        assert!(range.contains(&10));
        assert!(range.contains(&-1000));
        assert!(!range.contains(&11));
        assert_eq!(range.to_string(), "At most 10");
    }

    #[test]
    fn the_range_of_one_version_holds_only_it() {
        let range = VersionRange::is(10);

        assert!(range.contains(&10));
        assert!(!range.contains(&11));
        assert!(!range.contains(&9));
    }

    #[test]
    fn the_ranges_that_have_a_version_in_common_are_overlapped() {
        assert!(VersionRange::between(10, 20).is_overlapped_by(&VersionRange::between(15, 25)));
        assert!(VersionRange::between(10, 20).is_overlapped_by(&VersionRange::at_least(15)));
        assert!(VersionRange::between(10, 20).is_overlapped_by(&VersionRange::at_most(15)));
        assert!(!VersionRange::between(10, 20).is_overlapped_by(&VersionRange::between(21, 30)));
        assert!(VersionRange::all().is_overlapped_by(&VersionRange::between(1, 2)));
        assert!(!VersionRange::<i32>::empty().is_overlapped_by(&VersionRange::all()));
    }

    #[test]
    fn the_intersection_of_two_ranges_is_the_versions_they_share() {
        let intersection =
            VersionRange::between(10, 20).intersection_with(&VersionRange::between(15, 25));
        assert!(intersection.contains(&15));
        assert!(intersection.contains(&20));
        assert!(!intersection.contains(&21));

        let intersection =
            VersionRange::between(10, 20).intersection_with(&VersionRange::between(30, 40));
        assert!(intersection.is_empty());

        let intersection = VersionRange::all().intersection_with(&VersionRange::between(10, 20));
        assert_eq!(intersection, VersionRange::between(10, 20));
    }
}
