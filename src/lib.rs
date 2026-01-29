//! # Flywheel
//!
//! A zero-flicker terminal compositor for Agentic CLIs.
//!
//! Flywheel is a purpose-built TUI engine designed for high-frequency token streaming
//! (100+ tokens/s) without flickering, blocking, or latency.
//!
//! ## Core Concepts
//!
//! - **Double-buffered rendering**: Current and Next buffers with minimal diff
//! - **Dirty rectangles**: Only re-render changed regions
//! - **Actor model**: Isolated threads for input, rendering, and agent logic
//! - **Optimistic append**: Fast path for streaming text that bypasses diffing
//!
//! ## Example
//!
//! ```rust,ignore
//! use flywheel::{Buffer, Cell, Rect};
//!
//! // Create a buffer for a 80x24 terminal
//! let mut buffer = Buffer::new(80, 24);
//!
//! // Write a cell
//! buffer.set(0, 0, Cell::new('H'));
//! ```

#![warn(missing_docs)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::must_use_candidate)]

pub mod buffer;
pub mod layout;
pub mod terminal;
pub mod actor;
pub mod widget;

// Re-exports for convenience
pub use buffer::{Buffer, Cell, CellFlags, Modifiers, Rgb};
pub use layout::{Layout, Rect, Region, RegionId};
pub use actor::{Engine, EngineConfig, InputEvent, KeyCode, KeyModifiers, RenderCommand, AgentEvent};
pub use widget::{StreamWidget, StreamConfig, AppendResult, ScrollBuffer};
