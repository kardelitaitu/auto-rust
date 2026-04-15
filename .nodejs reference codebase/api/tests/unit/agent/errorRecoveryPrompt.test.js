/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Unit tests for api/agent/errorRecoveryPrompt.js
 * @module tests/unit/agent/errorRecoveryPrompt.test
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

describe("api/agent/errorRecoveryPrompt.js", () => {
  let errorRecoveryPrompt;

  beforeEach(async () => {
    vi.clearAllMocks();
    const module = await import("@api/agent/errorRecoveryPrompt.js");
    errorRecoveryPrompt = module.errorRecoveryPrompt || module.default;
  });

  describe("Constructor", () => {
    it("should define error patterns", () => {
      expect(errorRecoveryPrompt.errorPatterns).toBeDefined();
      expect(
        errorRecoveryPrompt.errorPatterns.selector_not_found,
      ).toBeDefined();
      expect(
        errorRecoveryPrompt.errorPatterns.element_not_visible,
      ).toBeDefined();
      expect(errorRecoveryPrompt.errorPatterns.timeout).toBeDefined();
    });

    it("should have recovery strategies for each pattern", () => {
      for (const [_type, pattern] of Object.entries(
        errorRecoveryPrompt.errorPatterns,
      )) {
        expect(pattern.recovery).toBeDefined();
        expect(pattern.recovery.length).toBeGreaterThan(0);
      }
    });

    it("should have keywords for pattern matching", () => {
      for (const [_type, pattern] of Object.entries(
        errorRecoveryPrompt.errorPatterns,
      )) {
        expect(pattern.patterns).toBeDefined();
        expect(pattern.patterns.length).toBeGreaterThan(0);
      }
    });
  });

  describe("_classifyError()", () => {
    it("should classify selector_not_found errors", () => {
      expect(errorRecoveryPrompt._classifyError("Selector not found")).toBe(
        "selector_not_found",
      );
      expect(errorRecoveryPrompt._classifyError("Cannot find element")).toBe(
        "selector_not_found",
      );
      expect(
        errorRecoveryPrompt._classifyError("no element with selector"),
      ).toBe("selector_not_found");
    });

    it("should classify element_not_visible errors", () => {
      expect(errorRecoveryPrompt._classifyError("Element is not visible")).toBe(
        "element_not_visible",
      );
      expect(errorRecoveryPrompt._classifyError("Element is hidden")).toBe(
        "element_not_visible",
      );
      expect(errorRecoveryPrompt._classifyError("Element obscured")).toBe(
        "element_not_visible",
      );
    });

    it("should classify timeout errors", () => {
      expect(errorRecoveryPrompt._classifyError("Operation timeout")).toBe(
        "timeout",
      );
      expect(errorRecoveryPrompt._classifyError("Navigation timed out")).toBe(
        "timeout",
      );
      expect(errorRecoveryPrompt._classifyError("wait timed out")).toBe(
        "timeout",
      );
    });

    it("should classify action_failed errors", () => {
      expect(errorRecoveryPrompt._classifyError("Action failed")).toBe(
        "action_failed",
      );
      expect(errorRecoveryPrompt._classifyError("Click failed")).toBe(
        "action_failed",
      );
    });

    it("should classify verification_failed errors", () => {
      expect(errorRecoveryPrompt._classifyError("Verification failed")).toBe(
        "verification_failed",
      );
      expect(errorRecoveryPrompt._classifyError("Check failed")).toBe(
        "verification_failed",
      );
    });

    it("should classify navigation_failed errors", () => {
      expect(errorRecoveryPrompt._classifyError("Navigation failed")).toBe(
        "navigation_failed",
      );
      expect(errorRecoveryPrompt._classifyError("Goto failed")).toBe(
        "navigation_failed",
      );
    });

    it("should classify stuck errors", () => {
      expect(errorRecoveryPrompt._classifyError("Agent stuck")).toBe("stuck");
      expect(errorRecoveryPrompt._classifyError("No progress")).toBe("stuck");
    });

    it("should return unknown for unclassified errors", () => {
      expect(errorRecoveryPrompt._classifyError("Random error")).toBe(
        "unknown",
      );
    });
  });

  describe("generateRecoveryPrompt()", () => {
    it("should generate a prompt with error information", () => {
      const prompt = errorRecoveryPrompt.generateRecoveryPrompt(
        "Selector not found",
        { action: "click", selector: "#btn" },
        1,
        {},
      );

      expect(prompt).toContain("Previous Attempt Failed");
      expect(prompt).toContain("selector_not_found");
      expect(prompt).toContain("Selector not found");
    });

    it("should include recovery strategies for known error types", () => {
      const prompt = errorRecoveryPrompt.generateRecoveryPrompt(
        "Selector not found",
        {},
        1,
        {},
      );

      expect(prompt).toContain("Recovery Strategies");
      expect(prompt).toContain("selector");
    });

    it("should include general strategies for unknown error types", () => {
      const prompt = errorRecoveryPrompt.generateRecoveryPrompt(
        "Unknown error type",
        {},
        1,
        {},
      );

      expect(prompt).toContain("General Recovery Strategies");
    });

    it("should include goal reminder when context has goal", () => {
      const prompt = errorRecoveryPrompt.generateRecoveryPrompt(
        "Timeout",
        {},
        1,
        {
          goal: "Submit form",
        },
      );

      expect(prompt).toContain("Goal Reminder");
      expect(prompt).toContain("Submit form");
    });

    it("should include multiple failure warning for 3+ attempts", () => {
      const prompt = errorRecoveryPrompt.generateRecoveryPrompt(
        "Timeout",
        {},
        3,
        {},
      );

      expect(prompt).toContain("Multiple Failures Detected");
    });

    it("should not include multiple failure warning for less than 3 attempts", () => {
      const prompt = errorRecoveryPrompt.generateRecoveryPrompt(
        "Timeout",
        {},
        2,
        {},
      );

      expect(prompt).not.toContain("Multiple Failures Detected");
    });

    it("should include failed action details", () => {
      const failedAction = { action: "click", selector: "#submit" };
      const prompt = errorRecoveryPrompt.generateRecoveryPrompt(
        "Error",
        failedAction,
        1,
        {},
      );

      expect(prompt).toContain("#submit");
    });
  });

  describe("getQuickHint()", () => {
    it("should return hint for known error types", () => {
      const hint = errorRecoveryPrompt.getQuickHint("Selector not found");
      expect(hint).toContain("Hint");
      expect(hint).toContain("selector");
    });

    it("should return default hint for unknown errors", () => {
      const hint = errorRecoveryPrompt.getQuickHint("Random error");
      expect(hint).toContain("Hint");
      expect(hint).toContain("different approach");
    });
  });

  describe("getRecoveryActions()", () => {
    it("should return selector retry actions for selector_not_found", () => {
      const actions = errorRecoveryPrompt.getRecoveryActions(
        "Selector not found",
        {
          selector: "#btn",
        },
      );

      expect(actions.length).toBeGreaterThanOrEqual(2);
      expect(actions[0].action).toBe("click");
      expect(actions[1].selector).toContain("text=");
    });

    it("should return scroll action for element_not_visible", () => {
      const actions = errorRecoveryPrompt.getRecoveryActions(
        "Element not visible",
        {},
      );

      expect(actions.length).toBeGreaterThan(0);
      expect(actions[0].action).toBe("scroll");
    });

    it("should return wait action for timeout", () => {
      const actions = errorRecoveryPrompt.getRecoveryActions(
        "Timeout error",
        {},
      );

      expect(actions.length).toBeGreaterThan(0);
      expect(actions[0].action).toBe("wait");
    });

    it("should return generic wait action for unknown errors", () => {
      const actions = errorRecoveryPrompt.getRecoveryActions(
        "Unknown error",
        {},
      );

      expect(actions.length).toBeGreaterThan(0);
      expect(actions[0].action).toBe("wait");
    });

    it("should include rationale for each action", () => {
      const actions = errorRecoveryPrompt.getRecoveryActions("Timeout", {});

      for (const action of actions) {
        expect(action.rationale).toBeDefined();
      }
    });
  });

  describe("isRecoverable()", () => {
    it("should return true for recoverable errors", () => {
      expect(errorRecoveryPrompt.isRecoverable("Selector not found")).toBe(
        true,
      );
      expect(errorRecoveryPrompt.isRecoverable("Timeout")).toBe(true);
      expect(errorRecoveryPrompt.isRecoverable("Element not visible")).toBe(
        true,
      );
    });

    it("should return false for non-recoverable errors", () => {
      expect(errorRecoveryPrompt.isRecoverable("Fatal error")).toBe(false);
      expect(errorRecoveryPrompt.isRecoverable("System crash")).toBe(false);
      expect(errorRecoveryPrompt.isRecoverable("Out of memory")).toBe(false);
    });

    it("should be case insensitive", () => {
      expect(errorRecoveryPrompt.isRecoverable("FATAL ERROR")).toBe(false);
    });
  });

  describe("getMaxAttempts()", () => {
    it("should return correct max attempts for selector_not_found", () => {
      expect(errorRecoveryPrompt.getMaxAttempts("Selector not found")).toBe(3);
    });

    it("should return correct max attempts for element_not_visible", () => {
      expect(errorRecoveryPrompt.getMaxAttempts("Element not visible")).toBe(2);
    });

    it("should return correct max attempts for timeout", () => {
      expect(errorRecoveryPrompt.getMaxAttempts("Timeout")).toBe(2);
    });

    it("should return correct max attempts for action_failed", () => {
      expect(errorRecoveryPrompt.getMaxAttempts("Action failed")).toBe(3);
    });

    it("should return default 3 for unknown error types", () => {
      expect(errorRecoveryPrompt.getMaxAttempts("Unknown error")).toBe(3);
    });
  });
});
