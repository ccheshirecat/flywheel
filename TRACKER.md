# Flywheel Development Tracker

> Living document for tracking progress, decisions, and blockers.

**Last Updated:** 2026-01-29T12:34:00+07:00

---

## Current Phase: 2 — Diffing Engine

### Phase Overview

| Phase | Name | Status | Target Completion |
|-------|------|--------|-------------------|
| 1 | Core Primitives | ✅ Complete | 2026-01-29 |
| 2 | Diffing Engine | 🟡 In Progress | — |
| 3 | Actor Model | ⬜ Not Started | — |
| 4 | Streaming Widget | ⬜ Not Started | — |
| 5 | C FFI & Polish | ⬜ Not Started | — |

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

## Phase 2: Diffing Engine

**Goal:** Minimal ANSI output, single syscall.

### Tasks

| ID | Task | Status | Notes |
|----|------|--------|-------|
| 2.1 | `OutputBuffer` struct (pre-allocated Vec<u8>) | ⬜ | |
| 2.2 | ANSI escape sequence helpers | ⬜ | Cursor move, colors, etc. |
| 2.3 | `render_diff()` function | ⬜ | Current → Next diffing |
| 2.4 | Cursor movement optimization | ⬜ | Skip if adjacent |
| 2.5 | Color change optimization | ⬜ | Skip if same as last |
| 2.6 | Dirty-rect aware iteration | ⬜ | Only diff changed regions |
| 2.7 | Integration with crossterm | ⬜ | Raw mode, actual output |
| 2.8 | Benchmark: Full buffer diff | ⬜ | Target: < 500µs |

### Exit Criteria
- [ ] `render_diff()` produces minimal ANSI output
- [ ] Single `write()` syscall confirmed via strace/dtruss
- [ ] Benchmark: 200×50 buffer diff < 500µs

---

## Phase 3: Actor Model

**Goal:** Non-blocking input, frame timing.

### Tasks

| ID | Task | Status | Notes |
|----|------|--------|-------|
| 3.1 | Message types (InputEvent, RenderCommand, AgentEvent) | ⬜ | |
| 3.2 | Channel setup (crossbeam MPSC) | ⬜ | |
| 3.3 | Input thread implementation | ⬜ | crossterm event poll |
| 3.4 | Render thread implementation | ⬜ | Tick-based, reconcile state |
| 3.5 | Main loop coordinator | ⬜ | |
| 3.6 | `smoke_test` binary | ⬜ | Prove non-blocking input |
| 3.7 | FPS counter / debug logging | ⬜ | |

### Exit Criteria
- [ ] `smoke_test` runs without blocking on simulated 100ms delays
- [ ] Typing characters appears instantly
- [ ] FPS logged to debug output

---

## Phase 4: Streaming Widget

**Goal:** Optimistic append fast path.

### Tasks

| ID | Task | Status | Notes |
|----|------|--------|-------|
| 4.1 | `StreamWidget` struct | ⬜ | Cursor tracking, content buffer |
| 4.2 | `can_fast_path_append()` | ⬜ | Width check, no newline, no scroll |
| 4.3 | Fast path: direct cursor-write | ⬜ | Bypass diffing |
| 4.4 | Slow path: dirty-rect fallback | ⬜ | Full re-render of widget region |
| 4.5 | Line wrapping detection | ⬜ | |
| 4.6 | Scroll handling | ⬜ | |
| 4.7 | `streaming_demo` binary | ⬜ | 100 tokens/s simulation |

### Exit Criteria
- [ ] Fast path works for simple appends
- [ ] Slow path correctly handles wraps/scrolls
- [ ] 100 tokens/s with zero flicker (visual inspection)

---

## Phase 5: C FFI & Polish

**Goal:** Cross-language bindings.

### Tasks

| ID | Task | Status | Notes |
|----|------|--------|-------|
| 5.1 | Define C API surface | ⬜ | |
| 5.2 | `#[no_mangle] extern "C"` exports | ⬜ | |
| 5.3 | Header file generation (cbindgen) | ⬜ | |
| 5.4 | Test C program linking | ⬜ | |
| 5.5 | Documentation (rustdoc) | ⬜ | |
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
