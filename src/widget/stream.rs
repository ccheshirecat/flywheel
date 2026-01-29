//! Stream Widget: The core streaming text display widget.
//!
//! This widget provides optimistic append with automatic fallback to
//! slow-path rendering when needed.

use super::scroll_buffer::ScrollBuffer;
use crate::buffer::{Buffer, Cell, Rgb};
use crate::layout::Rect;
use std::io::Write;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// Configuration for the stream widget.
#[derive(Debug, Clone)]
pub struct StreamConfig {
    /// Maximum lines to keep in scrollback.
    pub max_scrollback: usize,
    /// Default foreground color.
    pub default_fg: Rgb,
    /// Default background color.
    pub default_bg: Rgb,
    /// Whether to auto-scroll when new content arrives.
    pub auto_scroll: bool,
    /// Whether to enable word wrapping.
    pub word_wrap: bool,
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            max_scrollback: 10000,
            default_fg: Rgb::new(220, 220, 220),
            default_bg: Rgb::DEFAULT_BG,
            auto_scroll: true,
            word_wrap: true,
        }
    }
}

/// Result of an append operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppendResult {
    /// Content was appended using fast path (direct cursor write).
    FastPath {
        /// Number of characters appended.
        chars: usize,
        /// Starting column of the append.
        start_col: u16,
        /// Row of the append.
        row: u16,
    },
    /// Content required slow path (dirty rect for diffing).
    SlowPath {
        /// The dirty rectangle that needs re-rendering.
        dirty_rect: Rect,
    },
    /// No content was appended (empty string).
    Empty,
}

/// A streaming text widget optimized for LLM token output.
///
/// This widget maintains its own content buffer and provides two
/// rendering paths:
///
/// - **Fast path**: Direct cursor-based append for simple cases
/// - **Slow path**: Full dirty-rect re-render for complex cases
pub struct StreamWidget {
    /// Widget bounds within the terminal.
    bounds: Rect,
    /// Configuration.
    config: StreamConfig,
    /// Content buffer.
    content: ScrollBuffer,
    /// Current cursor column within the visible area.
    cursor_col: u16,
    /// Current cursor row within the visible area.
    cursor_row: u16,
    /// Current foreground color.
    current_fg: Rgb,
    /// Current background color.
    current_bg: Rgb,
    /// Whether the widget needs a full redraw.
    needs_full_redraw: bool,
    /// Dirty rectangles accumulated since last render.
    dirty_rects: Vec<Rect>,
}

impl StreamWidget {
    /// Create a new stream widget with the given bounds.
    pub fn new(bounds: Rect) -> Self {
        Self::with_config(bounds, StreamConfig::default())
    }

    /// Create a new stream widget with custom configuration.
    pub fn with_config(bounds: Rect, config: StreamConfig) -> Self {
        Self {
            bounds,
            current_fg: config.default_fg,
            current_bg: config.default_bg,
            content: ScrollBuffer::new(config.max_scrollback),
            config,
            cursor_col: 0,
            cursor_row: 0,
            needs_full_redraw: true,
            dirty_rects: Vec::new(),
        }
    }

    /// Get the widget bounds.
    pub fn bounds(&self) -> Rect {
        self.bounds
    }

    /// Set new bounds for the widget.
    pub fn set_bounds(&mut self, bounds: Rect) {
        if bounds != self.bounds {
            self.bounds = bounds;
            self.needs_full_redraw = true;
        }
    }

    /// Set the foreground color for subsequent text.
    pub fn set_fg(&mut self, fg: Rgb) {
        self.current_fg = fg;
    }

    /// Set the background color for subsequent text.
    pub fn set_bg(&mut self, bg: Rgb) {
        self.current_bg = bg;
    }

    /// Reset colors to defaults.
    pub fn reset_colors(&mut self) {
        self.current_fg = self.config.default_fg;
        self.current_bg = self.config.default_bg;
    }

    /// Check if fast path append is possible for the given text.
    ///
    /// Fast path is possible when:
    /// 1. We're at the bottom of the scroll buffer
    /// 2. The text doesn't contain newlines
    /// 3. The text fits on the current line without wrapping
    /// 4. No scrolling is needed
    fn can_fast_path(&self, text: &str) -> bool {
        // Must be at bottom for fast path
        if !self.content.at_bottom() {
            return false;
        }

        // No newlines allowed in fast path
        if text.contains('\n') {
            return false;
        }

        // Check if text fits on current line
        let text_width = UnicodeWidthStr::width(text);
        let available = (self.bounds.width as usize).saturating_sub(self.cursor_col as usize);

        text_width <= available
    }

