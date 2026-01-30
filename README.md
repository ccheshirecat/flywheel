<p align="center">
  <h1 align="center">Flywheel</h1>
  <p align="center">
    <strong>The Zero-Flicker Terminal Compositor for Agentic CLIs</strong>
  </p>
  <p align="center">
    A high-performance, Rust-native rendering engine purpose-built for streaming LLM outputs at 60+ FPS without tearing, flickering, or input lag.
  </p>
</p>

<p align="center">
  <a href="#quickstart">Quickstart</a> •
  <a href="#features">Features</a> •
  <a href="#architecture">Architecture</a> •
  <a href="#api-reference">API</a> •
  <a href="#examples">Examples</a>
</p>

---

## The Problem

Building an "AI coding assistant" CLI that streams LLM responses directly to the terminal sounds simple—until you try it. Existing TUI frameworks are designed for **static layouts** (menus, dashboards) that update sporadically. When used for high-frequency streaming (50+ tokens/second), they suffer from:

| Issue | Symptom |
|-------|---------|
| **Flickering** | `clear()` + `redraw()` on every character creates strobing artifacts. |
| **Blocking** | Render calls starve the input handler, making `Ctrl+C` unresponsive. |
| **Inefficiency** | Diffing the entire 80x24 grid for 1 new character is O(n²) waste. |
| **State Desync** | Direct `stdout` writes conflict with the framework's internal cursor tracking. |

**Flywheel was designed from the ground up to solve this.**

---

## Features

| Feature | Description |
|---------|-------------|
| 🚀 **Zero-Flicker Rendering** | Double-buffered diffing outputs only the *delta* between frames. No screen clears. |
| ⚡ **Sub-Millisecond Input Latency** | Actor model decouples input polling from rendering. `Ctrl+C` always works. |
| 🎯 **Fast Path Optimization** | For simple character appends, bypass the buffer entirely—emit ANSI codes directly. |
| 📜 **Infinite Scrollback** | `StreamWidget` stores 100k+ lines efficiently with "sticky scroll" UX. |
| 🎨 **True Color (24-bit RGB)** | Full RGB attribute support for syntax highlighting and theming. |
| 🦀 **100% Safe Rust** | No `unsafe` blocks. Memory-safe concurrency with `crossbeam` channels. |
| 🔌 **C FFI** | Stable `extern "C"` interface for Python, Node.js, Go, and C/C++ bindings. |

---

## Quickstart

### Installation

```toml
[dependencies]
flywheel = { git = "https://github.com/ccheshirecat/flywheel.git" }
```

### Minimal Example

```rust
use flywheel::{Engine, StreamWidget, Rect, Rgb};

fn main() -> std::io::Result<()> {
    let mut engine = Engine::new()?;
    let mut stream = StreamWidget::new(Rect::new(0, 0, engine.width(), engine.height()));

    // Simulate LLM streaming
    for token in ["Hello, ", "world! ", "This ", "is ", "Flywheel."] {
        stream.set_fg(Rgb::new(0, 255, 128)); // Green text
        
        // Just push. The engine handles Fast/Slow path automatically.
        stream.push(&engine, token);
        
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    // Event loop
    while engine.is_running() {
        for event in engine.poll_input() {
            match event {
                flywheel::InputEvent::Key { code: flywheel::KeyCode::Esc, .. } => engine.stop(),
                _ => {}
            }
        }
    }

    Ok(())
}
```

### Run the Demo

```bash
cargo run --example streaming_demo --release
```

This showcases:
- 100% GPU-free flicker elimination at 60 FPS
- 3000+ characters/second matrix generation
- Real-time input handling with cursor blinking
- Live CPU/Memory usage display

---

## Architecture

Flywheel implements a **3-Actor Pipeline**:

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│   Input Actor   │────▶│  Main Thread    │────▶│ Renderer Actor  │
│  (crossterm)    │     │  (Your Code)    │     │    (stdout)     │
└─────────────────┘     └─────────────────┘     └─────────────────┘
        │                       │                       │
    Keyboard            Buffer Updates           ANSI Sequences
    Mouse Events        Widget Logic             Diff Output
    Resize              State Management         Cursor Control
