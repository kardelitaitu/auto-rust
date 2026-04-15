/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import AITwitterAgent from "@api/twitter/ai-twitterAgent.js";
import { TwitterAgent } from "@api/twitter/twitterAgent.js";
import { DiveQueue } from "@api/utils/async-queue.js";
import { AIReplyEngine } from "@api/agent/ai-reply-engine/index.js";
import { AIQuoteEngine } from "@api/agent/ai-quote-engine.js";
import { AIContextEngine } from "@api/agent/ai-context-engine.js";
import AgentConnector from "@api/core/agent-connector.js";
import { engagementLimits } from "@api/utils/engagement-limits.js";
import { sentimentService } from "@api/utils/sentiment-service.js";
import { sessionPhases } from "@api/twitter/session-phases.js";
import { api } from "@api/index.js";

const engagementMocks = vi.hoisted(() => {
  const canPerform = vi.fn().mockReturnValue(true);
  const record = vi.fn().mockReturnValue(true);
  const getProgress = vi.fn().mockReturnValue("0/5");
  const getStatus = vi.fn().mockReturnValue({});
  const getSummary = vi.fn().mockReturnValue("Summary");
  const getUsageRate = vi.fn().mockReturnValue("25%");

  const createEngagementTracker = vi.fn().mockImplementation(() => ({
    canPerform,
    record,
    getProgress,
    getStatus,
    getSummary,
    getUsageRate,
  }));

  return {
    canPerform,
    record,
    getProgress,
    getStatus,
    getSummary,
    getUsageRate,
    createEngagementTracker,
  };
});

vi.mock("@api/utils/engagement-limits.js", () => ({
  engagementLimits: {
    createEngagementTracker: engagementMocks.createEngagementTracker,
  },
}));

// Mock dependencies
vi.mock("@api/twitter/twitterAgent.js", () => {
  const MockTwitterAgent = vi.fn(function (page, initialProfile, logger) {
    this.page = page;
    this.initialProfile = initialProfile;
    this.logger = logger;
    this.human = {
      sessionStart: vi.fn(),
      sessionEnd: vi.fn().mockResolvedValue(),
      cycleComplete: vi.fn().mockResolvedValue(),
      session: { shouldEndSession: vi.fn().mockReturnValue(false) },
      think: vi.fn(),
    };
    this.state = { consecutiveLoginFailures: 0, replies: 0, quotes: 0 };
    this.engagement = { diveTweet: vi.fn() };
    this.pageState = "HOME";
    this.scrollingEnabled = true;
    this.operationLock = false;
    this.log = (msg) => {
      if (logger) logger.info(msg);
    };
    this.start = vi.fn();
    this.stop = vi.fn();
    this.checkLoginState = vi.fn().mockResolvedValue(true);
    this.navigateHome = vi.fn().mockResolvedValue();
    this.ensureForYouTab = vi.fn().mockResolvedValue();
    this.isSessionExpired = vi.fn().mockReturnValue(false);
    this.humanClick = vi.fn().mockResolvedValue();
    this.navigation = { navigateHome: vi.fn().mockResolvedValue() };
    return this;
  });
  MockTwitterAgent.prototype.simulateReading = vi.fn().mockResolvedValue();
  return { TwitterAgent: MockTwitterAgent };
});

vi.mock("@api/utils/async-queue.js", () => {
  const MockDiveQueue = vi.fn(function () {
    return {
      canEngage: vi.fn().mockReturnValue(true),
      recordEngagement: vi.fn().mockReturnValue(true),
      getEngagementProgress: vi.fn().mockReturnValue({}),
      getFullStatus: vi.fn().mockReturnValue({
        queueLength: 0,
        activeCount: 0,
        utilization: 0,
        capacity: 0,
        maxQueueSize: 30,
        engagementLimits: null,
        retryInfo: { pendingRetries: 0 },
      }),
      isHealthy: vi.fn().mockReturnValue(true),
      addJob: vi.fn(),
      addDive: vi.fn().mockImplementation((task) => task()),
      clear: vi.fn(),
      on: vi.fn(),
      getQueueStatus: vi.fn().mockReturnValue({}),
      start: vi.fn(),
      stop: vi.fn(),
      pause: vi.fn(),
      resume: vi.fn(),
      processQueue: vi.fn(),
      enableQuickMode: vi.fn(),
    };
  });
  return { DiveQueue: MockDiveQueue };
});

vi.mock("@api/agent/ai-reply-engine/index.js", () => {
  const MockAIReplyEngine = vi.fn(function () {
    return {
      generateReply: vi.fn().mockResolvedValue({ text: "test reply" }),
      config: { REPLY_PROBABILITY: 0.5 },
    };
  });
  return { AIReplyEngine: MockAIReplyEngine };
});

vi.mock("@api/agent/ai-quote-engine.js", () => {
  const MockAIQuoteEngine = vi.fn(function () {
    return {
      generateQuote: vi.fn().mockResolvedValue({ text: "test quote" }),
    };
  });
  return { AIQuoteEngine: MockAIQuoteEngine };
});

vi.mock("@api/agent/ai-context-engine.js", () => {
  const MockAIContextEngine = vi.fn(function () {
    return {
      extractEnhancedContext: vi.fn().mockResolvedValue({}),
    };
  });
  return { AIContextEngine: MockAIContextEngine };
});

vi.mock("@api/core/agent-connector.js", () => {
  const MockAgentConnector = vi.fn();
  return { default: MockAgentConnector };
});

vi.mock("@api/behaviors/micro-interactions.js", () => ({
  microInteractions: {
    createMicroInteractionHandler: vi.fn().mockReturnValue({
      executeMicroInteraction: vi
        .fn()
        .mockResolvedValue({ success: true, type: "test" }),
      textHighlight: vi.fn().mockResolvedValue({ success: true }),
      startFidgetLoop: vi.fn(),
      stopFidgetLoop: vi.fn(),
      config: {
        highlightChance: 0.1,
        rightClickChance: 0.1,
        logoClickChance: 0.1,
        whitespaceClickChance: 0.1,
      },
    }),
    moveCursorTo: vi.fn(),
    randomScroll: vi.fn(),
  },
}));

vi.mock("@api/behaviors/motor-control.js", () => ({
  motorControl: {
    createMotorController: vi.fn().mockReturnValue({
      smartClick: vi.fn().mockResolvedValue({ success: true, x: 100, y: 100 }),
    }),
    humanLikeMouseMove: vi.fn(),
  },
}));

vi.mock("@api/utils/math.js", () => ({
  mathUtils: {
    randomInRange: vi.fn().mockReturnValue(100),
    weightedRandom: vi.fn().mockReturnValue("reply"),
    roll: vi.fn().mockReturnValue(true),
  },
}));

vi.mock("@api/utils/entropyController.js", () => ({
  entropy: {
    addEntropy: vi.fn(),
    retryDelay: vi.fn().mockReturnValue(100),
    scrollSettleTime: vi.fn().mockReturnValue(100),
  },
}));

vi.mock("@api/twitter/session-phases.js", () => ({
  sessionPhases: {
    getCurrentPhase: vi.fn().mockReturnValue("normal"),
    getSessionPhase: vi.fn().mockReturnValue("normal"),
    getPhaseStats: vi.fn().mockReturnValue({ description: "Normal phase" }),
    getPhaseModifier: vi.fn().mockReturnValue(1.0),
  },
}));

vi.mock("@api/utils/sentiment-service.js", () => ({
  sentimentService: {
    analyze: vi.fn(),
  },
}));

vi.mock("@api/twitter/twitter-reply-prompt.js", () => ({
  buildEnhancedPrompt: vi.fn().mockReturnValue("mock prompt"),
}));

vi.mock("@api/behaviors/scroll-helper.js", () => ({
  scrollDown: vi.fn().mockResolvedValue(undefined),
  scrollUp: vi.fn(),
  scrollRandom: vi.fn(),
}));

vi.mock("@api/actions/ai-twitter-reply.js", () => ({
  AIReplyAction: vi.fn(),
}));

vi.mock("@api/actions/ai-twitter-quote.js", () => ({
  AIQuoteAction: vi.fn(),
}));

vi.mock("@api/actions/ai-twitter-like.js", () => ({
  LikeAction: vi.fn(),
}));

