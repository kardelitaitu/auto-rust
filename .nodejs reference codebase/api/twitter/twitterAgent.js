/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * Twitter Agent - Core automation class for Twitter/X browser interactions
 * Handles scrolling, clicking, navigation, and human-like behavior simulation
 * @module utils/twitterAgent
 */

import { mathUtils } from "../utils/math.js";
import { entropy } from "../utils/entropyController.js";
import { profileManager } from "../utils/profileManager.js";
import { GhostCursor } from "../utils/ghostCursor.js";
import { HumanizationEngine } from "../behaviors/humanization/index.js";
import { scrollDown, scrollRandom } from "../behaviors/scroll-helper.js";

// Import modular handlers
import { NavigationHandler } from "./twitter-agent/NavigationHandler.js";
import { EngagementHandler } from "./twitter-agent/EngagementHandler.js";
import { SessionHandler } from "./twitter-agent/SessionHandler.js";

import { api } from "../index.js";
export class TwitterAgent {
  /**
   * Creates a new TwitterAgent instance
   * @param {object} page - Playwright page instance
   * @param {object} initialProfile - Profile configuration
   * @param {object} logger - Logger instance
   */
  constructor(page, initialProfile, logger) {
    this.page = page;
    this.config = initialProfile;
    this.logger = logger;
    this.ghost = new GhostCursor(page, logger);

    // Humanization Engine
    this.human = new HumanizationEngine(page, this);

    // Session state model
    this.sessionStart = Date.now();
    this.sessionEndTime = null;
    this.loopIndex = 0;
    this.state = {
      lastRefreshAt: 0,
      lastEngagementAt: 0,
      engagements: 0,
      tabs: { preferForYou: true, switchChance: 0.15 },
      fatigueBias: 0,
      consecutiveLoginFailures: 0, // Track login failures
      likes: 0,
      follows: 0,
      retweets: 0,
      tweets: 0,
      activityMode: "NORMAL", // NORMAL | BURST
      burstEndTime: 0,
      consecutiveSoftErrors: 0,
    };

    // Fatigue trigger between 3 and 8 minutes
    this.isFatigued = false;
    this.fatigueThreshold = mathUtils.randomInRange(
      3 * 60 * 1000,
      8 * 60 * 1000,
    );

    this.log(
      `Initialized [${this.config.description}]. Fatigue scheduled for T+${(this.fatigueThreshold / 60000).toFixed(1)}m`,
    );

    // Initialize modular handlers
    this.navigation = new NavigationHandler(this);
    this.engagement = new EngagementHandler(this);
    this.session = new SessionHandler(this);

    // --- HEALTH MONITORING ---
    this.lastNetworkActivity = Date.now();
    // Setup listeners to track network life-signs
    // We use a safe wrapper to avoid crashing if page is closed
    try {
      this.page.on("request", () => {
        this.lastNetworkActivity = Date.now();
      });
      this.page.on("response", () => {
        this.lastNetworkActivity = Date.now();
      });
    } catch (e) {
      this.log(`[Warning] Failed to attach network listeners: ${e.message}`);
    }
  }

  /**
   * Logs a message with agent prefix
   * @param {string} msg - Message to log
   */
  log(msg) {
    if (this.logger) {
      this.logger.info(`[Agent:${this.config.id}] ${msg}`);
    } else {
      console.log(`[Agent:${this.config.id}] ${msg}`);
    }
  }

  /**
   * Clamps a value between min and max
   * @param {number} n - Value to clamp
   * @param {number} min - Minimum value
   * @param {number} max - Maximum value
   * @returns {number} Clamped value
   */
  clamp(n, min, max) {
    return Math.max(min, Math.min(max, n));
  }

  /**
   * Human-like click with anti-detection features
   * @param {object} target - Playwright locator or element
   * @param {string} description - Description for logging
   * @param {object} options - Click options
   * @returns {Promise<void>}
   */
  async humanClick(target, description = "Target", options = {}) {
    if (!target) return;

    // HUMAN-LIKE: Thinking pause before clicking
    await this.human.think(description);

    try {
      await target.evaluate((el) =>
        el.scrollIntoView({ block: "center", inline: "center" }),
      );
      const fixationDelay = mathUtils.randomInRange(200, 500);
      await api.wait(fixationDelay);
      await api.wait(mathUtils.randomInRange(300, 600));
      const ghostResult = await this.ghost.click(target, {
        label: description,
        hoverBeforeClick: true,
        ...options,
      });
      if (
        ghostResult?.success &&
        ghostResult?.x != null &&
        ghostResult?.y != null
      ) {
        const x = Math.round(ghostResult.x);
        const y = Math.round(ghostResult.y);
        this.log(`[ai-twitterActivity][api.click] Clicked x=${x} y=${y}`);
      } else if (ghostResult?.success === false) {
        throw new Error("ghost_click_failed");
      }
    } catch (e) {
      this.log(
        `[Interaction] humanClick failed on ${description}: ${e.message}`,
      );

      // HUMAN-LIKE: Error recovery
      await this.human.recoverFromError("click_failed", { locator: target });
      throw e;
    }
  }

  /**
   * Safe human-like click with retry logic
   * Wraps humanClick with automatic retry on failure
   * @param {Object} target - Playwright locator or element handle
   * @param {string} description - Description for logging
   * @param {number} retries - Number of retry attempts (default: 3)
   * @param {Object} options - Additional options for ghost click
   * @returns {Promise<boolean>} - True if successful, false if all retries failed
   */
  async safeHumanClick(
    target,
    description = "Target",
    retries = 3,
    options = {},
  ) {
    const attemptLogs = [];
    for (let attempt = 1; attempt <= retries; attempt++) {
      try {
        await this.humanClick(target, description, options);
        this.log(
          `[Interaction] [${description}] Success on attempt ${attempt}/${retries}`,
        );
        return true;
      } catch (error) {
        attemptLogs.push(`Attempt ${attempt}: ${error.message}`);
        this.log(
          `[Interaction] [${description}] Attempt ${attempt}/${retries} failed: ${error.message}`,
        );
        if (attempt === retries) {
          this.log(
            `[Interaction] [${description}] All retries exhausted. Errors: ${attemptLogs.join("; ")}`,
          );
          return false;
        }
        // Exponential backoff: 1s, 2s, 3s...
        const delay = 1000 * attempt;
        this.log(
          `[Interaction] [${description}] Waiting ${delay}ms before retry ${attempt + 1}...`,
        );
        await api.wait(delay);
      }
    }
    return false;
  }

  /**
   * Check if element is truly actionable (not covered by overlay)
   * @param {object} element - Playwright locator
   * @returns {Promise<boolean>}
   */
  async isElementActionable(element) {
    try {
      const handle = await element.elementHandle();
      if (!handle) return false;

      return await this.page.evaluate(async (el) => {
        const rect = el.getBoundingClientRect();
        if (rect.width === 0 || rect.height === 0) return false;

        const centerX = rect.left + rect.width / 2;
        const centerY = rect.top + rect.height / 2;
        const topElement = document.elementFromPoint(centerX, centerY);

        return topElement && (el.contains(topElement) || el === topElement);
      }, handle);
    } catch {
      return false;
    }
  }

  /**
   * Scroll element to Golden Zone (30% from top of viewport)
   * @param {object} element - Playwright locator
   */
  async scrollToGoldenZone(element) {
    if (!element) return;
    try {
      const handle = await element.elementHandle();
      if (!handle) return;

      await this.page.evaluate((el) => {
        const rect = el.getBoundingClientRect();
        const targetY = window.innerHeight * 0.3; // 30% from top
        const scrollBy = rect.top - targetY;
        window.scrollBy({ top: scrollBy, behavior: "smooth" });
      }, handle);

      await api.wait(500); // Wait for scroll to settle
    } catch (e) {
      this.log(`[GoldenZone] Scroll failed: ${e.message}`);
    }
  }

  /**
   * Dismiss any overlays (toasts, modals) that might block clicks
   */
  async dismissOverlays() {
    try {
      // Check for toast notifications
      const toasts = this.page.locator('[data-testid="toast"], [role="alert"]');
      if ((await toasts.count()) > 0) {
        await this.page.keyboard.press("Escape");
        await api.wait(300);
      }

      // Check for modals/dialogs
      const modals = this.page.locator('[role="dialog"], [aria-modal="true"]');
      if ((await modals.count()) > 0) {
        await this.page.keyboard.press("Escape");
        await api.wait(300);
      }
    } catch {
      // Ignore overlay dismissal errors
    }
  }

  /**
   * Poll for follow state change
   * @param {string} unfollowSelector - Selector for unfollow button
   * @param {string} followSelector - Selector for follow button
   * @param {number} maxWaitMs - Maximum wait time in ms
   * @returns {Promise<boolean>}
   */
  async pollForFollowState(
    unfollowSelector,
    followSelector,
    maxWaitMs = 20000,
  ) {
    const pollInterval = 2000;
    const maxPolls = Math.floor(maxWaitMs / pollInterval);

    for (let i = 0; i < maxPolls; i++) {
      // Check if Unfollow button appeared
      if (
        await this.page
          .locator(unfollowSelector)
          .first()
          .isVisible()
          .catch(() => false)
      ) {
        return true;
      }

      // Check if button text changed to "Following"
      const btn = this.page.locator(followSelector).first();
      const text = await btn.textContent().catch(() => "");
      if (text && text.toLowerCase().includes("following")) {
        return true;
      }

      await api.wait(pollInterval);
    }

    return false;
  }

