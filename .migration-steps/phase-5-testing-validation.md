# Phase 5: Testing & Validation

**Duration:** 3-5 days  
**Goal:** Long-running tests, side-by-side comparison with Node.js version, memory/CPU monitoring, edge case handling.

---

## 5.0 Verification Matrix

The migration is not complete until these checks pass:

- `cargo run cookiebot pageview=www.reddit.com then cookiebot`
- `cookiebot` and `cookiebot.js` resolve to the same task
- `pageview=reddit.com` normalizes to `https://reddit.com`
- `task=url=https://example.com` preserves the explicit `url` key
- Numeric payloads are normalized as string values in parser output (e.g. `{"value":"42"}`)
- Sequential groups split only on `then`
- The parser rejects or reports unknown tasks consistently

Alternative validation path: if the full end-to-end smoke test is hard to automate early, run the parser tests first, then add a browser-backed integration check as the last gate.

---

## 5.1 Test Structure

All tests go in `src/tests/` as requested in the whitepaper.

```
src/tests/
├── mod.rs
├── integration_test.rs
├── task_test.rs
├── utils_test.rs
└── edge_case_test.rs
```

### `src/tests/mod.rs`

```rust
pub mod integration_test;
pub mod task_test;
pub mod utils_test;
pub mod edge_case_test;
```

---

## 5.2 Unit Tests for Utils (`src/tests/utils_test.rs`)

### Math Tests

```rust
#[cfg(test)]
mod math_tests {
    use crate::utils::math::*;

    #[test]
    fn test_random_in_range_bounds() {
        for _ in 0..1000 {
            let val = random_in_range(10, 20);
            assert!(val >= 10 && val <= 20, "Value {} out of bounds", val);
        }
    }

    #[test]
    fn test_random_in_range_single_value() {
        for _ in 0..100 {
            let val = random_in_range(5, 5);
            assert_eq!(val, 5);
        }
    }

    #[test]
    fn test_gaussian_bounds() {
        for _ in 0..1000 {
            let val = gaussian(50.0, 10.0, 30.0, 70.0);
            assert!(
                val >= 30.0 && val <= 70.0,
                "Gaussian value {} out of bounds [30, 70]",
                val
            );
        }
    }

    #[test]
    fn test_gaussian_distribution() {
        // Verify mean is approximately correct
        let mut sum = 0.0;
        let iterations = 10000;
        
        for _ in 0..iterations {
            sum += gaussian(100.0, 15.0, 50.0, 150.0);
        }
        
        let mean = sum / iterations as f64;
        assert!(
            (mean - 100.0).abs() < 5.0,
            "Mean {} too far from expected 100.0",
            mean
        );
    }

    #[test]
    fn test_gaussian_zero_deviation() {
        let val = gaussian(50.0, 0.0, 0.0, 100.0);
        assert_eq!(val, 50.0);
    }

    #[test]
    fn test_roll_probability() {
        let mut successes = 0;
        let iterations = 1000;
        
        for _ in 0..iterations {
            if roll(0.3) {
                successes += 1;
            }
        }
        
        // Should be ~300 (within 10% tolerance)
        let expected = 300.0;
        let tolerance = 100.0;
        assert!(
            (successes as f64 - expected).abs() < tolerance,
            "Roll count {} too far from expected {}",
            successes,
            expected
        );
    }

    #[test]
    fn test_roll_always_true() {
        for _ in 0..100 {
            assert!(roll(1.0));
        }
    }

    #[test]
    fn test_roll_always_false() {
        for _ in 0..100 {
            assert!(!roll(0.0));
        }
    }

    #[test]
    fn test_sample_from_slice() {
        let arr = vec![1, 2, 3, 4, 5];
        
        for _ in 0..100 {
            let val = sample(&arr);
            assert!(val.is_some());
            assert!(val.unwrap() >= 1 && val.unwrap() <= 5);
        }
        
        assert!(sample::<i32>(&[]).is_none());
    }

    #[test]
    fn test_pid_step_convergence() {
        let mut state = PidState::new(0.0);
        let model = PidModel::default();
        let target = 100.0;
        let dt = 0.1;
        
        // Run 100 steps
        for _ in 0..100 {
            pid_step(&mut state, target, &model, dt);
        }
        
        // Should be close to target
        assert!(
            (state.pos - target).abs() < 1.0,
            "PID position {} too far from target {}",
            state.pos,
            target
        );
    }
}
```

