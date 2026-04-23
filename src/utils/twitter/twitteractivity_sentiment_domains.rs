//! Domain-specific sentiment analysis for Twitter.
//! Provides keyword sets and detection for Tech, Crypto, Gaming, Sports, and Entertainment domains.

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

impl std::fmt::Display for SentimentDomain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SentimentDomain::General => write!(f, "General"),
            SentimentDomain::Tech => write!(f, "Tech"),
            SentimentDomain::Crypto => write!(f, "Crypto"),
            SentimentDomain::Gaming => write!(f, "Gaming"),
            SentimentDomain::Sports => write!(f, "Sports"),
            SentimentDomain::Entertainment => write!(f, "Entertainment"),
        }
    }
}

// ============================================================================
// Tech Twitter Keywords
// ============================================================================

const TECH_POSITIVE: &[&str] = &[
    // Shipping/Launch
    "shipping",
    "shipped",
    "launched",
    "deployed",
    "released",
    "going live",
    "live in prod",
    "production release",
    // Code Quality
    "clean code",
    "elegant solution",
    "beautiful code",
    "well designed",
    "refactored",
    "optimized",
    "performance boost",
    "scalable",
    "well architected",
    "solid architecture",
    // Development Wins
    "tests passing",
    "ci green",
    "build passing",
    "no bugs",
    "merged",
    "pr approved",
    "code review passed",
    "feature complete",
    "milestone reached",
    "ship it",
    // Technology Positive
    "upgrade",
    "migration successful",
    "new stack",
    "modern",
    "best practice",
    "robust",
    "maintainable",
    "documentation",
    "well documented",
    "great dx",
    "developer experience",
    "automation",
    "automated",
    "streamlined",
    // Business/Startup
    "funding",
    "series a",
    "series b",
    "ipo",
    "acquisition",
    "partnership",
    "customer win",
    "growth",
    "traction",
    "product market fit",
    "pmf",
    "revenue",
    "profitable",
    "unicorn",
    "valuation",
    "success",
    // Community
    "open source",
    "oss",
    "contributor",
    "maintainer",
    "community driven",
    "collaboration",
    "teamwork",
    "hackathon",
    "meetup",
    "conference",
    "talk",
];

const TECH_NEGATIVE: &[&str] = &[
    // Problems/Issues
    "bug",
    "regression",
    "outage",
    "downtime",
    "incident",
    "production issue",
    "hotfix",
    "firefighting",
    "on-call",
    "alert",
    "paged",
    "wake up",
    "middle of the night",
    // Code Quality Issues
    "technical debt",
    "tech debt",
    "spaghetti code",
    "hack",
    "workaround",
    "kludge",
    "brittle",
    "fragile",
    "legacy",
    "messy",
    "ugly code",
    "anti-pattern",
    // Development Blockers
    "merge conflict",
    "ci failed",
    "build broken",
    "tests failing",
    "blocking issue",
    "showstopper",
    "critical bug",
    "blocked",
    "stuck",
    "can't ship",
    // Business/Startup Negative
    "layoffs",
    "shutdown",
    "pivot",
    "failed",
    "bankrupt",
    "burnout",
    "toxic culture",
    "bad management",
    "running out of money",
    "cash flow",
    // Technology Negative
    "deprecated",
    "end of life",
    "eol",
    "sunset",
    "breaking change",
    "migration hell",
    "vendor lock-in",
    "compatibility issue",
    "not backward compatible",
    // Security
    "vulnerability",
    "security issue",
    "data breach",
    "exploit",
    "zero-day",
    "patch tuesday",
    "cve",
    "hack",
    "hacked",
    // Performance
    "slow",
    "latency",
    "performance issue",
    "bottleneck",
    "memory leak",
    "cpu spike",
    "crash",
];

// ============================================================================
// Crypto Twitter Keywords
// ============================================================================

