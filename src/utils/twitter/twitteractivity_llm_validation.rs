//! LLM output validation and sanitization for Twitter.

use anyhow::Result;
use std::sync::OnceLock;

/// Banned AI-sounding words list.
pub const BANNED_WORDS: &[&str] = &[
    "tapestry",
    "testament",
    "symphony",
    "delve",
    "foster",
    "crucial",
    "landscape",
    "game-changer",
    "underscore",
    "utilize",
    "enhance",
    "spearhead",
    "resonate",
    "vibrant",
    "seamless",
    "robust",
    "dynamic",
    "realm",
    "nuance",
    "harness",
    "leverage",
    "meticulous",
    "paradigm",
    "synergy",
    "holistic",
    "integral",
    "pivotal",
    "noteworthy",
    "compelling",
    "intriguing",
    "fascinating",
    "captivating",
    "enthralling",
    "empower",
    "revolutionize",
    "deep dive",
    "unpack",
    "in conclusion",
    "moreover",
    "furthermore",
    "it's important to note",
    "ah,",
    "i see",
    "as a",
];

/// Validates and sanitizes LLM-generated text for Twitter.
pub fn validate_reply(text: &str) -> Result<String> {
    let mut sanitized = text.trim().to_string();

    // Step 0: Strip markdown code block wrapping (```json...```) that some LLMs add.
    // This must happen before any other parsing.
    if let Some(inner) = strip_code_block(&sanitized) {
        sanitized = inner;
    }

    // Extract content if response is wrapped in JSON format
    if sanitized.starts_with('[') || sanitized.starts_with('{') {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&sanitized) {
            if let Some(arr) = val.as_array() {
                let mut found = None;
                for item in arr {
                    if let Some(obj) = item.as_object() {
                        if let Some(content) = obj.get("content").and_then(|v| v.as_str()) {
                            found = Some(content.to_string());
                            break;
                        }
                        if let Some(content) = obj.get("reply").and_then(|v| v.as_str()) {
                            found = Some(content.to_string());
                            break;
                        }
                        if let Some(content) = obj.get("text").and_then(|v| v.as_str()) {
                            found = Some(content.to_string());
                            break;
                        }
                    } else if let Some(s) = item.as_str() {
                        found = Some(s.to_string());
                        break;
                    }
                }
                if let Some(f) = found {
                    sanitized = f;
                }
            } else if let Some(obj) = val.as_object() {
                if let Some(content) = obj.get("content").and_then(|v| v.as_str()) {
                    sanitized = content.to_string();
                } else if let Some(content) = obj.get("reply").and_then(|v| v.as_str()) {
                    sanitized = content.to_string();
                } else if let Some(content) = obj.get("text").and_then(|v| v.as_str()) {
                    sanitized = content.to_string();
                }
            }
        }
    }

    // Remove "replies:" or "replies :" prefix lines (local LLMs often output
    // multiple reply options prefixed with "replies:" and "content:").
    // Take only the first non-empty content line.
    let lines: Vec<&str> = sanitized
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();

    // Check if the response looks like a multi-reply format with "replies:" / "content:"
    let has_replies_prefix = lines
        .iter()
        .any(|l| l.to_lowercase() == "replies:" || l.to_lowercase() == "replies :");
    let has_content_prefix = lines
        .iter()
        .any(|l| l.to_lowercase().starts_with("content:"));

    if has_replies_prefix || has_content_prefix {
        // Extract the first actual content line
        let extracted: Vec<&str> = lines
            .iter()
            .filter_map(|l| {
                let lower = l.to_lowercase();
                if lower == "replies:" || lower == "replies :" {
                    None
                } else if let Some(text) = l
                    .trim()
                    .strip_prefix("content:")
                    .or(l.trim().strip_prefix("Content:"))
                {
                    let trimmed = text.trim().trim_end_matches(',');
                    if !trimmed.is_empty() {
                        Some(trimmed)
                    } else {
                        None
                    }
                } else if let Some(text) = l
                    .trim()
                    .strip_prefix("replies:")
                    .or(l.trim().strip_prefix("Replies:"))
                {
                    let trimmed = text.trim().trim_end_matches(',');
                    if !trimmed.is_empty() {
                        Some(trimmed)
                    } else {
                        None
                    }
                } else {
                    // Regular line — only include if it looks like actual content
                    // (not a label line like "content:" with no text)
                    let cleaned = l.trim_end_matches(',');
                    if !cleaned.is_empty() {
                        Some(cleaned)
                    } else {
                        None
                    }
                }
            })
            .collect();

        if let Some(first) = extracted.first() {
            sanitized = first.to_string();
        } else {
            // Prefixes detected but no actual content extracted — don't
            // let a bare label like "replies:" pass through as the reply.
            anyhow::bail!("LLM response contains only labels without content");
        }
    }

    // Remove asterisk emphasis (**word** and *word*)
    sanitized = sanitized.replace("**", "").replace('*', "");

    // Filter out JSON symbols for safety (e.g. {}, [], "", and =), replacing or removing them
    sanitized = sanitized
        .chars()
        .filter(|&c| c != '{' && c != '}' && c != '[' && c != ']' && c != '"' && c != '=')
        .collect();

    // Enforce character limit
    if sanitized.len() > 270 {
        // Leave room for ...
        sanitized = truncate_to_word_boundary(&sanitized, 270);
    }

    // Remove @mentions
    sanitized = remove_mentions(&sanitized);

    // Remove #hashtags
    sanitized = remove_hashtags(&sanitized);

    // Remove emojis (basic Unicode range check)
    sanitized = remove_emojis(&sanitized);

    // Check for banned AI words
    if let Some(banned_word) = check_banned_words(&sanitized) {
        anyhow::bail!("Reply contains banned AI word: '{banned_word}'");
    }

    // Ensure non-empty
    if sanitized.is_empty() {
        anyhow::bail!("Generated reply is empty after sanitization");
    }

    Ok(sanitized)
}

