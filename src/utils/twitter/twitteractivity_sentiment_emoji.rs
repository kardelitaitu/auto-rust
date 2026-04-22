//! Emoji sentiment analysis utilities.
//! Provides comprehensive emoji sentiment lexicon with 300+ emojis.

use std::collections::HashMap;
use std::sync::OnceLock;

/// Positive emojis with sentiment strength (1.0-3.0).
/// Organized by category for maintainability.
const POSITIVE_EMOJIS: &[(&str, f32)] = &[
    // === Smiling Faces ===
    ("😊", 2.0),
    ("😄", 2.5),
    ("😃", 2.5),
    ("😁", 2.5),
    ("😆", 2.5),
    ("😅", 1.5),
    ("😂", 2.0),
    ("☺️", 2.0),
    ("😊", 2.0),
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
    ("😆", 2.5),
    ("😀", 2.5),
    ("🤣", 2.5),
    ("😹", 2.5),
    // === Hearts ===
    ("❤️", 3.0),
    ("🧡", 2.5),
    ("💛", 2.5),
    ("💚", 2.5),
    ("💙", 2.5),
    ("💜", 2.5),
    ("🖤", 1.0),
    ("🤍", 2.0),
    ("🤎", 2.0),
    ("💔", -3.0),
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
    // === Gestures - Positive ===
    ("👍", 2.0),
    ("👎", -2.0),
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
    ("🙏", 2.0),
    ("👐", 1.5),
    ("👐🏻", 1.5),
    // === Celebration ===
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
    ("🎊", 3.0),
    // === Animals - Positive ===
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
    ("🐌", 0.5),
    ("🐞", 1.0),
    ("🐢", 1.0),
    ("🐙", 1.0),
    ("🦕", 1.5),
    ("🦖", 1.5),
    ("🦄", 2.5),
    ("🐝", 1.0),
    ("🐛", 0.5),
    ("🦗", 0.5),
    // === Food - Positive ===
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
    ("🍻", 2.0),
    ("🥂", 2.5),
    ("🍷", 2.0),
    ("🍸", 2.0),
    ("🍹", 2.0),
    ("🧃", 1.5),
    ("☕", 1.5),
    // === Activities - Positive ===
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
    ("🎯", 2.0),
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
    // === Travel - Positive ===
    ("🚗", 1.5),
    ("🚕", 1.5),
    ("🚙", 1.5),
    ("🚌", 1.0),
    ("🚎", 1.0),
    ("🏎️", 2.0),
    ("🚓", 1.0),
    ("🚑", 0.5),
    ("🚒", 1.0),
    ("🚐", 1.0),
    ("🚚", 1.0),
    ("🚛", 1.0),
    ("🚜", 1.0),
    ("🏍️", 2.0),
    ("🛵", 1.5),
    ("🚲", 1.5),
    ("🛴", 1.5),
    ("⛵", 2.0),
    ("🚤", 2.0),
    ("🛳️", 1.5),
    ("⛴️", 1.5),
    ("✈️", 2.0),
    ("🚁", 1.5),
    ("🚀", 3.0),
    ("🛸", 2.0),
    ("🌍", 2.0),
    ("🌎", 2.0),
    ("🌏", 2.0),
    ("🌋", 1.0),
    ("🏔️", 2.0),
    ("⛰️", 1.5),
    ("🏕️", 2.0),
    ("🏖️", 2.5),
    ("🏜️", 1.0),
    ("🏝️", 2.5),
    // === Objects - Positive ===
    ("💎", 2.5),
    ("💍", 2.5),
    ("👑", 2.0),
    ("💼", 1.5),
    ("🎒", 1.5),
    ("👝", 1.5),
    ("👛", 1.5),
    ("👜", 1.5),
    ("👜", 1.5),
    ("💎", 2.5),
    ("📱", 1.5),
    ("💻", 1.5),
    ("⌨️", 1.0),
    ("🖥️", 1.5),
    ("🖨️", 1.0),
    ("🖱️", 1.0),
    ("💽", 1.0),
    ("💾", 1.0),
    ("💿", 1.0),
    ("📀", 1.0),
    ("🧮", 1.0),
    ("🎥", 2.0),
    ("🎞️", 1.5),
    ("📽️", 1.5),
    ("🎬", 2.0),
    ("📺", 1.5),
    ("📷", 2.0),
    ("📸", 2.0),
    ("📹", 2.0),
    ("📼", 1.5),
    ("🔔", 1.5),
    ("🔕", -0.5),
    ("🎵", 2.0),
    ("🎶", 2.0),
    ("🎙️", 1.5),
    ("🎚️", 1.0),
    ("🎛️", 1.0),
    ("🎼", 2.0),
    ("🎹", 2.0),
    ("🎸", 2.0),
    // === Symbols - Positive ===
    ("✅", 2.5),
    ("✔️", 2.0),
    ("☑️", 2.0),
    ("➕", 1.5),
    ("➖", -0.5),
    ("➗", 0.5),
    ("✖️", -1.0),
    ("💲", 1.5),
    ("💰", 2.5),
    ("💵", 2.5),
    ("💴", 2.5),
    ("💶", 2.5),
    ("💷", 2.5),
    ("🪙", 2.0),
    ("💳", 1.5),
    ("💸", 2.0),
    ("💹", 2.5),
    ("📈", 2.5),
    ("📉", -1.5),
    ("📊", 1.5),
    ("📋", 1.0),
    ("📌", 1.0),
    ("📍", 1.0),
    ("📎", 0.5),
    ("📏", 0.5),
    ("📐", 0.5),
    ("✂️", 0.5),
    ("📃", 0.5),
    ("📄", 0.5),
    ("📂", 0.5),
    ("🔖", 1.0),
    ("🏷️", 1.0),
    ("💡", 2.0),
    ("🔦", 1.0),
    ("🔬", 1.5),
    ("🔭", 1.5),
    ("📡", 1.0),
    ("💉", -1.0),
    ("💊", -0.5),
    ("🩺", 0.5),
];

