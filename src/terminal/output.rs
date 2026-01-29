//! `OutputBuffer`: Single-syscall output buffer for ANSI sequences.

use crate::buffer::Rgb;
use std::io::Write;

/// Pre-allocated buffer for building ANSI escape sequences.
///
/// All output is accumulated here, then flushed in a single `write()` syscall
/// to prevent terminal flickering.
pub struct OutputBuffer {
    data: Vec<u8>,
}

impl OutputBuffer {
    /// Create a new output buffer with the given capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            data: Vec::with_capacity(capacity),
        }
    }

    /// Create a buffer sized for a typical terminal (4KB).
    pub fn new() -> Self {
        Self::with_capacity(4096)
    }

    /// Clear the buffer for reuse.
    #[inline]
    pub fn clear(&mut self) {
        self.data.clear();
    }

    /// Get the buffer contents.
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    /// Get the buffer length.
    #[inline]
    pub const fn len(&self) -> usize {
        self.data.len()
    }

    /// Check if buffer is empty.
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Write raw bytes.
    #[inline]
    pub fn write_raw(&mut self, bytes: &[u8]) {
        self.data.extend_from_slice(bytes);
    }

    /// Write a string.
    #[inline]
    pub fn write_str(&mut self, s: &str) {
        self.data.extend_from_slice(s.as_bytes());
    }

    /// Move cursor to (x, y) position (1-indexed for ANSI).
    #[inline]
    pub fn cursor_move(&mut self, x: u16, y: u16) {
        // CSI row ; col H
        write!(self.data, "\x1b[{};{}H", y + 1, x + 1).unwrap();
    }

    /// Hide cursor.
    #[inline]
    pub fn cursor_hide(&mut self) {
        self.data.extend_from_slice(b"\x1b[?25l");
    }

    /// Show cursor.
    #[inline]
    pub fn cursor_show(&mut self) {
        self.data.extend_from_slice(b"\x1b[?25h");
    }

    /// Set foreground color (true color).
    #[inline]
    pub fn set_fg(&mut self, color: Rgb) {
        write!(self.data, "\x1b[38;2;{};{};{}m", color.r, color.g, color.b).unwrap();
    }

    /// Set background color (true color).
    #[inline]
    pub fn set_bg(&mut self, color: Rgb) {
        write!(self.data, "\x1b[48;2;{};{};{}m", color.r, color.g, color.b).unwrap();
    }

    /// Reset all attributes.
    #[inline]
    pub fn reset_attrs(&mut self) {
        self.data.extend_from_slice(b"\x1b[0m");
    }

    /// Clear the entire screen.
    #[inline]
    pub fn clear_screen(&mut self) {
        self.data.extend_from_slice(b"\x1b[2J");
    }

    /// Flush to a writer in a single syscall.
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying writer fails.
    pub fn flush_to<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        writer.write_all(&self.data)?;
        writer.flush()
    }
}

impl Default for OutputBuffer {
    fn default() -> Self {
        Self::new()
    }
}