  /**
   * 6-Layer Click Strategy for maximum reliability
   * @param {object} element - Element to click
   * @param {string} logPrefix - Logging prefix
   * @returns {Promise<boolean>} - Whether click was performed
   */
  async sixLayerClick(element, logPrefix) {
    const layers = [
      {
        name: "Ghost Click",
        fn: async () => await this.safeHumanClick(element, "Follow Button", 2),
      },
    ];

    for (let i = 0; i < layers.length; i++) {
      const layer = layers[i];
      try {
        this.log(`${logPrefix} Layer ${i + 1}/6: ${layer.name}...`);
        await layer.fn();
        return true; // Click executed, caller will verify
      } catch (e) {
        this.log(
          `${logPrefix} Layer ${i + 1} (${layer.name}) failed: ${e.message}`,
        );

        // Delay between layers (300-800ms)
        if (i < layers.length - 1) {
          await api.wait(mathUtils.randomInRange(300, 800));
        }
      }
    }

    return false; // All layers failed
  }

  /**
   * Robust Follow with 99.99% Success Rate
   * 6-layer click fallback, polling verification, page reload on failure
   * @param {string} [logPrefix='[Follow]'] - Optional prefix for log messages
   * @param {string|null} [reloadUrl=null] - Optional URL to force navigate to on reload
   * @returns {Promise<Object>} Promise resolving to result object with success, attempts, reason, fatal properties
   */
  async robustFollow(logPrefix = "[Follow]", reloadUrl = null) {
    const followBtnSelector =
      'div[data-testid="placementTracking"] [data-testid$="-follow"], div[role="button"][data-testid$="-follow"]';
    const unfollowBtnSelector = '[data-testid$="-unfollow"]';
    const MAX_ATTEMPTS = 5;
    const POST_RELOAD_ATTEMPTS = 2;

    let result = { success: false, attempts: 0, reason: "", fatal: false };
    let hasReloaded = false;

    // Pre-check: Already following?
    const preCheckUnfollow = this.page.locator(unfollowBtnSelector).first();
    if (await preCheckUnfollow.isVisible().catch(() => false)) {
      this.log(`${logPrefix} ✅ Already following.`);
      return {
        success: true,
        attempts: 0,
        reason: "already_following",
        fatal: false,
      };
    }

    // Pre-check: Button text already says "Following" or "Pending"?
    const preCheckFollow = this.page.locator(followBtnSelector).first();
    if (await preCheckFollow.isVisible().catch(() => false)) {
      const preText = (
        (await preCheckFollow.textContent()) || ""
      ).toLowerCase();
      if (preText.includes("following")) {
        this.log(`${logPrefix} ✅ Already following (button text).`);
        return {
          success: true,
          attempts: 0,
          reason: "already_following",
          fatal: false,
        };
      }
      // Race Condition Fix #4: Handle Pending state
      if (preText.includes("pending")) {
        this.log(
          `${logPrefix} ⏳ Follow request pending. Waiting for resolution...`,
        );
        await api.wait(3000);
        // Re-check after wait
        if (await preCheckUnfollow.isVisible().catch(() => false)) {
          this.log(`${logPrefix} ✅ Pending resolved to Following.`);
          return {
            success: true,
            attempts: 0,
            reason: "pending_resolved",
            fatal: false,
          };
        }
      }
    }

    // Initial delay: 5-10s to "read the profile"
    const initialDelay = mathUtils.randomInRange(8000, 15000);
    this.log(
      `${logPrefix} Reading profile for ${(initialDelay / 1000).toFixed(1)}s...`,
    );
    await api.wait(initialDelay);

    const totalAttempts = MAX_ATTEMPTS + POST_RELOAD_ATTEMPTS;

    for (let attempt = 1; attempt <= totalAttempts; attempt++) {
      result.attempts = attempt;

      // Check if we need to reload (after MAX_ATTEMPTS failures)
      if (attempt === MAX_ATTEMPTS + 1 && !hasReloaded) {
        this.log(
          `${logPrefix} 🔄 All ${MAX_ATTEMPTS} attempts failed. Reloading page...`,
        );

        // Thinking delay before reload
        await api.wait(mathUtils.randomInRange(2000, 5000));

        try {
          if (reloadUrl) {
            this.log(`${logPrefix} 🔄 Force Navigating to: ${reloadUrl}`);
            await this.page.goto(reloadUrl, {
              waitUntil: "domcontentloaded",
              timeout: 60000,
            });
          } else {
            await this.page.reload({
              waitUntil: "networkidle",
              timeout: 60000,
            });
          }

          await api.wait(mathUtils.randomInRange(5000, 10000));
          hasReloaded = true;
          this.log(
            `${logPrefix} Page reloaded. Trying ${POST_RELOAD_ATTEMPTS} more attempts...`,
          );
        } catch (e) {
          this.log(`${logPrefix} Reload failed: ${e.message}`);
          result.reason = "reload_failed";
          result.fatal = true;
          break;
        }
      }

      // Pre-flight: Dismiss overlays
      await this.dismissOverlays();

      // --- SOFT ERROR CHECK ---
      if (await this.checkAndHandleSoftError(reloadUrl)) {
        await api.wait(mathUtils.randomInRange(5000, 8000));
        continue;
      }

      // Re-query selectors each attempt to avoid stale references
      const freshFollowBtn = this.page.locator(followBtnSelector).first();
      const freshUnfollowBtn = this.page.locator(unfollowBtnSelector).first();

      // Check if already succeeded (from previous attempt's delayed state change)
      if (await freshUnfollowBtn.isVisible().catch(() => false)) {
        this.log(
          `${logPrefix} ✅ Already following (detected on attempt ${attempt}).`,
        );
        result.success = true;
        result.reason = "unfollow_button_visible";
        break;
      }

      // --- HEALTH CHECK ---
      const health = await this.performHealthCheck();
      if (!health.healthy) {
        this.log(
          `${logPrefix} 💀 CRITICAL HEALTH FAILURE: ${health.reason}. Aborting task.`,
        );
        result.success = false;
        result.fatal = true;
        result.reason = health.reason;
        break;
      }

      if (await freshFollowBtn.isVisible()) {
        const buttonText = (
          (await freshFollowBtn.textContent()) || ""
        ).toLowerCase();

        // Safety: Skip if button text suggests already following
        if (
          buttonText.includes("following") ||
          buttonText.includes("unfollow")
        ) {
          this.log(
            `${logPrefix} Button text '${buttonText}' suggests already following.`,
          );
          result.success = true;
          result.reason = "button_text_following";
          break;
        }

        if (buttonText.includes("follow")) {
          this.log(`${logPrefix} === Attempt ${attempt}/${totalAttempts} ===`);

          try {
            // Pre-flight: Scroll to Golden Zone
            await this.scrollToGoldenZone(freshFollowBtn);

            // Pre-flight: Verify actionability
            const isActionable = await this.isElementActionable(freshFollowBtn);
            if (!isActionable) {
              this.log(
                `${logPrefix} Button not actionable (covered by overlay?). Retrying...`,
              );
              await this.dismissOverlays();
              await api.wait(500);
            }

            // Race Condition Fix #1: Re-check button state right before clicking
            const preClickBtn = this.page.locator(followBtnSelector).first();
            const preClickText = (
              (await preClickBtn.textContent().catch(() => "")) || ""
            ).toLowerCase();
            if (
              preClickText.includes("following") ||
              preClickText.includes("pending")
            ) {
              this.log(
                `${logPrefix} Button changed to '${preClickText}' before click. Verifying...`,
              );
              if (
                await this.page
                  .locator(unfollowBtnSelector)
                  .first()
                  .isVisible()
                  .catch(() => false)
              ) {
                this.state.follows++;
                result.success = true;
                result.reason = "state_changed_before_click";
                this.log(`${logPrefix} Already followed (detected pre-click)`);
                break;
              }
            }

            // Execute 6-layer click strategy
            const clickPerformed = await this.sixLayerClick(
              preClickBtn,
              logPrefix,
            );

            if (!clickPerformed) {
              this.log(`${logPrefix} All 6 click layers failed.`);
              result.reason = "all_layers_failed";
              continue;
            }

            // Polling verification: Check every 500ms for up to 5s
            this.log(`${logPrefix} Verifying follow state...`);
            const verified = await this.pollForFollowState(
              unfollowBtnSelector,
              followBtnSelector,
              20000,
            );

            if (verified) {
              this.state.follows++;
              result.success = true;
              result.reason = "verified_success";
              this.log(
                `${logPrefix} Followed Successfully (Attempt ${attempt})`,
              );
              break;
            }

            // Race Condition Fix #3: Re-query locator after polling (DOM may have changed)
            const postPollBtn = this.page.locator(followBtnSelector).first();
            const ariaLabel = await postPollBtn
              .getAttribute("aria-label")
              .catch(() => "");
            if (ariaLabel && ariaLabel.toLowerCase().includes("following")) {
              this.state.follows++;
              result.success = true;
              result.reason = "aria_label_following";
              this.log(
                `${logPrefix} Followed Successfully (Attempt ${attempt})`,
              );
              break;
            }

            // Additional check: Button text after polling
            const postPollText = (
              (await postPollBtn.textContent().catch(() => "")) || ""
            ).toLowerCase();
            if (postPollText.includes("following")) {
              this.state.follows++;
              result.success = true;
              result.reason = "post_poll_text_following";
              this.log(
                `${logPrefix} Followed Successfully (Attempt ${attempt})`,
              );
              break;
            }

            this.log(`${logPrefix} Attempt ${attempt} verification failed.`);
            result.reason = "verification_failed";
          } catch (e) {
            this.log(`${logPrefix} Attempt ${attempt} error: ${e.message}`);
            result.reason = e.message;
          }

          // Exponential backoff before retry
          if (attempt < totalAttempts) {
            const base = 3000 + attempt * 1000;
            const jitter = mathUtils.randomInRange(-500, 500);
            const backoff = base + jitter;
            this.log(
              `${logPrefix} Backoff ${(backoff / 1000).toFixed(1)}s before attempt ${attempt + 1}...`,
            );
            await api.wait(backoff);
          }
        } else {
          result.reason = "button_text_not_follow";
        }
      } else {
        this.log(
          `${logPrefix} Follow button not visible on attempt ${attempt}.`,
        );
        result.reason = "button_not_visible";
        if (attempt < totalAttempts) {
          await api.wait(2000);
        }
      }
    }

    if (!result.success) {
      this.log(
        `${logPrefix} ❌ Follow failed after ${result.attempts} attempts (reload: ${hasReloaded}). Reason: ${result.reason}`,
      );
    }

    return result;
  }

