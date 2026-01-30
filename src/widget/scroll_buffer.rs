//! Scroll buffer: Ring buffer for storing scrollback history.
//!
//! This provides efficient storage for text content that may scroll
//! off the visible area, with O(1) append and scroll operations.

use std::collections::VecDeque;
use crate::buffer::Cell;

/// A line of text with associated style information.
#[derive(Debug, Clone)]
pub struct StyledLine {
    /// The text content of the line.
    pub content: Vec<Cell>,
    /// Whether this line was soft-wrapped (vs. hard newline).
    pub wrapped: bool,
}

impl StyledLine {
    /// Create a new styled line.
    pub const fn new(content: Vec<Cell>, wrapped: bool) -> Self {
        Self { content, wrapped }
    }

    /// Create an empty line.
    pub const fn empty() -> Self {
        Self {
            content: Vec::new(),
            wrapped: false,
        }
    }
}

/// Ring buffer for storing lines with scrollback.
///
/// The scroll buffer maintains a fixed number of lines in memory,
/// automatically discarding old lines when capacity is exceeded.
#[derive(Debug)]
pub struct ScrollBuffer {
    /// Lines stored in the buffer.
    lines: VecDeque<StyledLine>,
    /// Maximum number of lines to retain.
    max_lines: usize,
    /// Current scroll offset from the bottom (0 = at bottom).
    scroll_offset: usize,
}

impl ScrollBuffer {
    /// Create a new scroll buffer with the given capacity.
    pub fn new(max_lines: usize) -> Self {
        let mut lines = VecDeque::with_capacity(max_lines);
        lines.push_back(StyledLine::empty());

        Self {
            lines,
            max_lines,
            scroll_offset: 0,
        }
    }

    /// Get the total number of lines in the buffer.
    pub fn len(&self) -> usize {
        self.lines.len()
    }

    /// Check if the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// Get the current line (the line being appended to).
    ///
    /// # Panics
    ///
    /// Panics if the buffer is empty (which should never happen).
    pub fn current_line(&self) -> &StyledLine {
        self.lines.back().expect("Buffer should never be empty")
    }

    /// Get a mutable reference to the current line.
    ///
    /// # Panics
    ///
    /// Panics if the buffer is empty (which should never happen).
    pub fn current_line_mut(&mut self) -> &mut StyledLine {
        self.lines.back_mut().expect("Buffer should never be empty")
    }

    /// Append cells to the current line.
    pub fn append(&mut self, cells: impl IntoIterator<Item = Cell>) {
        self.current_line_mut().content.extend(cells);
    }

    /// Start a new line.
    ///
    /// # Arguments
    ///
    /// * `wrapped` - Whether the new line is due to soft wrapping.
    pub fn newline(&mut self, wrapped: bool) {
        // Trim excess lines if at capacity
        while self.lines.len() >= self.max_lines {
            self.lines.pop_front();
        }

        self.lines.push_back(StyledLine::new(Vec::new(), wrapped));
    }

    /// Get a line by index from the top of the buffer.
    pub fn get(&self, index: usize) -> Option<&StyledLine> {
        self.lines.get(index)
    }

    /// Get visible lines for a given viewport height.
    ///
    /// Returns an iterator over lines that should be visible,
    /// accounting for scroll offset.
    pub fn visible_lines(&self, viewport_height: usize) -> impl Iterator<Item = &StyledLine> {
        let total = self.lines.len();
        let end = total.saturating_sub(self.scroll_offset);
        let start = end.saturating_sub(viewport_height);

        self.lines.range(start..end)
    }

    /// Scroll up by the given number of lines.
    pub fn scroll_up(&mut self, lines: usize) {
        let max_offset = self.lines.len().saturating_sub(1);
        self.scroll_offset = (self.scroll_offset + lines).min(max_offset);
    }

    /// Scroll down by the given number of lines.
    pub const fn scroll_down(&mut self, lines: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(lines);
    }

    /// Scroll to the bottom (latest content).
    pub const fn scroll_to_bottom(&mut self) {
        self.scroll_offset = 0;
    }

    /// Check if we're scrolled to the bottom.
    pub const fn at_bottom(&self) -> bool {
        self.scroll_offset == 0
    }

    /// Clear all content.
    pub fn clear(&mut self) {
        self.lines.clear();
        self.lines.push_back(StyledLine::empty());
        self.scroll_offset = 0;
    }

