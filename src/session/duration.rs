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
