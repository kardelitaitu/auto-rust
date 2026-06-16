//! Type-safe newtypes for tweet identifiers and status URLs.
//!
//! Prevents silent mixups between tweet IDs, usernames, URLs, and other
//! stringly-typed values by encoding the semantic type at compile time.
//!
//! # Examples
//!
//! ```
//! use auto::utils::twitter::TweetId;
//!
//! let id = TweetId::new("12345").unwrap();
//! assert_eq!(id.as_ref(), "12345");
//! assert_eq!(id.to_string(), "12345");
//! ```
//!
//! ```
//! use auto::utils::twitter::StatusUrl;
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
    #[allow(clippy::expect_used)]
    fn from(s: String) -> Self {
        Self::new(s).expect("TweetId::from called with empty string")
    }
}

impl From<&str> for TweetId {
    #[allow(clippy::expect_used)]
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
/// let url = auto::utils::twitter::StatusUrl::new("/username/status/12345").unwrap();
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
    #[allow(clippy::expect_used)]
    fn from(s: String) -> Self {
        Self::new(s).expect("StatusUrl::from called with empty string")
    }
}

impl From<&str> for StatusUrl {
    #[allow(clippy::expect_used)]
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

// ============================================================================
// State Machine — typed states for reply/quote composer flow
// ============================================================================

/// States in the reply/quote composer flow.
///
/// Encodes the transition: `Idle → ComposerOpen → TextEntered → Posted`.
/// Replaces implicit procedural order (where the caller must manually ensure
/// the correct sequence) with a tracked state that can be checked at runtime.
///
/// # Transitions
///
/// | From | To | How |
/// |---|---|---|
/// | `Idle` | `ComposerOpen` | `click_reply_button()` or `click_quote_button()`
/// | `ComposerOpen` | `TextEntered` | `type_reply()` or `type_quote()`
/// | `TextEntered` | `Posted` | `post_reply_with_retry()` or `post_quote_with_retry()`
///
/// # Example
///
/// ```rust
/// use auto::utils::twitter::ComposerFlow;
///
/// let mut flow = ComposerFlow::new();
/// assert!(flow.is_idle());
///
/// flow.record_composer_opened().unwrap();
/// assert!(flow.is_composer_open());
///
/// flow.record_text_entered().unwrap();
/// assert!(flow.has_text());
///
/// flow.record_posted().unwrap();
/// assert!(flow.is_posted());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplyFlowState {
    /// Initial state — no composer open.
    Idle,
    /// Reply/quote composer is visible and ready for text input.
    ComposerOpen,
    /// Text has been entered into the composer, ready to post.
    TextEntered,
    /// Post was successfully submitted.
    Posted,
}

/// Type-safe controller for the reply/quote composer flow.
///
/// Tracks the current state of the composer interaction and provides
/// transition methods that enforce the valid state machine order. Each
/// transition returns `Result` to prevent accidental misuse.
///
/// # Valid Transitions
///
/// ```ignore
/// Idle --record_composer_opened()--> ComposerOpen
/// ComposerOpen --record_text_entered()--> TextEntered
/// TextEntered --record_posted()--> Posted
/// ```
///
/// Any other transition returns `Err(FlowError)` with a descriptive message.
#[derive(Debug, Clone)]
pub struct ComposerFlow {
    state: ReplyFlowState,
}

impl ComposerFlow {
    /// Create a new flow in `Idle` state.
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: ReplyFlowState::Idle,
        }
    }

    /// Return the current state.
    #[must_use]
    pub fn state(&self) -> ReplyFlowState {
        self.state
    }

    // -- Query helpers --

    /// Check if in `Idle` state (no composer open).
    #[must_use]
    pub fn is_idle(&self) -> bool {
        self.state == ReplyFlowState::Idle
    }

    /// Check if in `ComposerOpen` state (composer visible, ready for text).
    #[must_use]
    pub fn is_composer_open(&self) -> bool {
        self.state == ReplyFlowState::ComposerOpen
    }

    /// Check if in `TextEntered` state (text entered, ready to post).
    #[must_use]
    pub fn has_text(&self) -> bool {
        self.state == ReplyFlowState::TextEntered
    }

    /// Check if in `Posted` state (post submitted successfully).
    #[must_use]
    pub fn is_posted(&self) -> bool {
        self.state == ReplyFlowState::Posted
    }

    // -- Transition methods --

    /// Transition from `Idle` → `ComposerOpen`.
    ///
    /// Call this after successfully clicking the reply or quote button.
    ///
    /// # Errors
    ///
    /// Returns `FlowError::InvalidTransition` if not in `Idle` state.
    pub fn record_composer_opened(&mut self) -> Result<&mut Self, FlowError> {
        require_state(self.state, ReplyFlowState::Idle)?;
        self.state = ReplyFlowState::ComposerOpen;
        Ok(self)
    }

    /// Transition from `ComposerOpen` → `TextEntered`.
    ///
    /// Call this after successfully typing text into the composer.
    ///
    /// # Errors
    ///
    /// Returns `FlowError::InvalidTransition` if not in `ComposerOpen` state.
    pub fn record_text_entered(&mut self) -> Result<&mut Self, FlowError> {
        require_state(self.state, ReplyFlowState::ComposerOpen)?;
        self.state = ReplyFlowState::TextEntered;
        Ok(self)
    }

    /// Transition from `TextEntered` → `Posted`.
    ///
    /// Call this after successfully submitting the post.
    ///
    /// # Errors
    ///
    /// Returns `FlowError::InvalidTransition` if not in `TextEntered` state.
    pub fn record_posted(&mut self) -> Result<&mut Self, FlowError> {
        require_state(self.state, ReplyFlowState::TextEntered)?;
        self.state = ReplyFlowState::Posted;
        Ok(self)
    }

    /// Reset the flow back to `Idle` state.
    pub fn reset(&mut self) {
        self.state = ReplyFlowState::Idle;
    }
}