const CRYPTO_POSITIVE: &[&str] = &[
    // Price/Gains
    "moon",
    "to the moon",
    "pump",
    "gains",
    "green candle",
    "ath",
    "all time high",
    "breakout",
    "bullish",
    "bull run",
    "parabolic",
    "sending it",
    "number go up",
    // Holding Strategy
    "hodl",
    "diamond hands",
    "holding strong",
    "not selling",
    "accumulation",
    "accumulating",
    "buying the dip",
    "dca",
    "long term",
    "patient",
    "conviction",
    // Project Development
    "mainnet",
    "testnet",
    "upgrade",
    "hard fork",
    "soft fork",
    "partnership",
    "integration",
    "listing",
    "exchange listing",
    "adoption",
    "mass adoption",
    "institutional adoption",
    "roadmap",
    "delivering",
    "milestone",
    // Technology
    "staking",
    "yield farming",
    "defi",
    "liquidity",
    "tvl",
    "governance",
    "dao",
    "decentralized",
    "non-custodial",
    "layer 2",
    "scaling",
    "zk",
    "zero knowledge",
    // Community
    "community",
    "holders",
    "strong hands",
    "telegram",
    "discord",
    "twitter army",
    "shilling",
    "hype",
    "bullish community",
    // Positive News
    "announcement",
    "ama",
    "undervalued",
    "gem",
    "hidden gem",
    "100x",
    "1000x",
    "early",
    "ground floor",
];

const CRYPTO_NEGATIVE: &[&str] = &[
    // Price/Losses
    "rekt",
    "dump",
    "crash",
    "bearish",
    "bear market",
    "red candle",
    "loss",
    "liquidation",
    "margin call",
    "bag holder",
    "holding bags",
    "down bad",
    "ape'd in",
    "number go down",
    "bleeding",
    "blood",
    // Scams/Security
    "rug pull",
    "scam",
    "exit scam",
    "honeypot",
    "fake project",
    "exploit",
    "hack",
    "stolen",
    "drained",
    "compromised",
    "phishing",
    "social engineering",
    "private keys",
    "seed phrase",
    // Project Issues
    "delayed",
    "postponed",
    "no delivery",
    "vaporware",
    "empty promises",
    "overpromised",
    "fud",
    "uncertainty",
    "team sold",
    "dev dumped",
    "insider selling",
    "unlock",
    // Market Sentiment
    "capitulation",
    "crypto winter",
    "dead project",
    "ghost town",
    "abandoned",
    "no volume",
    "dying",
    // Regulatory
    "ban",
    "regulation",
    "sec lawsuit",
    "investigation",
    "crackdown",
    "illegal",
    "compliance issue",
    "delisted",
    // Trading
    "fomo",
    "fud",
    "whale dumping",
    "manipulation",
    "pump and dump",
    "exit liquidity",
    "jeet",
];

// ============================================================================
// Gaming Keywords
// ============================================================================

const GAMING_POSITIVE: &[&str] = &[
    // Wins/Achievements
    "epic win",
    "victory",
    "clutch",
    "pentakill",
    "ace",
    "legendary",
    "mvp",
    "potg",
    "play of the game",
    "achievement unlocked",
    "trophy",
    "platinum",
    "100%",
    "completionist",
    "perfect run",
    "flawless",
    // Items/Drops
    "legendary drop",
    "rare drop",
    "loot",
    "epic loot",
    "gacha luck",
    "pull rate",
    "5 star",
    "shiny",
    "god roll",
    "perfect stats",
    "meta build",
    // Progress
    "level up",
    "rank up",
    "promotion",
    "climbing",
    "speedrun",
    "pb",
    "personal best",
    "world record",
    "no hit run",
    "one shot",
    "challenge complete",
    // Games/Events
    "game of the year",
    "goty",
    "masterpiece",
    "must play",
    "dlc",
    "expansion",
    "season pass",
    "battle pass",
    "tournament",
    "championship",
    "worlds",
    "majors",
    "lan",
    "convention",
    "e3",
    "gamescom",
    // Streaming
    "twitch",
    "youtube gaming",
    "content creator",
    "streamer",
    "subscriber",
    "donation",
    "hype",
    "raid",
    "host",
    "viewer milestone",
    // Fun/Enjoyment
    "addictive",
    "can't stop playing",
    "masterpiece",
    "incredible",
    "stunning",
    "beautiful graphics",
    "amazing soundtrack",
];

