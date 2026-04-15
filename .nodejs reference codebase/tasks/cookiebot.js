/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview A task that navigates to a random URL from a predefined list loaded from a file.
 * @module tasks/cookieBotRandom
 */

import { api } from "../api/index.js";
import { createLogger } from "../api/core/logger.js";
import fs from "fs/promises";
import path from "path";
import { fileURLToPath } from "url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));

const CONFIG = {
  sitesFile: "../config/popularsite.txt",

  taskTimeoutMs: 240000,
  navigationTimeout: 60000,
  responsivenessTimeout: 15000,

  loopCountMin: 5,
  loopCountMax: 10,

  minReadSecond: 3,
  maxReadSecond: 6,

  scrollPausesMin: 4,
  scrollPausesMax: 8,
  scrollAmountMin: 300,
  scrollAmountMax: 600,

  postLoadDelay: 1000,
  postScrollDelay: 1000,
};

const SITES_FILE = path.join(__dirname, CONFIG.sitesFile);

let urls = [];

// Read the list of sites from the external file when the module is first loaded.
try {
  const data = await fs.readFile(SITES_FILE, "utf8");
  urls = data
    .split("\n")
    .map((line) => {
      let url = line.trim();
      if (url.startsWith("http_")) {
        url = url.replace("http_", "http:");
      }
      if (url.startsWith("https_")) {
        url = url.replace("https_", "https:");
      }
      return url;
    })
    .filter((line) => line.startsWith("http"));

  if (urls.length === 0) {
    console.error(
      "[cookieBotRandom.js] Warning: popularsite.txt was read, but it is empty or contains no valid URLs.",
    );
  }
} catch (error) {
  console.error(
    `[cookieBotRandom.js] CRITICAL: Failed to read site list from ${SITES_FILE}. The task will not be able to run. Error: ${error.message}`,
  );
}

/**
 * An automation task that navigates to a random URL.
 * @param {object} page - The Playwright page object.
 * @param {object} payload - The payload data for the task.
 * @param {string} payload.browserInfo - A unique identifier for the browser.
 */
