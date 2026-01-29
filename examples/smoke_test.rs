//! Smoke test: Verify the actor model with non-blocking input.
//!
//! This example demonstrates:
//! - Engine initialization with raw mode and alternate screen
//! - Non-blocking input polling
//! - Frame-based rendering
//! - Graceful shutdown on 'q' or Escape

use flywheel::{Cell, Engine, InputEvent, KeyCode, Rgb};
use std::time::Duration;

fn main() -> std::io::Result<()> {
    println!("Starting Flywheel Smoke Test...");
    println!("Press 'q' or Escape to quit");
    println!("Type any key to see it echoed");
    std::thread::sleep(Duration::from_secs(1));

    // Create the engine
    let mut engine = Engine::new()?;

    // Draw initial UI
    let width = engine.width();
    let height = engine.height();

    // Title bar
    let title = "Flywheel Smoke Test - Press 'q' to quit";
    let bg_title = Rgb::new(40, 80, 120);
    for x in 0..width {
        engine.set_cell(x, 0, Cell::new(' ').with_bg(bg_title));
    }
    engine.draw_text(2, 0, title, Rgb::WHITE, bg_title);

    // Info text
    let info_y = 2;
    engine.draw_text(2, info_y, &format!("Terminal size: {}x{}", width, height), Rgb::new(180, 180, 180), Rgb::DEFAULT_BG);
    engine.draw_text(2, info_y + 1, "Frame: 0", Rgb::new(180, 180, 180), Rgb::DEFAULT_BG);
    engine.draw_text(2, info_y + 2, "Last key: (none)", Rgb::new(180, 180, 180), Rgb::DEFAULT_BG);

    // Instructions
    engine.draw_text(2, info_y + 4, "Type to see characters appear below:", Rgb::new(150, 200, 150), Rgb::DEFAULT_BG);

    // Request initial draw
    engine.request_redraw();

    // Typing area
    let mut typed = String::new();
    let type_y = info_y + 5;

    // Main loop
    while engine.is_running() {
        engine.begin_frame();

        // Process all pending input events
        while let Some(event) = engine.poll_input() {
            match event {
                InputEvent::Key { code, modifiers } => {
                    match code {
                        KeyCode::Char('q') | KeyCode::Esc => {
                            engine.stop();
                        }
                        KeyCode::Char('c') if modifiers.control => {
                            engine.stop();
                        }
                        KeyCode::Char(c) => {
                            typed.push(c);
                            if typed.len() > (width - 4) as usize {
                                typed.clear();
                            }
                            // Update typed text
                            engine.buffer_mut().clear_rect(2, type_y, width - 4, 1);
                            engine.draw_text(2, type_y, &typed, Rgb::new(100, 255, 100), Rgb::DEFAULT_BG);
                        }
                        KeyCode::Backspace => {
                            typed.pop();
                            engine.buffer_mut().clear_rect(2, type_y, width - 4, 1);
                            engine.draw_text(2, type_y, &typed, Rgb::new(100, 255, 100), Rgb::DEFAULT_BG);
                        }
                        KeyCode::Enter => {
                            typed.clear();
                            engine.buffer_mut().clear_rect(2, type_y, width - 4, 1);
                        }
                        _ => {}
                    }

                    // Update last key display
                    let key_str = format!("Last key: {:?} (modifiers: {:?})", code, modifiers);
                    engine.buffer_mut().clear_rect(2, info_y + 2, width - 4, 1);
                    engine.draw_text(2, info_y + 2, &key_str, Rgb::new(255, 200, 100), Rgb::DEFAULT_BG);
                }

                InputEvent::Resize { width: w, height: h } => {
                    engine.handle_resize(w, h);
                    // Update size display
                    let size_str = format!("Terminal size: {}x{}", w, h);
                    engine.buffer_mut().clear_rect(2, info_y, width - 4, 1);
                    engine.draw_text(2, info_y, &size_str, Rgb::new(180, 180, 180), Rgb::DEFAULT_BG);
                }

                InputEvent::Error(e) => {
                    eprintln!("Input error: {}", e);
                }

                InputEvent::Shutdown => {
                    engine.stop();
                }

                _ => {}
            }
        }

        // Update frame counter
        let frame_str = format!("Frame: {}", engine.frame_count());
        engine.buffer_mut().clear_rect(2, info_y + 1, 30, 1);
        engine.draw_text(2, info_y + 1, &frame_str, Rgb::new(180, 180, 180), Rgb::DEFAULT_BG);

        engine.end_frame();
    }

    Ok(())
}