/// Negative emojis with sentiment strength (-1.0 to -3.0).
const NEGATIVE_EMOJIS: &[(&str, f32)] = &[
    // === Crying/Sad Faces ===
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
    ("😨", -1.5),
    ("😬", -1.0),
    ("😕", -1.0),
    ("😖", -2.0),
    ("😗", -1.0),
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
    // === Angry/Threatening ===
    ("👿", -3.0),
    ("😈", -2.0),
    ("💀", -2.5),
    ("☠️", -3.0),
    ("💩", -3.0),
    ("🤡", -1.0),
    ("👹", -2.5),
    ("👺", -2.5),
    ("👻", -1.0),
    ("👽", -0.5),
    ("🎃", -0.5),
    ("😺", 1.5),
    ("😸", 1.5),
    ("😹", 2.0),
    ("😻", 2.5),
    // === Gestures - Negative ===
    ("👎", -2.0),
    ("👊", -1.5),
    ("🤛", -1.5),
    ("🤜", -1.5),
    ("🖕", -3.0),
    ("✊", -0.5),
    ("🖖", -0.5),
    ("🤞", -0.5),
    ("🤟", -0.5),
    ("👈", -0.5),
    ("👉", -0.5),
    ("👆", -0.5),
    ("👇", -0.5),
    ("☝️", -0.5),
    // === Negative Symbols ===
    ("❌", -2.0),
    ("❎", -2.0),
    ("⛔", -2.0),
    ("🚫", -2.5),
    ("🛑", -1.5),
    ("⚠️", -1.5),
    ("🚸", -1.0),
    ("⛔", -2.0),
    ("🔞", -1.5),
    ("📵", -1.5),
    ("🚭", -1.5),
    ("🚯", -2.0),
    ("🚱", -1.0),
    ("🚳", -1.0),
    ("🚷", -1.5),
    ("💣", -2.5),
    ("🔪", -3.0),
    ("🗡️", -2.0),
    ("⚔️", -1.5),
    ("🛡️", 1.0),
    ("🔨", -0.5),
    ("⛏️", -0.5),
    ("⚒️", -0.5),
    ("🛠️", -0.5),
    ("🔧", -0.5),
    ("🔩", -0.5),
    ("⚙️", -0.5),
    ("🗜️", -0.5),
    ("⚖️", -0.5),
    ("🔗", -0.5),
    ("⛓️", -2.0),
    ("🧰", -0.5),
    ("🧲", -0.5),
    ("⚗️", -0.5),
    ("🧪", -0.5),
    ("🧫", -0.5),
    ("🧬", -0.5),
    ("🔍", -0.5),
    ("🔎", -0.5),
    ("🕯️", -1.0),
    ("📢", -0.5),
    ("📣", -0.5),
    ("📯", -0.5),
    ("🔔", -0.5),
    ("🔕", -1.0),
    // === Weather - Negative ===
    ("⛈️", -2.0),
    ("🌩️", -1.5),
    ("🌨️", -1.0),
    ("❄️", -0.5),
    ("☃️", 1.0),
    ("⛄", 1.0),
    ("🌬️", -1.0),
    ("🌪️", -2.5),
    ("🌫️", -1.0),
    ("🌧️", -1.5),
    ("☔", -1.0),
    ("☁️", -0.5),
    ("🌂", -0.5),
    // === Animals - Negative ===
    ("🐍", -1.5),
    ("🦂", -1.5),
    ("🕷️", -1.5),
    ("🕸️", -1.0),
    ("🦇", -1.0),
    ("🐀", -2.0),
    ("🐁", -2.0),
    ("🦀", -1.0),
    ("🦞", -0.5),
    ("🦐", -0.5),
    ("🦑", -0.5),
    ("🐚", -0.5),
    ("🐌", -0.5),
    ("🦋", 1.0),
    ("🐛", -0.5),
    ("🐜", -0.5),
    ("🐝", -0.5),
    ("🐞", -0.5),
    ("🦗", -0.5),
    ("🕷️", -1.5),
    // === Food - Negative ===
    ("🥬", -0.5),
    ("🥦", -0.5),
    ("🫑", -0.5),
    ("🌶️", -0.5),
    ("🧄", -0.5),
    ("🧅", -0.5),
    ("🍄", -0.5),
    ("🥜", -0.5),
    // === Medical - Negative ===
    ("💉", -2.0),
    ("💊", -1.5),
    ("🩸", -2.5),
    ("🩹", -1.0),
    ("🩺", -0.5),
    ("🦻", -1.5),
    ("🦼", -1.0),
    ("🦽", -1.0),
    ("🦾", -0.5),
    ("🦿", -1.5),
    ("🩰", -0.5),
    ("🩱", -0.5),
    ("🩲", -0.5),
    ("🩳", -0.5),
];

