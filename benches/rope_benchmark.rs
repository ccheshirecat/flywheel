//! RopeBuffer benchmark: Measure chunked storage performance.
//!
//! Target: < 1µs per append, efficient at scale

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use flywheel::{Cell, RopeBuffer, ChunkedLine, Rgb};

fn rope_append_single(c: &mut Criterion) {
    c.bench_function("rope_append_char", |b| {
        let mut buffer = RopeBuffer::new(10_000);
        b.iter(|| {
            buffer.append([Cell::new('x')].into_iter());
        });
    });
}

fn rope_newline(c: &mut Criterion) {
    c.bench_function("rope_newline", |b| {
        let mut buffer = RopeBuffer::new(100_000);
        b.iter(|| {
            buffer.newline();
        });
    });
}

fn rope_append_line(c: &mut Criterion) {
    let line: Vec<Cell> = (0..80).map(|i| {
        Cell::new(('A' as u8 + (i % 26) as u8) as char)
            .with_fg(Rgb::new(128, 200, 100))
    }).collect();
    
    c.bench_function("rope_append_80_cells", |b| {
        let mut buffer = RopeBuffer::new(10_000);
        b.iter(|| {
            buffer.append(black_box(line.iter().copied()));
        });
    });
}

fn rope_push_line(c: &mut Criterion) {
    let line = ChunkedLine::new(
        (0..80).map(|i| Cell::new(('A' as u8 + (i % 26) as u8) as char)).collect(),
        false,
    );
    
    c.bench_function("rope_push_line", |b| {
        let mut buffer = RopeBuffer::new(100_000);
        b.iter(|| {
            buffer.push_line(black_box(line.clone()));
        });
    });
}

fn rope_get_line(c: &mut Criterion) {
    let mut buffer = RopeBuffer::new(100_000);
    for _ in 0..50_000 {
        buffer.newline();
    }
    
    c.bench_function("rope_get_line_50k", |b| {
        b.iter(|| {
            buffer.get_line(black_box(25_000))
        });
    });
}

fn rope_visible_lines(c: &mut Criterion) {
    let mut buffer = RopeBuffer::new(100_000);
    for i in 0..10_000 {
        buffer.append([Cell::new(('A' as u8 + (i % 26) as u8) as char)].into_iter());
        buffer.newline();
    }
    
    c.bench_function("rope_visible_50_lines", |b| {
        b.iter(|| {
            let visible: Vec<_> = buffer.visible_lines(black_box(50)).collect();
            black_box(visible)
        });
    });
}

fn rope_scale_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("rope_scale");
    
    for line_count in [1_000, 10_000, 100_000] {
        group.bench_with_input(
            BenchmarkId::new("push_lines", line_count),
            &line_count,
            |b, &count| {
                b.iter(|| {
                    let mut buffer = RopeBuffer::unbounded();
                    for _ in 0..count {
                        buffer.newline();
                    }
                    black_box(buffer.len())
                });
            },
        );
    }
    
    group.finish();
}

fn rope_memory_stats(c: &mut Criterion) {
    let mut buffer = RopeBuffer::new(100_000);
    for _ in 0..50_000 {
        buffer.append((0..80).map(|_| Cell::new('x')));
        buffer.newline();
    }
    
    c.bench_function("rope_memory_stats_50k", |b| {
        b.iter(|| {
            black_box(buffer.memory_stats())
        });
    });
}

criterion_group!(
    benches,
    rope_append_single,
    rope_newline,
    rope_append_line,
    rope_push_line,
    rope_get_line,
    rope_visible_lines,
    rope_scale_comparison,
    rope_memory_stats,
);
criterion_main!(benches);
