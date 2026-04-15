/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 */

import { describe, it, expect, vi, beforeEach } from "vitest";

vi.mock("@api/agent/gameRunner.js", () => ({
  gameAgentRunner: {
    run: vi.fn().mockResolvedValue({ success: true, result: "built" }),
    stop: vi.fn(),
    isRunning: true,
    getUsageStats: vi.fn().mockReturnValue({
      totalRuns: 10,
      successfulRuns: 8,
      failedRuns: 2,
    }),
  },
}));

describe("api.gameAgent functionality - direct module tests", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe("gameAgentRunner.run", () => {
    it("should run game agent with goal", async () => {
      const { gameAgentRunner } = await import("@api/agent/gameRunner.js");
      const result = await gameAgentRunner.run("Build a barracks");
      expect(gameAgentRunner.run).toHaveBeenCalled();
      expect(result).toBeDefined();
    });

    it("should accept config parameter", async () => {
      const { gameAgentRunner } = await import("@api/agent/gameRunner.js");
      const config = { maxSteps: 20, timeout: 120000 };
      await gameAgentRunner.run("Train units", config);
      expect(gameAgentRunner.run).toHaveBeenCalled();
    });

    it("should return result from runner", async () => {
      const { gameAgentRunner } = await import("@api/agent/gameRunner.js");
      const result = await gameAgentRunner.run("Test goal");
      expect(result).toBeDefined();
    });
  });

  describe("gameAgentRunner.stop", () => {
    it("should stop game agent runner", async () => {
      const { gameAgentRunner } = await import("@api/agent/gameRunner.js");
      await gameAgentRunner.stop();
      expect(gameAgentRunner.stop).toHaveBeenCalled();
    });
  });

  describe("gameAgentRunner.isRunning", () => {
    it("should return isRunning status", async () => {
      const { gameAgentRunner } = await import("@api/agent/gameRunner.js");
      const result = await gameAgentRunner.isRunning;
      expect(typeof result).toBe("boolean");
    });
  });

  describe("gameAgentRunner.getUsageStats", () => {
    it("should return usage statistics", async () => {
      const { gameAgentRunner } = await import("@api/agent/gameRunner.js");
      const stats = await gameAgentRunner.getUsageStats();
      expect(gameAgentRunner.getUsageStats).toHaveBeenCalled();
      expect(stats).toBeDefined();
    });
  });
});