/// Neutral/context-dependent emojis.
const NEUTRAL_EMOJIS: &[(&str, f32)] = &[
    ("🤔", 0.0),
    ("😐", 0.0),
    ("😑", 0.0),
    ("😶", 0.0),
    ("🙄", -0.5),
    ("😏", -0.5),
    ("😒", -0.5),
    ("😔", -0.5),
    ("😕", -0.5),
    ("😷", -0.5),
    ("🤔", 0.0),
    ("😶‍🌫️", 0.0),
    ("😮‍💨", -0.5),
    ("🫠", -0.5),
    ("🫢", 0.0),
    ("🫣", 0.0),
    ("🫡", 0.5),
    ("🫥", -0.5),
    ("🫤", 0.0),
    ("😬", -0.5),
    ("🤥", -1.0),
    ("🫨", 0.0),
    // === Objects - Neutral ===
    ("📱", 0.5),
    ("💻", 0.5),
    ("⌨️", 0.0),
    ("🖥️", 0.5),
    ("🖨️", 0.0),
    ("🖱️", 0.0),
    ("💽", 0.0),
    ("💾", 0.0),
    ("💿", 0.0),
    ("📀", 0.0),
    ("🧮", 0.0),
    ("📞", 0.0),
    ("📟", 0.0),
    ("📠", 0.0),
    ("📺", 0.5),
    ("📻", 0.5),
    ("🎙️", 0.5),
    ("🎚️", 0.0),
    ("🎛️", 0.0),
    ("🧭", 0.5),
    ("⏱️", 0.0),
    ("⏲️", 0.0),
    ("⏰", 0.5),
    ("🕰️", 0.0),
    ("⌛", -0.5),
    ("⏳", -0.5),
    ("📡", 0.5),
    ("🔋", 0.5),
    ("🔌", 0.0),
    ("💻", 0.5),
    ("💡", 1.0),
    ("🔦", 0.5),
    ("🕯️", 0.0),
    ("🧯", 0.5),
    ("🛢️", 0.0),
    ("💸", 1.0),
    ("💵", 1.0),
    ("💴", 1.0),
    ("💶", 1.0),
    ("💷", 1.0),
    ("🪙", 1.0),
    ("💰", 1.5),
    ("💳", 0.5),
    ("💎", 1.5),
    ("⚖️", 0.0),
    ("🧰", 0.0),
    ("🔧", 0.0),
    ("🔨", 0.0),
    ("⚒️", 0.0),
    ("🛠️", 0.0),
    ("⛏️", 0.0),
    ("🔩", 0.0),
    ("⚙️", 0.0),
    ("🧱", 0.0),
    ("⛓️", -0.5),
    ("🧲", 0.0),
    ("🔫", -2.0),
    ("💣", -2.0),
    ("🧨", -1.5),
    ("🪓", -0.5),
];

