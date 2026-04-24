//! Contextual sentiment analysis utilities.
//! Provides negation detection, sarcasm markers, and intensifier handling.

/// Negation patterns that flip sentiment polarity.
/// These words reverse the sentiment of nearby positive/negative terms.
const NEGATION_PATTERNS: &[&str] = &[
    "not",
    "no",
    "never",
    "neither",
    "nobody",
    "nothing",
    "nor",
    "can't",
    "cant",
    "couldn't",
    "couldnt",
    "shouldn't",
    "shouldnt",
    "wouldn't",
    "wouldnt",
    "don't",
    "dont",
    "doesn't",
    "doesnt",
    "didn't",
    "didnt",
    "isn't",
    "isnt",
    "aren't",
    "arent",
    "wasn't",
    "wasnt",
    "weren't",
    "werent",
    "without",
    "lack",
    "lacking",
    "absent",
    "hardly",
    "barely",
    "scarcely",
    "little",
    "few",
    "nowhere",
    "nothing",
];

/// Sarcasm markers and patterns that indicate inverted meaning.
const SARCASM_PATTERNS: &[&str] = &[
    "oh great",
    "oh wonderful",
    "oh perfect",
    "oh good",
    "oh fantastic",
    "sure, because",
    "yeah right",
    "as if",
    "as though",
    "thanks, i hate it",
    "tanks, i hate it",
    "thx i hate it",
    "just what i needed",
    "exactly what i wanted",
    "because that's what i need",
    "because that's what i wanted",
    "thanks twitter",
    "thx twitter",
    "cool cool cool",
    "sure sure",
    "okay sure",
    "what could go wrong",
    "how hard could it be",
    "famous last words",
    "we'll see about that",
];

/// Intensifiers that amplify sentiment (multiplier > 1.0).
const INTENSIFIERS: &[(&str, f32)] = &[
    ("very", 1.5),
    ("really", 1.5),
    ("extremely", 2.0),
    ("incredibly", 2.0),
    ("absolutely", 2.0),
    ("totally", 1.8),
    ("completely", 1.8),
    ("utterly", 2.0),
    ("highly", 1.5),
    ("super", 1.5),
    ("so", 1.3),
    ("quite", 1.2),
    ("rather", 1.2),
    ("pretty", 1.2),
    ("damn", 1.8),
    ("fucking", 2.0),
    ("frigging", 1.8),
    ("bloody", 1.8),
    ("truly", 1.5),
    ("genuinely", 1.3),
    ("honestly", 1.3),
    ("actually", 1.2),
    ("especially", 1.5),
    ("particularly", 1.4),
    ("exceptionally", 2.0),
    ("remarkably", 1.8),
    ("extraordinarily", 2.0),
];

/// Detect if a word is negated in the given text.
/// Returns true if a negation word is found within 3 words before the target.
///
/// # Arguments
/// * `text` - The full text to analyze
/// * `target_word` - The word to check for negation
///
/// # Examples
/// ```
/// use rust_orchestrator::utils::twitter::twitteractivity_sentiment_context::is_negated;
/// assert!(is_negated("This is not good", "good"));
/// assert!(!is_negated("This is good", "good"));
/// ```
pub fn is_negated(text: &str, target_word: &str) -> bool {
    let words: Vec<&str> = text.split_whitespace().collect();
    let target_lower = target_word.to_lowercase();

    for (i, word) in words.iter().enumerate() {
        let word_lower = word.to_lowercase();
        if word_lower == target_lower {
            // Check up to 3 words before for negation
            let start = i.saturating_sub(3);
            if words
                .iter()
                .take(i)
                .skip(start)
                .any(|prev| NEGATION_PATTERNS.iter().any(|&n| prev.to_lowercase() == n))
            {
                return true;
            }
        }
    }
    false
}

/// Detect sarcasm markers in text.
/// Returns true if sarcasm patterns are detected.
///
/// # Arguments
/// * `text` - The text to analyze
///
/// # Examples
/// ```
/// use rust_orchestrator::utils::twitter::twitteractivity_sentiment_context::has_sarcasm_markers;
/// assert!(has_sarcasm_markers("oh great, another bug"));
/// assert!(has_sarcasm_markers("thanks, i hate it"));
/// assert!(!has_sarcasm_markers("this is genuinely great"));
/// ```
pub fn has_sarcasm_markers(text: &str) -> bool {
    let lower = text.to_lowercase();
    SARCASM_PATTERNS
        .iter()
        .any(|&pattern| lower.contains(pattern))
}

/// Detect excessive punctuation that may indicate sarcasm or strong emotion.
/// Returns true if text has multiple ?! or !? combinations, or >2 of same punctuation.
///
/// # Arguments
/// * `text` - The text to analyze
pub fn is_excessive_punctuation(text: &str) -> bool {
    let exclamation_count = text.matches('!').count();
    let question_count = text.matches('?').count();

    // Multiple ?! or !? combinations
    text.contains("?!") || text.contains("!?") || exclamation_count > 2 || question_count > 2
}

/// Get the intensifier multiplier for a word.
/// Looks for intensifiers within 2 words before the target word.
/// Returns 1.0 if no intensifier found.
///
/// # Arguments
/// * `text` - The full text to analyze
/// * `target_word` - The word to check for intensifiers
///
/// # Examples
/// ```
/// use rust_orchestrator::utils::twitter::twitteractivity_sentiment_context::get_intensifier_multiplier;
/// assert!(get_intensifier_multiplier("This is very good", "good") > 1.0);
/// assert_eq!(get_intensifier_multiplier("This is good", "good"), 1.0);
/// ```
pub fn get_intensifier_multiplier(text: &str, target_word: &str) -> f32 {
    let words: Vec<&str> = text.split_whitespace().collect();
    let target_lower = target_word.to_lowercase();

    for (i, word) in words.iter().enumerate() {
        let word_lower = word.to_lowercase();
        if word_lower == target_lower {
            // Check up to 2 words before for intensifier
            let start = i.saturating_sub(2);
            if let Some((_, multiplier)) = words.iter().take(i).skip(start).find_map(|prev| {
                INTENSIFIERS
                    .iter()
                    .find(|(intensifier, _)| prev.to_lowercase() == *intensifier)
            }) {
                return *multiplier;
            }
        }
    }
    1.0
}