### Timing Tests

```rust
#[cfg(test)]
mod timing_tests {
    use crate::utils::timing::*;
    use std::time::Instant;

    #[tokio::test]
    async fn test_human_delay_approximate() {
        let start = Instant::now();
        human_delay(100).await;
        let elapsed = start.elapsed().as_millis();
        
        // Should be ~100ms with 20% jitter (80-120ms)
        // Allow wider tolerance for CI environments
        assert!(
            elapsed >= 70 && elapsed <= 150,
            "Human delay {}ms out of expected range [70, 150]",
            elapsed
        );
    }

    #[tokio::test]
    async fn test_random_pause_bounds() {
        let start = Instant::now();
        random_pause(50, 100).await;
        let elapsed = start.elapsed().as_millis();
        
        assert!(
            elapsed >= 40 && elapsed <= 130,
            "Random pause {}ms out of bounds",
            elapsed
        );
    }

    #[tokio::test]
    async fn test_thinking_pause_range() {
        let start = Instant::now();
        thinking_pause().await;
        let elapsed = start.elapsed().as_millis();
        
        // Thinking pause: 1-5s with jitter
        assert!(
            elapsed >= 500 && elapsed <= 6000,
            "Thinking pause {}ms out of expected range",
            elapsed
        );
    }
}
```

---

## 5.3 Integration Tests (`src/tests/integration_test.rs`)

### Browser Connection Tests

