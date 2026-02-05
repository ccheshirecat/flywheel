//! Terminal Widget: Embedded terminal emulator for Flywheel.
//!
//! This widget uses `vt100` to provide a full terminal emulation
//! within a Flywheel widget. It handles ANSI escape sequences,
//! colors, and scrolling.

use crate::buffer::{Buffer, Cell, Rgb};
use crate::layout::Rect;
use crate::actor::InputEvent;
use crate::widget::Widget;
use std::sync::{Arc, Mutex};

/// A terminal emulator widget.
pub struct Terminal {
    bounds: Rect,
    parser: Arc<Mutex<vt100::Parser>>,
    needs_redraw: bool,
}

impl Terminal {
    /// Create a new terminal widget with the given bounds.
    pub fn new(bounds: Rect) -> Self {
        Self {
            bounds,
            parser: Arc::new(Mutex::new(vt100::Parser::new(bounds.height, bounds.width, 0))),
            needs_redraw: true,
        }
    }

    /// Process a chunk of bytes through the terminal emulator.
    pub fn write(&mut self, data: &[u8]) {
        if let Ok(mut parser) = self.parser.lock() {
            parser.process(data);
            self.needs_redraw = true;
        }
    }

    /// Clear the terminal content.
    pub fn clear(&mut self) {
        if let Ok(mut parser) = self.parser.lock() {
            *parser = vt100::Parser::new(self.bounds.height, self.bounds.width, 0);
            self.needs_redraw = true;
        }
    }
}

impl Widget for Terminal {
    fn bounds(&self) -> Rect {
        self.bounds
    }

    fn set_bounds(&mut self, bounds: Rect) {
        if bounds != self.bounds {
            self.bounds = bounds;
            if let Ok(mut parser) = self.parser.lock() {
                parser.set_size(bounds.height, bounds.width);
            }
            self.needs_redraw = true;
        }
    }

    #[allow(clippy::cast_possible_truncation)]
    fn render(&self, buffer: &mut Buffer) {
        let Ok(parser) = self.parser.lock() else { return };
        let screen = parser.screen();

        for y in 0..self.bounds.height {
            for x in 0..self.bounds.width {
                if let Some(cell) = screen.cell(y, x) {
                    let mut fl_cell = Cell::from_char(cell.contents().chars().next().unwrap_or(' '));
                    
                    // FG Color
                    match cell.fgcolor() {
                        vt100::Color::Rgb(r, g, b) => {
                            fl_cell.set_fg(Rgb::new(r, g, b));
                        }
                        vt100::Color::Idx(i) => {
                            fl_cell.set_fg(ansi_to_rgb(i));
                        }
                        vt100::Color::Default => {}
                    }
                    
                    // BG Color
                    match cell.bgcolor() {
                        vt100::Color::Rgb(r, g, b) => {
                            fl_cell.set_bg(Rgb::new(r, g, b));
                        }
                        vt100::Color::Idx(i) => {
                            fl_cell.set_bg(ansi_to_rgb(i));
                        }
                        vt100::Color::Default => {}
                    }
                    
                    buffer.set(self.bounds.x + x, self.bounds.y + y, fl_cell);
                }
            }
        }
    }

    fn handle_input(&mut self, _event: &InputEvent) -> bool {
        // Terminal doesn't handle input locally by default, 
        // it just consumes it if it's targeted?
        // Actually, the caller usually maps keys to bytes and writes to the PTY.
        false
    }

    fn needs_redraw(&self) -> bool {
        self.needs_redraw
    }

    fn clear_redraw(&mut self) {
        self.needs_redraw = false;
    }
}

/// Convert ANSI color index to RGB.
const fn ansi_to_rgb(idx: u8) -> Rgb {
    match idx {
        0 => Rgb::new(0, 0, 0),
        1 => Rgb::new(128, 0, 0),
        2 => Rgb::new(0, 128, 0),
        3 => Rgb::new(128, 128, 0),
        4 => Rgb::new(0, 0, 128),
        5 => Rgb::new(128, 0, 128),
        6 => Rgb::new(0, 128, 128),
        7 => Rgb::new(192, 192, 192),
        8 => Rgb::new(128, 128, 128),
        9 => Rgb::new(255, 0, 0),
        10 => Rgb::new(0, 255, 0),
        11 => Rgb::new(255, 255, 0),
        12 => Rgb::new(0, 0, 255),
        13 => Rgb::new(255, 0, 255),
        14 => Rgb::new(0, 255, 255),
        15 => Rgb::new(255, 255, 255),
        16..=231 => {
            let i = idx - 16;
            let r = (i / 36) % 6;
            let g = (i / 6) % 6;
            let b = i % 6;
            Rgb::new(
                if r == 0 { 0 } else { r * 40 + 55 },
                if g == 0 { 0 } else { g * 40 + 55 },
                if b == 0 { 0 } else { b * 40 + 55 },
            )
        }
        232..=255 => {
            let v = (idx - 232) * 10 + 8;
            Rgb::new(v, v, v)
        }
    }
}
