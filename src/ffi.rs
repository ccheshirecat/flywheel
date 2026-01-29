//! C Foreign Function Interface (FFI) for Flywheel.
//!
//! This module provides a C-compatible API for using Flywheel from
//! other programming languages. All functions are `extern "C"` with
//! stable ABI.
//!
//! # Safety
//!
//! All functions that accept pointers require valid, non-null pointers.
//! The caller is responsible for proper memory management of handles.
//!
//! # Example (C)
//!
//! ```c
//! #include "flywheel.h"
//!
//! int main() {
//!     FlywheelEngine* engine = flywheel_engine_new();
//!     if (!engine) return 1;
//!
//!     flywheel_engine_draw_text(engine, 0, 0, "Hello from C!", 0xFFFFFF, 0x000000);
//!     flywheel_engine_request_redraw(engine);
//!
//!     // Main loop...
//!
//!     flywheel_engine_destroy(engine);
//!     return 0;
//! }
//! ```

// FFI modules intentionally use unsafe and no_mangle
#![allow(unsafe_op_in_unsafe_fn)]
#![allow(clippy::missing_safety_doc)]
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use crate::actor::{Engine, InputEvent, KeyCode};
use crate::buffer::{Cell, Rgb};
use crate::layout::Rect;
use crate::widget::{AppendResult, StreamWidget};
use std::ffi::CStr;
use std::os::raw::{c_char, c_int, c_uint};
use std::ptr;

// =============================================================================
// Opaque Handle Types
// =============================================================================

/// Opaque handle to a Flywheel engine.
pub struct FlywheelEngine(Engine);

/// Opaque handle to a stream widget.
pub struct FlywheelStream(StreamWidget);

// =============================================================================
// Result and Error Codes
// =============================================================================

/// Result codes for FFI functions.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlywheelResult {
    /// Operation succeeded.
    Ok = 0,
    /// Null pointer passed.
    NullPointer = 1,
    /// Invalid UTF-8 string.
    InvalidUtf8 = 2,
    /// I/O error.
    IoError = 3,
    /// Out of bounds.
    OutOfBounds = 4,
    /// Engine not running.
    NotRunning = 5,
}

/// Input event type from polling.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlywheelEventType {
    /// No event available.
    None = 0,
    /// Key press event.
    Key = 1,
    /// Terminal resize event.
    Resize = 2,
    /// Error event.
    Error = 3,
    /// Shutdown event.
    Shutdown = 4,
}

/// Key event data.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FlywheelKeyEvent {
    /// The character (for printable keys), or 0.
    pub char_code: u32,
    /// Special key code (see FLYWHEEL_KEY_* constants).
    pub key_code: c_int,
    /// Modifier flags.
    pub modifiers: c_uint,
}

/// Resize event data.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FlywheelResizeEvent {
    /// New width.
    pub width: u16,
    /// New height.
    pub height: u16,
}

/// Polled event structure.
#[repr(C)]
pub struct FlywheelEvent {
    /// Event type.
    pub event_type: FlywheelEventType,
    /// Key event data (valid if event_type == Key).
    pub key: FlywheelKeyEvent,
    /// Resize event data (valid if event_type == Resize).
    pub resize: FlywheelResizeEvent,
}

// Key code constants
/// No special key.
pub const FLYWHEEL_KEY_NONE: c_int = 0;
/// Enter key.
pub const FLYWHEEL_KEY_ENTER: c_int = 1;
/// Escape key.
pub const FLYWHEEL_KEY_ESCAPE: c_int = 2;
/// Backspace key.
pub const FLYWHEEL_KEY_BACKSPACE: c_int = 3;
/// Tab key.
pub const FLYWHEEL_KEY_TAB: c_int = 4;
/// Left arrow.
pub const FLYWHEEL_KEY_LEFT: c_int = 5;
/// Right arrow.
pub const FLYWHEEL_KEY_RIGHT: c_int = 6;
/// Up arrow.
pub const FLYWHEEL_KEY_UP: c_int = 7;
/// Down arrow.
pub const FLYWHEEL_KEY_DOWN: c_int = 8;
/// Home key.
pub const FLYWHEEL_KEY_HOME: c_int = 9;
/// End key.
pub const FLYWHEEL_KEY_END: c_int = 10;
/// Page Up.
pub const FLYWHEEL_KEY_PAGE_UP: c_int = 11;
/// Page Down.
pub const FLYWHEEL_KEY_PAGE_DOWN: c_int = 12;
/// Delete key.
pub const FLYWHEEL_KEY_DELETE: c_int = 13;

