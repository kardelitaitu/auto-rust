//! Benchmarks for accessibility locator parsing
//!
//! Run with: `cargo bench --bench accessibility_locator --features accessibility-locator`

use auto::utils::accessibility_locator::parse_selector_input;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

fn benchmark_css_selectors(c: &mut Criterion) {
    let mut group = c.benchmark_group("css_selectors");

    let selectors = [
        "#button",
        ".class-name",
        "button[aria-label='Like']",
        "div.container > span[data-testid='tweet-text']",
        "article[role='article'][data-testid='tweet']:nth-child(2) > div > div > span",
    ];

    for selector in &selectors {
        group.bench_with_input(
            BenchmarkId::new("parse", selector.len()),
            selector,
            |b, sel| b.iter(|| parse_selector_input(black_box(sel))),
        );
    }

    group.finish();
}

fn benchmark_accessibility_locators(c: &mut Criterion) {
    let mut group = c.benchmark_group("accessibility_locators");

    let locators = [
        "role=button[name='Save']",
        "role=link[name='Profile'][scope='main']",
        "role=button[name='Follow'][match=contains]",
        "role=checkbox[name='Enable notifications'][scope='settings'][match=exact]",
    ];

    for locator in &locators {
        group.bench_with_input(
            BenchmarkId::new("parse", locator.len()),
            locator,
            |b, loc| b.iter(|| parse_selector_input(black_box(loc))),
        );
    }

    group.finish();
}

fn benchmark_complex_selectors(c: &mut Criterion) {
    let mut group = c.benchmark_group("complex_selectors");

    group.bench_function("deep_css", |b| {
        b.iter(|| {
            parse_selector_input(black_box(
                "html > body > main > article > div > section > button[aria-label='Submit']",
            ))
        })
    });

    group.bench_function("long_accessibility", |b| {
        b.iter(|| {
            parse_selector_input(black_box(
                "role=button[name='Click here to submit your application'][scope='main content'][match=contains]",
            ))
        })
    });

    group.bench_function("attribute_heavy_css", |b| {
        b.iter(|| {
            parse_selector_input(black_box(
                "[data-testid='button'][data-variant='primary'][data-size='large'][disabled]",
            ))
        })
    });

    group.finish();
}

fn benchmark_parse_error_cases(c: &mut Criterion) {
    let mut group = c.benchmark_group("error_cases");

    group.bench_function("empty_input", |b| {
        b.iter(|| parse_selector_input(black_box("   ")))
    });

    group.bench_function("missing_name", |b| {
        b.iter(|| parse_selector_input(black_box("role=button")))
    });

    group.bench_function("invalid_role", |b| {
        b.iter(|| parse_selector_input(black_box("role=Button[name='Save']")))
    });

    group.bench_function("malformed_segment", |b| {
        b.iter(|| parse_selector_input(black_box("role=button[name='Save'")))
    });

    group.finish();
}

fn benchmark_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("throughput");

    let selectors: Vec<&str> = vec![
        "#button",
        "role=button[name='Save']",
        ".class-name",
        "role=link[name='Profile'][scope='main']",
        "div[data-testid='container']",
    ];

    group.bench_function("batch_5_selectors", |b| {
        b.iter(|| {
            for s in &selectors {
                black_box(parse_selector_input(black_box(s)).unwrap());
            }
        })
    });

    group.bench_function("batch_10_selectors", |b| {
        b.iter(|| {
            let extended: Vec<&str> = selectors.iter().chain(selectors.iter()).copied().collect();
            for s in &extended {
                black_box(parse_selector_input(black_box(s)).unwrap());
            }
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_css_selectors,
    benchmark_accessibility_locators,
    benchmark_complex_selectors,
    benchmark_parse_error_cases,
    benchmark_throughput
);
criterion_main!(benches);
