/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { SessionHandler } from "@api/twitter/twitter-agent/SessionHandler.js";
import { mathUtils } from "@api/utils/math.js";
import * as scrollHelper from "@api/behaviors/scroll-helper.js";
import { api } from "@api/index.js";

// Mock dependencies
vi.mock("@api/index.js", () => ({
  api: {
    wait: vi.fn().mockResolvedValue(),
    scroll: {
      down: vi.fn().mockResolvedValue(),
      random: vi.fn().mockResolvedValue(),
    },
    exists: vi.fn().mockResolvedValue(false),
    visible: vi.fn().mockResolvedValue(false),
    getCurrentUrl: vi.fn().mockReturnValue("https://x.com/home"),
    goto: vi.fn().mockResolvedValue(),
    emulateMedia: vi.fn().mockResolvedValue(),
  },
}));

vi.mock("@api/utils/math.js", () => ({
  mathUtils: {
    roll: vi.fn(),
    randomInRange: vi.fn(),
    random: vi.fn(),
  },
}));

vi.mock("@api/behaviors/scroll-helper.js", () => ({
  scrollDown: vi.fn(),
  scrollRandom: vi.fn(),
  scrollUp: vi.fn(),
  scrollToTop: vi.fn(),
  scrollToBottom: vi.fn(),
}));