// Modifier flags
/// Shift modifier.
pub const FLYWHEEL_MOD_SHIFT: c_uint = 1;
/// Control modifier.
pub const FLYWHEEL_MOD_CTRL: c_uint = 2;
/// Alt modifier.
pub const FLYWHEEL_MOD_ALT: c_uint = 4;
/// Super/Command modifier.
pub const FLYWHEEL_MOD_SUPER: c_uint = 8;

// =============================================================================
// Engine Functions
// =============================================================================

/// Create a new Flywheel engine with default configuration.
///
/// Returns NULL on failure.
#[unsafe(no_mangle)]
pub extern "C" fn flywheel_engine_new() -> *mut FlywheelEngine {
    match Engine::new() {
        Ok(engine) => Box::into_raw(Box::new(FlywheelEngine(engine))),
        Err(_) => ptr::null_mut(),
    }
}

/// Destroy a Flywheel engine.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn flywheel_engine_destroy(engine: *mut FlywheelEngine) {
    if !engine.is_null() {
        drop(Box::from_raw(engine));
    }
}

/// Get the terminal width.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn flywheel_engine_width(engine: *const FlywheelEngine) -> u16 {
    if engine.is_null() {
        return 0;
    }
    (*engine).0.width()
}

/// Get the terminal height.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn flywheel_engine_height(engine: *const FlywheelEngine) -> u16 {
    if engine.is_null() {
        return 0;
    }
    (*engine).0.height()
}

/// Check if the engine is still running.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn flywheel_engine_is_running(engine: *const FlywheelEngine) -> bool {
    if engine.is_null() {
        return false;
    }
    (*engine).0.is_running()
}

/// Stop the engine.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn flywheel_engine_stop(engine: *mut FlywheelEngine) {
    if !engine.is_null() {
        (*engine).0.stop();
    }
}

/// Poll for the next input event.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn flywheel_engine_poll_event(
    engine: *const FlywheelEngine,
    event_out: *mut FlywheelEvent,
) -> FlywheelEventType {
    if engine.is_null() || event_out.is_null() {
        return FlywheelEventType::None;
    }

    match (*engine).0.poll_input() {
        Some(InputEvent::Key { code, modifiers }) => {
            let (char_code, key_code) = convert_key_code(&code);
            let mods = convert_modifiers(&modifiers);

            (*event_out).event_type = FlywheelEventType::Key;
            (*event_out).key = FlywheelKeyEvent {
                char_code,
                key_code,
                modifiers: mods,
            };
            FlywheelEventType::Key
        }
        Some(InputEvent::Resize { width, height }) => {
            (*event_out).event_type = FlywheelEventType::Resize;
            (*event_out).resize = FlywheelResizeEvent { width, height };
            FlywheelEventType::Resize
        }
        Some(InputEvent::Shutdown) => {
            (*event_out).event_type = FlywheelEventType::Shutdown;
            FlywheelEventType::Shutdown
        }
        Some(InputEvent::Error(_)) => {
            (*event_out).event_type = FlywheelEventType::Error;
            FlywheelEventType::Error
        }
        _ => {
            (*event_out).event_type = FlywheelEventType::None;
            FlywheelEventType::None
        }
    }
}

/// Handle a resize event.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn flywheel_engine_handle_resize(
    engine: *mut FlywheelEngine,
    width: u16,
    height: u16,
) {
    if !engine.is_null() {
        (*engine).0.handle_resize(width, height);
    }
}

