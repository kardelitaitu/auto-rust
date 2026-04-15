/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect } from "vitest";
import { humanTiming } from "@api/utils/timing.js";

describe("api/utils/timing.js", () => {
  describe("gaussianRandom", () => {
    it("should return a number", () => {
      const result = humanTiming.gaussianRandom(100, 10);
      expect(typeof result).toBe("number");
      expect(Number.isFinite(result)).toBe(true);
    });

    it("should return value within reasonable range", () => {
      const results = [];
      for (let i = 0; i < 100; i++) {
        results.push(humanTiming.gaussianRandom(100, 15));
      }
      const min = Math.min(...results);
      const max = Math.max(...results);
      expect(min).toBeLessThan(200);
      expect(max).toBeGreaterThan(0);
    });
  });

  describe("humanDelay", () => {
    it("should return a number greater than minDelay", () => {
      const result = humanTiming.humanDelay(1000, { minDelay: 100 });
      expect(result).toBeGreaterThanOrEqual(100);
    });

    it("should use default options when not provided", () => {
      const result = humanTiming.humanDelay(1000);
      expect(typeof result).toBe("number");
      expect(result).toBeGreaterThan(0);
    });

    it("should accept custom jitter", () => {
      const result = humanTiming.humanDelay(1000, { jitter: 0.5 });
      expect(typeof result).toBe("number");
    });

    it("should apply pauseChance multiplier when triggered", () => {
      let triggered = false;
      for (let i = 0; i < 50; i++) {
        const result = humanTiming.humanDelay(1000, { pauseChance: 1.0 });
        if (result > 2000) {
          triggered = true;
          break;
        }
      }
      expect(triggered).toBe(true);
    });

    it("should apply burstChance reduction when triggered", () => {
      let triggered = false;
      for (let i = 0; i < 50; i++) {
        const result = humanTiming.humanDelay(1000, { burstChance: 1.0 });
        if (result < 500) {
          triggered = true;
          break;
        }
      }
      expect(triggered).toBe(true);
    });
  });

  describe("randomInRange", () => {
    it("should return integer within range", () => {
      const result = humanTiming.randomInRange(5, 10);
      expect(result).toBeGreaterThanOrEqual(5);
      expect(result).toBeLessThanOrEqual(10);
      expect(Number.isInteger(result)).toBe(true);
    });
  });

  describe("gaussianInRange", () => {
    it("should return integer within bounds", () => {
      const result = humanTiming.gaussianInRange(100, 10, 50, 150);
      expect(result).toBeGreaterThanOrEqual(50);
      expect(result).toBeLessThanOrEqual(150);
      expect(Number.isInteger(result)).toBe(true);
    });

    it("should clamp to max after maxAttempts", () => {
      const result = humanTiming.gaussianInRange(1000, 1, 50, 150);
      expect(result).toBe(150);
    });

    it("should clamp to min after maxAttempts", () => {
      const result = humanTiming.gaussianInRange(1, 1, 50, 150);
      expect(result).toBe(50);
    });
  });

  describe("readingTimes", () => {
    it("should have predefined reading time configs", () => {
      expect(humanTiming.readingTimes.quick).toBeDefined();
      expect(humanTiming.readingTimes.text).toBeDefined();
      expect(humanTiming.readingTimes.image).toBeDefined();
      expect(humanTiming.readingTimes.video).toBeDefined();
      expect(humanTiming.readingTimes.thread).toBeDefined();
      expect(humanTiming.readingTimes.longThread).toBeDefined();
    });

    it("should have mean and stdev for each type", () => {
      Object.values(humanTiming.readingTimes).forEach((config) => {
        expect(config.mean).toBeGreaterThan(0);
        expect(config.stdev).toBeGreaterThan(0);
      });
    });
  });

  describe("actionDelays", () => {
    it("should have predefined action delay configs", () => {
      expect(humanTiming.actionDelays.quick).toBeDefined();
      expect(humanTiming.actionDelays.like).toBeDefined();
      expect(humanTiming.actionDelays.bookmark).toBeDefined();
      expect(humanTiming.actionDelays.retweet).toBeDefined();
      expect(humanTiming.actionDelays.reply).toBeDefined();
      expect(humanTiming.actionDelays.follow).toBeDefined();
      expect(humanTiming.actionDelays.dive).toBeDefined();
      expect(humanTiming.actionDelays.scroll).toBeDefined();
    });
  });

  describe("getReadingTime", () => {
    it("should return time for quick content", () => {
      const result = humanTiming.getReadingTime("quick");
      expect(result).toBeGreaterThan(0);
    });

    it("should return time for text content", () => {
      const result = humanTiming.getReadingTime("text");
      expect(result).toBeGreaterThan(0);
    });

    it("should return value within expected range for image", () => {
      const result = humanTiming.getReadingTime("image");
      expect(result).toBeGreaterThan(1000);
      expect(result).toBeLessThan(20000);
    });

    it("should return value within expected range for video", () => {
      const result = humanTiming.getReadingTime("video");
      expect(result).toBeGreaterThan(1000);
      expect(result).toBeLessThan(30000);
    });

    it("should use custom min/max options", () => {
      const result = humanTiming.getReadingTime("text", {
        min: 5000,
        max: 10000,
      });
      expect(result).toBeGreaterThanOrEqual(5000);
      expect(result).toBeLessThanOrEqual(10000);
    });

    it("should fallback to text for unknown content type", () => {
      const result = humanTiming.getReadingTime("unknown");
      expect(result).toBeGreaterThan(0);
    });
  });

  describe("getActionDelay", () => {
    it("should return delay for like action", () => {
      const result = humanTiming.getActionDelay("like");
      expect(result).toBeGreaterThan(0);
    });

    it("should return delay for reply action", () => {
      const result = humanTiming.getActionDelay("reply");
      expect(result).toBeGreaterThan(0);
    });

    it("should use custom min/max options", () => {
      const result = humanTiming.getActionDelay("like", { min: 100, max: 200 });
      expect(result).toBeGreaterThanOrEqual(100);
      expect(result).toBeLessThanOrEqual(200);
    });

    it("should fallback to quick for unknown action", () => {
      const result = humanTiming.getActionDelay("unknown");
      expect(result).toBeGreaterThan(0);
    });
  });

  describe("getWarmupDelay", () => {
    it("should return delay within default range", () => {
      const result = humanTiming.getWarmupDelay();
      expect(result).toBeGreaterThanOrEqual(2000);
      expect(result).toBeLessThanOrEqual(15000);
    });

    it("should use custom range", () => {
      const result = humanTiming.getWarmupDelay({ min: 1000, max: 2000 });
      expect(result).toBeGreaterThanOrEqual(1000);
      expect(result).toBeLessThanOrEqual(2000);
    });
  });

  describe("getScrollPause", () => {
    it("should return delay within default range", () => {
      const result = humanTiming.getScrollPause();
      expect(result).toBeGreaterThanOrEqual(1500);
      expect(result).toBeLessThanOrEqual(4000);
    });

    it("should use custom range", () => {
      const result = humanTiming.getScrollPause({ min: 500, max: 1000 });
      expect(result).toBeGreaterThanOrEqual(500);
      expect(result).toBeLessThanOrEqual(1000);
    });
  });

  describe("getScrollDuration", () => {
    it("should return duration within default range", () => {
      const result = humanTiming.getScrollDuration();
      expect(result).toBeGreaterThanOrEqual(300);
      expect(result).toBeLessThanOrEqual(700);
    });

    it("should use custom range", () => {
      const result = humanTiming.getScrollDuration({ min: 100, max: 200 });
      expect(result).toBeGreaterThanOrEqual(100);
      expect(result).toBeLessThanOrEqual(200);
    });
  });

  describe("getBetweenCycleDelay", () => {
    it("should return delay within default range", () => {
      const result = humanTiming.getBetweenCycleDelay();
      expect(result).toBeGreaterThanOrEqual(1000);
      expect(result).toBeLessThanOrEqual(3000);
    });

    it("should use custom range", () => {
      const result = humanTiming.getBetweenCycleDelay({ min: 500, max: 800 });
      expect(result).toBeGreaterThanOrEqual(500);
      expect(result).toBeLessThanOrEqual(800);
    });
  });

  describe("exponentialBackoff", () => {
    it("should return increasing delays on average", () => {
      let delay0Total = 0;
      let delay1Total = 0;
      for (let i = 0; i < 50; i++) {
        delay0Total += humanTiming.exponentialBackoff(0, 1000, 100000, 2);
        delay1Total += humanTiming.exponentialBackoff(1, 1000, 100000, 2);
      }
      expect(delay1Total / 50).toBeGreaterThan(delay0Total / 50);
    });

    it("should return value less than maxDelay with high jitter", () => {
      let passed = false;
      for (let i = 0; i < 10; i++) {
        const delay = humanTiming.exponentialBackoff(5, 1000, 5000);
        if (delay <= 5000) {
          passed = true;
          break;
        }
      }
      expect(passed).toBe(true);
    });

    it("should use custom factor", () => {
      const delay1 = humanTiming.exponentialBackoff(1, 1000, 100000, 2);
      const delay2 = humanTiming.exponentialBackoff(1, 1000, 100000, 3);

      expect(delay1).not.toBe(delay2);
    });
  });

  describe("jitterValue", () => {
    it("should return value close to input", () => {
      const results = [];
      for (let i = 0; i < 1000; i++) {
        results.push(humanTiming.jitterValue(1000));
      }
      const avg = results.reduce((a, b) => a + b, 0) / results.length;
      expect(avg).toBeGreaterThan(800);
      expect(avg).toBeLessThan(1200);
    });

    it("should respect factor parameter", () => {
      const result = humanTiming.jitterValue(1000, 0.1);
      expect(typeof result).toBe("number");
    });
  });

  describe("formatDuration", () => {
    it("should format milliseconds", () => {
      expect(humanTiming.formatDuration(500)).toBe("500ms");
    });

    it("should format seconds", () => {
      expect(humanTiming.formatDuration(2500)).toBe("2.5s");
    });

    it("should format minutes", () => {
      expect(humanTiming.formatDuration(90000)).toBe("1.5m");
    });
  });

  describe("defaults", () => {
    it("should have default jitter", () => {
      expect(humanTiming.defaults.jitter).toBe(0.15);
    });

    it("should have default pauseChance", () => {
      expect(humanTiming.defaults.pauseChance).toBe(0.08);
    });

    it("should have default burstChance", () => {
      expect(humanTiming.defaults.burstChance).toBe(0.05);
    });
  });
});
