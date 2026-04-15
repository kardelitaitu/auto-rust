/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview AI-Enhanced Twitter Agent
 * Extends TwitterAgent with AI reply capability when diving into tweets
 * @module utils/ai-twitterAgent
 */

import { TwitterAgent } from "./twitterAgent.js";
import { AIReplyEngine } from "../agent/ai-reply-engine/index.js";
import { AIQuoteEngine } from "../agent/ai-quote-engine.js";
import { AIContextEngine } from "../agent/ai-context-engine.js";
import { microInteractions } from "../behaviors/micro-interactions.js";
import { motorControl } from "../behaviors/motor-control.js";
import AgentConnector from "../core/agent-connector.js";
import { mathUtils } from "../utils/math.js";
import { entropy } from "../utils/entropyController.js";
import { engagementLimits } from "../utils/engagement-limits.js";
import { sessionPhases } from "./session-phases.js";
import { sentimentService } from "../utils/sentiment-service.js";
import { scrollDown, scrollRandom } from "../behaviors/scroll-helper.js";
import { DiveQueue } from "../utils/async-queue.js";
import { AIReplyAction } from "../actions/ai-twitter-reply.js";
import { AIQuoteAction } from "../actions/ai-twitter-quote.js";
import { LikeAction } from "../actions/ai-twitter-like.js";
import { BookmarkAction } from "../actions/ai-twitter-bookmark.js";
import { RetweetAction } from "../actions/ai-twitter-retweet.js";
import { GoHomeAction } from "../actions/ai-twitter-go-home.js";
import { FollowAction } from "../actions/ai-twitter-follow.js";
import { ActionRunner } from "../actions/advanced-index.js";
import { TWITTER_TIMEOUTS } from "../constants/timeouts.js";
import { HumanInteraction } from "../behaviors/human-interaction.js";
import { createBufferedLogger } from "../core/logger.js";
import { api } from "../index.js";

/**
 * @deprecated Use config.getEngagementLimits() instead (supports env overrides)
 */
const DEFAULT_ENGAGEMENT_LIMITS = {
  replies: 3,
  retweets: 1,
  quotes: 1,
  likes: 5,
  follows: 2,
  bookmarks: 2,
};

/**
 * Page states for diving operation control
 */
const PAGE_STATE = {
  HOME: "HOME", // Home feed page
  DIVING: "DIVING", // Currently diving into a tweet
  TWEET_PAGE: "TWEET_PAGE", // Viewing a tweet page
  RETURNING: "RETURNING", // Returning from tweet to home
};

const createPageOps = (page) => {
  const url = () => api.getCurrentUrl();
  const urlSync = async () => (page.url ? await api.getCurrentUrl() : "");
  const goto = (targetUrl, options) => api.goto(targetUrl, options);
  const waitForURL = (matcher, options) => api.waitForURL(matcher, options);
  const waitForSelector = (selector, options = {}) => {
    if (options.state === "visible") {
      return api.waitVisible(selector, { timeout: options.timeout });
    }
    if (options.state === "hidden") {
      return api.waitHidden(selector, { timeout: options.timeout });
    }
    return page.waitForSelector
      ? page.waitForSelector(selector, options)
      : Promise.resolve();
  };

  return {
    page,
    url,
    urlSync,
    goto,
    waitForURL,
    waitForSelector,
    locator: (...args) => page.locator(...args),
    evaluate: (...args) => page.evaluate(...args),
    queryAll: (...args) => (page.$$ ? page.$$(...args) : []),
    keyboardPress: (key, options) => api.getPage().keyboard.press(key, options),
    keyboardType: (text, options) => api.type(null, text, options), // api.type handles keyboard typing if selector is null
    mouseMove: (x, y, options) => api.cursor.move({ x, y }, options),
    mouseWheel: (deltaX, deltaY) =>
      page.mouse?.wheel ? page.mouse.wheel(deltaX, deltaY) : Promise.resolve(),
    viewportSize: () => (page.viewportSize ? page.viewportSize() : null),
    click: (selector, options) => api.click(selector, options),
    type: (selector, text, options) => api.type(selector, text, options),
    wait: (duration) => api.wait(duration),
  };
};

export class AITwitterAgent extends TwitterAgent {
  constructor(page, initialProfile, logger, options = {}) {
    super(page, initialProfile, logger);

    // Store full twitter config for action handlers (separate from profile config)
    this.twitterConfig = options.config || {};
    this.pageOps = createPageOps(page);
    this.page.waitForTimeout = (duration) => api.wait(duration);

    // ================================================================
    // DIVE LOCK MECHANISM - Prevents scroller interference
    // ================================================================
    this.pageState = PAGE_STATE.HOME; // Current page state
    this.scrollingEnabled = true; // Scrolling allowed flag
    this.operationLock = false; // Operation in progress flag
    this._operationLockTimestamp = undefined; // For lock monitoring
    this.diveLockAcquired = false; // Dive operation lock
    this.isPosting = false; // Posting in progress flag (prevents popup-closer interference)
    this.homeUrl = "https://x.com/home"; // Home page URL

    // Log buffering for wait messages (prevent log spam)
    this.lastWaitLogTime = 0; // Timestamp of last wait log
    this.waitLogInterval = options.waitLogInterval ?? 10000; // Log wait messages every 10s

    // Initialize DiveQueue for race-condition-free tweet dives
    // Force sequential processing (maxConcurrent: 1) to prevent overlapping dives
    this.diveQueue = new DiveQueue({
      maxConcurrent: 1,
      maxQueueSize: 30,
      defaultTimeout: 20000,
      fallbackEngagement: false, // Disable autonomous fallbacks during AI dives
      logger: this.logger,
      replies: options.engagementLimits?.replies ?? 3,
      retweets: options.engagementLimits?.retweets ?? 1,
      quotes: options.engagementLimits?.quotes ?? 1,
      likes: options.engagementLimits?.likes ?? 5,
      follows: options.engagementLimits?.follows ?? 2,
      bookmarks: options.engagementLimits?.bookmarks ?? 2,
    });

    // Quick mode flag for timeout scenarios
    this.quickModeEnabled = false;
    this.sessionActive = false;

    // Initialize AgentConnector for AI requests
    this.agentConnector = new AgentConnector();

    // Initialize AI Reply Engine with AgentConnector
    // Values come from config/settings.json → ai-twitterActivity.js → here
    this.replyEngine = new AIReplyEngine(this.agentConnector, {
      replyProbability: options.replyProbability ?? 0.5, // Default from settings.json
      maxRetries: options.maxRetries ?? 2,
    });

    // Initialize AI Quote Engine with AgentConnector
    // Values come from config/settings.json → ai-twitterActivity.js → here
    this.quoteEngine = new AIQuoteEngine(this.agentConnector, {
      quoteProbability: options.quoteProbability ?? 0.5, // Default from settings.json
      maxRetries: options.maxRetries ?? 2,
    });

    // Initialize Enhanced Context Engine for better AI replies
    this.contextEngine = new AIContextEngine({
      maxReplies: 30,
      sentimentThreshold: 0.3,
      includeMetrics: true,
    });

    this.aiStats = {
      attempts: 0,
      replies: 0,
      skips: 0,
      safetyBlocks: 0,
      errors: 0,
    };

    // Initialize engagement limits tracker
    const customLimits = options.engagementLimits || DEFAULT_ENGAGEMENT_LIMITS;
    this.engagementTracker =
      engagementLimits.createEngagementTracker(customLimits);

    // ================================================================
    // ENGAGEMENT TRACKER SYNCHRONIZATION
    // Override engagementTracker methods to delegate to DiveQueue
    // This ensures both systems use the same counters to prevent over-engagement
    // ================================================================
    const originalCanPerform = this.engagementTracker.canPerform.bind(
      this.engagementTracker,
    );
    const originalRecord = this.engagementTracker.record.bind(
      this.engagementTracker,
    );
    const originalGetProgress = this.engagementTracker.getProgress.bind(
      this.engagementTracker,
    );
    const originalGetStatus = this.engagementTracker.getStatus.bind(
      this.engagementTracker,
    );
    const originalGetSummary = this.engagementTracker.getSummary.bind(
      this.engagementTracker,
    );

    // Override canPerform to check both trackers (conservative - requires both to allow)
    this.engagementTracker.canPerform = (action) => {
      const trackerAllows = originalCanPerform(action);
      const queueAllows = this.diveQueue.canEngage(action);
      return trackerAllows && queueAllows;
    };

    // Override record to update both systems atomically
    this.engagementTracker.record = (action) => {
      // Only record if both allow it
      if (!originalCanPerform(action) || !this.diveQueue.canEngage(action)) {
        return false;
      }

      // Record in both systems
      const trackerResult = originalRecord(action);
      const queueResult = this.diveQueue.recordEngagement(action);

      // Return true only if both succeeded
      return trackerResult && queueResult;
    };

    // Override getProgress to combine data from both
    this.engagementTracker.getProgress = (action) => {
      const trackerProgress = originalGetProgress(action);
      const queueProgress = this.diveQueue.getEngagementProgress()[action];

      // Use the more restrictive of the two
      if (queueProgress) {
        return `${queueProgress.current}/${queueProgress.limit}`;
      }
      return trackerProgress;
    };

    // Override getStatus to combine data from both
    this.engagementTracker.getStatus = () => {
      const trackerStatus = originalGetStatus();
      const queueProgress = this.diveQueue.getEngagementProgress();

      // Merge statuses, using DiveQueue data where available
      const mergedStatus = { ...trackerStatus };
      for (const [action, data] of Object.entries(queueProgress)) {
        if (mergedStatus[action]) {
          mergedStatus[action].current = data.current;
          mergedStatus[action].limit = data.limit;
          mergedStatus[action].remaining = data.remaining;
          mergedStatus[action].percentage = data.percentUsed + "%";
        }
      }
      return mergedStatus;
    };

    // Override getSummary to use DiveQueue data
    this.engagementTracker.getSummary = () => {
      const queueProgress = this.diveQueue.getEngagementProgress();
      const summary = [];
      for (const [action, data] of Object.entries(queueProgress)) {
        if (data.limit !== Infinity && data.limit > 0) {
          summary.push(`${action}: ${data.current}/${data.limit}`);
        }
      }
      return summary.join(", ") || originalGetSummary();
    };

    const microConfig = {
      highlightChance: 0.03,
      rightClickChance: 0.02,
      logoClickChance: 0.05,
      whitespaceClickChance: 0.04,
      fidgetChance: 0.08,
      fidgetInterval: { min: 15000, max: 45000 },
    };
    const persona = api.getPersona();
    const microMoveChance =
      typeof persona.microMoveChance === "number"
        ? persona.microMoveChance
        : microConfig.fidgetChance;
    const speed =
      typeof persona.speed === "number" && persona.speed > 0
        ? persona.speed
        : 1;
    const actionScale = Math.max(0.5, Math.min(1.5, microMoveChance / 0.06));
    microConfig.highlightChance = Math.min(
      0.12,
      microConfig.highlightChance * actionScale,
    );
    microConfig.rightClickChance = Math.min(
      0.08,
      microConfig.rightClickChance * actionScale,
    );
    microConfig.logoClickChance = Math.min(
      0.12,
      microConfig.logoClickChance * actionScale,
    );
    microConfig.whitespaceClickChance = Math.min(
      0.1,
      microConfig.whitespaceClickChance * actionScale,
    );
    microConfig.fidgetChance = Math.max(
      0.02,
      Math.min(0.2, microMoveChance * 1.2),
    );
    const baseInterval = Math.max(8000, Math.round(20000 / speed));
    microConfig.fidgetInterval = {
      min: Math.round(baseInterval * 0.6),
      max: Math.round(baseInterval * 1.6),
    };
    this.microHandler =
      microInteractions.createMicroInteractionHandler(microConfig);

    // Initialize Motor Control handler
    this.motorHandler = motorControl.createMotorController({
      layoutShiftThreshold: 5,
      spiralSearchAttempts: 4,
      retryDelay: 100,
      maxRetries: 3,
      targetTimeout: 5000,
    });

    // Session phase tracking
    this.sessionStart = Date.now();
    this.sessionDuration = 0;
    this.currentPhase = "warmup";
    this.lastPhaseLogged = null;

    // Track processed tweets to avoid re-diving
    this._processedTweetIds = new Set();

    // Track tweet IDs for mutual exclusion (quote vs retweet)
    this._quotedTweetIds = new Set();
    this._retweetedTweetIds = new Set();

    // Mutual exclusion config
    const engagementConfig = this.twitterConfig?.engagement || {};
    this._mutualExclusionConfig = {
      enabled: engagementConfig.mutualExclusion?.enabled ?? true,
      preventQuoteAfterRetweet:
        engagementConfig.mutualExclusion?.preventQuoteAfterRetweet ?? true,
      preventRetweetAfterQuote:
        engagementConfig.mutualExclusion?.preventRetweetAfterQuote ?? true,
    };

    // Scroll tracking for exploration (fixes: re-diving same area, insufficient scroll)
    this._lastScrollY = 0;
    this._lastScrollTime = 0;
    this._minScrollPerDive = 400; // Minimum pixels to scroll before diving
    this._scrollExplorationThreshold = 600; // Exploration scroll distance

    // Initialize Action Handlers (stateful per browser instance)
    const actionInstances = {
      reply: new AIReplyAction(this),
      quote: new AIQuoteAction(this),
      like: new LikeAction(this),
      bookmark: new BookmarkAction(this),
      retweet: new RetweetAction(this),
      follow: new FollowAction(this),
      goHome: new GoHomeAction(this),
    };

    // Initialize Action Runner for smart probability redistribution
    this.actionRunner = new ActionRunner(this, actionInstances);

    // Keep individual actions accessible
    this.actions = actionInstances;

    // Initialize HumanInteraction for fallback methods
    this.humanInteraction = new HumanInteraction(page);

    // Initialize BufferedLogger for high-frequency logs
    this.queueLogger = createBufferedLogger("QueueMonitor", {
      flushInterval: 10000, // Flush every 10 seconds
      maxBufferSize: 50, // Max 50 entries before flush
      groupByLevel: true,
    });

    this.engagementLogger = createBufferedLogger("EngagementTracker", {
      flushInterval: 30000, // Flush every 30 seconds
      maxBufferSize: 20,
      groupByLevel: true,
    });

    this.log(
      `[AITwitterAgent] Initialized (replyProbability: ${this.replyEngine.config.REPLY_PROBABILITY})`,
    );
    this.log(
      `[AITwitterAgent] Engagement limits: ${this.engagementTracker.getSummary()}`,
    );
    this.log(
      `[AITwitterAgent] Session phases: warmup(0-10%) → active(10-80%) → cooldown(80-100%)`,
    );
    this.log(
      `[DiveLock] State management initialized: HOME (scrolling enabled)`,
    );
    this.log(
      `[AITwitterAgent] Action handlers initialized: reply, quote, like, bookmark, retweet, follow, goHome`,
    );

    // ================================================================
    // SYNCHRONIZED LOCKING - Ensure base handlers respect AI dive limits
    // ================================================================
    // Override base engagement.diveTweet to delegate to our locked version
    // this ensures all calls (even from auto-scroller or sessions) use DiveQueue
    this.engagement.diveTweet = async () => {
      this.log(
        "[DiveLock] 🔒 Base dive request intercepted - synchronizing with AI DiveQueue",
      );
      return await this.diveTweet();
    };
  }

