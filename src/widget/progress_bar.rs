//! Progress Bar Widget: Horizontal progress indicator.
//!
//! A horizontal progress bar with customizable styling and optional
//! percentage/label display.

use crate::actor::InputEvent;
use crate::buffer::{Buffer, Cell, Rgb};
use crate::layout::Rect;
use super::traits::Widget;

/// Visual style for the progress bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Default)]
pub enum ProgressStyle {
    /// Classic solid bar: ████████░░░░
    Solid,
    /// ASCII style: [========  ]
    Ascii,
    /// Block characters: ▓▓▓▓▓▓░░░░
    #[default]
    Block,
    /// Thin line: ───────────
    Line,
}


/// Configuration for the progress bar widget.
#[derive(Debug, Clone)]
pub struct ProgressBarConfig {
    /// Style of the bar.
    pub style: ProgressStyle,
    /// Filled portion color.
    pub filled_fg: Rgb,
    /// Empty portion color.
    pub empty_fg: Rgb,
    /// Background color.
    pub bg: Rgb,
    /// Whether to show percentage text.
    pub show_percentage: bool,
    /// Percentage text color.
    pub percentage_fg: Rgb,
    /// Optional label to show.
    pub label: Option<String>,
    /// Label color.
    pub label_fg: Rgb,
}

impl Default for ProgressBarConfig {
    fn default() -> Self {
        Self {
            style: ProgressStyle::Block,
            filled_fg: Rgb::new(0, 200, 100),
            empty_fg: Rgb::new(60, 60, 60),
            bg: Rgb::new(30, 30, 30),
            show_percentage: true,
            percentage_fg: Rgb::WHITE,
            label: None,
            label_fg: Rgb::new(150, 150, 150),
        }
    }
}

/// A horizontal progress bar widget.
#[derive(Debug)]
pub struct ProgressBar {
    /// Current progress (0.0 to 1.0).
    progress: f32,
    /// Widget bounds.
    bounds: Rect,
    /// Configuration.
    config: ProgressBarConfig,
    /// Needs redraw flag.
    dirty: bool,
}

impl ProgressBar {
    /// Create a new progress bar with the given bounds.
    pub fn new(bounds: Rect) -> Self {
        Self {
            progress: 0.0,
            bounds,
            config: ProgressBarConfig::default(),
            dirty: true,
        }
    }

    /// Create a new progress bar with custom configuration.
    pub const fn with_config(bounds: Rect, config: ProgressBarConfig) -> Self {
        Self {
            progress: 0.0,
            bounds,
            config,
            dirty: true,
        }
    }

    /// Set the progress value (clamped to 0.0-1.0).
    pub const fn set_progress(&mut self, progress: f32) {
        self.progress = progress.clamp(0.0, 1.0);
        self.dirty = true;
    }

    /// Get the current progress value.
    pub const fn progress(&self) -> f32 {
        self.progress
    }

    /// Set the label text.
    pub fn set_label(&mut self, label: impl Into<String>) {
        self.config.label = Some(label.into());
        self.dirty = true;
    }

    /// Clear the label.
    pub fn clear_label(&mut self) {
        self.config.label = None;
        self.dirty = true;
    }

    /// Increment progress by a delta (clamped).
    pub fn increment(&mut self, delta: f32) {
        self.set_progress(self.progress + delta);
    }

    /// Check if progress is complete (>= 1.0).
    pub fn is_complete(&self) -> bool {
        self.progress >= 1.0
    }

    /// Get the filled and empty characters for the current style.
    const fn style_chars(&self) -> (char, char) {
        match self.config.style {
            ProgressStyle::Solid => ('█', '░'),
            ProgressStyle::Ascii => ('=', ' '),
            ProgressStyle::Block => ('▓', '░'),
            ProgressStyle::Line => ('─', '─'),
        }
    }
}

impl Widget for ProgressBar {
    fn bounds(&self) -> Rect {
        self.bounds
    }

    fn set_bounds(&mut self, bounds: Rect) {
        self.bounds = bounds;
        self.dirty = true;
    }

    #[allow(clippy::cast_possible_truncation)]
    #[allow(clippy::cast_sign_loss)]
    #[allow(clippy::cast_precision_loss)]
    fn render(&self, buffer: &mut Buffer) {
        let x = self.bounds.x;
        let y = self.bounds.y;
        let width = self.bounds.width as usize;

        // Clear the line with background
        for i in 0..self.bounds.width {
            buffer.set(x + i, y, Cell::new(' ').with_bg(self.config.bg));
        }

        // Calculate space for label and percentage
        let label_len = self.config.label.as_ref().map_or(0, |l| l.chars().count() + 1);
        let pct_len = if self.config.show_percentage { 5 } else { 0 }; // " 100%"
        let bar_width = width.saturating_sub(label_len + pct_len);

        if bar_width == 0 {
            return;
        }

        let mut offset = x;

        // Draw label if present
        if let Some(ref label) = self.config.label {
            for c in label.chars().take(width / 3) {
                buffer.set(offset, y, Cell::new(c)
                    .with_fg(self.config.label_fg)
                    .with_bg(self.config.bg));
                offset += 1;
            }
            buffer.set(offset, y, Cell::new(' ').with_bg(self.config.bg));
            offset += 1;
        }

        // Draw progress bar
        let (filled_char, empty_char) = self.style_chars();
        let filled_count = (self.progress * bar_width as f32).round() as usize;

        for i in 0..bar_width {
            let (c, fg) = if i < filled_count {
                (filled_char, self.config.filled_fg)
            } else {
                (empty_char, self.config.empty_fg)
            };
            buffer.set(offset + i as u16, y, Cell::new(c)
                .with_fg(fg)
                .with_bg(self.config.bg));
        }
        offset += bar_width as u16;

        // Draw percentage
        if self.config.show_percentage {
            let pct = format!(" {:>3}%", (self.progress * 100.0).round() as u32);
            for c in pct.chars() {
                buffer.set(offset, y, Cell::new(c)
                    .with_fg(self.config.percentage_fg)
                    .with_bg(self.config.bg));
                offset += 1;
            }
        }
    }

    fn handle_input(&mut self, _event: &InputEvent) -> bool {
        // Progress bar doesn't handle input
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
    fn test_progress_bar_basic() {
        let mut bar = ProgressBar::new(Rect::new(0, 0, 80, 1));
        
        assert_eq!(bar.progress(), 0.0);
        
        bar.set_progress(0.5);
        assert_eq!(bar.progress(), 0.5);
        
        bar.set_progress(1.5); // Should clamp
        assert_eq!(bar.progress(), 1.0);
    }

    #[test]
    fn test_progress_bar_increment() {
        let mut bar = ProgressBar::new(Rect::new(0, 0, 80, 1));
        
        bar.increment(0.25);
        assert!((bar.progress() - 0.25).abs() < f32::EPSILON);
        
        bar.increment(0.25);
        assert!((bar.progress() - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_progress_bar_complete() {
        let mut bar = ProgressBar::new(Rect::new(0, 0, 80, 1));
        
        assert!(!bar.is_complete());
        
        bar.set_progress(1.0);
        assert!(bar.is_complete());
    }
}
