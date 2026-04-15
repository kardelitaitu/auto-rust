/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { calculateBackoffDelay, withRetry } from "@api/utils/retry.js";

describe("api/utils/retry.js", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.spyOn(console, "warn").mockImplementation(() => {});
    vi.spyOn(console, "error").mockImplementation(() => {});
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  describe("calculateBackoffDelay", () => {
    it("should calculate exponential backoff", () => {
      const delay0 = calculateBackoffDelay(0, {
        baseDelay: 1000,
        factor: 2,
        jitterMin: 1,
        jitterMax: 1,
      });
      const delay1 = calculateBackoffDelay(1, {
        baseDelay: 1000,
        factor: 2,
        jitterMin: 1,
        jitterMax: 1,
      });
      const delay2 = calculateBackoffDelay(2, {
        baseDelay: 1000,
        factor: 2,
        jitterMin: 1,
        jitterMax: 1,
      });

      expect(delay0).toBe(1000);
      expect(delay1).toBe(2000);
      expect(delay2).toBe(4000);
    });

    it("should respect maxDelay", () => {
      const delay = calculateBackoffDelay(10, {
        baseDelay: 1000,
        factor: 2,
        maxDelay: 5000,
        jitterMin: 1,
        jitterMax: 1,
      });

      expect(delay).toBe(5000);
    });

    it("should use default options", () => {
      const delay = calculateBackoffDelay(0);
      // Default: baseDelay 1000, factor 2, jitterMin 0.5, jitterMax 1.5
      // attempt 0: 1000 * 2^0 = 1000, jitter [0.5, 1.5] => [500, 1500]
      expect(delay).toBeGreaterThanOrEqual(500);
      expect(delay).toBeLessThanOrEqual(1500);
    });

    it("should apply jitter", () => {
      vi.spyOn(Math, "random").mockReturnValue(0.5);

      const delay = calculateBackoffDelay(0, {
        baseDelay: 1000,
        jitterMin: 0.5,
        jitterMax: 1.5,
      });

      expect(delay).toBe(1000); // 1000 * (0.5 + 0.5 * (1.5 - 0.5)) = 1000
    });

    it("should use min jitter when min > max", () => {
      const delay = calculateBackoffDelay(0, {
        baseDelay: 1000,
        jitterMin: 2,
        jitterMax: 1,
      });

      expect(delay).toBeGreaterThanOrEqual(1000);
      expect(delay).toBeLessThanOrEqual(2000);
    });

    it("should use max jitter when min < max", () => {
      vi.spyOn(Math, "random").mockReturnValue(0.5);

      const delay = calculateBackoffDelay(0, {
        baseDelay: 1000,
        jitterMin: 1,
        jitterMax: 2,
      });

      expect(delay).toBe(1500); // 1000 * (1 + 0.5 * (2 - 1)) = 1500
    });

    it("should floor the result", () => {
      vi.spyOn(Math, "random").mockReturnValue(0.333);

      const delay = calculateBackoffDelay(0, {
        baseDelay: 1000,
        jitterMin: 1,
        jitterMax: 1.5,
      });

      expect(delay).toBe(1166); // floor(1000 * 1.1665) = 1166
    });
  });

  describe("withRetry", () => {
    it("should return successful result on first try", async () => {
      const operation = vi.fn().mockResolvedValue("success");

      const result = await withRetry(operation, { retries: 3 });

      expect(result).toBe("success");
      expect(operation).toHaveBeenCalledTimes(1);
    });
  });
});
