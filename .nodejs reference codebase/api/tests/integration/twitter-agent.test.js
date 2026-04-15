/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Integration tests for TwitterAgent module
 */

import { describe, it, expect, vi, beforeEach } from "vitest";

vi.mock("@api/core/logger.js", () => ({
  createLogger: vi.fn(() => ({
    info: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
    debug: vi.fn(),
    success: vi.fn(),
  })),
}));

vi.mock("@api/utils/math.js", () => ({
  mathUtils: {
    gaussian: vi.fn((mean) => mean),
    randomInRange: vi.fn((min, max) => (min + max) / 2),
    roll: vi.fn().mockReturnValue(false),
  },
}));

vi.mock("@api/utils/entropyController.js", () => ({
  entropy: { add: vi.fn() },
}));

vi.mock("@api/utils/profileManager.js", () => ({
  profileManager: { get: vi.fn().mockReturnValue({}) },
}));

vi.mock("@api/behaviors/scroll-helper.js", () => ({
  scrollDown: vi.fn(),
  scrollRandom: vi.fn(),
}));

vi.mock("@api/twitter/twitter-agent/NavigationHandler.js", () => ({
  NavigationHandler: vi.fn().mockImplementation(() => ({
    navigateHome: vi.fn().mockResolvedValue(true),
    ensureForYouTab: vi.fn().mockResolvedValue(true),
  })),
}));

vi.mock("@api/twitter/twitter-agent/EngagementHandler.js", () => ({
  EngagementHandler: vi.fn().mockImplementation(() => ({
    handleLike: vi.fn().mockResolvedValue(true),
  })),
}));

vi.mock("@api/twitter/twitter-agent/SessionHandler.js", () => ({
  SessionHandler: vi.fn().mockImplementation(() => ({
    checkLoginState: vi.fn().mockResolvedValue(true),
  })),
}));

vi.mock("@api/utils/engagement-limits.js", () => ({
  engagementLimits: {
    createEngagementTracker: vi.fn().mockReturnValue({
      canPerform: vi.fn().mockReturnValue(true),
      record: vi.fn().mockReturnValue(true),
      getProgress: vi.fn().mockReturnValue("0/5"),
      getStatus: vi.fn().mockReturnValue({}),
      getSummary: vi.fn().mockReturnValue("Summary"),
      getUsageRate: vi.fn().mockReturnValue("25%"),
    }),
  },
}));

vi.mock("@api/behaviors/humanization/index.js", () => ({
  HumanizationEngine: vi.fn().mockImplementation(() => ({
    think: vi.fn().mockResolvedValue(undefined),
    recoverFromError: vi.fn().mockResolvedValue(undefined),
  })),
}));

vi.mock("@api/utils/ghostCursor.js", () => ({
  GhostCursor: vi.fn().mockImplementation(() => ({
    click: vi.fn().mockResolvedValue({ success: true, x: 100, y: 100 }),
  })),
}));

vi.mock("@api/index.js", () => ({
  api: {
    wait: vi.fn().mockResolvedValue(undefined),
  },
}));

