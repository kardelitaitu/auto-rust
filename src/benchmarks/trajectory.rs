//! Benchmarks for mouse trajectory generation
//!
//! Run with: `cargo bench --bench trajectory`

use auto::utils::mouse::trajectory::{
    generate_arc_curve, generate_bezier_curve_with_config, generate_muscle_path, Point,
};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

fn benchmark_bezier_curves(c: &mut Criterion) {
    let mut group = c.benchmark_group("bezier_curve");
    let start = Point::new(100.0, 100.0);
    let end = Point::new(500.0, 400.0);

    for steps in [10_u32, 50, 100, 200] {
        group.bench_with_input(BenchmarkId::new("steps", steps), &steps, |b, &steps| {
            b.iter(|| {
                black_box(generate_bezier_curve_with_config(
                    black_box(&start),
                    black_box(&end),
                    black_box(30.0),
                    Some(black_box(steps)),
                ))
            })
        });
    }

    group.finish();
}

fn benchmark_arc_curves(c: &mut Criterion) {
    let mut group = c.benchmark_group("arc_curve");

    let cases = [
        (
            "short_distance",
            Point::new(100.0, 100.0),
            Point::new(200.0, 150.0),
        ),
        (
            "medium_distance",
            Point::new(100.0, 100.0),
            Point::new(500.0, 400.0),
        ),
        (
            "long_distance",
            Point::new(100.0, 100.0),
            Point::new(1000.0, 800.0),
        ),
    ];

    for (label, start, end) in cases {
        group.bench_function(label, |b| {
            b.iter(|| black_box(generate_arc_curve(black_box(&start), black_box(&end))))
        });
    }

    group.finish();
}

fn benchmark_muscle_paths(c: &mut Criterion) {
    let mut group = c.benchmark_group("muscle_path");

    let cases = [
        (
            "short_distance",
            Point::new(100.0, 100.0),
            Point::new(200.0, 150.0),
        ),
        (
            "medium_distance",
            Point::new(100.0, 100.0),
            Point::new(500.0, 400.0),
        ),
        (
            "long_distance",
            Point::new(100.0, 100.0),
            Point::new(1000.0, 800.0),
        ),
    ];

    for (label, start, end) in cases {
        group.bench_function(label, |b| {
            b.iter(|| black_box(generate_muscle_path(black_box(&start), black_box(&end))))
        });
    }

    group.finish();
}

fn benchmark_trajectory_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("trajectory_comparison");
    let start = Point::new(100.0, 100.0);
    let end = Point::new(500.0, 400.0);

    group.bench_function("bezier", |b| {
        b.iter(|| {
            black_box(generate_bezier_curve_with_config(
                black_box(&start),
                black_box(&end),
                black_box(30.0),
                Some(black_box(50)),
            ))
        })
    });

    group.bench_function("arc", |b| {
        b.iter(|| black_box(generate_arc_curve(black_box(&start), black_box(&end))))
    });

    group.bench_function("muscle", |b| {
        b.iter(|| black_box(generate_muscle_path(black_box(&start), black_box(&end))))
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_bezier_curves,
    benchmark_arc_curves,
    benchmark_muscle_paths,
    benchmark_trajectory_comparison
);
criterion_main!(benches);
