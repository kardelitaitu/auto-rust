/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { AITwitterAgent } from "@api/twitter/ai-twitterAgent.js";

// Hoisted mock classes
const actionMocks = vi.hoisted(() => {
  class MockAction {
    constructor(agent) {
      this.agent = agent;
      this.enabled = true;
      this.getStats = () => ({});
    }
  }

  class MockActionRunner {
    constructor(agent, actions) {
      this.agent = agent;
      this.actions = actions;
      this.getStats = () => ({});
    }
  }

  return { MockAction, MockActionRunner };
});

// Mock dependencies
vi.mock("@api/utils/async-queue.js", () => ({
  DiveQueue: class {
    constructor() {
      this.canEngage = () => true;
      this.recordEngagement = () => true;
      this.getEngagementProgress = () => ({
        likes: { current: 1, limit: 10, remaining: 9, percentUsed: 10 },
        replies: { current: 1, limit: 10, remaining: 9, percentUsed: 10 },
        retweets: { current: 1, limit: 10, remaining: 9, percentUsed: 10 },
        quotes: { current: 1, limit: 10, remaining: 9, percentUsed: 10 },
        follows: { current: 1, limit: 10, remaining: 9, percentUsed: 10 },
        bookmarks: { current: 1, limit: 10, remaining: 9, percentUsed: 10 },
      });
      this.disableQuickMode = () => {};
      this.resetEngagement = () => {};
      this.enqueue = () => {};
      this.on = () => {};
    }
  },
}));

vi.mock("@api/utils/engagement-limits.js", () => {
  const engagementLimits = {
    defaults: {},
    thresholds: {},
    createEngagementTracker: (limits) => ({
      limits: limits || {},
      stats: {
        replies: 0,
        retweets: 0,
        quotes: 0,
        likes: 0,
        follows: 0,
        bookmarks: 0,
      },
      canPerform: () => true,
      record: () => true,
      getProgress: () => "0/5",
      getStatus: () => ({}),
      getSummary: () => "Summary",
      getUsageRate: () => "25%",
      history: [],
      getRemaining: () => 5,
      getUsage: () => 0,
      getProgressPercent: () => "0%",
      isNearLimit: () => false,
      isExhausted: () => false,
      isAnyExhausted: () => false,
      hasRemainingCapacity: () => true,
    }),
  };

  return {
    engagementLimits,
    default: engagementLimits,
  };
});

vi.mock("@api/core/agent-connector.js", () => {
  const mockConnector = vi.fn().mockImplementation(function () {
    return {};
  });
  return {
    AgentConnector: mockConnector,
    default: mockConnector,
  };
});

const engineMocks = vi.hoisted(() => ({
  AIReplyEngine: class {
    constructor() {
      this.getStats = () => ({});
      this.updateConfig = () => {};
      this.config = { REPLY_PROBABILITY: 0.5 };
    }
  },
  AIQuoteEngine: class {
    constructor() {
      this.getStats = () => ({});
      this.updateConfig = () => {};
      this.config = { QUOTE_PROBABILITY: 0.5 };
    }
  },
  AIContextEngine: class {
    constructor() {}
  },
}));

vi.mock("@api/agent/ai-reply-engine/index.js", () => ({
  AIReplyEngine: engineMocks.AIReplyEngine,
}));

vi.mock("@api/agent/ai-quote-engine.js", () => ({
  AIQuoteEngine: engineMocks.AIQuoteEngine,
}));

vi.mock("@api/agent/ai-context-engine.js", () => ({
  AIContextEngine: engineMocks.AIContextEngine,
}));

vi.mock("@api/behaviors/micro-interactions.js", () => {
  const microInteractions = {
    createMicroInteractionHandler: vi.fn().mockImplementation(() => ({})),
    defaults: {},
  };
  return {
    microInteractions,
    default: microInteractions,
  };
});

vi.mock("@api/behaviors/motor-control.js", () => {
  const motorControl = {
    createMotorController: vi.fn().mockImplementation(() => ({})),
    defaults: {},
  };
  return {
    motorControl,
    default: motorControl,
  };
});

vi.mock("@api/utils/entropyController.js", () => ({
  entropy: {},
}));