    /// Append text using the fast path.
    ///
    /// This directly emits ANSI sequences without going through the diffing
    /// engine. Only call this after checking `can_fast_path()`.
    fn append_fast_path(&mut self, text: &str) -> AppendResult {
        let start_col = self.cursor_col;
        let row = self.cursor_row;
        let mut char_count = 0;

        // Append to content buffer
        self.content.append(text);

        // Update cursor position
        for grapheme in text.graphemes(true) {
            let width = UnicodeWidthStr::width(grapheme);
            self.cursor_col += width as u16;
            char_count += 1;
        }

        AppendResult::FastPath {
            chars: char_count,
            start_col,
            row,
        }
    }

    /// Append text using the slow path.
    ///
    /// This processes the text, handling newlines and wrapping, and marks
    /// the affected area as dirty for the diffing engine.
    fn append_slow_path(&mut self, text: &str) -> AppendResult {
        let initial_row = self.cursor_row;
        let mut max_row = self.cursor_row;
        let min_col = self.cursor_col;
        let mut max_col = self.cursor_col;

        for ch in text.chars() {
            match ch {
                '\n' => {
                    // Hard newline
                    self.content.newline(false);
                    max_col = max_col.max(self.cursor_col);
                    self.cursor_col = 0;
                    self.cursor_row += 1;

                    // Check for scroll
                    if self.cursor_row >= self.bounds.height {
                        self.handle_scroll();
                    }
                }
                '\r' => {
                    // Carriage return
                    self.cursor_col = 0;
                }
                '\t' => {
                    // Tab - expand to spaces
                    let spaces = 4 - (self.cursor_col % 4);
                    for _ in 0..spaces {
                        self.append_char(' ');
                    }
                }
                _ => {
                    self.append_char(ch);
                }
            }

            max_row = max_row.max(self.cursor_row);
            max_col = max_col.max(self.cursor_col);
        }

        // Calculate dirty rect
        let dirty_rect = Rect {
            x: self.bounds.x + min_col.min(0),
            y: self.bounds.y + initial_row,
            width: self.bounds.width,
            height: (max_row - initial_row + 1).max(1),
        };

        self.dirty_rects.push(dirty_rect);

        AppendResult::SlowPath { dirty_rect }
    }

    /// Append a single character, handling wrapping.
    fn append_char(&mut self, ch: char) {
        let char_width = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0) as u16;

        // Check for wrap
        if self.cursor_col + char_width > self.bounds.width {
            if self.config.word_wrap {
                self.content.newline(true);
                self.cursor_col = 0;
                self.cursor_row += 1;

                if self.cursor_row >= self.bounds.height {
                    self.handle_scroll();
                }
            } else {
                // No wrap - just don't add the character
                return;
            }
        }

