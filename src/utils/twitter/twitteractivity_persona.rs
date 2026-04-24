//! Persona and behavior weight management for Twitter automation.
//! Selects and applies behavior profiles to tune task decisions.

use crate::utils::profile::{BrowserProfile, ProfilePreset};
use rand::Rng;
use serde_json::{json, Value};
use tracing::instrument;

/// Persona weights that influence task decision-making.
/// These are multiplied against base probabilities to produce final actions.
#[derive(Debug, Clone)]
pub struct PersonaWeights {
    /// Likelihood to like a tweet (0.0–1.0)
    pub like_prob: f64,
    /// Likelihood to retweet (0.0–1.0)
    pub retweet_prob: f64,
    /// Likelihood to quote tweet with commentary (0.0–1.0)
    pub quote_prob: f64,
    /// Likelihood to follow a user from a tweet (0.0–1.0)
    pub follow_prob: f64,
    /// Likelihood to reply to a tweet (0.0–1.0)
    pub reply_prob: f64,
    /// Likelihood to bookmark a tweet (0.0–1.0)
    pub bookmark_prob: f64,
    /// Likelihood to dive into a thread (0.0–1.0)
    pub thread_dive_prob: f64,
    /// Base interest multiplier (modulates sentiment response)
    pub interest_multiplier: f64,
}

impl Default for PersonaWeights {
    fn default() -> Self {
        Self {
            like_prob: 0.3,
            retweet_prob: 0.1,
            quote_prob: 0.05,
            follow_prob: 0.05,
            reply_prob: 0.02,
            bookmark_prob: 0.0,
            thread_dive_prob: 0.2,
            interest_multiplier: 1.0,
        }
    }
}

impl PersonaWeights {
    /// Modifies weights based on detected sentiment in the current feed.
    /// Positive sentiment slightly increases engagement probabilities,
    /// while negative sentiment suppresses engagement.
    pub fn with_sentiment_modulation(mut self, sentiment_score: f64) -> Self {
        // sentiment_score is in [-1.0, +1.0]
        // Scale and apply to interest_multiplier
        let boost = (sentiment_score * 0.5) + 0.5; // normalize to [0, 1]
        self.interest_multiplier = 0.5 + (boost * 0.5); // [0.5, 1.0]
        self
    }

    /// Applies profile-based variance — randomizes weights within ±profile_variance%.
    pub fn with_profile_variance(mut self, profile: &BrowserProfile) -> Self {
        let variance = profile.action_delay_variance_pct.base / 100.0; // e.g., 0.5 = ±50%
        let mut rng = rand::thread_rng();

        macro_rules! perturb {
            ($field:expr) => {{
                let jitter = rng.gen_range(-variance..=variance);
                let new_val = $field * (1.0 + jitter);
                new_val.clamp(0.0, 1.0)
            }};
        }

        self.like_prob = perturb!(self.like_prob);
        self.retweet_prob = perturb!(self.retweet_prob);
        self.quote_prob = perturb!(self.quote_prob);
        self.follow_prob = perturb!(self.follow_prob);
        self.reply_prob = perturb!(self.reply_prob);
        self.bookmark_prob = perturb!(self.bookmark_prob);
        self.thread_dive_prob = perturb!(self.thread_dive_prob);

        self
    }

    /// Clamps all probabilities to ensure they are within \[0,1\].
    pub fn normalized(mut self) -> Self {
        macro_rules! clamp {
            ($field:expr) => {
                $field.clamp(0.0, 1.0)
            };
        }
        self.like_prob = clamp!(self.like_prob);
        self.retweet_prob = clamp!(self.retweet_prob);
        self.quote_prob = clamp!(self.quote_prob);
        self.follow_prob = clamp!(self.follow_prob);
        self.reply_prob = clamp!(self.reply_prob);
        self.bookmark_prob = clamp!(self.bookmark_prob);
        self.thread_dive_prob = clamp!(self.thread_dive_prob);
        self
    }
}

fn effective_probability(base_probability: f64, persona: &PersonaWeights) -> f64 {
    (base_probability * persona.interest_multiplier).clamp(0.0, 1.0)
}

