# Flywheel Architecture

> A zero-flicker terminal compositor for Agentic CLIs

**Version:** 0.1.0  
**Last Updated:** 2026-01-29  
**Status:** Implementation Phase 1

---

## 1. Mission Statement

Modern Terminal User Interfaces for AI Agents suffer from a performance crisis. Frameworks like Ink (React) and BubbleTea (Elm) fail to handle high-frequency token streaming (100+ tokens/s) without flickering, high CPU usage, or input latency.

Flywheel rejects the "Web-to-Terminal" abstraction. We are building a purpose-built engine for Agentic CLIs.

---

## 2. Performance Targets

| Metric | Target | Rationale |
|--------|--------|-----------|
| Frame Time | < 1ms | 16ms is not the budget; 16ms is an eternity |
| Flicker | Zero | Updates must be atomic—no cleared screens, no half-drawn frames |
| Input Latency | Zero blocking | Input loop never waits for renderer or LLM |
| Token Throughput | 100+ tokens/s | Must handle streaming LLM output without degradation |

---

## 3. Architectural Axioms

### Axiom A: Retained Mode Memory, Immediate Mode Flush

We maintain screen state in two buffers (Current and Next). We do NOT redraw the whole screen every frame. We calculate the diff, generate a minimal ANSI sequence, and flush it in a **single `write()` syscall**.

```
┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│   Current   │    │    Next     │    │   Output    │
│   Buffer    │───▶│   Buffer    │───▶│   (ANSI)    │──▶ stdout
│  (visible)  │diff│  (staging)  │gen │  Vec<u8>    │
└─────────────┘    └─────────────┘    └─────────────┘
                                            │
                                            ▼
                                    Single write() syscall
```

### Axiom B: Dirty-Rect Optimization

For LLM streaming, we cannot diff the entire screen every frame. We support "Dirty Rectangles." If a widget updates, only its specific region is checked for diffs.

```rust
struct DirtyRegion {
    rect: Rect,
    generation: u64,  // Monotonic counter for invalidation tracking
}
```

### Axiom C: Actor Model (Thread Isolation)

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Input     │     │   Network   │     │   Render    │
│   Thread    │     │   Thread    │     │   Thread    │
│             │     │             │     │             │
│ crossterm   │     │ LLM/Agent   │     │ ONLY thread │
│ event poll  │     │ logic       │     │ touching    │
│             │     │             │     │ stdout      │
└──────┬──────┘     └──────┬──────┘     └──────┬──────┘
       │                   │                   │
       │    ┌──────────────┴───────────────┐   │
       └───▶│      crossbeam-channel       │◀──┘
            │      (lock-free MPSC)        │
            └──────────────────────────────┘
```

### Axiom D: Optimistic Append (The Agent Fast Path)

For streaming text, if a new token arrives and does NOT invalidate layout (no wrap, no scroll), we **bypass the diffing engine** and emit a direct cursor-write command.

```rust
fn can_fast_path_append(widget: &StreamWidget, token: &str) -> bool {
    let token_width = unicode_width::UnicodeWidthStr::width(token);
    let current_col = widget.cursor_col;
    let widget_width = widget.rect.width;
    
    current_col + token_width <= widget_width
        && !token.contains('\n')
        && widget.scroll_offset == widget.max_scroll
}
```

---

## 4. Memory Layout Decisions

### 4.1 Cell Structure (16 bytes)

```rust
#[repr(C)]
pub struct Cell {
    // Inline grapheme storage (4 bytes)
    // Covers ASCII, Latin-1, CJK, most Unicode
    grapheme: [u8; 4],
    
    // Grapheme metadata (2 bytes)
    grapheme_len: u8,      // Actual length of UTF-8 in grapheme[]
    display_width: u8,     // 0=continuation, 1=normal, 2=wide (CJK)
    
    // True color (6 bytes)
    fg: Rgb,               // [u8; 3]
    bg: Rgb,               // [u8; 3]
    
    // Modifiers (1 byte, bitflags)
    modifiers: Modifiers,  // Bold, Italic, Underline, etc.
    
    // Flags (1 byte)
    flags: CellFlags,      // Overflow indicator, dirty, etc.
}

