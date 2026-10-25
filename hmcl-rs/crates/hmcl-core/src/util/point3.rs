//! A point or a size in three dimensions.
//!
//! The launcher reads the size of the schematics and the positions of the
//! structures the player keeps, so it needs a value of three whole numbers.

/// A point or a size in three dimensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Point3I {
    /// The position on the axis of the x.
    pub x: i32,
    /// The position on the axis of the y.
    pub y: i32,
    /// The position on the axis of the z.
    pub z: i32,
}

impl Point3I {
    /// Creates a point of three dimensions.
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    /// Returns the number of the positions of the box the point describes.
    ///
    /// A point with a negative side gives a negative volume, so the caller
    /// checks the sides before it uses the volume.
    pub const fn volume(self) -> i64 {
        self.x as i64 * self.y as i64 * self.z as i64
    }

    /// Returns whether every side is positive.
    pub const fn is_positive(self) -> bool {
        self.x > 0 && self.y > 0 && self.z > 0
    }
}

impl std::fmt::Display for Point3I {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}x{}x{}", self.x, self.y, self.z)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_volume_of_the_points_is_computed() {
        assert_eq!(Point3I::new(2, 3, 4).volume(), 24);
        assert_eq!(Point3I::new(0, 3, 4).volume(), 0);
        assert!(Point3I::new(2, 3, 4).is_positive());
        assert!(!Point3I::new(-2, 3, 4).is_positive());
    }

    #[test]
    fn the_points_are_written() {
        assert_eq!(Point3I::new(2, 3, 4).to_string(), "2x3x4");
    }
}
