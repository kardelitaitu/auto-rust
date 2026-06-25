use crate::llm::models::ChatMessage;
use crate::llm::reply_strategies::{build_reply_prompt, StrategyContext};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TwitterPersona {
    Default,
    GenZ,
    Professional,
    Satirical,
    Brief,
}

impl TwitterPersona {
    pub fn select_for_session(session_id: &str) -> Self {
        let hash = session_id
            .bytes()
            .fold(0u32, |acc, b| acc.wrapping_add(b as u32));
        match hash % 5 {
            0 => TwitterPersona::Default,
            1 => TwitterPersona::GenZ,
            2 => TwitterPersona::Professional,
            3 => TwitterPersona::Satirical,
            _ => TwitterPersona::Brief,
        }
    }
}

#[must_use]
pub fn reply_engine_system_prompt(persona: TwitterPersona) -> String {
    let mut base_prompt = match persona {
        TwitterPersona::Default => r#"You are a deeply opinionated, extremely online Twitter power user.
React to the tweet and replies naturally but assertively.
- Sound casual and human, like a real person scrolling Twitter.
- Keep it short: strictly 1 or 2 sentences maximum."#.to_string(),
        TwitterPersona::GenZ => r#"You are a Gen-Z Twitter user.
React to the tweet using very online Gen-Z internet slang and high energy.
- Use lowercase starting characters, casual phrasing, and internet slang (e.g., "fr", "no cap", "it's giving", "let him cook", "bruh", "wild", "real").
- Do NOT use emojis.
- Keep it extremely casual and short."#.to_string(),
        TwitterPersona::Professional => r#"You are an insightful tech professional and builder.
React to the tweet with a professional, clean, and knowledgeable perspective.
- Sound articulate, logical, and expert.
- Focus on practical reality, software concepts, or building blocks.
- Keep it short: strictly 1 or 2 sentences maximum."#.to_string(),
        TwitterPersona::Satirical => r#"You are a witty, dry, and slightly sarcastic Twitter user.
React to the tweet with dry humor, irony, or mild sarcasm.
- Highlight absurdity or make a clever, slightly cynical observation.
- Be funny but not mean-spirited.
- Keep it short: strictly 1 or 2 sentences maximum."#.to_string(),
        TwitterPersona::Brief => r#"You are an extremely brief Twitter user.
React to the tweet in a minimal way.
- Keep it strictly to one short sentence or phrase (3 to 6 words maximum).
- Make it direct and punchy (e.g., "this is the way", "absolutely correct", "couldn't agree more")."#.to_string(),
    };

    let constraints = r#"

## FORMATTING RULES (CRITICAL)
- NO @mentions, NO #hashtags, NO emojis, NO asterisks — NEVER, under any circumstance.
- NO quotes around your reply.
- NO asterisk emphasis — do NOT write *word* or **word** for emphasis.
- No period at end. Lowercase start preferred.
- NEVER just say "Yes", "Okay", "I agree", or "Ah". You MUST generate a unique, complete thought.
- NEVER act like an AI assistant. You are just a person talking on the internet.

## BANNED WORDS (sound like AI — never use)
Tapestry, Testament, Symphony, Delve, Foster, Crucial, Landscape, Game-changer,
Underscore, Utilize, Enhance, Spearhead, Resonate, Vibrant, Seamless, Robust,
Dynamic, Realm, Nuance, Harness, Leverage, Meticulous, Paradigm, Synergy,
Holistic, Integral, Pivotal, Noteworthy, Compelling, Intriguing, Fascinating,
Captivating, Enthralling, Empower, Revolutionize, Deep dive, Unpack, Ah,, I see,, As a, It's important to note, Furthermore, Moreover, In conclusion, Ultimately, Indeed

## IMAGE HANDLING
If image provided: analyze visuals, comment on a specific visual detail.

Reply ONLY with your raw response text. DO NOT wrap it in JSON. Output immediately."#;

    base_prompt.push_str(constraints);
    base_prompt
}

