/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * Unit tests for api/tests/index.js
 */

import { describe, it, expect, vi, beforeEach } from "vitest";

// Mock the sub-modules
vi.mock("./unit/index.js", () => ({
  runAllUnitTests: vi.fn().mockResolvedValue({
    passed: 6,
    failed: 0,
    tests: [{ name: "test1", status: "loaded" }],
  }),
}));

vi.mock("./integration/index.js", () => ({
  runAllIntegrationTests: vi.fn().mockResolvedValue({
    passed: 3,
    failed: 0,
    tests: [{ name: "test1", status: "loaded" }],
  }),
}));

vi.mock("./edge-cases/index.js", () => ({
  runAllEdgeCaseTests: vi.fn().mockResolvedValue({
    passed: 0,
    failed: 0,
    tests: [],
  }),
}));

describe("api/tests/index.js", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe("exports", () => {
    it("should export runAllTests function", async () => {
      const indexModule = await import("./index.js");

      expect(indexModule.runAllTests).toBeDefined();
      expect(typeof indexModule.runAllTests).toBe("function");
    });

    it("should export testStats object", async () => {
      const indexModule = await import("./index.js");

      expect(indexModule.testStats).toBeDefined();
      expect(indexModule.testStats.unit).toBeDefined();
      expect(indexModule.testStats.integration).toBeDefined();
      expect(indexModule.testStats.edgeCases).toBeDefined();
    });

    it("should have testStats with correct structure", async () => {
      const indexModule = await import("./index.js");

      expect(indexModule.testStats.unit.count).toBe(11);
      expect(indexModule.testStats.unit.category).toBe("Unit Tests");
      expect(indexModule.testStats.integration.count).toBe(10);
      expect(indexModule.testStats.edgeCases.count).toBe(4);
      expect(indexModule.testStats.total).toBe(25);
    });
  });

  describe("runAllTests", () => {
    it("should return results object with correct structure", async () => {
      const { runAllTests } = await import("./index.js");

      const results = await runAllTests();

      expect(results).toHaveProperty("unit");
      expect(results).toHaveProperty("integration");
      expect(results).toHaveProperty("edgeCases");
      expect(results).toHaveProperty("total");
      expect(results).toHaveProperty("failed");
      expect(results).toHaveProperty("success");
    });

    it("should calculate total correctly", async () => {
      const { runAllTests } = await import("./index.js");

      const results = await runAllTests();

      // 6 + 3 + 0 = 9
      expect(results.total).toBe(9);
    });

    it("should calculate failed correctly", async () => {
      const { runAllTests } = await import("./index.js");

      const results = await runAllTests();

      expect(results.failed).toBe(0);
    });

    it("should set success to true when no failures", async () => {
      const { runAllTests } = await import("./index.js");

      const results = await runAllTests();

      expect(results.success).toBe(true);
    });

    it("should call all sub-module run functions", async () => {
      const { runAllTests } = await import("./index.js");
      const { runAllUnitTests } = await import("./unit/index.js");
      const { runAllIntegrationTests } = await import("./integration/index.js");
      const { runAllEdgeCaseTests } = await import("./edge-cases/index.js");

      await runAllTests();

      expect(runAllUnitTests).toHaveBeenCalled();
      expect(runAllIntegrationTests).toHaveBeenCalled();
      expect(runAllEdgeCaseTests).toHaveBeenCalled();
    });
  });
});