export default async function cookieBotRandom(page, payload) {
  const startTime = process.hrtime.bigint();
  const browserInfo = payload.browserInfo || "unknown_profile";
  const logger = createLogger("cookiebot.js");

  try {
    // Use a Promise.race to enforce a global timeout for the "work" phase
    await Promise.race([
      api.withPage(
        page,
        async () => {
          await api.init(page, {
            logger,
            lite: true,
            blockNotifications: true,
            blockDialogs: true,
            autoBanners: false,
            muteAudio: true,
          });

          // ─── Block Video Load ──────────────────────────────────────────
          await page.route("**/*", (route) => {
            const request = route.request();
            const type = request.resourceType();
            const url = request.url();

            // Block media resource types (video/audio streams)
            // Also block common video domains to prevent script-based loading
            if (
              type === "media" ||
              /video|googlevideo|youtube|vimeo|twitch|tiktok|dailymotion|stream/i.test(
                url,
              )
            ) {
              return route.abort().catch(() => {});
            }
            return route.fallback().catch(() => {});
          });
          // ───────────────────────────────────────────────────────────────

          logger.info(`URL list size: ${urls.length}`);
          if (urls.length === 0) {
            logger.error(
              "URL list from popularsite.txt is empty or failed to load. Aborting task.",
            );
            return;
          }

          const loopCount = api.randomInRange(
            CONFIG.loopCountMin,
            CONFIG.loopCountMax,
          );
          logger.info(`Starting random visits loop for ${loopCount} times.`);

          const abortSignal = payload.abortSignal;

          for (let i = 0; i < loopCount; i++) {
            if (page.isClosed() || abortSignal?.aborted) break;

            const randomUrl = urls[Math.floor(Math.random() * urls.length)];
            logger.info(
              `(${i + 1} of ${loopCount}) Navigating to: ${randomUrl}`,
            );

            try {
              // 1. Navigate with a timeout
              await api.goto(randomUrl, {
                waitUntil: "domcontentloaded",
                timeout: CONFIG.navigationTimeout,
              });

              // 2. Check responsiveness
              try {
                await api.waitFor(
                  async () => {
                    return await api.eval(() => true).catch(() => false);
                  },
                  { timeout: CONFIG.responsivenessTimeout },
                );
              } catch (_e) {
                logger.warn(
                  `Page ${randomUrl} is unresponsive after load. Skipping.`,
                );
                continue;
              }
              await api.wait(CONFIG.postLoadDelay);

              // 3. Scroll/Read
              const domain = new URL(randomUrl).hostname;
              logger.info(`${domain} init scrolling`);
              const scrollStart = Date.now();
              await api.scroll.read(null, {
                pauses: api.randomInRange(
                  CONFIG.scrollPausesMin,
                  CONFIG.scrollPausesMax,
                ),
                scrollAmount: api.randomInRange(
                  CONFIG.scrollAmountMin,
                  CONFIG.scrollAmountMax,
                ),
              });
              const scrollDuration = (
                (Date.now() - scrollStart) /
                1000
              ).toFixed(1);
              logger.info(`${domain} scrolled for ${scrollDuration}s`);

              await api.wait(CONFIG.postScrollDelay);
              const readTime = (
                api.randomInRange(
                  CONFIG.minReadSecond * 10,
                  CONFIG.maxReadSecond * 10,
                ) / 10
              ).toFixed(1);
              logger.info(`${domain} init simulate reading for ${readTime} s`);
              await api.wait(parseFloat(readTime) * 1000);
            } catch (navError) {
              if (
                navError.message.includes(
                  "interrupted by another navigation",
                ) ||
                navError.message.includes("Session closed") ||
                navError.message.includes("Page has been closed")
              ) {
                logger.warn(
                  `Navigation to ${randomUrl} was interrupted (or page closed).`,
                );
                break; // Stop the loop if the page or session is gone
              } else if (
                navError.message.includes("timeout") ||
                navError.message.includes("Timeout")
              ) {
                logger.warn(
                  `Visit to ${randomUrl} timed out. Skipping to next.`,
                );
              } else if (navError.message.includes("net::ERR_")) {
                logger.warn(`Network error visiting ${randomUrl}`);
              } else {
                logger.error(
                  `Failed to load ${randomUrl}:`,
                  navError.message.split("\n")[0],
                );
                if (page.isClosed()) break;
              }
            }
          }
        },
        { taskName: "cookiebot", sessionId: browserInfo },
      ),
      new Promise((_, reject) =>
        setTimeout(
          () =>
            reject(
              new Error(
                `Cookiebot task exceeded ${CONFIG.taskTimeoutMs}ms limit`,
              ),
            ),
          CONFIG.taskTimeoutMs,
        ),
      ),
    ]);
  } catch (error) {
    if (error.message.includes("exceeded") && error.message.includes("limit")) {
      logger.warn(`Task forced to stop: ${error.message}`);
    } else if (
      error.message.includes("Target page, context or browser has been closed")
    ) {
      logger.warn(`Task interrupted: Browser/Page closed.`);
    } else {
      logger.error(`### CRITICAL ERROR in main task loop:`, error);
    }
  } finally {
    try {
      if (page && !page.isClosed()) {
        await Promise.race([
          page.close(),
          new Promise((r) => setTimeout(r, 5000)),
        ]);
        logger.debug(`Page closed successfully.`);
      }
    } catch (closeError) {
      logger.warn(`Error closing page: ${closeError.message}`);
    }
    const endTime = process.hrtime.bigint();
    const durationInSeconds = (
      Number(endTime - startTime) / 1_000_000_000
    ).toFixed(2);
    logger.info(`Total task duration: ${durationInSeconds} seconds`);
  }
}
