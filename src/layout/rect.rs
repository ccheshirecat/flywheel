//! Rect: A rectangle primitive for layout calculations.

/// A rectangle defined by position and size.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Rect {
    /// X coordinate (column) of the top-left corner.
    pub x: u16,
    /// Y coordinate (row) of the top-left corner.
    pub y: u16,
    /// Width in columns.
    pub width: u16,
    /// Height in rows.
    pub height: u16,
}

impl Rect {
    /// Create a new rectangle.
    #[inline]
    pub const fn new(x: u16, y: u16, width: u16, height: u16) -> Self {
        Self { x, y, width, height }
    }

    /// Create a rectangle from a terminal size (full screen).
    #[inline]
    pub const fn from_size(width: u16, height: u16) -> Self {
        Self::new(0, 0, width, height)
    }

    /// Zero-sized rectangle.
    pub const ZERO: Self = Self::new(0, 0, 0, 0);

    /// Get the area (number of cells).
    #[inline]
    pub const fn area(&self) -> u32 {
        (self.width as u32) * (self.height as u32)
    }

    /// Check if the rectangle is empty.
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.width == 0 || self.height == 0
    }

    /// Get the right edge (exclusive).
    #[inline]
    pub const fn right(&self) -> u16 {
        self.x.saturating_add(self.width)
    }

    /// Get the bottom edge (exclusive).
    #[inline]
    pub const fn bottom(&self) -> u16 {
        self.y.saturating_add(self.height)
    }

    /// Check if a point is inside the rectangle.
    #[inline]
    pub const fn contains(&self, x: u16, y: u16) -> bool {
        x >= self.x && x < self.right() && y >= self.y && y < self.bottom()
    }

    /// Check if this rectangle intersects with another.
    #[inline]
    pub const fn intersects(&self, other: &Self) -> bool {
        self.x < other.right()
            && self.right() > other.x
            && self.y < other.bottom()
            && self.bottom() > other.y
    }

    /// Shrink the rectangle by a margin on all sides.
    #[inline]
    #[must_use]
    pub const fn shrink(&self, margin: u16) -> Self {
        let m2 = margin * 2;
        if self.width <= m2 || self.height <= m2 {
            return Self::ZERO;
        }
        Self::new(self.x + margin, self.y + margin, self.width - m2, self.height - m2)
    }

    /// Split horizontally at a given column offset.
    pub fn split_horizontal(&self, at: u16) -> (Self, Self) {
        let at = at.min(self.width);
        (
            Self::new(self.x, self.y, at, self.height),
            Self::new(self.x + at, self.y, self.width - at, self.height),
        )
    }

    /// Split vertically at a given row offset.
    pub fn split_vertical(&self, at: u16) -> (Self, Self) {
        let at = at.min(self.height);
        (
            Self::new(self.x, self.y, self.width, at),
            Self::new(self.x, self.y + at, self.width, self.height - at),
        )
    }
}

impl std::fmt::Debug for Rect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Rect({}, {} {}x{})", self.x, self.y, self.width, self.height)
    }
}