  /**
   * Override navigateHome to reset scroll tracking
   */
  async navigateHome() {
    await super.navigateHome();
    this.log("[Navigation] Resetting scroll tracking for exploration...");
    this._lastScrollY = 0;
    this._lastScrollTime = Date.now();
  }

  // ================================================================
  // DIVE LOCK MECHANISM - State Management Methods
  // ================================================================

  /**
   * Force release operation lock (primarily for testing)
   */
  clearLock() {
    this.operationLock = false;
    this._operationLockTimestamp = undefined;
    this.diveLockAcquired = false;
    this.scrollingEnabled = true;
  }

  /**
   * Start dive operation - acquires lock and disables scrolling
   * Prevents scroller interference during tweet diving
   */
  async startDive() {
    // Wait for any ongoing operations to complete
    let firstWait = true;
    while (this.operationLock) {
      const now = Date.now();

      // Only log every 10 seconds to prevent log spam
      if (firstWait || now - this.lastWaitLogTime >= this.waitLogInterval) {
        this.log(
          `[DiveLock] ⏳ Waiting for existing operation to complete... (${((now - this.lastWaitLogTime) / 1000).toFixed(0)}s since last check)`,
        );
        this.lastWaitLogTime = now;
        firstWait = false;
      }

      await new Promise((resolve) => setTimeout(resolve, 100));
    }

    // Acquire operation lock
    this.operationLock = true;
    this._operationLockTimestamp = Date.now();
    this.diveLockAcquired = true;
    this.pageState = PAGE_STATE.DIVING;
    this.scrollingEnabled = false;
    this.lastWaitLogTime = 0; // Reset wait log timestamp

    this.log(
      `[DiveLock] 🔒 Dive operation started - scrolling disabled (state: ${this.pageState})`,
    );
    return true;
  }

  /**
   * End dive operation - releases lock and optionally returns to home
   * @param {boolean} success - Whether the dive was successful
   * @param {boolean} returnHome - Whether to return to home page
   */
  async endDive(success = true, returnHome = false) {
    if (returnHome) {
      this.pageState = PAGE_STATE.RETURNING;
      this.scrollingEnabled = false;

      try {
        // Navigate back to home
        await this._safeNavigateHome();
        // Reduced wait - let simulateReading() handle actual reading delay
        await this.wait(500);

        // Verify we're on home page
        const currentUrl = await api.getCurrentUrl();
        if (currentUrl.includes("/home") || currentUrl === "https://x.com/") {
          this.log(`[DiveLock] ✓ Successfully returned to home`);
        }
      } catch (error) {
        this.log(
          `[DiveLock] Warning: Failed to return to home: ${error.message}`,
        );
      }

      // Reset to home state
      this.pageState = PAGE_STATE.HOME;
      this.scrollingEnabled = true;
    } else {
      // Just update state based on success
      this.pageState = success ? PAGE_STATE.TWEET_PAGE : PAGE_STATE.HOME;
      this.scrollingEnabled = true;
    }

    if (success) {
      await this._postDiveHomeScroll();
    }

    // Release operation lock ONLY after all sequences are finished
    this.operationLock = false;
    this.diveLockAcquired = false;
    this.isScanning = false; // Force scan release if set

    this.log(
      `[DiveLock] 🔓 Dive operation ended (success: ${success}, state: ${this.pageState})`,
    );
  }

  /**
   * Check if currently diving (operation in progress)
   */
  isDiving() {
    return this.operationLock && this.pageState === PAGE_STATE.DIVING;
  }

  /**
   * Check if currently on tweet page
   */
  async isOnTweetPage() {
    const url = await this.pageOps.urlSync();
    return url.includes("/status/") || this.pageState === PAGE_STATE.TWEET_PAGE;
  }

  /**
   * Check if scrolling is allowed
   */
  canScroll() {
    return this.scrollingEnabled && !this.operationLock;
  }

  /**
   * Get current page state
   */
  async getPageState() {
    return {
      state: this.pageState,
      scrollingEnabled: this.scrollingEnabled,
      operationLock: this.operationLock,
      url: await this.pageOps.urlSync(),
    };
  }

  async wait(duration) {
    return api.wait(duration);
  }

  async waitVisible(selector, options = {}) {
    return api.waitVisible(selector, options);
  }

  /**
   * Log current dive status
   */
  async logDiveStatus() {
    const status = await this.getPageState();
    this.log(
      `[DiveStatus] State: ${status.state}, Scrolling: ${status.scrollingEnabled}, Lock: ${status.operationLock}, URL: ${status.url}`,
    );
  }

  /**
   * Safe navigation to home page
   */
  async _safeNavigateHome() {
    try {
      const currentUrl = await this.pageOps.urlSync();
      if (currentUrl.includes("/home") || currentUrl === "https://x.com/") {
        this.log("[DiveLock] Already on home page");
        return true;
      }

      // Use keyboard navigation first (Escape to close composer if open)
      await this.pageOps.keyboardPress("Escape");
      await this.pageOps.wait(500);

      // Try to navigate to home
      await this.navigation.navigateHome();
      return true;
    } catch (error) {
      this.log(`[DiveLock] Navigation error: ${error.message}`);
      // Fallback: direct navigation
      try {
        await api.goto("https://x.com/home", {
          waitUntil: "domcontentloaded",
          timeout: TWITTER_TIMEOUTS.NAVIGATION,
        });
        return true;
      } catch (e) {
        this.log(`[DiveLock] Fallback navigation failed: ${e.message}`);
        return false;
      }
    }
  }

  /**
   * Wait for dive operation to complete
   */
  async waitForDiveComplete() {
    while (this.operationLock) {
      await new Promise((resolve) => setTimeout(resolve, 100));
    }
  }

  /**
   * Check if session should continue (respecting dive operations)
   */
  shouldContinueSession() {
    if (this.operationLock) {
      this.log("[Session] Waiting for diving operation to complete...");
      return false;
    }
    return true;
  }

  /**
   * Perform idle cursor movement when scrolling is disabled
   * Maintains human-like behavior during dive operations
   */
  async performIdleCursorMovement() {
    try {
      const viewport = this.pageOps.viewportSize() || {
        width: 1280,
        height: 720,
      };

      // Small random cursor movements for human-like behavior
      for (let i = 0; i < 3; i++) {
        const x = Math.random() * (viewport.width - 200) + 100;
        const y = Math.random() * (viewport.height - 200) + 100;
        await this.pageOps.mouseMove(x, y);
        await this.wait(Math.random() * 1000 + 500);
      }
    } catch (_error) {
      // Silent fail - cursor movement is optional
    }
  }

  /**
   * Debug logging - only logs when debug mode is enabled
   * @param {string} message - Message to log
   */
  logDebug(message) {
    this.log(`[DEBUG] ${message}`);
  }

  logWarn(message) {
    this.log(`[WARN] ${message}`);
  }

  /**
   * Update session phase based on elapsed time
   * Call this periodically during session
   */
  updateSessionPhase() {
    this.sessionDuration = Date.now() - this.sessionStart;
    const totalDuration = this.sessionEndTime
      ? this.sessionEndTime - this.sessionStart
      : this.sessionDuration * 5; // Fallback: estimate 5x current duration
    const newPhase = sessionPhases.getSessionPhase(
      this.sessionDuration,
      totalDuration,
    );

    if (newPhase !== this.currentPhase) {
      const phaseInfo = sessionPhases.getPhaseStats(newPhase);
      this.log(
        `[Phase] Transition: ${this.currentPhase} → ${newPhase} (${phaseInfo.description})`,
      );
      this.currentPhase = newPhase;
    }

    return this.currentPhase;
  }

  /**
   * Get phase-modified probability for an action
   * Applies session phase modifiers to base probabilities
   */
  getPhaseModifiedProbability(action, baseProbability) {
    this.updateSessionPhase();
    const modifier = sessionPhases.getPhaseModifier(action, this.currentPhase);
    const adjusted = baseProbability * modifier;

    if (this.currentPhase !== "active") {
      this.log(
        `[PhaseMod] ${action}: ${(baseProbability * 100).toFixed(1)}% × ${modifier.toFixed(2)} = ${(adjusted * 100).toFixed(1)}% (${this.currentPhase})`,
      );
    }

    return adjusted;
  }

  /**
   * Perform post-dive home scroll to simulate discovery
   */
  async _postDiveHomeScroll() {
    try {
      const _steps = mathUtils.randomInRange(2, 4);
      const distance = mathUtils.randomInRange(200, 500);
      await scrollDown(distance);
      await this.wait(mathUtils.randomInRange(400, 800));
    } catch (_error) {
      // ignore
    }
  }

  /**
   * Proper cleanup of all agent resources, timers, and listeners.
   */
  shutdownLegacy() {
    this.log("Agent shutting down (Legacy)...");
    try {
      if (this.diveQueue) {
        this.diveQueue.shutdown();
      }
      if (this.queueLogger) {
        this.queueLogger.shutdown();
      }
      if (this.engagementLogger) {
        this.engagementLogger.shutdown();
      }
    } catch (e) {
      this.log(`[Shutdown] Error during legacy cleanup: ${e.message}`);
    }
  }

  /**
   * Get current session progress percentage
   */
  getSessionProgress() {
    this.sessionDuration = Date.now() - this.sessionStart;
    const totalDuration = this.sessionEndTime
      ? this.sessionEndTime - this.sessionStart
      : this.sessionDuration * 5; // Fallback: estimate 5x current duration
    return Math.min(100, (this.sessionDuration / totalDuration) * 100);
  }

  /**
   * Check if we're in cooldown phase (wind-down behavior)
   */
  isInCooldown() {
    this.updateSessionPhase();
    return this.currentPhase === "cooldown";
  }

  /**
   * Check if we're in warmup phase (cautious behavior)
   */
  isInWarmup() {
    this.updateSessionPhase();
    return this.currentPhase === "warmup";
  }

  /**
   * Trigger a micro-interaction during reading/pauses
   * Adds human fidgeting behaviors
   */
  async triggerMicroInteraction(context = "reading") {
    try {
      const roll = Math.random();
      const actionThreshold =
        this.microHandler.config.highlightChance +
        this.microHandler.config.rightClickChance +
        this.microHandler.config.logoClickChance +
        this.microHandler.config.whitespaceClickChance;

      if (roll > actionThreshold) {
        this.log(`[Micro] No micro-interaction triggered (${context})`);
        return { success: false, reason: "probability_skip" };
      }

      const result = await this.microHandler.executeMicroInteraction(
        this.pageOps.page,
        {
          logger: {
            info: (msg) => this.log(msg),
            error: (msg) => this.log(`[Micro-Error] ${msg}`),
          },
        },
      );

      if (result.success) {
        this.log(`[Micro] Executed ${result.type} (${context})`);
      }

      return result;
    } catch (error) {
      this.log(`[Micro] Error during micro-interaction: ${error.message}`);
      return { success: false, reason: error.message };
    }
  }

  /**
   * Text highlighting micro-interaction
   * Simulates humans highlighting interesting text
   */
  async highlightText(selector = 'article [data-testid="tweetText"]') {
    return await this.microHandler.textHighlight(this.pageOps.page, {
      logger: {
        info: (msg) => this.log(msg),
        error: (msg) => this.log(`[Micro-Error] ${msg}`),
      },
      selector,
    });
  }