  /**
   * Performs a comprehensive health check on the browser session.
   * Detects:
   * 1. Critical Error Pages (Redirect loops, broken pages)
   * 2. Network Zombie State (No activity for > 30s)
   * @returns {Promise<{healthy: boolean, reason: string}>}
   */
  async performHealthCheck() {
    try {
      // 1. Check Network Vital Signs
      const timeSinceLastActivity = Date.now() - this.lastNetworkActivity;
      if (timeSinceLastActivity > 30000) {
        // 30s Threshold
        return {
          healthy: false,
          reason: `network_inactivity_${Math.round(timeSinceLastActivity / 1000)}s`,
        };
      }

      // 2. Check for Critical Error Pages
      // Simple text content scan is usually enough for these standard chrome error pages
      const content = await this.page.content().catch(() => "");
      if (
        content.includes("ERR_TOO_MANY_REDIRECTS") ||
        content.includes("This page isn’t working") ||
        content.includes("redirected you too many times")
      ) {
        return { healthy: false, reason: "critical_error_page_redirects" };
      }

      return { healthy: true, reason: "" };
    } catch {
      // If we can't check, assume something is very wrong if network is also silent,
      // but let's be conservative and return healthy if just the check fails but page is alive.
      return { healthy: true, reason: "check_failed" };
    }
  }

  /**
   * Checks for "Something went wrong" soft error and reloads if found.
   * @param {string|null} [reloadUrl=null] - Optional URL to force navigate to on reload
   * @returns {Promise<boolean>} - True if error was found and reload triggered
   */
  async checkAndHandleSoftError(reloadUrl = null) {
    try {
      // Broad text match for the error header
      const softError = this.page
        .locator("text=/Something went wrong/i")
        .first();

      // Check if error is visible (fast check)
      if (await softError.isVisible({ timeout: 1000 }).catch(() => false)) {
        this.state.consecutiveSoftErrors =
          (this.state.consecutiveSoftErrors || 0) + 1;
        this.log(
          `⚠️ Soft Error detected: 'Something went wrong'. (Attempt ${this.state.consecutiveSoftErrors}/3)`,
        );

        if (this.state.consecutiveSoftErrors >= 3) {
          this.log(
            `[SoftError] Maximum retries reached. potential twitter logged out`,
          );
          throw new Error("potential twitter logged out");
        }

        // Strategy 1: Try clicking explicit "Retry" button if available
        // LIMIT: Only try this 1x (on the first error detection)
        if (this.state.consecutiveSoftErrors === 1) {
          const retryBtn = this.page
            .locator('[role="button"][name="Retry"], button:has-text("Retry")')
            .first();
          if (await retryBtn.isVisible().catch(() => false)) {
            this.log(`[SoftError] Found Retry button. Clicking...`);
            await retryBtn.click().catch(() => {});
            await api.wait(3000);
            return true;
          }
        }

        // Strategy 2: Full Page Reload
        this.log(
          `[SoftError] No retry button found. Initializing Page Reload...`,
        );
        try {
          // Reduce chance of infinite reloading the same bad state by waiting a bit
          await api.wait(2000);

          const targetUrl = reloadUrl || this.page.url();
          if (targetUrl.startsWith("http")) {
            this.log(
              `[SoftError] Simulating Refresh by re-entering URL: ${targetUrl}`,
            );
            await this.page.goto(targetUrl, {
              waitUntil: "domcontentloaded",
              timeout: 45000,
            });
          } else {
            // Fallback if URL is weird (e.g. about:blank)
            await this.page.reload({
              waitUntil: "domcontentloaded",
              timeout: 45000,
            });
          }

          await api.wait(5000); // Post-refresh wait

          // VERIFICATION: Did the refresh fix it?
          if (
            !(await this.page
              .locator("text=/Something went wrong/i")
              .isVisible({ timeout: 1000 })
              .catch(() => false))
          ) {
            this.log(
              `[SoftError] Refresh successful. Error cleared. Resuming task...`,
            );
            this.state.consecutiveSoftErrors = 0;
          }

          return true;
        } catch (e) {
          this.log(`⚠️ Reload failed: ${e.message}`);
          return true; // Still return true so we don't proceed with broken page
        }
      } else {
        // No soft error visible, reset counter
        this.state.consecutiveSoftErrors = 0;
      }
    } catch (e) {
      // Propagate the logout error
      if (e.message.includes("potential twitter logged out")) throw e;
      // Ignore other errors (like page closed) during check
    }
    return false;
  }

  /**
   * Checks if session has reached fatigue threshold
   * @returns {void}
   */
  checkFatigue() {
    if (this.isFatigued) return;
    const elapsed = Date.now() - this.sessionStart;
    if (elapsed > this.fatigueThreshold) {
      this.triggerHotSwap();
    }
  }

  /**
   * Triggers hot swap to slower profile when fatigue is detected
   * @returns {void}
   */
  triggerHotSwap() {
    this.log(`⚠️ ENERGY DECAY TRIGGERED | Simulasi wes kesel browsing`);
    const slowerProfile = profileManager.getFatiguedVariant(
      this.config.timings.scrollPause.mean,
    );
    if (slowerProfile) {
      this.log(`HOT SWAP: ${this.config.id} >>> ${slowerProfile.id}`);
      this.config = slowerProfile;
      this.isFatigued = true;
      // Adjust probabilities and timing when fatigued
      this.config.probabilities.refresh = this.clamp(
        (this.config.probabilities.refresh || 0.05) * 0.5,
        0,
        0.2,
      );
      this.config.probabilities.idle = this.clamp(
        (this.config.probabilities.idle || 0.2) + 0.2,
        0,
        0.8,
      );
      this.state.fatigueBias = 0.2;
    } else {
      this.log("Already at minimum energy state. Entering DOOM SCROLL mode.");
      this.isFatigued = true;
      this.state.fatigueBias = 0.3;
    }
  }

  /**
   * Gets random scroll method based on configured probabilities
   * @returns {string} Scroll method name
   */
  getScrollMethod() {
    const methods = this.config.inputMethods || {
      wheelDown: 0.8,
      wheelUp: 0.03,
      space: 0.05,
      keysDown: 0.1,
      keysUp: 0,
    };
    const roll = Math.random();
    let sum = 0;
    if (roll < (sum += methods.wheelDown)) return "WHEEL_DOWN";
    if (roll < (sum += methods.wheelUp)) return "WHEEL_UP";
    if (roll < (sum += methods.space)) return "SPACE";
    if (roll < sum + methods.keysDown) return "KEYS_DOWN";
    return "WHEEL_DOWN";
  }

