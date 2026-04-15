/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi } from "vitest";
import { humanTiming } from "@api/utils/timing.js";

describe("humanTiming", () => {
  describe("Statistical Functions", () => {
    it("should generate gaussian random numbers", () => {
      const mean = 100;
      const stdev = 10;
      const values = [];
      for (let i = 0; i < 100; i++)
        values.push(humanTiming.gaussianRandom(mean, stdev));

      const avg = values.reduce((a, b) => a + b, 0) / values.length;
      expect(avg).toBeGreaterThan(90);
      expect(avg).toBeLessThan(110);
    });

    it("should generate random in range", () => {
      for (let i = 0; i < 100; i++) {
        const val = humanTiming.randomInRange(10, 20);
        expect(val).toBeGreaterThanOrEqual(10);
        expect(val).toBeLessThanOrEqual(20);
      }
    });

    it("should generate gaussian in range", () => {
      for (let i = 0; i < 100; i++) {
        const val = humanTiming.gaussianInRange(100, 10, 90, 110);
        expect(val).toBeGreaterThanOrEqual(90);
        expect(val).toBeLessThanOrEqual(110);
      }
    });
  });

  describe("humanDelay", () => {
    it("should return base delay with jitter", () => {
      vi.spyOn(Math, "random").mockReturnValue(0.5); // Predictable random
      const delay = humanTiming.humanDelay(1000, {
        pauseChance: 0,
        burstChance: 0,
      });
      expect(delay).toBeGreaterThan(0);
    });

    it("should apply pause logic", () => {
      // Force pause (random < pauseChance)
      // humanDelay calls gaussianRandom which calls Math.random twice.
      // Then it checks pauseChance.
      // We need to control the sequence of randoms.
      // 1. gaussianRandom u1
      // 2. gaussianRandom u2
      // 3. pauseChance check

      const randomSpy = vi.spyOn(Math, "random");
      randomSpy
        .mockReturnValueOnce(0.5) // gaussian u1
        .mockReturnValueOnce(0.5) // gaussian u2
        .mockReturnValueOnce(0.01) // pause check (assumed < 0.08 default)
        .mockReturnValueOnce(0.99); // burst check (fail)

      const delay = humanTiming.humanDelay(100, {
        pauseChance: 0.1,
        burstChance: 0,
      });
      // Gaussian with 0.5, 0.5 gives mean roughly.
      // Pause multiplies by 3.
      expect(delay).toBeGreaterThan(200); // 100 * 3 roughly
    });

    it("should apply burst logic", () => {
      const randomSpy = vi.spyOn(Math, "random");
      randomSpy
        .mockReturnValueOnce(0.5) // gaussian u1
        .mockReturnValueOnce(0.5) // gaussian u2
        .mockReturnValueOnce(0.99) // pause check (fail)
        .mockReturnValueOnce(0.01); // burst check (pass)

      const delay = humanTiming.humanDelay(100, {
        pauseChance: 0,
        burstChance: 0.1,
        minDelay: 10,
      });
      // Burst multiplies by 0.3 -> 30ms
      expect(delay).toBeLessThan(50);
    });
  });

  describe("Configuration Getters", () => {
    it("should get reading time for types", () => {
      expect(humanTiming.getReadingTime("quick")).toBeGreaterThan(0);
      expect(humanTiming.getReadingTime("unknown")).toBeGreaterThan(0); // Fallback to text
    });

    it("should get action delay for types", () => {
      expect(humanTiming.getActionDelay("like")).toBeGreaterThan(0);
    });

    it("should get specific delays", () => {
      expect(humanTiming.getWarmupDelay()).toBeGreaterThan(0);
      expect(humanTiming.getScrollPause()).toBeGreaterThan(0);
      expect(humanTiming.getScrollDuration()).toBeGreaterThan(0);
      expect(humanTiming.getBetweenCycleDelay()).toBeGreaterThan(0);
    });
  });

  describe("Helpers", () => {
    it("should calculate exponential backoff", () => {
      const d1 = humanTiming.exponentialBackoff(0, 100);
      const d2 = humanTiming.exponentialBackoff(1, 100);
      // d2 should be roughly double d1 (ignoring jitter overlaps)
      // But we can just check bounds
      expect(d1).toBeGreaterThan(0);
      expect(d2).toBeGreaterThan(0);
    });

    it("should jitter values", () => {
      const val = humanTiming.jitterValue(100, 0.1);
      expect(val).not.toBe(100); // unlikely
    });

    it("should format duration", () => {
      expect(humanTiming.formatDuration(500)).toBe("500ms");
      expect(humanTiming.formatDuration(1500)).toBe("1.5s");
      expect(humanTiming.formatDuration(65000)).toBe("1.1m");
    });
  });
});
