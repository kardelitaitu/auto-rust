//! Sentiment analysis helper functions, constants, and standalone public API.
//!
//! Extracted from `sentiment/analyzer.rs` — spec 0020.

use super::types::*;
use serde_json::Value;

// Re-export the canonical extract_tweet_text from actions module.
pub(crate) use crate::utils::twitter::twitteractivity_actions::extract_tweet_text;

// ============================================================================
// Strategy Constants
// ============================================================================

/// Negation patterns that flip sentiment polarity.
const NEGATION_PATTERNS: &[&str] = &[
    "not",
    "no",
    "never",
    "neither",
    "nobody",
    "nothing",
    "nor",
    "can't",
    "cant",
    "couldn't",
    "couldnt",
    "shouldn't",
    "shouldnt",
    "wouldn't",
    "wouldnt",
    "don't",
    "dont",
    "doesn't",
    "doesnt",
    "didn't",
    "didnt",
    "isn't",
    "isnt",
    "aren't",
    "arent",
    "wasn't",
    "wasnt",
    "weren't",
    "werent",
    "without",
    "lack",
    "lacking",
    "absent",
    "hardly",
    "barely",
    "scarcely",
    "little",
    "few",
    "nowhere",
    "nothing",
];

/// Intensifiers that amplify sentiment (multiplier > 1.0).
const INTENSIFIERS: &[(&str, f32)] = &[
    ("very", 1.5),
    ("really", 1.5),
    ("extremely", 2.0),
    ("incredibly", 2.0),
    ("absolutely", 2.0),
    ("totally", 1.8),
    ("completely", 1.8),
    ("utterly", 2.0),
    ("highly", 1.5),
    ("super", 1.5),
    ("so", 1.3),
    ("quite", 1.2),
    ("rather", 1.2),
    ("pretty", 1.2),
    ("damn", 1.8),
    ("fucking", 2.0),
    ("frigging", 1.8),
    ("bloody", 1.8),
    ("truly", 1.5),
    ("genuinely", 1.3),
    ("honestly", 1.3),
    ("actually", 1.2),
    ("especially", 1.5),
    ("particularly", 1.4),
    ("exceptionally", 2.0),
    ("remarkably", 1.8),
    ("extraordinarily", 2.0),
];

/// Sarcasm markers and patterns that indicate inverted meaning.
const SARCASM_PATTERNS: &[&str] = &[
    "oh great",
    "oh wonderful",
    "oh perfect",
    "oh good",
    "oh fantastic",
    "sure, because",
    "yeah right",
    "as if",
    "as though",
    "thanks, i hate it",
    "tanks, i hate it",
    "thx i hate it",
    "just what i needed",
    "exactly what i wanted",
    "because that's what i need",
    "because that's what i wanted",
    "thanks twitter",
    "thx twitter",
    "cool cool cool",
    "sure sure",
    "okay sure",
    "what could go wrong",
    "how hard could it be",
    "famous last words",
    "we'll see about that",
];

// ============================================================================
// Keyword Lexicons
// ============================================================================

pub(crate) const POSITIVE_WORDS: &[&str] = &[
    "good",
    "great",
    "awesome",
    "amazing",
    "excellent",
    "love",
    "like",
    "nice",
    "wonderful",
    "fantastic",
    "best",
    "happy",
    "glad",
    "joy",
    "cool",
    "brilliant",
    "thank",
    "thanks",
    "appreciate",
    "beautiful",
    "perfect",
    "ideal",
    "superb",
    "outstanding",
    "impressive",
    "enjoy",
    "fun",
    "yes",
    "win",
    "won",
    "celebrate",
    "congrats",
    "congratulations",
    "well done",
    "welldone",
    "spot on",
    "correct",
    "right",
    "smart",
    "wise",
    "kind",
    "friendly",
    "helpful",
    "support",
    "bless",
    "marvelous",
    "pleasure",
    "delighted",
    "thrilled",
    "excited",
    "yay",
    "😊",
    "❤️",
    "🔥",
    "💯",
    "👏",
];

