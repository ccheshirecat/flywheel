/**
 * @file flywheel.h
 * @brief Flywheel - Zero-flicker terminal compositor for Agentic CLIs
 * 
 * This header provides the C API for Flywheel, enabling high-frequency
 * token streaming (100+ tokens/s) without flickering.
 * 
 * @version 0.1.0
 * @date 2026-01-29
 */

#ifndef FLYWHEEL_H
#define FLYWHEEL_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Version information */
#define FLYWHEEL_VERSION "0.1.0"
#define FLYWHEEL_VERSION_MAJOR 0
#define FLYWHEEL_VERSION_MINOR 1
#define FLYWHEEL_VERSION_PATCH 0

/* ============================================================================
 * Opaque Handle Types
 * ============================================================================ */

/** Opaque handle to a Flywheel engine. */
typedef struct FlywheelEngine FlywheelEngine;

/** Opaque handle to a stream widget. */
typedef struct FlywheelStream FlywheelStream;

/* ============================================================================
 * Enums and Constants
 * ============================================================================ */

/** Result codes for FFI functions. */
typedef enum FlywheelResult {
    FLYWHEEL_RESULT_OK = 0,
    FLYWHEEL_RESULT_NULL_POINTER = 1,
    FLYWHEEL_RESULT_INVALID_UTF8 = 2,
    FLYWHEEL_RESULT_IO_ERROR = 3,
    FLYWHEEL_RESULT_OUT_OF_BOUNDS = 4,
    FLYWHEEL_RESULT_NOT_RUNNING = 5,
} FlywheelResult;

/** Input event type from polling. */
typedef enum FlywheelEventType {
    FLYWHEEL_EVENT_NONE = 0,
    FLYWHEEL_EVENT_KEY = 1,
    FLYWHEEL_EVENT_RESIZE = 2,
    FLYWHEEL_EVENT_ERROR = 3,
    FLYWHEEL_EVENT_SHUTDOWN = 4,
} FlywheelEventType;

/* Key code constants */
#define FLYWHEEL_KEY_NONE       0
#define FLYWHEEL_KEY_ENTER      1
#define FLYWHEEL_KEY_ESCAPE     2
#define FLYWHEEL_KEY_BACKSPACE  3
#define FLYWHEEL_KEY_TAB        4
#define FLYWHEEL_KEY_LEFT       5
#define FLYWHEEL_KEY_RIGHT      6
#define FLYWHEEL_KEY_UP         7
#define FLYWHEEL_KEY_DOWN       8
#define FLYWHEEL_KEY_HOME       9
#define FLYWHEEL_KEY_END        10
#define FLYWHEEL_KEY_PAGE_UP    11
#define FLYWHEEL_KEY_PAGE_DOWN  12
#define FLYWHEEL_KEY_DELETE     13

/* Modifier flags */
#define FLYWHEEL_MOD_SHIFT  1
#define FLYWHEEL_MOD_CTRL   2
#define FLYWHEEL_MOD_ALT    4
#define FLYWHEEL_MOD_SUPER  8

/* ============================================================================
 * Event Structures
 * ============================================================================ */

/** Key event data. */
typedef struct FlywheelKeyEvent {
    uint32_t char_code;     /**< Character code (for printable keys), or 0. */
    int key_code;           /**< Special key code (FLYWHEEL_KEY_*). */
    unsigned int modifiers; /**< Modifier flags (FLYWHEEL_MOD_*). */
} FlywheelKeyEvent;

/** Resize event data. */
typedef struct FlywheelResizeEvent {
    uint16_t width;  /**< New terminal width. */
    uint16_t height; /**< New terminal height. */
} FlywheelResizeEvent;

/** Polled event structure. */
typedef struct FlywheelEvent {
    FlywheelEventType event_type; /**< Type of event. */
    FlywheelKeyEvent key;         /**< Key event data (if event_type == KEY). */
    FlywheelResizeEvent resize;   /**< Resize event data (if event_type == RESIZE). */
} FlywheelEvent;

/* ============================================================================
 * Engine Functions
 * ============================================================================ */

/**
 * Create a new Flywheel engine with default configuration.
 * 
 * The engine initializes the terminal in raw mode with alternate screen.
 * 
 * @return Handle to the engine, or NULL on failure.
 */
FlywheelEngine* flywheel_engine_new(void);

/**
 * Destroy a Flywheel engine and restore terminal state.
 * 
 * @param engine Engine handle (NULL is a no-op).
 */
void flywheel_engine_destroy(FlywheelEngine* engine);

/**
 * Get the terminal width in columns.
 * 
 * @param engine Engine handle.
 * @return Terminal width, or 0 if engine is NULL.
 */
uint16_t flywheel_engine_width(const FlywheelEngine* engine);

/**
 * Get the terminal height in rows.
 * 
 * @param engine Engine handle.
 * @return Terminal height, or 0 if engine is NULL.
 */
uint16_t flywheel_engine_height(const FlywheelEngine* engine);

/**
 * Check if the engine is still running.
 * 
 * @param engine Engine handle.
 * @return true if running, false otherwise.
 */
bool flywheel_engine_is_running(const FlywheelEngine* engine);

/**
 * Stop the engine.
 * 
 * @param engine Engine handle.
 */
void flywheel_engine_stop(FlywheelEngine* engine);

/**
 * Poll for the next input event (non-blocking).
 * 
 * @param engine Engine handle.
 * @param event_out Pointer to event structure to populate.
 * @return Event type.
 */
FlywheelEventType flywheel_engine_poll_event(const FlywheelEngine* engine, FlywheelEvent* event_out);

