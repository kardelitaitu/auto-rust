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

const logger = createLogger("api/twitter/intent-post.js");

/**
 * Navigates to the Twitter intent URL to compose a new tweet.
 * @param {string} text - The text to include in the tweet
 * @returns {Promise<{success: boolean, reason?: string}>}
 * @example
 * await api.twitter.intent.post('Hello world! 🚀');
 */
export async function post(text) {
  const timeoutPromise = new Promise((resolve) =>
    setTimeout(() => resolve({ success: false, reason: "timeout" }), 20000),
  );

  const actionPromise = (async () => {
    let navigated = false;
    try {
      if (!text) {
        logger.error("Missing text for post intent");
        return { success: false, reason: "missing_parameters" };
      }

      getPage();
      const intentUrl = `https://x.com/intent/tweet?text=${encodeURIComponent(text)}`;
      logger.info(`Navigating to intent: ${intentUrl}`);

      navigated = true;
      await goto(intentUrl);
      await waitForLoadState("domcontentloaded");
      await wait(mathUtils.randomInRange(1500, 3000));

      const tweetBtn = '[data-testid="tweetButton"]';

      if (await visible(tweetBtn)) {
        logger.info("Clicking tweet button");
        await click(tweetBtn);
        await wait(mathUtils.randomInRange(3000, 5000));
        return { success: true };
      } else {
        logger.error("Tweet button not found");
        return { success: false, reason: "tweet_button_not_found" };
      }
    } catch (error) {
      logger.error(`Unhandled error in post intent: ${error.message}`);
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
