//! Type-safe newtypes for tweet identifiers and status URLs.
//!
//! Prevents silent mixups between tweet IDs, usernames, URLs, and other
//! stringly-typed values by encoding the semantic type at compile time.
//!
//! # Examples
//!
//! ```
//! use crate::utils::twitter::twitteractivity_types::TweetId;
//!
//! let id = TweetId::new("12345").unwrap();
//! assert_eq!(id.as_ref(), "12345");
//! assert_eq!(id.to_string(), "12345");
//! ```
//!
//! ```
//! use crate::utils::twitter::twitteractivity_types::StatusUrl;
//!
//! let url = StatusUrl::new("/user/status/12345").unwrap();
//! assert_eq!(url.tweet_id(), Some("12345"));
//! ```

use std::fmt;
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::str::FromStr;

/// A validated tweet identifier.
///
/// Wraps a non-empty `String` and provides type safety so tweet IDs cannot
/// be accidentally swapped with usernames, URLs, or other string types.
///
/// # Validation
///
/// Construction succeeds as long as the string is non-empty. This is
/// intentionally lenient because tweet IDs come from multiple sources:
/// real numeric IDs from the Twitter API, test identifiers, and DOM
/// attributes — all of which are valid.
#[derive(Debug, Clone)]
pub struct TweetId(String);

impl TweetId {
    /// Create a new `TweetId`, validating that the value is non-empty.
    ///
    /// # Errors
    ///
    /// Returns `Err` with a descriptive message if `id` is empty.
    pub fn new(id: impl Into<String>) -> Result<Self, String> {
        let s = id.into();
        if s.is_empty() {
            Err("Tweet ID cannot be empty".to_string())
        } else {
            Ok(Self(s))
        }
    }