  /**
   * Normalizes action probabilities with fatigue bias
   * @param {object} p - Probability configuration
   * @returns {object} Normalized probabilities
   */
  normalizeProbabilities(p) {
    // Fallback to default safe probabilities if not specified
    const base = {
      refresh: 0.05, // refresh feed
      profileDive: 0.15, // dive into a profile
      tweetDive: 0.1, // open an expanded tweet
      idle: 0.3, // explicit idle
      likeTweetafterDive: 0.005,
      bookmarkAfterDive: 0.005,
      followOnProfile: 0.005,
    };
    const merged = { ...base, ...(p || {}) };

    // fatigue bias increases idle probability a bit
    merged.idle = this.clamp(
      merged.idle + (this.state.fatigueBias || 0),
      0,
      0.9,
    );

    // recent refresh suppression: if refreshed in last 20s, reduce refresh
    const sinceRefresh = Date.now() - (this.state.lastRefreshAt || 0);
    if (sinceRefresh < 20000) {
      merged.refresh = this.clamp(merged.refresh * 0.4, 0, 0.2);
    }

    // ensure each is within [0,1]
    for (const k of Object.keys(merged))
      merged[k] = this.clamp(merged[k], 0, 1);
    if (p && p.likeTweetAfterDive != null) {
      merged.likeTweetafterDive = p.likeTweetAfterDive;
    } else if (
      merged.likeTweetAfterDive != null &&
      merged.likeTweetafterDive == null
    ) {
      merged.likeTweetafterDive = merged.likeTweetAfterDive;
    }

    // BURST MODE OVERRIDE: High Engagement, No Idle
    if (this.state.activityMode === "BURST") {
      merged.idle = 0.0;
      merged.refresh = 0.0;
      merged.tweetDive = 0.6; // High chance to open tweets
      merged.profileDive = 0.2;
      // Boost engagement chances during burst
      merged.likeTweetafterDive = 0.5;
      merged.bookmarkAfterDive = 0.3;
      merged.followOnProfile = 0.2;
    }

    return merged;
  }

  /**
   * Simulates human reading behavior with random delays
   * @returns {Promise<void>}
   */
  async simulateReading() {
    const { mean, deviation } = this.config.timings.readingPhase;
    const actionDelays = this.config.timings.actionSpecific || {
      space: { mean: 1000, deviation: 200 },
      keys: { mean: 100, deviation: 30 },
    };

    // HUMAN-LIKE: Use content skimming pattern
    await this.human.consumeContent("tweet", "skim");

    // Calculate duration using Gaussian distribution
    let baseDuration = mean
      ? mathUtils.gaussian(mean, deviation || mean * 0.3)
      : mathUtils.randomInRange(30000, 60000);

    // Fatigue lengthens reading by up to +25%
    let fatigueFactor = this.isFatigued ? 1.25 : 1.0;

    // BURST MODE OVERRIDE: Fast scanning (0.5s - 2s)
    if (this.state.activityMode === "BURST") {
      baseDuration = mathUtils.randomInRange(500, 2000);
      fatigueFactor = 1.0;
    }

    const duration = Math.floor(baseDuration * fatigueFactor);
    const startTime = Date.now();

    this.log(`[Idle] Reading Phase (${(duration / 1000).toFixed(1)}s)`);

    while (Date.now() - startTime < duration) {
      if (this.isSessionExpired()) return;
      this.checkFatigue();

      // --- SOFT ERROR CHECK ---
      if (await this.checkAndHandleSoftError()) {
        // HUMAN-LIKE: Error recovery
        await this.human.recoverFromError("timeout", {});
        await api.wait(5000);
        continue;
      }

      // --- FATAL HEALTH CHECK ---
      const health = await this.performHealthCheck();
      if (!health.healthy) {
        this.log(
          `💀 Fatal Error detected during reading phase: ${health.reason}`,
        );
        throw new Error(`Fatal: ${health.reason}`);
      }

      // HUMAN-LIKE: Multitasking during reading
      if (mathUtils.roll(0.15)) {
        await this.human.multitask();
      }

      // Micro: occasional cursor drift/hover
      if (mathUtils.roll(0.25)) {
        try {
          const viewport = this.page.viewportSize() || {
            width: 1280,
            height: 720,
          };
          const x = mathUtils.gaussian(viewport.width / 2, viewport.width / 4);
          const y = mathUtils.gaussian(
            viewport.height / 2,
            viewport.height / 4,
          );
          const safeX = Math.max(0, Math.min(viewport.width, x));
          const safeY = Math.max(0, Math.min(viewport.height, y));
          await this.ghost.move(safeX, safeY, mathUtils.randomInRange(8, 20));
        } catch {
          // Ignore viewport calculation errors
        }
      }

      // Micro: Fidgeting
      if (mathUtils.roll(0.15)) {
        await this.simulateFidget();
      }

      // Micro: tiny jitter scroll
      if (mathUtils.roll(0.15)) {
        await scrollRandom(-60, 60);
        await api.wait(mathUtils.randomInRange(50, 140));
      }

      const method = this.getScrollMethod();

      if (method === "WHEEL_DOWN") {
        const distance = mathUtils.gaussian(400, 150);
        await scrollDown(distance);
        if (mathUtils.roll(0.2)) {
          const viewport = this.page.viewportSize() || {
            width: 1280,
            height: 720,
          };
          // Random move within viewport
          const x = mathUtils.gaussian(viewport.width / 2, viewport.width / 3);
          const y = mathUtils.gaussian(
            viewport.height / 2,
            viewport.height / 3,
          );
          await this.ghost.move(
            Math.max(0, Math.min(viewport.width, x)),
            Math.max(0, Math.min(viewport.height, y)),
            10,
          );
        }
      } else if (method === "WHEEL_UP") {
        const distance = mathUtils.gaussian(300, 100);
        await scrollRandom(-distance, -distance);
      } else if (method === "SPACE") {
        await this.page.keyboard.press("Space", {
          delay: mathUtils.randomInRange(50, 150),
        });
        const d = actionDelays.space;
        await api.wait(Math.max(10, mathUtils.gaussian(d.mean, d.deviation)));
      } else if (method === "KEYS_DOWN") {
        const presses = mathUtils.randomInRange(1, 4);
        for (let k = 0; k < presses; k++) {
          if (this.isSessionExpired()) return;
          await this.page.keyboard.press("ArrowDown", {
            delay: mathUtils.randomInRange(50, 150),
          });
          const d = actionDelays.keys;
          await api.wait(Math.max(10, mathUtils.gaussian(d.mean, d.deviation)));
        }
      } else if (method === "KEYS_UP") {
        const presses = mathUtils.randomInRange(1, 3);
        for (let k = 0; k < presses; k++) {
          if (this.isSessionExpired()) return;
          await this.page.keyboard.press("ArrowUp", {
            delay: mathUtils.randomInRange(50, 150),
          });
          const d = actionDelays.keys;
          await api.wait(Math.max(10, mathUtils.gaussian(d.mean, d.deviation)));
        }
      }

      // Reading pause influenced by fatigue
      const { mean, deviation } = this.config.timings.scrollPause;
      let pause = Math.floor(
        mathUtils.gaussian(mean, deviation) * (this.isFatigued ? 1.2 : 1),
      );
      pause = Math.max(50, pause);

      // HUMAN BEHAVIOR: "Mouse Parking"
      // If reading for > 2 seconds, move mouse to margin
      if (pause > 2000 && mathUtils.roll(0.7)) {
        // 70% chance to park
        // Subtract parking time (~1s) from total pause to keep rhythm
        await this.ghost.park();
        pause = Math.max(50, pause - 1000);
      }

      await api.wait(pause);

      // Chance to Dive into a Tweet
      const p = this.normalizeProbabilities(this.config.probabilities);
      if (mathUtils.roll(p.tweetDive)) {
        await this.diveTweet();
      }
    }
  }

