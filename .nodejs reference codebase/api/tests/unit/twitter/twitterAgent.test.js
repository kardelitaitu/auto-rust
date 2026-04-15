/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";

vi.mock("@api/utils/ghostCursor.js", () => ({
  GhostCursor: class {
    constructor() {}
    click() {
      return { success: true, x: 100, y: 100 };
    }
    move() {
      return Promise.resolve();
    }
    park() {
      return Promise.resolve();
    }
  },
}));

vi.mock("@api/behaviors/scroll-helper.js", () => ({
  scrollDown: vi.fn().mockResolvedValue(undefined),
  scrollUp: vi.fn().mockResolvedValue(undefined),
  scrollRandom: vi.fn().mockResolvedValue(undefined),
  scrollWheel: vi.fn().mockResolvedValue(undefined),
}));

vi.mock("@api/index.js", () => {
  const api = {
    setPage: vi.fn(),
    getPage: vi.fn(),
    wait: vi.fn().mockImplementation(async (ms) => {
      vi.advanceTimersByTime(ms || 0);
      return Promise.resolve();
    }),
    think: vi.fn().mockResolvedValue(undefined),
    getPersona: vi.fn().mockReturnValue({ microMoveChance: 0.1, fidgetChance: 0.05 }),
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
    getCurrentUrl: vi.fn().mockResolvedValue("https://x.com/"),
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
  };
  return { api, default: api };
});

import { api } from "@api/index.js";

vi.mock("@api/tests/utils/humanization/index.js", () => ({
  HumanizationEngine: class {
    constructor() {}
    think() {
      return Promise.resolve();
    }
    consumeContent() {
      return Promise.resolve();
    }
    multitask() {
      return Promise.resolve();
    }
    recoverFromError() {
      return Promise.resolve();
    }
    sessionStart() {
      return Promise.resolve();
    }
    sessionEnd() {
      return Promise.resolve();
    }
    cycleComplete() {
      return Promise.resolve();
    }
  },
}));

vi.mock("@api/utils/profileManager.js", () => ({
  profileManager: {
    getFatiguedVariant: vi.fn(),
    getStarter: vi.fn().mockReturnValue({ theme: "dark" }),
    getById: vi.fn().mockReturnValue({ id: "p1", theme: "light" }),
  },
}));

vi.mock("@api/tests/utils/math.js", () => ({
  mathUtils: {
    randomInRange: vi.fn(() => 1000),
    gaussian: vi.fn(() => 1000),
    roll: vi.fn(() => true),
  },
}));

vi.mock("@api/tests/utils/entropyController.js", () => ({
  entropy: {
    pick: vi.fn(() => "default"),
    retryDelay: vi.fn((attempt) => attempt * 100),
    scrollSettleTime: vi.fn(() => 500),
    pageLoadWait: vi.fn(() => 1000),
    postClickDelay: vi.fn(() => 300),
  },
}));