  /**
   * Start background fidget loop during long reads
   */
  startFidgetLoop() {
    return this.microHandler.startFidgetLoop(this.pageOps.page, {
      logger: {
        info: (msg) => this.log(msg),
        error: (msg) => this.log(`[Micro-Error] ${msg}`),
      },
    });
  }

  /**
   * Stop background fidget loop
   */
  stopFidgetLoop() {
    return this.microHandler.stopFidgetLoop();
  }

  /**
   * Override simulateFidget to use micro-interactions module
   * Replaces parent's TEXT_SELECT/MOUSE_WIGGLE/OVERSHOOT with:
   * - Text highlighting
   * - Random right-click
   * - Logo clicks
   * - Whitespace clicks
   */
  async simulateFidget() {
    try {
      const result = await this.microHandler.executeMicroInteraction(
        this.pageOps.page,
        {
          logger: {
            info: (msg) => this.log(msg.replace("[Micro]", "[Fidget]")),
            error: (msg) => this.log(`[Fidget-Error] ${msg}`),
          },
        },
      );

      if (!result.success) {
        // Commented out - too noisy
        // this.log(`[Fidget] Skipped: ${result.reason || 'no action'}`);
      }
    } catch (error) {
      this.log(`[Fidget] Error: ${error.message}`);
    }
  }

  /**
   * Smart Click using motor control
   * Uses continuous target tracking, overlap protection, and spiral recovery
   */
  async smartClick(context, options = {}) {
    const { verifySelector = null, timeout = 5000 } = options;

    try {
      const result = await this.motorHandler.smartClick(
        this.pageOps.page,
        null,
        {
          logger: {
            info: (msg) => this.log(msg),
            warn: (msg) => this.log(msg),
            debug: (msg) => this.log(`[Motor-Debug] ${msg}`),
          },
          context,
          verifySelector,
          timeout,
        },
      );

      if (result.success) {
        this.log(
          `[Motor] Smart click: ${context} @ (${Math.round(result.x)}, ${Math.round(result.y)}${result.recovered ? ", recovered" : ""}${result.usedFallback ? ", fallback" : ""})`,
        );
      } else {
        this.log(`[Motor] Smart click failed: ${result.reason}`);
      }

      return result;
    } catch (error) {
      this.log(`[Motor] Error: ${error.message}`);
      return { success: false, reason: error.message };
    }
  }

  /**
   * Click with smart selector fallback and verification
   */
  async smartClickElement(selector, fallbacks = [], options = {}) {
    const { verifySelector = null, verifyTimeout = 500 } = options;

    try {
      const result = await this.motorHandler.smartClick(
        this.pageOps.page,
        { primary: selector, fallbacks },
        {
          logger: {
            info: (msg) => this.log(msg),
            warn: (msg) => this.log(msg),
            debug: (msg) => this.log(`[Motor-Debug] ${msg}`),
          },
          verifySelector,
          verifyTimeout,
        },
      );

      if (result.success) {
        this.log(
          `[Motor] Clicked: ${result.selector} @ (${Math.round(result.x)}, ${Math.round(result.y)}${result.recovered ? ", recovered" : ""})`,
        );
      }

      return result;
    } catch (error) {
      this.log(`[Motor] Click error: ${error.message}`);
      return { success: false, reason: error.message };
    }
  }

  /**
   * Override diveTweet to use split-phase processing
   * Phase 1: Scan (Find & Extract) - Locked by simple boolean
   * Phase 2: Process (AI & Action) - Locked by DiveQueue (Timeout monitored)
   */
  async diveTweet() {
    const diveId = `dive_${Date.now()}_${Math.random().toString(36).substr(2, 5)}`;

    // PHASE 1: SCANNING (Quick, no heavy timeout needed)
    // --------------------------------------------------
    if (this.isScanning) {
      this.log(`[DiveLock] ⚠️ Scan already in progress, skipping ${diveId}`);
      return;
    }

    this.isScanning = true;

    try {
      this.log(`[DiveLock] 🔍 Starting SCAN phase for ${diveId}`);

      // Execute the dive logic.
      // We pass a callback that wraps ONLY the AI/Action part in the heavy DiveQueue.
      // The rest of the function (scrolling, extracting) runs immediately.
      await this._diveTweetWithAI(async (aiTask) => {
        this.log(`[DiveQueue] 🧠 Entering AI Processing Queue for ${diveId}`);
        return await this.diveQueue.addDive(
          aiTask,
          null, // No fallback for now
          {
            taskName: diveId,
            timeout: TWITTER_TIMEOUTS.DIVE_TIMEOUT, // 120s timeout applies ONLY here
            priority: 10,
          },
        );
      });
      return;
    } catch (error) {
      this.log(`[DiveLock] ❌ Error in diveTweet: ${error.message}`);
      return;
    } finally {
      this.isScanning = false;
      // Note: _diveTweetWithAI handles endDive() internally,
      // but we ensure cleanup if something crashed before that.
      if (this.diveLockAcquired) {
        await this.endDive(false, true);
      }
    }
  }

  /**
   * Internal method: diveTweet with AI reply (original logic)
   * Now includes dive locking to prevent scroller interference
   * @param {Function} queueWrapper - Optional wrapper for the AI execution phase
   */
  async _diveTweetWithAI(queueWrapper = null) {
    try {
      if (this.abortSignal?.aborted) {
        this.log("[Dive] Session aborted, skipping dive");
        return;
      }
      const page = this.pageOps?.page;
      const isClosed = page?.isClosed?.();
      const browserConnected = page?.context?.()?.browser?.()?.isConnected?.();
      if (isClosed || browserConnected === false) {
        this.log("[Dive] Session inactive, aborting dive");
        return;
      }
      // ================================================================
      // ACQUIRE DIVE LOCK - Prevents scroller interference
      // ================================================================
      await this.startDive();

      // ENSURE EXPLORATION SCROLL - Prevent re-diving same area
      await this._ensureExplorationScroll();

      // HUMAN-LIKE: Before searching for tweets, "process" the view (Thinking pause)
      if (Math.random() < 0.7) {
        this.log(
          "[Dive] Processing feed before selecting target (thinking)...",
        );
        await this.human.think("scout");
      }

      let targetTweet = null;
      let _skippedCount = 0;
      // Skip probability for top tweets to avoid deterministic "first one" selection
      const _skipTopTweets = Math.random() < 0.35;

      const tweetCandidates = [];
      const tweets = this.pageOps.locator('article[data-testid="tweet"]');
      const count = await tweets.count();

      if (count > 0) {
        for (let i = 0; i < Math.min(count, 12); i++) {
          if (this.abortSignal?.aborted) return;
          const t = tweets.nth(i);
          const box = await t.boundingBox();
          // Extended viewport range to catch more tweets
          if (box && box.height > 0 && box.y > -100 && box.y < 1200) {
            tweetCandidates.push({ locator: t, index: i, y: box.y });
          }
        }
      }

      if (tweetCandidates.length > 0) {
        // Pick randomly from candidates, but favor those slightly lower down
        // to avoid always picking the top-most one which triggers upward scrolls.
        const weights = tweetCandidates.map((c, i) => (i < 2 ? 0.3 : 1.0));
        const totalWeight = weights.reduce((a, b) => a + b, 0);
        let r = Math.random() * totalWeight;
        let selection = tweetCandidates[0];

        for (let i = 0; i < tweetCandidates.length; i++) {
          r -= weights[i];
          if (r <= 0) {
            selection = tweetCandidates[i];
            break;
          }
        }

        targetTweet = selection.locator;
        this.log(
          `[Dive] Selected tweet candidate at index ${selection.index} (Y: ${Math.round(selection.y)})`,
        );
      }

      if (!targetTweet) {
        for (let attempt = 0; attempt < 3; attempt++) {
          if (this.abortSignal?.aborted) return;

          this.log("[Dive] No suitable tweets. Scrolling...");
          // Increased scroll distance (300 -> 600) with variance for natural behavior
          const variance = 0.85 + Math.random() * 0.3;
          const scrollAmount = Math.floor(600 * variance);
          if (this.abortSignal?.aborted) return;
          await scrollDown(scrollAmount);
          await this.wait(entropy.retryDelay(attempt));

          // Update scroll tracking
          this._lastScrollY = await this.pageOps.evaluate(() => window.scrollY);
          this._lastScrollTime = Date.now();

          // Search again after scroll
          const freshTweets = this.pageOps.locator(
            'article[data-testid="tweet"]',
          );
          const freshCount = await freshTweets.count();
          if (freshCount > 0) {
            for (let i = 0; i < Math.min(freshCount, 5); i++) {
              const t = freshTweets.nth(i);
              const box = await t.boundingBox();
              if (box && box.height > 0 && box.y > 0 && box.y < 800) {
                targetTweet = t;
                break;
              }
            }
          }
          if (targetTweet) break;
        }
      }

      if (!targetTweet) {
        this.log("[Dive] No suitable tweets found. Refreshing Home...");
        await api.goto("https://x.com/");
        await this.ensureForYouTab();
        // Release lock before returning
        await this.endDive(false, true);
        return;
      }

      // Get username from tweet for URL construction
      let username = "unknown";
      try {
        const tweetTextEl = targetTweet
          .locator('[data-testid="tweetText"]')
          .first();
        if ((await tweetTextEl.count()) > 0) {
          const parent = await tweetTextEl.$x("..");
          if (parent && parent[0]) {
            const link = await parent[0].$('a[href*="/"]');
            if (link) {
              const href = await link.getAttribute("href");
              username = href?.replace(/^\/|\/$/g, "") || "unknown";
            }
          }
        }
      } catch (_error) {
        username = "unknown";
      }

      // Determine click target
      let clickTarget = null;
      const timeStamp = targetTweet.locator("time").first();
      const permalinkLink = targetTweet.locator('a[href*="/status/"]').first();
      const tweetTextEl = targetTweet
        .locator('[data-testid="tweetText"]')
        .first();

      if (
        (await permalinkLink.count()) > 0 &&
        (await api.visible(permalinkLink))
      ) {
        clickTarget = permalinkLink;
        this.log("[Debug] Targeting tweet permalink link (Primary).");
      } else if (
        (await tweetTextEl.count()) > 0 &&
        (await api.visible(tweetTextEl))
      ) {
        clickTarget = tweetTextEl;
        this.log("[Debug] Targeting tweet text body (Fallback).");
      } else if (
        (await timeStamp.count()) > 0 &&
        (await api.visible(timeStamp))
      ) {
        const parentLink = timeStamp.locator("xpath=./ancestor::a[1]");
        clickTarget = (await parentLink.count()) > 0 ? parentLink : timeStamp;
        this.log("[Debug] Targeting tweet permalink/time (Fallback).");
      } else {
        clickTarget = targetTweet;
        this.log("[Debug] Targeting entire tweet card (Last Resort).");
      }

      // Use api.click() — includes scroll-to-golden-view, stability check,
      // obstruction guard, ghost cursor, and built-in 3x retry with recovery.
      const CLICK_NAV_TIMEOUT = 3000;

      if (clickTarget) {
        this.log("[Attempt] api.click on Permalink...");
        try {
          await api.click(clickTarget, { precision: "high" });
        } catch (clickErr) {
          this.log(`[Fail] api.click threw: ${clickErr.message}`);
        }
      }

      let tweetUrl = "";
      let expanded = false;

      try {
        await this.pageOps.waitForURL("**/status/**", {
          timeout: CLICK_NAV_TIMEOUT,
        });
        tweetUrl = await api.getCurrentUrl();
        this.log("[Success] Navigated to tweet page.");
        this.log(`[Debug] Tweet URL: ${tweetUrl}`);
        expanded = true;
      } catch (_error) {
        this.log(
          "[Fail] api.click did not navigate. Retrying with NATIVE click...",
        );
        if (clickTarget) {
          await clickTarget.click({ force: true });
        }
        try {
          await this.pageOps.waitForURL("**/status/**", {
            timeout: CLICK_NAV_TIMEOUT,
          });
          tweetUrl = await api.getCurrentUrl();
          this.log("[Success] Native Click navigated to tweet.");
          expanded = true;
        } catch (_error2) {
          this.log("[Fail] Failed to expand tweet. Aborting dive.");
          await this.endDive(false, true);
          return;
        }
      }

      if (!expanded) {
        await this.endDive(false, true);
        return;
      }

      // Skip if already processed this tweet
      const tweetIdMatch = tweetUrl && tweetUrl.match(/status\/(\d+)/);
      const currentTweetId = tweetIdMatch ? tweetIdMatch[1] : null;
      if (currentTweetId) {
        if (this._processedTweetIds.has(currentTweetId)) {
          this.log(
            `[AI] Already processed tweet ${currentTweetId}, skipping...`,
          );
          await this.endDive(true, true);
          return;
        }
        this._processedTweetIds.add(currentTweetId);
        this.log(`[AI] Tracking new tweet ${currentTweetId}`);
      }

      // ================================================================
      // EXTRACT TWEET CONTENT AFTER NAVIGATION (PRIMARY SOURCE)
      // ================================================================
      this.log("[AI] Extracting tweet content from full page...");

      // Wait for tweet content to fully load
      await this.waitVisible('[data-testid="tweetText"]', {
        timeout: TWITTER_TIMEOUTS.ELEMENT_VISIBLE,
      }).catch(() => {});
      await this.wait(1000);

      // Extract FRESH tweet text AFTER navigation
      let tweetText = "";
      const freshTextEl = this.pageOps
        .locator('[data-testid="tweetText"]')
        .first();

      if ((await freshTextEl.count()) > 0) {
        tweetText = await freshTextEl.innerText().catch(() => "");
        this.log(`[AI] Extracted tweet text (${tweetText.length} chars)`);

        if (tweetText.length < 10) {
          this.log(
            "[AI] Tweet text too short, trying alternative selectors...",
          );
          // Try alternative selectors
          const alternatives = [
            '[data-testid="tweetText"]',
            "[lang] > div",
            'article [role="group"]',
            ".tweet-text",
          ];

          for (const selector of alternatives) {
            const altEl = this.pageOps.locator(selector).first();
            if ((await altEl.count()) > 0) {
              const altText = await altEl.innerText().catch(() => "");
              if (altText.length > tweetText.length) {
                tweetText = altText;
                this.log(
                  `[AI] Found better text (${altText.length} chars) with ${selector}`,
                );
              }
            }
          }
        }
      } else {
        this.log("[AI] WARNING: Could not find tweet text element!");
      }

      // Extract username from page URL
      if (username === "unknown") {
        try {
          const url = await api.getCurrentUrl();
          const match = url && url.match(/x\.com\/(\w+)\/status/);
          if (match) username = match[1];
        } catch (error) {
          this.log(`[AI] Username extraction failed: ${error.message}`);
        }
      }

      // Validate we have tweet text
      if (!tweetText || tweetText.length < 5) {
        this.log(
          "[AI] WARNING: Could not extract valid tweet text. Skipping AI reply.",
        );
        // Still read the page and return normally
        await this._readExpandedTweet();
        await this.endDive(true, true);
        return;
      }

      // ================================================================
      // AI DECISION: Use Action Runner for smart probability redistribution
      // ================================================================
      let selectedAction = null;
      let actionSuccess = false;
      let enhancedContext = {};

      // Set current tweet ID for mutual exclusion checks
      this.actionRunner.setCurrentTweetId(currentTweetId);

      // 1. Select Action (Fast, probability based) - OUTSIDE QUEUE
      // This prevents holding the queue lock just to decide what to do
      selectedAction = this.actionRunner.selectAction();

      if (selectedAction) {
        this.log(`[AI-Engage] Selected: ${selectedAction.toUpperCase()}`);

        // 2. Pre-fetch Context (Heavy scrolling) - OUTSIDE QUEUE
        // This ensures we only block the queue for actual AI generation
        // Skip if the selected action explicitly declares it doesn't need context (e.g. API macros map their own context)
        const needsContext =
          this.actions[selectedAction]?.needsContext !== false;
        if (
          (selectedAction === "reply" || selectedAction === "quote") &&
          needsContext
        ) {
          this.log(`[AI-Engage] Pre-fetching context for ${selectedAction}...`);
          try {
            enhancedContext = await this.contextEngine.extractEnhancedContext(
              this.pageOps.page,
              tweetUrl,
              tweetText,
              username,
            );
          } catch (err) {
            this.log(`[AI-Engage] Context extraction failed: ${err.message}`);
          }
        } else if (selectedAction === "follow") {
          // 3. Pre-navigate to author's profile page BEFORE entering queue
          // Profile URL is derived from tweet URL: strip /status/{id} suffix
          const profileUrl = tweetUrl?.replace(/\/status\/\d+.*/, "") || null;
          if (!profileUrl || username === "unknown") {
            this.log(
              "[AI-Engage] follow: no valid profile URL, skipping action.",
            );
            selectedAction = null;
          } else {
            this.log(
              `[AI-Engage] follow → clicking author link to navigate to: ${profileUrl}`,
            );
            try {
              // Click the author username link — href is relative on status pages: /username
              // No goto fallback (stealth: avoid direct URL navigation)
              const authorLink = this.pageOps
                .locator(`[data-testid="User-Name"] a[href="/${username}"]`)
                .first();
              await this.humanClick(authorLink, "Author profile link");
              await this.wait(mathUtils.randomInRange(1500, 3000));
              this.log(
                `[AI-Engage] follow: arrived at profile page for @${username}`,
              );
            } catch (navErr) {
              this.log(
                `[AI-Engage] follow: click failed (${navErr.message}), skipping.`,
              );
              selectedAction = null;
            }
          }
        }
      } else {
        this.log(`[AI-Engage] No action selected (all at limits or disabled)`);
      }

      // Define the AI logic as a standalone async function
      const executeAILogic = async () => {
        const action = selectedAction;
        let success = false;
        let reason;

        if (!action) {
          // Already logged above
        } else {
          const profileUrl = tweetUrl?.replace(/\/status\/\d+.*/, "") || null;
          const context = {
            tweetText,
            username,
            tweetUrl,
            profileUrl,
            tweetElement: this.pageOps
              .locator('article[data-testid="tweet"]')
              .first(),
            enhancedContext: enhancedContext, // Pass pre-fetched context
          };

          const result = await this.actionRunner.executeAction(action, context);
          success = result.success && result.executed;
          reason = result.reason;

          if (success) {
            this.log(`[AI-Engage] ✅ ${action} executed: ${reason}`);
          } else if (!result.executed) {
            this.log(`[AI-Engage] ⏭ ${action} skipped: ${reason}`);
          } else {
            this.log(`[AI-Engage] ❌ ${action} failed: ${reason}`);
          }
        }
        return { action, success };
      };

      // Execute via wrapper (Queue) or directly
      if (queueWrapper) {
        const wrapperResult = await queueWrapper(executeAILogic);
        if (wrapperResult.success) {
          selectedAction = wrapperResult.result.action;
          actionSuccess = wrapperResult.result.success;
        } else {
          this.log(`[AI-Engage] Queue/AI failed: ${wrapperResult.error}`);
          actionSuccess = false;
        }
      } else {
        // Legacy mode / Direct execution
        const res = await executeAILogic();
        selectedAction = res.action;
        actionSuccess = res.success;
      }

      // Store tweet context for Post-Dive engagement (if needed for fallbacks)
      this._lastTweetText = tweetText;
      this._lastUsername = username;
      this._lastTweetUrl = tweetUrl;

      // ================================================================
      // Read expanded tweet or navigate home immediately if successful
      // ================================================================
      if (actionSuccess) {
        this.log(
          `[Dive] Action successful, skipping reading phase and returning home...`,
        );
      } else {
        await this._readExpandedTweet();
      }

      // ================================================================
      // RELEASE DIVE LOCK AND RETURN TO HOME
      // ================================================================
      // Only navigate home if action was NOT successful
      await this.endDive(true, true);
    } catch (error) {
      this.log("[Dive] Dive sequence failed: " + error.message);
      // Release lock on error
      await this.endDive(false, true);
    }
  }

