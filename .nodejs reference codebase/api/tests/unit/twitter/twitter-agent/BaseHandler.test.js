/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from "vitest";

vi.mock("@api/index.js", () => ({
  api: {
    visible: vi.fn().mockResolvedValue(false),
    wait: vi.fn().mockResolvedValue(undefined),
    eval: vi.fn().mockResolvedValue(""),
    goto: vi.fn().mockResolvedValue(undefined),
    reload: vi.fn().mockResolvedValue(undefined),
    getCurrentUrl: vi.fn().mockResolvedValue("https://x.com"),
    scroll: {
      focus: vi.fn().mockResolvedValue(undefined),
    },
  },
}));

vi.mock("@api/utils/math.js", () => ({
  mathUtils: {
    randomInRange: vi.fn().mockReturnValue(100),
    gaussian: vi.fn().mockReturnValue(0.5),
  },
}));

vi.mock("@api/utils/entropyController.js", () => ({
  entropy: {
    getRandom: vi.fn().mockReturnValue(0.5),
  },
}));

describe("BaseHandler", () => {
  let BaseHandler;
  let mockAgent;

  beforeEach(async () => {
    vi.clearAllMocks();

    const module =
      await import("../../../../twitter/twitter-agent/BaseHandler.js");
    BaseHandler = module.BaseHandler;

    mockAgent = {
      page: {
        url: vi.fn().mockReturnValue("https://x.com"),
        locator: vi.fn().mockReturnValue({
          first: vi.fn().mockReturnValue({
            evaluate: vi.fn().mockResolvedValue(undefined),
          }),
          evaluate: vi.fn().mockResolvedValue(undefined),
          elementHandle: vi.fn().mockResolvedValue({}),
        }),
        evaluate: vi.fn().mockResolvedValue(""),
      },
      config: {
        probabilities: {
          tweetDive: 0.3,
          profileDive: 0.2,
          followOnProfile: 0.1,
        },
        maxSessionDuration: 45 * 60 * 1000,
        getFatiguedVariant: vi.fn().mockReturnValue(null),
        timings: {
          scrollPause: { mean: 1000 },
        },
      },
      logger: {
        info: vi.fn(),
        warn: vi.fn(),
        error: vi.fn(),
        debug: vi.fn(),
      },
      human: {
        think: vi.fn().mockResolvedValue(undefined),
        recoverFromError: vi.fn().mockResolvedValue(undefined),
      },
      ghost: {
        click: vi.fn().mockResolvedValue({ success: true, x: 100, y: 100 }),
      },
      state: {
        consecutiveSoftErrors: 0,
        fatigueBias: 0,
        activityMode: "NORMAL",
      },
      sessionStart: Date.now() - 1000,
      sessionEndTime: null,
      loopIndex: 0,
      isFatigued: false,
      fatigueThreshold: 30 * 60 * 1000,
      lastNetworkActivity: Date.now(),
      mathUtils: {
        randomInRange: vi.fn().mockReturnValue(100),
      },
    };
  });

  describe("constructor", () => {
    it("should create instance with agent properties", () => {
      const handler = new BaseHandler(mockAgent);

      expect(handler.agent).toBe(mockAgent);
      expect(handler.page).toBe(mockAgent.page);
      expect(handler.config).toBe(mockAgent.config);
      expect(handler.logger).toBe(mockAgent.logger);
      expect(handler.human).toBe(mockAgent.human);
      expect(handler.ghost).toBe(mockAgent.ghost);
    });

    it("should set up state getters/setters", () => {
      const handler = new BaseHandler(mockAgent);

      expect(handler.state).toBe(mockAgent.state);
      handler.state = { test: true };
      expect(mockAgent.state).toEqual({ test: true });
    });

    it("should set up sessionStart getter/setter", () => {
      const handler = new BaseHandler(mockAgent);
      const startTime = Date.now() - 5000;

      expect(handler.sessionStart).toBe(mockAgent.sessionStart);
      handler.sessionStart = startTime;
      expect(mockAgent.sessionStart).toBe(startTime);
    });

    it("should set up loopIndex getter/setter", () => {
      const handler = new BaseHandler(mockAgent);

      expect(handler.loopIndex).toBe(mockAgent.loopIndex);
      handler.loopIndex = 5;
      expect(mockAgent.loopIndex).toBe(5);
    });

    it("should set up isFatigued getter/setter", () => {
      const handler = new BaseHandler(mockAgent);

      expect(handler.isFatigued).toBe(mockAgent.isFatigued);
      handler.isFatigued = true;
      expect(mockAgent.isFatigued).toBe(true);
    });

    it("should set up lastNetworkActivity getter/setter", () => {
      const handler = new BaseHandler(mockAgent);
      const time = Date.now();

      expect(handler.lastNetworkActivity).toBe(mockAgent.lastNetworkActivity);
      handler.lastNetworkActivity = time;
      expect(mockAgent.lastNetworkActivity).toBe(time);
    });
  });

  describe("log", () => {
    it("should log using logger.info", () => {
      const handler = new BaseHandler(mockAgent);
      handler.log("Test message");

      expect(mockAgent.logger.info).toHaveBeenCalledWith("Test message");
    });

    it("should fallback to console.log if no logger", () => {
      const consoleSpy = vi.spyOn(console, "log").mockImplementation(() => {});
      mockAgent.logger = null;

      const handler = new BaseHandler(mockAgent);
      handler.log("Test message");

      expect(consoleSpy).toHaveBeenCalledWith("Test message");
      consoleSpy.mockRestore();
    });
  });

  describe("clamp", () => {
    it("should clamp value to min", () => {
      const handler = new BaseHandler(mockAgent);
      expect(handler.clamp(-5, 0, 10)).toBe(0);
    });

    it("should clamp value to max", () => {
      const handler = new BaseHandler(mockAgent);
      expect(handler.clamp(15, 0, 10)).toBe(10);
    });

    it("should return value if within range", () => {
      const handler = new BaseHandler(mockAgent);
      expect(handler.clamp(5, 0, 10)).toBe(5);
    });
  });

  describe("checkFatigue", () => {
    it("should return false when not fatigued", () => {
      const handler = new BaseHandler(mockAgent);
      handler.sessionStart = Date.now();
      handler.fatigueThreshold = 60 * 60 * 1000; // 1 hour

      expect(handler.checkFatigue()).toBe(false);
    });

    it("should set isFatigued and return true when threshold exceeded", () => {
      const handler = new BaseHandler(mockAgent);
      handler.sessionStart = Date.now() - 60 * 60 * 1000; // 1 hour ago
      handler.fatigueThreshold = 30 * 60 * 1000; // 30 minutes

      expect(handler.checkFatigue()).toBe(true);
      expect(handler.isFatigued).toBe(true);
    });
  });

  describe("triggerHotSwap", () => {
    it("should not change config if no fatigued variant", () => {
      mockAgent.config.getFatiguedVariant.mockReturnValue(null);
      const handler = new BaseHandler(mockAgent);
      const originalConfig = handler.config;

      handler.triggerHotSwap();

      expect(handler.config).toBe(originalConfig);
    });

    it("should change config if fatigued variant exists", () => {
      const fatiguedConfig = { timings: { scrollPause: { mean: 2000 } } };
      mockAgent.config.getFatiguedVariant.mockReturnValue(fatiguedConfig);
      const handler = new BaseHandler(mockAgent);

      handler.triggerHotSwap();

      expect(handler.config).toBe(fatiguedConfig);
    });
  });

  describe("getScrollMethod", () => {
    it("should return a valid scroll method", () => {
      const handler = new BaseHandler(mockAgent);
      const validMethods = ["WHEEL_DOWN", "PAGE_DOWN", "ARROW_DOWN"];

      const method = handler.getScrollMethod();

      expect(validMethods).toContain(method);
    });
  });

  describe("normalizeProbabilities", () => {
    it("should merge with default probabilities", () => {
      const handler = new BaseHandler(mockAgent);
      const result = handler.normalizeProbabilities({});

      expect(result).toHaveProperty("tweetDive");
      expect(result).toHaveProperty("profileDive");
      expect(result).toHaveProperty("followOnProfile");
    });

    it("should clamp probabilities to 0-1 range", () => {
      const handler = new BaseHandler(mockAgent);
      const result = handler.normalizeProbabilities({
        tweetDive: 2,
        profileDive: -1,
      });

      expect(result.tweetDive).toBe(1);
      expect(result.profileDive).toBe(0);
    });

    it("should apply fatigue bias when active", () => {
      mockAgent.state.fatigueBias = 0.5;
      const handler = new BaseHandler(mockAgent);
      const result = handler.normalizeProbabilities({});

      expect(result.tweetDive).toBeLessThan(
        mockAgent.config.probabilities.tweetDive,
      );
    });

    it("should apply burst mode adjustments", () => {
      mockAgent.state.activityMode = "BURST";
      const handler = new BaseHandler(mockAgent);
      const result = handler.normalizeProbabilities({});

      expect(result.tweetDive).toBe(0.2);
      expect(result.profileDive).toBe(0.2);
    });
  });

  describe("isSessionExpired", () => {
    it("should return false when session is active", () => {
      const handler = new BaseHandler(mockAgent);
      handler.sessionStart = Date.now();

      expect(handler.isSessionExpired()).toBe(false);
    });

    it("should return true when session exceeded max duration", () => {
      const handler = new BaseHandler(mockAgent);
      handler.sessionStart = Date.now() - 50 * 60 * 1000; // 50 minutes ago

      expect(handler.isSessionExpired()).toBe(true);
    });
  });

  describe("performHealthCheck", () => {
    it("should return healthy when network is active", async () => {
      const handler = new BaseHandler(mockAgent);
      handler.lastNetworkActivity = Date.now();
      const result = await handler.performHealthCheck();
      expect(result.healthy).toBe(true);
    });

    it("should return unhealthy when network is inactive", async () => {
      const handler = new BaseHandler(mockAgent);
      handler.lastNetworkActivity = Date.now() - 60000;
      const result = await handler.performHealthCheck();
      expect(result.healthy).toBe(false);
      expect(result.reason).toContain("network_inactivity");
    });
  });

  describe("isElementActionable", () => {
    it("should return false when element handle is null", async () => {
      const handler = new BaseHandler(mockAgent);
      const mockElement = {
        elementHandle: vi.fn().mockResolvedValue(null),
      };

      const result = await handler.isElementActionable(mockElement);

      expect(result).toBe(false);
    });

    it("should return true when element is actionable", async () => {
      mockAgent.page.evaluate.mockResolvedValue(true);
      const handler = new BaseHandler(mockAgent);
      const mockElement = {
        elementHandle: vi.fn().mockResolvedValue({}),
      };

      const result = await handler.isElementActionable(mockElement);

      expect(result).toBe(true);
    });

    it("should return false when evaluation throws", async () => {
      mockAgent.page.evaluate.mockRejectedValue(new Error("Eval error"));
      const handler = new BaseHandler(mockAgent);
      const mockElement = {
        elementHandle: vi.fn().mockResolvedValue({}),
      };

      const result = await handler.isElementActionable(mockElement);

      expect(result).toBe(false);
    });
  });
});
