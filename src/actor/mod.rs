//! Actor Model: Message-passing concurrency for the TUI engine.
//!
//! This module implements a simple actor system using crossbeam channels:
//! - **Input Actor**: Polls terminal events, forwards to main loop
//! - **Render Actor**: Receives render commands, diffs and flushes
//! - **Main Loop**: Coordinates between actors, handles application logic
//!
//! # Architecture
//!
//! ```text
//! ┌──────────────┐     InputEvent      ┌──────────────┐
//! │ Input Thread │ ─────────────────▶  │              │
//! └──────────────┘                     │  Main Loop   │
//!                                      │              │
//! ┌──────────────┐    RenderCommand    │              │
//! │Render Thread │ ◀───────────────── │              │
//! └──────────────┘                     └──────────────┘
//!                                            │
//!                                            │ AgentEvent
//!                                            ▼
//!                                      ┌──────────────┐
//!                                      │ Agent/Network│
//!                                      └──────────────┘
//! ```

mod messages;
mod input;
mod renderer;
mod engine;

pub use messages::{InputEvent, RenderCommand, AgentEvent, KeyCode, KeyModifiers, MouseButton, MouseEvent};
pub use input::InputActor;
pub use renderer::RendererActor;
pub use engine::{Engine, EngineConfig};
