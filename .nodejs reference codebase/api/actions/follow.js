/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { getPage } from "../core/context.js";
import { createLogger } from "../core/logger.js";
import { mathUtils } from "../utils/math.js";
import { wait } from "../interactions/wait.js";
import { visible } from "../interactions/queries.js";
import { click } from "../interactions/actions.js";
import metricsCollector from "../utils/metrics.js";

const logger = createLogger("api/follow.js");

/**
 * Follow the author of the currently open tweet/profile page.
 *
 * Performs:
 * 1. Already-following guard (unfollow button or text state)
 * 2. Click Follow button
 * 3. Polling verification (up to 5 polls x 1.5s)
 *
 * @param {object} [options]
 * @param {string} [options.username] - For logging purposes only
 * @param {number} [options.maxAttempts=2] - How many click attempts before giving up
 * @returns {Promise<{success: boolean, reason: string, method: string}>}
 */
export async function followWithAPI(options = {}) {
  const page = getPage();
  const { username = "unknown", maxAttempts = 2 } = options;

  logger.info(`Starting api.followWithAPI() for @${username}...`);

  // X.com selectors: follow button uses data-testid ending in "-follow"
  // Note: Button can be either <div role="button"> or <button role="button">
  const followSel = '[role="button"][data-testid$="-follow"]';
  const unfollowSel = '[data-testid$="-unfollow"]';

  try {
    // Already following guard
    logger.info(
      `[followWithAPI] Checking if already following @${username}...`,
    );
    if (await visible(unfollowSel)) {
      logger.info(`Already following @${username}.`);
      return {
        success: true,
        reason: "already_following",
        method: "followAPI",
      };
    }

    logger.info(`[followWithAPI] Finding follow button...`);

    // Check if follow button is visible first (avoid 30s timeout on hidden elements)
    if (!(await visible(followSel))) {
      logger.error(`[followWithAPI] Follow button not visible on page`);
      return {
        success: false,
        reason: "button_not_visible",
        method: "followAPI",
      };
    }

    const followBtn = page.locator(followSel).first();
    logger.info(`[followWithAPI] Getting button text...`);
    // Use timeout to avoid hanging on problematic elements
    const btnText = (
      await followBtn.textContent({ timeout: 5000 }).catch(() => "")
    ).toLowerCase();
    logger.info(`[followWithAPI] Button text: "${btnText}"`);
    if (btnText.includes("following") || btnText.includes("pending")) {
      logger.info(`Already following @${username} (state: ${btnText}).`);
      return {
        success: true,
        reason: "already_following",
        method: "followAPI",
      };
    }

    // Attempt clicks
    for (let attempt = 1; attempt <= maxAttempts; attempt++) {
      logger.info(
        `[followWithAPI] Click attempt ${attempt}/${maxAttempts} (ghost cursor)...`,
      );

      await click(followSel);

      // Poll for state change
      let verified = false;
      for (let poll = 0; poll < 5; poll++) {
        await wait(1500);
        if (await visible(unfollowSel)) {
          verified = true;
          break;
        }
        const txt = (
          await followBtn.textContent().catch(() => "")
        ).toLowerCase();
        if (txt.includes("following") || txt.includes("pending")) {
          verified = true;
          break;
        }
      }

      if (verified) {
        logger.info(
          `✅ api.followWithAPI: successfully followed @${username}!`,
        );
        metricsCollector.recordSocialAction("follow", 1);
        return { success: true, reason: "success", method: "followAPI" };
      }

      logger.warn(`[followWithAPI] Verification failed on attempt ${attempt}.`);
      if (attempt < maxAttempts) {
        await wait(mathUtils.randomInRange(3000, 6000));
      }
    }

    logger.error(`❌ api.followWithAPI: failed after ${maxAttempts} attempts`);
    return {
      success: false,
      reason: "verification_failed",
      method: "followAPI",
    };
  } catch (error) {
    logger.error(`api.followWithAPI error: ${error.message}`);
    return { success: false, reason: error.message, method: "followAPI" };
  }
}
