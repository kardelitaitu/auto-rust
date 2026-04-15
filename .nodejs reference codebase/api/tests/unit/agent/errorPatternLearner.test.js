/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Unit tests for api/agent/errorPatternLearner.js
 * @module tests/unit/agent/errorPatternLearner.test
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

describe("api/agent/errorPatternLearner.js", () => {
  let errorPatternLearner;

  beforeEach(async () => {
    vi.clearAllMocks();
    vi.useFakeTimers();
    const module = await import("@api/agent/errorPatternLearner.js");
    errorPatternLearner = module.errorPatternLearner || module.default;
    errorPatternLearner.clear();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  describe("Constructor", () => {
    it("should initialize with empty patterns map", () => {
      expect(errorPatternLearner.patterns).toBeInstanceOf(Map);
      expect(errorPatternLearner.patterns.size).toBe(0);
    });

    it("should have maxPatterns of 100", () => {
      expect(errorPatternLearner.maxPatterns).toBe(100);
    });

    it("should have minOccurrences of 2", () => {
      expect(errorPatternLearner.minOccurrences).toBe(2);
    });
  });

  describe("recordError()", () => {
    it("should record a new error pattern", () => {
      const error = "Element not found: #submit-btn";
      const context = {
        url: "https://example.com",
        goal: "Submit form",
        action: "click",
        pageType: "form",
      };

      errorPatternLearner.recordError(error, context);

      expect(errorPatternLearner.patterns.size).toBe(1);
    });

    it("should increment count for repeated errors", () => {
      const error = "Timeout error";
      const context = { url: "https://example.com" };

      errorPatternLearner.recordError(error, context);
      errorPatternLearner.recordError(error, context);

      const patterns = errorPatternLearner.getAllPatterns();
      expect(patterns[0].count).toBe(2);
    });

    it("should track occurrences with timestamps", () => {
      errorPatternLearner.recordError("Test error", { action: "click" });
      errorPatternLearner.recordError("Test error", { action: "click" });

      const patterns = errorPatternLearner.getAllPatterns();
      expect(patterns[0].occurrences.length).toBe(2);
      expect(patterns[0].occurrences[0].action).toBe("click");
    });

    it("should keep only last 10 occurrences", () => {
      for (let i = 0; i < 15; i++) {
        errorPatternLearner.recordError("Test error", { action: "click" });
      }

      const patterns = errorPatternLearner.getAllPatterns();
      expect(patterns[0].occurrences.length).toBe(10);
    });

    it("should trim patterns when exceeding maxPatterns", () => {
      errorPatternLearner.maxPatterns = 3;

      // Create unique errors with different URLs to ensure unique keys
      for (let i = 0; i < 5; i++) {
        errorPatternLearner.recordError(`Unique error type ${i}`, {
          url: `https://unique${i}.com`,
          action: `action${i}`,
        });
      }

      // Should have trimmed to ~3 (60% of 5 = 3 after removing 20%)
      expect(errorPatternLearner.patterns.size).toBeLessThanOrEqual(4);
    });

    it("should update lastSeen on repeated errors", () => {
      errorPatternLearner.recordError("Test error", {});
      const firstPattern = errorPatternLearner.getAllPatterns()[0];
      const firstSeen = firstPattern.lastSeen;

      vi.advanceTimersByTime(1000);
      errorPatternLearner.recordError("Test error", {});
      const updatedPattern = errorPatternLearner.getAllPatterns()[0];

      expect(updatedPattern.lastSeen).toBeGreaterThan(firstSeen);
    });
  });

  describe("_classifyError()", () => {
    it("should classify selector errors", () => {
      expect(errorPatternLearner._classifyError("Selector not found")).toBe(
        "selector_not_found",
      );
      expect(errorPatternLearner._classifyError("Element not found")).toBe(
        "selector_not_found",
      );
    });

    it("should classify visibility errors", () => {
      expect(errorPatternLearner._classifyError("Element is not visible")).toBe(
        "element_not_visible",
      );
      expect(errorPatternLearner._classifyError("Element is hidden")).toBe(
        "element_not_visible",
      );
    });

    it("should classify timeout errors", () => {
      expect(errorPatternLearner._classifyError("Operation timeout")).toBe(
        "timeout",
      );
      expect(errorPatternLearner._classifyError("Navigation timed out")).toBe(
        "timeout",
      );
    });

    it("should classify verification errors", () => {
      expect(errorPatternLearner._classifyError("Verification failed")).toBe(
        "verification_failed",
      );
      expect(
        errorPatternLearner._classifyError("Failed to verify element"),
      ).toBe("verification_failed");
    });

    it("should return unknown for unclassified errors", () => {
      expect(errorPatternLearner._classifyError("Random error")).toBe(
        "unknown",
      );
    });
  });

  describe("_normalizeUrl()", () => {
    it("should normalize valid URLs", () => {
      const normalized = errorPatternLearner._normalizeUrl(
        "https://example.com/path?query=1",
      );
      expect(normalized).toBe("example.com/path");
    });

    it("should handle invalid URLs by returning lowercase", () => {
      const result = errorPatternLearner._normalizeUrl("NOT-A-URL");
      expect(result).toBe("not-a-url");
    });
  });

  describe("_stringSimilarity()", () => {
    it("should return 1 for identical strings", () => {
      expect(
        errorPatternLearner._stringSimilarity("hello world", "hello world"),
      ).toBe(1);
    });

    it("should be case insensitive", () => {
      expect(
        errorPatternLearner._stringSimilarity("Hello World", "hello world"),
      ).toBe(1);
    });

    it("should calculate similarity based on word overlap", () => {
      const similarity = errorPatternLearner._stringSimilarity(
        "click submit button",
        "click send button",
      );
      expect(similarity).toBeGreaterThan(0);
      expect(similarity).toBeLessThan(1);
    });

    it("should return 0 for completely different strings", () => {
      const similarity = errorPatternLearner._stringSimilarity("abc", "xyz");
      expect(similarity).toBe(0);
    });
  });

  describe("getWarning()", () => {
    it("should return null when no relevant patterns", () => {
      const result = errorPatternLearner.getWarning({
        url: "https://example.com",
      });
      expect(result).toBeNull();
    });

    it("should return warning when pattern has enough occurrences", () => {
      errorPatternLearner.recordError("Test error", {
        url: "https://example.com",
      });
      errorPatternLearner.recordError("Test error", {
        url: "https://example.com",
      });

      const warning = errorPatternLearner.getWarning({
        url: "https://example.com",
      });
      expect(warning).toContain("Warning");
      expect(warning).toContain("2 times");
    });

    it("should return null when pattern has insufficient occurrences", () => {
      errorPatternLearner.recordError("Test error", {
        url: "https://example.com",
      });

      const warning = errorPatternLearner.getWarning({
        url: "https://example.com",
      });
      expect(warning).toBeNull();
    });
  });

  describe("getPreventionStrategies()", () => {
    it("should return empty array when no relevant patterns", () => {
      const strategies = errorPatternLearner.getPreventionStrategies({
        url: "https://example.com",
      });
      expect(strategies).toEqual([]);
    });

    it("should return strategies for known error types", () => {
      errorPatternLearner.recordError("Selector not found", {
        url: "https://example.com",
      });
      errorPatternLearner.recordError("Selector not found", {
        url: "https://example.com",
      });

      const strategies = errorPatternLearner.getPreventionStrategies({
        url: "https://example.com",
      });
      expect(strategies.length).toBeGreaterThan(0);
      expect(strategies[0].strategy).toContain("selector");
    });

    it("should generate different strategies for different error types", () => {
      errorPatternLearner.recordError("Element not visible", {
        url: "https://example.com",
      });
      errorPatternLearner.recordError("Element not visible", {
        url: "https://example.com",
      });

      const strategies = errorPatternLearner.getPreventionStrategies({
        url: "https://example.com",
      });
      expect(strategies[0].strategy).toContain("Scroll");
    });

    it("should limit strategies to top 3", () => {
      for (let i = 0; i < 5; i++) {
        errorPatternLearner.recordError(`Error type ${i}`, {
          url: "https://example.com",
          action: `action${i}`,
        });
        errorPatternLearner.recordError(`Error type ${i}`, {
          url: "https://example.com",
          action: `action${i}`,
        });
      }

      const strategies = errorPatternLearner.getPreventionStrategies({
        url: "https://example.com",
      });
      expect(strategies.length).toBeLessThanOrEqual(3);
    });
  });

  describe("_calculateRelevance()", () => {
    it("should give high relevance for same URL", () => {
      const pattern = {
        context: { url: "https://example.com", goal: null, action: null },
        count: 5,
      };
      const context = { url: "https://example.com" };

      const relevance = errorPatternLearner._calculateRelevance(
        pattern,
        context,
      );
      expect(relevance).toBeGreaterThanOrEqual(0.5);
    });

    it("should give partial relevance for related URLs", () => {
      const pattern = {
        context: { url: "https://example.com/page1", goal: null, action: null },
        count: 5,
      };
      const context = { url: "https://example.com" };

      const relevance = errorPatternLearner._calculateRelevance(
        pattern,
        context,
      );
      expect(relevance).toBeGreaterThan(0);
      expect(relevance).toBeLessThan(0.5);
    });

    it("should add relevance for same action", () => {
      const pattern = {
        context: { url: null, goal: null, action: "click" },
        count: 5,
      };
      const context = { action: "click" };

      const relevance = errorPatternLearner._calculateRelevance(
        pattern,
        context,
      );
      expect(relevance).toBe(0.2);
    });

    it("should cap relevance at 1", () => {
      const pattern = {
        context: { url: "https://example.com", goal: "test", action: "click" },
        count: 5,
      };
      const context = {
        url: "https://example.com",
        goal: "test",
        action: "click",
      };

      const relevance = errorPatternLearner._calculateRelevance(
        pattern,
        context,
      );
      expect(relevance).toBeLessThanOrEqual(1);
    });
  });

  describe("getStats()", () => {
    it("should return stats with totalPatterns", () => {
      errorPatternLearner.recordError("Error 1", { url: "https://a.com" });
      errorPatternLearner.recordError("Error 2", { url: "https://b.com" });

      const stats = errorPatternLearner.getStats();
      expect(stats.totalPatterns).toBe(2);
    });

    it("should categorize patterns by error type", () => {
      errorPatternLearner.recordError("Selector not found", {});
      errorPatternLearner.recordError("Timeout error", {});

      const stats = errorPatternLearner.getStats();
      expect(stats.byType["selector_not_found"]).toBe(1);
      expect(stats.byType["timeout"]).toBe(1);
    });

    it("should include minOccurrences", () => {
      const stats = errorPatternLearner.getStats();
      expect(stats.minOccurrences).toBe(2);
    });
  });

  describe("getAllPatterns()", () => {
    it("should return empty array when no patterns", () => {
      const patterns = errorPatternLearner.getAllPatterns();
      expect(patterns).toEqual([]);
    });

    it("should return all patterns as array", () => {
      errorPatternLearner.recordError("Error 1", { url: "https://a.com" });
      errorPatternLearner.recordError("Error 2", { url: "https://b.com" });

      const patterns = errorPatternLearner.getAllPatterns();
      expect(patterns.length).toBe(2);
    });
  });

  describe("clear()", () => {
    it("should clear all patterns", () => {
      errorPatternLearner.recordError("Test error", {});
      expect(errorPatternLearner.patterns.size).toBe(1);

      errorPatternLearner.clear();
      expect(errorPatternLearner.patterns.size).toBe(0);
    });
  });

  describe("export() and import()", () => {
    it("should export patterns as serializable array", () => {
      errorPatternLearner.recordError("Test error", {
        url: "https://example.com",
      });

      const exported = errorPatternLearner.export();
      expect(Array.isArray(exported)).toBe(true);
      expect(exported[0]).toHaveProperty("key");
      expect(exported[0]).toHaveProperty("errorType");
      expect(exported[0]).toHaveProperty("count");
    });

    it("should import patterns from exported data", () => {
      errorPatternLearner.recordError("Test error", {
        url: "https://example.com",
      });
      const exported = errorPatternLearner.export();

      errorPatternLearner.clear();
      expect(errorPatternLearner.patterns.size).toBe(0);

      errorPatternLearner.import(exported);
      expect(errorPatternLearner.patterns.size).toBe(1);
    });

    it("should restore pattern data on import", () => {
      errorPatternLearner.recordError("Test error", {
        url: "https://example.com",
        goal: "Test",
      });
      errorPatternLearner.recordError("Test error", {
        url: "https://example.com",
        goal: "Test",
      });
      const exported = errorPatternLearner.export();

      errorPatternLearner.clear();
      errorPatternLearner.import(exported);

      const patterns = errorPatternLearner.getAllPatterns();
      expect(patterns[0].count).toBe(2);
      expect(patterns[0].context.url).toBe("https://example.com");
    });
  });

  describe("_getKey()", () => {
    it("should generate key from error type, URL, and action", () => {
      const key = errorPatternLearner._getKey("Selector not found", {
        url: "https://example.com/path",
        action: "click",
      });
      expect(key).toBe("selector_not_found|example.com/path|click");
    });

    it("should use unknown for missing context", () => {
      const key = errorPatternLearner._getKey("Some error", {});
      expect(key).toContain("unknown");
    });
  });

  describe("_trimPatterns()", () => {
    it("should remove oldest 20% of patterns", () => {
      errorPatternLearner.maxPatterns = 10;

      for (let i = 0; i < 10; i++) {
        vi.advanceTimersByTime(100);
        errorPatternLearner.recordError(`Error ${i}`, {
          url: `https://example${i}.com`,
        });
      }

      // Add one more to trigger trim
      errorPatternLearner.recordError("Error 10", {
        url: "https://example10.com",
      });

      // Should have removed ~20% (2 patterns)
      expect(errorPatternLearner.patterns.size).toBeLessThanOrEqual(9);
    });
  });
});
