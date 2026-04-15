/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Unit tests for api/agent/goalDecomposer.js
 * @module tests/unit/agent/goalDecomposer.test
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

describe("api/agent/goalDecomposer.js", () => {
  let goalDecomposer;

  beforeEach(async () => {
    vi.clearAllMocks();
    const module = await import("@api/agent/goalDecomposer.js");
    goalDecomposer = module.goalDecomposer || module.default;
  });

  describe("Constructor", () => {
    it("should initialize with goal patterns", () => {
      expect(goalDecomposer.goalPatterns).toBeDefined();
      expect(goalDecomposer.goalPatterns.login).toBeDefined();
      expect(goalDecomposer.goalPatterns.purchase).toBeDefined();
      expect(goalDecomposer.goalPatterns.post).toBeDefined();
      expect(goalDecomposer.goalPatterns.search).toBeDefined();
      expect(goalDecomposer.goalPatterns.register).toBeDefined();
    });

    it("should have keywords for each pattern", () => {
      expect(goalDecomposer.goalPatterns.login.keywords).toContain("login");
      expect(goalDecomposer.goalPatterns.purchase.keywords).toContain("buy");
      expect(goalDecomposer.goalPatterns.post.keywords).toContain("tweet");
    });

    it("should have steps for each pattern", () => {
      expect(goalDecomposer.goalPatterns.login.steps.length).toBeGreaterThan(0);
      expect(goalDecomposer.goalPatterns.purchase.steps.length).toBeGreaterThan(
        0,
      );
    });
  });

  describe("decompose()", () => {
    it("should decompose login goal", async () => {
      const result = await goalDecomposer.decompose(
        "login to my account",
        "https://example.com",
      );
      expect(result.pattern).toBe("login");
      expect(result.steps.length).toBeGreaterThan(0);
      expect(result.currentStep).toBe(0);
    });

    it("should decompose purchase goal", async () => {
      const result = await goalDecomposer.decompose(
        "buy a product",
        "https://shop.example.com",
      );
      expect(result.pattern).toBe("purchase");
      expect(result.totalSteps).toBeGreaterThan(0);
    });

    it("should decompose post goal", async () => {
      const result = await goalDecomposer.decompose(
        "post a tweet",
        "https://x.com",
      );
      expect(result.pattern).toBe("post");
    });

    it("should decompose search goal", async () => {
      const result = await goalDecomposer.decompose(
        "find information about AI",
        "https://google.com",
      );
      expect(result.pattern).toBe("search");
    });

    it("should decompose register goal", async () => {
      const result = await goalDecomposer.decompose(
        "sign up for an account",
        "https://example.com/register",
      );
      expect(result.pattern).toBe("register");
    });

    it("should use heuristic for unknown goals", async () => {
      const result = await goalDecomposer.decompose(
        "do something random",
        "https://example.com",
      );
      expect(result.pattern).toBe("heuristic");
      expect(result.steps.length).toBeGreaterThan(0);
    });

    it("should detect action words in heuristic", async () => {
      const result = await goalDecomposer.decompose(
        "click on the button",
        "https://example.com",
      );
      expect(result.pattern).toBe("heuristic");
      expect(result.steps.some((s) => s.subGoal.includes("Click"))).toBe(true);
    });

    it("should detect multiple action words", async () => {
      const result = await goalDecomposer.decompose(
        "click and type something",
        "https://example.com",
      );
      expect(result.steps.length).toBeGreaterThanOrEqual(2);
    });

    it("should use case-insensitive matching", async () => {
      const result = await goalDecomposer.decompose(
        "LOGIN to my account",
        "https://example.com",
      );
      expect(result.pattern).toBe("login");
    });
  });

  describe("_matchPattern()", () => {
    it("should return null for no match", () => {
      const result = goalDecomposer._matchPattern("do something random");
      expect(result).toBeNull();
    });

    it("should match exact keywords", () => {
      const result = goalDecomposer._matchPattern("login");
      expect(result.name).toBe("login");
    });

    it("should match partial keywords", () => {
      const result = goalDecomposer._matchPattern("user wants to sign in now");
      expect(result.name).toBe("login");
    });
  });

  describe("_heuristicDecompose()", () => {
    it("should create generic steps for no action words", () => {
      const result = goalDecomposer._heuristicDecompose(
        "do something",
        "https://example.com",
      );
      expect(result.pattern).toBe("heuristic");
      expect(result.steps.length).toBe(4);
      expect(result.steps[0].subGoal).toContain("Analyze");
    });

    it("should detect click action", () => {
      const result = goalDecomposer._heuristicDecompose(
        "click the button",
        "https://example.com",
      );
      expect(result.steps.some((s) => s.subGoal.includes("Click"))).toBe(true);
    });

    it("should detect navigate action", () => {
      const result = goalDecomposer._heuristicDecompose(
        "navigate to home page",
        "https://example.com",
      );
      expect(result.steps.some((s) => s.subGoal.includes("Navigate"))).toBe(
        true,
      );
    });

    it("should detect scroll action", () => {
      const result = goalDecomposer._heuristicDecompose(
        "scroll down to find more",
        "https://example.com",
      );
      expect(result.steps.some((s) => s.subGoal.includes("Scroll"))).toBe(true);
    });
  });

  describe("getNextStep()", () => {
    it("should return next step from decomposition", () => {
      const decomposition = {
        pattern: "login",
        steps: [
          { subGoal: "Step 1", expectedOutcome: "Done 1" },
          { subGoal: "Step 2", expectedOutcome: "Done 2" },
        ],
        currentStep: 0,
        totalSteps: 2,
      };

      const next = goalDecomposer.getNextStep(decomposition);
      expect(next.subGoal).toBe("Step 1");
      expect(next.stepNumber).toBe(1);
      expect(next.totalSteps).toBe(2);
    });

    it("should return null when all steps completed", () => {
      const decomposition = {
        pattern: "login",
        steps: [{ subGoal: "Step 1", expectedOutcome: "Done 1" }],
        currentStep: 1,
        totalSteps: 1,
      };

      const next = goalDecomposer.getNextStep(decomposition);
      expect(next).toBeNull();
    });
  });

  describe("advanceStep()", () => {
    it("should increment current step", () => {
      const decomposition = {
        pattern: "login",
        steps: [],
        currentStep: 0,
        totalSteps: 5,
      };

      const updated = goalDecomposer.advanceStep(decomposition);
      expect(updated.currentStep).toBe(1);
    });

    it("should not mutate original decomposition", () => {
      const decomposition = {
        pattern: "login",
        steps: [],
        currentStep: 0,
        totalSteps: 5,
      };

      goalDecomposer.advanceStep(decomposition);
      expect(decomposition.currentStep).toBe(0);
    });
  });

  describe("isComplete()", () => {
    it("should return false when steps remain", () => {
      const decomposition = {
        currentStep: 1,
        totalSteps: 3,
      };

      expect(goalDecomposer.isComplete(decomposition)).toBe(false);
    });

    it("should return true when all steps done", () => {
      const decomposition = {
        currentStep: 3,
        totalSteps: 3,
      };

      expect(goalDecomposer.isComplete(decomposition)).toBe(true);
    });

    it("should return true when currentStep exceeds totalSteps", () => {
      const decomposition = {
        currentStep: 5,
        totalSteps: 3,
      };

      expect(goalDecomposer.isComplete(decomposition)).toBe(true);
    });
  });

  describe("getProgress()", () => {
    it("should calculate progress percentage", () => {
      const decomposition = {
        currentStep: 1,
        totalSteps: 4,
      };

      expect(goalDecomposer.getProgress(decomposition)).toBe(25);
    });

    it("should return 100 for zero total steps", () => {
      const decomposition = {
        currentStep: 0,
        totalSteps: 0,
      };

      expect(goalDecomposer.getProgress(decomposition)).toBe(100);
    });

    it("should return 0 for no progress", () => {
      const decomposition = {
        currentStep: 0,
        totalSteps: 5,
      };

      expect(goalDecomposer.getProgress(decomposition)).toBe(0);
    });

    it("should return 100 for complete progress", () => {
      const decomposition = {
        currentStep: 5,
        totalSteps: 5,
      };

      expect(goalDecomposer.getProgress(decomposition)).toBe(100);
    });

    it("should round percentage", () => {
      const decomposition = {
        currentStep: 1,
        totalSteps: 3,
      };

      // 1/3 = 33.33... should round to 33
      expect(goalDecomposer.getProgress(decomposition)).toBe(33);
    });
  });

  describe("Edge Cases", () => {
    it("should handle goal with multiple matching keywords", () => {
      // 'sign in' matches login, but should return first match
      const result = goalDecomposer._matchPattern("sign in to login");
      expect(result).toBeDefined();
    });

    it("should handle empty goal", async () => {
      const result = await goalDecomposer.decompose("", "https://example.com");
      expect(result.pattern).toBe("heuristic");
    });

    it("should handle goal with only whitespace", async () => {
      const result = await goalDecomposer.decompose(
        "   ",
        "https://example.com",
      );
      expect(result.pattern).toBe("heuristic");
    });
  });
});