  /**
   * Quick fallback engagement when AI pipeline times out
   * Performs basic engagement without AI processing
   */
  async _quickFallbackEngagement() {
    try {
      this.log("[DiveQueue-Fallback] Executing quick engagement (no AI)...");

      const engagementRoll = Math.random();
      const engagementType =
        engagementRoll < 0.4
          ? "like"
          : engagementRoll < 0.7
            ? "bookmark"
            : "none";

      if (engagementType === "like") {
        if (this.diveQueue.canEngage("likes")) {
          await this.handleLike();
          this.diveQueue.recordEngagement("likes");
          return { engagementType: "like", success: true };
        }
      } else if (engagementType === "bookmark") {
        if (this.diveQueue.canEngage("bookmarks")) {
          await this.handleBookmark();
          this.diveQueue.recordEngagement("bookmarks");
          return { engagementType: "bookmark", success: true };
        }
      }

      this.log("[DiveQueue-Fallback] No engagement performed (limits reached)");
      return { engagementType: "none", success: true };
    } catch (_error) {
      this.log(`[DiveQueue-Fallback] Engagement failed: ${_error.message}`);
      return { engagementType: "error", success: false, error: _error.message };
    }
  }

  /**
   * Ensure sufficient exploration scroll before diving
   * Prevents re-diving into the same feed area
   */
  async _ensureExplorationScroll() {
    const currentY = await this.pageOps.evaluate(() => window.scrollY);
    const docHeight = await this.pageOps.evaluate(
      () => document.body.scrollHeight,
    );

    // HUMAN-LIKE: If we just navigated home (_lastScrollY === 0) or are at top,
    // FORCE a skim scroll to simulate "looking for content" before clicking anything.
    const isNewSession = this._lastScrollY === 0;

    const scrollDelta = currentY - this._lastScrollY;
    // If scrolled enough or near bottom, no need for extra scroll (unless new session)
    if (
      !isNewSession &&
      (scrollDelta >= this._minScrollPerDive || currentY > docHeight - 1500)
    ) {
      return true;
    }

    // If at top or new session, need to scroll significantly
    if (currentY < 100 || isNewSession) {
      const reason = isNewSession
        ? "New session/navigated home"
        : "At top of feed";
      this.log(`[Scroll] ${reason}, performing forced exploration scroll...`);
      // Scroll with variance (0.8x - 1.2x) for natural behavior
      // For new sessions, we might want a bigger scroll
      const baseScroll = isNewSession ? 800 : this._scrollExplorationThreshold;
      const variance = 0.7 + Math.random() * 0.6;
      const scrollAmount = Math.floor(baseScroll * variance);

      await scrollDown(scrollAmount);
      await this.wait(mathUtils.randomInRange(1000, 2500));
      this._lastScrollY = await this.pageOps.evaluate(() => window.scrollY);
      this._lastScrollTime = Date.now();
      return true;
    }

    return true;
  }

  /**
   * Read expanded tweet page (reading, media, replies, etc.)
   */
  async _readExpandedTweet() {
    // Text highlighting before reading
    if (Math.random() < 0.15) {
      await this.highlightText().catch(() => {});
    }

    const persona = api.getPersona();
    const hoverMin =
      persona && typeof persona.hoverMin === "number" ? persona.hoverMin : 1000;
    const hoverMax =
      persona && typeof persona.hoverMax === "number" ? persona.hoverMax : 3000;
    const readMin = Math.max(4000, Math.round(hoverMin * 2.5));
    const readMax = Math.max(readMin + 2000, Math.round(hoverMax * 4));
    const readTime = mathUtils.randomInRange(readMin, readMax);
    this.log(`[Idle] Reading expanded tweet for ${readTime}ms...`);

    // During long reads, occasionally trigger micro-interactions
    const fidgetDuringRead = readTime > 8000;
    let fidgetInterval = null;

    if (fidgetDuringRead) {
      fidgetInterval = this.startFidgetLoop();
    }

    const readStart = Date.now();
    while (Date.now() - readStart < readTime) {
      if (this.abortSignal?.aborted) break;
      const remaining = readTime - (Date.now() - readStart);
      const segment = Math.max(
        600,
        Math.min(2500, mathUtils.randomInRange(800, 2000)),
      );
      await this.wait(Math.min(segment, remaining));
      try {
        await api.maybeDistract([
          '[data-testid="sidebarColumn"] a',
          '[data-testid="trend"]',
          '[aria-label*="timeline"]',
        ]);
      } catch (error) {
        void error;
      }
    }

    if (fidgetInterval) {
      this.stopFidgetLoop();
    }

    // Optional micro-interaction after reading
    if (Math.random() < 0.2) {
      await this.triggerMicroInteraction("post_read");
    }

    // Optional media
    if (mathUtils.roll(0.2)) {
      const media = this.pageOps.locator('[data-testid="tweetPhoto"]').first();
      if ((await media.count()) > 0 && (await api.visible(media))) {
        this.log("[Action] Open media viewer");
        await this.humanClick(media, "Media Viewer", { precision: "high" });
        const viewMin = Math.max(3000, Math.round(hoverMin * 2));
        const viewMax = Math.max(viewMin + 1500, Math.round(hoverMax * 3));
        const viewTime = mathUtils.randomInRange(viewMin, viewMax);
        this.log(
          `[Media] Viewing media for ${(viewTime / 1000).toFixed(1)}s...`,
        );
        await this.wait(viewTime);
        await this.pageOps.keyboardPress("Escape", {
          delay: mathUtils.randomInRange(50, 150),
        });
        await this.wait(mathUtils.randomInRange(400, 900));
      }
    }

    // Read replies
    this.log("[Scroll] Reading replies...");
    const scrollSpeed =
      persona && typeof persona.scrollSpeed === "number"
        ? persona.scrollSpeed
        : 1;
    const replyMin = Math.max(150, Math.round(300 * scrollSpeed));
    const replyMax = Math.min(900, Math.round(600 * scrollSpeed));
    await scrollRandom(replyMin, replyMax);
    await this.wait(mathUtils.randomInRange(2000, 4000));

    const returnMin = Math.max(120, Math.round(240 * scrollSpeed));
    const returnMax = Math.min(900, Math.round(660 * scrollSpeed));
    await scrollRandom(returnMin, returnMax);
    await this.wait(mathUtils.randomInRange(1000, 2000));

    // Note: Post-Dive engagement removed to favor strictly single-action-per-dive policy

    // Clear cached tweet context
    this._lastTweetText = null;
    this._lastUsername = null;
    this._lastTweetUrl = null;

    // Idle and return home
    await this.wait(mathUtils.randomInRange(1200, 2400));
    await this.navigateHome();
    await this.wait(mathUtils.randomInRange(1500, 3000));
  }

