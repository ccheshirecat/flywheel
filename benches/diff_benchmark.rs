//! Diffing engine benchmark: Measure buffer diff performance.
//!
//! Target: < 500µs for 200×50 buffer

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use flywheel::{Buffer, Cell, Rgb};
use flywheel::buffer::diff::{render_full_diff, render_full, DiffState};

/// Create a buffer with random-ish content for benchmarking.
fn create_test_buffer(width: u16, height: u16, seed: u8) -> Buffer {
    let mut buffer = Buffer::new(width, height);
    for y in 0..height {
        for x in 0..width {
            let c = ((x + y + seed as u16) % 26 + 65) as u8 as char; // A-Z
            let cell = Cell::new(c)
                .with_fg(Rgb::new(
                    ((x * 3 + seed as u16) % 256) as u8,
                    ((y * 7 + seed as u16) % 256) as u8,
                    ((x + y + seed as u16) % 256) as u8,
                ))
                .with_bg(Rgb::new(20, 20, 30));
            buffer.set(x, y, cell);
        }
    }
    buffer
}

fn diff_identical_buffers(c: &mut Criterion) {
    let buffer = create_test_buffer(200, 50, 0);
    let buffer_clone = buffer.clone();

    c.bench_function("diff_200x50_identical", |b| {
        b.iter(|| {
            let mut output = Vec::with_capacity(4096);
            let mut state = DiffState::new();
            render_full_diff(
                black_box(&buffer),
                black_box(&buffer_clone),
                &mut output,
                &mut state,
            )
        })
    });
}

fn diff_single_cell_change(c: &mut Criterion) {
    let buffer_a = create_test_buffer(200, 50, 0);
    let mut buffer_b = buffer_a.clone();
    // Change a single cell in the middle
    buffer_b.set(100, 25, Cell::new('X').with_fg(Rgb::new(255, 0, 0)));

    c.bench_function("diff_200x50_single_change", |b| {
        b.iter(|| {
            let mut output = Vec::with_capacity(4096);
            let mut state = DiffState::new();
            render_full_diff(
                black_box(&buffer_a),
                black_box(&buffer_b),
                &mut output,
                &mut state,
            )
        })
    });
}

fn diff_many_changes(c: &mut Criterion) {
    let buffer_a = create_test_buffer(200, 50, 0);
    let buffer_b = create_test_buffer(200, 50, 1); // Different seed = different content

    c.bench_function("diff_200x50_full_change", |b| {
        b.iter(|| {
            let mut output = Vec::with_capacity(65536);
            let mut state = DiffState::new();
            render_full_diff(
                black_box(&buffer_a),
                black_box(&buffer_b),
                &mut output,
                &mut state,
            )
        })
    });
}

fn diff_line_change(c: &mut Criterion) {
    let buffer_a = create_test_buffer(200, 50, 0);
    let mut buffer_b = buffer_a.clone();
    // Change one full line
    for x in 0..200 {
        buffer_b.set(x, 25, Cell::new('*').with_fg(Rgb::new(255, 255, 0)));
    }

    c.bench_function("diff_200x50_line_change", |b| {
        b.iter(|| {
            let mut output = Vec::with_capacity(4096);
            let mut state = DiffState::new();
            render_full_diff(
                black_box(&buffer_a),
                black_box(&buffer_b),
                &mut output,
                &mut state,
            )
        })
    });
}

fn full_render(c: &mut Criterion) {
    let buffer = create_test_buffer(200, 50, 0);

    c.bench_function("render_full_200x50", |b| {
        b.iter(|| {
            let mut output = Vec::with_capacity(65536);
            render_full(black_box(&buffer), &mut output)
        })
    });
}

fn diff_various_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("diff_by_size");
    
    for (width, height) in [(80, 24), (120, 40), (200, 50), (300, 80)] {
        let buffer_a = create_test_buffer(width, height, 0);
        let buffer_b = create_test_buffer(width, height, 1);
        
        group.bench_with_input(
            BenchmarkId::new("full_change", format!("{}x{}", width, height)),
            &(buffer_a, buffer_b),
            |b, (a, bb)| {
                b.iter(|| {
                    let mut output = Vec::with_capacity(65536);
                    let mut state = DiffState::new();
                    render_full_diff(black_box(a), black_box(bb), &mut output, &mut state)
                })
            },
        );
    }
    
    group.finish();
}

criterion_group!(
    benches,
    diff_identical_buffers,
    diff_single_cell_change,
    diff_many_changes,
    diff_line_change,
    full_render,
    diff_various_sizes,
);
criterion_main!(benches);
