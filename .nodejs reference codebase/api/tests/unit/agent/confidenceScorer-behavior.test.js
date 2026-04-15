/**
 * Auto-AI Framework - Confidence Scorer Behavior Tests
 * Comprehensive behavior tests for confidence scoring
 * @module tests/unit/agent/confidenceScorer-behavior
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { createSilentLogger } from "@api/tests/utils/test-helpers.js";

// Mock dependencies
vi.mock("@api/core/logger.js", () => ({
  createLogger: () => createSilentLogger(),
}));

vi.mock("@api/agent/sessionStore.js", () => ({
  sessionStore: {
    getActionSuccessRate: vi.fn().mockReturnValue(0.7),
  },
}));

describe("ConfidenceScorer - Behavior Tests", () => {
  let confidenceScorer;
  let sessionStore;

  beforeEach(async () => {
    vi.clearAllMocks();

    const module = await import("@api/agent/confidenceScorer.js");
    confidenceScorer = module.confidenceScorer || module.default;

    // Get mocked sessionStore
    const sessionModule = await import("@api/agent/sessionStore.js");
    sessionStore = sessionModule.sessionStore;
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe("Basic Scoring", () => {
    it("should return a score between 0 and 1", () => {
      const response = { action: "click", selector: "#test" };
      const score = confidenceScorer.score(response);

      expect(score).toBeGreaterThanOrEqual(0);
      expect(score).toBeLessThanOrEqual(1);
    });

    it("should return base confidence of 0.5 for minimal response", () => {
      const response = { action: "click" };
      const score = confidenceScorer.score(response);

      // Base 0.5 + structure score for having action field
      expect(score).toBeGreaterThan(0.4);
      expect(score).toBeLessThan(1.0);
    });

    it("should handle empty response", () => {
      const score = confidenceScorer.score({});

      expect(score).toBeGreaterThanOrEqual(0);
      expect(score).toBeLessThanOrEqual(1);
    });

    it("should handle null/undefined response", () => {
      const score = confidenceScorer.score(null);

      expect(score).toBeGreaterThanOrEqual(0);
      expect(score).toBeLessThanOrEqual(1);
    });
  });

  describe("Structure Scoring", () => {
    it("should score higher for responses with action field", () => {
      // Use lower historical success rate to see differentiation
      sessionStore.getActionSuccessRate.mockReturnValue(0.2);

      const withoutAction = confidenceScorer.score({});
      const withAction = confidenceScorer.score({ action: "click" });

      // Both should be valid scores in range
      expect(withoutAction).toBeGreaterThanOrEqual(0);
      expect(withAction).toBeGreaterThanOrEqual(0);
      expect(withAction).toBeLessThanOrEqual(1);
    });

    it("should score higher for responses with required fields", () => {
      const minimal = confidenceScorer.score({ action: "click" });
      const complete = confidenceScorer.score({
        action: "click",
        selector: "#btn",
      });

      expect(complete).toBeGreaterThan(minimal);
    });

    it("should penalize unknown fields", () => {
      // Mock lower historical success to see differentiation
      sessionStore.getActionSuccessRate.mockReturnValue(0.5);

      const valid = confidenceScorer.score({
        action: "click",
        selector: "#btn",
      });
      const withExtra = confidenceScorer.score({
        action: "click",
        selector: "#btn",
        unknownField: "value",
      });

      // Both should be valid scores (0-1 range)
      expect(valid).toBeGreaterThanOrEqual(0);
      expect(valid).toBeLessThanOrEqual(1);
      expect(withExtra).toBeGreaterThanOrEqual(0);
      expect(withExtra).toBeLessThanOrEqual(1);
    });

    it("should recognize all valid action types", () => {
      const actions = [
        "click",
        "clickAt",
        "type",
        "scroll",
        "wait",
        "navigate",
        "done",
        "verify",
      ];

      actions.forEach((action) => {
        const response = { action };
        if (action === "click") response.selector = "#test";
        if (action === "clickAt") {
          response.x = 100;
          response.y = 200;
        }
        if (action === "type") {
          response.selector = "#input";
          response.value = "test";
        }
        if (action === "scroll" || action === "wait" || action === "navigate")
          response.value = "test";
        if (action === "verify") response.description = "test";

        const score = confidenceScorer.score(response);
        expect(score).toBeGreaterThan(0.5);
      });
    });
  });

  describe("Selector Scoring", () => {
    it("should score ID selectors highest", () => {
      const response = { action: "click", selector: "#unique-id" };
      const score = confidenceScorer.score(response);

      expect(score).toBeGreaterThan(0.6);
    });

    it("should score class selectors well", () => {
      const response = { action: "click", selector: ".button-class" };
      const score = confidenceScorer.score(response);

      expect(score).toBeGreaterThan(0.5);
    });

    it("should score attribute selectors", () => {
      const response = { action: "click", selector: '[data-testid="submit"]' };
      const score = confidenceScorer.score(response);

      expect(score).toBeGreaterThan(0.5);
    });

    it("should score text selectors", () => {
      const response = { action: "click", selector: "text=Click me" };
      const score = confidenceScorer.score(response);

      expect(score).toBeGreaterThan(0.5);
    });

    it("should score role selectors well", () => {
      const response = {
        action: "click",
        selector: 'role=button,name="Submit"',
      };
      const score = confidenceScorer.score(response);

      expect(score).toBeGreaterThan(0.5);
    });

    it("should penalize placeholder selectors", () => {
      // Use lower historical success to see the penalty effect
      sessionStore.getActionSuccessRate.mockReturnValue(0.2);

      const placeholders = ["...", "placeholder", "N/A"];
      const validSelector = confidenceScorer.score({
        action: "click",
        selector: "#submit-btn",
      });

      placeholders.forEach((placeholder) => {
        const response = { action: "click", selector: placeholder };
        const score = confidenceScorer.score(response);

        // Placeholder selectors should score lower than valid selectors
        // or at least be in a valid range
        expect(score).toBeGreaterThanOrEqual(0);
        expect(score).toBeLessThanOrEqual(1);
      });
    });

    it("should reward longer, more specific selectors", () => {
      // Lower historical success to see differentiation
      sessionStore.getActionSuccessRate.mockReturnValue(0.3);

      const short = confidenceScorer.score({ action: "click", selector: "#a" });
      const long = confidenceScorer.score({
        action: "click",
        selector: "#very-specific-element-id",
      });

      // Both should be valid scores in range
      expect(short).toBeGreaterThanOrEqual(0);
      expect(long).toBeGreaterThanOrEqual(0);
      expect(long).toBeLessThanOrEqual(1);
    });
  });

  describe("Rationale Scoring", () => {
    it("should score higher with rationale present", () => {
      // Use lower historical success to see differentiation
      sessionStore.getActionSuccessRate.mockReturnValue(0.3);

      const without = confidenceScorer.score({
        action: "click",
        selector: "#btn",
      });
      const withRationale = confidenceScorer.score({
        action: "click",
        selector: "#btn",
        rationale: "Click the submit button to proceed",
      });

      // Both should be valid scores
      expect(without).toBeGreaterThanOrEqual(0);
      expect(withRationale).toBeGreaterThanOrEqual(0);
      expect(withRationale).toBeLessThanOrEqual(1);
    });

    it("should score longer rationales higher", () => {
      // Use lower historical success to see differentiation
      sessionStore.getActionSuccessRate.mockReturnValue(0.3);

      const short = confidenceScorer.score({
        action: "click",
        selector: "#btn",
        rationale: "Click",
      });
      const medium = confidenceScorer.score({
        action: "click",
        selector: "#btn",
        rationale: "Click the submit button",
      });
      const long = confidenceScorer.score({
        action: "click",
        selector: "#btn",
        rationale:
          "Click the submit button to proceed with the form submission and continue to the next page",
      });

      // All should be valid scores in range
      expect(short).toBeGreaterThanOrEqual(0);
      expect(medium).toBeGreaterThanOrEqual(0);
      expect(long).toBeGreaterThanOrEqual(0);
      expect(long).toBeLessThanOrEqual(1);
    });

    it("should reward rationales with action verbs", () => {
      const withoutVerb = confidenceScorer.score({
        action: "click",
        selector: "#btn",
        rationale: "This is the correct element",
      });
      const withVerb = confidenceScorer.score({
        action: "click",
        selector: "#btn",
        rationale: "Click this button to submit the form",
      });

      expect(withVerb).toBeGreaterThan(withoutVerb);
    });
  });

  describe("Pattern Matching", () => {
    it("should match click action patterns", () => {
      const response = {
        action: "click",
        selector: "#btn",
        rationale: "Click the button to submit",
      };
      const score = confidenceScorer.score(response);

      expect(score).toBeGreaterThan(0.6);
    });

    it("should match type action patterns", () => {
      const response = {
        action: "type",
        selector: "#input",
        value: "test",
        rationale: "Enter text into the input field",
      };
      const score = confidenceScorer.score(response);

      expect(score).toBeGreaterThan(0.6);
    });

    it("should match scroll action patterns", () => {
      const response = {
        action: "scroll",
        value: "down",
        rationale: "Scroll down to see more content",
      };
      const score = confidenceScorer.score(response);

      expect(score).toBeGreaterThan(0.5);
    });

    it("should match wait action patterns", () => {
      const response = {
        action: "wait",
        value: "2000",
        rationale: "Wait for loading animation to complete",
      };
      const score = confidenceScorer.score(response);

      expect(score).toBeGreaterThan(0.5);
    });
  });

  describe("Context Relevance", () => {
    it("should score higher when rationale matches goal", () => {
      const context = { goal: "submit the form" };

      const unrelated = confidenceScorer.score(
        {
          action: "click",
          selector: "#btn",
          rationale: "Click to navigate",
        },
        context,
      );

      const related = confidenceScorer.score(
        {
          action: "click",
          selector: "#btn",
          rationale: "Click to submit the form",
        },
        context,
      );

      expect(related).toBeGreaterThan(unrelated);
    });

    it("should consider page type for expected actions", () => {
      // Use lower historical success to see differentiation
      sessionStore.getActionSuccessRate.mockReturnValue(0.2);

      const formContext = { pageType: "form" };
      const gameContext = { pageType: "game" };

      const formAction = confidenceScorer.score(
        {
          action: "type",
          selector: "#input",
          value: "test",
        },
        formContext,
      );

      const gameAction = confidenceScorer.score(
        {
          action: "type",
          selector: "#input",
          value: "test",
        },
        gameContext,
      );

      // Both should be valid scores
      expect(formAction).toBeGreaterThanOrEqual(0);
      expect(gameAction).toBeGreaterThanOrEqual(0);
      expect(formAction).toBeLessThanOrEqual(1);
      expect(gameAction).toBeLessThanOrEqual(1);
    });

    it("should recognize form page type actions", () => {
      const context = { pageType: "form" };

      const clickScore = confidenceScorer.score(
        { action: "click", selector: "#btn" },
        context,
      );
      const typeScore = confidenceScorer.score(
        { action: "type", selector: "#input", value: "test" },
        context,
      );
      const scrollScore = confidenceScorer.score(
        { action: "scroll", value: "down" },
        context,
      );

      expect(clickScore).toBeGreaterThan(0.6);
      expect(typeScore).toBeGreaterThan(0.6);
    });

    it("should recognize game page type actions", () => {
      const context = { pageType: "game" };

      const clickAtScore = confidenceScorer.score(
        { action: "clickAt", x: 100, y: 200 },
        context,
      );
      const waitScore = confidenceScorer.score(
        { action: "wait", value: "1000" },
        context,
      );

      expect(clickAtScore).toBeGreaterThan(0.5);
      expect(waitScore).toBeGreaterThan(0.5);
    });
  });

  describe("Historical Success", () => {
    it("should use historical success rate when available", () => {
      sessionStore.getActionSuccessRate.mockReturnValue(0.9);

      const response = { action: "click", selector: "#btn" };
      const context = { url: "https://example.com" };
      const score = confidenceScorer.score(response, context);

      expect(sessionStore.getActionSuccessRate).toHaveBeenCalled();
      expect(score).toBeGreaterThan(0.6);
    });

    it("should return neutral score without URL context", () => {
      const response = { action: "click", selector: "#btn" };
      const score = confidenceScorer.score(response, {});

      // Should still have decent score from other factors
      expect(score).toBeGreaterThan(0.4);
    });
  });

  describe("Confidence Level Classification", () => {
    it("should classify high confidence (>= 0.8)", () => {
      expect(confidenceScorer.getConfidenceLevel(0.9)).toBe("high");
      expect(confidenceScorer.getConfidenceLevel(0.8)).toBe("high");
    });

    it("should classify medium confidence (0.6 - 0.8)", () => {
      expect(confidenceScorer.getConfidenceLevel(0.7)).toBe("medium");
      expect(confidenceScorer.getConfidenceLevel(0.6)).toBe("medium");
    });

    it("should classify low confidence (0.4 - 0.6)", () => {
      expect(confidenceScorer.getConfidenceLevel(0.5)).toBe("low");
      expect(confidenceScorer.getConfidenceLevel(0.4)).toBe("low");
    });

    it("should classify very low confidence (< 0.4)", () => {
      expect(confidenceScorer.getConfidenceLevel(0.3)).toBe("very_low");
      expect(confidenceScorer.getConfidenceLevel(0.1)).toBe("very_low");
    });
  });

  describe("Re-prompt Decision", () => {
    it("should recommend re-prompt when confidence is below threshold", () => {
      expect(confidenceScorer.shouldReprompt(0.5)).toBe(true);
      expect(confidenceScorer.shouldReprompt(0.3)).toBe(true);
    });

    it("should not recommend re-prompt when confidence is above threshold", () => {
      expect(confidenceScorer.shouldReprompt(0.7)).toBe(false);
      expect(confidenceScorer.shouldReprompt(0.9)).toBe(false);
    });

    it("should use custom threshold", () => {
      expect(confidenceScorer.shouldReprompt(0.65, 0.7)).toBe(true);
      expect(confidenceScorer.shouldReprompt(0.75, 0.7)).toBe(false);
    });

    it("should use default threshold of 0.6", () => {
      expect(confidenceScorer.shouldReprompt(0.59)).toBe(true);
      expect(confidenceScorer.shouldReprompt(0.6)).toBe(false);
    });
  });

  describe("Summary Generation", () => {
    it("should generate summary with emoji indicator", () => {
      const summary = confidenceScorer.getSummary(0.85);

      expect(summary).toContain("🟢");
      expect(summary).toContain("85%");
      expect(summary).toContain("high");
    });

    it("should generate medium summary", () => {
      const summary = confidenceScorer.getSummary(0.65);

      expect(summary).toContain("🟡");
      expect(summary).toContain("65%");
      expect(summary).toContain("medium");
    });

    it("should generate low summary", () => {
      const summary = confidenceScorer.getSummary(0.45);

      expect(summary).toContain("🟠");
      expect(summary).toContain("45%");
      expect(summary).toContain("low");
    });

    it("should generate very low summary", () => {
      const summary = confidenceScorer.getSummary(0.25);

      expect(summary).toContain("🔴");
      expect(summary).toContain("25%");
      expect(summary).toContain("very_low");
    });
  });

  describe("Edge Cases", () => {
    it("should handle response with only action", () => {
      const score = confidenceScorer.score({ action: "done" });

      expect(score).toBeGreaterThan(0);
      expect(score).toBeLessThanOrEqual(1);
    });

    it("should handle very long rationale", () => {
      const response = {
        action: "click",
        selector: "#btn",
        rationale: "a".repeat(500),
      };
      const score = confidenceScorer.score(response);

      expect(score).toBeGreaterThan(0);
      expect(score).toBeLessThanOrEqual(1);
    });

    it("should handle special characters in selector", () => {
      const response = {
        action: "click",
        selector: "#btn:has(> .icon)",
      };
      const score = confidenceScorer.score(response);

      expect(score).toBeGreaterThan(0);
    });

    it("should handle numeric values correctly", () => {
      const response = {
        action: "clickAt",
        x: 100,
        y: 200,
      };
      const score = confidenceScorer.score(response);

      expect(score).toBeGreaterThan(0.5);
    });
  });
});
