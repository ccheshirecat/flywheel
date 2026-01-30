//! Widget trait and common widget implementations.
//!
//! This module defines the core `Widget` trait that all UI components implement,
//! along with commonly used widgets like `TextInput` and `StatusBar`.

use crate::buffer::Buffer;
use crate::layout::Rect;
use crate::actor::InputEvent;

/// A UI component that can be rendered to a buffer and handle input.
///
/// All widgets implement this trait, allowing them to be composed into
/// complex layouts and handled uniformly by the rendering system.
pub trait Widget {
    /// Get the current bounds of this widget.
    fn bounds(&self) -> Rect;

    /// Set the bounds of this widget.
    ///
    /// Called when the layout changes (e.g., terminal resize).
    fn set_bounds(&mut self, bounds: Rect);

    /// Render this widget to the given buffer.
    ///
    /// The widget should only write to cells within its bounds.
    fn render(&self, buffer: &mut Buffer);

    /// Handle an input event.
    ///
    /// Returns `true` if the event was consumed by this widget,
    /// `false` if it should propagate to other widgets.
    fn handle_input(&mut self, event: &InputEvent) -> bool;

    /// Check if this widget needs to be redrawn.
    fn needs_redraw(&self) -> bool;

    /// Clear the redraw flag after rendering.
    fn clear_redraw(&mut self);
}
