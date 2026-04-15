/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * Unit tests for api/tests/integration/index.js
 */

import { describe, it, expect, vi, beforeEach } from "vitest";

// Mock the integration test modules
vi.mock("./agent-connector-health.test.js", () => ({
  default: { describe: vi.fn() },
}));

vi.mock("./circuit-breaker.test.js", () => ({
  default: { describe: vi.fn() },
}));

vi.mock("./request-queue.test.js", () => ({
  default: { describe: vi.fn() },
}));

vi.mock("./cloud-client.test.js", () => ({
  default: { describe: vi.fn() },
}));

vi.mock("./agent-connector.test.js", () => ({
  default: { describe: vi.fn() },
}));

vi.mock("./unified-api.test.js", () => ({
  default: { describe: vi.fn() },
}));

describe("api/tests/integration/index.js", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe("exports", () => {
    it("should export all test modules", async () => {
      const indexModule = await import("./index.js");

      expect(indexModule.agentConnectorHealth).toBeDefined();
      expect(indexModule.circuitBreaker).toBeDefined();
      expect(indexModule.requestQueue).toBeDefined();
      expect(indexModule.cloudClient).toBeDefined();
      expect(indexModule.agentConnector).toBeDefined();
      expect(indexModule.unifiedApi).toBeDefined();
    });

    it("should export runAllIntegrationTests function", async () => {
      const indexModule = await import("./index.js");

      expect(indexModule.runAllIntegrationTests).toBeDefined();
      expect(typeof indexModule.runAllIntegrationTests).toBe("function");
    });
  });

  describe("runAllIntegrationTests", () => {
    it("should return results object with correct structure", async () => {
      const { runAllIntegrationTests } = await import("./index.js");

      const results = await runAllIntegrationTests();

      expect(results).toHaveProperty("passed");
      expect(results).toHaveProperty("failed");
      expect(results).toHaveProperty("tests");
      expect(typeof results.passed).toBe("number");
      expect(typeof results.failed).toBe("number");
      expect(Array.isArray(results.tests)).toBe(true);
    });

    it("should load all test modules successfully", async () => {
      const { runAllIntegrationTests } = await import("./index.js");

      const results = await runAllIntegrationTests();

      // 6 modules: agent-connector-health, circuit-breaker, request-queue, cloud-client, agent-connector, unified-api
      expect(results.passed).toBe(6);
      expect(results.failed).toBe(0);
      expect(results.tests.length).toBe(6);
    });

    it("should include all expected test module names", async () => {
      const { runAllIntegrationTests } = await import("./index.js");

      const results = await runAllIntegrationTests();
      const testNames = results.tests.map((t) => t.name);

      expect(testNames).toContain("agent-connector-health");
      expect(testNames).toContain("circuit-breaker");
      expect(testNames).toContain("request-queue");
      expect(testNames).toContain("cloud-client");
      expect(testNames).toContain("agent-connector");
      expect(testNames).toContain("unified-api");
    });

    it("should mark all tests as loaded status", async () => {
      const { runAllIntegrationTests } = await import("./index.js");

      const results = await runAllIntegrationTests();

      results.tests.forEach((test) => {
        expect(test.status).toBe("loaded");
      });
    });
  });
});
