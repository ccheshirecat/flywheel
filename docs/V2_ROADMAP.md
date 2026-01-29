# Flywheel V2 Roadmap

## Overview

V2 represents a significant architectural evolution focused on three pillars:
1. **Async-First Architecture** - Remove blocking patterns, enable tokio/async-std compatibility
2. **Memory Efficiency** - Rope-based storage for 1M+ line documents
3. **Robustness** - Eliminate all visual artifacts (ghost characters, residue bugs)

---

## Phase 1: Buffer Synchronization Fix (Priority: Critical) ✅ COMPLETED

### Problem
The "ghost character" bug occurs because Fast Path writes directly to the terminal, bypassing the Buffer. The Renderer's `current_buffer` doesn't know about these writes, so diffing skips them.

### Solution: Force Full Redraw on Path Switch

Instead of complex shadow tracking, we use a simpler approach:
- After any `RawOutput`, set `needs_full_redraw = true`
- The next `Update` command triggers a full redraw, resyncing state
- This maintains Fast Path performance for consecutive writes
- Only pays the resync cost when switching from Fast to Slow path

### Implementation
```rust
fn write_raw(&mut self, bytes: &[u8]) -> io::Result<()> {
    self.stdout.write_all(bytes)?;
    self.stdout.flush()?;
    // Force resync on next Update
    self.needs_full_redraw = true;
    self.diff_state.reset();
    Ok(())
}
```

### Status: ✅ Complete (Committed: 2024-01-30)

---

## Phase 2: Async-Friendly Ticker (Priority: High) ✅ COMPLETED

### Problem
`Engine::end_frame()` calls `std::thread::sleep()`, blocking the calling thread. This is incompatible with async runtimes like tokio.

### Solution: TickerActor

Created a dedicated `TickerActor` that generates timing events independently:

```rust
// Spawn ticker for 60 FPS
let ticker = TickerActor::spawn(Duration::from_micros(16_666));

// Event-driven loop using select!
while engine.is_running() {
    select! {
        recv(engine.input_receiver()) -> event => handle_input(event),
        recv(ticker.receiver()) -> tick => {
            generate_content();
            engine.request_update();
        }
    }
}

// Clean shutdown
ticker.join();
```

### Features
- Non-blocking: Ticker runs in dedicated thread
- Smart pacing: Uses `try_send` to prevent tick queue buildup
- Clean shutdown: Respects shutdown signal via `AtomicBool`
- Frame info: `Tick` struct includes frame number and elapsed time

### Status: ✅ Complete (Committed: 2024-01-30)

---

## Phase 3: Rope-Based ScrollBuffer (Priority: Medium)

### Problem
`ScrollBuffer` uses `VecDeque<StyledLine>` where each `StyledLine` is a `Vec<Cell>`.
- 1M lines = 1M heap allocations
- Insert/delete at middle is O(n)
- Poor cache locality

### Solution: Chunked Rope

```rust
const CHUNK_SIZE: usize = 64;  // Lines per chunk

struct Chunk {
    lines: ArrayVec<StyledLine, CHUNK_SIZE>,
}

struct RopeBuffer {
    chunks: Vec<Chunk>,
    total_lines: usize,
}

impl RopeBuffer {
    fn get_line(&self, index: usize) -> Option<&StyledLine> {
        let chunk_idx = index / CHUNK_SIZE;
        let line_idx = index % CHUNK_SIZE;
        self.chunks.get(chunk_idx)?.lines.get(line_idx)
    }

    fn push_line(&mut self, line: StyledLine) {
        if self.chunks.is_empty() || self.chunks.last().unwrap().lines.is_full() {
            self.chunks.push(Chunk::new());
        }
        self.chunks.last_mut().unwrap().lines.push(line);
        self.total_lines += 1;
    }
}
```

### Benefits
- **Fewer allocations**: 1M lines = ~16K chunk allocations (vs 1M)
- **Better locality**: Lines in same chunk are contiguous
- **Faster scroll**: Jump to chunk, then index within

### Tasks
- [ ] Create `src/buffer/rope.rs`
- [ ] Implement `RopeBuffer` with chunk-based storage
- [ ] Add `Iterator` support for rendering
- [ ] Benchmark: `VecDeque` vs `RopeBuffer` at 100K, 500K, 1M lines
- [ ] Migrate `ScrollBuffer` to use `RopeBuffer` internally

### Estimated Effort: 4-6 hours

---

## Phase 4: Widget System (Priority: Medium)

### Current State
Only `StreamWidget` exists. Users must manually compose UI.

### V2 Widgets

```rust
// Layout containers
struct VSplit { top: Box<dyn Widget>, bottom: Box<dyn Widget>, ratio: f32 }
struct HSplit { left: Box<dyn Widget>, right: Box<dyn Widget>, ratio: f32 }
struct Stack { layers: Vec<Box<dyn Widget>> }  // For modals/overlays

// Common widgets
struct TextInput { content: String, cursor: usize, focused: bool }
struct StatusBar { left: String, center: String, right: String }
struct ProgressBar { progress: f32, style: ProgressStyle }

// Trait
trait Widget {
    fn bounds(&self) -> Rect;
    fn set_bounds(&mut self, bounds: Rect);
    fn render(&self, buffer: &mut Buffer);
    fn handle_input(&mut self, event: &InputEvent) -> bool;  // Returns "consumed"
}
```

### Tasks
- [ ] Define `Widget` trait in `src/widget/mod.rs`
- [ ] Implement `TextInput` widget
- [ ] Implement `StatusBar` widget
- [ ] Implement `VSplit` / `HSplit` containers
- [ ] Add focus management system
- [ ] Create `widgets_demo.rs` example

### Estimated Effort: 6-8 hours

---

## Phase 5: Documentation & Polish (Priority: High)

### Tasks
- [ ] Rustdoc for all public APIs
- [ ] Architecture diagram (Mermaid in README)
- [ ] Performance tuning guide
- [ ] Migration guide (V1 → V2)
- [ ] Publish to crates.io

---

## Implementation Order

| Phase | Priority | Status | Effort |
|-------|----------|--------|--------|
| 1. Buffer Sync Fix | Critical | ✅ Done | 1h |
| 2. Async Ticker | High | ✅ Done | 2h |
| 3. Rope Buffer | Medium | 📋 Planned | 4-6h |
| 4. Widget System | Medium | 📋 Planned | 6-8h |
| 5. Docs & Polish | High | 📋 Planned | 2-3h |

**Completed: 3 hours | Remaining: 12-17 hours**

---

## Out of Scope (V3+)

- WASM target for browser terminals
- Plugin system for custom widgets
- Multiplexer support (tmux pane detection)
- GPU acceleration (wgpu backend)
- Accessibility (screen reader support)
