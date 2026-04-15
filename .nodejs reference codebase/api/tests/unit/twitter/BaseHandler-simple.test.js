/**
 * Unit tests for api/twitter/twitter-agent/BaseHandler.js
 * Tests only pure methods without browser dependencies
 */

import { describe, it, expect, vi, beforeEach } from "vitest";

// Mock only the external dependencies
vi.mock("@api/tests/index.js", () => ({
  api: {
    visible: vi.fn().mockResolvedValue(false),
    wait: vi.fn().mockResolvedValue(undefined),
  },
}));

vi.mock("@api/tests/utils/math.js", () => ({
  mathUtils: {
    randomInRange: vi.fn((min, max) => (min + max) / 2),
    gaussian: vi.fn((mean, dev) => mean),
    roll: vi.fn(() => false),
  },
}));

vi.mock("@api/tests/utils/entropyController.js", () => ({
  entropy: {},
}));

import { BaseHandler } from "@api/twitter/twitter-agent/BaseHandler.js";
import { mathUtils } from "@api/tests/utils/math.js";

describe("BaseHandler.js - Pure Methods", () => {
  let mockAgent;
  let handler;

  beforeEach(() => {
    vi.clearAllMocks();

    mockAgent = {
      page: {},
      config: {
        timings: {
          scrollPause: { mean: 1000 },
          readingPhase: { mean: 5000, deviation: 1000 },
        },
        probabilities: {
          tweetDive: 0.3,
          profileDive: 0.2,
          followOnProfile: 0.1,
        },
        maxSessionDuration: 45 * 60 * 1000,
        getFatiguedVariant: vi.fn().mockReturnValue(null),
      },
      logger: { info: vi.fn(), warn: vi.fn(), error: vi.fn() },
      human: { think: vi.fn() },
      ghost: {},
      mathUtils: mathUtils,
      state: {
        consecutiveSoftErrors: 0,
        fatigueBias: 0,
        activityMode: "NORMAL",
      },
      sessionStart: Date.now() - 1000,
      loopIndex: 0,
      isFatigued: false,
      fatigueThreshold: 30 * 60 * 1000,
      lastNetworkActivity: Date.now(),
    };

    handler = new BaseHandler(mockAgent);
  });

  describe("constructor", () => {
    it("should initialize with agent properties", () => {
      expect(handler.agent).toBe(mockAgent);
      expect(handler.page).toBe(mockAgent.page);
      expect(handler.config).toBe(mockAgent.config);
      expect(handler.logger).toBe(mockAgent.logger);
    });
  });

  describe("getters and setters", () => {
    it("should get/set state", () => {
      expect(handler.state).toBe(mockAgent.state);

      const newState = { test: "value" };
      handler.state = newState;
      expect(mockAgent.state).toBe(newState);
    });

    it("should get/set sessionStart", () => {
      expect(handler.sessionStart).toBe(mockAgent.sessionStart);

      handler.sessionStart = 12345;
      expect(mockAgent.sessionStart).toBe(12345);
    });

    it("should get/set loopIndex", () => {
      expect(handler.loopIndex).toBe(mockAgent.loopIndex);

      handler.loopIndex = 5;
      expect(mockAgent.loopIndex).toBe(5);
    });

    it("should get/set isFatigued", () => {
      expect(handler.isFatigued).toBe(mockAgent.isFatigued);

      handler.isFatigued = true;
      expect(mockAgent.isFatigued).toBe(true);
    });

    it("should get/set fatigueThreshold", () => {
      expect(handler.fatigueThreshold).toBe(mockAgent.fatigueThreshold);

      handler.fatigueThreshold = 60000;
      expect(mockAgent.fatigueThreshold).toBe(60000);
    });

    it("should get/set lastNetworkActivity", () => {
      expect(handler.lastNetworkActivity).toBe(mockAgent.lastNetworkActivity);

      handler.lastNetworkActivity = 11111;
      expect(mockAgent.lastNetworkActivity).toBe(11111);
    });
  });

  describe("log", () => {
    it("should log via logger.info", () => {
      handler.log("test message");

      expect(handler.logger.info).toHaveBeenCalledWith("test message");
    });

    it("should fallback to console.log when no logger", () => {
      const consoleSpy = vi.spyOn(console, "log").mockImplementation(() => {});
      handler.logger = null;

      handler.log("fallback");

      expect(consoleSpy).toHaveBeenCalledWith("fallback");
      consoleSpy.mockRestore();
    });
  });

  describe("clamp", () => {
    it("should return value within range", () => {
      expect(handler.clamp(5, 0, 10)).toBe(5);
    });

    it("should clamp to min", () => {
      expect(handler.clamp(-5, 0, 10)).toBe(0);
    });

    it("should clamp to max", () => {
      expect(handler.clamp(15, 0, 10)).toBe(10);
    });
  });

  describe("checkFatigue", () => {
    it("should return false when not fatigued", () => {
      handler.sessionStart = Date.now();
      handler.fatigueThreshold = 60000;

      expect(handler.checkFatigue()).toBe(false);
      expect(handler.isFatigued).toBe(false);
    });

    it("should return true when threshold exceeded", () => {
      handler.sessionStart = Date.now() - 61000;
      handler.fatigueThreshold = 60000;

      expect(handler.checkFatigue()).toBe(true);
      expect(handler.isFatigued).toBe(true);
    });
  });

  describe("triggerHotSwap", () => {
    it("should call config.getFatiguedVariant", () => {
      handler.triggerHotSwap();

      expect(handler.config.getFatiguedVariant).toHaveBeenCalled();
    });

    it("should update config when variant returned", () => {
      const slowerProfile = { timings: { scrollPause: { mean: 2000 } } };
      handler.config.getFatiguedVariant.mockReturnValue(slowerProfile);

      handler.triggerHotSwap();

      expect(handler.config).toBe(slowerProfile);
    });

    it("should not update config when no variant", () => {
      handler.config.getFatiguedVariant.mockReturnValue(null);
      const originalConfig = handler.config;

      handler.triggerHotSwap();

      expect(handler.config).toBe(originalConfig);
    });
  });

  describe("getScrollMethod", () => {
    it("should return valid scroll method", () => {
      const validMethods = ["WHEEL_DOWN", "PAGE_DOWN", "ARROW_DOWN"];

      for (let i = 0; i < 10; i++) {
        expect(validMethods).toContain(handler.getScrollMethod());
      }
    });
  });

  describe("normalizeProbabilities", () => {
    it("should merge with config probabilities", () => {
      const result = handler.normalizeProbabilities({ newProb: 0.5 });

      expect(result.newProb).toBe(0.5);
      expect(result.tweetDive).toBe(0.3);
    });

    it("should clamp values to 0-1 range", () => {
      const result = handler.normalizeProbabilities({ test: 1.5, test2: -0.5 });

      expect(result.test).toBe(1);
      expect(result.test2).toBe(0);
    });

    it("should apply fatigue bias", () => {
      handler.state.fatigueBias = 0.5;

      const result = handler.normalizeProbabilities({});

      expect(result.tweetDive).toBe(0.3 * 0.7);
      expect(result.profileDive).toBe(0.2 * 0.7);
      expect(result.followOnProfile).toBe(0.1 * 0.5);
    });

    it("should apply burst mode adjustments", () => {
      handler.state.activityMode = "BURST";

      const result = handler.normalizeProbabilities({});

      expect(result.tweetDive).toBe(0.2);
      expect(result.profileDive).toBe(0.2);
    });
  });

  describe("isSessionExpired", () => {
    it("should return false when within duration", () => {
      handler.sessionStart = Date.now() - 1000;
      handler.config.maxSessionDuration = 60000;

      expect(handler.isSessionExpired()).toBe(false);
    });

    it("should return true when exceeded", () => {
      handler.sessionStart = Date.now() - 61000;
      handler.config.maxSessionDuration = 60000;

      expect(handler.isSessionExpired()).toBe(true);
    });

    it("should use default 45 minutes when not configured", () => {
      handler.sessionStart = Date.now() - 46 * 60 * 1000;
      handler.config.maxSessionDuration = undefined;

      expect(handler.isSessionExpired()).toBe(true);
    });
  });
});
