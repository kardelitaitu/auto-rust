//! Emoji sentiment analysis strategy.

use std::collections::HashMap;
use std::sync::OnceLock;

/// Positive emojis with sentiment strength (1.0-3.0).
const POSITIVE_EMOJIS: &[(&str, f32)] = &[
    ("😊", 2.0),
    ("😄", 2.5),
    ("😃", 2.5),
    ("😁", 2.5),
    ("😆", 2.5),
    ("😅", 1.5),
    ("😂", 2.0),
    ("☺️", 2.0),
    ("😍", 3.0),
    ("🥰", 3.0),
    ("😘", 2.5),
    ("😗", 2.0),
    ("😙", 2.0),
    ("😚", 2.0),
    ("🙂", 1.5),
    ("🤗", 2.5),
    ("🤩", 3.0),
    ("😌", 1.5),
    ("😛", 1.5),
    ("😜", 1.5),
    ("😝", 1.5),
    ("😋", 1.5),
    ("😎", 2.0),
    ("🤓", 1.0),
    ("🥳", 3.0),
    ("🤠", 2.0),
    ("😇", 2.0),
    ("🤫", 1.0),
    ("🤭", 1.5),
    ("😏", 1.0),
    ("😀", 2.5),
    ("🤣", 2.5),
    ("😹", 2.5),
    ("❤️", 3.0),
    ("🧡", 2.5),
    ("💛", 2.5),
    ("💚", 2.5),
    ("💙", 2.5),
    ("💜", 2.5),
    ("🖤", 1.0),
    ("🤍", 2.0),
    ("🤎", 2.0),
    ("❣️", 2.5),
    ("💕", 3.0),
    ("💖", 3.0),
    ("💗", 3.0),
    ("💘", 3.0),
    ("💝", 3.0),
    ("💞", 3.0),
    ("💟", 2.5),
    ("💓", 3.0),
    ("💌", 2.0),
    ("👍", 2.0),
    ("👏", 2.5),
    ("🙌", 3.0),
    ("👐", 1.5),
    ("🤲", 1.5),
    ("🤝", 2.0),
    ("🙏", 2.0),
    ("✌️", 1.5),
    ("🤟", 2.5),
    ("🤘", 1.5),
    ("👌", 2.0),
    ("🤌", 1.0),
    ("🤏", 0.5),
    ("👋", 1.0),
    ("🤙", 1.5),
    ("💪", 2.0),
    ("🎉", 3.0),
    ("🎊", 3.0),
    ("🎈", 2.5),
    ("🎁", 2.5),
    ("🎀", 2.0),
    ("🏆", 3.0),
    ("🥇", 3.0),
    ("🥈", 2.5),
    ("🥉", 2.0),
    ("🏅", 3.0),
    ("🎯", 2.0),
    ("🔥", 2.5),
    ("💯", 3.0),
    ("✨", 2.5),
    ("⭐", 2.5),
    ("🌟", 3.0),
    ("💫", 2.0),
    ("🌈", 2.5),
    ("☀️", 2.0),
    ("🌞", 2.5),
    ("🌻", 2.5),
    ("🌸", 2.0),
    ("🌺", 2.0),
    ("🌹", 2.5),
    ("🌷", 2.0),
    ("💐", 2.5),
    ("🍾", 2.5),
    ("🥂", 2.5),
    ("🍻", 2.0),
    ("🐶", 1.5),
    ("🐱", 1.5),
    ("🐰", 1.5),
    ("🦊", 1.5),
    ("🐻", 1.5),
    ("🐼", 2.0),
    ("🐨", 1.5),
    ("🐯", 1.5),
    ("🦁", 2.0),
    ("🐮", 1.0),
    ("🐷", 1.5),
    ("🐸", 1.5),
    ("🐵", 1.5),
    ("🐔", 1.0),
    ("🐧", 1.5),
    ("🐦", 1.5),
    ("🦆", 1.0),
    ("🦅", 1.5),
    ("🦉", 1.5),
    ("🦋", 2.0),
    ("🐞", 1.0),
    ("🐢", 1.0),
    ("🐙", 1.0),
    ("🦕", 1.5),
    ("🦖", 1.5),
    ("🦄", 2.5),
    ("🐝", 1.0),
    ("🍕", 2.0),
    ("🍔", 2.0),
    ("🍟", 2.0),
    ("🌭", 1.5),
    ("🍿", 2.0),
    ("🍫", 2.0),
    ("🍬", 2.0),
    ("🍭", 2.0),
    ("🍮", 2.0),
    ("🍯", 2.0),
    ("🍰", 2.5),
    ("🎂", 3.0),
    ("🧁", 2.5),
    ("🥧", 2.0),
    ("🍦", 2.0),
    ("🍩", 2.0),
    ("🍪", 2.0),
    ("🍺", 2.0),
    ("🍷", 2.0),
    ("🍸", 2.0),
    ("🍹", 2.0),
    ("🧃", 1.5),
    ("☕", 1.5),
    ("⚽", 1.5),
    ("🏀", 1.5),
    ("🏈", 1.5),
    ("⚾", 1.5),
    ("🎾", 1.5),
    ("🏐", 1.5),
    ("🏉", 1.5),
    ("🎱", 1.0),
    ("🏓", 1.5),
    ("🏸", 1.5),
    ("🏒", 1.5),
    ("🏑", 1.5),
    ("🥍", 1.5),
    ("🏏", 1.5),
    ("🎣", 1.5),
    ("🎮", 2.0),
    ("🎲", 1.5),
    ("🧩", 1.5),
    ("♟️", 1.5),
    ("🎨", 2.0),
    ("🎭", 1.5),
    ("🎪", 2.0),
    ("🎬", 2.0),
    ("🎵", 2.0),
    ("🎶", 2.0),
    ("🎸", 2.0),
    ("🎹", 2.0),
    ("🎺", 1.5),
    ("🎻", 2.0),
    ("🏎️", 2.0),
    ("🏍️", 2.0),
    ("🚀", 3.0),
    ("💎", 2.5),
    ("💍", 2.5),
    ("👑", 2.0),
    ("✅", 2.5),
    ("✔️", 2.0),
    ("☑️", 2.0),
    ("💰", 2.5),
    ("💵", 2.5),
    ("📈", 2.5),
    ("💡", 2.0),
];

