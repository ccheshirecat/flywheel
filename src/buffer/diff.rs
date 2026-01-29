//! Diffing Engine: Generate minimal ANSI sequences from buffer changes.
//!
//! This module implements the core anti-flicker logic:
//! 1. Compare Current and Next buffers
//! 2. Generate minimal ANSI escape sequences for changed cells
//! 3. Optimize cursor movements (skip if adjacent)
//! 4. Track color state to avoid redundant SGR sequences
//!
//! All output is accumulated in a single buffer and flushed with one syscall.

use super::{Buffer, Cell, CellFlags, Modifiers, Rgb};
use crate::layout::Rect;
use std::io::Write;

/// State tracker for the diffing algorithm.
///
/// This tracks the "current" terminal state (cursor position, colors, modifiers)
/// to minimize the number of escape sequences we need to emit.
#[derive(Debug, Clone)]
pub struct DiffState {
    /// Last known cursor X position (0-indexed).
    cursor_x: u16,
    /// Last known cursor Y position (0-indexed).
    cursor_y: u16,
    /// Last emitted foreground color.
    fg: Option<Rgb>,
    /// Last emitted background color.
    bg: Option<Rgb>,
    /// Last emitted modifiers.
    modifiers: Option<Modifiers>,
}

impl Default for DiffState {
    fn default() -> Self {
        Self::new()
    }
}

impl DiffState {
    /// Create a new diff state with unknown terminal state.
    pub const fn new() -> Self {
        Self {
            cursor_x: 0,
            cursor_y: 0,
            fg: None,
            bg: None,
            modifiers: None,
        }
    }

    /// Reset the state (e.g., after a full screen clear).
    pub const fn reset(&mut self) {
        self.fg = None;
        self.bg = None;
        self.modifiers = None;
        // Force cursor move on next write
        self.cursor_x = u16::MAX;
        self.cursor_y = u16::MAX;
    }
}

/// Result of a diff operation.
#[derive(Debug, Clone, Default)]
pub struct DiffResult {
    /// Number of cells that were different.
    pub cells_changed: usize,
    /// Number of cursor move sequences emitted.
    pub cursor_moves: usize,
    /// Number of color change sequences emitted.
    pub color_changes: usize,
    /// Number of modifier change sequences emitted.
    pub modifier_changes: usize,
}

/// Render the difference between two buffers into an ANSI sequence buffer.
///
/// This is the core diffing function. It compares `current` and `next` buffers,
/// generating minimal ANSI escape sequences for only the cells that changed.
///
/// # Optimizations
///
/// 1. **Cursor movement**: Skips explicit moves when writing adjacent cells
/// 2. **Color tracking**: Only emits color changes when fg/bg actually differ
/// 3. **Modifier tracking**: Only emits modifier changes when needed
/// 4. **Dirty rectangles**: Only iterates over specified dirty regions
///
/// # Arguments
///
/// * `current` - The currently displayed buffer
/// * `next` - The buffer to transition to
/// * `dirty_rects` - Regions to check for changes (empty = full buffer)
/// * `output` - Buffer to write ANSI sequences to
/// * `state` - Mutable state tracking cursor/color positions
///
/// # Returns
///
/// Statistics about the diff operation.
pub fn render_diff(
    current: &Buffer,
    next: &Buffer,
    dirty_rects: &[Rect],
    output: &mut Vec<u8>,
    state: &mut DiffState,
) -> DiffResult {
    debug_assert_eq!(current.width(), next.width());
    debug_assert_eq!(current.height(), next.height());

    let mut result = DiffResult::default();
    let width = current.width();
    let height = current.height();

    // If no dirty rects specified, diff the entire buffer
    let full_rect = Rect::from_size(width, height);
    let rects: &[Rect] = if dirty_rects.is_empty() {
        std::slice::from_ref(&full_rect)
    } else {
        dirty_rects
    };

    for rect in rects {
        diff_rect(current, next, *rect, output, state, &mut result);
    }

    result
}

