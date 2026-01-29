//! Input Actor: Dedicated thread for polling terminal events.
//!
//! This actor runs in its own thread and uses crossterm's event polling
//! to capture keyboard, mouse, and resize events without blocking the
//! main application.

use super::messages::{InputEvent, KeyCode, KeyModifiers, MouseButton, MouseEvent};
use crossbeam_channel::Sender;
use crossterm::event::{self, Event, KeyEventKind};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

/// Input actor that polls terminal events.
pub struct InputActor {
    /// Handle to the input thread.
    handle: Option<JoinHandle<()>>,
    /// Flag to signal shutdown.
    shutdown: Arc<AtomicBool>,
}

impl InputActor {
    /// Spawn the input actor thread.
    ///
    /// # Arguments
    ///
    /// * `sender` - Channel to send input events to the main loop.
    /// * `poll_timeout` - How long to wait for events before checking shutdown.
    ///
    /// # Returns
    ///
    /// The input actor handle.
    pub fn spawn(sender: Sender<InputEvent>, poll_timeout: Duration) -> Self {
        let shutdown = Arc::new(AtomicBool::new(false));
        let shutdown_clone = shutdown.clone();

        let handle = thread::Builder::new()
            .name("flywheel-input".to_string())
            .spawn(move || {
                Self::run_loop(sender, shutdown_clone, poll_timeout);
            })
            .expect("Failed to spawn input thread");

        Self {
            handle: Some(handle),
            shutdown,
        }
    }

    /// Signal the input thread to shutdown.
    pub fn shutdown(&self) {
        self.shutdown.store(true, Ordering::Relaxed);
    }

    /// Wait for the input thread to finish.
    pub fn join(mut self) {
        self.shutdown();
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }

    /// Main input polling loop.
    fn run_loop(sender: Sender<InputEvent>, shutdown: Arc<AtomicBool>, poll_timeout: Duration) {
        loop {
            // Check for shutdown
            if shutdown.load(Ordering::Relaxed) {
                let _ = sender.send(InputEvent::Shutdown);
                break;
            }

            // Poll for events with timeout
            match event::poll(poll_timeout) {
                Ok(true) => {
                    // Event available, read it
                    match event::read() {
                        Ok(event) => {
                            if let Some(input_event) = Self::convert_event(event) {
                                if sender.send(input_event).is_err() {
                                    // Receiver dropped, exit
                                    break;
                                }
                            }
                        }
                        Err(e) => {
                            let _ = sender.send(InputEvent::Error(e.to_string()));
                        }
                    }
                }
                Ok(false) => {
                    // No event, continue loop (will check shutdown)
                }
                Err(e) => {
                    let _ = sender.send(InputEvent::Error(e.to_string()));
                }
            }
        }
    }

    /// Convert a crossterm event to our InputEvent.
    fn convert_event(event: Event) -> Option<InputEvent> {
        match event {
            Event::Key(key_event) => {
                // Only process key press events (not release or repeat)
                if key_event.kind != KeyEventKind::Press {
                    return None;
                }

                let code = Self::convert_key_code(key_event.code)?;
                let modifiers = Self::convert_modifiers(key_event.modifiers);

                Some(InputEvent::Key { code, modifiers })
            }

            Event::Mouse(mouse_event) => Self::convert_mouse_event(mouse_event),

            Event::Resize(width, height) => Some(InputEvent::Resize { width, height }),

            Event::FocusGained => Some(InputEvent::FocusGained),

            Event::FocusLost => Some(InputEvent::FocusLost),

            Event::Paste(text) => Some(InputEvent::Paste(text)),
        }
    }

    /// Convert crossterm KeyCode to our KeyCode.
    fn convert_key_code(code: event::KeyCode) -> Option<KeyCode> {
        Some(match code {
            event::KeyCode::Char(c) => KeyCode::Char(c),
            event::KeyCode::F(n) => KeyCode::F(n),
            event::KeyCode::Backspace => KeyCode::Backspace,
            event::KeyCode::Enter => KeyCode::Enter,
            event::KeyCode::Left => KeyCode::Left,
            event::KeyCode::Right => KeyCode::Right,
            event::KeyCode::Up => KeyCode::Up,
            event::KeyCode::Down => KeyCode::Down,
            event::KeyCode::Home => KeyCode::Home,
            event::KeyCode::End => KeyCode::End,
            event::KeyCode::PageUp => KeyCode::PageUp,
            event::KeyCode::PageDown => KeyCode::PageDown,
            event::KeyCode::Tab => KeyCode::Tab,
            event::KeyCode::BackTab => KeyCode::BackTab,
            event::KeyCode::Delete => KeyCode::Delete,
            event::KeyCode::Insert => KeyCode::Insert,
            event::KeyCode::Esc => KeyCode::Esc,
            event::KeyCode::Null => KeyCode::Null,
            _ => return None, // Ignore other key codes
        })
    }

    /// Convert crossterm KeyModifiers to our KeyModifiers.
    fn convert_modifiers(mods: event::KeyModifiers) -> KeyModifiers {
        KeyModifiers {
            shift: mods.contains(event::KeyModifiers::SHIFT),
            control: mods.contains(event::KeyModifiers::CONTROL),
            alt: mods.contains(event::KeyModifiers::ALT),
            super_key: mods.contains(event::KeyModifiers::SUPER),
        }
    }

    /// Convert crossterm MouseEvent to our InputEvent.
    fn convert_mouse_event(mouse: event::MouseEvent) -> Option<InputEvent> {
        let modifiers = Self::convert_modifiers(mouse.modifiers);

        match mouse.kind {
            event::MouseEventKind::Down(button) => {
                let button = Self::convert_mouse_button(button)?;
                Some(InputEvent::MouseDown(MouseEvent {
                    x: mouse.column,
                    y: mouse.row,
                    button: Some(button),
                    modifiers,
                }))
            }
            event::MouseEventKind::Up(button) => {
                let button = Self::convert_mouse_button(button)?;
                Some(InputEvent::MouseUp(MouseEvent {
                    x: mouse.column,
                    y: mouse.row,
                    button: Some(button),
                    modifiers,
                }))
            }
            event::MouseEventKind::Moved => Some(InputEvent::MouseMove(MouseEvent {
                x: mouse.column,
                y: mouse.row,
                button: None,
                modifiers,
            })),
            event::MouseEventKind::Drag(button) => {
                let button = Self::convert_mouse_button(button)?;
                Some(InputEvent::MouseMove(MouseEvent {
                    x: mouse.column,
                    y: mouse.row,
                    button: Some(button),
                    modifiers,
                }))
            }
            event::MouseEventKind::ScrollUp => Some(InputEvent::MouseScroll {
                x: mouse.column,
                y: mouse.row,
                delta: 1,
            }),
            event::MouseEventKind::ScrollDown => Some(InputEvent::MouseScroll {
                x: mouse.column,
                y: mouse.row,
                delta: -1,
            }),
            _ => None,
        }
    }

    /// Convert crossterm MouseButton to our MouseButton.
    fn convert_mouse_button(button: event::MouseButton) -> Option<MouseButton> {
        Some(match button {
            event::MouseButton::Left => MouseButton::Left,
            event::MouseButton::Right => MouseButton::Right,
            event::MouseButton::Middle => MouseButton::Middle,
        })
    }
}

impl Drop for InputActor {
    fn drop(&mut self) {
        self.shutdown();
    }
}
