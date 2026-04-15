/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { api } from "../index.js";
/**
 * Retweet Action
 * Handles pure retweeting (no quote) of tweets
 * @module utils/actions/ai-twitter-retweet
 */

import { createLogger } from "../core/logger.js";
import { mathUtils } from "../utils/math.js";

/**
 * RetweetAction - Handles retweet operations
 */
export class RetweetAction {
  /**
   * Creates a new RetweetAction instance
   * @param {object} agent - Agent instance
   * @param {object} options - Configuration options
   */
  constructor(agent, _options = {}) {
    this.agent = agent;
    this.logger = createLogger("ai-twitter-retweet.js");
    this.engagementType = "retweets";

    this.stats = {
      attempts: 0,
      successes: 0,
      failures: 0,
      skipped: 0,
    };

    this.loadConfig();
  }

  loadConfig() {
    if (this.agent?.twitterConfig?.actions?.retweet) {
      const actionConfig = this.agent.twitterConfig.actions.retweet;
      this.probability = actionConfig.probability ?? 0.2;
      this.enabled = actionConfig.enabled !== false;
    } else {
      this.probability = 0.2;
      this.enabled = true;
    }
    this.logger.info(
      `[RetweetAction] Initialized (enabled: ${this.enabled}, probability: ${(this.probability * 100).toFixed(0)}%)`,
    );
  }

  async canExecute(_context = {}) {
    if (!this.agent) {
      return { allowed: false, reason: "agent_not_initialized" };
    }

    if (this.agent.diveQueue && !this.agent.diveQueue.canEngage("retweets")) {
      return { allowed: false, reason: "engagement_limit_reached" };
    }

    return { allowed: true, reason: null };
  }