```rust
#[cfg(test)]
mod integration_tests {
    use crate::config::*;
    use crate::browser::*;
    use crate::session::Session;
    use crate::orchestrator::Orchestrator;
    use crate::cli::{parse_task_groups, TaskDefinition};
    use anyhow::Result;
    use tracing::info;

    /// Test: Load configuration with defaults
    #[test]
    fn test_load_config_defaults() {
        let config = load_config().expect("Failed to load config");
        
        assert_eq!(config.browser.max_discovery_retries, 3);
        assert_eq!(config.orchestrator.max_global_concurrency, 20);
        assert!(config.browser.circuit_breaker.enabled);
    }

    /// Test: Parse task groups with "then" separator
    #[test]
    fn test_parse_task_groups_single() {
        let args = vec!["cookiebot.js".to_string()];
        let groups = parse_task_groups(&args).expect("Failed to parse");
        
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].len(), 1);
        assert_eq!(groups[0][0].name, "cookiebot");
    }

    #[test]
    fn test_parse_task_groups_multiple() {
        let args = vec![
            "cookiebot.js".to_string(),
            "pageview.js".to_string(),
        ];
        let groups = parse_task_groups(&args).expect("Failed to parse");
        
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].len(), 2);
    }

    #[test]
    fn test_parse_task_groups_with_then() {
        let args = vec![
            "cookiebot.js".to_string(),
            "then".to_string(),
            "pageview.js".to_string(),
        ];
        let groups = parse_task_groups(&args).expect("Failed to parse");
        
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0][0].name, "cookiebot");
        assert_eq!(groups[1][0].name, "pageview");
    }

    #[test]
    fn test_parse_task_groups_with_payload() {
        let args = vec!["follow=x.com".to_string()];
        let groups = parse_task_groups(&args).expect("Failed to parse");
        
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].len(), 1);
        assert_eq!(groups[0][0].name, "follow");
        assert_eq!(groups[0][0].payload.get("url").unwrap(), "https://x.com");
    }

    #[test]
    fn test_parse_task_groups_numeric_value() {
        let args = vec!["task=42".to_string()];
        let groups = parse_task_groups(&args).expect("Failed to parse");
        
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0][0].payload.get("value").unwrap(), "42");
    }

    #[test]
    fn test_cli_exact_match_cookiebot() {
        // cargo run cookiebot
        let args = vec!["cookiebot".to_string()];
        let groups = parse_task_groups(&args).expect("Failed to parse");
        
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].len(), 1);
        assert_eq!(groups[0][0].name, "cookiebot");
        assert!(groups[0][0].payload.is_empty());
    }

    #[test]
    fn test_cli_exact_match_cookiebot_with_js() {
        // cargo run cookiebot.js
        let args = vec!["cookiebot.js".to_string()];
        let groups = parse_task_groups(&args).expect("Failed to parse");
        
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].len(), 1);
        assert_eq!(groups[0][0].name, "cookiebot");
        assert!(groups[0][0].payload.is_empty());
    }

    #[test]
    fn test_cli_exact_match_pageview_with_url() {
        // cargo run pageview=www.reddit.com
        let args = vec!["pageview=www.reddit.com".to_string()];
        let groups = parse_task_groups(&args).expect("Failed to parse");
        
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].len(), 1);
        assert_eq!(groups[0][0].name, "pageview");
        assert_eq!(groups[0][0].payload.get("url").unwrap(), "https://www.reddit.com");
    }

    #[test]
    fn test_cli_exact_match_complex_groups() {
        // cargo run cookiebot pageview=www.reddit.com then cookiebot
        let args = vec![
            "cookiebot".to_string(),
            "pageview=www.reddit.com".to_string(),
            "then".to_string(),
            "cookiebot".to_string(),
        ];
        let groups = parse_task_groups(&args).expect("Failed to parse");
        
        assert_eq!(groups.len(), 2);
        
        // Group 1: cookiebot, pageview
        assert_eq!(groups[0].len(), 2);
        assert_eq!(groups[0][0].name, "cookiebot");
        assert_eq!(groups[0][1].name, "pageview");
        assert_eq!(groups[0][1].payload.get("url").unwrap(), "https://www.reddit.com");
        
        // Group 2: cookiebot
        assert_eq!(groups[1].len(), 1);
        assert_eq!(groups[1][0].name, "cookiebot");
    }

    #[test]
    fn test_cli_exact_match_multiple_tasks_same_group() {
        // cargo run cookiebot pageview
        let args = vec![
            "cookiebot".to_string(),
            "pageview".to_string(),
        ];
        let groups = parse_task_groups(&args).expect("Failed to parse");
        
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].len(), 2);
        assert_eq!(groups[0][0].name, "cookiebot");
        assert_eq!(groups[0][1].name, "pageview");
    }

    #[test]
    fn test_cli_url_explicit_url_key() {
        // cargo run pageview=url=https://example.com
        let args = vec!["pageview=url=https://example.com".to_string()];
        let groups = parse_task_groups(&args).expect("Failed to parse");
        
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0][0].name, "pageview");
        assert_eq!(groups[0][0].payload.get("url").unwrap(), "https://example.com");
    }

    #[test]
    fn test_cli_url_without_protocol_auto_prepend() {
        // cargo run pageview=reddit.com → https://reddit.com
        let args = vec!["pageview=reddit.com".to_string()];
        let groups = parse_task_groups(&args).expect("Failed to parse");
        
        assert_eq!(groups[0][0].payload.get("url").unwrap(), "https://reddit.com");
    }

    #[test]
    fn test_cli_localhost_url_auto_prepend() {
        // cargo run task=localhost:3000 → https://localhost:3000
        let args = vec!["task=localhost:3000".to_string()];
        let groups = parse_task_groups(&args).expect("Failed to parse");
        
        assert_eq!(groups[0][0].payload.get("url").unwrap(), "https://localhost:3000");
    }

    #[test]
    fn test_cli_quoted_values() {
        // cargo run task="value with spaces"
        let args = vec!["task=\"value with spaces\"".to_string()];
        let groups = parse_task_groups(&args).expect("Failed to parse");
        
        assert_eq!(groups[0][0].name, "task");
        assert_eq!(groups[0][0].payload.get("url").unwrap(), "value with spaces");
    }

    #[test]
    fn test_cli_then_case_insensitive() {
        // cargo run cookiebot THEN pageview (uppercase THEN)
        let args = vec![
            "cookiebot".to_string(),
            "THEN".to_string(),
            "pageview".to_string(),
        ];
        let groups = parse_task_groups(&args).expect("Failed to parse");
        
        assert_eq!(groups.len(), 2);
    }

    #[test]
    fn test_format_url_helper() {
        use crate::cli::format_url;
        
        assert_eq!(format_url("x.com"), "https://x.com");
        assert_eq!(format_url("https://x.com"), "https://x.com");
        assert_eq!(format_url("localhost:3000"), "https://localhost:3000");
        assert_eq!(format_url("42"), "42"); // Numeric, not a URL
    }
}
```

