/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect } from "vitest";
import {
  POSITIVE_LEXICON,
  NEGATIVE_LEXICON,
  AROUSAL_MARKERS,
  DOMINANCE_MARKERS,
  SARCASM_MARKERS,
  URGENCY_MARKERS,
  TOXICITY_MARKERS,
  EMOJI_SENTIMENT,
  CONTEXTUAL_PATTERNS,
  ACTION_GATES,
  PERSONALITY_PROFILES,
  TOPIC_KEYWORDS,
  SENTIMENT_THRESHOLDS,
} from "@api/utils/sentiment-data.js";

import sentimentDataDefault from "@api/utils/sentiment-data.js";

describe("sentiment-data.js", () => {
  describe("Named Exports", () => {
    it("should export POSITIVE_LEXICON", () => {
      expect(POSITIVE_LEXICON).toBeDefined();
      expect(typeof POSITIVE_LEXICON).toBe("object");
    });

    it("should export NEGATIVE_LEXICON", () => {
      expect(NEGATIVE_LEXICON).toBeDefined();
      expect(typeof NEGATIVE_LEXICON).toBe("object");
    });

    it("should export AROUSAL_MARKERS", () => {
      expect(AROUSAL_MARKERS).toBeDefined();
      expect(typeof AROUSAL_MARKERS).toBe("object");
    });

    it("should export DOMINANCE_MARKERS", () => {
      expect(DOMINANCE_MARKERS).toBeDefined();
      expect(typeof DOMINANCE_MARKERS).toBe("object");
    });

    it("should export SARCASM_MARKERS", () => {
      expect(SARCASM_MARKERS).toBeDefined();
      expect(typeof SARCASM_MARKERS).toBe("object");
    });

    it("should export URGENCY_MARKERS", () => {
      expect(URGENCY_MARKERS).toBeDefined();
      expect(typeof URGENCY_MARKERS).toBe("object");
    });

    it("should export TOXICITY_MARKERS", () => {
      expect(TOXICITY_MARKERS).toBeDefined();
      expect(typeof TOXICITY_MARKERS).toBe("object");
    });

    it("should export EMOJI_SENTIMENT", () => {
      expect(EMOJI_SENTIMENT).toBeDefined();
      expect(typeof EMOJI_SENTIMENT).toBe("object");
    });

    it("should export CONTEXTUAL_PATTERNS", () => {
      expect(CONTEXTUAL_PATTERNS).toBeDefined();
      expect(typeof CONTEXTUAL_PATTERNS).toBe("object");
    });

    it("should export ACTION_GATES", () => {
      expect(ACTION_GATES).toBeDefined();
      expect(typeof ACTION_GATES).toBe("object");
    });

    it("should export PERSONALITY_PROFILES", () => {
      expect(PERSONALITY_PROFILES).toBeDefined();
      expect(typeof PERSONALITY_PROFILES).toBe("object");
    });

    it("should export TOPIC_KEYWORDS", () => {
      expect(TOPIC_KEYWORDS).toBeDefined();
      expect(typeof TOPIC_KEYWORDS).toBe("object");
    });

    it("should export SENTIMENT_THRESHOLDS", () => {
      expect(SENTIMENT_THRESHOLDS).toBeDefined();
      expect(typeof SENTIMENT_THRESHOLDS).toBe("object");
    });

    it("should export default object", () => {
      expect(sentimentDataDefault).toBeDefined();
      expect(typeof sentimentDataDefault).toBe("object");
    });
  });

  describe("POSITIVE_LEXICON", () => {
    it("should have all required categories", () => {
      expect(POSITIVE_LEXICON).toHaveProperty("strong");
      expect(POSITIVE_LEXICON).toHaveProperty("moderate");
      expect(POSITIVE_LEXICON).toHaveProperty("weak");
      expect(POSITIVE_LEXICON).toHaveProperty("achievement");
      expect(POSITIVE_LEXICON).toHaveProperty("love");
      expect(POSITIVE_LEXICON).toHaveProperty("grateful");
      expect(POSITIVE_LEXICON).toHaveProperty("trust");
      expect(POSITIVE_LEXICON).toHaveProperty("beauty");
      expect(POSITIVE_LEXICON).toHaveProperty("energy");
      expect(POSITIVE_LEXICON).toHaveProperty("wise");
    });

    it("should have arrays containing strings", () => {
      for (const [_category, words] of Object.entries(POSITIVE_LEXICON)) {
        expect(Array.isArray(words)).toBe(true);
        expect(words.length).toBeGreaterThan(0);
        words.forEach((word) => {
          expect(typeof word).toBe("string");
        });
      }
    });

    it("should have strong category with most intense words", () => {
      const strong = POSITIVE_LEXICON.strong;
      expect(strong.length).toBeGreaterThan(10);
      expect(strong).toContain("love");
      expect(strong).toContain("amazing");
      expect(strong).toContain("wonderful");
    });

    it("should have love category", () => {
      const love = POSITIVE_LEXICON.love;
      expect(love).toContain("love");
      expect(love).toContain("adore");
      expect(love).toContain("cherish");
    });
  });

  describe("NEGATIVE_LEXICON", () => {
    it("should have all required categories", () => {
      expect(NEGATIVE_LEXICON).toHaveProperty("strong");
      expect(NEGATIVE_LEXICON).toHaveProperty("moderate");
      expect(NEGATIVE_LEXICON).toHaveProperty("weak");
      expect(NEGATIVE_LEXICON).toHaveProperty("tragedy");
      expect(NEGATIVE_LEXICON).toHaveProperty("grief");
      expect(NEGATIVE_LEXICON).toHaveProperty("violence");
      expect(NEGATIVE_LEXICON).toHaveProperty("scam");
      expect(NEGATIVE_LEXICON).toHaveProperty("controversy");
      expect(NEGATIVE_LEXICON).toHaveProperty("failure");
      expect(NEGATIVE_LEXICON).toHaveProperty("pain");
      expect(NEGATIVE_LEXICON).toHaveProperty("betrayal");
    });

    it("should have arrays containing strings", () => {
      for (const [_category, words] of Object.entries(NEGATIVE_LEXICON)) {
        expect(Array.isArray(words)).toBe(true);
        expect(words.length).toBeGreaterThan(0);
        words.forEach((word) => {
          expect(typeof word).toBe("string");
        });
      }
    });

    it("should have strong category with most intense words", () => {
      const strong = NEGATIVE_LEXICON.strong;
      expect(strong.length).toBeGreaterThan(10);
      expect(strong).toContain("hate");
      expect(strong).toContain("horrible");
      expect(strong).toContain("terrible");
    });

    it("should have tragedy category", () => {
      const tragedy = NEGATIVE_LEXICON.tragedy;
      expect(tragedy).toContain("death");
      expect(tragedy).toContain("died");
    });

    it("should have scam category", () => {
      const scam = NEGATIVE_LEXICON.scam;
      expect(scam).toContain("scam");
      expect(scam).toContain("fraud");
    });
  });

  describe("AROUSAL_MARKERS", () => {
    it("should have high, moderate, and low arousal levels", () => {
      expect(AROUSAL_MARKERS).toHaveProperty("high");
      expect(AROUSAL_MARKERS).toHaveProperty("moderate");
      expect(AROUSAL_MARKERS).toHaveProperty("low");
    });

    it("should have markers array in each arousal level", () => {
      expect(Array.isArray(AROUSAL_MARKERS.high.markers)).toBe(true);
      expect(AROUSAL_MARKERS.high.markers.length).toBeGreaterThan(0);
      expect(Array.isArray(AROUSAL_MARKERS.moderate.markers)).toBe(true);
      expect(AROUSAL_MARKERS.moderate.markers.length).toBeGreaterThan(0);
      expect(Array.isArray(AROUSAL_MARKERS.low.markers)).toBe(true);
      expect(AROUSAL_MARKERS.low.markers.length).toBeGreaterThan(0);
    });

    it("should have markers containing strings", () => {
      AROUSAL_MARKERS.high.markers.forEach((marker) => {
        expect(typeof marker).toBe("string");
      });
      AROUSAL_MARKERS.moderate.markers.forEach((marker) => {
        expect(typeof marker).toBe("string");
      });
      AROUSAL_MARKERS.low.markers.forEach((marker) => {
        expect(typeof marker).toBe("string");
      });
    });

    it("should have emojis array in each arousal level", () => {
      expect(Array.isArray(AROUSAL_MARKERS.high.emojis)).toBe(true);
      expect(Array.isArray(AROUSAL_MARKERS.moderate.emojis)).toBe(true);
      expect(Array.isArray(AROUSAL_MARKERS.low.emojis)).toBe(true);
    });

    it("should have allCapsWeight and exclamationWeight in high arousal", () => {
      expect(typeof AROUSAL_MARKERS.high.allCapsWeight).toBe("number");
      expect(typeof AROUSAL_MARKERS.high.exclamationWeight).toBe("number");
      expect(AROUSAL_MARKERS.high.allCapsWeight).toBeGreaterThan(0);
      expect(AROUSAL_MARKERS.high.exclamationWeight).toBeGreaterThan(0);
    });

    it("should contain expected high arousal markers", () => {
      const highMarkers = AROUSAL_MARKERS.high.markers.join(" ").toLowerCase();
      expect(highMarkers).toContain("omg");
      expect(highMarkers).toContain("wtf");
    });
  });

  describe("DOMINANCE_MARKERS", () => {
    it("should have assertive, submissive, and neutral categories", () => {
      expect(DOMINANCE_MARKERS).toHaveProperty("assertive");
      expect(DOMINANCE_MARKERS).toHaveProperty("submissive");
      expect(DOMINANCE_MARKERS).toHaveProperty("neutral");
    });

    it("should have words arrays in each category", () => {
      expect(Array.isArray(DOMINANCE_MARKERS.assertive.words)).toBe(true);
      expect(DOMINANCE_MARKERS.assertive.words.length).toBeGreaterThan(0);
      expect(Array.isArray(DOMINANCE_MARKERS.submissive.words)).toBe(true);
      expect(DOMINANCE_MARKERS.submissive.words.length).toBeGreaterThan(0);
      expect(Array.isArray(DOMINANCE_MARKERS.neutral.words)).toBe(true);
      expect(DOMINANCE_MARKERS.neutral.words.length).toBeGreaterThan(0);
    });

    it("should have words containing strings", () => {
      DOMINANCE_MARKERS.assertive.words.forEach((word) => {
        expect(typeof word).toBe("string");
      });
      DOMINANCE_MARKERS.submissive.words.forEach((word) => {
        expect(typeof word).toBe("string");
      });
      DOMINANCE_MARKERS.neutral.words.forEach((word) => {
        expect(typeof word).toBe("string");
      });
    });

    it("should have patterns in assertive category", () => {
      expect(Array.isArray(DOMINANCE_MARKERS.assertive.patterns)).toBe(true);
      DOMINANCE_MARKERS.assertive.patterns.forEach((pattern) => {
        expect(pattern instanceof RegExp).toBe(true);
      });
    });

    it("should have patterns in submissive category", () => {
      expect(Array.isArray(DOMINANCE_MARKERS.submissive.patterns)).toBe(true);
      DOMINANCE_MARKERS.submissive.patterns.forEach((pattern) => {
        expect(pattern instanceof RegExp).toBe(true);
      });
    });

    it("should contain expected assertive words", () => {
      const words = DOMINANCE_MARKERS.assertive.words.join(" ").toLowerCase();
      expect(words).toContain("must");
      expect(words).toContain("should");
      expect(words).toContain("need");
    });

    it("should contain expected submissive words", () => {
      const words = DOMINANCE_MARKERS.submissive.words.join(" ").toLowerCase();
      expect(words).toContain("maybe");
      expect(words).toContain("perhaps");
      expect(words).toContain("possibly");
    });
  });

  describe("SARCASM_MARKERS", () => {
    it("should have explicit, contradiction, context_inversion, and dry_humor categories", () => {
      expect(SARCASM_MARKERS).toHaveProperty("explicit");
      expect(SARCASM_MARKERS).toHaveProperty("contradiction");
      expect(SARCASM_MARKERS).toHaveProperty("context_inversion");
      expect(SARCASM_MARKERS).toHaveProperty("dry_humor");
    });

    it("should have markers array in explicit category", () => {
      expect(Array.isArray(SARCASM_MARKERS.explicit.markers)).toBe(true);
      expect(SARCASM_MARKERS.explicit.markers.length).toBeGreaterThan(0);
      SARCASM_MARKERS.explicit.markers.forEach((marker) => {
        expect(typeof marker).toBe("string");
      });
    });

    it("should have emojis in explicit category", () => {
      expect(Array.isArray(SARCASM_MARKERS.explicit.emojis)).toBe(true);
    });

    it("should have patterns in contradiction category", () => {
      expect(Array.isArray(SARCASM_MARKERS.contradiction.patterns)).toBe(true);
      SARCASM_MARKERS.contradiction.patterns.forEach((pattern) => {
        expect(pattern instanceof RegExp).toBe(true);
      });
    });

    it("should have examples in context_inversion category", () => {
      expect(Array.isArray(SARCASM_MARKERS.context_inversion.examples)).toBe(
        true,
      );
      SARCASM_MARKERS.context_inversion.examples.forEach((example) => {
        expect(example).toHaveProperty("pattern");
        expect(example).toHaveProperty("confidence");
        expect(typeof example.confidence).toBe("number");
        expect(example.confidence).toBeGreaterThanOrEqual(0);
        expect(example.confidence).toBeLessThanOrEqual(1);
      });
    });

    it("should have markers in dry_humor category", () => {
      expect(Array.isArray(SARCASM_MARKERS.dry_humor.markers)).toBe(true);
      SARCASM_MARKERS.dry_humor.markers.forEach((marker) => {
        expect(typeof marker).toBe("string");
      });
    });
  });

  describe("URGENCY_MARKERS", () => {
    it("should have urgent, timeSensitive, scheduled, and relaxed categories", () => {
      expect(URGENCY_MARKERS).toHaveProperty("urgent");
      expect(URGENCY_MARKERS).toHaveProperty("timeSensitive");
      expect(URGENCY_MARKERS).toHaveProperty("scheduled");
      expect(URGENCY_MARKERS).toHaveProperty("relaxed");
    });

    it("should have arrays containing strings", () => {
      expect(Array.isArray(URGENCY_MARKERS.urgent)).toBe(true);
      expect(URGENCY_MARKERS.urgent.length).toBeGreaterThan(0);
      URGENCY_MARKERS.urgent.forEach((marker) => {
        expect(typeof marker).toBe("string");
      });

      expect(Array.isArray(URGENCY_MARKERS.timeSensitive)).toBe(true);
      URGENCY_MARKERS.timeSensitive.forEach((marker) => {
        expect(typeof marker).toBe("string");
      });

      expect(Array.isArray(URGENCY_MARKERS.scheduled)).toBe(true);
      URGENCY_MARKERS.scheduled.forEach((marker) => {
        expect(typeof marker).toBe("string");
      });

      expect(Array.isArray(URGENCY_MARKERS.relaxed)).toBe(true);
      URGENCY_MARKERS.relaxed.forEach((marker) => {
        expect(typeof marker).toBe("string");
      });
    });

    it("should contain expected urgent markers", () => {
      const urgent = URGENCY_MARKERS.urgent.join(" ").toLowerCase();
      expect(urgent).toContain("urgent");
      expect(urgent).toContain("breaking");
      expect(urgent).toContain("emergency");
    });

    it("should contain expected relaxed markers", () => {
      const relaxed = URGENCY_MARKERS.relaxed.join(" ").toLowerCase();
      expect(relaxed).toContain("eventually");
      expect(relaxed).toContain("someday");
    });
  });

  describe("TOXICITY_MARKERS", () => {
    it("should have slurs_insults, hostility, personalAttacks, aggression, and dehumanization", () => {
      expect(TOXICITY_MARKERS).toHaveProperty("slurs_insults");
      expect(TOXICITY_MARKERS).toHaveProperty("hostility");
      expect(TOXICITY_MARKERS).toHaveProperty("personalAttacks");
      expect(TOXICITY_MARKERS).toHaveProperty("aggression");
      expect(TOXICITY_MARKERS).toHaveProperty("dehumanization");
    });

    it("should have slurs_insults as array of strings", () => {
      expect(Array.isArray(TOXICITY_MARKERS.slurs_insults)).toBe(true);
      expect(TOXICITY_MARKERS.slurs_insults.length).toBeGreaterThan(0);
      TOXICITY_MARKERS.slurs_insults.forEach((marker) => {
        expect(typeof marker).toBe("string");
      });
    });

    it("should have hostility as array of strings", () => {
      expect(Array.isArray(TOXICITY_MARKERS.hostility)).toBe(true);
      expect(TOXICITY_MARKERS.hostility.length).toBeGreaterThan(0);
      TOXICITY_MARKERS.hostility.forEach((marker) => {
        expect(typeof marker).toBe("string");
      });
    });

    it("should have personalAttacks as array of regex", () => {
      expect(Array.isArray(TOXICITY_MARKERS.personalAttacks)).toBe(true);
      expect(TOXICITY_MARKERS.personalAttacks.length).toBeGreaterThan(0);
      TOXICITY_MARKERS.personalAttacks.forEach((pattern) => {
        expect(pattern instanceof RegExp).toBe(true);
      });
    });

    it("should have aggression with markers array and intensity", () => {
      expect(Array.isArray(TOXICITY_MARKERS.aggression.markers)).toBe(true);
      TOXICITY_MARKERS.aggression.markers.forEach((marker) => {
        expect(typeof marker).toBe("string");
      });
      expect(typeof TOXICITY_MARKERS.aggression.intensity).toBe("number");
      expect(TOXICITY_MARKERS.aggression.intensity).toBeGreaterThan(0);
    });

    it("should have dehumanization as array of strings", () => {
      expect(Array.isArray(TOXICITY_MARKERS.dehumanization)).toBe(true);
      expect(TOXICITY_MARKERS.dehumanization.length).toBeGreaterThan(0);
      TOXICITY_MARKERS.dehumanization.forEach((marker) => {
        expect(typeof marker).toBe("string");
      });
    });
  });

  describe("EMOJI_SENTIMENT", () => {
    it("should be a non-empty object", () => {
      expect(Object.keys(EMOJI_SENTIMENT).length).toBeGreaterThan(0);
    });

    it("should have emoji keys", () => {
      expect(EMOJI_SENTIMENT).toHaveProperty("😊");
      expect(EMOJI_SENTIMENT).toHaveProperty("😍");
      expect(EMOJI_SENTIMENT).toHaveProperty("😢");
      expect(EMOJI_SENTIMENT).toHaveProperty("❤️");
    });

    it("should have numeric values between -1 and 1", () => {
      for (const [_emoji, score] of Object.entries(EMOJI_SENTIMENT)) {
        expect(typeof score).toBe("number");
        expect(score).toBeGreaterThanOrEqual(-1);
        expect(score).toBeLessThanOrEqual(1);
      }
    });

    it("should have positive emojis with positive scores", () => {
      expect(EMOJI_SENTIMENT["😊"]).toBeGreaterThan(0);
      expect(EMOJI_SENTIMENT["😍"]).toBeGreaterThan(0);
      expect(EMOJI_SENTIMENT["❤️"]).toBeGreaterThan(0);
      expect(EMOJI_SENTIMENT["🎉"]).toBeGreaterThan(0);
    });

    it("should have negative emojis with negative scores", () => {
      expect(EMOJI_SENTIMENT["😢"]).toBeLessThan(0);
      expect(EMOJI_SENTIMENT["😭"]).toBeLessThan(0);
      expect(EMOJI_SENTIMENT["😡"]).toBeLessThan(0);
    });

    it("should have neutral emojis with score around 0", () => {
      expect(EMOJI_SENTIMENT["😐"]).toBe(0);
      expect(EMOJI_SENTIMENT["🤔"]).toBe(0);
    });
  });

  describe("CONTEXTUAL_PATTERNS", () => {
    it("should have multiple pattern objects", () => {
      expect(Object.keys(CONTEXTUAL_PATTERNS).length).toBeGreaterThan(0);
    });

    it("should have fakePositivity pattern", () => {
      expect(CONTEXTUAL_PATTERNS.fakePositivity).toBeDefined();
      expect(CONTEXTUAL_PATTERNS.fakePositivity).toHaveProperty("condition");
      expect(CONTEXTUAL_PATTERNS.fakePositivity).toHaveProperty(
        "classification",
      );
      expect(CONTEXTUAL_PATTERNS.fakePositivity).toHaveProperty("actionGate");
    });

    it("should have restrainedGrief pattern", () => {
      expect(CONTEXTUAL_PATTERNS.restrainedGrief).toBeDefined();
    });

    it("should have passionateAdvocacy pattern", () => {
      expect(CONTEXTUAL_PATTERNS.passionateAdvocacy).toBeDefined();
    });

    it("should have toxicRanting pattern", () => {
      expect(CONTEXTUAL_PATTERNS.toxicRanting).toBeDefined();
    });

    it("should have intellectualDebate pattern", () => {
      expect(CONTEXTUAL_PATTERNS.intellectualDebate).toBeDefined();
    });

    it("should have sarcasticCommentary pattern", () => {
      expect(CONTEXTUAL_PATTERNS.sarcasticCommentary).toBeDefined();
    });

    it("should have crisis pattern", () => {
      expect(CONTEXTUAL_PATTERNS.crisis).toBeDefined();
    });

    it("should have celebration pattern", () => {
      expect(CONTEXTUAL_PATTERNS.celebration).toBeDefined();
    });

    it("should have callable condition functions", () => {
      for (const [_name, pattern] of Object.entries(CONTEXTUAL_PATTERNS)) {
        expect(typeof pattern.condition).toBe("function");
      }
    });

    it("should execute condition functions without error", () => {
      const mockAnalysis = {
        valence: 0.5,
        sarcasm: 0.3,
        arousal: 0.4,
        dominance: 0.5,
        urgency: 0.5,
        toxicity: 0.1,
      };

      for (const [_name, pattern] of Object.entries(CONTEXTUAL_PATTERNS)) {
        expect(() => pattern.condition(mockAnalysis)).not.toThrow();
      }
    });

    it("should have classification strings", () => {
      for (const [_name, pattern] of Object.entries(CONTEXTUAL_PATTERNS)) {
        expect(typeof pattern.classification).toBe("string");
      }
    });

    it("should have actionGate in fakePositivity pattern", () => {
      expect(typeof CONTEXTUAL_PATTERNS.fakePositivity.actionGate).toBe(
        "object",
      );
    });

    it("should have adjustment in fakePositivity", () => {
      expect(CONTEXTUAL_PATTERNS.fakePositivity.adjustment).toBeDefined();
      expect(typeof CONTEXTUAL_PATTERNS.fakePositivity.adjustment).toBe(
        "object",
      );
    });
  });

  describe("ACTION_GATES", () => {
    it("should have reply, like, quote, retweet, bookmark gates", () => {
      expect(ACTION_GATES).toHaveProperty("reply");
      expect(ACTION_GATES).toHaveProperty("like");
      expect(ACTION_GATES).toHaveProperty("quote");
      expect(ACTION_GATES).toHaveProperty("retweet");
      expect(ACTION_GATES).toHaveProperty("bookmark");
    });

    it("should have minValence in each gate", () => {
      expect(typeof ACTION_GATES.reply.minValence).toBe("number");
      expect(typeof ACTION_GATES.like.minValence).toBe("number");
      expect(typeof ACTION_GATES.quote.minValence).toBe("number");
      expect(typeof ACTION_GATES.retweet.minValence).toBe("number");
      expect(typeof ACTION_GATES.bookmark.minValence).toBe("number");
    });

    it("should have maxToxicity in each gate", () => {
      expect(typeof ACTION_GATES.reply.maxToxicity).toBe("number");
      expect(typeof ACTION_GATES.like.maxToxicity).toBe("number");
      expect(typeof ACTION_GATES.quote.maxToxicity).toBe("number");
      expect(typeof ACTION_GATES.retweet.maxToxicity).toBe("number");
      expect(typeof ACTION_GATES.bookmark.maxToxicity).toBe("number");
    });

    it("should have description in each gate", () => {
      expect(typeof ACTION_GATES.reply.description).toBe("string");
      expect(typeof ACTION_GATES.like.description).toBe("string");
      expect(typeof ACTION_GATES.quote.description).toBe("string");
      expect(typeof ACTION_GATES.retweet.description).toBe("string");
      expect(typeof ACTION_GATES.bookmark.description).toBe("string");
    });

    it("should have boolean sarcasmOk in some gates", () => {
      expect(typeof ACTION_GATES.reply.sarcasmOk).toBe("boolean");
      expect(typeof ACTION_GATES.like.sarcasmOk).toBe("boolean");
      expect(typeof ACTION_GATES.quote.sarcasmOk).toBe("boolean");
      expect(typeof ACTION_GATES.retweet.sarcasmOk).toBe("boolean");
      expect(typeof ACTION_GATES.bookmark.sarcasmOk).toBe("boolean");
    });

    it("should have exclusiveNegative arrays in some gates", () => {
      expect(Array.isArray(ACTION_GATES.like.exclusiveNegative)).toBe(true);
      expect(Array.isArray(ACTION_GATES.retweet.exclusiveNegative)).toBe(true);
    });

    it("should have mustNotHavePattern arrays", () => {
      expect(Array.isArray(ACTION_GATES.reply.mustNotHavePattern)).toBe(true);
      expect(Array.isArray(ACTION_GATES.like.mustNotHavePattern)).toBe(true);
    });
  });

  describe("PERSONALITY_PROFILES", () => {
    it("should have all required personality profiles", () => {
      expect(PERSONALITY_PROFILES).toHaveProperty("observer");
      expect(PERSONALITY_PROFILES).toHaveProperty("enthusiast");
      expect(PERSONALITY_PROFILES).toHaveProperty("analyst");
      expect(PERSONALITY_PROFILES).toHaveProperty("joker");
      expect(PERSONALITY_PROFILES).toHaveProperty("advocate");
      expect(PERSONALITY_PROFILES).toHaveProperty("empath");
    });

    it("should have required properties in each profile", () => {
      const requiredProps = [
        "replyProbability",
        "preferredTones",
        "sarcasticTolerance",
        "toxicityTolerance",
        "dominancePreference",
        "arousalThreshold",
        "negativeBias",
        "description",
      ];

      for (const [_name, profile] of Object.entries(PERSONALITY_PROFILES)) {
        requiredProps.forEach((prop) => {
          expect(profile).toHaveProperty(prop);
        });
      }
    });

    it("should have numeric values in valid ranges", () => {
      for (const [_name, profile] of Object.entries(PERSONALITY_PROFILES)) {
        expect(typeof profile.replyProbability).toBe("number");
        expect(profile.replyProbability).toBeGreaterThanOrEqual(0);
        expect(profile.replyProbability).toBeLessThanOrEqual(1);

        expect(typeof profile.sarcasticTolerance).toBe("number");
        expect(profile.sarcasticTolerance).toBeGreaterThanOrEqual(0);
        expect(profile.sarcasticTolerance).toBeLessThanOrEqual(1);

        expect(typeof profile.toxicityTolerance).toBe("number");
        expect(profile.toxicityTolerance).toBeGreaterThanOrEqual(0);
        expect(profile.toxicityTolerance).toBeLessThanOrEqual(1);

        expect(typeof profile.dominancePreference).toBe("number");
        expect(profile.dominancePreference).toBeGreaterThanOrEqual(0);
        expect(profile.dominancePreference).toBeLessThanOrEqual(1);

        expect(typeof profile.arousalThreshold).toBe("number");
        expect(profile.arousalThreshold).toBeGreaterThanOrEqual(0);
        expect(profile.arousalThreshold).toBeLessThanOrEqual(1);

        expect(typeof profile.negativeBias).toBe("number");
        expect(profile.negativeBias).toBeGreaterThanOrEqual(-1);
        expect(profile.negativeBias).toBeLessThanOrEqual(1);
      }
    });

    it("should have arrays for preferredTones", () => {
      for (const [_name, profile] of Object.entries(PERSONALITY_PROFILES)) {
        expect(Array.isArray(profile.preferredTones)).toBe(true);
        expect(profile.preferredTones.length).toBeGreaterThan(0);
        profile.preferredTones.forEach((tone) => {
          expect(typeof tone).toBe("string");
        });
      }
    });

    it("should have description strings", () => {
      for (const [_name, profile] of Object.entries(PERSONALITY_PROFILES)) {
        expect(typeof profile.description).toBe("string");
        expect(profile.description.length).toBeGreaterThan(0);
      }
    });

    it("should have distinct characteristics for each profile", () => {
      expect(PERSONALITY_PROFILES.enthusiast.replyProbability).toBeGreaterThan(
        PERSONALITY_PROFILES.observer.replyProbability,
      );
      expect(PERSONALITY_PROFILES.enthusiast.negativeBias).toBeGreaterThan(
        PERSONALITY_PROFILES.empath.negativeBias,
      );
      expect(PERSONALITY_PROFILES.empath.toxicityTolerance).toBeLessThan(
        PERSONALITY_PROFILES.joker.toxicityTolerance,
      );
    });
  });

  describe("TOPIC_KEYWORDS", () => {
    it("should have all required topic categories", () => {
      expect(TOPIC_KEYWORDS).toHaveProperty("politics");
      expect(TOPIC_KEYWORDS).toHaveProperty("religion");
      expect(TOPIC_KEYWORDS).toHaveProperty("socialJustice");
      expect(TOPIC_KEYWORDS).toHaveProperty("health");
      expect(TOPIC_KEYWORDS).toHaveProperty("technology");
      expect(TOPIC_KEYWORDS).toHaveProperty("social");
    });

    it("should have arrays containing strings", () => {
      for (const [_topic, words] of Object.entries(TOPIC_KEYWORDS)) {
        expect(Array.isArray(words)).toBe(true);
        expect(words.length).toBeGreaterThan(0);
        words.forEach((word) => {
          expect(typeof word).toBe("string");
        });
      }
    });

    it("should contain expected politics keywords", () => {
      const politics = TOPIC_KEYWORDS.politics.join(" ").toLowerCase();
      expect(politics).toContain("election");
      expect(politics).toContain("vote");
      expect(politics).toContain("president");
    });

    it("should contain expected health keywords", () => {
      const health = TOPIC_KEYWORDS.health.join(" ").toLowerCase();
      expect(health).toContain("vaccine");
      expect(health).toContain("mental health");
      expect(health).toContain("depression");
    });
  });

  describe("SENTIMENT_THRESHOLDS", () => {
    it("should have skip thresholds as numbers", () => {
      expect(typeof SENTIMENT_THRESHOLDS.skipLike).toBe("number");
      expect(typeof SENTIMENT_THRESHOLDS.skipRetweet).toBe("number");
      expect(typeof SENTIMENT_THRESHOLDS.skipReply).toBe("number");
      expect(typeof SENTIMENT_THRESHOLDS.skipQuote).toBe("number");
      expect(typeof SENTIMENT_THRESHOLDS.skipBookmark).toBe("number");
    });

    it("should have advanced thresholds as numbers", () => {
      expect(typeof SENTIMENT_THRESHOLDS.toxicityRedLine).toBe("number");
      expect(typeof SENTIMENT_THRESHOLDS.griefThreshold).toBe("number");
      expect(typeof SENTIMENT_THRESHOLDS.spamConfidence).toBe("number");
      expect(typeof SENTIMENT_THRESHOLDS.authenticityMin).toBe("number");
    });

    it("should have allowExpand as boolean", () => {
      expect(typeof SENTIMENT_THRESHOLDS.allowExpand).toBe("boolean");
    });

    it("should have valid numeric values", () => {
      expect(SENTIMENT_THRESHOLDS.skipLike).toBeGreaterThan(0);
      expect(SENTIMENT_THRESHOLDS.skipRetweet).toBeGreaterThan(0);
      expect(SENTIMENT_THRESHOLDS.skipReply).toBeGreaterThan(0);
      expect(SENTIMENT_THRESHOLDS.skipQuote).toBeGreaterThan(0);
      expect(SENTIMENT_THRESHOLDS.skipBookmark).toBeGreaterThan(0);

      expect(SENTIMENT_THRESHOLDS.toxicityRedLine).toBeGreaterThan(0);
      expect(SENTIMENT_THRESHOLDS.toxicityRedLine).toBeLessThanOrEqual(1);
      expect(SENTIMENT_THRESHOLDS.griefThreshold).toBeLessThan(0);
      expect(SENTIMENT_THRESHOLDS.spamConfidence).toBeGreaterThan(0);
      expect(SENTIMENT_THRESHOLDS.spamConfidence).toBeLessThanOrEqual(1);
      expect(SENTIMENT_THRESHOLDS.authenticityMin).toBeGreaterThan(0);
      expect(SENTIMENT_THRESHOLDS.authenticityMin).toBeLessThanOrEqual(1);
    });

    it("should have skip thresholds in ascending order", () => {
      expect(SENTIMENT_THRESHOLDS.skipRetweet).toBeLessThan(
        SENTIMENT_THRESHOLDS.skipLike,
      );
      expect(SENTIMENT_THRESHOLDS.skipLike).toBeLessThan(
        SENTIMENT_THRESHOLDS.skipReply,
      );
      expect(SENTIMENT_THRESHOLDS.skipQuote).toBeLessThan(
        SENTIMENT_THRESHOLDS.skipReply,
      );
    });
  });

  describe("Default Export", () => {
    it("should match all named exports", () => {
      expect(sentimentDataDefault.POSITIVE_LEXICON).toBe(POSITIVE_LEXICON);
      expect(sentimentDataDefault.NEGATIVE_LEXICON).toBe(NEGATIVE_LEXICON);
      expect(sentimentDataDefault.AROUSAL_MARKERS).toBe(AROUSAL_MARKERS);
      expect(sentimentDataDefault.DOMINANCE_MARKERS).toBe(DOMINANCE_MARKERS);
      expect(sentimentDataDefault.SARCASM_MARKERS).toBe(SARCASM_MARKERS);
      expect(sentimentDataDefault.URGENCY_MARKERS).toBe(URGENCY_MARKERS);
      expect(sentimentDataDefault.TOXICITY_MARKERS).toBe(TOXICITY_MARKERS);
      expect(sentimentDataDefault.EMOJI_SENTIMENT).toBe(EMOJI_SENTIMENT);
      expect(sentimentDataDefault.CONTEXTUAL_PATTERNS).toBe(
        CONTEXTUAL_PATTERNS,
      );
      expect(sentimentDataDefault.ACTION_GATES).toBe(ACTION_GATES);
      expect(sentimentDataDefault.PERSONALITY_PROFILES).toBe(
        PERSONALITY_PROFILES,
      );
      expect(sentimentDataDefault.TOPIC_KEYWORDS).toBe(TOPIC_KEYWORDS);
      expect(sentimentDataDefault.SENTIMENT_THRESHOLDS).toBe(
        SENTIMENT_THRESHOLDS,
      );
    });
  });

  describe("Data Integrity", () => {
    it("should not have empty arrays in POSITIVE_LEXICON", () => {
      for (const [category, words] of Object.entries(POSITIVE_LEXICON)) {
        expect(words.length).toBeGreaterThan(
          0,
          `Category ${category} should not be empty`,
        );
      }
    });

    it("should not have empty arrays in NEGATIVE_LEXICON", () => {
      for (const [category, words] of Object.entries(NEGATIVE_LEXICON)) {
        expect(words.length).toBeGreaterThan(
          0,
          `Category ${category} should not be empty`,
        );
      }
    });

    it("should have consistent scoring ranges in thresholds", () => {
      expect(SENTIMENT_THRESHOLDS.toxicityRedLine).toBeGreaterThan(
        SENTIMENT_THRESHOLDS.skipReply,
      );
      expect(SENTIMENT_THRESHOLDS.spamConfidence).toBeGreaterThan(0.5);
    });

    it("should have personality profiles with complementary traits", () => {
      expect(PERSONALITY_PROFILES.empath.toxicityTolerance).toBeLessThan(
        PERSONALITY_PROFILES.joker.toxicityTolerance,
      );
      expect(PERSONALITY_PROFILES.enthusiast.replyProbability).toBeGreaterThan(
        PERSONALITY_PROFILES.observer.replyProbability,
      );
    });

    it("should have action gates with appropriate risk levels", () => {
      expect(ACTION_GATES.retweet.minValence).toBeGreaterThan(
        ACTION_GATES.reply.minValence,
      );
      expect(ACTION_GATES.retweet.maxToxicity).toBeLessThan(
        ACTION_GATES.reply.maxToxicity,
      );
      expect(ACTION_GATES.bookmark.maxToxicity).toBeGreaterThan(
        ACTION_GATES.retweet.maxToxicity,
      );
    });
  });

  describe("CONTEXTUAL_PATTERNS branch coverage", () => {
    it("should trigger restrainedGrief condition", () => {
      const analysis = { valence: -0.5, arousal: 0.2 };
      expect(CONTEXTUAL_PATTERNS.restrainedGrief.condition(analysis)).toBe(
        true,
      );
    });

    it("should not trigger restrainedGrief for positive valence", () => {
      const analysis = { valence: 0.5, arousal: 0.2 };
      expect(CONTEXTUAL_PATTERNS.restrainedGrief.condition(analysis)).toBe(
        false,
      );
    });

    it("should trigger passionateAdvocacy condition", () => {
      const analysis = {
        valence: 0.5,
        arousal: 0.5,
        dominance: 0.7,
        urgency: 0.7,
        toxicity: 0.3,
      };
      expect(CONTEXTUAL_PATTERNS.passionateAdvocacy.condition(analysis)).toBe(
        true,
      );
    });

    it("should not trigger passionateAdvocacy when toxicity is high", () => {
      const analysis = {
        valence: 0.5,
        arousal: 0.5,
        dominance: 0.7,
        urgency: 0.7,
        toxicity: 0.5,
      };
      expect(CONTEXTUAL_PATTERNS.passionateAdvocacy.condition(analysis)).toBe(
        false,
      );
    });

    it("should trigger toxicRanting condition", () => {
      const analysis = { arousal: 0.8, toxicity: 0.7, dominance: 0.3 };
      expect(CONTEXTUAL_PATTERNS.toxicRanting.condition(analysis)).toBe(true);
    });

    it("should not trigger toxicRanting when dominance is high", () => {
      const analysis = { arousal: 0.8, toxicity: 0.7, dominance: 0.6 };
      expect(CONTEXTUAL_PATTERNS.toxicRanting.condition(analysis)).toBe(false);
    });

    it("should trigger intellectualDebate condition", () => {
      const analysis = { dominance: 0.7, toxicity: 0.2, arousal: 0.5 };
      expect(CONTEXTUAL_PATTERNS.intellectualDebate.condition(analysis)).toBe(
        true,
      );
    });

    it("should not trigger intellectualDebate when toxicity is high", () => {
      const analysis = { dominance: 0.7, toxicity: 0.4, arousal: 0.5 };
      expect(CONTEXTUAL_PATTERNS.intellectualDebate.condition(analysis)).toBe(
        false,
      );
    });

    it("should trigger crisis condition", () => {
      const analysis = { valence: -0.6, urgency: 0.95, toxicity: 0.3 };
      expect(CONTEXTUAL_PATTERNS.crisis.condition(analysis)).toBe(true);
    });

    it("should trigger crisis with high toxicity", () => {
      const analysis = { valence: 0, urgency: 0.95, toxicity: 0.8 };
      expect(CONTEXTUAL_PATTERNS.crisis.condition(analysis)).toBe(true);
    });

    it("should trigger celebration condition", () => {
      const analysis = { arousal: 0.8, valence: 0.7, toxicity: 0.1 };
      expect(CONTEXTUAL_PATTERNS.celebration.condition(analysis)).toBe(true);
    });

    it("should not trigger celebration when valence is low", () => {
      const analysis = { arousal: 0.8, valence: 0.5, toxicity: 0.1 };
      expect(CONTEXTUAL_PATTERNS.celebration.condition(analysis)).toBe(false);
    });
  });
});
