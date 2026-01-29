//! Engine: Main coordinator that ties actors together.
//!
//! The Engine is the entry point for applications using Flywheel.
//! It manages the terminal, spawns actors, and provides the main
//! event loop.

use super::messages::{InputEvent, RenderCommand};
use super::{InputActor, RendererActor};
use crate::buffer::{Buffer, Cell, Rgb};
use crate::layout::Rect;
use crossbeam_channel::{bounded, Receiver, Sender, TryRecvError};
use crossterm::{
    cursor,
    event::EnableMouseCapture,
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io::{self};
use std::time::{Duration, Instant};

/// Configuration for the Engine.
#[derive(Debug, Clone)]
pub struct EngineConfig {
    /// Target frames per second.
    pub target_fps: u32,
    /// Input poll timeout.
    pub input_poll_timeout: Duration,
    /// Whether to enable mouse capture.
    pub enable_mouse: bool,
    /// Whether to use alternate screen buffer.
    pub alternate_screen: bool,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            target_fps: 60,
            input_poll_timeout: Duration::from_millis(10),
            enable_mouse: false,
            alternate_screen: true,
        }
    }
}

/// The main Flywheel engine.
///
/// This coordinates between the input and render actors, providing
/// a simple interface for applications.
pub struct Engine {
    /// Configuration.
    config: EngineConfig,
    /// Input event receiver.
    input_rx: Receiver<InputEvent>,
    /// Render command sender.
    render_tx: Sender<RenderCommand>,
    /// Input actor handle.
    input_actor: Option<InputActor>,
    /// Renderer actor handle.
    #[allow(dead_code)]
    renderer_actor: Option<RendererActor>,
    /// Application buffer (for modifications).
    buffer: Buffer,
    /// Terminal width.
    width: u16,
    /// Terminal height.
    height: u16,
    /// Frame timing.
    frame_start: Instant,
    frame_duration: Duration,
    frame_count: u64,
    /// Whether the engine is running.
    running: bool,
}

impl Engine {
    /// Create a new engine with default configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if terminal setup fails (raw mode, alternate screen, etc.).
    pub fn new() -> io::Result<Self> {
        Self::with_config(EngineConfig::default())
    }

    /// Create a new engine with custom configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if terminal setup fails.
    pub fn with_config(config: EngineConfig) -> io::Result<Self> {
        // Get terminal size
        let (width, height) = terminal::size()?;

        // Enter raw mode and alternate screen
        terminal::enable_raw_mode()?;

        let mut stdout = io::stdout();
        if config.alternate_screen {
            execute!(stdout, EnterAlternateScreen)?;
        }
        if config.enable_mouse {
            execute!(stdout, EnableMouseCapture)?;
        }
        execute!(stdout, cursor::Hide)?;

        // Create channels
        let (input_tx, input_rx) = bounded::<InputEvent>(64);
        let (render_tx, render_rx) = bounded::<RenderCommand>(16);

        // Spawn actors
        let input_actor = InputActor::spawn(input_tx, config.input_poll_timeout);
        let renderer_actor = RendererActor::spawn(render_rx, width, height);

        let frame_duration = Duration::from_secs(1) / config.target_fps;

        Ok(Self {
            config,
            input_rx,
            render_tx,
            input_actor: Some(input_actor),
            renderer_actor: Some(renderer_actor),
            buffer: Buffer::new(width, height),
            width,
            height,
            frame_start: Instant::now(),
            frame_duration,
            frame_count: 0,
            running: true,
        })
    }

    /// Get the terminal width.
    pub const fn width(&self) -> u16 {
        self.width
    }

    /// Get the terminal height.
    pub const fn height(&self) -> u16 {
        self.height
    }

    /// Get a reference to the buffer.
    pub const fn buffer(&self) -> &Buffer {
        &self.buffer
    }

    /// Get a mutable reference to the buffer.
    pub const fn buffer_mut(&mut self) -> &mut Buffer {
        &mut self.buffer
    }

    /// Get the input receiver for event-driven loops.
    pub const fn input_receiver(&self) -> &Receiver<InputEvent> {
        &self.input_rx
    }

    /// Check if the engine is still running.
    pub const fn is_running(&self) -> bool {
        self.running
    }

    /// Stop the engine.
    pub const fn stop(&mut self) {
        self.running = false;
    }

