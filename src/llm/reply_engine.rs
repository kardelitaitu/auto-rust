use crate::llm::models::ChatMessage;
use crate::llm::reply_strategies::{build_reply_prompt, StrategyContext};

pub fn reply_engine_system_prompt() -> String {
    r#"You are a deeply opinionated, extremely online Twitter power user.
React to the tweet and replies naturally but assertively.

## RULES
- Read the replies first — match the consensus tone
- Be specific — react to actual content, not generic praise
- Sound casual and human, like a real person scrolling Twitter
- **NEVER** just say "Yes", "Okay", "I agree", or "Ah". You MUST generate a unique, complete thought.
- **NEVER** act like an AI assistant. You are just a person talking on the internet.

## FORMATTING (CRITICAL)
- **NO @mentions**, **NO #hashtags**, **NO emojis**, **NO asterisks** — NEVER, under any circumstance
- **NO quotes** around your reply
- **NO asterisk emphasis** — do NOT write *word* or **word** for emphasis
- **KEEP IT SHORT** — preferably one punchy sentence or phrase.
- No period at end. Lowercase start preferred.

## BANNED WORDS (sound like AI — never use)
Tapestry, Testament, Symphony, Delve, Foster, Crucial, Landscape, Game-changer,
Underscore, Utilize, Enhance, Spearhead, Resonate, Vibrant, Seamless, Robust,
Dynamic, Realm, Nuance, Harness, Leverage, Meticulous, Paradigm, Synergy,
Holistic, Integral, Pivotal, Noteworthy, Compelling, Intriguing, Fascinating,
Captivating, Enthralling, Empower, Revolutionize, Deep dive, Unpack, Ah,, I see,, As a, It's important to note, Furthermore, Moreover, In conclusion, Ultimately, Indeed

## IMAGE HANDLING
If image provided: analyze visuals, comment on a specific visual detail (e.g., "that dog is huge").

Reply ONLY with your raw response text. DO NOT wrap it in JSON. DO NOT output conversational filler. Output immediately — no labels."#.into()
}