  /**
   * Override runSession to use dive queue and respect operation lock
   * Prevents scroller interference during tweet diving operations
   */
  async runSession(
    cycles = 10,
    minDurationSec = 0,
    maxDurationSec = 0,
    options = {},
  ) {
    const abortSignal = options.abortSignal;
    this.abortSignal = abortSignal || null;
    if (api.isSessionActive && !api.isSessionActive()) {
      this.log("[AITwitterAgent] Session inactive, aborting");
      return;
    }
    if (abortSignal?.aborted) {
      this.log("[AITwitterAgent] Session aborted before start");
      return;
    }
    this.sessionActive = true;
    this.log(
      `[AITwitterAgent] Starting AI-Enhanced Session with DiveQueue and Lock Management...`,
    );
    try {
      // ================================================================
      // SETUP PHASE
      // ================================================================

      // Enable quick mode if we're in cooldown phase
      const originalDiveTweet = this.diveTweet.bind(this);

      // Wrap diveTweet to enable quick mode when needed
      this.diveTweet = async () => {
        try {
          if (this.isInCooldown() && !this.quickModeEnabled) {
            this.log("[DiveQueue] Enabling quick mode for cooldown phase");
            this.diveQueue.enableQuickMode();
            this.quickModeEnabled = true;
          }
          const result = await originalDiveTweet();
          return result;
        } catch (error) {
          this.log(`[DiveQueue] Wrapped diveTweet error: ${error.message}`);
          throw error;
        }
      };

      // Initialize session from parent
      const sessionUrl = await api.getCurrentUrl();
      this.log(`Starting Session on ${sessionUrl}`);

      if (abortSignal?.aborted) {
        this.log("[AITwitterAgent] Session aborted");
        return;
      }

      if (minDurationSec > 0 && maxDurationSec > 0) {
        const durationMs = mathUtils.randomInRange(
          minDurationSec * 1000,
          maxDurationSec * 1000,
        );
        this.sessionEndTime = Date.now() + durationMs;
        this.log(`Session Timer Set: ${(durationMs / 1000).toFixed(1)}s`);
      } else {
        this.log(`Session Mode: Fixed Cycles (${cycles})`);
      }

      // Navigate to home if not already there
      if (abortSignal?.aborted) {
        this.log("[AITwitterAgent] Session aborted");
        return;
      }

      if (!sessionUrl.includes("home")) {
        await this.navigateHome();
      }

      // Theme enforcement - apply early to prevent flashing
      const theme = this.twitterConfig.theme || "dark";
      if (abortSignal?.aborted) {
        this.log("[AITwitterAgent] Session aborted");
        return;
      }

      if (theme) {
        this.log(`Enforcing theme: ${theme}`);
        await api.emulateMedia({ colorScheme: theme });
      }

      // Human-like session warmup
      await this.human.sessionStart();
      if (abortSignal?.aborted) {
        this.log("[AITwitterAgent] Session aborted");
        return;
      }

      // Initial login check
      for (let i = 0; i < 3; i++) {
        if (abortSignal?.aborted) {
          this.log("[AITwitterAgent] Session aborted");
          return;
        }
        const isLoggedIn = await this.checkLoginState();
        if (isLoggedIn) break;

        if (i < 2) {
          const delay = entropy.retryDelay(i, 5000);
          this.log(
            `[Validation] Login check failed (${i + 1}/3). Retrying in ${(delay / 1000).toFixed(1)}s...`,
          );
          await this.wait(delay);
        }
      }

      if (this.state.consecutiveLoginFailures >= 3) {
        this.log(
          "🛑 Aborting session: Not logged in (3 consecutive failures).",
        );
        return;
      }

      // ================================================================
      // MAIN SESSION LOOP WITH OPERATION LOCK CHECKS
      // ================================================================

      while (true) {
        if (abortSignal?.aborted) {
          this.log("[AITwitterAgent] Session aborted");
          return;
        }
        // ============================================================
        // CHECK END CONDITIONS
        // ============================================================

        // Check if should end session naturally (only if no explicit end time set)
        const elapsed = Date.now() - this.sessionStart;
        if (
          !this.sessionEndTime &&
          this.human.session.shouldEndSession(elapsed)
        ) {
          this.log(`⏳ Natural session end reached. Finishing...`);
          break;
        }

        // Check session timeout
        if (this.isSessionExpired()) {
          this.log(`⏳ Session Time Limit Reached. Finishing...`);
          break;
        }

        // Check login failures
        if (this.state.consecutiveLoginFailures >= 3) {
          this.log(
            `🛑 ABORTING: Detected 'Not Logged In' state 3 times consecutively.`,
          );
          break;
        }

        // Check cycle limit
        if (!this.sessionEndTime && this.loopIndex >= cycles) {
          this.log(`Session Cycle Limit Reached (${cycles}). Finishing...`);
          break;
        }

        this.loopIndex += 1;
        this.log(
          `--- Loop ${this.loopIndex} ${this.sessionEndTime ? "" : `of ${cycles}`} ---`,
        );

        // ============================================================
        // HEALTH CHECK - Verify connection health before proceeding
        // ============================================================
        const healthCheckInterval = 10; // Check every 10 loops
        if (this.loopIndex % healthCheckInterval === 0) {
          this.log("[Health] Performing periodic health check...");
          const health = await this.performHealthCheck();

          if (!health.healthy) {
            this.logWarn(
              `[Health] Unhealthy: ${health.reason}. Attempting recovery...`,
            );
            // Attempt recovery by navigating home
            try {
              await this.navigateHome();
              await this.wait(2000);
              const retryHealth = await this.performHealthCheck();
              if (!retryHealth.healthy) {
                this.logWarn(
                  "[Health] Recovery failed, continuing with caution",
                );
              } else {
                this.log("[Health] Recovery successful");
              }
            } catch (e) {
              this.logWarn(`[Health] Recovery attempt failed: ${e.message}`);
            }
          } else {
            this.log("[Health] Connection healthy");
          }
        }

        // Boredom pause every 4th cycle
        if (this.loopIndex % 4 === 0 && mathUtils.roll(0.25)) {
          await this.human.session.boredomPause(this.pageOps.page);
        }

        // ============================================================
        // CHECK OPERATION LOCK BEFORE CONTINUING
        // ============================================================

        // Wait for diving operation to complete if lock is acquired
        let firstSessionWait = true;
        if (this.operationLock) {
          while (this.operationLock) {
            if (abortSignal?.aborted) {
              this.log("[AITwitterAgent] Session aborted");
              return;
            }
            const now = Date.now();

            // Only log every 10 seconds to prevent log spam
            if (
              firstSessionWait ||
              now - this.lastWaitLogTime >= this.waitLogInterval
            ) {
              this.log(
                `[Session] ⏳ Waiting for diving operation to complete... (${((now - this.lastWaitLogTime) / 1000).toFixed(0)}s elapsed)`,
              );
              this.lastWaitLogTime = now;
              firstSessionWait = false;
            }

            await new Promise((resolve) => setTimeout(resolve, 100));
          }
          this.log(
            "[Session] ✓ Diving operation completed, continuing session",
          );
          this.lastWaitLogTime = 0; // Reset wait log timestamp
        }

        // ============================================================
        // BURST MODE STATE MACHINE
        // ============================================================

        const now = Date.now();
        if (this.state.activityMode === "BURST") {
          if (now > this.state.burstEndTime) {
            this.state.activityMode = "NORMAL";
            this.log("📉 Burst Mode Ended. Returning to normal pace.");
          }
        } else if (this.state.activityMode === "NORMAL") {
          if (!this.isFatigued && mathUtils.roll(0.1)) {
            this.state.activityMode = "BURST";
            const duration = mathUtils.randomInRange(30000, 60000);
            this.state.burstEndTime = now + duration;
            this.log(
              `🔥 >>> ENTERING BURST MODE! High intensity for ${(duration / 1000).toFixed(1)}s`,
            );
          }
        }

        // ============================================================
        // SIMULATE READING (WITH SCROLL LOCK CHECK)
        // ============================================================

        if (this.operationLock) {
          // Skip reading during diving operations
          this.log("[Session] Skipping reading (diving operation in progress)");
        } else {
          await this.simulateReading();
          if (this.isSessionExpired()) break;
        }

        // ============================================================
        // DECISION LOGIC
        // ============================================================

        // Wait for lock again after reading with buffered logging
        let secondSessionWait = true;
        if (this.operationLock) {
          while (this.operationLock) {
            if (abortSignal?.aborted) {
              this.log("[AITwitterAgent] Session aborted");
              return;
            }
            const now = Date.now();

            // Only log every 10 seconds to prevent log spam
            if (
              secondSessionWait ||
              now - this.lastWaitLogTime >= this.waitLogInterval
            ) {
              this.log(
                `[Session] ⏳ Waiting for diving operation after reading... (${((now - this.lastWaitLogTime) / 1000).toFixed(0)}s elapsed)`,
              );
              this.lastWaitLogTime = now;
              secondSessionWait = false;
            }

            await new Promise((resolve) => setTimeout(resolve, 100));
          }
          this.lastWaitLogTime = 0; // Reset wait log timestamp
        }

        // Check end conditions again
        if (this.isSessionExpired()) break;
        if (abortSignal?.aborted) {
          this.log("[AITwitterAgent] Session aborted");
          return;
        }

        const p = this.normalizeProbabilities(this.twitterConfig.probabilities);
        const roll = Math.random();
        const refreshCutoff = p.refresh;
        const profileCutoff = refreshCutoff + p.profileDive;
        const tweetCutoff = profileCutoff + p.tweetDive;

        if (roll < refreshCutoff) {
          this.log("[Branch] Refresh Feed");
          this.state.lastRefreshAt = Date.now();
          await this.navigateHome();
          const refreshDelay = Math.max(50, mathUtils.gaussian(1500, 600));
          await api.think(refreshDelay);
        } else if (roll < profileCutoff) {
          await this.diveProfile();
        } else if (roll < tweetCutoff) {
          await this.diveTweet();
          // endDive(true, true) already handles: navigate to home, wait 500ms, release lock
          // Next loop iteration will call simulateReading() naturally — no extra call needed
        } else {
          this.log("[Branch] Idle (Staring at screen)");
          const idleCfg = this.twitterConfig.timings?.actionSpecific?.idle || {
            mean: 5000,
            deviation: 2000,
          };
          const duration = Math.max(
            1000,
            mathUtils.gaussian(idleCfg.mean, idleCfg.deviation),
          );
          await api.think(duration);
        }

        // Wind-down if under 20s remaining
        if (this.sessionEndTime && this.sessionEndTime - Date.now() < 20000) {
          this.log(
            "Winding down session... Navigating Home and idling briefly.",
          );
          await this.navigateHome();
          await this.wait(mathUtils.randomInRange(1500, 3000));
          break;
        }

        // Human-like cycle complete
        await this.human.cycleComplete();
      }

      // ================================================================
      // CLEANUP PHASE
      // ================================================================

      // Wait for any pending operations with buffered logging
      let finalCleanupWait = true;
      if (this.operationLock) {
        while (this.operationLock) {
          const now = Date.now();

          // Only log every 10 seconds to prevent log spam
          if (
            finalCleanupWait ||
            now - this.lastWaitLogTime >= this.waitLogInterval
          ) {
            this.log(
              `[Session] ⏳ Waiting for final diving operation to complete... (${((now - this.lastWaitLogTime) / 1000).toFixed(0)}s elapsed)`,
            );
            this.lastWaitLogTime = now;
            finalCleanupWait = false;
          }

          await new Promise((resolve) => setTimeout(resolve, 100));
        }
        this.lastWaitLogTime = 0; // Reset wait log timestamp
      }

      // Session wrap-up
      await this.human.sessionEnd();

      // Flush buffered logs
      await this.flushLogs();

      this.log("Session Complete.");

      // Final stats are printed by ai-twitterActivity.js to avoid duplication
    } finally {
      this.sessionActive = false;
      this.abortSignal = null;
    }
  }

  /**
   * Override simulateReading to respect scrolling lock
   * When diving operation is in progress, skip scrolling and perform idle cursor movement
   */
  async simulateReading() {
    if (this.abortSignal?.aborted) return;
    // Check if scrolling is enabled or lock is active
    if (!this.scrollingEnabled || this.operationLock || this.isScanning) {
      this.log("[Idle] ⏸ Reading delay paused: Operation lock active");

      // Perform idle cursor movement for human-like behavior
      await this.performIdleCursorMovement();
      return;
    }

    // Continue with parent's simulateReading implementation
    await super.simulateReading();
  }

  /**
   * Get dive queue status for monitoring (uses BufferedLogger)
   */
  getQueueStatus() {
    const status = this.diveQueue.getFullStatus();

    // Use buffered logger for high-frequency queue status updates
    this.queueLogger.info(
      `Queue: ${status.queueLength} queued, ${status.activeCount} active, ${status.utilization}% utilized`,
    );
    this.queueLogger.info(
      `Capacity: ${status.capacity}/${status.maxQueueSize}`,
    );

    if (status.engagementLimits) {
      const limits = status.engagementLimits;
      this.queueLogger.info(
        `Limits: likes(${limits.likes.used}/${limits.likes.limit}) ` +
          `replies(${limits.replies.used}/${limits.replies.limit}) ` +
          `quotes(${limits.quotes.used}/${limits.quotes.limit}) ` +
          `bookmarks(${limits.bookmarks.used}/${limits.bookmarks.limit})`,
      );
    }

    if (status.retryInfo && status.retryInfo.pendingRetries > 0) {
      this.queueLogger.warn(
        `${status.retryInfo.pendingRetries} retries pending`,
      );
    }

    return status;
  }

