//! Ticker Actor: Dedicated thread for generating timing events.
//!
//! This actor provides a regular "tick" signal for animation and
//! frame pacing. It decouples timing from the main thread, enabling
//! async-friendly applications.

use crossbeam_channel::{bounded, Receiver, Sender};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

/// A tick event sent at regular intervals.
#[derive(Debug, Clone, Copy)]
pub struct Tick {
    /// Frame number (monotonically increasing).
    pub frame: u64,
    /// Time elapsed since the ticker was started.
    pub elapsed: Duration,
}

/// Ticker actor that generates regular timing events.
pub struct TickerActor {
    /// Handle to the ticker thread.
    handle: Option<JoinHandle<()>>,
    /// Flag to signal shutdown.
    shutdown: Arc<AtomicBool>,
    /// Receiver for tick events.
    tick_rx: Receiver<Tick>,
}

impl TickerActor {
    /// Spawn a new ticker actor with the given interval.
    ///
    /// # Arguments
    ///
    /// * `interval` - Time between ticks (e.g., 16ms for ~60 FPS).
    ///
    /// # Returns
    ///
    /// The ticker actor with its tick receiver accessible.
    ///
    /// # Panics
    ///
    /// Panics if the OS fails to spawn the ticker thread.
    #[allow(clippy::missing_panics_doc)]
    pub fn spawn(interval: Duration) -> Self {
        let shutdown = Arc::new(AtomicBool::new(false));
        let shutdown_clone = shutdown.clone();

        // Bounded channel with small buffer - we don't want ticks to queue up
        let (tick_tx, tick_rx) = bounded(2);

        let handle = thread::Builder::new()
            .name("flywheel-ticker".to_string())
            .spawn(move || {
                Self::run_loop(&tick_tx, &shutdown_clone, interval);
            })
            .expect("Failed to spawn ticker thread");

        Self {
            handle: Some(handle),
            shutdown,
            tick_rx,
        }
    }

    /// Get a reference to the tick receiver.
    ///
    /// Use this with `select!` for event-driven loops:
    ///
    /// ```ignore
    /// loop {
    ///     select! {
    ///         recv(engine.input_receiver()) -> event => handle_input(event),
    ///         recv(ticker.receiver()) -> tick => {
    ///             generate_frame();
    ///             engine.request_update();
    ///         }
    ///     }
    /// }
    /// ```
    #[inline]
    pub const fn receiver(&self) -> &Receiver<Tick> {
        &self.tick_rx
    }

    /// Signal the ticker to shutdown.
    pub fn shutdown(&self) {
        self.shutdown.store(true, Ordering::Relaxed);
    }

    /// Wait for the ticker thread to finish.
    pub fn join(mut self) {
        self.shutdown();
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }

    /// Main ticker loop.
    fn run_loop(tick_tx: &Sender<Tick>, shutdown: &Arc<AtomicBool>, interval: Duration) {
        let start = Instant::now();
        let mut frame = 0u64;
        let mut next_tick = start + interval;

        loop {
            if shutdown.load(Ordering::Relaxed) {
                break;
            }

            let now = Instant::now();
            if now >= next_tick {
                // Time to tick
                let tick = Tick {
                    frame,
                    elapsed: now - start,
                };

                // Non-blocking send - if buffer is full, skip this tick
                // (receiver is too slow, prevent queue buildup)
                let _ = tick_tx.try_send(tick);

                frame += 1;
                next_tick += interval;

                // Handle case where we're behind (catch up without queuing)
                if next_tick < now {
                    next_tick = now + interval;
                }
            } else {
                // Sleep until next tick
                let sleep_duration = next_tick - now;
                thread::sleep(sleep_duration.min(Duration::from_millis(1)));
            }
        }
    }
}

impl Drop for TickerActor {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ticker_basic() {
        let ticker = TickerActor::spawn(Duration::from_millis(10));

        // Should receive ticks
        let tick = ticker.receiver().recv_timeout(Duration::from_millis(100));
        assert!(tick.is_ok());
        assert_eq!(tick.unwrap().frame, 0);

        // Second tick
        let tick2 = ticker.receiver().recv_timeout(Duration::from_millis(50));
        assert!(tick2.is_ok());

        ticker.join();
    }

    #[test]
    fn test_ticker_shutdown() {
        let ticker = TickerActor::spawn(Duration::from_millis(100));
        ticker.shutdown();

        // Should stop receiving ticks after shutdown
        thread::sleep(Duration::from_millis(50));
        ticker.join();
    }
}
