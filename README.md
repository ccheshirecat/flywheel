# Flywheel

**A zero-flicker terminal compositor for Agentic CLIs.**

Flywheel is a high-performance rendering engine designed specifically for streaming LLM outputs in the terminal. It solves the "flickering" problem common in traditional TUI libraries when updating at high frequencies (50+ frames per second).

![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Status](https://img.shields.io/badge/status-stable-green.svg)

## Why Flywheel?

Traditional TUI libraries (like `ratatui` or `cursive`) are optimized for full-screen application layouts that update sporadically. When used for high-speed text streaming (like an LLM response), they often suffer from:

1.  **Flickering**: Clearing and redrawing the screen causes visual artifacts.
2.  **Blocking**: Rendering can block the input thread, making the UI unresponsive.
3.  **Inefficiency**: Re-diffing the entire screen for every new character is wasteful.

Flywheel addresses this with:

-   **Double-buffered Rendering**: Prevents tearing and flickering.
-   **Optimistic Append**: A "fast path" for appending text that bypasses the diffing engine entirely for zero-latency updates.
-   **Actor Model**: Input, rendering, and logic run in separate threads.
-   **Dirty Rectangles**: Only changed regions are re-rendered.

## Features

-   🚀 **High Performance**: Renders 100+ tokens/s seamlessly.
-   🦀 **Rust Core**: Memory-safe, concurrent, and fast.
-   🔌 **C FFI**: Bindings for C, C++, Python (via CFFI), Node.js, etc.
-   🔄 **Input Handling**: Non-blocking keyboard, mouse, and resize events.
-   📜 **Scroll Buffer**: efficient O(1) scrollback storage.

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
flywheel = { git = "https://github.com/yourusername/flywheel.git" }
```

## Usage (Rust)

```rust
use flywheel::{Engine, StreamWidget, Rect, AppendResult};

fn main() -> std::io::Result<()> {
    // Initialize engine (sets up terminal, starts actor threads)
    let mut engine = Engine::new()?;
    
    // Create a streaming widget
    let mut stream = StreamWidget::new(Rect::new(0, 0, 80, 24));
    
    // Append text (automatically handles wrapping and scrolling)
    stream.append("Hello, ");
    stream.append("world!");
    
    // Use the fast path for high-frequency updates
    if let AppendResult::FastPath { .. } = stream.append(" Streaming...") {
        // ... optimized render ...
    }
    
    // Main loop
    while engine.is_running() {
        if let Some(event) = engine.poll_input() {
            // Handle input...
        }
        
        // Render
        engine.begin_frame();
        stream.render(engine.buffer_mut());
        engine.end_frame();
    }
    
    Ok(())
}
```

## Usage (C/C++)

```c
#include "flywheel.h"

int main() {
    FlywheelEngine* engine = flywheel_engine_new();
    FlywheelStream* stream = flywheel_stream_new(0, 0, 80, 24);

    flywheel_stream_append(stream, "Hello from C!");
    flywheel_stream_render(stream, engine);
    
    flywheel_engine_request_update(engine);
    
    // ...
    
    flywheel_stream_destroy(stream);
    flywheel_engine_destroy(engine);
    return 0;
}
```

## Examples

Run the streaming demo to see it in action:

```bash
cargo run --example streaming_demo --release
```

## Architecture

Flywheel uses a 3-stage pipeline:

1.  **Input Actor**: Polls `crossterm` events and sends them to the engine via a channel.
2.  **Engine (Main Thread)**: Updates application state, handles business logic, and manages the "Next" buffer.
3.  **Renderer Actor**: Receives render commands, diffs "Next" vs "Current" buffers, and flushes optimized ANSI codes to stdout.

For high-speed streaming, the `StreamWidget` can bypass the diffing stage ("Fast Path") and emit ANSI codes directly for simple appends, falling back to the diffing engine ("Slow Path") only when wrapping or scrolling occurs.

## License

MIT