#[must_use]
pub fn quote_engine_system_prompt(persona: TwitterPersona) -> String {
    let mut base_prompt = match persona {
        TwitterPersona::Default => r#"You are a real Twitter user crafting an authentic quote tweet take.
Read the tweet and replies, then add your own take that builds on what the community is saying."#.to_string(),
        TwitterPersona::GenZ => r#"You are a Gen-Z Twitter user crafting a quote tweet take.
Use very online Gen-Z internet slang, lowercase start, and high casual energy (e.g. "fr", "it's giving", "real")."#.to_string(),
        TwitterPersona::Professional => r#"You are a tech professional/builder quote tweeting.
Provide an insightful, clean, and knowledgeable comment building on the technical/practical aspect."#.to_string(),
        TwitterPersona::Satirical => r#"You are a dry, witty quote tweeter.
Add a slightly sarcastic, dryly humorous, or ironic comment on the tweet's theme."#.to_string(),
        TwitterPersona::Brief => r#"You are a minimal quote tweeter.
Keep your quote commentary strictly to a very short phrase (3 to 6 words maximum)."#.to_string(),
    };

    let constraints = r#"

## LANGUAGE MATCHING (CRITICAL)
1. Detect the primary language of the tweet and replies.
2. You MUST quote tweet using that exact same language.
3. Utilize native internet culture phrasing for that specific language. Do not translate English idioms.

## FORMATTING RULES (CRITICAL)
- NO @mentions, NO #hashtags, NO emojis, NO asterisks — NEVER, under any circumstance.
- NO quotes around your reply.
- NO asterisk emphasis — do NOT write *word* or **word** for emphasis.
- KEEP IT SHORT — strictly 1 or 2 sentences maximum.
- Lowercase start preferred.

## BANNED WORDS (sound like AI — never use)
Tapestry, Testament, Symphony, Delve, Foster, Crucial, Landscape, Game-changer,
Underscore, Utilize, Enhance, Spearhead, Resonate, Vibrant, Seamless, Robust,
Dynamic, Realm, Nuance, Harness, Leverage, Meticulous, Paradigm, Synergy,
Holistic, Integral, Pivotal, Noteworthy, Compelling, Intriguing, Fascinating,
Captivating, Enthralling, Empower, Revolutionize, Deep dive, Unpack, Ah,, I see,, As a, It's important to note, Furthermore, Moreover, In conclusion, Ultimately, Indeed

Just output the quote tweet text itself."#;

    base_prompt.push_str(constraints);
    base_prompt
}

#[must_use]
pub fn reply_engine_user_prompt(
    tweet_author: &str,
    tweet_text: &str,
    replies: &[(&str, &str)],
) -> String {
    let mut prompt = format!("Tweet by @{}:\n{}", tweet_author, tweet_text.trim());

    if !replies.is_empty() {
        prompt.push_str("\n\nReplies:\n");
        for (author, text) in replies {
            prompt.push_str(&format!("@{}: {}\n", author, text.trim()));
        }
    }

    prompt.push_str("\n\nYour reply:");
    prompt
}

#[must_use]
pub fn quote_engine_user_prompt(
    tweet_author: &str,
    tweet_text: &str,
    replies: &[(&str, &str)],
) -> String {
    let mut prompt = format!(
        "Quote this tweet by @{}:\n{}",
        tweet_author,
        tweet_text.trim()
    );

    if !replies.is_empty() {
        prompt.push_str("\n\nCommunity replies:\n");
        for (author, text) in replies {
            prompt.push_str(&format!("@{}: {}\n", author, text.trim()));
        }
    }

    prompt.push_str("\n\nYour quote tweet:");
    prompt
}

#[must_use]
pub fn build_reply_messages(
    tweet_author: &str,
    tweet_text: &str,
    replies: &[(&str, &str)],
    context: &StrategyContext,
    persona: TwitterPersona,
) -> Vec<ChatMessage> {
    let system = reply_engine_system_prompt(persona);

    // Convert replies to owned format
    let replies_owned: Vec<(String, String)> = replies
        .iter()
        .map(|(a, t)| (a.to_string(), t.to_string()))
        .collect();

    // Use strategy-based prompt
    let user = build_reply_prompt(tweet_text, tweet_author, &replies_owned, context, false);

    vec![ChatMessage::system(system), ChatMessage::user(user)]
}

