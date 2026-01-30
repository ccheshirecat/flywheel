//! Status Bar Widget: Three-section status bar.
//!
//! A horizontal status bar with left, center, and right sections.
//! Commonly used at the top or bottom of the terminal.

use crate::actor::InputEvent;
use crate::buffer::{Buffer, Cell, Rgb};
use crate::layout::Rect;
use super::traits::Widget;

/// Configuration for the status bar widget.
#[derive(Debug, Clone)]
pub struct StatusBarConfig {
    /// Background color.
    pub bg: Rgb,
    /// Left section text color.
    pub left_fg: Rgb,
    /// Center section text color.
    pub center_fg: Rgb,
    /// Right section text color.
    pub right_fg: Rgb,
}

impl Default for StatusBarConfig {
    fn default() -> Self {
        Self {
            bg: Rgb::new(40, 40, 40),
            left_fg: Rgb::WHITE,
            center_fg: Rgb::new(150, 150, 150),
            right_fg: Rgb::new(100, 200, 100),
        }
    }
}

/// A three-section status bar (left, center, right).
#[derive(Debug)]
pub struct StatusBar {
    /// Left section content.
    left: String,
    /// Center section content.
    center: String,
    /// Right section content.
    right: String,
    /// Widget bounds.
    bounds: Rect,
    /// Configuration.
    config: StatusBarConfig,
    /// Needs redraw flag.
    dirty: bool,
}

impl StatusBar {
    /// Create a new status bar with the given bounds.
    pub fn new(bounds: Rect) -> Self {
        Self {
            left: String::new(),
            center: String::new(),
            right: String::new(),
            bounds,
            config: StatusBarConfig::default(),
            dirty: true,
        }
    }

    /// Create a new status bar with custom configuration.
    pub const fn with_config(bounds: Rect, config: StatusBarConfig) -> Self {
        Self {
            left: String::new(),
            center: String::new(),
            right: String::new(),
            bounds,
            config,
            dirty: true,
        }
    }

    /// Set the left section content.
    pub fn set_left(&mut self, text: impl Into<String>) {
        self.left = text.into();
        self.dirty = true;
    }

    /// Set the center section content.
    pub fn set_center(&mut self, text: impl Into<String>) {
        self.center = text.into();
        self.dirty = true;
    }

    /// Set the right section content.
    pub fn set_right(&mut self, text: impl Into<String>) {
        self.right = text.into();
        self.dirty = true;
    }

    /// Set all sections at once.
    pub fn set_all(&mut self, left: impl Into<String>, center: impl Into<String>, right: impl Into<String>) {
        self.left = left.into();
        self.center = center.into();
        self.right = right.into();
        self.dirty = true;
    }

    /// Get the left section content.
    pub fn left(&self) -> &str {
        &self.left
    }

    /// Get the center section content.
    pub fn center(&self) -> &str {
        &self.center
    }

    /// Get the right section content.
    pub fn right(&self) -> &str {
        &self.right
    }
}

impl Widget for StatusBar {
    fn bounds(&self) -> Rect {
        self.bounds
    }

    fn set_bounds(&mut self, bounds: Rect) {
        self.bounds = bounds;
        self.dirty = true;
    }

    fn render(&self, buffer: &mut Buffer) {
        let x = self.bounds.x;
        let y = self.bounds.y;
        let width = self.bounds.width as usize;

        // Clear the line with background
        for i in 0..self.bounds.width {
            buffer.set(x + i, y, Cell::new(' ').with_bg(self.config.bg));
        }

        // Draw left section (left-aligned)
        let left_chars: Vec<char> = self.left.chars().collect();
        for (i, &c) in left_chars.iter().take(width / 3).enumerate() {
            #[allow(clippy::cast_possible_truncation)]
            let px = x + i as u16;
            buffer.set(px, y, Cell::new(c)
                .with_fg(self.config.left_fg)
                .with_bg(self.config.bg));
        }

        // Draw center section (centered)
        let center_chars: Vec<char> = self.center.chars().collect();
        let center_len = center_chars.len().min(width / 3);
        #[allow(clippy::cast_possible_truncation)]
        let center_start = x + ((width - center_len) / 2) as u16;
        for (i, &c) in center_chars.iter().take(center_len).enumerate() {
            #[allow(clippy::cast_possible_truncation)]
            let px = center_start + i as u16;
            buffer.set(px, y, Cell::new(c)
                .with_fg(self.config.center_fg)
                .with_bg(self.config.bg));
        }

        // Draw right section (right-aligned)
        let right_chars: Vec<char> = self.right.chars().collect();
        let right_len = right_chars.len().min(width / 3);
        #[allow(clippy::cast_possible_truncation)]
        let right_start = x + (width - right_len) as u16;
        for (i, &c) in right_chars.iter().take(right_len).enumerate() {
            #[allow(clippy::cast_possible_truncation)]
            let px = right_start + i as u16;
            buffer.set(px, y, Cell::new(c)
                .with_fg(self.config.right_fg)
                .with_bg(self.config.bg));
        }
    }

    fn handle_input(&mut self, _event: &InputEvent) -> bool {
        // Status bar doesn't handle input
        false
    }

    fn needs_redraw(&self) -> bool {
        self.dirty
    }

    fn clear_redraw(&mut self) {
        self.dirty = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_bar_basic() {
        let mut bar = StatusBar::new(Rect::new(0, 0, 80, 1));
        
        bar.set_left("Left");
        bar.set_center("Center");
        bar.set_right("Right");
        
        assert_eq!(bar.left(), "Left");
        assert_eq!(bar.center(), "Center");
        assert_eq!(bar.right(), "Right");
    }

    #[test]
    fn test_status_bar_set_all() {
        let mut bar = StatusBar::new(Rect::new(0, 0, 80, 1));
        
        bar.set_all("A", "B", "C");
        
        assert_eq!(bar.left(), "A");
        assert_eq!(bar.center(), "B");
        assert_eq!(bar.right(), "C");
    }
}