    /// Create a `TweetId` without validation.
    ///
    /// Use this when the value is known to be valid (e.g., from a JSON parse
    /// that already checked for presence).
    pub fn from_unchecked(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Return the inner string as a `&str`.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume the `TweetId` and return the inner `String`.
    #[must_use]
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl fmt::Display for TweetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for TweetId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Deref for TweetId {
    type Target = str;

    fn deref(&self) -> &str {
        &self.0
    }
}

impl PartialEq for TweetId {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for TweetId {}

impl Hash for TweetId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl From<String> for TweetId {
    fn from(s: String) -> Self {
        Self::new(s).expect("TweetId::from called with empty string")
    }
}

impl From<&str> for TweetId {
    fn from(s: &str) -> Self {
        Self::new(s).expect("TweetId::from called with empty string")
    }
}

impl FromStr for TweetId {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

// ============================================================================

/// A validated Twitter/X status URL.
///
/// Wraps a non-empty string that represents a status URL path or absolute URL.
/// Provides type safety so status URLs cannot be accidentally swapped with
/// regular URLs, tweet IDs, or other string types.
///
/// # Examples
///
/// ```
/// let url = StatusUrl::new("/username/status/12345").unwrap();
/// assert_eq!(url.tweet_id(), Some("12345"));
/// ```
#[derive(Debug, Clone)]
pub struct StatusUrl(String);

impl StatusUrl {
    /// Create a new `StatusUrl`, validating that the value is non-empty.
    ///
    /// # Errors
    ///
    /// Returns `Err` with a descriptive message if `url` is empty.
    pub fn new(url: impl Into<String>) -> Result<Self, String> {
        let s = url.into();
        if s.is_empty() {
            Err("Status URL cannot be empty".to_string())
        } else {
            Ok(Self(s))
        }
    }

    /// Create a `StatusUrl` without validation.
    ///
    /// Use this when the value is known to exist (e.g., from JSON that
    /// already checked for presence).
    pub fn from_unchecked(url: impl Into<String>) -> Self {
        Self(url.into())
    }

    /// Extract the tweet ID from this status URL.
    ///
    /// Parses the last segment after `/status/` in the URL, stripping any
    /// trailing query, fragment, or path components.
    #[must_use]
    pub fn tweet_id(&self) -> Option<&str> {
        self.0
            .split("/status/")
            .nth(1)
            .and_then(|tail| tail.split(['?', '/', '#']).next())
            .filter(|id| !id.is_empty())
    }

    /// Return the inner string as a `&str`.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume the `StatusUrl` and return the inner `String`.
    #[must_use]
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl fmt::Display for StatusUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for StatusUrl {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Deref for StatusUrl {
    type Target = str;

    fn deref(&self) -> &str {
        &self.0
    }
}

impl PartialEq for StatusUrl {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for StatusUrl {}

impl Hash for StatusUrl {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl From<String> for StatusUrl {
    fn from(s: String) -> Self {
        Self::new(s).expect("StatusUrl::from called with empty string")
    }
}

impl From<&str> for StatusUrl {
    fn from(s: &str) -> Self {
        Self::new(s).expect("StatusUrl::from called with empty string")
    }
}

impl FromStr for StatusUrl {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tweet_id_creation() {
        let id = TweetId::new("12345").unwrap();
        assert_eq!(id.as_str(), "12345");
    }

    #[test]
    fn tweet_id_rejects_empty() {
        assert!(TweetId::new("").is_err());
    }

    #[test]
    fn tweet_id_from_unchecked_works() {
        let id = TweetId::from_unchecked("test_id");
        assert_eq!(id.as_ref(), "test_id");
    }

    #[test]
    fn tweet_id_display() {
        let id = TweetId::from_unchecked("abc123");
        assert_eq!(format!("{id}"), "abc123");
    }

    #[test]
    fn tweet_id_hash_and_eq() {
        let a = TweetId::from_unchecked("same");
        let b = TweetId::from_unchecked("same");
        let c = TweetId::from_unchecked("different");

        assert_eq!(a, b);
        assert_ne!(a, c);

        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(a.clone());
        set.insert(b.clone());
        assert_eq!(set.len(), 1, "equal TweetIds deduplicate in hash set");
    }

    #[test]
    fn tweet_id_from_string() {
        let id: TweetId = "hello".into();
        assert_eq!(id.as_str(), "hello");
    }

    #[test]
    fn tweet_id_into_inner() {
        let id = TweetId::from_unchecked("owned");
        assert_eq!(id.into_inner(), "owned");
    }

    #[test]
    fn status_url_creation() {
        let url = StatusUrl::new("/user/status/12345").unwrap();
        assert_eq!(url.as_str(), "/user/status/12345");
    }

    #[test]
    fn status_url_rejects_empty() {
        assert!(StatusUrl::new("").is_err());
    }

    #[test]
    fn status_url_tweet_id_extraction() {
        let url = StatusUrl::from_unchecked("/user/status/12345");
        assert_eq!(url.tweet_id(), Some("12345"));
    }

    #[test]
    fn status_url_tweet_id_with_query() {
        let url = StatusUrl::from_unchecked("https://x.com/user/status/12345?lang=en");
        assert_eq!(url.tweet_id(), Some("12345"));
    }

    #[test]
    fn status_url_tweet_id_with_fragment() {
        let url = StatusUrl::from_unchecked("/user/status/12345#reply-1");
        assert_eq!(url.tweet_id(), Some("12345"));
    }

    #[test]
    fn status_url_tweet_id_trailing_slash() {
        let url = StatusUrl::from_unchecked("/user/status/12345/");
        assert_eq!(url.tweet_id(), Some("12345"));
    }

    #[test]
    fn status_url_tweet_id_non_status_url() {
        let url = StatusUrl::from_unchecked("https://x.com/home");
        assert_eq!(url.tweet_id(), None);
    }

    #[test]
    fn status_url_display() {
        let url = StatusUrl::from_unchecked("/status/1");
        assert_eq!(format!("{url}"), "/status/1");
    }

    #[test]
    fn status_url_clone_and_eq() {
        let a = StatusUrl::from_unchecked("/status/1");
        let b = StatusUrl::from_unchecked("/status/1");
        assert_eq!(a, b);
    }

    #[test]
    fn status_url_into_inner() {
        let url = StatusUrl::from_unchecked("/status/42");
        assert_eq!(url.into_inner(), "/status/42");
    }

    #[test]
    #[should_panic(expected = "TweetId::from called with empty string")]
    fn tweet_id_from_empty_string_panics() {
        let _: TweetId = "".into();
    }

    #[test]
    #[should_panic(expected = "StatusUrl::from called with empty string")]
    fn status_url_from_empty_string_panics() {
        let _: StatusUrl = "".into();
    }
}