/// Diff a single rectangular region.
fn diff_rect(
    current: &Buffer,
    next: &Buffer,
    rect: Rect,
    output: &mut Vec<u8>,
    state: &mut DiffState,
    result: &mut DiffResult,
) {
    let width = current.width();

    // Clamp rect to buffer bounds
    let x_end = (rect.x + rect.width).min(width);
    let y_end = (rect.y + rect.height).min(current.height());

    for y in rect.y..y_end {
        for x in rect.x..x_end {
            let idx = (y as usize) * (width as usize) + (x as usize);
            let current_cell = &current.cells()[idx];
            let next_cell = &next.cells()[idx];

            // Skip if cells are identical
            if current_cell == next_cell {
                continue;
            }

            // Skip wide-character continuation cells (handled by the main cell)
            if next_cell.is_wide_continuation() {
                continue;
            }

            result.cells_changed += 1;

            // Emit cursor move if not adjacent to last position
            if state.cursor_y != y || state.cursor_x != x {
                emit_cursor_move(output, x, y);
                state.cursor_x = x;
                state.cursor_y = y;
                result.cursor_moves += 1;
            }

            // Handle modifier resets first
            // If we need to disable any modifiers, we must emit a full reset (\x1b[0m)
            // which also clears colors.
            let next_mods = next_cell.modifiers();
            let current_mods = state.modifiers.unwrap_or(Modifiers::empty());
            let removed_mods = current_mods.difference(next_mods);

            if !removed_mods.is_empty() {
                output.extend_from_slice(b"\x1b[0m");
                state.fg = None;
                state.bg = None;
                state.modifiers = None;
            }

            // Emit color changes if needed
            if state.fg != Some(next_cell.fg()) {
                emit_fg_color(output, next_cell.fg());
                state.fg = Some(next_cell.fg());
                result.color_changes += 1;
            }

            if state.bg != Some(next_cell.bg()) {
                emit_bg_color(output, next_cell.bg());
                state.bg = Some(next_cell.bg());
                result.color_changes += 1;
            }

            // Emit modifier additions if needed
            if state.modifiers != Some(next_mods) {
                // Logic here only handles additions because we already handled removals
                // (if any removal occurred, we reset state.modifiers to None)
                emit_modifiers(output, next_mods, state.modifiers);
                state.modifiers = Some(next_mods);
                result.modifier_changes += 1;
            }

            // Emit the grapheme
            emit_grapheme(output, next_cell, next);

            // Update cursor position (advances by display width)
            let advance = u16::from(next_cell.display_width().max(1));
            state.cursor_x += advance;
        }
    }
}

/// Emit a cursor move sequence.
///
/// Uses the most compact representation:
/// - `\x1b[H` for home (1,1)
/// - `\x1b[{row};{col}H` for absolute positioning
#[inline]
fn emit_cursor_move(output: &mut Vec<u8>, x: u16, y: u16) {
    // ANSI uses 1-indexed positions
    let row = y + 1;
    let col = x + 1;

    if row == 1 && col == 1 {
        output.extend_from_slice(b"\x1b[H");
    } else if col == 1 {
        // Move to column 1 of row N
        let _ = write!(output, "\x1b[{row}H");
    } else {
        let _ = write!(output, "\x1b[{row};{col}H");
    }
}

/// Emit a foreground color sequence (true color).
#[inline]
fn emit_fg_color(output: &mut Vec<u8>, color: Rgb) {
    let _ = write!(output, "\x1b[38;2;{};{};{}m", color.r, color.g, color.b);
}

/// Emit a background color sequence (true color).
#[inline]
fn emit_bg_color(output: &mut Vec<u8>, color: Rgb) {
    let _ = write!(output, "\x1b[48;2;{};{};{}m", color.r, color.g, color.b);
}

/// Emit modifier change sequences.
///
/// This handles the transition from one set of modifiers to another,
/// emitting reset + set sequences as needed.
fn emit_modifiers(output: &mut Vec<u8>, new: Modifiers, old: Option<Modifiers>) {
    let old = old.unwrap_or(Modifiers::empty());

    // If we're removing modifiers, we need to reset first
    let removed = old.difference(new);
    if removed.is_empty() {
        // Only adding modifiers, no need to reset
        let added = new.difference(old);
        emit_modifier_set(output, added);
    } else {
        // Reset all attributes, then re-apply what we want
        output.extend_from_slice(b"\x1b[0m");
        // Note: After reset, colors are also reset, so caller should
        // re-emit colors. For now, we emit all new modifiers.
        emit_modifier_set(output, new);
    }
}

