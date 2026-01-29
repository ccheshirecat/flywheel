//! Region and Layout: Pre-computed static layout regions.

use super::rect::Rect;

/// Unique identifier for a layout region.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct RegionId(pub u16);

impl RegionId {
    /// Create a new region ID.
    pub const fn new(id: u16) -> Self {
        Self(id)
    }
}

/// A layout region with position and dirty tracking.
#[derive(Clone, Debug)]
pub struct Region {
    /// Unique identifier.
    pub id: RegionId,
    /// Position and size.
    pub rect: Rect,
    /// Z-index for overlays (higher = on top).
    pub z_index: u8,
    /// Dirty generation (incremented when content changes).
    pub dirty_generation: u64,
}

impl Region {
    /// Create a new region.
    pub fn new(id: RegionId, rect: Rect) -> Self {
        Self {
            id,
            rect,
            z_index: 0,
            dirty_generation: 0,
        }
    }

    /// Set the z-index.
    pub fn with_z_index(mut self, z: u8) -> Self {
        self.z_index = z;
        self
    }

    /// Mark the region as dirty.
    pub fn mark_dirty(&mut self) {
        self.dirty_generation += 1;
    }
}

/// Pre-computed layout with static regions.
#[derive(Clone, Debug)]
pub struct Layout {
    /// Flat list of regions (no tree).
    pub regions: Vec<Region>,
    /// Terminal size.
    pub terminal_size: (u16, u16),
    /// Global generation counter.
    generation: u64,
}

impl Layout {
    /// Create a new layout for the given terminal size.
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            regions: Vec::new(),
            terminal_size: (width, height),
            generation: 0,
        }
    }

    /// Add a region to the layout.
    pub fn add_region(&mut self, region: Region) {
        self.regions.push(region);
    }

    /// Get a region by ID.
    pub fn get(&self, id: RegionId) -> Option<&Region> {
        self.regions.iter().find(|r| r.id == id)
    }

    /// Get a mutable region by ID.
    pub fn get_mut(&mut self, id: RegionId) -> Option<&mut Region> {
        self.regions.iter_mut().find(|r| r.id == id)
    }

    /// Get all dirty regions.
    pub fn dirty_regions(&self) -> impl Iterator<Item = &Region> {
        self.regions.iter().filter(|r| r.dirty_generation > 0)
    }

    /// Clear all dirty flags.
    pub fn clear_dirty(&mut self) {
        for region in &mut self.regions {
            region.dirty_generation = 0;
        }
    }

    /// Resize the layout and recompute regions.
    pub fn resize(&mut self, width: u16, height: u16) {
        self.terminal_size = (width, height);
        self.generation += 1;
        // Subclasses should override to recompute region positions
    }
}