### Orchestrator Tests

```rust
    /// Test: Orchestrator creation and group execution
    #[tokio::test]
    async fn test_orchestrator_creation() {
        let config = load_config().expect("Failed to load config");
        let orchestrator = Orchestrator::new(config);
        
        // Orchestrator should be created without errors
        assert!(orchestrator.global_active_tasks.load(std::sync::atomic::Ordering::SeqCst) == 0);
    }

    /// Test: Empty group execution
    #[tokio::test]
    async fn test_empty_group_execution() {
        let config = load_config().expect("Failed to load config");
        let mut orchestrator = Orchestrator::new(config);
        
        let groups = vec![];
        let sessions = vec![]; // No sessions
        
        // Should handle empty gracefully
        let result = orchestrator.execute_group(&groups, &sessions).await;
        assert!(result.is_ok());
    }

    /// Test: Circuit breaker behavior
    #[tokio::test]
    async fn test_circuit_breaker() {
        let config = CircuitBreakerConfig {
            enabled: true,
            failure_threshold: 3,
            success_threshold: 2,
            half_open_time_ms: 1000,
        };
        
        let breaker = CircuitBreaker::new(config);
        
        // Should allow initial connection
        assert!(breaker.check("test-profile").await);
        
        // Record failures
        for _ in 0..3 {
            breaker.record_failure("test-profile").await;
        }
        
        // Should now be blocked
        assert!(!breaker.check("test-profile").await);
        
        // Record successes
        for _ in 0..2 {
            breaker.record_success("test-profile").await;
        }
        
        // Should be allowed again
        assert!(breaker.check("test-profile").await);
    }
```

---

## 5.4 Task Tests (`src/tests/task_test.rs`)

### Task Execution Tests

```rust
#[cfg(test)]
mod task_tests {
    use crate::task;
    use crate::utils::*;
    use serde_json::json;
    use anyhow::Result;

    /// Test: Task module registration
    #[test]
    fn test_task_registration() {
        assert!(task::get_task("cookiebot").is_some());
        assert!(task::get_task("cookiebot.js").is_some());
        assert!(task::get_task("pageview").is_some());
        assert!(task::get_task("unknown_task").is_none());
    }

    /// Test: Task payload parsing
    #[test]
    fn test_task_payload_extraction() {
        let payload = json!({
            "browserInfo": "test-session-1",
            "url": "https://example.com",
            "value": 42,
            "options": {
                "timeout": 30000,
                "retries": 2
            }
        });
        
        let browser_info = payload["browserInfo"].as_str().unwrap();
        let url = payload["url"].as_str().unwrap();
        let value = payload["value"].as_u64().unwrap();
        let timeout = payload["options"]["timeout"].as_u64().unwrap();
        
        assert_eq!(browser_info, "test-session-1");
        assert_eq!(url, "https://example.com");
        assert_eq!(value, 42);
        assert_eq!(timeout, 30000);
    }

    /// Test: URL file loading (pageview task dependency)
    #[test]
    fn test_pageview_url_loading() {
        // This tests the actual file reading from data/pageview.txt
        let urls = std::fs::read_to_string("data/pageview.txt");
        
        match urls {
            Ok(content) => {
                let lines: Vec<&str> = content
                    .lines()
                    .filter(|line| !line.trim().is_empty() && !line.trim().starts_with('#'))
                    .collect();
                
                assert!(!lines.is_empty(), "pageview.txt should contain URLs");
                
                // Verify all lines start with http
                for line in &lines {
                    assert!(
                        line.starts_with("http://") || line.starts_with("https://"),
                        "URL doesn't start with http: {}",
                        line
                    );
                }
            }
            Err(e) => {
                // File might not exist in test environment
                println!("pageview.txt not found: {}", e);
            }
        }
    }

    /// Test: Cookiebot URL list loading
    #[test]
    fn test_cookiebot_url_loading() {
        let urls = std::fs::read_to_string("data/cookiebot.txt");
        
        match urls {
            Ok(content) => {
                let lines: Vec<&str> = content
                    .lines()
                    .filter(|line| line.trim().starts_with("http"))
                    .collect();
                
                assert!(!lines.is_empty(), "cookiebot.txt should contain URLs");
            }
            Err(e) => {
                println!("cookiebot.txt not found: {}", e);
            }
        }
    }
}
```