impl Default for ComposerFlow {
    fn default() -> Self {
        Self::new()
    }
}

/// Error type for invalid state transitions in the composer flow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlowError {
    from: ReplyFlowState,
    expected: ReplyFlowState,
}

impl FlowError {
    #[must_use]
    pub fn new(from: ReplyFlowState, expected: ReplyFlowState) -> Self {
        Self { from, expected }
    }

    /// The actual state that caused the error.
    #[must_use]
    pub fn from_state(&self) -> ReplyFlowState {
        self.from
    }

    /// The expected state required for the transition.
    #[must_use]
    pub fn expected_state(&self) -> ReplyFlowState {
        self.expected
    }
}

impl std::fmt::Display for FlowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "invalid composer flow transition: expected {:?}, got {:?}",
            self.expected, self.from
        )
    }
}

impl std::error::Error for FlowError {}

fn require_state(actual: ReplyFlowState, expected: ReplyFlowState) -> Result<(), FlowError> {
    if actual == expected {
        Ok(())
    } else {
        Err(FlowError::new(actual, expected))
    }
}

// ============================================================================
// Outcome Enums — typed results for engagement actions
// ============================================================================

/// Outcome of a single engagement action (like, retweet, reply, bookmark, quote).
///
/// Replaces the ambiguous `Result<bool>` pattern where `false` could mean
/// "already done", "element not found", or "action failed".
///
/// # Examples
///
/// ```
/// use auto::utils::twitter::EngagementOutcome;
///
/// let outcome = EngagementOutcome::Completed;
/// match outcome {
///     EngagementOutcome::Completed => println!("action done"),
///     EngagementOutcome::AlreadyDone => println!("was already done"),
///     EngagementOutcome::ElementNotFound => println!("button missing"),
///     EngagementOutcome::Failed => println!("action failed"),
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngagementOutcome {
    /// Action completed successfully.
    Completed,
    /// Action was already performed (e.g., tweet already liked).
    AlreadyDone,
    /// Required UI element not found (button, composer, etc.).
    ElementNotFound,
    /// Action failed after attempt (network, timing, etc.).
    Failed,
}

/// Outcome of a follow action.
///
/// Replaces the ambiguous `Result<bool>` pattern from `follow_from_tweet()`
/// and `robust_follow()`.
///
/// # Examples
///
/// ```
/// use auto::utils::twitter::FollowOutcome;
///
/// let outcome = FollowOutcome::Followed;
/// match outcome {
///     FollowOutcome::Followed => println!("now following"),
///     FollowOutcome::AlreadyFollowing => println!("already following"),
///     FollowOutcome::ButtonNotFound => println!("no follow button"),
///     FollowOutcome::Failed => println!("follow failed"),
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FollowOutcome {
    /// Successfully followed.
    Followed,
    /// Already following this user.
    AlreadyFollowing,
    /// Follow button not visible or not found.
    ButtonNotFound,
    /// Follow attempted but failed (retries exhausted, verification failed).
    Failed,
}

