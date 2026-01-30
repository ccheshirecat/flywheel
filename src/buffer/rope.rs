//! Rope Buffer: Chunked storage for efficient large document handling.
//!
//! This module provides a rope-based data structure optimized for:
//! - Large documents (1M+ lines) with minimal allocations
//! - O(1) append and O(log n) random access
//! - Good cache locality through chunking

use crate::buffer::Cell;

/// Number of lines per chunk.
/// Tuned for a balance between overhead and cache utilization.
const CHUNK_SIZE: usize = 64;

/// A chunk of lines stored contiguously.
#[derive(Debug, Clone)]
struct Chunk {
    /// Lines in this chunk.
    lines: Vec<ChunkedLine>,
}

impl Chunk {
    /// Create a new empty chunk.
    fn new() -> Self {
        Self {
            lines: Vec::with_capacity(CHUNK_SIZE),
        }
    }

    /// Check if the chunk is full.
    const fn is_full(&self) -> bool {
        self.lines.len() >= CHUNK_SIZE
    }

    /// Get the number of lines in this chunk.
    const fn len(&self) -> usize {
        self.lines.len()
    }

    /// Check if the chunk is empty.
    #[allow(dead_code)]
    const fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// Push a line to this chunk.
    fn push(&mut self, line: ChunkedLine) {
        self.lines.push(line);
    }

    /// Get a line by index within this chunk.
    fn get(&self, index: usize) -> Option<&ChunkedLine> {
        self.lines.get(index)
    }

    /// Get a mutable line by index within this chunk.
    fn get_mut(&mut self, index: usize) -> Option<&mut ChunkedLine> {
        self.lines.get_mut(index)
    }
}

/// A line stored in the rope buffer.
#[derive(Debug, Clone)]
pub struct ChunkedLine {
    /// The cells in this line.
    pub content: Vec<Cell>,
    /// Whether this line was soft-wrapped.
    pub wrapped: bool,
}

impl ChunkedLine {
    /// Create a new line with the given content.
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

    /// Get the number of cells in this line.
    pub const fn len(&self) -> usize {
        self.content.len()
    }

    /// Check if the line is empty.
    pub const fn is_empty(&self) -> bool {
        self.content.is_empty()
    }
}

/// A rope-based line buffer for efficient large document storage.
///
/// Instead of storing each line as a separate allocation, lines are
/// grouped into chunks of `CHUNK_SIZE` lines. This reduces:
/// - Memory fragmentation
/// - Allocation overhead
/// - Cache misses during iteration
#[derive(Debug)]
pub struct RopeBuffer {
    /// Chunks of lines.
    chunks: Vec<Chunk>,
    /// Total number of lines.
    total_lines: usize,
    /// Maximum number of lines to retain (0 = unlimited).
    max_lines: usize,
    /// Current scroll offset from bottom.
    scroll_offset: usize,
}

impl RopeBuffer {
    /// Create a new rope buffer with the given maximum capacity.
    ///
    /// # Arguments
    ///
    /// * `max_lines` - Maximum lines to retain. 0 means unlimited.
    pub fn new(max_lines: usize) -> Self {
        let mut buffer = Self {
            chunks: Vec::new(),
            total_lines: 0,
            max_lines,
            scroll_offset: 0,
        };
        // Start with one empty line
        buffer.push_line(ChunkedLine::empty());
        buffer
    }

    /// Create an unbounded rope buffer.
    pub fn unbounded() -> Self {
        Self::new(0)
    }

    /// Get the total number of lines.
    pub const fn len(&self) -> usize {
        self.total_lines
    }

    /// Check if the buffer is empty.
    pub const fn is_empty(&self) -> bool {
        self.total_lines == 0
    }

    /// Get the number of chunks.
    pub const fn chunk_count(&self) -> usize {
        self.chunks.len()
    }

    /// Get a line by global index.
    pub fn get_line(&self, index: usize) -> Option<&ChunkedLine> {
        if index >= self.total_lines {
            return None;
        }
        let chunk_idx = index / CHUNK_SIZE;
        let line_idx = index % CHUNK_SIZE;
        self.chunks.get(chunk_idx)?.get(line_idx)
    }

    /// Get a mutable reference to a line by global index.
    pub fn get_line_mut(&mut self, index: usize) -> Option<&mut ChunkedLine> {
        if index >= self.total_lines {
            return None;
        }
        let chunk_idx = index / CHUNK_SIZE;
        let line_idx = index % CHUNK_SIZE;
        self.chunks.get_mut(chunk_idx)?.get_mut(line_idx)
    }

    /// Get the current (last) line.
    pub fn current_line(&self) -> Option<&ChunkedLine> {
        if self.total_lines == 0 {
            return None;
        }
        self.get_line(self.total_lines - 1)
    }

    /// Get a mutable reference to the current (last) line.
    pub fn current_line_mut(&mut self) -> Option<&mut ChunkedLine> {
        if self.total_lines == 0 {
            return None;
        }
        let idx = self.total_lines - 1;
        self.get_line_mut(idx)
    }

    /// Push a new line to the buffer.
    pub fn push_line(&mut self, line: ChunkedLine) {
        // Check if we need a new chunk
        if self.chunks.is_empty() || self.chunks.last().is_none_or(Chunk::is_full) {
            self.chunks.push(Chunk::new());
        }

        // Push to the last chunk
        if let Some(chunk) = self.chunks.last_mut() {
            chunk.push(line);
            self.total_lines += 1;
        }

        // Enforce max_lines if set
        if self.max_lines > 0 && self.total_lines > self.max_lines {
            self.trim_front();
        }
    }

    /// Add a new empty line.
    pub fn newline(&mut self) {
        self.push_line(ChunkedLine::empty());
    }

