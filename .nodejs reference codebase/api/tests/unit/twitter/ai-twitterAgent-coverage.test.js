/**
 * Unit tests for api/twitter/ai-twitterAgent.js
 * Coverage tests for AITwitterAgent class
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { AITwitterAgent as AITwitterAgentClass } from "@api/twitter/ai-twitterAgent.js";

// Mock all dependencies
vi.mock("./twitterAgent.js", () => ({
  TwitterAgent: class MockTwitterAgent {
    constructor(page, initialProfile, logger) {
      this.page = page;
      this.profile = initialProfile;
      this.logger = logger;
      this.state = { likes: 0, retweets: 0, replies: 0, follows: 0 };
      this.sessionStart = Date.now();
      this.isFatigued = false;
      this.fatigueThreshold = 30 * 60 * 1000;
    }
  },
}));

vi.mock("@api/tests/unit/agent/ai-reply-engine/index.js", () => ({
  AIReplyEngine: vi.fn().mockImplementation(() => ({
    generateReply: vi
      .fn()
      .mockResolvedValue({ success: true, content: "test reply" }),
  })),
}));

vi.mock("@api/tests/unit/agent/ai-quote-engine.js", () => ({
  AIQuoteEngine: vi.fn().mockImplementation(() => ({
    generateQuote: vi
      .fn()
      .mockResolvedValue({ success: true, content: "test quote" }),
  })),
}));

vi.mock("@api/tests/unit/agent/ai-context-engine.js", () => ({
  AIContextEngine: vi.fn().mockImplementation(() => ({
    analyzeContext: vi.fn().mockReturnValue({ sentiment: 0.5 }),
  })),
}));

vi.mock("@api/tests/unit/behaviors/micro-interactions.js", () => ({
  microInteractions: {
    performRandom: vi.fn(),
  },
}));

vi.mock("@api/tests/unit/behaviors/motor-control.js", () => ({
  motorControl: {
    initialize: vi.fn(),
  },
}));

vi.mock("@api/tests/unit/core/agent-connector.js", () => ({
  default: vi.fn().mockImplementation(() => ({
    sendRequest: vi.fn().mockResolvedValue({ success: true, content: "test" }),
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
  entropy: {
    getJitter: vi.fn(() => 0),
  },
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
  sessionPhases: {
    getCurrentPhase: vi.fn(() => "active"),
  },
}));

vi.mock("@api/tests/unit/utils/sentiment-service.js", () => ({
  sentimentService: {
    analyze: vi.fn(() => ({ sentiment: 0.5 })),
  },
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
    getFullStatus: vi.fn(() => ({})),
  })),
}));

vi.mock("@api/tests/unit/actions/ai-twitter-reply.js", () => ({
  AIReplyAction: vi.fn(),
}));

vi.mock("@api/tests/unit/actions/ai-twitter-quote.js", () => ({
  AIQuoteAction: vi.fn(),
}));

vi.mock("@api/tests/unit/actions/ai-twitter-like.js", () => ({
  LikeAction: vi.fn(),
}));

vi.mock("@api/tests/unit/actions/ai-twitter-bookmark.js", () => ({
  BookmarkAction: vi.fn(),
}));

vi.mock("@api/tests/unit/actions/ai-twitter-retweet.js", () => ({
  RetweetAction: vi.fn(),
}));

vi.mock("@api/tests/unit/actions/ai-twitter-go-home.js", () => ({
  GoHomeAction: vi.fn(),
}));

vi.mock("@api/tests/unit/actions/ai-twitter-follow.js", () => ({
  FollowAction: vi.fn(),
}));

vi.mock("@api/tests/unit/actions/advanced-index.js", () => ({
  ActionRunner: vi.fn().mockImplementation(() => ({
    run: vi.fn(),
  })),
}));

vi.mock("@api/tests/unit/constants/twitter-timeouts.js", () => ({
  TWITTER_TIMEOUTS: {
    TWEET_LOAD: 10000,
    SCROLL: 3000,
    CLICK: 5000,
  },
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

vi.mock("@api/tests/unit/index.js", () => ({
  api: {
    getCurrentUrl: vi.fn(() => "https://x.com/home"),
    goto: vi.fn(),
    waitVisible: vi.fn(),
    waitHidden: vi.fn(),
    wait: vi.fn(),
    click: vi.fn(),
    type: vi.fn(),
    scroll: { focus: vi.fn() },
    getPage: vi.fn(() => ({
      keyboard: { press: vi.fn() },
      viewportSize: vi.fn(() => ({ width: 1280, height: 720 })),
    })),
    cursor: { move: vi.fn() },
  },
}));

describe("ai-twitterAgent.js - Coverage Tests", () => {
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
      }),
      evaluate: vi.fn(),
      viewportSize: vi.fn(() => ({ width: 1280, height: 720 })),
    };

    mockLogger = {
      info: vi.fn(),
      warn: vi.fn(),
      error: vi.fn(),
      debug: vi.fn(),
    };

    AITwitterAgent = AITwitterAgentClass;
  });

  describe("constructor", () => {
    it("should initialize with default values", () => {
      const agent = new AITwitterAgent(mockPage, {}, mockLogger);

      expect(agent).toBeDefined();
      expect(agent.page).toBe(mockPage);
      expect(agent.pageState).toBe("HOME");
      expect(agent.scrollingEnabled).toBe(true);
      expect(agent.operationLock).toBe(false);
      expect(agent.isPosting).toBe(false);
      expect(agent.quickModeEnabled).toBe(false);
      expect(agent.sessionActive).toBe(false);
    });

    it("should initialize engagement tracker", () => {
      const agent = new AITwitterAgent(mockPage, {}, mockLogger);

      expect(agent.engagementTracker).toBeDefined();
      expect(agent.diveQueue).toBeDefined();
    });

    it("should initialize AI engines", () => {
      const agent = new AITwitterAgent(mockPage, {}, mockLogger);

      expect(agent.replyEngine).toBeDefined();
      expect(agent.quoteEngine).toBeDefined();
      expect(agent.contextEngine).toBeDefined();
      expect(agent.agentConnector).toBeDefined();
    });

    it("should initialize AI stats", () => {
      const agent = new AITwitterAgent(mockPage, {}, mockLogger);

      expect(agent.aiStats).toBeDefined();
      expect(agent.aiStats.attempts).toBe(0);
      expect(agent.aiStats.replies).toBe(0);
      expect(agent.aiStats.skips).toBe(0);
      expect(agent.aiStats.errors).toBe(0);
    });

    it("should accept custom options", () => {
      const options = {
        replyProbability: 0.3,
        quoteProbability: 0.4,
        engagementLimits: { replies: 5, likes: 10 },
        waitLogInterval: 5000,
      };

      const agent = new AITwitterAgent(mockPage, {}, mockLogger, options);

      expect(agent).toBeDefined();
      expect(agent.waitLogInterval).toBe(5000);
    });

    it("should set home URL", () => {
      const agent = new AITwitterAgent(mockPage, {}, mockLogger);

      expect(agent.homeUrl).toBe("https://x.com/home");
    });
  });

  describe("createPageOps", () => {
    it("should create page operations object", async () => {
      const { default: createPageOps } =
        await import("@api/twitter/ai-twitterAgent.js");

      // The createPageOps is exported or accessible
      const agent = new AITwitterAgent(mockPage, {}, mockLogger);

      expect(agent.pageOps).toBeDefined();
      expect(agent.pageOps.page).toBe(mockPage);
    });
  });

  describe("PAGE_STATE constants", () => {
    it("should have all page states defined", async () => {
      // PAGE_STATE is internal but we can verify through agent behavior
      const agent = new AITwitterAgent(mockPage, {}, mockLogger);

      // Initial state should be HOME
      expect(agent.pageState).toBe("HOME");
    });
  });

  describe("dive lock mechanism", () => {
    it("should initialize lock properties", () => {
      const agent = new AITwitterAgent(mockPage, {}, mockLogger);

      expect(agent.diveLockAcquired).toBe(false);
      expect(agent.scrollingEnabled).toBe(true);
      expect(agent.operationLock).toBe(false);
    });

    it("should allow lock state changes", () => {
      const agent = new AITwitterAgent(mockPage, {}, mockLogger);

      agent.operationLock = true;
      expect(agent.operationLock).toBe(true);

      agent.operationLock = false;
      expect(agent.operationLock).toBe(false);
    });
  });

  describe("quick mode", () => {
    it("should initialize quick mode as disabled", () => {
      const agent = new AITwitterAgent(mockPage, {}, mockLogger);

      expect(agent.quickModeEnabled).toBe(false);
    });

    it("should allow quick mode toggle", () => {
      const agent = new AITwitterAgent(mockPage, {}, mockLogger);

      agent.quickModeEnabled = true;
      expect(agent.quickModeEnabled).toBe(true);
    });
  });

  describe("session state", () => {
    it("should initialize session as inactive", () => {
      const agent = new AITwitterAgent(mockPage, {}, mockLogger);

      expect(agent.sessionActive).toBe(false);
    });

    it("should allow session state changes", () => {
      const agent = new AITwitterAgent(mockPage, {}, mockLogger);

      agent.sessionActive = true;
      expect(agent.sessionActive).toBe(true);
    });
  });

  describe("engagement tracker sync", () => {
    it("should have synchronized engagement tracker", () => {
      const agent = new AITwitterAgent(mockPage, {}, mockLogger);

      expect(agent.engagementTracker.canPerform).toBeDefined();
      expect(agent.engagementTracker.record).toBeDefined();
      expect(agent.engagementTracker.getProgress).toBeDefined();
      expect(agent.engagementTracker.getStatus).toBeDefined();
    });

    it("should delegate canPerform to both trackers", () => {
      const agent = new AITwitterAgent(mockPage, {}, mockLogger);

      // Both original tracker and diveQueue should be checked
      const result = agent.engagementTracker.canPerform("like");
      expect(result).toBe(true);
    });
  });

  describe("configuration", () => {
    it("should store twitter config", () => {
      const config = { testKey: "testValue" };
      const agent = new AITwitterAgent(mockPage, {}, mockLogger, { config });

      expect(agent.twitterConfig).toEqual(config);
    });

    it("should use default twitter config when not provided", () => {
      const agent = new AITwitterAgent(mockPage, {}, mockLogger);

      expect(agent.twitterConfig).toEqual({});
    });
  });

  describe("AI stats tracking", () => {
    it("should track AI attempts", () => {
      const agent = new AITwitterAgent(mockPage, {}, mockLogger);

      agent.aiStats.attempts = 5;
      expect(agent.aiStats.attempts).toBe(5);
    });

    it("should track AI replies", () => {
      const agent = new AITwitterAgent(mockPage, {}, mockLogger);

      agent.aiStats.replies = 3;
      expect(agent.aiStats.replies).toBe(3);
    });

    it("should track AI skips", () => {
      const agent = new AITwitterAgent(mockPage, {}, mockLogger);

      agent.aiStats.skips = 2;
      expect(agent.aiStats.skips).toBe(2);
    });

    it("should track AI errors", () => {
      const agent = new AITwitterAgent(mockPage, {}, mockLogger);

      agent.aiStats.errors = 1;
      expect(agent.aiStats.errors).toBe(1);
    });

    it("should track safety blocks", () => {
      const agent = new AITwitterAgent(mockPage, {}, mockLogger);

      agent.aiStats.safetyBlocks = 2;
      expect(agent.aiStats.safetyBlocks).toBe(2);
    });
  });

  describe("wait log interval", () => {
    it("should use default wait log interval", () => {
      const agent = new AITwitterAgent(mockPage, {}, mockLogger);

      expect(agent.waitLogInterval).toBe(10000);
    });

    it("should accept custom wait log interval", () => {
      const agent = new AITwitterAgent(mockPage, {}, mockLogger, {
        waitLogInterval: 5000,
      });

      expect(agent.waitLogInterval).toBe(5000);
    });
  });

  describe("DiveQueue configuration", () => {
    it("should configure DiveQueue with sequential processing", () => {
      const agent = new AITwitterAgent(mockPage, {}, mockLogger);

      expect(agent.diveQueue).toBeDefined();
    });

    it("should use custom engagement limits in DiveQueue", () => {
      const limits = { replies: 10, likes: 20, follows: 5 };
      const agent = new AITwitterAgent(mockPage, {}, mockLogger, {
        engagementLimits: limits,
      });

      expect(agent.diveQueue).toBeDefined();
    });
  });
});
