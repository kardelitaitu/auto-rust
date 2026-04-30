//! Tests for browser port configuration via environment variables.
//!
//! Tests cover:
//! - Default port ranges (when env vars are unset)
//! - Custom port ranges via BRAVE_PORT_START, BRAVE_PORT_END, etc.
//! - Validation: START < END
//! - Validation: ports must be in valid range (1024-65535)
//! - Edge cases: single port, invalid values, empty strings

use std::env;
use std::sync::Mutex;

// Mutex to ensure tests run sequentially (env vars are process-wide)
static ENV_MUTEX: Mutex<()> = Mutex::new(());

// Constants matching browser.rs defaults
const DEFAULT_BRAVE_PORT_START: u16 = 9001;
const DEFAULT_BRAVE_PORT_END: u16 = 9050;
const DEFAULT_CHROME_PORT_START: u16 = 9222;
const DEFAULT_CHROME_PORT_END: u16 = 9230;
const MIN_PORT: u16 = 1024;
const MAX_PORT: u16 = 65535;

/// Parse port range from environment variables with validation.
fn parse_port_range(
    start_var: &str,
    end_var: &str,
    default_start: u16,
    default_end: u16,
) -> (u16, u16) {
    let mut start = parse_port_env(start_var, default_start);
    let mut end = parse_port_env(end_var, default_end);

    // Validate: START must be <= END (swap if needed)
    if start > end {
        std::mem::swap(&mut start, &mut end);
    }

    // Validate: ports must be in valid range (1024-65535)
    let clamped_start = start.clamp(MIN_PORT, MAX_PORT);
    let clamped_end = end.clamp(MIN_PORT, MAX_PORT);

    (clamped_start, clamped_end)
}

/// Parse a single port from environment variable.
fn parse_port_env(var_name: &str, default: u16) -> u16 {
    match env::var(var_name) {
        Ok(val) => match val.parse::<u16>() {
            Ok(port) => port,
            Err(_) => default,
        },
        Err(_) => default,
    }
}

/// Get Brave browser port range from environment or defaults.
fn get_brave_port_range() -> (u16, u16) {
    parse_port_range(
        "BRAVE_PORT_START",
        "BRAVE_PORT_END",
        DEFAULT_BRAVE_PORT_START,
        DEFAULT_BRAVE_PORT_END,
    )
}