/**
 * Handle a terminal resize event.
 * 
 * @param engine Engine handle.
 * @param width New width.
 * @param height New height.
 */
void flywheel_engine_handle_resize(FlywheelEngine* engine, uint16_t width, uint16_t height);

/**
 * Request a full screen redraw.
 * 
 * @param engine Engine handle.
 */
void flywheel_engine_request_redraw(const FlywheelEngine* engine);

/**
 * Request a diff-based screen update.
 * 
 * @param engine Engine handle.
 */
void flywheel_engine_request_update(const FlywheelEngine* engine);

/**
 * Begin a new frame. Call at the start of your render loop.
 * 
 * @param engine Engine handle.
 */
void flywheel_engine_begin_frame(FlywheelEngine* engine);

/**
 * End a frame and request update. Handles frame rate limiting.
 * 
 * @param engine Engine handle.
 */
void flywheel_engine_end_frame(FlywheelEngine* engine);

/**
 * Set a single cell at the given position.
 * 
 * @param engine Engine handle.
 * @param x Column (0-indexed).
 * @param y Row (0-indexed).
 * @param c ASCII character.
 * @param fg Foreground color (0xRRGGBB).
 * @param bg Background color (0xRRGGBB).
 */
void flywheel_engine_set_cell(FlywheelEngine* engine, uint16_t x, uint16_t y, 
                               char c, uint32_t fg, uint32_t bg);

/**
 * Draw text at the given position.
 * 
 * @param engine Engine handle.
 * @param x Starting column (0-indexed).
 * @param y Row (0-indexed).
 * @param text UTF-8 null-terminated string.
 * @param fg Foreground color (0xRRGGBB).
 * @param bg Background color (0xRRGGBB).
 * @return Number of columns used.
 */
uint16_t flywheel_engine_draw_text(FlywheelEngine* engine, uint16_t x, uint16_t y,
                                    const char* text, uint32_t fg, uint32_t bg);

/**
 * Clear the entire buffer to default (black background, empty cells).
 * 
 * @param engine Engine handle.
 */
void flywheel_engine_clear(FlywheelEngine* engine);

/**
 * Fill a rectangle with a character.
 * 
 * @param engine Engine handle.
 * @param x Starting column.
 * @param y Starting row.
 * @param width Rectangle width.
 * @param height Rectangle height.
 * @param c Fill character.
 * @param fg Foreground color.
 * @param bg Background color.
 */
void flywheel_engine_fill_rect(FlywheelEngine* engine, uint16_t x, uint16_t y,
                                uint16_t width, uint16_t height,
                                char c, uint32_t fg, uint32_t bg);

/* ============================================================================
 * Stream Widget Functions
 * ============================================================================ */

/**
 * Create a new stream widget for high-frequency text streaming.
 * 
 * @param x Widget X position.
 * @param y Widget Y position.
 * @param width Widget width.
 * @param height Widget height.
 * @return Handle to the stream widget.
 */
FlywheelStream* flywheel_stream_new(uint16_t x, uint16_t y, uint16_t width, uint16_t height);

/**
 * Destroy a stream widget.
 * 
 * @param stream Stream widget handle (NULL is a no-op).
 */
void flywheel_stream_destroy(FlywheelStream* stream);

/**
 * Append text to the stream widget.
 * 
 * Uses fast path when possible (no newlines, fits on line).
 * 
 * @param stream Stream widget handle.
 * @param text UTF-8 null-terminated string.
 * @return 1 if fast path was used, 0 if slow path, -1 on error.
 */
int flywheel_stream_append(FlywheelStream* stream, const char* text);

/**
 * Render the stream widget to the engine's buffer.
 * 
 * @param stream Stream widget handle.
 * @param engine Engine handle.
 */
void flywheel_stream_render(FlywheelStream* stream, FlywheelEngine* engine);

/**
 * Clear all content in the stream widget.
 * 
 * @param stream Stream widget handle.
 */
void flywheel_stream_clear(FlywheelStream* stream);

/**
 * Set the foreground color for subsequent text.
 * 
 * @param stream Stream widget handle.
 * @param color Color (0xRRGGBB).
 */
void flywheel_stream_set_fg(FlywheelStream* stream, uint32_t color);

/**
 * Set the background color for subsequent text.
 * 
 * @param stream Stream widget handle.
 * @param color Color (0xRRGGBB).
 */
void flywheel_stream_set_bg(FlywheelStream* stream, uint32_t color);

/**
 * Scroll the stream widget up by the given number of lines.
 * 
 * @param stream Stream widget handle.
 * @param lines Number of lines to scroll.
 */
void flywheel_stream_scroll_up(FlywheelStream* stream, size_t lines);

/**
 * Scroll the stream widget down by the given number of lines.
 * 
 * @param stream Stream widget handle.
 * @param lines Number of lines to scroll.
 */
void flywheel_stream_scroll_down(FlywheelStream* stream, size_t lines);

/* ============================================================================
 * Utility Functions
 * ============================================================================ */

/**
 * Create an RGB color value from components.
 * 
 * @param r Red component (0-255).
 * @param g Green component (0-255).
 * @param b Blue component (0-255).
 * @return 24-bit color value (0xRRGGBB).
 */
uint32_t flywheel_rgb(uint8_t r, uint8_t g, uint8_t b);

/**
 * Get the Flywheel version string.
 * 
 * @return Static version string (do not free).
 */
const char* flywheel_version(void);

#ifdef __cplusplus
}
#endif

#endif /* FLYWHEEL_H */
