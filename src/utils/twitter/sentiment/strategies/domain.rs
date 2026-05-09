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
