/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Unit tests for utils/ai-reply-engine/decision.js
 * @module tests/unit/ai-reply-engine-decision.test
 */

import { describe, it, expect, vi, beforeEach } from "vitest";

vi.mock("@api/core/logger.js", () => ({
  createLogger: vi.fn(() => ({
    info: vi.fn(),
    warn: vi.fn(),
    debug: vi.fn(),
    error: vi.fn(),
  })),
}));

vi.mock("@api/utils/math.js", () => ({
  mathUtils: {
    roll: vi.fn(),
  },
}));

vi.mock("@api/utils/sentiment-service.js", () => ({
  sentimentService: {
    analyze: vi.fn(),
  },
}));

vi.mock("@api/utils/twitter-reply-prompt.js", () => ({
  REPLY_SYSTEM_PROMPT: "You are a helpful assistant",
  getStrategyInstruction: vi.fn(() => "Be casual and friendly"),
  getReplyLengthGuidance: vi.fn(() => "Keep it brief"),
  getSentimentGuidance: vi.fn(() => "Be positive"),
}));

describe("ai-reply-engine/decision", () => {
  let shouldReply;
  let applySafetyFilters;
  let AIReplyEngine;
  let mathUtils;
  let sentimentService;
  let mockEngine;

  const baseSentiment = {
    isNegative: false,
    score: 0,
    dimensions: {
      valence: { valence: 0 },
      sarcasm: { sarcasm: 0 },
      toxicity: { toxicity: 0 },
    },
    composite: {
      riskLevel: "low",
      engagementStyle: "neutral",
      conversationType: "general",
    },
  };

  beforeEach(async () => {
    vi.clearAllMocks();
    vi.resetModules();

    const module = await import("../../agent/ai-reply-engine/decision.js");
    shouldReply = module.shouldReply;

    const engineModule = await import("../../agent/ai-reply-engine/index.js");
    AIReplyEngine = engineModule.default;

    const mathModule = await import("@api/utils/math.js");
    mathUtils = mathModule.mathUtils;

    const sentimentModule = await import("@api/utils/sentiment-service.js");
    sentimentService = sentimentModule.sentimentService;

    mockEngine = new AIReplyEngine(
      { processRequest: vi.fn(), sessionId: "test" },
      { replyProbability: 1, maxRetries: 1 },
    );
  }, 20000);

  describe("shouldReply", () => {
    it("should skip when probability roll fails", async () => {
      mathUtils.roll.mockReturnValue(false);

      const result = await shouldReply(mockEngine, "hello world", "user");

      expect(result.decision).toBe("skip");
      expect(result.reason).toBe("probability");
    });

    it("should skip negative content", async () => {
      mathUtils.roll.mockReturnValue(true);
      sentimentService.analyze.mockReturnValue({
        ...baseSentiment,
        isNegative: true,
        score: 0.5,
      });

      const result = await shouldReply(mockEngine, "this is bad", "user");

      expect(result.decision).toBe("skip");
      expect(result.reason).toBe("negative_content");
    });

    it("should skip high risk conversations", async () => {
      mathUtils.roll.mockReturnValue(true);
      sentimentService.analyze.mockReturnValue({
        ...baseSentiment,
        composite: { ...baseSentiment.composite, riskLevel: "high" },
      });

      const result = await shouldReply(mockEngine, "some content", "user");

      expect(result.decision).toBe("skip");
      expect(result.reason).toBe("high_risk_conversation");
    });

    it("should proceed when all checks pass", async () => {
      const result = await shouldReply(mockEngine, "some content", "user");

      expect(result.decision).toBe("skip");
      expect(result.reason).toBe("high_risk_conversation");
    });

    it("should proceed when all checks pass", async () => {
      mathUtils.roll.mockReturnValue(true);
      sentimentService.analyze.mockReturnValue(baseSentiment);

      mockEngine.generateReply = vi
        .fn()
        .mockResolvedValue({ success: true, reply: "Great reply!" });
      mockEngine.validateReply = vi
        .fn()
        .mockReturnValue({ valid: true, reason: "passed" });
      mockEngine.applySafetyFilters = vi
        .fn()
        .mockReturnValue({ safe: true, reason: "passed" });

      const result = await shouldReply(
        mockEngine,
        "This is a good tweet",
        "user",
      );

      expect(result.decision).toBe("reply");
    });

    it("should increment attempts counter", async () => {
      mathUtils.roll.mockReturnValue(false);

      await shouldReply(mockEngine, "test", "user");

      expect(mockEngine.stats.attempts).toBe(1);
    });

    it("should increment skips counter on probability skip", async () => {
      mathUtils.roll.mockReturnValue(false);

      await shouldReply(mockEngine, "test", "user");

      expect(mockEngine.stats.skips).toBe(1);
    });

    it("should return null action on probability skip", async () => {
      mathUtils.roll.mockReturnValue(false);

      const result = await shouldReply(mockEngine, "test", "user");

      expect(result.action).toBeNull();
    });
  });

  describe("applySafetyFilters (via engine)", () => {
    it("should reject empty text", () => {
      const result = mockEngine.applySafetyFilters("");
      expect(result.safe).toBe(false);
      expect(result.reason).toBe("empty_text");
    });

    it("should reject text that is too short", () => {
      const result = mockEngine.applySafetyFilters("hi");
      expect(result.safe).toBe(false);
      expect(result.reason).toBe("too_short");
    });

    it("should reject text with excluded keywords", () => {
      const result = mockEngine.applySafetyFilters(
        "I love politics and elections",
      );
      expect(result.safe).toBe(false);
      expect(result.reason).toContain("excluded_keyword");
    });

    it("should reject text with excessive caps", () => {
      const result = mockEngine.applySafetyFilters(
        "THIS IS ALL CAPS AND VERY LONG TEXT THAT SHOULD BE BLOCKED",
      );
      expect(result.safe).toBe(false);
      expect(result.reason).toBe("excessive_caps");
    });

    it("should reject text with too many emojis", () => {
      const emojis = "😀".repeat(10);
      const result = mockEngine.applySafetyFilters(emojis);
      expect(result.safe).toBe(false);
      expect(result.reason).toBe("too_many_emojis");
    });

    it("should accept valid text", () => {
      const result = mockEngine.applySafetyFilters(
        "This is a normal tweet about technology",
      );
      expect(result.safe).toBe(true);
    });
  });

  describe("shouldReply error handling", () => {
    it("should handle null author username", async () => {
      mathUtils.roll.mockReturnValue(true);
      sentimentService.analyze.mockReturnValue({
        ...baseSentiment,
        isNegative: false,
      });

      const result = await shouldReply(mockEngine, "test content", null);
      expect(result).toBeDefined();
    });

    it("should handle empty tweet text", async () => {
      mathUtils.roll.mockReturnValue(true);
      sentimentService.analyze.mockReturnValue({
        ...baseSentiment,
        isNegative: false,
      });

      const result = await shouldReply(mockEngine, "", "user");
      expect(result).toBeDefined();
    });
  });

  describe("applySafetyFilters edge cases", () => {
    it("should reject empty string (edge case behavior)", () => {
      const result = mockEngine.applySafetyFilters("");
      expect(result.safe).toBe(false);
    });

    it("should handle unicode characters", () => {
      const result = mockEngine.applySafetyFilters("Hello 🌍 World");
      expect(result).toBeDefined();
    });

    it("should handle very long text", () => {
      const longText = "a".repeat(1000);
      const result = mockEngine.applySafetyFilters(longText);
      expect(result).toBeDefined();
    });

    it("should reject text exceeding max length", () => {
      const longTweet = "a".repeat(501);
      const result = mockEngine.applySafetyFilters(longTweet);
      expect(result.reason).toBe("too_long");
    });

    it("should accept exactly minimum length", () => {
      const result = mockEngine.applySafetyFilters("hello world");
      expect(result.safe).toBe(true);
    });
  });

  describe("shouldReply validation paths", () => {
    it("should skip when safety filter blocks", async () => {
      mathUtils.roll.mockReturnValue(true);
      sentimentService.analyze.mockReturnValue(baseSentiment);
      mockEngine.applySafetyFilters = vi
        .fn()
        .mockReturnValue({ safe: false, reason: "too_short" });

      const result = await shouldReply(mockEngine, "hi", "user");

      expect(result.decision).toBe("skip");
      expect(result.reason).toBe("safety");
    });
  });
});
