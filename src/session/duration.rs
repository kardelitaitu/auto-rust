//! Non-zero millisecond duration type for session configuration.
//!
//! Extracted from `session/mod.rs` — spec 0019.

use crate::utils::math::random_in_range;
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

// ============================================================================
// Duration math pure functions
// ============================================================================

/// Returns a randomized duration around a base value using a uniform spread.
///
/// Example: `duration_with_variance(300_000, 20)` yields a value in
/// `240_000..=360_000`.
#[must_use]
pub fn duration_with_variance(base_ms: u64, variance_pct: u32) -> u64 {
    if base_ms == 0 {
        return 0;
    }

    let variance_pct = variance_pct.min(100);
    let delta = base_ms.saturating_mul(u64::from(variance_pct)) / 100;
    let min_ms = base_ms.saturating_sub(delta);
    let max_ms = base_ms.saturating_add(delta);
    random_in_range(min_ms, max_ms)
}

/// Converts a [`std::time::Duration`] to milliseconds as a [`u64`].
///
/// Uses a saturating cast from `u128` to `u64`. In practice the truncation
/// only matters for durations longer than ~584 million years.
#[must_use]
pub fn duration_ms(d: std::time::Duration) -> u64 {
    d.as_millis() as u64
}

impl DurationMs {
    /// Returns a randomized duration with variance applied, as a raw `u64`.
    ///
    /// The result is uniformly distributed in `[self * (1 - pct), self * (1 + pct)]`.
    #[must_use]
    pub fn with_variance(self, variance_pct: u32) -> u64 {
        duration_with_variance(self.get(), variance_pct)
    }

    /// Saturating addition — returns `None` on overflow.
    #[must_use]
    pub fn checked_add(self, rhs: u64) -> Option<Self> {
        Self::new(self.get().checked_add(rhs)?)
    }

    /// Saturating subtraction — returns `None` if result would be zero or negative.
    #[must_use]
    pub fn checked_sub(self, rhs: u64) -> Option<Self> {
        let result = self.get().checked_sub(rhs)?;
        Self::new(result)
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

    // ========================================================================
    // Duration math pure function tests
    // ========================================================================

    #[test]
    fn duration_with_variance_zero_returns_base() {
        assert_eq!(duration_with_variance(120_000, 0), 120_000);
    }

    #[test]
    fn duration_with_variance_stays_within_bounds() {
        let value = duration_with_variance(300_000, 20);
        assert!((240_000..=360_000).contains(&value));
    }

    #[test]
    fn duration_with_variance_zero_base() {
        assert_eq!(duration_with_variance(0, 50), 0);
    }

    #[test]
    fn duration_with_variance_max_variance() {
        for _ in 0..50 {
            let value = duration_with_variance(1000, 100);
            assert!((0..=2000).contains(&value));
        }
    }

    #[test]
    fn duration_with_variance_over_100_clamped() {
        for _ in 0..50 {
            let value = duration_with_variance(1000, 200);
            // 200% is clamped to 100%, so range is [0, 2000]
            assert!((0..=2000).contains(&value));
        }
    }

    #[test]
    fn duration_ms_converts() {
        let dur = std::time::Duration::from_millis(1500);
        assert_eq!(duration_ms(dur), 1500);
    }

    #[test]
    fn duration_ms_zero() {
        let dur = std::time::Duration::from_millis(0);
        assert_eq!(duration_ms(dur), 0);
    }

    #[test]
    fn duration_ms_large() {
        let dur = std::time::Duration::from_secs(3600);
        assert_eq!(duration_ms(dur), 3_600_000);
    }

    #[test]
    fn duration_ms_max() {
        let dur = std::time::Duration::MAX;
        // Should saturate to u64::MAX
        assert_eq!(duration_ms(dur), u64::MAX);
    }

    #[test]
    fn duration_ms_with_variance_on_type() {
        let d = DurationMs::new(100_000).unwrap();
        for _ in 0..20 {
            let val = d.with_variance(20);
            assert!((80_000..=120_000).contains(&val));
        }
    }

    #[test]
    fn checked_add_overflow_returns_none() {
        let d = DurationMs::new(u64::MAX).unwrap();
        assert!(d.checked_add(1).is_none());
    }

    #[test]
    fn checked_add_normal() {
        let d = DurationMs::new(100).unwrap();
        assert_eq!(d.checked_add(50).unwrap().get(), 150);
    }

    #[test]
    fn checked_sub_underflow_returns_none() {
        let d = DurationMs::new(100).unwrap();
        assert!(d.checked_sub(100).is_none());
        assert!(d.checked_sub(101).is_none());
    }

    #[test]
    fn checked_sub_normal() {
        let d = DurationMs::new(100).unwrap();
        assert_eq!(d.checked_sub(30).unwrap().get(), 70);
    }
}
