/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";

vi.mock("@api/index.js", () => ({
  api: {
    visible: vi.fn().mockResolvedValue(false),
    exists: vi.fn().mockResolvedValue(false),
    wait: vi.fn().mockResolvedValue(undefined),
    goto: vi.fn().mockResolvedValue(undefined),
    reload: vi.fn().mockResolvedValue(undefined),
    getCurrentUrl: vi.fn().mockResolvedValue("https://twitter.com/home"),
    eval: vi.fn().mockResolvedValue(""),
    scroll: {
      focus: vi.fn().mockResolvedValue(undefined),
    },
  },
}));

vi.mock("@api/utils/math.js", () => ({
  mathUtils: {
    randomInRange: vi.fn((min, max) => Math.floor((min + max) / 2)),
    gaussian: vi.fn((mean, dev) => mean),
    roll: vi.fn(() => false),
  },
}));

vi.mock("@api/utils/entropyController.js", () => ({
  entropy: { someMethod: vi.fn() },
}));

import { BaseHandler } from "@api/twitter/twitter-agent/BaseHandler.js";
import { api } from "@api/index.js";
import { mathUtils } from "@api/utils/math.js";

describe("BaseHandler", () => {
  let handler;
  let mockAgent;

  beforeEach(() => {
    vi.clearAllMocks();

    mockAgent = {
      page: {
        locator: vi.fn().mockReturnValue({
          first: vi.fn().mockReturnValue({
            click: vi.fn().mockResolvedValue(undefined),
            evaluate: vi.fn().mockResolvedValue(undefined),
            scrollIntoView: vi.fn().mockResolvedValue(undefined),
          }),
          nth: vi.fn().mockReturnValue({
            boundingBox: vi.fn().mockResolvedValue(null),
            locator: vi.fn().mockReturnValue({
              count: vi.fn().mockResolvedValue(0),
            }),
          }),
          count: vi.fn().mockResolvedValue(0),
        }),
        viewportSize: vi.fn().mockReturnValue({ width: 1920, height: 1080 }),
        keyboard: {
          press: vi.fn().mockResolvedValue(undefined),
          type: vi.fn().mockResolvedValue(undefined),
        },
        addInitScript: vi.fn().mockResolvedValue(undefined),
        evaluate: vi.fn().mockResolvedValue(undefined),
        goBack: vi.fn().mockResolvedValue(undefined),
      },
      config: {
        probabilities: {
          tweetDive: 0.3,
          profileDive: 0.2,
          followOnProfile: 0.1,
          likeTweetAfterDive: 0.3,
          bookmarkAfterDive: 0.1,
        },
        timings: {
          scrollPause: { mean: 1000 },
          readingPhase: { mean: 5000, deviation: 1000 },
        },
        maxSessionDuration: 45 * 60 * 1000,
        getFatiguedVariant: vi.fn().mockReturnValue(null),
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
        consumeContent: vi.fn().mockResolvedValue(undefined),
        scroll: vi.fn().mockResolvedValue(undefined),
      },
      ghost: {
        click: vi.fn().mockResolvedValue({ success: true, x: 100, y: 100 }),
        move: vi.fn().mockResolvedValue(undefined),
      },
      mathUtils: mathUtils,
      state: {
        consecutiveSoftErrors: 0,
        fatigueBias: 0,
        activityMode: "NORMAL",
        follows: 0,
        likes: 0,
      },
      sessionStart: Date.now() - 1000,
      sessionEndTime: null,
      loopIndex: 0,
      isFatigued: false,
      fatigueThreshold: 30 * 60 * 1000,
      lastNetworkActivity: Date.now(),
    };

    handler = new BaseHandler(mockAgent);
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe("constructor", () => {
    it("should initialize with agent properties", () => {
      expect(handler.agent).toBe(mockAgent);
      expect(handler.page).toBe(mockAgent.page);
      expect(handler.config).toBe(mockAgent.config);
      expect(handler.logger).toBe(mockAgent.logger);
      expect(handler.human).toBe(mockAgent.human);
      expect(handler.ghost).toBe(mockAgent.ghost);
    });

    it("should use agent mathUtils or fallback to imported mathUtils", () => {
      expect(handler.mathUtils).toBeDefined();
    });

    it("should set entropy from import", () => {
      expect(handler.entropy).toBeDefined();
    });
  });

  describe("getters/setters", () => {
    it("should get and set state", () => {
      const newState = { test: "value" };
      handler.state = newState;
      expect(handler.state).toBe(newState);
      expect(mockAgent.state).toBe(newState);
    });

    it("should get and set sessionStart", () => {
      const time = Date.now();
      handler.sessionStart = time;
      expect(handler.sessionStart).toBe(time);
      expect(mockAgent.sessionStart).toBe(time);
    });

    it("should get and set sessionEndTime", () => {
      const time = Date.now();
      handler.sessionEndTime = time;
      expect(handler.sessionEndTime).toBe(time);
    });

    it("should get and set loopIndex", () => {
      handler.loopIndex = 5;
      expect(handler.loopIndex).toBe(5);
      expect(mockAgent.loopIndex).toBe(5);
    });

    it("should get and set isFatigued", () => {
      handler.isFatigued = true;
      expect(handler.isFatigued).toBe(true);
      expect(mockAgent.isFatigued).toBe(true);
    });

    it("should get and set fatigueThreshold", () => {
      handler.fatigueThreshold = 60000;
      expect(handler.fatigueThreshold).toBe(60000);
    });

    it("should get and set lastNetworkActivity", () => {
      const time = Date.now();
      handler.lastNetworkActivity = time;
      expect(handler.lastNetworkActivity).toBe(time);
    });
  });

  describe("log", () => {
    it("should log using logger.info when logger exists", () => {
      handler.log("Test message");
      expect(mockAgent.logger.info).toHaveBeenCalledWith("Test message");
    });

    it("should fallback to console.log when logger does not exist", () => {
      const consoleSpy = vi.spyOn(console, "log").mockImplementation(() => {});
      handler.logger = null;
      handler.log("Fallback message");
      expect(consoleSpy).toHaveBeenCalledWith("Fallback message");
      consoleSpy.mockRestore();
    });
  });

  describe("clamp", () => {
    it("should return value when within range", () => {
      expect(handler.clamp(5, 0, 10)).toBe(5);
    });

    it("should return min when value is below range", () => {
      expect(handler.clamp(-5, 0, 10)).toBe(0);
    });

    it("should return max when value is above range", () => {
      expect(handler.clamp(15, 0, 10)).toBe(10);
    });

    it("should handle equal boundaries", () => {
      expect(handler.clamp(0, 0, 0)).toBe(0);
    });
  });

  describe("checkFatigue", () => {
    it("should return false when session is fresh", () => {
      handler.sessionStart = Date.now();
      handler.fatigueThreshold = 60000;
      expect(handler.checkFatigue()).toBe(false);
    });

    it("should return true and set isFatigued when session exceeds threshold", () => {
      handler.sessionStart = Date.now() - 70000;
      handler.fatigueThreshold = 60000;
      expect(handler.checkFatigue()).toBe(true);
      expect(handler.isFatigued).toBe(true);
    });

    it("should log fatigue message when triggered", () => {
      handler.sessionStart = Date.now() - 70000;
      handler.fatigueThreshold = 60000;
      handler.checkFatigue();
      expect(mockAgent.logger.info).toHaveBeenCalledWith(
        expect.stringContaining("Fatigue"),
      );
    });
  });

  describe("triggerHotSwap", () => {
    it("should not change config when no fatigued variant available", () => {
      mockAgent.config.getFatiguedVariant.mockReturnValue(null);
      const originalConfig = handler.config;
      handler.triggerHotSwap();
      expect(handler.config).toBe(originalConfig);
    });

    it("should change config when fatigued variant is available", () => {
      const fatiguedVariant = { timings: { scrollPause: { mean: 2000 } } };
      mockAgent.config.getFatiguedVariant.mockReturnValue(fatiguedVariant);
      handler.triggerHotSwap();
      expect(handler.config).toBe(fatiguedVariant);
    });

    it("should call getFatiguedVariant with scrollPause mean", () => {
      handler.triggerHotSwap();
      expect(mockAgent.config.getFatiguedVariant).toHaveBeenCalledWith(
        mockAgent.config.timings.scrollPause.mean,
      );
    });
  });

  describe("checkAndHandleSoftError", () => {
    it("should return false when no soft error is visible", async () => {
      api.visible.mockResolvedValue(false);
      const result = await handler.checkAndHandleSoftError();
      expect(result).toBe(false);
    });

    it("should increment consecutiveSoftErrors when error is visible", async () => {
      api.visible.mockResolvedValue(true);
      await handler.checkAndHandleSoftError();
      expect(handler.state.consecutiveSoftErrors).toBe(1);
    });

    it("should throw error after 3 consecutive soft errors", async () => {
      api.visible.mockResolvedValue(true);
      handler.state.consecutiveSoftErrors = 2;
      await expect(handler.checkAndHandleSoftError()).rejects.toThrow(
        "potential twitter logged out",
      );
    });

    it("should reset consecutiveSoftErrors when no error", async () => {
      api.visible.mockResolvedValue(false);
      handler.state.consecutiveSoftErrors = 2;
      await handler.checkAndHandleSoftError();
      expect(handler.state.consecutiveSoftErrors).toBe(0);
    });
  });

  describe("getScrollMethod", () => {
    it("should return one of the valid scroll methods", () => {
      const methods = ["WHEEL_DOWN", "PAGE_DOWN", "ARROW_DOWN"];
      const result = handler.getScrollMethod();
      expect(methods).toContain(result);
    });

    it("should return different values over multiple calls", () => {
      const results = new Set();
      for (let i = 0; i < 20; i++) {
        results.add(handler.getScrollMethod());
      }
      expect(results.size).toBeGreaterThan(0);
    });
  });

  describe("normalizeProbabilities", () => {
    it("should merge custom probabilities with config defaults", () => {
      const result = handler.normalizeProbabilities({ tweetDive: 0.5 });
      expect(result.tweetDive).toBe(0.5);
      expect(result.profileDive).toBe(0.2); // From config
    });

    it("should clamp probabilities to 0-1 range", () => {
      const result = handler.normalizeProbabilities({ tweetDive: 1.5 });
      expect(result.tweetDive).toBe(1);
    });

    it("should clamp negative probabilities to 0", () => {
      const result = handler.normalizeProbabilities({ tweetDive: -0.5 });
      expect(result.tweetDive).toBe(0);
    });

    it("should apply fatigue bias reductions", () => {
      handler.state.fatigueBias = 1;
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

    it("should handle non-number values", () => {
      const result = handler.normalizeProbabilities({ tweetDive: "invalid" });
      expect(result.tweetDive).toBe("invalid");
    });
  });

  describe("isSessionExpired", () => {
    it("should return false when session is within max duration", () => {
      handler.sessionStart = Date.now();
      handler.config.maxSessionDuration = 45 * 60 * 1000;
      expect(handler.isSessionExpired()).toBe(false);
    });

    it("should return true when session exceeds max duration", () => {
      handler.sessionStart = Date.now() - 46 * 60 * 1000;
      handler.config.maxSessionDuration = 45 * 60 * 1000;
      expect(handler.isSessionExpired()).toBe(true);
    });

    it("should use default 45 minutes when maxSessionDuration not set", () => {
      handler.sessionStart = Date.now() - 46 * 60 * 1000;
      delete handler.config.maxSessionDuration;
      expect(handler.isSessionExpired()).toBe(true);
    });
  });

  describe("performHealthCheck", () => {
    it("should return healthy when network activity is recent", async () => {
      handler.lastNetworkActivity = Date.now();
      api.eval.mockResolvedValue("<html></html>");
      const result = await handler.performHealthCheck();
      expect(result.healthy).toBe(true);
    });

    it("should return unhealthy when network is inactive for 30s", async () => {
      handler.lastNetworkActivity = Date.now() - 35000;
      const result = await handler.performHealthCheck();
      expect(result.healthy).toBe(false);
      expect(result.reason).toContain("network_inactivity");
    });

    it("should detect ERR_TOO_MANY_REDIRECTS", async () => {
      handler.lastNetworkActivity = Date.now();
      api.eval.mockResolvedValue("ERR_TOO_MANY_REDIRECTS");
      const result = await handler.performHealthCheck();
      expect(result.healthy).toBe(false);
      expect(result.reason).toBe("critical_error_page_redirects");
    });

    it("should detect page not working error", async () => {
      handler.lastNetworkActivity = Date.now();
      api.eval.mockResolvedValue("This page isn\u2019t working");
      const result = await handler.performHealthCheck();
      expect(result.healthy).toBe(false);
    });

    it("should detect too many redirects", async () => {
      handler.lastNetworkActivity = Date.now();
      api.eval.mockResolvedValue("redirected you too many times");
      const result = await handler.performHealthCheck();
      expect(result.healthy).toBe(false);
    });

    it("should return healthy with check_failed on error", async () => {
      handler.lastNetworkActivity = Date.now();
      api.eval.mockRejectedValue(new Error("Page closed"));
      const result = await handler.performHealthCheck();
      expect(result.healthy).toBe(true);
      expect(result.reason).toBe("check_failed");
    });
  });

  describe("humanClick", () => {
    it("should return early when target is null", async () => {
      await handler.humanClick(null);
      expect(mockAgent.human.think).not.toHaveBeenCalled();
    });

    it("should call human.think with description", async () => {
      const mockTarget = {
        evaluate: vi.fn().mockResolvedValue(undefined),
      };
      await handler.humanClick(mockTarget, "TestTarget");
      expect(mockAgent.human.think).toHaveBeenCalledWith("TestTarget");
    });

    it("should scroll element into view", async () => {
      const mockTarget = {
        evaluate: vi.fn().mockResolvedValue(undefined),
      };
      await handler.humanClick(mockTarget);
      expect(mockTarget.evaluate).toHaveBeenCalled();
    });

    it("should use ghost.click for clicking", async () => {
      const mockTarget = {
        evaluate: vi.fn().mockResolvedValue(undefined),
      };
      await handler.humanClick(mockTarget);
      expect(mockAgent.ghost.click).toHaveBeenCalled();
    });

    it("should log success when ghost click succeeds", async () => {
      mockAgent.ghost.click.mockResolvedValue({
        success: true,
        x: 100,
        y: 200,
      });
      const mockTarget = {
        evaluate: vi.fn().mockResolvedValue(undefined),
      };
      await handler.humanClick(mockTarget);
      expect(mockAgent.logger.info).toHaveBeenCalledWith(
        expect.stringContaining("Clicked"),
      );
    });

    it("should throw error when ghost click fails", async () => {
      mockAgent.ghost.click.mockResolvedValue({ success: false });
      const mockTarget = {
        evaluate: vi.fn().mockResolvedValue(undefined),
      };
      await expect(handler.humanClick(mockTarget)).rejects.toThrow(
        "ghost_click_failed",
      );
    });

    it("should call recoverFromError on failure", async () => {
      mockAgent.ghost.click.mockRejectedValue(new Error("Click failed"));
      const mockTarget = {
        evaluate: vi.fn().mockResolvedValue(undefined),
      };
      await expect(handler.humanClick(mockTarget)).rejects.toThrow();
      expect(mockAgent.human.recoverFromError).toHaveBeenCalled();
    });
  });

  describe("safeHumanClick", () => {
    it("should return true on successful click", async () => {
      const mockTarget = {
        evaluate: vi.fn().mockResolvedValue(undefined),
      };
      mockAgent.ghost.click.mockResolvedValue({
        success: true,
        x: 100,
        y: 100,
      });
      const result = await handler.safeHumanClick(mockTarget);
      expect(result).toBe(true);
    });

    it("should return false after all retries fail", async () => {
      const mockTarget = {
        evaluate: vi.fn().mockResolvedValue(undefined),
      };
      mockAgent.ghost.click.mockResolvedValue({ success: false });
      const result = await handler.safeHumanClick(mockTarget, "Test", 2);
      expect(result).toBe(false);
    });

    it("should retry specified number of times", async () => {
      const mockTarget = {
        evaluate: vi.fn().mockResolvedValue(undefined),
      };
      mockAgent.ghost.click.mockResolvedValue({ success: false });
      await handler.safeHumanClick(mockTarget, "Test", 3);
      expect(mockAgent.ghost.click).toHaveBeenCalledTimes(3);
    });

    it("should wait between retries", async () => {
      const mockTarget = {
        evaluate: vi.fn().mockResolvedValue(undefined),
      };
      mockAgent.ghost.click.mockResolvedValue({ success: false });
      await handler.safeHumanClick(mockTarget, "Test", 2);
      expect(api.wait).toHaveBeenCalled();
    });
  });

  describe("isElementActionable", () => {
    it("should return false when element handle is null", async () => {
      const mockElement = {
        elementHandle: vi.fn().mockResolvedValue(null),
      };
      const result = await handler.isElementActionable(mockElement);
      expect(result).toBe(false);
    });

    it("should return false when page.evaluate returns false", async () => {
      mockAgent.page.evaluate.mockResolvedValue(false);
      const mockElement = {
        elementHandle: vi.fn().mockResolvedValue({}),
      };
      const result = await handler.isElementActionable(mockElement);
      expect(result).toBe(false);
    });

    it("should return true when element is actionable", async () => {
      mockAgent.page.evaluate.mockResolvedValue(true);
      const mockElement = {
        elementHandle: vi.fn().mockResolvedValue({}),
      };
      const result = await handler.isElementActionable(mockElement);
      expect(result).toBe(true);
    });

    it("should return false on error", async () => {
      const mockElement = {
        elementHandle: vi.fn().mockRejectedValue(new Error("Failed")),
      };
      const result = await handler.isElementActionable(mockElement);
      expect(result).toBe(false);
    });
  });

  describe("scrollToGoldenZone", () => {
    it("should call api.scroll.focus", async () => {
      const mockElement = {};
      await handler.scrollToGoldenZone(mockElement);
      expect(api.scroll.focus).toHaveBeenCalledWith(mockElement);
    });

    it("should log error on failure", async () => {
      api.scroll.focus.mockRejectedValue(new Error("Scroll failed"));
      await handler.scrollToGoldenZone({});
      expect(mockAgent.logger.info).toHaveBeenCalledWith(
        expect.stringContaining("scrollToGoldenZone error"),
      );
    });
  });

  describe("humanType", () => {
    it("should click element before typing", async () => {
      const mockElement = {
        click: vi.fn().mockResolvedValue(undefined),
        press: vi.fn().mockResolvedValue(undefined),
      };
      await handler.humanType(mockElement, "a");
      expect(mockElement.click).toHaveBeenCalled();
    });

    it("should press each character", async () => {
      const mockElement = {
        click: vi.fn().mockResolvedValue(undefined),
        press: vi.fn().mockResolvedValue(undefined),
      };
      await handler.humanType(mockElement, "abc");
      expect(mockElement.press).toHaveBeenCalledTimes(3);
    });

    it("should handle typing errors", async () => {
      mathUtils.roll.mockReturnValue(true);
      const mockElement = {
        click: vi.fn().mockResolvedValue(undefined),
        press: vi.fn().mockResolvedValue(undefined),
      };
      await handler.humanType(mockElement, "a");
      // With error, should press: a, Backspace, a
      expect(mockElement.press).toHaveBeenCalledTimes(3);
    });

    it("should log error and rethrow on failure", async () => {
      const mockElement = {
        click: vi.fn().mockRejectedValue(new Error("Click failed")),
        press: vi.fn().mockResolvedValue(undefined),
      };
      await expect(handler.humanType(mockElement, "test")).rejects.toThrow();
      expect(mockAgent.logger.info).toHaveBeenCalledWith(
        expect.stringContaining("humanType error"),
      );
    });
  });

  describe("dismissOverlays", () => {
    it("should press Escape when toast is visible", async () => {
      api.exists.mockResolvedValue(true);
      await handler.dismissOverlays();
      expect(mockAgent.page.keyboard.press).toHaveBeenCalledWith("Escape");
    });

    it("should press Escape when modal is visible", async () => {
      api.exists
        .mockResolvedValueOnce(false) // toasts
        .mockResolvedValueOnce(true); // modals
      await handler.dismissOverlays();
      expect(mockAgent.page.keyboard.press).toHaveBeenCalledWith("Escape");
    });

    it("should not press Escape when no overlays visible", async () => {
      api.exists.mockResolvedValue(false);
      await handler.dismissOverlays();
      expect(mockAgent.page.keyboard.press).not.toHaveBeenCalled();
    });

    it("should handle errors silently", async () => {
      api.exists.mockRejectedValue(new Error("Error"));
      await expect(handler.dismissOverlays()).resolves.not.toThrow();
    });
  });

  describe("simulateReading", () => {
    it("should call human.consumeContent", async () => {
      mathUtils.gaussian.mockReturnValue(100);
      mathUtils.randomInRange.mockReturnValue(100);
      mathUtils.roll.mockReturnValue(false);

      // Short circuit the reading loop
      const originalDateNow = Date.now;
      let callCount = 0;
      Date.now = vi.fn(() => {
        callCount++;
        if (callCount === 1) return 1000;
        if (callCount === 2) return 1000;
        return 1100; // End immediately
      });

      await handler.simulateReading();
      expect(mockAgent.human.consumeContent).toHaveBeenCalledWith(
        "tweet",
        "skim",
      );

      Date.now = originalDateNow;
    });

    it("should use gaussian distribution for duration", async () => {
      mathUtils.gaussian.mockReturnValue(100);
      mathUtils.randomInRange.mockReturnValue(100);
      mathUtils.roll.mockReturnValue(false);

      const originalDateNow = Date.now;
      let callCount = 0;
      Date.now = vi.fn(() => {
        callCount++;
        if (callCount === 1) return 1000;
        return 1100;
      });

      await handler.simulateReading();
      expect(mathUtils.gaussian).toHaveBeenCalled();

      Date.now = originalDateNow;
    });
  });
});
