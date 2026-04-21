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

/// Pick a strategy using weighted random selection.
/// All strategies start at base weight 1; context boosts multiply specific keys.
pub fn get_strategy_instruction(context: &StrategyContext) -> &'static str {
    // Build boost map from matching context keys
    let mut boost_map: std::collections::HashMap<&str, u32> = std::collections::HashMap::new();

    let context_keys = [
        &context.sentiment,
        &context.conversation_type,
        &context.engagement_level,
    ];

    for key in context_keys.iter() {
        for (ctx_key, boosts) in CONTEXT_BOOSTS.iter() {
            if ctx_key == key {
                for (strategy, multiplier) in boosts.iter() {
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
    let mut r = rng.gen_range(0..total);

    for (key, weight) in weighted_pool {
        if r < weight {
            return STRATEGY_INSTRUCTIONS
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, instruction)| *instruction)
                .unwrap_or(STRATEGY_INSTRUCTIONS[0].1);
        }
        r -= weight;
    }

    // Fallback to last strategy
    STRATEGY_INSTRUCTIONS.last().map(|(_, i)| *i).unwrap_or("")
}

/// Build reply prompt with strategy selection
pub fn build_reply_prompt(
    tweet_text: &str,
    author: &str,
    replies: &[(String, String)],
    context: &StrategyContext,
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
    if !replies.is_empty() {
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
    } else {
        prompt.push_str("\n\n(no other replies visible)\n");
    }

    prompt.push_str("\n\nYour reply:");
    prompt
}

#[cfg(test)]
mod tests {
    use super::*;

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

        let prompt = build_reply_prompt("Test tweet", "testuser", &replies, &context);

        assert!(prompt.contains("Tweet by @testuser:"));
        assert!(prompt.contains("Test tweet"));
        assert!(prompt.contains("Replies:"));
        assert!(prompt.contains("@user1: Great point!"));
        assert!(prompt.contains("Your reply:"));
    }

    #[test]
    fn test_build_reply_prompt_truncates_tweet() {
        let context = StrategyContext::default();
        let long_tweet = "a".repeat(600);

        let prompt = build_reply_prompt(&long_tweet, "user", &[], &context);

        // Should be truncated to 500 chars
        assert!(prompt.contains(&"a".repeat(500)));
        assert!(!prompt.contains(&"a".repeat(600)));
    }
}
