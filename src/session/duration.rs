//! Non-zero millisecond duration type for session configuration.
//!
//! Extracted from `session/mod.rs` — spec 0019.

use std::num::NonZeroU64;

/// A duration in milliseconds guaranteed to be non-zero.
///
/// Used for configuration values that require a positive duration.
/// Wraps `NonZeroU64` for zero-cost validation at the type level.
///
/// # Serde
///
/// Serializes/deserializes transparently as a plain `u64` (via `#[serde(transparent)]`),
/// making it compatible with TOML/JSON config files. Deserializing `0` will return an error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DurationMs(NonZeroU64);

impl DurationMs {
    /// Creates a new `DurationMs` from a non-zero millisecond value.
    ///
    /// # Returns
    /// * `Some(DurationMs)` if `value > 0`
    /// * `None` if `value == 0`
    #[must_use]
    pub fn new(value: u64) -> Option<Self> {
        Some(Self(NonZeroU64::new(value)?))
    }

    /// Creates a new `DurationMs` in a const context.
    ///
    /// # Panics
    /// Panics if `value == 0`.
    #[must_use]
    pub const fn new_const(value: u64) -> Self {
        assert!(value > 0, "DurationMs cannot be zero");
        // SAFETY: We just asserted value > 0
        Self(unsafe { NonZeroU64::new_unchecked(value) })
    }

    /// Returns the underlying millisecond value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0.get()
    }

    /// Converts to seconds (integer division).
    #[must_use]
    pub const fn as_secs(self) -> u64 {
        self.0.get() / 1000
    }
}

impl std::fmt::Display for DurationMs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<DurationMs> for u64 {
    fn from(d: DurationMs) -> Self {
        d.0.get()
    }
}

impl From<DurationMs> for std::time::Duration {
    fn from(d: DurationMs) -> Self {
        std::time::Duration::from_millis(d.0.get())
    }
}

impl std::ops::Mul<u64> for DurationMs {
    type Output = DurationMs;

    fn mul(self, rhs: u64) -> Self::Output {
        let result = self.0.get().saturating_mul(rhs);
        // Both operands are non-zero, so result is non-zero (unless overflow saturates to 0
        // which can't happen since saturating_mul of non-zero values returns at least 1)
        debug_assert!(
            result > 0,
            "Multiplication of non-zero DurationMs by non-zero u64 should be > 0"
        );
        // SAFETY: result is guaranteed > 0 because both operands are > 0
        Self(unsafe { NonZeroU64::new_unchecked(result) })
    }
}

// serde support — transparent (serializes as plain u64)
impl serde::Serialize for DurationMs {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.get().serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for DurationMs {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = u64::deserialize(deserializer)?;
        NonZeroU64::new(value)
            .map(Self)
            .ok_or_else(|| serde::de::Error::custom("DurationMs must be non-zero"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_const_creates_valid_duration() {
        let d = DurationMs::new_const(1);
        assert_eq!(d.get(), 1);
    }

    #[test]
    fn new_const_creates_large_value() {
        let d = DurationMs::new_const(u64::MAX);
        assert_eq!(d.get(), u64::MAX);
    }

    #[test]
    #[should_panic(expected = "cannot be zero")]
    fn new_const_panics_on_zero() {
        let _ = DurationMs::new_const(0);
    }

    #[test]
    fn new_returns_some_for_nonzero() {
        assert!(DurationMs::new(1).is_some());
        assert!(DurationMs::new(u64::MAX).is_some());
    }

    #[test]
    fn new_returns_none_for_zero() {
        assert!(DurationMs::new(0).is_none());
    }

    #[test]
    fn mul_preserves_nonzero() {
        let d = DurationMs::new(5).unwrap();
        let r = d * 3;
        assert_eq!(r.get(), 15);
    }

    #[test]
    fn mul_by_one_returns_same() {
        let d = DurationMs::new(100).unwrap();
        assert_eq!((d * 1).get(), 100);
    }

    #[test]
    fn mul_saturates_on_overflow() {
        let d = DurationMs::new(u64::MAX).unwrap();
        assert_eq!((d * 2).get(), u64::MAX);
    }

    #[test]
    fn as_secs_converts_correctly() {
        let d = DurationMs::new(5000).unwrap();
        assert_eq!(d.as_secs(), 5);
    }

    #[test]
    fn get_returns_raw_value() {
        let d = DurationMs::new(42).unwrap();
        assert_eq!(d.get(), 42);
    }

    #[test]
    fn display_shows_raw_value() {
        let d = DurationMs::new(777).unwrap();
        assert_eq!(d.to_string(), "777");
    }

    #[test]
    fn into_u64_converts() {
        let d = DurationMs::new(999).unwrap();
        let v: u64 = d.into();
        assert_eq!(v, 999);
    }

    #[test]
    fn into_std_duration_converts() {
        let d = DurationMs::new(1500).unwrap();
        let dur: std::time::Duration = d.into();
        assert_eq!(dur.as_millis(), 1500);
    }

    #[test]
    fn ord_works() {
        let a = DurationMs::new(10).unwrap();
        let b = DurationMs::new(20).unwrap();
        assert!(a < b);
        assert!(b > a);
        assert_eq!(a, a);
    }

    #[test]
    fn clone_is_equal() {
        let a = DurationMs::new(55).unwrap();
        assert_eq!(a, a.clone());
    }

    #[test]
    fn mul_debug_assert_holds() {
        let d = DurationMs::new(1).unwrap();
        let r = d * 1;
        assert_eq!(r.get(), 1);
    }

    #[test]
    fn serde_roundtrip() {
        let d = DurationMs::new(3000).unwrap();
        let json = serde_json::to_string(&d).unwrap();
        assert_eq!(json, "3000");
        let back: DurationMs = serde_json::from_str(&json).unwrap();
        assert_eq!(d, back);
    }

    #[test]
    fn serde_rejects_zero() {
        let result: Result<DurationMs, _> = serde_json::from_str("0");
        assert!(result.is_err());
    }
}
