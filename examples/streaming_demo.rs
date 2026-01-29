//! Matrix Streaming Demo: High-speed infinite generation with input.
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::many_single_char_names)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::missing_const_for_fn)]
//!
//! Demonstrates:
//! - Infinite scrolling with high throughput
//! - Zero-flicker rendering at max FPS
//! - Responsive input handling during heavy load
//! - Per-character color attribute updates

use flywheel::{
    Cell, Engine, InputEvent, KeyCode, Rect, Rgb, StreamWidget,
};
use std::time::{Duration, Instant};
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};
use crossbeam_channel::RecvTimeoutError;

/// Simple LCG for deterministic randomness without dependencies.
struct Rng {
    state: u64,
}

impl Rng {
    const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    const fn next_u32(&mut self) -> u32 {
        self.state = self.state.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
        (self.state >> 32) as u32
    }

    fn next_float(&mut self) -> f32 {
        (self.next_u32() as f32) / (u32::MAX as f32)
    }

    fn next_char(&mut self) -> char {
        // Printable ASCII: 33-126
        let val = (self.next_u32() % 94) + 33;
        val as u8 as char
    }

    fn next_color(&mut self) -> Rgb {
        // High saturation colors
        let hue = self.next_float() * 6.0;
        let x = (1.0 - (hue % 2.0 - 1.0).abs()) * 255.0;
        let c = 255.0;
        
        let (r, g, b) = match hue as i32 {
            0 => (c, x, 0.0),
            1 => (x, c, 0.0),
            2 => (0.0, c, x),
            3 => (0.0, x, c),
            4 => (x, 0.0, c),
            _ => (c, 0.0, x),
        };
        
        Rgb::new(r as u8, g as u8, b as u8)
    }
}