const GAMING_NEGATIVE: &[&str] = &[
    // Losses/Failures
    "game over",
    "wipe",
    "defeat",
    "loss streak",
    "throw",
    "thrown game",
    "feeder",
    "inting",
    "throwing",
    "ff",
    "surrender",
    // Technical Issues
    "lag",
    "disconnect",
    "dc",
    "server down",
    "maintenance",
    "bug",
    "glitch",
    "exploit",
    "cheater",
    "hacker",
    "fps drop",
    "crash",
    "optimization issue",
    "stuttering",
    "input lag",
    "frame drop",
    "performance issue",
    // Game Design
    "nerf",
    "nerfed",
    "too weak",
    "unbalanced",
    "pay to win",
    "p2w",
    "microtransactions",
    "loot box",
    "grind",
    "grindy",
    "repetitive",
    "boring",
    "trash",
    "disappointing",
    "waste of money",
    // Community
    "toxic",
    "griefer",
    "griefing",
    "trolling",
    "smurf",
    "smurfing",
    "boosting",
    "scripting",
    "tilted",
    "rage quit",
    "report",
    "banned",
    // Business
    "overpriced",
    "dlc hell",
    "season pass",
    "battle pass grind",
    "cash grab",
    "milking",
    "dead game",
    "no players",
    "shutdown",
    "server closing",
    "end of support",
];

// ============================================================================
// Sports Keywords
// ============================================================================

const SPORTS_POSITIVE: &[&str] = &[
    // Wins
    "victory",
    "won",
    "champion",
    "championship",
    "trophy",
    "gold medal",
    "first place",
    "playoffs",
    "finals",
    "super bowl",
    "world series",
    "world cup",
    "finals mvp",
    // Performance
    "clutch",
    "game winner",
    "buzzer beater",
    "touchdown",
    "home run",
    "hat trick",
    "no hitter",
    "perfect game",
    "record breaking",
    "milestone",
    "career high",
    // Teams/Players
    "mvp",
    "all star",
    "hall of fame",
    "legend",
    "goat",
    "greatest of all time",
    "dynasty",
    // Fan Experience
    "amazing game",
    "thriller",
    "overtime",
    "comeback",
    "underdog",
    "cinderella",
    "miracle",
];

const SPORTS_NEGATIVE: &[&str] = &[
    // Losses
    "lost",
    "defeat",
    "eliminated",
    "relegated",
    "blown lead",
    "choke",
    "collapse",
    // Poor Performance
    "terrible game",
    "worst performance",
    "benchwarmer",
    "injured",
    "out for season",
    "career ending",
    "trade",
    "waived",
    "cut",
    "free agent",
    // Controversy
    "scandal",
    "suspension",
    "ban",
    "doping",
    "referee",
    "bad call",
    "controversial",
    "fighting",
    "ejected",
    "flagrant",
];

// ============================================================================
// Entertainment Keywords
// ============================================================================

const ENT_POSITIVE: &[&str] = &[
    // Movies/TV
    "masterpiece",
    "must watch",
    "binge worthy",
    "phenomenal",
    "oscar",
    "emmy",
    "golden globe",
    "award winning",
    "blockbuster",
    "hit",
    "standing ovation",
    "critically acclaimed",
    // Music
    "album of the year",
    "grammy",
    "chart topper",
    "platinum",
    "hit song",
    "banger",
    "earworm",
    "on repeat",
    "concert",
    "live performance",
    "tour",
    "festival",
    // Celebrities
    "icon",
    "legend",
    "talented",
    "gifted",
    "star power",
    "breaking the internet",
    "viral",
    "trending",
    // Recommendations
    "highly recommend",
    "don't miss",
    "see it",
    "watch it",
    "listen to",
    "streaming",
    "netflix",
    "spotify",
];

const ENT_NEGATIVE: &[&str] = &[
    // Movies/TV
    "bomb",
    "flop",
    "disappointing",
    "waste of time",
    "boring",
    "predictable",
    "cliche",
    "terrible acting",
    "cancelled",
    "not renewed",
    "axed",
    // Music
    "worst album",
    "disappointing",
    "sold out",
    "lip sync",
    "concert cancelled",
    "tour postponed",
    // Controversy
    "scandal",
    "cancelled",
    "controversy",
    "backlash",
    "boycott",
    "apology",
    "dragged",
    // Reviews
    "rotten tomatoes",
    "bad reviews",
    "critics hate",
    "one star",
    "avoid",
    "skip it",
];