/// Emit SGR sequences for a set of modifiers.
fn emit_modifier_set(output: &mut Vec<u8>, modifiers: Modifiers) {
    if modifiers.contains(Modifiers::BOLD) {
        output.extend_from_slice(b"\x1b[1m");
    }
    if modifiers.contains(Modifiers::DIM) {
        output.extend_from_slice(b"\x1b[2m");
    }
    if modifiers.contains(Modifiers::ITALIC) {
        output.extend_from_slice(b"\x1b[3m");
    }
    if modifiers.contains(Modifiers::UNDERLINE) {
        output.extend_from_slice(b"\x1b[4m");
    }
    if modifiers.contains(Modifiers::BLINK) {
        output.extend_from_slice(b"\x1b[5m");
    }
    if modifiers.contains(Modifiers::REVERSED) {
        output.extend_from_slice(b"\x1b[7m");
    }
    if modifiers.contains(Modifiers::HIDDEN) {
        output.extend_from_slice(b"\x1b[8m");
    }
    if modifiers.contains(Modifiers::STRIKETHROUGH) {
        output.extend_from_slice(b"\x1b[9m");
    }
}

/// Emit a grapheme to the output buffer.
#[inline]
fn emit_grapheme(output: &mut Vec<u8>, cell: &Cell, buffer: &Buffer) {
    if cell.flags().contains(CellFlags::OVERFLOW) {
        // Look up in overflow storage
        if let Some(idx) = cell.overflow_index()
            && let Some(grapheme) = buffer.get_overflow(idx) {
                output.extend_from_slice(grapheme.as_bytes());
                return;
            }
        // Fallback: emit a replacement character
        output.extend_from_slice("�".as_bytes());
    } else if let Some(grapheme) = cell.grapheme() {
        output.extend_from_slice(grapheme.as_bytes());
    } else {
        // Empty cell, emit space
        output.push(b' ');
    }
}

/// Perform a full buffer diff (convenience function).
///
/// This diffs the entire buffer without dirty rect optimization.
pub fn render_full_diff(
    current: &Buffer,
    next: &Buffer,
    output: &mut Vec<u8>,
    state: &mut DiffState,
) -> DiffResult {
    render_diff(current, next, &[], output, state)
}

