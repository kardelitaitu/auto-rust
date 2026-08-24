//! Twitter reply strategy selection system.
//!
//! Provides 32 distinct reply styles with context-aware weighted selection.
//! Mirrors the Node.js reference implementation.

use rand::Rng;

/// All 32 reply strategies with base weights.
/// Base weight 1 = equally likely by default. Context boosts multiply the weight.
pub const STRATEGY_POOL: &[(&str, u32)] = &[
    // ── Positive ─────────────────────────────────────────────────────────
    ("COMPLIMENT", 1), // Genuine praise
    ("HYPEMAN", 1),    // Wildly excited
    ("HYPE_REPLY", 1), // Celebrate specific thing
    ("SIMP", 1),       // Over-the-top stan praise
    ("WHOLEsome", 1),  // Kind and supportive
    ("LOWKEY", 1),     // Understated agreement
    // ── Personal ─────────────────────────────────────────────────────────
    ("NOSTALGIC", 1), // Personal memory
    ("RELATABLE", 1), // "Same" sentiment
    // ── Humor ────────────────────────────────────────────────────────────
    ("WITTY", 1),     // Playful observation
    ("DRY_WIT", 1),   // Deadpan humor
    ("SARCASTIC", 1), // Biting sarcasm
    ("TROLL", 1),     // Playful teasing
    ("NITPICK", 1),   // Pedantic correction
    ("UNHINGED", 1),  // Chaotic energy
    // ── Skepticism ───────────────────────────────────────────────────────
    ("CONTRARIAN", 1), // Push back
    ("CALLOUT", 1),    // Point out irony
    ("DISMISSIVE", 1), // Brush off claim
    // ── Expertise ────────────────────────────────────────────────────────
    ("CLOUT", 1),    // Expert confidence
    ("HOT_TAKE", 1), // Provocative opinion
    ("HELPFUL", 1),  // Share useful info
    // ── Observation ──────────────────────────────────────────────────────
    ("OBSERVATION", 1), // Hyper-specific detail
    ("CURIOUS", 1),     // Casual curiosity
    ("QUESTION", 1),    // Ask specific question
    // ── Short/Minimal ────────────────────────────────────────────────────
    ("MINIMALIST", 1), // One word/phrase
    ("SLANG", 1),      // Internet slang
    ("REACTION", 1),   // Pure exclamation
    ("CONFUSED", 1),   // Genuine bewilderment
    // ── Persona ──────────────────────────────────────────────────────────
    ("GEN_Z", 1),  // TikTok energy
    ("BOOMER", 1), // Out-of-touch earnest
    ("NPC", 1),    // Average person
    ("ZEN", 1),    // Philosophical wisdom
    ("SMUG", 1),   // Confident self-satisfaction
];

/// Context → which strategies get a weight boost (multiply base by this value)
pub const CONTEXT_BOOSTS: &[(&str, &[(&str, u32)])] = &[
    (
        "humorous",
        &[
            ("SLANG", 3),
            ("WITTY", 3),
            ("SARCASTIC", 3),
            ("TROLL", 2),
            ("UNHINGED", 2),
            ("REACTION", 3),
            ("MINIMALIST", 2),
        ],
    ),
    (
        "entertainment",
        &[
            ("SLANG", 3),
            ("REACTION", 3),
            ("HYPEMAN", 2),
            ("WITTY", 2),
            ("SIMP", 2),
            ("GEN_Z", 2),
        ],
    ),
    (
        "news",
        &[
            ("OBSERVATION", 3),
            ("CURIOUS", 3),
            ("HOT_TAKE", 2),
            ("QUESTION", 2),
            ("CALLOUT", 2),
            ("HELPFUL", 2),
        ],
    ),
    (
        "politics",
        &[
            ("OBSERVATION", 3),
            ("CONTRARIAN", 3),
            ("CALLOUT", 2),
            ("DRY_WIT", 2),
            ("NITPICK", 2),
            ("SARCASTIC", 2),
        ],
    ),
    (
        "finance",
        &[
            ("OBSERVATION", 2),
            ("HOT_TAKE", 2),
            ("CLOUT", 2),
            ("HELPFUL", 2),
            ("CONTRARIAN", 2),
            ("CURIOUS", 2),
        ],
    ),
    (
        "tech",
        &[
            ("OBSERVATION", 2),
            ("CURIOUS", 3),
            ("HOT_TAKE", 2),
            ("CLOUT", 2),
            ("NITPICK", 2),
            ("HELPFUL", 2),
        ],
    ),
    (
        "science",
        &[
            ("CURIOUS", 3),
            ("OBSERVATION", 2),
            ("HELPFUL", 3),
            ("NITPICK", 2),
            ("ZEN", 2),
            ("QUESTION", 2),
        ],
    ),
    (
        "emotional",
        &[
            ("NOSTALGIC", 3),
            ("RELATABLE", 3),
            ("WHOLEsome", 2),
            ("HYPE_REPLY", 2),
            ("COMPLIMENT", 2),
        ],
    ),
    (
        "personal",
        &[
            ("NOSTALGIC", 3),
            ("RELATABLE", 3),
            ("WHOLEsome", 2),
            ("COMPLIMENT", 2),
        ],
    ),
    (
        "viral",
        &[
            ("MINIMALIST", 3),
            ("REACTION", 3),
            ("SLANG", 2),
            ("HYPEMAN", 2),
            ("GEN_Z", 2),
            ("UNHINGED", 2),
        ],
    ),
    (
        "negative",
        &[
            ("CONTRARIAN", 3),
            ("DISMISSIVE", 2),
            ("DRY_WIT", 2),
            ("SARCASTIC", 2),
            ("QUESTION", 2),
            ("OBSERVATION", 2),
        ],
    ),
    (
        "critical",
        &[
            ("CALLOUT", 3),
            ("CONTRARIAN", 2),
            ("NITPICK", 2),
            ("SARCASTIC", 2),
            ("DRY_WIT", 2),
        ],
    ),
    (
        "wholesome",
        &[
            ("WHOLEsome", 4),
            ("COMPLIMENT", 2),
            ("RELATABLE", 2),
            ("HYPE_REPLY", 2),
        ],
    ),
    (
        "chaotic",
        &[
            ("UNHINGED", 4),
            ("TROLL", 3),
            ("CONFUSED", 2),
            ("GEN_Z", 2),
            ("REACTION", 2),
        ],
    ),
    (
        "debate",
        &[
            ("CONTRARIAN", 3),
            ("CALLOUT", 2),
            ("HOT_TAKE", 2),
            ("NITPICK", 2),
        ],
    ),
    (
        "gaming",
        &[
            ("UNHINGED", 2),
            ("CLOUT", 2),
            ("HYPEMAN", 2),
            ("SIMP", 2),
            ("GEN_Z", 2),
        ],
    ),
    (
        "food",
        &[
            ("SIMP", 2),
            ("NITPICK", 2),
            ("RELATABLE", 2),
            ("WHOLEsome", 2),
            ("ZEN", 2),
        ],
    ),
    (
        "informative",
        &[("HELPFUL", 4), ("OBSERVATION", 2), ("CURIOUS", 2)],
    ),
    (
        "sarcastic",
        &[("SARCASTIC", 4), ("DRY_WIT", 2), ("TROLL", 2)],
    ),
    ("smug", &[("SMUG", 4), ("CLOUT", 2), ("HOT_TAKE", 2)]),
];