/// Outcome of posting a reply or quote tweet.
///
/// Replaces the ambiguous `Result<bool>` pattern from `post_reply()`
/// and `post_quote()`.
///
/// # Examples
///
/// ```
/// use auto::utils::twitter::PostOutcome;
///
/// let outcome = PostOutcome::Posted;
/// match outcome {
///     PostOutcome::Posted => println!("posted"),
///     PostOutcome::ComposerNotFound => println!("no composer"),
///     PostOutcome::Failed => println!("post failed"),
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PostOutcome {
    /// Post confirmed successful.
    Posted,
    /// Composer or post button not found.
    ComposerNotFound,
    /// Post attempted but failed.
    Failed,
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

    // ========================================================================
    // EngagementOutcome tests
    // ========================================================================

    #[test]
    fn engagement_outcome_all_variants_exist() {
        let completed = EngagementOutcome::Completed;
        let already_done = EngagementOutcome::AlreadyDone;
        let not_found = EngagementOutcome::ElementNotFound;
        let failed = EngagementOutcome::Failed;

        // Verify Debug/Display for each variant
        assert!(!format!("{completed:?}").is_empty());
        assert!(!format!("{already_done:?}").is_empty());
        assert!(!format!("{not_found:?}").is_empty());
        assert!(!format!("{failed:?}").is_empty());
    }

    #[test]
    fn engagement_outcome_eq() {
        assert_eq!(EngagementOutcome::Completed, EngagementOutcome::Completed);
        assert_ne!(EngagementOutcome::Completed, EngagementOutcome::AlreadyDone);
        assert_ne!(
            EngagementOutcome::AlreadyDone,
            EngagementOutcome::ElementNotFound
        );
        assert_ne!(
            EngagementOutcome::ElementNotFound,
            EngagementOutcome::Failed
        );
    }

    #[test]
    fn engagement_outcome_clone() {
        let outcome = EngagementOutcome::Completed;
        let cloned = outcome.clone();
        assert_eq!(outcome, cloned);
    }

    #[test]
    fn engagement_outcome_debug() {
        assert_eq!(format!("{:?}", EngagementOutcome::Completed), "Completed");
        assert_eq!(
            format!("{:?}", EngagementOutcome::AlreadyDone),
            "AlreadyDone"
        );
        assert_eq!(
            format!("{:?}", EngagementOutcome::ElementNotFound),
            "ElementNotFound"
        );
        assert_eq!(format!("{:?}", EngagementOutcome::Failed), "Failed");
    }

    // ========================================================================
    // FollowOutcome tests
    // ========================================================================

    #[test]
    fn follow_outcome_all_variants_exist() {
        let followed = FollowOutcome::Followed;
        let already = FollowOutcome::AlreadyFollowing;
        let not_found = FollowOutcome::ButtonNotFound;
        let failed = FollowOutcome::Failed;

        assert!(!format!("{followed:?}").is_empty());
        assert!(!format!("{already:?}").is_empty());
        assert!(!format!("{not_found:?}").is_empty());
        assert!(!format!("{failed:?}").is_empty());
    }

    #[test]
    fn follow_outcome_eq() {
        assert_eq!(FollowOutcome::Followed, FollowOutcome::Followed);
        assert_ne!(FollowOutcome::Followed, FollowOutcome::AlreadyFollowing);
        assert_ne!(
            FollowOutcome::AlreadyFollowing,
            FollowOutcome::ButtonNotFound
        );
        assert_ne!(FollowOutcome::ButtonNotFound, FollowOutcome::Failed);
    }

    #[test]
    fn follow_outcome_clone() {
        let outcome = FollowOutcome::Followed;
        let cloned = outcome.clone();
        assert_eq!(outcome, cloned);
    }

    #[test]
    fn follow_outcome_debug() {
        assert_eq!(format!("{:?}", FollowOutcome::Followed), "Followed");
        assert_eq!(
            format!("{:?}", FollowOutcome::AlreadyFollowing),
            "AlreadyFollowing"
        );
        assert_eq!(
            format!("{:?}", FollowOutcome::ButtonNotFound),
            "ButtonNotFound"
        );
        assert_eq!(format!("{:?}", FollowOutcome::Failed), "Failed");
    }

    // ========================================================================
    // PostOutcome tests
    // ========================================================================

    #[test]
    fn post_outcome_all_variants_exist() {
        let posted = PostOutcome::Posted;
        let not_found = PostOutcome::ComposerNotFound;
        let failed = PostOutcome::Failed;

        assert!(!format!("{posted:?}").is_empty());
        assert!(!format!("{not_found:?}").is_empty());
        assert!(!format!("{failed:?}").is_empty());
    }

    #[test]
    fn post_outcome_eq() {
        assert_eq!(PostOutcome::Posted, PostOutcome::Posted);
        assert_ne!(PostOutcome::Posted, PostOutcome::ComposerNotFound);
        assert_ne!(PostOutcome::ComposerNotFound, PostOutcome::Failed);
    }

    #[test]
    fn post_outcome_clone() {
        let outcome = PostOutcome::Posted;
        let cloned = outcome.clone();
        assert_eq!(outcome, cloned);
    }

    #[test]
    fn post_outcome_debug() {
        assert_eq!(format!("{:?}", PostOutcome::Posted), "Posted");
        assert_eq!(
            format!("{:?}", PostOutcome::ComposerNotFound),
            "ComposerNotFound"
        );
        assert_eq!(format!("{:?}", PostOutcome::Failed), "Failed");
    }
}
