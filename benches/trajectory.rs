//! Benchmarks for mouse trajectory generation
//!
//! Run with: `cargo bench --bench trajectory`

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};

/// Simple Point struct for benchmarking
#[derive(Debug, Clone, Copy)]
struct Point {
    x: f64,
    y: f64,
}

impl Point {
    fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

/// Generate Bezier curve (simplified for benchmark)
fn generate_bezier_curve(start: &Point, end: &Point, spread: f64, steps: u32) -> Vec<Point> {
    let mut points = Vec::new();
    let cp1 = Point::new(
        (start.x + end.x) / 2.0 + spread,
        (start.y + end.y) / 2.0 + spread,
    );
    let cp2 = Point::new(
        (start.x + end.x) / 2.0 - spread,
        (start.y + end.y) / 2.0 - spread,
    );

    for i in 0..=steps {
        let t = i as f64 / steps as f64;
        let x = (1.0 - t).powi(3) * start.x
            + 3.0 * (1.0 - t).powi(2) * t * cp1.x
            + 3.0 * (1.0 - t) * t.powi(2) * cp2.x
            + t.powi(3) * end.x;
        let y = (1.0 - t).powi(3) * start.y
            + 3.0 * (1.0 - t).powi(2) * t * cp1.y
            + 3.0 * (1.0 - t) * t.powi(2) * cp2.y
            + t.powi(3) * end.y;
        points.push(Point::new(x, y));
    }
    points
}

/// Generate arc curve (simplified for benchmark)
fn generate_arc_curve(start: &Point, end: &Point, curvature: f64, steps: u32) -> Vec<Point> {
    let mut points = Vec::new();
    let mid_x = (start.x + end.x) / 2.0;
    let mid_y = (start.y + end.y) / 2.0;
    let offset_x = (end.y - start.y) * curvature;
    let offset_y = (start.x - end.x) * curvature;
    let control = Point::new(mid_x + offset_x, mid_y + offset_y);

    for i in 0..=steps {
        let t = i as f64 / steps as f64;
        let one_minus_t = 1.0 - t;
        let x = one_minus_t * one_minus_t * start.x
            + 2.0 * one_minus_t * t * control.x
            + t * t * end.x;
        let y = one_minus_t * one_minus_t * start.y
            + 2.0 * one_minus_t * t * control.y
            + t * t * end.y;
        points.push(Point::new(x, y));
    }
    points
}

/// Generate muscle path with jitter simulation
fn generate_muscle_path(start: &Point, end: &Point, steps: u32) -> Vec<Point> {
    let mut points = Vec::new();
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let distance = (dx * dx + dy * dy).sqrt();
    
    for i in 0..=steps {
        let t = i as f64 / steps as f64;
        let base_x = start.x + dx * t;
        let base_y = start.y + dy * t;
        
        // Add muscle jitter based on distance remaining
        let jitter = (distance * (1.0 - t) * 0.01).sin() * 2.0;
        let x = base_x + jitter * (1.0 - t);
        let y = base_y + jitter * t;
        
        points.push(Point::new(x, y));
    }
    points
}

fn benchmark_bezier_curves(c: &mut Criterion) {
    let mut group = c.benchmark_group("bezier_curve");
    let start = Point::new(100.0, 100.0);
    let end = Point::new(500.0, 400.0);

    for steps in [10, 50, 100, 200].iter() {
        group.bench_with_input(
            BenchmarkId::new("steps", steps),
            steps,
            |b, &steps| {
                b.iter(|| {
                    generate_bezier_curve(
                        black_box(&start),
                        black_box(&end),
                        black_box(30.0),
                        black_box(steps),
                    )
                })
            },
        );
    }
    group.finish();
}

fn benchmark_arc_curves(c: &mut Criterion) {
    let mut group = c.benchmark_group("arc_curve");
    let start = Point::new(100.0, 100.0);
    let end = Point::new(500.0, 400.0);

    for curvature in [0.1, 0.3, 0.5, 0.8].iter() {
        group.bench_with_input(
            BenchmarkId::new("curvature", curvature),
            curvature,
            |b, &curvature| {
                b.iter(|| {
                    generate_arc_curve(
                        black_box(&start),
                        black_box(&end),
                        black_box(curvature),
                        black_box(50),
                    )
                })
            },
        );
    }
    group.finish();
}

fn benchmark_muscle_paths(c: &mut Criterion) {
    let mut group = c.benchmark_group("muscle_path");
    
    // Short distance
    let short_start = Point::new(100.0, 100.0);
    let short_end = Point::new(200.0, 150.0);
    
    // Medium distance
    let medium_start = Point::new(100.0, 100.0);
    let medium_end = Point::new(500.0, 400.0);
    
    // Long distance
    let long_start = Point::new(100.0, 100.0);
    let long_end = Point::new(1000.0, 800.0);

    group.bench_function("short_distance", |b| {
        b.iter(|| {
            generate_muscle_path(
                black_box(&short_start),
                black_box(&short_end),
                black_box(20),
            )
        })
    });

    group.bench_function("medium_distance", |b| {
        b.iter(|| {
            generate_muscle_path(
                black_box(&medium_start),
                black_box(&medium_end),
                black_box(50),
            )
        })
    });

    group.bench_function("long_distance", |b| {
        b.iter(|| {
            generate_muscle_path(
                black_box(&long_start),
                black_box(&long_end),
                black_box(100),
            )
        })
    });
    
    group.finish();
}

fn benchmark_trajectory_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("trajectory_comparison");
    let start = Point::new(100.0, 100.0);
    let end = Point::new(500.0, 400.0);

    group.bench_function("bezier_50_steps", |b| {
        b.iter(|| generate_bezier_curve(black_box(&start), black_box(&end), black_box(30.0), black_box(50)))
    });

    group.bench_function("arc_50_steps", |b| {
        b.iter(|| generate_arc_curve(black_box(&start), black_box(&end), black_box(0.3), black_box(50)))
    });

    group.bench_function("muscle_50_steps", |b| {
        b.iter(|| generate_muscle_path(black_box(&start), black_box(&end), black_box(50)))
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