    /// Append cells to the current line.
    pub fn append(&mut self, cells: impl Iterator<Item = Cell>) {
        if let Some(line) = self.current_line_mut() {
            line.content.extend(cells);
        }
    }

    /// Clear all content.
    pub fn clear(&mut self) {
        self.chunks.clear();
        self.total_lines = 0;
        self.scroll_offset = 0;
        self.push_line(ChunkedLine::empty());
    }

    /// Get the current scroll offset.
    pub const fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    /// Scroll up by the given number of lines.
    pub fn scroll_up(&mut self, lines: usize) {
        let max_offset = self.total_lines.saturating_sub(1);
        self.scroll_offset = (self.scroll_offset + lines).min(max_offset);
    }

    /// Scroll down by the given number of lines.
    pub const fn scroll_down(&mut self, lines: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(lines);
    }

    /// Scroll to the bottom (most recent content).
    pub const fn scroll_to_bottom(&mut self) {
        self.scroll_offset = 0;
    }

    /// Iterate over lines in the visible range.
    ///
    /// Returns an iterator over (`line_index`, &`ChunkedLine`) for lines
    /// that should be visible given the viewport height.
    pub fn visible_lines(&self, viewport_height: usize) -> impl Iterator<Item = (usize, &ChunkedLine)> {
        let end = self.total_lines.saturating_sub(self.scroll_offset);
        let start = end.saturating_sub(viewport_height);
        
        (start..end).filter_map(move |i| {
            self.get_line(i).map(|line| (i, line))
        })
    }

    /// Trim lines from the front to stay within `max_lines`.
    fn trim_front(&mut self) {
        while self.total_lines > self.max_lines && !self.chunks.is_empty() {
            // Remove the first chunk
            let removed_chunk = self.chunks.remove(0);
            self.total_lines -= removed_chunk.len();
            
            // Adjust scroll offset
            if self.scroll_offset > removed_chunk.len() {
                self.scroll_offset -= removed_chunk.len();
            } else {
                self.scroll_offset = 0;
            }
        }
    }

    /// Get memory usage statistics.
    pub fn memory_stats(&self) -> RopeMemoryStats {
        let mut total_cells = 0;
        for chunk in &self.chunks {
            for line in &chunk.lines {
                total_cells += line.content.len();
            }
        }

        RopeMemoryStats {
            chunks: self.chunks.len(),
            lines: self.total_lines,
            cells: total_cells,
            bytes_estimated: self.chunks.len() * std::mem::size_of::<Chunk>()
                + self.total_lines * std::mem::size_of::<ChunkedLine>()
                + total_cells * std::mem::size_of::<Cell>(),
        }
    }
}

/// Memory usage statistics for a rope buffer.
#[derive(Debug, Clone, Copy)]
pub struct RopeMemoryStats {
    /// Number of chunks.
    pub chunks: usize,
    /// Number of lines.
    pub lines: usize,
    /// Number of cells.
    pub cells: usize,
    /// Estimated memory usage in bytes.
    pub bytes_estimated: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rope_buffer_basic() {
        let mut buffer = RopeBuffer::new(1000);
        
        assert_eq!(buffer.len(), 1); // Starts with empty line
        
        buffer.append([Cell::new('H'), Cell::new('i')].into_iter());
        assert_eq!(buffer.current_line().unwrap().len(), 2);
        
        buffer.newline();
        assert_eq!(buffer.len(), 2);
    }

    #[test]
    fn test_rope_buffer_chunks() {
        let mut buffer = RopeBuffer::unbounded();
        
        // Add enough lines to create multiple chunks
        for i in 0..200 {
            buffer.newline();
            buffer.append([Cell::new(char::from_u32(('a' as u32) + (i % 26)).unwrap())].into_iter());
        }
        
        // Should have multiple chunks
        assert!(buffer.chunk_count() > 1);
        assert_eq!(buffer.len(), 201); // 1 initial + 200 new
    }

    #[test]
    fn test_rope_buffer_max_lines() {
        let mut buffer = RopeBuffer::new(100);
        
        for _ in 0..200 {
            buffer.newline();
        }
        
        // Should be trimmed to around max_lines
        // Note: trimming is chunk-based, so might be slightly over
        assert!(buffer.len() <= 100 + CHUNK_SIZE);
    }

    #[test]
    fn test_rope_buffer_scroll() {
        let mut buffer = RopeBuffer::new(1000);
        
        for _ in 0..50 {
            buffer.newline();
        }
        
        assert_eq!(buffer.scroll_offset(), 0);
        
        buffer.scroll_up(10);
        assert_eq!(buffer.scroll_offset(), 10);
        
        buffer.scroll_down(5);
        assert_eq!(buffer.scroll_offset(), 5);
        
        buffer.scroll_to_bottom();
        assert_eq!(buffer.scroll_offset(), 0);
    }

    #[test]
    fn test_rope_buffer_visible_lines() {
        let mut buffer = RopeBuffer::new(1000);
        
        for i in 0..20 {
            buffer.append([Cell::new(char::from_u32('a' as u32 + i).unwrap())].into_iter());
            buffer.newline();
        }
        
        let visible: Vec<_> = buffer.visible_lines(10).collect();
        assert_eq!(visible.len(), 10);
    }

    #[test]
    fn test_rope_buffer_memory_stats() {
        let mut buffer = RopeBuffer::new(1000);
        
        for _ in 0..100 {
            buffer.append([Cell::new('x'); 80].into_iter());
            buffer.newline();
        }
        
        let stats = buffer.memory_stats();
        assert_eq!(stats.lines, 101);
        assert_eq!(stats.cells, 8000);
        assert!(stats.bytes_estimated > 0);
    }
}
