//! Benchmarks for accessibility locator parsing
//!
//! Run with: `cargo bench --bench accessibility_locator`

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};

/// Parsed selector types (simplified for benchmark)
#[derive(Debug, Clone, PartialEq)]
enum ParsedSelector {
    Css(String),
    Accessibility(AccessibilityLocator),
}

/// Accessibility locator structure
#[derive(Debug, Clone, PartialEq)]
struct AccessibilityLocator {
    role: String,
    name: Option<String>,
    scope: Option<String>,
    match_mode: MatchMode,
}

#[derive(Debug, Clone, PartialEq)]
enum MatchMode {
    Exact,
    Contains,
}

/// Parse errors
#[derive(Debug, PartialEq)]
enum LocatorParseError {
    EmptyInput,
    InvalidRole,
    MissingRequiredField(&'static str),
    MalformedSegment(String),
}

/// Parse selector input (simplified implementation for benchmarking)
fn parse_selector_input(input: &str) -> Result<ParsedSelector, LocatorParseError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(LocatorParseError::EmptyInput);
    }

    // Check for role= prefix (simplified parsing)
    if trimmed.starts_with("role=") {
        let rest = &trimmed[5..];
        // Find role name (until [ or end)
        let role_end = rest.find('[').unwrap_or(rest.len());
        let role = &rest[..role_end];
        
        if role.is_empty() || role.chars().next().unwrap().is_uppercase() {
            return Err(LocatorParseError::InvalidRole);
        }

        let mut locator = AccessibilityLocator {
            role: role.to_string(),
            name: None,
            scope: None,
            match_mode: MatchMode::Exact,
        };

        // Parse segments like [name='value']
        let mut remaining = &rest[role_end..];
        while remaining.starts_with('[') {
            let close_idx = remaining.find(']').ok_or_else(|| {
                LocatorParseError::MalformedSegment(remaining.to_string())
            })?;
            let segment = &remaining[1..close_idx];
            
            if let Some(eq_idx) = segment.find('=') {
                let key = &segment[..eq_idx];
                let value = &segment[eq_idx + 2..segment.len() - 1]; // Strip quotes
                
                match key {
                    "name" => locator.name = Some(value.to_string()),
                    "scope" => locator.scope = Some(value.to_string()),
                    "match" => locator.match_mode = match value {
                        "exact" => MatchMode::Exact,
                        "contains" => MatchMode::Contains,
                        _ => MatchMode::Exact,
                    },
                    _ => {}
                }
            }
            
            remaining = &remaining[close_idx + 1..];
        }

        if locator.name.is_none() {
            return Err(LocatorParseError::MissingRequiredField("name"));
        }

        return Ok(ParsedSelector::Accessibility(locator));
    }

    // Default to CSS selector
    Ok(ParsedSelector::Css(trimmed.to_string()))
}

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
            |b, sel| {
                b.iter(|| parse_selector_input(black_box(sel)))
            },
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
            |b, loc| {
                b.iter(|| parse_selector_input(black_box(loc)))
            },
        );
    }
    
    group.finish();
}

fn benchmark_complex_selectors(c: &mut Criterion) {
    let mut group = c.benchmark_group("complex_selectors");

    // Deeply nested CSS
    group.bench_function("deep_css", |b| {
        b.iter(|| {
            parse_selector_input(black_box(
                "html > body > main > article > div > section > button[aria-label='Submit']"
            ))
        })
    });

    // Long accessibility locator with multiple modifiers
    group.bench_function("long_accessibility", |b| {
        b.iter(|| {
            parse_selector_input(black_box(
                "role=button[name='Click here to submit your application'][scope='main content'][match=contains]"
            ))
        })
    });

    // Mixed selectors
    group.bench_function("attribute_heavy_css", |b| {
        b.iter(|| {
            parse_selector_input(black_box(
                "[data-testid='button'][data-variant='primary'][data-size='large'][disabled]"
            ))
        })
    });

    group.finish();
}

fn benchmark_parse_error_cases(c: &mut Criterion) {
    let mut group = c.benchmark_group("error_cases");

    // Empty input (fast fail)
    group.bench_function("empty_input", |b| {
        b.iter(|| parse_selector_input(black_box("   ")))
    });

    // Missing name (validation error)
    group.bench_function("missing_name", |b| {
        b.iter(|| parse_selector_input(black_box("role=button")))
    });

    // Invalid role (validation error)
    group.bench_function("invalid_role", |b| {
        b.iter(|| parse_selector_input(black_box("role=Button[name='Save']")))
    });

    // Malformed segment (parse error)
    group.bench_function("malformed_segment", |b| {
        b.iter(|| parse_selector_input(black_box("role=button[name='Save'")))
    });

    group.finish();
}

fn benchmark_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("throughput");
    
    // Measure batch parsing throughput
    let selectors: Vec<&str> = vec![
        "#button",
        "role=button[name='Save']",
        ".class-name",
        "role=link[name='Profile'][scope='main']",
        "div[data-testid='container']",
    ];

    group.bench_function("batch_5_selectors", |b| {
        b.iter(|| {
            selectors.iter().map(|s| parse_selector_input(black_box(s)).unwrap()).count()
        })
    });

    group.bench_function("batch_10_selectors", |b| {
        b.iter(|| {
            let extended: Vec<&str> = selectors.iter()
                .chain(selectors.iter())
                .copied()
                .collect();
            extended.iter().map(|s| parse_selector_input(black_box(s)).unwrap()).count()
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
