/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * Unit tests for api/tests/unit/index.js
 */

import { describe, it, expect, vi, beforeEach } from "vitest";

// Mock all the imported test modules
vi.mock("./ai-twitter-activity.test.js", () => ({
  default: { describe: vi.fn() },
}));

vi.mock("./ai-twitterAgent.test.js", () => ({
  default: { describe: vi.fn() },
}));

vi.mock("./async-queue.test.js", () => ({
  default: { describe: vi.fn() },
}));

vi.mock("./config-service.test.js", () => ({
  default: { describe: vi.fn() },
}));

vi.mock("./engagement-limits.test.js", () => ({
  default: { describe: vi.fn() },
}));

vi.mock("./human-interaction.test.js", () => ({
  default: { describe: vi.fn() },
}));

describe("unit/index.js", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe("exports", () => {
    it("should export all test modules", async () => {
      const indexModule = await import("./index.js");

      expect(indexModule.aiTwitterActivity).toBeDefined();
      expect(indexModule.aiTwitterAgent).toBeDefined();
      expect(indexModule.asyncQueue).toBeDefined();
      expect(indexModule.configService).toBeDefined();
      expect(indexModule.engagementLimits).toBeDefined();
      expect(indexModule.humanInteraction).toBeDefined();
    });

    it("should export runAllUnitTests function", async () => {
      const indexModule = await import("./index.js");

      expect(indexModule.runAllUnitTests).toBeDefined();
      expect(typeof indexModule.runAllUnitTests).toBe("function");
    });
  });

  describe("runAllUnitTests", () => {
    it("should return results object with correct structure", async () => {
      const { runAllUnitTests } = await import("./index.js");

      const results = await runAllUnitTests();

      expect(results).toHaveProperty("passed");
      expect(results).toHaveProperty("failed");
      expect(results).toHaveProperty("tests");
      expect(typeof results.passed).toBe("number");
      expect(typeof results.failed).toBe("number");
      expect(Array.isArray(results.tests)).toBe(true);
    });

    it("should load all test modules successfully", async () => {
      const { runAllUnitTests } = await import("./index.js");

      const results = await runAllUnitTests();

      // Should have 6 test modules
      expect(results.passed).toBe(6);
      expect(results.failed).toBe(0);
      expect(results.tests.length).toBe(6);
    });

    it("should include all expected test module names", async () => {
      const { runAllUnitTests } = await import("./index.js");

      const results = await runAllUnitTests();
      const testNames = results.tests.map((t) => t.name);

      expect(testNames).toContain("ai-twitter-activity");
      expect(testNames).toContain("ai-twitter-agent");
      expect(testNames).toContain("async-queue");
      expect(testNames).toContain("config-service");
      expect(testNames).toContain("engagement-limits");
      expect(testNames).toContain("human-interaction");
    });

    it("should mark all tests as loaded status", async () => {
      const { runAllUnitTests } = await import("./index.js");

      const results = await runAllUnitTests();

      results.tests.forEach((test) => {
        expect(test.status).toBe("loaded");
      });
    });

    it("should not have any failed tests", async () => {
      const { runAllUnitTests } = await import("./index.js");

      const results = await runAllUnitTests();

      expect(results.failed).toBe(0);
      results.tests.forEach((test) => {
        expect(test.status).not.toBe("failed");
      });
    });
  });
});