describe("SessionHandler", () => {
  let handler;
  let mockAgent;
  let mockPage;
  let mockLogger;
  let mockEngagement;
  let mockNavigation;
  let mockHuman;

  beforeEach(() => {
    // Reset mocks
    vi.clearAllMocks();

    // Setup mathUtils mocks
    mathUtils.roll.mockReturnValue(false);
    mathUtils.randomInRange.mockReturnValue(100);
    mathUtils.random.mockReturnValue(0.5);

    // Setup scrollHelper mocks
    scrollHelper.scrollDown.mockResolvedValue();
    scrollHelper.scrollRandom.mockResolvedValue();
    scrollHelper.scrollUp.mockResolvedValue();
    scrollHelper.scrollToTop.mockResolvedValue();
    scrollHelper.scrollToBottom.mockResolvedValue();

    mockPage = {
      waitForTimeout: vi.fn().mockResolvedValue(),
      emulateMedia: vi.fn().mockResolvedValue(),
      keyboard: { press: vi.fn().mockResolvedValue() },
      mouse: {
        move: vi.fn().mockResolvedValue(),
        down: vi.fn().mockResolvedValue(),
        up: vi.fn().mockResolvedValue(),
        click: vi.fn().mockResolvedValue(),
        wheel: vi.fn().mockResolvedValue(),
      },
      locator: vi.fn().mockReturnValue({
        count: vi.fn().mockResolvedValue(0),
        first: vi
          .fn()
          .mockReturnValue({ isVisible: vi.fn().mockResolvedValue(false) }),
        isVisible: vi.fn().mockResolvedValue(false),
        nth: vi.fn().mockReturnValue({
          isVisible: vi.fn().mockResolvedValue(false),
          boundingBox: vi.fn().mockResolvedValue(null),
        }),
        waitFor: vi.fn().mockResolvedValue(),
        click: vi.fn().mockResolvedValue(),
        press: vi.fn().mockResolvedValue(),
      }),
      getByText: vi.fn().mockReturnValue({
        first: vi
          .fn()
          .mockReturnValue({ isVisible: vi.fn().mockResolvedValue(false) }),
      }),
      viewportSize: vi.fn().mockReturnValue({ width: 1920, height: 1080 }),
      url: vi.fn().mockReturnValue("https://x.com/home"),
      goto: vi.fn().mockResolvedValue(),
    };

    mockLogger = {
      info: vi.fn(),
      warn: vi.fn(),
      error: vi.fn(),
      log: vi.fn(),
    };

    mockHuman = {
      think: vi.fn(),
      randomMouseMovement: vi.fn().mockResolvedValue(),
    };

    mockEngagement = {
      diveTweet: vi.fn().mockResolvedValue(),
      diveProfile: vi.fn().mockResolvedValue(),
    };
    mockNavigation = {
      navigateHome: vi.fn().mockResolvedValue(),
      ensureForYouTab: vi.fn().mockResolvedValue(),
    };

    mockAgent = {
      page: mockPage,
      config: {
        maxSessionDuration: 3600000,
        theme: "dark",
        probabilities: {
          tweetDive: 0.3,
        },
      },
      logger: mockLogger,
      state: {
        consecutiveLoginFailures: 0,
        isFatigued: false,
        activityMode: "NORMAL",
        burstEndTime: 0,
        engagements: 0,
        tweets: 0,
      },
      sessionStart: Date.now(),
      sessionEndTime: null,
      human: mockHuman,
      ghost: { move: vi.fn().mockResolvedValue() },
      navigation: mockNavigation,
      engagement: mockEngagement,
    };

    handler = new SessionHandler(mockAgent);

    handler.engagement = mockEngagement;
    handler.navigation = mockNavigation;

    // Mock safeHumanClick method from BaseHandler (if inherited/mixed in, or mocked on instance)
    handler.safeHumanClick = vi.fn().mockResolvedValue(true);

    // Mock internal methods to isolate tests
    vi.spyOn(handler, "performHealthCheck").mockResolvedValue({
      healthy: true,
    });
    vi.spyOn(handler, "checkAndHandleSoftError").mockResolvedValue(false);
    vi.spyOn(handler, "getScrollMethod").mockReturnValue("WHEEL_DOWN");
  });

  describe("runSession", () => {
    beforeEach(() => {
      // Mock internal methods to isolate runSession logic
      vi.spyOn(handler, "executeEngagementCycle").mockResolvedValue();
      vi.spyOn(handler, "isSessionExpired").mockReturnValue(false);
      mockNavigation.navigateHome = vi.fn().mockResolvedValue();
    });

    it("should set session duration if min/max provided", async () => {
      mathUtils.randomInRange.mockReturnValue(5000);

      // Mock loop to run once
      handler.isSessionExpired
        .mockReturnValueOnce(false)
        .mockReturnValueOnce(true);

      await handler.runSession(10, 5, 10);

      expect(mathUtils.randomInRange).toHaveBeenCalledWith(5000, 10000);
      expect(handler.sessionEndTime).toBeGreaterThan(0);
      expect(mockLogger.info).toHaveBeenCalledWith(
        expect.stringContaining("Session time limit set"),
      );
    });

    it("should apply theme if configured", async () => {
      mockAgent.config.theme = "dark";
      handler.isSessionExpired.mockReturnValue(true); // Exit loop immediately

      await handler.runSession();

      expect(api.emulateMedia).toHaveBeenCalledWith({ colorScheme: "dark" });
      expect(mockLogger.info).toHaveBeenCalledWith(
        expect.stringContaining("Applied theme"),
      );
    });

    it("should handle theme application failure", async () => {
      mockAgent.config.theme = "dark";
      api.emulateMedia.mockRejectedValue(new Error("Theme error"));
      handler.isSessionExpired.mockReturnValue(true);

      await handler.runSession();

      expect(mockLogger.info).toHaveBeenCalledWith(
        expect.stringContaining("Failed to apply theme"),
      );
    });

    it("should retry navigation home up to 3 times", async () => {
      mockNavigation.navigateHome
        .mockRejectedValueOnce(new Error("Fail 1"))
        .mockRejectedValueOnce(new Error("Fail 2"))
        .mockResolvedValueOnce();

      handler.isSessionExpired.mockReturnValue(true);

      await handler.runSession();

      expect(mockNavigation.navigateHome).toHaveBeenCalledTimes(3);
      expect(mockLogger.info).toHaveBeenCalledWith(
        expect.stringContaining("Navigation attempt 1/3 failed"),
      );
      expect(mockLogger.info).toHaveBeenCalledWith(
        expect.stringContaining("Navigation attempt 2/3 failed"),
      );
    });

    it("should abort if 3+ consecutive login failures", async () => {
      mockAgent.state.consecutiveLoginFailures = 3;

      await handler.runSession();

      expect(mockLogger.info).toHaveBeenCalledWith(
        expect.stringContaining("Critical: 3+ consecutive login failures"),
      );
      expect(handler.executeEngagementCycle).not.toHaveBeenCalled();
    });

    it("should run loop until cycles complete", async () => {
      await handler.runSession(2);

      // Loop runs: 0 -> check(false) -> execute -> increment -> 1 -> check(false) -> execute -> increment -> 2 -> check(stop)
      // Actually check loop completion condition: loopIndex >= cycles

      expect(handler.executeEngagementCycle).toHaveBeenCalledTimes(2);
      expect(mockLogger.info).toHaveBeenCalledWith(
        expect.stringContaining("Completed 2 engagement cycles"),
      );
    });

    it("should handle burst mode transition", async () => {
      mockAgent.state.activityMode = "BURST";
      mockAgent.state.burstEndTime = Date.now() - 1000; // Already passed

      handler.isSessionExpired
        .mockReturnValueOnce(false)
        .mockReturnValueOnce(true);

      await handler.runSession(1);

      expect(mockAgent.state.activityMode).toBe("NORMAL");
      expect(mockLogger.info).toHaveBeenCalledWith(
        expect.stringContaining("Burst mode ended"),
      );
    });
  });

  describe("executeEngagementCycle", () => {
    beforeEach(() => {
      vi.spyOn(handler, "performHealthCheck").mockResolvedValue({
        healthy: true,
      });
      vi.spyOn(handler, "dismissOverlays").mockResolvedValue();
      vi.spyOn(handler, "simulateReading").mockResolvedValue();
      vi.spyOn(handler, "simulateFidget").mockResolvedValue();
      mockEngagement.diveTweet = vi.fn().mockResolvedValue();
      mockEngagement.diveProfile = vi.fn().mockResolvedValue();
    });

    it("should return if health check fails", async () => {
      handler.performHealthCheck.mockResolvedValue({
        healthy: false,
        reason: "Test fail",
      });

      await handler.executeEngagementCycle();

      expect(mockLogger.info).toHaveBeenCalledWith(
        expect.stringContaining("Health check failed"),
      );
      expect(handler.dismissOverlays).not.toHaveBeenCalled();
    });

    it("should execute tweet engagement (60% chance)", async () => {
      vi.spyOn(Math, "random").mockReturnValue(0.5); // < 0.6

      await handler.executeEngagementCycle();

      expect(mockEngagement.diveTweet).toHaveBeenCalled();
      expect(mockEngagement.diveProfile).not.toHaveBeenCalled();
      expect(handler.simulateReading).not.toHaveBeenCalled();
    });

    it("should execute profile exploration (25% chance)", async () => {
      vi.spyOn(Math, "random").mockReturnValue(0.7); // 0.6 <= x < 0.85

      await handler.executeEngagementCycle();

      expect(mockEngagement.diveProfile).toHaveBeenCalled();
      expect(mockEngagement.diveTweet).not.toHaveBeenCalled();
    });

    it("should execute simulated reading (15% chance)", async () => {
      vi.spyOn(Math, "random").mockReturnValue(0.9); // >= 0.85

      await handler.executeEngagementCycle();

      expect(handler.simulateReading).toHaveBeenCalled();
    });

    it("should trigger fidgeting based on probability", async () => {
      vi.spyOn(Math, "random").mockReturnValue(0.5); // Tweet engagement
      mathUtils.roll.mockReturnValue(true); // Fidget triggers

      await handler.executeEngagementCycle();

      expect(handler.simulateFidget).toHaveBeenCalled();
    });

    it("should increment engagements count", async () => {
      mockAgent.state.engagements = 0;
      vi.spyOn(Math, "random").mockReturnValue(0.5);

      await handler.executeEngagementCycle();

      expect(mockAgent.state.engagements).toBe(1);
    });
  });

  describe("isSessionExpired", () => {
    it("should expire if fatigued", () => {
      // isFatigued is a getter on BaseHandler that proxies to agent.isFatigued
      mockAgent.isFatigued = true;
      expect(handler.isSessionExpired()).toBe(true);
    });

    it("should expire if time limit exceeded", () => {
      handler.sessionEndTime = Date.now() - 1000;
      expect(handler.isSessionExpired()).toBe(true);
    });

    it("should expire if consecutive login failures >= 3", () => {
      mockAgent.state.consecutiveLoginFailures = 3;
      expect(handler.isSessionExpired()).toBe(true);
    });

    it("should return false if healthy", () => {
      mockAgent.state.isFatigued = false;
      handler.sessionEndTime = Date.now() + 10000;
      expect(handler.isSessionExpired()).toBe(false);
    });
  });

  describe("simulateReading", () => {
    it("should handle burst mode short reading", async () => {
      mockAgent.state.activityMode = "BURST";
      await handler.simulateReading();
      expect(mockLogger.info).toHaveBeenCalledWith(
        expect.stringContaining("Burst mode"),
      );
    });

    it("should execute reading loop with random behaviors", async () => {
      mockAgent.state.activityMode = "NORMAL";

      // Setup time mocking to allow loop to run once or twice
      let now = 1000;
      const originalDateNow = Date.now;
      Date.now = vi.fn(() => {
        now += 5000; // Advance time by 5s each call
        return now;
      });

      mathUtils.randomInRange.mockReturnValue(8000); // Duration 8s
      mathUtils.roll.mockReturnValue(true); // Trigger all probability checks

      await handler.simulateReading();

      expect(mockLogger.info).toHaveBeenCalledWith(
        expect.stringContaining("Simulating human reading"),
      );
      expect(scrollHelper.scrollDown).toHaveBeenCalled();
      expect(mockNavigation.ensureForYouTab).toHaveBeenCalled();
      expect(api.wait).toHaveBeenCalled();
      expect(mockHuman.randomMouseMovement).toHaveBeenCalled();
      expect(mockEngagement.diveTweet).toHaveBeenCalled();

      // Restore Date.now
      Date.now = originalDateNow;
    });

    it("should stop reading if health check fails", async () => {
      mockAgent.state.activityMode = "NORMAL";
      handler.performHealthCheck.mockResolvedValue({ healthy: false });

      let now = 1000;
      const originalDateNow = Date.now;
      Date.now = vi.fn(() => now);
      mathUtils.randomInRange.mockReturnValue(20000);

      await handler.simulateReading();

      expect(handler.performHealthCheck).toHaveBeenCalled();
      // Should break loop

      Date.now = originalDateNow;
    });
  });

  describe("simulateFidget", () => {
    it("should handle TEXT_SELECT fidget", async () => {
      // Mock random to select TEXT_SELECT (index 0)
      vi.spyOn(Math, "random").mockReturnValue(0.0); // 0 -> TEXT_SELECT

      // Mock api.visible to return true for elements
      api.visible.mockResolvedValue(true);

      // Mock text elements
      const mockElement = {
        isVisible: vi.fn().mockResolvedValue(true),
        boundingBox: vi
          .fn()
          .mockResolvedValue({ x: 10, y: 10, width: 100, height: 20 }),
      };

      mockPage.locator.mockReturnValue({
        count: vi.fn().mockResolvedValue(5),
        nth: vi.fn().mockReturnValue(mockElement),
      });

      await handler.simulateFidget();

      expect(mockLogger.info).toHaveBeenCalledWith(
        expect.stringContaining("Simulating text select"),
      );
      expect(mockPage.mouse.move).toHaveBeenCalled();
      expect(mockPage.mouse.down).toHaveBeenCalled();
      expect(mockPage.mouse.up).toHaveBeenCalled();
    });

    it("should handle RANDOM_CLICK fidget", async () => {
      // Mock random to select RANDOM_CLICK (index 1)
      // 1/3 = 0.33. So 0.4 -> index 1
      vi.spyOn(Math, "random").mockReturnValue(0.4);

      await handler.simulateFidget();

      expect(mockLogger.info).toHaveBeenCalledWith(
        expect.stringContaining("Simulating random click"),
      );
      expect(mockPage.mouse.click).toHaveBeenCalled();
    });

    it("should handle SCROLL_JITTER fidget", async () => {
      // Mock random to select SCROLL_JITTER (index 2)
      // 2/3 = 0.66. So 0.8 -> index 2
      vi.spyOn(Math, "random").mockReturnValue(0.8);

      await handler.simulateFidget();

      expect(mockLogger.info).toHaveBeenCalledWith(
        expect.stringContaining("Simulating scroll jitter"),
      );
      expect(scrollHelper.scrollRandom).toHaveBeenCalled();
    });
  });

  describe("simulateReading (soft error)", () => {
    it("should break if soft error occurs", async () => {
      // Re-mock the spy for this specific test
      const softErrorSpy = vi
        .spyOn(handler, "checkAndHandleSoftError")
        .mockResolvedValue(true);
      vi.spyOn(handler, "performHealthCheck").mockResolvedValue({
        healthy: true,
      });

      await handler.simulateReading();

      expect(softErrorSpy).toHaveBeenCalled();
      expect(mockLogger.info).toHaveBeenCalledWith(
        expect.stringContaining("Reading simulation completed"),
      );
    });

    it("should use random scroll if not WHEEL_DOWN", async () => {
      vi.spyOn(handler, "checkAndHandleSoftError").mockResolvedValue(false);
      vi.spyOn(handler, "performHealthCheck").mockResolvedValue({
        healthy: true,
      });
      vi.spyOn(handler, "getScrollMethod").mockReturnValue("PAGE_DOWN");

      // Mock duration to run loop at least once
      const startTime = Date.now();
      const realDateNow = Date.now;
      let callCount = 0;
      global.Date.now = vi.fn(() => {
        callCount++;
        if (callCount > 5) return startTime + 30000; // End loop
        return startTime + callCount * 1000;
      });

      mathUtils.randomInRange.mockReturnValue(10000); // 10s duration
      mathUtils.roll.mockImplementation((prob) => {
        if (prob === 0.15) return true; // Scroll
        return false;
      });

      await handler.simulateReading();

      expect(scrollHelper.scrollRandom).toHaveBeenCalled();

      global.Date.now = realDateNow;
    });
  });

  describe("simulateFidget", () => {
    it("should handle errors gracefully", async () => {
      // Force TEXT_SELECT to ensure locator is called
      vi.spyOn(Math, "random").mockReturnValue(0);

      // Force error by mocking locator to throw
      mockPage.locator.mockImplementation(() => {
        throw new Error("Fidget error");
      });

      await handler.simulateFidget();

      expect(mockLogger.info).toHaveBeenCalledWith(
        expect.stringContaining("[Fidget] Error: Fidget error"),
      );
    });
  });

  describe("humanType", () => {
    it("should type text with delays and typos", async () => {
      const mockElement = {
        click: vi.fn().mockResolvedValue(),
        press: vi.fn().mockResolvedValue(),
      };

      mathUtils.roll.mockReturnValue(true); // Trigger typos

      await handler.humanType(mockElement, "hi");

      expect(mockLogger.info).toHaveBeenCalledWith(
        expect.stringContaining("Simulating human typing"),
      );
      expect(mockElement.click).toHaveBeenCalled();
      expect(mockElement.press).toHaveBeenCalledWith("h");
      expect(mockElement.press).toHaveBeenCalledWith("i");
      expect(mockElement.press).toHaveBeenCalledWith("Backspace"); // Typo correction
    });

    it("should handle errors during typing", async () => {
      const mockElement = {
        click: vi.fn().mockRejectedValue(new Error("Click failed")),
        press: vi.fn(),
      };

      await expect(handler.humanType(mockElement, "hi")).rejects.toThrow(
        "Click failed",
      );

      expect(mockLogger.info).toHaveBeenCalledWith(
        expect.stringContaining("[Type] Error: Click failed"),
      );
    });
  });

  describe("postTweet", () => {
    it("should post tweet successfully via button", async () => {
      const mockComposerBtn = {
        count: vi.fn().mockResolvedValue(1),
        isVisible: vi.fn().mockResolvedValue(true),
      };

      const mockPostBtn = {
        count: vi.fn().mockResolvedValue(1),
        isVisible: vi.fn().mockResolvedValue(true),
      };

      // Setup locator mocks
      mockPage.locator.mockImplementation((selector) => {
        if (selector.includes("SideNav_NewTweet_Button"))
          return mockComposerBtn;
        if (selector.includes("tweetButton")) return mockPostBtn;
        return {
          waitFor: vi.fn().mockResolvedValue(),
          count: vi.fn().mockResolvedValue(0),
        };
      });

      // Mock api.exists and api.visible to return true for composer and post buttons
      api.exists.mockResolvedValue(true);
      api.visible.mockResolvedValue(true);

      // Spy on humanType
      vi.spyOn(handler, "humanType").mockResolvedValue();

      const result = await handler.postTweet("Hello World");

      expect(handler.safeHumanClick).toHaveBeenCalledWith(
        mockComposerBtn,
        "Composer Button",
      );
      expect(handler.humanType).toHaveBeenCalled();
      expect(handler.safeHumanClick).toHaveBeenCalledWith(
        mockPostBtn,
        "Post Button",
      );
      expect(result).toBe(true);
      expect(mockAgent.state.tweets).toBe(1);
    });

    it("should use fallback URL if composer button not found", async () => {
      // Mock api.exists and api.visible to return false for composer button
      api.exists.mockResolvedValue(false);
      api.visible.mockResolvedValue(false);

      const mockComposerBtn = {
        count: vi.fn().mockResolvedValue(0),
        isVisible: vi.fn().mockResolvedValue(false),
      };

      mockPage.locator.mockImplementation((selector) => {
        if (selector.includes("SideNav_NewTweet_Button"))
          return mockComposerBtn;
        // ... other locators return defaults
        return {
          waitFor: vi.fn().mockResolvedValue(),
          count: vi.fn().mockResolvedValue(1),
          isVisible: vi.fn().mockResolvedValue(true),
        };
      });

      vi.spyOn(handler, "humanType").mockResolvedValue();

      await handler.postTweet("Hello");

      expect(api.goto).toHaveBeenCalledWith("https://x.com/compose/tweet");
    });

    it("should return false if posting fails", async () => {
      vi.spyOn(handler, "humanType").mockRejectedValue(
        new Error("Typing failed"),
      );

      const result = await handler.postTweet("Hello");

      expect(result).toBe(false);
      expect(mockLogger.info).toHaveBeenCalledWith(
        expect.stringContaining("Failed to post tweet"),
      );
    });
  });

  describe("checkLoginState", () => {
    it('should return false if "Sign in" text is visible', async () => {
      mockPage.getByText.mockReturnValue({
        first: vi
          .fn()
          .mockReturnValue({ isVisible: vi.fn().mockResolvedValue(true) }),
      });

      const result = await handler.checkLoginState();

      expect(result).toBe(false);
      expect(mockAgent.state.consecutiveLoginFailures).toBe(1);
    });

    it("should return false if login selectors are visible", async () => {
      mockPage.getByText.mockReturnValue({
        first: vi
          .fn()
          .mockReturnValue({ isVisible: vi.fn().mockResolvedValue(false) }),
      });

      mockPage.locator.mockReturnValue({
        first: vi
          .fn()
          .mockReturnValue({ isVisible: vi.fn().mockResolvedValue(true) }),
      });

      const result = await handler.checkLoginState();

      expect(result).toBe(false);
    });

    it("should return true if no login indicators found", async () => {
      mockPage.getByText.mockReturnValue({
        first: vi
          .fn()
          .mockReturnValue({ isVisible: vi.fn().mockResolvedValue(false) }),
      });

      // Create a mock for primaryColumn that returns true for visibility
      const mockPrimaryColumn = {
        first: vi
          .fn()
          .mockReturnValue({ isVisible: vi.fn().mockResolvedValue(false) }),
        count: vi.fn().mockResolvedValue(1),
      };

      mockPage.locator.mockReturnValue(mockPrimaryColumn);

      // Mock api.visible to return false for login selectors, true for primaryColumn
      api.visible.mockImplementation((element) => {
        if (element === mockPrimaryColumn) {
          return Promise.resolve(true);
        }
        return Promise.resolve(false);
      });

      const result = await handler.checkLoginState();

      expect(result).toBe(true);
      expect(mockAgent.state.consecutiveLoginFailures).toBe(0);
    });

    it("should return false if primary timeline not visible on home", async () => {
      mockPage.url.mockReturnValue("https://x.com/home");
      mockPage.getByText.mockReturnValue({
        first: () => ({ isVisible: () => Promise.resolve(false) }),
      });

      // Create mock for primaryColumn
      const mockPrimaryColumn = {
        first: () => ({ isVisible: () => Promise.resolve(false) }),
        count: () => Promise.resolve(0),
      };

      mockPage.locator.mockReturnValue(mockPrimaryColumn);

      // Mock api.visible to return false for primaryColumn (triggering the warning)
      api.visible.mockResolvedValue(false);

      const result = await handler.checkLoginState();

      expect(result).toBe(false);
      expect(mockLogger.info).toHaveBeenCalledWith(
        expect.stringContaining("Suspected not logged in"),
      );
    });

    it("should handle errors during check", async () => {
      mockPage.getByText.mockImplementation(() => {
        throw new Error("Check error");
      });

      const result = await handler.checkLoginState();

      expect(result).toBe(false);
      expect(mockLogger.info).toHaveBeenCalledWith(
        expect.stringContaining("checkLoginState failed"),
      );
    });
  });
});