/// Strategy instructions - the CRITICAL INSTRUCTION for each strategy
pub const STRATEGY_INSTRUCTIONS: &[(&str, &str)] = &[
    // ── Positive ─────────────────────────────────────────────────────────
    ("COMPLIMENT", "\n**CRITICAL INSTRUCTION**: You MUST write a ONE-SENTENCE genuine compliment about the tweet. NEVER write \"Okay\" or \"Yes\". Keep it to 1 short sentence. No mentions. No Emoji."),
    ("HYPEMAN", "\n**CRITICAL INSTRUCTION**: You MUST hype this up wildly. Sound genuinely, aggressively excited. NEVER write \"Okay\" or \"Yes\". Keep it short. lowercase. No mentions. No Emoji."),
    ("HYPE_REPLY", "\n**CRITICAL INSTRUCTION**: You MUST cheer on or celebrate the exact specific thing mentioned in the tweet. NEVER write \"Okay\" or \"Yes\". Keep it short. No mentions. No Emoji."),
    ("SIMP", "\n**CRITICAL INSTRUCTION**: You MUST over-the-top praise one specific detail in the tweet. Sound like a genuine stan. NEVER write \"Okay\" or \"Yes\". Keep it to 1 sentence. No mentions. No Emoji."),
    ("WHOLEsome", "\n**CRITICAL INSTRUCTION**: You MUST be genuinely kind and supportive. No sarcasm. Just pure wholesome energy. NEVER write \"Okay\" or \"Yes\". Keep it short. No mentions. No Emoji."),
    ("LOWKEY", "\n**CRITICAL INSTRUCTION**: You MUST react with highly understated, deadpan agreement. NEVER write \"Okay\" or \"Yes\". Very short phrase only. No mentions. No Emoji."),

    // ── Personal ─────────────────────────────────────────────────────────
    ("NOSTALGIC", "\n**CRITICAL INSTRUCTION**: You MUST share a brief personal memory related to the tweet. NEVER write \"Okay\" or \"Yes\". Keep it to 1 sentence, around 15 words or less. No mentions. No Emoji."),
    ("RELATABLE", "\n**CRITICAL INSTRUCTION**: You MUST fiercely validate the tweet with a \"same\" or \"relatable\" one-sentence personal angle. NEVER write \"Okay\" or \"Yes\". Keep it short. No mentions. No Emoji."),

    // ── Humor ────────────────────────────────────────────────────────────
    ("WITTY", "\n**CRITICAL INSTRUCTION**: You MUST make a witty, playful observation about the tweet. NEVER write \"Okay\" or \"Yes\". Keep it to 1 punchy sentence. No mentions. No Emoji."),
    ("DRY_WIT", "\n**CRITICAL INSTRUCTION**: You MUST use deadpan dry humor about the tweet topic. No exclamation marks. NEVER write \"Okay\" or \"Yes\". 1 short sentence. No mentions. No Emoji."),
    ("SARCASTIC", "\n**CRITICAL INSTRUCTION**: You MUST use biting sarcasm that's more pointed than dry wit. Playfully mean, never cruel. NEVER write \"Okay\" or \"Yes\". Keep it to 1 short sentence. No mentions. No Emoji."),
    ("TROLL", "\n**CRITICAL INSTRUCTION**: You MUST playful tease or gently roast the tweet without being mean. Light trolling only. NEVER write \"Okay\" or \"Yes\". Keep it to 1 short sentence. No mentions. No Emoji."),
    ("NITPICK", "\n**CRITICAL INSTRUCTION**: You MUST pedantically but funnily correct or nitpick a tiny detail in the tweet. Be the ackshually person. NEVER write \"Okay\" or \"Yes\". Keep it to 1 sentence. No mentions. No Emoji."),
    ("UNHINGED", "\n**CRITICAL INSTRUCTION**: You MUST go fully unhinged — chaotic energy, absurd comparison, or wildly random take. Embrace the chaos. NEVER write \"Okay\" or \"Yes\". Keep it short. lowercase preferred. No mentions. No Emoji."),

    // ── Skepticism ───────────────────────────────────────────────────────
    ("CONTRARIAN", "\n**CRITICAL INSTRUCTION**: You MUST respectfully push back or flip the take. Offer a different angle without being hostile. NEVER write \"Okay\" or \"Yes\". Keep it to 1 short sentence. No mentions. No Emoji."),
    ("CALLOUT", "\n**CRITICAL INSTRUCTION**: You MUST point out an irony or obvious contradiction in the tweet in one short sentence. NEVER write \"Okay\" or \"Yes\". Keep it short. No mentions. No Emoji."),
    ("DISMISSIVE", "\n**CRITICAL INSTRUCTION**: You MUST brush off the tweet's claim with confident indifference. Never hostile, just unimpressed. NEVER write \"Okay\" or \"Yes\". Keep it short. No mentions. No Emoji."),

    // ── Expertise ────────────────────────────────────────────────────────
    ("CLOUT", "\n**CRITICAL INSTRUCTION**: You MUST write one short, highly confident line, acting as if you are an expert on this tweet's topic. NEVER write \"Okay\" or \"Yes\". Keep it short. No mentions. No Emoji."),
    ("HOT_TAKE", "\n**CRITICAL INSTRUCTION**: You MUST give a confident short opinion that sounds slightly provocative or surprising regarding the tweet. NEVER write \"Okay\" or \"Yes\". 1 short sentence. No mentions. No Emoji."),
    ("HELPFUL", "\n**CRITICAL INSTRUCTION**: You MUST share a genuinely useful fact, tip, or resource related to the tweet. Sound helpful not preachy. NEVER write \"Okay\" or \"Yes\". Keep it to 1 short sentence. No mentions. No Emoji."),

    // ── Observation ──────────────────────────────────────────────────────
    ("OBSERVATION", "\n**CRITICAL INSTRUCTION**: You MUST make a hyper-specific, casual observation about the tweet content. Avoid formal grammar. NEVER write \"Okay\" or \"Yes\". Keep it up to 12 words. No mentions. No Emoji."),
    ("CURIOUS", "\n**CRITICAL INSTRUCTION**: You MUST express casual, specific curiosity about a detail in the tweet. NEVER write \"Okay\" or \"Yes\". Keep it short. No mentions. No Emoji."),
    ("QUESTION", "\n**CRITICAL INSTRUCTION**: You MUST ask a specific, highly relevant question about the tweet. NEVER write \"Okay\" or \"Yes\". Keep it to 1 short sentence. No mentions. No Emoji."),

    // ── Short/Minimal ────────────────────────────────────────────────────
    ("MINIMALIST", "\n**CRITICAL INSTRUCTION**: React with exactly ONE highly positive expressive word or extremely short phrase (2-4 words). lowercase. NEVER write \"Okay\" or \"Yes\". No mentions. No Emoji."),
    ("SLANG", "\n**CRITICAL INSTRUCTION**: You MUST use casual internet slang. lowercase ONLY. NEVER write \"Okay\" or \"Yes\". Keep it very brief, under 10 words. No mentions. No Emoji."),
    ("REACTION", "\n**CRITICAL INSTRUCTION**: You MUST provide pure unfiltered reaction — one punchy exclamation sentence. lowercase. NEVER write \"Okay\" or \"Yes\". Under 5 words. No mentions. No Emoji."),
    ("CONFUSED", "\n**CRITICAL INSTRUCTION**: You MUST express genuine confusion or bewilderment about the tweet's claim. NOT sarcastic — real confusion. NEVER write \"Okay\" or \"Yes\". Keep it short. No mentions. No Emoji."),

    // ── Persona ──────────────────────────────────────────────────────────
    ("GEN_Z", "\n**CRITICAL INSTRUCTION**: You MUST use very online Gen Z slang and energy. Think TikTok comments section. NEVER write \"Okay\" or \"Yes\". Keep it brief, lowercase only. No mentions. No Emoji."),
    ("BOOMER", "\n**CRITICAL INSTRUCTION**: You MUST respond like a slightly out-of-touch older person trying to relate. Maybe slightly confused but earnest. NEVER write \"Okay\" or \"Yes\". Keep it to 1 sentence. No mentions. No Emoji."),
    ("NPC", "\n**CRITICAL INSTRUCTION**: You MUST respond like a totally average, default person. No strong opinions. Basic reaction. NEVER write \"Okay\" or \"Yes\". Keep it very short. No mentions. No Emoji."),
    ("ZEN", "\n**CRITICAL INSTRUCTION**: You MUST respond with calm, philosophical wisdom about the tweet topic. Sound like someone who has found inner peace. NEVER write \"Okay\" or \"Yes\". Keep it to 1 short sentence. No mentions. No Emoji."),
    ("SMUG", "\n**CRITICAL INSTRUCTION**: You MUST reply with smug self-satisfaction, like you already knew this. Confident but not aggressive. NEVER write \"Okay\" or \"Yes\". Keep it short. No mentions. No Emoji."),
];