/// Request a full redraw.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn flywheel_engine_request_redraw(engine: *const FlywheelEngine) {
    if !engine.is_null() {
        (*engine).0.request_redraw();
    }
}

/// Request a diff-based update.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn flywheel_engine_request_update(engine: *const FlywheelEngine) {
    if !engine.is_null() {
        (*engine).0.request_update();
    }
}

/// Begin a new frame.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn flywheel_engine_begin_frame(engine: *mut FlywheelEngine) {
    if !engine.is_null() {
        (*engine).0.begin_frame();
    }
}

/// End a frame and request update.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn flywheel_engine_end_frame(engine: *mut FlywheelEngine) {
    if !engine.is_null() {
        (*engine).0.end_frame();
    }
}

/// Set a cell at the given position.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn flywheel_engine_set_cell(
    engine: *mut FlywheelEngine,
    x: u16,
    y: u16,
    c: c_char,
    fg: u32,
    bg: u32,
) {
    if engine.is_null() {
        return;
    }
    let cell = Cell::new(c as u8 as char)
        .with_fg(Rgb::from_u32(fg))
        .with_bg(Rgb::from_u32(bg));
    (*engine).0.set_cell(x, y, cell);
}

/// Draw text at the given position.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn flywheel_engine_draw_text(
    engine: *mut FlywheelEngine,
    x: u16,
    y: u16,
    text: *const c_char,
    fg: u32,
    bg: u32,
) -> u16 {
    if engine.is_null() || text.is_null() {
        return 0;
    }

    let text_str = match CStr::from_ptr(text).to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };

    (*engine)
        .0
        .draw_text(x, y, text_str, Rgb::from_u32(fg), Rgb::from_u32(bg))
}

/// Clear the entire buffer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn flywheel_engine_clear(engine: *mut FlywheelEngine) {
    if !engine.is_null() {
        (*engine).0.clear();
    }
}

/// Fill a rectangle with a character.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn flywheel_engine_fill_rect(
    engine: *mut FlywheelEngine,
    x: u16,
    y: u16,
    width: u16,
    height: u16,
    c: c_char,
    fg: u32,
    bg: u32,
) {
    if engine.is_null() {
        return;
    }
    let cell = Cell::new(c as u8 as char)
        .with_fg(Rgb::from_u32(fg))
        .with_bg(Rgb::from_u32(bg));
    (*engine).0.fill_rect(Rect::new(x, y, width, height), cell);
}

// =============================================================================
// Stream Widget Functions
// =============================================================================

/// Create a new stream widget.
#[unsafe(no_mangle)]
pub extern "C" fn flywheel_stream_new(x: u16, y: u16, width: u16, height: u16) -> *mut FlywheelStream {
    let widget = StreamWidget::new(Rect::new(x, y, width, height));
    Box::into_raw(Box::new(FlywheelStream(widget)))
}

/// Destroy a stream widget.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn flywheel_stream_destroy(stream: *mut FlywheelStream) {
    if !stream.is_null() {
        drop(Box::from_raw(stream));
    }
}

/// Append text to the stream widget.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn flywheel_stream_append(
    stream: *mut FlywheelStream,
    text: *const c_char,
) -> c_int {
    if stream.is_null() || text.is_null() {
        return -1;
    }

    let text_str = match CStr::from_ptr(text).to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    match (*stream).0.append(text_str) {
        AppendResult::FastPath { .. } => 1,
        AppendResult::SlowPath { .. } => 0,
        AppendResult::Empty => 0,
    }
}

/// Render the stream widget to the engine's buffer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn flywheel_stream_render(
    stream: *mut FlywheelStream,
    engine: *mut FlywheelEngine,
) {
    if stream.is_null() || engine.is_null() {
        return;
    }
    (*stream).0.render((*engine).0.buffer_mut());
}

