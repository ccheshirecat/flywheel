//! Cell: The atomic unit of terminal display.
//!
//! # Memory Layout
//!
//! The `Cell` struct is carefully designed for cache efficiency:
//! - 16 bytes total, allowing 4 cells per cache line (64 bytes)
//! - Inline grapheme storage covers 99%+ of real-world characters
//! - Complex graphemes (emoji ZWJ sequences) spill to an external `HashMap`
//!
//! ```text
//! ┌───────────────────────────────────────────────────────────────────────────┐
//! │  Cell Layout (16 bytes)                                                   │
//! ├─────────────┬─────────────┬───────────┬───────────┬─────┬───────┬─────────┤
//! │  grapheme   │  len + width│    fg     │    bg     │ mod │ flags │ padding │
//! │  [u8; 4]    │  u8 + u8    │  [u8; 3]  │  [u8; 3]  │ u8  │  u8   │ [u8; 2] │
//! │  4 bytes    │  2 bytes    │  3 bytes  │  3 bytes  │ 1b  │  1b   │  2 b    │
//! └─────────────┴─────────────┴───────────┴───────────┴─────┴───────┴─────────┘
//! ```

use bitflags::bitflags;
use std::hash::{Hash, Hasher};

/// True-color RGB representation.
///
/// Uses 3 bytes for 24-bit color depth, supporting 16.7 million colors.
/// This is essential for precise brand colors in commercial applications.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Default, Hash)]
pub struct Rgb {
    /// Red channel (0-255)
    pub r: u8,
    /// Green channel (0-255)
    pub g: u8,
    /// Blue channel (0-255)
    pub b: u8,
}

impl Rgb {
    /// Create a new RGB color.
    #[inline]
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Black (0, 0, 0)
    pub const BLACK: Self = Self::new(0, 0, 0);
    /// White (255, 255, 255)
    pub const WHITE: Self = Self::new(255, 255, 255);
    /// Default foreground (white)
    pub const DEFAULT_FG: Self = Self::WHITE;
    /// Default background (black)
    pub const DEFAULT_BG: Self = Self::BLACK;

    /// Create from a 24-bit hex color (e.g., 0xFF5500).
    #[inline]
    pub const fn from_u32(hex: u32) -> Self {
        Self::new(
            ((hex >> 16) & 0xFF) as u8,
            ((hex >> 8) & 0xFF) as u8,
            (hex & 0xFF) as u8,
        )
    }
}

impl std::fmt::Debug for Rgb {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }
}

impl From<(u8, u8, u8)> for Rgb {
    #[inline]
    fn from((r, g, b): (u8, u8, u8)) -> Self {
        Self::new(r, g, b)
    }
}

impl From<u32> for Rgb {
    /// Convert from a 24-bit hex color (e.g., 0xFF5500)
    #[inline]
    fn from(hex: u32) -> Self {
        Self::new(
            ((hex >> 16) & 0xFF) as u8,
            ((hex >> 8) & 0xFF) as u8,
            (hex & 0xFF) as u8,
        )
    }
}

bitflags! {
    /// Text style modifiers.
    ///
    /// These can be combined using bitwise OR.
    ///
    /// # Example
    /// ```
    /// use flywheel::Modifiers;
    /// let style = Modifiers::BOLD | Modifiers::ITALIC;
    /// ```
    #[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct Modifiers: u8 {
        /// Bold text
        const BOLD = 0b0000_0001;
        /// Dim/faint text
        const DIM = 0b0000_0010;
        /// Italic text
        const ITALIC = 0b0000_0100;
        /// Underlined text
        const UNDERLINE = 0b0000_1000;
        /// Blinking text
        const BLINK = 0b0001_0000;
        /// Reversed colors (fg/bg swapped)
        const REVERSED = 0b0010_0000;
        /// Hidden/invisible text
        const HIDDEN = 0b0100_0000;
        /// Strikethrough text
        const STRIKETHROUGH = 0b1000_0000;
    }
}

impl std::fmt::Debug for Modifiers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        bitflags::parser::to_writer(self, f)
    }
}

bitflags! {
    /// Cell-level flags for special states.
    #[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct CellFlags: u8 {
        /// Grapheme overflows inline storage; check overflow HashMap
        const OVERFLOW = 0b0000_0001;
        /// Cell has been modified since last render
        const DIRTY = 0b0000_0010;
        /// This cell is a continuation of a wide character
        const WIDE_CONTINUATION = 0b0000_0100;
    }
}

impl std::fmt::Debug for CellFlags {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        bitflags::parser::to_writer(self, f)
    }
}