  /**
   * Check if queue is healthy
   */
  isQueueHealthy() {
    return this.diveQueue.isHealthy();
  }

  /**
   * Handle AI reply decision and execution
   * Flow: Probability check → Sentiment check → Enhanced Context → Reply
   */
  async handleAIReply(tweetText, username, options = {}) {
    const { url = "" } = options;

    // ================================================================
    // Check if this is a pre-validated reply from _diveTweetWithAI
    // If so, skip probability check and go directly to sentiment analysis
    // ================================================================
    if (options.action === "reply") {
      this.log(`[AI-Replies] Pre-validated reply - skipping probability check`);
    } else {
      // Original flow - make decision here
      this.aiStats.attempts++;
      this.log(`[AI] Analyzing tweet from @${username}...`);
    }
    this.log(`[AI] Tweet URL: ${url}`);

    // ================================================================
    // STEP 1: Sentiment analysis (skip negative content)
    // ================================================================
    this.log(`[Sentiment] Analyzing tweet sentiment...`);
    const sentimentResult = sentimentService.analyze(tweetText);

    // Log basic sentiment (backward compatible)
    this.log(
      `[SentimentGuard] ${sentimentResult.isNegative ? "🚫 NEGATIVE" : "✅ Neutral/Positive"} content (score: ${sentimentResult.score.toFixed(2)})`,
    );

    // Log advanced dimensions
    this.log(
      `[Sentiment] Dimensions - Valence: ${sentimentResult.dimensions.valence.valence.toFixed(2)}, ` +
        `Arousal: ${sentimentResult.dimensions.arousal.arousal.toFixed(2)}, ` +
        `Dominance: ${sentimentResult.dimensions.dominance.dominance.toFixed(2)}, ` +
        `Sarcasm: ${sentimentResult.dimensions.sarcasm.sarcasm.toFixed(2)}`,
    );

    // Log engagement recommendations
    if (sentimentResult.engagement.warnings.length > 0) {
      this.log(
        `[Sentiment] Warnings: ${sentimentResult.engagement.warnings.join(", ")}`,
      );
    }

    if (sentimentResult.isNegative) {
      this.log(`[AI-Replies] Skipped (negative sentiment)`);
      this.aiStats.skips++;
      return;
    }

    // Check advanced risk factors
    if (sentimentResult.composite.riskLevel === "high") {
      this.log(
        `[AI-Replies] Skipped (high risk: ${sentimentResult.composite.conversationType})`,
      );
      this.aiStats.skips++;
      return;
    }

    // ================================================================
    // STEP 3: Enhanced Context Capture (metrics, sentiment, tone)
    // ================================================================
    this.log(`[AI-Context] Extracting enhanced context...`);
    const enhancedContext = await this.contextEngine.extractEnhancedContext(
      this.pageOps.page,
      url,
      tweetText,
      username,
    );

    this.log(
      `[AI-Context] Enhanced: sentiment=${enhancedContext.sentiment?.overall}, tone=${enhancedContext.tone?.primary}, engagement=${enhancedContext.engagementLevel}, ${enhancedContext.replies.length} replies`,
    );

    // ================================================================
    // STEP 4: Generate reply with enhanced context
    // ================================================================

    // If pre-validated, skip shouldReply() call and generate reply directly
    if (options.action === "reply") {
      const decision = await this.replyEngine.generateReply(
        tweetText,
        username,
        enhancedContext,
      );

      if (decision.success && decision.reply) {
        // Check engagement limits
        const canReply =
          this.engagementTracker.canPerform("replies") &&
          this.diveQueue.canEngage("replies");
        if (!canReply) {
          this.log(`[AI-Replies] Skipped (engagement limit reached)`);
          this.aiStats.skips++;
          return;
        }
        this.log(
          `[AI-Replies] Generated reply: "${decision.reply.substring(0, 30)}..."`,
        );
        await this.executeAIReply(decision.reply);
        this.aiStats.replies++;
        return;
      } else {
        this.log(
          `[AI-Replies] Failed to generate reply: ${decision.reason || "unknown error"}`,
        );
        this.aiStats.errors++;
        return;
      }
    }

    // Original flow - use shouldReply() for decision
    const decision = await this.replyEngine.shouldReply(
      tweetText,
      username,
      enhancedContext,
    );

    // ================================================================
    // STEP 5: Execute decision
    // ================================================================
    switch (decision.decision) {
      case "reply": {
        // Check engagement limits from both systems
        const canReply =
          this.engagementTracker.canPerform("replies") &&
          this.diveQueue.canEngage("replies");
        if (!canReply) {
          this.log(
            `[AI-Replies] Skipped (engagement limit reached - tracker: ${this.engagementTracker.canPerform("replies")}, queue: ${this.diveQueue.canEngage("replies")})`,
          );
          this.aiStats.skips++;
          return;
        }
        this.log(
          `[AI-Replies] Generating reply: "${decision.reply?.substring(0, 30)}..."`,
        );
        await this.executeAIReply(decision.reply);
        this.aiStats.replies++;
        break;
      }

      case "skip": {
        this.log(`[AI-Replies] Skipped (${decision.reason})`);
        this.aiStats.skips++;
        break;
      }

      default: {
        this.log(`[AI-Replies] Skipped (no decision)`);
        this.aiStats.skips++;
        break;
      }
    }
  }

  async executeAIReply(replyText) {
    try {
      this.log("[AI] Executing reply with human-like behavior...");
      this.isPosting = true; // Prevent popup-closer interference

      // Use the new human-like reply engine
      const result = await this.replyEngine.executeReply(
        this.pageOps.page,
        replyText,
      );

      if (result.success) {
        this.log(`[AI] Reply posted successfully via ${result.method}`);
        this.state.replies++;

        // Record engagement in both systems
        if (this.engagementTracker.record("replies")) {
          const progress = this.engagementTracker.getProgress("replies");
          this.log(`[Engagement] ${progress} Replies posted`);
        }
      } else {
        this.logWarn(
          `[AI] Reply failed: ${result.reason} (method: ${result.method})`,
        );
      }

      return result.success;
    } catch (error) {
      this.log(`[AI] Failed to post reply: ${error.message}`);
      return false;
    } finally {
      this.isPosting = false; // Reset posting flag
    }
  }

  async executeAIQuote(quoteText, _tweetUrl = "") {
    try {
      this.log("[AI] Executing quote with human-like behavior...");
      this.isPosting = true; // Prevent popup-closer interference

      const result = await this.quoteEngine.executeQuote(
        this.pageOps.page,
        quoteText,
      );

      if (result.success) {
        this.log(`[AI] Quote posted successfully via ${result.method}`);
        this.state.quotes++;

        if (this.engagementTracker.record("quotes")) {
          const progress = this.engagementTracker.getProgress("quotes");
          this.log(`[Engagement] ${progress} Quotes posted`);
        }
      } else {
        this.logWarn(
          `[AI] Quote failed: ${result.reason} (method: ${result.method})`,
        );
      }

      return result.success;
    } catch (error) {
      this.log(`[AI] Failed to post quote: ${error.message}`);
      return false;
    } finally {
      this.isPosting = false; // Reset posting flag
    }
  }

  /**
   * Verify reply was posted by scanning DOM for reply content
   * @param {string} replyText - The text we attempted to post
   * @returns {Promise<boolean>} True if reply found in DOM
   */
  async verifyReplyPosted(replyText) {
    try {
      // Get current page URL to verify we're still on tweet page
      const currentUrl = await api.getCurrentUrl();
      if (!currentUrl.includes("/status/")) {
        this.log("[Verify] No longer on tweet page");
        return false;
      }

      // Wait a bit for DOM to update
      await this.wait(1000);

      // Look for the reply text in article elements (newly posted reply)
      const articles = await this.pageOps.queryAll("article");
      this.log(`[Verify] Found ${articles.length} articles on page`);

      // Get the first few words of our reply to search for
      const searchWords = replyText
        .split(" ")
        .slice(0, 5)
        .join(" ")
        .toLowerCase();
      this.log(`[Verify] Searching for: "${searchWords}"`);

      // Check each article for our reply text
      for (let i = 0; i < Math.min(articles.length, 5); i++) {
        try {
          const article = articles[i];
          const articleText = await api.text(article).catch(() => "");
          const articleTextLower = articleText.toLowerCase();

          // Check if article contains our reply text (partial match)
          if (
            articleTextLower.includes(searchWords) ||
            (articleTextLower.length > 5 &&
              searchWords.includes(articleTextLower.slice(0, 30)))
          ) {
            this.log(`[Verify] Found reply in article ${i + 1}`);
            return true;
          }
        } catch (_error) {
          // Skip failed articles
        }
      }

      // Alternative: Check if composer is closed (reply submitted)
      const composer = this.pageOps
        .locator('[data-testid="tweetText"]')
        .first();
      const composerCount = await composer.count();

      if (composerCount > 0) {
        const composerText = await composer.innerText().catch(() => "");
        // If composer is empty or cleared, reply was likely posted
        if (!composerText || composerText.trim().length === 0) {
          this.log("[Verify] Composer cleared - reply likely posted");
          return true;
        }
      }

      this.log("[Verify] Could not verify reply in DOM");
      return false;
    } catch (error) {
      this.log(`[Verify] Error checking reply: ${error.message}`);
      return false;
    }
  }

