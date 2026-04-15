/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { getPage } from "../core/context.js";
import { createLogger } from "../core/logger.js";
import { wait, waitForLoadState } from "../interactions/wait.js";
import { click } from "../interactions/actions.js";
import { back, goto } from "../interactions/navigation.js";
import { visible } from "../interactions/queries.js";
import { mathUtils } from "../utils/math.js";

const logger = createLogger("api/twitter/intent-retweet.js");

/**
 * Navigates to the Twitter retweet intent URL and confirms the action.
 * @param {string} url - The URL of the tweet to retweet
 * @returns {Promise<{success: boolean, reason?: string}>}
 * @example
 * await api.twitter.intent.retweet('https://x.com/user/status/123456789');
 */
export async function retweet(url) {
  const timeoutPromise = new Promise((resolve) =>
    setTimeout(() => resolve({ success: false, reason: "timeout" }), 20000),
  );

  const actionPromise = (async () => {
    let navigated = false;
    try {
      getPage(); // ensure context
      logger.info(`Extracting tweet ID from ${url}`);

      const match = url.match(/\/status\/(\d+)/);
      if (!match) {
        logger.error("Invalid tweet URL format");
        return { success: false, reason: "invalid_tweet_url" };
      }
      const tweetId = match[1];

      const intentUrl = `https://x.com/intent/retweet?tweet_id=${tweetId}`;
      logger.info(`Navigating to intent: ${intentUrl}`);

      navigated = true;
      await goto(intentUrl);
      await waitForLoadState("domcontentloaded");
      await wait(mathUtils.randomInRange(1500, 3000));

      const confirmBtn = '[data-testid="confirmationSheetConfirm"]';

      if (await visible(confirmBtn)) {
        logger.info("Clicking confirm button");
        await click(confirmBtn);
        await wait(mathUtils.randomInRange(3000, 5000));
        return { success: true };
      } else {
        logger.warn("Confirm button not found. May already be retweeted.");
        return { success: false, reason: "confirm_button_not_found" };
      }
    } catch (error) {
      logger.error(`Unhandled error in retweet intent: ${error.message}`);
      return {
        success: false,
        reason: "unhandled_error",
        error: error.message,
      };
    } finally {
      if (navigated) {
        logger.info("Returning to previous page");
        await back().catch(() => {});
      }
    }
  })();

  return Promise.race([actionPromise, timeoutPromise]);
}