/// Truncates text to `max_length` at word boundary.
fn truncate_to_word_boundary(text: &str, max_length: usize) -> String {
    if text.len() <= max_length {
        return text.to_string();
    }

    // Find last space before max_length (leave room for "...")
    let truncate_limit = max_length.saturating_sub(3);
    // Use char-safe boundary to avoid panic on multi-byte UTF-8
    let safe_boundary = text.floor_char_boundary(truncate_limit);
    let truncate_at = text[..safe_boundary].rfind(' ').unwrap_or(safe_boundary);

    format!("{}...", &text[..truncate_at])
}

fn compile_regex(pattern: &str) -> regex::Regex {
    match regex::Regex::new(pattern) {
        Ok(r) => r,
        Err(_) => match regex::Regex::new("") {
            Ok(r) => r,
            Err(_) => unreachable!("Regex parser failed on empty pattern"),
        },
    }
}

fn mentions_regex() -> &'static regex::Regex {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    RE.get_or_init(|| compile_regex(r"@\w+"))
}

fn hashtags_regex() -> &'static regex::Regex {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    RE.get_or_init(|| compile_regex(r"#(\w+)"))
}

/// Removes @mentions from text.
fn remove_mentions(text: &str) -> String {
    mentions_regex().replace_all(text, "").to_string()
}

/// Removes #hashtags from text.
fn remove_hashtags(text: &str) -> String {
    hashtags_regex().replace_all(text, "$1").to_string()
}

/// Removes emojis from text (comprehensive Unicode ranges).
fn remove_emojis(text: &str) -> String {
    text.chars()
        .filter(|c| {
            let cp = *c as u32;
            !(0x1F600..=0x1F64F).contains(&cp)    // Emoticons
                && !(0x1F300..=0x1F5FF).contains(&cp)  // Misc Symbols + Pictographs
                && !(0x1F680..=0x1F6FF).contains(&cp)  // Transport + Map
                && !(0x1F1E0..=0x1F1FF).contains(&cp)  // Flags (Regional Indicators)
                && !(0x2600..=0x26FF).contains(&cp)     // Misc Symbols
                && !(0x2700..=0x27BF).contains(&cp)     // Dingbats
                && !(0x1F900..=0x1F9FF).contains(&cp)   // Supplemental Symbols + Pictographs
                && !(0x1FA00..=0x1FAFF).contains(&cp)   // Symbols Extended-A
                && !(0x1F3FB..=0x1F3FF).contains(&cp)   // Skin tone modifiers
                && !(0xFE00..=0xFE0F).contains(&cp)     // Variation Selectors
                && cp != 0x200D // ZWJ (Zero Width Joiner)
        })
        .collect()
}

/// Compiled regex for banned AI word detection (word-boundary matching).
///
/// Uses a single alternation regex to match any banned word at word boundaries.
/// Pre-compiled via `OnceLock` to avoid re-compiling on every validation call.
fn banned_words_regex() -> &'static regex::Regex {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    RE.get_or_init(|| {
        let alts: Vec<String> = BANNED_WORDS.iter().map(|w| regex::escape(w)).collect();
        let pattern = format!("\\b(?:{})\\b", alts.join("|"));
        compile_regex(&pattern)
    })
}