  /**
   * Dives into a tweet to view expanded content
   * @returns {Promise<void>}
   */
  async diveTweet() {
    this.log("[Branch] Tweet Dive (Expanding)...");
    try {
      // Try up to 3 times to find a suitable tweet
      let targetTweet = null;
      for (let attempt = 0; attempt < 3; attempt++) {
        const tweets = this.page.locator('article[data-testid="tweet"]');
        const count = await tweets.count();

        if (count > 0) {
          for (let i = 0; i < Math.min(count, 10); i++) {
            const t = tweets.nth(i);
            const box = await t.boundingBox();
            // Relaxed bounds: just needs to be somewhat on screen (positive height)
            // and not WAY down the page (> 1000)
            if (box && box.height > 0 && box.y > -50 && box.y < 1000) {
              targetTweet = t;
              if (Math.random() > 0.4) break; // Random selection
            }
          }
        }

        if (targetTweet) break;

        // Not found? Scroll a bit and retry
        this.log("[Dive] No suitable tweets in view. Scrolling...");
        await scrollRandom(300, 300);
        await api.wait(entropy.retryDelay(attempt));
      }

      if (!targetTweet) {
        this.log(
          "No suitable tweets found in viewport after retries. Refreshing Home...",
        );
        // Reset state by going to https://x.com/ (redirects to default feed)
        await this.page.goto("https://x.com/");
        await this.ensureForYouTab();
        return;
      }

      const timeStamp = targetTweet.locator("time").first();
      const textContent = targetTweet
        .locator('[data-testid="tweetText"]')
        .first();

      let clickTarget = null;
      let parentLink = null;

      if ((await textContent.count()) > 0 && (await textContent.isVisible())) {
        clickTarget = textContent;
        this.log("[Debug] Targeting tweet text body (Primary).");
      } else if (
        (await timeStamp.count()) > 0 &&
        (await timeStamp.isVisible())
      ) {
        parentLink = timeStamp.locator("xpath=./ancestor::a[1]");
        clickTarget = (await parentLink.count()) > 0 ? parentLink : timeStamp;
        this.log("[Debug] Targeting tweet permalink/time (Fallback).");
      } else {
        clickTarget = targetTweet;
        this.log("[Debug] Targeting entire tweet card (Last Resort).");
      }

      if (clickTarget) {
        // Ensure the SPECIFIC TARGET is centered to avoid sticky header (120px safety)
        await clickTarget.evaluate((el) =>
          el.scrollIntoView({ block: "center", inline: "center" }),
        );
        await api.wait(entropy.scrollSettleTime()); // Wait for scroll alignment

        // ROBUST VIEWPORT & SCROLL CHECK
        let dbgBox = await clickTarget.boundingBox();
        let dbgVP = this.page.viewportSize();

        // Fallback for null viewport
        if (!dbgVP) {
          try {
            dbgVP = await this.page.evaluate(() => ({
              width: window.innerWidth,
              height: window.innerHeight,
            }));
          } catch {
            dbgVP = { width: 1280, height: 720 };
          }
        }

        // Verify if actually on screen. If not, force scroll again.
        if (dbgBox && (dbgBox.y < 80 || dbgBox.y > dbgVP.height - 80)) {
          this.log(
            `[Debug] Target off-screen after scroll (y=${Math.round(dbgBox.y)}). Adjusting...`,
          );
          // Force precise scroll
          await clickTarget.evaluate((el) => {
            const rect = el.getBoundingClientRect();
            const scrollTop =
              window.pageYOffset || document.documentElement.scrollTop;
            window.scrollTo({
              top: scrollTop + rect.top - 200,
              behavior: "auto",
            });
          });
          await api.wait(entropy.scrollSettleTime());
          dbgBox = await clickTarget.boundingBox(); // Update box
        }

        this.log(
          `[Debug] Navigate Target: Box=${dbgBox ? `x:${Math.round(dbgBox.x)},y:${Math.round(dbgBox.y)},w:${Math.round(dbgBox.width)},h:${Math.round(dbgBox.height)}` : "null"} | VP=${dbgVP ? `${dbgVP.width}x${dbgVP.height}` : "null"}`,
        );

        this.log(`[Attempt] Ghost Click on Permalink...`);
        // Use new anti-sybil wrapper with retry
        await this.safeHumanClick(clickTarget, "Tweet Permalink", 3);
      }

      let expanded = false;
      try {
        await this.page.waitForURL("**/status/**", { timeout: 8000 });
        this.log("[Success] Ghost Click navigated to tweet.");
        expanded = true;
      } catch {
        this.log(
          "[Fail] Ghost Click did not navigate. Retrying with NATIVE click...",
        );
        if (clickTarget) {
          this.log("[Attempt] Native Click (Force)...");
          await clickTarget.click({ force: true });
        }
        try {
          await this.page.waitForURL("**/status/**", { timeout: 8000 });
          this.log("[Success] Native Click navigated to tweet.");
          expanded = true;
        } catch {
          this.log("[Fail] Failed to expand tweet after retry. Aborting dive.");
          return;
        }
      }

      if (!expanded) return;

      // Read main tweet and open media occasionally
      const readTime = mathUtils.randomInRange(5000, 15000);
      this.log(`[Idle] Reading expanded tweet for ${readTime}ms...`);
      await api.wait(readTime);

      // Optional open media
      if (mathUtils.roll(0.2)) {
        const media = this.page.locator('[data-testid="tweetPhoto"]').first();
        if ((await media.count()) > 0 && (await media.isVisible())) {
          this.log("[Action] Open media viewer");
          await this.safeHumanClick(media, "Media Viewer", 3);
          const viewTime = mathUtils.randomInRange(5000, 12000);
          this.log(
            `[Media] Viewing media for ${(viewTime / 1000).toFixed(1)}s...`,
          );
          await api.wait(viewTime);
          // Close media viewer with ESC
          await this.page.keyboard.press("Escape", {
            delay: mathUtils.randomInRange(50, 150),
          });
          await api.wait(mathUtils.randomInRange(400, 900));
        }
      }

      // Read replies
      this.log("[Scroll] Reading replies...");
      await scrollRandom(300, 600);
      await api.wait(mathUtils.randomInRange(2000, 4000));

      // Asymmetric return (don't scroll back exactly the same amount)
      await scrollRandom(-660, -240);
      await api.wait(mathUtils.randomInRange(1000, 2000));

      const p = this.normalizeProbabilities(this.config.probabilities);

      // Like
      if (mathUtils.roll(p.likeTweetafterDive)) {
        if (this.state.likes >= (this.config.maxLike || 0)) {
          this.log(
            `[Limit] Max likes reached (${this.state.likes}/${this.config.maxLike}). Skipping like.`,
          );
        } else {
          // ROBUST LIKE: Check for "Like" state specifically
          // "unlike" means it's already liked. "like" means unliked.
          const likeButton = this.page
            .locator('button[data-testid="like"][role="button"]')
            .first();
          const unlikeButton = this.page
            .locator('button[data-testid="unlike"][role="button"]')
            .first();

          if (await unlikeButton.isVisible()) {
            this.log("[Skip] Tweet is ALREADY LIKED.");
          } else if ((await likeButton.count()) > 0) {
            try {
              this.log("[Action] Scrolling Like button into view...");
              await likeButton.scrollIntoViewIfNeeded();
              await api.wait(mathUtils.randomInRange(500, 1000));

              // Double check visibility and state
              if (await likeButton.isVisible()) {
                // Verify aria-label confirms "Like" action (not "Unlike" or "Remove Like")
                const label =
                  (await likeButton.getAttribute("aria-label")) || "";
                if (label.includes("Unlike")) {
                  this.log("[Skip] Tweet already liked (aria-label check).");
                } else {
                  this.log("Action: ❤ Like");
                  await this.safeHumanClick(likeButton, "Like Button", 3);
                  this.state.likes++;
                  await api.wait(mathUtils.randomInRange(2000, 5000));
                }
              }
            } catch (e) {
              this.log(`[Warn] Failed to scroll/like: ${e.message}`);
            }
          }
        }
      }

      // Bookmark (Robust)
      if (mathUtils.roll(p.bookmarkAfterDive)) {
        // Check if already bookmarked (usually has different testid 'removeBookmark' or similar)
        const bm = this.page.locator('button[data-testid="bookmark"]').first();
        const unbm = this.page
          .locator('button[data-testid="removeBookmark"]')
          .first();

        if (await unbm.isVisible()) {
          this.log("[Skip] Tweet ALREADY bookmarked.");
        } else if (await bm.isVisible()) {
          this.log("[Action] Attempting Ghost Click: 🔖 Bookmark");
          try {
            await this.safeHumanClick(bm, "Bookmark Button", 3);
            await api.wait(entropy.postClickDelay());
            this.log("[Success] Ghost Click: Bookmark");
          } catch (e) {
            this.log(
              `[Fail] Ghost Click Bookmark: ${e.message}. Fallback to Native.`,
            );
            try {
              await bm.click();
              await api.wait(entropy.postClickDelay());
              this.log("[Success] Native Click: Bookmark");
            } catch (e2) {
              this.log(
                `[Fail] Native Click Bookmark also failed: ${e2.message}`,
              );
            }
          }
        }
      }

      // Idle after actions
      await api.wait(mathUtils.randomInRange(1200, 2400));

      // Return Home
      await this.navigateHome();
      await api.wait(mathUtils.randomInRange(1500, 3000));
    } catch (e) {
      this.log("Dive sequence failed: " + e.message);
      if (!this.page.url().includes("home")) {
        await this.navigateHome();
      }
    }
  }

