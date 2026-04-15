/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Unit tests for tasks/twitterFollow.js
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import twitterFollowTask from "@tasks/twitterFollow.js";
import { TwitterAgent } from "@api/twitter/twitterAgent.js";
import { profileManager } from "@api/utils/profileManager.js";
import { ReferrerEngine } from "@api/utils/urlReferrer.js";
import metricsCollector from "@api/utils/metrics.js";

vi.mock("@api/index.js", () => {
  const api = {
    setPage: vi.fn(),
    getPage: vi.fn(),
    wait: vi.fn().mockResolvedValue(undefined),
    think: vi.fn().mockResolvedValue(undefined),
    getPersona: vi
      .fn()
      .mockReturnValue({ microMoveChance: 0.1, fidgetChance: 0.05 }),
    scroll: Object.assign(vi.fn().mockResolvedValue(undefined), {
      toTop: vi.fn().mockResolvedValue(undefined),
      back: vi.fn().mockResolvedValue(undefined),
      read: vi.fn().mockResolvedValue(undefined),
      focus: vi.fn().mockResolvedValue(undefined),
    }),
    visible: vi.fn().mockImplementation(async (el) => {
      if (el && typeof el.isVisible === "function") return await el.isVisible();
      if (el && typeof el.count === "function") return (await el.count()) > 0;
      return true;
    }),
    exists: vi.fn().mockImplementation(async (el) => {
      if (el && typeof el.count === "function") return (await el.count()) > 0;
      return el !== null;
    }),
    getCurrentUrl: vi.fn().mockResolvedValue("https://x.com/home"),
    goto: vi.fn().mockResolvedValue(undefined),
    reload: vi.fn().mockResolvedValue(undefined),
    eval: vi.fn().mockResolvedValue("mock result"),
    text: vi.fn().mockResolvedValue("mock text"),
    click: vi.fn().mockResolvedValue(undefined),
    type: vi.fn().mockResolvedValue(undefined),
    emulateMedia: vi.fn().mockResolvedValue(undefined),
    setExtraHTTPHeaders: vi.fn().mockResolvedValue(undefined),
    clearContext: vi.fn(),
    checkSession: vi.fn().mockResolvedValue(true),
    isSessionActive: vi.fn().mockReturnValue(true),
    waitVisible: vi.fn().mockResolvedValue(undefined),
    count: vi.fn().mockResolvedValue(1),
    waitForLoadState: vi.fn().mockResolvedValue(undefined),
  };
  return { api, default: api };
});
import { api } from "@api/index.js";

// Mocks
vi.mock("@api/twitter/twitterAgent.js");
vi.mock("@api/utils/profileManager.js");
vi.mock("@api/utils/urlReferrer.js");
vi.mock("@api/utils/metrics.js");
vi.mock("@api/utils/screenshot.js");
vi.mock("@api/utils/browserPatch.js", () => ({
  applyHumanizationPatch: vi.fn().mockResolvedValue(undefined),
}));
vi.mock("@api/utils/utils.js", () => ({
  createLogger: vi.fn(() => ({
    info: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
    success: vi.fn(),
    debug: vi.fn(),
  })),
}));
vi.mock("@api/utils/math.js", () => ({
  mathUtils: {
    randomInRange: vi.fn().mockReturnValue(100),
    roll: vi.fn().mockReturnValue(true),
  },
}));