/// Context for strategy selection
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StrategyContext {
    pub sentiment: String,         // e.g., "humorous", "news", "emotional"
    pub conversation_type: String, // e.g., "tech", "politics", "gaming"
    pub engagement_level: String,  // e.g., "high", "viral"
}

/// Map a keyword topic to the matching CONTEXT_BOOSTS key.
const TOPIC_KEYWORDS: &[(&str, &[&str])] = &[
    (
        "tech",
        &[
            "programming",
            "software",
            "coding",
            "developer",
            "engineer",
            "ai",
            "artificial intelligence",
            "machine learning",
            "llm",
            "gpt",
            "python",
            "rust",
            "javascript",
            "typescript",
            "react",
            "api",
            "github",
            "open source",
            "startup",
            "saas",
            "cloud",
            "server",
            "database",
            "backend",
            "frontend",
            "deploy",
            "docker",
            "kubernetes",
            "linux",
            "terminal",
            "cli",
            "debug",
            "algorithm",
            "data science",
        ],
    ),
    (
        "politics",
        &[
            "biden",
            "trump",
            "election",
            "vote",
            "senate",
            "congress",
            "democracy",
            "republican",
            "democrat",
            "gop",
            "political",
            "government",
            "policy",
            "legislation",
            "supreme court",
            "protest",
            "campaign",
            "president",
            "governor",
            "senator",
        ],
    ),
    (
        "gaming",
        &[
            "game",
            "gaming",
            "playstation",
            "ps5",
            "xbox",
            "nintendo",
            "switch",
            "steam",
            "pc gaming",
            "rpg",
            "fps",
            "open world",
            "minecraft",
            "fortnite",
            "valorant",
            "league of legends",
            "esports",
            "game dev",
            "indie game",
            "console",
            "gpu",
        ],
    ),
    (
        "food",
        &[
            "food",
            "recipe",
            "cooking",
            "restaurant",
            "chef",
            "cuisine",
            "pizza",
            "pasta",
            "sushi",
            "burger",
            "coffee",
            "wine",
            "dinner",
            "lunch",
            "breakfast",
            "baking",
            "grill",
            "spicy",
            "dessert",
            "vegan",
            "vegetarian",
            "delicious",
            "tasty",
        ],
    ),
    (
        "science",
        &[
            "science",
            "research",
            "study",
            "nasa",
            "space",
            "physics",
            "biology",
            "chemistry",
            "astronomy",
            "quantum",
            "dna",
            "genome",
            "climate",
            "vaccine",
            "experiment",
            "laboratory",
            "scientist",
            "discovery",
            "evolution",
            "particle",
        ],
    ),
    (
        "finance",
        &[
            "finance",
            "crypto",
            "bitcoin",
            "ethereum",
            "stock",
            "investment",
            "market",
            "money",
            "trading",
            "wall street",
            "economy",
            "inflation",
            "recession",
            "dividend",
            "portfolio",
            "etf",
            "interest rate",
            "federal reserve",
            "banking",
            "ipo",
        ],
    ),
    (
        "entertainment",
        &[
            "movie",
            "film",
            "cinema",
            "tv",
            "television",
            "music",
            "celebrity",
            "show",
            "netflix",
            "hbo",
            "disney",
            "actor",
            "actress",
            "director",
            "album",
            "song",
            "concert",
            "tour",
            "series",
            "streaming",
            "oscar",
            "grammy",
        ],
    ),
    (
        "news",
        &[
            "breaking",
            "news",
            "report",
            "update",
            "just in",
            "now",
            "announcement",
            "breaking news",
            "headline",
            "developing story",
        ],
    ),
    (
        "debate",
        &[
            "debate",
            "argument",
            "disagree",
            "actually",
            "well actually",
            "controversial",
            "hot take",
            "unpopular opinion",
        ],
    ),
];

