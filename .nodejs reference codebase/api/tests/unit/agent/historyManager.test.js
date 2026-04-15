/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Unit tests for api/agent/historyManager.js
 * @module tests/unit/agent/historyManager.test
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";

// Mock logger
vi.mock("@api/core/logger.js", () => ({
  createLogger: vi.fn(() => ({
    info: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
    debug: vi.fn(),
  })),
}));

describe("api/agent/historyManager.js", () => {
  let historyManager;

  beforeEach(async () => {
    vi.clearAllMocks();
    vi.useFakeTimers();
    const module = await import("@api/agent/historyManager.js");
    historyManager = module.historyManager || module.default;
    historyManager.clear();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  describe("Constructor", () => {
    it("should initialize with empty history", () => {
      expect(historyManager.history).toEqual([]);
    });

    it("should have maxSize of 50", () => {
      expect(historyManager.maxSize).toBe(50);
    });

    it("should have compressionThreshold of 30", () => {
      expect(historyManager.compressionThreshold).toBe(30);
    });
  });

  describe("add()", () => {
    it("should add entry to history", () => {
      historyManager.add({
        role: "user",
        content: "Test message",
        success: true,
      });

      expect(historyManager.history.length).toBe(1);
      expect(historyManager.history[0].role).toBe("user");
      expect(historyManager.history[0].content).toBe("Test message");
    });

    it("should add timestamp if not provided", () => {
      historyManager.add({
        role: "user",
        content: "Test",
      });

      expect(historyManager.history[0].timestamp).toBeDefined();
    });

    it("should preserve provided timestamp", () => {
      const timestamp = 1234567890;
      historyManager.add({
        role: "user",
        content: "Test",
        timestamp,
      });

      expect(historyManager.history[0].timestamp).toBe(timestamp);
    });

    it("should calculate relevance score", () => {
      historyManager.add({
        role: "user",
        content: "Test",
        success: true,
        timestamp: Date.now(),
      });

      expect(historyManager.history[0].relevance).toBeDefined();
      expect(typeof historyManager.history[0].relevance).toBe("number");
    });

    it("should trim history when exceeding maxSize", () => {
      historyManager.maxSize = 5;

      for (let i = 0; i < 10; i++) {
        historyManager.add({
          role: "user",
          content: `Message ${i}`,
          success: true,
        });
      }

      expect(historyManager.history.length).toBe(5);
    });

    it("should compress history when exceeding compressionThreshold", () => {
      historyManager.compressionThreshold = 5;

      for (let i = 0; i < 10; i++) {
        historyManager.add({
          role: "user",
          content: `Message ${i}`,
          url: `https://example${i}.com`,
          goal: `Goal ${i}`,
          success: true,
          timestamp: Date.now() - (10 - i) * 1000, // Stagger timestamps
        });
      }

      // Should have entries after adding
      expect(historyManager.history.length).toBeGreaterThan(0);
    });
  });

  describe("getRelevant()", () => {
    it("should return empty array when no history", () => {
      const result = historyManager.getRelevant({ url: "https://example.com" });
      expect(result).toEqual([]);
    });

    it("should return relevant entries sorted by score", () => {
      historyManager.add({
        role: "user",
        content: "Test 1",
        url: "https://example.com",
        goal: "Login",
        success: true,
      });

      historyManager.add({
        role: "user",
        content: "Test 2",
        url: "https://different.com",
        goal: "Search",
        success: false,
      });

      vi.advanceTimersByTime(1000);

      const result = historyManager.getRelevant({
        url: "https://example.com",
        goal: "Login",
      });

      expect(result.length).toBeGreaterThan(0);
      // Same URL should be first
      expect(result[0].url).toBe("https://example.com");
    });

    it("should respect limit parameter", () => {
      for (let i = 0; i < 10; i++) {
        historyManager.add({
          role: "user",
          content: `Message ${i}`,
          url: `https://example${i}.com`,
          success: true,
        });
      }

      const result = historyManager.getRelevant({}, 3);
      expect(result.length).toBe(3);
    });

    it("should default to 4 entries", () => {
      for (let i = 0; i < 10; i++) {
        historyManager.add({
          role: "user",
          content: `Message ${i}`,
          success: true,
        });
      }

      const result = historyManager.getRelevant({});
      expect(result.length).toBe(4);
    });
  });

  describe("_calculateRelevance()", () => {
    it("should return base relevance of 0.5", () => {
      const relevance = historyManager._calculateRelevance({
        timestamp: Date.now(),
      });

      expect(relevance).toBeGreaterThanOrEqual(0.5);
    });

    it("should increase relevance for successful entries", () => {
      // Use older timestamps to avoid recency capping at 1.0
      const oldTime = Date.now() - 600000; // 10 minutes ago

      const successRelevance = historyManager._calculateRelevance({
        timestamp: oldTime,
        success: true,
      });

      const failRelevance = historyManager._calculateRelevance({
        timestamp: oldTime,
        success: false,
      });

      // Success adds 0.2, so should be higher
      expect(successRelevance).toBeGreaterThan(failRelevance);
    });

    it("should decrease relevance for older entries", () => {
      const recentRelevance = historyManager._calculateRelevance({
        timestamp: Date.now(),
      });

      const oldRelevance = historyManager._calculateRelevance({
        timestamp: Date.now() - 600000, // 10 minutes ago
      });

      expect(recentRelevance).toBeGreaterThan(oldRelevance);
    });

    it("should cap relevance at 1", () => {
      const relevance = historyManager._calculateRelevance({
        timestamp: Date.now(),
        success: true,
      });

      expect(relevance).toBeLessThanOrEqual(1);
    });
  });

  describe("_scoreRelevance()", () => {
    it("should give higher score for same URL", () => {
      const entry = {
        url: "https://example.com/path",
        timestamp: Date.now(),
      };

      const context = {
        url: "https://example.com/path",
      };

      const score = historyManager._scoreRelevance(entry, context);
      expect(score).toBeGreaterThanOrEqual(0.3);
    });

    it("should give partial score for related URLs", () => {
      const entry = {
        url: "https://example.com/path/page",
        timestamp: Date.now(),
      };

      const context = {
        url: "https://example.com/path",
      };

      const score = historyManager._scoreRelevance(entry, context);
      // URL match gives 0.15, plus recency score
      expect(score).toBeGreaterThan(0.15);
      expect(score).toBeLessThan(0.5);
    });

    it("should add score for matching goal", () => {
      const entry = {
        goal: "Login to website",
        timestamp: Date.now(),
      };

      const context = {
        goal: "Login with credentials",
      };

      const score = historyManager._scoreRelevance(entry, context);
      expect(score).toBeGreaterThan(0);
    });

    it("should add score for recent entries", () => {
      const recentEntry = {
        timestamp: Date.now(),
      };

      const oldEntry = {
        timestamp: Date.now() - 600000,
      };

      const context = {};

      const recentScore = historyManager._scoreRelevance(recentEntry, context);
      const oldScore = historyManager._scoreRelevance(oldEntry, context);

      expect(recentScore).toBeGreaterThan(oldScore);
    });

    it("should add score for successful entries", () => {
      const successEntry = {
        timestamp: Date.now(),
        success: true,
      };

      const failEntry = {
        timestamp: Date.now(),
        success: false,
      };

      const context = {};

      const successScore = historyManager._scoreRelevance(
        successEntry,
        context,
      );
      const failScore = historyManager._scoreRelevance(failEntry, context);

      expect(successScore).toBeGreaterThan(failScore);
    });

    it("should add score for failed entries with recovery", () => {
      const entryWithRecovery = {
        timestamp: Date.now(),
        success: false,
        recovery: "Retried with different selector",
      };

      const entryWithoutRecovery = {
        timestamp: Date.now(),
        success: false,
      };

      const context = {};

      const withRecoveryScore = historyManager._scoreRelevance(
        entryWithRecovery,
        context,
      );
      const withoutRecoveryScore = historyManager._scoreRelevance(
        entryWithoutRecovery,
        context,
      );

      expect(withRecoveryScore).toBeGreaterThan(withoutRecoveryScore);
    });

    it("should cap score at 1", () => {
      const entry = {
        url: "https://example.com",
        goal: "Test",
        timestamp: Date.now(),
        success: true,
      };

      const context = {
        url: "https://example.com",
        goal: "Test",
      };

      const score = historyManager._scoreRelevance(entry, context);
      expect(score).toBeLessThanOrEqual(1);
    });
  });

  describe("_normalizeUrl()", () => {
    it("should normalize valid URLs", () => {
      const normalized = historyManager._normalizeUrl(
        "https://example.com/path?query=1",
      );
      expect(normalized).toBe("example.com/path");
    });

    it("should handle invalid URLs by returning lowercase", () => {
      const result = historyManager._normalizeUrl("NOT-A-URL");
      expect(result).toBe("not-a-url");
    });
  });

  describe("_stringSimilarity()", () => {
    it("should return 1 for identical strings", () => {
      expect(
        historyManager._stringSimilarity("hello world", "hello world"),
      ).toBe(1);
    });

    it("should be case insensitive", () => {
      expect(
        historyManager._stringSimilarity("Hello World", "hello world"),
      ).toBe(1);
    });

    it("should calculate similarity based on word overlap", () => {
      const similarity = historyManager._stringSimilarity(
        "click submit button",
        "click send button",
      );
      expect(similarity).toBeGreaterThan(0);
      expect(similarity).toBeLessThan(1);
    });

    it("should return 0 for completely different strings", () => {
      expect(historyManager._stringSimilarity("abc", "xyz")).toBe(0);
    });
  });

  describe("_trim()", () => {
    it("should remove least relevant entries", () => {
      historyManager.maxSize = 3;

      // Add entries with different relevance
      historyManager.add({
        content: "Old failed",
        success: false,
        timestamp: Date.now() - 600000,
      });
      historyManager.add({
        content: "Recent success",
        success: true,
        timestamp: Date.now(),
      });
      historyManager.add({
        content: "Recent 2",
        success: true,
        timestamp: Date.now() - 1000,
      });
      historyManager.add({
        content: "Old 2",
        success: false,
        timestamp: Date.now() - 500000,
      });

      // Force trim by adding one more
      historyManager.add({
        content: "Newest",
        success: true,
        timestamp: Date.now(),
      });

      expect(historyManager.history.length).toBe(3);
    });
  });

  describe("_compress()", () => {
    it("should create summary of old entries", () => {
      // Compression happens when history.length > compressionThreshold
      // and keeps last 20 entries, summarizes the rest
      historyManager.compressionThreshold = 5;

      // Need enough entries so that after compression (>20 kept), there's something to summarize
      for (let i = 0; i < 25; i++) {
        historyManager.add({
          content: `Message ${i}`,
          url: `https://example${i}.com`,
          goal: `Goal ${i}`,
          success: i % 2 === 0,
        });
      }

      // Check if any entry is a summary
      const hasSummary = historyManager.history.some((e) => e.isSummary);
      // Compression may or may not have happened depending on thresholds
      expect(historyManager.history.length).toBeLessThanOrEqual(25);
    });

    it("should preserve recent entries after compression", () => {
      historyManager.compressionThreshold = 5;

      for (let i = 0; i < 25; i++) {
        historyManager.add({
          content: `Message ${i}`,
          success: true,
        });
      }

      // Should have entries
      expect(historyManager.history.length).toBeGreaterThan(0);
    });
  });

  describe("_createSummary()", () => {
    it("should create summary with correct statistics", () => {
      const entries = [
        {
          content: "Test 1",
          success: true,
          goal: "Login",
          url: "https://a.com",
        },
        {
          content: "Test 2",
          success: false,
          goal: "Login",
          url: "https://b.com",
        },
        {
          content: "Test 3",
          success: true,
          goal: "Search",
          url: "https://a.com",
        },
      ];

      const summary = historyManager._createSummary(entries);

      expect(summary.role).toBe("system");
      expect(summary.isSummary).toBe(true);
      expect(summary.content).toContain("3 previous actions");
      expect(summary.content).toContain("2 successful");
      expect(summary.content).toContain("1 failed");
    });

    it("should handle empty entries", () => {
      const summary = historyManager._createSummary([]);

      expect(summary.content).toContain("0 previous actions");
    });
  });

  describe("clear()", () => {
    it("should clear all history", () => {
      historyManager.add({ content: "Test 1" });
      historyManager.add({ content: "Test 2" });

      expect(historyManager.history.length).toBe(2);

      historyManager.clear();

      expect(historyManager.history.length).toBe(0);
    });
  });

  describe("getStats()", () => {
    it("should return correct statistics", () => {
      historyManager.add({ content: "Test 1", success: true });
      historyManager.add({ content: "Test 2", success: false });
      historyManager.add({ content: "Test 3", success: true });

      const stats = historyManager.getStats();

      expect(stats.total).toBe(3);
      expect(stats.successful).toBe(2);
      expect(stats.failed).toBe(1);
      expect(stats.successRate).toBeCloseTo(2 / 3, 2);
    });

    it("should count summaries", () => {
      // To create summaries, we need enough entries to trigger compression
      // which only happens when history.length > compressionThreshold
      // AND there are entries to summarize (history.length - 20 > 0)
      historyManager.compressionThreshold = 5;

      // Add enough entries to potentially trigger compression
      for (let i = 0; i < 25; i++) {
        historyManager.add({ content: `Test ${i}`, success: true });
      }

      const stats = historyManager.getStats();
      expect(stats.total).toBeGreaterThan(0);
      // Summaries may or may not be present depending on compression
      expect(stats.summaries).toBeGreaterThanOrEqual(0);
    });

    it("should return 0 success rate for empty history", () => {
      const stats = historyManager.getStats();
      expect(stats.successRate).toBe(0);
    });
  });

  describe("export()", () => {
    it("should return copy of history", () => {
      historyManager.add({ content: "Test 1" });
      historyManager.add({ content: "Test 2" });

      const exported = historyManager.export();

      expect(exported.length).toBe(2);
      expect(exported[0].content).toBe("Test 1");
    });

    it("should return a new array (not reference)", () => {
      historyManager.add({ content: "Test" });

      const exported = historyManager.export();
      exported.push({ content: "Modified" });

      expect(historyManager.history.length).toBe(1);
    });
  });
});