/// Clear the stream widget content.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn flywheel_stream_clear(stream: *mut FlywheelStream) {
    if !stream.is_null() {
        (*stream).0.clear();
    }
}

/// Set the foreground color for subsequent text.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn flywheel_stream_set_fg(stream: *mut FlywheelStream, color: u32) {
    if !stream.is_null() {
        (*stream).0.set_fg(Rgb::from_u32(color));
    }
}

/// Set the background color for subsequent text.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn flywheel_stream_set_bg(stream: *mut FlywheelStream, color: u32) {
    if !stream.is_null() {
        (*stream).0.set_bg(Rgb::from_u32(color));
    }
}

/// Scroll the stream widget up.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn flywheel_stream_scroll_up(stream: *mut FlywheelStream, lines: usize) {
    if !stream.is_null() {
        (*stream).0.scroll_up(lines);
    }
}

/// Scroll the stream widget down.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn flywheel_stream_scroll_down(stream: *mut FlywheelStream, lines: usize) {
    if !stream.is_null() {
        (*stream).0.scroll_down(lines);
    }
}

// =============================================================================
// Color Utilities
// =============================================================================

/// Create an RGB color from components.
#[unsafe(no_mangle)]
pub extern "C" fn flywheel_rgb(r: u8, g: u8, b: u8) -> u32 {
    ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}

// =============================================================================
// Version Information
// =============================================================================

/// Get the Flywheel version string.
#[unsafe(no_mangle)]
pub extern "C" fn flywheel_version() -> *const c_char {
    static VERSION: &[u8] = b"0.1.0\0";
    VERSION.as_ptr().cast::<c_char>()
}

// =============================================================================
// Helper Functions
// =============================================================================

fn convert_key_code(code: &KeyCode) -> (u32, c_int) {
    match code {
        KeyCode::Char(c) => (*c as u32, FLYWHEEL_KEY_NONE),
        KeyCode::Enter => (0, FLYWHEEL_KEY_ENTER),
        KeyCode::Esc => (0, FLYWHEEL_KEY_ESCAPE),
        KeyCode::Backspace => (0, FLYWHEEL_KEY_BACKSPACE),
        KeyCode::Tab => (0, FLYWHEEL_KEY_TAB),
        KeyCode::Left => (0, FLYWHEEL_KEY_LEFT),
        KeyCode::Right => (0, FLYWHEEL_KEY_RIGHT),
        KeyCode::Up => (0, FLYWHEEL_KEY_UP),
        KeyCode::Down => (0, FLYWHEEL_KEY_DOWN),
        KeyCode::Home => (0, FLYWHEEL_KEY_HOME),
        KeyCode::End => (0, FLYWHEEL_KEY_END),
        KeyCode::PageUp => (0, FLYWHEEL_KEY_PAGE_UP),
        KeyCode::PageDown => (0, FLYWHEEL_KEY_PAGE_DOWN),
        KeyCode::Delete => (0, FLYWHEEL_KEY_DELETE),
        _ => (0, FLYWHEEL_KEY_NONE),
    }
}

fn convert_modifiers(mods: &crate::actor::KeyModifiers) -> c_uint {
    let mut result = 0;
    if mods.shift {
        result |= FLYWHEEL_MOD_SHIFT;
    }
    if mods.control {
        result |= FLYWHEEL_MOD_CTRL;
    }
    if mods.alt {
        result |= FLYWHEEL_MOD_ALT;
    }
    if mods.super_key {
        result |= FLYWHEEL_MOD_SUPER;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flywheel_rgb() {
        assert_eq!(flywheel_rgb(255, 128, 64), 0xFF8040);
        assert_eq!(flywheel_rgb(0, 0, 0), 0x000000);
        assert_eq!(flywheel_rgb(255, 255, 255), 0xFFFFFF);
    }

    #[test]
    fn test_flywheel_version() {
        unsafe {
            let version = flywheel_version();
            let version_str = CStr::from_ptr(version).to_str().unwrap();
            assert_eq!(version_str, "0.1.0");
        }
    }
}