// ============================================================================
// Domain Detection
// ============================================================================

/// Detect domain from tweet content using keyword scoring.
///
/// # Arguments
/// * `text` - The text to analyze
///
/// # Returns
/// Detected SentimentDomain (General if no clear domain)
pub fn detect_domain(text: &str) -> SentimentDomain {
    let lower = text.to_lowercase();

    // Domain indicator keywords
    let crypto_indicators = [
        "btc",
        "eth",
        "crypto",
        "bitcoin",
        "ethereum",
        "defi",
        "nft",
        "hodl",
        "blockchain",
        "altcoin",
        "token",
        "binance",
        "coinbase",
        "wallet",
        "gas fee",
    ];
    let tech_indicators = [
        "code",
        "dev",
        "programming",
        "software",
        "github",
        "pr",
        "merge",
        "deploy",
        "shipping",
        "startup",
        "api",
        "sdk",
        "framework",
        "library",
        "package",
        "debug",
        "compile",
        "build",
        "test",
        "ci/cd",
    ];
    let gaming_indicators = [
        "gaming",
        "game",
        "twitch",
        "esports",
        "streamer",
        "valorant",
        "fortnite",
        "steam",
        "xbox",
        "playstation",
        "nintendo",
        "pc gaming",
        "lol",
        "league of legends",
        "overwatch",
        "cod",
        "ranked",
        "pentakill",
        "clutch",
        "mvp",
    ];
    let sports_indicators = [
        "nfl",
        "nba",
        "mlb",
        "soccer",
        "football",
        "basketball",
        "baseball",
        "touchdown",
        "goal",
        "playoffs",
        "championship",
        "team",
        "coach",
    ];
    let entertainment_indicators = [
        "movie",
        "film",
        "netflix",
        "tv show",
        "album",
        "concert",
        "celebrity",
        "oscar",
        "music",
        "song",
        "streaming",
        "spotify",
    ];

    // Count indicators
    let crypto_score = crypto_indicators
        .iter()
        .filter(|&&w| lower.contains(w))
        .count();
    let tech_score = tech_indicators
        .iter()
        .filter(|&&w| lower.contains(w))
        .count();
    let gaming_score = gaming_indicators
        .iter()
        .filter(|&&w| lower.contains(w))
        .count();
    let sports_score = sports_indicators
        .iter()
        .filter(|&&w| lower.contains(w))
        .count();
    let entertainment_score = entertainment_indicators
        .iter()
        .filter(|&&w| lower.contains(w))
        .count();

    // Find highest scoring domain
    let mut scores = [
        (SentimentDomain::Crypto, crypto_score),
        (SentimentDomain::Tech, tech_score),
        (SentimentDomain::Gaming, gaming_score),
        (SentimentDomain::Sports, sports_score),
        (SentimentDomain::Entertainment, entertainment_score),
    ];

    scores.sort_by(|a, b| b.1.cmp(&a.1));

    // Return highest if it has at least 2 indicators, otherwise General
    if scores[0].1 >= 2 {
        return scores[0].0;
    } else if scores[0].1 == 1 {
        // With 1 indicator, still might be domain-specific
        // Check if there are domain keywords in the text
        match scores[0].0 {
            SentimentDomain::Crypto if has_crypto_keywords(&lower) => {
                return SentimentDomain::Crypto
            }
            SentimentDomain::Tech if has_tech_keywords(&lower) => return SentimentDomain::Tech,
            SentimentDomain::Gaming if has_gaming_keywords(&lower) => {
                return SentimentDomain::Gaming
            }
            SentimentDomain::Sports if has_sports_keywords(&lower) => {
                return SentimentDomain::Sports
            }
            SentimentDomain::Entertainment if has_ent_keywords(&lower) => {
                return SentimentDomain::Entertainment
            }
            _ => {}
        }
    }

    SentimentDomain::General
}