fn main() -> std::io::Result<()> {
    println!("Flywheel Matrix Streaming Demo");
    println!("==============================");
    println!("Simulating high-speed infinite generation.");
    println!("Type in the input box to test responsiveness.");
    println!("Press Escape to quit (or Ctrl+C).\n");

    std::thread::sleep(Duration::from_secs(1));

    let mut engine = Engine::new()?;
    let mut width = engine.width();
    let mut height = engine.height();

    // Layout
    let header_height = 2;
    let footer_height = 1;
    let content_height = height.saturating_sub(header_height + footer_height);

    let mut stream = StreamWidget::new(Rect::new(0, header_height, width, content_height));
    
    // Initial colors
    let header_bg = Rgb::new(20, 20, 20);
    let footer_bg = Rgb::new(30, 30, 30);

    // Draw header
    for x in 0..width {
        engine.set_cell(x, 0, Cell::new(' ').with_bg(header_bg));
        engine.set_cell(x, 1, Cell::new('=').with_fg(Rgb::new(50, 50, 50)).with_bg(header_bg));
    }
    engine.draw_text(2, 0, "FLYWHEEL MATRIX STRESS TEST", Rgb::new(0, 255, 100), header_bg);

    engine.request_redraw();

    // State
    let mut rng = Rng::new(12345);
    let mut token_count = 0u64;
    let mut frame_count = 0u64;
    let start_time = Instant::now();
    let mut user_input = String::new();

    
    // Resource monitoring
    let mut sys = System::new_with_specifics(
        RefreshKind::new()
            .with_cpu(CpuRefreshKind::everything())
            .with_memory(MemoryRefreshKind::everything())
    );
    // Initial refresh to set baseline
    std::thread::sleep(Duration::from_millis(100));
    sys.refresh_cpu();
    sys.refresh_memory();
    
    // We want to generate as fast as possible, but batch it per frame
    // to avoid excessive syscalls.


    // Event Loop
    let target_frame_time = Duration::from_micros(16_666); // ~60 FPS
    let mut last_tick = Instant::now();

    while engine.is_running() {
        let now = Instant::now();
        let time_since_tick = now.duration_since(last_tick);
        // If we missed the window, poll immediately (timeout 0)
        let timeout = target_frame_time.checked_sub(time_since_tick).unwrap_or(Duration::ZERO);

        match engine.input_receiver().recv_timeout(timeout) {
            Ok(event) => {
                // --- FAST PATH: Input Event (Instant Echo) ---
                match event {
                    InputEvent::Key { code, modifiers } => match code {
                        KeyCode::Esc => engine.stop(),
                        KeyCode::Char('c') if modifiers.control => engine.stop(),
                        KeyCode::Char('r') if modifiers.control => {
                            // Reset
                            stream.clear();
                            token_count = 0;
                            user_input.clear();
                        }
                        KeyCode::Char(c) => {
                            if !modifiers.control && !modifiers.alt {
                                user_input.push(c);
                            }
                        },
                        KeyCode::Backspace => { user_input.pop(); }
                        KeyCode::Enter => { user_input.clear(); },
                        _ => {
                            // Pass nav keys to widget
                            // Mapping simplified for demo
                            match code {
                                KeyCode::Up => stream.scroll_up(1),
                                KeyCode::Down => stream.scroll_down(1),
                                KeyCode::PageUp => stream.scroll_up(10),
                                KeyCode::PageDown => stream.scroll_down(10),
                                _ => {}
                            }
                        }
                    },
                    InputEvent::MouseScroll { delta, .. } => {
                        if delta > 0 { stream.scroll_up(1); }
                        else { stream.scroll_down(1); }
                    },
                    InputEvent::Resize { width: w, height: h } => {
                        width = w;
                        height = h;
                        engine.handle_resize(w, h);
                        let new_h = h.saturating_sub(header_height + footer_height);
                        stream.set_bounds(Rect::new(0, header_height, w, new_h));
                    }
                    InputEvent::Shutdown => engine.stop(),
                    _ => {}
                }

                // Immediate UI update for input responsiveness
                let cx = 4 + u16::try_from(user_input.len()).unwrap_or(u16::MAX);
                
                // Clear input line first (visual hack: just write spaces or redraw footer?)
                // Since we don't assume full redraw, we should clear the line.
                // But full footer redraw is cheap.
                for x in 0..width {
                    engine.set_cell(x, height - 1, Cell::new(' ').with_bg(footer_bg));
                }
                
                // Redraw Status + Input
                // (We reuse the status string from last frame or standard one?)
                // For input latency, we care about the INPUT text.
                // Re-calculating FPS here is overkill? Just use last stats.
                // We'll skip complex stats for input echo to be ultra-fast.
                let prompt = "> ";
                engine.draw_text(2, height - 1, prompt, Rgb::new(0, 255, 255), footer_bg);
                
                // Draw Input
                engine.draw_text(4, height - 1, &user_input, Rgb::WHITE, footer_bg);
                
                // Draw Cursor
                if cx < width {
                    engine.set_cell(cx, height - 1, Cell::new('█').with_fg(Rgb::new(0, 255, 255)).with_bg(footer_bg));
                }
                
                // Flush Input Update
                engine.request_update();
            }
            Err(RecvTimeoutError::Timeout) => {
                // --- TICK PATH: Matrix Generation (60Hz) ---
                last_tick = Instant::now();
                engine.begin_frame();

                // 1. Generate Matrix Text
                let mut fast_output: Vec<u8> = Vec::with_capacity(4096);
                
                // Batch size: 50
                for _ in 0..50 {
                    token_count += 1;
                    
                    // Simple random text
                    let color = rng.next_color();
                    stream.set_fg(color);
                    let c = rng.next_char();
                    
                    let mut buf = [0u8; 4];
                    let s_char = c.encode_utf8(&mut buf);
                    
                    stream.append_fast_into(s_char, &mut fast_output);
                } // End generation loop

                // Flush Matrix
                if !fast_output.is_empty() {
                    engine.write_raw(fast_output);
                }

                // 2. Update Stats (Throttle)
                frame_count += 1;
                if frame_count % 30 == 0 {
                    sys.refresh_cpu();
                    sys.refresh_memory();
                    
                    let elapsed = start_time.elapsed().as_secs_f32();
                    let fps = if elapsed > 0.0 { frame_count as f32 / elapsed } else { 0.0 };
                    let mem_mb = sys.used_memory() as f32 / 1024.0 / 1024.0;
                    let cpu = sys.global_cpu_info().cpu_usage();
                    
                    let status = format!(
                        "Esc: Quit | Ctrl+R: Reset | Chars: {token_count} | FPS: {fps:.1} | CPU: {cpu:.1}% | Mem: {mem_mb:.1}MB"
                    );
                    
                     // Draw Full Footer with Stats
                    for x in 0..width {
                        engine.set_cell(x, height - 1, Cell::new(' ').with_bg(footer_bg));
                    }
                    let status_len = u16::try_from(status.len()).unwrap_or(u16::MAX);
                    let stat_x = width.saturating_sub(status_len + 2);
                    engine.draw_text(stat_x, height - 1, &status, Rgb::new(150, 150, 150), footer_bg);
                    
                    // Redraw Input (since we cleared footer)
                    engine.draw_text(2, height - 1, "> ", Rgb::new(0, 255, 255), footer_bg);
                    engine.draw_text(4, height - 1, &user_input, Rgb::WHITE, footer_bg);
                    let input_len = u16::try_from(user_input.len()).unwrap_or(u16::MAX);
                    let cx = 4 + input_len;
                    if cx < width {
                        engine.set_cell(cx, height - 1, Cell::new('█').with_fg(Rgb::new(0, 255, 255)).with_bg(footer_bg));
                    }
                    
                    engine.request_update();
                }

                // 3. Render StreamWidget
                if stream.needs_redraw() || frame_count % 60 == 0 {
                    stream.render(engine.buffer_mut());
                }

                engine.end_frame();
            }
            Err(_) => engine.stop(), // Disconnected or other error
        }
    }

    Ok(())
}
