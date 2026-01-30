//! Text Input Widget: Single-line text input with cursor.
//!
//! A focused, single-line text input widget with cursor blinking,
//! character insertion, deletion, and navigation.

use crate::actor::{InputEvent, KeyCode};
use crate::buffer::{Buffer, Cell, Rgb};
use crate::layout::Rect;
use super::traits::Widget;

/// Configuration for the text input widget.
#[derive(Debug, Clone)]
pub struct TextInputConfig {
    /// Foreground color for text.
    pub fg: Rgb,
    /// Background color.
    pub bg: Rgb,
    /// Cursor color.
    pub cursor_fg: Rgb,
    /// Placeholder text shown when empty.
    pub placeholder: String,
    /// Placeholder text color.
    pub placeholder_fg: Rgb,
    /// Prompt prefix (e.g., "> ").
    pub prompt: String,
    /// Prompt color.
    pub prompt_fg: Rgb,
}

impl Default for TextInputConfig {
    fn default() -> Self {
        Self {
            fg: Rgb::WHITE,
            bg: Rgb::new(30, 30, 30),
            cursor_fg: Rgb::new(0, 255, 255),
            placeholder: String::new(),
            placeholder_fg: Rgb::new(100, 100, 100),
            prompt: String::from("> "),
            prompt_fg: Rgb::new(0, 255, 255),
        }
    }
}

/// A single-line text input widget with cursor and editing support.
#[derive(Debug)]
pub struct TextInput {
    /// Current text content.
    content: String,
    /// Cursor position (byte offset, not char offset for simplicity).
    cursor: usize,
    /// Widget bounds.
    bounds: Rect,
    /// Whether this widget has focus.
    focused: bool,
    /// Configuration.
    config: TextInputConfig,
    /// Frame counter for cursor blinking.
    frame: u64,
    /// Needs redraw flag.
    dirty: bool,
}

impl TextInput {
    /// Create a new text input widget with the given bounds.
    pub fn new(bounds: Rect) -> Self {
        Self {
            content: String::new(),
            cursor: 0,
            bounds,
            focused: true,
            config: TextInputConfig::default(),
            frame: 0,
            dirty: true,
        }
    }

    /// Create a new text input widget with custom configuration.
    pub const fn with_config(bounds: Rect, config: TextInputConfig) -> Self {
        Self {
            content: String::new(),
            cursor: 0,
            bounds,
            focused: true,
            config,
            frame: 0,
            dirty: true,
        }
    }

    /// Get the current text content.
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Set the content, moving cursor to end.
    pub fn set_content(&mut self, content: &str) {
        self.content = content.to_string();
        self.cursor = self.content.len();
        self.dirty = true;
    }

    /// Clear the content.
    pub fn clear(&mut self) {
        self.content.clear();
        self.cursor = 0;
        self.dirty = true;
    }

    /// Check if the input is empty.
    pub const fn is_empty(&self) -> bool {
        self.content.is_empty()
    }

    /// Set focus state.
    pub const fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
        self.dirty = true;
    }

    /// Check if focused.
    pub const fn is_focused(&self) -> bool {
        self.focused
    }

    /// Advance frame for cursor blink animation.
    pub const fn tick(&mut self) {
        self.frame = self.frame.wrapping_add(1);
        // Only mark dirty if focused (cursor blink matters)
        if self.focused && self.frame.is_multiple_of(15) {
            self.dirty = true;
        }
    }

    /// Insert a character at the cursor position.
    fn insert_char(&mut self, c: char) {
        self.content.insert(self.cursor, c);
        self.cursor += c.len_utf8();
        self.dirty = true;
    }

    /// Delete the character before the cursor.
    fn backspace(&mut self) {
        if self.cursor > 0 {
            // Find the previous char boundary
            let prev = self.content[..self.cursor]
                .char_indices()
                .last()
                .map_or(0, |(i, _)| i);
            self.content.remove(prev);
            self.cursor = prev;
            self.dirty = true;
        }
    }

    /// Delete the character at the cursor.
    fn delete(&mut self) {
        if self.cursor < self.content.len() {
            self.content.remove(self.cursor);
            self.dirty = true;
        }
    }

    /// Move cursor left.
    fn cursor_left(&mut self) {
        if self.cursor > 0 {
            // Find previous char boundary
            self.cursor = self.content[..self.cursor]
                .char_indices()
                .last()
                .map_or(0, |(i, _)| i);
            self.dirty = true;
        }
    }

    /// Move cursor right.
    fn cursor_right(&mut self) {
        if self.cursor < self.content.len() {
            // Find next char boundary
            if let Some(c) = self.content[self.cursor..].chars().next() {
                self.cursor += c.len_utf8();
                self.dirty = true;
            }
        }
    }

    /// Move cursor to start.
    const fn cursor_home(&mut self) {
        if self.cursor != 0 {
            self.cursor = 0;
            self.dirty = true;
        }
    }

    /// Move cursor to end.
    const fn cursor_end(&mut self) {
        let end = self.content.len();
        if self.cursor != end {
            self.cursor = end;
            self.dirty = true;
        }
    }
}