describe("tasks/twitterFollow", () => {
  let mockPage;
  let mockAgent;
  let mockReferrerEngine;

  beforeEach(() => {
    vi.clearAllMocks();

    mockPage = {
      goto: vi.fn().mockResolvedValue(undefined),
      waitForSelector: vi.fn().mockResolvedValue(undefined),
      waitForTimeout: vi.fn().mockResolvedValue(undefined),
      setExtraHTTPHeaders: vi.fn().mockResolvedValue(undefined),
      emulateMedia: vi.fn().mockResolvedValue(undefined),
      url: vi.fn().mockReturnValue("https://x.com/profile"),
      locator: vi.fn().mockReturnValue({
        first: vi.fn().mockReturnThis(),
        count: vi.fn().mockResolvedValue(1),
        isVisible: vi.fn().mockResolvedValue(true),
      }),
      isClosed: vi.fn().mockReturnValue(false),
      close: vi.fn().mockResolvedValue(undefined),
      waitForLoadState: vi.fn().mockResolvedValue(undefined),
    };

    api.getPage.mockReturnValue(mockPage);
    api.setPage.mockReturnValue(undefined);

    mockAgent = {
      config: {
        probabilities: {
          refresh: 1,
          profileDive: 1,
          tweetDive: 1,
          idle: 0.1,
          likeTweetAfterDive: 1,
          bookmarkAfterDive: 1,
          followOnProfile: 1,
        },
        timings: {
          readingPhase: { mean: 1000, deviation: 100 },
        },
      },
      simulateReading: vi.fn().mockResolvedValue(undefined),
      humanClick: vi.fn().mockResolvedValue(undefined),
      robustFollow: vi.fn().mockResolvedValue({ success: true, attempts: 1 }),
      navigateHome: vi.fn().mockResolvedValue(undefined),
      checkLoginState: vi.fn().mockResolvedValue(true),
      sessionStart: Date.now(),
    };
    TwitterAgent.mockImplementation(function () {
      return mockAgent;
    });

    mockReferrerEngine = {
      generateContext: vi.fn().mockReturnValue({
        strategy: "dynamic",
        referrer: "https://google.com",
        headers: { Referer: "https://google.com" },
        targetWithParams: "https://x.com/status/123?utm=test",
      }),
    };
    ReferrerEngine.mockImplementation(function () {
      return mockReferrerEngine;
    });

    profileManager.getStarter.mockReturnValue({ theme: "dark" });
    profileManager.getById.mockReturnValue({ id: "p1", theme: "light" });
  });

  it("should complete follow task successfully", async () => {
    const payload = {
      browserInfo: "test",
      targetUrl: "https://x.com/user/status/123",
    };
    await twitterFollowTask(mockPage, payload);

    expect(api.goto).toHaveBeenCalled();
    expect(mockAgent.simulateReading).toHaveBeenCalled();
    expect(mockAgent.humanClick).toHaveBeenCalled();
    expect(mockAgent.robustFollow).toHaveBeenCalled();
    expect(metricsCollector.recordSocialAction).toHaveBeenCalledWith(
      "follow",
      1,
    );
    expect(mockPage.close).toHaveBeenCalled();
  });

  it("should handle custom profile ID in payload", async () => {
    const payload = { browserInfo: "test", profileId: "p1" };
    await twitterFollowTask(mockPage, payload);
    expect(profileManager.getById).toHaveBeenCalledWith("p1");
    expect(api.emulateMedia).toHaveBeenCalledWith({ colorScheme: "light" });
  });

  it("should handle profile navigation retry via avatar if handle click fails", async () => {
    // First call to url() returns status, second returns profile
    api.getCurrentUrl
      .mockResolvedValueOnce("https://x.com/user/status/123")
      .mockResolvedValueOnce("https://x.com/user/status/123")
      .mockResolvedValue("https://x.com/user");

    // Make handle selector not visible, avatar visible
    api.visible.mockImplementation(async (selector) => {
      if (selector.includes('href="/user"]')) return false; // handle not visible
      if (selector.includes("Avatar")) return true; // avatar visible
      return true;
    });

    await twitterFollowTask(mockPage, { browserInfo: "test" });
    // Should click avatar when handle not visible
    expect(mockAgent.humanClick).toHaveBeenCalled();
  });

  it("should handle navigation failures with retry", async () => {
    api.goto
      .mockRejectedValueOnce(new Error("nav error"))
      .mockResolvedValueOnce(undefined);

    await twitterFollowTask(mockPage, { browserInfo: "test" });
    expect(api.goto).toHaveBeenCalledTimes(2);
  });

  it("should handle fatal redirect loop error", async () => {
    api.goto.mockRejectedValue(new Error("ERR_TOO_MANY_REDIRECTS"));
    await twitterFollowTask(mockPage, { browserInfo: "test" });
    // Should catch and log, not crash
  });
});
