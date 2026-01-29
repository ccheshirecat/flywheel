//! Buffer module: Core data structures for the double-buffer rendering system.
//!
//! This module contains:
//! - [`Cell`]: The atomic unit of display, optimized for cache efficiency
//! - [`Buffer`]: A grid of cells representing the terminal screen
//! - [`Rgb`]: True-color representation
//! - [`Modifiers`]: Text style bitflags

mod cell;
mod buffer;

pub use cell::{Cell, CellFlags, Modifiers, Rgb};
pub use buffer::Buffer;
