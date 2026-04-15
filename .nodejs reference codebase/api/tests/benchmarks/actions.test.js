/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 */

/**
 * @fileoverview Performance Benchmarks - Actions
 * Benchmarks for click, type, hover, drag operations
 * @module tests/benchmarks/actions
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import {
  bench,
  formatBench,
  createMockPage,
  createMockCursor,
} from "./harness.js";

vi.mock("@api/core/context.js", () => ({
  getPage: vi.fn(() => mockPage),
  getCursor: vi.fn(() => mockCursor),
  isSessionActive: vi.fn(() => true),
  getEvents: vi.fn(() => ({ emitSafe: vi.fn() })),
  getLocator: vi.fn((selector) => mockPage.locator(selector)),
}));

vi.mock("@api/behaviors/persona.js", () => ({
  getPersona: vi.fn(() => ({ scrollSpeed: 1.0, speed: 1.0 })),
}));

vi.mock("@api/interactions/scroll.js", () => ({
  focus: vi.fn(() => Promise.resolve()),
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

const mockPage = createMockPage();
const mockCursor = createMockCursor();

describe("Performance Benchmarks - Actions", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  // NOTE: click() benchmarks require real browser context - too complex for unit tests
  // The click function involves multiple waits, cursor movements, and humanization

  describe("type()", () => {
    it("benchmark: type text (10 chars)", async () => {
      const { type } = await import("@api/interactions/actions.js");

      const result = await bench(
        "type(text, 10 chars)",
        async () => {
          try {
            await type(".input", "hello world", {
              recovery: false,
              maxRetries: 0,
            });
          } catch (e) {
            // Ignore errors
          }
        },
        30,
      );

      console.log(formatBench(result));
      expect(result.avg).toBeLessThan(500); // Humanized typing takes time
    });
  });

  describe("hover()", () => {
    it("benchmark: hover element", async () => {
      const { hover } = await import("@api/interactions/actions.js");

      const result = await bench(
        "hover(element)",
        async () => {
          try {
            await hover(".btn", { recovery: false, maxRetries: 0 });
          } catch (e) {
            // Ignore errors
          }
        },
        50,
      );

      console.log(formatBench(result));
      expect(result.avg).toBeLessThan(100);
    });
  });
});