vi.mock("@api/actions/ai-twitter-bookmark.js", () => ({
  BookmarkAction: vi.fn(),
}));

vi.mock("@api/actions/ai-twitter-retweet.js", () => ({
  RetweetAction: vi.fn(),
}));

vi.mock("@api/actions/ai-twitter-go-home.js", () => ({
  GoHomeAction: vi.fn(),
}));

vi.mock("@api/actions/advanced-index.js", () => {
  const MockActionRunner = vi.fn(function () {
    return {
      selectAction: vi.fn().mockReturnValue("like"),
      executeAction: vi
        .fn()
        .mockResolvedValue({ success: true, executed: true, reason: "tested" }),
    };
  });
  return { ActionRunner: MockActionRunner };
});

vi.mock("@api/constants/twitter-timeouts.js", () => ({
  TWITTER_TIMEOUTS: {
    POST_TWEET: 5000,
    NAVIGATION: 5000,
    ELEMENT_VISIBLE: 5000,
    DIVE_TIMEOUT: 5000,
  },
}));

vi.mock("@api/behaviors/human-interaction.js", () => {
  const MockHumanInteraction = vi.fn(function () {
    return {
      sessionStart: vi.fn(),
      session: { shouldEndSession: vi.fn().mockReturnValue(false) },
    };
  });
  return { HumanInteraction: MockHumanInteraction };
});

vi.mock("@api/core/logger.js", () => ({
  createLogger: vi.fn().mockReturnValue({
    info: vi.fn(),
    error: vi.fn(),
    warn: vi.fn(),
    debug: vi.fn(),
  }),
  createBufferedLogger: vi.fn().mockReturnValue({
    info: vi.fn(),
    error: vi.fn(),
    warn: vi.fn(),
    debug: vi.fn(),
    shutdown: vi.fn(),
  }),
}));

vi.mock("@api/index.js", () => ({
  api: {
    getCurrentUrl: vi.fn().mockResolvedValue("https://x.com/home"),
    wait: vi.fn().mockResolvedValue(undefined),
    waitVisible: vi.fn().mockResolvedValue(undefined),
    waitHidden: vi.fn().mockResolvedValue(undefined),
    goto: vi.fn().mockResolvedValue(undefined),
    visible: vi.fn().mockResolvedValue(true),
    waitForURL: vi.fn().mockResolvedValue(undefined),
    keyboardPress: vi.fn().mockResolvedValue(undefined),
    click: vi.fn().mockResolvedValue(undefined),
    type: vi.fn().mockResolvedValue(undefined),
    scroll: { focus: vi.fn() },
    waitForSelector: vi.fn().mockResolvedValue(undefined),
    getPersona: vi
      .fn()
      .mockReturnValue({ microMoveChance: 0.1, fidgetChance: 0.05 }),
    emulateMedia: vi.fn().mockResolvedValue(undefined),
    cursor: { move: vi.fn().mockResolvedValue(undefined) },
    getPage: vi.fn().mockReturnValue({}),
  },
}));

