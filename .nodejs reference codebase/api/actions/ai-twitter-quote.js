/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * AI Quote Action
 * Handles AI-generated quote tweets
 * @module utils/actions/ai-twitter-quote
 */

import { createLogger } from "../core/logger.js";

export class AIQuoteAction {
  constructor(agent, _options = {}) {
    this.agent = agent;
    this.logger = createLogger("ai-twitter-quote.js");
    this.engagementType = "quotes";

    this.stats = {
      attempts: 0,
      successes: 0,
      failures: 0,
      skipped: 0,
    };

    this.loadConfig();
  }

  loadConfig() {
    if (this.agent?.twitterConfig?.actions?.quote) {
      const actionConfig = this.agent.twitterConfig.actions.quote;
      this.probability = actionConfig.probability ?? 0.2;
      this.enabled = actionConfig.enabled !== false;
    } else {
      this.probability = 0.2;
      this.enabled = true;
    }
    this.logger.info(
      `[AIQuoteAction] Initialized (enabled: ${this.enabled}, probability: ${(this.probability * 100).toFixed(0)}%)`,
    );
  }

  async canExecute(context = {}) {
    if (!this.agent) {
      return { allowed: false, reason: "agent_not_initialized" };
    }

    if (!context.tweetText) {
      return { allowed: false, reason: "no_tweet_text" };
    }

    if (!context.username) {
      return { allowed: false, reason: "no_username" };
    }

    if (!context.tweetUrl) {
      return { allowed: false, reason: "no_tweet_url" };
    }

    if (this.agent.diveQueue && !this.agent.diveQueue.canEngage("quotes")) {
      return { allowed: false, reason: "engagement_limit_reached" };
    }

    if (this.enabled === false) {
      return { allowed: false, reason: "action_disabled" };
    }

    return { allowed: true, reason: null };
  }

  async execute(context = {}) {
    this.stats.attempts++;

    const { tweetText, username, tweetUrl, tweetElement } = context;

    this.logger.info(`[AIQuoteAction] Executing quote for @${username}`);

    try {
      if (tweetElement && this.agent?.scrollToGoldenZone) {
        try {
          await this.agent.scrollToGoldenZone(tweetElement);
        } catch (_error) {
          void _error;
        }
      }
      // STEP 1: Extract enhanced context (scroll down to read replies)
      let enhancedContext = context.enhancedContext;

      if (!enhancedContext || Object.keys(enhancedContext).length === 0) {
        this.logger.info(`[AIQuoteAction] Loading replies for context...`);
        enhancedContext = await this.agent.contextEngine.extractEnhancedContext(
          this.agent.page,
          tweetUrl,
          tweetText,
          username,
        );
      } else {
        this.logger.info(`[AIQuoteAction] Using pre-calculated context`);
      }

      this.logger.info(
        `[AIQuoteAction] Context: ${enhancedContext.replies?.length || 0} replies, sentiment: ${enhancedContext.sentiment?.overall || "unknown"}`,
      );

      // STEP 2: Generate quote with context
      const result = await this.agent.quoteEngine.generateQuote(
        tweetText,
        username,
        enhancedContext,
      );

      if (result.success && result.quote) {
        const posted = await this.agent.executeAIQuote(result.quote, tweetUrl);

        if (posted) {
          this.stats.successes++;
          // Record engagement so DiveQueue enforces the limit on next attempt
          this.agent.diveQueue?.recordEngagement("quotes");

          // Track tweet ID for mutual exclusion
          const tweetIdMatch = tweetUrl && tweetUrl.match(/status\/(\d+)/);
          if (tweetIdMatch && this.agent._quotedTweetIds) {
            this.agent._quotedTweetIds.add(tweetIdMatch[1]);
            this.logger.info(
              `[AIQuoteAction] Tracked tweet ${tweetIdMatch[1]} for mutual exclusion`,
            );
          }

          this.logger.info(
            `[AIQuoteAction] ✅ Quote posted: "${result.quote.substring(0, 30)}..."`,
          );

          return {
            success: true,
            executed: true,
            reason: "success",
            newEngagement: true,
            data: {
              quote: result.quote,
              username,
              tweetUrl,
            },
            engagementType: this.engagementType,
          };
        } else {
          this.stats.failures++;
          this.logger.warn(
            `[AIQuoteAction] ❌ Failed to physically post quote to page`,
          );
          return {
            success: false,
            executed: true,
            reason: "ui_post_failed",
            newEngagement: false,
            data: { error: "Failed to post quote in UI" },
            engagementType: this.engagementType,
          };
        }
      } else {
        this.stats.failures++;
        const reason = result.reason || "ai_generation_failed";

        this.logger.warn(`[AIQuoteAction] ❌ Failed: ${reason}`);

        return {
          success: false,
          executed: true,
          reason,
          newEngagement: false,
          data: { error: result.reason },
          engagementType: this.engagementType,
        };
      }
    } catch (error) {
      this.stats.failures++;
      this.logger.error(`[AIQuoteAction] Exception: ${error.message}`);

      return {
        success: false,
        executed: true,
        reason: "exception",
        newEngagement: false,
        data: { error: error.message },
        engagementType: this.engagementType,
      };
    }
  }

  async tryExecute(context = {}) {
    const can = await this.canExecute(context);
    if (!can.allowed) {
      this.stats.skipped++;
      return {
        success: false,
        executed: false,
        reason: can.reason,
        newEngagement: false,
        engagementType: this.engagementType,
      };
    }

    if (Math.random() > this.probability) {
      this.stats.skipped++;
      return {
        success: false,
        executed: false,
        reason: "probability",
        newEngagement: false,
        engagementType: this.engagementType,
      };
    }

    return await this.execute(context);
  }

  getStats() {
    const total = this.stats.attempts;
    const successRate =
      total > 0 ? ((this.stats.successes / total) * 100).toFixed(1) : "0.0";

    return {
      attempts: this.stats.attempts,
      successes: this.stats.successes,
      failures: this.stats.failures,
      skipped: this.stats.skipped,
      successRate: `${successRate}%`,
      engagementType: this.engagementType,
    };
  }

  resetStats() {
    this.stats = {
      attempts: 0,
      successes: 0,
      failures: 0,
      skipped: 0,
    };
  }
}