/// Get Chrome browser port range from environment or defaults.
fn get_chrome_port_range() -> (u16, u16) {
    parse_port_range(
        "CHROME_PORT_START",
        "CHROME_PORT_END",
        DEFAULT_CHROME_PORT_START,
        DEFAULT_CHROME_PORT_END,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn _clear_port_env_vars() {
        env::remove_var("BRAVE_PORT_START");
        env::remove_var("BRAVE_PORT_END");
        env::remove_var("CHROME_PORT_START");
        env::remove_var("CHROME_PORT_END");
    }

    fn setup_test_env() -> std::sync::MutexGuard<'static, ()> {
        // Handle poisoned mutex - if a previous test panicked, recover and clean up
        let guard = match ENV_MUTEX.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                // Mutex was poisoned by a panic in another test, but we can still use it
                poisoned.into_inner()
            }
        };
        _clear_port_env_vars();
        guard
    }

    // ============================================================================
    // Default Port Range Tests
    // ============================================================================

    #[test]
    fn test_default_brave_port_range() {
        let _guard = setup_test_env();
        let (start, end) = get_brave_port_range();
        assert_eq!(start, 9001);
        assert_eq!(end, 9050);
        assert!(start < end, "START should be less than END");
    }

    #[test]
    fn test_default_chrome_port_range() {
        let _guard = setup_test_env();
        let (start, end) = get_chrome_port_range();
        assert_eq!(start, 9222);
        assert_eq!(end, 9230);
        assert!(start < end, "START should be less than END");
    }

    // ============================================================================
    // Custom Port Range Tests
    // ============================================================================

    #[test]
    fn test_custom_brave_port_range() {
        let _guard = setup_test_env();
        env::set_var("BRAVE_PORT_START", "9100");
        env::set_var("BRAVE_PORT_END", "9150");

        let (start, end) = get_brave_port_range();
        assert_eq!(start, 9100);
        assert_eq!(end, 9150);
        assert!(start < end);
    }

    #[test]
    fn test_custom_chrome_port_range() {
        let _guard = setup_test_env();
        env::set_var("CHROME_PORT_START", "9300");
        env::set_var("CHROME_PORT_END", "9350");

        let (start, end) = get_chrome_port_range();
        assert_eq!(start, 9300);
        assert_eq!(end, 9350);
        assert!(start < end);
    }

    #[test]
    fn test_single_port_range() {
        let _guard = setup_test_env();
        // When START == END, should return same port for both
        env::set_var("BRAVE_PORT_START", "9005");
        env::set_var("BRAVE_PORT_END", "9005");

        let (start, end) = get_brave_port_range();
        assert_eq!(start, 9005);
        assert_eq!(end, 9005);
        assert_eq!(start, end, "Single port range should have START == END");
    }

    // ============================================================================
    // Validation: START < END (Auto-swap)
    // ============================================================================

    #[test]
    fn test_swapped_ports_get_corrected() {
        let _guard = setup_test_env();
        // If START > END, they should be swapped
        env::set_var("BRAVE_PORT_START", "9050");
        env::set_var("BRAVE_PORT_END", "9001");

        let (start, end) = get_brave_port_range();
        assert_eq!(start, 9001, "Should be the smaller value");
        assert_eq!(end, 9050, "Should be the larger value");
        assert!(start <= end, "After swap, START should be <= END");
    }

    #[test]
    fn test_partial_overlap_swapped() {
        let _guard = setup_test_env();
        env::set_var("CHROME_PORT_START", "9250");
        env::set_var("CHROME_PORT_END", "9220");

        let (start, end) = get_chrome_port_range();
        assert_eq!(start, 9220);
        assert_eq!(end, 9250);
        assert!(start < end);
    }

    // ============================================================================
    // Validation: Port Range Bounds (1024-65535)
    // ============================================================================

    #[test]
    fn test_ports_below_min_get_clamped() {
        let _guard = setup_test_env();
        // Ports below 1024 should be clamped to 1024
        env::set_var("BRAVE_PORT_START", "100");
        env::set_var("BRAVE_PORT_END", "500");

        let (start, end) = get_brave_port_range();
        assert_eq!(start, 1024, "Should clamp to MIN_PORT");
        assert_eq!(end, 1024, "Should clamp to MIN_PORT");
    }

    #[test]
    fn test_ports_above_max_get_clamped() {
        let _guard = setup_test_env();
        // Ports above 65535 (u16 max) can't even be parsed, so this tests
        // the boundary at exactly 65535
        env::set_var("CHROME_PORT_START", "65530");
        env::set_var("CHROME_PORT_END", "65535");

        let (start, end) = get_chrome_port_range();
        assert_eq!(start, 65530);
        assert_eq!(end, 65535);
    }

    #[test]
    fn test_start_below_min_gets_clamped() {
        let _guard = setup_test_env();
        env::set_var("BRAVE_PORT_START", "500");
        env::set_var("BRAVE_PORT_END", "9100");

        let (start, end) = get_brave_port_range();
        assert_eq!(start, 1024, "Should clamp to MIN_PORT");
        assert_eq!(end, 9100);
        assert!(start < end);
    }

    #[test]
    fn test_end_below_min_gets_clamped() {
        let _guard = setup_test_env();
        // This also tests the swap: 9100 > 500, so they swap first, then clamp
        env::set_var("BRAVE_PORT_START", "9100");
        env::set_var("BRAVE_PORT_END", "500");

        let (start, end) = get_brave_port_range();
        // After swap: 500, 9100 -> after clamp: 1024, 9100
        assert_eq!(start, 1024, "Should clamp to MIN_PORT after swap");
        assert_eq!(end, 9100);
        assert!(start < end);
    }

    // ============================================================================
    // Invalid Input Handling
    // ============================================================================

    #[test]
    fn test_invalid_start_port_uses_default() {
        let _guard = setup_test_env();
        env::set_var("BRAVE_PORT_START", "not_a_number");
        env::set_var("BRAVE_PORT_END", "9050");

        let (start, end) = get_brave_port_range();
        assert_eq!(
            start, DEFAULT_BRAVE_PORT_START,
            "Should use default for invalid"
        );
        assert_eq!(end, 9050);
    }

    #[test]
    fn test_invalid_end_port_uses_default() {
        let _guard = setup_test_env();
        env::set_var("BRAVE_PORT_START", "9001");
        env::set_var("BRAVE_PORT_END", "");

        let (start, end) = get_brave_port_range();
        assert_eq!(start, 9001);
        assert_eq!(
            end, DEFAULT_BRAVE_PORT_END,
            "Should use default for invalid"
        );
    }

    #[test]
    fn test_both_invalid_use_defaults() {
        let _guard = setup_test_env();
        env::set_var("CHROME_PORT_START", "abc");
        env::set_var("CHROME_PORT_END", "xyz");

        let (start, end) = get_chrome_port_range();
        assert_eq!(start, DEFAULT_CHROME_PORT_START);
        assert_eq!(end, DEFAULT_CHROME_PORT_END);
    }

    #[test]
    fn test_empty_string_uses_default() {
        let _guard = setup_test_env();
        env::set_var("BRAVE_PORT_START", "");
        env::set_var("BRAVE_PORT_END", "");

        let (start, end) = get_brave_port_range();
        assert_eq!(start, DEFAULT_BRAVE_PORT_START);
        assert_eq!(end, DEFAULT_BRAVE_PORT_END);
    }

    #[test]
    fn test_negative_number_uses_default() {
        let _guard = setup_test_env();
        // Negative numbers can't be parsed as u16, so they fall back to default
        env::set_var("CHROME_PORT_START", "-100");
        env::set_var("CHROME_PORT_END", "9222");

        let (start, end) = get_chrome_port_range();
        assert_eq!(
            start, DEFAULT_CHROME_PORT_START,
            "Negative should use default"
        );
        assert_eq!(end, 9222);
    }

    // ============================================================================
    // Only One Variable Set
    // ============================================================================

    #[test]
    fn test_only_start_set_uses_default_end() {
        let _guard = setup_test_env();
        // Set only START (use value < default end 9050 to avoid swap)
        env::set_var("BRAVE_PORT_START", "9040");
        // BRAVE_PORT_END not set (defaults to 9050)

        let (start, end) = get_brave_port_range();
        assert_eq!(start, 9040, "START should be the value we set");
        assert_eq!(
            end, DEFAULT_BRAVE_PORT_END,
            "END should use default since not set"
        );
    }

    #[test]
    fn test_only_end_set_uses_default_start() {
        let _guard = setup_test_env();
        // BRAVE_PORT_START not set
        env::set_var("BRAVE_PORT_END", "9500");

        let (start, end) = get_brave_port_range();
        assert_eq!(
            start, DEFAULT_BRAVE_PORT_START,
            "Should use default for missing"
        );
        assert_eq!(end, 9500);
    }

    // ============================================================================
    // Range Size Tests
    // ============================================================================

    #[test]
    fn test_large_valid_range() {
        let _guard = setup_test_env();
        env::set_var("BRAVE_PORT_START", "1024");
        env::set_var("BRAVE_PORT_END", "65535");

        let (start, end) = get_brave_port_range();
        assert_eq!(start, 1024);
        assert_eq!(end, 65535);
    }

    #[test]
    fn test_small_valid_range() {
        let _guard = setup_test_env();
        env::set_var("CHROME_PORT_START", "5000");
        env::set_var("CHROME_PORT_END", "5001");

        let (start, end) = get_chrome_port_range();
        assert_eq!(start, 5000);
        assert_eq!(end, 5001);
    }

    // ============================================================================
    // Boundary Tests
    // ============================================================================

    #[test]
    fn test_exact_min_port() {
        let _guard = setup_test_env();
        env::set_var("BRAVE_PORT_START", "1024");
        env::set_var("BRAVE_PORT_END", "1024");

        let (start, end) = get_brave_port_range();
        assert_eq!(start, 1024);
        assert_eq!(end, 1024);
    }

    #[test]
    fn test_exact_max_port() {
        let _guard = setup_test_env();
        env::set_var("CHROME_PORT_START", "65535");
        env::set_var("CHROME_PORT_END", "65535");

        let (start, end) = get_chrome_port_range();
        assert_eq!(start, 65535);
        assert_eq!(end, 65535);
    }

    #[test]
    fn test_one_below_min_one_at_min() {
        let _guard = setup_test_env();
        env::set_var("BRAVE_PORT_START", "1023");
        env::set_var("BRAVE_PORT_END", "1024");

        let (start, end) = get_brave_port_range();
        assert_eq!(start, 1024); // 1023 clamped to 1024
        assert_eq!(end, 1024);
    }

    #[test]
    fn test_whitespace_in_env_var() {
        let _guard = setup_test_env();
        // Whitespace should cause parse failure, falling back to default
        env::set_var("BRAVE_PORT_START", " 9001 ");
        env::set_var("BRAVE_PORT_END", "9050");

        let (start, end) = get_brave_port_range();
        // Whitespace causes parse to fail, uses default
        assert_eq!(start, DEFAULT_BRAVE_PORT_START);
        assert_eq!(end, 9050);
    }
}