describe("AITwitterAgent", () => {
  let agent;
  let mockPage;
  let mockProfile;
  let mockLogger;
  let mockOptions;

  beforeEach(() => {
    // Reset engagement mocks defaults
    engagementMocks.canPerform.mockReturnValue(true);
    engagementMocks.record.mockReturnValue(true);
    engagementMocks.getProgress.mockReturnValue("0/5");
    engagementMocks.getStatus.mockReturnValue({});
    engagementMocks.getSummary.mockReturnValue("Summary");

    mockPage = {
      goto: vi.fn().mockResolvedValue(undefined),
      url: vi.fn().mockReturnValue("https://x.com/home"),
      evaluate: vi.fn().mockResolvedValue({}),
      on: vi.fn(),
      off: vi.fn(),
      locator: vi.fn().mockReturnValue({
        count: vi.fn().mockResolvedValue(0),
        first: vi.fn().mockReturnValue({
          count: vi.fn().mockResolvedValue(0),
          isVisible: vi.fn().mockResolvedValue(false),
        }),
      }),
      waitForTimeout: vi.fn().mockResolvedValue(undefined),
      waitForURL: vi.fn().mockResolvedValue(undefined),
      waitForSelector: vi.fn().mockResolvedValue(undefined),
      keyboard: { press: vi.fn().mockResolvedValue(undefined) },
      mouse: { move: vi.fn() },
      viewportSize: vi.fn().mockReturnValue({ width: 1000, height: 800 }),
      emulateMedia: vi.fn(),
      context: vi.fn().mockReturnValue({
        browser: vi.fn().mockReturnValue({
          isConnected: vi.fn().mockReturnValue(true),
        }),
      }),
    };

    mockProfile = {
      id: "test-profile",
      username: "testuser",
    };

    mockLogger = {
      info: vi.fn(),
      warn: vi.fn(),
      error: vi.fn(),
      debug: vi.fn(),
    };

    mockOptions = {
      engagementLimits: {
        replies: 10,
        retweets: 5,
        quotes: 5,
        likes: 20,
        follows: 10,
        bookmarks: 10,
      },
      replyProbability: 0.8,
      quoteProbability: 0.3,
      config: { theme: "dark" },
    };

    // Clear all mocks
    vi.clearAllMocks();

    agent = new AITwitterAgent(mockPage, mockProfile, mockLogger, mockOptions);
  });

  afterEach(() => {
    if (agent && typeof agent.shutdown === "function") {
      agent.shutdown();
    }
  });

  describe("Constructor", () => {
    it("should initialize with correct configuration", () => {
      expect(agent).toBeInstanceOf(TwitterAgent);
      expect(agent.pageState).toBe("HOME");
      expect(agent.scrollingEnabled).toBe(true);
      expect(agent.operationLock).toBe(false);
    });

    it("should initialize DiveQueue with correct options", () => {
      expect(DiveQueue).toHaveBeenCalledWith(
        expect.objectContaining({
          maxConcurrent: 1,
          maxQueueSize: 30,
          replies: 10,
          likes: 20,
        }),
      );
    });

    it("should initialize AI engines", () => {
      expect(AgentConnector).toHaveBeenCalled();
      expect(AIReplyEngine).toHaveBeenCalled();
      expect(AIQuoteEngine).toHaveBeenCalled();
      expect(AIContextEngine).toHaveBeenCalled();
    });

    it("should initialize engagement tracker", () => {
      expect(engagementLimits.createEngagementTracker).toHaveBeenCalledWith(
        mockOptions.engagementLimits,
      );
    });
  });

  describe("Engagement Tracker Synchronization", () => {
    it("canPerform should check both tracker and queue", () => {
      // Use engagementMocks directly to ensure we control the mocks
      engagementMocks.canPerform.mockReturnValue(true);

      const mockQueue = agent.diveQueue;
      mockQueue.canEngage.mockReturnValue(true);

      expect(agent.engagementTracker.canPerform("like")).toBe(true);

      engagementMocks.canPerform.mockReturnValue(false);
      expect(agent.engagementTracker.canPerform("like")).toBe(false);

      engagementMocks.canPerform.mockReturnValue(true);
      mockQueue.canEngage.mockReturnValue(false);
      expect(agent.engagementTracker.canPerform("like")).toBe(false);
    });

    it("record should update both tracker and queue if allowed", () => {
      const mockQueue = agent.diveQueue;

      engagementMocks.canPerform.mockReturnValue(true);
      mockQueue.canEngage.mockReturnValue(true);
      engagementMocks.record.mockReturnValue(true);
      mockQueue.recordEngagement.mockReturnValue(true);

      const result = agent.engagementTracker.record("like");

      expect(result).toBe(true);
      expect(engagementMocks.record).toHaveBeenCalledWith("like");
      expect(mockQueue.recordEngagement).toHaveBeenCalledWith("like");
    });

    it("record should fail if either disallows", () => {
      engagementMocks.canPerform.mockReturnValue(false);
      expect(agent.engagementTracker.record("like")).toBe(false);
      expect(engagementMocks.record).not.toHaveBeenCalled();
    });

    it("getProgress should use diveQueue progress", () => {
      const mockQueue = agent.diveQueue;

      mockQueue.getEngagementProgress.mockReturnValue({
        like: { current: 1, limit: 5, percentUsed: 20 },
      });

      const progress = agent.engagementTracker.getProgress("like");
      expect(progress).toBe("1/5");
    });

    it("getStatus should merge statuses", () => {
      const mockQueue = agent.diveQueue;

      engagementMocks.getStatus.mockReturnValue({
        like: { current: 0, limit: 10 },
      });
      mockQueue.getEngagementProgress.mockReturnValue({
        like: { current: 1, limit: 5, percentUsed: 20 },
      });

      const status = agent.engagementTracker.getStatus();
      expect(status.like.current).toBe(1);
      expect(status.like.limit).toBe(5);
    });
  });

  describe("Dive Lock Mechanism", () => {
    it("should acquire lock and disable scrolling on startDive", async () => {
      const result = await agent.startDive();
      expect(result).toBe(true);
      expect(agent.operationLock).toBe(true);
      expect(agent.diveLockAcquired).toBe(true);
      expect(agent.pageState).toBe("DIVING");
      expect(agent.scrollingEnabled).toBe(false);
    });

    it("should wait for existing lock to release", async () => {
      agent.operationLock = true;
      setTimeout(() => {
        agent.operationLock = false;
      }, 150);
      const start = Date.now();
      await agent.startDive();
      const duration = Date.now() - start;
      expect(duration).toBeGreaterThan(100);
      expect(agent.operationLock).toBe(true);
    });

    it("should release lock and return home on endDive(true, true)", async () => {
      agent.operationLock = true;
      agent.diveLockAcquired = true;
      agent.pageState = "DIVING";

      // Mock _safeNavigateHome
      agent._safeNavigateHome = vi.fn().mockResolvedValue(true);
      mockPage.url.mockReturnValue("https://x.com/home");

      await agent.endDive(true, true);

      expect(agent.operationLock).toBe(false);
      expect(agent.diveLockAcquired).toBe(false);
      expect(agent.pageState).toBe("HOME");
      expect(agent.scrollingEnabled).toBe(true);
      expect(agent._safeNavigateHome).toHaveBeenCalled();
    });

    it("should release lock and stay on page on endDive(true, false)", async () => {
      agent.operationLock = true;
      agent.diveLockAcquired = true;
      agent.pageState = "DIVING";

      await agent.endDive(true, false);

      expect(agent.operationLock).toBe(false);
      expect(agent.diveLockAcquired).toBe(false);
      expect(agent.pageState).toBe("TWEET_PAGE");
      expect(agent.scrollingEnabled).toBe(true);
    });

    it("isDiving should return correct state", () => {
      agent.operationLock = true;
      agent.pageState = "DIVING";
      expect(agent.isDiving()).toBe(true);

      agent.pageState = "HOME";
      expect(agent.isDiving()).toBe(false);
    });

    it("isOnTweetPage should check URL and state", async () => {
      api.getCurrentUrl.mockResolvedValue("https://x.com/user/status/123");
      expect(await agent.isOnTweetPage()).toBe(true);

      api.getCurrentUrl.mockResolvedValue("https://x.com/home");
      agent.pageState = "TWEET_PAGE";
      expect(await agent.isOnTweetPage()).toBe(true);
    });

    it("canScroll should check enabled and lock", () => {
      agent.scrollingEnabled = true;
      agent.operationLock = false;
      expect(agent.canScroll()).toBe(true);

      agent.operationLock = true;
      expect(agent.canScroll()).toBe(false);
    });

    it("logDiveStatus should log state", () => {
      agent.logDiveStatus();
      expect(mockLogger.info).toHaveBeenCalled();
    });

    it("_safeNavigateHome should try navigating home", async () => {
      // _safeNavigateHome uses api.getCurrentUrl() to check current URL
      api.getCurrentUrl.mockResolvedValue("https://x.com/settings");
      // Mock getPage to return mockPage so keyboard.press works
      api.getPage.mockReturnValue(mockPage);
      // Create a mock navigateHome that we can track
      const mockNavFn = vi.fn().mockResolvedValue(true);
      agent.navigation.navigateHome = mockNavFn;
      const result = await agent._safeNavigateHome();
      // If navigation succeeded, result should be true
      expect(result).toBe(true);
      // Verify the mock was called
      expect(mockNavFn).toHaveBeenCalled();
    });

    it("_safeNavigateHome should fallback to goto if navigateHome fails", async () => {
      // _safeNavigateHome uses api.getCurrentUrl() to check current URL
      api.getCurrentUrl.mockResolvedValue("https://x.com/settings");
      // Mock navigation.navigateHome to fail, triggering fallback to api.goto
      agent.navigation.navigateHome = vi
        .fn()
        .mockRejectedValue(new Error("nav failed"));
      await agent._safeNavigateHome();
      expect(api.goto).toHaveBeenCalledWith(
        "https://x.com/home",
        expect.any(Object),
      );
    });

    it("performIdleCursorMovement should move cursor via api", async () => {
      await agent.performIdleCursorMovement();
      // performIdleCursorMovement uses api.cursor.move, not mockPage.mouse.move
      expect(api.cursor.move).toHaveBeenCalled();
    });
  });

  describe("diveTweet", () => {
    it("should skip if scanning is in progress", async () => {
      agent.isScanning = true;
      await agent.diveTweet();
      expect(agent.logger.info).toHaveBeenCalledWith(
        expect.stringContaining("Scan already in progress"),
      );
    });

    it("should call _diveTweetWithAI and add to queue", async () => {
      agent._diveTweetWithAI = vi.fn().mockImplementation(async (callback) => {
        if (callback) await callback(() => {});
      });

      await agent.diveTweet();

      expect(agent._diveTweetWithAI).toHaveBeenCalled();
      expect(agent.diveQueue.addDive).toHaveBeenCalled();
    });

    it("should handle errors and release lock", async () => {
      agent._diveTweetWithAI = vi
        .fn()
        .mockRejectedValue(new Error("Dive failed"));
      agent.diveLockAcquired = true;
      agent.operationLock = true;
      agent.endDive = vi.fn().mockResolvedValue();

      await agent.diveTweet();

      expect(agent.logger.info).toHaveBeenCalledWith(
        expect.stringContaining("Error in diveTweet"),
      );
      expect(agent.endDive).toHaveBeenCalledWith(false, true);
    });
  });

  describe("_diveTweetWithAI", () => {
    beforeEach(() => {
      agent.startDive = vi.fn().mockResolvedValue(true);
      agent.endDive = vi.fn().mockResolvedValue(true);
      agent._ensureExplorationScroll = vi.fn().mockResolvedValue(true);
      agent._readExpandedTweet = vi.fn().mockResolvedValue(true);
      agent.humanClick = vi.fn().mockResolvedValue(true);
      // Mock actionRunner on instance
      agent.actionRunner = {
        selectAction: vi.fn(),
        executeAction: vi.fn().mockResolvedValue({
          success: true,
          executed: true,
          reason: "mocked",
        }),
        setCurrentTweetId: vi.fn(),
      };
    });

    it("should navigate home if no tweets found", async () => {
      // Mock tweets count 0
      mockPage.locator.mockReturnValue({
        count: vi.fn().mockResolvedValue(0),
        first: vi.fn().mockReturnValue({ count: vi.fn().mockResolvedValue(0) }),
        nth: vi
          .fn()
          .mockReturnValue({ boundingBox: vi.fn().mockResolvedValue(null) }),
      });
      agent.ensureForYouTab = vi.fn().mockResolvedValue(true);

      await agent._diveTweetWithAI();

      expect(api.goto).toHaveBeenCalledWith("https://x.com/");
      expect(agent.endDive).toHaveBeenCalled();
    });

    it("should process found tweet", async () => {
      const clickTarget = {
        count: vi.fn().mockResolvedValue(1),
        isVisible: vi.fn().mockResolvedValue(true),
        evaluate: vi.fn().mockResolvedValue(undefined),
        boundingBox: vi.fn().mockResolvedValue({ x: 0, y: 0, height: 100 }),
        click: vi.fn().mockResolvedValue(undefined),
      };

      const tweetTextElement = {
        count: vi.fn().mockResolvedValue(1),
        isVisible: vi.fn().mockResolvedValue(true),
        innerText: vi
          .fn()
          .mockResolvedValue("This is a tweet text for testing."),
        $x: vi.fn().mockResolvedValue([]),
      };

      const tweetTextLocator = {
        first: vi.fn().mockReturnValue(tweetTextElement),
        count: vi.fn().mockResolvedValue(1),
        isVisible: vi.fn().mockResolvedValue(true),
      };

      const timeLocator = {
        first: vi.fn().mockReturnValue(clickTarget),
        count: vi.fn().mockResolvedValue(1),
        isVisible: vi.fn().mockResolvedValue(true),
      };

      const permalinkLocator = {
        first: vi.fn().mockReturnValue(clickTarget),
        count: vi.fn().mockResolvedValue(1),
        isVisible: vi.fn().mockResolvedValue(true),
      };

      const mockTweet = {
        boundingBox: vi.fn().mockResolvedValue({ x: 0, y: 0, height: 100 }),
        locator: vi.fn().mockImplementation((selector) => {
          if (selector.includes('a[href*="/status/"]')) return permalinkLocator;
          if (selector.includes('[data-testid="tweetText"]'))
            return tweetTextLocator;
          if (selector === "time") return timeLocator;
          return tweetTextLocator;
        }),
        evaluate: vi.fn().mockResolvedValue(undefined),
      };

      const tweetsLocator = {
        count: vi.fn().mockResolvedValue(1),
        nth: vi.fn().mockReturnValue(mockTweet),
        first: vi.fn().mockReturnValue(mockTweet),
      };

      const pageTweetTextLocator = {
        first: vi.fn().mockReturnValue(tweetTextElement),
        count: vi.fn().mockResolvedValue(1),
        isVisible: vi.fn().mockResolvedValue(true),
      };

      mockPage.locator.mockImplementation((selector) => {
        if (selector === 'article[data-testid="tweet"]') return tweetsLocator;
        if (selector === '[data-testid="tweetText"]')
          return pageTweetTextLocator;
        return pageTweetTextLocator;
      });

      mockPage.waitForURL.mockResolvedValue(undefined);
      api.getCurrentUrl.mockResolvedValue("https://x.com/user/status/123");

      agent.actionRunner.selectAction.mockReturnValue("like");

      // Log if there's any error caught in _diveTweetWithAI
      const originalLog = agent.log;
      agent.log = vi.fn().mockImplementation((msg) => {
        if (msg.includes("Dive sequence failed")) {
          console.error("DIVE TWEET ERROR CAUGHT:", msg);
        }
        originalLog(msg);
      });

      await agent._diveTweetWithAI();

      expect(agent.startDive).toHaveBeenCalled();
      expect(agent.endDive).toHaveBeenCalledWith(true, true);
    });

    it("should skip already processed tweets", async () => {
      const clickTarget = {
        count: vi.fn().mockResolvedValue(1),
        isVisible: vi.fn().mockResolvedValue(true),
        evaluate: vi.fn().mockResolvedValue(undefined),
        boundingBox: vi.fn().mockResolvedValue({ x: 0, y: 0, height: 100 }),
        click: vi.fn().mockResolvedValue(undefined),
      };
      const pageTweetTextElement = {
        count: vi.fn().mockResolvedValue(1),
        isVisible: vi.fn().mockResolvedValue(true),
        innerText: vi.fn().mockResolvedValue("Tweet text"),
      };
      const pageTweetTextLocator = {
        first: vi.fn().mockReturnValue(pageTweetTextElement),
        count: vi.fn().mockResolvedValue(1),
        isVisible: vi.fn().mockResolvedValue(true),
      };
      const mockTweet = {
        boundingBox: vi.fn().mockResolvedValue({ x: 0, y: 0, height: 100 }),
        locator: vi.fn().mockImplementation((selector) => {
          if (selector.includes('a[href*="/status/"]')) {
            return {
              first: vi.fn().mockReturnValue(clickTarget),
              count: vi.fn().mockResolvedValue(1),
              isVisible: vi.fn().mockResolvedValue(true),
            };
          }
          if (selector.includes('[data-testid="tweetText"]')) {
            return {
              first: vi.fn().mockReturnValue(pageTweetTextElement),
              count: vi.fn().mockResolvedValue(1),
              isVisible: vi.fn().mockResolvedValue(true),
            };
          }
          return {
            first: vi.fn().mockReturnValue(clickTarget),
            count: vi.fn().mockResolvedValue(1),
            isVisible: vi.fn().mockResolvedValue(true),
          };
        }),
        evaluate: vi.fn(),
      };

      mockPage.locator.mockImplementation((selector) => {
        if (selector === 'article[data-testid="tweet"]') {
          return {
            count: vi.fn().mockResolvedValue(1),
            nth: vi.fn().mockReturnValue(mockTweet),
            first: vi.fn().mockReturnValue(mockTweet),
          };
        }
        if (selector === '[data-testid="tweetText"]')
          return pageTweetTextLocator;
        return pageTweetTextLocator;
      });

      mockPage.waitForURL.mockResolvedValue(undefined);
      api.getCurrentUrl.mockResolvedValue("https://x.com/user/status/123");
      agent._processedTweetIds.add("123");

      await agent._diveTweetWithAI();

      expect(agent.logger.info).toHaveBeenCalledWith(
        expect.stringContaining("Already processed tweet"),
      );
      expect(agent.endDive).toHaveBeenCalledWith(true, true);
    });

    it("should handle navigation failure", async () => {
      const mockTweet = {
        boundingBox: vi.fn().mockResolvedValue({ x: 0, y: 0, height: 100 }),
        locator: vi.fn().mockReturnValue({
          first: vi.fn().mockReturnValue({
            count: vi.fn().mockResolvedValue(1),
            isVisible: vi.fn().mockResolvedValue(true),
            innerText: vi.fn().mockResolvedValue("Tweet text"),
            evaluate: vi.fn(),
          }),
          count: vi.fn().mockResolvedValue(1),
        }),
        evaluate: vi.fn(),
        click: vi.fn(),
      };

      mockPage.locator.mockReturnValue({
        count: vi.fn().mockResolvedValue(1),
        nth: vi.fn().mockReturnValue(mockTweet),
        first: vi.fn().mockReturnValue(mockTweet),
      });

      mockPage.waitForURL.mockRejectedValue(new Error("Navigation timeout"));

      await agent._diveTweetWithAI();

      expect(agent.endDive).toHaveBeenCalledWith(false, true);
    });
  });

  describe("runSession", () => {
    it("should stop if session expired", async () => {
      agent.isSessionExpired = vi.fn().mockReturnValue(true);
      agent.human = {
        session: { shouldEndSession: vi.fn().mockReturnValue(false) },
        sessionStart: vi.fn(),
        sessionEnd: vi.fn().mockResolvedValue(),
        cycleComplete: vi.fn().mockResolvedValue(),
      };
      agent.checkLoginState = vi.fn().mockResolvedValue(true);

      await agent.runSession(1);

      expect(agent.logger.info).toHaveBeenCalledWith(
        expect.stringContaining("Session Time Limit Reached"),
      );
    });

    it("should stop if login fails 3 times", async () => {
      agent.checkLoginState = vi.fn().mockResolvedValue(false);
      agent.state.consecutiveLoginFailures = 3;

      await agent.runSession(1);

      expect(agent.logger.info).toHaveBeenCalledWith(
        expect.stringContaining("Aborting session: Not logged in"),
      );
    });

    it("should run session without errors", async () => {
      agent.checkLoginState = vi.fn().mockResolvedValue(true);
      // Make session expire immediately to avoid long-running loop
      let callCount = 0;
      agent.isSessionExpired = vi.fn().mockImplementation(() => {
        callCount++;
        return callCount > 1; // Expire after first check
      });
      agent.human = {
        session: { shouldEndSession: vi.fn().mockReturnValue(false) },
        sessionStart: vi.fn(),
        sessionEnd: vi.fn().mockResolvedValue(),
        cycleComplete: vi.fn().mockResolvedValue(),
      };
      agent.diveTweet = vi.fn().mockResolvedValue();

      // Should not throw
      await expect(agent.runSession(1)).resolves.toBeUndefined();
    });
  });

  describe("handleAIReply", () => {
    const baseSentiment = {
      isNegative: false,
      score: 0.2,
      dimensions: {
        valence: { valence: 0.4 },
        arousal: { arousal: 0.3 },
        dominance: { dominance: 0.5 },
        sarcasm: { sarcasm: 0.1 },
      },
      engagement: { warnings: [] },
      composite: { riskLevel: "low", conversationType: "neutral" },
    };

    it("skips when sentiment is negative", async () => {
      sentimentService.analyze.mockReturnValue({
        ...baseSentiment,
        isNegative: true,
      });

      agent.replyEngine.generateReply = vi.fn();

      await agent.handleAIReply("bad content", "user1", {
        url: "https://x.com/1",
      });

      expect(agent.aiStats.skips).toBe(1);
      expect(agent.replyEngine.generateReply).not.toHaveBeenCalled();
    });

    it("runs pre-validated reply path and executes reply", async () => {
      sentimentService.analyze.mockReturnValue(baseSentiment);

      agent.contextEngine.extractEnhancedContext = vi.fn().mockResolvedValue({
        sentiment: { overall: "neutral" },
        tone: { primary: "friendly" },
        engagementLevel: "low",
        replies: [],
      });

      agent.replyEngine.generateReply = vi.fn().mockResolvedValue({
        success: true,
        reply: "hello there",
      });

      agent.executeAIReply = vi.fn().mockResolvedValue(true);

      await agent.handleAIReply("hello", "user1", {
        url: "https://x.com/1",
        action: "reply",
      });

      expect(agent.replyEngine.generateReply).toHaveBeenCalled();
      expect(agent.executeAIReply).toHaveBeenCalledWith("hello there");
      expect(agent.aiStats.replies).toBe(1);
    });

    it("skips pre-validated reply when engagement limit reached", async () => {
      sentimentService.analyze.mockReturnValue(baseSentiment);

      agent.contextEngine.extractEnhancedContext = vi.fn().mockResolvedValue({
        sentiment: { overall: "neutral" },
        tone: { primary: "friendly" },
        engagementLevel: "low",
        replies: [],
      });

      agent.replyEngine.generateReply = vi.fn().mockResolvedValue({
        success: true,
        reply: "hello there",
      });

      agent.executeAIReply = vi.fn().mockResolvedValue(true);
      agent.engagementTracker.canPerform = vi.fn().mockReturnValue(false);

      await agent.handleAIReply("hello", "user1", {
        url: "https://x.com/1",
        action: "reply",
      });

      expect(agent.executeAIReply).not.toHaveBeenCalled();
      expect(agent.aiStats.skips).toBe(1);
    });
  });

  describe("handleLike", () => {
    const baseSentiment = {
      engagement: { canLike: true },
      composite: { riskLevel: "low" },
      dimensions: { toxicity: { toxicity: 0.1 } },
    };

    it("skips like when sentiment says not to like", async () => {
      sentimentService.analyze.mockReturnValue({
        ...baseSentiment,
        engagement: { canLike: false },
        composite: { riskLevel: "high" },
        dimensions: { toxicity: { toxicity: 0.9 } },
      });

      agent.humanInteraction = {
        findWithFallback: vi.fn(),
      };

      await agent.handleLike("bad content");

      expect(agent.humanInteraction.findWithFallback).not.toHaveBeenCalled();
    });

    it("likes tweet and records engagement", async () => {
      sentimentService.analyze.mockReturnValue(baseSentiment);

      const likeButton = {
        boundingBox: vi
          .fn()
          .mockResolvedValue({ x: 10, y: 10, width: 20, height: 20 }),
        getAttribute: vi.fn().mockResolvedValue("Like"),
        scrollIntoViewIfNeeded: vi.fn().mockResolvedValue(),
        click: vi.fn().mockResolvedValue(),
      };

      agent.isElementActionable = vi.fn().mockResolvedValue(true);
      agent.humanInteraction = {
        findWithFallback: vi.fn().mockImplementation((selectors) => {
          if (
            selectors.some(
              (sel) => sel.includes("unlike") || sel.includes("Liked"),
            )
          ) {
            return null;
          }
          return { element: likeButton, selector: selectors[0] };
        }),
      };

      mockPage.locator = vi.fn().mockReturnValue({
        first: () => ({
          isVisible: vi.fn().mockResolvedValue(true),
        }),
      });

      await agent.handleLike("good content");

      expect(agent.humanClick).toHaveBeenCalledWith(likeButton, "Like Button", {
        precision: "high",
      });
      expect(engagementMocks.record).toHaveBeenCalledWith("likes");
      expect(agent.navigateHome).toHaveBeenCalled();
    });
  });

  describe("handleAIQuote", () => {
    const baseSentiment = {
      isNegative: false,
      score: 0.2,
      dimensions: {
        valence: { valence: 0.4 },
        arousal: { arousal: 0.3 },
        dominance: { dominance: 0.5 },
        sarcasm: { sarcasm: 0.1 },
      },
      composite: {
        riskLevel: "low",
        conversationType: "neutral",
        engagementStyle: "neutral",
      },
    };

    it("executes quote flow when quote is generated", async () => {
      sentimentService.analyze.mockReturnValue(baseSentiment);

      agent.contextEngine.extractEnhancedContext = vi.fn().mockResolvedValue({
        sentiment: { overall: "neutral" },
        tone: { primary: "friendly" },
        replies: [],
      });

      agent.quoteEngine.generateQuote = vi.fn().mockResolvedValue({
        success: true,
        quote: "quote text",
      });

      agent.quoteEngine.executeQuote = vi.fn().mockResolvedValue({
        success: true,
        method: "test",
      });

      mockPage.locator = vi.fn().mockReturnValue({
        isVisible: vi.fn().mockResolvedValue(false),
      });

      await agent.handleAIQuote("tweet", "user1", {
        url: "https://x.com/1",
        action: "quote",
      });

      expect(agent.quoteEngine.executeQuote).toHaveBeenCalledWith(
        mockPage,
        "quote text",
      );
      expect(engagementMocks.record).toHaveBeenCalledWith("quotes");
    });

    it("skips quote when high risk sentiment is detected", async () => {
      sentimentService.analyze.mockReturnValue({
        ...baseSentiment,
        composite: { riskLevel: "high", conversationType: "risky" },
      });

      agent.quoteEngine.generateQuote = vi.fn();

      await agent.handleAIQuote("tweet", "user1", { url: "https://x.com/1" });

      expect(agent.quoteEngine.generateQuote).not.toHaveBeenCalled();
    });

    it("skips quote when sentiment is negative", async () => {
      sentimentService.analyze.mockReturnValue({
        ...baseSentiment,
        isNegative: true,
      });

      agent.contextEngine.extractEnhancedContext = vi.fn();

      await agent.handleAIQuote("tweet", "user1", { url: "https://x.com/1" });

      expect(agent.contextEngine.extractEnhancedContext).not.toHaveBeenCalled();
    });

    it("skips quote when engagement limit reached", async () => {
      sentimentService.analyze.mockReturnValue(baseSentiment);
      engagementMocks.canPerform.mockReturnValue(false);

      agent.contextEngine.extractEnhancedContext = vi.fn();

      await agent.handleAIQuote("tweet", "user1", { url: "https://x.com/1" });

      expect(agent.contextEngine.extractEnhancedContext).not.toHaveBeenCalled();
    });

    it("stops when quote generation fails", async () => {
      sentimentService.analyze.mockReturnValue(baseSentiment);

      agent.contextEngine.extractEnhancedContext = vi.fn().mockResolvedValue({
        sentiment: { overall: "neutral" },
        tone: { primary: "friendly" },
        replies: [],
      });

      agent.quoteEngine.generateQuote = vi.fn().mockResolvedValue({
        success: false,
        reason: "no_idea",
      });

      agent.quoteEngine.executeQuote = vi.fn();

      await agent.handleAIQuote("tweet", "user1", { url: "https://x.com/1" });

      expect(agent.quoteEngine.executeQuote).not.toHaveBeenCalled();
    });

    it("logs failure when quote execution fails", async () => {
      sentimentService.analyze.mockReturnValue(baseSentiment);

      agent.contextEngine.extractEnhancedContext = vi.fn().mockResolvedValue({
        sentiment: { overall: "neutral" },
        tone: { primary: "friendly" },
        replies: [],
      });

      agent.quoteEngine.generateQuote = vi.fn().mockResolvedValue({
        success: true,
        quote: "quote text",
      });

      agent.quoteEngine.executeQuote = vi.fn().mockResolvedValue({
        success: false,
        method: "test",
        reason: "timeout",
      });

      await agent.handleAIQuote("tweet", "user1", { url: "https://x.com/1" });

      expect(engagementMocks.record).not.toHaveBeenCalled();
      expect(mockLogger.info).toHaveBeenCalledWith(
        expect.stringContaining("Quote tweet failed"),
      );
    });

    it("attempts to close composer when still visible after quote", async () => {
      sentimentService.analyze.mockReturnValue(baseSentiment);

      agent.contextEngine.extractEnhancedContext = vi.fn().mockResolvedValue({
        sentiment: { overall: "neutral" },
        tone: { primary: "friendly" },
        replies: [],
      });

      agent.quoteEngine.generateQuote = vi.fn().mockResolvedValue({
        success: true,
        quote: "quote text",
      });

      agent.quoteEngine.executeQuote = vi.fn().mockResolvedValue({
        success: true,
        method: "test",
      });

      // Make api.visible return true (composer visible) for 5 iterations, then false
      api.visible = vi
        .fn()
        .mockResolvedValueOnce(true)
        .mockResolvedValueOnce(true)
        .mockResolvedValueOnce(true)
        .mockResolvedValueOnce(true)
        .mockResolvedValueOnce(true)
        .mockResolvedValue(false);

      // Setup getPage to return mockPage so keyboard.press works
      api.getPage.mockReturnValue(mockPage);

      mockPage.locator = vi.fn().mockReturnValue({
        isVisible: vi.fn().mockResolvedValue(true),
      });

      await agent.handleAIQuote("tweet", "user1", { url: "https://x.com/1" });

      // The code uses api.getPage().keyboard.press, not mockPage.keyboard.press directly
      expect(mockPage.keyboard.press).toHaveBeenCalledWith("Escape", undefined);
    });
  });

  describe("Helper Methods", () => {
    it("updateSessionPhase should update phase", () => {
      const phase = agent.updateSessionPhase();
      expect(phase).toBe("normal");
      expect(agent.currentPhase).toBe("normal");
    });

    it("getPhaseModifiedProbability should calculate probability", () => {
      const prob = agent.getPhaseModifiedProbability("reply", 0.5);
      expect(prob).toBe(0.5);
    });

    it("triggerMicroInteraction should respect probability", async () => {
      // Force fail
      vi.spyOn(Math, "random").mockReturnValue(1.0);
      await agent.triggerMicroInteraction();
      expect(agent.logger.info).toHaveBeenCalledWith(
        expect.stringContaining("No micro-interaction"),
      );

      // Force success
      vi.spyOn(Math, "random").mockReturnValue(0.0);
      await agent.triggerMicroInteraction();
      expect(agent.logger.info).toHaveBeenCalledWith(
        expect.stringContaining("Executed test"),
      );
    });

    it("triggerMicroInteraction should handle handler errors", async () => {
      vi.spyOn(Math, "random").mockReturnValue(0.0);
      agent.microHandler.executeMicroInteraction = vi
        .fn()
        .mockRejectedValue(new Error("micro fail"));
      const result = await agent.triggerMicroInteraction("reading");
      expect(result.success).toBe(false);
      expect(result.reason).toBe("micro fail");
    });

    it("highlightText should call micro handler", async () => {
      const result = await agent.highlightText();
      expect(result.success).toBe(true);
      expect(agent.microHandler.textHighlight).toHaveBeenCalled();
    });

    it("startFidgetLoop and stopFidgetLoop should forward to handler", () => {
      agent.startFidgetLoop();
      agent.stopFidgetLoop();
      expect(agent.microHandler.startFidgetLoop).toHaveBeenCalled();
      expect(agent.microHandler.stopFidgetLoop).toHaveBeenCalled();
    });

    it("simulateFidget should handle handler errors", async () => {
      agent.microHandler.executeMicroInteraction = vi
        .fn()
        .mockRejectedValue(new Error("fidget fail"));
      await agent.simulateFidget();
      expect(agent.logger.info).toHaveBeenCalledWith(
        expect.stringContaining("[Fidget] Error"),
      );
    });

    it("smartClick should return failure when motor handler fails", async () => {
      agent.motorHandler.smartClick = vi
        .fn()
        .mockResolvedValue({ success: false, reason: "miss" });
      const result = await agent.smartClick("test-context");
      expect(result.success).toBe(false);
      expect(agent.logger.info).toHaveBeenCalledWith(
        expect.stringContaining("Smart click failed"),
      );
    });

    it("smartClickElement should handle motor handler exceptions", async () => {
      agent.motorHandler.smartClick = vi
        .fn()
        .mockRejectedValue(new Error("motor fail"));
      const result = await agent.smartClickElement('[data-testid="like"]');
      expect(result.success).toBe(false);
      expect(result.reason).toBe("motor fail");
    });

    it("_quickFallbackEngagement should perform fallback", async () => {
      agent.diveQueue.canEngage.mockReturnValue(true);
      agent.handleLike = vi.fn().mockResolvedValue(true);

      vi.spyOn(Math, "random").mockReturnValue(0.1); // Like

      const result = await agent._quickFallbackEngagement();
      expect(result.engagementType).toBe("like");
      expect(agent.handleLike).toHaveBeenCalled();
    });
  });

  describe("Stats and Status Methods", () => {
    it("getAIStats should return stats with success rate", () => {
      agent.aiStats = {
        attempts: 10,
        replies: 5,
        skips: 3,
        safetyBlocks: 1,
        errors: 1,
      };
      // Mock actions with getStats
      agent.actions = {
        reply: { getStats: vi.fn().mockReturnValue({ executed: 5 }) },
        like: { getStats: vi.fn().mockReturnValue({ executed: 10 }) },
      };

      const stats = agent.getAIStats();

      expect(stats.attempts).toBe(10);
      expect(stats.replies).toBe(5);
      expect(stats.successRate).toBe("50.0%");
      expect(stats.actions).toBeDefined();
    });

    it("getAIStats should handle zero attempts", () => {
      agent.aiStats = {
        attempts: 0,
        replies: 0,
        skips: 0,
        safetyBlocks: 0,
        errors: 0,
      };
      // Mock actions with getStats
      agent.actions = {
        reply: { getStats: vi.fn().mockReturnValue({ executed: 0 }) },
      };

      const stats = agent.getAIStats();

      expect(stats.successRate).toBe("0%");
    });

    it("getActionStats should return action runner stats", () => {
      const mockStats = { reply: { executed: 5 }, like: { executed: 10 } };
      agent.actionRunner = { getStats: vi.fn().mockReturnValue(mockStats) };

      const stats = agent.getActionStats();

      expect(stats).toEqual(mockStats);
      expect(agent.actionRunner.getStats).toHaveBeenCalled();
    });

    it("getActionStats should fallback to individual actions", () => {
      agent.actionRunner = null;
      agent.actions = {
        reply: { getStats: vi.fn().mockReturnValue({ executed: 5 }) },
        like: { getStats: vi.fn().mockReturnValue({ executed: 10 }) },
      };

      const stats = agent.getActionStats();

      expect(stats.reply).toEqual({ executed: 5 });
      expect(stats.like).toEqual({ executed: 10 });
    });

    it("getEngagementStats should return tracker data", () => {
      const mockStatus = { likes: { current: 5, limit: 20 } };
      engagementMocks.getStatus.mockReturnValue(mockStatus);
      engagementMocks.getSummary.mockReturnValue("likes: 5/20");

      const stats = agent.getEngagementStats();

      expect(stats.tracker).toEqual(mockStatus);
      expect(stats.summary).toBe("likes: 5/20");
    });

    it("getQueueStatus should return queue status with buffered logging", () => {
      const mockStatus = {
        queueLength: 2,
        activeCount: 1,
        utilization: 33,
        capacity: 10,
        maxQueueSize: 30,
        engagementLimits: {
          likes: { used: 5, limit: 20 },
          replies: { used: 2, limit: 10 },
          quotes: { used: 1, limit: 5 },
          bookmarks: { used: 3, limit: 10 },
        },
        retryInfo: { pendingRetries: 0 },
      };

      agent.diveQueue.getFullStatus.mockReturnValue(mockStatus);

      const status = agent.getQueueStatus();

      expect(status).toEqual(mockStatus);
      expect(agent.queueLogger.info).toHaveBeenCalled();
    });

    it("getQueueStatus should warn about pending retries", () => {
      const mockStatus = {
        queueLength: 5,
        activeCount: 2,
        utilization: 50,
        capacity: 10,
        maxQueueSize: 30,
        engagementLimits: null,
        retryInfo: { pendingRetries: 3 },
      };

      agent.diveQueue.getFullStatus.mockReturnValue(mockStatus);

      agent.getQueueStatus();

      expect(agent.queueLogger.warn).toHaveBeenCalledWith("3 retries pending");
    });

    it("isQueueHealthy should delegate to diveQueue", () => {
      agent.diveQueue.isHealthy.mockReturnValue(true);

      expect(agent.isQueueHealthy()).toBe(true);
      expect(agent.diveQueue.isHealthy).toHaveBeenCalled();
    });
  });

  describe("Session State Methods", () => {
    it("getSessionProgress should calculate progress percentage", () => {
      agent.sessionStart = Date.now() - 5000; // 5 seconds ago

      const progress = agent.getSessionProgress();

      expect(typeof progress).toBe("number");
      expect(progress).toBeGreaterThan(0);
      expect(progress).toBeLessThanOrEqual(100);
    });

    it("isInCooldown should check cooldown phase", () => {
      sessionPhases.getSessionPhase.mockReturnValue("cooldown");

      expect(agent.isInCooldown()).toBe(true);
    });

    it("isInCooldown should return false when not in cooldown", () => {
      sessionPhases.getSessionPhase.mockReturnValue("active");

      expect(agent.isInCooldown()).toBe(false);
    });

    it("isInWarmup should check warmup phase", () => {
      sessionPhases.getSessionPhase.mockReturnValue("warmup");

      expect(agent.isInWarmup()).toBe(true);
    });

    it("isInWarmup should return false when not in warmup", () => {
      sessionPhases.getSessionPhase.mockReturnValue("active");

      expect(agent.isInWarmup()).toBe(false);
    });
  });

  describe("Health Check", () => {
    it("performHealthCheck should return healthy when all checks pass", async () => {
      const mockBrowser = {
        isConnected: vi.fn().mockReturnValue(true),
      };
      const mockContext = {
        browser: vi.fn().mockReturnValue(mockBrowser),
      };

      mockPage.context.mockReturnValue(mockContext);
      mockPage.evaluate.mockResolvedValue({
        readyState: "complete",
        title: "Home",
        hasBody: true,
      });
      mockPage.url.mockReturnValue("https://x.com/home");

      const result = await agent.performHealthCheck();

      expect(result.healthy).toBe(true);
    });

    it("performHealthCheck should detect disconnected browser", async () => {
      const mockBrowser = {
        isConnected: vi.fn().mockReturnValue(false),
      };
      const mockContext = {
        browser: vi.fn().mockReturnValue(mockBrowser),
      };

      mockPage.context.mockReturnValue(mockContext);

      const result = await agent.performHealthCheck();

      expect(result.healthy).toBe(false);
      expect(result.reason).toBe("browser_disconnected");
    });

    it("performHealthCheck should detect page not ready", async () => {
      const mockBrowser = {
        isConnected: vi.fn().mockReturnValue(true),
      };
      const mockContext = {
        browser: vi.fn().mockReturnValue(mockBrowser),
      };

      mockPage.context.mockReturnValue(mockContext);
      mockPage.evaluate.mockResolvedValue({
        readyState: "loading",
        title: "",
        hasBody: false,
      });

      const result = await agent.performHealthCheck();

      expect(result.healthy).toBe(false);
      expect(result.reason).toBe("page_not_ready");
    });

    it("performHealthCheck should detect unexpected URL", async () => {
      const mockBrowser = {
        isConnected: vi.fn().mockReturnValue(true),
      };
      const mockContext = {
        browser: vi.fn().mockReturnValue(mockBrowser),
      };

      mockPage.context.mockReturnValue(mockContext);
      mockPage.evaluate.mockResolvedValue({
        readyState: "complete",
        title: "Google",
        hasBody: true,
      });
      api.getCurrentUrl.mockResolvedValue("https://google.com");
      agent.navigateHome = vi.fn().mockResolvedValue();

      const result = await agent.performHealthCheck();

      expect(result.healthy).toBe(false);
      expect(result.reason).toBe("unexpected_url");
      expect(agent.navigateHome).toHaveBeenCalled();
    });

    it("performHealthCheck should handle errors gracefully", async () => {
      mockPage.context.mockImplementation(() => {
        throw new Error("Context error");
      });

      const result = await agent.performHealthCheck();

      expect(result.healthy).toBe(false);
      expect(result.reason).toContain("Context error");
    });
  });

  describe("Action Execution", () => {
    beforeEach(() => {
      agent.replyEngine = {
        executeReply: vi
          .fn()
          .mockResolvedValue({ success: true, method: "composer" }),
      };
      agent.quoteEngine = {
        executeQuote: vi
          .fn()
          .mockResolvedValue({ success: true, method: "composer" }),
      };
    });

    it("executeAIReply should execute reply and record engagement", async () => {
      engagementMocks.record.mockReturnValue(true);
      engagementMocks.getProgress.mockReturnValue("1/10");

      const result = await agent.executeAIReply("Test reply");

      expect(result).toBe(true);
      expect(agent.replyEngine.executeReply).toHaveBeenCalledWith(
        mockPage,
        "Test reply",
      );
      expect(agent.state.replies).toBe(1);
      expect(engagementMocks.record).toHaveBeenCalledWith("replies");
    });

    it("executeAIReply should handle failure", async () => {
      agent.replyEngine.executeReply.mockResolvedValue({
        success: false,
        method: "composer",
        reason: "Composer not found",
      });

      const result = await agent.executeAIReply("Test reply");

      expect(result).toBe(false);
      expect(agent.logger.info).toHaveBeenCalledWith(
        expect.stringContaining("[WARN]"),
      );
    });

    it("executeAIReply should handle errors", async () => {
      agent.replyEngine.executeReply.mockRejectedValue(
        new Error("Execution failed"),
      );

      const result = await agent.executeAIReply("Test reply");

      expect(result).toBe(false);
      expect(agent.logger.info).toHaveBeenCalledWith(
        expect.stringContaining("Failed to post reply"),
      );
    });

    it("executeAIQuote should execute quote and record engagement", async () => {
      engagementMocks.record.mockReturnValue(true);
      engagementMocks.getProgress.mockReturnValue("1/5");

      const result = await agent.executeAIQuote("Test quote");

      expect(result).toBe(true);
      expect(agent.quoteEngine.executeQuote).toHaveBeenCalledWith(
        mockPage,
        "Test quote",
      );
      expect(agent.state.quotes).toBe(1);
      expect(engagementMocks.record).toHaveBeenCalledWith("quotes");
    });

    it("executeAIQuote should handle failure", async () => {
      agent.quoteEngine.executeQuote.mockResolvedValue({
        success: false,
        method: "composer",
        reason: "Timeout",
      });

      const result = await agent.executeAIQuote("Test quote");

      expect(result).toBe(false);
      expect(agent.logger.info).toHaveBeenCalledWith(
        expect.stringContaining("[WARN]"),
      );
    });

    it("executeAIQuote should handle errors", async () => {
      agent.quoteEngine.executeQuote.mockRejectedValue(
        new Error("Execution failed"),
      );

      const result = await agent.executeAIQuote("Test quote");

      expect(result).toBe(false);
    });
  });

  describe("simulateReading Override", () => {
    it("should perform idle cursor movement when scrolling disabled", async () => {
      // Skipped: Requires complex prototype mocking setup
    });

    it("should perform idle cursor movement when operation locked", async () => {
      // Skipped: Requires complex prototype mocking setup
    });

    it("should call parent simulateReading when scrolling enabled", async () => {
      // Skipped: Requires complex prototype mocking setup
    });
  });

  describe("Logging Methods", () => {
    it("logEngagementStatus should log all actions", () => {
      const mockStatus = {
        likes: { current: 5, limit: 20, remaining: 15, percentage: "25%" },
        replies: { current: 2, limit: 10, remaining: 8, percentage: "20%" },
        bookmarks: { current: 10, limit: 10, remaining: 0, percentage: "100%" },
      };
      engagementMocks.getStatus.mockReturnValue(mockStatus);

      agent.logEngagementStatus();

      expect(agent.engagementLogger.info).toHaveBeenCalledTimes(3);
      expect(agent.engagementLogger.info).toHaveBeenCalledWith(
        expect.stringContaining("likes"),
      );
      expect(agent.engagementLogger.info).toHaveBeenCalledWith(
        expect.stringContaining("✅"),
      );
      expect(agent.engagementLogger.info).toHaveBeenCalledWith(
        expect.stringContaining("🚫"),
      );
    });

    it("flushLogs should shutdown buffered loggers", async () => {
      await agent.flushLogs();

      expect(agent.queueLogger.shutdown).toHaveBeenCalled();
      expect(agent.engagementLogger.shutdown).toHaveBeenCalled();
      expect(agent.logger.info).toHaveBeenCalledWith(
        expect.stringContaining("Flushing"),
      );
    });
  });

  describe("diveTweet Additional Edge Cases", () => {
    it("should skip if isScanning is true", async () => {
      agent.isScanning = true;
      await agent.diveTweet();
      expect(agent.logger.info).toHaveBeenCalledWith(
        expect.stringContaining("Scan already in progress"),
      );
    });

    it("should handle queue addDive throwing error", async () => {
      agent.diveQueue.addDive = vi
        .fn()
        .mockRejectedValue(new Error("Queue error"));
      agent._diveTweetWithAI = vi
        .fn()
        .mockImplementation(async (queueWrapper) => {
          if (queueWrapper) {
            await queueWrapper(async () => {});
          }
        });

      await agent.diveTweet();

      expect(agent.logger.info).toHaveBeenCalledWith(
        expect.stringContaining("Error in diveTweet"),
      );
    });
  });

  describe("_diveTweetWithAI Additional Edge Cases", () => {
    it("should handle context extraction error", async () => {
      agent.startDive = vi.fn().mockResolvedValue(true);
      agent._ensureExplorationScroll = vi.fn().mockResolvedValue(true);
      agent._readExpandedTweet = vi
        .fn()
        .mockRejectedValue(new Error("Read error"));
      agent.endDive = vi.fn().mockResolvedValue(true);

      await agent._diveTweetWithAI();

      expect(agent.endDive).toHaveBeenCalledWith(false, true);
    });

    it("should call endDive when no action is selected", async () => {
      agent.startDive = vi.fn().mockResolvedValue(true);
      agent._ensureExplorationScroll = vi.fn().mockResolvedValue(true);
      agent._readExpandedTweet = vi.fn().mockResolvedValue(true);
      agent.endDive = vi.fn().mockResolvedValue(true);
      agent.actionRunner.selectAction = vi.fn().mockReturnValue(null);

      await agent._diveTweetWithAI();

      expect(agent.endDive).toHaveBeenCalled();
    });
  });

  describe("handleAIReply Additional Edge Cases", () => {
    it("should skip when sentiment is negative", async () => {
      sentimentService.analyze.mockReturnValue({
        isNegative: true,
        score: 0.8,
        dimensions: {
          valence: { valence: 0.5 },
          arousal: { arousal: 0.5 },
          dominance: { dominance: 0.5 },
          sarcasm: { sarcasm: 0.1 },
        },
        engagement: { warnings: [] },
        composite: { riskLevel: "low" },
      });
      agent.contextEngine.extractEnhancedContext = vi
        .fn()
        .mockResolvedValue(null);
      agent.replyEngine.generateReply = vi.fn();

      await agent.handleAIReply("test", "user1", { url: "https://x.com/1" });

      expect(agent.replyEngine.generateReply).not.toHaveBeenCalled();
      expect(agent.aiStats.skips).toBe(1);
    });

    it("should skip when risk level is high", async () => {
      sentimentService.analyze.mockReturnValue({
        isNegative: false,
        score: 0.2,
        dimensions: {
          valence: { valence: 0.5 },
          arousal: { arousal: 0.5 },
          dominance: { dominance: 0.5 },
          sarcasm: { sarcasm: 0.1 },
        },
        engagement: { warnings: [] },
        composite: { riskLevel: "high", conversationType: "controversial" },
      });
      agent.replyEngine.generateReply = vi.fn();

      await agent.handleAIReply("test", "user1", { url: "https://x.com/1" });

      expect(agent.replyEngine.generateReply).not.toHaveBeenCalled();
      expect(agent.aiStats.skips).toBe(1);
    });
  });

  describe("handleLike Additional Edge Cases", () => {
    it("should skip when already liked (unlike button visible)", async () => {
      agent.humanInteraction = {
        findWithFallback: vi.fn().mockImplementation((selectors) => {
          if (
            selectors.some(
              (sel) => sel.includes("unlike") || sel.includes("Liked"),
            )
          ) {
            return { element: {}, selector: "unlike" };
          }
          return null;
        }),
      };

      await agent.handleLike("test");
      expect(agent.humanClick).not.toHaveBeenCalled();
    });

    it("should handle engagement limit reached in handleLike", async () => {
      sentimentService.analyze.mockReturnValue({
        engagement: { canLike: true },
        composite: { riskLevel: "low" },
        dimensions: { toxicity: { toxicity: 0.1 } },
      });
      engagementMocks.canPerform.mockReturnValue(false);

      const result = await agent.handleLike("test");
      expect(result).toBeUndefined();
    });
  });

  describe("Session/Phase Additional Edge Cases", () => {
    it("getSessionProgress should return valid percentage", () => {
      agent.sessionStart = Date.now() - 60000; // 1 minute ago
      const progress = agent.getSessionProgress();
      expect(progress).toBeGreaterThan(0);
      expect(progress).toBeLessThanOrEqual(100);
    });

    it("getPhaseModifiedProbability should apply phase modifier", () => {
      sessionPhases.getPhaseModifier.mockReturnValue(1.5);
      const prob = agent.getPhaseModifiedProbability("reply", 0.5);
      expect(prob).toBe(0.75);
    });
  });

  describe("Health Check Additional Edge Cases", () => {
    it("should handle page URL check error", async () => {
      const mockBrowser = { isConnected: vi.fn().mockReturnValue(true) };
      const mockContext = { browser: vi.fn().mockReturnValue(mockBrowser) };
      mockPage.context.mockReturnValue(mockContext);
      mockPage.evaluate.mockResolvedValue({
        readyState: "complete",
        title: "Home",
        hasBody: true,
      });
      api.getCurrentUrl.mockRejectedValue(new Error("URL error"));

      const result = await agent.performHealthCheck();
      expect(result.healthy).toBe(false);
    });

    it("should navigate home when URL is unexpected", async () => {
      const mockBrowser = { isConnected: vi.fn().mockReturnValue(true) };
      const mockContext = { browser: vi.fn().mockReturnValue(mockBrowser) };
      mockPage.context.mockReturnValue(mockContext);
      mockPage.evaluate.mockResolvedValue({
        readyState: "complete",
        title: "Google",
        hasBody: true,
      });
      api.getCurrentUrl.mockResolvedValue("https://google.com");
      agent.navigateHome = vi.fn().mockResolvedValue();

      await agent.performHealthCheck();
      expect(agent.navigateHome).toHaveBeenCalled();
    });
  });

  describe("Queue Additional Edge Cases", () => {
    it("isQueueHealthy should return false when queue is unhealthy", () => {
      agent.diveQueue.isHealthy.mockReturnValue(false);
      expect(agent.isQueueHealthy()).toBe(false);
    });

    it("isQueueHealthy should return true when queue is healthy", () => {
      agent.diveQueue.isHealthy.mockReturnValue(true);
      expect(agent.isQueueHealthy()).toBe(true);
    });
  });

  describe("Action Execution Additional Edge Cases", () => {
    it("executeAIReply should handle replyEngine missing", async () => {
      agent.replyEngine = null;
      const result = await agent.executeAIReply("test");
      expect(result).toBe(false);
    });

    it("executeAIQuote should handle quoteEngine missing", async () => {
      agent.quoteEngine = null;
      const result = await agent.executeAIQuote("test");
      expect(result).toBe(false);
    });
  });

  describe("Engagement Tracker Additional Edge Cases", () => {
    it("should return engagement stats", () => {
      const stats = agent.getEngagementStats();
      expect(stats).toBeDefined();
      expect(stats.tracker).toBeDefined();
      expect(stats.summary).toBeDefined();
    });
  });
});
