//! Buffer: A grid of cells representing the terminal screen.
//!
//! The buffer uses contiguous memory allocation for cache efficiency.
//! Cells are stored in row-major order.

use super::cell::{Cell, CellFlags, Rgb};
use std::collections::HashMap;

/// A grid of cells representing the terminal screen.
///
/// The buffer stores cells in a contiguous `Vec` for cache efficiency.
/// Access is in row-major order: `index = y * width + x`.
///
/// # Overflow Storage
///
/// Complex graphemes (> 4 bytes) are stored in a separate `HashMap`.
/// The cell contains an index into this overflow storage when the
/// `OVERFLOW` flag is set.
#[derive(Clone)]
pub struct Buffer {
    /// Contiguous cell storage (row-major order).
    cells: Vec<Cell>,
    /// Terminal width in columns.
    width: u16,
    /// Terminal height in rows.
    height: u16,
    /// Overflow storage for complex graphemes.
    overflow: HashMap<u32, String>,
    /// Next overflow index to assign.
    next_overflow_index: u32,
}

impl Buffer {
    /// Create a new buffer with the given dimensions.
    ///
    /// All cells are initialized to empty (space with default colors).
    ///
    /// # Panics
    /// Panics if width or height is 0.
    pub fn new(width: u16, height: u16) -> Self {
        assert!(width > 0 && height > 0, "Buffer dimensions must be non-zero");
        let size = (width as usize) * (height as usize);
        Self {
            cells: vec![Cell::EMPTY; size],
            width,
            height,
            overflow: HashMap::new(),
            next_overflow_index: 0,
        }
    }

    /// Get the buffer width.
    #[inline]
    pub const fn width(&self) -> u16 {
        self.width
    }

    /// Get the buffer height.
    #[inline]
    pub const fn height(&self) -> u16 {
        self.height
    }

    /// Get the total number of cells.
    #[inline]
    pub fn len(&self) -> usize {
        self.cells.len()
    }

    /// Check if the buffer is empty (should never be true after construction).
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    /// Get a reference to the underlying cell slice.
    #[inline]
    pub fn cells(&self) -> &[Cell] {
        &self.cells
    }

    /// Get a mutable reference to the underlying cell slice.
    #[inline]
    pub fn cells_mut(&mut self) -> &mut [Cell] {
        &mut self.cells
    }

    /// Convert (x, y) coordinates to a linear index.
    ///
    /// Returns `None` if coordinates are out of bounds.
    #[inline]
    pub fn index_of(&self, x: u16, y: u16) -> Option<usize> {
        if x < self.width && y < self.height {
            Some((y as usize) * (self.width as usize) + (x as usize))
        } else {
            None
        }
    }

    /// Convert a linear index to (x, y) coordinates.
    #[inline]
    pub fn coords_of(&self, index: usize) -> Option<(u16, u16)> {
        if index < self.cells.len() {
            let x = (index % (self.width as usize)) as u16;
            let y = (index / (self.width as usize)) as u16;
            Some((x, y))
        } else {
            None
        }
    }

    /// Get a reference to a cell at (x, y).
    ///
    /// Returns `None` if coordinates are out of bounds.
    #[inline]
    pub fn get(&self, x: u16, y: u16) -> Option<&Cell> {
        self.index_of(x, y).map(|i| &self.cells[i])
    }

    /// Get a mutable reference to a cell at (x, y).
    ///
    /// Returns `None` if coordinates are out of bounds.
    #[inline]
    pub fn get_mut(&mut self, x: u16, y: u16) -> Option<&mut Cell> {
        self.index_of(x, y).map(|i| &mut self.cells[i])
    }

    /// Set a cell at (x, y).
    ///
    /// Returns `false` if coordinates are out of bounds.
    #[inline]
    pub fn set(&mut self, x: u16, y: u16, cell: Cell) -> bool {
        if let Some(idx) = self.index_of(x, y) {
            self.cells[idx] = cell;
            true
        } else {
            false
        }
    }

    /// Set a grapheme at (x, y), handling overflow automatically.
    ///
    /// For wide characters (CJK), this also sets a continuation cell
    /// at (x+1, y).
    ///
    /// Returns the display width of the grapheme, or 0 if out of bounds.
    pub fn set_grapheme(&mut self, x: u16, y: u16, grapheme: &str, fg: Rgb, bg: Rgb) -> u8 {
        let Some(idx) = self.index_of(x, y) else {
            return 0;
        };

        let width = unicode_width::UnicodeWidthStr::width(grapheme) as u8;

        // Try to create an inline cell
        let cell = if let Some(mut cell) = Cell::from_grapheme(grapheme) {
            cell.set_fg(fg).set_bg(bg);
            cell
        } else {
            // Overflow: store in HashMap
            let overflow_idx = self.next_overflow_index;
            self.next_overflow_index += 1;
            self.overflow.insert(overflow_idx, grapheme.to_string());
            Cell::overflow(overflow_idx, width).with_fg(fg).with_bg(bg)
        };

        self.cells[idx] = cell;

        // Handle wide characters (CJK)
        if width == 2 {
            if let Some(next_idx) = self.index_of(x + 1, y) {
                self.cells[next_idx] = Cell::wide_continuation().with_bg(bg);
            }
        }

        width
    }