```

### Core Axioms

| Axiom | Principle |
|-------|-----------|
| **A: Double Buffering** | `Next` buffer holds pending changes. `Current` buffer holds what's on screen. Diffing produces minimal escape sequences. |
| **B: Append-Optimized** | `StreamWidget::append()` returns `FastPath` or `SlowPath`. Fast path bypasses diffing for O(1) writes. |
| **C: Thread Isolation** | Only `Renderer Actor` touches `stdout`. Zero contention. Zero deadlocks. |
| **D: Event-Driven** | Main loop uses `recv_timeout()` for input events. No polling. No sleeping. Sub-ms latency. |

### Fast Path vs Slow Path

```rust
let result = stream.append("x");

match result {
    AppendResult::FastPath { row, start_col, .. } => {
        // Character appended within viewport, no wrapping.
        // Emit: MoveTo(row, col) + SetColor + PrintChar
        // Cost: ~20 bytes to stdout
    }
    AppendResult::SlowPath => {
        // Wrapping or scrolling required.
        // Full frame rendered via diffing engine.
        // Cost: ~200 bytes to stdout (only changed cells)
    }
}
```

The `append_fast_into()` helper encapsulates this:

```rust
let mut raw_output = Vec::new();
stream.append_fast_into("x", &mut raw_output);
engine.write_raw(raw_output); // Sends RawOutput command to Renderer
```

---

## API Reference

### `Engine`

The central coordinator. Manages terminal lifecycle and actor threads.

```rust
// Initialization
let mut engine = Engine::new()?;                    // Default config
let mut engine = Engine::with_config(config)?;     // Custom FPS, mouse, etc.

// Dimensions
engine.width();   // Terminal columns
engine.height();  // Terminal rows

// Event Loop
engine.is_running();                                // Check if still alive
engine.poll_input();                                // Non-blocking: Vec<InputEvent>
engine.input_receiver().recv_timeout(duration);     // Blocking: for event-driven loops

// Rendering
engine.buffer_mut();        // Get mutable reference to the Next buffer
engine.request_update();    // Send buffer to Renderer (diff-based)
engine.request_redraw();    // Send buffer to Renderer (full redraw)
engine.write_raw(bytes);    // Bypass buffer, write ANSI directly (Fast Path)

// Lifecycle
engine.stop();              // Signal shutdown
```

### `StreamWidget`

A scrolling text viewport optimized for streaming content.

```rust
let mut stream = StreamWidget::new(Rect::new(x, y, width, height));

// Styling
stream.set_fg(Rgb::new(255, 128, 0));  // Orange text
stream.set_bg(Rgb::new(20, 20, 20));   // Dark background
stream.set_bold(true);

// Content (Recommended API)
stream.push(&engine, "Hello");          // Automatic Fast/Slow path handling
stream.newline();
stream.clear();

// Low-level API (for advanced use cases)
stream.append("text");                  // Returns AppendResult, manual handling
stream.append_fast_into("x", &mut buf); // Manual Fast Path with raw output

// Scrolling (Sticky Scroll: auto-scroll only if at bottom)
stream.scroll_up(lines);
stream.scroll_down(lines);

// Rendering
stream.render(&mut buffer);             // Write to Buffer
stream.needs_redraw();                  // Check if dirty
```

### `Buffer`

Low-level grid of cells representing the terminal screen.

```rust
let mut buffer = Buffer::new(80, 24);

buffer.set(x, y, Cell::new('A').with_fg(Rgb::RED));
buffer.get(x, y);                      // Option<&Cell>
buffer.draw_text(x, y, "text", fg, bg);
buffer.fill_rect(x, y, w, h, cell);
buffer.clear();
```

### `InputEvent`

Events received from the terminal.

```rust
match event {
    InputEvent::Key { code, modifiers } => { /* KeyCode::Char, Esc, Enter, etc. */ }
    InputEvent::MouseClick { x, y, button } => { /* Left, Right, Middle */ }
    InputEvent::MouseScroll { x, y, delta } => { /* +1 up, -1 down */ }
    InputEvent::Resize { width, height } => { /* Terminal resized */ }
    InputEvent::Shutdown => { /* SIGTERM or similar */ }
    _ => {}
}
```

---

## V2 Widgets

Flywheel V2 introduces a proper widget system with composable UI components.

### `TextInput`

Single-line text input with cursor, editing, and navigation:

```rust
use flywheel::{TextInput, Widget, Rect};

