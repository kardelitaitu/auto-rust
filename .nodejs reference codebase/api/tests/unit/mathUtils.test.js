/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, afterEach } from "vitest";
import { mathUtils } from "@api/utils/math.js";

describe("mathUtils", () => {
  describe("gaussian", () => {
    it("should return value within bounds", () => {
      for (let i = 0; i < 100; i++) {
        const val = mathUtils.gaussian(100, 10, 80, 120);
        expect(val).toBeGreaterThanOrEqual(80);
        expect(val).toBeLessThanOrEqual(120);
      }
    });

    it("should handle zero random values and apply min only", () => {
      vi.spyOn(Math, "random")
        .mockReturnValueOnce(0)
        .mockReturnValueOnce(0.5)
        .mockReturnValueOnce(0)
        .mockReturnValueOnce(0.5);
      const val = mathUtils.gaussian(10, 5, 12);
      expect(val).toBeGreaterThanOrEqual(12);
    });

    it("should apply max only when provided", () => {
      vi.spyOn(Math, "random")
        .mockReturnValueOnce(0.5)
        .mockReturnValueOnce(0.5);
      const val = mathUtils.gaussian(100, 100, undefined, 50);
      expect(val).toBeLessThanOrEqual(50);
    });

    it("should return floored value without bounds", () => {
      vi.spyOn(Math, "random")
        .mockReturnValueOnce(0.25)
        .mockReturnValueOnce(0.75);
      const val = mathUtils.gaussian(0, 1);
      expect(Number.isInteger(val)).toBe(true);
    });

    it("should apply both min and max bounds together", () => {
      vi.spyOn(Math, "random")
        .mockReturnValueOnce(0.5)
        .mockReturnValueOnce(0.5);
      const val = mathUtils.gaussian(100, 100, 10, 20);
      expect(val).toBeGreaterThanOrEqual(10);
      expect(val).toBeLessThanOrEqual(20);
    });

    afterEach(() => {
      vi.restoreAllMocks();
    });
  });

  describe("randomInRange", () => {
    it("should return value within bounds", () => {
      for (let i = 0; i < 100; i++) {
        const val = mathUtils.randomInRange(10, 20);
        expect(val).toBeGreaterThanOrEqual(10);
        expect(val).toBeLessThanOrEqual(20);
      }
    });

    it("should return min when random is 0", () => {
      vi.spyOn(Math, "random").mockReturnValue(0);
      expect(mathUtils.randomInRange(5, 9)).toBe(5);
      vi.restoreAllMocks();
    });

    it("should return max when random is near 1", () => {
      vi.spyOn(Math, "random").mockReturnValue(0.999999);
      expect(mathUtils.randomInRange(5, 9)).toBe(9);
      vi.restoreAllMocks();
    });
  });

  describe("roll", () => {
    it("should return true if under threshold", () => {
      vi.spyOn(Math, "random").mockReturnValue(0.4);
      expect(mathUtils.roll(0.5)).toBe(true);
    });

    it("should return false if over threshold", () => {
      vi.spyOn(Math, "random").mockReturnValue(0.6);
      expect(mathUtils.roll(0.5)).toBe(false);
    });

    it("should return false when threshold is 0", () => {
      vi.spyOn(Math, "random").mockReturnValue(0.0);
      expect(mathUtils.roll(0)).toBe(false);
    });

    it("should return true when threshold is 1", () => {
      vi.spyOn(Math, "random").mockReturnValue(0.9999);
      expect(mathUtils.roll(1)).toBe(true);
    });

    afterEach(() => {
      vi.restoreAllMocks();
    });
  });

  describe("sample", () => {
    it("should return random element", () => {
      const arr = [1, 2, 3];
      vi.spyOn(Math, "random").mockReturnValue(0.5); // Index 1
      expect(mathUtils.sample(arr)).toBe(2);
    });

    it("should return null for empty array", () => {
      expect(mathUtils.sample([])).toBeNull();
      expect(mathUtils.sample(null)).toBeNull();
    });

    it("should return first element when random is 0", () => {
      const arr = [9, 8, 7];
      vi.spyOn(Math, "random").mockReturnValue(0);
      expect(mathUtils.sample(arr)).toBe(9);
      vi.restoreAllMocks();
    });

    it("should return last element when random is near 1", () => {
      const arr = [9, 8, 7];
      vi.spyOn(Math, "random").mockReturnValue(0.9999);
      expect(mathUtils.sample(arr)).toBe(7);
      vi.restoreAllMocks();
    });

    it("should return the only element in a single-item array", () => {
      const arr = [42];
      vi.spyOn(Math, "random").mockReturnValue(0.75);
      expect(mathUtils.sample(arr)).toBe(42);
      vi.restoreAllMocks();
    });

    it("should handle nullish input explicitly", () => {
      expect(mathUtils.sample(undefined)).toBeNull();
      expect(mathUtils.sample("")).toBeNull();
    });
  });

  describe("pidStep", () => {
    it("should calculate PID control output", () => {
      const state = { pos: 100 };
      const target = 150;
      const model = { Kp: 1, Ki: 0.1, Kd: 0.5 };

      const result = mathUtils.pidStep(state, target, model, 0.1);

      expect(result).toBeDefined();
      expect(typeof result).toBe("number");
    });

    it("should handle integral windup limits", () => {
      const state = { pos: 0, integral: 100 };
      const target = 1000;
      const model = { Kp: 1, Ki: 1, Kd: 0 };

      const result = mathUtils.pidStep(state, target, model, 0.1);

      expect(state.integral).toBeLessThanOrEqual(10);
      expect(state.integral).toBeGreaterThanOrEqual(-10);
    });

    it("should handle initial state without prevError", () => {
      const state = { pos: 50 };
      const target = 100;
      const model = { Kp: 1, Ki: 0, Kd: 1 };

      const result = mathUtils.pidStep(state, target, model, 0.1);

      expect(typeof result).toBe("number");
    });

    it("should update prevError and pos on successive steps", () => {
      const state = { pos: 10 };
      const model = { Kp: 1, Ki: 0, Kd: 0 };

      const first = mathUtils.pidStep(state, 15, model, 0.1);
      const second = mathUtils.pidStep(state, 20, model, 0.1);

      expect(first).toBe(15);
      expect(second).toBe(20);
      expect(state.prevError).toBe(5);
    });
  });
});
