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
    let header_height = 0u16;
    let footer_height = 2u16;
    let footer_bg = Rgb::new(30, 30, 30);
    let content_height = height.saturating_sub(header_height + footer_height);

    let mut stream = StreamWidget::new(Rect::new(0, header_height, width, content_height));
    
    // Initial colors


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
    // Initial render
    engine.begin_frame();
    draw_demo_footer(&mut engine, width, height, &user_input, "Initializing...", footer_bg, 0);
    engine.request_update();
    engine.end_frame();

    // Event Loop
    let target_frame_time = Duration::from_micros(16_666); // ~60 FPS
    let mut last_tick = Instant::now();
    let mut status_line = String::from("Starting...");

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

                // Redraw Footer immediately
                draw_demo_footer(
                    &mut engine, 
                    width, 
                    height, 
                    &user_input, 
                    &status_line, 
                    footer_bg,
                    frame_count
                );
                engine.request_update();
            }
            Err(RecvTimeoutError::Timeout) => {
                // --- TICK PATH: Matrix Generation (60Hz) ---
                last_tick = Instant::now();
                let mut buffer_dirty = false;

                // 1. Generate Matrix Text (Fast Path - Bypass Buffer)
                let mut fast_output: Vec<u8> = Vec::with_capacity(4096);
                for _ in 0..50 {
                    token_count += 1;
                    let color = rng.next_color();
                    stream.set_fg(color);
                    let c = rng.next_char();
                    let mut buf = [0u8; 4];
                    let s_char = c.encode_utf8(&mut buf);
                    if stream.append_fast_into(s_char, &mut fast_output) {
                        // All good, handled by RawOutput
                    } else {
                        // Slow path hit (wrap or scroll)
                        buffer_dirty = true;
                    }
                }

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
                    
                    status_line = format!(
                        "Chars: {token_count} | FPS: {fps:.1} | CPU: {cpu:.1}% | Mem: {mem_mb:.1}MB"
                    );
                    buffer_dirty = true;
                }

                // 3. Blink Cursor or Handle Slow Path redrawing
                if frame_count % 15 == 0 || stream.needs_redraw() || buffer_dirty {
                    draw_demo_footer(
                        &mut engine, 
                        width, 
                        height, 
                        &user_input, 
                        &status_line, 
                        footer_bg,
                        frame_count
                    );
                    
                    if stream.needs_redraw() || frame_count % 60 == 0 {
                        stream.render(engine.buffer_mut());
                    }
                    
                    engine.request_update();
                }
            }
            Err(_) => break, // Disconnected or other error
        }
    }

    Ok(())
}

/// Helper to draw consistent UI.
fn draw_demo_footer(
    engine: &mut Engine,
    width: u16,
    height: u16,
    user_input: &str,
    status: &str,
    bg: Rgb,
    frame_count: u64,
) {
    let y = height.saturating_sub(1);
    
    // Clear line
    for x in 0..width {
        engine.set_cell(x, y, Cell::new(' ').with_bg(bg));
    }

    // Input left
    engine.draw_text(2, y, "> ", Rgb::new(0, 255, 255), bg);
    engine.draw_text(4, y, user_input, Rgb::WHITE, bg);

    // Cursor
    let input_len = u16::try_from(user_input.len()).unwrap_or(0);
    let cx = 4 + input_len;
    if cx < width && (frame_count % 30 < 15 || input_len > 0) {
        // Blink except when typing
        engine.set_cell(cx, y, Cell::new('█').with_fg(Rgb::new(0, 255, 255)).with_bg(bg));
    }

    // Status right
    if !status.is_empty() {
        let status_len = u16::try_from(status.len()).unwrap_or(0);
        let stat_x = width.saturating_sub(status_len + 2);
        if stat_x > cx + 2 {
            engine.draw_text(stat_x, y, status, Rgb::new(150, 150, 150), bg);
        }
    }
}
