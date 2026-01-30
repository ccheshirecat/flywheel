//! Widget Demo: Showcases the new widget system.
//!
//! Demonstrates:
//! - TextInput widget with cursor editing
//! - StatusBar widget with three sections
//! - ProgressBar widget with animation
//! - Widget trait composition

use flywheel::{
    Cell, Engine, InputEvent, KeyCode, Rect, Rgb,
    Widget, TextInput, StatusBar, ProgressBar,
    TickerActor,
};
use std::time::Duration;
use crossbeam_channel::select;

fn main() -> std::io::Result<()> {
    let mut engine = Engine::new()?;
    let width = engine.width();
    let height = engine.height();

    // Create widgets
    let mut status_bar = StatusBar::new(Rect::new(0, 0, width, 1));
    status_bar.set_all("🚀 Flywheel Widgets Demo", "v2.0", "Press ESC to exit");

    let mut text_input = TextInput::new(Rect::new(0, height - 1, width, 1));
    
    // Progress bar in the center area
    let mut progress_bar = ProgressBar::new(Rect::new(2, 3, width - 4, 1));
    progress_bar.set_label("Loading");

    // Ticker for animations
    let ticker = TickerActor::spawn(Duration::from_millis(50));

    // Initial render
    render_all(&mut engine, &status_bar, &text_input, &progress_bar);
    engine.request_update();

    // Main loop
    while engine.is_running() {
        select! {
            recv(engine.input_receiver()) -> result => {
                if let Ok(event) = result {
                    match &event {
                        InputEvent::Key { code, modifiers } => {
                            match code {
                                KeyCode::Esc => engine.stop(),
                                KeyCode::Char('c') if modifiers.control => engine.stop(),
                                KeyCode::Enter => {
                                    // Handle input submission
                                    let content = text_input.content().to_string();
                                    if !content.is_empty() {
                                        status_bar.set_center(format!("You typed: {}", content));
                                        text_input.clear();
                                    }
                                }
                                _ => {
                                    // Pass to text input
                                    text_input.handle_input(&event);
                                }
                            }
                        }
                        InputEvent::Resize { width: w, height: h } => {
                            engine.handle_resize(*w, *h);
                            status_bar.set_bounds(Rect::new(0, 0, *w, 1));
                            text_input.set_bounds(Rect::new(0, h - 1, *w, 1));
                            progress_bar.set_bounds(Rect::new(2, 3, w - 4, 1));
                        }
                        InputEvent::Shutdown => engine.stop(),
                        _ => {}
                    }

                    render_all(&mut engine, &status_bar, &text_input, &progress_bar);
                    engine.request_update();
                }
            }
            
            recv(ticker.receiver()) -> result => {
                if result.is_ok() {
                    // Animate progress bar
                    progress_bar.increment(0.01);
                    if progress_bar.is_complete() {
                        progress_bar.set_progress(0.0);
                        progress_bar.set_label("Loading");
                    }
                    if progress_bar.progress() > 0.5 {
                        progress_bar.set_label("Almost there");
                    }

                    // Tick text input for cursor blink
                    text_input.tick();

                    // Update status bar right section with frame info
                    let frame = result.unwrap().frame;
                    status_bar.set_right(format!("Frame: {}", frame));

                    render_all(&mut engine, &status_bar, &text_input, &progress_bar);
                    engine.request_update();
                }
            }
        }
    }

    ticker.join();
    Ok(())
}

fn render_all(
    engine: &mut Engine,
    status_bar: &StatusBar,
    text_input: &TextInput,
    progress_bar: &ProgressBar,
) {
    let width = engine.width();
    let height = engine.height();

    // Clear center area
    {
        let buffer = engine.buffer_mut();
        for y in 1..height - 1 {
            for x in 0..width {
                buffer.set(x, y, Cell::new(' ').with_bg(Rgb::new(20, 20, 30)));
            }
        }
    }

    // Draw instructions
    let instructions = [
        "Welcome to the Flywheel Widget Demo!",
        "",
        "Features demonstrated:",
        "  • StatusBar - Three-section header (top)",
        "  • ProgressBar - Animated loading bar",
        "  • TextInput - Type something below!",
        "",
        "Keyboard:",
        "  Type     → Input text",
        "  ←/→      → Move cursor",
        "  Enter    → Submit input",
        "  ESC      → Exit",
    ];

    for (i, line) in instructions.iter().enumerate() {
        #[allow(clippy::cast_possible_truncation)]
        let y = 5 + i as u16;
        if y < height - 2 {
            engine.draw_text(4, y, line, Rgb::new(180, 180, 180), Rgb::new(20, 20, 30));
        }
    }

    // Render widgets
    let buffer = engine.buffer_mut();
    status_bar.render(buffer);
    progress_bar.render(buffer);
    text_input.render(buffer);
}
