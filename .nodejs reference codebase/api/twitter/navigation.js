/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * Twitter Navigation Module
 * Reusable navigation and reading functions for Twitter/X
 * @module api/twitter/navigation
 */

import { api } from "../index.js";
import { mathUtils } from "../utils/math.js";
import { createLogger } from "../core/logger.js";

const logger = createLogger("api/twitter/navigation.js");

// Navigation selectors
const HOME_BUTTON_SELECTOR = '[data-testid="AppTabBar_Home_Link"]';
const X_LOGO_SELECTOR = '[aria-label="X"]';

// Reading configuration
const HOME_READ_MIN_MS = 30000; // 30 seconds
const HOME_READ_MAX_MS = 60000; // 60 seconds
const HOME_HYDRATION_MS = 3000;

/**
 * Navigate to home feed by clicking the home button.
 * Falls back to X logo or direct navigation if needed.
 * @param {object} [options] - Navigation options
 * @param {boolean} [options.readFeed=true] - Whether to simulate reading after navigation
 * @param {number} [options.readDurationMs] - Custom read duration in ms (default: 30-60s random)
 * @returns {Promise<{success: boolean, reason: string}>}
 */
export async function home(options = {}) {
  const { readFeed = true, readDurationMs = null } = options;

  logger.info("Navigating to home feed...");

  // Try to click Home button
  const homeBtn = api.getPage().locator(HOME_BUTTON_SELECTOR).first();

  try {
    if (await api.visible(homeBtn).catch(() => false)) {
      logger.info("Clicking Home button...");
      await api.click(homeBtn);
      await api.wait(mathUtils.randomInRange(800, 1500));

      // Verify navigation
      const currentUrl = await api.getCurrentUrl();
      if (
        currentUrl.includes("/home") ||
        currentUrl === "https://x.com/" ||
        currentUrl === "https://x.com"
      ) {
        logger.info("Successfully navigated to home via button click.");
      } else {
        logger.info(
          "Home button click may not have navigated, waiting for page...",
        );
        await api.waitForURL("**/home**", { timeout: 5000 }).catch(() => {});
      }
    } else {
      // Fallback: try X logo
      const xLogo = api.getPage().locator(X_LOGO_SELECTOR).first();
      if (await api.visible(xLogo).catch(() => false)) {
        logger.info("Home button not visible, clicking X logo...");
        await api.click(xLogo);
        await api.wait(mathUtils.randomInRange(800, 1500));
      } else {
        // Fallback: direct navigation
        logger.info("No navigation buttons visible, using direct URL...");
        await api.goto("https://x.com/home", { waitUntil: "domcontentloaded" });
      }
    }
  } catch (navError) {
    logger.warn(
      `Navigation interaction failed: ${navError.message}. Using direct URL...`,
    );
    await api.goto("https://x.com/home", { waitUntil: "domcontentloaded" });
  }

  // Wait for page hydration
  logger.info(`Waiting ${HOME_HYDRATION_MS}ms for page hydration...`);
  await api.wait(HOME_HYDRATION_MS);

  // Simulate reading home feed
  if (readFeed) {
    const duration =
      readDurationMs ||
      mathUtils.randomInRange(HOME_READ_MIN_MS, HOME_READ_MAX_MS);
    const durationMin = (duration / 60000).toFixed(1);
    logger.info(`Simulating home feed reading for ${durationMin} minutes...`);

    const readStart = Date.now();
    while (Date.now() - readStart < duration) {
      // Use api.scroll.read for human-like scrolling with pauses
      await api.scroll.read(null, {
        pauses: mathUtils.randomInRange(1, 3),
        scrollAmount: mathUtils.randomInRange(200, 600),
      });

      // Random pause between read cycles (simulates engagement)
      await api.wait(mathUtils.randomInRange(1000, 4000));
    }

    logger.info("Finished reading home feed.");
  }

  return { success: true, reason: "home_navigated" };
}

/**
 * Check if currently on home page
 * @returns {Promise<boolean>}
 */
export async function isOnHome() {
  const url = await api.getCurrentUrl();
  return (
    url.includes("/home") || url === "https://x.com/" || url === "https://x.com"
  );
}

export default {
  home,
  isOnHome,
};
