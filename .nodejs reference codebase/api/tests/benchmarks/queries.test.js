/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 */

/**
 * @fileoverview Performance Benchmarks - Queries
 * Benchmarks for text, attr, visible, count operations
 * @module tests/benchmarks/queries
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { bench, formatBench, createMockPage } from "./harness.js";

vi.mock("@api/core/context.js", () => ({
  getPage: vi.fn(() => mockPage),
  isSessionActive: vi.fn(() => true),
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

describe("Performance Benchmarks - Queries", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe("text()", () => {
    it("benchmark: get element text", async () => {
      const { text } = await import("@api/interactions/queries.js");

      const result = await bench(
        "text(element)",
        async () => {
          await text(".element");
        },
        100,
      );

      console.log(formatBench(result));
      expect(result.avg).toBeLessThan(50);
    });
  });

  describe("attr()", () => {
    it("benchmark: get element attribute", async () => {
      const { attr } = await import("@api/interactions/queries.js");

      const result = await bench(
        "attr(element, name)",
        async () => {
          await attr("a", "href");
        },
        100,
      );

      console.log(formatBench(result));
      expect(result.avg).toBeLessThan(50);
    });
  });

  describe("visible()", () => {
    it("benchmark: check element visibility", async () => {
      const { visible } = await import("@api/interactions/queries.js");

      const result = await bench(
        "visible(element)",
        async () => {
          await visible(".element");
        },
        100,
      );

      console.log(formatBench(result));
      expect(result.avg).toBeLessThan(50);
    });
  });

  describe("count()", () => {
    it("benchmark: count elements", async () => {
      const { count } = await import("@api/interactions/queries.js");

      const result = await bench(
        "count(selector)",
        async () => {
          await count(".item");
        },
        100,
      );

      console.log(formatBench(result));
      expect(result.avg).toBeLessThan(50);
    });
  });

  describe("exists()", () => {
    it("benchmark: check element exists", async () => {
      const { exists } = await import("@api/interactions/queries.js");

      const result = await bench(
        "exists(selector)",
        async () => {
          await exists(".element");
        },
        100,
      );

      console.log(formatBench(result));
      expect(result.avg).toBeLessThan(50);
    });
  });
});