vi.mock("@api/twitter/session-phases.js", () => ({
  sessionPhases: {},
}));

vi.mock("@api/utils/sentiment-service.js", () => ({
  sentimentService: {},
}));

vi.mock("@api/utils/math.js", () => ({
  mathUtils: {
    randomInRange: vi.fn().mockReturnValue(100),
    getRandomItem: vi.fn().mockImplementation((arr) => arr[0]),
  },
}));

vi.mock("@api/core/logger.js", () => ({
  createLogger: vi.fn().mockReturnValue({
    info: vi.fn(),
    debug: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
  }),
  createBufferedLogger: vi.fn().mockReturnValue({
    info: vi.fn(),
    debug: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
    shutdown: vi.fn().mockResolvedValue(true),
  }),
}));

// Mock browser-agent/twitterAgent since AITwitterAgent extends it
vi.mock("@api/twitter/twitterAgent.js", () => ({
  TwitterAgent: class {
    constructor(page, profile, logger) {
      this.page = page;
      this.config = profile;
      this.profile = profile;
      this.logger = logger;
      this.state = { consecutiveLoginFailures: 0, replies: 0, quotes: 0 };
      this.sessionStart = Date.now();
      this.loopIndex = 0;
      this.ghost = { click: vi.fn().mockResolvedValue({ success: true }) };
      this.human = {
        sessionStart: vi.fn().mockResolvedValue(),
        sessionEnd: vi.fn().mockResolvedValue(),
        cycleComplete: vi.fn().mockResolvedValue(),
        session: { shouldEndSession: vi.fn().mockReturnValue(false) },
        think: vi.fn().mockResolvedValue(),
      };
      this.navigation = { navigateHome: vi.fn().mockResolvedValue() };
      this.engagement = { diveTweet: vi.fn().mockResolvedValue() };
      this.log = vi.fn().mockImplementation((msg) => {
        if (this.logger && this.logger.info) this.logger.info(msg);
      });
      this.logWarn = vi.fn();
      this.logError = vi.fn();
    }
    navigateHome() {
      return Promise.resolve();
    }
    checkLoginState() {
      return Promise.resolve(true);
    }
    isSessionExpired() {
      return false;
    }
    simulateReading() {
      return Promise.resolve();
    }
    shutdown() {}
  },
}));

// Mock Actions
vi.mock("@api/actions/ai-twitter-reply.js", () => ({
  AIReplyAction: actionMocks.MockAction,
}));
vi.mock("@api/actions/ai-twitter-quote.js", () => ({
  AIQuoteAction: actionMocks.MockAction,
}));
vi.mock("@api/actions/ai-twitter-like.js", () => ({
  LikeAction: actionMocks.MockAction,
}));
vi.mock("@api/actions/ai-twitter-bookmark.js", () => ({
  BookmarkAction: actionMocks.MockAction,
}));
vi.mock("@api/actions/ai-twitter-retweet.js", () => ({
  RetweetAction: actionMocks.MockAction,
}));
vi.mock("@api/actions/ai-twitter-go-home.js", () => ({
  GoHomeAction: actionMocks.MockAction,
}));
vi.mock("@api/actions/ai-twitter-follow.js", () => ({
  FollowAction: actionMocks.MockAction,
}));
vi.mock("@api/actions/advanced-index.js", () => ({
  ActionRunner: actionMocks.MockActionRunner,
}));

vi.mock("@api/behaviors/scroll-helper.js", () => ({
  scrollDown: vi.fn(),
  scrollRandom: vi.fn(),
}));

vi.mock("@api/behaviors/human-interaction.js", () => ({
  HumanInteraction: vi.fn(),
}));

