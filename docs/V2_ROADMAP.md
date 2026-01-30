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

## Phase 3: Rope-Based Buffer (Priority: Medium) ✅ COMPLETED

### Problem
`ScrollBuffer` uses `VecDeque<StyledLine>` where each `StyledLine` is a `Vec<Cell>`.
- 1M lines = 1M heap allocations
- Insert/delete at middle is O(n)
- Poor cache locality

### Solution: RopeBuffer

Created `src/buffer/rope.rs` with chunked storage:

```rust
const CHUNK_SIZE: usize = 64;  // Lines per chunk

let mut buffer = RopeBuffer::new(10_000);  // Max 10K lines
buffer.append([Cell::new('H'), Cell::new('i')].into_iter());
buffer.newline();

// Efficient iteration
for (i, line) in buffer.visible_lines(24) {
    // Render line...
}

// Memory diagnostics
let stats = buffer.memory_stats();
println!("Lines: {}, Chunks: {}", stats.lines, stats.chunks);
```

### Benefits
- **Fewer allocations**: 1M lines = ~16K chunk allocations (vs 1M)
- **Better locality**: Lines in same chunk are contiguous
- **Automatic trimming**: Old lines removed when max_lines exceeded
- **Memory stats**: `memory_stats()` for debugging

### Status: ✅ Complete (Committed: 2024-01-30)

---

## Phase 4: Widget System (Priority: Medium) ✅ COMPLETED

### Solution: Widget Trait + Core Widgets

Created a proper widget system in `src/widget/`:

**Widget Trait** (`traits.rs`)
```rust
pub trait Widget {
    fn bounds(&self) -> Rect;
    fn set_bounds(&mut self, bounds: Rect);
    fn render(&self, buffer: &mut Buffer);
    fn handle_input(&mut self, event: &InputEvent) -> bool;
    fn needs_redraw(&self) -> bool;
    fn clear_redraw(&mut self);
}
```

**Implemented Widgets:**
- `TextInput` - Single-line input with cursor, editing, navigation
- `StatusBar` - Three-section header (left, center, right)
- `ProgressBar` - Animated progress with multiple styles

**Demo:** `examples/widgets_demo.rs`

### Status: ✅ Complete (Committed: 2024-01-30)

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
| 3. Rope Buffer | Medium | ✅ Done | 2h |
| 4. Widget System | Medium | ✅ Done | 3h |
| 5. Docs & Polish | High | 📋 In Progress | 1h |

**Completed: 8 hours | Remaining: 1-2 hours**

---

## Out of Scope (V3+)

- WASM target for browser terminals
- Plugin system for custom widgets
- Multiplexer support (tmux pane detection)
- GPU acceleration (wgpu backend)
- Accessibility (screen reader support)