/// Lazy-initialized emoji sentiment map for efficient lookups.
static EMOJI_SENTIMENT_MAP: OnceLock<HashMap<String, f32>> = OnceLock::new();

/// Initialize the emoji sentiment map if not already done.
fn get_emoji_map() -> &'static HashMap<String, f32> {
    EMOJI_SENTIMENT_MAP.get_or_init(|| {
        let mut map = HashMap::new();

        // Add positive emojis
        for &(emoji, score) in POSITIVE_EMOJIS {
            map.insert(emoji.to_string(), score);
        }

        // Add negative emojis
        for &(emoji, score) in NEGATIVE_EMOJIS {
            map.insert(emoji.to_string(), score);
        }

        // Add neutral emojis
        for &(emoji, score) in NEUTRAL_EMOJIS {
            map.insert(emoji.to_string(), score);
        }

        map
    })
}

/// Get sentiment score for a single emoji.
/// Returns 0.0 if emoji not found in lexicon.
///
/// # Arguments
/// * `emoji` - The emoji character to analyze
pub fn get_emoji_sentiment(emoji: char) -> f32 {
    get_emoji_map()
        .get(&emoji.to_string())
        .copied()
        .unwrap_or(0.0)
}

/// Extract emojis from text and calculate average sentiment score.
/// Returns 0.0 if no emojis found.
///
/// # Arguments
/// * `text` - The text to analyze
///
/// # Returns
/// Average sentiment score of all emojis in text (-3.0 to +3.0)
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

/// Check if text contains any emojis from our lexicon.
///
/// # Arguments
/// * `text` - The text to check
pub fn has_emojis(text: &str) -> bool {
    let emoji_map = get_emoji_map();
    text.chars().any(|ch| {
        if ch.is_ascii() {
            return false;
        }
        emoji_map.get(&ch.to_string()).is_some()
    })
}

/// Count emojis in text.
///
/// # Arguments
/// * `text` - The text to analyze
pub fn count_emojis(text: &str) -> usize {
    let emoji_map = get_emoji_map();
    text.chars()
        .filter(|&ch| {
            if ch.is_ascii() {
                return false;
            }
            emoji_map.get(&ch.to_string()).is_some()
        })
        .count()
}

