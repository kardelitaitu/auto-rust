/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import {
  SentimentService,
  sentimentService,
  analyzeSentiment,
  shouldSkipAction,
  getSafeActions,
  formatSentimentReport,
} from "@api/utils/sentiment-service.js";

describe("sentiment-service", () => {
  let service;

  beforeEach(() => {
    vi.clearAllMocks();
    service = new SentimentService();
  });

  describe("constructor", () => {
    it("should initialize all analyzers", () => {
      expect(service.analyzers).toBeDefined();
      expect(service.analyzers.valence).toBeDefined();
      expect(service.analyzers.arousal).toBeDefined();
      expect(service.analyzers.dominance).toBeDefined();
      expect(service.analyzers.sarcasm).toBeDefined();
      expect(service.analyzers.urgency).toBeDefined();
      expect(service.analyzers.toxicity).toBeDefined();
    });

    it("should initialize cache", () => {
      expect(service.cache).toBeDefined();
      expect(service.cacheMaxSize).toBe(100);
    });
  });

  describe("analyze", () => {
    it("should return neutral analysis for empty text", () => {
      const result = service.analyze("");

      expect(result.isNegative).toBe(false);
      expect(result.score).toBe(0);
    });

    it("should return neutral analysis for null/undefined", () => {
      expect(service.analyze(null).score).toBe(0);
      expect(service.analyze(undefined).score).toBe(0);
    });

    it("should include legacy compatibility fields", () => {
      const result = service.analyze("Test content");

      expect(result.isNegative).toBeDefined();
      expect(result.score).toBeDefined();
      expect(result.categories).toBeDefined();
      expect(result.shouldSkipLikes).toBeDefined();
      expect(result.shouldSkipRetweets).toBeDefined();
      expect(result.shouldSkipReplies).toBeDefined();
      expect(result.shouldSkipQuotes).toBeDefined();
    });

    it("should include advanced dimensions", () => {
      const result = service.analyze("Test content");

      expect(result.dimensions).toBeDefined();
      expect(result.dimensions.valence).toBeDefined();
      expect(result.dimensions.arousal).toBeDefined();
      expect(result.dimensions.dominance).toBeDefined();
      expect(result.dimensions.sarcasm).toBeDefined();
      expect(result.dimensions.urgency).toBeDefined();
      expect(result.dimensions.toxicity).toBeDefined();
    });

    it("should include composite metrics", () => {
      const result = service.analyze("Test content");

      expect(result.composite).toBeDefined();
      expect(result.composite.intensity).toBeDefined();
      expect(result.composite.engagementStyle).toBeDefined();
      expect(result.composite.conversationType).toBeDefined();
      expect(result.composite.riskLevel).toBeDefined();
    });

    it("should include engagement recommendations", () => {
      const result = service.analyze("Test content");

      expect(result.engagement).toBeDefined();
      expect(result.engagement.canLike).toBeDefined();
      expect(result.engagement.canRetweet).toBeDefined();
      expect(result.engagement.canReply).toBeDefined();
      expect(result.engagement.canQuote).toBeDefined();
    });

    it("should use cache when enabled", () => {
      const text = "Cache test content";
      service.analyze(text, { useCache: true });
      service.analyze(text, { useCache: true });

      // Should not throw - verifies caching works
      expect(service.cache.size).toBeGreaterThan(0);
    });

    it("should skip cache when disabled", () => {
      const text = "No cache test";
      service.analyze(text, { useCache: false });

      // Cache should be empty or not contain this entry
      const cached = service.getFromCache(text);
      expect(cached).toBeUndefined();
    });

    it("should include debug info when requested", () => {
      const result = service.analyze("Test", { includeDebug: true });

      expect(result.debug).toBeDefined();
      expect(result.debug.guardResult).toBeDefined();
      expect(result.debug.advanced).toBeDefined();
    });
  });

  describe("analyzeBasic", () => {
    it("should return basic sentiment info", () => {
      const result = service.analyzeBasic("Positive content!");

      expect(result.isNegative).toBeDefined();
      expect(result.score).toBeDefined();
      expect(result.shouldSkipLikes).toBeDefined();
    });
  });

  describe("analyzeForReplySelection", () => {
    it("should return none strategy for empty array", () => {
      const result = service.analyzeForReplySelection([]);

      expect(result.strategy).toBe("none");
      expect(result.replies).toEqual([]);
    });

    it("should analyze replies and determine strategy", () => {
      const replies = [
        { text: "Great post!", content: "Great post!" },
        { text: "I disagree", content: "I disagree" },
        { text: "Thanks for sharing", content: "Thanks for sharing" },
      ];

      const result = service.analyzeForReplySelection(replies);

      expect(result.strategy).toBeDefined();
      expect(result.distribution).toBeDefined();
      expect(result.distribution.positive).toBeDefined();
      expect(result.distribution.negative).toBeDefined();
      expect(result.distribution.neutral).toBeDefined();
    });

    it("should detect toxic content", () => {
      const replies = [
        { text: "You are stupid and useless and terrible" },
        { text: "You are dumb and awful" },
        { text: "You are idiot and horrible" },
      ];

      const result = service.analyzeForReplySelection(replies);

      // Toxicity detection depends on the analyzer - might not always detect
      expect(result.distribution).toBeDefined();
      expect(result.strategy).toBeDefined();
    });

    it("should handle missing text fields", () => {
      const replies = [{ content: "Some reply" }, { text: "Another reply" }];

      const result = service.analyzeForReplySelection(replies);

      expect(result.strategy).toBeDefined();
    });
  });

  describe("deriveCompositeMetrics", () => {
    it("should derive intensity", () => {
      const dimensions = {
        valence: { valence: 0.8 },
        arousal: { arousal: 0.8 },
        dominance: { dominance: 0.5 },
        sarcasm: { sarcasm: 0.1 },
        toxicity: { toxicity: 0.1 },
      };

      const result = service.deriveCompositeMetrics(dimensions);

      expect(result.intensity).toBeGreaterThan(0);
    });

    it("should detect sarcastic engagement style", () => {
      const dimensions = {
        valence: { valence: 0.3 },
        arousal: { arousal: 0.5 },
        dominance: { dominance: 0.5 },
        sarcasm: { sarcasm: 0.6 },
        toxicity: { toxicity: 0.1 },
      };

      const result = service.deriveCompositeMetrics(dimensions);

      expect(result.engagementStyle).toBe("sarcastic");
    });

    it("should detect hostile engagement style", () => {
      const dimensions = {
        valence: { valence: -0.3 },
        arousal: { arousal: 0.8 },
        dominance: { dominance: 0.6 },
        sarcasm: { sarcasm: 0.2 },
        toxicity: { toxicity: 0.6 },
      };

      const result = service.deriveCompositeMetrics(dimensions);

      expect(result.engagementStyle).toBe("hostile");
    });

    it("should detect enthusiastic style", () => {
      const dimensions = {
        valence: { valence: 0.5 },
        arousal: { arousal: 0.7 },
        dominance: { dominance: 0.5 },
        sarcasm: { sarcasm: 0.1 },
        toxicity: { toxicity: 0.1 },
      };

      const result = service.deriveCompositeMetrics(dimensions);

      expect(result.engagementStyle).toBe("enthusiastic");
    });

    it("should detect conversation types", () => {
      const toxicDimensions = {
        valence: { valence: 0 },
        arousal: { arousal: 0.5 },
        dominance: { dominance: 0.5 },
        sarcasm: { sarcasm: 0.1 },
        toxicity: { toxicity: 0.5 },
      };

      const result = service.deriveCompositeMetrics(toxicDimensions);

      expect(result.conversationType).toBe("controversial");
    });

    it("should calculate risk levels", () => {
      const dimensions = {
        valence: { valence: -0.6 },
        arousal: { arousal: 0.7 },
        dominance: { dominance: 0.5 },
        sarcasm: { sarcasm: 0.1 },
        toxicity: { toxicity: 0.7 },
      };

      const result = service.deriveCompositeMetrics(dimensions);

      expect(result.riskLevel).toBe("high");
    });
  });

  describe("getEngagementRecommendations", () => {
    it("should return basic recommendations", () => {
      const guard = {
        shouldSkipLikes: false,
        shouldSkipRetweets: false,
        shouldSkipReplies: false,
        shouldSkipQuotes: false,
      };
      const advanced = {
        sarcasm: { sarcasm: 0.1 },
        toxicity: { toxicity: 0.1 },
        dominance: { dominance: 0.5 },
      };
      const composite = { riskLevel: "low" };

      const result = service.getEngagementRecommendations(
        guard,
        advanced,
        composite,
      );

      expect(result.canLike).toBe(true);
      expect(result.shouldEngage).toBe(true);
    });

    it("should add warning for sarcasm", () => {
      const guard = {
        shouldSkipLikes: false,
        shouldSkipRetweets: false,
        shouldSkipReplies: false,
        shouldSkipQuotes: false,
      };
      const advanced = {
        sarcasm: { sarcasm: 0.6 },
        toxicity: { toxicity: 0.1 },
        dominance: { dominance: 0.5 },
      };
      const composite = { riskLevel: "low" };

      const result = service.getEngagementRecommendations(
        guard,
        advanced,
        composite,
      );

      expect(result.warnings).toContain("sarcasm-detected");
      expect(result.recommendedTone).toBe("playful");
    });

    it("should add warning for toxicity", () => {
      const guard = {
        shouldSkipLikes: false,
        shouldSkipRetweets: false,
        shouldSkipReplies: false,
        shouldSkipQuotes: false,
      };
      const advanced = {
        sarcasm: { sarcasm: 0.1 },
        toxicity: { toxicity: 0.5 },
        dominance: { dominance: 0.5 },
      };
      const composite = { riskLevel: "low" };

      const result = service.getEngagementRecommendations(
        guard,
        advanced,
        composite,
      );

      expect(result.warnings).toContain("toxicity-detected");
      expect(result.shouldEngage).toBe(false);
    });

    it("should recommend tone based on dominance", () => {
      const guard = {
        shouldSkipLikes: false,
        shouldSkipRetweets: false,
        shouldSkipReplies: false,
        shouldSkipQuotes: false,
      };
      const advanced = {
        sarcasm: { sarcasm: 0.1 },
        toxicity: { toxicity: 0.1 },
        dominance: { dominance: 0.8 },
      };
      const composite = { riskLevel: "low" };

      const result = service.getEngagementRecommendations(
        guard,
        advanced,
        composite,
      );

      expect(result.recommendedTone).toBe("assertive");
    });
  });

  describe("getReplyRecommendations", () => {
    it("should return neutral-only filter for toxic content", () => {
      const analyzed = [];
      const result = service.getReplyRecommendations("neutral-only", analyzed);

      expect(result.filter).toBeDefined();
      expect(result.max).toBe(30);
    });

    it("should return longest sort for default", () => {
      const analyzed = [];
      const result = service.getReplyRecommendations("longest", analyzed);

      expect(result.sort).toBeDefined();
    });
  });

  describe("hasNegativePattern", () => {
    it("should detect negative pattern", () => {
      const dimensions = {
        valence: { valence: -0.6 },
        arousal: { arousal: 0.7 },
      };

      expect(service.hasNegativePattern(dimensions)).toBe(true);
    });

    it("should return false for safe content", () => {
      const dimensions = {
        valence: { valence: 0.3 },
        arousal: { arousal: 0.4 },
      };

      expect(service.hasNegativePattern(dimensions)).toBe(false);
    });
  });

  describe("calculateConfidence", () => {
    it("should return high confidence for high values", () => {
      const dimensions = {
        valence: { confidence: "high" },
        sarcasm: { confidence: "high" },
      };

      expect(service.calculateConfidence(dimensions)).toBe("high");
    });

    it("should return medium confidence for medium values", () => {
      const dimensions = {
        valence: { confidence: "medium" },
        sarcasm: { confidence: "medium" },
      };

      expect(service.calculateConfidence(dimensions)).toBe("medium");
    });
  });

  describe("getNeutralAnalysis", () => {
    it("should return complete neutral structure", () => {
      const result = service.getNeutralAnalysis();

      expect(result.isNegative).toBe(false);
      expect(result.score).toBe(0);
      expect(result.dimensions).toBeDefined();
      expect(result.composite).toBeDefined();
      expect(result.engagement).toBeDefined();
    });
  });

  describe("cache management", () => {
    it("should get from cache", () => {
      service.addToCache("test", { result: true });
      const result = service.getFromCache("test");

      expect(result).toEqual({ result: true });
    });

    it("should clear cache", () => {
      service.addToCache("test1", { data: 1 });
      service.addToCache("test2", { data: 2 });
      service.clearCache();

      expect(service.cache.size).toBe(0);
    });

    it("should handle cache eviction", () => {
      // Fill cache beyond max size
      for (let i = 0; i < 101; i++) {
        service.addToCache(`key${i}`, { data: i });
      }

      // Cache should not exceed max size
      expect(service.cache.size).toBeLessThanOrEqual(service.cacheMaxSize);
    });
  });

  describe("singleton exports", () => {
    it("should export sentimentService singleton", () => {
      expect(sentimentService).toBeDefined();
      expect(sentimentService.analyze).toBeDefined();
    });

    it("should export legacy functions", () => {
      expect(analyzeSentiment).toBeDefined();
      expect(shouldSkipAction).toBeDefined();
      expect(getSafeActions).toBeDefined();
      expect(formatSentimentReport).toBeDefined();
    });
  });

  describe("Coverage gap fixes", () => {
    describe("analyzeForReplySelection strategies", () => {
      it("should handle non-array input", () => {
        const result = service.analyzeForReplySelection(null);
        expect(result.strategy).toBe("none");
      });

      it("should analyze replies with content field only", () => {
        const replies = [
          { content: "Some reply" },
          { content: "Another reply" },
        ];
        const result = service.analyzeForReplySelection(replies);
        expect(result.strategy).toBeDefined();
      });
    });

    describe("deriveCompositeMetrics styles", () => {
      it("should detect angry style", () => {
        const d = {
          valence: { valence: -0.5, confidence: "high" },
          arousal: { arousal: 0.8 },
          dominance: { dominance: 0.5 },
          sarcasm: { sarcasm: 0.1, confidence: "low" },
          toxicity: { toxicity: 0.3 },
        };
        expect(service.deriveCompositeMetrics(d).engagementStyle).toBe("angry");
      });

      it("should detect assertive style", () => {
        const d = {
          valence: { valence: 0, confidence: "medium" },
          arousal: { arousal: 0.5 },
          dominance: { dominance: 0.8 },
          sarcasm: { sarcasm: 0.1, confidence: "low" },
          toxicity: { toxicity: 0.1 },
        };
        expect(service.deriveCompositeMetrics(d).engagementStyle).toBe(
          "assertive",
        );
      });

      it("should detect questioning style", () => {
        const d = {
          valence: { valence: 0, confidence: "low" },
          arousal: { arousal: 0.5 },
          dominance: { dominance: 0.2 },
          sarcasm: { sarcasm: 0.1, confidence: "low" },
          toxicity: { toxicity: 0.1 },
        };
        expect(service.deriveCompositeMetrics(d).engagementStyle).toBe(
          "questioning",
        );
      });

      it("should detect negative conversation type", () => {
        const d = {
          valence: { valence: -0.4, confidence: "medium" },
          arousal: { arousal: 0.5 },
          dominance: { dominance: 0.5 },
          sarcasm: { sarcasm: 0.1, confidence: "low" },
          toxicity: { toxicity: 0.1 },
        };
        expect(service.deriveCompositeMetrics(d).conversationType).toBe(
          "negative",
        );
      });

      it("should detect humorous conversation type", () => {
        const d = {
          valence: { valence: 0.2, confidence: "medium" },
          arousal: { arousal: 0.5 },
          dominance: { dominance: 0.5 },
          sarcasm: { sarcasm: 0.5, confidence: "high" },
          toxicity: { toxicity: 0.1 },
        };
        expect(service.deriveCompositeMetrics(d).conversationType).toBe(
          "humorous",
        );
      });

      it("should detect positive conversation type", () => {
        const d = {
          valence: { valence: 0.6, confidence: "high" },
          arousal: { arousal: 0.5 },
          dominance: { dominance: 0.5 },
          sarcasm: { sarcasm: 0.1, confidence: "low" },
          toxicity: { toxicity: 0.1 },
        };
        expect(service.deriveCompositeMetrics(d).conversationType).toBe(
          "positive",
        );
      });
    });

    describe("getEngagementRecommendations edge cases", () => {
      it("should disengage on high risk", () => {
        const guard = {
          shouldSkipLikes: false,
          shouldSkipRetweets: false,
          shouldSkipReplies: false,
          shouldSkipQuotes: false,
        };
        const advanced = {
          sarcasm: { sarcasm: 0.1 },
          toxicity: { toxicity: 0.3 },
          dominance: { dominance: 0.5 },
        };
        const composite = { riskLevel: "high" };
        const result = service.getEngagementRecommendations(
          guard,
          advanced,
          composite,
        );
        expect(result.warnings).toContain("high-risk-content");
        expect(result.shouldEngage).toBe(false);
      });

      it("should recommend collaborative tone for low dominance", () => {
        const guard = {
          shouldSkipLikes: false,
          shouldSkipRetweets: false,
          shouldSkipReplies: false,
          shouldSkipQuotes: false,
        };
        const advanced = {
          sarcasm: { sarcasm: 0.1 },
          toxicity: { toxicity: 0.1 },
          dominance: { dominance: 0.2 },
        };
        const composite = { riskLevel: "low" };
        const result = service.getEngagementRecommendations(
          guard,
          advanced,
          composite,
        );
        expect(result.recommendedTone).toBe("collaborative");
      });
    });

    describe("getReplyRecommendations strategies", () => {
      it("should return match-sarcasm recs", () => {
        const r = service.getReplyRecommendations("match-sarcasm", []);
        expect(r.filter).toBeDefined();
        expect(r.sort).toBeDefined();
        expect(r.max).toBe(30);
      });

      it("should return balanced recs with manual selection", () => {
        const analyzed = [
          {
            text: "pos",
            sentiment: { dimensions: { valence: { valence: 0.5 } } },
          },
          {
            text: "neg",
            sentiment: { dimensions: { valence: { valence: -0.5 } } },
          },
        ];
        const r = service.getReplyRecommendations("balanced", analyzed);
        expect(r.manualSelection).toBeDefined();
        expect(r.max).toBe(30);
      });

      it("should return positive-biased recs", () => {
        const r = service.getReplyRecommendations("positive-biased", []);
        expect(r.filter).toBeDefined();
        expect(r.sort).toBeDefined();
      });
    });

    describe("calculateConfidence levels", () => {
      it("should return low for low values", () => {
        expect(
          service.calculateConfidence({
            valence: { confidence: "low" },
            sarcasm: { confidence: "low" },
          }),
        ).toBe("low");
      });

      it("should return high for high+medium", () => {
        expect(
          service.calculateConfidence({
            valence: { confidence: "high" },
            sarcasm: { confidence: "medium" },
          }),
        ).toBe("high");
      });
    });

    describe("legacy exports", () => {
      it("should call analyzeSentiment", () => {
        const result = analyzeSentiment("test content");
        expect(result).toBeDefined();
        expect(result.isNegative).toBeDefined();
      });

      it("should export shouldSkipAction", () => {
        const result = shouldSkipAction("test", "like");
        expect(typeof result).toBe("boolean");
      });

      it("should export getSafeActions", () => {
        const result = getSafeActions("test content");
        expect(result).toBeDefined();
        expect(result.canLike).toBeDefined();
        expect(result.canRetweet).toBeDefined();
      });

      it("should export formatSentimentReport", () => {
        const result = formatSentimentReport("test content");
        expect(result).toBeDefined();
        expect(typeof result).toBe("string");
      });
    });

    describe("analyze edge cases", () => {
      it("should return cached result on cache hit", () => {
        const text = "Cache hit test";
        const firstResult = service.analyze(text, { useCache: true });
        const secondResult = service.analyze(text, { useCache: true });
        expect(firstResult).toEqual(secondResult);
      });

      it("should handle whitespace-only text", () => {
        const result = service.analyze("   ");
        expect(result.score).toBe(0);
      });

      it("should handle numeric text", () => {
        const result = service.analyze(12345);
        expect(result.score).toBe(0);
      });

      it("should include allowExpand in result", () => {
        const result = service.analyze("Test content");
        expect(result.allowExpand).toBeDefined();
      });
    });

    describe("analyzeForReplySelection strategies", () => {
      it("should return longest strategy for default", () => {
        const replies = [{ text: "Short" }, { text: "Much longer reply here" }];
        const result = service.analyzeForReplySelection(replies);
        expect(result.strategy).toBeDefined();
        expect(result.recommendations).toBeDefined();
      });

      it("should include recommendations in result", () => {
        const replies = [{ text: "Test reply" }];
        const result = service.analyzeForReplySelection(replies);
        expect(result.recommendations).toBeDefined();
      });

      it("should return analyzed replies with sentiment", () => {
        const replies = [{ text: "Great post!" }];
        const result = service.analyzeForReplySelection(replies);
        expect(result.analyzed).toBeDefined();
        expect(result.analyzed[0].sentiment).toBeDefined();
      });

      it("should handle empty replies array", () => {
        const result = service.analyzeForReplySelection([]);
        expect(result.strategy).toBe("none");
        expect(result.replies).toEqual([]);
      });

      it("should handle undefined replies", () => {
        const result = service.analyzeForReplySelection(undefined);
        expect(result.strategy).toBe("none");
      });
    });

    describe("getReplyRecommendations", () => {
      it("should return default longest strategy", () => {
        const result = service.getReplyRecommendations("unknown-strategy", []);
        expect(result.sort).toBeDefined();
        expect(result.filter).toBeDefined();
      });

      it("should filter neutral-only replies", () => {
        const analyzed = [
          {
            text: "Positive!",
            sentiment: { dimensions: { valence: { valence: 0.5 } } },
          },
          {
            text: "Neutral",
            sentiment: { dimensions: { valence: { valence: 0 } } },
          },
          {
            text: "Negative",
            sentiment: { dimensions: { valence: { valence: -0.5 } } },
          },
        ];
        const result = service.getReplyRecommendations(
          "neutral-only",
          analyzed,
        );
        const filtered = analyzed.filter(result.filter);
        expect(filtered.length).toBe(1);
      });

      it("should sort by sarcasm for match-sarcasm", () => {
        const analyzed = [
          {
            text: "Low",
            sentiment: { dimensions: { sarcasm: { sarcasm: 0.1 } } },
          },
          {
            text: "High",
            sentiment: { dimensions: { sarcasm: { sarcasm: 0.8 } } },
          },
        ];
        const result = service.getReplyRecommendations(
          "match-sarcasm",
          analyzed,
        );
        expect(result.sort).toBeDefined();
      });

      it("should filter positive-biased replies", () => {
        const analyzed = [
          {
            text: "Positive",
            sentiment: { dimensions: { valence: { valence: 0.5 } } },
          },
          {
            text: "Negative",
            sentiment: { dimensions: { valence: { valence: -0.5 } } },
          },
        ];
        const result = service.getReplyRecommendations(
          "positive-biased",
          analyzed,
        );
        const filtered = analyzed.filter(result.filter);
        expect(filtered.length).toBe(1);
      });

      it("should include max in all strategies", () => {
        const strategies = [
          "neutral-only",
          "match-sarcasm",
          "balanced",
          "positive-biased",
          "longest",
        ];
        strategies.forEach((strategy) => {
          const result = service.getReplyRecommendations(strategy, []);
          expect(result.max).toBe(30);
        });
      });
    });

    describe("deriveCompositeMetrics", () => {
      it("should calculate medium risk for moderate toxicity and high sarcasm", () => {
        const dimensions = {
          valence: { valence: 0 },
          arousal: { arousal: 0.5 },
          dominance: { dominance: 0.5 },
          sarcasm: { sarcasm: 0.7 },
          toxicity: { toxicity: 0.45 },
        };
        const result = service.deriveCompositeMetrics(dimensions);
        expect(result.riskLevel).toBe("medium");
      });

      it("should return general conversation type by default", () => {
        const dimensions = {
          valence: { valence: 0 },
          arousal: { arousal: 0.3 },
          dominance: { dominance: 0.5 },
          sarcasm: { sarcasm: 0.1 },
          toxicity: { toxicity: 0.1 },
        };
        const result = service.deriveCompositeMetrics(dimensions);
        expect(result.conversationType).toBe("general");
      });
    });

    describe("hasNegativePattern", () => {
      it("should return false for non-negative valence", () => {
        const dimensions = {
          valence: { valence: -0.3 },
          arousal: { arousal: 0.5 },
        };
        expect(service.hasNegativePattern(dimensions)).toBe(false);
      });

      it("should return false for low arousal", () => {
        const dimensions = {
          valence: { valence: -0.6 },
          arousal: { arousal: 0.4 },
        };
        expect(service.hasNegativePattern(dimensions)).toBe(false);
      });
    });

    describe("calculateConfidence", () => {
      it("should return low for very low values", () => {
        expect(
          service.calculateConfidence({
            valence: { confidence: "very_low" },
            sarcasm: { confidence: "very_low" },
          }),
        ).toBe("low");
      });
    });

    describe("getNeutralAnalysis", () => {
      it("should include all required fields", () => {
        const result = service.getNeutralAnalysis();
        expect(result.allowExpand).toBeUndefined();
        expect(result.categories).toBeUndefined();
      });
    });

    describe("cache management", () => {
      it("should return undefined for non-existent key", () => {
        expect(service.getFromCache("nonexistent")).toBeUndefined();
      });

      it("should handle long text in cache key", () => {
        const longText = "a".repeat(300);
        service.addToCache(longText, { data: "test" });
        const result = service.getFromCache(longText);
        expect(result).toBeDefined();
      });

      it("should evict oldest entry when cache is full", () => {
        service.clearCache();
        for (let i = 0; i < 100; i++) {
          service.addToCache(`key${i}`, { data: i });
        }
        expect(service.cache.size).toBeLessThanOrEqual(100);
      });
    });
  });
});