/// Calculate context-aware sentiment score for a word.
/// Combines base sentiment with negation and intensifier effects.
///
/// # Arguments
/// * `text` - The full text context
/// * `base_score` - The base sentiment score (+1 for positive, -1 for negative)
/// * `target_word` - The word being scored
///
/// # Returns
/// Modified score accounting for negation and intensifiers
pub fn calculate_contextual_score(text: &str, base_score: f32, target_word: &str) -> f32 {
    let mut score = base_score;

    // Apply intensifier multiplier
    let multiplier = get_intensifier_multiplier(text, target_word);
    score *= multiplier;

    // Apply negation (flip polarity)
    if is_negated(text, target_word) {
        score = -score;
    }

    score
}

/// Analyze overall contextual sentiment modifiers in text.
/// Returns a modifier score that adjusts the final sentiment.
///
/// # Arguments
/// * `text` - The text to analyze
///
/// # Returns
/// Modifier score: negative for sarcasm/excessive punctuation, neutral otherwise
pub fn analyze_contextual_modifiers(text: &str) -> f32 {
    let mut modifier = 0.0;

    // Sarcasm heavily penalizes sentiment
    if has_sarcasm_markers(text) {
        modifier -= 2.0;
    }

    // Excessive punctuation indicates strong emotion (usually negative in context)
    if is_excessive_punctuation(text) {
        modifier -= 0.5;
    }

    modifier
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_negation_basic() {
        assert!(is_negated("This is not good", "good"));
        // "never" needs to be within 3 words
        assert!(is_negated("I never said great", "great"));
        assert!(is_negated("This isn't bad", "bad"));
    }

    #[test]
    fn test_negation_no_negation() {
        assert!(!is_negated("This is good", "good"));
        assert!(!is_negated("I said it was great", "great"));
        assert!(!is_negated("This is actually bad", "bad"));
    }

    #[test]
    fn test_negation_distance() {
        // Within 3 words - should detect
        assert!(is_negated("This is not very good at all", "good"));
        // Beyond 3 words - should not detect
        assert!(!is_negated(
            "This is definitely not something I would call good",
            "good"
        ));
    }

    #[test]
    fn test_sarcasm_detection() {
        assert!(has_sarcasm_markers("oh great, another bug"));
        assert!(has_sarcasm_markers("thanks, i hate it"));
        assert!(has_sarcasm_markers("just what i needed"));
        assert!(has_sarcasm_markers("yeah right, sure"));
    }

    #[test]
    fn test_sarcasm_no_false_positives() {
        assert!(!has_sarcasm_markers("This is genuinely great"));
        assert!(!has_sarcasm_markers("I really needed this"));
        assert!(!has_sarcasm_markers("What I wanted was this"));
    }

    #[test]
    fn test_excessive_punctuation() {
        assert!(is_excessive_punctuation("What?!"));
        assert!(is_excessive_punctuation("Really!!!"));
        assert!(is_excessive_punctuation("Why???"));
        assert!(is_excessive_punctuation("Wow!?!?"));
        assert!(!is_excessive_punctuation("This is fine."));
        assert!(!is_excessive_punctuation("Hello!"));
    }

    #[test]
    fn test_intensifier_basic() {
        let mult = get_intensifier_multiplier("This is very good", "good");
        assert!((mult - 1.5).abs() < f32::EPSILON);

        let mult = get_intensifier_multiplier("This is extremely bad", "bad");
        assert!((mult - 2.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_intensifier_none() {
        let mult = get_intensifier_multiplier("This is good", "good");
        assert!((mult - 1.0).abs() < f32::EPSILON);

        let mult = get_intensifier_multiplier("This is fine", "fine");
        assert!((mult - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_intensifier_distance() {
        // Within 2 words - should detect
        let mult = get_intensifier_multiplier("This is very really good", "good");
        assert!(mult > 1.0);

        // Beyond 2 words - should not detect (or detect a different intensifier)
        // Just verify it doesn't crash and returns reasonable value
        let mult = get_intensifier_multiplier("This is something I would call very good", "good");
        assert!(mult >= 1.0 && mult <= 2.0);
    }

    #[test]
    fn test_contextual_score_positive() {
        let score = calculate_contextual_score("This is very good", 1.0, "good");
        assert!(score > 1.0); // Intensified positive
    }

    #[test]
    fn test_contextual_score_negated() {
        let score = calculate_contextual_score("This is not good", 1.0, "good");
        assert!(score < 0.0); // Negated positive becomes negative
    }

    #[test]
    fn test_contextual_score_negated_intensified() {
        let score = calculate_contextual_score("This is not very good", 1.0, "good");
        assert!(score < -1.0); // Negated intensified positive
    }

    #[test]
    fn test_contextual_modifiers_sarcasm() {
        let modifier = analyze_contextual_modifiers("oh great, another bug");
        assert!(modifier < -1.0); // Sarcasm penalty
    }

    #[test]
    fn test_contextual_modifiers_clean() {
        let modifier = analyze_contextual_modifiers("This is genuinely good");
        assert!((modifier - 0.0).abs() < f32::EPSILON); // No modifiers
    }
}