/// Check for crypto-specific keywords beyond indicators.
fn has_crypto_keywords(text: &str) -> bool {
    CRYPTO_POSITIVE
        .iter()
        .chain(CRYPTO_NEGATIVE.iter())
        .any(|&w| text.contains(w))
}

/// Check for tech-specific keywords beyond indicators.
fn has_tech_keywords(text: &str) -> bool {
    TECH_POSITIVE
        .iter()
        .chain(TECH_NEGATIVE.iter())
        .any(|&w| text.contains(w))
}

/// Check for gaming-specific keywords beyond indicators.
fn has_gaming_keywords(text: &str) -> bool {
    GAMING_POSITIVE
        .iter()
        .chain(GAMING_NEGATIVE.iter())
        .any(|&w| text.contains(w))
}

/// Check for sports-specific keywords beyond indicators.
fn has_sports_keywords(text: &str) -> bool {
    SPORTS_POSITIVE
        .iter()
        .chain(SPORTS_NEGATIVE.iter())
        .any(|&w| text.contains(w))
}

/// Check for entertainment-specific keywords beyond indicators.
fn has_ent_keywords(text: &str) -> bool {
    ENT_POSITIVE
        .iter()
        .chain(ENT_NEGATIVE.iter())
        .any(|&w| text.contains(w))
}

// ============================================================================
// Domain Sentiment Analysis
// ============================================================================

/// Analyze sentiment with domain-specific keywords.
///
/// # Arguments
/// * `text` - The text to analyze
/// * `domain` - The domain to use for keyword matching
///
/// # Returns
/// Sentiment score (positive or negative contribution)
pub fn analyze_domain_sentiment(text: &str, domain: SentimentDomain) -> f32 {
    let lower = text.to_lowercase();
    let mut score = 0.0;

    // Get domain-specific keyword sets
    let (positive, negative): (&[&str], &[&str]) = match domain {
        SentimentDomain::Tech => (TECH_POSITIVE, TECH_NEGATIVE),
        SentimentDomain::Crypto => (CRYPTO_POSITIVE, CRYPTO_NEGATIVE),
        SentimentDomain::Gaming => (GAMING_POSITIVE, GAMING_NEGATIVE),
        SentimentDomain::Sports => (SPORTS_POSITIVE, SPORTS_NEGATIVE),
        SentimentDomain::Entertainment => (ENT_POSITIVE, ENT_NEGATIVE),
        SentimentDomain::General => (&[], &[]),
    };

    // Score positive keywords (weighted 1.5x for domain specificity)
    for &word in positive {
        if lower.contains(word) {
            score += 1.5;
        }
    }

    // Score negative keywords
    for &word in negative {
        if lower.contains(word) {
            score -= 1.5;
        }
    }

    score
}

/// Get domain keyword statistics for debugging/analysis.
#[derive(Debug, Default)]
pub struct DomainStats {
    pub domain: SentimentDomain,
    pub positive_matches: Vec<String>,
    pub negative_matches: Vec<String>,
    pub indicator_count: usize,
}