/// A single terminal cell.
///
/// This is the atomic unit of display in Flywheel. Each cell contains:
/// - A grapheme (the character to display)
/// - Foreground and background colors
/// - Text modifiers (bold, italic, etc.)
///
/// # Memory Layout
///
/// The struct is carefully laid out to be exactly 16 bytes:
/// - 4 bytes for inline grapheme storage
/// - 2 bytes for grapheme metadata (length + display width)
/// - 6 bytes for colors (3 bytes fg + 3 bytes bg)
/// - 1 byte for modifiers
/// - 1 byte for flags
/// - 2 bytes padding (power-of-2 alignment)
///
/// # Grapheme Handling
///
/// Most characters (ASCII, Latin, CJK) fit within the 4-byte inline storage.
/// For complex graphemes like emoji ZWJ sequences (👨‍👩‍👧‍👦), we set the
/// `OVERFLOW` flag and store an index in the grapheme bytes that points
/// to an external overflow storage.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Cell {
    /// Inline grapheme storage (UTF-8 bytes).
    /// For overflowed graphemes, this contains a u32 index.
    grapheme: [u8; 4],
    /// Actual byte length of the grapheme (0-4, or 0 if overflowed).
    grapheme_len: u8,
    /// Display width of the grapheme (0=continuation, 1=normal, 2=wide CJK).
    display_width: u8,
    /// Foreground color.
    fg: Rgb,
    /// Background color.
    bg: Rgb,
    /// Text modifiers (bold, italic, etc.).
    modifiers: Modifiers,
    /// Cell flags (overflow, dirty, etc.).
    flags: CellFlags,
    /// Padding to reach 16 bytes (power of 2, cache-friendly).
    _padding: [u8; 2],
}

// Compile-time assertion: Cell must be exactly 16 bytes
const _: () = assert!(
    std::mem::size_of::<Cell>() == 16,
    "Cell must be exactly 16 bytes for cache efficiency"
);

impl Default for Cell {
    fn default() -> Self {
        Self::EMPTY
    }
}

impl Cell {
    /// An empty cell (space character with default colors).
    pub const EMPTY: Self = Self {
        grapheme: [b' ', 0, 0, 0],
        grapheme_len: 1,
        display_width: 1,
        fg: Rgb::DEFAULT_FG,
        bg: Rgb::DEFAULT_BG,
        modifiers: Modifiers::empty(),
        flags: CellFlags::empty(),
        _padding: [0, 0],
    };

    /// Create a new cell with a single ASCII character.
    ///
    /// # Panics
    /// Panics if the character is not ASCII.
    #[inline]
    pub fn new(c: char) -> Self {
        debug_assert!(c.is_ascii(), "Use Cell::from_char for non-ASCII");
        Self {
            grapheme: [c as u8, 0, 0, 0],
            grapheme_len: 1,
            display_width: 1,
            fg: Rgb::DEFAULT_FG,
            bg: Rgb::DEFAULT_BG,
            modifiers: Modifiers::empty(),
            flags: CellFlags::empty(),
            _padding: [0, 0],
        }
    }

    /// Create a cell from any character.
    ///
    /// Returns `None` if the character's UTF-8 encoding exceeds 4 bytes
    /// (which never happens for a single `char`, but may happen for
    /// grapheme clusters when using `from_grapheme`).
    #[inline]
    #[allow(clippy::missing_panics_doc)]
    pub fn from_char(c: char) -> Self {
        let mut grapheme = [0u8; 4];
        let s = c.encode_utf8(&mut grapheme);
        let len = u8::try_from(s.len()).unwrap();
        let width = u8::try_from(unicode_width::UnicodeWidthChar::width(c).unwrap_or(0)).unwrap();

        Self {
            grapheme,
            grapheme_len: len,
            display_width: width,
            fg: Rgb::DEFAULT_FG,
            bg: Rgb::DEFAULT_BG,
            modifiers: Modifiers::empty(),
            flags: CellFlags::empty(),
            _padding: [0, 0],
        }
    }

    /// Create a cell from a grapheme string.
    ///
    /// If the grapheme fits in 4 bytes, it's stored inline.
    /// Otherwise, returns `None` and the caller should use overflow storage.
    #[inline]
    #[allow(clippy::missing_panics_doc)]
    pub fn from_grapheme(s: &str) -> Option<Self> {
        let bytes = s.as_bytes();
        if bytes.len() > 4 {
            // Caller needs to handle overflow
            return None;
        }

        let mut grapheme = [0u8; 4];
        grapheme[..bytes.len()].copy_from_slice(bytes);
        let width = u8::try_from(unicode_width::UnicodeWidthStr::width(s)).unwrap_or(1);

        Some(Self {
            grapheme,
            grapheme_len: u8::try_from(bytes.len()).unwrap(),
            display_width: width,
            fg: Rgb::DEFAULT_FG,
            bg: Rgb::DEFAULT_BG,
            modifiers: Modifiers::empty(),
            flags: CellFlags::empty(),
            _padding: [0, 0],
        })
    }