pub(crate) const NEGATIVE_WORDS: &[&str] = &[
    "bad",
    "terrible",
    "awful",
    "worst",
    "hate",
    "dislike",
    "horrible",
    "disgusting",
    "poor",
    "sad",
    "angry",
    "mad",
    "upset",
    "annoyed",
    "disappointed",
    "fail",
    "failed",
    "failure",
    "wrong",
    "error",
    "mistake",
    "bug",
    "broken",
    "useless",
    "waste",
    "sucks",
    "sucked",
    "suck",
    "hell",
    "shit",
    "damn",
    "fuck",
    "fucking",
    "idiot",
    "stupid",
    "dumb",
    "ridiculous",
    "absurd",
    "fake",
    "scam",
    "liar",
    "lies",
    "lying",
    "toxic",
    "abuse",
    "abusive",
    "harassment",
    "harassing",
    "block",
    "report",
    "spam",
    "spammer",
    "clown",
    "joke",
    "pathetic",
    "disaster",
    "mess",
    "nightmare",
    "regret",
    "depressing",
    "depressed",
    "anxious",
    "anxiety",
    "cry",
    "crying",
    "😢",
    "😡",
    "💩",
];

// ============================================================================
// Contextual Analysis Helpers
// ============================================================================

/// Calculate context-aware sentiment score for a word.
pub(crate) fn calculate_contextual_score(text: &str, base_score: f32, target_word: &str) -> f32 {
    let mut score = base_score;
    let multiplier = get_intensifier_multiplier(text, target_word);
    score *= multiplier;
    if is_negated(text, target_word) {
        score = -score;
    }
    score
}

pub(crate) fn is_negated(text: &str, target_word: &str) -> bool {
    let words: Vec<&str> = text.split_whitespace().collect();
    let target_lower = target_word.to_lowercase();
    for (i, word) in words.iter().enumerate() {
        if word.to_lowercase() == target_lower {
            let start = i.saturating_sub(3);
            if words
                .iter()
                .take(i)
                .skip(start)
                .any(|prev| NEGATION_PATTERNS.iter().any(|&n| prev.to_lowercase() == n))
            {
                return true;
            }
        }
    }
    false
}

pub(crate) fn get_intensifier_multiplier(text: &str, target_word: &str) -> f32 {
    let words: Vec<&str> = text.split_whitespace().collect();
    let target_lower = target_word.to_lowercase();
    for (i, word) in words.iter().enumerate() {
        if word.to_lowercase() == target_lower {
            let start = i.saturating_sub(2);
            if let Some((_, multiplier)) = words.iter().take(i).skip(start).find_map(|prev| {
                INTENSIFIERS
                    .iter()
                    .find(|(intensifier, _)| prev.to_lowercase() == *intensifier)
            }) {
                return *multiplier;
            }
        }
    }
    1.0
}

pub(crate) fn analyze_contextual_modifiers(text: &str) -> f32 {
    let mut modifier = 0.0;
    if has_sarcasm_markers(text) {
        modifier -= 2.0;
    }
    if is_excessive_punctuation(text) {
        modifier -= 0.5;
    }
    modifier
}

pub(crate) fn has_sarcasm_markers(text: &str) -> bool {
    let lower = text.to_lowercase();
    SARCASM_PATTERNS
        .iter()
        .any(|&pattern| lower.contains(pattern))
}

pub(crate) fn is_excessive_punctuation(text: &str) -> bool {
    let exclamation_count = text.matches('!').count();
    let question_count = text.matches('?').count();
    text.contains("?!") || text.contains("!?") || exclamation_count > 2 || question_count > 2
}

// ============================================================================
// Score Conversion Helpers
// ============================================================================

#[must_use]
pub fn sentiment_score(sentiment: Sentiment) -> i32 {
    match sentiment {
        Sentiment::Positive => 1,
        Sentiment::Neutral => 0,
        Sentiment::Negative => -1,
    }
}

pub(crate) fn sentiment_to_score(s: Sentiment) -> f32 {
    match s {
        Sentiment::Positive => 1.0,
        Sentiment::Neutral => 0.0,
        Sentiment::Negative => -1.0,
    }
}

pub(crate) fn score_to_sentiment(score: f32) -> Sentiment {
    if score > 0.3 {
        Sentiment::Positive
    } else if score < -0.3 {
        Sentiment::Negative
    } else {
        Sentiment::Neutral
    }
}

