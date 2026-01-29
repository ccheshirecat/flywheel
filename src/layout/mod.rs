//! Layout module: Pre-computed static regions for efficient rendering.
//!
//! Layouts are computed once at initialization or on terminal resize.
//! There is no tree traversal at render time - just a flat Vec<Region>.

mod rect;
mod region;

pub use rect::Rect;
pub use region::{Layout, Region, RegionId};
