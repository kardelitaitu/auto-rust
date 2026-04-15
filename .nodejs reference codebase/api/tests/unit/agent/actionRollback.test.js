/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Unit tests for api/agent/actionRollback.js
 * @module tests/unit/agent/actionRollback.test
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

describe("api/agent/actionRollback.js", () => {
  let actionRollback;

  beforeEach(async () => {
    vi.clearAllMocks();
    const module = await import("@api/agent/actionRollback.js");
    actionRollback = module.actionRollback || module.default;
    actionRollback.clearHistory();
  });

  describe("Constructor", () => {
    it("should initialize with empty history", () => {
      expect(actionRollback.actionHistory).toEqual([]);
    });

    it("should have maxHistory of 50", () => {
      expect(actionRollback.maxHistory).toBe(50);
    });

    it("should define critical actions", () => {
      expect(actionRollback.criticalActions.has("navigate")).toBe(true);
      expect(actionRollback.criticalActions.has("click")).toBe(true);
      expect(actionRollback.criticalActions.has("type")).toBe(true);
      expect(actionRollback.criticalActions.has("drag")).toBe(true);
    });
  });

  describe("recordAction()", () => {
    it("should record action with pre-state", () => {
      const preState = { url: "https://example.com", timestamp: Date.now() };
      const action = { action: "click", selector: "#btn" };
      const result = { success: true };

      actionRollback.recordAction(preState, action, result);

      expect(actionRollback.actionHistory.length).toBe(1);
      expect(actionRollback.actionHistory[0].preState).toEqual(preState);
      expect(actionRollback.actionHistory[0].action).toEqual(action);
    });

    it("should not record if preState is null", () => {
      actionRollback.recordAction(null, { action: "click" }, {});
      expect(actionRollback.actionHistory.length).toBe(0);
    });

    it("should trim history when over maxHistory", () => {
      actionRollback.maxHistory = 5;

      for (let i = 0; i < 10; i++) {
        actionRollback.recordAction(
          { url: `https://example.com/${i}` },
          { action: "click" },
          {},
        );
      }

      expect(actionRollback.actionHistory.length).toBe(5);
    });
  });

  describe("isCriticalAction()", () => {
    it("should return true for critical actions", () => {
      expect(actionRollback.isCriticalAction({ action: "click" })).toBe(true);
      expect(actionRollback.isCriticalAction({ action: "navigate" })).toBe(
        true,
      );
      expect(actionRollback.isCriticalAction({ action: "type" })).toBe(true);
      expect(actionRollback.isCriticalAction({ action: "drag" })).toBe(true);
    });

    it("should return false for non-critical actions", () => {
      expect(actionRollback.isCriticalAction({ action: "hover" })).toBe(false);
      expect(actionRollback.isCriticalAction({ action: "wait" })).toBe(false);
      expect(actionRollback.isCriticalAction({ action: "scroll" })).toBe(false);
    });
  });

  describe("rollbackLast()", () => {
    it("should return false when no history", async () => {
      const mockPage = { url: vi.fn().mockReturnValue("https://example.com") };
      const result = await actionRollback.rollbackLast(mockPage);
      expect(result).toBe(false);
    });

    it("should rollback last action successfully", async () => {
      const mockPage = {
        url: vi.fn().mockReturnValue("https://example.com/new"),
        goto: vi.fn().mockResolvedValue(undefined),
        evaluate: vi.fn().mockResolvedValue({ x: 0, y: 0 }),
      };

      actionRollback.recordAction(
        { url: "https://example.com/old", scrollPosition: { x: 100, y: 200 } },
        { action: "click", selector: "#btn" },
        {},
      );

      const result = await actionRollback.rollbackLast(mockPage);
      expect(result).toBe(true);
      expect(mockPage.goto).toHaveBeenCalledWith("https://example.com/old", {
        waitUntil: "domcontentloaded",
      });
    });

    it("should restore scroll position", async () => {
      const mockPage = {
        url: vi.fn().mockReturnValue("https://example.com"),
        evaluate: vi.fn().mockResolvedValue(undefined),
      };

      actionRollback.recordAction(
        { url: "https://example.com", scrollPosition: { x: 50, y: 100 } },
        { action: "click", selector: "#btn" },
        {},
      );

      await actionRollback.rollbackLast(mockPage);
      expect(mockPage.evaluate).toHaveBeenCalled();
    });

    it("should restore form value for type actions", async () => {
      const mockPage = {
        url: vi.fn().mockReturnValue("https://example.com"),
        evaluate: vi.fn().mockResolvedValue(undefined),
      };

      actionRollback.recordAction(
        {
          url: "https://example.com",
          scrollPosition: { x: 0, y: 0 },
          formValue: "original text",
        },
        { action: "type", selector: "#input" },
        {},
      );

      await actionRollback.rollbackLast(mockPage);
      expect(mockPage.evaluate).toHaveBeenCalled();
    });

    it("should return false on rollback failure", async () => {
      const mockPage = {
        url: vi.fn().mockReturnValue("https://example.com/new"),
        goto: vi.fn().mockRejectedValue(new Error("Navigation failed")),
      };

      actionRollback.recordAction(
        { url: "https://example.com/old" },
        { action: "click" },
        {},
      );

      const result = await actionRollback.rollbackLast(mockPage);
      expect(result).toBe(false);
    });
  });

  describe("rollbackMultiple()", () => {
    it("should rollback multiple actions", async () => {
      // Record 3 actions with scroll positions to ensure rollback succeeds
      for (let i = 0; i < 3; i++) {
        actionRollback.recordAction(
          {
            url: "https://example.com",
            scrollPosition: { x: i * 10, y: i * 10 },
          },
          { action: "click", selector: `#btn${i}` },
          {},
        );
      }

      const mockPage = {
        url: vi.fn().mockReturnValue("https://example.com"),
        evaluate: vi.fn().mockResolvedValue(undefined),
      };

      const rolledBack = await actionRollback.rollbackMultiple(mockPage, 2);
      expect(rolledBack).toBe(2);
      expect(actionRollback.actionHistory.length).toBe(1);
    });

    it("should stop on first failure", async () => {
      const mockPage = {
        url: vi.fn().mockReturnValue("https://example.com/new"),
        goto: vi
          .fn()
          .mockResolvedValueOnce(undefined)
          .mockRejectedValueOnce(new Error("Failed")),
      };

      actionRollback.recordAction(
        { url: "https://example.com/old1" },
        { action: "click" },
        {},
      );
      actionRollback.recordAction(
        { url: "https://example.com/old2" },
        { action: "click" },
        {},
      );

      const rolledBack = await actionRollback.rollbackMultiple(mockPage, 5);
      expect(rolledBack).toBe(1);
    });

    it("should return 0 when no history", async () => {
      const mockPage = { url: vi.fn() };
      const rolledBack = await actionRollback.rollbackMultiple(mockPage, 5);
      expect(rolledBack).toBe(0);
    });
  });

  describe("getHistory()", () => {
    it("should return copy of history", () => {
      actionRollback.recordAction(
        { url: "https://example.com" },
        { action: "click" },
        {},
      );

      const history = actionRollback.getHistory();
      expect(history.length).toBe(1);

      // Modifying returned array shouldn't affect internal history
      history.push({ fake: true });
      expect(actionRollback.actionHistory.length).toBe(1);
    });

    it("should return empty array when no history", () => {
      const history = actionRollback.getHistory();
      expect(history).toEqual([]);
    });
  });

  describe("clearHistory()", () => {
    it("should clear all history", () => {
      actionRollback.recordAction(
        { url: "https://example.com" },
        { action: "click" },
        {},
      );
      actionRollback.recordAction(
        { url: "https://example.com" },
        { action: "type" },
        {},
      );

      expect(actionRollback.actionHistory.length).toBe(2);

      actionRollback.clearHistory();
      expect(actionRollback.actionHistory.length).toBe(0);
    });
  });

  describe("getStats()", () => {
    it("should return statistics", () => {
      // Reset maxHistory to default
      actionRollback.maxHistory = 50;
      actionRollback.recordAction(
        { url: "https://example.com" },
        { action: "click" },
        {},
      );

      const stats = actionRollback.getStats();
      expect(stats.historySize).toBe(1);
      expect(stats.maxHistory).toBe(50);
      expect(stats.criticalActions).toContain("click");
    });

    it("should return empty stats when no history", () => {
      const stats = actionRollback.getStats();
      expect(stats.historySize).toBe(0);
    });
  });

  describe("Edge Cases", () => {
    it("should handle capturePreState failure gracefully", async () => {
      const mockPage = {
        url: vi.fn().mockImplementation(() => {
          throw new Error("Page closed");
        }),
      };

      const state = await actionRollback.capturePreState(mockPage, {
        action: "click",
      });
      expect(state).toBeNull();
    });

    it("should handle rollback without URL change", async () => {
      const mockPage = {
        url: vi.fn().mockReturnValue("https://example.com"),
        goto: vi.fn(),
        evaluate: vi.fn().mockResolvedValue(undefined),
      };

      actionRollback.recordAction(
        { url: "https://example.com" },
        { action: "click" },
        {},
      );

      await actionRollback.rollbackLast(mockPage);
      expect(mockPage.goto).not.toHaveBeenCalled();
    });
  });
});