        // Add character to content
        self.content.append(&ch.to_string());
        self.cursor_col += char_width;
    }

    /// Handle scrolling when cursor goes past bottom.
    fn handle_scroll(&mut self) {
        // Keep cursor at bottom row
        self.cursor_row = self.bounds.height - 1;

        // If auto-scroll is on, stay at bottom
        if self.config.auto_scroll {
            self.content.scroll_to_bottom();
        }

        // Full redraw needed when scrolling
        self.needs_full_redraw = true;
    }

    /// Append text to the widget.
    ///
    /// This automatically chooses between fast and slow path based on
    /// the text content and current state.
    pub fn append(&mut self, text: &str) -> AppendResult {
        if text.is_empty() {
            return AppendResult::Empty;
        }

        if self.can_fast_path(text) {
            self.append_fast_path(text)
        } else {
            self.append_slow_path(text)
        }
    }

    /// Render the widget to a buffer.
    ///
    /// This renders the visible content to the given buffer.
    pub fn render(&mut self, buffer: &mut Buffer) {
        let viewport_height = self.bounds.height as usize;

        // Get visible lines
        let visible_lines: Vec<_> = self.content.visible_lines(viewport_height).collect();

        // Render each line
        for (row, line) in visible_lines.iter().enumerate() {
            let y = self.bounds.y + row as u16;
            if y >= self.bounds.y + self.bounds.height {
                break;
            }

            let mut col = 0u16;
            for grapheme in line.content.graphemes(true) {
                if col >= self.bounds.width {
                    break;
                }

                let x = self.bounds.x + col;
                let width = buffer.set_grapheme(x, y, grapheme, self.current_fg, self.current_bg);
                col += width as u16;
            }

            // Clear rest of line
            while col < self.bounds.width {
                let x = self.bounds.x + col;
                buffer.set(x, y, Cell::new(' ').with_fg(self.current_fg).with_bg(self.current_bg));
                col += 1;
            }
        }

        // Clear any remaining rows
        for row in visible_lines.len()..viewport_height {
            let y = self.bounds.y + row as u16;
            for col in 0..self.bounds.width {
                let x = self.bounds.x + col;
                buffer.set(x, y, Cell::new(' ').with_fg(self.current_fg).with_bg(self.current_bg));
            }
        }

        self.needs_full_redraw = false;
        self.dirty_rects.clear();
    }

    /// Write fast-path output directly to an output buffer.
    ///
    /// This generates ANSI sequences for direct terminal output,
    /// bypassing the buffer diffing.
    pub fn write_fast_path(
        &self,
        result: AppendResult,
        text: &str,
        output: &mut Vec<u8>,
    ) {
        if let AppendResult::FastPath { start_col, row, .. } = result {
            // Move cursor to position
            let abs_x = self.bounds.x + start_col + 1; // 1-indexed
            let abs_y = self.bounds.y + row + 1; // 1-indexed

            let _ = write!(output, "\x1b[{};{}H", abs_y, abs_x);

            // Set colors
            let _ = write!(
                output,
                "\x1b[38;2;{};{};{}m\x1b[48;2;{};{};{}m",
                self.current_fg.r, self.current_fg.g, self.current_fg.b,
                self.current_bg.r, self.current_bg.g, self.current_bg.b
            );

            // Write text
            output.extend_from_slice(text.as_bytes());
        }
    }

    /// Check if a full redraw is needed.
    pub fn needs_redraw(&self) -> bool {
        self.needs_full_redraw || !self.dirty_rects.is_empty()
    }

    /// Get the dirty rectangles.
    pub fn dirty_rects(&self) -> &[Rect] {
        &self.dirty_rects
    }

    /// Mark the widget for full redraw.
    pub fn invalidate(&mut self) {
        self.needs_full_redraw = true;
    }

    /// Clear all content.
    pub fn clear(&mut self) {
        self.content.clear();
        self.cursor_col = 0;
        self.cursor_row = 0;
        self.needs_full_redraw = true;
    }

    /// Scroll up by the given number of lines.
    pub fn scroll_up(&mut self, lines: usize) {
        self.content.scroll_up(lines);
        self.needs_full_redraw = true;
    }

    /// Scroll down by the given number of lines.
    pub fn scroll_down(&mut self, lines: usize) {
        self.content.scroll_down(lines);
        self.needs_full_redraw = true;
    }

    /// Get the current cursor position within the widget.
    pub fn cursor_position(&self) -> (u16, u16) {
        (self.cursor_col, self.cursor_row)
    }

    /// Get the number of lines in the buffer.
    pub fn line_count(&self) -> usize {
        self.content.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_widget_new() {
        let widget = StreamWidget::new(Rect::new(0, 0, 80, 24));
        assert_eq!(widget.bounds().width, 80);
        assert_eq!(widget.bounds().height, 24);
        assert_eq!(widget.cursor_position(), (0, 0));
    }

    #[test]
    fn test_stream_widget_append_fast_path() {
        let mut widget = StreamWidget::new(Rect::new(0, 0, 80, 24));
        let result = widget.append("Hello");

        match result {
            AppendResult::FastPath { chars, start_col, row } => {
                assert_eq!(chars, 5);
                assert_eq!(start_col, 0);
                assert_eq!(row, 0);
            }
            _ => panic!("Expected fast path"),
        }

        assert_eq!(widget.cursor_position(), (5, 0));
    }

    #[test]
    fn test_stream_widget_append_slow_path_newline() {
        let mut widget = StreamWidget::new(Rect::new(0, 0, 80, 24));
        let result = widget.append("Hello\nWorld");

        match result {
            AppendResult::SlowPath { .. } => {}
            _ => panic!("Expected slow path due to newline"),
        }

        assert_eq!(widget.cursor_position(), (5, 1));
    }

    #[test]
    fn test_stream_widget_wrap() {
        let mut widget = StreamWidget::new(Rect::new(0, 0, 10, 24));
        
        // Append text that will wrap
        widget.append("12345678901234567890");
        
        // Should have wrapped to line 2
        assert!(widget.cursor_row > 0);
    }

    #[test]
    fn test_stream_widget_render() {
        let mut widget = StreamWidget::new(Rect::new(0, 0, 10, 3));
        widget.append("Line 1\nLine 2\nLine 3");

        let mut buffer = Buffer::new(10, 3);
        widget.render(&mut buffer);

        // Check that content was rendered
        let cell = buffer.get(0, 0).unwrap();
        assert_eq!(cell.grapheme(), Some("L"));
    }
}
