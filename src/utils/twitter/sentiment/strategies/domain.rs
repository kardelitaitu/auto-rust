//! Domain-specific sentiment analysis strategy.

use serde::{Deserialize, Serialize};

/// Domain types for sentiment analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash, Default)]
pub enum SentimentDomain {
    #[default]
    General,
    Tech,
    Crypto,
    Gaming,
    Sports,
    Entertainment,
}

// Tech keywords
const TECH_POSITIVE: &[&str] = &[
    "shipping",
    "shipped",
    "launched",
    "deployed",
    "released",
    "live in prod",
    "clean code",
    "elegant solution",
    "optimized",
    "performance boost",
    "tests passing",
    "ci green",
    "pr approved",
    "code review passed",
    "open source",
    "oss",
    "contributor",
    "maintainer",
];
const TECH_NEGATIVE: &[&str] = &[
    "bug",
    "regression",
    "outage",
    "downtime",
    "incident",
    "hotfix",
    "technical debt",
    "tech debt",
    "spaghetti code",
    "hack",
    "brittle",
    "ci failed",
    "build broken",
    "tests failing",
    "critical bug",
];

// Crypto keywords
const CRYPTO_POSITIVE: &[&str] = &[
    "moon",
    "pump",
    "gains",
    "ath",
    "bullish",
    "bull run",
    "parabolic",
    "hodl",
    "diamond hands",
    "accumulation",
    "buying the dip",
    "mainnet",
    "testnet",
    "upgrade",
    "adoption",
    "staking",
    "defi",
];
const CRYPTO_NEGATIVE: &[&str] = &[
    "rekt",
    "dump",
    "crash",
    "bearish",
    "bear market",
    "liquidation",
    "rug pull",
    "scam",
    "exit scam",
    "honeypot",
    "fud",
];

// Gaming keywords
const GAMING_POSITIVE: &[&str] = &[
    "epic win",
    "victory",
    "clutch",
    "pentakill",
    "legendary",
    "mvp",
    "achievement unlocked",
    "trophy",
    "platinum",
    "perfect run",
    "level up",
    "rank up",
    "speedrun",
    "pb",
    "goty",
];
const GAMING_NEGATIVE: &[&str] = &[
    "game over",
    "wipe",
    "defeat",
    "loss streak",
    "throw",
    "ff",
    "surrender",
    "lag",
    "disconnect",
    "server down",
    "cheater",
    "hacker",
    "fps drop",
];

// Sports keywords
const SPORTS_POSITIVE: &[&str] = &[
    "victory",
    "won",
    "champion",
    "championship",
    "trophy",
    "gold medal",
    "playoffs",
    "finals",
    "touchdown",
    "home run",
    "hat trick",
    "goat",
];
const SPORTS_NEGATIVE: &[&str] = &[
    "lost",
    "defeat",
    "eliminated",
    "relegated",
    "blown lead",
    "choke",
    "injured",
    "out for season",
    "career ending",
];

// Entertainment keywords
const ENT_POSITIVE: &[&str] = &[
    "masterpiece",
    "must watch",
    "binge worthy",
    "phenomenal",
    "oscar",
    "emmy",
    "hit song",
    "banger",
    "live performance",
    "viral",
    "trending",
];
const ENT_NEGATIVE: &[&str] = &[
    "bomb",
    "flop",
    "disappointing",
    "waste of time",
    "boring",
    "cancelled",
    "rotten tomatoes",
    "bad reviews",
    "one star",
];

/// Detect domain from tweet content.
pub fn detect_domain(text: &str) -> SentimentDomain {
    let lower = text.to_lowercase();

    let crypto_indicators = ["btc", "eth", "crypto", "bitcoin", "ethereum", "defi", "nft"];
    let tech_indicators = [
        "code",
        "dev",
        "programming",
        "software",
        "github",
        "pr",
        "deploy",
    ];
    let gaming_indicators = [
        "gaming", "game", "twitch", "esports", "streamer", "valorant",
    ];
    let sports_indicators = ["nfl", "nba", "mlb", "soccer", "football", "basketball"];
    let ent_indicators = ["movie", "film", "netflix", "tv show", "album", "concert"];

    let mut scores = [
        (
            SentimentDomain::Crypto,
            count_matches(&lower, &crypto_indicators),
        ),
        (
            SentimentDomain::Tech,
            count_matches(&lower, &tech_indicators),
        ),
        (
            SentimentDomain::Gaming,
            count_matches(&lower, &gaming_indicators),
        ),
        (
            SentimentDomain::Sports,
            count_matches(&lower, &sports_indicators),
        ),
        (
            SentimentDomain::Entertainment,
            count_matches(&lower, &ent_indicators),
        ),
    ];

    scores.sort_by_key(|b| std::cmp::Reverse(b.1));

    if scores[0].1 >= 1 {
        scores[0].0
    } else {
        SentimentDomain::General
    }
}

