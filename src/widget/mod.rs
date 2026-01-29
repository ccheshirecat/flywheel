//! Streaming Widget: Optimistic append for high-frequency token streaming.
//!
//! This module implements a specialized widget for displaying streaming text
//! from LLM agents at 100+ tokens per second without flickering.
//!
//! # Architecture
//!
//! The streaming widget uses two rendering paths:
//!
//! 1. **Fast Path**: When appending text that fits on the current line without
//!    wrapping or scrolling, we bypass the full diffing engine and emit direct
//!    ANSI sequences for the new characters. This is the common case.
//!
//! 2. **Slow Path**: When text wraps to a new line or causes scrolling, we
//!    mark the affected region as dirty and let the diffing engine handle it.
//!
//! # Example
//!
//! ```rust,ignore
//! use flywheel::widget::StreamWidget;
//!
//! let mut stream = StreamWidget::new(Rect::new(0, 0, 80, 24));
//! stream.append("Hello, ");
//! stream.append("world!\n");
//! stream.append("Streaming tokens...");
//! ```

mod stream;
mod scroll_buffer;

pub use stream::{StreamWidget, StreamConfig, AppendResult};
pub use scroll_buffer::ScrollBuffer;
