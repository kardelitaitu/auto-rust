/**
 * Auto-AI Framework - Twitter Agent Index Tests
 * Tests for twitter-agent module exports and context
 */

import { describe, it, expect, vi, beforeEach } from "vitest";

// Mock BaseHandler
vi.mock("@api/twitter/twitter-agent/BaseHandler.js", () => ({
  BaseHandler: class MockBaseHandler {
    constructor(agent) {
      this.agent = agent;
      this.page = agent.page;
      this.config = agent.config;
      this.logger = agent.logger;
      this.state = agent.state;
      this.human = agent.human;
      this.ghost = agent.ghost;
      this.mathUtils = agent.mathUtils;
    }

    log(msg) {
      this.logger?.info(msg);
    }
  },
}));

// Mock dependencies
vi.mock("@api/index.js", () => ({
  api: {
    goto: vi.fn().mockResolvedValue(),
    wait: vi.fn().mockResolvedValue(),
    visible: vi.fn().mockResolvedValue(true),
  },
}));

vi.mock("@api/utils/math.js", () => ({
  mathUtils: {
    roll: vi.fn().mockReturnValue(false),
    randomInRange: vi.fn().mockReturnValue(100),
    random: vi.fn().mockReturnValue(0.5),
  },
}));

vi.mock("@api/behaviors/scroll-helper.js", () => ({
  scrollRandom: vi.fn().mockResolvedValue(),
  scrollDown: vi.fn().mockResolvedValue(),
}));

describe("twitter-agent module - Check Imports/Context", () => {
  let mockAgent;

  beforeEach(() => {
    vi.clearAllMocks();

    mockAgent = {
      page: {
        locator: vi.fn().mockReturnValue({
          first: vi.fn().mockReturnValue({
            isVisible: vi.fn().mockResolvedValue(true),
          }),
        }),
        waitForSelector: vi.fn().mockResolvedValue(),
        waitForTimeout: vi.fn().mockResolvedValue(),
        keyboard: { press: vi.fn() },
        mouse: { move: vi.fn(), click: vi.fn() },
      },
      config: { theme: "dark" },
      logger: {
        info: vi.fn(),
        warn: vi.fn(),
        error: vi.fn(),
      },
      state: {
        consecutiveLoginFailures: 0,
        isFatigued: false,
        engagements: 0,
      },
      human: {
        think: vi.fn().mockResolvedValue(),
      },
      ghost: {
        click: vi.fn().mockResolvedValue({ success: true }),
        move: vi.fn().mockResolvedValue(),
      },
      mathUtils: {
        roll: vi.fn().mockReturnValue(false),
        randomInRange: vi.fn().mockReturnValue(100),
        random: vi.fn().mockReturnValue(0.5),
      },
      navigation: {
        navigateHome: vi.fn().mockResolvedValue(),
        ensureForYouTab: vi.fn().mockResolvedValue(),
      },
      engagement: {
        diveTweet: vi.fn().mockResolvedValue(),
        diveProfile: vi.fn().mockResolvedValue(),
      },
    };
  });

  it("should import NavigationHandler", async () => {
    const { NavigationHandler } =
      await import("../../../../twitter/twitter-agent/NavigationHandler.js");
    expect(NavigationHandler).toBeDefined();
    expect(typeof NavigationHandler).toBe("function");
  });

  it("should import SessionHandler", async () => {
    const { SessionHandler } =
      await import("../../../../twitter/twitter-agent/SessionHandler.js");
    expect(SessionHandler).toBeDefined();
    expect(typeof SessionHandler).toBe("function");
  });

  it("should import InteractionHandler", async () => {
    const { InteractionHandler } =
      await import("../../../../twitter/twitter-agent/InteractionHandler.js");
    expect(InteractionHandler).toBeDefined();
    expect(typeof InteractionHandler).toBe("function");
  });

  it("should import EngagementHandler", async () => {
    const { EngagementHandler } =
      await import("../../../../twitter/twitter-agent/EngagementHandler.js");
    expect(EngagementHandler).toBeDefined();
    expect(typeof EngagementHandler).toBe("function");
  });

  it("should import BaseHandler", async () => {
    const { BaseHandler } =
      await import("@api/twitter/twitter-agent/BaseHandler.js");
    expect(BaseHandler).toBeDefined();
    expect(typeof BaseHandler).toBe("function");
  });

  it("should create NavigationHandler instance", async () => {
    const { NavigationHandler } =
      await import("../../../../twitter/twitter-agent/NavigationHandler.js");
    const handler = new NavigationHandler(mockAgent);
    expect(handler).toBeDefined();
    expect(handler.page).toBe(mockAgent.page);
  });

  it("should create SessionHandler instance", async () => {
    const { SessionHandler } =
      await import("../../../../twitter/twitter-agent/SessionHandler.js");
    const handler = new SessionHandler(mockAgent);
    expect(handler).toBeDefined();
    expect(handler.page).toBe(mockAgent.page);
  });

  it("should create InteractionHandler instance", async () => {
    const { InteractionHandler } =
      await import("../../../../twitter/twitter-agent/InteractionHandler.js");
    const handler = new InteractionHandler(mockAgent);
    expect(handler).toBeDefined();
    expect(handler.page).toBe(mockAgent.page);
  });

  it("should create EngagementHandler instance", async () => {
    const { EngagementHandler } =
      await import("../../../../twitter/twitter-agent/EngagementHandler.js");
    const handler = new EngagementHandler(mockAgent);
    expect(handler).toBeDefined();
    expect(handler.page).toBe(mockAgent.page);
  });
});