let mut input = TextInput::new(Rect::new(0, 23, 80, 1));

// Configure
input.set_content("Initial text");
input.set_focused(true);

// Handle input events
if input.handle_input(&event) {
    // Event was consumed by the widget
}

// Render
input.render(buffer);

// Get content
let text = input.content();
```

### `StatusBar`

Three-section status bar (left, center, right):

```rust
use flywheel::{StatusBar, Widget, Rect};

let mut status = StatusBar::new(Rect::new(0, 0, 80, 1));
status.set_all("Flywheel", "v2.0", "60 FPS");

// Or set individually
status.set_left("App Name");
status.set_center("Status");
status.set_right("12:34");

status.render(buffer);
```

### `ProgressBar`

Animated horizontal progress indicator:

```rust
use flywheel::{ProgressBar, Widget, Rect, ProgressStyle};

let mut progress = ProgressBar::new(Rect::new(0, 5, 60, 1));
progress.set_progress(0.5);  // 50%
progress.set_label("Loading");
progress.increment(0.1);     // +10%

progress.render(buffer);
```

### Widget Trait

All widgets implement the `Widget` trait:

```rust
pub trait Widget {
    fn bounds(&self) -> Rect;
    fn set_bounds(&mut self, bounds: Rect);
    fn render(&self, buffer: &mut Buffer);
    fn handle_input(&mut self, event: &InputEvent) -> bool;
    fn needs_redraw(&self) -> bool;
    fn clear_redraw(&mut self);
}
```

---

## Examples

### Event-Driven Loop with TickerActor (Recommended)

Use the V2 `TickerActor` for non-blocking frame pacing:

```rust
use flywheel::{Engine, TickerActor, InputEvent, KeyCode};
use crossbeam_channel::select;
use std::time::Duration;

let engine = Engine::new()?;
let ticker = TickerActor::spawn(Duration::from_micros(16_666)); // 60 FPS

while engine.is_running() {
    select! {
        recv(engine.input_receiver()) -> result => {
            if let Ok(event) = result {
                match event {
                    InputEvent::Key { code: KeyCode::Esc, .. } => engine.stop(),
                    _ => handle_input(event),
                }
            }
        }
        recv(ticker.receiver()) -> _ => {
            // Tick: generate content, update animations
            generate_content(&mut stream);
            stream.render(engine.buffer_mut());
            engine.request_update();
        }
    }
}

ticker.join();
```

### Legacy Event Loop

For simpler applications without the ticker:

```rust
use crossbeam_channel::RecvTimeoutError;
use std::time::Duration;

let target_fps = Duration::from_micros(16_666); // 60 FPS
let mut last_tick = Instant::now();

while engine.is_running() {
    let timeout = target_fps.saturating_sub(last_tick.elapsed());
    
    match engine.input_receiver().recv_timeout(timeout) {
        Ok(event) => {
            // Handle input IMMEDIATELY
            handle_input(event);
            redraw_ui(&mut engine);
            engine.request_update();
        }
        Err(RecvTimeoutError::Timeout) => {
            // Tick: generate content, update animations
            last_tick = Instant::now();
            generate_content(&mut stream);
            stream.render(engine.buffer_mut());
            engine.request_update();
        }
        Err(_) => break,
    }
}
```

### C FFI Usage

```c
#include "flywheel.h"

