/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Unit tests for utils/ai-reply-engine/config.js
 * @module tests/unit/ai-reply-engine-config.test
 */

import { describe, it, expect, vi } from "vitest";
import {
  SAFETY_FILTERS,
  AIReplyEngine,
} from "@api/agent/ai-reply-engine/config.js";

vi.mock("@api/core/logger.js", () => ({
  createLogger: vi.fn(() => ({
    info: vi.fn(),
    warn: vi.fn(),
    debug: vi.fn(),
    error: vi.fn(),
  })),
}));

describe("ai-reply-engine/config", () => {
  describe("SAFETY_FILTERS", () => {
    it("should have minTweetLength defined", () => {
      expect(SAFETY_FILTERS.minTweetLength).toBe(10);
    });

    it("should have maxTweetLength defined", () => {
      expect(SAFETY_FILTERS.maxTweetLength).toBe(500);
    });

    it("should have excludedKeywords array", () => {
      expect(Array.isArray(SAFETY_FILTERS.excludedKeywords)).toBe(true);
      expect(SAFETY_FILTERS.excludedKeywords.length).toBeGreaterThan(0);
    });

    it("should contain political keywords", () => {
      expect(SAFETY_FILTERS.excludedKeywords).toContain("politics");
      expect(SAFETY_FILTERS.excludedKeywords).toContain("trump");
      expect(SAFETY_FILTERS.excludedKeywords).toContain("biden");
    });

    it("should contain NSFW keywords", () => {
      expect(SAFETY_FILTERS.excludedKeywords).toContain("nsfw");
      expect(SAFETY_FILTERS.excludedKeywords).toContain("porn");
    });

    it("should contain scam/spam keywords", () => {
      expect(SAFETY_FILTERS.excludedKeywords).toContain("dm me");
      expect(SAFETY_FILTERS.excludedKeywords).toContain("free crypto");
    });
  });

  describe("AIReplyEngine Constructor", () => {
    it("should create instance with default options", () => {
      const engine = new AIReplyEngine({ processRequest: vi.fn() });

      expect(engine.config.REPLY_PROBABILITY).toBe(0.05);
      expect(engine.config.MAX_REPLY_LENGTH).toBe(280);
      expect(engine.config.MIN_REPLY_LENGTH).toBe(10);
      expect(engine.config.MAX_RETRIES).toBe(2);
      expect(engine.config.SAFETY_FILTERS).toBe(SAFETY_FILTERS);
    });

    it("should create instance with custom options", () => {
      const engine = new AIReplyEngine(
        { processRequest: vi.fn() },
        { replyProbability: 0.5, maxRetries: 5 },
      );

      expect(engine.config.REPLY_PROBABILITY).toBe(0.5);
      expect(engine.config.MAX_RETRIES).toBe(5);
    });

    it("should initialize stats with zeros", () => {
      const engine = new AIReplyEngine({ processRequest: vi.fn() });

      expect(engine.stats.attempts).toBe(0);
      expect(engine.stats.successes).toBe(0);
      expect(engine.stats.skips).toBe(0);
      expect(engine.stats.failures).toBe(0);
      expect(engine.stats.safetyBlocks).toBe(0);
      expect(engine.stats.errors).toBe(0);
    });

    it("should store agent reference", () => {
      const agent = { processRequest: vi.fn(), sessionId: "test-123" };
      const engine = new AIReplyEngine(agent);

      expect(engine.agent).toBe(agent);
    });
  });

  describe("updateConfig", () => {
    it("should update replyProbability", () => {
      const engine = new AIReplyEngine({ processRequest: vi.fn() });
      engine.updateConfig({ replyProbability: 0.8 });

      expect(engine.config.REPLY_PROBABILITY).toBe(0.8);
    });

    it("should update maxRetries", () => {
      const engine = new AIReplyEngine({ processRequest: vi.fn() });
      engine.updateConfig({ maxRetries: 10 });

      expect(engine.config.MAX_RETRIES).toBe(10);
    });

    it("should ignore undefined options", () => {
      const engine = new AIReplyEngine(
        { processRequest: vi.fn() },
        { replyProbability: 0.3 },
      );
      engine.updateConfig({
        replyProbability: undefined,
        maxRetries: undefined,
      });

      expect(engine.config.REPLY_PROBABILITY).toBe(0.3);
      expect(engine.config.MAX_RETRIES).toBe(2);
    });
  });

  describe("getStats", () => {
    it("should return zero rates when no attempts", () => {
      const engine = new AIReplyEngine({ processRequest: vi.fn() });
      const stats = engine.getStats();

      expect(stats.successRate).toBe("0%");
      expect(stats.skipRate).toBe("0%");
    });

    it("should calculate success rate correctly", () => {
      const engine = new AIReplyEngine({ processRequest: vi.fn() });
      engine.stats.attempts = 10;
      engine.stats.successes = 3;
      engine.stats.skips = 7;

      const stats = engine.getStats();

      expect(stats.successRate).toBe("30.0%");
      expect(stats.skipRate).toBe("70.0%");
    });

    it("should include all stat fields", () => {
      const engine = new AIReplyEngine({ processRequest: vi.fn() });
      engine.stats.attempts = 5;
      engine.stats.successes = 2;
      engine.stats.skips = 2;
      engine.stats.failures = 1;

      const stats = engine.getStats();

      expect(stats.attempts).toBe(5);
      expect(stats.successes).toBe(2);
      expect(stats.skips).toBe(2);
      expect(stats.failures).toBe(1);
    });
  });

  describe("resetStats", () => {
    it("should reset all stats to zero", () => {
      const engine = new AIReplyEngine({ processRequest: vi.fn() });
      engine.stats.attempts = 100;
      engine.stats.successes = 50;
      engine.stats.skips = 30;
      engine.stats.failures = 15;
      engine.stats.safetyBlocks = 5;
      engine.stats.errors = 2;

      engine.resetStats();

      expect(engine.stats.attempts).toBe(0);
      expect(engine.stats.successes).toBe(0);
      expect(engine.stats.skips).toBe(0);
      expect(engine.stats.failures).toBe(0);
      expect(engine.stats.safetyBlocks).toBe(0);
      expect(engine.stats.errors).toBe(0);
    });
  });
});
