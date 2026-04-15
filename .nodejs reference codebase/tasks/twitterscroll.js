/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview A task that navigates to Twitter/X home and performs random scrolling.
 * @module tasks/twitterscroll
 */

import { createRandomScroller } from "../api/utils/randomScrolling.js";
import { createLogger } from "../api/core/logger.js";

/**
 * An automation task that navigates to Twitter/X and scrolls.
 * @param {object} page - The Playwright page object.
 * @param {object} payload - The payload data for the task.
 * @param {string} payload.browserInfo - A unique identifier for the browser.
 */
export default async function twitterscroll(page, payload) {
  const startTime = process.hrtime.bigint();
  const delay = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
  const browserInfo = payload.browserInfo || "unknown_profile";
  const logger = createLogger(`twitterscroll.js [${browserInfo}]`);
  logger.info(`Starting task...`);

  // Apply visibility spoofing
  const { applyHumanizationPatch } =
    await import("../api/utils/browserPatch.js");
  await applyHumanizationPatch(page, logger);

  const randomScrolling = createRandomScroller(page);

  try {
    // Navigate to Twitter/X home
    logger.info(`[TwitterScroll] Navigating to https://x.com/home`);
    await page.goto("https://x.com/home", {
      waitUntil: "domcontentloaded",
      timeout: 60000, // 60 seconds
    });

    // Wait random 5-20 seconds
    const waitTime = Math.floor(Math.random() * (20 - 5 + 1)) + 5; // 5-20 seconds
    // logger.info(`[TwitterScroll] Waiting ${waitTime} seconds before scrolling`);
    await delay(waitTime * 1000);

    // Random scrolling 10-600 seconds
    const scrollDuration = Math.random() * 590 + 10; // 10-600 seconds
    // logger.info(
    //     `[TwitterScroll] Starting random scrolling for ${scrollDuration.toFixed(2)} seconds`
    // );
    await randomScrolling(scrollDuration);

    logger.info(`[TwitterScroll] Task completed successfully`);
  } catch (error) {
    if (
      error.message.includes("Target page, context or browser has been closed")
    ) {
      logger.warn(
        `[TwitterScroll] Task interrupted: Browser/Page closed (likely Ctrl+C).`,
      );
    } else {
      logger.error(`[TwitterScroll] ### CRITICAL ERROR:`, error);
    }
  } finally {
    // logger.info(`[TwitterScroll] --- Reached FINALLY block ---`);
    try {
      if (page && !page.isClosed()) {
        logger.debug(`Page is open. Attempting page.close()...`);
        await page.close();
        logger.debug(`page.close() command EXECUTED.`);
      } else {
        logger.debug(`Page was already closed or not created.`);
      }
    } catch (closeError) {
      logger.error(`### CRITICAL ERROR trying to close page:`, closeError);
    }
    const endTime = process.hrtime.bigint();
    const durationInSeconds = (
      Number(endTime - startTime) / 1_000_000_000
    ).toFixed(2);
    logger.info(`Finished task. Task duration: ${durationInSeconds} seconds.`);
  }
}