/// Selects a PersonaWeights configuration based on the provided weights dictionary.
/// The `weights` JSON may include any of: `like_prob`, `retweet_prob`, `quote_prob`, `follow_prob`, `reply_prob`, `thread_dive_prob`, `interest_multiplier`.
/// Any missing weights default to the provided config probabilities.
#[instrument]
pub fn select_persona_weights(
    weights: Option<&Value>,
    config_probs: &crate::config::TwitterProbabilitiesConfig,
) -> PersonaWeights {
    let mut persona = PersonaWeights {
        like_prob: config_probs.like_probability,
        retweet_prob: config_probs.retweet_probability,
        quote_prob: config_probs.quote_probability,
        follow_prob: config_probs.follow_probability,
        reply_prob: config_probs.reply_probability,
        bookmark_prob: config_probs.bookmark_probability,
        thread_dive_prob: config_probs.thread_dive_probability,
        interest_multiplier: 1.0,
    };

    log::info!("Persona probabilities from config: like={:.3}, retweet={:.3}, quote={:.3}, follow={:.3}, reply={:.3}, bookmark={:.3}, dive={:.3}",
        persona.like_prob, persona.retweet_prob, persona.quote_prob, persona.follow_prob, persona.reply_prob, persona.bookmark_prob, persona.thread_dive_prob);

    if let Some(w) = weights {
        let mut overrides = Vec::new();
        if let Some(v) = w.get("like_prob").and_then(|v: &Value| v.as_f64()) {
            persona.like_prob = v;
            overrides.push(format!("like={:.3}", v));
        }
        if let Some(v) = w.get("retweet_prob").and_then(|v: &Value| v.as_f64()) {
            persona.retweet_prob = v;
            overrides.push(format!("retweet={:.3}", v));
        }
        if let Some(v) = w.get("quote_prob").and_then(|v: &Value| v.as_f64()) {
            persona.quote_prob = v;
            overrides.push(format!("quote={:.3}", v));
        }
        if let Some(v) = w.get("follow_prob").and_then(|v: &Value| v.as_f64()) {
            persona.follow_prob = v;
            overrides.push(format!("follow={:.3}", v));
        }
        if let Some(v) = w.get("reply_prob").and_then(|v: &Value| v.as_f64()) {
            persona.reply_prob = v;
            overrides.push(format!("reply={:.3}", v));
        }
        if let Some(v) = w.get("bookmark_prob").and_then(|v: &Value| v.as_f64()) {
            persona.bookmark_prob = v;
            overrides.push(format!("bookmark={:.3}", v));
        }
        if let Some(v) = w.get("thread_dive_prob").and_then(|v: &Value| v.as_f64()) {
            persona.thread_dive_prob = v;
            overrides.push(format!("dive={:.3}", v));
        }
        if let Some(v) = w
            .get("interest_multiplier")
            .and_then(|v: &Value| v.as_f64())
        {
            persona.interest_multiplier = v;
            overrides.push(format!("interest_multiplier={:.3}", v));
        }
        if !overrides.is_empty() {
            log::info!("Persona overrides from payload: {}", overrides.join(", "));
        }
    }

    let final_persona = persona.normalized();
    log::info!("Final persona probabilities: like={:.3}, retweet={:.3}, quote={:.3}, follow={:.3}, reply={:.3}, bookmark={:.3}, dive={:.3}",
        final_persona.like_prob, final_persona.retweet_prob, final_persona.quote_prob, final_persona.follow_prob, final_persona.reply_prob, final_persona.bookmark_prob, final_persona.thread_dive_prob);
    final_persona
}

/// Applies the behavior profile's sentiment modulation and variance to the base persona.
/// This is the integrated machine: weights ← profile characteristics + feed sentiment.
#[instrument]
pub fn apply_behavior_profile(
    persona: PersonaWeights,
    profile: &BrowserProfile,
    sentiment_score: f64,
) -> PersonaWeights {
    persona
        .with_sentiment_modulation(sentiment_score)
        .with_profile_variance(profile)
        .normalized()
}

/// Decides whether to like a tweet given the persona weights.
/// Returns `true` if the randomized chance was met.
pub fn should_like(persona: &PersonaWeights) -> bool {
    let prob = effective_probability(persona.like_prob, persona);
    let mut rng = rand::thread_rng();
    let result = rng.gen_bool(prob);
    log::debug!("should_like: prob={:.3}, result={}", prob, result);
    result
}

/// Decides whether to retweet.
pub fn should_retweet(persona: &PersonaWeights) -> bool {
    let mut rng = rand::thread_rng();
    rng.gen_bool(effective_probability(persona.retweet_prob, persona))
}

