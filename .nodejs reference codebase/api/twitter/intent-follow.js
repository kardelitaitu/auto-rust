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

const logger = createLogger("api/twitter/intent-follow.js");

/**
 * Navigates to the Twitter follow intent URL and confirms the action.
 * @param {string} url - The URL of the user to follow (e.g., https://x.com/username)
 * @returns {Promise<{success: boolean, reason?: string}>}
 * @example
 * await api.twitter.intent.follow('https://x.com/elonmusk');
 */
export async function follow(url) {
  const timeoutPromise = new Promise((resolve) =>
    setTimeout(() => resolve({ success: false, reason: "timeout" }), 20000),
  );

  const actionPromise = (async () => {
    let navigated = false;
    try {
      getPage();
      logger.info(`Extracting username from ${url}`);

      let screenName = "";
      if (url.includes("x.com/") || url.includes("twitter.com/")) {
        // Remove protocol and domain to get pathname
        const pathname = url.split(/\.com\//)[1] || "";
        const parts = pathname.split("/").filter(Boolean);
        if (parts.length > 0) {
          screenName = parts[0];
        }
      } else if (url.startsWith("@")) {
        screenName = url.slice(1).split("/")[0];
      }

      if (
        !screenName ||
        [
          "intent",
          "home",
          "explore",
          "messages",
          "notifications",
          "search",
          "settings",
        ].includes(screenName.toLowerCase())
      ) {
        logger.error(`Invalid screenName extracted: ${screenName}`);
        return { success: false, reason: "invalid_username" };
      }

      const intentUrl = `https://x.com/intent/follow?screen_name=${screenName}`;
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
        logger.warn("Confirm button not found. May already be following.");
        return { success: false, reason: "confirm_button_not_found" };
      }
    } catch (error) {
      logger.error(`Unhandled error in follow intent: ${error.message}`);
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
