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

const logger = createLogger("api/twitter/intent-quote.js");

/**
 * Navigates to the Twitter intent URL to quote a tweet.
 * @param {string} url - The URL of the tweet to quote
 * @param {string} text - The text to include in the quote
 * @returns {Promise<{success: boolean, reason?: string}>}
 * @example
 * await api.twitter.intent.quote('https://x.com/user/status/123', 'Great tweet!');
 * or
 * const myQuote = "Check this out!\n\nIt's a multi-line quote.";
 * await api.twitter.intent.quote(url, myQuote);
 */
export async function quote(url, text) {
  const timeoutPromise = new Promise((resolve) =>
    setTimeout(() => resolve({ success: false, reason: "timeout" }), 20000),
  );

  const actionPromise = (async () => {
    let navigated = false;
    try {
      if (!url || !text) {
        logger.error("Missing URL or text for quote intent");
        return { success: false, reason: "missing_parameters" };
      }

      getPage();
      logger.info(`Extracting tweet ID from ${url}`);

      const match = url.match(/\/status\/(\d+)/);
      if (!match) {
        logger.error("Invalid tweet URL format");
        return { success: false, reason: "invalid_tweet_url" };
      }
      const _tweetId = match[1];

      const intentUrl = `https://x.com/intent/tweet?url=${encodeURIComponent(url)}&text=${encodeURIComponent(text)}`;
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
      logger.error(`Unhandled error in quote intent: ${error.message}`);
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