int main() {
    FlywheelEngine* engine = flywheel_engine_new();
    FlywheelStream* stream = flywheel_stream_new(0, 0, 80, 24);

    flywheel_stream_set_fg(stream, 0, 255, 128);
    flywheel_stream_append(stream, "Hello from C!");
    flywheel_stream_render(stream, flywheel_engine_buffer(engine));
    flywheel_engine_request_update(engine);

    // Event loop...

    flywheel_stream_destroy(stream);
    flywheel_engine_destroy(engine);
    return 0;
}
```

---

## Performance

Benchmarked on Apple Silicon (criterion, release build):

### Cell Operations

| Operation | Time |
|-----------|------|
| Cell equality (same) | 2.09 ns |
| Cell equality (diff grapheme) | 650 ps |
| Cell equality (diff color) | 921 ps |
| Cell from ASCII char | 1.73 ns |
| Cell from CJK char | 2.56 ns |

### Buffer Diffing (200×50 = 10,000 cells)

| Scenario | Time | Notes |
|----------|------|-------|
| Identical buffers | 33.3 µs | No-op diff |
| Single cell change | 33.6 µs | Minimal output |
| Line change (200 cells) | 33.7 µs | Optimized cursor moves |
| Full change (10K cells) | 289 µs | ~2.9M cells/second |
| Full render | 318 µs | No diffing |

### RopeBuffer (Chunked Storage)

| Operation | Time | Notes |
|-----------|------|-------|
| Append single char | 2.69 ns | O(1) amortized |
| Newline | 9.29 ns | Creates new line |
| Append 80 cells | 178 ns | Full line |
| Push complete line | 93 ns | Pre-built line |
| Get line (50K lines) | 537 ps | O(1) chunk lookup |
| Visible lines iterator | 194 ns | 50 lines |
| Push 100K lines | 404 µs | ~247M lines/second |

### Scaling

| Buffer Size | Diff Time (full change) |
|-------------|-------------------------|
| 80×24 (1,920 cells) | 55 µs |
| 120×40 (4,800 cells) | 140 µs |
| 200×50 (10,000 cells) | 291 µs |
| 300×80 (24,000 cells) | 693 µs |

### Run Benchmarks

```bash
cargo bench --bench cell_benchmark
cargo bench --bench diff_benchmark
cargo bench --bench rope_benchmark
cargo bench --bench comparison_benchmark  # Flywheel vs Ratatui
```

### Flywheel vs Ratatui (Head-to-Head)

| Operation | Flywheel | Ratatui | Speedup |
|-----------|----------|---------|---------|
| **Buffer Creation (80×24)** | 546 ns | 2.02 µs | **3.7×** |
| **Buffer Creation (200×50)** | 3.17 µs | 9.96 µs | **3.1×** |
| **Cell Write** | 1.32 ns | 1.53 ns | **1.2×** |
| **Buffer Fill (80×24)** | 796 ns | 3.20 µs | **4.0×** |
| **Buffer Fill (200×50)** | 4.17 µs | 16.1 µs | **3.9×** |
| **Buffer Diff (80×24)** | 8.05 µs | 21.6 µs | **2.7×** |
| **Buffer Diff (200×50)** | 39.2 µs | 109 µs | **2.8×** |
| **Cell Clone/Copy** | 1.77 ns | 2.03 ns | **1.1×** |
| **Text Render (47 chars)** | 91.1 ns | 137 ns | **1.5×** |

---

## Comparison

| Feature | Flywheel | ratatui | crossterm (raw) |
|---------|----------|---------|-----------------|
| Zero-flicker streaming | ✅ | ❌ | ❌ |
| Non-blocking input | ✅ | ❌ | ✅ |
| Fast Path optimization | ✅ | ❌ | N/A |
| Sticky scroll | ✅ | ❌ | N/A |
| Actor-based rendering | ✅ | ❌ | ❌ |
| Widget system | ✅ | ✅ | ❌ |
| RopeBuffer (1M+ lines) | ✅ | ❌ | N/A |
| C FFI | ✅ | ❌ | ❌ |

---

## Roadmap

### V2.0 ✅ Complete
- [x] Buffer synchronization fix (ghost character elimination)
- [x] Async-friendly TickerActor
- [x] RopeBuffer for 1M+ line documents
- [x] Widget system (TextInput, StatusBar, ProgressBar)
- [x] Comprehensive documentation and benchmarks

### Future
- [ ] **V2.1**: Layout containers (VSplit, HSplit, Stack)
- [ ] **V2.2**: Focus management system
- [ ] **V3.0**: WASM target for browser terminals
- [ ] **V3.1**: Plugin system for custom widgets

---

## License

MIT

---

<p align="center">
  <sub>Built with ❤️ for the AI-native CLI era.</sub>
</p>
