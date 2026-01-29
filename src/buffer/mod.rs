//! Buffer module: Core data structures for the double-buffer rendering system.
//!
//! This module contains:
//! - [`Cell`]: The atomic unit of display, optimized for cache efficiency
//! - [`Buffer`]: A grid of cells representing the terminal screen
//! - [`Rgb`]: True-color representation
//! - [`Modifiers`]: Text style bitflags
//! - [`diff`]: Diffing engine for generating minimal ANSI sequences

mod cell;
#[allow(clippy::module_inception)]
mod buffer;
pub mod diff;

pub use cell::{Cell, CellFlags, Modifiers, Rgb};
pub use buffer::Buffer;