/// Decides whether to quote tweet.
pub fn should_quote(persona: &PersonaWeights) -> bool {
    let mut rng = rand::thread_rng();
    rng.gen_bool(effective_probability(persona.quote_prob, persona))
}

/// Decides whether to follow the author.
pub fn should_follow(persona: &PersonaWeights) -> bool {
    let mut rng = rand::thread_rng();
    rng.gen_bool(effective_probability(persona.follow_prob, persona))
}

/// Decides whether to reply.
pub fn should_reply(persona: &PersonaWeights) -> bool {
    let mut rng = rand::thread_rng();
    rng.gen_bool(effective_probability(persona.reply_prob, persona))
}

/// Decides whether to bookmark a tweet.
pub fn should_bookmark(persona: &PersonaWeights) -> bool {
    let mut rng = rand::thread_rng();
    rng.gen_bool(effective_probability(persona.bookmark_prob, persona))
}

/// Decides whether to dive into the thread.
pub fn should_dive(persona: &PersonaWeights) -> bool {
    let mut rng = rand::thread_rng();
    rng.gen_bool(effective_probability(persona.thread_dive_prob, persona))
}

/// Builds a persona payload for task configuration.
/// Returns a JSON-compatible Value that can be passed to the task.
pub fn build_persona_config(
    weights: Option<PersonaWeights>,
    profile_preset: Option<ProfilePreset>,
) -> Value {
    let weights = weights.unwrap_or_default();
    let profile = profile_preset.unwrap_or(ProfilePreset::Average);

    json!({
        "weights": {
            "like_prob": weights.like_prob,
            "retweet_prob": weights.retweet_prob,
            "quote_prob": weights.quote_prob,
            "follow_prob": weights.follow_prob,
            "reply_prob": weights.reply_prob,
            "bookmark_prob": weights.bookmark_prob,
            "thread_dive_prob": weights.thread_dive_prob,
            "interest_multiplier": weights.interest_multiplier,
        },
        "profile": profile,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_persona_weights_default() {
        let weights = PersonaWeights::default();
        assert_eq!(weights.like_prob, 0.3);
        assert_eq!(weights.retweet_prob, 0.1);
        assert_eq!(weights.quote_prob, 0.05);
        assert_eq!(weights.follow_prob, 0.05);
        assert_eq!(weights.reply_prob, 0.02);
        assert_eq!(weights.bookmark_prob, 0.0);
        assert_eq!(weights.thread_dive_prob, 0.2);
        assert_eq!(weights.interest_multiplier, 1.0);
    }

    #[test]
    fn test_persona_weights_with_sentiment_modulation_positive() {
        let weights = PersonaWeights::default();
        let modulated = weights.with_sentiment_modulation(1.0);
        // Positive sentiment should increase interest multiplier
        assert!(modulated.interest_multiplier >= 0.5);
        assert!(modulated.interest_multiplier <= 1.0);
    }

    #[test]
    fn test_persona_weights_with_sentiment_modulation_negative() {
        let weights = PersonaWeights::default();
        let modulated = weights.with_sentiment_modulation(-1.0);
        // Negative sentiment should decrease interest multiplier
        assert!(modulated.interest_multiplier >= 0.5);
        assert!(modulated.interest_multiplier <= 1.0);
    }

    #[test]
    fn test_persona_weights_with_sentiment_modulation_neutral() {
        let weights = PersonaWeights::default();
        let modulated = weights.with_sentiment_modulation(0.0);
        // Neutral sentiment should give middle value
        assert!(modulated.interest_multiplier >= 0.5);
        assert!(modulated.interest_multiplier <= 1.0);
    }

    #[test]
    fn test_persona_weights_normalized() {
        let weights = PersonaWeights {
            like_prob: 1.5,
            retweet_prob: -0.5,
            quote_prob: 0.5,
            follow_prob: 0.3,
            reply_prob: 2.0,
            bookmark_prob: -1.0,
            thread_dive_prob: 0.8,
            interest_multiplier: 1.0,
        };
        let normalized = weights.normalized();
        assert_eq!(normalized.like_prob, 1.0);
        assert_eq!(normalized.retweet_prob, 0.0);
        assert_eq!(normalized.quote_prob, 0.5);
        assert_eq!(normalized.follow_prob, 0.3);
        assert_eq!(normalized.reply_prob, 1.0);
        assert_eq!(normalized.bookmark_prob, 0.0);
        assert_eq!(normalized.thread_dive_prob, 0.8);
    }

    #[test]
    fn test_select_persona_weights_none() {
        let config_probs = crate::config::TwitterProbabilitiesConfig::default();
        let persona = select_persona_weights(None, &config_probs);
        assert!(persona.like_prob >= 0.0 && persona.like_prob <= 1.0);
        assert!(persona.retweet_prob >= 0.0 && persona.retweet_prob <= 1.0);
    }

    #[test]
    fn test_select_persona_weights_with_overrides() {
        let config_probs = crate::config::TwitterProbabilitiesConfig::default();
        let weights = json!({
            "like_prob": 0.8,
            "retweet_prob": 0.4,
            "follow_prob": 0.2
        });
        let persona = select_persona_weights(Some(&weights), &config_probs);
        assert_eq!(persona.like_prob, 0.8);
        assert_eq!(persona.retweet_prob, 0.4);
        assert_eq!(persona.follow_prob, 0.2);
    }

    #[test]
    fn test_select_persona_weights_partial_overrides() {
        let config_probs = crate::config::TwitterProbabilitiesConfig::default();
        let weights = json!({
            "like_prob": 0.7
        });
        let persona = select_persona_weights(Some(&weights), &config_probs);
        assert_eq!(persona.like_prob, 0.7);
        // Other values should come from config
        assert!(persona.retweet_prob >= 0.0);
    }

    #[test]
    fn test_should_like_probability_bounds() {
        let weights = PersonaWeights {
            like_prob: 0.0,
            ..Default::default()
        };
        // With 0 probability, should always be false (statistically)
        let false_count = (0..100).filter(|_| should_like(&weights)).count();
        assert_eq!(false_count, 0);

        let weights = PersonaWeights {
            like_prob: 1.0,
            ..Default::default()
        };
        // With 1.0 probability, should always be true (statistically)
        let true_count = (0..100).filter(|_| should_like(&weights)).count();
        assert_eq!(true_count, 100);
    }

    #[test]
    fn test_interest_multiplier_suppresses_decisions() {
        let weights = PersonaWeights {
            like_prob: 1.0,
            retweet_prob: 1.0,
            quote_prob: 1.0,
            follow_prob: 1.0,
            reply_prob: 1.0,
            bookmark_prob: 1.0,
            thread_dive_prob: 1.0,
            interest_multiplier: 0.0,
        };

        assert!(!should_like(&weights));
        assert!(!should_retweet(&weights));
        assert!(!should_quote(&weights));
        assert!(!should_follow(&weights));
        assert!(!should_reply(&weights));
        assert!(!should_bookmark(&weights));
        assert!(!should_dive(&weights));
    }

    #[test]
    fn test_interest_multiplier_boosts_and_clamps_decisions() {
        let weights = PersonaWeights {
            like_prob: 0.6,
            retweet_prob: 0.6,
            quote_prob: 0.6,
            follow_prob: 0.6,
            reply_prob: 0.6,
            bookmark_prob: 0.6,
            thread_dive_prob: 0.6,
            interest_multiplier: 2.0,
        };

        assert!(should_like(&weights));
        assert!(should_retweet(&weights));
        assert!(should_quote(&weights));
        assert!(should_follow(&weights));
        assert!(should_reply(&weights));
        assert!(should_bookmark(&weights));
        assert!(should_dive(&weights));
    }

    #[test]
    fn test_should_retweet_probability_bounds() {
        let weights = PersonaWeights {
            retweet_prob: 0.0,
            ..Default::default()
        };
        let false_count = (0..100).filter(|_| should_retweet(&weights)).count();
        assert_eq!(false_count, 0);

        let weights = PersonaWeights {
            retweet_prob: 1.0,
            ..Default::default()
        };
        let true_count = (0..100).filter(|_| should_retweet(&weights)).count();
        assert_eq!(true_count, 100);
    }

    #[test]
    fn test_should_quote_probability_bounds() {
        let weights = PersonaWeights {
            quote_prob: 0.0,
            ..Default::default()
        };
        let false_count = (0..100).filter(|_| should_quote(&weights)).count();
        assert_eq!(false_count, 0);

        let weights = PersonaWeights {
            quote_prob: 1.0,
            ..Default::default()
        };
        let true_count = (0..100).filter(|_| should_quote(&weights)).count();
        assert_eq!(true_count, 100);
    }

    #[test]
    fn test_should_follow_probability_bounds() {
        let weights = PersonaWeights {
            follow_prob: 0.0,
            ..Default::default()
        };
        let false_count = (0..100).filter(|_| should_follow(&weights)).count();
        assert_eq!(false_count, 0);

        let weights = PersonaWeights {
            follow_prob: 1.0,
            ..Default::default()
        };
        let true_count = (0..100).filter(|_| should_follow(&weights)).count();
        assert_eq!(true_count, 100);
    }

    #[test]
    fn test_should_reply_probability_bounds() {
        let weights = PersonaWeights {
            reply_prob: 0.0,
            ..Default::default()
        };
        let false_count = (0..100).filter(|_| should_reply(&weights)).count();
        assert_eq!(false_count, 0);

        let weights = PersonaWeights {
            reply_prob: 1.0,
            ..Default::default()
        };
        let true_count = (0..100).filter(|_| should_reply(&weights)).count();
        assert_eq!(true_count, 100);
    }

    #[test]
    fn test_should_bookmark_probability_bounds() {
        let weights = PersonaWeights {
            bookmark_prob: 0.0,
            ..Default::default()
        };
        let false_count = (0..100).filter(|_| should_bookmark(&weights)).count();
        assert_eq!(false_count, 0);

        let weights = PersonaWeights {
            bookmark_prob: 1.0,
            ..Default::default()
        };
        let true_count = (0..100).filter(|_| should_bookmark(&weights)).count();
        assert_eq!(true_count, 100);
    }

    #[test]
    fn test_should_dive_probability_bounds() {
        let weights = PersonaWeights {
            thread_dive_prob: 0.0,
            ..Default::default()
        };
        let false_count = (0..100).filter(|_| should_dive(&weights)).count();
        assert_eq!(false_count, 0);

        let weights = PersonaWeights {
            thread_dive_prob: 1.0,
            ..Default::default()
        };
        let true_count = (0..100).filter(|_| should_dive(&weights)).count();
        assert_eq!(true_count, 100);
    }

    #[test]
    fn test_build_persona_config_no_weights() {
        let config = build_persona_config(None, None);
        assert!(config.is_object());
        assert!(config.get("weights").is_some());
        assert!(config.get("profile").is_some());
    }

    #[test]
    fn test_build_persona_config_with_weights() {
        let weights = PersonaWeights {
            like_prob: 0.5,
            retweet_prob: 0.2,
            ..Default::default()
        };
        let config = build_persona_config(Some(weights), None);
        let weights_obj = config.get("weights").unwrap();
        assert_eq!(weights_obj.get("like_prob").unwrap().as_f64().unwrap(), 0.5);
        assert_eq!(
            weights_obj.get("retweet_prob").unwrap().as_f64().unwrap(),
            0.2
        );
    }

    #[test]
    fn test_build_persona_config_with_profile() {
        let config = build_persona_config(None, Some(ProfilePreset::Teen));
        assert!(config.is_object());
        let profile = config.get("profile").unwrap();
        assert!(profile.is_string());
    }

    #[test]
    fn test_persona_weights_clamping_in_decision_functions() {
        let weights = PersonaWeights {
            like_prob: 2.0,
            retweet_prob: -1.0,
            quote_prob: 1.5,
            follow_prob: -0.5,
            reply_prob: 3.0,
            bookmark_prob: -2.0,
            thread_dive_prob: 0.5,
            interest_multiplier: 1.0,
        };
        // All functions should clamp to [0, 1] before use
        let _ = should_like(&weights);
        let _ = should_retweet(&weights);
        let _ = should_quote(&weights);
        let _ = should_follow(&weights);
        let _ = should_reply(&weights);
        let _ = should_bookmark(&weights);
        let _ = should_dive(&weights);
    }

    #[test]
    fn test_apply_behavior_profile_integration() {
        let persona = PersonaWeights::default();
        let profile = BrowserProfile::average();
        let sentiment_score = 0.5;

        let result = apply_behavior_profile(persona, &profile, sentiment_score);
        // Result should be normalized
        assert!(result.like_prob >= 0.0 && result.like_prob <= 1.0);
        assert!(result.retweet_prob >= 0.0 && result.retweet_prob <= 1.0);
        assert!(result.interest_multiplier >= 0.5);
    }
}