/// Checks if text contains banned AI words using word-boundary matching.
///
/// Uses `\b` word boundaries to avoid false positives on common substrings
/// like `tapestry` matching inside longer words like `tapestries`.
fn check_banned_words(text: &str) -> Option<String> {
    let text_lower = text.to_lowercase();
    let re = banned_words_regex();
    re.find(&text_lower).map(|m| m.as_str().to_string())
}

/// Strip markdown code block wrapping (```...```) from LLM output.
///
/// Some LLMs wrap their JSON/text output in markdown code blocks with an optional
/// language tag like ```json or ```. This function removes the outermost code block
/// fences if present, returning the inner content without the fences.
///
/// # Examples
///
/// ```
/// let text = "```json\n{\"key\": \"value\"}\n```";
/// assert_eq!(strip_code_block(text), Some("{\"key\": \"value\"}".to_string()));
/// ```
fn strip_code_block(text: &str) -> Option<String> {
    let trimmed = text.trim();
    // Check for opening ``` fence (with optional language tag)
    if !trimmed.starts_with("```") {
        return None;
    }
    // Find the closing ``` fence
    let closing = trimmed.rfind("```")?;
    if closing == 0 {
        return None; // Only one fence, malformed
    }
    // Extract content between first ``` line and last ```
    let after_opening = &trimmed[3..]; // skip opening ```
                                       // Skip the rest of the opening fence line (language tag like `json`)
    let content_start = after_opening.find('\n').map(|i| i + 1).unwrap_or(0);
    // Safety: ensure closing fence is actually after content start
    let content_begin = 3 + content_start;
    if closing <= content_begin {
        return None;
    }
    let content = &trimmed[content_begin..closing];
    let stripped = content.trim();
    if stripped.is_empty() {
        None
    } else {
        Some(stripped.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_reply_extracts_json_reply() {
        let json_input = r#"[
            {"content": "that video editor sounds like an unnecessarily complicated way to make cat videos"},
            {"content": "so it is mostly just a fancy decompression tool then"}
        ]"#;
        let result = validate_reply(json_input).unwrap();
        assert_eq!(
            result,
            "that video editor sounds like an unnecessarily complicated way to make cat videos"
        );
    }

    #[test]
    fn test_validate_reply_filters_json_symbols() {
        let text = "Hello {world} [test] \"quote\" = equal";
        let result = validate_reply(text).unwrap();
        assert_eq!(result, "Hello world test quote  equal");
    }

    #[test]
    fn test_validate_reply_truncates_long_text() {
        let long_text = "a".repeat(300);
        let result = validate_reply(&long_text).unwrap();
        assert!(result.len() <= 270);
        assert!(result.ends_with("..."));
    }

    #[test]
    fn test_validate_reply_removes_mentions() {
        let text = "Great point @user! I agree with @someone else.";
        let result = validate_reply(text).unwrap();
        assert!(!result.contains("@user"));
        assert!(!result.contains("@someone"));
    }

    #[test]
    fn test_validate_reply_removes_hashtags() {
        let text = "This is #amazing and #awesome!";
        let result = validate_reply(text).unwrap();
        assert!(!result.contains("#"));
        assert!(result.contains("amazing"));
        assert!(result.contains("awesome"));
    }

    #[test]
    fn test_validate_reply_removes_emojis() {
        let text = "Love this! ❤️ 🔥 👍";
        let result = validate_reply(text).unwrap();
        assert!(!result.contains("❤"));
        assert!(!result.contains("🔥"));
        assert!(!result.contains("👍"));
    }

    #[test]
    fn test_check_banned_words_detects_ai_speak() {
        assert!(check_banned_words("This is crucial for the landscape").is_some());
        assert!(check_banned_words("Let me delve into this").is_some());
        assert!(check_banned_words("Normal text without banned words").is_none());
    }

    #[test]
    fn test_truncate_to_word_boundary() {
        let text = "This is a long sentence with many words that needs truncation";
        let result = truncate_to_word_boundary(text, 30);
        assert!(result.len() <= 33); // 30 + "..."
        assert!(result.ends_with("..."));
    }

    #[test]
    fn test_validate_reply_strips_replies_content_prefix() {
        let llm_output = r#"replies:
content: i bet karpathy will change everything,
content: practical workflows are fine until the models actually break them,
content: yeah another list in a sea of information overload,
content: maybe i'm just too old for this,
"#;
        let result = validate_reply(llm_output).unwrap();
        assert_eq!(result, "i bet karpathy will change everything");
        assert!(!result.contains("replies:"));
        assert!(!result.contains("content:"));
    }

    #[test]
    fn test_validate_reply_strips_replies_content_single() {
        let llm_output = "content: this is a single reply";
        let result = validate_reply(llm_output).unwrap();
        assert_eq!(result, "this is a single reply");
    }

    #[test]
    fn test_validate_reply_skips_replies_label_without_content() {
        let llm_output = "replies: ";
        // No content lines — should fall through to existing validation (will fail empty check)
        let result = validate_reply(llm_output);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_reply_strips_markdown_code_block() {
        // LLM output wrapped in ```json code block with indented content: lines
        let llm_output = r#"```json

    content: that the fully async versus collocated sync RL training is just pure genius
  ,

    content: can't wait to see what happens when the agent tries those continual learning papers

```"#;
        let result = validate_reply(llm_output).unwrap();
        assert_eq!(
            result,
            "that the fully async versus collocated sync RL training is just pure genius"
        );
        assert!(!result.contains("```"));
        assert!(!result.contains("content:"));
        assert!(!result.contains("json"));
    }

    #[test]
    fn test_validate_reply_strips_code_block_with_single_content() {
        // Single content: line inside a markdown code block
        let llm_output = r#"```
content: this is a simple reply
```"#;
        let result = validate_reply(llm_output).unwrap();
        assert_eq!(result, "this is a simple reply");
    }

    #[test]
    fn test_validate_reply_strips_code_block_without_content_prefix() {
        // Code block with plain text (no content: prefix)
        let llm_output = r#"```json
this is a plain reply without prefixes
```"#;
        let result = validate_reply(llm_output).unwrap();
        assert_eq!(result, "this is a plain reply without prefixes");
    }

    #[test]
    fn test_validate_reply_handles_indented_content_lines() {
        // content: lines with leading whitespace (common LLM output style)
        let llm_output = "replies:\n    content: first reply with leading spaces,\n    content: second reply here,";
        let result = validate_reply(llm_output).unwrap();
        assert_eq!(result, "first reply with leading spaces");
        assert!(!result.contains("content:"));
    }

    #[test]
    fn test_validate_reply_normal_text_unaffected() {
        // Regular text without prefixes should be unchanged
        let text = "Great point! I totally agree with this take.";
        let result = validate_reply(text).unwrap();
        assert_eq!(result, "Great point! I totally agree with this take.");
    }

    #[test]
    fn test_validate_reply_removes_asterisks() {
        let text = "This is **bold** and *italic* text";
        let result = validate_reply(text).unwrap();
        assert!(!result.contains("**"));
        assert!(!result.contains("*"));
    }

    #[test]
    fn test_validate_reply_empty_after_sanitization() {
        let text = "@user #tag 😀";
        let result = validate_reply(text);
        assert!(result.is_ok());
        let sanitized = result.unwrap();
        assert!(!sanitized.contains("@"));
        assert!(!sanitized.contains("#"));
    }

    #[test]
    fn test_remove_mentions_function() {
        let text = "Hello @user1 and @user2";
        let result = remove_mentions(text);
        assert!(!result.contains("@user1"));
        assert!(!result.contains("@user2"));
        assert!(result.contains("Hello"));
    }

    #[test]
    fn test_remove_hashtags_function() {
        let text = "This is #tech and #coding";
        let result = remove_hashtags(text);
        assert!(!result.contains("#"));
        assert!(result.contains("tech"));
        assert!(result.contains("coding"));
    }

    #[test]
    fn test_remove_emojis_function() {
        let text = "Test 😀 🔥 👍";
        let result = remove_emojis(text);
        assert!(!result.contains("😀"));
        assert!(!result.contains("🔥"));
        assert!(!result.contains("👍"));
        assert!(result.contains("Test"));
    }

    #[test]
    fn test_remove_emojis_supplemental_and_extended() {
        // 🥰 = U+1F970 (Supplemental Symbols range 0x1F900-0x1F9FF)
        // 🥸 = U+1F978 (same range)
        // 🩺 = U+1FA7A (Symbols Extended-A range 0x1FA00-0x1FAFF)
        let text = "Health 🥰 disguise 🥸 stethoscope 🩺";
        let result = remove_emojis(text);
        assert!(!result.contains("🥰"), "🥰 (0x1F970) should be removed");
        assert!(!result.contains("🥸"), "🥸 (0x1F978) should be removed");
        assert!(!result.contains("🩺"), "🩺 (0x1FA7A) should be removed");
        assert!(result.contains("Health"));
        assert!(result.contains("disguise"));
        assert!(result.contains("stethoscope"));
    }

    #[test]
    fn test_remove_emojis_skin_tone_modifiers() {
        // Skin tone modifiers: 🏻 (U+1F3FB) through 🏿 (U+1F3FF)
        let text = "tone 🏻 🏼 🏽 🏾 🏿 done";
        let result = remove_emojis(text);
        assert!(
            !result.contains("🏻"),
            "skin tone 0x1F3FB should be removed"
        );
        assert!(
            !result.contains("🏿"),
            "skin tone 0x1F3FF should be removed"
        );
        assert!(result.contains("tone"));
        assert!(result.contains("done"));
    }

    #[test]
    fn test_strip_code_block_removes_fences() {
        let text = "```json\nhello world\n```";
        assert_eq!(strip_code_block(text), Some("hello world".to_string()));
    }

    #[test]
    fn test_strip_code_block_no_fence() {
        assert_eq!(strip_code_block("hello world"), None);
    }

    #[test]
    fn test_strip_code_block_empty_content() {
        assert_eq!(strip_code_block("```\n```"), None);
    }

    #[test]
    fn test_strip_code_block_no_closing_fence() {
        assert_eq!(strip_code_block("```json\nhello"), None);
    }

    #[test]
    fn test_strip_code_block_with_leading_whitespace() {
        let text = "  ```json\n  hello\n  ```";
        let result = strip_code_block(text);
        assert!(result.is_some());
        let inner = result.unwrap();
        assert_eq!(inner, "hello");
        assert!(!inner.starts_with("  "));
    }

    #[test]
    fn test_strip_code_block_inner_has_whitespace() {
        let text = "```\n    content: hello\n    content: world\n```";
        let result = strip_code_block(text).unwrap();
        assert!(result.contains("content: hello"));
        // Content should be trimmed, so "    content: hello" -> "content: hello"
        assert!(!result.starts_with("    "));
    }

    #[test]
    fn test_strip_code_block_trailing_content_after_closing_fence() {
        // Content after the closing ``` fence should be discarded.
        let text = "```\nhello\n```trailing";
        let result = strip_code_block(text).unwrap();
        assert_eq!(result, "hello");
        assert!(!result.contains("trailing"));
    }

    #[test]
    fn test_strip_code_block_no_newline_after_opening() {
        // Opening fence with no newline (language tag on same line, no content)
        // The function requires a newline after the opening fence line to distinguish
        // the language tag from content. Without it, the content is empty.
        let text = "```json\ncontent\n```";
        let result = strip_code_block(text).unwrap();
        assert_eq!(result, "content");
    }

    #[test]
    fn test_strip_code_block_lone_fence() {
        // Just ``` with no content at all
        assert_eq!(strip_code_block("```"), None);
    }

    #[test]
    fn test_strip_code_block_only_fences_empty_lines() {
        // ``` with only whitespace/newlines between fences
        assert_eq!(strip_code_block("```\n   \n```"), None);
        assert_eq!(strip_code_block("```\n\n```"), None);
    }

    #[test]
    fn test_strip_code_block_multiple_blocks() {
        // Multiple code blocks — only the outermost (first opening to last closing) should be stripped
        let text = "```\nfirst\n```\nsome text\n```\nsecond\n```";
        let result = strip_code_block(text).unwrap();
        // rfind finds the LAST ```, so content spans from first ``` to last ```
        assert!(result.contains("first"));
        assert!(result.contains("some text"));
        assert!(result.contains("second"));
    }

    #[test]
    fn test_strip_code_block_only_backticks() {
        // Multiple backticks with nothing else
        assert_eq!(strip_code_block("````"), None);
        assert_eq!(strip_code_block("```\n```"), None);
        assert_eq!(strip_code_block("```"), None);
    }

    #[test]
    fn test_validate_reply_content_with_empty_value() {
        // content: with no actual text after it
        let input = "content: ";
        let result = validate_reply(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_reply_strips_banned_words_after_processing() {
        // Banned word appears after sanitization stripping
        let input = "This is a {pivotal} moment";
        // After JSON symbol removal: "This is a pivotal moment"
        let result = validate_reply(input);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("pivotal"));
    }

    #[test]
    fn test_validate_reply_code_block_and_json_combined() {
        // LLM wraps JSON array in a code block
        let input = r#"```json
[
  {"content": "nested in code block and json"},
  {"content": "second option"}
]
```"#;
        let result = validate_reply(input).unwrap();
        // The first content field IS "nested in code block and json" — the word "json"
        // legitimately appears in the extracted reply text.
        assert!(result.starts_with("nested in code block"));
        assert!(!result.contains("```"));
        assert!(!result.contains("["));
        assert!(!result.contains("second"));
    }

    #[test]
    fn test_validate_reply_only_whitespace() {
        assert!(validate_reply("   ").is_err());
        assert!(validate_reply("\n\n\n").is_err());
        assert!(validate_reply("").is_err());
    }

    #[test]
    fn test_validate_reply_all_content_lines_empty_after_trim() {
        // Multiple content: lines but all have empty values
        let input = "replies:\ncontent: ,\ncontent: ,";
        let result = validate_reply(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_reply_passes_through_normal_text() {
        // Normal human text should pass through mostly unchanged
        let text = "this is a normal human reply without any ai buzzwords";
        let result = validate_reply(text).unwrap();
        assert_eq!(result, text);
    }

    #[test]
    fn test_truncate_to_word_boundary_short_text() {
        let text = "Short";
        let result = truncate_to_word_boundary(text, 30);
        assert_eq!(result, "Short");
    }

    #[test]
    fn test_truncate_to_word_boundary_no_space() {
        let text = "Verylongwordwithoutspaces";
        let result = truncate_to_word_boundary(text, 10);
        assert!(result.len() <= 13); // 10 + "..."
    }

    mod proptests {
        use super::*;
        use proptest::collection::vec;
        use proptest::prelude::*;

        proptest! {
            /// Output of remove_emojis is never longer than input.
            #[test]
            fn proptest_remove_emojis_output_length(
                chars in vec(any::<char>(), 0..50),
            ) {
                let text: String = chars.into_iter().collect();
                let result = remove_emojis(&text);
                prop_assert!(result.len() <= text.len(),
                    "result.len()={} > text.len()={} for text={:?}",
                    result.len(), text.len(), text);
            }

            /// No emoji codepoints remain in the output of remove_emojis.
            #[test]
            fn proptest_remove_emojis_no_emoji_remains(
                chars in vec(any::<char>(), 0..50),
            ) {
                let text: String = chars.into_iter().collect();
                let result = remove_emojis(&text);
                for c in result.chars() {
                    let cp = c as u32;
                    prop_assert!(
                        !is_emoji_codepoint(cp),
                        "emoji U+{cp:04X} remains in output for input={:?}",
                        text
                    );
                }
            }

            /// Characters that are not emojis are preserved in order.
            #[test]
            fn proptest_remove_emojis_preserves_non_emoji(
                prefix in "[a-zA-Z0-9 ,.!?]{0,10}",
                suffix in "[a-zA-Z0-9 ,.!?]{0,10}",
            ) {
                let text = format!("{prefix}😀🔥👍{suffix}");
                let result = remove_emojis(&text);
                prop_assert!(result.contains(&prefix),
                    "prefix {:?} should be preserved", prefix);
                prop_assert!(result.contains(&suffix),
                    "suffix {:?} should be preserved", suffix);
                prop_assert!(!result.contains("😀"));
                prop_assert!(!result.contains("🔥"));
                prop_assert!(!result.contains("👍"));
            }
        }

        /// Helper: check if a codepoint is in any emoji range.
        fn is_emoji_codepoint(cp: u32) -> bool {
            (0x1F600..=0x1F64F).contains(&cp)    // Emoticons
                || (0x1F300..=0x1F5FF).contains(&cp)  // Misc Symbols + Pictographs
                || (0x1F680..=0x1F6FF).contains(&cp)  // Transport + Map
                || (0x1F1E0..=0x1F1FF).contains(&cp)  // Flags (Regional Indicators)
                || (0x2600..=0x26FF).contains(&cp)     // Misc Symbols
                || (0x2700..=0x27BF).contains(&cp)     // Dingbats
                || (0x1F900..=0x1F9FF).contains(&cp)   // Supplemental Symbols + Pictographs
                || (0x1FA00..=0x1FAFF).contains(&cp)   // Symbols Extended-A
                || (0x1F3FB..=0x1F3FF).contains(&cp)   // Skin tone modifiers
                || (0xFE00..=0xFE0F).contains(&cp)     // Variation Selectors
                || cp == 0x200D // ZWJ (Zero Width Joiner)
        }
    }
}
