//! Comparison benchmark: Flywheel vs Ratatui
//!
//! Fair comparison of equivalent operations between the two libraries.

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};

// Flywheel imports
use flywheel::{Buffer as FlywheelBuffer, Cell as FlywheelCell, Rgb as FlywheelRgb};
use flywheel::buffer::diff::{render_full_diff, DiffState};

// Ratatui imports
use ratatui::buffer::Buffer as RatatuiBuffer;
use ratatui::layout::Rect as RatatuiRect;
use ratatui::style::{Color as RatatuiColor, Style as RatatuiStyle};

/// Benchmark: Buffer creation
fn bench_buffer_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("buffer_creation");
    
    for (width, height) in [(80, 24), (200, 50)] {
        group.bench_with_input(
            BenchmarkId::new("flywheel", format!("{}x{}", width, height)),
            &(width, height),
            |b, &(w, h)| {
                b.iter(|| FlywheelBuffer::new(black_box(w), black_box(h)))
            },
        );
        
        group.bench_with_input(
            BenchmarkId::new("ratatui", format!("{}x{}", width, height)),
            &(width, height),
            |b, &(w, h)| {
                b.iter(|| RatatuiBuffer::empty(RatatuiRect::new(0, 0, black_box(w), black_box(h))))
            },
        );
    }
    
    group.finish();
}

/// Benchmark: Single cell write
fn bench_cell_write(c: &mut Criterion) {
    let mut group = c.benchmark_group("cell_write");
    
    // Flywheel
    let mut fw_buffer = FlywheelBuffer::new(200, 50);
    group.bench_function("flywheel", |b| {
        b.iter(|| {
            fw_buffer.set(
                black_box(100), 
                black_box(25), 
                FlywheelCell::new('X').with_fg(FlywheelRgb::new(255, 0, 0))
            )
        })
    });
    
    // Ratatui
    let mut rt_buffer = RatatuiBuffer::empty(RatatuiRect::new(0, 0, 200, 50));
    group.bench_function("ratatui", |b| {
        b.iter(|| {
            let cell = &mut rt_buffer[(black_box(100), black_box(25))];
            cell.set_char('X');
            cell.set_style(RatatuiStyle::default().fg(RatatuiColor::Rgb(255, 0, 0)));
        })
    });
    
    group.finish();
}

/// Benchmark: Fill entire buffer
fn bench_buffer_fill(c: &mut Criterion) {
    let mut group = c.benchmark_group("buffer_fill");
    
    for (width, height) in [(80, 24), (200, 50)] {
        // Flywheel
        let mut fw_buffer = FlywheelBuffer::new(width, height);
        let fw_cell = FlywheelCell::new('█').with_fg(FlywheelRgb::new(0, 255, 0));
        
        group.bench_with_input(
            BenchmarkId::new("flywheel", format!("{}x{}", width, height)),
            &(width, height),
            |b, &(w, h)| {
                b.iter(|| {
                    for y in 0..h {
                        for x in 0..w {
                            fw_buffer.set(x, y, fw_cell);
                        }
                    }
                })
            },
        );
        
        // Ratatui
        let mut rt_buffer = RatatuiBuffer::empty(RatatuiRect::new(0, 0, width, height));
        let rt_style = RatatuiStyle::default().fg(RatatuiColor::Rgb(0, 255, 0));
        
        group.bench_with_input(
            BenchmarkId::new("ratatui", format!("{}x{}", width, height)),
            &(width, height),
            |b, &(w, h)| {
                b.iter(|| {
                    for y in 0..h {
                        for x in 0..w {
                            let cell = &mut rt_buffer[(x, y)];
                            cell.set_char('█');
                            cell.set_style(rt_style);
                        }
                    }
                })
            },
        );
    }
    
    group.finish();
}

