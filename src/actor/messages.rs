//! Message types for actor communication.
//!
//! These enums define the protocol between actors in the system.

use std::time::Instant;

/// Key codes for keyboard input.
///
/// This is a simplified subset of crossterm's KeyCode, designed
/// for the needs of agentic CLIs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyCode {
    /// A printable character.
    Char(char),
    /// Function key (F1-F12).
    F(u8),
    /// Backspace key.
    Backspace,
    /// Enter/Return key.
    Enter,
    /// Left arrow.
    Left,
    /// Right arrow.
    Right,
    /// Up arrow.
    Up,
    /// Down arrow.
    Down,
    /// Home key.
    Home,
    /// End key.
    End,
    /// Page Up.
    PageUp,
    /// Page Down.
    PageDown,
    /// Tab key.
    Tab,
    /// Backtab (Shift+Tab).
    BackTab,
    /// Delete key.
    Delete,
    /// Insert key.
    Insert,
    /// Escape key.
    Esc,
    /// Null (Ctrl+Space on some terminals).
    Null,
}

/// Key modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct KeyModifiers {
    /// Shift key held.
    pub shift: bool,
    /// Control key held.
    pub control: bool,
    /// Alt/Option key held.
    pub alt: bool,
    /// Super/Command/Windows key held.
    pub super_key: bool,
}

impl KeyModifiers {
    /// No modifiers.
    pub const NONE: Self = Self {
        shift: false,
        control: false,
        alt: false,
        super_key: false,
    };

    /// Check if any modifier is active.
    pub fn any(&self) -> bool {
        self.shift || self.control || self.alt || self.super_key
    }
}

/// Mouse button.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    /// Left mouse button.
    Left,
    /// Right mouse button.
    Right,
    /// Middle mouse button.
    Middle,
}

/// Mouse event details.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MouseEvent {
    /// X coordinate (column).
    pub x: u16,
    /// Y coordinate (row).
    pub y: u16,
    /// Mouse button involved (if any).
    pub button: Option<MouseButton>,
    /// Key modifiers held during mouse event.
    pub modifiers: KeyModifiers,
}

/// Events from the input thread.
///
/// These are sent from the input actor to the main loop.
#[derive(Debug, Clone)]
pub enum InputEvent {
    /// A key was pressed.
    Key {
        /// The key code.
        code: KeyCode,
        /// Modifiers held during keypress.
        modifiers: KeyModifiers,
    },

    /// Mouse button pressed.
    MouseDown(MouseEvent),

    /// Mouse button released.
    MouseUp(MouseEvent),

    /// Mouse moved (only if tracking enabled).
    MouseMove(MouseEvent),

    /// Mouse scroll.
    MouseScroll {
        /// X coordinate.
        x: u16,
        /// Y coordinate.
        y: u16,
        /// Scroll delta (positive = up, negative = down).
        delta: i16,
    },

    /// Terminal was resized.
    Resize {
        /// New width in columns.
        width: u16,
        /// New height in rows.
        height: u16,
    },

    /// Focus gained.
    FocusGained,

    /// Focus lost.
    FocusLost,

    /// Paste event (bracketed paste).
    Paste(String),

    /// Input thread encountered an error.
    Error(String),

    /// Input thread is shutting down.
    Shutdown,
}

/// Commands sent to the render thread.
#[derive(Debug)]
pub enum RenderCommand {
    /// Request a full redraw.
    FullRedraw,

    /// Request a diff-based update.
    Update,

    /// Resize the buffers.
    Resize {
        /// New width.
        width: u16,
        /// New height.
        height: u16,
    },

    /// Set the cursor position and visibility.
    SetCursor {
        /// X position (None = hide cursor).
        x: Option<u16>,
        /// Y position.
        y: u16,
    },

    /// Shutdown the render thread.
    Shutdown,
}

/// Events from agent/network threads.
///
/// These represent async data arriving from external sources.
#[derive(Debug, Clone)]
pub enum AgentEvent {
    /// Token(s) received from agent stream.
    Tokens {
        /// The text content.
        content: String,
        /// Source identifier (for multi-agent scenarios).
        source_id: u32,
        /// Whether this is the final chunk.
        is_final: bool,
    },

    /// Agent started a new response.
    ResponseStart {
        /// Source identifier.
        source_id: u32,
    },

    /// Agent finished responding.
    ResponseEnd {
        /// Source identifier.
        source_id: u32,
    },

    /// Agent encountered an error.
    Error {
        /// Error message.
        message: String,
        /// Source identifier.
        source_id: u32,
    },

    /// Connection status changed.
    ConnectionStatus {
        /// Whether connected.
        connected: bool,
        /// Source identifier.
        source_id: u32,
    },
}

/// Frame timing information.
#[derive(Debug, Clone)]
pub struct FrameInfo {
    /// Frame number since engine start.
    pub frame_number: u64,
    /// Time when this frame started.
    pub frame_start: Instant,
    /// Duration of the previous frame's render.
    pub last_render_time: std::time::Duration,
    /// Current FPS (smoothed).
    pub fps: f32,
}

impl Default for FrameInfo {
    fn default() -> Self {
        Self {
            frame_number: 0,
            frame_start: Instant::now(),
            last_render_time: std::time::Duration::ZERO,
            fps: 0.0,
        }
    }
}