/// Generate a full redraw sequence (no diffing).
///
/// This is used for initial render or when the terminal state is unknown.
pub fn render_full(buffer: &Buffer, output: &mut Vec<u8>) {
    let width = buffer.width();
    let height = buffer.height();

    // Hide cursor during redraw
    output.extend_from_slice(b"\x1b[?25l");

    // Move to home
    output.extend_from_slice(b"\x1b[H");

    let mut last_fg: Option<Rgb> = None;
    let mut last_bg: Option<Rgb> = None;
    let mut last_mods: Option<Modifiers> = None;

    for y in 0..height {
        if y > 0 {
            // Move to start of next line
            output.extend_from_slice(b"\r\n");
        }

        for x in 0..width {
            let idx = (y as usize) * (width as usize) + (x as usize);
            let cell = &buffer.cells()[idx];

            // Skip continuation cells
            if cell.is_wide_continuation() {
                continue;
            }

            // Emit colors if changed
            if last_fg != Some(cell.fg()) {
                emit_fg_color(output, cell.fg());
                last_fg = Some(cell.fg());
            }
            if last_bg != Some(cell.bg()) {
                emit_bg_color(output, cell.bg());
                last_bg = Some(cell.bg());
            }
            if last_mods != Some(cell.modifiers()) {
                emit_modifiers(output, cell.modifiers(), last_mods);
                last_mods = Some(cell.modifiers());
            }

            emit_grapheme(output, cell, buffer);
        }
    }

    // Reset attributes and show cursor
    output.extend_from_slice(b"\x1b[0m\x1b[?25h");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diff_identical_buffers() {
        let a = Buffer::new(10, 5);
        let b = Buffer::new(10, 5);
        let mut output = Vec::new();
        let mut state = DiffState::new();

        let result = render_full_diff(&a, &b, &mut output, &mut state);

        assert_eq!(result.cells_changed, 0);
        assert!(output.is_empty());
    }

    #[test]
    fn test_diff_single_cell_change() {
        let a = Buffer::new(10, 5);
        let mut b = Buffer::new(10, 5);

        b.set(5, 2, Cell::new('X'));

        let mut output = Vec::new();
        let mut state = DiffState::new();

        let result = render_full_diff(&a, &b, &mut output, &mut state);

        assert_eq!(result.cells_changed, 1);
        assert!(!output.is_empty());
        // Should contain cursor move and the character
        let output_str = String::from_utf8_lossy(&output);
        assert!(output_str.contains('X'));
    }

    #[test]
    fn test_diff_adjacent_cells_no_cursor_move() {
        let a = Buffer::new(10, 5);
        let mut b = Buffer::new(10, 5);

        // Three adjacent cells on same row
        b.set(0, 0, Cell::new('A'));
        b.set(1, 0, Cell::new('B'));
        b.set(2, 0, Cell::new('C'));

        let mut output = Vec::new();
        let mut state = DiffState::new();

        let result = render_full_diff(&a, &b, &mut output, &mut state);

        assert_eq!(result.cells_changed, 3);
        // No cursor moves needed: cursor starts at (0,0) and cells are adjacent
        assert_eq!(result.cursor_moves, 0);
    }

    #[test]
    fn test_diff_color_tracking() {
        let a = Buffer::new(10, 5);
        let mut b = Buffer::new(10, 5);

        let red = Rgb::new(255, 0, 0);
        // Both cells have same fg color, but will also emit bg color first time
        b.set(0, 0, Cell::new('A').with_fg(red));
        b.set(1, 0, Cell::new('B').with_fg(red)); // Same colors, no additional changes

        let mut output = Vec::new();
        let mut state = DiffState::new();

        let result = render_full_diff(&a, &b, &mut output, &mut state);

        // Two color changes for first cell (fg and bg), none for second (same colors)
        assert_eq!(result.color_changes, 2);
    }

    #[test]
    fn test_diff_dirty_rect() {
        let a = Buffer::new(20, 10);
        let mut b = Buffer::new(20, 10);

        // Changes outside dirty rect
        b.set(0, 0, Cell::new('X'));

        // Changes inside dirty rect
        b.set(10, 5, Cell::new('Y'));

        let mut output = Vec::new();
        let mut state = DiffState::new();

        // Only diff a region that includes (10,5) but not (0,0)
        let dirty = vec![Rect::new(8, 4, 5, 3)];
        let result = render_diff(&a, &b, &dirty, &mut output, &mut state);

        // Should only detect the change at (10,5)
        assert_eq!(result.cells_changed, 1);
    }

    #[test]
    fn test_cursor_move_optimization() {
        let mut output = Vec::new();

        // Home position uses short sequence
        emit_cursor_move(&mut output, 0, 0);
        assert_eq!(&output, b"\x1b[H");

        output.clear();

        // Column 1 uses shorter sequence
        emit_cursor_move(&mut output, 0, 5);
        assert_eq!(&output, b"\x1b[6H"); // Row 6 (1-indexed)

        output.clear();

        // General position
        emit_cursor_move(&mut output, 10, 5);
        assert_eq!(&output, b"\x1b[6;11H"); // Row 6, Col 11 (1-indexed)
    }

    #[test]
    fn test_render_full() {
        let mut buffer = Buffer::new(3, 2);
        buffer.set(0, 0, Cell::new('A'));
        buffer.set(1, 0, Cell::new('B'));
        buffer.set(2, 0, Cell::new('C'));

        let mut output = Vec::new();
        render_full(&buffer, &mut output);

        let output_str = String::from_utf8_lossy(&output);
        // Should start with hide cursor and home
        assert!(output_str.starts_with("\x1b[?25l\x1b[H"));
        // Should contain all characters
        assert!(output_str.contains('A'));
        assert!(output_str.contains('B'));
        assert!(output_str.contains('C'));
        // Should end with reset and show cursor
        assert!(output_str.ends_with("\x1b[0m\x1b[?25h"));
    }
}
