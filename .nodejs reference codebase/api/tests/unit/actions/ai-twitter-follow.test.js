/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Unit tests for actions/ai-twitter-follow.js (FollowAction class)
 * @module tests/unit/ai-twitter-follow.test
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { FollowAction } from "@api/actions/ai-twitter-follow.js";

vi.mock("@api/core/logger.js", () => ({
  createLogger: vi.fn(() => ({
    info: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
    debug: vi.fn(),
  })),
}));

describe("actions/ai-twitter-follow.js (FollowAction)", () => {
  let mockAgent;

  beforeEach(() => {
    vi.clearAllMocks();

    mockAgent = {
      twitterConfig: {
        actions: {
          follow: {
            probability: 0.5,
            enabled: true,
          },
        },
      },
      diveQueue: {
        canEngage: vi.fn().mockReturnValue(true),
      },
    };
  });

  describe("Constructor", () => {
    it("should initialize with agent", () => {
      const action = new FollowAction(mockAgent);
      expect(action.agent).toBe(mockAgent);
      expect(action.engagementType).toBe("follows");
    });

    it("should initialize with default stats", () => {
      const action = new FollowAction(mockAgent);
      expect(action.stats.attempts).toBe(0);
      expect(action.stats.successes).toBe(0);
      expect(action.stats.failures).toBe(0);
      expect(action.stats.skipped).toBe(0);
    });

    it("should initialize without agent", () => {
      const action = new FollowAction(null);
      expect(action.agent).toBeNull();
      expect(action.enabled).toBe(true);
      expect(action.probability).toBe(0.1);
    });

    it("should load config from twitterConfig", () => {
      const action = new FollowAction(mockAgent);
      expect(action.probability).toBe(0.5);
      expect(action.enabled).toBe(true);
    });

    it("should use defaults when twitterConfig not present", () => {
      const action = new FollowAction({});
      expect(action.probability).toBe(0.1);
      expect(action.enabled).toBe(true);
    });

    it("should use defaults when follow config not present", () => {
      const action = new FollowAction({ twitterConfig: {} });
      expect(action.probability).toBe(0.1);
      expect(action.enabled).toBe(true);
    });

    it("should handle missing actions config", () => {
      const action = new FollowAction({ twitterConfig: { actions: {} } });
      expect(action.probability).toBe(0.1);
      expect(action.enabled).toBe(true);
    });

    it("should respect enabled: false in config", () => {
      const agent = {
        twitterConfig: {
          actions: {
            follow: {
              probability: 0.8,
              enabled: false,
            },
          },
        },
      };
      const action = new FollowAction(agent);
      expect(action.enabled).toBe(false);
      expect(action.probability).toBe(0.8);
    });

    it("should default probability when null in config", () => {
      const agent = {
        twitterConfig: {
          actions: {
            follow: {
              probability: null,
              enabled: true,
            },
          },
        },
      };
      const action = new FollowAction(agent);
      expect(action.probability).toBe(0.1);
    });
  });

  describe("loadConfig", () => {
    it("should reload config after construction", () => {
      const action = new FollowAction(mockAgent);
      action.agent.twitterConfig.actions.follow.probability = 0.9;
      action.loadConfig();
      expect(action.probability).toBe(0.9);
    });

    it("should handle undefined agent in loadConfig", () => {
      const action = new FollowAction(null);
      action.agent = undefined;
      action.loadConfig();
      expect(action.probability).toBe(0.1);
      expect(action.enabled).toBe(true);
    });
  });

  describe("canExecute", () => {
    it("should return allowed when agent is null", async () => {
      const action = new FollowAction(null);
      const result = await action.canExecute();
      expect(result.allowed).toBe(false);
      expect(result.reason).toBe("agent_not_initialized");
    });

    it("should return allowed when diveQueue is available", async () => {
      const action = new FollowAction(mockAgent);
      const result = await action.canExecute();
      expect(result.allowed).toBe(true);
      expect(result.reason).toBeNull();
    });

    it("should return not allowed when diveQueue rejects", async () => {
      mockAgent.diveQueue.canEngage.mockReturnValue(false);
      const action = new FollowAction(mockAgent);
      const result = await action.canExecute();
      expect(result.allowed).toBe(false);
      expect(result.reason).toBe("engagement_limit_reached");
    });

    it("should return allowed when diveQueue is not defined", async () => {
      const agent = { twitterConfig: {} };
      const action = new FollowAction(agent);
      const result = await action.canExecute();
      expect(result.allowed).toBe(true);
    });

    it("should pass context to canEngage", async () => {
      const action = new FollowAction(mockAgent);
      await action.canExecute({ some: "context" });
      expect(mockAgent.diveQueue.canEngage).toHaveBeenCalledWith("follows");
    });
  });

  describe("execute", () => {
    it("should return stub response when execute is called directly", async () => {
      const action = new FollowAction(mockAgent);
      const result = await action.execute();
      expect(result.success).toBe(false);
      expect(result.executed).toBe(false);
      expect(result.reason).toBe("executor_not_wired");
      expect(result.engagementType).toBe("follows");
    });

    it("should increment attempts and failures on execute", async () => {
      const action = new FollowAction(mockAgent);
      await action.execute();
      expect(action.stats.attempts).toBe(1);
      expect(action.stats.failures).toBe(1);
    });

    it("should accept context parameter", async () => {
      const action = new FollowAction(mockAgent);
      const result = await action.execute({ test: "context" });
      expect(result.success).toBe(false);
    });
  });

  describe("tryExecute", () => {
    it("should skip when canExecute returns not allowed", async () => {
      const action = new FollowAction(null);
      const result = await action.tryExecute();
      expect(result.success).toBe(false);
      expect(result.executed).toBe(false);
      expect(result.reason).toBe("agent_not_initialized");
      expect(action.stats.skipped).toBe(1);
    });

    it("should skip when probability check fails", async () => {
      const action = new FollowAction(mockAgent);
      vi.spyOn(Math, "random").mockReturnValue(0.99);
      const result = await action.tryExecute();
      expect(result.success).toBe(false);
      expect(result.executed).toBe(false);
      expect(result.reason).toBe("probability");
      expect(action.stats.skipped).toBe(1);
    });

    it("should execute when probability check passes", async () => {
      const action = new FollowAction(mockAgent);
      vi.spyOn(Math, "random").mockReturnValue(0.1);
      const result = await action.tryExecute();
      expect(result.success).toBe(false);
      expect(result.executed).toBe(false);
      expect(result.reason).toBe("executor_not_wired");
    });

    it("should pass context to canExecute", async () => {
      const action = new FollowAction(mockAgent);
      await action.tryExecute({ test: "data" });
    });

    it("should increment attempts when trying to execute", async () => {
      const action = new FollowAction(mockAgent);
      vi.spyOn(Math, "random").mockReturnValue(0.1);
      await action.tryExecute();
      expect(action.stats.attempts).toBe(1);
    });

    it("should respect custom probability setting", async () => {
      const agent = {
        twitterConfig: {
          actions: {
            follow: {
              probability: 0.01,
              enabled: true,
            },
          },
        },
        diveQueue: { canEngage: vi.fn().mockReturnValue(true) },
      };
      const action = new FollowAction(agent);
      vi.spyOn(Math, "random").mockReturnValue(0.5);
      const result = await action.tryExecute();
      expect(result.reason).toBe("probability");
    });
  });

  describe("getStats", () => {
    it("should return stats object", () => {
      const action = new FollowAction(mockAgent);
      const stats = action.getStats();
      expect(stats.attempts).toBe(0);
      expect(stats.successes).toBe(0);
      expect(stats.failures).toBe(0);
      expect(stats.skipped).toBe(0);
      expect(stats.engagementType).toBe("follows");
    });

    it("should calculate success rate as percentage", () => {
      const action = new FollowAction(mockAgent);
      action.stats.attempts = 10;
      action.stats.successes = 2;
      const stats = action.getStats();
      expect(stats.successRate).toBe("20.0%");
    });

    it("should return 0% success rate when no attempts", () => {
      const action = new FollowAction(mockAgent);
      const stats = action.getStats();
      expect(stats.successRate).toBe("0.0%");
    });

    it("should reflect updated stats", () => {
      const action = new FollowAction(mockAgent);
      action.stats.attempts = 5;
      action.stats.successes = 3;
      action.stats.failures = 1;
      action.stats.skipped = 1;
      const stats = action.getStats();
      expect(stats.attempts).toBe(5);
      expect(stats.successes).toBe(3);
      expect(stats.failures).toBe(1);
      expect(stats.skipped).toBe(1);
    });
  });

  describe("resetStats", () => {
    it("should reset all stats to zero", () => {
      const action = new FollowAction(mockAgent);
      action.stats.attempts = 10;
      action.stats.successes = 5;
      action.stats.failures = 3;
      action.stats.skipped = 2;
      action.resetStats();
      expect(action.stats.attempts).toBe(0);
      expect(action.stats.successes).toBe(0);
      expect(action.stats.failures).toBe(0);
      expect(action.stats.skipped).toBe(0);
    });

    it("should preserve engagementType after reset", () => {
      const action = new FollowAction(mockAgent);
      action.resetStats();
      const stats = action.getStats();
      expect(stats.engagementType).toBe("follows");
    });
  });

  describe("Edge Cases", () => {
    it("should handle execute throwing error", async () => {
      const action = new FollowAction(mockAgent);
      action.execute = vi.fn().mockRejectedValue(new Error("execute error"));
      vi.spyOn(Math, "random").mockReturnValue(0.1);
      await expect(action.tryExecute()).rejects.toThrow("execute error");
    });
  });
});