#[must_use]
pub fn build_quote_messages(
    tweet_author: &str,
    tweet_text: &str,
    replies: &[(&str, &str)],
    context: &StrategyContext,
    persona: TwitterPersona,
) -> Vec<ChatMessage> {
    let system = quote_engine_system_prompt(persona);

    // Convert replies to owned format
    let replies_owned: Vec<(String, String)> = replies
        .iter()
        .map(|(a, t)| (a.to_string(), t.to_string()))
        .collect();

    // Use strategy-based prompt for quote tweets too
    let user = build_reply_prompt(tweet_text, tweet_author, &replies_owned, context, false);

    vec![ChatMessage::system(system), ChatMessage::user(user)]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::models::Role;

    #[test]
    fn test_system_prompt_contains_rules() {
        let prompt = reply_engine_system_prompt(TwitterPersona::Default);
        assert!(prompt.contains("RULES"));
        assert!(prompt.contains("BANNED WORDS"));
    }

    #[test]
    fn test_user_prompt_formats_correctly() {
        let replies = vec![("user1", "Great point!"), ("user2", "I disagree")];
        let user = reply_engine_user_prompt("testuser", "Hello world!", &replies);

        assert!(user.contains("Tweet by @testuser:"));
        assert!(user.contains("Replies:"));
        assert!(user.contains("@user1: Great point!"));
    }

    #[test]
    fn test_build_messages_includes_system_and_user() {
        let context = StrategyContext::default();
        let replies = vec![("user1", "reply text")];
        let messages = build_reply_messages(
            "author",
            "tweet text",
            &replies,
            &context,
            TwitterPersona::Default,
        );

        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, Role::System);
        assert_eq!(messages[1].role, Role::User);
    }

    #[test]
    fn test_reply_engine_system_prompt_not_empty() {
        let prompt = reply_engine_system_prompt(TwitterPersona::Default);
        assert!(!prompt.is_empty());
    }

    #[test]
    fn test_quote_engine_system_prompt_not_empty() {
        let prompt = quote_engine_system_prompt(TwitterPersona::Default);
        assert!(!prompt.is_empty());
    }

    #[test]
    fn test_quote_engine_system_prompt_contains_rules() {
        let prompt = quote_engine_system_prompt(TwitterPersona::Default);
        assert!(prompt.contains("LANGUAGE MATCHING"));
        assert!(prompt.contains("FORMATTING RULES"));
    }

    #[test]
    fn test_user_prompt_without_replies() {
        let replies: Vec<(&str, &str)> = vec![];
        let user = reply_engine_user_prompt("testuser", "Hello world!", &replies);

        assert!(user.contains("Tweet by @testuser:"));
        assert!(!user.contains("Replies:"));
    }

    #[test]
    fn test_quote_user_prompt_without_replies() {
        let replies: Vec<(&str, &str)> = vec![];
        let user = quote_engine_user_prompt("testuser", "Hello world!", &replies);

        assert!(user.contains("Quote this tweet by @testuser:"));
        assert!(!user.contains("Community replies:"));
    }

    #[test]
    fn test_quote_user_prompt_with_replies() {
        let replies = vec![("user1", "Great!"), ("user2", "Agreed")];
        let user = quote_engine_user_prompt("testuser", "Hello world!", &replies);

        assert!(user.contains("Community replies:"));
        assert!(user.contains("@user1: Great!"));
    }

    #[test]
    fn test_build_quote_messages_structure() {
        let context = StrategyContext::default();
        let replies = vec![("user1", "reply")];
        let messages = build_quote_messages(
            "author",
            "tweet text",
            &replies,
            &context,
            TwitterPersona::Default,
        );

        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, Role::System);
        assert_eq!(messages[1].role, Role::User);
    }

    #[test]
    fn test_build_reply_messages_empty_replies() {
        let context = StrategyContext::default();
        let replies: Vec<(&str, &str)> = vec![];
        let messages = build_reply_messages(
            "author",
            "tweet text",
            &replies,
            &context,
            TwitterPersona::Default,
        );

        assert_eq!(messages.len(), 2);
    }

    #[test]
    fn test_build_quote_messages_empty_replies() {
        let context = StrategyContext::default();
        let replies: Vec<(&str, &str)> = vec![];
        let messages = build_quote_messages(
            "author",
            "tweet text",
            &replies,
            &context,
            TwitterPersona::Default,
        );

        assert_eq!(messages.len(), 2);
    }

    #[test]
    fn test_user_prompt_with_single_reply() {
        let replies = vec![("user1", "Only reply")];
        let user = reply_engine_user_prompt("testuser", "tweet", &replies);

        assert!(user.contains("@user1: Only reply"));
    }

    #[test]
    fn test_user_prompt_with_multiple_replies() {
        let replies = vec![("user1", "first"), ("user2", "second"), ("user3", "third")];
        let user = reply_engine_user_prompt("testuser", "tweet", &replies);

        assert!(user.contains("@user1: first"));
        assert!(user.contains("@user2: second"));
        assert!(user.contains("@user3: third"));
    }

    #[test]
    fn test_reply_engine_system_prompt_banned_words() {
        let prompt = reply_engine_system_prompt(TwitterPersona::Default);
        assert!(prompt.contains("BANNED WORDS"));
        assert!(prompt.contains("Tapestry"));
    }

    #[test]
    fn test_quote_engine_system_prompt_banned_words() {
        let prompt = quote_engine_system_prompt(TwitterPersona::Default);
        assert!(prompt.contains("BANNED WORDS"));
        assert!(prompt.contains("Tapestry"));
    }

    #[test]
    fn test_reply_engine_system_prompt_formatting_rules() {
        let prompt = reply_engine_system_prompt(TwitterPersona::Default);
        assert!(prompt.contains("FORMATTING"));
        assert!(prompt.contains("NO @mentions"));
    }

    #[test]
    fn test_quote_engine_system_prompt_formatting_rules() {
        let prompt = quote_engine_system_prompt(TwitterPersona::Default);
        assert!(prompt.contains("FORMATTING"));
        assert!(prompt.contains("NO @mentions"));
    }

    #[test]
    fn test_user_prompt_ends_with_your_reply() {
        let replies = vec![("user1", "reply")];
        let user = reply_engine_user_prompt("testuser", "tweet", &replies);

        assert!(user.ends_with("Your reply:"));
    }

    #[test]
    fn test_quote_user_prompt_ends_with_quote_tweet() {
        let replies = vec![("user1", "reply")];
        let user = quote_engine_user_prompt("testuser", "tweet", &replies);

        assert!(user.ends_with("Your quote tweet:"));
    }

    #[test]
    fn test_build_reply_messages_content_order() {
        let context = StrategyContext::default();
        let replies = vec![("user1", "reply")];
        let messages = build_reply_messages(
            "author",
            "tweet text",
            &replies,
            &context,
            TwitterPersona::Default,
        );

        assert_eq!(messages[0].role, Role::System);
        assert_eq!(messages[1].role, Role::User);
    }

    #[test]
    fn test_build_quote_messages_content_order() {
        let context = StrategyContext::default();
        let replies = vec![("user1", "reply")];
        let messages = build_quote_messages(
            "author",
            "tweet text",
            &replies,
            &context,
            TwitterPersona::Default,
        );

        assert_eq!(messages[0].role, Role::System);
        assert_eq!(messages[1].role, Role::User);
    }

    #[test]
    fn test_reply_engine_system_prompt_image_handling() {
        let prompt = reply_engine_system_prompt(TwitterPersona::Default);
        assert!(prompt.contains("IMAGE HANDLING"));
    }

    #[test]
    fn test_quote_engine_system_prompt_tone_adaptation() {
        let prompt = quote_engine_system_prompt(TwitterPersona::Default);
        assert!(prompt.contains("LANGUAGE MATCHING"));
    }

    #[test]
    fn test_user_prompt_trims_tweet_text() {
        let replies = vec![];
        let user = reply_engine_user_prompt("testuser", "  tweet with spaces  ", &replies);

        assert!(user.contains("tweet with spaces"));
    }

    #[test]
    fn test_quote_user_prompt_trims_tweet_text() {
        let replies = vec![];
        let user = quote_engine_user_prompt("testuser", "  tweet with spaces  ", &replies);

        assert!(user.contains("tweet with spaces"));
    }

    #[test]
    fn test_persona_selection_is_deterministic() {
        let p1 = TwitterPersona::select_for_session("session-123");
        let p2 = TwitterPersona::select_for_session("session-123");
        assert_eq!(p1, p2);

        let mut distinct = false;
        for i in 0..100 {
            let session = format!("session-{}", i);
            if TwitterPersona::select_for_session(&session) != TwitterPersona::Default {
                distinct = true;
                break;
            }
        }
        assert!(
            distinct,
            "Should generate distinct personas across different session IDs"
        );
    }
}
