# Flywheel Development Tracker

> Living document for tracking progress, decisions, and blockers.

**Last Updated:** 2026-01-29T12:34:00+07:00

---

## Current Phase: ✅ Project Complete

### Phase Overview

| Phase | Name | Status | Target Completion |
|-------|------|--------|-------------------|
| 1 | Core Primitives | ✅ Complete | 2026-01-29 |
| 2 | Diffing Engine | ✅ Complete | 2026-01-29 |
| 3 | Actor Model | ✅ Complete | 2026-01-29 |
| 4 | Streaming Widget | ✅ Complete | 2026-01-29 |
| 5 | C FFI & Polish | ✅ Complete | 2026-01-29 |

---

## Phase 1: Core Primitives ✅

**Goal:** Memory layout decisions locked in, zero allocations in hot path.

### Tasks

| ID | Task | Status | Notes |
|----|------|--------|-------|
| 1.1 | Project scaffolding (Cargo.toml, module structure) | ✅ | |
| 1.2 | `Rgb` color struct | ✅ | 3 bytes, Copy, Eq |
| 1.3 | `Modifiers` bitflags | ✅ | Bold, Italic, Underline, etc. |
| 1.4 | `Cell` struct with inline grapheme | ✅ | 16 bytes achieved |
| 1.5 | `Rect` primitive | ✅ | x, y, width, height |
| 1.6 | `Buffer` struct (contiguous cells) | ✅ | Row-major, overflow HashMap |
| 1.7 | `Region` and `Layout` structs | ✅ | Pre-computed static regions |
| 1.8 | Unit tests for `Cell` equality | ✅ | 25 tests passing |
| 1.9 | Clippy + rustfmt configuration | ✅ | Strict linting |
| 1.10 | Benchmark: Cell comparison | ✅ | See results below |

### Benchmark Results (2026-01-29)

| Benchmark | Time | Notes |
|-----------|------|-------|
| `cell_eq_diff_grapheme` | 666 ps | < 1ns ✅ (hot path) |
| `cell_eq_diff_color` | 937 ps | < 1ns ✅ |
| `cell_eq_same` | 2.17 ns | Full field comparison |
| `cell_from_char_ascii` | 1.73 ns | |
| `cell_from_char_cjk` | 2.58 ns | |

### Exit Criteria
- [x] `cargo test` passes (25 tests)
- [x] `cargo clippy` — warnings only (const fn suggestions)
- [x] `cargo bench` shows Cell::eq < 1ns for diff path
- [x] `std::mem::size_of::<Cell>() == 16`

**Git Commit:** `d5839eb` - feat: Phase 1 - Core primitives (Cell, Buffer, Layout)

---

## Phase 2: Diffing Engine ✅

**Goal:** Minimal ANSI output, single syscall.

### Tasks

| ID | Task | Status | Notes |
|----|------|--------|-------|
| 2.1 | `OutputBuffer` struct (pre-allocated Vec<u8>) | ✅ | Used directly in diff functions |
| 2.2 | ANSI escape sequence helpers | ✅ | emit_cursor_move, emit_fg_color, etc. |
| 2.3 | `render_diff()` function | ✅ | Current → Next diffing |
| 2.4 | Cursor movement optimization | ✅ | Skip if adjacent |
| 2.5 | Color change optimization | ✅ | Skip if same as last |
| 2.6 | Dirty-rect aware iteration | ✅ | Only diff changed regions |
| 2.7 | `render_full()` function | ✅ | Full buffer render for initial draw |
| 2.8 | Benchmark: Full buffer diff | ✅ | 283µs < 500µs target ✓ |

### Benchmark Results (2026-01-29)

| Benchmark | Time | Notes |
|-----------|------|-------|
| `diff_200x50_identical` | 26.7 µs | Fast skip path |
| `diff_200x50_single_change` | 27.2 µs | Minimal output |
| `diff_200x50_full_change` | 283 µs | < 500µs ✅ |
| `diff_200x50_line_change` | 27.2 µs | Line update |
| `render_full_200x50` | 270 µs | Initial draw |
| `diff_80x24` | 53 µs | Standard terminal |
| `diff_300x80` | 671 µs | Large terminal |

### Exit Criteria
- [x] `render_diff()` produces minimal ANSI output
- [x] Benchmark: 200×50 buffer diff < 500µs (achieved: 283µs)
- [x] 32 unit tests passing

**Git Commit:** `796a794` - feat: Phase 2 - Diffing engine with dirty-rect support

---

## Phase 3: Actor Model ✅

**Goal:** Non-blocking input, frame timing.

### Tasks

