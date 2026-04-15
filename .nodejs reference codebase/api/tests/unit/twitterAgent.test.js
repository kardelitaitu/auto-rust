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

vi.mock("@api/utils/humanization/index.js", () => ({
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
    session = { shouldEndSession: () => true };
  },
}));
vi.mock("@api/utils/profileManager.js", () => ({
  profileManager: {
    getFatiguedVariant: vi.fn(),
    getStarter: vi.fn().mockReturnValue({ theme: "dark" }),
    getById: vi.fn().mockReturnValue({ id: "p1", theme: "light" }),
  },
}));
vi.mock("@api/utils/math.js", () => ({
  mathUtils: {
    randomInRange: vi.fn(() => 1000),
    gaussian: vi.fn(() => 1000),
    roll: vi.fn(() => true),
  },
}));
vi.mock("@api/utils/entropyController.js", () => ({
  entropy: {
    pick: vi.fn(() => "default"),
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
        browser: vi
          .fn()
          .mockReturnValue({ isConnected: vi.fn().mockReturnValue(true) }),
      }),
      title: vi.fn().mockResolvedValue("mock title"),
      locator: vi.fn().mockReturnValue({
        first: vi.fn().mockReturnThis(),
        count: vi.fn().mockResolvedValue(1),
        isVisible: vi.fn().mockResolvedValue(true),
        boundingBox: vi
          .fn()
          .mockResolvedValue({ x: 0, y: 0, width: 100, height: 100 }),
        click: vi.fn().mockResolvedValue(undefined),
        textContent: vi.fn().mockResolvedValue("mock text"),
        getAttribute: vi.fn().mockResolvedValue("mock attr"),
        elementHandle: vi.fn().mockResolvedValue({}),
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
    };

    api.getPage.mockReturnValue(mockPage);
    api.setPage.mockReturnValue(undefined);

    ({ TwitterAgent } = await import("../../twitter/twitterAgent.js"));
    ({ profileManager } = await import("@api/utils/profileManager.js"));

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
      inputMethods: {
        wheelDown: 0.8,
        wheelUp: 0.05,
        space: 0.05,
        keysDown: 0.1,
        keysUp: 0,
      },
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

  describe("function existence", () => {
    it("navigateHome should be a function", () => {
      expect(typeof agent.navigateHome).toBe("function");
    });

    it("postTweet should be a function", () => {
      expect(typeof agent.postTweet).toBe("function");
    });

    it("humanClick should be a function", () => {
      expect(typeof agent.humanClick).toBe("function");
    });

    it("safeHumanClick should be a function", () => {
      expect(typeof agent.safeHumanClick).toBe("function");
    });

    it("robustFollow should be a function", () => {
      expect(typeof agent.robustFollow).toBe("function");
    });

    it("scrollToGoldenZone should be a function", () => {
      expect(typeof agent.scrollToGoldenZone).toBe("function");
    });
  });
});