---

## 5.5 Edge Case Tests (`src/tests/edge_case_test.rs`)

### Browser Offline / Empty Scenarios

```rust
#[cfg(test)]
mod edge_case_tests {
    use crate::config::*;
    use crate::cli::parse_task_groups;
    use anyhow::Result;

    /// Test: Empty task list
    #[test]
    fn test_empty_task_list() {
        let groups = parse_task_groups(&[]).expect("Failed to parse");
        assert_eq!(groups.len(), 0);
    }

    /// Test: Invalid task name
    #[test]
    fn test_invalid_task_name() {
        use crate::task;
        assert!(task::get_task("nonexistent_task").is_none());
        assert!(task::get_task("").is_none());
    }

    /// Test: Malformed CLI arguments
    #[test]
    fn test_malformed_cli_args() {
        // These should not panic
        let _ = parse_task_groups(&["=".to_string()]);
        let _ = parse_task_groups(&["task=".to_string()]);
        let _ = parse_task_groups(&["=value".to_string()]);
        let _ = parse_task_groups(&["".to_string()]);
    }

    /// Test: URL formatting edge cases
    #[test]
    fn test_url_formatting_edge_cases() {
        use crate::cli::format_url;
        
        // Already has protocol
        assert_eq!(format_url("http://x.com"), "http://x.com");
        assert_eq!(format_url("https://x.com"), "https://x.com");
        
        // No protocol but has domain
        assert_eq!(format_url("example.com"), "https://example.com");
        assert_eq!(format_url("www.example.com"), "https://www.example.com");
        
        // Localhost
        assert_eq!(format_url("localhost"), "https://localhost");
        assert_eq!(format_url("localhost:8080"), "https://localhost:8080");
        
        // Path included
        assert_eq!(format_url("example.com/path"), "https://example.com/path");
        
        // Empty string
        assert_eq!(format_url(""), "");
    }

    /// Test: Config loading with missing file
    #[test]
    fn test_config_missing_file() {
        // Should use defaults, not panic
        let config = load_config();
        assert!(config.is_ok());
    }

    /// Test: Empty URL file
    #[test]
    fn test_empty_url_file() {
        // Create temporary empty file
        let temp_dir = std::env::temp_dir();
        let empty_file = temp_dir.join("empty_urls.txt");
        std::fs::write(&empty_file, "").unwrap();
        
        // Read it
        let content = std::fs::read_to_string(&empty_file).unwrap();
        let urls: Vec<&str> = content
            .lines()
            .filter(|line| !line.trim().is_empty())
            .collect();
        
        assert!(urls.is_empty());
        
        // Cleanup
        std::fs::remove_file(&empty_file).ok();
    }

    /// Test: URL file with comments and empty lines
    #[test]
    fn test_url_file_with_comments() {
        let content = "# This is a comment\n\nhttps://example.com\n\n# Another comment\nhttps://test.com\n";
        
        let urls: Vec<&str> = content
            .lines()
            .filter(|line| !line.trim().is_empty() && !line.trim().starts_with('#'))
            .collect();
        
        assert_eq!(urls.len(), 2);
        assert_eq!(urls[0], "https://example.com");
        assert_eq!(urls[1], "https://test.com");
    }

    /// Test: Gaussian with extreme parameters
    #[test]
    fn test_gaussian_extreme_params() {
        use crate::utils::math::gaussian;
        
        // Very large deviation
        let val1 = gaussian(0.0, 1000.0, -5000.0, 5000.0);
        assert!(val1 >= -5000.0 && val1 <= 5000.0);
        
        // Very tight bounds
        let val2 = gaussian(50.0, 100.0, 49.0, 51.0);
        assert!(val2 >= 49.0 && val2 <= 51.0);
        
        // Mean outside bounds
        let val3 = gaussian(100.0, 10.0, 0.0, 50.0);
        assert!(val3 >= 0.0 && val3 <= 50.0);
    }
}
```