| ID | Task | Status | Notes |
|----|------|--------|-------|
| 3.1 | Message types (InputEvent, RenderCommand, AgentEvent) | ✅ | Full keyboard/mouse/resize support |
| 3.2 | Channel setup (crossbeam MPSC) | ✅ | Bounded channels (64 input, 16 render) |
| 3.3 | Input thread implementation | ✅ | InputActor with crossterm polling |
| 3.4 | Render thread implementation | ✅ | RendererActor with double buffering |
| 3.5 | Main loop coordinator (Engine) | ✅ | Terminal setup, actor spawning, API |
| 3.6 | `smoke_test` binary | ✅ | Interactive key echo demo |
| 3.7 | Frame timing | ✅ | 60 FPS target with sleep-based limiting |

### Components

- **`InputActor`**: Dedicated thread polling crossterm events, converts to `InputEvent`
- **`RendererActor`**: Owns double buffers, receives `RenderCommand`, performs diffing
- **`Engine`**: Entry point for applications, manages terminal state, coordinates actors
- **`messages.rs`**: `InputEvent`, `RenderCommand`, `AgentEvent`, `KeyCode`, `KeyModifiers`

### Exit Criteria
- [x] smoke_test runs with non-blocking input
- [x] Typing characters appears instantly
- [x] Frame counter updates at 60 FPS
- [x] 34 unit tests passing

**Git Commit:** `1399019` - feat: Phase 3 - Actor model with crossbeam channels

## Phase 4: Streaming Widget ✅

**Goal:** Optimistic append fast path.

### Tasks

| ID | Task | Status | Notes |
|----|------|--------|-------|
| 4.1 | `StreamWidget` struct | ✅ | Cursor tracking, content buffer |
| 4.2 | `can_fast_path()` check | ✅ | Width check, no newline, at bottom |
| 4.3 | Fast path: direct cursor-write | ✅ | Bypass diffing with write_fast_path() |
| 4.4 | Slow path: dirty-rect fallback | ✅ | Full re-render via append_slow_path() |
| 4.5 | Line wrapping detection | ✅ | Configurable word_wrap option |
| 4.6 | Scroll handling / ScrollBuffer | ✅ | Ring buffer with configurable max_scrollback |
| 4.7 | `streaming_demo` binary | ✅ | 100 tokens/s simulation |

### Components

- **`StreamWidget`**: Main widget with dual-path rendering
  - `append()`: Auto-selects fast or slow path
  - `write_fast_path()`: Direct ANSI output bypassing buffers
  - `render()`: Renders content to buffer for slow path
- **`ScrollBuffer`**: Ring buffer for scrollback history
  - O(1) append and scroll operations
  - Configurable capacity (default: 10,000 lines)
- **`AppendResult`**: Enum indicating which path was taken

### Exit Criteria
- [x] Fast path works for simple appends
- [x] Slow path correctly handles wraps/scrolls
- [x] streaming_demo runs at 100 tokens/s
- [x] 44 unit tests passing

**Git Commit:** `4946d98` - feat: Phase 4 - Streaming widget with optimistic append

---

## Phase 5: C FFI & Polish ✅

**Goal:** Cross-language bindings.

### Tasks

| ID | Task | Status | Notes |
|----|------|--------|-------|
| 5.1 | Define C API surface | ✅ | Opaque handles for Engine, Stream, Buffer |
| 5.2 | `#[unsafe(no_mangle)] extern "C"` exports | ✅ | Engine, Widget, and Utility functions |
| 5.3 | Header file generation | ✅ | `include/flywheel.h` created manually |
| 5.4 | Test C program linking | ✅ | Validated with GCC and test_ffi.c |
| 5.5 | Documentation (rustdoc) | ✅ | All public items documented |

### Output
- `libflywheel.dylib` / `.so` / `.a` (in target/debug or release)
- `include/flywheel.h` C header file

**Git Commit:** `ac77e30` - feat: Phase 5 - C FFI and Polish
| 5.6 | README with usage examples | ⬜ | |

### Exit Criteria
- [ ] C program compiles and links
- [ ] Basic operations work from C
- [ ] Documentation complete

---

## Decision Log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-01-29 | True color (24-bit RGB) over 256-color palette | Brand precision for commercial product |
| 2026-01-29 | Inline 4-byte grapheme + overflow HashMap | Optimize for 99% case, handle edge cases |
| 2026-01-29 | Static pre-computed layouts | Agentic CLIs have predictable layouts |
| 2026-01-29 | crossterm for terminal backend | Cross-platform abstraction |

---

## Blockers

*None currently.*

---

## Notes & Ideas

- Consider `termtosvg` for automated visual regression testing
- Investigate `io_uring` for async I/O on Linux (future optimization)
- Profile with `perf` / Instruments once we have the smoke test

---

## Git Checkpoint Strategy

Commit after completing:
- [ ] Phase 1: `feat: core primitives (Cell, Buffer, Layout)`
- [ ] Phase 2: `feat: diffing engine with dirty-rect support`
- [ ] Phase 3: `feat: actor model with crossbeam channels`
- [ ] Phase 4: `feat: streaming widget with optimistic append`
- [ ] Phase 5: `feat: C FFI bindings`