  /**
   * Ultra-human-like typing simulation
   * Mimics real human typing patterns to avoid detection
   */
  async humanTypingWithTypos(inputEl, text) {
    const chars = text.split("");
    const persona = api.getPersona();
    const speed =
      typeof persona.speed === "number" && persona.speed > 0
        ? persona.speed
        : 1;
    const typoRate =
      typeof persona.typoRate === "number" ? persona.typoRate : 0.08;
    const correctionRate =
      typeof persona.correctionRate === "number"
        ? persona.correctionRate
        : 0.85;
    const hesitation =
      typeof persona.hesitation === "number" ? persona.hesitation : 0.15;

    // Keyboard layout for proximity typos
    const keyboardLayout = {
      q: ["w", "a", "1"],
      w: ["q", "e", "a", "s", "2"],
      e: ["w", "r", "d", "s", "3", "4"],
      r: ["e", "t", "f", "g", "4", "5"],
      t: ["r", "y", "g", "h", "5", "6"],
      y: ["t", "u", "h", "j", "6", "7"],
      u: ["y", "i", "j", "k", "7", "8"],
      i: ["u", "o", "k", "l", "8", "9"],
      o: ["i", "p", "l", ";", "9", "0"],
      p: ["o", "[", "'", "0"],
      a: ["q", "w", "s", "z"],
      s: ["w", "e", "d", "x", "z", "a"],
      d: ["e", "r", "f", "c", "x", "s"],
      f: ["r", "t", "g", "v", "c", "d"],
      g: ["t", "y", "h", "b", "v", "f"],
      h: ["y", "u", "j", "n", "b", "g"],
      j: ["u", "i", "k", "m", "n", "h"],
      k: ["i", "o", "l", ",", "m", "j"],
      l: ["o", "p", ";", ",", ".", "k"],
      z: ["a", "s", "x"],
      x: ["z", "c", "d", "s"],
      c: ["x", "v", "f", "d"],
      v: ["c", "b", "g", "f"],
      b: ["v", "n", "h", "g"],
      n: ["b", "m", "j", "h"],
      m: ["n", ",", "j", "k"],
      0: ["9", "p", "o"],
      1: ["2", "q"],
      2: ["1", "3", "w", "q"],
      3: ["2", "4", "e", "w"],
      4: ["3", "5", "r", "e"],
      5: ["4", "6", "t", "r"],
      6: ["5", "7", "y", "t"],
      7: ["6", "8", "u", "y"],
      8: ["7", "9", "i", "u"],
      9: ["8", "0", "o", "i"],
      ".": [",", "l", ";", "/"],
      ",": ["m", ".", "k", "j", "n"],
      ";": ["l", "'", "p", "/"],
      "'": [";", "p", "[", "]"],
      "[": ["p", "'", "]", "\\"],
      "]": ["[", "\\"],
      "\\": ["]"],
      "-": ["0", "=", "["],
      "=": ["-", "[", "]"],
    };

    // Shift-required characters
    const shiftChars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ!@#$%^&*()_+{}|:"<>?~';

    // Track typing state
    let i = 0;
    let consecutiveErrors = 0;

    while (i < chars.length) {
      const char = chars[i];
      const isUpperCase = char >= "A" && char <= "Z";
      const isShiftChar = shiftChars.includes(char);
      const isSpace = char === " ";
      const isPunctuation = ".,!?;:;'\"-()[]{}".includes(char);
      const isNewline = char === "\n";
      const prevChar = i > 0 ? chars[i - 1] : null;
      // Context-aware typing speed
      let baseDelay;

      // Position-based speed patterns
      const positionProgress = i / chars.length;
      const charsTyped = i;
      const charsRemaining = chars.length - i;

      if (charsTyped < 3) {
        // Very slow start (finding keyboard)
        baseDelay = mathUtils.randomInRange(200, 400);
      } else if (charsTyped < 10) {
        // Warming up
        baseDelay = mathUtils.randomInRange(120, 250);
      } else if (positionProgress > 0.8) {
        // Slowing down at end
        baseDelay = mathUtils.randomInRange(100, 200);
      } else {
        // Normal typing rhythm
        baseDelay = mathUtils.randomInRange(60, 150);
      }

      // Context adjustments
      if (isUpperCase || isShiftChar) {
        // Hold shift - takes extra time
        baseDelay += mathUtils.randomInRange(50, 150);
      }

      if (isSpace) {
        // Word boundary pause
        baseDelay += mathUtils.randomInRange(80, 200);

        // Sometimes longer pause between sentences
        if (prevChar === "." || prevChar === "!" || prevChar === "?") {
          baseDelay += mathUtils.randomInRange(200, 500);
        }
      }

      if (isPunctuation && !isSpace) {
        // Punctuation pause
        baseDelay += mathUtils.randomInRange(30, 100);

        // Longer for sentence end
        if (char === "." || char === "!" || char === "?") {
          baseDelay += mathUtils.randomInRange(100, 300);
        }
      }

      if (isNewline) {
        baseDelay += mathUtils.randomInRange(200, 400);
      }

      baseDelay = Math.max(20, Math.round(baseDelay / speed));

      // ERROR SIMULATION
      // Determine if we should make an error
      let makeError = false;
      let errorType = null;

      if (charsTyped >= 3 && charsRemaining >= 2 && consecutiveErrors < 2) {
        const errorRoll = Math.random();

        // 8% base error rate
        if (errorRoll < typoRate) {
          makeError = true;

          // Error type distribution:
          // 40% - adjacent key
          // 25% - double letter
          // 20% - skipped letter
          // 10% - transposition
          // 5% - no error (just pause)
          const errorRoll2 = Math.random();
          if (errorRoll2 < 0.4) errorType = "adjacent";
          else if (errorRoll2 < 0.65) errorType = "double";
          else if (errorRoll2 < 0.85) errorType = "skip";
          else if (errorRoll2 < 0.95) errorType = "transposition";
          else errorType = "pause";

          if (Math.random() > correctionRate) {
            errorType = "pause";
          }
        }
      }

      if (makeError && errorType !== "pause") {
        switch (errorType) {
          case "adjacent": {
            const charLower = char.toLowerCase();
            const adjacent = keyboardLayout[charLower];
            if (adjacent && Math.random() < 0.7) {
              const wrongChar =
                adjacent[Math.floor(Math.random() * adjacent.length)];
              await this.pageOps.keyboardType(wrongChar, { delay: baseDelay });
              await this.wait(mathUtils.randomInRange(80, 200));
              await this.pageOps.keyboardPress("Backspace", {
                delay: mathUtils.randomInRange(40, 100),
              });
              await this.wait(mathUtils.randomInRange(30, 80));
              await this.pageOps.keyboardType(char, {
                delay: baseDelay + mathUtils.randomInRange(20, 60),
              });
              consecutiveErrors++;
            } else {
              await this.pageOps.keyboardType(char, { delay: baseDelay });
            }
            break;
          }

          case "double":
            // Type the same char twice by accident
            if (Math.random() < 0.6) {
              await this.pageOps.keyboardType(char, { delay: baseDelay });
              await this.wait(mathUtils.randomInRange(20, 60));
              await this.pageOps.keyboardType(char, { delay: baseDelay });
              await this.wait(mathUtils.randomInRange(50, 150));
              // Backspace once to fix
              await this.pageOps.keyboardPress("Backspace", {
                delay: mathUtils.randomInRange(40, 100),
              });
              consecutiveErrors++;
            } else {
              await this.pageOps.keyboardType(char, { delay: baseDelay });
            }
            break;

          case "skip": {
            if (i < chars.length - 1 && Math.random() < 0.5) {
              const nextCharTyped = chars[i + 1];
              await this.pageOps.keyboardType(nextCharTyped, {
                delay: baseDelay,
              });
              await this.wait(mathUtils.randomInRange(50, 120));
              await this.pageOps.keyboardPress("Backspace", {
                delay: mathUtils.randomInRange(40, 100),
              });
              await this.pageOps.keyboardType(char, {
                delay: baseDelay + mathUtils.randomInRange(20, 80),
              });
              consecutiveErrors++;
            } else {
              await this.pageOps.keyboardType(char, { delay: baseDelay });
            }
            break;
          }

          case "transposition":
            // Type char in wrong order (like "teh" for "the")
            // Handle naturally as skip + adjacent
            await this.pageOps.keyboardType(char, { delay: baseDelay });
            consecutiveErrors++;
            break;
        }
      } else if (errorType === "pause") {
        // Human paused while typing (thinking)
        await this.wait(mathUtils.randomInRange(300, 800));
        await this.pageOps.keyboardType(char, { delay: baseDelay });
      } else {
        // Normal typing - add slight variation
        await this.pageOps.keyboardType(char, {
          delay: baseDelay + mathUtils.randomInRange(-10, 20),
        });
        consecutiveErrors = 0;
      }

      const thinkChance = Math.min(0.12, 0.02 + hesitation * 0.2);
      if (Math.random() < thinkChance && charsTyped > 5 && charsRemaining > 5) {
        await this.wait(mathUtils.randomInRange(200, 600));
      }

      // Micro mouse movements (hand adjustment) - 5% chance
      if (Math.random() < 0.05) {
        const dx = mathUtils.randomInRange(-20, 20);
        const dy = mathUtils.randomInRange(-10, 10);
        await this.pageOps.mouseMove(dx, dy, { steps: 2 });
      }

      i++;
    }

    // Final human touch - sometimes cursor moves away at end
    await this.wait(mathUtils.randomInRange(100, 300));
  }

  /**
   * Handle fallback action when AI skips
   */
  async handleFallback(action) {
    try {
      switch (action) {
        case "bookmark": {
          // Check engagement limits from both systems
          const canBookmark =
            this.engagementTracker.canPerform("bookmarks") &&
            this.diveQueue.canEngage("bookmarks");
          if (!canBookmark) {
            this.log("[AI-Fallback] Bookmark limit reached, skipping");
            return;
          }

          const bm = this.pageOps
            .locator('button[data-testid="bookmark"]')
            .first();
          if ((await bm.count()) > 0 && (await api.visible(bm))) {
            this.log("[AI-Fallback] Bookmarking tweet");
            await this.humanClick(bm, "Bookmark", { precision: "high" });

            if (this.engagementTracker.record("bookmarks")) {
              const progress = this.engagementTracker.getProgress("bookmarks");
              this.log(`[Engagement] ${progress} Bookmarks used`);
            }

            // Also record in dive queue
            if (this.diveQueue.canEngage("bookmarks")) {
              this.diveQueue.recordEngagement("bookmarks");
              const queueProgress = this.diveQueue.getEngagementProgress();
              this.log(
                `[DiveQueue-Engagement] Bookmarks: ${queueProgress.bookmarks.current}/${queueProgress.bookmarks.limit}`,
              );
            }
          }
          break;
        }

        case "like":
          await this.handleLike();
          break;

        case "none":
        default:
          this.log("[AI-Fallback] No action taken");
          break;
      }
    } catch (error) {
      this.log(`[AI-Fallback] Error: ${error.message}`);
    }
  }

  /**
   * Handle like button - Robust implementation with smart selectors and fallbacks
   * Optionally accepts tweetText for sentiment analysis
   */
  async handleLike(tweetText = null) {
    try {
      // ================================================================
      // SENTIMENT CHECK - Skip likes on negative content
      // ================================================================
      if (tweetText) {
        const sentimentResult = sentimentService.analyze(tweetText);
        if (!sentimentResult.engagement.canLike) {
          this.log(
            `[Sentiment] 🚫 Skipping like on negative content ` +
              `(risk: ${sentimentResult.composite.riskLevel}, ` +
              `toxicity: ${sentimentResult.dimensions.toxicity.toxicity.toFixed(2)})`,
          );
          return;
        }
      }

      // Check engagement limits from both systems
      const canLike =
        this.engagementTracker.canPerform("likes") &&
        this.diveQueue.canEngage("likes");
      if (!canLike) {
        this.log("[Like] Limit reached, skipping");
        return;
      }

      // ================================================================
      // SMART SELECTOR WITH FALLBACKS (using HumanInteraction)
      // ================================================================
      const likeSelectors = [
        'button[data-testid="like"][role="button"]',
        '[data-testid="like"]',
        '[aria-label="Like"]',
        'svg[aria-label*="Like"]',
        'button:has-text("Like"):not([aria-label*="Unlike"])',
      ];

      const likeResult = await this.humanInteraction.findWithFallback(
        likeSelectors,
        {
          visible: true,
          timeout: 5000,
        },
      );

      if (!likeResult) {
        this.log("[Like] ❌ Could not find like button with any selector");
        return;
      }

      const likeButton = likeResult.element;
      const selectedSelector = likeResult.selector;

      // Check if already liked
      const unlikeSelectors = [
        'button[data-testid="unlike"][role="button"]',
        '[data-testid="unlike"]',
        '[aria-label="Unlike"]',
        '[aria-label*="Liked"]',
      ];

      const unlikeResult = await this.humanInteraction.findWithFallback(
        unlikeSelectors,
        {
          visible: true,
          timeout: 1000,
        },
      );
      if (unlikeResult) {
        this.log(
          `[Skip] Tweet is ALREADY LIKED (found: ${unlikeResult.selector})`,
        );
        return;
      }

      // ================================================================
      // PRE-CLICK VERIFICATION
      // ============================================================================================

      // Wait for element to be stable
      let stableCount = 0;
      const maxStableAttempts = 10;
      let lastBoundingBox = null;

      while (stableCount < 3 && stableCount < maxStableAttempts) {
        try {
          const bbox = await likeButton.boundingBox();
          if (bbox) {
            if (lastBoundingBox) {
              const dx = Math.abs(bbox.x - lastBoundingBox.x);
              const dy = Math.abs(bbox.y - lastBoundingBox.y);
              if (dx < 3 && dy < 3) {
                stableCount++;
              } else {
                stableCount = 0;
              }
            }
            lastBoundingBox = bbox;
          }
        } catch (error) {
          this.log(`[Like] Element stability check failed: ${error.message}`);
        }

        if (stableCount < 3) {
          await this.wait(100);
          stableCount++;
        }
      }

      // Check aria-label to confirm not already liked
      try {
        const ariaLabel = (await likeButton.getAttribute("aria-label")) || "";
        if (
          ariaLabel.toLowerCase().includes("unlike") ||
          ariaLabel.toLowerCase().includes("liked")
        ) {
          this.log(`[Skip] Tweet already liked (aria-label: ${ariaLabel})`);
          return;
        }
      } catch (error) {
        this.log(`[Like] Aria check failed: ${error.message}`);
      }

      // Verify element is actionable (not covered by overlay)
      const isActionable = await this.isElementActionable(likeButton);
      if (!isActionable) {
        this.log(
          "[Like] ⚠️ Element may be covered, trying scroll adjustment...",
        );
        try {
          await likeButton.scrollIntoViewIfNeeded();
          await this.wait(500);
        } catch (error) {
          this.log(`[Like] Scroll adjustment failed: ${error.message}`);
        }
      }

      // ================================================================
      // EXECUTE CLICK
      // ================================================================
      this.log(`[Action] ❤ Like (selector: ${selectedSelector})`);

      // Try humanClick first, then fallback to native
      try {
        await this.humanClick(likeButton, "Like Button", { precision: "high" });
      } catch (e) {
        this.log(
          `[Like] humanClick failed: ${e.message}, using native click...`,
        );
        try {
          await likeButton.click({
            timeout: TWITTER_TIMEOUTS.ELEMENT_CLICKABLE,
          });
        } catch (e2) {
          this.log(`[Like] Native click failed: ${e2.message}`);
          return;
        }
      }

      // ================================================================
      // POST-CLICK VERIFICATION & TRACKING
      // ================================================================
      await this.wait(mathUtils.randomInRange(1000, 2000));

      // Check if like was registered (button should now show "Unlike")
      let likeRegistered = false;
      for (const selector of unlikeSelectors) {
        try {
          const el = this.pageOps.locator(selector).first();
          if (await api.visible(el).catch(() => false)) {
            likeRegistered = true;
            break;
          }
        } catch (error) {
          this.log(`[Like] Post-click verification error: ${error.message}`);
        }
      }

      if (likeRegistered) {
        this.log(`[Like] ✓ Successfully liked tweet`);

        // Record engagement
        if (this.engagementTracker.record("likes")) {
          const progress = this.engagementTracker.getProgress("likes");
          this.log(`[Engagement] ${progress} Likes given`);
        }
      } else {
        this.log(`[Like] ⚠️ Like may not have registered`);
      }

      // Return to home page to continue main loop
      await this.navigateHome();
      await this.wait(mathUtils.randomInRange(1500, 3000));
    } catch (error) {
      this.log(`[Like] ❌ Error: ${error.message}`);
    }
  }