    /// Get the grapheme at (x, y), including overflow lookup.
    ///
    /// Returns `None` if out of bounds or if it's a continuation cell.
    pub fn get_grapheme(&self, x: u16, y: u16) -> Option<&str> {
        let cell = self.get(x, y)?;

        if cell.is_wide_continuation() {
            return None;
        }

        if cell.flags().contains(CellFlags::OVERFLOW) {
            let idx = cell.overflow_index()?;
            self.overflow.get(&idx).map(String::as_str)
        } else {
            cell.grapheme()
        }
    }

    /// Get an overflow grapheme by its index.
    ///
    /// This is used by the diffing engine when rendering overflow cells.
    #[inline]
    pub fn get_overflow(&self, index: u32) -> Option<&str> {
        self.overflow.get(&index).map(String::as_str)
    }

    /// Fill a rectangular region with a cell.
    pub fn fill_rect(&mut self, x: u16, y: u16, width: u16, height: u16, cell: Cell) {
        for row in y..(y + height).min(self.height) {
            for col in x..(x + width).min(self.width) {
                if let Some(idx) = self.index_of(col, row) {
                    self.cells[idx] = cell;
                }
            }
        }
    }

    /// Clear the entire buffer (fill with empty cells).
    pub fn clear(&mut self) {
        self.cells.fill(Cell::EMPTY);
        self.overflow.clear();
        self.next_overflow_index = 0;
    }

    /// Clear a rectangular region.
    pub fn clear_rect(&mut self, x: u16, y: u16, width: u16, height: u16) {
        self.fill_rect(x, y, width, height, Cell::EMPTY);
    }

    /// Resize the buffer, preserving content where possible.
    ///
    /// New cells are initialized to empty.
    pub fn resize(&mut self, new_width: u16, new_height: u16) {
        if new_width == self.width && new_height == self.height {
            return;
        }

        let new_size = (new_width as usize) * (new_height as usize);
        let mut new_cells = vec![Cell::EMPTY; new_size];

        // Copy existing content
        let copy_width = self.width.min(new_width) as usize;
        let copy_height = self.height.min(new_height) as usize;

        for y in 0..copy_height {
            let old_start = y * (self.width as usize);
            let new_start = y * (new_width as usize);
            new_cells[new_start..new_start + copy_width]
                .copy_from_slice(&self.cells[old_start..old_start + copy_width]);
        }

        self.cells = new_cells;
        self.width = new_width;
        self.height = new_height;
    }

    /// Copy content from another buffer.
    ///
    /// The buffers must have the same dimensions.
    pub fn copy_from(&mut self, other: &Buffer) {
        debug_assert_eq!(self.width, other.width);
        debug_assert_eq!(self.height, other.height);
        self.cells.copy_from_slice(&other.cells);
        self.overflow.clone_from(&other.overflow);
        self.next_overflow_index = other.next_overflow_index;
    }

    /// Swap the contents of two buffers.
    ///
    /// This is O(1) - just pointer swaps.
    pub fn swap(&mut self, other: &mut Buffer) {
        std::mem::swap(&mut self.cells, &mut other.cells);
        std::mem::swap(&mut self.width, &mut other.width);
        std::mem::swap(&mut self.height, &mut other.height);
        std::mem::swap(&mut self.overflow, &mut other.overflow);
        std::mem::swap(&mut self.next_overflow_index, &mut other.next_overflow_index);
    }

    /// Get an iterator over rows.
    pub fn rows(&self) -> impl Iterator<Item = &[Cell]> {
        self.cells.chunks(self.width as usize)
    }

    /// Get a mutable iterator over rows.
    pub fn rows_mut(&mut self) -> impl Iterator<Item = &mut [Cell]> {
        self.cells.chunks_mut(self.width as usize)
    }

    /// Get memory usage in bytes (approximate).
    pub fn memory_usage(&self) -> usize {
        let cells_size = self.cells.len() * std::mem::size_of::<Cell>();
        let overflow_size: usize = self.overflow.values().map(|s| s.len() + 32).sum();
        cells_size + overflow_size + std::mem::size_of::<Self>()
    }
}

