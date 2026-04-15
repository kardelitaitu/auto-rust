/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import {
  createPipeline,
  createSyncPipeline,
  loggingMiddleware,
  isNonRetryableError,
  validationMiddleware,
  retryMiddleware,
  recoveryMiddleware,
  metricsMiddleware,
  rateLimitMiddleware,
} from "@api/core/middleware.js";
import * as context from "@api/core/context.js";
import * as contextState from "@api/core/context-state.js";

vi.mock("@api/tests/core/logger.js", () => ({
  createLogger: () => ({
    debug: vi.fn(),
    error: vi.fn(),
    info: vi.fn(),
    warn: vi.fn(),
  }),
}));

describe("api/core/middleware.js", () => {
  let mockEvents;

  beforeEach(() => {
    vi.clearAllMocks();
    mockEvents = {
      emitSafe: vi.fn(),
    };
    vi.spyOn(context, "getEvents").mockReturnValue(mockEvents);
    vi.spyOn(contextState, "getStateSection").mockReturnValue({
      used: 0,
      max: 10,
    });
    vi.spyOn(contextState, "updateStateSection").mockReturnValue();
  });

  describe("createPipeline", () => {
    it("should execute middleware in order", async () => {
      const order = [];
      const m1 = async (ctx, next) => {
        order.push(1);
        return await next();
      };
      const m2 = async (ctx, next) => {
        order.push(2);
        return await next();
      };
      const action = async (ctx) => {
        order.push(3);
        return "done";
      };

      const pipeline = createPipeline(m1, m2);
      const result = await pipeline(action, { data: "test" });

      expect(order).toEqual([1, 2, 3]);
      expect(result).toBe("done");
    });

    it("should allow middleware to short-circuit", async () => {
      const m1 = async (ctx, next) => "short-circuit";
      const m2 = vi.fn();
      const action = vi.fn();

      const pipeline = createPipeline(m1, m2);
      const result = await pipeline(action, {});

      expect(result).toBe("short-circuit");
      expect(m2).not.toHaveBeenCalled();
      expect(action).not.toHaveBeenCalled();
    });
  });

  describe("createSyncPipeline", () => {
    it("should execute sync middleware", () => {
      const m1 = (ctx, next) => next() + 1;
      const m2 = (ctx, next) => next() + 2;
      const action = (ctx) => 0;

      const pipeline = createSyncPipeline(m1, m2);
      const result = pipeline(action, {});

      expect(result).toBe(3);
    });
  });

  describe("loggingMiddleware", () => {
    it("should log action execution", async () => {
      const m = loggingMiddleware({ logTime: true });
      const next = vi.fn().mockResolvedValue("ok");
      const ctx = { action: "test-action", selector: ".btn" };

      await m(ctx, next);
      expect(next).toHaveBeenCalled();
    });

    it("should respect disabled logging options", async () => {
      const m = loggingMiddleware({
        logArgs: false,
        logResult: false,
        logTime: false,
      });
      const next = vi.fn().mockResolvedValue("ok");
      await m({ action: "test" }, next);
      expect(next).toHaveBeenCalled();
    });

    it("should log errors", async () => {
      const m = loggingMiddleware();
      const next = vi.fn().mockRejectedValue(new Error("fail"));
      await expect(m({ action: "test" }, next)).rejects.toThrow("fail");
    });
  });

  describe("validationMiddleware", () => {
    it("should validate DOM actions", async () => {
      const m = validationMiddleware();
      const next = vi.fn();

      await expect(
        m({ action: "click", selector: null }, next),
      ).rejects.toThrow("Selector is required");
      await expect(m({ action: "click", selector: " " }, next)).rejects.toThrow(
        "Empty selector",
      );
      await expect(m({ action: "click", selector: 123 }, next)).rejects.toThrow(
        "Expected string or Locator",
      );

      await m({ action: "click", selector: ".btn" }, next);
      expect(next).toHaveBeenCalledTimes(1);

      // Should support Mock Locator (simplistic check)
      const mockLocator = { constructor: { name: "Locator" } };
      await m({ action: "click", selector: mockLocator }, next);
      expect(next).toHaveBeenCalledTimes(2);
    });

    it("should skip validation for non-DOM actions", async () => {
      const m = validationMiddleware();
      const next = vi.fn();
      await m({ action: "other" }, next);
      expect(next).toHaveBeenCalled();
    });

    it("should validate timeoutMs", async () => {
      const m = validationMiddleware();
      await expect(
        m({ action: "wait", options: { timeoutMs: -1 } }, vi.fn()),
      ).rejects.toThrow("timeoutMs must be non-negative");
    });

    it("should validate maxRetries", async () => {
      const m = validationMiddleware();
      await expect(
        m(
          { action: "click", selector: "#a", options: { maxRetries: -1 } },
          vi.fn(),
        ),
      ).rejects.toThrow("maxRetries must be non-negative");
    });
  });

  describe("retryMiddleware", () => {
    it("should retry on failure", async () => {
      const m = retryMiddleware({ maxRetries: 2, backoffMultiplier: 1 });
      let calls = 0;
      const next = vi.fn().mockImplementation(() => {
        calls++;
        if (calls < 3) throw new Error("fail");
        return "success";
      });

      const result = await m({}, next);
      expect(result).toBe("success");
      expect(calls).toBe(3);
    });

    it("should throw after max retries", async () => {
      const m = retryMiddleware({ maxRetries: 1, backoffMultiplier: 1 });
      const next = vi.fn().mockRejectedValue(new Error("permanent fail"));

      await expect(m({}, next)).rejects.toThrow("permanent fail");
      expect(next).toHaveBeenCalledTimes(2);
    });

    it("should respect shouldRetry condition", async () => {
      const m = retryMiddleware({
        maxRetries: 5,
        shouldRetry: (e) => e.message === "retry me",
      });
      const next = vi.fn().mockRejectedValue(new Error("dont retry"));

      await expect(m({}, next)).rejects.toThrow("dont retry");
      expect(next).toHaveBeenCalledTimes(1);
    });

    it("should throw when retry budget exceeded", async () => {
      vi.spyOn(contextState, "getStateSection").mockReturnValue({
        used: 10,
        max: 10,
      });
      const m = retryMiddleware({ maxRetries: 3 });
      const next = vi.fn().mockResolvedValue("ok");

      await expect(m({}, next)).rejects.toThrow(
        "Session retry budget exhausted",
      );
    });
  });

  describe("isNonRetryableError", () => {
    it("should detect non-retryable error messages and codes", () => {
      expect(isNonRetryableError(new Error("Target closed"))).toBe(true);
      expect(isNonRetryableError({ code: "PAGE_CLOSED" })).toBe(true);
      expect(isNonRetryableError(new Error("temporary issue"))).toBe(false);
      expect(isNonRetryableError(null)).toBe(false);
    });
  });

  describe("recoveryMiddleware", () => {
    it("should retry on detached errors", async () => {
      const m = recoveryMiddleware();
      let failedOnce = false;
      const next = vi.fn().mockImplementation(() => {
        if (!failedOnce) {
          failedOnce = true;
          throw new Error("Element is detached");
        }
        return "recovered";
      });

      const result = await m({ action: "click" }, next);
      expect(result).toBe("recovered");
      expect(next).toHaveBeenCalledTimes(2);
    });

    it("should retry with force on obscured errors", async () => {
      const m = recoveryMiddleware();
      let failedOnce = false;
      const context = { action: "click", options: {} };
      const next = vi.fn().mockImplementation(() => {
        if (!failedOnce) {
          failedOnce = true;
          throw new Error("Element is obscured by another element");
        }
        return "recovered";
      });

      const result = await m(context, next);
      expect(result).toBe("recovered");
      expect(context.options.force).toBe(true);
      expect(next).toHaveBeenCalledTimes(2);
    });

    it("should bypass recovery if disabled", async () => {
      const m = recoveryMiddleware({
        scrollOnDetached: false,
        retryOnObscured: false,
      });
      const next = vi.fn().mockRejectedValue(new Error("detached obscured"));
      await expect(m({ action: "click" }, next)).rejects.toThrow();
    });

    it("should handle error without message", async () => {
      const m = recoveryMiddleware();
      const next = vi.fn().mockRejectedValue({});
      await expect(m({ action: "click" }, next)).rejects.toEqual({});
    });
  });

  describe("metricsMiddleware", () => {
    it("should emit metrics on success", async () => {
      const m = metricsMiddleware();
      const next = vi.fn().mockResolvedValue("ok");

      await m({ action: "test" }, next);

      expect(mockEvents.emitSafe).toHaveBeenCalledWith(
        "on:metrics",
        expect.objectContaining({
          action: "test",
          success: true,
        }),
      );
    });

    it("should respect disabled emitEvents", async () => {
      const m = metricsMiddleware({ emitEvents: false });
      const next = vi.fn().mockResolvedValue("ok");
      await m({ action: "test" }, next);
      expect(mockEvents.emitSafe).not.toHaveBeenCalled();
    });

    it("should emit metrics on failure", async () => {
      const m = metricsMiddleware();
      const next = vi.fn().mockRejectedValue(new Error("fail"));

      await expect(m({ action: "test" }, next)).rejects.toThrow();

      expect(mockEvents.emitSafe).toHaveBeenCalledWith(
        "on:metrics",
        expect.objectContaining({
          action: "test",
          success: false,
        }),
      );
    });
  });

  describe("rateLimitMiddleware", () => {
    it("should reset window after 1 second", async () => {
      vi.useFakeTimers();
      const m = rateLimitMiddleware({ maxPerSecond: 1 });
      const next = vi.fn().mockResolvedValue("ok");

      await m({}, next); // actionCount = 1

      vi.advanceTimersByTime(1100);

      await m({}, next); // should not throttle because window reset
      expect(next).toHaveBeenCalledTimes(2);
      vi.useRealTimers();
    });

    it("should throttle actions exceeding limit", async () => {
      vi.useFakeTimers();
      const m = rateLimitMiddleware({ maxPerSecond: 2 });
      const next = vi.fn().mockResolvedValue("ok");

      await m({}, next);
      await m({}, next);

      // Third call should be throttled
      const p3 = m({}, next);
      await vi.advanceTimersByTimeAsync(1000);
      await p3;

      expect(next).toHaveBeenCalledTimes(3);
      vi.useRealTimers();
    });
  });
});
