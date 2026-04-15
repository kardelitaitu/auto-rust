/**
 * Simple Agent Core Integration Test
 * Basic agent functionality
 */

import { describe, it, expect, vi } from "vitest";

vi.mock("@api/core/logger.js", () => ({
  createLogger: vi.fn(() => ({
    info: vi.fn(),
    debug: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
  })),
}));

vi.mock("@api/utils/math.js", () => ({
  mathUtils: {
    randomInRange: vi.fn((min, max) => (min + max) / 2),
    gaussian: vi.fn((mean) => mean),
    roll: vi.fn(() => false),
  },
}));

describe("Simple Agent Core Integration", () => {
  it("should import observer module", async () => {
    const observer = await import("../../agent/observer.js");
    expect(observer.see).toBeDefined();
  });

  it("should import executor module", async () => {
    const executor = await import("../../agent/executor.js");
    expect(executor.doAction).toBeDefined();
  });

  it("should import finder module", async () => {
    const finder = await import("../../agent/finder.js");
    expect(finder.find).toBeDefined();
  });

  it("should import agent runner", async () => {
    const runner = await import("../../agent/runner.js");
    expect(runner.agentRunner).toBeDefined();
  });

  it("should import token counter", async () => {
    const tokenCounter = await import("../../agent/tokenCounter.js");
    expect(tokenCounter.estimateTokens).toBeDefined();
  });
});
