/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Unit tests for api/agent/selfHealingPrompt.js
 * @module tests/unit/agent/selfHealingPrompt.test
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

describe("api/agent/selfHealingPrompt.js", () => {
  let selfHealingPrompt;

  beforeEach(async () => {
    vi.clearAllMocks();
    vi.useFakeTimers();
    const module = await import("@api/agent/selfHealingPrompt.js");
    selfHealingPrompt = module.selfHealingPrompt || module.default;
    selfHealingPrompt.clear();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  describe("Constructor", () => {
    it("should initialize with empty recentFailures", () => {
      expect(selfHealingPrompt.recentFailures).toEqual([]);
    });

    it("should have maxFailures of 10", () => {
      expect(selfHealingPrompt.maxFailures).toBe(10);
    });
  });

  describe("recordFailure()", () => {
    it("should add failure to recentFailures", () => {
      const failure = {
        action: { action: "click", selector: "#btn" },
        error: "Selector not found",
      };

      selfHealingPrompt.recordFailure(failure);

      expect(selfHealingPrompt.recentFailures.length).toBe(1);
    });

    it("should add timestamp if not provided", () => {
      selfHealingPrompt.recordFailure({
        action: { action: "click" },
        error: "Error",
      });

      expect(selfHealingPrompt.recentFailures[0].timestamp).toBeDefined();
    });

    it("should preserve provided timestamp", () => {
      const timestamp = 1234567890;
      selfHealingPrompt.recordFailure({
        action: { action: "click" },
        error: "Error",
        timestamp,
      });

      expect(selfHealingPrompt.recentFailures[0].timestamp).toBe(timestamp);
    });

    it("should keep only maxFailures most recent failures", () => {
      for (let i = 0; i < 15; i++) {
        selfHealingPrompt.recordFailure({
          action: { action: "click" },
          error: `Error ${i}`,
        });
      }

      expect(selfHealingPrompt.recentFailures.length).toBe(10);
    });

    it("should keep the most recent failures when trimming", () => {
      for (let i = 0; i < 12; i++) {
        selfHealingPrompt.recordFailure({
          action: { action: "click" },
          error: `Error ${i}`,
        });
      }

      expect(selfHealingPrompt.recentFailures[0].error).toBe("Error 2");
    });
  });

  describe("generateHealingInstructions()", () => {
    it("should return empty string when no failures", () => {
      const result = selfHealingPrompt.generateHealingInstructions({});
      expect(result).toBe("");
    });

    it("should return empty string when no relevant failures", () => {
      selfHealingPrompt.recordFailure({
        action: { action: "click" },
        error: "Error",
        context: { url: "https://old.com" },
      });

      vi.advanceTimersByTime(600000);

      const result = selfHealingPrompt.generateHealingInstructions({
        url: "https://different.com",
        goal: "Different goal",
      });

      expect(result).toBe("");
    });

    it("should include recent failures", () => {
      selfHealingPrompt.recordFailure({
        action: { action: "click", selector: "#btn" },
        error: "Selector not found",
        context: { url: "https://example.com" },
      });

      const result = selfHealingPrompt.generateHealingInstructions({
        url: "https://example.com",
      });

      expect(result).toContain("Recent Failures to Avoid");
      expect(result).toContain("Selector not found");
    });

    it("should include general advice", () => {
      selfHealingPrompt.recordFailure({
        action: { action: "click" },
        error: "Selector not found",
        context: {},
      });

      const result = selfHealingPrompt.generateHealingInstructions({});

      expect(result).toContain("General Advice");
    });

    it("should limit to last 3 relevant failures", () => {
      for (let i = 0; i < 5; i++) {
        selfHealingPrompt.recordFailure({
          action: { action: "click" },
          error: `Error ${i}`,
          context: { url: "https://example.com" },
        });
      }

      const result = selfHealingPrompt.generateHealingInstructions({
        url: "https://example.com",
      });

      const failureMatches = result.match(/\*\*Action\*\*:/g);
      expect(failureMatches.length).toBeLessThanOrEqual(3);
    });
  });

  describe("_getRelevantFailures()", () => {
    it("should include failures with same URL", () => {
      selfHealingPrompt.recordFailure({
        action: { action: "click" },
        error: "Error",
        context: { url: "https://example.com" },
      });

      const relevant = selfHealingPrompt._getRelevantFailures({
        url: "https://example.com",
      });

      expect(relevant.length).toBe(1);
    });

    it("should include failures with similar goal", () => {
      selfHealingPrompt.recordFailure({
        action: { action: "click" },
        error: "Error",
        context: { goal: "Login to website" },
      });

      const relevant = selfHealingPrompt._getRelevantFailures({
        goal: "Login with credentials",
      });

      expect(relevant.length).toBe(1);
    });

    it("should include recent failures (within 5 minutes)", () => {
      selfHealingPrompt.recordFailure({
        action: { action: "click" },
        error: "Error",
        context: {},
      });

      const relevant = selfHealingPrompt._getRelevantFailures({});

      expect(relevant.length).toBe(1);
    });

    it("should exclude old failures with different URL and goal", () => {
      selfHealingPrompt.recordFailure({
        action: { action: "click" },
        error: "Error",
        context: { url: "https://old.com", goal: "Old goal" },
      });

      vi.advanceTimersByTime(600000);

      const relevant = selfHealingPrompt._getRelevantFailures({
        url: "https://different.com",
        goal: "Different goal",
      });

      expect(relevant.length).toBe(0);
    });
  });

  describe("_formatFailure()", () => {
    it("should format failure with action and error", () => {
      const formatted = selfHealingPrompt._formatFailure({
        action: { action: "click" },
        error: "Test error",
      });

      expect(formatted).toContain("click");
      expect(formatted).toContain("Test error");
    });

    it("should include alternative when present", () => {
      const formatted = selfHealingPrompt._formatFailure({
        action: { action: "click" },
        error: "Error",
        alternative: "Try different selector",
      });

      expect(formatted).toContain("Alternative");
      expect(formatted).toContain("Try different selector");
    });

    it("should handle missing action", () => {
      const formatted = selfHealingPrompt._formatFailure({
        error: "Error",
      });

      expect(formatted).toContain("unknown");
    });
  });

  describe("_classifyError()", () => {
    it("should classify selector errors", () => {
      expect(selfHealingPrompt._classifyError("Selector not found")).toBe(
        "selector_not_found",
      );
      expect(selfHealingPrompt._classifyError("Element not found")).toBe(
        "selector_not_found",
      );
    });

    it("should classify visibility errors", () => {
      expect(selfHealingPrompt._classifyError("Not visible")).toBe(
        "element_not_visible",
      );
      expect(selfHealingPrompt._classifyError("Element hidden")).toBe(
        "element_not_visible",
      );
    });

    it("should classify timeout errors", () => {
      expect(selfHealingPrompt._classifyError("Timeout")).toBe("timeout");
      expect(selfHealingPrompt._classifyError("Timed out")).toBe("timeout");
    });

    it("should return unknown for other errors", () => {
      expect(selfHealingPrompt._classifyError("Random error")).toBe("unknown");
    });

    it("should return unknown for null/undefined", () => {
      expect(selfHealingPrompt._classifyError(null)).toBe("unknown");
      expect(selfHealingPrompt._classifyError(undefined)).toBe("unknown");
    });
  });

  describe("_normalizeUrl()", () => {
    it("should normalize valid URLs", () => {
      const normalized = selfHealingPrompt._normalizeUrl(
        "https://example.com/path?q=1",
      );
      expect(normalized).toBe("example.com/path");
    });

    it("should handle invalid URLs", () => {
      const result = selfHealingPrompt._normalizeUrl("NOT-A-URL");
      expect(result).toBe("not-a-url");
    });
  });

  describe("_stringSimilarity()", () => {
    it("should return 1 for identical strings", () => {
      expect(selfHealingPrompt._stringSimilarity("test", "test")).toBe(1);
    });

    it("should be case insensitive", () => {
      expect(selfHealingPrompt._stringSimilarity("Test", "test")).toBe(1);
    });

    it("should calculate similarity based on word overlap", () => {
      const similarity = selfHealingPrompt._stringSimilarity(
        "click submit button",
        "click send button",
      );
      expect(similarity).toBeGreaterThan(0);
      expect(similarity).toBeLessThan(1);
    });

    it("should return 0 for completely different strings", () => {
      expect(selfHealingPrompt._stringSimilarity("abc", "xyz")).toBe(0);
    });
  });

  describe("generateRecoveryPrompt()", () => {
    it("should generate recovery prompt with error", () => {
      const prompt = selfHealingPrompt.generateRecoveryPrompt({
        error: "Selector not found",
      });

      expect(prompt).toContain("Recovery Mode");
      expect(prompt).toContain("Selector not found");
    });

    it("should include recovery options", () => {
      const prompt = selfHealingPrompt.generateRecoveryPrompt({
        error: "Error",
      });

      expect(prompt).toContain("Recovery Options");
      expect(prompt).toContain("different selector");
      expect(prompt).toContain("Wait and retry");
    });

    it("should include alternative when present", () => {
      const prompt = selfHealingPrompt.generateRecoveryPrompt({
        error: "Error",
        alternative: "Try using text selector",
      });

      expect(prompt).toContain("Suggested Alternative");
      expect(prompt).toContain("Try using text selector");
    });

    it("should not include alternative section when not present", () => {
      const prompt = selfHealingPrompt.generateRecoveryPrompt({
        error: "Error",
      });

      expect(prompt).not.toContain("Suggested Alternative");
    });
  });

  describe("isInRecoveryMode()", () => {
    it("should return false when no recent failures", () => {
      expect(selfHealingPrompt.isInRecoveryMode()).toBe(false);
    });

    it("should return false when less than 2 failures in last minute", () => {
      selfHealingPrompt.recordFailure({
        action: { action: "click" },
        error: "Error 1",
      });

      expect(selfHealingPrompt.isInRecoveryMode()).toBe(false);
    });

    it("should return true when 2+ failures in last minute", () => {
      selfHealingPrompt.recordFailure({
        action: { action: "click" },
        error: "Error 1",
      });
      selfHealingPrompt.recordFailure({
        action: { action: "click" },
        error: "Error 2",
      });

      expect(selfHealingPrompt.isInRecoveryMode()).toBe(true);
    });

    it("should return false when failures are older than 1 minute", () => {
      selfHealingPrompt.recordFailure({
        action: { action: "click" },
        error: "Error 1",
        timestamp: Date.now() - 70000,
      });
      selfHealingPrompt.recordFailure({
        action: { action: "click" },
        error: "Error 2",
        timestamp: Date.now() - 65000,
      });

      expect(selfHealingPrompt.isInRecoveryMode()).toBe(false);
    });
  });

  describe("getStats()", () => {
    it("should return statistics", () => {
      selfHealingPrompt.recordFailure({
        action: { action: "click" },
        error: "Selector not found",
      });
      selfHealingPrompt.recordFailure({
        action: { action: "type" },
        error: "Timeout",
      });

      const stats = selfHealingPrompt.getStats();

      expect(stats.totalFailures).toBe(2);
      expect(stats.errorTypes.selector_not_found).toBe(1);
      expect(stats.errorTypes.timeout).toBe(1);
    });

    it("should include inRecoveryMode status", () => {
      const stats = selfHealingPrompt.getStats();
      expect(stats.inRecoveryMode).toBeDefined();
    });
  });

  describe("clear()", () => {
    it("should clear all failures", () => {
      selfHealingPrompt.recordFailure({
        action: { action: "click" },
        error: "Error",
      });

      expect(selfHealingPrompt.recentFailures.length).toBe(1);

      selfHealingPrompt.clear();

      expect(selfHealingPrompt.recentFailures.length).toBe(0);
    });
  });

  describe("getRecentFailures()", () => {
    it("should return recent failures", () => {
      for (let i = 0; i < 5; i++) {
        selfHealingPrompt.recordFailure({
          action: { action: "click" },
          error: `Error ${i}`,
        });
      }

      const failures = selfHealingPrompt.getRecentFailures(3);
      expect(failures.length).toBe(3);
      expect(failures[0].error).toBe("Error 2");
      expect(failures[1].error).toBe("Error 3");
      expect(failures[2].error).toBe("Error 4");
    });

    it("should default to returning 5 failures", () => {
      for (let i = 0; i < 10; i++) {
        selfHealingPrompt.recordFailure({
          action: { action: "click" },
          error: `Error ${i}`,
        });
      }

      const failures = selfHealingPrompt.getRecentFailures();
      expect(failures.length).toBe(5);
    });

    it("should return all failures if less than count", () => {
      selfHealingPrompt.recordFailure({
        action: { action: "click" },
        error: "Error 1",
      });

      const failures = selfHealingPrompt.getRecentFailures(5);
      expect(failures.length).toBe(1);
    });
  });
});