/// Get detailed emoji analysis with breakdown.
///
/// # Arguments
/// * `text` - The text to analyze
///
/// # Returns
/// Tuple of (average_score, positive_count, negative_count, neutral_count)
pub fn analyze_emoji_detailed(text: &str) -> (f32, u32, u32, u32) {
    let emoji_map = get_emoji_map();
    let mut total_score = 0.0;
    let mut positive_count = 0u32;
    let mut negative_count = 0u32;
    let mut neutral_count = 0u32;

    for ch in text.chars() {
        if ch.is_ascii() {
            continue;
        }

        if let Some(&score) = emoji_map.get(&ch.to_string()) {
            total_score += score;
            if score > 0.5 {
                positive_count += 1;
            } else if score < -0.5 {
                negative_count += 1;
            } else {
                neutral_count += 1;
            }
        }
    }

    let total = positive_count + negative_count + neutral_count;
    let avg = if total > 0 {
        total_score / total as f32
    } else {
        0.0
    };

    (avg, positive_count, negative_count, neutral_count)
}

/// Classify emoji sentiment as Positive, Neutral, or Negative.
///
/// # Arguments
/// * `text` - The text to analyze
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EmojiSentiment {
    Positive,
    Neutral,
    Negative,
}

pub fn classify_emoji_sentiment(text: &str) -> EmojiSentiment {
    let score = analyze_emoji_sentiment(text);
    if score > 0.5 {
        EmojiSentiment::Positive
    } else if score < -0.5 {
        EmojiSentiment::Negative
    } else {
        EmojiSentiment::Neutral
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_positive_emoji_sentiment() {
        assert!(analyze_emoji_sentiment("I love this! 😍❤️🔥") > 1.0);
        assert!(analyze_emoji_sentiment("Great job! 👏🎉") > 1.0);
        assert!(analyze_emoji_sentiment("Perfect! 💯✨") > 2.0);
    }

    #[test]
    fn test_negative_emoji_sentiment() {
        assert!(analyze_emoji_sentiment("This sucks 😢💔😡") < -1.0);
        assert!(analyze_emoji_sentiment("Terrible! 😠👎") < -1.0);
        assert!(analyze_emoji_sentiment("Worst ever 💩🤮") < -2.0);
    }

    #[test]
    fn test_mixed_emoji_sentiment() {
        let score = analyze_emoji_sentiment("Okay 🙂❤️");
        // Mixed positive and neutral should be slightly positive
        assert!(score > 0.0);
    }

    #[test]
    fn test_no_emojis() {
        assert!((analyze_emoji_sentiment("No emojis here") - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_has_emojis() {
        assert!(has_emojis("Hello 😊"));
        assert!(!has_emojis("Hello :)"));
    }

    #[test]
    fn test_count_emojis() {
        // Test with emojis we know are in the lexicon
        let count1 = count_emojis("Hello 😍🎉🔥");
        assert_eq!(count1, 3);

        let count2 = count_emojis("No emojis");
        assert_eq!(count2, 0);

        let count3 = count_emojis("😍😍😍");
        assert_eq!(count3, 3);
    }

    #[test]
    fn test_detailed_analysis() {
        let (avg, pos, neg, neu) = analyze_emoji_detailed("Great! 😍🎉😢");
        assert!(avg > 0.0);
        assert_eq!(pos, 2);
        assert_eq!(neg, 1);
        assert_eq!(neu, 0);
    }

    #[test]
    fn test_classify_sentiment() {
        assert_eq!(
            classify_emoji_sentiment("Love! 😍❤️"),
            EmojiSentiment::Positive
        );
        assert_eq!(
            classify_emoji_sentiment("Hate! 😠💔"),
            EmojiSentiment::Negative
        );
        assert_eq!(classify_emoji_sentiment("Meh 🤔"), EmojiSentiment::Neutral);
    }

    #[test]
    fn test_individual_emoji_scores() {
        assert!(get_emoji_sentiment('😍') > 2.0);
        assert!(get_emoji_sentiment('😢') < -2.0);
        assert!((get_emoji_sentiment('🤔') - 0.0).abs() < f32::EPSILON);
    }
}