/// Negative emojis with sentiment strength (-1.0 to -3.0).
const NEGATIVE_EMOJIS: &[(&str, f32)] = &[
    ("😢", -2.5),
    ("😭", -3.0),
    ("😞", -2.0),
    ("😟", -1.5),
    ("😠", -2.5),
    ("😡", -3.0),
    ("🤬", -3.0),
    ("😤", -2.0),
    ("😩", -2.5),
    ("😫", -2.5),
    ("😨", -2.0),
    ("😰", -2.0),
    ("😱", -2.5),
    ("😳", -1.0),
    ("🥺", -1.0),
    ("😦", -1.5),
    ("😧", -1.5),
    ("😬", -1.0),
    ("😕", -1.0),
    ("😖", -2.0),
    ("😣", -2.0),
    ("😥", -2.0),
    ("😮", -1.0),
    ("🤐", -1.0),
    ("😯", -1.0),
    ("😪", -1.5),
    ("😴", -0.5),
    ("😵", -2.0),
    ("🤒", -1.5),
    ("🤕", -1.5),
    ("🤢", -3.0),
    ("🤮", -3.0),
    ("🤧", -1.5),
    ("😷", -1.0),
    ("😶", -0.5),
    ("😐", -0.5),
    ("😑", -0.5),
    ("😒", -1.5),
    ("🙄", -1.5),
    ("😏", -0.5),
    ("😔", -2.0),
    ("😓", -1.5),
    ("😿", -2.5),
    ("👿", -3.0),
    ("😈", -2.0),
    ("💀", -2.5),
    ("☠️", -3.0),
    ("💩", -3.0),
    ("🤡", -1.0),
    ("👹", -2.5),
    ("👺", -2.5),
    ("👻", -1.0),
    ("👎", -2.0),
    ("🖕", -3.0),
    ("❌", -2.0),
    ("⛔", -2.0),
    ("🚫", -2.5),
    ("🛑", -1.5),
    ("⚠️", -1.5),
    ("🚮", -2.0),
    ("🆘", -1.5),
    ("🚯", -2.0),
    ("🚱", -1.0),
    ("🚷", -1.5),
    ("💣", -2.5),
    ("🔪", -3.0),
    ("🗡️", -2.0),
    ("⛓️", -2.0),
    ("🌪️", -2.5),
    ("🌧️", -1.5),
    ("🐀", -2.0),
    ("🐁", -2.0),
    ("🐍", -1.5),
    ("🦂", -1.5),
    ("🕷️", -1.5),
    ("💉", -2.0),
    ("💊", -1.5),
    ("🩸", -2.5),
    ("💔", -3.0),
];

/// Neutral/context-dependent emojis.
const NEUTRAL_EMOJIS: &[(&str, f32)] = &[
    ("🤔", 0.0),
    ("😮‍💨", -0.5),
    ("🫠", -0.5),
    ("🫥", -0.5),
    ("🤥", -1.0),
    ("🔫", -2.0),
    ("🧨", -1.5),
];

static EMOJI_SENTIMENT_MAP: OnceLock<HashMap<String, f32>> = OnceLock::new();

fn get_emoji_map() -> &'static HashMap<String, f32> {
    EMOJI_SENTIMENT_MAP.get_or_init(|| {
        let mut map = HashMap::new();
        for &(emoji, score) in POSITIVE_EMOJIS {
            map.insert(emoji.to_string(), score);
        }
        for &(emoji, score) in NEGATIVE_EMOJIS {
            map.insert(emoji.to_string(), score);
        }
        for &(emoji, score) in NEUTRAL_EMOJIS {
            map.insert(emoji.to_string(), score);
        }
        map
    })
}