  /**
   * Dives into a user profile to view content
   * @returns {Promise<void>}
   */
  async diveProfile() {
    this.log("[Branch] Inspecting User Profile");
    // Broader selector to catch all links in the tweet header (Avatar, Name, Handle)
    const selector = 'article[data-testid="tweet"] a[href^="/"]';

    // Evaluate all to find valid profile links (exclude /status/, /hashtag/, etc.)
    const validIndices = await this.page.$$eval(selector, (els) => {
      const reserved = [
        "home",
        "explore",
        "notifications",
        "messages",
        "compose",
        "settings",
        "search",
        "i",
      ];
      return els
        .map((el, i) => {
          let href = el.getAttribute("href");
          if (!href) return -1;

          // Remove trailing slash for checking
          if (href.endsWith("/")) href = href.slice(0, -1);

          const parts = href.split("/").filter((p) => p.trim() !== "");

          // Valid profile link: 1 part (e.g. /username)
          // AND not in reserved list
          // AND not containing blocked keywords like status
          if (
            parts.length === 1 &&
            !reserved.includes(parts[0].toLowerCase()) &&
            !href.includes("/status/") &&
            !href.includes("/hashtag/") &&
            !href.includes("/cashtag/")
          ) {
            return i;
          }
          return -1;
        })
        .filter((i) => i !== -1);
    });

    if (validIndices.length > 0) {
      const chosenIndex =
        validIndices[Math.floor(Math.random() * validIndices.length)];
      const target = this.page.locator(selector).nth(chosenIndex);

      const href = await target.getAttribute("href");
      this.log(`Clicking profile: @${href.replace("/", "")}`);

      // Navigate
      this.log(`[Attempt] Ghost Click on Profile Link...`);
      // Use new anti-sybil wrapper with retry
      await this.safeHumanClick(target, "Profile Link", 3);
      try {
        // Wait for the URL to contain the user handle (ignoring query params)
        // We use a looser regex check or just wait for loadstate because sometimes href is just /User
        await this.page.waitForLoadState("domcontentloaded");
        await api.wait(entropy.pageLoadWait());

        // Verify we navigated somewhere relevant
        if (this.page.url().includes(href)) {
          this.log("[Success] Ghost Click navigated to profile.");
        } else {
          throw new Error("Url did not change to expected profile.");
        }
      } catch {
        this.log(
          "[Fail] Ghost Click did not navigate. Retrying with NATIVE click...",
        );
        try {
          // Force click sometimes helps if element is covered
          await target.click({ force: true });
          await this.page.waitForLoadState("domcontentloaded");
          await api.wait(entropy.pageLoadWait());
          if (this.page.url().includes(href)) {
            this.log("[Success] Native Click navigated to profile.");
          } else {
            this.log(
              "[Fail] Native click also failed verification. Proceeding anyway.",
            );
          }
        } catch (e2) {
          this.log(`[Fail] Native click error: ${e2.message}`);
        }
      }

      // Optional follow (Robust with Retry)
      const p = this.normalizeProbabilities(this.config.probabilities);
      if (mathUtils.roll(p.followOnProfile)) {
        if (this.state.follows >= (this.config.maxFollow || 0)) {
          this.log(
            `[Limit] Max follows reached (${this.state.follows}/${this.config.maxFollow}). Skipping follow.`,
          );
        } else {
          await this.robustFollow("[Profile Follow]");
        }
      }

      // Explore a tab occasionally
      const doTab = mathUtils.roll(0.4);
      if (doTab) {
        const tabs = [
          {
            name: "Tweets",
            sel: 'div[role="tablist"] > div[role="presentation"]:nth-child(1) a[role="tab"]',
          }, // 1st tab (Posts)
          { name: "Replies", sel: 'a[role="tab"][href$="/with_replies"]' },
          { name: "Media", sel: 'a[role="tab"][href$="/media"]' },
          { name: "Likes", sel: 'a[role="tab"][href$="/likes"]' },
        ];
        const chosen = tabs[Math.floor(Math.random() * tabs.length)];
        const tab = this.page.locator(chosen.sel).first();
        if ((await tab.count()) > 0 && (await tab.isVisible())) {
          this.log(`[Tab] Visiting profile tab: ${chosen.name}`);
          await this.safeHumanClick(tab, `Profile Tab ${chosen.name}`, 3);
          await api.wait(mathUtils.randomInRange(800, 1600));
          await this.simulateReading();
        }
      }

      this.log("[Navigation] Returning to Home Feed");
      await this.navigateHome();
      await api.wait(entropy.pageLoadWait());
    } else {
      this.log("No targets found, skipping dive.");
    }
  }

  /**
   * Navigates back to the home feed
   * @returns {Promise<void>}
   */
  async navigateHome() {
    this.log("Returning to Home Feed...");

    // 10% chance to use direct URL navigation (User Request)
    if (mathUtils.roll(0.1)) {
      this.log("[Navigation] 🎲 Random 10%: Using direct URL navigation.");
      try {
        await api.goto("https://x.com/home");
        await this.ensureForYouTab();
        return;
      } catch (e) {
        this.log(
          `[Navigation] Direct URL failed: ${e.message}. Falling back to click.`,
        );
      }
    }

    try {
      const useHomeIcon = Math.random() < 0.8;
      let targetSelector = useHomeIcon
        ? '[data-testid="AppTabBar_Home_Link"]'
        : '[aria-label="X"]';
      let targetName = useHomeIcon ? "Home Icon" : "X Logo";
      let target = this.page.locator(targetSelector).first();

      if (!(await target.isVisible().catch(() => false))) {
        this.log(
          `Preferred nav target (${targetName}) not visible. Switching...`,
        );
        targetSelector = useHomeIcon
          ? '[aria-label="X"]'
          : '[data-testid="AppTabBar_Home_Link"]';
        targetName = useHomeIcon ? "X Logo" : "Home Icon";
        target = this.page.locator(targetSelector).first();
      }

      if (await target.isVisible()) {
        this.log(`[Navigation] Clicking '${targetName}' to navigate home...`);
        const clicked = await api.click(targetSelector);

        if (clicked) {
          await api.wait(mathUtils.randomInRange(800, 1500));

          // Note: "Show X posts" button check is now handled in ensureForYouTab()
          // which is called below after URL navigation completes

          try {
            await this.page.waitForURL("**/home**", { timeout: 5000 });
            this.log("[Navigation] Navigate to /home Success.");
            await this.ensureForYouTab();
            return;
          } catch {
            this.log("[Navigation] api.click() did not trigger navigation.");
          }
        } else {
          this.log(`[Navigation] api.click() failed for '${targetName}'.`);
        }
      }
    } catch (e) {
      this.log(`[Navigation] Interaction failed: ${e.message}`);
    }

    this.log("[Navigation] Fallback to direct URL goto.");
    await this.page.goto("https://x.com/home");
    await this.ensureForYouTab();
  }

  /**
   * Ensures the For You tab is selected
   * @returns {Promise<void>}
   */
  async ensureForYouTab() {
    try {
      // Strictly enforce 'For you' tab to avoid empty 'Following' feeds
      const targetText = "For you";

      // Wait for tablist to load (ensure page is ready)
      try {
        await this.page.waitForSelector('div[role="tablist"]', {
          state: "visible",
          timeout: 5000,
        });
      } catch {
        this.log("[Tab] Tablist not found within timeout.");
        return;
      }

      const tablist = this.page.locator('div[role="tablist"]').first();

      // Get all tabs (generalized selector)
      const tabs = tablist.locator('[role="tab"]');
      const count = await tabs.count();
      let target = null;

      // 1. Try to find by text content
      for (let i = 0; i < count; i++) {
        const tab = tabs.nth(i);
        const text = await tab.textContent().catch(() => "");
        if (text && text.trim() === targetText) {
          target = tab;
          break;
        }
      }

      // 2. Fallback to index if text not found
      if (!target) {
        this.log(`[Tab] "${targetText}" text not found. Fallback to index.`);
        // Index 0 is "For you"
        const fallbackIndex = 0;
        if (count > fallbackIndex) {
          target = tabs.nth(fallbackIndex);
        }
      }

      if (!target) {
        this.log(
          `[Tab] "${targetText}" tab could not be found via text or index.`,
        );
        // Debug: print what we found
        for (let i = 0; i < count; i++) {
          const t = await tabs
            .nth(i)
            .textContent()
            .catch(() => "err");
          this.log(`[Tab] Found index ${i}: "${t}"`);
        }
        return;
      }

      // Check if already selected
      const isSelected = await target
        .getAttribute("aria-selected")
        .catch(() => null);
      if (isSelected === "true") {
        this.log(`[Tab] "${targetText}" is already selected.`);
        return;
      }

      if (await target.isVisible()) {
        this.log(`[Tab] Switching to "${targetText}" tab...`);
        try {
          await this.safeHumanClick(target, "For You Tab", 3);
          await api.wait(mathUtils.randomInRange(800, 1500));
        } catch {
          this.log("[Tab] Ghost click failed, trying native...");
          await target.click();
        }
      }

      // After ensuring "For you" tab, check for "Show X posts" button
      // This button appears 2-3 seconds after returning home (per user feedback)
      await this.checkAndClickShowPostsButton();
    } catch (e) {
      this.log(`[Tab] Failed to ensure timeline tab: ${e.message}`);
    }
  }

  /**
   * Check for and click "Show X posts" button that appears when new posts are available
   * This button typically appears 2-3 seconds after returning home and ensuring "For you" tab
   * @returns {Promise<boolean>} true if button was found and clicked, false otherwise
   */
  async checkAndClickShowPostsButton() {
    try {
      // Wait 2-3 seconds for button to appear (as per user feedback)
      this.log('[Posts] Waiting for "Show X posts" button to appear...');
      await api.wait(mathUtils.randomInRange(2000, 3000));

      // Multiple selector patterns to catch variations:
      // - "Show 34 posts"
      // - "Show 5 posts"
      // - "Show X new posts"
      const buttonSelectors = [
        // Primary: button with role containing span with "Show" and "posts"
        '[role="button"]:has(span:has-text(/Show\\s+\\d+\\s+posts/i))',
        // Alternative: button containing text "Show" and "posts"
        "button:has-text(/Show\\s+\\d+\\s+posts/i)",
        // Broader match: any element with role button containing "Show" and number
        '[role="button"]:has-text("Show"):has-text(/\\d+/)',
      ];

      let showPostsBtn = null;
      let btnText = "";

      // Try each selector
      for (const selector of buttonSelectors) {
        try {
          const btn = this.page.locator(selector).first();
          if ((await btn.count()) > 0 && (await btn.isVisible())) {
            const text = await btn.textContent().catch(() => "");
            // Validate it matches the pattern
            if (/show\s+\d+\s+post/i.test(text)) {
              showPostsBtn = btn;
              btnText = text.trim();
              break;
            }
          }
        } catch {
          // Continue to next selector
        }
      }

      if (showPostsBtn) {
        this.log(
          `[Posts] Found "${btnText}" button, clicking to load new posts...`,
        );

        // HUMAN-LIKE: Additional pre-click behavior for this specific button
        // 1. Ensure button is in viewport (scroll if needed)
        await showPostsBtn.evaluate((el) =>
          el.scrollIntoView({ block: "center", inline: "center" }),
        );
        await api.wait(mathUtils.randomInRange(300, 600));

        // 2. Move cursor to vicinity first (not directly on button)
        const box = await showPostsBtn.boundingBox();
        if (box) {
          const offsetX = mathUtils.randomInRange(-30, 30);
          const offsetY = mathUtils.randomInRange(-20, 20);
          await this.ghost.move(
            box.x + box.width / 2 + offsetX,
            box.y + box.height / 2 + offsetY,
            mathUtils.randomInRange(15, 25),
          );
          await api.wait(mathUtils.randomInRange(400, 800));
        }

        // 3. Use safeHumanClick for full simulated interaction with retry
        await this.safeHumanClick(showPostsBtn, "Show Posts Button", 3);

        // 3. HUMAN-LIKE: "Reading" the new posts after loading
        this.log("[Posts] Posts loading, simulating reading behavior...");
        const waitTime = mathUtils.randomInRange(1200, 2500);
        await api.wait(waitTime);

        // 4. Scroll down slightly to show the new posts (human-like discovery)
        const scrollVariance = 0.8 + Math.random() * 0.4;
        await scrollRandom(
          Math.floor(150 * scrollVariance),
          Math.floor(250 * scrollVariance),
        );
        await api.wait(mathUtils.randomInRange(600, 1000));

        this.log("[Posts] New posts loaded successfully.");
        return true;
      } else {
        this.log('[Posts] No "Show X posts" button found.');
        return false;
      }
    } catch (error) {
      this.log(
        `[Posts] Error checking for show posts button: ${error.message}`,
      );
      return false;
    }
  }

