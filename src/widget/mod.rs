//! Widget System: Composable UI components for terminal applications.
//!
//! This module provides a collection of widgets that implement the [`Widget`] trait,
//! allowing them to be composed into complex layouts and rendered to the terminal.
//!
//! # Available Widgets
//!
//! - [`StreamWidget`] - Scrolling text viewport for streaming content (LLM output)
//! - [`TextInput`] - Single-line text input with cursor
//! - [`StatusBar`] - Three-section status bar (left, center, right)
//! - [`ProgressBar`] - Horizontal progress indicator
//!
//! # Widget Trait
//!
//! All widgets implement the [`Widget`] trait, which provides:
//! - `bounds()` / `set_bounds()` - Layout management
//! - `render(&mut Buffer)` - Draw to the buffer
//! - `handle_input(&InputEvent) -> bool` - Input handling
//! - `needs_redraw()` / `clear_redraw()` - Dirty tracking
//!
//! # Example
//!
//! ```rust,ignore
//! use flywheel::widget::{Widget, TextInput, StatusBar};
//! use flywheel::Rect;
//!
//! let mut input = TextInput::new(Rect::new(0, 23, 80, 1));
//! let mut status = StatusBar::new(Rect::new(0, 0, 80, 1));
//!
//! status.set_all("Flywheel", "v0.1.0", "60 FPS");
//! input.set_content("Hello, world!");
//!
//! // Render both widgets
//! input.render(buffer);
//! status.render(buffer);
//! ```

mod traits;
mod stream;
mod scroll_buffer;
mod text_input;
mod status_bar;
mod progress_bar;
mod terminal;

pub use traits::Widget;
pub use stream::{StreamWidget, StreamConfig, AppendResult};
pub use scroll_buffer::ScrollBuffer;
pub use text_input::{TextInput, TextInputConfig};
pub use status_bar::{StatusBar, StatusBarConfig};
pub use progress_bar::{ProgressBar, ProgressBarConfig, ProgressStyle};
pub use terminal::Terminal;

