/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { vi, describe, it, expect, beforeEach } from "vitest";

vi.mock("@api/index.js", () => {
  const api = {
    setPage: vi.fn(),
    getPage: vi.fn(),
    withPage: vi.fn(),
    wait: vi.fn().mockResolvedValue(undefined),
    click: vi.fn().mockResolvedValue(undefined),
    visible: vi.fn().mockImplementation(async (el) => {
      if (!el) return false;
      if (typeof el.isVisible === "function") return await el.isVisible();
      if (typeof el.count === "function") return (await el.count()) > 0;
      return true;
    }),
    exists: vi.fn().mockImplementation(async (el) => {
      if (!el) return false;
      if (typeof el.count === "function") return (await el.count()) > 0;
      return el !== null;
    }),
    getCurrentUrl: vi.fn().mockResolvedValue("https://x.com/home"),
    reload: vi.fn().mockResolvedValue(undefined),
    goto: vi.fn().mockResolvedValue(undefined),
    eval: vi.fn().mockResolvedValue([]),
    text: vi.fn().mockResolvedValue("mock text"),
    scroll: Object.assign(vi.fn().mockResolvedValue(undefined), {
      toTop: vi.fn().mockResolvedValue(undefined),
      back: vi.fn().mockResolvedValue(undefined),
      focus: vi.fn().mockResolvedValue(undefined),
    }),
    count: vi.fn().mockResolvedValue(1),
  };
  return { api, default: api };
});

import { api } from "@api/index.js";
import { EngagementHandler } from "@api/twitter/twitter-agent/EngagementHandler.js";
import { mathUtils } from "@api/utils/math.js";

describe("EngagementHandler", () => {
  let handler;
  let mockAgent;
  let mockPage;
  let mockLogger;
  let mockGhost;

  beforeEach(() => {
    vi.clearAllMocks();

    mathUtils.randomInRange = vi.fn((min, max) => min);
    mathUtils.roll = vi.fn(() => true);

    const mockLocator = {
      first: vi.fn().mockReturnThis(),
      count: vi.fn().mockImplementation(async () => 1),
      isVisible: vi.fn().mockResolvedValue(true),
      boundingBox: vi
        .fn()
        .mockResolvedValue({ x: 0, y: 0, width: 100, height: 100 }),
      click: vi.fn().mockResolvedValue(undefined),
      all: vi.fn().mockResolvedValue([]),
      textContent: vi.fn().mockResolvedValue("follow"),
      getAttribute: vi.fn().mockResolvedValue("mock attr"),
      evaluate: vi.fn().mockResolvedValue(undefined),
    };

    mockPage = {
      locator: vi.fn().mockImplementation((sel) => {
        // Return different visibility based on selector to avoid "Already following"
        const isUnfollow =
          sel.includes("unfollow") ||
          sel.includes("Following") ||
          sel.includes("Pending");
        return {
          ...mockLocator,
          count: vi.fn().mockResolvedValue(isUnfollow ? 0 : 1),
          isVisible: vi.fn().mockResolvedValue(!isUnfollow),
        };
      }),
      url: vi.fn().mockReturnValue("https://x.com/home"),
      isClosed: vi.fn().mockReturnValue(false),
      context: vi.fn().mockReturnValue({
        browser: vi
          .fn()
          .mockReturnValue({ isConnected: vi.fn().mockReturnValue(true) }),
      }),
      evaluate: vi.fn().mockResolvedValue(undefined),
      goBack: vi.fn().mockResolvedValue(undefined),
    };

    api.withPage.mockImplementation((callback) => callback(mockPage));
    api.withPage.mockReturnValue(undefined);

    mockLogger = {
      info: vi.fn(),
      warn: vi.fn(),
      error: vi.fn(),
      log: vi.fn(),
    };

    mockGhost = {
      click: vi.fn().mockResolvedValue({ success: true, x: 50, y: 50 }),
      move: vi.fn().mockResolvedValue(undefined),
    };

    mockAgent = {
      page: mockPage,
      log: vi.fn(),
      ghost: mockGhost,
      state: { follows: 0, likes: 0, bookmarks: 0, consecutiveSoftErrors: 0 },
      human: {
        safeHumanClick: vi.fn().mockResolvedValue(true),
        humanClick: vi.fn().mockResolvedValue({ success: true }),
        fixation: vi.fn().mockResolvedValue(undefined),
        microMove: vi.fn().mockResolvedValue(undefined),
        think: vi.fn().mockResolvedValue(undefined),
        recoverFromError: vi.fn().mockResolvedValue(undefined),
        consumeContent: vi.fn().mockResolvedValue(undefined),
        scroll: vi.fn().mockResolvedValue(undefined),
      },
      mathUtils: mathUtils,
      config: {
        timings: { readingPhase: { mean: 1000, deviation: 300 } },
        probabilities: { likeTweetAfterDive: 0.3, bookmarkAfterDive: 0.1 },
      },
      sessionStart: Date.now(),
      fatigueThreshold: 1000000,
    };

    handler = new EngagementHandler(mockAgent);
  });

  describe("likeTweet", () => {
    it("should return false when like button not found", async () => {
      mockPage.locator = vi.fn().mockImplementation(() => ({
        first: vi.fn().mockReturnThis(),
        count: vi.fn().mockResolvedValue(0),
        isVisible: vi.fn().mockResolvedValue(false),
      }));
      const result = await handler.likeTweet({});
      expect(result).toBe(false);
    });

    it("should handle already liked state", async () => {
      mockPage.locator = vi.fn().mockImplementation(() => ({
        first: vi.fn().mockReturnThis(),
        count: vi.fn().mockResolvedValue(1),
        isVisible: vi.fn().mockResolvedValue(false),
      }));
      const result = await handler.likeTweet({});
      expect(result).toBe(false);
    });
  });

  describe("bookmarkTweet", () => {
    it("should return false when bookmark button not found", async () => {
      mockPage.locator = vi.fn().mockImplementation(() => ({
        first: vi.fn().mockReturnThis(),
        count: vi.fn().mockResolvedValue(0),
        isVisible: vi.fn().mockResolvedValue(false),
      }));
      const result = await handler.bookmarkTweet({});
      expect(result).toBe(false);
    });
  });

  describe("dive methods", () => {
    it("should have diveTweet method", () => {
      expect(typeof handler.diveTweet).toBe("function");
    });

    it("should have diveProfile method", () => {
      expect(typeof handler.diveProfile).toBe("function");
    });
  });

  describe("pollForFollowState", () => {
    it("should return true on state change", async () => {
      let pollCount = 0;
      const checkVisibility = () => {
        pollCount++;
        return pollCount > 0;
      };
      mockPage.locator = vi.fn().mockImplementation(() => ({
        first: vi.fn().mockReturnThis(),
        count: vi.fn().mockImplementation(() => 1),
        isVisible: vi.fn().mockImplementation(checkVisibility),
        textContent: vi.fn().mockResolvedValue("Following"),
      }));
      const result = await handler.pollForFollowState(
        '[data-testid="unfollow"]',
        '[data-testid="follow"]',
        100,
      );
      expect(result).toBe(true);
    });
  });
});
