/**
 * Unit tests for api/twitter/ai-twitterAgent.js
 * Real scenario tests for AITwitterAgent
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { AITwitterAgent as AITwitterAgentClass } from "@api/twitter/ai-twitterAgent.js";

// Mock dependencies
vi.mock("./twitterAgent.js", () => ({
  TwitterAgent: class MockTwitterAgent {
    constructor(page, initialProfile, logger) {
      this.page = page;
      this.profile = initialProfile;
      this.logger = logger;
      this.state = { likes: 0, retweets: 0, replies: 0, follows: 0 };
      this.sessionStart = Date.now();
      this.isFatigued = false;
    }
  },
}));

vi.mock("@api/tests/unit/agent/ai-reply-engine/index.js", () => ({
  AIReplyEngine: vi.fn().mockImplementation(() => ({
    generateReply: vi.fn(),
  })),
}));

vi.mock("@api/tests/unit/agent/ai-quote-engine.js", () => ({
  AIQuoteEngine: vi.fn().mockImplementation(() => ({
    generateQuote: vi.fn(),
  })),
}));

vi.mock("@api/tests/unit/agent/ai-context-engine.js", () => ({
  AIContextEngine: vi.fn().mockImplementation(() => ({})),
}));

vi.mock("@api/tests/unit/behaviors/micro-interactions.js", () => ({
  microInteractions: { performRandom: vi.fn() },
}));

vi.mock("@api/tests/unit/behaviors/motor-control.js", () => ({
  motorControl: { initialize: vi.fn() },
}));

vi.mock("@api/tests/unit/core/agent-connector.js", () => ({
  default: vi.fn().mockImplementation(() => ({
    sendRequest: vi.fn(),
  })),
}));

vi.mock("@api/tests/unit/utils/math.js", () => ({
  mathUtils: {
    randomInRange: vi.fn((min, max) => (min + max) / 2),
    gaussian: vi.fn((mean) => mean),
    roll: vi.fn(() => false),
  },
}));

vi.mock("@api/tests/unit/utils/entropyController.js", () => ({
  entropy: { getJitter: vi.fn(() => 0) },
}));

vi.mock("@api/tests/unit/utils/engagement-limits.js", () => ({
  engagementLimits: {
    createEngagementTracker: vi.fn(() => ({
      canPerform: vi.fn(() => true),
      record: vi.fn(() => true),
      getProgress: vi.fn(() => "0/5"),
      getStatus: vi.fn(() => ({})),
      getSummary: vi.fn(() => []),
    })),
  },
}));

vi.mock("./session-phases.js", () => ({
  sessionPhases: { getCurrentPhase: vi.fn(() => "active") },
}));

vi.mock("@api/tests/unit/utils/sentiment-service.js", () => ({
  sentimentService: { analyze: vi.fn(() => ({ sentiment: 0.5 })) },
}));

vi.mock("@api/tests/unit/behaviors/scroll-helper.js", () => ({
  scrollDown: vi.fn(),
  scrollRandom: vi.fn(),
}));

vi.mock("@api/tests/unit/utils/async-queue.js", () => ({
  DiveQueue: vi.fn().mockImplementation(() => ({
    canEngage: vi.fn(() => true),
    recordEngagement: vi.fn(() => true),
    getEngagementProgress: vi.fn(() => ({})),
    addDive: vi.fn(),
    clear: vi.fn(),
  })),
}));

vi.mock("@api/tests/unit/actions/ai-twitter-reply.js", () => ({ AIReplyAction: vi.fn() }));
vi.mock("@api/tests/unit/actions/ai-twitter-quote.js", () => ({ AIQuoteAction: vi.fn() }));
vi.mock("@api/tests/unit/actions/ai-twitter-like.js", () => ({ LikeAction: vi.fn() }));
vi.mock("@api/tests/unit/actions/ai-twitter-bookmark.js", () => ({
  BookmarkAction: vi.fn(),
}));
vi.mock("@api/tests/unit/actions/ai-twitter-retweet.js", () => ({ RetweetAction: vi.fn() }));
vi.mock("@api/tests/unit/actions/ai-twitter-go-home.js", () => ({ GoHomeAction: vi.fn() }));
vi.mock("@api/tests/unit/actions/ai-twitter-follow.js", () => ({ FollowAction: vi.fn() }));
vi.mock("@api/tests/unit/actions/advanced-index.js", () => ({
  ActionRunner: vi.fn().mockImplementation(() => ({ run: vi.fn() })),
}));
vi.mock("@api/tests/unit/constants/twitter-timeouts.js", () => ({
  TWITTER_TIMEOUTS: { TWEET_LOAD: 10000, SCROLL: 3000, CLICK: 5000 },
}));
vi.mock("@api/tests/unit/behaviors/human-interaction.js", () => ({
  HumanInteraction: vi.fn().mockImplementation(() => ({})),
}));
vi.mock("@api/tests/unit/core/logger.js", () => ({
  createBufferedLogger: vi.fn(() => ({
    info: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
    debug: vi.fn(),
    flush: vi.fn(),
  })),
}));

const mockApi = {
  getCurrentUrl: vi.fn(() => "https://x.com/home"),
  goto: vi.fn().mockResolvedValue(undefined),
  waitVisible: vi.fn().mockResolvedValue(true),
  waitHidden: vi.fn().mockResolvedValue(true),
  wait: vi.fn().mockResolvedValue(undefined),
  click: vi.fn().mockResolvedValue(undefined),
  type: vi.fn().mockResolvedValue(undefined),
  scroll: { focus: vi.fn().mockResolvedValue(undefined) },
  getPage: vi.fn(() => ({
    keyboard: { press: vi.fn() },
    viewportSize: vi.fn(() => ({ width: 1280, height: 720 })),
  })),
  cursor: { move: vi.fn() },
};

vi.mock("@api/tests/unit/index.js", () => ({
  api: mockApi,
}));

describe("ai-twitterAgent.js - Real Scenario Tests", () => {
  let AITwitterAgent;
  let mockPage;
  let mockLogger;

  beforeEach(() => {
    vi.clearAllMocks();

    mockPage = {
      url: vi.fn(() => "https://x.com/home"),
      locator: vi.fn().mockReturnValue({
        first: vi.fn().mockReturnValue({
          isVisible: vi.fn().mockResolvedValue(false),
        }),
        count: vi.fn().mockResolvedValue(0),
        nth: vi.fn().mockReturnValue({
          isVisible: vi.fn().mockResolvedValue(false),
        }),
      }),
      evaluate: vi.fn().mockResolvedValue(null),
      viewportSize: vi.fn(() => ({ width: 1280, height: 720 })),
      mouse: { wheel: vi.fn() },
      $$: vi.fn().mockReturnValue([]),
    };

    mockLogger = {
      info: vi.fn(),
      warn: vi.fn(),
      error: vi.fn(),
      debug: vi.fn(),
    };

    AITwitterAgent = AITwitterAgentClass;
  });

  describe("page operations", () => {
    it("should create pageOps with url function", () => {
      const agent = new AITwitterAgent(mockPage, {}, mockLogger);

      expect(agent.pageOps).toBeDefined();
      expect(typeof agent.pageOps.url).toBe("function");
    });

    it("should create pageOps with goto function", () => {
      const agent = new AITwitterAgent(mockPage, {}, mockLogger);

      expect(typeof agent.pageOps.goto).toBe("function");
    });

    it("should create pageOps with wait function", () => {
      const agent = new AITwitterAgent(mockPage, {}, mockLogger);

      expect(typeof agent.pageOps.wait).toBe("function");
    });

    it("should create pageOps with click function", () => {
      const agent = new AITwitterAgent(mockPage, {}, mockLogger);

      expect(typeof agent.pageOps.click).toBe("function");
    });

    it("should create pageOps with type function", () => {
      const agent = new AITwitterAgent(mockPage, {}, mockLogger);

      expect(typeof agent.pageOps.type).toBe("function");
    });

    it("should create pageOps with locator function", () => {
      const agent = new AITwitterAgent(mockPage, {}, mockLogger);

      expect(typeof agent.pageOps.locator).toBe("function");
    });

    it("should create pageOps with evaluate function", () => {
      const agent = new AITwitterAgent(mockPage, {}, mockLogger);

      expect(typeof agent.pageOps.evaluate).toBe("function");
    });

    it("should create pageOps with keyboardPress function", () => {
      const agent = new AITwitterAgent(mockPage, {}, mockLogger);

      expect(typeof agent.pageOps.keyboardPress).toBe("function");
    });

    it("should create pageOps with mouseMove function", () => {
      const agent = new AITwitterAgent(mockPage, {}, mockLogger);

      expect(typeof agent.pageOps.mouseMove).toBe("function");
    });

    it("should create pageOps with viewportSize function", () => {
      const agent = new AITwitterAgent(mockPage, {}, mockLogger);

      expect(typeof agent.pageOps.viewportSize).toBe("function");
    });
  });

  describe("inherited TwitterAgent properties", () => {
    it("should have page property", () => {
      const agent = new AITwitterAgent(mockPage, {}, mockLogger);

      expect(agent.page).toBe(mockPage);
    });

    it("should have state property", () => {
      const agent = new AITwitterAgent(mockPage, {}, mockLogger);

      expect(agent.state).toBeDefined();
    });

    it("should have logger property", () => {
      const agent = new AITwitterAgent(mockPage, {}, mockLogger);

      expect(agent.logger).toBe(mockLogger);
    });

    it("should track session start time", () => {
      const agent = new AITwitterAgent(mockPage, {}, mockLogger);

      expect(agent.sessionStart).toBeDefined();
      expect(typeof agent.sessionStart).toBe("number");
    });

    it("should track fatigue state", () => {
      const agent = new AITwitterAgent(mockPage, {}, mockLogger);

      expect(agent.isFatigued).toBe(false);
    });
  });

  describe("engagement limits integration", () => {
    it("should use default engagement limits when not specified", () => {
      const agent = new AITwitterAgent(mockPage, {}, mockLogger);

      expect(agent.engagementTracker).toBeDefined();
    });

    it("should accept custom engagement limits", () => {
      const customLimits = {
        replies: 10,
        retweets: 5,
        quotes: 3,
        likes: 20,
        follows: 8,
        bookmarks: 7,
      };

      const agent = new AITwitterAgent(mockPage, {}, mockLogger, {
        engagementLimits: customLimits,
      });

      expect(agent.engagementTracker).toBeDefined();
    });

    it("should provide engagement progress", () => {
      const agent = new AITwitterAgent(mockPage, {}, mockLogger);

      const progress = agent.engagementTracker.getProgress("like");
      expect(progress).toBeDefined();
    });

    it("should provide engagement status", () => {
      const agent = new AITwitterAgent(mockPage, {}, mockLogger);

      const status = agent.engagementTracker.getStatus();
      expect(status).toBeDefined();
    });

    it("should provide engagement summary", () => {
      const agent = new AITwitterAgent(mockPage, {}, mockLogger);

      const summary = agent.engagementTracker.getSummary();
      // Summary is an object or array depending on implementation
      expect(summary).toBeDefined();
    });

    it("should check if action can be performed", () => {
      const agent = new AITwitterAgent(mockPage, {}, mockLogger);

      const canPerform = agent.engagementTracker.canPerform("like");
      expect(typeof canPerform).toBe("boolean");
    });

    it("should record engagement actions", () => {
      const agent = new AITwitterAgent(mockPage, {}, mockLogger);

      const result = agent.engagementTracker.record("like");
      expect(typeof result).toBe("boolean");
    });
  });

  describe("configuration handling", () => {
    it("should store config with all options", () => {
      const options = {
        config: { customKey: "customValue" },
        replyProbability: 0.7,
        quoteProbability: 0.6,
        maxRetries: 5,
        waitLogInterval: 3000,
        engagementLimits: { likes: 15 },
      };

      const agent = new AITwitterAgent(mockPage, {}, mockLogger, options);

      expect(agent.twitterConfig).toEqual({ customKey: "customValue" });
      expect(agent.waitLogInterval).toBe(3000);
    });

    it("should handle empty options", () => {
      const agent = new AITwitterAgent(mockPage, {}, mockLogger, {});

      expect(agent.twitterConfig).toEqual({});
      expect(agent.waitLogInterval).toBe(10000);
    });

    it("should handle undefined options", () => {
      const agent = new AITwitterAgent(mockPage, {}, mockLogger);

      expect(agent.twitterConfig).toEqual({});
    });
  });

  describe("page state management", () => {
    it("should start in HOME state", () => {
      const agent = new AITwitterAgent(mockPage, {}, mockLogger);

      expect(agent.pageState).toBe("HOME");
    });

    it("should allow state transitions", () => {
      const agent = new AITwitterAgent(mockPage, {}, mockLogger);

      agent.pageState = "DIVING";
      expect(agent.pageState).toBe("DIVING");

      agent.pageState = "RETURNING";
      expect(agent.pageState).toBe("RETURNING");

      agent.pageState = "HOME";
      expect(agent.pageState).toBe("HOME");
    });
  });

  describe("operation locks", () => {
    it("should manage operation lock", () => {
      const agent = new AITwitterAgent(mockPage, {}, mockLogger);

      expect(agent.operationLock).toBe(false);

      agent.operationLock = true;
      expect(agent.operationLock).toBe(true);

      agent.operationLock = false;
      expect(agent.operationLock).toBe(false);
    });

    it("should manage dive lock", () => {
      const agent = new AITwitterAgent(mockPage, {}, mockLogger);

      expect(agent.diveLockAcquired).toBe(false);

      agent.diveLockAcquired = true;
      expect(agent.diveLockAcquired).toBe(true);
    });

    it("should manage posting state", () => {
      const agent = new AITwitterAgent(mockPage, {}, mockLogger);

      expect(agent.isPosting).toBe(false);

      agent.isPosting = true;
      expect(agent.isPosting).toBe(true);
    });

    it("should manage scrolling state", () => {
      const agent = new AITwitterAgent(mockPage, {}, mockLogger);

      expect(agent.scrollingEnabled).toBe(true);

      agent.scrollingEnabled = false;
      expect(agent.scrollingEnabled).toBe(false);
    });
  });

  describe("session management", () => {
    it("should manage session active state", () => {
      const agent = new AITwitterAgent(mockPage, {}, mockLogger);

      expect(agent.sessionActive).toBe(false);

      agent.sessionActive = true;
      expect(agent.sessionActive).toBe(true);
    });

    it("should track session start", () => {
      const before = Date.now();
      const agent = new AITwitterAgent(mockPage, {}, mockLogger);
      const after = Date.now();

      expect(agent.sessionStart).toBeGreaterThanOrEqual(before);
      expect(agent.sessionStart).toBeLessThanOrEqual(after);
    });
  });

  describe("quick mode", () => {
    it("should manage quick mode state", () => {
      const agent = new AITwitterAgent(mockPage, {}, mockLogger);

      expect(agent.quickModeEnabled).toBe(false);

      agent.quickModeEnabled = true;
      expect(agent.quickModeEnabled).toBe(true);
    });
  });

  describe("AI statistics", () => {
    it("should provide initial stats", () => {
      const agent = new AITwitterAgent(mockPage, {}, mockLogger);

      expect(agent.aiStats.attempts).toBe(0);
      expect(agent.aiStats.replies).toBe(0);
      expect(agent.aiStats.skips).toBe(0);
      expect(agent.aiStats.safetyBlocks).toBe(0);
      expect(agent.aiStats.errors).toBe(0);
    });

    it("should allow stats modification", () => {
      const agent = new AITwitterAgent(mockPage, {}, mockLogger);

      agent.aiStats.attempts++;
      agent.aiStats.replies++;

      expect(agent.aiStats.attempts).toBe(1);
      expect(agent.aiStats.replies).toBe(1);
    });
  });

  describe("DiveQueue integration", () => {
    it("should have DiveQueue instance", () => {
      const agent = new AITwitterAgent(mockPage, {}, mockLogger);

      expect(agent.diveQueue).toBeDefined();
    });

    it("should provide DiveQueue methods", () => {
      const agent = new AITwitterAgent(mockPage, {}, mockLogger);

      expect(typeof agent.diveQueue.canEngage).toBe("function");
      expect(typeof agent.diveQueue.recordEngagement).toBe("function");
      expect(typeof agent.diveQueue.getEngagementProgress).toBe("function");
    });
  });

  describe("engine initialization", () => {
    it("should initialize ReplyEngine", () => {
      const agent = new AITwitterAgent(mockPage, {}, mockLogger);

      expect(agent.replyEngine).toBeDefined();
    });

    it("should initialize QuoteEngine", () => {
      const agent = new AITwitterAgent(mockPage, {}, mockLogger);

      expect(agent.quoteEngine).toBeDefined();
    });

    it("should initialize ContextEngine", () => {
      const agent = new AITwitterAgent(mockPage, {}, mockLogger);

      expect(agent.contextEngine).toBeDefined();
    });

    it("should initialize AgentConnector", () => {
      const agent = new AITwitterAgent(mockPage, {}, mockLogger);

      expect(agent.agentConnector).toBeDefined();
    });
  });
});