    /// Poll for the next input event (non-blocking).
    ///
    /// Returns `None` if no event is available.
    pub fn poll_input(&self) -> Option<InputEvent> {
        match self.input_rx.try_recv() {
            Ok(event) => Some(event),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                Some(InputEvent::Error("Input channel disconnected".to_string()))
            }
        }
    }

    /// Wait for the next input event (blocking with timeout).
    pub fn wait_input(&self, timeout: Duration) -> Option<InputEvent> {
        self.input_rx.recv_timeout(timeout).ok()
    }

    /// Drain all pending input events.
    pub fn drain_input(&self) -> Vec<InputEvent> {
        let mut events = Vec::new();
        while let Ok(event) = self.input_rx.try_recv() {
            events.push(event);
        }
        events
    }

    /// Request a full redraw.
    pub fn request_redraw(&self) {
        let _ = self.render_tx.send(RenderCommand::FullRedraw(Box::new(self.buffer.clone())));
    }

    /// Request a diff-based update.
    pub fn request_update(&self) {
        let _ = self.render_tx.send(RenderCommand::Update(Box::new(self.buffer.clone())));
    }

    /// Set the cursor position (or hide it).
    pub fn set_cursor(&self, x: Option<u16>, y: u16) {
        let _ = self.render_tx.send(RenderCommand::SetCursor { x, y });
    }

    /// Write raw bytes to the output (Fast Path).
    pub fn write_raw(&self, bytes: Vec<u8>) {
        let _ = self.render_tx.send(RenderCommand::RawOutput { bytes });
    }

    /// Handle a resize event.
    pub fn handle_resize(&mut self, width: u16, height: u16) {
        self.width = width;
        self.height = height;
        self.buffer.resize(width, height);
        let _ = self.render_tx.send(RenderCommand::Resize { width, height });
    }

    /// Begin a new frame.
    ///
    /// Call this at the start of your render loop.
    pub fn begin_frame(&mut self) {
        self.frame_start = Instant::now();
    }

    /// End a frame and request update.
    ///
    /// This will sleep if necessary to maintain the target FPS.
    pub fn end_frame(&mut self) {
        self.frame_count += 1;

        // Request render
        self.request_update();

        // Frame rate limiting
        let elapsed = self.frame_start.elapsed();
        if elapsed < self.frame_duration {
            std::thread::sleep(self.frame_duration - elapsed);
        }
    }

    /// Get the current frame count.
    pub const fn frame_count(&self) -> u64 {
        self.frame_count
    }

    /// Convenience: Set a cell in the buffer.
    pub fn set_cell(&mut self, x: u16, y: u16, cell: Cell) {
        self.buffer.set(x, y, cell);
    }

    /// Convenience: Set a grapheme in the buffer.
    pub fn set_grapheme(&mut self, x: u16, y: u16, grapheme: &str, fg: Rgb, bg: Rgb) -> u8 {
        self.buffer.set_grapheme(x, y, grapheme, fg, bg)
    }

    /// Convenience: Clear the buffer.
    pub fn clear(&mut self) {
        self.buffer.clear();
    }

    /// Convenience: Fill a rectangle.
    pub fn fill_rect(&mut self, rect: Rect, cell: Cell) {
        self.buffer.fill_rect(rect.x, rect.y, rect.width, rect.height, cell);
    }

    /// Draw text at a position.
    ///
    /// Returns the number of columns used.
    pub fn draw_text(&mut self, x: u16, y: u16, text: &str, fg: Rgb, bg: Rgb) -> u16 {
        let mut col = x;
        for grapheme in unicode_segmentation::UnicodeSegmentation::graphemes(text, true) {
            if col >= self.width {
                break;
            }
            let width = self.buffer.set_grapheme(col, y, grapheme, fg, bg);
            col += u16::from(width);
        }
        col - x
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        // Stop actors
        if let Some(actor) = self.input_actor.take() {
            actor.join();
        }

        let _ = self.render_tx.send(RenderCommand::Shutdown);

        // Restore terminal state
        let mut stdout = io::stdout();
        let _ = execute!(stdout, cursor::Show);
        if self.config.enable_mouse {
            let _ = execute!(stdout, crossterm::event::DisableMouseCapture);
        }
        if self.config.alternate_screen {
            let _ = execute!(stdout, LeaveAlternateScreen);
        }
        let _ = terminal::disable_raw_mode();
    }
}
