//! Renderer Actor: Dedicated thread for rendering to the terminal.
//!
//! This actor owns the terminal and double buffers. It receives render
//! commands from the main loop and performs the actual diffing and
//! output flushing.

use super::messages::RenderCommand;
use crate::buffer::diff::{render_diff, render_full, DiffState};
use crate::buffer::Buffer;
use crate::layout::Rect;
use crossbeam_channel::Receiver;
use std::io::{self, Stdout, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

/// Renderer actor that handles terminal output.
pub struct RendererActor {
    /// Handle to the render thread.
    handle: Option<JoinHandle<()>>,
    /// Flag to signal shutdown.
    shutdown: Arc<AtomicBool>,
}

/// Render statistics for debugging/profiling.
#[derive(Debug, Clone, Default)]
pub struct RenderStats {
    /// Total frames rendered.
    pub frames: u64,
    /// Total cells changed across all frames.
    pub cells_changed: u64,
    /// Total bytes written to terminal.
    pub bytes_written: u64,
    /// Average render time in microseconds.
    pub avg_render_us: u64,
    /// Last render time in microseconds.
    pub last_render_us: u64,
}

/// Internal renderer state.
struct Renderer {
    /// Current (visible) buffer.
    current: Buffer,
    /// Next (being drawn) buffer.
    next: Buffer,
    /// Diff state for cursor/color tracking.
    diff_state: DiffState,
    /// Pre-allocated output buffer.
    output: Vec<u8>,
    /// Terminal stdout handle.
    stdout: Stdout,
    /// Render statistics.
    stats: RenderStats,
    /// Dirty rectangles for next render.
    dirty_rects: Vec<Rect>,
    /// Whether a full redraw is needed.
    needs_full_redraw: bool,
    /// Cursor position (None = hidden).
    cursor_x: Option<u16>,
    cursor_y: u16,
}

impl Renderer {
    /// Create a new renderer with the given dimensions.
    fn new(width: u16, height: u16) -> io::Result<Self> {
        let current = Buffer::new(width, height);
        let next = Buffer::new(width, height);

        Ok(Self {
            current,
            next,
            diff_state: DiffState::new(),
            output: Vec::with_capacity(65536),
            stdout: io::stdout(),
            stats: RenderStats::default(),
            dirty_rects: Vec::new(),
            needs_full_redraw: true,
            cursor_x: None,
            cursor_y: 0,
        })
    }

    /// Get a mutable reference to the next buffer.
    #[allow(dead_code)]
    pub fn buffer_mut(&mut self) -> &mut Buffer {
        &mut self.next
    }

    /// Mark the entire screen as dirty.
    fn mark_full_dirty(&mut self) {
        self.needs_full_redraw = true;
    }

    /// Add a dirty rectangle.
    #[allow(dead_code)]
    fn mark_dirty(&mut self, rect: Rect) {
        self.dirty_rects.push(rect);
    }

    /// Perform a render cycle.
    fn render(&mut self) -> io::Result<()> {
        let start = Instant::now();
        self.output.clear();

        if self.needs_full_redraw {
            // Full redraw
            render_full(&self.next, &mut self.output);
            self.needs_full_redraw = false;
            self.diff_state.reset();
        } else {
            // Diff-based update
            let _result = render_diff(
                &self.current,
                &self.next,
                &self.dirty_rects,
                &mut self.output,
                &mut self.diff_state,
            );
        }

        self.dirty_rects.clear();

        // Handle cursor position
        if let Some(x) = self.cursor_x {
            // Show cursor at position
            use std::io::Write as IoWrite;
            let _ = write!(
                &mut self.output,
                "\x1b[{};{}H\x1b[?25h",
                self.cursor_y + 1,
                x + 1
            );
        } else {
            // Hide cursor
            self.output.extend_from_slice(b"\x1b[?25l");
        }

        // Flush to terminal in a single write
        if !self.output.is_empty() {
            self.stdout.write_all(&self.output)?;
            self.stdout.flush()?;
        }

        // Swap buffers
        self.current.copy_from(&self.next);

        // Update stats
        let elapsed = start.elapsed();
        self.stats.frames += 1;
        self.stats.bytes_written += self.output.len() as u64;
        self.stats.last_render_us = elapsed.as_micros() as u64;

        // Smoothed average
        if self.stats.avg_render_us == 0 {
            self.stats.avg_render_us = self.stats.last_render_us;
        } else {
            self.stats.avg_render_us =
                (self.stats.avg_render_us * 15 + self.stats.last_render_us) / 16;
        }

        Ok(())
    }

    /// Resize buffers.
    fn resize(&mut self, width: u16, height: u16) {
        self.current.resize(width, height);
        self.next.resize(width, height);
        self.mark_full_dirty();
    }

    /// Set cursor position.
    fn set_cursor(&mut self, x: Option<u16>, y: u16) {
        self.cursor_x = x;
        self.cursor_y = y;
    }
}

impl RendererActor {
    /// Spawn the renderer actor thread.
    ///
    /// # Arguments
    ///
    /// * `receiver` - Channel to receive render commands from.
    /// * `width` - Initial terminal width.
    /// * `height` - Initial terminal height.
    ///
    /// # Returns
    ///
    /// The renderer actor handle.
    pub fn spawn(receiver: Receiver<RenderCommand>, width: u16, height: u16) -> Self {
        let shutdown = Arc::new(AtomicBool::new(false));
        let shutdown_clone = shutdown.clone();

        let handle = thread::Builder::new()
            .name("flywheel-render".to_string())
            .spawn(move || {
                if let Err(e) = Self::run_loop(receiver, shutdown_clone, width, height) {
                    eprintln!("Render thread error: {e}");
                }
            })
            .expect("Failed to spawn render thread");

        Self {
            handle: Some(handle),
            shutdown,
        }
    }

    /// Signal the render thread to shutdown.
    pub fn shutdown(&self) {
        self.shutdown.store(true, Ordering::Relaxed);
    }

    /// Wait for the render thread to finish.
    pub fn join(mut self) {
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }

    /// Main render loop.
    fn run_loop(
        receiver: Receiver<RenderCommand>,
        shutdown: Arc<AtomicBool>,
        width: u16,
        height: u16,
    ) -> io::Result<()> {
        let mut renderer = Renderer::new(width, height)?;

        loop {
            // Check for shutdown
            if shutdown.load(Ordering::Relaxed) {
                break;
            }

            // Wait for command with timeout
            match receiver.recv_timeout(Duration::from_millis(16)) {
                Ok(command) => match command {
                    RenderCommand::FullRedraw => {
                        renderer.mark_full_dirty();
                        renderer.render()?;
                    }
                    RenderCommand::Update => {
                        renderer.render()?;
                    }
                    RenderCommand::Resize { width, height } => {
                        renderer.resize(width, height);
                    }
                    RenderCommand::SetCursor { x, y } => {
                        renderer.set_cursor(x, y);
                    }
                    RenderCommand::Shutdown => {
                        break;
                    }
                },
                Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                    // No command, just continue
                }
                Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                    // Channel closed, exit
                    break;
                }
            }
        }

        Ok(())
    }
}

impl Drop for RendererActor {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_renderer_new() {
        let renderer = Renderer::new(80, 24).unwrap();
        assert_eq!(renderer.current.width(), 80);
        assert_eq!(renderer.current.height(), 24);
        assert!(renderer.needs_full_redraw);
    }

    #[test]
    fn test_renderer_resize() {
        let mut renderer = Renderer::new(80, 24).unwrap();
        renderer.needs_full_redraw = false;
        renderer.resize(100, 30);
        assert_eq!(renderer.current.width(), 100);
        assert_eq!(renderer.next.width(), 100);
        assert!(renderer.needs_full_redraw);
    }
}
