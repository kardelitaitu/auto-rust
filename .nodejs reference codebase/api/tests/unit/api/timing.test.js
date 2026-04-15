/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";

// Mock dependencies BEFORE importing the module under test
vi.mock("@api/utils/math.js", () => ({
  mathUtils: {
    gaussian: vi.fn((mean, stdDev) => mean),
    randomInRange: vi.fn((min, max) => min),
  },
}));

vi.mock("@api/behaviors/persona.js", () => ({
  getPersona: vi.fn().mockReturnValue({ speed: 1 }),
}));

vi.mock("@api/core/context.js", () => ({
  getPage: vi.fn(),
}));

vi.mock("@api/core/logger.js", () => ({
  createLogger: () => ({
    debug: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
  }),
}));

// Mock humanTiming but keep the real functions from timing.js
vi.mock("@api/utils/timing.js", () => ({
  humanTiming: {
    humanDelay: vi.fn((ms) => Promise.resolve(ms)),
  },
}));

// Import AFTER mocks are set up - this gets the real module
import {
  think,
  delay,
  gaussian,
  randomInRange,
} from "@api/behaviors/timing.js";
import { getPage } from "@api/core/context.js";
import { humanTiming } from "@api/utils/timing.js";
import { mathUtils } from "@api/utils/math.js";

describe("api/behaviors/timing.js", () => {
  let mockPage;

  beforeEach(() => {
    vi.clearAllMocks();
    vi.useFakeTimers();

    mockPage = {
      evaluate: vi.fn().mockImplementation(async (fn) => {
        if (typeof fn === "function") {
          const originalPerformance = global.performance;
          global.performance = {
            getEntriesByType: vi
              .fn()
              .mockReturnValue([{ duration: 100, startTime: 10 }]),
          };
          global.window = { scrollBy: vi.fn(), scrollY: 0 };

          try {
            const result = await fn();
            return result;
          } catch (__e) {
            return { lcp: 0, lag: 0 };
          } finally {
            global.performance = originalPerformance;
          }
        }
        return {
          lcp: 100,
          lag: 10,
        };
      }),
    };

    getPage.mockReturnValue(mockPage);
    humanTiming.humanDelay.mockImplementation((ms) => Promise.resolve(ms));
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  describe("think", () => {
    it("should wait for specified duration", async () => {
      const promise = think(1000);

      await Promise.resolve();
      await Promise.resolve();

      expect(humanTiming.humanDelay).toHaveBeenCalledWith(
        1000,
        expect.any(Object),
      );

      await vi.advanceTimersByTimeAsync(1000);
      await promise;
    });

    it("should apply impatience multiplier if page is slow", async () => {
      mockPage.evaluate.mockResolvedValue({
        lcp: 3000,
        lag: 100,
      });

      const promise = think(1000);

      await Promise.resolve();
      await Promise.resolve();

      await vi.advanceTimersByTimeAsync(1000);
      await promise;

      expect(humanTiming.humanDelay).toHaveBeenCalledWith(
        750,
        expect.any(Object),
      );
    });

    it("should handle evaluate failure gracefully", async () => {
      mockPage.evaluate.mockRejectedValue(new Error("Eval failed"));

      const promise = think(1000);

      await Promise.resolve();
      await Promise.resolve();

      await vi.advanceTimersByTimeAsync(1000);
      await promise;

      expect(humanTiming.humanDelay).toHaveBeenCalledWith(
        1000,
        expect.any(Object),
      );
    });
  });

  describe("delay", () => {
    it("should wait for specified duration", async () => {
      const promise = delay(500);

      expect(humanTiming.humanDelay).toHaveBeenCalledWith(500);

      await vi.advanceTimersByTimeAsync(500);
      await promise;
    });
  });

  describe("gaussian", () => {
    it("should delegate to mathUtils", () => {
      // mathUtils.gaussian is mocked to return mean (10)
      const result = gaussian(10, 2);
      expect(result).toBe(10);
    });
  });

  describe("randomInRange", () => {
    it("should delegate to mathUtils", () => {
      // mathUtils.randomInRange is mocked to return min (10)
      const result = randomInRange(10, 20);
      expect(result).toBe(10);
    });
  });
});