/// Benchmark: Buffer diff (comparing two buffers)
fn bench_buffer_diff(c: &mut Criterion) {
    let mut group = c.benchmark_group("buffer_diff");
    
    for (width, height) in [(80, 24), (200, 50)] {
        // Flywheel diff
        let mut fw_buf_a = FlywheelBuffer::new(width, height);
        let mut fw_buf_b = FlywheelBuffer::new(width, height);
        
        // Fill with different content
        for y in 0..height {
            for x in 0..width {
                let c = ((x + y) % 26 + 65) as u8 as char;
                fw_buf_a.set(x, y, FlywheelCell::new(c));
                let c2 = ((x + y + 1) % 26 + 65) as u8 as char;
                fw_buf_b.set(x, y, FlywheelCell::new(c2));
            }
        }
        
        group.bench_with_input(
            BenchmarkId::new("flywheel", format!("{}x{}", width, height)),
            &(width, height),
            |b, _| {
                b.iter(|| {
                    let mut output = Vec::with_capacity(65536);
                    let mut state = DiffState::new();
                    render_full_diff(
                        black_box(&fw_buf_a),
                        black_box(&fw_buf_b),
                        &mut output,
                        &mut state,
                    )
                })
            },
        );
        
        // Ratatui diff
        let mut rt_buf_a = RatatuiBuffer::empty(RatatuiRect::new(0, 0, width, height));
        let mut rt_buf_b = RatatuiBuffer::empty(RatatuiRect::new(0, 0, width, height));
        
        for y in 0..height {
            for x in 0..width {
                let c = ((x + y) % 26 + 65) as u8 as char;
                rt_buf_a[(x, y)].set_char(c);
                let c2 = ((x + y + 1) % 26 + 65) as u8 as char;
                rt_buf_b[(x, y)].set_char(c2);
            }
        }
        
        group.bench_with_input(
            BenchmarkId::new("ratatui", format!("{}x{}", width, height)),
            &(width, height),
            |b, _| {
                b.iter(|| {
                    // Ratatui's diff returns a Vec of updates
                    let updates = rt_buf_b.diff(black_box(&rt_buf_a));
                    black_box(updates)
                })
            },
        );
    }
    
    group.finish();
}

/// Benchmark: Cell clone/copy
fn bench_cell_clone(c: &mut Criterion) {
    let mut group = c.benchmark_group("cell_clone");
    
    // Flywheel cell (Copy)
    let fw_cell = FlywheelCell::new('X')
        .with_fg(FlywheelRgb::new(255, 128, 64))
        .with_bg(FlywheelRgb::new(32, 32, 32));
    
    group.bench_function("flywheel_copy", |b| {
        b.iter(|| {
            let _copy = black_box(fw_cell);
        })
    });
    
    // Ratatui cell (Clone - it has a String inside)
    let mut rt_cell = ratatui::buffer::Cell::default();
    rt_cell.set_char('X');
    rt_cell.set_style(RatatuiStyle::default()
        .fg(RatatuiColor::Rgb(255, 128, 64))
        .bg(RatatuiColor::Rgb(32, 32, 32)));
    
    group.bench_function("ratatui_clone", |b| {
        b.iter(|| {
            let _clone = black_box(rt_cell.clone());
        })
    });
    
    group.finish();
}

/// Benchmark: Text rendering (setting multiple cells)
fn bench_text_render(c: &mut Criterion) {
    let mut group = c.benchmark_group("text_render");
    
    let text = "Hello, World! This is a benchmark test string.";
    
    // Flywheel
    let mut fw_buffer = FlywheelBuffer::new(200, 50);
    let fg = FlywheelRgb::new(255, 255, 255);
    let bg = FlywheelRgb::new(0, 0, 0);
    
    group.bench_function("flywheel", |b| {
        b.iter(|| {
            for (i, ch) in text.chars().enumerate() {
                #[allow(clippy::cast_possible_truncation)]
                fw_buffer.set(i as u16, 0, FlywheelCell::new(ch).with_fg(fg).with_bg(bg));
            }
        })
    });
    
    // Ratatui
    let mut rt_buffer = RatatuiBuffer::empty(RatatuiRect::new(0, 0, 200, 50));
    let style = RatatuiStyle::default()
        .fg(RatatuiColor::Rgb(255, 255, 255))
        .bg(RatatuiColor::Rgb(0, 0, 0));
    
    group.bench_function("ratatui", |b| {
        b.iter(|| {
            for (i, ch) in text.chars().enumerate() {
                #[allow(clippy::cast_possible_truncation)]
                let cell = &mut rt_buffer[(i as u16, 0)];
                cell.set_char(ch);
                cell.set_style(style);
            }
        })
    });
    
    group.finish();
}

criterion_group!(
    benches,
    bench_buffer_creation,
    bench_cell_write,
    bench_buffer_fill,
    bench_buffer_diff,
    bench_cell_clone,
    bench_text_render,
);
criterion_main!(benches);