    /// Create an overflow cell with an index to external storage.
    ///
    /// The index is stored in the grapheme bytes as a little-endian u32.
    #[inline]
    pub const fn overflow(index: u32, display_width: u8) -> Self {
        Self {
            grapheme: index.to_le_bytes(),
            grapheme_len: 0, // Indicates overflow
            display_width,
            fg: Rgb::DEFAULT_FG,
            bg: Rgb::DEFAULT_BG,
            modifiers: Modifiers::empty(),
            flags: CellFlags::OVERFLOW,
            _padding: [0, 0],
        }
    }

    /// Create a wide-character continuation cell.
    ///
    /// This is placed after a wide CJK character that takes 2 columns.
    #[inline]
    pub const fn wide_continuation() -> Self {
        Self {
            grapheme: [0, 0, 0, 0],
            grapheme_len: 0,
            display_width: 0,
            fg: Rgb::DEFAULT_FG,
            bg: Rgb::DEFAULT_BG,
            modifiers: Modifiers::empty(),
            flags: CellFlags::WIDE_CONTINUATION,
            _padding: [0, 0],
        }
    }

    /// Get the grapheme as a string slice.
    ///
    /// Returns `None` if this is an overflow cell (caller should check `is_overflow()`
    /// and look up the grapheme in the overflow storage).
    #[inline]
    #[allow(unsafe_code)]
    pub fn grapheme(&self) -> Option<&str> {
        if self.flags.contains(CellFlags::OVERFLOW) {
            return None;
        }
        // SAFETY: We only store valid UTF-8 in the grapheme bytes
        Some(unsafe {
            std::str::from_utf8_unchecked(&self.grapheme[..self.grapheme_len as usize])
        })
    }

    /// Get the overflow index if this is an overflow cell.
    #[inline]
    pub const fn overflow_index(&self) -> Option<u32> {
        if self.flags.contains(CellFlags::OVERFLOW) {
            Some(u32::from_le_bytes(self.grapheme))
        } else {
            None
        }
    }

    /// Check if this cell uses overflow storage.
    #[inline]
    pub const fn is_overflow(&self) -> bool {
        self.flags.contains(CellFlags::OVERFLOW)
    }

    /// Check if this is a wide-character continuation.
    #[inline]
    pub const fn is_wide_continuation(&self) -> bool {
        self.flags.contains(CellFlags::WIDE_CONTINUATION)
    }

    /// Get the display width (0, 1, or 2).
    #[inline]
    pub const fn display_width(&self) -> u8 {
        self.display_width
    }

    /// Get the foreground color.
    #[inline]
    pub const fn fg(&self) -> Rgb {
        self.fg
    }

    /// Get the background color.
    #[inline]
    pub const fn bg(&self) -> Rgb {
        self.bg
    }

    /// Get the modifiers.
    #[inline]
    pub const fn modifiers(&self) -> Modifiers {
        self.modifiers
    }

    /// Get the flags.
    #[inline]
    pub const fn flags(&self) -> CellFlags {
        self.flags
    }

    /// Set the foreground color.
    #[inline]
    pub const fn set_fg(&mut self, fg: Rgb) -> &mut Self {
        self.fg = fg;
        self
    }

    /// Set the background color.
    #[inline]
    pub const fn set_bg(&mut self, bg: Rgb) -> &mut Self {
        self.bg = bg;
        self
    }

    /// Set the modifiers.
    #[inline]
    pub const fn set_modifiers(&mut self, modifiers: Modifiers) -> &mut Self {
        self.modifiers = modifiers;
        self
    }

    /// Set the foreground color (builder pattern).
    #[inline]
    #[must_use]
    pub const fn with_fg(mut self, fg: Rgb) -> Self {
        self.fg = fg;
        self
    }

    /// Set the background color (builder pattern).
    #[inline]
    #[must_use]
    pub const fn with_bg(mut self, bg: Rgb) -> Self {
        self.bg = bg;
        self
    }

    /// Set the modifiers (builder pattern).
    #[inline]
    #[must_use]
    pub const fn with_modifiers(mut self, modifiers: Modifiers) -> Self {
        self.modifiers = modifiers;
        self
    }

    /// Reset the cell to empty (space with default colors).
    #[inline]
    pub const fn reset(&mut self) {
        *self = Self::EMPTY;
    }
}

