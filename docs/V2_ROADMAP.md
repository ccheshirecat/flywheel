# Flywheel V2 Roadmap

## Overview

V2 represents a significant architectural evolution focused on three pillars:
1. **Async-First Architecture** - Remove blocking patterns, enable tokio/async-std compatibility
2. **Memory Efficiency** - Rope-based storage for 1M+ line documents
3. **Robustness** - Eliminate all visual artifacts (ghost characters, residue bugs)

---

## Phase 1: Buffer Synchronization Fix (Priority: Critical)

### Problem
The "ghost character" bug occurs because Fast Path writes directly to the terminal, bypassing the Buffer. The Renderer's `current_buffer` doesn't know about these writes, so diffing skips them.

### Solution: Shadow Tracking

```rust
struct Renderer {
    current_buffer: Buffer,
    shadow_mask: BitVec,  // Tracks cells "defiled" by RawOutput
}

impl Renderer {
    fn write_raw(&mut self, bytes: &[u8], dirty_region: Rect) {
        self.stdout.write_all(bytes)?;
        // Mark cells as "unknown" - force re-emit on next diff
        for y in dirty_region.y..dirty_region.y + dirty_region.height {
            for x in dirty_region.x..dirty_region.x + dirty_region.width {
                self.shadow_mask.set(y * width + x, true);
            }
        }
    }

    fn render_diff(&mut self, next: &Buffer) {
        for (i, (current, next)) in self.current_buffer.iter().zip(next.iter()).enumerate() {
            // Force re-emit if shadow mask is set
            if self.shadow_mask.get(i) || current != next {
                self.emit_cell(i, next);
                self.shadow_mask.set(i, false);
            }
        }
    }
}
```

### Tasks
- [ ] Add `shadow_mask: BitVec` to `Renderer`
- [ ] Modify `RenderCommand::RawOutput` to include dirty region
- [ ] `write_fast_path` must report affected cells
- [ ] `render_diff` checks shadow mask before skipping cells

### Estimated Effort: 2-3 hours

---

## Phase 2: Async-Friendly Ticker (Priority: High)

### Problem
`Engine::end_frame()` calls `std::thread::sleep()`, blocking the calling thread. This is incompatible with async runtimes like tokio.

### Current
```rust
pub fn end_frame(&mut self) {
    self.request_update();
    let elapsed = self.frame_start.elapsed();
    if elapsed < self.frame_duration {
        std::thread::sleep(self.frame_duration - elapsed);  // BLOCKING!
    }
}
```

### Solution: Decouple Timing from Engine

Option A: **Ticker Actor** (Recommended)
```rust
struct TickerActor {
    interval: Duration,
    tick_tx: Sender<()>,
}

impl TickerActor {
    fn run(self) {
        loop {
            std::thread::sleep(self.interval);
            if self.tick_tx.send(()).is_err() {
                break;
            }
        }
    }
}

// User code
loop {
    select! {
        recv(engine.input_receiver()) -> event => handle_input(event),
        recv(engine.tick_receiver()) -> _ => {
            generate_content();
            engine.request_update();
        }
    }
}
```

Option B: **User-Controlled Timing**
```rust
// Remove end_frame entirely. User manages their own timing.
loop {
    let deadline = Instant::now() + Duration::from_millis(16);
    
    // Process events until deadline
    while Instant::now() < deadline {
        match engine.input_receiver().recv_timeout(deadline - Instant::now()) {
            Ok(event) => handle(event),
            Err(Timeout) => break,
        }
    }
    
    // Render
    engine.request_update();
}
```

### Tasks
- [ ] Create `TickerActor` in `src/actor/ticker.rs`
- [ ] Add `tick_rx: Receiver<()>` to `Engine`
- [ ] Add `Engine::tick_receiver()` accessor
- [ ] Deprecate `end_frame()` (keep for backward compat)
- [ ] Update `streaming_demo.rs` to use `select!`

### Estimated Effort: 3-4 hours

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

| Phase | Priority | Effort | Dependencies |
|-------|----------|--------|--------------|
| 1. Buffer Sync Fix | Critical | 2-3h | None |
| 2. Async Ticker | High | 3-4h | None |
| 3. Rope Buffer | Medium | 4-6h | None |
| 4. Widget System | Medium | 6-8h | Phase 1 |
| 5. Docs & Polish | High | 2-3h | All |

**Total Estimated Effort: 17-24 hours**

---

## Out of Scope (V3+)

- WASM target for browser terminals
- Plugin system for custom widgets
- Multiplexer support (tmux pane detection)
- GPU acceleration (wgpu backend)
- Accessibility (screen reader support)
