use crate::llm::models::ChatMessage;

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

## FORMATTING (CRITICAL)
- NO @mentions, NO #hashtags, NO emojis, NO asterisks
- NO quotes around your reply
- KEEP IT SHORT - preferably one punchy sentence or phrase
- Lowercase start preferred

Reply ONLY with your raw response text. DO NOT wrap it in JSON."#.into()
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
    let user = reply_engine_user_prompt(tweet_author, tweet_text, replies);

    vec![ChatMessage::system(system), ChatMessage::user(user)]
}

pub fn build_quote_messages(
    tweet_author: &str,
    tweet_text: &str,
    replies: &[(&str, &str)],
) -> Vec<ChatMessage> {
    let system = quote_engine_system_prompt();
    let user = quote_engine_user_prompt(tweet_author, tweet_text, replies);

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
}