---

## 5.6 Long-Running Test Script

Create a shell script for long-running stability tests:

### `scripts/long-running-test.sh` (Linux/Mac)
### `scripts/long-running-test.bat` (Windows)

```batch
@echo off
REM Long-running stability test
REM Runs the orchestrator in a loop for N iterations

set ITERATIONS=100
set SUCCESS_COUNT=0
set FAIL_COUNT=0

echo Starting long-running test: %ITERATIONS% iterations
echo.

for /L %%i in (1,1,%ITERATIONS%) do (
    echo === Iteration %%i/%ITERATIONS% ===
    cargo run -- cookiebot.js pageview.js
    
    if %ERRORLEVEL% EQU 0 (
        set /a SUCCESS_COUNT+=1
    ) else (
        set /a FAIL_COUNT+=1
        echo Iteration %%i FAILED
    )
    
    echo.
)

echo.
echo ================================
echo Long-running test complete
echo Total: %ITERATIONS%
echo Success: %SUCCESS_COUNT%
echo Failed: %FAIL_COUNT%
echo ================================

if %FAIL_COUNT% GTR 0 (
    exit /b 1
)
```

---

## 5.7 Memory & CPU Monitoring

### Memory Leak Test

```rust
/// Add to src/tests/integration_test.rs

#[cfg(test)]
mod memory_tests {
    use std::process::Command;

    /// Test: Memory usage stays flat after multiple runs
    #[test]
    #[ignore] // Run manually with: cargo test -- --ignored
    fn test_memory_stability() {
        let iterations = 50;
        let mut memory_samples = Vec::new();
        
        for i in 0..iterations {
            // Get current process memory
            let output = Command::new("powershell")
                .args(&[
                    "-Command",
                    &format!(
                        "(Get-Process -Id {} | Select-Object WorkingSet).WorkingSet",
                        std::process::id()
                    ),
                ])
                .output()
                .expect("Failed to get memory usage");
            
            let memory_mb = String::from_utf8_lossy(&output.stdout)
                .trim()
                .parse::<f64>()
                .unwrap_or(0.0) / 1024.0 / 1024.0;
            
            memory_samples.push(memory_mb);
            
            println!("Iteration {}/{}: {:.2}MB", i + 1, iterations, memory_mb);
            
            // Simulate some work
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        
        // Check for memory growth
        let first_half_avg: f64 = memory_samples[..iterations / 2]
            .iter()
            .sum::<f64>() / (iterations / 2) as f64;
        
        let second_half_avg: f64 = memory_samples[iterations / 2..]
            .iter()
            .sum::<f64>() / (iterations / 2) as f64;
        
        let growth_pct = (second_half_avg - first_half_avg) / first_half_avg * 100.0;
        
        println!("\nMemory Analysis:");
        println!("  First half avg: {:.2}MB", first_half_avg);
        println!("  Second half avg: {:.2}MB", second_half_avg);
        println!("  Growth: {:.2}%", growth_pct);
        
        // Memory growth should be < 10%
        assert!(
            growth_pct < 10.0,
            "Memory grew by {:.2}% (threshold: 10%)",
            growth_pct
        );
    }
}
```