/// Detect the conversation type from tweet text using keyword matching.
///
/// Returns one of: "tech", "politics", "gaming", "food", "science",
/// "finance", "entertainment", "news", "debate", or empty string if unknown.
///
/// Uses the topic with the most keyword hits; ties broken by order of the list.
#[must_use]
pub fn classify_conversation_type(tweet_text: &str) -> String {
    let lower = tweet_text.to_lowercase();

    let mut best_topic: &str = "";
    let mut best_count: usize = 0;

    for (topic, keywords) in TOPIC_KEYWORDS {
        let count = keywords.iter().filter(|kw| lower.contains(*kw)).count();
        if count > best_count {
            best_count = count;
            best_topic = topic;
        }
    }

    best_topic.to_string()
}

/// Build a full `StrategyContext` from sentiment and tweet text.
///
/// - `sentiment` → sets `sentiment` field (positive→"wholesome", negative→"critical")
/// - `tweet_text` → auto-detects `conversation_type` (tech/politics/gaming/etc.)
#[must_use]
pub fn sentiment_to_strategy_context(
    sentiment: crate::utils::twitter::sentiment::Sentiment,
    tweet_text: &str,
) -> StrategyContext {
    let sentiment_tag = match sentiment {
        crate::utils::twitter::sentiment::Sentiment::Positive => "wholesome".to_string(),
        crate::utils::twitter::sentiment::Sentiment::Neutral => String::new(),
        crate::utils::twitter::sentiment::Sentiment::Negative => "critical".to_string(),
    };
    let conversation_type = classify_conversation_type(tweet_text);
    StrategyContext {
        sentiment: sentiment_tag,
        conversation_type,
        engagement_level: String::new(),
    }
}

/// Pick a strategy using weighted random selection.
/// All strategies start at base weight 1; context boosts multiply specific keys.
#[must_use]
pub fn get_strategy_instruction(context: &StrategyContext) -> &'static str {
    // Build boost map from matching context keys
    let mut boost_map: std::collections::HashMap<&str, u32> = std::collections::HashMap::new();

    let context_keys = [
        &context.sentiment,
        &context.conversation_type,
        &context.engagement_level,
    ];

    for key in &context_keys {
        for (ctx_key, boosts) in CONTEXT_BOOSTS {
            if ctx_key == key {
                for (strategy, multiplier) in *boosts {
                    let entry = boost_map.entry(*strategy).or_insert(1);
                    *entry = (*entry).max(*multiplier);
                }
            }
        }
    }

    // Apply boosts to pool weights
    let weighted_pool: Vec<(&str, u32)> = STRATEGY_POOL
        .iter()
        .map(|(key, base)| (*key, base * boost_map.get(key).copied().unwrap_or(1)))
        .collect();

    // Weighted random pick
    let total: u32 = weighted_pool.iter().map(|(_, w)| w).sum();
    let mut rng = rand::thread_rng();
    let mut r = if total > 0 {
        rng.gen_range(0..total)
    } else {
        0
    };

    for (key, weight) in weighted_pool {
        if r < weight {
            return STRATEGY_INSTRUCTIONS
                .iter()
                .find(|(k, _)| *k == key)
                .map_or(STRATEGY_INSTRUCTIONS[0].1, |(_, instruction)| *instruction);
        }
        r -= weight;
    }

    // Fallback to last strategy
    STRATEGY_INSTRUCTIONS.last().map_or("", |(_, i)| *i)
}