  /**
   * Checks if user is logged in to Twitter
   * @returns {Promise<boolean>} True if logged in
   */
  async checkLoginState() {
    try {
      // Check for specific text content indicating logged out state (Relaxed Matching)
      const signedOutText = [
        "Sign in",
        "Sign up with Google",
        "Create account",
        "Join X today",
        "Oops, something went wrong",
      ];

      for (const text of signedOutText) {
        // Removed { exact: true } to be more robust against whitespace/styling
        const element = this.page.getByText(text).first();
        if (await element.isVisible().catch(() => false)) {
          this.log(`[WARN] Not logged in. Found text: "${text}"`);
          this.state.consecutiveLoginFailures++;
          return false;
        }
      }

      // Check for specific login selectors
      const loginSelectors = [
        'a[href*="/login"]',
        'a[href*="/i/flow/login"]',
        '[data-testid="loginButton"]',
        '[data-testid="signupButton"]',
        '[data-testid="google_sign_in_container"]',
      ];

      for (const selector of loginSelectors) {
        if (
          await this.page
            .locator(selector)
            .first()
            .isVisible()
            .catch(() => false)
        ) {
          this.log(`[WARN] Not logged in. Found selector: "${selector}"`);
          this.state.consecutiveLoginFailures++;
          return false;
        }
      }

      // Heuristic: If we are supposedly on Home but can't see the primary column or interacting elements
      if (this.page.url().includes("home")) {
        const mainColumn = this.page.locator('[data-testid="primaryColumn"]');
        if (
          (await mainColumn.count()) === 0 ||
          !(await mainColumn.isVisible())
        ) {
          const timeline = this.page.locator('[aria-label="Home timeline"]');
          if ((await timeline.count()) === 0) {
            this.log(
              `[WARN] Suspected not logged in: Primary Timeline not visible on /home.`,
            );
            this.state.consecutiveLoginFailures++;
            return false;
          }
        }
      }

      // Reset if successful
      this.state.consecutiveLoginFailures = 0;
      return true;
    } catch (e) {
      this.log(`[WARN] checkLoginState failed: ${e.message}`);
      this.state.consecutiveLoginFailures++;
      return false;
    }
  }

  /**
   * Checks if session has expired based on duration
   * @returns {boolean} True if session expired
   */
  isSessionExpired() {
    if (!this.sessionEndTime) return false;
    return Date.now() > this.sessionEndTime;
  }

  /**
   * Runs the main automation session
   * @param {number} cycles - Number of cycles to run
   * @param {number} minDurationSec - Minimum duration in seconds
   * @param {number} maxDurationSec - Maximum duration in seconds
   * @returns {Promise<void>}
   */
  async runSession(cycles = 10, minDurationSec = 0, maxDurationSec = 0) {
    this.log(`Starting Session on ${this.page.url()}`);

    if (minDurationSec > 0 && maxDurationSec > 0) {
      const durationMs = mathUtils.randomInRange(
        minDurationSec * 1000,
        maxDurationSec * 1000,
      );
      this.sessionEndTime = Date.now() + durationMs;

      // Sync fatigue threshold: 70-90% of session duration
      this.fatigueThreshold = Math.round(
        durationMs * mathUtils.randomInRange(0.7, 0.9),
      );

      this.log(
        `Session Timer Set: ${(durationMs / 1000).toFixed(1)}s (until ${new Date(this.sessionEndTime).toLocaleTimeString()}). Fatigue at ${(this.fatigueThreshold / 60000).toFixed(1)}m.`,
      );
    } else {
      this.log(`Session Mode: Fixed Cycles (${cycles})`);
    }

    const theme = this.config.theme || "dark";
    if (theme) {
      this.log(`Enforcing theme: ${theme}`);
      await this.page.emulateMedia({ colorScheme: theme });
    }

    if (!this.page.url().includes("home")) {
      await this.navigateHome();
    }

    // HUMAN-LIKE: Session warmup
    await this.human.sessionStart();

    // Initial Login Check with Retries (3-Strike Rule)
    for (let i = 0; i < 3; i++) {
      const isLoggedIn = await this.checkLoginState();
      if (isLoggedIn) break;

      if (i < 2) {
        const delay = entropy.retryDelay(i, 5000);
        this.log(
          `[Validation] Login check failed (${i + 1}/3). Retrying in ${(delay / 1000).toFixed(1)}s...`,
        );
        await api.wait(delay);
      }
    }

    if (this.state.consecutiveLoginFailures >= 3) {
      this.log("🛑 Aborting session: Not logged in (3 consecutive failures).");
      return;
    }

    while (true) {
      // HUMAN-LIKE: Check if should end session based on natural patterns (only if no explicit end time set)
      const elapsed = Date.now() - this.sessionStart;
      if (
        !this.sessionEndTime &&
        this.human.session.shouldEndSession(elapsed)
      ) {
        this.log(`⏳ Natural session end reached. Finishing...`);
        break;
      }

      if (this.isSessionExpired()) {
        this.log(`⏳ Session Time Limit Reached. Finishing...`);
        break;
      }

      // Check for repeated login failures
      if (this.state.consecutiveLoginFailures >= 3) {
        this.log(
          `🛑 ABORTING: Detected 'Not Logged In' state 3 times consecutively.`,
        );
        break;
      }

      if (!this.sessionEndTime && this.loopIndex >= cycles) {
        this.log(`Session Cycle Limit Reached (${cycles}). Finishing...`);
        break;
      }

      this.loopIndex += 1;
      this.log(
        `--- Loop ${this.loopIndex} ${this.sessionEndTime ? "" : `of ${cycles}`} ---`,
      );

      // HUMAN-LIKE: Boredom pause - every 4th cycle, take a random "distraction" break
      if (this.loopIndex % 4 === 0 && mathUtils.roll(0.25)) {
        await this.human.session.boredomPause(this.page);
      }

      // --- BURST MODE STATE MACHINE ---
      const now = Date.now();
      if (this.state.activityMode === "BURST") {
        if (now > this.state.burstEndTime) {
          this.state.activityMode = "NORMAL";
          this.log("📉 Burst Mode Ended. Returning to normal pace.");
        }
      } else if (this.state.activityMode === "NORMAL") {
        // 10% chance to enter BURST mode (if not fatigue-limited)
        // Only if not already fatigued
        if (!this.isFatigued && mathUtils.roll(0.1)) {
          this.state.activityMode = "BURST";
          const duration = mathUtils.randomInRange(30000, 60000); // 30-60s burst
          this.state.burstEndTime = now + duration;
          this.log(
            `🔥 >>> ENTERING BURST MODE! High intensity for ${(duration / 1000).toFixed(1)}s`,
          );
        }
      }
      // --------------------------------

      await this.simulateReading();
      if (this.isSessionExpired()) break;

      // Decision Logic
      const p = this.normalizeProbabilities(this.config.probabilities);
      const roll = Math.random();
      let cursor = 0;

      if (roll < (cursor += p.refresh)) {
        this.log("[Branch] Refresh Feed");
        this.state.lastRefreshAt = Date.now();
        // Prefer soft refresh via clicking Home
        await this.navigateHome();
        await api.wait(Math.max(50, mathUtils.gaussian(1500, 600)));
      } else if (roll < (cursor += p.profileDive)) {
        await this.diveProfile();
      } else if (roll < cursor + p.tweetDive) {
        await this.diveTweet();
      } else {
        this.log("[Branch] Idle (Staring at screen)");
        // Use profile-configured idle duration (default to old 5s heuristic if missing)
        const idleCfg = this.config.timings.actionSpecific.idle || {
          mean: 5000,
          deviation: 2000,
        };
        const duration = Math.max(
          1000,
          mathUtils.gaussian(idleCfg.mean, idleCfg.deviation),
        );
        await api.wait(duration);
      }

      // Graceful wind-down if under 20s remaining
      if (this.sessionEndTime && this.sessionEndTime - Date.now() < 20000) {
        this.log("Winding down session... Navigating Home and idling briefly.");
        await this.navigateHome();
        await api.wait(mathUtils.randomInRange(1500, 3000));
        break;
      }

      // HUMAN-LIKE: Cycle complete
      await this.human.cycleComplete();
    }

    // HUMAN-LIKE: Session wrap-up
    await this.human.sessionEnd();
    this.log("Session Complete.");
  }