describe("AITwitterAgent Gaps", () => {
  let mockPage;
  let mockBrowser;
  let mockContext;
  let agent;

  beforeEach(() => {
    mockBrowser = {
      isConnected: vi.fn().mockReturnValue(true),
    };
    mockContext = {
      browser: vi.fn().mockReturnValue(mockBrowser),
    };
    mockPage = {
      context: vi.fn().mockReturnValue(mockContext),
      evaluate: vi.fn().mockResolvedValue({
        readyState: "complete",
        title: "X",
        hasBody: true,
      }),
      url: vi.fn().mockReturnValue("https://x.com/home"),
      on: vi.fn(),
      waitForTimeout: vi.fn().mockResolvedValue(true),
      locator: vi.fn().mockReturnValue({
        isVisible: vi.fn().mockResolvedValue(false),
        catch: vi.fn().mockImplementation((fn) => fn()),
      }),
      keyboard: {
        press: vi.fn().mockResolvedValue(true),
      },
      mouse: {
        move: vi.fn().mockResolvedValue(true),
      },
    };

    const mockLogger = {
      info: vi.fn(),
      debug: vi.fn(),
      warn: vi.fn(),
      error: vi.fn(),
    };

    agent = new AITwitterAgent(
      mockPage,
      { name: "test", description: "test" },
      mockLogger,
      {
        engagementLimits: {
          replies: 5,
          retweets: 5,
          quotes: 5,
          likes: 5,
          follows: 5,
          bookmarks: 5,
        },
      },
    );
  });

  afterEach(async () => {
    if (agent) await agent.shutdown();
    vi.restoreAllMocks();
  });

  describe("performHealthCheck", () => {
    it("should return unhealthy if browser is disconnected", async () => {
      mockBrowser.isConnected.mockReturnValue(false);
      const result = await agent.performHealthCheck();
      expect(result.healthy).toBe(false);
      expect(result.reason).toBe("browser_disconnected");
    });

    it("should return unhealthy if page is not ready", async () => {
      mockPage.evaluate.mockResolvedValue({
        readyState: "loading",
        title: "X",
        hasBody: true,
      });
      const result = await agent.performHealthCheck();
      expect(result.healthy).toBe(false);
      expect(result.reason).toBe("page_not_ready");
    });

    it("should navigate home if on unexpected URL", async () => {
      mockPage.url.mockReturnValue("https://random-site.com");
      const navigateHomeSpy = vi
        .spyOn(agent, "navigateHome")
        .mockResolvedValue(true);
      const result = await agent.performHealthCheck();
      expect(result.healthy).toBe(false);
      expect(result.reason).toBe("unexpected_url");
      expect(navigateHomeSpy).toHaveBeenCalled();
    });

    it("should handle evaluation errors gracefully", async () => {
      mockPage.evaluate.mockRejectedValue(new Error("Eval failed"));
      const result = await agent.performHealthCheck();
      expect(result.healthy).toBe(false);
      expect(result.reason).toBe("page_not_ready");
    });

    it("should handle general errors in health check", async () => {
      mockPage.context.mockImplementation(() => {
        throw new Error("Context failed");
      });
      const result = await agent.performHealthCheck();
      expect(result.healthy).toBe(false);
      expect(result.reason).toBe("Context failed");
    });
  });

  describe("Stats and Reporting", () => {
    it("getAIStats should return combined stats", () => {
      const stats = agent.getAIStats();
      expect(stats).toHaveProperty("successRate");
      expect(stats).toHaveProperty("actions");
    });

    it("getEngagementStats should return tracker status", () => {
      const stats = agent.getEngagementStats();
      expect(stats).toHaveProperty("tracker");
      expect(stats).toHaveProperty("summary");
    });

    it("logEngagementStatus should log info for each action", () => {
      agent.engagementLogger = { info: vi.fn() };
      // Mock engagementTracker.getStatus to return some data
      agent.engagementTracker.getStatus = vi.fn().mockReturnValue({
        replies: { remaining: 5, percentage: "50%", current: 5, limit: 10 },
      });
      agent.logEngagementStatus();
      expect(agent.engagementLogger.info).toHaveBeenCalled();
    });
  });

  describe("Cleanup", () => {
    it("flushLogs should shutdown loggers", async () => {
      agent.queueLogger = { shutdown: vi.fn().mockResolvedValue() };
      agent.engagementLogger = { shutdown: vi.fn().mockResolvedValue() };
      await agent.flushLogs();
      expect(agent.queueLogger.shutdown).toHaveBeenCalled();
      expect(agent.engagementLogger.shutdown).toHaveBeenCalled();
    });
  });
});
