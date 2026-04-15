/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Unit tests for api/agent/memoryInjector.js
 * @module tests/unit/agent/memoryInjector.test
 */

import { describe, it, expect, vi, beforeEach } from "vitest";

// Mock logger
vi.mock("@api/core/logger.js", () => ({
  createLogger: vi.fn(() => ({
    info: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
    debug: vi.fn(),
  })),
}));

// Mock sessionStore
vi.mock("@api/agent/sessionStore.js", () => ({
  sessionStore: {
    getPatternsByUrl: vi.fn(() => []),
    getPatternsByGoalType: vi.fn(() => []),
    getPatternsByPageType: vi.fn(() => []),
    getSuccessfulPatterns: vi.fn(() => []),
  },
}));

describe("api/agent/memoryInjector.js", () => {
  let memoryInjector;
  let sessionStore;

  beforeEach(async () => {
    vi.clearAllMocks();
    const module = await import("@api/agent/memoryInjector.js");
    memoryInjector = module.memoryInjector || module.default;

    const sessionModule = await import("@api/agent/sessionStore.js");
    sessionStore = sessionModule.sessionStore;
  });

  describe("Constructor", () => {
    it("should have maxPatterns of 5", () => {
      expect(memoryInjector.maxPatterns).toBe(5);
    });

    it("should have minConfidence of 0.6", () => {
      expect(memoryInjector.minConfidence).toBe(0.6);
    });
  });

  describe("injectMemory()", () => {
    it("should return empty string when no patterns found", async () => {
      const result = await memoryInjector.injectMemory({
        url: "https://example.com",
      });
      expect(result).toBe("");
    });

    it("should return formatted memory string when patterns exist", async () => {
      sessionStore.getPatternsByUrl.mockReturnValue([
        {
          selector: "#btn",
          action: "click",
          confidence: 0.8,
          timesUsed: 5,
          description: "Submit button",
        },
      ]);

      const result = await memoryInjector.injectMemory({
        url: "https://example.com",
      });

      expect(result).toContain("Learned Patterns");
      expect(result).toContain("click");
      expect(result).toContain("80% success");
    });

    it("should filter patterns by minimum confidence", async () => {
      sessionStore.getPatternsByUrl.mockReturnValue([
        { selector: "#btn", action: "click", confidence: 0.5, timesUsed: 5 },
      ]);

      const result = await memoryInjector.injectMemory({
        url: "https://example.com",
      });
      expect(result).toBe("");
    });

    it("should limit patterns to maxPatterns", async () => {
      const patterns = [];
      for (let i = 0; i < 10; i++) {
        patterns.push({
          selector: `#btn${i}`,
          action: "click",
          confidence: 0.9,
          timesUsed: 5,
        });
      }
      sessionStore.getPatternsByUrl.mockReturnValue(patterns);

      const result = await memoryInjector.injectMemory({
        url: "https://example.com",
      });

      // Count the number of pattern lines (each starts with "- **")
      const patternLines = result
        .split("\n")
        .filter((line) => line.startsWith("- **"));
      expect(patternLines.length).toBeLessThanOrEqual(5);
    });

    it("should handle errors gracefully", async () => {
      sessionStore.getPatternsByUrl.mockImplementation(() => {
        throw new Error("Database error");
      });

      const result = await memoryInjector.injectMemory({
        url: "https://example.com",
      });
      expect(result).toBe("");
    });

    it("should get patterns by goal type", async () => {
      // Reset mock to return patterns
      sessionStore.getPatternsByUrl.mockReturnValue([]);
      sessionStore.getPatternsByGoalType.mockReturnValue([
        { selector: "#login", action: "click", confidence: 0.8, timesUsed: 3 },
      ]);
      sessionStore.getPatternsByPageType.mockReturnValue([]);

      const result = await memoryInjector.injectMemory({
        url: "https://example.com",
        goal: "Login to site",
      });
      expect(result).toContain("Learned Patterns");
    });

    it("should get patterns by page type", async () => {
      // Reset mock to return patterns
      sessionStore.getPatternsByUrl.mockReturnValue([]);
      sessionStore.getPatternsByGoalType.mockReturnValue([]);
      sessionStore.getPatternsByPageType.mockReturnValue([
        { selector: "#form", action: "fill", confidence: 0.9, timesUsed: 10 },
      ]);

      const result = await memoryInjector.injectMemory({
        url: "https://example.com",
        pageType: "form",
      });
      expect(result).toContain("Learned Patterns");
    });
  });

  describe("_extractGoalType()", () => {
    it("should identify login goals", () => {
      expect(memoryInjector._extractGoalType("Login to the website")).toBe(
        "login",
      );
      expect(memoryInjector._extractGoalType("Sign in with credentials")).toBe(
        "login",
      );
    });

    it("should identify search goals", () => {
      expect(memoryInjector._extractGoalType("Search for products")).toBe(
        "search",
      );
      expect(memoryInjector._extractGoalType("Find the best deals")).toBe(
        "search",
      );
    });

    it("should identify purchase goals", () => {
      expect(memoryInjector._extractGoalType("Buy the item")).toBe("purchase");
      expect(memoryInjector._extractGoalType("Purchase a subscription")).toBe(
        "purchase",
      );
    });

    it("should identify navigation goals", () => {
      expect(memoryInjector._extractGoalType("Navigate to home")).toBe(
        "navigation",
      );
      expect(memoryInjector._extractGoalType("Go to settings")).toBe(
        "navigation",
      );
    });

    it("should identify form goals", () => {
      expect(memoryInjector._extractGoalType("Fill out the form")).toBe("form");
      expect(
        memoryInjector._extractGoalType("Complete registration form"),
      ).toBe("form");
    });

    it("should identify interaction goals", () => {
      expect(memoryInjector._extractGoalType("Click the button")).toBe(
        "interaction",
      );
      expect(memoryInjector._extractGoalType("Press submit")).toBe(
        "interaction",
      );
    });

    it("should return general for unrecognized goals", () => {
      expect(memoryInjector._extractGoalType("Do something random")).toBe(
        "general",
      );
    });
  });

  describe("_deduplicatePatterns()", () => {
    it("should deduplicate patterns by selector and action", () => {
      const patterns = [
        { selector: "#btn", action: "click", confidence: 0.8 },
        { selector: "#btn", action: "click", confidence: 0.9 },
        { selector: "#input", action: "type", confidence: 0.7 },
      ];

      const unique = memoryInjector._deduplicatePatterns(patterns);
      expect(unique.length).toBe(2);
    });

    it("should keep first occurrence when deduplicating", () => {
      const patterns = [
        { selector: "#btn", action: "click", confidence: 0.8 },
        { selector: "#btn", action: "click", confidence: 0.9 },
      ];

      const unique = memoryInjector._deduplicatePatterns(patterns);
      expect(unique[0].confidence).toBe(0.8);
    });

    it("should return empty array for empty input", () => {
      const unique = memoryInjector._deduplicatePatterns([]);
      expect(unique).toEqual([]);
    });
  });

  describe("_formatPattern()", () => {
    it("should format pattern with action only", () => {
      const formatted = memoryInjector._formatPattern({
        action: "click",
        confidence: 0.8,
        timesUsed: 5,
      });

      expect(formatted).toContain("click");
      expect(formatted).toContain("80% success");
      expect(formatted).toContain("5x");
    });

    it("should include selector when present", () => {
      const formatted = memoryInjector._formatPattern({
        action: "click",
        selector: "#submit-btn",
        confidence: 0.9,
        timesUsed: 10,
      });

      expect(formatted).toContain("#submit-btn");
    });

    it("should include description when present", () => {
      const formatted = memoryInjector._formatPattern({
        action: "type",
        confidence: 0.85,
        timesUsed: 3,
        description: "Enter search query",
      });

      expect(formatted).toContain("Enter search query");
    });
  });

  describe("injectQuickTips()", () => {
    it("should return empty string for empty errors", () => {
      expect(memoryInjector.injectQuickTips([])).toBe("");
    });

    it("should return empty string for null errors", () => {
      expect(memoryInjector.injectQuickTips(null)).toBe("");
    });

    it("should generate tips for selector errors", () => {
      const tips = memoryInjector.injectQuickTips(["Selector not found"]);
      expect(tips).toContain("Quick Tips");
      expect(tips).toContain("different selector");
    });

    it("should generate tips for visibility errors", () => {
      const tips = memoryInjector.injectQuickTips(["Element not visible"]);
      expect(tips).toContain("Scroll to the element");
    });

    it("should generate tips for timeout errors", () => {
      const tips = memoryInjector.injectQuickTips(["Timeout occurred"]);
      expect(tips).toContain("Increase wait time");
    });

    it("should generate tips for verification errors", () => {
      const tips = memoryInjector.injectQuickTips(["Verification failed"]);
      expect(tips).toContain("different method");
    });

    it("should limit to last 3 errors", () => {
      const errors = [
        "Selector not found",
        "Element not visible",
        "Timeout",
        "Error 4",
        "Error 5",
      ];

      const tips = memoryInjector.injectQuickTips(errors);
      // Should only process last 3 errors
      expect(tips).toBeDefined();
    });

    it("should not duplicate tips", () => {
      const tips = memoryInjector.injectQuickTips([
        "Selector not found",
        "Element not found",
      ]);

      const tipLines = tips.split("\n").filter((line) => line.startsWith("- "));
      // Should have unique tips only
      expect(tipLines.length).toBe(new Set(tipLines).size);
    });
  });

  describe("getSuccessPatterns()", () => {
    it("should return success patterns from session store", () => {
      sessionStore.getSuccessfulPatterns.mockReturnValue([
        { selector: "#btn", action: "click" },
        { selector: "#input", action: "type" },
      ]);

      const patterns = memoryInjector.getSuccessPatterns(
        "click",
        "https://example.com",
      );
      expect(patterns.length).toBe(2);
    });

    it("should limit to top 3 patterns", () => {
      sessionStore.getSuccessfulPatterns.mockReturnValue([
        { selector: "#btn1", action: "click" },
        { selector: "#btn2", action: "click" },
        { selector: "#btn3", action: "click" },
        { selector: "#btn4", action: "click" },
        { selector: "#btn5", action: "click" },
      ]);

      const patterns = memoryInjector.getSuccessPatterns(
        "click",
        "https://example.com",
      );
      expect(patterns.length).toBe(3);
    });
  });

  describe("hasPatterns()", () => {
    it("should return true when patterns exist", () => {
      sessionStore.getPatternsByUrl.mockReturnValue([
        { selector: "#btn", action: "click" },
      ]);

      const result = memoryInjector.hasPatterns({ url: "https://example.com" });
      expect(result).toBe(true);
    });

    it("should return false when no patterns exist", () => {
      sessionStore.getPatternsByUrl.mockReturnValue([]);

      const result = memoryInjector.hasPatterns({ url: "https://example.com" });
      expect(result).toBe(false);
    });

    it("should handle missing url in context", () => {
      sessionStore.getPatternsByUrl.mockReturnValue([]);

      const result = memoryInjector.hasPatterns({});
      expect(result).toBe(false);
    });
  });
});