    /// Get the length of the current line in characters.
    pub fn current_line_len(&self) -> usize {
        self.current_line().content.len()
    }

    /// Rewrap all content to a new width.
    ///
    /// This is called when the widget bounds change to ensure content
    /// displays correctly at the new width.
    pub fn rewrap(&mut self, new_width: usize) {
        if new_width == 0 {
            return;
        }

        // Collect all content into logical lines (merging soft-wrapped lines)
        let mut logical_lines: Vec<Vec<Cell>> = Vec::new();
        let mut current_logical: Vec<Cell> = Vec::new();

        for line in &self.lines {
            current_logical.extend(line.content.iter().copied());
            if !line.wrapped {
                // Hard newline - end of logical line
                logical_lines.push(std::mem::take(&mut current_logical));
            }
        }
        // Don't forget the last line if it didn't end with a newline
        if !current_logical.is_empty() || logical_lines.is_empty() {
            logical_lines.push(current_logical);
        }

        // Re-wrap logical lines to new width
        self.lines.clear();
        for logical in logical_lines {
            if logical.is_empty() {
                self.lines.push_back(StyledLine::empty());
            } else {
                let chunks: Vec<_> = logical.chunks(new_width).collect();
                let chunk_count = chunks.len();
                for (i, chunk) in chunks.into_iter().enumerate() {
                    let wrapped = i < chunk_count - 1;
                    self.lines.push_back(StyledLine::new(chunk.to_vec(), wrapped));
                }
            }
        }

        // Ensure we always have at least one line
        if self.lines.is_empty() {
            self.lines.push_back(StyledLine::empty());
        }

        // Trim to max_lines if needed
        while self.lines.len() > self.max_lines {
            self.lines.pop_front();
        }

        // Reset scroll to bottom after rewrap
        self.scroll_offset = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text_to_cells(text: &str) -> Vec<Cell> {
        text.chars().map(Cell::from_char).collect()
    }

    #[test]
    fn test_scroll_buffer_new() {
        let buf = ScrollBuffer::new(100);
        assert_eq!(buf.len(), 1);
        assert!(buf.current_line().content.is_empty());
    }

    #[test]
    fn test_scroll_buffer_append() {
        let mut buf = ScrollBuffer::new(100);
        buf.append(text_to_cells("Hello"));
        buf.append(text_to_cells(", world!"));
        
        let content: String = buf.current_line().content.iter()
            .map(|c| c.grapheme().unwrap_or(""))
            .collect();
        assert_eq!(content, "Hello, world!");
    }

    #[test]
    fn test_scroll_buffer_newline() {
        let mut buf = ScrollBuffer::new(100);
        buf.append(text_to_cells("Line 1"));
        buf.newline(false);
        buf.append(text_to_cells("Line 2"));
        assert_eq!(buf.len(), 2);
        
        let l1: String = buf.get(0).unwrap().content.iter().map(|c| c.grapheme().unwrap_or("")).collect();
        assert_eq!(l1, "Line 1");
    }

    #[test]
    fn test_scroll_buffer_capacity() {
        let mut buf = ScrollBuffer::new(3);
        buf.append(text_to_cells("Line 1"));
        buf.newline(false);
        buf.append(text_to_cells("Line 2"));
        buf.newline(false);
        buf.append(text_to_cells("Line 3"));
        buf.newline(false);
        buf.append(text_to_cells("Line 4"));

        assert_eq!(buf.len(), 3);
        // Line 1 should have been discarded
        let l0: String = buf.get(0).unwrap().content.iter().map(|c| c.grapheme().unwrap_or("")).collect();
        assert_eq!(l0, "Line 2");
    }

    #[test]
    fn test_scroll_buffer_scroll() {
        let mut buf = ScrollBuffer::new(100);
        for i in 0..10 {
            buf.append(text_to_cells(&format!("Line {i}")));
            buf.newline(false);
        }

        assert!(buf.at_bottom());

        buf.scroll_up(3);
        assert!(!buf.at_bottom());
        assert_eq!(buf.scroll_offset, 3);

        buf.scroll_down(1);
        assert_eq!(buf.scroll_offset, 2);

        buf.scroll_to_bottom();
        assert!(buf.at_bottom());
    }
}