  async execute(context = {}) {
    this.stats.attempts++;

    const { tweetElement, tweetUrl } = context;

    this.logger.info(`[RetweetAction] Executing retweet`);

    if (!tweetElement) {
      this.logger.warn(`[RetweetAction] Missing tweetElement in context`);
      return {
        success: false,
        executed: false,
        reason: "missing_element",
        newEngagement: false,
        data: { tweetUrl },
        engagementType: this.engagementType,
      };
    }

    try {
      const result = await this.handleRetweet(tweetElement);

      if (result.success) {
        this.stats.successes++;
        this.logger.info(`[RetweetAction] ✅ Retweet posted`);

        return {
          success: true,
          executed: true,
          reason: "success",
          newEngagement: result.newEngagement ?? true,
          data: { tweetUrl },
          engagementType: this.engagementType,
        };
      } else {
        this.stats.failures++;
        this.logger.warn(`[RetweetAction] Failed: ${result.reason}`);

        return {
          success: false,
          executed: true,
          reason: result.reason,
          newEngagement: false,
          data: { tweetUrl },
          engagementType: this.engagementType,
        };
      }
    } catch (error) {
      this.stats.failures++;
      this.logger.error(`[RetweetAction] Exception: ${error.message}`);

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

  /**
   * Selects a strategy for retweeting
   * @returns {'keyboard' | 'click'}
   */
  selectStrategy() {
    const configStrategy =
      this.agent?.twitterConfig?.actions?.retweet?.strategy;
    if (configStrategy === "keyboard" || configStrategy === "click") {
      return configStrategy;
    }
    return Math.random() > 0.5 ? "keyboard" : "click";
  }

  async waitRandomDelay(min, max, mean, dev) {
    const delay =
      mean !== undefined && dev !== undefined
        ? mathUtils.gaussian(mean, dev, min, max)
        : mathUtils.randomInRange(min, max);
    await api.wait(1000);
    return delay;
  }

  /**
   * Handle retweet action on a tweet element
   * @param {Object} tweetElement - Playwright element handle for the tweet
   * @returns {Promise<{success: boolean, reason: string}>}
   */
  async handleRetweet(tweetElement) {
    // --- Pre-check: Is it already retweeted? ---
    try {
      if (this.agent.scrollToGoldenZone) {
        await this.agent.scrollToGoldenZone(tweetElement);
      } else {
        await tweetElement.scrollIntoViewIfNeeded();
      }
      await this.waitRandomDelay(250, 650, 420, 140);

      const unretweetBtnSelector = '[data-testid="unretweet"]';
      const unretweetBtn = tweetElement.locator(unretweetBtnSelector).first();

      // Check 1: unretweet button
      if (await api.visible(unretweetBtn).catch(() => false)) {
        this.logger.info(
          "[RetweetAction] Checker: Tweet is already retweeted (unretweet button visible)",
        );
        return {
          success: true,
          reason: "already_retweeted",
          newEngagement: false,
        };
      }

      // Check 2: Reposted label
      const repostedLabel = tweetElement
        .locator('[aria-label*="Reposted"]')
        .first();
      if (await api.visible(repostedLabel).catch(() => false)) {
        this.logger.info(
          "[RetweetAction] Checker: Tweet is already retweeted (aria-label match)",
        );
        return {
          success: true,
          reason: "already_retweeted",
          newEngagement: false,
        };
      }
    } catch (err) {
      this.logger.warn(`[RetweetAction] Pre-check failed: ${err.message}`);
    }

    // --- Execute Strategy ---
    const strategy = this.selectStrategy();
    this.logger.info(`[RetweetAction] Selected Strategy: ${strategy}`);

    if (strategy === "keyboard") {
      return await this.executeKeyboardStrategy(tweetElement);
    } else {
      return await this.executeClickStrategy(tweetElement);
    }
  }

  /**
   * Strategy 1: Keyboard Shortcut
   * Step 1: Scroll to golden view (target element)
   * Step 2: Click element
   * Step 3: Keyboard 'T' + Enter
   */
  async executeKeyboardStrategy(tweetElement) {
    const page = this.agent.page;
    try {
      // --- Step 1: Scroll to Golden Zone ---
      const specificSelector =
        '[data-testid="retweet"] [data-testid="app-text-transition-container"]';
      let target = tweetElement.locator(specificSelector).first();

      if ((await target.count()) === 0) {
        // Fallback to retweet button itself
        target = tweetElement.locator('[data-testid="retweet"]').first();
        if ((await target.count()) === 0) {
          // Fallback to tweet element
          target = tweetElement;
        }
      }

      if (this.agent.scrollToGoldenZone) {
        await this.agent.scrollToGoldenZone(target);
      } else {
        await target.scrollIntoViewIfNeeded();
      }
      await this.waitRandomDelay(300, 900, 520, 180);

      // --- Step 2: Click to focus ---
      this.logger.info(
        `[RetweetAction] Keyboard Strategy: Clicking element to focus`,
      );
      if (this.agent.humanClick) {
        await this.agent.humanClick(target, "Focus Element", {
          precision: "high",
        });
      } else {
        await target.click();
      }

      await this.waitRandomDelay(450, 1200, 760, 220);

      // --- Step 3: Keyboard 'T' ---
      this.logger.info(`[RetweetAction] Keyboard Strategy: Pressing 'T'`);
      await page.keyboard.press("t");

      await this.waitRandomDelay(420, 1100, 720, 210);

      // --- Step 4: Confirm (Enter) ---
      this.logger.info(
        `[RetweetAction] Keyboard Strategy: Pressing 'Enter' to confirm`,
      );
      await page.keyboard.press("Enter");
      await this.waitRandomDelay(320, 900, 540, 170);

      // --- Step 5: Verify ---
      const unretweetBtnSelector = '[data-testid="unretweet"]';
      try {
        await tweetElement
          .locator(unretweetBtnSelector)
          .first()
          .waitFor({ state: "visible", timeout: 5000 });

        if (this.agent.diveQueue) {
          this.agent.diveQueue.recordEngagement("retweets");
        }

        // Track tweet ID for mutual exclusion
        const tweetUrl = await this.agent.pageOps.urlSync();
        const tweetIdMatch = tweetUrl && tweetUrl.match(/status\/(\d+)/);
        if (tweetIdMatch && this.agent._retweetedTweetIds) {
          this.agent._retweetedTweetIds.add(tweetIdMatch[1]);
          this.logger.info(
            `[RetweetAction] Tracked tweet ${tweetIdMatch[1]} for mutual exclusion`,
          );
        }

        return {
          success: true,
          reason: "retweet_keyboard_success",
          newEngagement: true,
        };
      } catch (e) {
        this.logger.warn(
          `[RetweetAction] Keyboard verification failed: ${e.message}`,
        );
        return {
          success: false,
          reason: "retweet_verification_failed",
          newEngagement: false,
        };
      }
    } catch (error) {
      this.logger.error(
        `[RetweetAction] Keyboard Strategy Error: ${error.message}`,
      );
      return {
        success: false,
        reason: `keyboard_error: ${error.message}`,
        newEngagement: false,
      };
    }
  }

  /**
   * Strategy 2: Click (Legacy)
   * Original implementation using humanClick on buttons
   */
  async executeClickStrategy(tweetElement) {
    const page = this.agent.page;
    try {
      // Locate Button
      const retweetBtnSelector = '[data-testid="retweet"]';
      let retweetBtn = tweetElement.locator(retweetBtnSelector).first();

      if ((await retweetBtn.count()) === 0) {
        const ariaRepost = tweetElement
          .locator('[aria-label*="Repost"], [aria-label*="Retweet"]')
          .first();
        if (await api.visible(ariaRepost)) {
          this.logger.info(
            "[RetweetAction] Found button via aria-label fallback",
          );
          retweetBtn = ariaRepost;
        } else {
          return {
            success: false,
            reason: "retweet_button_not_found",
            newEngagement: false,
          };
        }
      }

      // Click Retweet Button
      if (this.agent.scrollToGoldenZone) {
        try {
          await this.agent.scrollToGoldenZone(retweetBtn);
        } catch (_error) {
          void _error;
        }
      }
      await this.agent.humanClick(retweetBtn, "Retweet/Repost Button", {
        precision: "high",
      });
      this.logger.info("[RetweetAction] Clicked retweet button");

      await this.waitRandomDelay(600, 1400, 920, 240);

      // Click Confirm
      const retweetConfirmSelector = '[data-testid="retweetConfirm"]';
      try {
        const confirmBtn = page.locator(retweetConfirmSelector).first();
        await confirmBtn.waitFor({ state: "visible", timeout: 3000 });

        if (this.agent.scrollToGoldenZone) {
          try {
            await this.agent.scrollToGoldenZone(confirmBtn);
          } catch (_error) {
            void _error;
          }
        }
        await this.agent.humanClick(confirmBtn, "Retweet Confirm", {
          precision: "high",
        });
        this.logger.info("[RetweetAction] Confirmed retweet via menu");
        await this.waitRandomDelay(350, 900, 560, 160);
      } catch (menuError) {
        this.logger.warn(
          `[RetweetAction] Retweet menu did not appear: ${menuError.message}`,
        );
        await page.keyboard.press("Escape");
        return {
          success: false,
          reason: "retweet_menu_failed",
          newEngagement: false,
        };
      }

      // Verification
      const unretweetBtnSelector = '[data-testid="unretweet"]';
      try {
        await tweetElement
          .locator(unretweetBtnSelector)
          .first()
          .waitFor({ state: "visible", timeout: 5000 });

        if (this.agent.diveQueue) {
          this.agent.diveQueue.recordEngagement("retweets");
        }

        // Track tweet ID for mutual exclusion
        const currentUrl = await api.getCurrentUrl();
        const tweetIdMatch = currentUrl && currentUrl.match(/status\/(\d+)/);
        if (tweetIdMatch && this.agent._retweetedTweetIds) {
          this.agent._retweetedTweetIds.add(tweetIdMatch[1]);
          this.logger.info(
            `[RetweetAction] Tracked tweet ${tweetIdMatch[1]} for mutual exclusion`,
          );
        }

        return {
          success: true,
          reason: "retweet_successful",
          newEngagement: true,
        };
      } catch (_verifyError) {
        this.logger.warn("[RetweetAction] Verification failed");
        return {
          success: false,
          reason: "retweet_verification_failed",
          newEngagement: false,
        };
      }
    } catch (error) {
      this.logger.error(
        `[RetweetAction] Click Strategy Error: ${error.message}`,
      );
      return {
        success: false,
        reason: `click_error: ${error.message}`,
        newEngagement: false,
      };
    }
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