/// Analyze text and return detailed domain statistics.
pub fn analyze_domain_stats(text: &str) -> DomainStats {
    let domain = detect_domain(text);
    let lower = text.to_lowercase();

    let (positive, negative): (&[&str], &[&str]) = match domain {
        SentimentDomain::Tech => (TECH_POSITIVE, TECH_NEGATIVE),
        SentimentDomain::Crypto => (CRYPTO_POSITIVE, CRYPTO_NEGATIVE),
        SentimentDomain::Gaming => (GAMING_POSITIVE, GAMING_NEGATIVE),
        SentimentDomain::Sports => (SPORTS_POSITIVE, SPORTS_NEGATIVE),
        SentimentDomain::Entertainment => (ENT_POSITIVE, ENT_NEGATIVE),
        SentimentDomain::General => (&[], &[]),
    };

    let positive_matches = positive
        .iter()
        .filter(|&&w| lower.contains(w))
        .map(|&w| w.to_string())
        .collect();

    let negative_matches = negative
        .iter()
        .filter(|&&w| lower.contains(w))
        .map(|&w| w.to_string())
        .collect();

    // Count indicators
    let indicators = [
        "btc",
        "eth",
        "crypto",
        "bitcoin",
        "ethereum",
        "defi",
        "nft",
        "hodl",
        "code",
        "dev",
        "programming",
        "software",
        "github",
        "pr",
        "merge",
        "gaming",
        "game",
        "twitch",
        "esports",
        "streamer",
        "nfl",
        "nba",
        "soccer",
        "football",
        "basketball",
        "movie",
        "film",
        "netflix",
        "tv show",
        "album",
        "concert",
    ];
    let indicator_count = indicators.iter().filter(|&&w| lower.contains(w)).count();

    DomainStats {
        domain,
        positive_matches,
        negative_matches,
        indicator_count,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_tech_domain() {
        assert_eq!(
            detect_domain("Just shipped a new feature! #coding #dev"),
            SentimentDomain::Tech
        );
        assert_eq!(
            detect_domain("CI is green, PR approved, merging now"),
            SentimentDomain::Tech
        );
        assert_eq!(
            detect_domain("Production outage, debugging all night"),
            SentimentDomain::Tech
        );
    }

    #[test]
    fn test_detect_crypto_domain() {
        assert_eq!(
            detect_domain("BTC to the moon! 🚀 Hodl strong! Ethereum"),
            SentimentDomain::Crypto
        );
        assert_eq!(
            detect_domain("New DeFi protocol launching on Ethereum blockchain"),
            SentimentDomain::Crypto
        );
        assert_eq!(
            detect_domain("Got rekt on my liquidation BTC"),
            SentimentDomain::Crypto
        );
    }

    #[test]
    fn test_detect_gaming_domain() {
        // pentakill + ranked = gaming specific
        assert_eq!(
            detect_domain("Epic pentakill in ranked! Climbing to Diamond league"),
            SentimentDomain::Gaming
        );
        // twitch + streamer + gaming = 3 indicators
        assert_eq!(
            detect_domain("Live on twitch streaming new game release streamer"),
            SentimentDomain::Gaming
        );
    }

    #[test]
    fn test_detect_general_domain() {
        assert_eq!(
            detect_domain("Having a great day today!"),
            SentimentDomain::General
        );
        assert_eq!(detect_domain("This is amazing!"), SentimentDomain::General);
    }

    #[test]
    fn test_domain_sentiment_tech_positive() {
        let score = analyze_domain_sentiment(
            "Just shipped a new feature! Tests passing, CI green!",
            SentimentDomain::Tech,
        );
        assert!(score > 0.0);
    }

    #[test]
    fn test_domain_sentiment_tech_negative() {
        let score = analyze_domain_sentiment(
            "Production outage, firefighting all night. Technical debt is killing us.",
            SentimentDomain::Tech,
        );
        assert!(score < 0.0);
    }

    #[test]
    fn test_domain_sentiment_crypto_positive() {
        let score = analyze_domain_sentiment(
            "BTC pumping! Diamond hands paying off. To the moon!",
            SentimentDomain::Crypto,
        );
        assert!(score > 0.0);
    }

    #[test]
    fn test_domain_sentiment_crypto_negative() {
        let score = analyze_domain_sentiment(
            "Got rekt. Liquidated. Rug pulled again.",
            SentimentDomain::Crypto,
        );
        assert!(score < 0.0);
    }

    #[test]
    fn test_domain_sentiment_gaming_positive() {
        let score = analyze_domain_sentiment(
            "Epic win! Legendary drop! New personal best!",
            SentimentDomain::Gaming,
        );
        assert!(score > 0.0);
    }

    #[test]
    fn test_domain_sentiment_gaming_negative() {
        let score = analyze_domain_sentiment(
            "Lag ruined the game. Cheater everywhere. Nerfed again.",
            SentimentDomain::Gaming,
        );
        assert!(score < 0.0);
    }

    #[test]
    fn test_analyze_domain_stats() {
        let stats = analyze_domain_stats("Just shipped! CI green, tests passing!");
        assert_eq!(stats.domain, SentimentDomain::Tech);
        assert!(!stats.positive_matches.is_empty());
    }
}
