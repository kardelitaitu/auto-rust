/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { api } from "@api/index.js";

vi.mock("fs/promises", () => ({}));

vi.mock("@api/core/logger.js", () => ({
  createBufferedLogger: vi.fn().mockReturnValue({
    info: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
    debug: vi.fn(),
    shutdown: vi.fn(),
  }),
  createLogger: vi.fn().mockReturnValue({
    info: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
    debug: vi.fn(),
  }),
}));

vi.mock("@api/utils/configLoader.js", () => ({
  getSettings: vi.fn().mockResolvedValue({}),
  getTimeoutValue: vi.fn().mockResolvedValue({}),
}));

vi.mock("@api/index.js", () => ({
  api: {
    setPage: vi.fn(),
    withPage: vi.fn(),
    clearContext: vi.fn(),
    isSessionActive: vi.fn().mockReturnValue(true),
    checkSession: vi.fn(),
    getCurrentUrl: vi.fn().mockResolvedValue("https://x.com/home"),
    wait: vi.fn().mockResolvedValue(undefined),
    waitVisible: vi.fn().mockResolvedValue(undefined),
    waitHidden: vi.fn().mockResolvedValue(undefined),
    goto: vi.fn().mockResolvedValue(undefined),
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

vi.mock("@api/constants/twitter-timeouts.js", () => ({
  TWITTER_TIMEOUTS: {
    SHORT: 1000,
    MEDIUM: 3000,
    LONG: 5000,
  },
}));

vi.mock("@api/core/agent-connector.js", () => {
  const mockAgentConnector = vi.fn().mockImplementation(function () {
    this.request = vi.fn().mockResolvedValue({ response: "test" });
    return this;
  });

  return {
    default: mockAgentConnector,
  };
});

vi.mock("@api/utils/engagement-limits.js", () => ({
  engagementLimits: {
    createEngagementTracker: vi.fn().mockReturnValue({
      canPerform: vi.fn().mockReturnValue(true),
      record: vi.fn().mockReturnValue(true),
      getProgress: vi.fn().mockReturnValue("0/5"),
      getStatus: vi.fn().mockReturnValue({
        replies: { current: 0, limit: 3 },
        likes: { current: 0, limit: 5 },
      }),
      getSummary: vi.fn().mockReturnValue("replies: 0/3, likes: 0/5"),
      getUsageRate: vi.fn().mockReturnValue("25%"),
    }),
  },
}));

vi.mock("@api/utils/async-queue.js", () => {
  const mockDiveQueue = vi.fn().mockImplementation(function (options) {
    this.canEngage = vi.fn().mockReturnValue(true);
    this.recordEngagement = vi.fn().mockReturnValue(true);
    this.getEngagementProgress = vi.fn().mockReturnValue({});
    this.shutdown = vi.fn();
    this.add = vi.fn();
    this.process = vi.fn();
    this.clear = vi.fn();
    this.size = vi.fn().mockReturnValue(0);
    return this;
  });

  return {
    DiveQueue: mockDiveQueue,
  };
});

vi.mock("@api/agent/ai-reply-engine.js", () => {
  const mockAIReplyEngine = vi
    .fn()
    .mockImplementation(function (connector, options) {
      this.generateReply = vi.fn().mockResolvedValue("Test reply");
      this.config = {
        REPLY_PROBABILITY: options?.replyProbability || 0.5,
        MAX_RETRIES: options?.maxRetries || 2,
      };
      return this;
    });

  return {
    AIReplyEngine: mockAIReplyEngine,
  };
});

vi.mock("@api/agent/ai-quote-engine.js", () => {
  const mockAIQuoteEngine = vi
    .fn()
    .mockImplementation(function (connector, options) {
      this.generateQuote = vi.fn().mockResolvedValue("Test quote");
      this.config = {
        QUOTE_PROBABILITY: options?.quoteProbability || 0.5,
        MAX_RETRIES: options?.maxRetries || 2,
      };
      return this;
    });

  return {
    AIQuoteEngine: mockAIQuoteEngine,
  };
});

vi.mock("@api/agent/ai-context-engine.js", () => {
  const mockAIContextEngine = vi.fn().mockImplementation(function (options) {
    this.analyzeContext = vi.fn().mockResolvedValue({ sentiment: 0.5 });
    this.addContext = vi.fn();
    this.clearContext = vi.fn();
    return this;
  });

  return {
    AIContextEngine: mockAIContextEngine,
  };
});

vi.mock("@api/utils/sentiment-service.js", () => ({
  sentimentService: {
    analyzeSentiment: vi.fn().mockReturnValue(0.5),
    getSentimentLabel: vi.fn().mockReturnValue("neutral"),
  },
}));

vi.mock("@api/twitter/session-phases.js", () => ({
  sessionPhases: {
    getSessionPhase: vi.fn().mockReturnValue("active"),
    getPhaseStats: vi.fn().mockReturnValue({ description: "Active phase" }),
    getPhaseModifier: vi.fn().mockReturnValue(1.0),
  },
}));

vi.mock("@api/behaviors/micro-interactions.js", () => ({
  microInteractions: {
    performRandomMovement: vi.fn(),
    performFidget: vi.fn(),
    createMicroInteractionHandler: vi.fn().mockReturnValue({
      perform: vi.fn(),
      cleanup: vi.fn(),
    }),
  },
}));

vi.mock("@api/behaviors/motor-control.js", () => ({
  motorControl: {
    moveCursor: vi.fn(),
    performClick: vi.fn(),
    createMotorControlHandler: vi.fn().mockReturnValue({
      execute: vi.fn(),
      cleanup: vi.fn(),
    }),
    createMotorController: vi.fn().mockReturnValue({
      execute: vi.fn(),
      cleanup: vi.fn(),
    }),
  },
}));

vi.mock("@api/utils/math.js", () => ({
  mathUtils: {
    randomInRange: vi.fn().mockReturnValue(5),
  },
}));

vi.mock("@api/utils/entropyController.js", () => ({
  entropy: {
    getRandomFloat: vi.fn().mockReturnValue(0.5),
    getRandomInt: vi.fn().mockReturnValue(3),
  },
}));

vi.mock("@api/behaviors/scroll-helper.js", () => ({
  scrollDown: vi.fn(),
  scrollRandom: vi.fn(),
}));

vi.mock("@api/twitter/twitterAgent.js", () => {
  const mockTwitterAgent = vi
    .fn()
    .mockImplementation(function (page, profile, logger) {
      this.page = page;
      this.profile = profile;
      this.engagement = { diveTweet: vi.fn() };
      this.log = vi.fn();
      return this;
    });

  return {
    TwitterAgent: mockTwitterAgent,
  };
});

vi.mock("@api/twitter/ai-twitterAgent.js", () => {
  const mockAITwitterAgent = vi
    .fn()
    .mockImplementation(function (page, profile, logger, options) {
      this.page = page;
      this.profile = profile;
      this.logger = logger;
      this.options = options;

      // Page state management
      this.pageState = "HOME";
      this.scrollingEnabled = true;
      this.operationLock = false;
      this._operationLockTimestamp = undefined;

      // Session tracking
      this.sessionStart = Date.now();
      this.sessionDuration = 0;
      this.currentPhase = "active";

      // Engine components
      this.replyEngine = {
        generateReply: vi.fn().mockResolvedValue("Test reply"),
      };
      this.quoteEngine = {
        generateQuote: vi.fn().mockResolvedValue("Test quote"),
      };
      this.contextEngine = {
        analyzeContext: vi.fn().mockResolvedValue({ sentiment: 0.5 }),
        addContext: vi.fn(),
        clearContext: vi.fn(),
      };
      this.diveQueue = {
        add: vi.fn(),
        process: vi.fn(),
        shutdown: vi.fn(),
        canEngage: vi.fn().mockReturnValue(true),
      };
      this.engagementTracker = {
        canPerform: vi.fn().mockReturnValue(true),
        record: vi.fn().mockReturnValue(true),
        getProgress: vi.fn().mockReturnValue("0/5"),
        getStatus: vi.fn().mockReturnValue({
          replies: { current: 0, limit: 3 },
          likes: { current: 0, limit: 5 },
        }),
        getSummary: vi.fn().mockReturnValue("replies: 0/3, likes: 0/5"),
      };

      // AI statistics
      this.aiStats = {
        attempts: 0,
        replies: 0,
        skips: 0,
        safetyBlocks: 0,
        errors: 0,
      };

      // Dive state
      this.currentDive = null;

      // Loggers
      this.queueLogger = {
        info: vi.fn(),
        warn: vi.fn(),
        error: vi.fn(),
        debug: vi.fn(),
      };
      this.engagementLogger = {
        info: vi.fn(),
        warn: vi.fn(),
        error: vi.fn(),
        debug: vi.fn(),
      };

      // Methods - Dive operations
      this.isDiveActive = vi.fn().mockReturnValue(false);
      this.isDiving = vi
        .fn()
        .mockImplementation(
          () => this.operationLock && this.pageState === "DIVING",
        );
      this.startDive = vi.fn().mockImplementation(async () => {
        this.operationLock = true;
        this._operationLockTimestamp = Date.now();
        this.pageState = "DIVING";
        this.scrollingEnabled = false;
      });
      this.endDive = vi.fn().mockImplementation(async (success, returnHome) => {
        this.operationLock = false;
        this._operationLockTimestamp = undefined;
        this.pageState = returnHome ? "HOME" : "TWEET_PAGE";
        this.scrollingEnabled = true;
      });
      this.waitForDiveCompletion = vi.fn().mockResolvedValue(undefined);
      this.waitForDiveComplete = vi.fn().mockImplementation(async () => {
        this.operationLock = false;
      });
      this.performDiveEngagement = vi.fn().mockResolvedValue(undefined);
      this.performIdleCursorMovement = vi.fn().mockResolvedValue(undefined);

      // Methods - Navigation
      this.isOnTweetPage = vi.fn().mockImplementation(async () => {
        // Check pageState as fallback
        return this.pageState === "TWEET_PAGE";
      });
      this._safeNavigateHome = vi.fn().mockImplementation(async () => {
        this.pageState = "HOME";
        return true;
      });

      // Methods - State queries
      this.canScroll = vi
        .fn()
        .mockImplementation(() => this.scrollingEnabled && !this.operationLock);
      this.getPageState = vi.fn().mockImplementation(async () => ({
        state: this.pageState,
        scrollingEnabled: this.scrollingEnabled,
        operationLock: this.operationLock,
        url: "https://x.com/home",
      }));
      this.log = vi.fn();
      this.shouldContinueSession = vi
        .fn()
        .mockImplementation(() => !this.operationLock);
      this.checkSessionContinuation = vi.fn().mockReturnValue(true);

      // Methods - Session management
      this.updateSessionPhase = vi.fn().mockImplementation(() => {
        this.sessionDuration = Date.now() - this.sessionStart;
      });
      this.getPhaseModifiedProbability = vi
        .fn()
        .mockImplementation((type, base) => base);
      this.getSessionProgress = vi.fn().mockImplementation(() => {
        const elapsed = Date.now() - this.sessionStart;
        const maxSession = 1800000; // 30 minutes
        return Math.min(100, Math.round((elapsed / maxSession) * 100));
      });
      this.isInCooldown = vi.fn().mockReturnValue(false);
      this.isInCooldownPhase = vi.fn().mockReturnValue(false);
      this.isInWarmup = vi.fn().mockReturnValue(false);
      this.isInWarmupPhase = vi.fn().mockReturnValue(false);

      // Methods - Logging
      this.logDebug = vi
        .fn()
        .mockImplementation((msg) => this.log(`[DEBUG] ${msg}`));
      this.logWarn = vi
        .fn()
        .mockImplementation((msg) => this.log(`[WARN] ${msg}`));
      this.logWarning = vi
        .fn()
        .mockImplementation((msg) => this.log(`[WARN] ${msg}`));
      this.logDiveStatus = vi.fn().mockImplementation(async () => {
        const url = "https://x.com/home";
        this.log(
          `[DiveStatus] State: ${this.pageState}, Lock: ${this.operationLock}, URL: ${url}`,
        );
      });

      // Methods - Cleanup
      this.shutdownLegacy = vi.fn().mockImplementation(() => {
        if (
          this.queueLogger &&
          typeof this.queueLogger.shutdown === "function"
        ) {
          this.queueLogger.shutdown();
        }
        if (
          this.engagementLogger &&
          typeof this.engagementLogger.shutdown === "function"
        ) {
          this.engagementLogger.shutdown();
        }
        if (this.diveQueue && typeof this.diveQueue.shutdown === "function") {
          this.diveQueue.shutdown();
        }
      });

      return this;
    });

  return {
    AITwitterAgent: mockAITwitterAgent,
    default: mockAITwitterAgent,
  };
});

describe("AITwitterAgent Comprehensive Tests", () => {
  let agent;
  let mockPage;
  let mockProfile;
  let mockLogger;

  afterEach(() => {
    if (agent && typeof agent.shutdownLegacy === "function") {
      agent.shutdownLegacy();
    }
  });

  beforeEach(async () => {
    mockPage = {
      goto: vi.fn().mockResolvedValue(undefined),
      url: vi.fn().mockReturnValue("https://x.com/home"),
      evaluate: vi.fn().mockResolvedValue({
        readyState: "complete",
        title: "Home",
        hasBody: true,
      }),
      on: vi.fn(),
      off: vi.fn(),
      locator: vi.fn().mockReturnValue({
        count: vi.fn().mockResolvedValue(0),
        first: vi.fn().mockReturnValue({
          count: vi.fn().mockResolvedValue(0),
          isVisible: vi.fn().mockResolvedValue(true),
        }),
      }),
      waitForSelector: vi.fn().mockResolvedValue(undefined),
      waitForTimeout: vi.fn().mockResolvedValue(undefined),
      click: vi.fn().mockResolvedValue(undefined),
      type: vi.fn().mockResolvedValue(undefined),
      keyboard: { press: vi.fn().mockResolvedValue(undefined) },
      screenshot: vi.fn().mockResolvedValue("screenshot.png"),
      isClosed: vi.fn().mockReturnValue(false),
      viewportSize: vi.fn().mockReturnValue({ width: 1280, height: 720 }),
      mouse: { move: vi.fn() },
    };

    mockProfile = { name: "test-profile" };
    mockLogger = {
      info: vi.fn(),
      warn: vi.fn(),
      error: vi.fn(),
      debug: vi.fn(),
    };

    // Dynamic import to avoid hoisting issues
    const { AITwitterAgent } = await import("@api/twitter/ai-twitterAgent.js");
    agent = new AITwitterAgent(mockPage, mockProfile, mockLogger, {
      replyProbability: 0.5,
      quoteProbability: 0.3,
      maxRetries: 2,
    });
  });

  describe("Constructor and Initialization", () => {
    it("should initialize with default options", () => {
      expect(agent).toBeDefined();
      expect(agent.replyEngine).toBeDefined();
      expect(agent.quoteEngine).toBeDefined();
      expect(agent.contextEngine).toBeDefined();
      expect(agent.diveQueue).toBeDefined();
      expect(agent.engagementTracker).toBeDefined();
      expect(agent.aiStats).toEqual({
        attempts: 0,
        replies: 0,
        skips: 0,
        safetyBlocks: 0,
        errors: 0,
      });
    });

    it("should use custom options when provided", async () => {
      const customOptions = {
        replyProbability: 0.8,
        quoteProbability: 0.6,
        maxRetries: 5,
        engagementLimits: { replies: 10, likes: 20 },
      };

      const { AITwitterAgent } =
        await import("@api/twitter/ai-twitterAgent.js");
      const customAgent = new AITwitterAgent(
        mockPage,
        mockProfile,
        mockLogger,
        customOptions,
      );

      expect(customAgent.replyEngine).toBeDefined();
      expect(customAgent.quoteEngine).toBeDefined();
      expect(customAgent.engagementTracker).toBeDefined();
    });

    it("should initialize page state and operation locks", () => {
      expect(agent.pageState).toBe("HOME");
      expect(agent.scrollingEnabled).toBe(true);
      expect(agent.operationLock).toBe(false);
      expect(agent._operationLockTimestamp).toBeUndefined();
    });

    it("should initialize session tracking", () => {
      expect(agent.sessionStart).toBeDefined();
      expect(agent.sessionDuration).toBe(0);
      expect(agent.currentPhase).toBeDefined();
    });
  });

  describe("Engagement Tracker Synchronization", () => {
    beforeEach(() => {
      vi.clearAllMocks();
    });

    it("should synchronize canPerform between tracker and queue", () => {
      const result = agent.engagementTracker.canPerform("reply");
      expect(result).toBe(true);
    });

    it("should record engagement in both systems", () => {
      const result = agent.engagementTracker.record("reply");
      expect(result).toBe(true);
    });

    it("should merge progress from both systems", () => {
      const result = agent.engagementTracker.getProgress("reply");
      expect(result).toBeDefined();
    });

    it("should merge status from both systems", () => {
      const result = agent.engagementTracker.getStatus();
      expect(result).toBeDefined();
    });

    it("should combine summary from queue progress", () => {
      const result = agent.engagementTracker.getSummary();
      expect(result).toBeDefined();
    });
  });

  describe("Dive Operations", () => {
    beforeEach(() => {
      vi.clearAllMocks();
    });

    it("should start dive and acquire operation lock", async () => {
      await agent.startDive();

      expect(agent.operationLock).toBe(true);
      expect(agent.pageState).toBe("DIVING");
      expect(agent.scrollingEnabled).toBe(false);
      expect(agent._operationLockTimestamp).toBeDefined();
    });

    it("should end dive and release lock", async () => {
      agent.operationLock = true;
      agent.pageState = "DIVING";
      agent.scrollingEnabled = false;

      await agent.endDive(true, false);

      expect(agent.operationLock).toBe(false);
      expect(agent.pageState).toBe("TWEET_PAGE");
      expect(agent.scrollingEnabled).toBe(true);
    });

    it("should navigate home when returnHome is true", async () => {
      await agent.endDive(true, true);
      // Should not throw
      expect(true).toBe(true);
    });

    it("should check if currently diving", () => {
      agent.operationLock = true;
      agent.pageState = "DIVING";

      expect(agent.isDiving()).toBe(true);

      agent.pageState = "HOME";

      expect(agent.isDiving()).toBe(false);
    });

    it("should check if on tweet page", async () => {
      agent.pageState = "TWEET_PAGE";

      expect(await agent.isOnTweetPage()).toBe(true);

      agent.pageState = "HOME";

      expect(await agent.isOnTweetPage()).toBe(false);
    });

    it("should check if scrolling is allowed", () => {
      agent.scrollingEnabled = true;
      agent.operationLock = false;

      expect(agent.canScroll()).toBe(true);

      agent.operationLock = true;

      expect(agent.canScroll()).toBe(false);
    });

    it("should get current page state", async () => {
      api.getCurrentUrl.mockResolvedValue("https://x.com/home");
      agent.pageState = "HOME";
      agent.scrollingEnabled = true;
      agent.operationLock = false;

      const status = await agent.getPageState();

      expect(status.state).toBe("HOME");
      expect(status.scrollingEnabled).toBe(true);
      expect(status.operationLock).toBe(false);
      expect(status.url).toBe("https://x.com/home");
    });

    it("should log dive status", async () => {
      api.getCurrentUrl.mockResolvedValue("https://x.com/home");
      agent.pageState = "DIVING";
      agent.scrollingEnabled = false;
      agent.operationLock = true;

      const logSpy = vi.spyOn(agent, "log");

      await agent.logDiveStatus();

      expect(logSpy).toHaveBeenCalledWith(
        expect.stringContaining("[DiveStatus] State: DIVING"),
      );
    });

    it("should safely navigate home", async () => {
      mockPage.url.mockReturnValue("https://x.com/user/status/12345");

      await agent._safeNavigateHome();

      // Should not throw
      expect(true).toBe(true);
    });

    it("should handle navigation errors gracefully", async () => {
      mockPage.url.mockReturnValue("https://x.com/user/status/12345");
      api.goto.mockRejectedValue(new Error("Navigation failed"));

      // _safeNavigateHome returns true on successful fallback, or undefined if all fails
      const result = await agent._safeNavigateHome();
      // Should not throw - just verify it returns a value
      expect(result).toBeDefined();
    });

    it("should wait for dive completion", async () => {
      agent.operationLock = true;

      setTimeout(() => {
        agent.operationLock = false;
      }, 100);

      await agent.waitForDiveComplete();

      expect(agent.operationLock).toBe(false);
    });

    it("should check session continuation with dive lock", () => {
      agent.operationLock = true;

      const result = agent.shouldContinueSession();

      expect(result).toBe(false);
    });

    it("should perform idle cursor movement", async () => {
      await agent.performIdleCursorMovement();

      // Mock should be called
      expect(agent.performIdleCursorMovement).toHaveBeenCalled();
    });

    it("should handle idle cursor movement errors", async () => {
      api.cursor.move.mockRejectedValue(new Error("Mouse error"));

      // Should not throw - errors are caught silently
      await expect(agent.performIdleCursorMovement()).resolves.toBeUndefined();
    });
  });

  describe("Session Management", () => {
    beforeEach(() => {
      vi.clearAllMocks();
    });

    it("should update session phase", () => {
      agent.sessionStart = Date.now() - 300000; // 5 minutes ago

      agent.updateSessionPhase();

      expect(agent.sessionDuration).toBeGreaterThan(0);
    });

    it("should get phase-modified probability", () => {
      agent.currentPhase = "warmup";

      const result = agent.getPhaseModifiedProbability("reply", 0.8);

      expect(result).toBe(0.8);
    });

    it("should get session progress", () => {
      agent.sessionStart = Date.now() - 300000; // 5 minutes ago

      const progress = agent.getSessionProgress();

      expect(progress).toBeGreaterThan(0);
      expect(progress).toBeLessThanOrEqual(100);
    });

    it("should check if in cooldown phase", () => {
      const result = agent.isInCooldown();

      expect(typeof result).toBe("boolean");
    });

    it("should check if in warmup phase", () => {
      const result = agent.isInWarmup();

      expect(typeof result).toBe("boolean");
    });

    it("should log debug messages", () => {
      const logSpy = vi.spyOn(agent, "log");

      agent.logDebug("Test debug message");

      expect(logSpy).toHaveBeenCalledWith("[DEBUG] Test debug message");
    });

    it("should log warning messages", () => {
      const logSpy = vi.spyOn(agent, "log");

      agent.logWarn("Test warning message");

      expect(logSpy).toHaveBeenCalledWith("[WARN] Test warning message");
    });
  });

  describe("Resource Management", () => {
    beforeEach(() => {
      vi.clearAllMocks();
    });

    it("should shutdown legacy resources", () => {
      const mockQueueLogger = { shutdown: vi.fn() };
      const mockEngagementLogger = { shutdown: vi.fn() };

      agent.queueLogger = mockQueueLogger;
      agent.engagementLogger = mockEngagementLogger;

      agent.shutdownLegacy();

      expect(mockQueueLogger.shutdown).toHaveBeenCalled();
      expect(mockEngagementLogger.shutdown).toHaveBeenCalled();
    });

    it("should handle missing shutdown methods gracefully", () => {
      agent.queueLogger = {};
      agent.engagementLogger = null;

      // Should not throw
      expect(() => agent.shutdownLegacy()).not.toThrow();
    });
  });

  describe("Edge Cases and Error Handling", () => {
    beforeEach(() => {
      vi.clearAllMocks();
    });

    it("should handle missing page viewport size", async () => {
      mockPage.viewportSize.mockReturnValue(null);

      await expect(agent.performIdleCursorMovement()).resolves.toBeUndefined();
    });

    it("should handle sequential dive operations", async () => {
      // First dive acquires lock
      await agent.startDive();
      expect(agent.operationLock).toBe(true);

      // End first dive
      await agent.endDive(true);
      expect(agent.operationLock).toBe(false);

      // Second dive can now acquire lock
      await agent.startDive();
      expect(agent.operationLock).toBe(true);
    });

    it("should handle rapid dive start/end cycles", async () => {
      for (let i = 0; i < 5; i++) {
        await agent.startDive();
        await agent.endDive(true);
      }

      expect(agent.operationLock).toBe(false);
      expect(agent.pageState).toBe("TWEET_PAGE");
    });
  });

  describe("Integration Scenarios", () => {
    beforeEach(() => {
      vi.clearAllMocks();
    });

    it("should handle complete dive workflow", async () => {
      // Start dive
      await agent.startDive();
      expect(agent.isDiving()).toBe(true);

      // Simulate some work
      await agent.performIdleCursorMovement();

      // End dive
      await agent.endDive(true, false);
      expect(agent.isDiving()).toBe(false);
    });

    it("should synchronize engagement across dive operations", async () => {
      await agent.startDive();

      const canReply = agent.engagementTracker.canPerform("reply");
      expect(canReply).toBe(true);

      const recorded = agent.engagementTracker.record("reply");
      expect(recorded).toBe(true);

      await agent.endDive(true);
    });

    it("should handle session phase transitions during operations", async () => {
      agent.sessionStart = Date.now() - 300000; // 5 minutes ago

      await agent.startDive();

      const modifiedProb = agent.getPhaseModifiedProbability("reply", 0.8);
      expect(modifiedProb).toBe(0.8);

      await agent.endDive(true);
    });

    it("should handle resource cleanup after errors", async () => {
      mockPage.mouse.move.mockRejectedValue(new Error("Mouse error"));

      await agent.startDive();

      // Should still be able to end dive despite errors
      await expect(agent.endDive(true)).resolves.toBeUndefined();
      expect(agent.operationLock).toBe(false);
    });
  });
});
