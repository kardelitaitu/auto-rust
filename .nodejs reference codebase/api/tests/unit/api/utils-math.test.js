/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect } from "vitest";
import { mathUtils } from "@api/utils/math.js";

describe("api/utils/math.js", () => {
  describe("gaussian", () => {
    it("should return a number within reasonable range for positive mean and dev", () => {
      const result = mathUtils.gaussian(100, 10);
      expect(typeof result).toBe("number");
      expect(Number.isFinite(result)).toBe(true);
    });

    it("should apply min constraint when provided", () => {
      const results = new Set();
      for (let i = 0; i < 100; i++) {
        results.add(mathUtils.gaussian(10, 5, 20, 100));
      }
      expect(Math.min(...results)).toBeGreaterThanOrEqual(20);
    });

    it("should apply max constraint when provided", () => {
      const results = new Set();
      for (let i = 0; i < 100; i++) {
        results.add(mathUtils.gaussian(100, 5, 0, 20));
      }
      expect(Math.max(...results)).toBeLessThanOrEqual(20);
    });

    it("should return an integer", () => {
      const result = mathUtils.gaussian(50, 10);
      expect(Number.isInteger(result)).toBe(true);
    });
  });

  describe("randomInRange", () => {
    it("should return integer within range [min, max]", () => {
      const result = mathUtils.randomInRange(5, 10);
      expect(result).toBeGreaterThanOrEqual(5);
      expect(result).toBeLessThanOrEqual(10);
    });

    it("should handle equal min and max", () => {
      const results = new Set();
      for (let i = 0; i < 100; i++) {
        results.add(mathUtils.randomInRange(5, 5));
      }
      expect(results.size).toBe(1);
      expect([...results][0]).toBe(5);
    });

    it("should return an integer", () => {
      const result = mathUtils.randomInRange(0, 100);
      expect(Number.isInteger(result)).toBe(true);
    });

    it("should handle negative ranges", () => {
      const result = mathUtils.randomInRange(-10, -5);
      expect(result).toBeGreaterThanOrEqual(-10);
      expect(result).toBeLessThanOrEqual(-5);
    });
  });

  describe("roll", () => {
    it("should return true for threshold of 1", () => {
      const results = [];
      for (let i = 0; i < 100; i++) {
        results.push(mathUtils.roll(1));
      }
      expect(results.every((r) => r === true)).toBe(true);
    });

    it("should return false for threshold of 0", () => {
      const results = [];
      for (let i = 0; i < 100; i++) {
        results.push(mathUtils.roll(0));
      }
      expect(results.every((r) => r === false)).toBe(true);
    });

    it("should return boolean", () => {
      const result = mathUtils.roll(0.5);
      expect(typeof result).toBe("boolean");
    });
  });

  describe("sample", () => {
    it("should return random element from array", () => {
      const arr = [1, 2, 3, 4, 5];
      const result = mathUtils.sample(arr);
      expect(arr).toContain(result);
    });

    it("should return null for empty array", () => {
      expect(mathUtils.sample([])).toBeNull();
    });

    it("should return null for null input", () => {
      expect(mathUtils.sample(null)).toBeNull();
    });

    it("should return null for undefined input", () => {
      expect(mathUtils.sample(undefined)).toBeNull();
    });

    it("should work with string array", () => {
      const arr = ["a", "b", "c"];
      const result = mathUtils.sample(arr);
      expect(arr).toContain(result);
    });

    it("should work with object array", () => {
      const arr = [{ a: 1 }, { b: 2 }];
      const result = mathUtils.sample(arr);
      expect(arr).toContain(result);
    });
  });

  describe("pidStep", () => {
    it("should update position based on PID output", () => {
      const state = { pos: 0, integral: 0, prevError: 0 };
      const target = 100;
      const model = { Kp: 0.5, Ki: 0.1, Kd: 0.2 };

      const result = mathUtils.pidStep(state, target, model, 0.1);
      expect(state.pos).not.toBe(0);
      expect(typeof result).toBe("number");
    });

    it("should limit integral term to prevent windup", () => {
      const state = { pos: 0, integral: 0, prevError: 0 };
      const target = 1000;
      const model = { Kp: 1, Ki: 1, Kd: 0 };

      for (let i = 0; i < 100; i++) {
        mathUtils.pidStep(state, target, model, 0.1);
      }

      expect(state.integral).toBeLessThanOrEqual(10);
      expect(state.integral).toBeGreaterThanOrEqual(-10);
    });

    it("should store prevError for next iteration", () => {
      const state = { pos: 0, integral: 0, prevError: 0 };
      const target = 100;
      const model = { Kp: 0.5, Ki: 0, Kd: 0 };

      mathUtils.pidStep(state, target, model, 0.1);
      expect(state.prevError).toBe(target - 0);
    });

    it("should use dt in derivative calculation", () => {
      const state1 = { pos: 0, integral: 0, prevError: 0 };
      const state2 = { pos: 0, integral: 0, prevError: 0 };
      const target = 100;
      const model = { Kp: 0.5, Ki: 0, Kd: 0.5 };

      mathUtils.pidStep(state1, target, model, 0.1);
      mathUtils.pidStep(state2, target, model, 1);

      expect(state1.pos).not.toBe(state2.pos);
    });

    it("should return the new position", () => {
      const state = { pos: 0, integral: 0, prevError: 0 };
      const target = 100;
      const model = { Kp: 0.5, Ki: 0, Kd: 0 };

      const result = mathUtils.pidStep(state, target, model, 0.1);

      expect(result).toBe(state.pos);
    });
  });
});