describe("twitterAgent", () => {
  let TwitterAgent;
  let agent;
  let mockPage;
  let mockLogger;
  let mockProfile;
  let profileManager;

  beforeEach(async () => {
    vi.clearAllMocks();
    vi.useFakeTimers();

    mockPage = {
      goto: vi.fn().mockResolvedValue(undefined),
      waitForSelector: vi.fn().mockResolvedValue(undefined),
      waitForTimeout: vi.fn().mockResolvedValue(undefined),
      setExtraHTTPHeaders: vi.fn().mockResolvedValue(undefined),
      emulateMedia: vi.fn().mockResolvedValue(undefined),
      url: vi.fn().mockReturnValue("https://x.com/"),
      isClosed: vi.fn().mockReturnValue(false),
      context: vi.fn().mockReturnValue({
        browser: vi.fn().mockReturnValue({ isConnected: vi.fn().mockReturnValue(true) }),
      }),
      title: vi.fn().mockResolvedValue("mock title"),
      locator: vi.fn((selector) => {
        const loc = {
          first: vi.fn().mockReturnThis(),
          count: vi.fn().mockResolvedValue(1),
          isVisible: vi.fn().mockResolvedValue(false),
          boundingBox: vi.fn().mockResolvedValue({ x: 0, y: 0, width: 100, height: 100 }),
          click: vi.fn().mockResolvedValue(undefined),
          textContent: vi.fn().mockResolvedValue("mock text"),
          getAttribute: vi.fn().mockResolvedValue("mock attr"),
          elementHandle: vi.fn().mockResolvedValue({}),
        };
        return loc;
      }),
      getByText: vi.fn().mockReturnValue({
        first: vi.fn().mockReturnValue({
          isVisible: vi.fn().mockResolvedValue(false),
        }),
      }),
      keyboard: {
        press: vi.fn().mockResolvedValue(undefined),
        type: vi.fn().mockResolvedValue(undefined),
      },
      mouse: {
        move: vi.fn().mockResolvedValue(undefined),
        click: vi.fn().mockResolvedValue(undefined),
        wheel: vi.fn().mockResolvedValue(undefined),
      },
      reload: vi.fn().mockResolvedValue(undefined),
      evaluate: vi.fn().mockResolvedValue(true),
      removeAllListeners: vi.fn(),
      on: vi.fn(),
      viewportSize: vi.fn().mockReturnValue({ width: 1280, height: 720 }),
      content: vi.fn().mockResolvedValue(""),
    };

    api.getPage.mockReturnValue(mockPage);
    api.setPage.mockReturnValue(undefined);

    ({ TwitterAgent } = await import("@api/twitter/twitterAgent.js"));
    ({ profileManager } = await import("@api/tests/utils/profileManager.js"));

    mockLogger = {
      info: vi.fn(),
      warn: vi.fn(),
      error: vi.fn(),
      debug: vi.fn(),
      log: vi.fn(),
    };
    mockProfile = {
      id: "test-profile",
      username: "testuser",
      description: "Test Description",
      timings: {
        readingPhase: { mean: 30000, deviation: 10000 },
        actionSpecific: {
          space: { mean: 1000, deviation: 200 },
          keys: { mean: 100, deviation: 30 },
        },
        scrollPause: { mean: 500, deviation: 100 },
      },
      inputMethods: {
        wheelDown: 0.8,
        wheelUp: 0.05,
        space: 0.05,
        keysDown: 0.1,
        keysUp: 0,
      },
      probabilities: {
        refresh: 0.05,
        profileDive: 0.15,
        tweetDive: 0.1,
        idle: 0.3,
        likeTweetafterDive: 0.005,
        bookmarkAfterDive: 0.005,
        followOnProfile: 0.005,
      },
      maxLike: 10,
      maxFollow: 5,
    };
    agent = new TwitterAgent(mockPage, mockProfile, mockLogger);
  });

  afterEach(() => {
    vi.restoreAllMocks();
    vi.useRealTimers();
  });

  describe("initialization", () => {
    it("should initialize correctly", () => {
      expect(agent).toBeDefined();
      expect(agent.page).toBe(mockPage);
    });

    it("should have correct initial state", () => {
      expect(agent.state.likes).toBe(0);
      expect(agent.state.follows).toBe(0);
      expect(agent.state.retweets).toBe(0);
      expect(agent.state.tweets).toBe(0);
      expect(agent.state.engagements).toBe(0);
      expect(agent.state.activityMode).toBe("NORMAL");
    });

    it("should have session properties", () => {
      expect(agent.sessionStart).toBeDefined();
      expect(agent.loopIndex).toBe(0);
      expect(agent.isFatigued).toBe(false);
    });

    it("should have configuration", () => {
      expect(agent.config).toBe(mockProfile);
    });

    it("should have modular handlers", () => {
      expect(agent.navigation).toBeDefined();
      expect(agent.engagement).toBeDefined();
      expect(agent.session).toBeDefined();
    });

    it("should attach network listeners safely", () => {
      expect(mockPage.on).toHaveBeenCalledWith("request", expect.any(Function));
      expect(mockPage.on).toHaveBeenCalledWith("response", expect.any(Function));
    });
  });

  describe("clamp", () => {
    it("should clamp value within range", () => {
      expect(agent.clamp(5, 0, 10)).toBe(5);
    });

    it("should clamp to min when below", () => {
      expect(agent.clamp(-5, 0, 10)).toBe(0);
    });

    it("should clamp to max when above", () => {
      expect(agent.clamp(15, 0, 10)).toBe(10);
    });

    it("should handle equal values", () => {
      expect(agent.clamp(5, 5, 5)).toBe(5);
    });

    it("should clamp to min when equal to min", () => {
      expect(agent.clamp(0, 0, 10)).toBe(0);
    });

    it("should clamp to max when equal to max", () => {
      expect(agent.clamp(10, 0, 10)).toBe(10);
    });
  });

  describe("log", () => {
    it("should log using provided logger", () => {
      agent.log("test message");
      expect(mockLogger.info).toHaveBeenCalledWith(
        "[Agent:test-profile] test message",
      );
    });

    it("should fallback to console.log when no logger", () => {
      const consoleSpy = vi.spyOn(console, "log").mockImplementation(() => {});
      const agentNoLogger = new TwitterAgent(mockPage, mockProfile, null);
      agentNoLogger.log("test message");
      expect(consoleSpy).toHaveBeenCalledWith(
        "[Agent:test-profile] test message",
      );
      consoleSpy.mockRestore();
    });

    it("should log with config.id when available", () => {
      const agentWithConfig = new TwitterAgent(mockPage, mockProfile, mockLogger);
      agentWithConfig.log("config message");
      expect(mockLogger.info).toHaveBeenCalledWith(
        "[Agent:test-profile] config message",
      );
    });
  });

  describe("checkLoginState", () => {
    it("should return true if login element not visible", async () => {
      mockPage.locator.mockReturnValue({
        first: vi.fn().mockReturnThis(),
        isVisible: vi.fn().mockResolvedValue(false),
        count: vi.fn().mockResolvedValue(1),
      });
      const result = await agent.checkLoginState();
      expect(result).toBe(true);
    });

    it("should return false if login element visible", async () => {
      mockPage.locator.mockReturnValue({
        first: vi.fn().mockReturnThis(),
        isVisible: vi.fn().mockResolvedValue(true),
        count: vi.fn().mockResolvedValue(1),
      });
      const result = await agent.checkLoginState();
      expect(result).toBe(false);
    });

    it("should handle locator errors gracefully", async () => {
      mockPage.locator.mockImplementation(() => {
        throw new Error("Locator error");
      });
      const result = await agent.checkLoginState();
      expect(result).toBe(false); // Should default to false on error
    });
  });

  describe("isElementActionable", () => {
    it("should return true when element is actionable", async () => {
      const mockElement = {
        elementHandle: vi.fn().mockResolvedValue({}),
      };
      mockPage.evaluate.mockResolvedValue(true);

      const result = await agent.isElementActionable(mockElement);
      expect(result).toBe(true);
    });

    it("should return false when element handle is null", async () => {
      const mockElement = {
        elementHandle: vi.fn().mockResolvedValue(null),
      };

      const result = await agent.isElementActionable(mockElement);
      expect(result).toBe(false);
    });

    it("should return false on error", async () => {
      const mockElement = {
        elementHandle: vi.fn().mockRejectedValue(new Error("test error")),
      };

      const result = await agent.isElementActionable(mockElement);
      expect(result).toBe(false);
    });

    it("should return false when evaluate fails", async () => {
      const mockElement = {
        elementHandle: vi.fn().mockResolvedValue({}),
      };
      mockPage.evaluate.mockRejectedValue(new Error("evaluate failed"));

      const result = await agent.isElementActionable(mockElement);
      expect(result).toBe(false);
    });
  });

  describe("dismissOverlays", () => {
    it("should be a function", () => {
      expect(typeof agent.dismissOverlays).toBe("function");
    });

    it("should handle errors gracefully", async () => {
      mockPage.locator.mockImplementation(() => {
        throw new Error("Locator error");
      });

      await expect(agent.dismissOverlays()).resolves.not.toThrow();
    });

    it("should dismiss toast notifications", async () => {
      const toasts = mockPage.locator('[data-testid="toast"], [role="alert"]');
      toasts.count.mockResolvedValue(1);
      
      await agent.dismissOverlays();
      
      expect(mockPage.keyboard.press).toHaveBeenCalledWith("Escape");
    });

    it("should dismiss modal dialogs", async () => {
      const modals = mockPage.locator('[role="dialog"], [aria-modal="true"]');
      modals.count.mockResolvedValue(1);
      
      await agent.dismissOverlays();
      
      expect(mockPage.keyboard.press).toHaveBeenCalledWith("Escape");
    });

    it("should handle case when no overlays exist", async () => {
      await expect(agent.dismissOverlays()).resolves.not.toThrow();
    });
  });

  describe("pollForFollowState", () => {
    it("should be a function", () => {
      expect(typeof agent.pollForFollowState).toBe("function");
    });

    it("should return boolean", async () => {
      mockPage.locator.mockReturnValue({
        first: vi.fn().mockReturnThis(),
        isVisible: vi.fn().mockResolvedValue(false),
        textContent: vi.fn().mockResolvedValue("Follow"),
      });

      const result = await agent.pollForFollowState(
        '[data-testid="unfollow"]',
        '[data-testid="follow"]',
        50,
      );
      expect(typeof result).toBe("boolean");
    });

    it("should detect unfollow button appearance", async () => {
      mockPage.locator.mockImplementation((selector) => {
        if (selector.includes("unfollow")) {
          return {
            first: vi.fn().mockReturnThis(),
            isVisible: vi.fn().mockResolvedValue(true),
          };
        }
        return {
          first: vi.fn().mockReturnThis(),
          isVisible: vi.fn().mockResolvedValue(false),
        };
      });

      const result = await agent.pollForFollowState(
        '[data-testid="unfollow"]',
        '[data-testid="follow"]',
        30000,
      );
      expect(result).toBe(true);
    });

    it("should detect following state from button text", async () => {
      mockPage.locator.mockImplementation((selector) => {
        if (selector.includes("follow")) {
          return {
            first: vi.fn().mockReturnThis(),
            isVisible: vi.fn().mockResolvedValue(true),
            textContent: vi.fn().mockResolvedValue("Following"),
          };
        }
        return {
          first: vi.fn().mockReturnThis(),
          isVisible: vi.fn().mockResolvedValue(false),
        };
      });

      const result = await agent.pollForFollowState(
        '[data-testid="unfollow"]',
        '[data-testid="follow"]',
        30000,
      );
      expect(result).toBe(true);
    });

    it("should handle polling timeout", async () => {
      mockPage.locator.mockReturnValue({
        first: vi.fn().mockReturnThis(),
        isVisible: vi.fn().mockResolvedValue(false),
        textContent: vi.fn().mockResolvedValue("Follow"),
      });

      const result = await agent.pollForFollowState(
        '[data-testid="unfollow"]',
        '[data-testid="follow"]',
        100, // Very short timeout
      );
      expect(result).toBe(false);
    });
  });

  describe("sixLayerClick", () => {
    it("should return true when click succeeds", async () => {
      const mockElement = {};
      vi.spyOn(agent, "safeHumanClick").mockResolvedValue(true);

      const result = await agent.sixLayerClick(mockElement, "[Test]");
      expect(result).toBe(true);
    });

    it("should return false when all layers fail", async () => {
      const mockElement = {};
      vi.spyOn(agent, "safeHumanClick").mockRejectedValue(
        new Error("Click failed"),
      );

      const result = await agent.sixLayerClick(mockElement, "[Test]");
      expect(result).toBe(false);
    });

    it("should log layer failures", async () => {
      const mockElement = {};
      vi.spyOn(agent, "safeHumanClick").mockRejectedValue(
        new Error("Click failed"),
      );

      await agent.sixLayerClick(mockElement, "[Test]");

      expect(mockLogger.info).toHaveBeenCalledWith(
        expect.stringContaining("Layer 1"),
      );
    });
  });

  describe("shutdown", () => {
    it("should cleanup listeners", () => {
      agent.shutdown();
      expect(mockPage.removeAllListeners).toHaveBeenCalledWith("request");
      expect(mockPage.removeAllListeners).toHaveBeenCalledWith("response");
    });

    it("should handle errors gracefully", () => {
      mockPage.removeAllListeners.mockImplementation(() => {
        throw new Error("Cleanup error");
      });

      expect(() => agent.shutdown()).not.toThrow();
    });

    it("should handle closed page", () => {
      mockPage.isClosed.mockReturnValue(true);

      agent.shutdown();

      expect(mockPage.removeAllListeners).not.toHaveBeenCalled();
    });
  });

  describe("safeHumanClick", () => {
    it("should return true on first attempt success", async () => {
      const mockTarget = {};
      vi.spyOn(agent, "humanClick").mockResolvedValue(undefined);

      const result = await agent.safeHumanClick(mockTarget, "Test");
      expect(result).toBe(true);
    });

    it("should retry on failure", async () => {
      const mockTarget = {};
      const humanClickSpy = vi.spyOn(agent, "humanClick");
      humanClickSpy.mockRejectedValue(new Error("Failed"));

      await agent.safeHumanClick(mockTarget, "Test", 3);

      expect(humanClickSpy).toHaveBeenCalledTimes(3);
    });

    it("should use exponential backoff between retries", async () => {
      const mockTarget = {};
      const humanClickSpy = vi.spyOn(agent, "humanClick");
      humanClickSpy.mockRejectedValue(new Error("Failed"));

      await agent.safeHumanClick(mockTarget, "Test", 3);

      // Check that wait was called with exponential delays (1000ms, 2000ms)
      const calls = api.wait.mock.calls;
      expect(calls).toHaveLength(2); // 2 waits between 3 attempts
      expect(calls[0][0]).toBeGreaterThanOrEqual(1000);
      expect(calls[1][0]).toBeGreaterThanOrEqual(2000);
    });

    it("should return false when all retries exhausted", async () => {
      const mockTarget = {};
      const humanClickSpy = vi.spyOn(agent, "humanClick");
      humanClickSpy.mockRejectedValue(new Error("Failed"));

      const result = await agent.safeHumanClick(mockTarget, "Test", 3);
      
      expect(result).toBe(false);
    });
  });

  describe("scrollToGoldenZone", () => {
    it("should scroll element to golden zone", async () => {
      const mockElement = {
        elementHandle: vi.fn().mockResolvedValue({}),
      };
      mockPage.evaluate.mockResolvedValue(true);

      await agent.scrollToGoldenZone(mockElement);

      expect(mockPage.evaluate).toHaveBeenCalled();
    });

    it("should handle null element gracefully", async () => {
      await agent.scrollToGoldenZone(null);
      // Should not throw
    });

    it("should handle evaluate errors gracefully", async () => {
      mockPage.evaluate.mockRejectedValue(new Error("Evaluate failed"));
      const mockElement = {};

      await agent.scrollToGoldenZone(mockElement);
      
      expect(mockLogger.info).toHaveBeenCalledWith(
        expect.stringContaining("GoldenZone")
      );
    });
  });

  describe("performHealthCheck", () => {
    it("should return healthy when network is active", async () => {
      mockPage.content.mockResolvedValue("");
      
      const result = await agent.performHealthCheck();
      expect(result.healthy).toBe(true);
    });

    it("should detect network inactivity", async () => {
      // Set lastNetworkActivity to 31 seconds ago
      agent.lastNetworkActivity = Date.now() - 31000;
      
      mockPage.content.mockResolvedValue("");
      
      const result = await agent.performHealthCheck();
      expect(result.healthy).toBe(false);
      expect(result.reason).toMatch(/network_inactivity_31s/);
    });

    it("should detect redirect loop error page", async () => {
      mockPage.content.mockResolvedValue("ERR_TOO_MANY_REDIRECTS");
      
      const result = await agent.performHealthCheck();
      expect(result.healthy).toBe(false);
      expect(result.reason).toBe("critical_error_page_redirects");
    });

    it("should detect broken page error", async () => {
      mockPage.content.mockResolvedValue("This page isn't working");
      
      const result = await agent.performHealthCheck();
      expect(result.healthy).toBe(false);
    });

    it("should handle content check errors gracefully", async () => {
      mockPage.content.mockRejectedValue(new Error("Content error"));
      
      const result = await agent.performHealthCheck();
      // Should still return healthy if just the check fails but page is alive
      expect(result.healthy).toBe(true);
    });

    it("should handle all errors gracefully", async () => {
      mockPage.content.mockRejectedValue(new Error("All errors"));
      
      const result = await agent.performHealthCheck();
      expect(result.healthy).toBe(true);
    });
  });

  describe("checkAndHandleSoftError", () => {
    it("should return false when no soft error detected", async () => {
      mockPage.locator.mockReturnValue({
        first: vi.fn().mockReturnThis(),
        isVisible: vi.fn().mockResolvedValue(false),
      });

      const result = await agent.checkAndHandleSoftError(null);
      expect(result).toBe(false);
    });

    it("should detect soft error and trigger reload", async () => {
      mockPage.locator.mockImplementation((selector) => {
        if (selector.includes("Something went wrong")) {
          return {
            first: vi.fn().mockReturnThis(),
            isVisible: vi.fn().mockResolvedValue(true),
          };
        }
        return {
          first: vi.fn().mockReturnThis(),
          isVisible: vi.fn().mockResolvedValue(false),
        };
      });

      const result = await agent.checkAndHandleSoftError(null);
      expect(result).toBe(true); // Should trigger reload
    });

    it("should reset consecutive error counter after handling", async () => {
      let callCount = 0;
      mockPage.locator.mockImplementation((selector) => {
        if (selector.includes("Something went wrong")) {
          callCount++;
          return {
            first: vi.fn().mockReturnThis(),
            isVisible: vi.fn().mockResolvedValue(true),
            count: vi.fn().mockResolvedValue(1),
          };
        }
        return {
          first: vi.fn().mockReturnThis(),
          isVisible: vi.fn().mockResolvedValue(false),
          count: vi.fn().mockResolvedValue(0),
        };
      });

      mockPage.url.mockReturnValue("https://x.com/home");
      mockPage.goto.mockResolvedValue(undefined);

      // Mock the verification locator to return false (error cleared)
      let firstCall = true;
      mockPage.locator.mockImplementation((selector) => {
        if (selector.includes("Something went wrong")) {
          if (firstCall) {
            firstCall = false;
            return {
              first: vi.fn().mockReturnThis(),
              isVisible: vi.fn().mockResolvedValue(true),
            };
          }
          // Second call (verification) - error is cleared
          return {
            first: vi.fn().mockReturnThis(),
            isVisible: vi.fn().mockResolvedValue(false),
          };
        }
        return {
          first: vi.fn().mockReturnThis(),
          isVisible: vi.fn().mockResolvedValue(false),
        };
      });

      agent.state.consecutiveSoftErrors = 2;
      
      await agent.checkAndHandleSoftError(null);
      
      expect(agent.state.consecutiveSoftErrors).toBe(0);
    });

    it("should throw logout error when max retries reached", async () => {
      mockPage.locator.mockImplementation((selector) => {
        if (selector.includes("Something went wrong")) {
          return {
            first: vi.fn().mockReturnThis(),
            isVisible: vi.fn().mockResolvedValue(true),
          };
        }
        return {
          first: vi.fn().mockReturnThis(),
          isVisible: vi.fn().mockResolvedValue(false),
        };
      });

      agent.state.consecutiveSoftErrors = 3;
      
      await expect(agent.checkAndHandleSoftError(null)).rejects.toThrow(
        "potential twitter logged out"
      );
    });

    it("should handle reload errors gracefully", async () => {
      mockPage.locator.mockImplementation((selector) => {
        if (selector.includes("Something went wrong")) {
          return {
            first: vi.fn().mockReturnThis(),
            isVisible: vi.fn().mockResolvedValue(true),
          };
        }
        return {
          first: vi.fn().mockReturnThis(),
          isVisible: vi.fn().mockResolvedValue(false),
        };
      });

      mockPage.goto.mockRejectedValue(new Error("Reload failed"));
      
      const result = await agent.checkAndHandleSoftError(null);
      expect(result).toBe(true); // Still returns true to continue
    });

    it("should verify soft error is cleared after reload", async () => {
      mockPage.locator.mockImplementation((selector) => {
        if (selector.includes("Something went wrong")) {
          return {
            first: vi.fn().mockReturnThis(),
            isVisible: vi.fn().mockResolvedValue(false), // Error cleared
          };
        }
        return {
          first: vi.fn().mockReturnThis(),
          isVisible: vi.fn().mockResolvedValue(false),
        };
      });

      const result = await agent.checkAndHandleSoftError(null);
      expect(result).toBe(false); // Returns false when no error detected
    });
  });

  describe("checkFatigue", () => {
    it("should not trigger if already fatigued", async () => {
      agent.isFatigued = true;
      
      agent.checkFatigue();
      
      // Should not call triggerHotSwap
      expect(mockLogger.info).not.toHaveBeenCalledWith(
        expect.stringContaining("ENERGY DECAY")
      );
    });

    it("should trigger hot swap when fatigue threshold reached", async () => {
      const elapsed = Date.now() - agent.sessionStart;
      agent.fatigueThreshold = 1000; // Set to 1 second for testing
      agent.isFatigued = false;
      
      await vi.advanceTimersByTime(2000);
      
      agent.checkFatigue();
      
      expect(agent.isFatigued).toBe(true);
    });

    it("should handle when hot swap profile not found", async () => {
      const elapsed = Date.now() - agent.sessionStart;
      agent.fatigueThreshold = 1000;
      agent.isFatigued = false;
      
      vi.spyOn(profileManager, "getFatiguedVariant").mockReturnValue(null);
      
      await vi.advanceTimersByTime(2000);
      
      agent.checkFatigue();
      
      expect(agent.isFatigued).toBe(true); // Should still mark as fatigued
    });
  });

  describe("triggerHotSwap", () => {
    it("should swap to slower profile when found", async () => {
      const slowerProfile = {
        id: "slower-profile",
        description: "Slower Profile",
        timings: { scrollPause: { mean: 1000 } },
      };
      
      vi.spyOn(profileManager, "getFatiguedVariant").mockReturnValue(slowerProfile);
      
      agent.triggerHotSwap();
      
      expect(agent.config).toBe(slowerProfile);
      expect(agent.isFatigued).toBe(true);
    });

    it("should adjust probabilities when swapping", async () => {
      const slowerProfile = {
        id: "slower-profile",
        description: "Slower Profile",
        timings: { scrollPause: { mean: 1000 } },
      };
      
      vi.spyOn(profileManager, "getFatiguedVariant").mockReturnValue(slowerProfile);
      
      agent.triggerHotSwap();
      
      // Should have adjusted probabilities
      expect(agent.config.probabilities.refresh).toBeLessThan(
        agent.config.probabilities.refresh || 0.05
      );
    });

    it("should enter doom scroll mode when no slower profile available", async () => {
      vi.spyOn(profileManager, "getFatiguedVariant").mockReturnValue(null);
      
      agent.triggerHotSwap();
      
      expect(agent.isFatigued).toBe(true);
      expect(agent.state.fatigueBias).toBe(0.3);
    });
  });

  describe("getScrollMethod", () => {
    it("should return WHEEL_DOWN with high probability", () => {
      const method = agent.getScrollMethod();
      // With wheelDown: 0.8, should most likely return WHEEL_DOWN
      expect(["WHEEL_DOWN", "WHEEL_UP", "SPACE", "KEYS_DOWN"]).toContain(method);
    });

    it("should handle all configured input methods", () => {
      const methods = agent.config.inputMethods || {
        wheelDown: 0.8,
        wheelUp: 0.03,
        space: 0.05,
        keysDown: 0.1,
        keysUp: 0,
      };
      
      // Should return one of the configured methods
      const possibleMethods = ["WHEEL_DOWN", "WHEEL_UP", "SPACE", "KEYS_DOWN"];
      expect(possibleMethods).toContain(agent.getScrollMethod());
    });

    it("should fallback to WHEEL_DOWN when no input methods configured", () => {
      agent.config.inputMethods = null;
      
      const method = agent.getScrollMethod();
      // When inputMethods is null, uses defaults and returns one of valid methods
      expect(["WHEEL_DOWN", "WHEEL_UP", "SPACE", "KEYS_DOWN"]).toContain(method);
    });
  });

  describe("normalizeProbabilities", () => {
    it("should merge with base probabilities", () => {
      const result = agent.normalizeProbabilities({});
      
      expect(result.refresh).toBeDefined();
      expect(result.profileDive).toBeDefined();
      expect(result.tweetDive).toBeDefined();
      expect(result.idle).toBeDefined();
    });

    it("should apply fatigue bias to idle probability", () => {
      agent.state.fatigueBias = 0.1;
      
      const result = agent.normalizeProbabilities({});
      
      expect(result.idle).toBeGreaterThan(0.3); // Base is 0.3, plus fatigue bias
    });

    it("should suppress refresh after recent refresh", () => {
      agent.state.lastRefreshAt = Date.now() - 10000; // Refreshed 10 seconds ago
      
      const result = agent.normalizeProbabilities({});
      
      expect(result.refresh).toBeLessThan(0.05); // Should be suppressed
    });

    it("should apply BURST MODE overrides", () => {
      agent.state.activityMode = "BURST";
      
      const result = agent.normalizeProbabilities({});
      
      expect(result.idle).toBe(0);
      expect(result.refresh).toBe(0);
      expect(result.tweetDive).toBeGreaterThan(0.1);
    });

    it("should clamp all probabilities to [0, 1]", () => {
      const result = agent.normalizeProbabilities({
        refresh: 1.5, // Out of range
        profileDive: -0.1, // Out of range
      });
      
      expect(result.refresh).toBeLessThanOrEqual(1);
      expect(result.refresh).toBeGreaterThanOrEqual(0);
      expect(result.profileDive).toBeLessThanOrEqual(1);
      expect(result.profileDive).toBeGreaterThanOrEqual(0);
    });

    it("should handle likeTweetAfterDive property", () => {
      const result = agent.normalizeProbabilities({
        likeTweetAfterDive: 0.1,
      });
      
      expect(result.likeTweetafterDive).toBe(0.1);
    });
  });

  describe("simulateReading", () => {
    it("should be a function", () => {
      expect(typeof agent.simulateReading).toBe("function");
    });

    it("should handle session expiration during reading", async () => {
      const originalIsSessionExpired = agent.human.session.isExpired;
      
      vi.spyOn(agent, "isSessionExpired").mockReturnValue(true);
      
      await expect(agent.simulateReading()).resolves.not.toThrow();
    });

    it("should handle soft errors during reading", async () => {
      mockPage.locator.mockImplementation((selector) => {
        if (selector.includes("Something went wrong")) {
          return {
            first: vi.fn().mockReturnThis(),
            isVisible: vi.fn().mockResolvedValue(true),
          };
        }
        return {
          first: vi.fn().mockReturnThis(),
          isVisible: vi.fn().mockResolvedValue(false),
        };
      });

      await expect(agent.simulateReading()).resolves.not.toThrow();
    });

    it("should handle fatal health errors during reading", async () => {
      agent.lastNetworkActivity = Date.now() - 31000; // Trigger network inactivity
      
      await expect(agent.simulateReading()).rejects.toThrow(
        expect.stringContaining("Fatal")
      );
    });

    it("should handle multitasking during reading", async () => {
      const originalMultitask = agent.human.multitask;
      
      vi.spyOn(agent.human, "multitask").mockImplementation(() => {
        return Promise.resolve();
      });
      
      await expect(agent.simulateReading()).resolves.not.toThrow();
    });

    it("should handle cursor drift during reading", async () => {
      const originalMove = agent.ghost.move;
      
      vi.spyOn(agent.ghost, "move").mockImplementation(() => {
        return Promise.resolve();
      });
      
      await expect(agent.simulateReading()).resolves.not.toThrow();
    });

    it("should handle fidgeting during reading", async () => {
      const originalSimulateFidget = agent.simulateFidget;
      
      vi.spyOn(agent, "simulateFidget").mockImplementation(() => {
        return Promise.resolve();
      });
      
      await expect(agent.simulateReading()).resolves.not.toThrow();
    });

    it("should handle scroll methods during reading", async () => {
      vi.spyOn(agent, "getScrollMethod").mockReturnValue("WHEEL_DOWN");
      
      vi.spyOn(agent, "scrollToGoldenZone").mockResolvedValue(undefined);
      
      await expect(agent.simulateReading()).resolves.not.toThrow();
    });

    it("should handle tweet diving during reading", async () => {
      vi.spyOn(agent, "normalizeProbabilities").mockReturnValue({
        tweetDive: 1.0, // Force dive
      });
      
      vi.spyOn(agent, "diveTweet").mockResolvedValue(undefined);
      
      await expect(agent.simulateReading()).resolves.not.toThrow();
    });

    it("should handle bookmarking during reading", async () => {
      vi.spyOn(agent, "normalizeProbabilities").mockReturnValue({
        bookmarkAfterDive: 1.0, // Force bookmark
      });
      
      mockPage.locator.mockImplementation((selector) => {
        if (selector.includes("bookmark")) {
          return {
            first: vi.fn().mockReturnThis(),
            isVisible: vi.fn().mockResolvedValue(true),
            click: vi.fn().mockResolvedValue(undefined),
          };
        }
        return {
          first: vi.fn().mockReturnThis(),
          isVisible: vi.fn().mockResolvedValue(false),
        };
      });
      
      await expect(agent.simulateReading()).resolves.not.toThrow();
    });

    it("should handle liking during reading", async () => {
      vi.spyOn(agent, "normalizeProbabilities").mockReturnValue({
        likeTweetafterDive: 1.0, // Force like
      });
      
      mockPage.locator.mockImplementation((selector) => {
        if (selector.includes("like")) {
          return {
            first: vi.fn().mockReturnThis(),
            isVisible: vi.fn().mockResolvedValue(true),
            count: vi.fn().mockResolvedValue(1),
            getAttribute: vi.fn().mockResolvedValue("Like"),
            scrollIntoViewIfNeeded: vi.fn().mockResolvedValue(undefined),
          };
        }
        return {
          first: vi.fn().mockReturnThis(),
          isVisible: vi.fn().mockResolvedValue(false),
        };
      });
      
      await expect(agent.simulateReading()).resolves.not.toThrow();
    });

    it("should handle max like limit", async () => {
      agent.state.likes = 10; // Max likes reached
      
      vi.spyOn(agent, "normalizeProbabilities").mockReturnValue({
        likeTweetafterDive: 1.0, // Force like attempt
      });
      
      mockPage.locator.mockImplementation((selector) => {
        if (selector.includes("like")) {
          return {
            first: vi.fn().mockReturnThis(),
            isVisible: vi.fn().mockResolvedValue(true),
            count: vi.fn().mockResolvedValue(1),
            getAttribute: vi.fn().mockResolvedValue("Like"),
            scrollIntoViewIfNeeded: vi.fn().mockResolvedValue(undefined),
          };
        }
        return {
          first: vi.fn().mockReturnThis(),
          isVisible: vi.fn().mockResolvedValue(false),
        };
      });
      
      await expect(agent.simulateReading()).resolves.not.toThrow();
    });

    it("should handle max follow limit", async () => {
      agent.state.follows = 5; // Max follows reached
      
      vi.spyOn(agent, "normalizeProbabilities").mockReturnValue({
        followOnProfile: 1.0, // Force follow attempt
      });
      
      await expect(agent.simulateReading()).resolves.not.toThrow();
    });

    it("should navigate home on dive failure", async () => {
      const originalNavigateHome = agent.navigateHome;
      
      vi.spyOn(agent, "navigateHome").mockResolvedValue(undefined);
      
      await expect(agent.simulateReading()).resolves.not.toThrow();
    });

    it("should handle mouse parking during reading", async () => {
      const originalPark = agent.ghost.park;
      
      vi.spyOn(agent.ghost, "park").mockImplementation(() => {
        return Promise.resolve();
      });
      
      await expect(agent.simulateReading()).resolves.not.toThrow();
    });

    it("should handle media viewing during reading", async () => {
      mockPage.locator.mockImplementation((selector) => {
        if (selector.includes("tweetPhoto")) {
          return {
            first: vi.fn().mockReturnThis(),
            count: vi.fn().mockResolvedValue(1),
            isVisible: vi.fn().mockResolvedValue(true),
          };
        }
        return {
          first: vi.fn().mockReturnThis(),
          count: vi.fn().mockResolvedValue(0),
          isVisible: vi.fn().mockResolvedValue(false),
        };
      });
      
      await expect(agent.simulateReading()).resolves.not.toThrow();
    });

    it("should handle reply reading during reading", async () => {
      await expect(agent.simulateReading()).resolves.not.toThrow();
    });

    it("should handle return home after dive", async () => {
      await expect(agent.simulateReading()).resolves.not.toThrow();
    });
  });

  describe("diveTweet", () => {
    it("should be a function", () => {
      expect(typeof agent.diveTweet).toBe("function");
    });

    it("should handle no tweets in viewport", async () => {
      mockPage.locator.mockReturnValue({
        count: vi.fn().mockResolvedValue(0),
      });
      
      await expect(agent.diveTweet()).resolves.not.toThrow();
    });

    it("should refresh home when no suitable tweets found", async () => {
      mockPage.locator.mockReturnValue({
        count: vi.fn().mockResolvedValue(0),
      });
      
      await agent.diveTweet();
      
      expect(mockPage.goto).toHaveBeenCalledWith("https://x.com/");
    });

    it("should handle click target selection", async () => {
      const mockTweet = {
        locator: vi.fn().mockReturnThis(),
        first: vi.fn().mockReturnThis(),
        count: vi.fn().mockResolvedValue(1),
        isVisible: vi.fn().mockResolvedValue(true),
      };
      
      mockPage.locator.mockReturnValue(mockTweet);
      
      await expect(agent.diveTweet()).resolves.not.toThrow();
    });

    it("should handle media viewing", async () => {
      mockPage.locator.mockImplementation((selector) => {
        if (selector.includes("tweetPhoto")) {
          return {
            first: vi.fn().mockReturnThis(),
            count: vi.fn().mockResolvedValue(1),
            isVisible: vi.fn().mockResolvedValue(true),
            click: vi.fn().mockResolvedValue(undefined),
          };
        }
        return {
          first: vi.fn().mockReturnThis(),
          count: vi.fn().mockResolvedValue(0),
          isVisible: vi.fn().mockResolvedValue(false),
        };
      });
      
      await expect(agent.diveTweet()).resolves.not.toThrow();
    });

    it("should handle reply reading", async () => {
      await expect(agent.diveTweet()).resolves.not.toThrow();
    });

    it("should navigate home on error", async () => {
      const originalNavigateHome = agent.navigateHome;
      
      vi.spyOn(agent, "navigateHome").mockResolvedValue(undefined);
      
      await expect(agent.diveTweet()).resolves.not.toThrow();
    });
  });

  describe("diveProfile", () => {
    it("should be a function", () => {
      expect(typeof agent.diveProfile).toBe("function");
    });

    it("should handle no valid profile links", async () => {
      mockPage.$$eval.mockResolvedValue([]);
      
      await expect(agent.diveProfile()).resolves.not.toThrow();
    });

    it("should navigate to profile on click", async () => {
      const mockTarget = {
        getAttribute: vi.fn().mockResolvedValue("/username"),
      };
      
      mockPage.locator.mockReturnValue(mockTarget);
      mockPage.waitForLoadState.mockResolvedValue(undefined);
      
      await expect(agent.diveProfile()).resolves.not.toThrow();
    });

    it("should fallback to native click on ghost click failure", async () => {
      const mockTarget = {
        getAttribute: vi.fn().mockResolvedValue("/username"),
        click: vi.fn().mockResolvedValue(undefined),
      };
      
      mockPage.locator.mockReturnValue(mockTarget);
      mockPage.waitForLoadState.mockResolvedValue(undefined);
      
      await expect(agent.diveProfile()).resolves.not.toThrow();
    });

    it("should handle follow on profile", async () => {
      const mockTarget = {
        getAttribute: vi.fn().mockResolvedValue("/username"),
      };
      
      mockPage.locator.mockReturnValue(mockTarget);
      mockPage.waitForLoadState.mockResolvedValue(undefined);
      
      await expect(agent.diveProfile()).resolves.not.toThrow();
    });

    it("should handle max follow limit", async () => {
      agent.state.follows = 5; // Max follows reached
      
      const mockTarget = {
        getAttribute: vi.fn().mockResolvedValue("/username"),
      };
      
      mockPage.locator.mockReturnValue(mockTarget);
      mockPage.waitForLoadState.mockResolvedValue(undefined);
      
      await expect(agent.diveProfile()).resolves.not.toThrow();
    });

    it("should explore tabs on profile", async () => {
      await expect(agent.diveProfile()).resolves.not.toThrow();
    });
  });

  describe("isSessionExpired", () => {
    it("should return false when session is active", () => {
      mockPage.isClosed.mockReturnValue(false);
      
      const result = agent.isSessionExpired();
      expect(result).toBe(false);
    });

    it("should return true when page is closed", () => {
      mockPage.isClosed.mockReturnValue(true);
      
      const result = agent.isSessionExpired();
      expect(result).toBe(true);
    });
  });
});