describe("TwitterAgent Integration", () => {
  let TwitterAgent;

  beforeEach(async () => {
    vi.clearAllMocks();
    vi.resetModules();
    const mod = await import("../../twitter/twitterAgent.js");
    TwitterAgent = mod.TwitterAgent;
  }, 30000);

  it("should export TwitterAgent class", () => {
    expect(TwitterAgent).toBeDefined();
    expect(typeof TwitterAgent).toBe("function");
  });

  describe("constructor", () => {
    let mockPage;
    let mockLogger;
    let mockConfig;

    beforeEach(() => {
      mockLogger = {
        info: vi.fn(),
        warn: vi.fn(),
        error: vi.fn(),
      };
      mockConfig = { id: "test" };
      mockPage = {
        locator: vi.fn(),
        keyboard: {},
        waitForSelector: vi.fn().mockResolvedValue(true),
        url: vi.fn().mockReturnValue("https://twitter.com"),
        mouse: { move: vi.fn() },
        on: vi.fn(),
      };
    });

    it("should create instance with valid inputs", () => {
      const agent = new TwitterAgent(mockPage, mockConfig, mockLogger);
      expect(agent).toBeDefined();
    });

    it("should store page reference", () => {
      const agent = new TwitterAgent(mockPage, mockConfig, mockLogger);
      expect(agent.page).toBe(mockPage);
    });

    it("should store config", () => {
      const agent = new TwitterAgent(mockPage, mockConfig, mockLogger);
      expect(agent.config).toBe(mockConfig);
    });

    it("should store logger", () => {
      const agent = new TwitterAgent(mockPage, mockConfig, mockLogger);
      expect(agent.logger).toBe(mockLogger);
    });

    it("should initialize sessionStart", () => {
      const agent = new TwitterAgent(mockPage, mockConfig, mockLogger);
      expect(agent.sessionStart).toBeDefined();
      expect(typeof agent.sessionStart).toBe("number");
    });

    it("should initialize state object", () => {
      const agent = new TwitterAgent(mockPage, mockConfig, mockLogger);
      expect(agent.state).toBeDefined();
    });

    it("should have human property", () => {
      const agent = new TwitterAgent(mockPage, mockConfig, mockLogger);
      expect(agent.human).toBeDefined();
    });

    it("should have ghost property", () => {
      const agent = new TwitterAgent(mockPage, mockConfig, mockLogger);
      expect(agent.ghost).toBeDefined();
    });

    it("should have state.tabs", () => {
      const agent = new TwitterAgent(mockPage, mockConfig, mockLogger);
      expect(agent.state.tabs).toBeDefined();
      expect(agent.state.tabs.preferForYou).toBe(true);
    });

    it("should handle null logger", () => {
      const agent = new TwitterAgent(mockPage, mockConfig, null);
      expect(agent).toBeDefined();
    });

    it("should handle empty config", () => {
      const agent = new TwitterAgent(mockPage, {}, mockLogger);
      expect(agent).toBeDefined();
    });
  });

  describe("instance properties", () => {
    let agent;
    let mockPage;
    let mockLogger;
    let mockConfig;

    beforeEach(() => {
      mockLogger = { info: vi.fn() };
      mockConfig = { id: "test" };
      mockPage = {
        locator: vi.fn(),
        keyboard: {},
        waitForSelector: vi.fn().mockResolvedValue(true),
        url: vi.fn().mockReturnValue("https://twitter.com"),
        mouse: { move: vi.fn() },
        on: vi.fn(),
      };
      agent = new TwitterAgent(mockPage, mockConfig, mockLogger);
    });

    it("should have navigation handler", () => {
      expect(agent.navigation).toBeDefined();
    });

    it("should have engagement handler", () => {
      expect(agent.engagement).toBeDefined();
    });

    it("should have session handler", () => {
      expect(agent.session).toBeDefined();
    });
  });

  describe("clamp method", () => {
    let agent;
    let mockPage;
    let mockLogger;
    let mockConfig;

    beforeEach(() => {
      mockLogger = { info: vi.fn() };
      mockConfig = { id: "test" };
      mockPage = {
        locator: vi.fn(),
        keyboard: {},
        waitForSelector: vi.fn().mockResolvedValue(true),
        url: vi.fn().mockReturnValue("https://twitter.com"),
        mouse: { move: vi.fn() },
        on: vi.fn(),
      };
      agent = new TwitterAgent(mockPage, mockConfig, mockLogger);
    });

    it("should return value when within range", () => {
      expect(agent.clamp(5, 0, 10)).toBe(5);
    });

    it("should clamp value above max", () => {
      expect(agent.clamp(15, 0, 10)).toBe(10);
    });

    it("should clamp value below min", () => {
      expect(agent.clamp(-5, 0, 10)).toBe(0);
    });

    it("should handle boundary values", () => {
      expect(agent.clamp(0, 0, 10)).toBe(0);
      expect(agent.clamp(10, 0, 10)).toBe(10);
    });
  });

  describe("log method", () => {
    let mockPage;
    let mockLogger;
    let mockConfig;

    beforeEach(() => {
      mockLogger = { info: vi.fn() };
      mockConfig = { id: "test" };
      mockPage = {
        locator: vi.fn(),
        keyboard: {},
        waitForSelector: vi.fn().mockResolvedValue(true),
        url: vi.fn().mockReturnValue("https://twitter.com"),
        mouse: { move: vi.fn() },
        on: vi.fn(),
      };
    });

    it("should log with logger when available", () => {
      const agent = new TwitterAgent(mockPage, mockConfig, mockLogger);
      agent.log("test message");
      expect(mockLogger.info).toHaveBeenCalledWith("[Agent:test] test message");
    });

    it("should handle null logger with console.log", () => {
      const agent = new TwitterAgent(mockPage, mockConfig, null);
      const consoleSpy = vi.spyOn(console, "log").mockImplementation(() => {});
      agent.log("test message");
      expect(consoleSpy).toHaveBeenCalledWith("[Agent:test] test message");
      consoleSpy.mockRestore();
    });
  });

  describe("shutdown method", () => {
    let agent;
    let mockPage;
    let mockLogger;
    let mockConfig;

    beforeEach(() => {
      mockLogger = { info: vi.fn() };
      mockConfig = { id: "test" };
      mockPage = {
        locator: vi.fn(),
        keyboard: {},
        waitForSelector: vi.fn().mockResolvedValue(true),
        url: vi.fn().mockReturnValue("https://twitter.com"),
        mouse: { move: vi.fn() },
        on: vi.fn(),
        removeAllListeners: vi.fn(),
        isClosed: vi.fn().mockReturnValue(false),
      };
      agent = new TwitterAgent(mockPage, mockConfig, mockLogger);
    });

    it("should remove network listeners", () => {
      agent.shutdown();
      expect(mockPage.removeAllListeners).toHaveBeenCalledWith("request");
      expect(mockPage.removeAllListeners).toHaveBeenCalledWith("response");
    });

    it("should handle closed page gracefully", () => {
      mockPage.isClosed = vi.fn().mockReturnValue(true);
      expect(() => agent.shutdown()).not.toThrow();
    });
  });
});