// ============================================================================
// Tweet-Level Analysis
// ============================================================================

// ============================================================================
// Feed-Level Analysis
// ============================================================================

#[must_use]
pub fn feed_sentiment_score(stats: &SentimentStats) -> f64 {
    let total = f64::from(stats.total());
    if total == 0.0 {
        return 0.0;
    }
    (f64::from(stats.positive) / total) - (f64::from(stats.negative) / total)
}

// ============================================================================
// Standalone Public API
// ============================================================================

#[must_use]
pub fn analyze_sentiment_sync(text: &str) -> Sentiment {
    super::core::SentimentAnalyzer::new().analyze_sentiment_sync(text)
}

#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn extract_thread_context(tweet_obj: &Value) -> Option<ThreadContext> {
    let reply_count = tweet_obj
        .get("replies")
        .and_then(|v| v.as_array())
        .map_or(0, |a| a.len() as u32);
    let mut reply_scores = Vec::new();
    if let Some(replies) = tweet_obj.get("replies").and_then(|v| v.as_array()) {
        for reply in replies {
            if let Some(text) = reply.get("text").and_then(|v| v.as_str()) {
                reply_scores.push(sentiment_to_score(analyze_sentiment_sync(text)));
            }
        }
    }
    let avg_reply_sentiment = if reply_scores.is_empty() {
        0.0
    } else {
        reply_scores.iter().sum::<f32>() / reply_scores.len() as f32
    };
    let tweet_text = extract_tweet_text(tweet_obj);
    Some(ThreadContext {
        reply_count,
        avg_reply_sentiment,
        is_reply: false,
        is_quote: false,
        thread_depth: 0,
        conversation_indicators: detect_conversation_indicators(&tweet_text),
    })
}

/// Compute a prestige/authority score (0.0–1.0) from author metadata.
///
/// Uses follower count, verification status, and account age to estimate
/// the author's credibility and influence. Called by `extract_user_reputation()`.
///
/// # Arguments
/// * `author` - JSON object containing author fields (followers_count, is_verified, created_at)
#[must_use]
pub fn author_prestige(author: &Value) -> f32 {
    let follower_count = author
        .get("followers_count")
        .and_then(|v| v.as_u64())
        .map(|c| c as u32)
        .or_else(|| {
            author
                .get("follower_count")
                .and_then(|v| v.as_u64())
                .map(|c| c as u32)
        })
        .unwrap_or(0);

    let is_verified = author
        .get("is_verified")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let account_age_days = if let Some(created_at) = author
        .get("created_at")
        .or_else(|| author.get("account_created_at"))
        .and_then(|v| v.as_str())
    {
        parse_timestamp_to_age_days(created_at)
    } else {
        365 // Default: assume 1 year old
    };

    compute_trust_score(follower_count, is_verified, account_age_days)
}

/// Extracts user reputation from a tweet object.
///
/// Reads author metadata from the tweet JSON to build a reputation profile.
/// Falls back to sensible defaults when fields are missing.
#[must_use]
pub fn extract_user_reputation(tweet_obj: &Value) -> Option<UserReputation> {
    // Try to extract author data — may be nested under "user" or "author",
    // or flattened at the tweet level for scraped data.
    let author = tweet_obj
        .get("user")
        .or_else(|| tweet_obj.get("author"))
        .unwrap_or(tweet_obj);

    let follower_count = author
        .get("followers_count")
        .and_then(|v| v.as_u64())
        .map(|c| c as u32)
        .or_else(|| {
            author
                .get("follower_count")
                .and_then(|v| v.as_u64())
                .map(|c| c as u32)
        })
        .unwrap_or(0);

    let is_verified = author
        .get("is_verified")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let account_age_days = if let Some(created_at) = author
        .get("created_at")
        .or_else(|| author.get("account_created_at"))
        .and_then(|v| v.as_str())
    {
        parse_timestamp_to_age_days(created_at)
    } else {
        365 // Default: assume 1 year old
    };

    let engagement_rate = compute_engagement_rate(author);
    let is_influential = follower_count > 10_000 || is_verified;
    let trust_score = author_prestige(author);

    Some(UserReputation {
        follower_count,
        is_verified,
        account_age_days,
        engagement_rate,
        is_influential,
        trust_score,
    })
}

