//! Cell benchmark: Measure Cell comparison performance.
//!
//! Target: < 1ns per comparison

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use flywheel::{Cell, Modifiers, Rgb};

fn cell_equality_same(c: &mut Criterion) {
    let cell_a = Cell::new('A')
        .with_fg(Rgb::new(255, 128, 64))
        .with_bg(Rgb::new(32, 32, 32))
        .with_modifiers(Modifiers::BOLD);
    let cell_b = cell_a;

    c.bench_function("cell_eq_same", |b| {
        b.iter(|| black_box(&cell_a) == black_box(&cell_b))
    });
}

fn cell_equality_different_grapheme(c: &mut Criterion) {
    let cell_a = Cell::new('A');
    let cell_b = Cell::new('B');

    c.bench_function("cell_eq_diff_grapheme", |b| {
        b.iter(|| black_box(&cell_a) == black_box(&cell_b))
    });
}

fn cell_equality_different_color(c: &mut Criterion) {
    let cell_a = Cell::new('A').with_fg(Rgb::new(255, 0, 0));
    let cell_b = Cell::new('A').with_fg(Rgb::new(0, 255, 0));

    c.bench_function("cell_eq_diff_color", |b| {
        b.iter(|| black_box(&cell_a) == black_box(&cell_b))
    });
}

fn cell_from_char(c: &mut Criterion) {
    c.bench_function("cell_from_char_ascii", |b| {
        b.iter(|| Cell::from_char(black_box('A')))
    });

    c.bench_function("cell_from_char_cjk", |b| {
        b.iter(|| Cell::from_char(black_box('日')))
    });
}

criterion_group!(
    benches,
    cell_equality_same,
    cell_equality_different_grapheme,
    cell_equality_different_color,
    cell_from_char,
);
criterion_main!(benches);