  /**
   * Handle bookmark button - Robust implementation with smart selectors and fallbacks
   */
  async handleBookmark() {
    try {
      // Check engagement limits from both systems
      const canBookmark =
        this.engagementTracker.canPerform("bookmarks") &&
        this.diveQueue.canEngage("bookmarks");
      if (!canBookmark) {
        this.log("[Bookmark] Limit reached, skipping");
        return;
      }

      // ================================================================
      // SMART SELECTOR WITH FALLBACKS (using HumanInteraction)
      // ================================================================
      const bookmarkSelectors = [
        'button[data-testid="bookmark"]',
        '[data-testid="bookmark"]',
        '[aria-label="Bookmark"]',
        'svg[aria-label*="Bookmark"]',
        'button:has-text("Bookmark")',
      ];

      const bookmarkResult = await this.humanInteraction.findWithFallback(
        bookmarkSelectors,
        {
          visible: true,
          timeout: 5000,
        },
      );

      if (!bookmarkResult) {
        this.log(
          "[Bookmark] ❌ Could not find bookmark button with any selector",
        );
        return;
      }

      const bm = bookmarkResult.element;
      const selectedSelector = bookmarkResult.selector;

      // Check if already bookmarked
      const removeBookmarkSelectors = [
        'button[data-testid="removeBookmark"]',
        '[data-testid="removeBookmark"]',
        '[aria-label="Remove Bookmark"]',
        '[aria-label="Bookmark saved"]',
      ];

      const removeResult = await this.humanInteraction.findWithFallback(
        removeBookmarkSelectors,
        { visible: true, timeout: 1000 },
      );
      if (removeResult) {
        this.log(
          `[Skip] Tweet ALREADY bookmarked (found: ${removeResult.selector})`,
        );
        return;
      }

      // ================================================================
      // PRE-CLICK VERIFICATION
      // ================================================================

      // Wait for element to be stable
      let stableCount = 0;
      const maxStableAttempts = 10;
      let lastBoundingBox = null;

      while (stableCount < 3 && stableCount < maxStableAttempts) {
        try {
          const bbox = await bm.boundingBox();
          if (bbox) {
            if (lastBoundingBox) {
              const dx = Math.abs(bbox.x - lastBoundingBox.x);
              const dy = Math.abs(bbox.y - lastBoundingBox.y);
              if (dx < 3 && dy < 3) {
                stableCount++;
              } else {
                stableCount = 0;
              }
            }
            lastBoundingBox = bbox;
          }
        } catch (error) {
          this.log(
            `[Bookmark] Element stability check failed: ${error.message}`,
          );
        }

        if (stableCount < 3) {
          await this.wait(100);
          stableCount++;
        }
      }

      // Verify element is actionable (not covered by overlay)
      const isActionable = await this.isElementActionable(bm);
      if (!isActionable) {
        this.log(
          "[Bookmark] ⚠️ Element may be covered, trying scroll adjustment...",
        );
        try {
          await bm.scrollIntoViewIfNeeded();
          await this.wait(500);
        } catch (error) {
          this.log(`[Bookmark] Scroll adjustment failed: ${error.message}`);
        }
      }

      // ================================================================
      // EXECUTE CLICK
      // ================================================================
      this.log(`[Action] 🔖 Bookmark (selector: ${selectedSelector})`);

      // Try humanClick first, then fallback to native
      try {
        await this.humanClick(bm, "Bookmark Button", { precision: "high" });
      } catch (e) {
        this.log(
          `[Bookmark] humanClick failed: ${e.message}, using native click...`,
        );
        try {
          await bm.click({ timeout: TWITTER_TIMEOUTS.ELEMENT_CLICKABLE });
        } catch (e2) {
          this.log(`[Bookmark] Native click failed: ${e2.message}`);
          return;
        }
      }

      // ================================================================
      // POST-CLICK VERIFICATION & TRACKING
      // ================================================================
      await this.wait(mathUtils.randomInRange(1000, 2000));

      // Check if bookmark was registered
      let bookmarkRegistered = false;
      for (const selector of removeBookmarkSelectors) {
        try {
          const el = this.pageOps.locator(selector).first();
          if (await api.visible(el).catch(() => false)) {
            bookmarkRegistered = true;
            break;
          }
        } catch (error) {
          this.log(
            `[Bookmark] Post-click verification error: ${error.message}`,
          );
        }
      }

      if (bookmarkRegistered) {
        this.log(`[Bookmark] ✓ Successfully bookmarked tweet`);

        if (this.engagementTracker.record("bookmarks")) {
          const progress = this.engagementTracker.getProgress("bookmarks");
          this.log(`[Engagement] ${progress} Bookmarks saved`);
        }
      } else {
        this.log(`[Bookmark] ⚠️ Bookmark may not have registered`);
      }

      // Return to home page to continue main loop
      await this.navigateHome();
      await this.wait(mathUtils.randomInRange(1500, 3000));
    } catch (error) {
      this.log(`[Bookmark] ❌ Error: ${error.message}`);
    }
  }

  /**
   * Handle AI Quote Tweet decision and execution
   * Flow: Probability check → Generate quote → Click Retweet → Select Quote → Type → Post
   */
  async handleAIQuote(tweetText, username, options = {}) {
    const { url = "" } = options;

    // ================================================================
    // Check if this is a pre-validated quote from _diveTweetWithAI
    // ================================================================
    if (options.action === "quote") {
      this.log(`[AI-Quote] Pre-validated quote - proceeding with generation`);
    } else {
      this.log(`[AI-Quote] Analyzing tweet from @${username}...`);
    }
    this.log(`[AI-Quote] Tweet URL: ${url}`);

    // ================================================================
    // NOTE: Probability check was done by the caller (handleAIEngage)
    // Proceed directly to sentiment analysis
    // ================================================================

    // ================================================================
    // STEP 1: Sentiment analysis (skip negative content)
    // ================================================================
    this.log(`[Sentiment] Analyzing tweet sentiment...`);
    const sentimentResult = sentimentService.analyze(tweetText);

    // Log basic sentiment (backward compatible)
    this.log(
      `[SentimentGuard] ${sentimentResult.isNegative ? "🚫 NEGATIVE" : "✅ Neutral/Positive"} content (score: ${sentimentResult.score.toFixed(2)})`,
    );

    // Log advanced dimensions
    this.log(
      `[Sentiment] Dimensions - Valence: ${sentimentResult.dimensions.valence.valence.toFixed(2)}, ` +
        `Arousal: ${sentimentResult.dimensions.arousal.arousal.toFixed(2)}, ` +
        `Dominance: ${sentimentResult.dimensions.dominance.dominance.toFixed(2)}, ` +
        `Sarcasm: ${sentimentResult.dimensions.sarcasm.sarcasm.toFixed(2)}`,
    );

    if (sentimentResult.isNegative) {
      this.log(`[AI-Quote] Skipped (negative sentiment)`);
      return;
    }

    // Check advanced risk factors
    if (sentimentResult.composite.riskLevel === "high") {
      this.log(
        `[AI-Quote] Skipped (high risk: ${sentimentResult.composite.conversationType})`,
      );
      return;
    }

    // ================================================================
    // STEP 2: Engagement limits check
    // ================================================================
    if (!this.engagementTracker.canPerform("quotes")) {
      this.log(`[AI-Quote] Skipped (engagement limit reached)`);
      return;
    }

    // ================================================================
    // STEP 3: Extract context (replies) for better AI quotes
    // ================================================================
    this.log(`[AI-Context] Loading replies for quote context...`);
    const enhancedContext = await this.contextEngine.extractEnhancedContext(
      this.pageOps.page,
      url,
      tweetText,
      username,
    );

    this.log(
      `[AI-Context] Enhanced: sentiment=${enhancedContext.sentiment?.overall}, tone=${enhancedContext.tone?.primary}, ${enhancedContext.replies.length} replies`,
    );

    // ================================================================
    // STEP 5: Generate AI quote
    // ================================================================
    this.log(`[AI-Quote] Generating quote tweet...`);

    const quoteResult = await this.quoteEngine.generateQuote(
      tweetText,
      username,
      {
        url,
        sentiment: sentimentResult.composite?.engagementStyle || "neutral",
        tone: sentimentResult.composite?.conversationType || "neutral",
        engagement: "low",
        replies: enhancedContext.replies,
      },
    );

    if (!quoteResult.success || !quoteResult.quote) {
      const reason = quoteResult.reason || "unknown";
      this.log(`[AI-Quote] ❌ Failed to generate quote (reason: ${reason})`);
      return;
    }

    // ================================================================
    // STEP 5: Display AI result BEFORE executing
    // ================================================================
    this.log(`[AI-Quote] AI QUOTE: "${quoteResult.quote}"`);

    // Execute quote with human-like behavior (uses 4 methods randomly)
    const result = await this.quoteEngine.executeQuote(
      this.pageOps.page,
      quoteResult.quote,
    );

    if (result.success) {
      if (this.engagementTracker.record("quotes")) {
        const progress = this.engagementTracker.getProgress("quotes");
        this.log(`[Engagement] ${progress} Quotes posted`);
      }
      this.log(
        `[AI-Quote] Quote tweet posted successfully via ${result.method}`,
      );

      // Additional verification: ensure composer is closed before proceeding
      this.log(`[AI-Quote] Verifying quote completion...`);
      let composerClosed = false;
      for (let i = 0; i < 5; i++) {
        try {
          const composerVisible = await api
            .visible(this.pageOps.locator('[data-testid="tweetTextarea_0"]'))
            .catch(() => false);
          if (!composerVisible) {
            composerClosed = true;
            this.log(`[AI-Quote] Composer verified closed`);
            break;
          }
          await this.wait(1000);
        } catch (e) {
          composerClosed = true;
          this.log(
            `[AI-Quote] Composer verification error (treating as closed): ${e.message}`,
          );
          break;
        }
      }

      if (!composerClosed) {
        this.log(
          `[AI-Quote] ⚠️ Composer may still be open, attempting to close...`,
        );
        try {
          await this.pageOps.keyboardPress("Escape");
          await this.wait(500);
        } catch (e) {
          this.log(`[AI-Quote] Composer close failed: ${e.message}`);
        }
      }
    } else {
      this.log(
        `[AI-Quote] Quote tweet failed: ${result.reason} (method: ${result.method})`,
      );
    }
  }

  /**
   * Get AI stats
   */
  getAIStats() {
    const actionStats = {};
    if (this.actions) {
      for (const [name, action] of Object.entries(this.actions)) {
        actionStats[name] = action.getStats();
      }
    }

    return {
      ...this.aiStats,
      successRate:
        this.aiStats.attempts > 0
          ? ((this.aiStats.replies / this.aiStats.attempts) * 100).toFixed(1) +
            "%"
          : "0%",
      actions: actionStats,
    };
  }

  /**
   * Get action stats
   */
  getActionStats() {
    if (this.actionRunner) {
      return this.actionRunner.getStats();
    }
    const stats = {};
    if (this.actions) {
      for (const [name, action] of Object.entries(this.actions)) {
        stats[name] = action.getStats();
      }
    }
    return stats;
  }

  /**
   * Perform health check on browser connection
   * Returns health status and attempts recovery if needed
   */
  async performHealthCheck() {
    try {
      // Get browser context
      const context = this.pageOps.page.context();
      const browser = context.browser();

      if (!browser || !browser.isConnected()) {
        this.logWarn("[Health] Browser disconnected, attempting recovery...");
        return { healthy: false, reason: "browser_disconnected" };
      }

      // Check page responsiveness
      const pageHealth = await this.pageOps
        .evaluate(() => {
          return {
            readyState: document.readyState,
            title: document.title,
            hasBody: !!document.body,
          };
        })
        .catch(() => ({
          readyState: "error",
          title: "",
          hasBody: false,
          error: "Page evaluation failed",
        }));

      if (
        pageHealth.readyState !== "complete" &&
        pageHealth.readyState !== "interactive"
      ) {
        this.logWarn("[Health] Page not fully loaded, attempting recovery...");
        return { healthy: false, reason: "page_not_ready" };
      }

      // Check if still on expected domain
      const currentUrl = await api.getCurrentUrl();
      if (
        !currentUrl.includes("x.com") &&
        !currentUrl.includes("twitter.com")
      ) {
        this.logWarn(
          `[Health] Unexpected URL: ${currentUrl}, navigating home...`,
        );
        await this.navigateHome();
        return { healthy: false, reason: "unexpected_url" };
      }

      return { healthy: true, reason: "" };
    } catch (error) {
      this.logWarn(`[Health] Health check failed: ${error.message}`);
      return { healthy: false, reason: error.message };
    }
  }

  //Get engagement stats
  getEngagementStats() {
    return {
      tracker: this.engagementTracker.getStatus(),
      summary: this.engagementTracker.getSummary(),
      usageRate: this.engagementTracker.getUsageRate(),
    };
  }
  //Log current engagement status
  logEngagementStatus() {
    const status = this.engagementTracker.getStatus();
    for (const [action, data] of Object.entries(status)) {
      const emoji =
        data.remaining === 0
          ? "🚫"
          : parseFloat(data.percentage) >= 80
            ? "⚠️"
            : "✅";
      this.engagementLogger?.info(
        `[Engagement] ${emoji} ${action}: ${data.current}/${data.limit} (${data.percentage} used)`,
      );
    }
  }

  //Flush all buffered loggers (call during cleanup)

  async flushLogs() {
    this.log("[Logger] Flushing buffered logs...");
    await Promise.all([
      this.queueLogger.shutdown(),
      this.engagementLogger.shutdown(),
    ]);
    this.log("[Logger] All buffered logs flushed");
  }
  //Finalize engagement logger entry
}
export default AITwitterAgent;
