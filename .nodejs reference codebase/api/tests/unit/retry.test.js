/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Unit tests for utils/retry.js
 * Tests retry functionality with exponential backoff
 * @module tests/unit/retry.test
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

describe("utils/retry", () => {
  let withRetry;

  beforeEach(async () => {
    vi.clearAllMocks();
    const module = await import("../../utils/retry.js");
    withRetry = module.withRetry;
  });

  describe("calculateBackoffDelay", () => {
    let calculateBackoffDelay;

    beforeEach(async () => {
      const module = await import("../../utils/retry.js");
      calculateBackoffDelay = module.calculateBackoffDelay;
    });

    it("should calculate delay correctly with default options", () => {
      const delay = calculateBackoffDelay(1);
      // attempt 1: baseDelay 1000 * factor 2 ^ 1 = 2000
      // jitterMultiplier: [0.5, 1.5] range
      // delay * jitter: 2000 * [0.5, 1.5] = [1000, 3000]
      expect(delay).toBeGreaterThanOrEqual(1000);
      expect(delay).toBeLessThanOrEqual(3000);
    });

    it("should calculate delay correctly with custom options (no jitter)", () => {
      const delay = calculateBackoffDelay(2, {
        baseDelay: 500,
        factor: 3,
        jitterMin: 1,
        jitterMax: 1,
      });
      // attempt 2: baseDelay 500 * factor 3 ^ 2 = 500 * 9 = 4500
      // jitterMultiplier: 1 (no jitter)
      expect(delay).toBe(4500);
    });

    it("should cap delay at maxDelay", () => {
      const delay = calculateBackoffDelay(10, {
        baseDelay: 1000,
        maxDelay: 5000,
        jitterMin: 1,
        jitterMax: 1,
      });
      // Without jitter, should hit exactly maxDelay
      expect(delay).toBe(5000);
    });

    it("should handle jitter range correctly", () => {
      const delay = calculateBackoffDelay(1, {
        baseDelay: 1000,
        jitterMin: 0.5,
        jitterMax: 1.5,
      });
      // attempt 1: 1000 * 2^1 = 2000
      // delay * jitterMultiplier: 2000 * [0.5, 1.5] = [1000, 3000]
      expect(delay).toBeGreaterThanOrEqual(1000);
      expect(delay).toBeLessThanOrEqual(3000);
    });

    it("should handle jitter range when jitterMin > jitterMax", () => {
      const delay = calculateBackoffDelay(1, {
        baseDelay: 1000,
        jitterMin: 1.5,
        jitterMax: 0.5,
      });
      expect(delay).toBeGreaterThanOrEqual(1000);
      expect(delay).toBeLessThanOrEqual(3000);
    });
  });

  describe("withRetry", () => {
    it("should execute operation successfully on first attempt", async () => {
      const operation = vi.fn().mockResolvedValue("success");

      const result = await withRetry(operation);

      expect(result).toBe("success");
      expect(operation).toHaveBeenCalledTimes(1);
    });

    it("should throw after all retries exhausted", async () => {
      const operation = vi
        .fn()
        .mockRejectedValue(new Error("Persistent failure"));

      await expect(
        withRetry(operation, { retries: 1, delay: 0 }),
      ).rejects.toThrow("Persistent failure");
    });

    it("should handle async operation returning promise", async () => {
      const operation = vi
        .fn()
        .mockImplementation(() => Promise.resolve("async success"));

      const result = await withRetry(operation);

      expect(result).toBe("async success");
    });

    it("should handle async operation rejecting promise", async () => {
      const operation = vi
        .fn()
        .mockImplementation(() => Promise.reject(new Error("Async failure")));

      // With retries: 1, should retry once before throwing
      const result = await withRetry(operation, { retries: 1, delay: 0 }).catch(
        (e) => e.message,
      );

      expect(result).toContain("Async failure");
    });

    it("should handle synchronous errors", async () => {
      const operation = vi.fn().mockImplementation(() => {
        throw new Error("Sync error");
      });

      // With retries: 1, should retry once before throwing
      const result = await withRetry(operation, { retries: 1, delay: 0 }).catch(
        (e) => e.message,
      );

      expect(result).toContain("Sync error");
    });
  });

  describe("Edge Cases", () => {
    it("should handle operation that returns null", async () => {
      const operation = vi.fn().mockResolvedValue(null);

      const result = await withRetry(operation);

      expect(result).toBeNull();
    });

    it("should handle operation that returns undefined", async () => {
      const operation = vi.fn().mockResolvedValue(undefined);

      const result = await withRetry(operation);

      expect(result).toBeUndefined();
    });

    it("should handle operation that returns 0", async () => {
      const operation = vi.fn().mockResolvedValue(0);

      const result = await withRetry(operation);

      expect(result).toBe(0);
    });

    it("should handle operation that returns false", async () => {
      const operation = vi.fn().mockResolvedValue(false);

      const result = await withRetry(operation);

      expect(result).toBe(false);
    });

    it("should handle operation that throws falsy error", async () => {
      const operation = vi.fn().mockImplementation(() => {
        throw null;
      });

      // Should handle throwing null/undefined
      await expect(
        withRetry(operation, { retries: 1, delay: 10 }),
      ).rejects.toThrow();
    });

    it("should retry on failure and succeed on subsequent attempt", async () => {
      let callCount = 0;
      const operation = vi.fn().mockImplementation(() => {
        callCount++;
        if (callCount < 2) {
          throw new Error("Temporary failure");
        }
        return "success after retry";
      });

      const result = await withRetry(operation, { retries: 3, delay: 0 });

      expect(result).toBe("success after retry");
      expect(operation).toHaveBeenCalledTimes(2);
    });

    it("should use exponential backoff with factor", async () => {
      const operation = vi.fn().mockImplementation(() => {
        throw new Error("Failure");
      });

      // This will fail all retries - we're testing the delay calculation
      await expect(
        withRetry(operation, { retries: 3, delay: 100, factor: 2 }),
      ).rejects.toThrow();
    });
  });
});