// For complex graphemes (emoji ZWJ sequences), we use:
// flags.OVERFLOW = true
// grapheme[0..4] = index into overflow HashMap<u32, String>
```

**Total: 16 bytes** — Fits 2 cells per cache line (64 bytes).

### 4.2 Buffer Layout

```rust
pub struct Buffer {
    cells: Vec<Cell>,      // Contiguous, row-major order
    width: u16,
    height: u16,
    overflow: HashMap<u32, String>,  // Rare: complex grapheme spillover
}
```

For a 200×50 terminal: `200 × 50 × 16 = 160KB` — Fits entirely in L2 cache.

### 4.3 Color Representation

```rust
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}
```

True color (24-bit RGB) for precise branding. No palette indirection.

---

## 5. Layout System

### 5.1 Pre-Computed Static Regions

Layouts are computed **once** at initialization or terminal resize. No tree traversal at render time.

```rust
pub struct Region {
    pub id: RegionId,
    pub rect: Rect,
    pub z_index: u8,           // For overlays
    pub dirty_generation: u64,  // Invalidation tracking
}

pub struct Layout {
    pub regions: Vec<Region>,   // Flat list, no tree
    pub terminal_size: (u16, u16),
}
```

### 5.2 Example Layout

```
┌─────────────────────────────────────────┐
│  Region 0: Header (z=0, rarely dirty)   │
├──────────────┬──────────────────────────┤
│ Region 1:    │ Region 2:                │
│ Sidebar      │ Main Content             │
│ (z=0)        │ (z=0, frequently dirty)  │
├──────────────┴──────────────────────────┤
│  Region 3: Input Bar (z=1, overlay)     │
└─────────────────────────────────────────┘
```

---

## 6. Diffing Algorithm

```rust
pub fn render_diff(
    current: &Buffer,
    next: &Buffer,
    dirty_rects: &[Rect],
    output: &mut Vec<u8>,
) {
    let mut last_x: Option<u16> = None;
    let mut last_y: Option<u16> = None;
    let mut last_fg: Option<Rgb> = None;
    let mut last_bg: Option<Rgb> = None;
    
    for rect in dirty_rects {
        for y in rect.y..(rect.y + rect.height) {
            for x in rect.x..(rect.x + rect.width) {
                let idx = (y as usize) * (current.width as usize) + (x as usize);
                let current_cell = &current.cells[idx];
                let next_cell = &next.cells[idx];
                
                if current_cell != next_cell {
                    // Emit cursor move only if not adjacent
                    if last_y != Some(y) || last_x.map(|lx| lx + 1) != Some(x) {
                        write_cursor_move(output, x, y);
                    }
                    
                    // Emit color changes only if different from last
                    if last_fg != Some(next_cell.fg) {
                        write_fg_color(output, next_cell.fg);
                        last_fg = Some(next_cell.fg);
                    }
                    if last_bg != Some(next_cell.bg) {
                        write_bg_color(output, next_cell.bg);
                        last_bg = Some(next_cell.bg);
                    }
                    
                    // Write grapheme
                    write_grapheme(output, next_cell);
                    
                    last_x = Some(x);
                    last_y = Some(y);
                }
            }
        }
    }
}
```

---

## 7. Actor Messages

```rust
pub enum InputEvent {
    Key(KeyEvent),
    Mouse(MouseEvent),
    Resize(u16, u16),
    Paste(String),
}

pub enum RenderCommand {
    Tick,
    ForceRedraw,
    Shutdown,
}

pub enum AgentEvent {
    TokenReceived(String),
    StreamStart,
    StreamEnd,
    Error(String),
}
```

---

## 8. Tech Stack

| Component | Choice | Rationale |
|-----------|--------|-----------|
| Language | Rust | Memory safety, zero-cost abstractions |
| Terminal Backend | `crossterm` | Cross-platform, raw mode, event handling |
| Concurrency | `crossbeam-channel` | Lock-free MPSC queues |
| Text Handling | `unicode-width` | Correct grapheme display widths |
| Benchmarking | `criterion` | Statistical microbenchmarks |
| FFI | `#[no_mangle] extern "C"` | C bindings for cross-language use |

---

## 9. Target Platforms

| Platform | Terminals | Priority |
|----------|-----------|----------|
| macOS | iTerm2, Terminal.app, Kitty, Ghostty | P0 |
| Linux | Alacritty, Ghostty, GNOME Terminal | P0 |
| Windows | Windows Terminal, ConPTY | P1 |

---

## 10. Non-Goals (Explicit)

1. **Not a widget library** — We provide primitives, not pre-built components.
2. **Not a layout engine** — No flexbox, no CSS. Static regions only.
3. **Not a TUI framework** — No event routing, no state management.
4. **Not backward compatible with VT100** — We require a modern terminal with true color support.

---

## 11. Success Criteria

The PoC is complete when:

1. ✅ `smoke_test` binary runs with 0 input latency during simulated 100ms "LLM delay"
2. ✅ `streaming_demo` binary renders 100 tokens/s with 0 flicker
3. ✅ `cargo bench` shows frame render < 1ms for 200×50 buffer
4. ✅ Visual inspection in Docker confirms no flicker at max token rate
5. ✅ C FFI bindings compile and link from a test C program
