/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Unit tests for api/agent/parallelExecutor.js
 * @module tests/unit/agent/parallelExecutor.test
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

describe("api/agent/parallelExecutor.js", () => {
  let parallelExecutor;

  beforeEach(async () => {
    vi.clearAllMocks();
    const module = await import("@api/agent/parallelExecutor.js");
    parallelExecutor = module.parallelExecutor || module.default;
  });

  describe("Constructor", () => {
    it("should define independent actions", () => {
      expect(parallelExecutor.independentActions.has("wait")).toBe(true);
      expect(parallelExecutor.independentActions.has("screenshot")).toBe(true);
      expect(parallelExecutor.independentActions.has("verify")).toBe(true);
    });

    it("should define dependent actions", () => {
      expect(parallelExecutor.dependentActions.has("click")).toBe(true);
      expect(parallelExecutor.dependentActions.has("type")).toBe(true);
      expect(parallelExecutor.dependentActions.has("navigate")).toBe(true);
      expect(parallelExecutor.dependentActions.has("scroll")).toBe(true);
    });
  });

  describe("_isIndependent()", () => {
    it("should return true for independent actions", () => {
      expect(parallelExecutor._isIndependent({ action: "wait" })).toBe(true);
      expect(parallelExecutor._isIndependent({ action: "screenshot" })).toBe(
        true,
      );
      expect(parallelExecutor._isIndependent({ action: "verify" })).toBe(true);
    });

    it("should return false for dependent actions", () => {
      expect(parallelExecutor._isIndependent({ action: "click" })).toBe(false);
      expect(parallelExecutor._isIndependent({ action: "type" })).toBe(false);
      expect(parallelExecutor._isIndependent({ action: "navigate" })).toBe(
        false,
      );
    });
  });

  describe("_categorizeActions()", () => {
    it("should separate actions into independent and dependent", () => {
      const actions = [
        { action: "wait" },
        { action: "click", selector: "#btn" },
        { action: "screenshot" },
        { action: "type", selector: "#input", value: "test" },
      ];

      const { independent, dependent } =
        parallelExecutor._categorizeActions(actions);

      expect(independent.length).toBe(2);
      expect(dependent.length).toBe(2);
    });

    it("should preserve original index", () => {
      const actions = [{ action: "wait" }, { action: "click" }];

      const { independent, dependent } =
        parallelExecutor._categorizeActions(actions);

      expect(independent[0]._originalIndex).toBe(0);
      expect(dependent[0]._originalIndex).toBe(1);
    });

    it("should handle empty actions array", () => {
      const { independent, dependent } = parallelExecutor._categorizeActions(
        [],
      );

      expect(independent.length).toBe(0);
      expect(dependent.length).toBe(0);
    });

    it("should handle all independent actions", () => {
      const actions = [{ action: "wait" }, { action: "screenshot" }];

      const { independent, dependent } =
        parallelExecutor._categorizeActions(actions);

      expect(independent.length).toBe(2);
      expect(dependent.length).toBe(0);
    });

    it("should handle all dependent actions", () => {
      const actions = [{ action: "click" }, { action: "type" }];

      const { independent, dependent } =
        parallelExecutor._categorizeActions(actions);

      expect(independent.length).toBe(0);
      expect(dependent.length).toBe(2);
    });
  });

  describe("executeSequence()", () => {
    it("should return empty array for empty actions", async () => {
      const results = await parallelExecutor.executeSequence([], vi.fn());
      expect(results).toEqual([]);
    });

    it("should execute single action without parallelization", async () => {
      const executeFn = vi.fn().mockResolvedValue({ success: true });
      const actions = [{ action: "click", selector: "#btn" }];

      const results = await parallelExecutor.executeSequence(
        actions,
        executeFn,
      );

      expect(results.length).toBe(1);
      expect(executeFn).toHaveBeenCalledTimes(1);
    });

    it("should execute independent actions in parallel", async () => {
      const executeFn = vi.fn().mockResolvedValue({ success: true });
      const actions = [
        { action: "wait", value: 100 },
        { action: "screenshot", name: "test" },
      ];

      const results = await parallelExecutor.executeSequence(
        actions,
        executeFn,
      );

      expect(results.length).toBe(2);
      expect(executeFn).toHaveBeenCalledTimes(2);
    });

    it("should execute dependent actions sequentially", async () => {
      const executeFn = vi.fn().mockResolvedValue({ success: true });
      const actions = [
        { action: "click", selector: "#btn" },
        { action: "type", selector: "#input", value: "test" },
      ];

      const results = await parallelExecutor.executeSequence(
        actions,
        executeFn,
      );

      expect(results.length).toBe(2);
    });

    it("should handle mixed independent and dependent actions", async () => {
      const executeFn = vi.fn().mockResolvedValue({ success: true });
      const actions = [
        { action: "wait" },
        { action: "click", selector: "#btn" },
        { action: "screenshot" },
      ];

      const results = await parallelExecutor.executeSequence(
        actions,
        executeFn,
      );

      expect(results.length).toBe(3);
    });

    it("should return results in original order", async () => {
      const executeFn = vi.fn().mockResolvedValue({ success: true });
      const actions = [
        { action: "click", selector: "#btn1" },
        { action: "wait" },
        { action: "click", selector: "#btn2" },
      ];

      const results = await parallelExecutor.executeSequence(
        actions,
        executeFn,
      );

      expect(results[0].action.selector).toBe("#btn1");
      expect(results[1].action.action).toBe("wait");
      expect(results[2].action.selector).toBe("#btn2");
    });

    it("should handle parallel action failures", async () => {
      let callCount = 0;
      const executeFn = vi.fn().mockImplementation(async (action) => {
        callCount++;
        if (callCount === 1) {
          throw new Error("First failed");
        }
        return { success: true };
      });
      const actions = [{ action: "wait" }, { action: "screenshot" }];

      const results = await parallelExecutor.executeSequence(
        actions,
        executeFn,
      );

      expect(results.length).toBe(2);
      expect(results[0].success).toBe(false);
      expect(results[1].success).toBe(true);
    });

    it("should stop sequential execution on failure", async () => {
      let callCount = 0;
      const executeFn = vi.fn().mockImplementation(() => {
        callCount++;
        if (callCount === 1) {
          throw new Error("First action failed");
        }
        return { success: true };
      });

      const actions = [
        { action: "click", selector: "#btn1" },
        { action: "click", selector: "#btn2" },
      ];

      const results = await parallelExecutor.executeSequence(
        actions,
        executeFn,
      );

      // Should only have result for first action since it failed
      expect(results.length).toBe(1);
      expect(results[0].success).toBe(false);
    });
  });

  describe("getStats()", () => {
    it("should calculate statistics from results", () => {
      const results = [
        { success: true },
        { success: true },
        { success: false },
        { success: true },
      ];

      const stats = parallelExecutor.getStats(results);

      expect(stats.total).toBe(4);
      expect(stats.successful).toBe(3);
      expect(stats.failed).toBe(1);
      expect(stats.successRate).toBe(0.75);
    });

    it("should handle empty results", () => {
      const stats = parallelExecutor.getStats([]);

      expect(stats.total).toBe(0);
      expect(stats.successful).toBe(0);
      expect(stats.failed).toBe(0);
      expect(stats.successRate).toBe(0);
    });

    it("should handle all successful results", () => {
      const results = [{ success: true }, { success: true }];

      const stats = parallelExecutor.getStats(results);

      expect(stats.successRate).toBe(1);
    });
  });

  describe("canParallelize()", () => {
    it("should return false for null actions", () => {
      expect(parallelExecutor.canParallelize(null)).toBe(false);
    });

    it("should return false for empty actions", () => {
      expect(parallelExecutor.canParallelize([])).toBe(false);
    });

    it("should return false for single action", () => {
      expect(parallelExecutor.canParallelize([{ action: "wait" }])).toBe(false);
    });

    it("should return false when less than 2 independent actions", () => {
      expect(
        parallelExecutor.canParallelize([
          { action: "wait" },
          { action: "click" },
        ]),
      ).toBe(false);
    });

    it("should return true when 2+ independent actions", () => {
      expect(
        parallelExecutor.canParallelize([
          { action: "wait" },
          { action: "screenshot" },
        ]),
      ).toBe(true);
    });

    it("should return true for many independent actions", () => {
      expect(
        parallelExecutor.canParallelize([
          { action: "wait" },
          { action: "screenshot" },
          { action: "verify" },
        ]),
      ).toBe(true);
    });
  });

  describe("estimateSpeedup()", () => {
    it("should estimate speedup for mixed actions", () => {
      const actions = [
        { action: "wait" },
        { action: "wait" },
        { action: "click" },
        { action: "type" },
      ];

      const estimate = parallelExecutor.estimateSpeedup(actions);

      expect(estimate.sequentialTime).toBeDefined();
      expect(estimate.parallelTime).toBeDefined();
      expect(estimate.speedup).toBeGreaterThan(0);
      expect(estimate.independentCount).toBe(2);
      expect(estimate.dependentCount).toBe(2);
    });

    it("should return speedup of 1 for no independent actions", () => {
      const actions = [{ action: "click" }, { action: "type" }];

      const estimate = parallelExecutor.estimateSpeedup(actions);

      expect(estimate.speedup).toBe(1);
      expect(estimate.independentCount).toBe(0);
    });

    it("should return speedup of 1 for empty actions", () => {
      const estimate = parallelExecutor.estimateSpeedup([]);

      expect(estimate.speedup).toBe(1);
    });

    it("should calculate correct sequential time", () => {
      const actions = [
        { action: "wait" }, // 100ms
        { action: "wait" }, // 100ms
        { action: "click" }, // 200ms
      ];

      const estimate = parallelExecutor.estimateSpeedup(actions);

      // Sequential: 100 + 100 + 200 = 400ms
      expect(estimate.sequentialTime).toBe(400);
    });

    it("should calculate correct parallel time", () => {
      const actions = [
        { action: "wait" }, // 100ms
        { action: "wait" }, // 100ms
        { action: "click" }, // 200ms
      ];

      const estimate = parallelExecutor.estimateSpeedup(actions);

      // Parallel: max(100, 100) + 200 = 300ms
      expect(estimate.parallelTime).toBe(300);
    });
  });

  describe("_orderByOriginalOrder()", () => {
    it("should return results in original order", () => {
      const results = [
        { action: { _originalIndex: 2 }, result: "c" },
        { action: { _originalIndex: 0 }, result: "a" },
        { action: { _originalIndex: 1 }, result: "b" },
      ];

      const originalActions = [
        { action: "a" },
        { action: "b" },
        { action: "c" },
      ];

      const ordered = parallelExecutor._orderByOriginalOrder(
        results,
        originalActions,
      );

      expect(ordered[0].result).toBe("a");
      expect(ordered[1].result).toBe("b");
      expect(ordered[2].result).toBe("c");
    });

    it("should filter out missing results", () => {
      const results = [{ action: { _originalIndex: 0 }, result: "a" }];

      const originalActions = [{ action: "a" }, { action: "b" }];

      const ordered = parallelExecutor._orderByOriginalOrder(
        results,
        originalActions,
      );

      expect(ordered.length).toBe(1);
      expect(ordered[0].result).toBe("a");
    });
  });
});