/// Compute the weighted pool for a given context without random selection.
/// Exposed for deterministic testing of weight distributions.
#[must_use]
#[cfg(test)]
pub fn get_weighted_pool(context: &StrategyContext) -> Vec<(&'static str, u32)> {
    let mut boost_map: std::collections::HashMap<&str, u32> = std::collections::HashMap::new();

    let context_keys = [
        &context.sentiment,
        &context.conversation_type,
        &context.engagement_level,
    ];

    for key in &context_keys {
        for (ctx_key, boosts) in CONTEXT_BOOSTS {
            if ctx_key == key {
                for (strategy, multiplier) in *boosts {
                    let entry = boost_map.entry(*strategy).or_insert(1);
                    *entry = (*entry).max(*multiplier);
                }
            }
        }
    }

    STRATEGY_POOL
        .iter()
        .map(|(key, base)| (*key, base * boost_map.get(key).copied().unwrap_or(1)))
        .collect()
}

/// Build reply prompt with strategy selection
#[must_use]
pub fn build_reply_prompt(
    tweet_text: &str,
    author: &str,
    replies: &[(String, String)],
    context: &StrategyContext,
    batch_mode: bool,
) -> String {
    let tweet_snippet = if tweet_text.len() > 500 {
        &tweet_text[..500]
    } else {
        tweet_text
    };

    let mut prompt = String::new();

    // Add strategy instruction
    prompt.push_str(get_strategy_instruction(context));

    // Add tweet
    prompt.push_str(&format!(
        "\n\nTweet by @{}:\n{}",
        author,
        tweet_snippet.trim()
    ));

    // Add replies
    if replies.is_empty() {
        prompt.push_str("\n\n(no other replies visible)\n");
    } else {
        prompt.push_str("\n\nReplies:\n");
        for (i, (reply_author, reply_text)) in replies.iter().take(20).enumerate() {
            // Strip hashtags and emojis from replies
            let clean_text = reply_text
                .chars()
                .filter(|c| {
                    // Filter out emoji Unicode ranges
                    let cp = *c as u32;
                    !(0x1F600..=0x1F64F).contains(&cp) &&  // Emoticons
                    !(0x1F300..=0x1F5FF).contains(&cp) &&  // Misc Symbols
                    !(0x1F680..=0x1F6FF).contains(&cp) &&  // Transport
                    !(0x1F1E0..=0x1F1FF).contains(&cp) &&  // Flags
                    !(0x2600..=0x26FF).contains(&cp) &&    // Misc symbols
                    !(0x2700..=0x27BF).contains(&cp) // Dingbats
                })
                .collect::<String>()
                .replace('#', "");

            prompt.push_str(&format!(
                "{}. @{}: {}\n",
                i + 1,
                reply_author,
                clean_text.trim()
            ));
        }
    }

    if batch_mode {
        prompt.push_str("\n\nGenerate ONE reply for each reply above. Respond with a JSON array of objects, each with a 'content' field containing the reply text.");
    } else {
        prompt.push_str("\n\nGenerate ONE single reply to the original tweet, taking the context of the other replies into account. Keep it strictly to 1 or 2 sentences maximum. Respond with ONLY the raw response text. DO NOT output JSON or any labels.");
    }
    prompt
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::twitter::sentiment::Sentiment;

    #[test]
    fn test_strategy_pool_has_32_strategies() {
        assert_eq!(STRATEGY_POOL.len(), 32);
    }

    #[test]
    fn test_all_strategies_have_instructions() {
        for (strategy, _) in STRATEGY_POOL {
            assert!(
                STRATEGY_INSTRUCTIONS.iter().any(|(s, _)| *s == *strategy),
                "Strategy {} missing instruction",
                strategy
            );
        }
    }

    #[test]
    fn test_get_strategy_returns_instruction() {
        let context = StrategyContext::default();
        let instruction = get_strategy_instruction(&context);
        assert!(instruction.contains("CRITICAL INSTRUCTION"));
    }

    #[test]
    fn test_context_boosts_apply() {
        let context = StrategyContext {
            sentiment: "humorous".to_string(),
            conversation_type: String::new(),
            engagement_level: String::new(),
        };

        // Should boost SLANG, WITTY, SARCASTIC, etc.
        let instruction = get_strategy_instruction(&context);
        assert!(instruction.contains("CRITICAL INSTRUCTION"));
    }

    #[test]
    fn test_build_reply_prompt_format() {
        let context = StrategyContext::default();
        let replies = vec![
            ("user1".to_string(), "Great point!".to_string()),
            ("user2".to_string(), "I agree".to_string()),
        ];

        let prompt = build_reply_prompt("Test tweet", "testuser", &replies, &context, false);

        assert!(prompt.contains("Tweet by @testuser:"));
        assert!(prompt.contains("Test tweet"));
        assert!(prompt.contains("Replies:"));
        assert!(prompt.contains("@user1: Great point!"));
        assert!(prompt.contains("Generate ONE single reply to the original tweet"));
    }

    #[test]
    fn test_build_reply_prompt_truncates_tweet() {
        let context = StrategyContext::default();
        let long_tweet = "a".repeat(600);

        let prompt = build_reply_prompt(&long_tweet, "user", &[], &context, false);

        // Should be truncated to 500 chars
        assert!(prompt.contains(&"a".repeat(500)));
        assert!(!prompt.contains(&"a".repeat(600)));
    }

    #[test]
    fn test_strategy_context_default() {
        let context = StrategyContext::default();
        assert_eq!(context.sentiment, "");
        assert_eq!(context.conversation_type, "");
        assert_eq!(context.engagement_level, "");
    }

    #[test]
    fn test_strategy_context_fields() {
        let context = StrategyContext {
            sentiment: "humorous".to_string(),
            conversation_type: "tech".to_string(),
            engagement_level: "high".to_string(),
        };

        assert_eq!(context.sentiment, "humorous");
        assert_eq!(context.conversation_type, "tech");
        assert_eq!(context.engagement_level, "high");
    }

    #[test]
    fn test_strategy_context_equality() {
        let ctx1 = StrategyContext {
            sentiment: "humorous".to_string(),
            conversation_type: "tech".to_string(),
            engagement_level: "high".to_string(),
        };

        let ctx2 = StrategyContext {
            sentiment: "humorous".to_string(),
            conversation_type: "tech".to_string(),
            engagement_level: "high".to_string(),
        };

        assert_eq!(ctx1, ctx2);
    }

    #[test]
    fn test_build_reply_prompt_no_replies() {
        let context = StrategyContext::default();
        let prompt = build_reply_prompt("Test tweet", "user", &[], &context, false);

        assert!(prompt.contains("(no other replies visible)"));
    }

    #[test]
    fn test_build_reply_prompt_filters_emoji() {
        let context = StrategyContext::default();
        let replies = vec![("user1".to_string(), "Great! 😂🎉".to_string())];

        let prompt = build_reply_prompt("Test", "user", &replies, &context, false);

        // Emoji should be filtered out
        assert!(!prompt.contains("😂"));
        assert!(!prompt.contains("🎉"));
        assert!(prompt.contains("Great!"));
    }

    #[test]
    fn test_build_reply_prompt_filters_hashtags() {
        let context = StrategyContext::default();
        let replies = vec![("user1".to_string(), "Great #test #hashtag".to_string())];

        let prompt = build_reply_prompt("Test", "user", &replies, &context, false);

        // Hashtags should be removed
        assert!(!prompt.contains("#test"));
        assert!(!prompt.contains("#hashtag"));
        assert!(prompt.contains("Great"));
    }

    #[test]
    fn test_build_reply_prompt_limits_replies() {
        let context = StrategyContext::default();
        let many_replies: Vec<(String, String)> = (0..25)
            .map(|i| (format!("user{}", i), format!("Reply {}", i)))
            .collect();

        let prompt = build_reply_prompt("Test", "user", &many_replies, &context, false);

        // Should only include first 20 replies
        assert!(prompt.contains("@user0:"));
        assert!(prompt.contains("@user19:"));
        assert!(!prompt.contains("@user20:"));
    }

    #[test]
    fn test_context_boosts_structure() {
        // Verify context boosts have expected structure
        for (context, boosts) in CONTEXT_BOOSTS {
            assert!(!context.is_empty());
            assert!(!boosts.is_empty());
            for (strategy, multiplier) in *boosts {
                assert!(!strategy.is_empty());
                assert!(*multiplier != 0);
            }
        }
    }

    #[test]
    fn test_strategy_pool_base_weights() {
        // All base weights should be 1
        for (_, weight) in STRATEGY_POOL {
            assert_eq!(*weight, 1);
        }
    }

    #[test]
    fn test_strategy_instructions_format() {
        for (strategy, instruction) in STRATEGY_INSTRUCTIONS {
            assert!(
                instruction.contains("CRITICAL INSTRUCTION"),
                "Strategy {} missing CRITICAL INSTRUCTION marker",
                strategy
            );
            assert!(
                instruction.contains("NEVER write \"Okay\" or \"Yes\""),
                "Strategy {} missing NEVER instruction",
                strategy
            );
        }
    }

    #[test]
    fn test_get_strategy_with_multiple_context_keys() {
        let context = StrategyContext {
            sentiment: "humorous".to_string(),
            conversation_type: "gaming".to_string(),
            engagement_level: "viral".to_string(),
        };

        let instruction = get_strategy_instruction(&context);
        assert!(instruction.contains("CRITICAL INSTRUCTION"));
    }

    #[test]
    fn test_get_strategy_with_tech_context() {
        let context = StrategyContext {
            sentiment: "tech".to_string(),
            conversation_type: String::new(),
            engagement_level: String::new(),
        };

        let instruction = get_strategy_instruction(&context);
        assert!(instruction.contains("CRITICAL INSTRUCTION"));
    }

    #[test]
    fn test_get_strategy_with_news_context() {
        let context = StrategyContext {
            sentiment: "news".to_string(),
            conversation_type: String::new(),
            engagement_level: String::new(),
        };

        let instruction = get_strategy_instruction(&context);
        assert!(instruction.contains("CRITICAL INSTRUCTION"));
    }

    // ── Sentiment-Driven Strategy Selection Tests ────────────────────────

    #[test]
    fn test_default_context_all_weights_equal() {
        let context = StrategyContext::default();
        let pool = get_weighted_pool(&context);

        assert_eq!(pool.len(), 32);
        for (strategy, weight) in &pool {
            assert_eq!(
                *weight, 1,
                "Strategy {strategy} should have weight 1 with default context, got {weight}"
            );
        }
    }

    #[test]
    fn test_positive_sentiment_boosts_wholesome_strategies() {
        let context = StrategyContext {
            sentiment: "wholesome".to_string(),
            conversation_type: String::new(),
            engagement_level: String::new(),
        };
        let pool = get_weighted_pool(&context);

        let find = |name: &str| -> u32 {
            pool.iter()
                .find(|(k, _)| *k == name)
                .map(|(_, w)| *w)
                .unwrap_or(0)
        };

        // Boosted strategies
        assert_eq!(
            find("WHOLEsome"),
            4,
            "WHOLEsome should get 4x boost with wholesome context"
        );
        assert_eq!(
            find("COMPLIMENT"),
            2,
            "COMPLIMENT should get 2x boost with wholesome context"
        );
        assert_eq!(
            find("RELATABLE"),
            2,
            "RELATABLE should get 2x boost with wholesome context"
        );
        assert_eq!(
            find("HYPE_REPLY"),
            2,
            "HYPE_REPLY should get 2x boost with wholesome context"
        );

        // Should be higher than contrast strategies
        assert!(
            find("WHOLEsome") > find("CALLOUT"),
            "WHOLEsome weight ({}) should exceed CALLOUT weight ({}) with positive sentiment",
            find("WHOLEsome"),
            find("CALLOUT")
        );
        assert!(
            find("WHOLEsome") > find("SARCASTIC"),
            "WHOLEsome weight should exceed SARCASTIC weight with positive sentiment"
        );

        // Non-boosted strategies should stay at 1
        assert_eq!(find("BOOMER"), 1, "BOOMER should stay at weight 1");
        assert_eq!(find("ZEN"), 1, "ZEN should stay at weight 1");
    }

    #[test]
    fn test_negative_sentiment_boosts_critical_strategies() {
        let context = StrategyContext {
            sentiment: "critical".to_string(),
            conversation_type: String::new(),
            engagement_level: String::new(),
        };
        let pool = get_weighted_pool(&context);

        let find = |name: &str| -> u32 {
            pool.iter()
                .find(|(k, _)| *k == name)
                .map(|(_, w)| *w)
                .unwrap_or(0)
        };

        // Boosted strategies
        assert_eq!(
            find("CALLOUT"),
            3,
            "CALLOUT should get 3x boost with critical context"
        );
        assert_eq!(
            find("CONTRARIAN"),
            2,
            "CONTRARIAN should get 2x boost with critical context"
        );
        assert_eq!(
            find("NITPICK"),
            2,
            "NITPICK should get 2x boost with critical context"
        );
        assert_eq!(
            find("SARCASTIC"),
            2,
            "SARCASTIC should get 2x boost with critical context"
        );
        assert_eq!(
            find("DRY_WIT"),
            2,
            "DRY_WIT should get 2x boost with critical context"
        );

        // Should be higher than wholesome strategies
        assert!(
            find("CALLOUT") > find("WHOLEsome"),
            "CALLOUT weight ({}) should exceed WHOLEsome weight ({}) with negative sentiment",
            find("CALLOUT"),
            find("WHOLEsome")
        );
        assert!(
            find("CALLOUT") > find("COMPLIMENT"),
            "CALLOUT weight should exceed COMPLIMENT weight with negative sentiment"
        );

        // Non-boosted strategies should stay at 1
        assert_eq!(find("NPC"), 1, "NPC should stay at weight 1");
        assert_eq!(find("SLANG"), 1, "SLANG should stay at weight 1");
    }

    #[test]
    fn test_neutral_sentiment_no_boosts() {
        let ctx = sentiment_to_strategy_context(Sentiment::Neutral, "just a random tweet");
        assert_eq!(
            ctx.sentiment, "",
            "Neutral sentiment should produce empty context string"
        );

        let pool = get_weighted_pool(&ctx);
        for (_, w) in &pool {
            assert_eq!(
                *w, 1,
                "All strategies should stay at weight 1 with neutral sentiment"
            );
        }
    }

    #[test]
    fn test_positive_sentiment_maps_to_wholesome() {
        let ctx = sentiment_to_strategy_context(Sentiment::Positive, "just a random tweet");
        assert_eq!(ctx.sentiment, "wholesome");
    }

    #[test]
    fn test_negative_sentiment_maps_to_critical() {
        let ctx = sentiment_to_strategy_context(Sentiment::Negative, "just a random tweet");
        assert_eq!(ctx.sentiment, "critical");
    }

    // ── Conversation Type Detection Tests ──────────────────────────────

    #[test]
    fn test_classify_tech_tweet() {
        let topic =
            classify_conversation_type("Just shipped a new Python API for our cloud startup");
        assert_eq!(topic, "tech", "Tech keywords should classify as tech");
    }

    #[test]
    fn test_classify_politics_tweet() {
        let topic = classify_conversation_type("The election results show democracy at work");
        assert_eq!(
            topic, "politics",
            "Politics keywords should classify as politics"
        );
    }

    #[test]
    fn test_classify_gaming_tweet() {
        let topic = classify_conversation_type(
            "Just finished that new PS5 game, the open world is incredible",
        );
        assert_eq!(topic, "gaming", "Gaming keywords should classify as gaming");
    }

    #[test]
    fn test_classify_food_tweet() {
        let topic =
            classify_conversation_type("Made a delicious vegan pasta recipe for dinner tonight");
        assert_eq!(topic, "food", "Food keywords should classify as food");
    }

    #[test]
    fn test_classify_science_tweet() {
        let topic =
            classify_conversation_type("New physics research from NASA shows quantum breakthrough");
        assert_eq!(
            topic, "science",
            "Science keywords should classify as science"
        );
    }

    #[test]
    fn test_classify_finance_tweet() {
        let topic = classify_conversation_type("Bitcoin and crypto stocks are crashing today");
        assert_eq!(
            topic, "finance",
            "Finance keywords should classify as finance"
        );
    }

    #[test]
    fn test_classify_entertainment_tweet() {
        let topic =
            classify_conversation_type("That new Netflix movie is the best film of the year");
        assert_eq!(
            topic, "entertainment",
            "Entertainment keywords should classify as entertainment"
        );
    }

    #[test]
    fn test_classify_news_tweet() {
        let topic =
            classify_conversation_type("Breaking news: major announcement from the president");
        assert_eq!(topic, "news", "News keywords should classify as news");
    }

    #[test]
    fn test_classify_debate_tweet() {
        let topic = classify_conversation_type(
            "Unpopular opinion but I actually disagree with this hot take",
        );
        assert_eq!(topic, "debate", "Debate keywords should classify as debate");
    }

    #[test]
    fn test_classify_unknown_tweet_returns_empty() {
        let topic = classify_conversation_type("Just had a wonderful walk in the park today");
        assert_eq!(topic, "", "Non-matching tweet should return empty string");
    }

    #[test]
    fn test_classify_short_tweet_no_false_positives() {
        // Short text shouldn't accidentally match keywords
        let topic = classify_conversation_type("Hello world");
        assert_eq!(topic, "", "Short casual text should return empty string");
    }

    #[test]
    fn test_classify_case_insensitive() {
        let topic = classify_conversation_type("I love PYTHON programming and AI");
        assert_eq!(topic, "tech", "Keyword matching should be case-insensitive");
    }

    #[test]
    fn test_classify_empty_text_returns_empty() {
        let topic = classify_conversation_type("");
        assert_eq!(topic, "", "Empty text should return empty string");
    }

    #[test]
    fn test_classify_wins_by_keyword_count() {
        // Mix of tech and food keywords — tech has more matches
        let topic = classify_conversation_type(
            "Built a Python API for my startup, then cooked pasta for dinner",
        );
        assert_eq!(
            topic, "tech",
            "Should pick topic with most keyword hits (tech > food)"
        );
    }

    // ── Sentiment + Conversation Type Integration Tests ─────────────────

    #[test]
    fn test_sentiment_with_conversation_type_sets_both_fields() {
        let ctx = sentiment_to_strategy_context(
            Sentiment::Positive,
            "This AI startup just shipped an amazing Python API",
        );
        assert_eq!(
            ctx.sentiment, "wholesome",
            "Sentiment should map to wholesome"
        );
        assert_eq!(
            ctx.conversation_type, "tech",
            "Tweet should classify as tech"
        );
    }

    #[test]
    fn test_sentiment_with_conversation_type_boosts_combined_weights() {
        // Positive sentiment (wholesome) + tech conversation type
        let ctx = sentiment_to_strategy_context(
            Sentiment::Positive,
            "This AI startup just shipped an amazing Python API",
        );
        let pool = get_weighted_pool(&ctx);

        let find = |name: &str| -> u32 {
            pool.iter()
                .find(|(k, _)| *k == name)
                .map(|(_, w)| *w)
                .unwrap_or(0)
        };

        // From wholesome sentiment
        assert_eq!(
            find("WHOLEsome"),
            4,
            "WHOLEsome should be 4x from sentiment"
        );
        // From tech conversation type
        assert_eq!(find("CURIOUS"), 3, "CURIOUS should be 3x from tech context");
        // From both
        assert_eq!(
            find("COMPLIMENT"),
            2,
            "COMPLIMENT should be 2x from sentiment"
        );
    }

    #[test]
    fn test_negative_politics_boosts_observation_and_contrarian() {
        // Negative sentiment (critical) + politics
        let ctx = sentiment_to_strategy_context(
            Sentiment::Negative,
            "The election results are terrible for democracy",
        );
        let pool = get_weighted_pool(&ctx);

        let find = |name: &str| -> u32 {
            pool.iter()
                .find(|(k, _)| *k == name)
                .map(|(_, w)| *w)
                .unwrap_or(0)
        };

        // From critical: CALLOUT 3, CONTRARIAN 2
        assert_eq!(find("CALLOUT"), 3, "CALLOUT should be 3x from critical");
        // From politics: OBSERVATION 3, CONTRARIAN 3
        assert_eq!(
            find("OBSERVATION"),
            3,
            "OBSERVATION should be 3x from politics context"
        );
        // Combined: CONTRARIAN gets max(2 from critical, 3 from politics) = 3
        assert_eq!(
            find("CONTRARIAN"),
            3,
            "CONTRARIAN should be 3x (max of critical 2 and politics 3)"
        );
    }

    /// Statistical test: runs many random selections to verify that positive vs negative
    /// sentiment produces measurably different strategy selections.
    #[test]
    fn test_positive_vs_negative_sentiment_measurably_different_selections() {
        use std::collections::HashMap;

        // Use generic tweet text that doesn't match any conversation type
        let positive_ctx = sentiment_to_strategy_context(
            Sentiment::Positive,
            "just a random tweet with no keywords",
        );
        let negative_ctx = sentiment_to_strategy_context(
            Sentiment::Negative,
            "just a random tweet with no keywords",
        );

        // Run many random selections for each sentiment
        let mut pos_counts: HashMap<&str, u32> = HashMap::new();
        let mut neg_counts: HashMap<&str, u32> = HashMap::new();

        let iterations = 2000;
        for _ in 0..iterations {
            let pos_instruction = get_strategy_instruction(&positive_ctx);
            let neg_instruction = get_strategy_instruction(&negative_ctx);

            for (name, instr) in STRATEGY_INSTRUCTIONS {
                if *instr == pos_instruction {
                    *pos_counts.entry(*name).or_insert(0) += 1;
                }
                if *instr == neg_instruction {
                    *neg_counts.entry(*name).or_insert(0) += 1;
                }
            }
        }

        let pos_wholesome = pos_counts.get("WHOLEsome").copied().unwrap_or(0);
        let neg_wholesome = neg_counts.get("WHOLEsome").copied().unwrap_or(0);

        // WHOLEsome has weight 4/38 (~10.5%) with positive, 1/38 (~2.6%) with negative
        // Expect ~210 vs ~52 out of 2000. Use 1.5x margin to avoid flaky failures.
        assert!(
            pos_wholesome > neg_wholesome * 2,
            "Positive sentiment should pick WHOLEsome much more often than negative. \
             Positive: {pos_wholesome}/{iterations}, Negative: {neg_wholesome}/{iterations}"
        );

        let pos_callout = pos_counts.get("CALLOUT").copied().unwrap_or(0);
        let neg_callout = neg_counts.get("CALLOUT").copied().unwrap_or(0);

        // CALLOUT has weight 3/38 (~7.9%) with negative, 1/38 (~2.6%) with positive
        // Expect ~158 vs ~52 out of 2000.
        assert!(
            neg_callout > pos_callout * 2,
            "Negative sentiment should pick CALLOUT much more often than positive. \
             Negative: {neg_callout}/{iterations}, Positive: {pos_callout}/{iterations}"
        );

        // Verify total selections match expected
        let pos_total: u32 = pos_counts.values().sum();
        let neg_total: u32 = neg_counts.values().sum();
        assert_eq!(
            pos_total, iterations,
            "All positive selections should be accounted for"
        );
        assert_eq!(
            neg_total, iterations,
            "All negative selections should be accounted for"
        );
    }
}