fn count_matches(text: &str, keywords: &[&str]) -> usize {
    keywords.iter().filter(|&&w| text.contains(w)).count()
}

/// Analyze sentiment with domain-specific keywords.
pub fn analyze_domain_sentiment(text: &str, domain: SentimentDomain) -> f32 {
    let lower = text.to_lowercase();
    let (positive, negative): (&[&str], &[&str]) = match domain {
        SentimentDomain::Tech => (TECH_POSITIVE, TECH_NEGATIVE),
        SentimentDomain::Crypto => (CRYPTO_POSITIVE, CRYPTO_NEGATIVE),
        SentimentDomain::Gaming => (GAMING_POSITIVE, GAMING_NEGATIVE),
        SentimentDomain::Sports => (SPORTS_POSITIVE, SPORTS_NEGATIVE),
        SentimentDomain::Entertainment => (ENT_POSITIVE, ENT_NEGATIVE),
        SentimentDomain::General => (&[], &[]),
    };

    let mut score = 0.0;
    for &word in positive {
        if lower.contains(word) {
            score += 1.5;
        }
    }
    for &word in negative {
        if lower.contains(word) {
            score -= 1.5;
        }
    }
    score
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // detect_domain Tests
    // ========================================================================

    #[test]
    fn test_detect_domain_general() {
        assert_eq!(detect_domain("I like pizza"), SentimentDomain::General);
    }

    #[test]
    fn test_detect_domain_general_empty() {
        assert_eq!(detect_domain(""), SentimentDomain::General);
    }

    #[test]
    fn test_detect_domain_crypto() {
        assert_eq!(detect_domain("Bitcoin is mooning"), SentimentDomain::Crypto);
        assert_eq!(detect_domain("I love ETH"), SentimentDomain::Crypto);
        assert_eq!(detect_domain("NFT market is hot"), SentimentDomain::Crypto);
    }

    #[test]
    fn test_detect_domain_tech() {
        assert_eq!(detect_domain("writing clean code"), SentimentDomain::Tech);
        assert_eq!(detect_domain("deploy to production"), SentimentDomain::Tech);
        assert_eq!(detect_domain("software engineering"), SentimentDomain::Tech);
    }

    #[test]
    fn test_detect_domain_gaming() {
        assert_eq!(
            detect_domain("streaming on twitch"),
            SentimentDomain::Gaming
        );
        assert_eq!(detect_domain("new game release"), SentimentDomain::Gaming);
        assert_eq!(detect_domain("esports tournament"), SentimentDomain::Gaming);
    }

    #[test]
    fn test_detect_domain_sports() {
        assert_eq!(detect_domain("NFL season is here"), SentimentDomain::Sports);
        assert_eq!(detect_domain("NBA finals"), SentimentDomain::Sports);
        assert_eq!(detect_domain("soccer match"), SentimentDomain::Sports);
    }

    #[test]
    fn test_detect_domain_entertainment() {
        assert_eq!(
            detect_domain("watching netflix"),
            SentimentDomain::Entertainment
        );
        assert_eq!(
            detect_domain("new movie review"),
            SentimentDomain::Entertainment
        );
        assert_eq!(
            detect_domain("tv show binge"),
            SentimentDomain::Entertainment
        );
    }

    #[test]
    fn test_detect_domain_case_insensitive() {
        assert_eq!(detect_domain("BITCOIN"), SentimentDomain::Crypto);
        assert_eq!(detect_domain("Code Review"), SentimentDomain::Tech);
        assert_eq!(detect_domain("GAME"), SentimentDomain::Gaming);
    }

    #[test]
    fn test_detect_domain_crypto_preferred_over_tech() {
        // "defi" appears in crypto and tech might also match "code", but crypto should win
        // since crypto indicators are checked first via scores sorting
        assert_eq!(
            detect_domain("defi protocol launch"),
            SentimentDomain::Crypto
        );
    }

    #[test]
    fn test_detect_domain_multiple_indicators() {
        // Sports has more matches
        assert_eq!(
            detect_domain("NBA basketball game tonight"),
            SentimentDomain::Sports
        );
    }

    // ========================================================================
    // analyze_domain_sentiment Tests
    // ========================================================================

    #[test]
    fn test_analyze_general_domain_no_sentiment() {
        let score = analyze_domain_sentiment("Any text at all", SentimentDomain::General);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_analyze_tech_positive() {
        // "tests passing" appears in TECH_POSITIVE
        let score = analyze_domain_sentiment("All tests passing finally", SentimentDomain::Tech);
        assert_eq!(score, 1.5);
    }

    #[test]
    fn test_analyze_tech_negative() {
        // "bug" and "critical bug" both match in lowercase
        let score = analyze_domain_sentiment("Critical bug in production", SentimentDomain::Tech);
        assert_eq!(score, -3.0);
    }

    #[test]
    fn test_analyze_tech_multiple_matches() {
        let score = analyze_domain_sentiment(
            "just shipped clean code, pr approved",
            SentimentDomain::Tech,
        );
        // shipped(1.5) + clean code(1.5) + pr approved(1.5) = 4.5
        assert_eq!(score, 4.5);
    }

    #[test]
    fn test_analyze_crypto_positive() {
        let score = analyze_domain_sentiment("Bitcoin is bullish", SentimentDomain::Crypto);
        assert_eq!(score, 1.5);
    }

    #[test]
    fn test_analyze_crypto_negative() {
        let score = analyze_domain_sentiment("That project is a scam", SentimentDomain::Crypto);
        assert_eq!(score, -1.5);
    }

    #[test]
    fn test_analyze_gaming_positive() {
        let score = analyze_domain_sentiment("What a clutch victory", SentimentDomain::Gaming);
        // victory(1.5) + clutch(1.5) = 3.0
        assert_eq!(score, 3.0);
    }

    #[test]
    fn test_analyze_sports_negative() {
        // "lost" and "eliminated" each match once
        let score = analyze_domain_sentiment("Team lost and eliminated", SentimentDomain::Sports);
        assert_eq!(score, -3.0);
    }

    #[test]
    fn test_analyze_sports_negative_single() {
        let score = analyze_domain_sentiment("that team always chokes", SentimentDomain::Sports);
        assert_eq!(score, -1.5);
    }

    #[test]
    fn test_analyze_sports_positive() {
        let score = analyze_domain_sentiment(
            "What a victory! Championship bound!",
            SentimentDomain::Sports,
        );
        // "victory"(1.5) + "champion"(1.5, within "championship") + "championship"(1.5) = 4.5
        assert_eq!(score, 4.5);
    }

    #[test]
    fn test_analyze_sports_positive_single() {
        let score = analyze_domain_sentiment("victory royale", SentimentDomain::Sports);
        assert_eq!(score, 1.5);
    }

    #[test]
    fn test_analyze_entertainment_positive() {
        let score = analyze_domain_sentiment(
            "That movie was a masterpiece",
            SentimentDomain::Entertainment,
        );
        assert_eq!(score, 1.5);
    }

    #[test]
    fn test_analyze_entertainment_negative() {
        let score = analyze_domain_sentiment("What a boring flop", SentimentDomain::Entertainment);
        // boring(1.5) + flop(1.5) = -3.0
        assert_eq!(score, -3.0);
    }

    #[test]
    fn test_analyze_mixed_sentiment() {
        let score = analyze_domain_sentiment("Clean code but still a bug", SentimentDomain::Tech);
        // clean code(1.5) + bug(-1.5) = 0.0
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_analyze_no_match() {
        let score = analyze_domain_sentiment("The weather is nice today", SentimentDomain::Tech);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_analyze_empty_text() {
        let score = analyze_domain_sentiment("", SentimentDomain::Crypto);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_analyze_domain_sentiment_case_insensitive() {
        let score = analyze_domain_sentiment("ALL TESTS PASSING", SentimentDomain::Tech);
        assert_eq!(score, 1.5);
    }

    // ========================================================================
    // SentimentDomain Enum Tests
    // ========================================================================

    #[test]
    fn test_sentiment_domain_default() {
        let domain = SentimentDomain::default();
        assert_eq!(domain, SentimentDomain::General);
    }

    #[test]
    fn test_sentiment_domain_variants() {
        assert_eq!(SentimentDomain::General as u8, 0);
        assert_eq!(SentimentDomain::Tech as u8, 1);
        assert_eq!(SentimentDomain::Crypto as u8, 2);
        assert_eq!(SentimentDomain::Gaming as u8, 3);
        assert_eq!(SentimentDomain::Sports as u8, 4);
        assert_eq!(SentimentDomain::Entertainment as u8, 5);
    }

    #[test]
    fn test_sentiment_domain_debug() {
        assert_eq!(format!("{:?}", SentimentDomain::Tech), "Tech");
        assert_eq!(format!("{:?}", SentimentDomain::Crypto), "Crypto");
    }

    #[test]
    fn test_sentiment_domain_serialize_roundtrip() {
        let domain = SentimentDomain::Gaming;
        let json = serde_json::to_string(&domain).expect("serialize");
        let restored: SentimentDomain = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(restored, domain);
    }

    // ========================================================================
    // count_matches Tests
    // ========================================================================

    #[test]
    fn test_count_matches_basic() {
        assert_eq!(
            count_matches("bitcoin and crypto", &["bitcoin", "crypto", "eth"]),
            2
        );
    }

    #[test]
    fn test_count_matches_no_matches() {
        assert_eq!(count_matches("hello world", &["foo", "bar"]), 0);
    }

    #[test]
    fn test_count_matches_all_match() {
        assert_eq!(count_matches("foo bar baz", &["foo", "bar", "baz"]), 3);
    }
}