/// Analyze emojis in text and calculate average sentiment score.
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn analyze_emoji_sentiment(text: &str) -> f32 {
    let emoji_map = get_emoji_map();
    let mut total_score = 0.0;
    let mut emoji_count = 0;

    for ch in text.chars() {
        if ch.is_ascii() {
            continue;
        }
        if let Some(&score) = emoji_map.get(&ch.to_string()) {
            total_score += score;
            emoji_count += 1;
        }
    }

    if emoji_count > 0 {
        total_score / emoji_count as f32
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_emoji_returns_zero() {
        let score = analyze_emoji_sentiment("this is a plain text tweet with no emoji");
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_empty_string_returns_zero() {
        let score = analyze_emoji_sentiment("");
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_single_positive_emoji() {
        let score = analyze_emoji_sentiment("I love this 😊");
        assert_eq!(score, 2.0);
    }

    #[test]
    fn test_single_strong_positive_emoji() {
        let score = analyze_emoji_sentiment("This is amazing 😍");
        assert_eq!(score, 3.0);
    }

    #[test]
    fn test_single_negative_emoji() {
        let score = analyze_emoji_sentiment("This is so sad 😢");
        assert_eq!(score, -2.5);
    }

    #[test]
    fn test_single_strong_negative_emoji() {
        let score = analyze_emoji_sentiment("I'm so angry 😡");
        assert_eq!(score, -3.0);
    }

    #[test]
    fn test_multiple_same_emoji() {
        let score = analyze_emoji_sentiment("😂😂😂");
        // Three laughing emojis: each scores 2.0, average = 2.0
        assert!((score - 2.0).abs() < 0.1, "expected ~2.0, got {}", score);
    }

    #[test]
    fn test_mixed_positive_and_negative() {
        let score = analyze_emoji_sentiment("Happy 😊 but also sad 😢");
        // (2.0 + -2.5) / 2 = -0.25
        assert!((score - (-0.25)).abs() < 0.001);
    }

    #[test]
    fn test_fire_emoji() {
        let score = analyze_emoji_sentiment("This is 🔥");
        assert_eq!(score, 2.5);
    }

    #[test]
    fn test_thumbs_down_emoji() {
        let score = analyze_emoji_sentiment("This is bad 👎");
        assert_eq!(score, -2.0);
    }

    #[test]
    fn test_poop_emoji() {
        let score = analyze_emoji_sentiment("This is 💩");
        assert_eq!(score, -3.0);
    }

    #[test]
    fn test_rocket_emoji() {
        let score = analyze_emoji_sentiment("To the moon 🚀");
        assert_eq!(score, 3.0);
    }

    #[test]
    fn test_unicode_text_no_emoji() {
        let score = analyze_emoji_sentiment("こんにちは世界");
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_mixed_emoji_with_ascii() {
        let score = analyze_emoji_sentiment("🎉🎊🎈 celebration");
        // (3.0 + 3.0 + 2.5) / 3 ≈ 2.833
        assert!((score - 2.833).abs() < 0.01);
    }

    #[test]
    fn test_thinking_emoji_neutral() {
        let score = analyze_emoji_sentiment("🤔 Interesting point");
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_all_positive_emojis_average() {
        let score = analyze_emoji_sentiment("😄😁😂");
        // (2.5 + 2.5 + 2.0) / 3 ≈ 2.333
        assert!((score - 2.333).abs() < 0.01);
    }

    #[test]
    fn test_emoji_in_long_text() {
        let score = analyze_emoji_sentiment(
            "This is a very long tweet with multiple sentences and it has a 🎉 emoji in it somewhere"
        );
        assert_eq!(score, 3.0);
    }

    #[test]
    fn test_only_ascii_returns_zero() {
        let score = analyze_emoji_sentiment("abcdefghijklmnopqrstuvwxyz1234567890!@#$%^&*()");
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_emoji_map_is_initialized() {
        let map = get_emoji_map();
        assert!(!map.is_empty());
        assert!(map.contains_key("😊"));
        assert!(map.contains_key("😢"));
        assert!(map.contains_key("🤔"));
    }

    #[test]
    fn test_emoji_lists_have_entries() {
        assert!(
            !POSITIVE_EMOJIS.is_empty(),
            "POSITIVE_EMOJIS should not be empty"
        );
        assert!(
            !NEGATIVE_EMOJIS.is_empty(),
            "NEGATIVE_EMOJIS should not be empty"
        );
        assert!(
            !NEUTRAL_EMOJIS.is_empty(),
            "NEUTRAL_EMOJIS should not be empty"
        );
    }

    #[test]
    fn test_skull_emoji() {
        let score = analyze_emoji_sentiment("That's scary 💀");
        assert_eq!(score, -2.5);
    }

    #[test]
    fn test_one_hundred_emoji() {
        let score = analyze_emoji_sentiment("Perfect score 💯");
        assert_eq!(score, 3.0);
    }

    #[test]
    fn test_whitespace_only() {
        let score = analyze_emoji_sentiment("   \n  \t  ");
        assert_eq!(score, 0.0);
    }
}