/// Compute a rough engagement rate from author metrics.
fn compute_engagement_rate(author: &Value) -> f32 {
    let followers = author
        .get("followers_count")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as f32;
    let total_likes = author
        .get("total_likes")
        .or_else(|| author.get("likes_count"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as f32;
    let total_tweets = author
        .get("total_tweets")
        .or_else(|| author.get("statuses_count"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as f32;

    if followers < 1.0 || total_tweets < 1.0 {
        return 0.05; // Default moderate engagement
    }

    // Engagement rate = total likes / (total tweets * followers)
    // Clamp to reasonable range
    let rate = total_likes / (total_tweets * followers);
    rate.clamp(0.001_f32, 0.5_f32)
}

/// Compute a trust score (0.0–1.0) from reputation signals.
fn compute_trust_score(follower_count: u32, is_verified: bool, account_age_days: u32) -> f32 {
    let mut score: f32 = 0.5; // Start neutral

    // Verification bonus
    if is_verified {
        score += 0.25;
    }

    // Follower count signal
    if follower_count > 100_000 {
        score += 0.1;
    } else if follower_count > 10_000 {
        score += 0.05;
    } else if follower_count < 10 {
        score -= 0.15;
    }

    // Account age signal
    if account_age_days > 1095 {
        // >3 years
        score += 0.1;
    } else if account_age_days > 365 {
        // >1 year
        score += 0.05;
    } else if account_age_days < 30 {
        // <1 month
        score -= 0.2;
    } else if account_age_days < 90 {
        // <3 months
        score -= 0.1;
    }

    score.clamp(0.0, 1.0)
}

/// Parse an ISO 8601 timestamp string into account age in days.
fn parse_timestamp_to_age_days(timestamp: &str) -> u32 {
    // Parse the date portion (YYYY-MM-DD) and compute elapsed days.
    // Avoids chrono API complexity — works with basic ISO formats.
    let date_str = timestamp.split('T').next().unwrap_or(timestamp);
    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    // Parse "YYYY-MM-DD" manually to Unix seconds
    if let Some(parsed_secs) = parse_iso_date_to_unix(date_str) {
        let age_secs = (now_secs - parsed_secs).max(0) as u64;
        return (age_secs / 86400) as u32;
    }

    365 // Default: assume 1 year old
}

/// Parse "YYYY-MM-DD" into Unix timestamp seconds.
fn parse_iso_date_to_unix(date_str: &str) -> Option<i64> {
    let parts: Vec<&str> = date_str.split('-').collect();
    if parts.len() != 3 {
        return None;
    }
    let year: i32 = parts[0].parse().ok()?;
    let month: u32 = parts[1].parse().ok()?;
    let day: u32 = parts[2].parse().ok()?;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    // Simple Unix timestamp calculation (days since epoch)
    let days = days_from_epoch(year, month, day);
    Some(days * 86400)
}

/// Compute days since Unix epoch for a given date.
fn days_from_epoch(year: i32, month: u32, day: u32) -> i64 {
    let y = year as i64;
    let m = month as i64;
    let d = day as i64;
    // Zeller-style calculation
    let era = if y >= 0 { y } else { y - 399 };
    let yoe = (era as u64).wrapping_rem(400) as i64;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + yoe / 400;

    era / 400 * 146097 + (doe + doy) - 719468
}

/// Compute a recency/freshness score (0.0–1.0) from a tweet timestamp.
///
/// 1.0 = just posted, 0.0 = very old (>7 days). Uses linear decay
/// to give higher weight to recent tweets. Called by `extract_temporal_factors()`.
///
/// # Arguments
/// * `tweet_obj` - JSON object containing timestamp/created_at/post_time fields
#[must_use]
pub fn tweet_recency(tweet_obj: &Value) -> f32 {
    let timestamp_str = tweet_obj
        .get("timestamp")
        .or_else(|| tweet_obj.get("created_at"))
        .or_else(|| tweet_obj.get("post_time"))
        .and_then(|v| v.as_str());

    let hours_since_post = if let Some(ts) = timestamp_str {
        let date_str = ts.split('T').next().unwrap_or(ts);
        if let Some(post_secs) = parse_iso_date_to_unix(date_str) {
            let now_secs = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;
            ((now_secs - post_secs).max(0) as f64 / 3600.0) as f32
        } else {
            24.0
        }
    } else {
        24.0
    };

    // Linear decay over 7 days: 1 hour → 0.99, 6 hours → 0.96,
    // 24 hours → 0.86, 3 days → 0.57, 7 days → 0.0
    if hours_since_post <= 0.0 {
        return 1.0;
    }
    let recency = 1.0 - (hours_since_post / 168.0).clamp(0.0, 1.0); // 168 = 7 days in hours
    recency.clamp(0.0, 1.0)
}

/// Extracts temporal factors from a tweet object.
///
/// Reads the tweet timestamp and computes time-of-day, day-of-week,
/// recency, and peak-hour signals.
#[must_use]
pub fn extract_temporal_factors(tweet_obj: &Value) -> Option<TemporalFactors> {
    let timestamp_str = tweet_obj
        .get("timestamp")
        .or_else(|| tweet_obj.get("created_at"))
        .or_else(|| tweet_obj.get("post_time"))
        .and_then(|v| v.as_str());

    let (hour_of_day, day_of_week, hours_since_post) = if let Some(ts) = timestamp_str {
        parse_temporal_from_timestamp(ts)
    } else {
        (12, 1, 24.0)
    };

    let is_peak_hour = matches!(hour_of_day, 6..=9 | 12..=13 | 17..=19);
    let trending_bias = compute_trending_bias(tweet_obj);
    let recency = tweet_recency(tweet_obj);

    Some(TemporalFactors {
        hour_of_day,
        day_of_week,
        hours_since_post,
        is_peak_hour,
        trending_bias,
        recency,
    })
}

/// Parse timestamp string into (hour, day_of_week, hours_since_post).
///
/// Note: Timezone offsets are intentionally ignored — peak-hour detection
/// uses local time which is more relevant for user behavior than UTC.
fn parse_temporal_from_timestamp(ts: &str) -> (u8, u8, f32) {
    // Extract date and time portions from ISO 8601
    let date_str = ts.split('T').next().unwrap_or(ts);
    let time_str = ts
        .split('T')
        .nth(1)
        .map(|t| t.split(['+', '-', 'Z']).next().unwrap_or(t))
        .unwrap_or("12:00:00");

    // Parse hour
    let hour = time_str
        .split(':')
        .next()
        .and_then(|h| h.parse::<u8>().ok())
        .unwrap_or(12);

    // Parse date parts for day of week
    let parts: Vec<&str> = date_str.split('-').collect();
    let dow = if parts.len() == 3 {
        if let (Ok(y), Ok(m), Ok(d)) = (
            parts[0].parse::<i32>(),
            parts[1].parse::<u32>(),
            parts[2].parse::<u32>(),
        ) {
            day_of_week_from_date(y, m, d)
        } else {
            1
        }
    } else {
        1
    };

    // Compute hours since post
    let hours_since = if let Some(post_secs) = parse_iso_date_to_unix(date_str) {
        let now_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        ((now_secs - post_secs).max(0) as f64 / 3600.0) as f32
    } else {
        24.0
    };

    (hour, dow, hours_since)
}

/// Compute day of week (0=Sun, 6=Sat) from year, month, day.
fn day_of_week_from_date(year: i32, month: u32, day: u32) -> u8 {
    let epoch_days = days_from_epoch(year, month, day);
    // Jan 1, 1970 was a Thursday (4)
    ((epoch_days + 4).rem_euclid(7)) as u8
}

// Re-export the canonical compute_trending_bias and detect_conversation_indicators
// from the shared state module (re-exported from state::types).
pub(crate) use crate::utils::twitter::state::compute_trending_bias;
pub use crate::utils::twitter::state::detect_conversation_indicators;

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ========================================================================
    // author_prestige() Tests
    // ========================================================================

    #[test]
    fn test_author_prestige_verified_influencer() {
        // Verified account with >100k followers and 5-year-old account
        let author = json!({
            "followers_count": 250_000,
            "is_verified": true,
            "created_at": "2021-06-10T12:00:00Z"
        });
        let score = author_prestige(&author);
        // Base 0.5 + verified 0.25 + >100k 0.1 + old account (>3y) 0.1 = 0.95
        assert!(
            score > 0.8,
            "Verified influencer should have high prestige, got {score}"
        );
        assert!(score <= 1.0, "Prestige should not exceed 1.0, got {score}");
    }

    #[test]
    fn test_author_prestige_unverified_zero_followers() {
        // Unverified account with 0 followers and 1-year-old account
        let author = json!({
            "followers_count": 0,
            "is_verified": false,
            "created_at": "2025-06-09T12:00:00Z"
        });
        let score = author_prestige(&author);
        // Base 0.5 + <10 followers -0.15 + >1y +0.05 ≈ 0.40
        assert!(
            score < 0.55,
            "Zero-follower account should have low prestige, got {score}"
        );
        assert!(
            score >= 0.0,
            "Prestige should not go below 0.0, got {score}"
        );
    }

    #[test]
    fn test_author_prestige_moderate_account() {
        // Unverified, 5000 followers, 2-year-old account
        let author = json!({
            "followers_count": 5_000,
            "is_verified": false,
            "created_at": "2024-06-10T12:00:00Z"
        });
        let score = author_prestige(&author);
        // Base 0.5 + 5000 followers (neither >10k nor <10) +0.0 + >1y +0.05 = 0.55
        assert!(
            score > 0.45 && score < 0.65,
            "Moderate account should be mid-range, got {score}"
        );
    }

    #[test]
    fn test_author_prestige_brand_new_account() {
        // Brand new account (<30 days)
        let author = json!({
            "followers_count": 5,
            "is_verified": false,
            "created_at": "2026-06-09T12:00:00Z"
        });
        let score = author_prestige(&author);
        // Base 0.5 + <10 followers -0.15 + <30d -0.2 = 0.15
        assert!(
            score < 0.3,
            "Brand-new account should have very low prestige, got {score}"
        );
    }

    #[test]
    fn test_author_prestige_empty_json() {
        // Empty JSON — all fields missing, should use defaults
        let author = json!({});
        let score = author_prestige(&author);
        // Defaults: followers=0, not verified, age=365d
        // Base 0.5 + <10 -0.15 + >1y +0.05 = 0.4
        assert!(
            score < 0.55 && score > 0.2,
            "Empty JSON should give default mid-low score, got {score}"
        );
    }

    #[test]
    fn test_author_prestige_follower_count_field_only() {
        // Only follower_count specified (alternate field name)
        let author = json!({
            "follower_count": 50_000
        });
        let score = author_prestige(&author);
        // Base 0.5 + >10k +0.05 + default age(365d) +0.05 = 0.6
        assert!(
            score > 0.5,
            "Should get moderate score from follower_count field, got {score}"
        );
    }

    #[test]
    fn test_author_prestige_just_below_influencer() {
        // 9,999 followers — just under the 10k threshold
        let author = json!({
            "followers_count": 9_999,
            "is_verified": false,
            "created_at": "2024-06-10T12:00:00Z"
        });
        let score = author_prestige(&author);
        // Base 0.5 + <10k but not <10 +0.0 + >1y +0.05 = 0.55
        assert!(
            (score - 0.55).abs() < 0.1,
            "Just-below-influencer should be ~0.55, got {score}"
        );
    }

    #[test]
    fn test_author_prestige_verified_but_no_followers() {
        // Verified but brand-new with no followers
        let author = json!({
            "followers_count": 1,
            "is_verified": true,
            "created_at": "2026-06-09T12:00:00Z"
        });
        let score = author_prestige(&author);
        // Base 0.5 + verified 0.25 + <10 -0.15 + <30d -0.2 = 0.4
        // Verification helps but doesn't fully offset newness and low followers
        assert!(
            score > 0.3 && score < 0.7,
            "Verified new account should be mid-range, got {score}"
        );
    }

    #[test]
    fn test_author_prestige_very_old_account() {
        // 10-year-old account with moderate followers
        let author = json!({
            "followers_count": 15_000,
            "is_verified": false,
            "created_at": "2016-06-10T12:00:00Z"
        });
        let score = author_prestige(&author);
        // Base 0.5 + >10k +0.05 + >3y +0.1 = 0.65
        assert!(
            score > 0.55,
            "Old account should get age bonus, got {score}"
        );
    }

    #[test]
    fn test_author_prestige_account_30_to_90_days() {
        // Account ~60 days old — should get the -0.1 penalty branch
        let author = json!({
            "followers_count": 500,
            "is_verified": false,
            "created_at": "2026-04-11T12:00:00Z"
        });
        let score = author_prestige(&author);
        // Base 0.5 + 500 followers (neither >10k nor <10) +0.0 + <90d -0.1 = 0.4
        assert!(
            score < 0.5,
            "30-90 day account should have penalty, got {score}"
        );
    }

    #[test]
    fn test_author_prestige_account_created_at_field() {
        // Uses "account_created_at" alternate field name
        let author = json!({
            "followers_count": 2_000,
            "is_verified": false,
            "account_created_at": "2024-06-10T12:00:00Z"
        });
        let score = author_prestige(&author);
        // Base 0.5 + >1y +0.05 = 0.55
        assert!(
            score > 0.45,
            "account_created_at field should be recognized, got {score}"
        );
    }

    // ========================================================================
    // tweet_recency() Tests
    // ========================================================================

    #[test]
    fn test_tweet_recency_very_old_tweet() {
        // Tweet from 2020 — definitely >7 days old, should score 0.0
        let tweet = json!({
            "created_at": "2020-01-15T10:30:00Z"
        });
        let score = tweet_recency(&tweet);
        assert!(
            score < 0.01,
            "Very old tweet should have near-zero recency, got {score}"
        );
    }

    #[test]
    fn test_tweet_recency_missing_timestamp() {
        // No timestamp fields at all — should use default 24h
        let tweet = json!({
            "text": "just a tweet with no timestamp",
            "author": "user1"
        });
        let score = tweet_recency(&tweet);
        // 24h default → 1.0 - 24/168 ≈ 0.857
        assert!(
            (score - 0.857).abs() < 0.01,
            "Missing timestamp should use 24h default (~0.857), got {score}"
        );
    }

    #[test]
    fn test_tweet_recency_unparseable_timestamp() {
        // Timestamp in unknown format — parsing should fail, use default 24h
        let tweet = json!({
            "timestamp": "not-a-real-date"
        });
        let score = tweet_recency(&tweet);
        // Default 24h → ~0.857
        assert!(
            score > 0.8 && score < 0.9,
            "Unparseable timestamp should use default, got {score}"
        );
    }

    #[test]
    fn test_tweet_recency_post_time_field() {
        // Uses "post_time" field (alternate name for timestamp)
        let tweet = json!({
            "post_time": "2020-01-15T10:30:00Z"
        });
        let score = tweet_recency(&tweet);
        assert!(
            score < 0.01,
            "Very old tweet via post_time should have near-zero recency, got {score}"
        );
    }

    #[test]
    fn test_tweet_recency_empty_json() {
        // Completely empty tweet object
        let tweet = json!({});
        let score = tweet_recency(&tweet);
        // Default 24h → ~0.857
        assert!(
            score > 0.8,
            "Empty tweet should use default (~0.857), got {score}"
        );
    }

    #[test]
    fn test_tweet_recency_range_check() {
        // Verify the score is always clamped to [0.0, 1.0]
        let very_old = json!({"created_at": "2010-01-01T00:00:00Z"});
        let score = tweet_recency(&very_old);
        assert!(score >= 0.0, "Recency should not go below 0.0, got {score}");
        assert!(score <= 1.0, "Recency should not exceed 1.0, got {score}");
    }

    #[test]
    fn test_tweet_recency_timestamp_field() {
        // Uses "timestamp" field (primary field name)
        let tweet = json!({
            "timestamp": "2020-01-15T10:30:00Z"
        });
        let score = tweet_recency(&tweet);
        assert!(
            score < 0.01,
            "Very old tweet via timestamp should have near-zero recency, got {score}"
        );
    }

    #[test]
    fn test_tweet_recency_date_only_no_time() {
        // Date without time portion (no 'T' separator)
        let tweet = json!({
            "created_at": "2020-01-15"
        });
        let score = tweet_recency(&tweet);
        assert!(
            score < 0.01,
            "Very old date-only tweet should have near-zero recency, got {score}"
        );
    }
}
