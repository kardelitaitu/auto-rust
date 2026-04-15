/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 */

/**
 * @fileoverview Performance Benchmarks - Timing
 * Benchmarks for wait, delay, think operations
 * @module tests/benchmarks/timing
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { bench, formatBench } from "./harness.js";

vi.mock("@api/core/context.js", () => ({
  getPage: vi.fn(() => ({ evaluate: () => Promise.resolve({}) })),
}));

vi.mock("@api/behaviors/persona.js", () => ({
  getPersona: vi.fn(() => ({ speed: 1.0 })),
}));

vi.mock("@api/utils/math.js", () => ({
  mathUtils: {
    randomInRange: vi.fn((min, max) => (min + max) / 2),
  },
}));

vi.mock("@api/core/logger.js", () => ({
  createLogger: vi.fn(() => ({
    info: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
    debug: vi.fn(),
  })),
}));

describe("Performance Benchmarks - Timing", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe("wait()", () => {
    it("benchmark: wait 100ms", async () => {
      const { wait } = await import("@api/interactions/wait.js");

      const result = await bench(
        "wait(100ms)",
        async () => {
          await wait(100);
        },
        50,
      );

      console.log(formatBench(result));
      // Actual time will be ~100ms + jitter
      expect(result.avg).toBeLessThan(200);
    });

    it("benchmark: wait 10ms", async () => {
      const { wait } = await import("@api/interactions/wait.js");

      const result = await bench(
        "wait(10ms)",
        async () => {
          await wait(10);
        },
        50,
      );

      console.log(formatBench(result));
      expect(result.avg).toBeLessThan(100);
    });
  });

  describe("delay()", () => {
    it("benchmark: delay 100ms", async () => {
      const { delay } = await import("@api/behaviors/timing.js");

      const result = await bench(
        "delay(100ms)",
        async () => {
          await delay(100);
        },
        50,
      );

      console.log(formatBench(result));
      expect(result.avg).toBeLessThan(200);
    });
  });

  describe("think()", () => {
    it("benchmark: think (random)", async () => {
      const { think } = await import("@api/behaviors/timing.js");

      const result = await bench(
        "think()",
        async () => {
          await think(100); // Small value for benchmark
        },
        20,
      );

      console.log(formatBench(result));
      expect(result.avg).toBeLessThan(200);
    });
  });
});