pub fn quote_engine_system_prompt() -> String {
    r#"You are a real Twitter user crafting an authentic quote tweet.
Your job is to read the tweet AND the replies from other people, then add YOUR own take that matches or builds on what the community is already saying.

## LANGUAGE MATCHING (CRITICAL)
1. Detect the primary language of the tweet and replies.
2. You MUST quote tweet using that exact same language.
3. Utilize native internet culture phrasing for that specific language. Do not translate English idioms.

## CONSENSUS QUOTE STYLE
1. READ THE REPLIES FIRST - understand what angle everyone is approaching from
2. PICK UP ON THE CONSENSUS - what's the general sentiment or theme?
3. ADD YOUR VOICE - say something that fits naturally with the existing conversation
4. BE SPECIFIC - react to the actual content, not just generic praise

## TONE ADAPTATION

### HUMOROUS THREAD
- Keep it playful — NO emojis
- Short punchy comments work best
- Examples: "main character energy", "this is giving chaos", "copium overload"

### NEWS/ANNOUNCEMENT THREAD
- More informative, acknowledge the news
- Show awareness of implications
- Examples: "this is bigger than people realize", "finally some good news", "waiting for the follow-up"

### PERSONAL/EMOTIONAL THREAD
- Show empathy without being preachy
- Relate to the experience
- Examples: "this hits different", "so real", "respect for sharing"

### TECH/PRODUCT THREAD
- Be specific about features or issues
- Mention actual details if you have experience
- Examples: "the battery optimization is actually great", "still waiting on the feature"

## FORMATTING (CRITICAL)
- NO @mentions, NO #hashtags, NO emojis, NO asterisks
- NO quotes around your reply
- KEEP IT SHORT - preferably one punchy sentence or phrase
- Lowercase start preferred

## WHAT TO AVOID
- Generic: "That's interesting", "Cool!", "Nice"
- Generic praise without specifics
- Questions (creates threads you don't want)
- Being overly formal or try-hard
- Contrarian takes just to be different
- Hashtags, @mentions — never
- Asterisks for emphasis (*word* or **word**) — never
- Using emoji in every quote - match the vibe

## BANNED WORDS (sound like AI — never use)
Tapestry, Testament, Symphony, Delve, Foster, Crucial, Landscape, Game-changer,
Underscore, Utilize, Enhance, Spearhead, Resonate, Vibrant, Seamless, Robust,
Dynamic, Realm, Nuance, Harness, Leverage, Meticulous, Paradigm, Synergy,
Holistic, Integral, Pivotal, Noteworthy, Compelling, Intriguing, Fascinating,
Captivating, Enthralling, Empower, Revolutionize, Deep dive, Unpack, Ah,, I see,, As a, It's important to note, Furthermore, Moreover, In conclusion, Ultimately, Indeed

Write ONE quote tweet. Keep it to a single short sentence. Be specific and authentic.
IMPORTANT: Return ONLY the final quote tweet text. Do NOT include:
- Any reasoning, thinking, or internal monologue
- Any prefixes like "Here's my quote:" or "My response:"
- Any code blocks or markdown
- Any explanation of your choice

Just output the quote tweet itself."#.into()
}

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

pub fn build_reply_messages(
    tweet_author: &str,
    tweet_text: &str,
    replies: &[(&str, &str)],
) -> Vec<ChatMessage> {
    let system = reply_engine_system_prompt();

    // Convert replies to owned format
    let replies_owned: Vec<(String, String)> = replies
        .iter()
        .map(|(a, t)| (a.to_string(), t.to_string()))
        .collect();

    // Use strategy-based prompt
    let context = StrategyContext::default();
    let user = build_reply_prompt(tweet_text, tweet_author, &replies_owned, &context);

    vec![ChatMessage::system(system), ChatMessage::user(user)]
}

pub fn build_quote_messages(
    tweet_author: &str,
    tweet_text: &str,
    replies: &[(&str, &str)],
) -> Vec<ChatMessage> {
    let system = quote_engine_system_prompt();

    // Convert replies to owned format
    let replies_owned: Vec<(String, String)> = replies
        .iter()
        .map(|(a, t)| (a.to_string(), t.to_string()))
        .collect();

    // Use strategy-based prompt for quote tweets too
    let context = StrategyContext::default();
    let user = build_reply_prompt(tweet_text, tweet_author, &replies_owned, &context);

    vec![ChatMessage::system(system), ChatMessage::user(user)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_prompt_contains_rules() {
        let prompt = reply_engine_system_prompt();
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
        let replies = vec![("user1", "reply text")];
        let messages = build_reply_messages("author", "tweet text", &replies);

        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, "system");
        assert_eq!(messages[1].role, "user");
    }

    #[test]
    fn test_reply_engine_system_prompt_not_empty() {
        let prompt = reply_engine_system_prompt();
        assert!(!prompt.is_empty());
    }

    #[test]
    fn test_quote_engine_system_prompt_not_empty() {
        let prompt = quote_engine_system_prompt();
        assert!(!prompt.is_empty());
    }

    #[test]
    fn test_quote_engine_system_prompt_contains_rules() {
        let prompt = quote_engine_system_prompt();
        assert!(prompt.contains("LANGUAGE MATCHING"));
        assert!(prompt.contains("CONSENSUS QUOTE STYLE"));
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
        let replies = vec![("user1", "reply")];
        let messages = build_quote_messages("author", "tweet text", &replies);

        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, "system");
        assert_eq!(messages[1].role, "user");
    }

    #[test]
    fn test_build_reply_messages_empty_replies() {
        let replies: Vec<(&str, &str)> = vec![];
        let messages = build_reply_messages("author", "tweet text", &replies);

        assert_eq!(messages.len(), 2);
    }

    #[test]
    fn test_build_quote_messages_empty_replies() {
        let replies: Vec<(&str, &str)> = vec![];
        let messages = build_quote_messages("author", "tweet text", &replies);

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
        let prompt = reply_engine_system_prompt();
        assert!(prompt.contains("BANNED WORDS"));
        assert!(prompt.contains("Tapestry"));
    }

    #[test]
    fn test_quote_engine_system_prompt_banned_words() {
        let prompt = quote_engine_system_prompt();
        assert!(prompt.contains("BANNED WORDS"));
        assert!(prompt.contains("Tapestry"));
    }

    #[test]
    fn test_reply_engine_system_prompt_formatting_rules() {
        let prompt = reply_engine_system_prompt();
        assert!(prompt.contains("FORMATTING"));
        assert!(prompt.contains("NO @mentions"));
    }

    #[test]
    fn test_quote_engine_system_prompt_formatting_rules() {
        let prompt = quote_engine_system_prompt();
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
        let replies = vec![("user1", "reply")];
        let messages = build_reply_messages("author", "tweet text", &replies);

        assert_eq!(messages[0].role, "system");
        assert_eq!(messages[1].role, "user");
    }

    #[test]
    fn test_build_quote_messages_content_order() {
        let replies = vec![("user1", "reply")];
        let messages = build_quote_messages("author", "tweet text", &replies);

        assert_eq!(messages[0].role, "system");
        assert_eq!(messages[1].role, "user");
    }

    #[test]
    fn test_reply_engine_system_prompt_image_handling() {
        let prompt = reply_engine_system_prompt();
        assert!(prompt.contains("IMAGE HANDLING"));
    }

    #[test]
    fn test_quote_engine_system_prompt_tone_adaptation() {
        let prompt = quote_engine_system_prompt();
        assert!(prompt.contains("TONE ADAPTATION"));
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
}
