//! Shared utility for reading typed values from `serde_json::Value` payloads.
//!
//! Provides base parsing functions for `u64`, `u32`, `i32`, and `bool` fields
//! with consistent error handling. Callers add domain-specific validation
//! (e.g., positive-value checks) on top and map [`PayloadError`] to their
//! own error types as needed.

use serde_json::Value;

/// Errors that can occur during payload field parsing.
#[derive(Debug, Clone, PartialEq)]
pub enum PayloadError {
    /// The key is absent from the payload or its value is `null`.
    Missing,
    /// The value is present but cannot be parsed as the expected type.
    Invalid(String),
}

impl std::fmt::Display for PayloadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PayloadError::Missing => write!(f, "key is missing or null"),
            PayloadError::Invalid(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for PayloadError {}

// ---------------------------------------------------------------------------
// u64
// ---------------------------------------------------------------------------

/// Read a `u64` from the payload at `key`.
///
/// Returns:
/// - `Ok(value)` if the key holds a non-negative integer (or a numeric string).
/// - `Err(PayloadError::Missing)` if the key is absent or `null`.
/// - `Err(PayloadError::Invalid(…))` if the value cannot be parsed.
pub fn read_u64(payload: &Value, key: &str) -> Result<u64, PayloadError> {
    match payload.get(key) {
        None | Some(Value::Null) => Err(PayloadError::Missing),
        Some(value) => {
            if let Some(n) = value.as_u64() {
                return Ok(n);
            }
            if let Some(n) = value.as_i64() {
                if n >= 0 {
                    return Ok(n as u64);
                }
            }
            if let Some(s) = value.as_str() {
                return s.parse::<u64>().map_err(|_| {
                    PayloadError::Invalid(format!("{key} must be a non-negative integer"))
                });
            }
            Err(PayloadError::Invalid(format!(
                "{key} must be a non-negative integer"
            )))
        }
    }
}

/// Read a `u64` from the payload, falling back to `default` when the key is
/// absent or `null`.
pub fn read_u64_or(payload: &Value, key: &str, default: u64) -> Result<u64, PayloadError> {
    match read_u64(payload, key) {
        Err(PayloadError::Missing) => Ok(default),
        other => other,
    }
}

// ---------------------------------------------------------------------------
// u32
// ---------------------------------------------------------------------------

/// Read a `u32` from the payload at `key`.
///
/// Returns:
/// - `Ok(value)` if the key holds a non-negative integer that fits in `u32`.
/// - `Err(PayloadError::Missing)` if the key is absent or `null`.
/// - `Err(PayloadError::Invalid(…))` if the value cannot be parsed or is too large.
pub fn read_u32(payload: &Value, key: &str) -> Result<u32, PayloadError> {
    let val = read_u64(payload, key)?;
    u32::try_from(val).map_err(|_| PayloadError::Invalid(format!("{key} must fit within a u32")))
}

/// Read a `u32` from the payload, falling back to `default` when the key is
/// absent or `null`.
pub fn read_u32_or(payload: &Value, key: &str, default: u32) -> Result<u32, PayloadError> {
    match read_u32(payload, key) {
        Err(PayloadError::Missing) => Ok(default),
        other => other,
    }
}

// ---------------------------------------------------------------------------
// i32
// ---------------------------------------------------------------------------

/// Read an `i32` from the payload at `key`.
///
/// Supports numbers and numeric strings.
///
/// Returns:
/// - `Ok(value)` if the key holds an integer that fits in `i32`.
/// - `Err(PayloadError::Missing)` if the key is absent or `null`.
/// - `Err(PayloadError::Invalid(…))` if the value cannot be parsed.
pub fn read_i32(payload: &Value, key: &str) -> Result<i32, PayloadError> {
    match payload.get(key) {
        None | Some(Value::Null) => Err(PayloadError::Missing),
        Some(value) => {
            if let Some(n) = value.as_i64() {
                return i32::try_from(n)
                    .map_err(|_| PayloadError::Invalid(format!("{key} must fit within an i32")));
            }
            if let Some(s) = value.as_str() {
                return s
                    .parse::<i32>()
                    .map_err(|_| PayloadError::Invalid(format!("{key} must be an integer")));
            }
            Err(PayloadError::Invalid(format!("{key} must be an integer")))
        }
    }
}

/// Read an `i32` from the payload, falling back to `default` when the key is
/// absent or `null`.
pub fn read_i32_or(payload: &Value, key: &str, default: i32) -> Result<i32, PayloadError> {
    match read_i32(payload, key) {
        Err(PayloadError::Missing) => Ok(default),
        other => other,
    }
}

// ---------------------------------------------------------------------------
// bool
// ---------------------------------------------------------------------------

/// Read a `bool` from the payload at `key`.
///
/// Supports actual booleans and case-insensitive string values
/// (`"true"`, `"1"`, `"yes"`, `"on"`, `"false"`, `"0"`, `"no"`, `"off"`).
///
/// Returns:
/// - `Ok(value)` if the key holds a boolean or parseable string.
/// - `Err(PayloadError::Missing)` if the key is absent or `null`.
/// - `Err(PayloadError::Invalid(…))` if the value cannot be parsed.
pub fn read_bool(payload: &Value, key: &str) -> Result<bool, PayloadError> {
    match payload.get(key) {
        None | Some(Value::Null) => Err(PayloadError::Missing),
        Some(value) => {
            if let Some(b) = value.as_bool() {
                return Ok(b);
            }
            if let Some(s) = value.as_str() {
                return match s.to_ascii_lowercase().as_str() {
                    "true" | "1" | "yes" | "on" => Ok(true),
                    "false" | "0" | "no" | "off" => Ok(false),
                    _ => Err(PayloadError::Invalid(format!("{key} must be a boolean"))),
                };
            }
            Err(PayloadError::Invalid(format!("{key} must be a boolean")))
        }
    }
}

/// Read a `bool` from the payload, falling back to `default` when the key is
/// absent or `null`.
pub fn read_bool_or(payload: &Value, key: &str, default: bool) -> Result<bool, PayloadError> {
    match read_bool(payload, key) {
        Err(PayloadError::Missing) => Ok(default),
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // -----------------------------------------------------------------------
    // read_u64
    // -----------------------------------------------------------------------
    #[test]
    fn read_u64_present() {
        assert_eq!(read_u64(&json!({"x": 42}), "x"), Ok(42));
    }

    #[test]
    fn read_u64_string_number() {
        assert_eq!(read_u64(&json!({"x": "42"}), "x"), Ok(42));
    }

    #[test]
    fn read_u64_missing() {
        assert_eq!(read_u64(&json!({}), "x"), Err(PayloadError::Missing));
    }

    #[test]
    fn read_u64_null() {
        assert_eq!(
            read_u64(&json!({"x": null}), "x"),
            Err(PayloadError::Missing)
        );
    }

    #[test]
    fn read_u64_negative_rejected() {
        assert!(read_u64(&json!({"x": -1}), "x").is_err());
    }

    #[test]
    fn read_u64_non_numeric_rejected() {
        assert!(read_u64(&json!({"x": "abc"}), "x").is_err());
    }

    // -----------------------------------------------------------------------
    // read_u64_or
    // -----------------------------------------------------------------------
    #[test]
    fn read_u64_or_present() {
        assert_eq!(read_u64_or(&json!({"x": 42}), "x", 99), Ok(42));
    }

    #[test]
    fn read_u64_or_defaults() {
        assert_eq!(read_u64_or(&json!({}), "x", 99), Ok(99));
    }

    // -----------------------------------------------------------------------
    // read_u32
    // -----------------------------------------------------------------------
    #[test]
    fn read_u32_present() {
        assert_eq!(read_u32(&json!({"x": 42}), "x"), Ok(42u32));
    }

    #[test]
    fn read_u32_too_large() {
        let big = u64::from(u32::MAX) + 1;
        assert!(read_u32(&json!({"x": big}), "x").is_err());
    }

    // -----------------------------------------------------------------------
    // read_u32_or
    // -----------------------------------------------------------------------
    #[test]
    fn read_u32_or_present() {
        assert_eq!(read_u32_or(&json!({"x": 7}), "x", 1), Ok(7u32));
    }

    #[test]
    fn read_u32_or_defaults() {
        assert_eq!(read_u32_or(&json!({}), "x", 1), Ok(1u32));
    }

    // -----------------------------------------------------------------------
    // read_i32
    // -----------------------------------------------------------------------
    #[test]
    fn read_i32_present() {
        assert_eq!(read_i32(&json!({"x": -100}), "x"), Ok(-100));
    }

    #[test]
    fn read_i32_string() {
        assert_eq!(read_i32(&json!({"x": "42"}), "x"), Ok(42));
    }

    #[test]
    fn read_i32_too_large() {
        let big = i64::from(i32::MAX) + 1;
        assert!(read_i32(&json!({"x": big}), "x").is_err());
    }

    // -----------------------------------------------------------------------
    // read_i32_or
    // -----------------------------------------------------------------------
    #[test]
    fn read_i32_or_present() {
        assert_eq!(read_i32_or(&json!({"x": -5}), "x", 0), Ok(-5));
    }

    #[test]
    fn read_i32_or_defaults() {
        assert_eq!(read_i32_or(&json!({}), "x", -1), Ok(-1));
    }

    // -----------------------------------------------------------------------
    // read_bool
    // -----------------------------------------------------------------------
    #[test]
    fn read_bool_present() {
        assert_eq!(read_bool(&json!({"x": true}), "x"), Ok(true));
    }

    #[test]
    fn read_bool_string_true() {
        assert_eq!(read_bool(&json!({"x": "true"}), "x"), Ok(true));
        assert_eq!(read_bool(&json!({"x": "1"}), "x"), Ok(true));
        assert_eq!(read_bool(&json!({"x": "yes"}), "x"), Ok(true));
        assert_eq!(read_bool(&json!({"x": "on"}), "x"), Ok(true));
    }

    #[test]
    fn read_bool_string_false() {
        assert_eq!(read_bool(&json!({"x": "false"}), "x"), Ok(false));
        assert_eq!(read_bool(&json!({"x": "0"}), "x"), Ok(false));
        assert_eq!(read_bool(&json!({"x": "no"}), "x"), Ok(false));
        assert_eq!(read_bool(&json!({"x": "off"}), "x"), Ok(false));
    }

    #[test]
    fn read_bool_missing() {
        assert_eq!(read_bool(&json!({}), "x"), Err(PayloadError::Missing));
    }

    // -----------------------------------------------------------------------
    // read_bool_or
    // -----------------------------------------------------------------------
    #[test]
    fn read_bool_or_present() {
        assert_eq!(read_bool_or(&json!({"x": false}), "x", true), Ok(false));
    }

    #[test]
    fn read_bool_or_defaults() {
        assert_eq!(read_bool_or(&json!({}), "x", true), Ok(true));
    }
}