impl Widget for TextInput {
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

        // Draw prompt
        let prompt_len = self.config.prompt.chars().count();
        for (i, c) in self.config.prompt.chars().enumerate() {
            #[allow(clippy::cast_possible_truncation)]
            let px = x + i as u16;
            if (px as usize) < x as usize + width {
                buffer.set(px, y, Cell::new(c)
                    .with_fg(self.config.prompt_fg)
                    .with_bg(self.config.bg));
            }
        }

        #[allow(clippy::cast_possible_truncation)]
        let text_start = x + prompt_len as u16;
        let text_width = width.saturating_sub(prompt_len);

        if self.content.is_empty() && !self.config.placeholder.is_empty() {
            // Draw placeholder
            for (i, c) in self.config.placeholder.chars().take(text_width).enumerate() {
                #[allow(clippy::cast_possible_truncation)]
                let px = text_start + i as u16;
                buffer.set(px, y, Cell::new(c)
                    .with_fg(self.config.placeholder_fg)
                    .with_bg(self.config.bg));
            }
        } else {
            // Draw content
            // Calculate visible window based on cursor position
            let cursor_char_pos = self.content[..self.cursor].chars().count();
            let content_chars: Vec<char> = self.content.chars().collect();
            
            // Calculate scroll offset to keep cursor visible
            let scroll_offset = if cursor_char_pos >= text_width {
                cursor_char_pos - text_width + 1
            } else {
                0
            };

            for (i, &c) in content_chars.iter().skip(scroll_offset).take(text_width).enumerate() {
                #[allow(clippy::cast_possible_truncation)]
                let px = text_start + i as u16;
                let is_cursor = self.focused 
                    && (i + scroll_offset) == cursor_char_pos
                    && self.frame % 30 < 15;

                if is_cursor {
                    buffer.set(px, y, Cell::new(c)
                        .with_fg(self.config.bg)
                        .with_bg(self.config.cursor_fg));
                } else {
                    buffer.set(px, y, Cell::new(c)
                        .with_fg(self.config.fg)
                        .with_bg(self.config.bg));
                }
            }

            // Draw cursor at end if needed
            #[allow(clippy::cast_possible_truncation)]
            let cursor_visual_pos = cursor_char_pos.saturating_sub(scroll_offset) as u16;
            if self.focused 
                && cursor_char_pos == content_chars.len() 
                && cursor_visual_pos < text_width as u16
                && self.frame % 30 < 15
            {
                let cx = text_start + cursor_visual_pos;
                buffer.set(cx, y, Cell::new('█')
                    .with_fg(self.config.cursor_fg)
                    .with_bg(self.config.bg));
            }
        }
    }

    fn handle_input(&mut self, event: &InputEvent) -> bool {
        if !self.focused {
            return false;
        }

        if let InputEvent::Key { code, modifiers } = event {
            match code {
                KeyCode::Char(c) => {
                    if !modifiers.control && !modifiers.alt {
                        self.insert_char(*c);
                        return true;
                    }
                }
                KeyCode::Backspace => {
                    self.backspace();
                    return true;
                }
                KeyCode::Delete => {
                    self.delete();
                    return true;
                }
                KeyCode::Left => {
                    self.cursor_left();
                    return true;
                }
                KeyCode::Right => {
                    self.cursor_right();
                    return true;
                }
                KeyCode::Home => {
                    self.cursor_home();
                    return true;
                }
                KeyCode::End => {
                    self.cursor_end();
                    return true;
                }
                _ => {}
            }
        }

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
    fn test_text_input_basic() {
        let mut input = TextInput::new(Rect::new(0, 0, 80, 1));
        
        // Insert characters
        input.insert_char('H');
        input.insert_char('i');
        assert_eq!(input.content(), "Hi");
        assert_eq!(input.cursor, 2);
    }

    #[test]
    fn test_text_input_backspace() {
        let mut input = TextInput::new(Rect::new(0, 0, 80, 1));
        input.set_content("Hello");
        
        input.backspace();
        assert_eq!(input.content(), "Hell");
    }

    #[test]
    fn test_text_input_cursor_movement() {
        let mut input = TextInput::new(Rect::new(0, 0, 80, 1));
        input.set_content("Hello");
        
        input.cursor_left();
        assert_eq!(input.cursor, 4);
        
        input.cursor_home();
        assert_eq!(input.cursor, 0);
        
        input.cursor_end();
        assert_eq!(input.cursor, 5);
    }
}