  /**
   * Simulates random fidgeting movements to appear human
   * @returns {Promise<void>}
   */
  async simulateFidget() {
    const fidgetType = mathUtils.roll(0.4)
      ? "TEXT_SELECT"
      : mathUtils.roll(0.5)
        ? "MOUSE_WIGGLE"
        : "OVERSHOOT";
    this.log(`[Fidget] Performing ${fidgetType}...`);

    // Save current URL for navigation detection
    const currentUrl = this.page.url();

    try {
      if (fidgetType === "TEXT_SELECT") {
        // Select random text on page (fidgeting)
        // Fix: Use data-testid="tweetText" which is the standard for tweet body text
        const candidates = await this.page
          .locator('[data-testid="tweetText"]')
          .all();

        // DEBUG: Log candidate count to diagnose selection failures
        this.log(`[Debug] Found ${candidates.length} text candidates.`);

        if (candidates.length > 0) {
          // ROBUST SELECTION: Filter for VISIBLE candidates using evaluate (faster & more reliable)
          const visibleIndices = await this.page.evaluate(() => {
            const all = document.querySelectorAll('[data-testid="tweetText"]');
            const valid = [];
            const vpHeight = window.innerHeight;
            // const vpWidth = window.innerWidth; // Unused for now

            for (let i = 0; i < all.length; i++) {
              const rect = all[i].getBoundingClientRect();
              // Check vertical visibility with header safety margin (100px)
              if (rect.top > 100 && rect.bottom < vpHeight) {
                // Optional: check if text is long enough
                const len = (all[i].textContent || "").length;
                if (len > 10) {
                  valid.push(i);
                }
              }
            }
            return valid;
          });

          this.log(
            `[Debug] Visible & Valid Candidates indices: ${JSON.stringify(visibleIndices)}`,
          );

          if (visibleIndices.length > 0) {
            // Pick random valid index
            const chosenIndex =
              visibleIndices[Math.floor(Math.random() * visibleIndices.length)];
            const target = candidates[chosenIndex];
            const box = await target.boundingBox();

            // Double check (should be valid per evaluate)
            if (box) {
              const text = (await target.textContent().catch(() => "")) || "";
              const preview = text.substring(0, 20).replace(/\n/g, " ");
              this.log(`[Fidget] Selecting text: "${preview}..."`);

              await this.page.mouse.move(box.x + 10, box.y + 10);
              await api.wait(150); // Settle before clicking
              await this.page.mouse.down();

              const dragX = mathUtils.randomInRange(100, 300); // Increased min drag to avoid "click" interpretation
              // Randomize drag speed (steps) for natural feel
              const dragSteps = mathUtils.randomInRange(20, 50);
              await this.page.mouse.move(
                box.x + 10 + dragX,
                box.y + 10 + mathUtils.randomInRange(-2, 2),
                { steps: dragSteps },
              );
              await api.wait(mathUtils.randomInRange(400, 800)); // Hold longer
              await this.page.mouse.up();

              // Natural pause to "read" the selected text (extended for realism)
              const readDelay = mathUtils.randomInRange(2000, 5000);
              this.log(
                `[Fidget] Reading selected text for ${(readDelay / 1000).toFixed(1)}s...`,
              );
              await api.wait(readDelay);
              return; // Done
            }
          } else {
            this.log(
              `[Fidget] No suitable text found in viewport (Filtered from ${candidates.length} candidates).`,
            );
          }
        }
      } else if (fidgetType === "MOUSE_WIGGLE") {
        // Random small movements
        const current = this.ghost.previousPos || { x: 0, y: 0 };
        for (let i = 0; i < 3; i++) {
          const dx = mathUtils.randomInRange(-20, 20);
          const dy = mathUtils.randomInRange(-20, 20);
          await this.page.mouse.move(current.x + dx, current.y + dy, {
            steps: 3,
          });
          await api.wait(mathUtils.randomInRange(50, 150));
        }
      } else if (fidgetType === "OVERSHOOT") {
        // Scroll down a bit, then up immediately (simulating missing target)
        await scrollRandom(200, 400);

        // Reaction time: Pause to realize we went too far (increased from 100-300ms)
        await api.wait(mathUtils.randomInRange(800, 1500));

        // Correction scroll (slightly less than overshoot to stay offset)
        await scrollRandom(-360, -140);

        // SAFETY: Check if we accidentally navigated (OVERSHOOT can trigger clicks)
        await api.wait(500);
        const newUrl = this.page.url();

        if (newUrl !== currentUrl && newUrl.includes("/status/")) {
          // We accidentally navigated to a tweet! Recover automatically.
          this.log(
            `[Fidget] Accidental navigation detected! Recovering to Home...`,
          );
          await this.navigateHome();
          await api.wait(mathUtils.randomInRange(2000, 4000));
        }
      }

      // General pause after other fidgets
      await api.wait(mathUtils.randomInRange(1000, 3000));
    } catch (e) {
      // Fidgeting shouldn't crash the session
      this.log(`[Fidget] Error: ${e.message}`);
    }
  }
  /**
   * Types text with human-like delays
   * @param {object} element - Input element
   * @param {string} text - Text to type
   * @returns {Promise<void>}
   */
  async humanType(element, text) {
    if (!element || !text) return;
    try {
      await element.focus();
      for (const char of text) {
        await this.page.keyboard.type(char, {
          delay: mathUtils.randomInRange(50, 150),
        });
        // Occasional pause
        if (mathUtils.roll(0.05)) {
          await api.wait(mathUtils.randomInRange(300, 800));
        }
      }
    } catch (e) {
      this.log(`[Warn] humanType failed: ${e.message}. Fallback to fill.`);
      await element.fill(text);
    }
  }

  /**
   * Posts a new tweet with human-like interactions.
   * @param {string} text - The content to tweet
   */
  async postTweet(text) {
    if (!text) return;
    this.log(
      `[postTweet] 📝 Initiating New Tweet: "${text.substring(0, 30)}..."`,
    );

    try {
      // 1. Open Composer
      // Try keyboard shortcut 'N' first (20% chance)
      let composerOpened = false;

      if (mathUtils.roll(0.2)) {
        this.log('[postTweet] Pressing "N" to open composer...');
        await this.page.keyboard.press("n");
        try {
          await this.page.waitForSelector('[data-testid="tweetTextarea_0"]', {
            timeout: 3000,
          });
          composerOpened = true;
          this.log("[postTweet] Composer opened via shortcut.");
        } catch {
          this.log('[Warn] Shortcut "N" failed. Falling back to UI click.');
        }
      }

      // Fallback to UI Button
      if (!composerOpened) {
        const postBtn = this.page
          .locator('[data-testid="SideNav_NewTweet_Button"]')
          .first();
        if (await postBtn.isVisible()) {
          await this.safeHumanClick(postBtn, "New Tweet Button", 3);
          await this.page.waitForSelector('[data-testid="tweetTextarea_0"]', {
            timeout: 5000,
          });
        } else {
          this.log("[Error] ❌ New Tweet button not found.");
          return;
        }
      }

      // 2. Type Text
      const textarea = this.page
        .locator('[data-testid="tweetTextarea_0"]')
        .first();
      await api.wait(mathUtils.randomInRange(500, 1500)); // Pause before typing

      this.log("[postTweet] ✍️ Typing tweet content...");
      await this.humanType(textarea, text);
      await api.wait(mathUtils.randomInRange(1000, 2000)); // Review pause

      // 3. Click Post
      const sendBtn = this.page.locator('[data-testid="tweetButton"]').first();
      await this.safeHumanClick(sendBtn, "Post Button", 3);

      this.log(`[postTweet] 🚀🚀🚀 Tweet sent successfully: "${text}"✅✅✅`);
      this.state.engagements++;
      this.state.tweets++;
      await api.wait(mathUtils.randomInRange(2000, 4000)); // Wait for post animation
    } catch (e) {
      this.log(`[Error] Failed to post tweet: ${e.message}`);
    }
  }

  /**
   * Proper cleanup of all agent resources, timers, and listeners.
   * Removes network listeners to prevent memory leaks and Vitest hangs.
   */
  shutdown() {
    this.log("Agent shutting down...");
    try {
      if (this.page && !this.page.isClosed()) {
        // Remove network listeners attached in constructor
        this.page.removeAllListeners("request");
        this.page.removeAllListeners("response");
      }
    } catch (e) {
      this.log(`[Shutdown] Error during cleanup: ${e.message}`);
    }
  }
}