impl PartialEq for Cell {
    /// Optimized equality check.
    ///
    /// We compare in order of most likely difference:
    /// 1. Grapheme bytes (most frequently changing)
    /// 2. Colors (next most common)
    /// 3. Modifiers and flags (rarely differ)
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        // Fast path: compare grapheme first (most likely to differ)
        self.grapheme == other.grapheme
            && self.grapheme_len == other.grapheme_len
            && self.fg == other.fg
            && self.bg == other.bg
            && self.modifiers == other.modifiers
            && self.flags == other.flags
            && self.display_width == other.display_width
    }
}

impl Eq for Cell {}

impl Hash for Cell {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.grapheme.hash(state);
        self.grapheme_len.hash(state);
        self.display_width.hash(state);
        self.fg.hash(state);
        self.bg.hash(state);
        self.modifiers.hash(state);
        self.flags.hash(state);
    }
}

impl std::fmt::Debug for Cell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let grapheme = self.grapheme().unwrap_or("<overflow>");
        f.debug_struct("Cell")
            .field("grapheme", &grapheme)
            .field("width", &self.display_width)
            .field("fg", &self.fg)
            .field("bg", &self.bg)
            .field("modifiers", &self.modifiers)
            .field("flags", &self.flags)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cell_size() {
        assert_eq!(std::mem::size_of::<Cell>(), 16);
    }

    #[test]
    fn test_rgb_from_tuple() {
        let rgb: Rgb = (255, 128, 0).into();
        assert_eq!(rgb.r, 255);
        assert_eq!(rgb.g, 128);
        assert_eq!(rgb.b, 0);
    }

    #[test]
    fn test_rgb_from_hex() {
        let rgb: Rgb = 0xFF8000.into();
        assert_eq!(rgb.r, 255);
        assert_eq!(rgb.g, 128);
        assert_eq!(rgb.b, 0);
    }

    #[test]
    fn test_cell_new_ascii() {
        let cell = Cell::new('A');
        assert_eq!(cell.grapheme(), Some("A"));
        assert_eq!(cell.display_width(), 1);
    }

    #[test]
    fn test_cell_from_char_unicode() {
        let cell = Cell::from_char('日');
        assert_eq!(cell.grapheme(), Some("日"));
        assert_eq!(cell.display_width(), 2); // CJK is double-width
    }

    #[test]
    fn test_cell_from_grapheme_fits() {
        let cell = Cell::from_grapheme("é").unwrap();
        assert_eq!(cell.grapheme(), Some("é"));
        assert_eq!(cell.display_width(), 1);
    }

    #[test]
    fn test_cell_from_grapheme_overflow() {
        // This emoji ZWJ sequence is > 4 bytes
        let result = Cell::from_grapheme("👨‍👩‍👧");
        assert!(result.is_none());
    }

    #[test]
    fn test_cell_overflow() {
        let cell = Cell::overflow(42, 2);
        assert!(cell.is_overflow());
        assert_eq!(cell.overflow_index(), Some(42));
        assert_eq!(cell.grapheme(), None);
    }

    #[test]
    fn test_cell_equality() {
        let a = Cell::new('A').with_fg(Rgb::new(255, 0, 0));
        let b = Cell::new('A').with_fg(Rgb::new(255, 0, 0));
        let c = Cell::new('A').with_fg(Rgb::new(0, 255, 0));

        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn test_cell_builder_pattern() {
        let cell = Cell::new('X')
            .with_fg(Rgb::new(255, 0, 0))
            .with_bg(Rgb::new(0, 0, 255))
            .with_modifiers(Modifiers::BOLD | Modifiers::ITALIC);

        assert_eq!(cell.fg(), Rgb::new(255, 0, 0));
        assert_eq!(cell.bg(), Rgb::new(0, 0, 255));
        assert!(cell.modifiers().contains(Modifiers::BOLD));
        assert!(cell.modifiers().contains(Modifiers::ITALIC));
    }

    #[test]
    fn test_modifiers_bitflags() {
        let mods = Modifiers::BOLD | Modifiers::UNDERLINE;
        assert!(mods.contains(Modifiers::BOLD));
        assert!(mods.contains(Modifiers::UNDERLINE));
        assert!(!mods.contains(Modifiers::ITALIC));
    }

    #[test]
    fn test_cell_reset() {
        let mut cell = Cell::new('X').with_fg(Rgb::new(255, 0, 0));
        cell.reset();
        assert_eq!(cell, Cell::EMPTY);
    }

    #[test]
    fn test_wide_continuation() {
        let cont = Cell::wide_continuation();
        assert!(cont.is_wide_continuation());
        assert_eq!(cont.display_width(), 0);
    }
}
