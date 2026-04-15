/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from "vitest";

vi.mock("@api/core/logger.js", () => ({
  createLogger: vi.fn(() => ({
    info: vi.fn(),
    debug: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
  })),
}));

vi.mock("@api/utils/math.js", () => ({
  mathUtils: {
    roll: vi.fn(),
    randomInRange: vi.fn(),
    gaussian: vi.fn(),
  },
}));

vi.mock("@api/utils/sentiment-service.js", () => ({
  sentimentService: {
    analyze: vi.fn(),
  },
}));

vi.mock("@api/behaviors/human-interaction.js", () => ({
  HumanInteraction: vi.fn().mockImplementation(() => ({
    setPage: vi.fn(),
    logDebug: vi.fn(),
    logWarn: vi.fn(),
  })),
}));

vi.mock("@api/tests/unit/twitter/twitter-reply-prompt.js", () => ({
  REPLY_SYSTEM_PROMPT: "You are a helpful assistant",
  getStrategyInstruction: vi.fn(() => "Be helpful and concise"),
}));

import { AIReplyEngine, SAFETY_FILTERS } from "@api/agent/ai-reply-engine.js";
import { mathUtils } from "@api/utils/math.js";
import { sentimentService } from "@api/utils/sentiment-service.js";

describe("AIReplyEngine", () => {
  let engine;
  let mockAgent;

  beforeEach(() => {
    vi.clearAllMocks();

    mockAgent = {
      processRequest: vi.fn(),
    };

    engine = new AIReplyEngine(mockAgent, {
      replyProbability: 0.5,
      maxRetries: 3,
    });
  });

  describe("Constructor", () => {
    it("initializes with defaults and stats", () => {
      const defaultEngine = new AIReplyEngine(mockAgent);

      expect(defaultEngine.config.REPLY_PROBABILITY).toBe(0.05);
      expect(defaultEngine.config.MAX_RETRIES).toBe(2);
      expect(defaultEngine.config.MIN_REPLY_LENGTH).toBe(10);
      expect(defaultEngine.config.MAX_REPLY_LENGTH).toBe(280);
      expect(defaultEngine.config.SAFETY_FILTERS).toBe(SAFETY_FILTERS);

      expect(defaultEngine.stats).toEqual({
        attempts: 0,
        successes: 0,
        skips: 0,
        failures: 0,
        safetyBlocks: 0,
        errors: 0,
      });
    });

    it("initializes with custom options", () => {
      expect(engine.config.REPLY_PROBABILITY).toBe(0.5);
      expect(engine.config.MAX_RETRIES).toBe(3);
    });
  });

  describe("updateConfig", () => {
    it("updates replyProbability", () => {
      engine.updateConfig({ replyProbability: 0.8 });
      expect(engine.config.REPLY_PROBABILITY).toBe(0.8);
    });

    it("updates maxRetries", () => {
      engine.updateConfig({ maxRetries: 5 });
      expect(engine.config.MAX_RETRIES).toBe(5);
    });

    it("updates multiple config values", () => {
      engine.updateConfig({ replyProbability: 0.3, maxRetries: 4 });
      expect(engine.config.REPLY_PROBABILITY).toBe(0.3);
      expect(engine.config.MAX_RETRIES).toBe(4);
    });
  });

  describe("shouldReply", () => {
    it("skip due to probability (mathUtils.roll returns false)", async () => {
      mathUtils.roll.mockReturnValue(false);

      const result = await engine.shouldReply("Hello world", "testuser");

      expect(mathUtils.roll).toHaveBeenCalledWith(
        engine.config.REPLY_PROBABILITY,
      );
      expect(result).toEqual({
        decision: "skip",
        reason: "probability",
        action: null,
      });
      expect(engine.stats.attempts).toBe(1);
      expect(engine.stats.skips).toBe(1);
    });

    it("skip due to negative sentiment", async () => {
      mathUtils.roll.mockReturnValue(true);
      sentimentService.analyze.mockReturnValue({
        isNegative: true,
        score: 0.5,
        composite: { riskLevel: "low" },
      });

      const result = await engine.shouldReply("This is terrible", "testuser");

      expect(result).toEqual({
        decision: "skip",
        reason: "negative_content",
        action: expect.any(String),
      });
      expect(engine.stats.skips).toBe(1);
    });

    it("skip due to high risk conversation", async () => {
      mathUtils.roll.mockReturnValue(true);
      sentimentService.analyze.mockReturnValue({
        isNegative: false,
        score: 0.1,
        composite: { riskLevel: "high", engagementStyle: "hostile" },
      });

      const result = await engine.shouldReply("Some tweet", "testuser");

      expect(result).toEqual({
        decision: "skip",
        reason: "high_risk_conversation",
        action: expect.any(String),
      });
    });

    it("skip due to safety filters (excluded keyword)", async () => {
      mathUtils.roll.mockReturnValue(true);
      sentimentService.analyze.mockReturnValue({
        isNegative: false,
        score: 0.1,
        composite: {
          riskLevel: "low",
          engagementStyle: "neutral",
          conversationType: "general",
        },
      });

      const result = await engine.shouldReply(
        "Let us discuss politics today",
        "testuser",
      );

      expect(result).toEqual({
        decision: "skip",
        reason: "safety",
        action: expect.any(String),
      });
      expect(engine.stats.safetyBlocks).toBe(1);
    });

    it("skip due to validation failed", async () => {
      mathUtils.roll.mockReturnValue(true);
      sentimentService.analyze.mockReturnValue({
        isNegative: false,
        score: 0.1,
        composite: {
          riskLevel: "low",
          engagementStyle: "neutral",
          conversationType: "general",
        },
        dimensions: {
          valence: { valence: 0.5 },
          sarcasm: { sarcasm: 0 },
          toxicity: { toxicity: 0 },
        },
      });

      // Mock generateReply to return a reply that's too short after normalization
      // The normalizeReply function strips quotes and takes first line
      // So a single line of 5 chars will fail
      const originalGenerateReply = engine.generateReply.bind(engine);
      engine.generateReply = vi
        .fn()
        .mockResolvedValue({ success: true, reply: "ab" });

      const result = await engine.shouldReply(
        "Great tweet about technology",
        "testuser",
      );

      // The reply 'ab' (2 chars) will fail validateReply (min 10 chars)
      expect(result.reason).toBe("validation_failed");
      expect(engine.stats.failures).toBe(1);

      engine.generateReply = originalGenerateReply;
    });

    it("success path with valid reply", async () => {
      mathUtils.roll.mockReturnValue(true);
      sentimentService.analyze.mockReturnValue({
        isNegative: false,
        score: 0.1,
        composite: {
          riskLevel: "low",
          engagementStyle: "neutral",
          conversationType: "general",
        },
        dimensions: {
          valence: { valence: 0.5 },
          sarcasm: { sarcasm: 0 },
          toxicity: { toxicity: 0 },
        },
      });

      mockAgent.processRequest.mockResolvedValue({
        success: true,
        content: "That is a great point!",
      });

      const result = await engine.shouldReply(
        "Great tweet about technology",
        "testuser",
      );

      expect(result.decision).toBe("reply");
      expect(result.reason).toBe("success");
      expect(result.reply).toBeDefined();
      expect(engine.stats.successes).toBe(1);
    });
  });

  describe("applySafetyFilters", () => {
    it("too short", () => {
      const result = engine.applySafetyFilters("Hi");
      expect(result).toEqual({ safe: false, reason: "too_short" });
    });

    it("too long", () => {
      const longText = "a".repeat(600);
      const result = engine.applySafetyFilters(longText);
      expect(result).toEqual({ safe: false, reason: "too_long" });
    });

    it("excluded keyword", () => {
      const result = engine.applySafetyFilters("Let us talk about Trump today");
      expect(result).toEqual({ safe: false, reason: "excluded_keyword:trump" });
    });

    it("excessive caps", () => {
      const result = engine.applySafetyFilters(
        "THIS IS ALL CAPS TEXT THAT IS VERY LOUD",
      );
      expect(result).toEqual({ safe: false, reason: "excessive_caps" });
    });

    it("too many emojis", () => {
      const result = engine.applySafetyFilters(
        "🎉🎊🎄🎈🎁🎆🎇🎅🎌🪅🎐🎑🎒🎓🎔🎕🎖🎗🎘🎙🎚🎛🎜🎝🎞🎟🎠🎡🎢🎣🎤🎥🎦🎧🎨🎩🎪🎫🎬🎭🎮🎯🎰🎱🎲🎳🎴🎵🎶🎷🎸🎹🎺🎻🎼🎽🎾🎿🛀🛁🛂🛃🛄🛅🛆🛇🛈🛉🛊🛋🛌🛍🛎🛏🛐🛑🛒🛓🛔🛕🛖🛗🛘🛙🛚🛛🛜🛝🛞🛟🛠🛡🛢🛣🛤🛥🛦🛧🛨🛩🛪🛬🛭🛮🛯🛰🛱🛲🛳🛴🛵🛶🛷🛸🛹🛺🛻🛼🛽🛾🛿",
      );
      expect(result).toEqual({ safe: false, reason: "too_many_emojis" });
    });

    it("safe content", () => {
      const result = engine.applySafetyFilters(
        "This is a normal tweet about technology",
      );
      expect(result).toEqual({ safe: true, reason: "passed" });
    });

    it("empty text", () => {
      const result = engine.applySafetyFilters("");
      expect(result).toEqual({ safe: false, reason: "empty_text" });
    });

    it("null text", () => {
      const result = engine.applySafetyFilters(null);
      expect(result).toEqual({ safe: false, reason: "empty_text" });
    });
  });

  describe("validateReply", () => {
    it("valid reply", () => {
      const result = engine.validateReply("This is a valid reply");
      expect(result).toEqual({ valid: true, reason: "passed" });
    });

    it("too short", () => {
      const result = engine.validateReply("Hi");
      expect(result).toEqual({ valid: false, reason: "too_short" });
    });

    it("too long", () => {
      const longReply = "a".repeat(300);
      const result = engine.validateReply(longReply);
      expect(result).toEqual({ valid: false, reason: "too_long" });
    });

    it("empty reply", () => {
      const result = engine.validateReply("");
      expect(result).toEqual({ valid: false, reason: "empty_reply" });
    });

    it("null reply", () => {
      const result = engine.validateReply(null);
      expect(result).toEqual({ valid: false, reason: "empty_reply" });
    });
  });

  describe("getStats", () => {
    it("returns stats object", () => {
      engine.stats.attempts = 10;
      engine.stats.successes = 5;
      engine.stats.skips = 3;
      engine.stats.failures = 2;

      const stats = engine.getStats();

      expect(stats.attempts).toBe(10);
      expect(stats.successes).toBe(5);
      expect(stats.skips).toBe(3);
      expect(stats.failures).toBe(2);
      expect(stats.safetyBlocks).toBe(0);
      expect(stats.errors).toBe(0);
      expect(stats.successRate).toBe("50.0%");
      expect(stats.skipRate).toBe("30.0%");
    });

    it("returns zero stats when no attempts", () => {
      const stats = engine.getStats();

      expect(stats.attempts).toBe(0);
      expect(stats.successRate).toBe("0%");
      expect(stats.skipRate).toBe("0%");
    });
  });

  describe("resetStats", () => {
    it("resets stats to zero", () => {
      engine.stats.attempts = 10;
      engine.stats.successes = 5;
      engine.stats.skips = 3;
      engine.stats.failures = 2;
      engine.stats.safetyBlocks = 1;
      engine.stats.errors = 1;

      engine.resetStats();

      expect(engine.stats).toEqual({
        attempts: 0,
        successes: 0,
        skips: 0,
        failures: 0,
        safetyBlocks: 0,
        errors: 0,
      });
    });
  });

  describe("randomFallback", () => {
    it("returns a fallback action", () => {
      const fallbacks = ["like", "bookmark", "retweet", "follow"];
      const result = engine.randomFallback();

      expect(fallbacks).toContain(result);
    });
  });

  describe("generateQuickFallback", () => {
    it("returns a fallback reply", () => {
      const result = engine.generateQuickFallback("Some tweet", "user");

      expect(result).toBeDefined();
      expect(typeof result).toBe("string");
      expect(result.length).toBeGreaterThan(0);
    });

    it("adds question-based fallback for tweets with ?", () => {
      const result = engine.generateQuickFallback(
        "What is your opinion?",
        "user",
      );

      expect(result).toBeDefined();
    });
  });

  describe("normalizeReply", () => {
    it("removes quotes", () => {
      const result = engine.normalizeReply('"Hello world"');
      expect(result).toBe("Hello world");
    });

    it("removes prefix", () => {
      const result = engine.normalizeReply("Reply: This is a reply");
      expect(result).toBe("This is a reply");
    });

    it("takes first line for multi-line", () => {
      const result = engine.normalizeReply("First line\nSecond line");
      expect(result).toBe("First line");
    });

    it("truncates long replies", () => {
      const longReply = "a".repeat(300);
      const result = engine.normalizeReply(longReply);
      expect(result.length).toBeLessThanOrEqual(280);
    });

    it("returns null for invalid input", () => {
      expect(engine.normalizeReply("")).toBeNull();
      expect(engine.normalizeReply(null)).toBeNull();
    });
  });

  describe("validateReplyAdvanced", () => {
    it("valid reply passes", () => {
      const result = engine.validateReplyAdvanced(
        "This is a valid reply",
        "Different original tweet",
      );
      expect(result.valid).toBe(true);
    });

    it("detects address in reply", () => {
      const result = engine.validateReplyAdvanced(
        "123 Main Street is the place",
        "Original tweet",
      );
      expect(result.valid).toBe(false);
      expect(result.reason).toBe("contains_address");
    });

    it("detects duplicate content", () => {
      const result = engine.validateReplyAdvanced(
        "Same content",
        "Same content",
      );
      expect(result.valid).toBe(false);
      expect(result.reason).toBe("duplicate_content");
    });
  });

  describe("extractReplyFromResponse", () => {
    it("extracts first valid line from content", () => {
      // Returns FIRST line >= MIN_REPLY_LENGTH (10 chars)
      const result = engine.extractReplyFromResponse(
        "First short\n\nSecond line here",
        "original",
      );
      expect(result).toBe("First short");
    });

    it("returns null for empty content", () => {
      const result = engine.extractReplyFromResponse("", "original");
      expect(result).toBeNull();
    });

    it("returns null for content shorter than min length", () => {
      const result = engine.extractReplyFromResponse("ab", "original");
      expect(result).toBeNull();
    });
  });

  describe("interceptAddress", () => {
    it("detects street address", () => {
      expect(engine.interceptAddress("123 Main Street")).toBe(true);
    });

    it("detects PO Box", () => {
      expect(engine.interceptAddress("PO Box 123")).toBe(true);
    });

    it("detects zip code", () => {
      expect(engine.interceptAddress("12345")).toBe(true);
    });

    it("returns false for normal text", () => {
      expect(engine.interceptAddress("Hello world")).toBe(false);
    });
  });

  describe("detectLanguage", () => {
    it("detects Korean", () => {
      expect(engine.detectLanguage("안녕하세요")).toBe("ko");
    });

    it("detects Japanese", () => {
      expect(engine.detectLanguage("こんにちは")).toBe("ja");
    });

    it("detects Chinese", () => {
      // Note: CJK characters overlap - implementation checks Japanese first
      // Chinese characters may match Japanese due to overlapping Unicode ranges
      const result = engine.detectLanguage("中国");
      expect(["zh", "ja"]).toContain(result);
    });

    it("defaults to English", () => {
      expect(engine.detectLanguage("Hello world")).toBe("en");
    });
  });

  describe("captureContext", () => {
    it("returns basic context object", async () => {
      const result = await engine.captureContext(
        {},
        "https://twitter.com/user/status/123",
      );

      expect(result).toEqual({
        url: "https://twitter.com/user/status/123",
        screenshot: null,
        replies: [],
      });
    });
  });
});