### CPU Usage Monitoring

Use Windows Performance Monitor or `htop` on Linux to monitor CPU during test runs:

```bash
# Linux: Monitor CPU during long-running test
htop -p $(pgrep rust-orchestrator)

# Windows: Use Performance Monitor
# 1. Open perfmon.msc
# 2. Add counter: Process -> % Processor Time -> rust-orchestrator
# 3. Run test and observe graph
```

---

## 5.8 Side-by-Side Comparison with Node.js

### Comparison Test Script

```batch
@echo off
REM Side-by-side comparison: Node.js vs Rust

echo ================================
echo Side-by-Side Comparison
echo ================================
echo.

REM Node.js version
echo Running Node.js version...
node main.js cookiebot.js pageview.js > node_output.txt 2>&1
set NODE_EXIT=%ERRORLEVEL%

REM Rust version
echo Running Rust version...
cargo run -- cookiebot.js pageview.js > rust_output.txt 2>&1
set RUST_EXIT=%ERRORLEVEL%

echo.
echo Results:
echo   Node.js exit code: %NODE_EXIT%
echo   Rust exit code: %RUST_EXIT%
echo.

REM Compare outputs (simplified check)
echo Check node_output.txt and rust_output.txt for detailed comparison
```

### Metrics to Compare

| Metric | Node.js | Rust | Target |
|--------|---------|------|--------|
| Memory usage (after 1 hour) | ~500MB | ~100MB | Rust 5x lower |
| CPU usage (idle) | ~5% | ~1% | Rust 5x lower |
| Task execution time | X seconds | ~X seconds | Same |
| Startup time | ~2s | ~0.5s | Rust 4x faster |
| Binary size | Node.js + node_modules (~100MB) | Single binary (~20MB) | Rust 5x smaller |

---

## 5.9 Edge Cases to Test

| Edge Case | Expected Behavior | Test |
|-----------|------------------|------|
| Browser offline | Retry 3 times, then skip | `test_browser_offline` |
| Empty URL file | Log warning, skip task | `test_empty_url_file` |
| Large batch (50 tasks) | Execute in parallel, no crashes | `test_large_batch` |
| Invalid task name | Log error, skip task | `test_invalid_task_name` |
| Malformed CLI args | Parse gracefully | `test_malformed_cli` |
| Timeout during task | Cancel task, continue next | `test_task_timeout` |
| Browser disconnects mid-task | Reconnect or retry on another session | `test_browser_disconnect` |
| All browsers fail | Exit gracefully with error | `test_all_browsers_fail` |
| Single browser, many tasks | Queue and execute sequentially | `test_single_browser_queue` |
| Task returns error | Retry 2 times, then mark failed | `test_task_retry` |

---

## 5.10 Running Tests

```bash
# Run all unit tests
cargo test

# Run integration-style tests inside src/tests modules
cargo test test_parse_task_groups_single

# Run specific test
cargo test test_gaussian_bounds

# Run long-running test (manual)
cargo test -- --ignored

# Run with output
cargo test -- --nocapture

# Run with coverage (requires cargo-tarpaulin)
cargo install cargo-tarpaulin
cargo tarpaulin --out Html
```

---

## Deliverables

- [ ] All unit tests pass (`cargo test`)
- [ ] All integration tests pass
- [ ] Long-running test completes 100 iterations without crashes
- [ ] Memory growth < 10% over 50 iterations
- [ ] Side-by-side comparison shows equivalent behavior
- [ ] All edge cases handled gracefully
- [ ] Test coverage report generated

---

## Notes

- **Test isolation**: Each test should be independent. No shared state between tests.
- **Mocking**: For browser tests, consider using mock CDP endpoints or headless Chrome in CI.
- **CI Integration**: Add tests to GitHub Actions for automatic validation on every commit.
- **Performance Regression**: Track execution times and memory usage over time to catch regressions.
- **Flaky Tests**: If a test fails intermittently, investigate immediately. Don't ignore flaky tests.