impl std::fmt::Debug for Buffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Buffer")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("overflow_count", &self.overflow.len())
            .field("memory_bytes", &self.memory_usage())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_new() {
        let buffer = Buffer::new(80, 24);
        assert_eq!(buffer.width(), 80);
        assert_eq!(buffer.height(), 24);
        assert_eq!(buffer.len(), 80 * 24);
    }

    #[test]
    #[should_panic]
    fn test_buffer_zero_width() {
        Buffer::new(0, 24);
    }

    #[test]
    fn test_buffer_get_set() {
        let mut buffer = Buffer::new(80, 24);
        let cell = Cell::new('X');
        assert!(buffer.set(5, 10, cell));
        assert_eq!(buffer.get(5, 10).unwrap().grapheme(), Some("X"));
    }

    #[test]
    fn test_buffer_bounds() {
        let buffer = Buffer::new(80, 24);
        assert!(buffer.get(79, 23).is_some());
        assert!(buffer.get(80, 23).is_none());
        assert!(buffer.get(79, 24).is_none());
    }

    #[test]
    fn test_buffer_index_coords() {
        let buffer = Buffer::new(80, 24);
        assert_eq!(buffer.index_of(5, 10), Some(10 * 80 + 5));
        assert_eq!(buffer.coords_of(10 * 80 + 5), Some((5, 10)));
    }

    #[test]
    fn test_buffer_set_grapheme() {
        let mut buffer = Buffer::new(80, 24);

        // ASCII
        let width = buffer.set_grapheme(0, 0, "A", Rgb::WHITE, Rgb::BLACK);
        assert_eq!(width, 1);
        assert_eq!(buffer.get_grapheme(0, 0), Some("A"));

        // CJK (wide character)
        let width = buffer.set_grapheme(5, 0, "日", Rgb::WHITE, Rgb::BLACK);
        assert_eq!(width, 2);
        assert_eq!(buffer.get_grapheme(5, 0), Some("日"));
        // Continuation cell should return None
        assert!(buffer.get(6, 0).unwrap().is_wide_continuation());
    }

    #[test]
    fn test_buffer_overflow() {
        let mut buffer = Buffer::new(80, 24);

        // Complex emoji (> 4 bytes UTF-8)
        let emoji = "👨‍👩‍👧‍👦";
        let width = buffer.set_grapheme(0, 0, emoji, Rgb::WHITE, Rgb::BLACK);
        assert!(width > 0);
        assert!(buffer.get(0, 0).unwrap().is_overflow());
        assert_eq!(buffer.get_grapheme(0, 0), Some(emoji));
    }

    #[test]
    fn test_buffer_fill_rect() {
        let mut buffer = Buffer::new(80, 24);
        let cell = Cell::new('X');
        buffer.fill_rect(10, 5, 3, 2, cell);

        assert_eq!(buffer.get(10, 5).unwrap().grapheme(), Some("X"));
        assert_eq!(buffer.get(11, 5).unwrap().grapheme(), Some("X"));
        assert_eq!(buffer.get(12, 5).unwrap().grapheme(), Some("X"));
        assert_eq!(buffer.get(10, 6).unwrap().grapheme(), Some("X"));
        assert_eq!(buffer.get(9, 5).unwrap().grapheme(), Some(" ")); // Outside rect
    }

    #[test]
    fn test_buffer_clear() {
        let mut buffer = Buffer::new(80, 24);
        buffer.set(5, 5, Cell::new('X'));
        buffer.clear();
        assert_eq!(buffer.get(5, 5), Some(&Cell::EMPTY));
    }

    #[test]
    fn test_buffer_resize() {
        let mut buffer = Buffer::new(80, 24);
        buffer.set(5, 5, Cell::new('X'));

        buffer.resize(100, 30);
        assert_eq!(buffer.width(), 100);
        assert_eq!(buffer.height(), 30);
        assert_eq!(buffer.get(5, 5).unwrap().grapheme(), Some("X")); // Preserved

        buffer.resize(10, 10);
        assert_eq!(buffer.get(5, 5).unwrap().grapheme(), Some("X")); // Still preserved
        assert!(buffer.get(15, 15).is_none()); // Out of bounds now
    }

    #[test]
    fn test_buffer_swap() {
        let mut a = Buffer::new(80, 24);
        let mut b = Buffer::new(80, 24);

        a.set(0, 0, Cell::new('A'));
        b.set(0, 0, Cell::new('B'));

        a.swap(&mut b);

        assert_eq!(a.get(0, 0).unwrap().grapheme(), Some("B"));
        assert_eq!(b.get(0, 0).unwrap().grapheme(), Some("A"));
    }

    #[test]
    fn test_buffer_memory_usage() {
        let buffer = Buffer::new(200, 50);
        let usage = buffer.memory_usage();
        // 200 * 50 * 16 = 160,000 bytes for cells, plus overhead
        assert!(usage >= 160_000);
        assert!(usage < 200_000); // Shouldn't be too much more
    }
}
