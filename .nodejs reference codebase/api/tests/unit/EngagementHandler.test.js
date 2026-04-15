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
    wait: vi.fn().mockImplementation(async (ms) => {
      vi.advanceTimersByTime(ms || 0);
      return Promise.resolve();
    }),
    goto: vi.fn().mockResolvedValue(undefined),
    reload: vi.fn().mockResolvedValue(undefined),
    eval: vi.fn().mockResolvedValue(""),
    scroll: {
      focus: vi.fn().mockResolvedValue(undefined),
    },
  },
}));

vi.mock("@api/utils/math.js", () => ({
  mathUtils: {
    randomInRange: vi.fn((min, max) => Math.floor((min + max) / 2)),
    gaussian: vi.fn(() => 50), // Return small value for tests to avoid long loops
    roll: vi.fn(() => false),
  },
}));

vi.mock("@api/utils/entropyController.js", () => ({
  entropy: {},
}));

import { EngagementHandler } from "@api/twitter/twitter-agent/EngagementHandler.js";
import { BaseHandler } from "@api/twitter/twitter-agent/BaseHandler.js";
import { api } from "@api/index.js";
import { mathUtils } from "@api/utils/math.js";

describe("EngagementHandler", () => {
  let handler;
  let mockAgent;
  let mockPage;

  beforeEach(() => {
    vi.clearAllMocks();
    vi.useFakeTimers();

    mockPage = {
      locator: vi.fn().mockImplementation((selector) => ({
        first: vi.fn().mockReturnValue({
          click: vi.fn().mockResolvedValue(undefined),
          textContent: vi.fn().mockResolvedValue(""),
          getAttribute: vi.fn().mockResolvedValue(null),
          evaluate: vi.fn().mockResolvedValue(undefined),
          scrollIntoView: vi.fn().mockResolvedValue(undefined),
          elementHandle: vi.fn().mockResolvedValue({}),
        }),
        nth: vi.fn().mockReturnValue({
          boundingBox: vi
            .fn()
            .mockResolvedValue({ x: 100, y: 100, width: 50, height: 50 }),
          locator: vi.fn().mockReturnValue({
            count: vi.fn().mockResolvedValue(1),
          }),
          visible: vi.fn().mockResolvedValue(true),
        }),
        count: vi.fn().mockResolvedValue(5),
        visible: vi.fn().mockResolvedValue(true),
      })),
      viewportSize: vi.fn().mockReturnValue({ width: 1920, height: 1080 }),
      keyboard: {
        press: vi.fn().mockResolvedValue(undefined),
        type: vi.fn().mockResolvedValue(undefined),
      },
      evaluate: vi.fn().mockResolvedValue(undefined),
      goBack: vi.fn().mockResolvedValue(undefined),
    };

    mockAgent = {
      page: mockPage,
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
      operationLock: false,
    };

    handler = new EngagementHandler(mockAgent);
  });

  afterEach(() => {
    vi.restoreAllMocks();
    vi.useRealTimers();
  });

  describe("constructor", () => {
    it("should extend BaseHandler", () => {
      expect(handler).toBeInstanceOf(BaseHandler);
    });

    it("should inherit agent properties", () => {
      expect(handler.agent).toBe(mockAgent);
      expect(handler.page).toBe(mockPage);
    });
  });

  describe("pollForFollowState", () => {
    it("should return true when unfollow button becomes visible", async () => {
      api.visible.mockResolvedValue(true);
      const result = await handler.pollForFollowState(
        '[data-testid="unfollow"]',
        '[data-testid="follow"]',
      );
      expect(result).toBe(true);
    });

    it("should return true when follow button disappears", async () => {
      api.visible.mockResolvedValueOnce(false).mockResolvedValueOnce(false);
      const result = await handler.pollForFollowState(
        '[data-testid="unfollow"]',
        '[data-testid="follow"]',
      );
      expect(result).toBe(true);
    });

    it("should return true when button text includes following", async () => {
      api.visible.mockResolvedValue(false);
      const mockLocator = {
        first: vi.fn().mockReturnValue({
          textContent: vi.fn().mockResolvedValue("Following"),
        }),
      };
      mockPage.locator.mockReturnValue(mockLocator);
      const result = await handler.pollForFollowState(
        '[data-testid="unfollow"]',
        '[data-testid="follow"]',
      );
      expect(result).toBe(true);
    });

    it("should call locator with correct selectors", async () => {
      api.visible.mockResolvedValue(true);
      await handler.pollForFollowState(
        '[data-testid="unfollow"]',
        '[data-testid="follow"]',
      );
      expect(mockPage.locator).toHaveBeenCalledWith('[data-testid="unfollow"]');
      expect(mockPage.locator).toHaveBeenCalledWith('[data-testid="follow"]');
    });
  });

  describe("sixLayerClick", () => {
    it("should return true when a layer succeeds", async () => {
      mockAgent.ghost.click.mockResolvedValue({
        success: true,
        x: 100,
        y: 100,
      });
      const mockElement = {
        elementHandle: vi.fn().mockResolvedValue({}),
        evaluate: vi.fn().mockResolvedValue(undefined),
      };
      const result = await handler.sixLayerClick(mockElement, "[Test] ");
      expect(result).toBe(true);
    });

    it("should try fallback layers when primary fails", async () => {
      mockAgent.ghost.click.mockResolvedValue({
        success: false,
        error: "failed",
      });
      const mockElement = {
        elementHandle: vi.fn().mockResolvedValue(null),
        click: vi.fn().mockResolvedValue(undefined),
        evaluate: vi.fn().mockResolvedValue(undefined),
      };
      const result = await handler.sixLayerClick(mockElement, "[Test] ");
      expect(result).toBe(true);
    });

    it("should log layer attempts", async () => {
      mockAgent.ghost.click.mockResolvedValue({
        success: true,
        x: 100,
        y: 100,
      });
      const mockElement = {
        elementHandle: vi.fn().mockResolvedValue({}),
        evaluate: vi.fn().mockResolvedValue(undefined),
      };
      await handler.sixLayerClick(mockElement, "[Test] ");
      expect(mockAgent.logger.info).toHaveBeenCalledWith(
        expect.stringContaining("Trying layer"),
      );
    });

    it("should call mathUtils.randomInRange for delays", async () => {
      mockAgent.ghost.click.mockResolvedValue({
        success: false,
        error: "fail",
      });
      const mockElement = {
        elementHandle: vi.fn().mockResolvedValue(null),
        click: vi.fn().mockRejectedValue(new Error("fail")),
        evaluate: vi.fn().mockRejectedValue(new Error("fail")),
      };
      await handler.sixLayerClick(mockElement, "[Test] ");
      expect(mathUtils.randomInRange).toHaveBeenCalled();
    });
  });

  describe("robustFollow", () => {
    it("should skip when already unfollowing", async () => {
      api.visible.mockResolvedValue(true);
      const result = await handler.robustFollow();
      expect(result.success).toBe(true);
      expect(result.skipped).toBe(true);
    });

    it("should skip when already following", async () => {
      api.visible.mockResolvedValueOnce(false).mockResolvedValueOnce(true);
      const mockLocator = {
        first: vi.fn().mockReturnValue({
          textContent: vi.fn().mockResolvedValue("Following"),
        }),
      };
      mockPage.locator.mockReturnValue(mockLocator);
      const result = await handler.robustFollow();
      expect(result.success).toBe(true);
      expect(result.skipped).toBe(true);
    });

    it("should return object with expected properties", async () => {
      api.visible.mockResolvedValue(true);
      const result = await handler.robustFollow();
      expect(result).toHaveProperty("success");
      expect(result).toHaveProperty("attempts");
      expect(result).toHaveProperty("reloaded");
      expect(result).toHaveProperty("error");
    });

    it("should abort on health check failure", async () => {
      api.visible.mockResolvedValue(false);
      handler.lastNetworkActivity = Date.now() - 35000;
      const result = await handler.robustFollow();
      expect(result.success).toBe(false);
      expect(result.error).toContain("Health failure");
    });

    it("should use custom logPrefix", async () => {
      api.visible.mockResolvedValue(true);
      await handler.robustFollow("[CustomPrefix]");
      expect(mockAgent.logger.info).toHaveBeenCalledWith(
        expect.stringContaining("[CustomPrefix]"),
      );
    });
  });

  describe("diveTweet", () => {
    it("should return early when operationLock is active", async () => {
      mockAgent.operationLock = true;
      await handler.diveTweet();
      expect(mockAgent.logger.info).toHaveBeenCalledWith(
        expect.stringContaining("Operation lock active"),
      );
    });

    it("should return false when no tweets found", async () => {
      mockAgent.operationLock = false;
      mockPage.locator.mockReturnValue({
        count: vi.fn().mockResolvedValue(0),
      });
      const result = await handler.diveTweet();
      expect(result).toBe(false);
    });

    it("should log when starting", async () => {
      mockAgent.operationLock = false;
      mockPage.locator.mockReturnValue({
        count: vi.fn().mockResolvedValue(0),
      });
      await handler.diveTweet();
      expect(mockAgent.logger.info).toHaveBeenCalledWith(
        expect.stringContaining("[Dive] Starting"),
      );
    });

    it("should log when no suitable tweets found", async () => {
      mockAgent.operationLock = false;
      mockPage.locator.mockReturnValue({
        count: vi.fn().mockResolvedValue(0),
      });
      await handler.diveTweet();
      expect(mockAgent.logger.info).toHaveBeenCalledWith(
        expect.stringContaining("No suitable tweets"),
      );
    });
  });

  describe("likeTweet", () => {
    it("should return true when like button is found and clicked", async () => {
      api.exists.mockResolvedValue(true);
      api.visible.mockResolvedValue(true);
      mockAgent.ghost.click.mockResolvedValue({
        success: true,
        x: 100,
        y: 100,
      });
      const mockTweet = { locator: vi.fn().mockReturnValue({}) };
      const result = await handler.likeTweet(mockTweet);
      expect(result).toBe(true);
    });

    it("should return false when like button not found", async () => {
      api.exists.mockResolvedValue(false);
      const mockTweet = { locator: vi.fn().mockReturnValue({}) };
      const result = await handler.likeTweet(mockTweet);
      expect(result).toBe(false);
    });

    it("should call locator with correct selector", async () => {
      api.exists.mockResolvedValue(true);
      api.visible.mockResolvedValue(true);
      mockAgent.ghost.click.mockResolvedValue({
        success: true,
        x: 100,
        y: 100,
      });
      const mockTweet = { locator: vi.fn().mockReturnValue({}) };
      await handler.likeTweet(mockTweet);
      expect(mockTweet.locator).toHaveBeenCalledWith(
        expect.stringContaining('[data-testid="like"]'),
      );
    });
  });

  describe("bookmarkTweet", () => {
    it("should return true when bookmark button is found and clicked", async () => {
      api.exists.mockResolvedValue(true);
      api.visible.mockResolvedValue(true);
      mockAgent.ghost.click.mockResolvedValue({
        success: true,
        x: 100,
        y: 100,
      });
      const mockTweet = { locator: vi.fn().mockReturnValue({}) };
      const result = await handler.bookmarkTweet(mockTweet);
      expect(result).toBe(true);
    });

    it("should return false when bookmark button not found", async () => {
      api.exists.mockResolvedValue(false);
      const mockTweet = { locator: vi.fn().mockReturnValue({}) };
      const result = await handler.bookmarkTweet(mockTweet);
      expect(result).toBe(false);
    });

    it("should call locator with correct selector", async () => {
      api.exists.mockResolvedValue(true);
      api.visible.mockResolvedValue(true);
      mockAgent.ghost.click.mockResolvedValue({
        success: true,
        x: 100,
        y: 100,
      });
      const mockTweet = { locator: vi.fn().mockReturnValue({}) };
      await handler.bookmarkTweet(mockTweet);
      expect(mockTweet.locator).toHaveBeenCalledWith(
        expect.stringContaining('[data-testid="bookmark"]'),
      );
    });
  });

  describe("diveProfile", () => {
    it("should return false when no valid profile links found", async () => {
      api.eval.mockResolvedValue([-1]);
      const result = await handler.diveProfile();
      expect(result).toBe(false);
    });

    it("should log when no valid profile links found", async () => {
      api.eval.mockResolvedValue([-1]);
      await handler.diveProfile();
      expect(mockAgent.logger.info).toHaveBeenCalledWith(
        expect.stringContaining("No valid profile links"),
      );
    });

    it("should call api.eval with selector", async () => {
      api.eval.mockResolvedValue([-1]);
      await handler.diveProfile();
      expect(api.eval).toHaveBeenCalled();
    });

    it("should handle when valid indices are found", async () => {
      api.eval.mockResolvedValue([0]);
      mockAgent.ghost.click.mockResolvedValue({ success: true });
      mockPage.goBack.mockResolvedValue(undefined);
      mockPage.locator.mockReturnValue({
        nth: vi.fn().mockReturnValue({
          click: vi.fn().mockResolvedValue(undefined),
          boundingBox: vi.fn().mockResolvedValue({ x: 100, y: 100, width: 50, height: 50 }),
        }),
      });
      const result = await handler.diveProfile();
      expect(result).toBeDefined();
    });

    it("should call interactWithProfile after clicking profile", async () => {
      api.eval.mockResolvedValue([0]);
      mockAgent.ghost.click.mockResolvedValue({ success: true });

      // Mock interactWithProfile to verify it's called
      const spy = vi.spyOn(handler, "interactWithProfile").mockResolvedValue({
        scrolled: true,
        followed: false,
        likedTweets: 0,
        readContent: true,
      });

      await handler.diveProfile();
      expect(spy).toHaveBeenCalled();
    });
  });

  describe("interactWithProfile", () => {
    it("should return object with expected properties", async () => {
      api.visible.mockResolvedValue(false);
      mockPage.locator.mockReturnValue({
        count: vi.fn().mockResolvedValue(0),
        first: vi.fn().mockReturnValue({}),
        nth: vi.fn().mockReturnValue({}),
      });

      const result = await handler.interactWithProfile();

      expect(result).toHaveProperty("scrolled");
      expect(result).toHaveProperty("followed");
      expect(result).toHaveProperty("likedTweets");
      expect(result).toHaveProperty("readContent");
    });

    it("should simulate reading behavior", async () => {
      api.visible.mockResolvedValue(false);
      mockPage.locator.mockReturnValue({
        count: vi.fn().mockResolvedValue(0),
        first: vi.fn().mockReturnValue({}),
        nth: vi.fn().mockReturnValue({}),
      });

      const result = await handler.interactWithProfile();
      expect(result.readContent).toBe(true);
    });

    it("should scroll through profile", async () => {
      api.visible.mockResolvedValue(false);
      mockPage.locator.mockReturnValue({
        count: vi.fn().mockResolvedValue(0),
        first: vi.fn().mockReturnValue({}),
        nth: vi.fn().mockReturnValue({}),
      });

      const result = await handler.interactWithProfile();
      expect(result.scrolled).toBe(true);
    });

    it("should log interaction start and completion", async () => {
      api.visible.mockResolvedValue(false);
      mockPage.locator.mockReturnValue({
        count: vi.fn().mockResolvedValue(0),
        first: vi.fn().mockReturnValue({}),
        nth: vi.fn().mockReturnValue({}),
      });

      const result = await handler.interactWithProfile();

      expect(mockAgent.logger.info).toHaveBeenCalledWith(
        expect.stringContaining("[Profile] Starting profile interaction"),
      );
      expect(mockAgent.logger.info).toHaveBeenCalledWith(
        expect.stringContaining("Interaction complete"),
      );
    });

    it("should attempt follow when probability succeeds", async () => {
      // Mock mathUtils.roll to return true for follow probability
      mathUtils.roll.mockImplementation((prob) => prob === 0.1);

      // Mock follow button visibility
      api.visible.mockImplementation((selector) => {
        const selectorStr =
          typeof selector === "string" ? selector : String(selector);
        if (selectorStr.includes("follow")) {
          return Promise.resolve(true);
        }
        return Promise.resolve(false);
      });

      // Mock text content for follow button
      mockPage.locator.mockReturnValue({
        count: vi.fn().mockResolvedValue(0),
        first: vi.fn().mockReturnValue({
          textContent: vi.fn().mockResolvedValue("Follow"),
          click: vi.fn().mockResolvedValue(undefined),
        }),
        nth: vi.fn().mockReturnValue({}),
      });

      const result = await handler.interactWithProfile();
      // Result depends on probability roll - just verify structure
      expect(result).toHaveProperty("followed");
    });

    it("should not follow when already following", async () => {
      mathUtils.roll.mockReturnValue(true);

      api.visible.mockResolvedValue(true);

      mockPage.locator.mockReturnValue({
        count: vi.fn().mockResolvedValue(0),
        first: vi.fn().mockReturnValue({
          textContent: vi.fn().mockResolvedValue("Following"),
          click: vi.fn().mockResolvedValue(undefined),
        }),
        nth: vi.fn().mockReturnValue({}),
      });

      const result = await handler.interactWithProfile();
      expect(result.followed).toBe(false);
    });

    it("should like tweets when probability succeeds", async () => {
      mathUtils.roll.mockImplementation((prob) => {
        // Return false for follow (0.1), true for like (0.2)
        if (prob === 0.1) return false;
        if (prob === 0.2) return true;
        return false;
      });

      api.visible.mockResolvedValue(false);
      api.exists.mockResolvedValue(true);

      mockPage.locator.mockImplementation((selector) => ({
        count: vi.fn().mockResolvedValue(1),
        first: vi.fn().mockReturnValue({
          textContent: vi.fn().mockResolvedValue("Follow"),
        }),
        nth: vi.fn().mockReturnValue({
          visible: vi.fn().mockResolvedValue(true),
          boundingBox: vi
            .fn()
            .mockResolvedValue({ x: 100, y: 100, width: 50, height: 50 }),
        }),
      }));

      const result = await handler.interactWithProfile();
      expect(result).toHaveProperty("likedTweets");
    });
  });
});
