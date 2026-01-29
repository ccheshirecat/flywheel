//! Streaming Demo: Demonstrates high-frequency token streaming.
//!
//! This example simulates an LLM agent streaming tokens at 100+ tokens/s
//! to show the zero-flicker rendering of the Flywheel engine.
//!
//! Press 'q' or Escape to quit.

use flywheel::{
    AppendResult, Cell, Engine, InputEvent, KeyCode, Rect, Rgb, StreamWidget,
};
use std::io::Write;
use std::time::{Duration, Instant};

/// Sample text to stream (simulating an LLM response).
const SAMPLE_TEXT: &str = r#"I'd be happy to help you understand how Flywheel achieves zero-flicker rendering!

## The Key Architecture

Flywheel uses a **double-buffered rendering** approach with several optimizations:

1. **Current and Next Buffers**: We maintain two buffers - one showing what's currently visible, and one being drawn to. This prevents tearing.

2. **Dirty Rectangle Tracking**: Instead of redrawing the entire screen, we track which regions have changed and only update those.

3. **Optimistic Append**: For streaming text (like this!), we use a fast path that bypasses the diffing engine entirely. When text is appended without line wraps, we emit direct ANSI sequences.

4. **Actor Model**: Input, rendering, and application logic run on separate threads, ensuring that typing never blocks and renders are smooth.

## Performance Targets

- Cell comparison: < 1ns (achieved: 666ps)
- Buffer diff (200×50): < 500µs (achieved: 283µs)
- Single syscall output: All ANSI sequences accumulated and flushed in one write()

## Why This Matters for Agentic CLIs

When an LLM agent streams 100+ tokens per second, traditional TUI frameworks struggle:
- They redraw the entire screen each frame
- Flickering becomes visible at high update rates
- Input becomes blocked during renders

Flywheel was designed specifically for this use case, making agentic CLIs feel polished and responsive.
"#;

fn main() -> std::io::Result<()> {
    println!("Flywheel Streaming Demo");
    println!("=======================");
    println!("This will simulate 100 tokens/s streaming.");
    println!("Press 'q' or Escape to quit.\n");

    std::thread::sleep(Duration::from_secs(2));

    // Create the engine
    let mut engine = Engine::new()?;

    let width = engine.width();
    let height = engine.height();

    // Create stream widget (leave 2 rows for header and 1 for footer)
    let header_height = 2;
    let footer_height = 1;
    let content_height = height.saturating_sub(header_height + footer_height);

    let mut stream = StreamWidget::new(Rect::new(0, header_height, width, content_height));
    stream.set_fg(Rgb::new(200, 200, 200));

    // Draw header
    let header_bg = Rgb::new(40, 80, 120);
    for x in 0..width {
        engine.set_cell(x, 0, Cell::new(' ').with_bg(header_bg));
        engine.set_cell(x, 1, Cell::new(' ').with_bg(Rgb::new(30, 60, 90)));
    }
    engine.draw_text(2, 0, "Flywheel Streaming Demo", Rgb::WHITE, header_bg);
    engine.draw_text(2, 1, "Simulating 100 tokens/s LLM output...", Rgb::new(180, 180, 180), Rgb::new(30, 60, 90));

    // Draw footer
    let footer_y = height - 1;
    let footer_bg = Rgb::new(30, 30, 30);
    for x in 0..width {
        engine.set_cell(x, footer_y, Cell::new(' ').with_bg(footer_bg));
    }
    engine.draw_text(2, footer_y, "Press 'q' to quit | Tokens: 0 | FPS: 0", Rgb::new(150, 150, 150), footer_bg);

    // Request initial draw
    engine.request_redraw();

    // Streaming state
    let text_chars: Vec<char> = SAMPLE_TEXT.chars().collect();
    let mut char_index = 0;
    let mut token_count = 0u64;
    let mut last_token_time = Instant::now();
    let token_interval = Duration::from_millis(10); // ~100 tokens/s

    let start_time = Instant::now();
    let mut frame_count = 0u64;

    // Pre-allocate output buffer for fast path
    let mut fast_output = Vec::with_capacity(256);

    // Main loop
    while engine.is_running() {
        engine.begin_frame();

        // Process input
        while let Some(event) = engine.poll_input() {
            match event {
                InputEvent::Key { code, modifiers } => match code {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        engine.stop();
                    }
                    KeyCode::Char('c') if modifiers.control => {
                        engine.stop();
                    }
                    KeyCode::Char('r') => {
                        // Reset
                        stream.clear();
                        char_index = 0;
                        token_count = 0;
                    }
                    _ => {}
                },
                InputEvent::Resize { width: w, height: h } => {
                    engine.handle_resize(w, h);
                    let new_content_height = h.saturating_sub(header_height + footer_height);
                    stream.set_bounds(Rect::new(0, header_height, w, new_content_height));
                }
                InputEvent::Shutdown => {
                    engine.stop();
                }
                _ => {}
            }
        }

        // Stream tokens at target rate
        let now = Instant::now();
        if char_index < text_chars.len() && now.duration_since(last_token_time) >= token_interval {
            // Get next "token" (here we simulate by sending 1-3 chars at a time)
            let chunk_size = ((char_index * 7) % 3) + 1; // Pseudo-random 1-3
            let end = (char_index + chunk_size).min(text_chars.len());
            let chunk: String = text_chars[char_index..end].iter().collect();

            // Append to stream widget
            let result = stream.append(&chunk);

            // If fast path, write directly to terminal
            if let AppendResult::FastPath { .. } = result {
                fast_output.clear();
                stream.write_fast_path(result, &chunk, &mut fast_output);

                // Write directly (bypassing buffer system for fast path)
                if !fast_output.is_empty() {
                    let mut stdout = std::io::stdout();
                    let _ = stdout.write_all(&fast_output);
                    let _ = stdout.flush();
                }
            }

            char_index = end;
            token_count += 1;
            last_token_time = now;
        }

        // Update footer periodically
        frame_count += 1;
        if frame_count % 10 == 0 {
            let elapsed = start_time.elapsed().as_secs_f32();
            let fps = if elapsed > 0.0 { frame_count as f32 / elapsed } else { 0.0 };

            // Clear and redraw footer
            for x in 0..width {
                engine.set_cell(x, footer_y, Cell::new(' ').with_bg(footer_bg));
            }
            let status = format!(
                "Press 'q' to quit | Tokens: {} | FPS: {:.1} | Chars: {}/{}",
                token_count, fps, char_index, text_chars.len()
            );
            engine.draw_text(2, footer_y, &status, Rgb::new(150, 150, 150), footer_bg);
        }

        // If stream needs redraw, render it to buffer
        if stream.needs_redraw() || frame_count % 30 == 0 {
            stream.render(engine.buffer_mut());
        }

        engine.end_frame();

        // If we've finished, show completion message
        if char_index >= text_chars.len() && frame_count % 60 == 0 {
            engine.draw_text(
                2,
                footer_y,
                &format!("Done! {} tokens streamed. Press 'r' to restart or 'q' to quit.", token_count),
                Rgb::new(100, 255, 100),
                footer_bg,
            );
        }
    }

    Ok(())
}
