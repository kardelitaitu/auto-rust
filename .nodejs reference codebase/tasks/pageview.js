/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { api } from "../api/index.js";
import { createLogger } from "../api/core/logger.js";
import { profileManager } from "../api/utils/profileManager.js";
import { ReferrerEngine } from "../api/utils/urlReferrer.js";
import { createSuccessResult, createFailedResult } from "../api/core/task-result.js";
import fs from "fs/promises";

const URL_FILE = "./tasks/pageview.txt";

/**
 * Load URLs from the text file
 */
async function loadUrls() {
  try {
    const content = await fs.readFile(URL_FILE, "utf-8");
    return content
      .split("\n")
      .map((line) => line.trim())
      .filter((line) => line && !line.startsWith("#"));
  } catch (_error) {
    return [];
  }
}

/**
 * Get a random URL from the file
 */
async function getRandomUrl() {
  const urls = await loadUrls();
  if (urls.length === 0) {
    throw new Error(`No URLs found in ${URL_FILE}`);
  }
  return urls[Math.floor(Math.random() * urls.length)];
}

/**
 * Ensure URL has proper protocol
 */
function ensureProtocol(url) {
  url = url.trim();
  if (!url.startsWith("http://") && !url.startsWith("https://")) {
    url = "https://" + url;
  }
  return url;
}

/**
 * Main pageview task - Returns structured result
 * @param {object} page - Playwright page instance
 * @param {object} payload - Task payload
 * @returns {Promise<Object>} Task result with status, data, and error
 */
export default async function pageview(page, payload) {
  const startTime = Date.now();
  const browserInfo = payload.browserInfo || "unknown_profile";
  const logger = createLogger("pageview.js");
  let targetUrl = null;

  logger.info("Starting pageview task...");

  try {
    return await api.withPage(
      page,
      async () => {
        // 1. Setup Profile & Persona
        let profile;
        try {
          profile = profileManager.getStarter();
          await api.init(page, {
            logger,
            persona: profile.persona || "casual",
            colorScheme: profile.theme || "dark",
          });
        } catch (e) {
          logger.warn(`Profile load failed: ${e.message}, using defaults`);
          await api.init(page, { logger, colorScheme: "dark" });
        }

        // 2. Determine target URL
        if (payload.url) {
          targetUrl = ensureProtocol(payload.url);
        } else {
          targetUrl = await getRandomUrl();
        }
        logger.info(`Target: ${targetUrl}`);

        // 3. Referrer
        const engine = new ReferrerEngine({ addUTM: false });
        const ctx = engine.generateContext(targetUrl);

        // 4. Navigation
        const taskTimeoutMs = 50000;
        
        await Promise.race([
          (async () => {
            await api.goto(targetUrl, {
              waitUntil: "domcontentloaded",
              timeout: 20000,
              warmup: true,
              warmupMouse: true,
              warmupPause: true,
              referer: ctx.referrer || undefined,
            });

            await api.wait(api.randomInRange(1000, 2000));

            // 5. Reading Simulation
            const readingMs = api.gaussian(
              profile?.timings?.readingPhase?.mean || 30000,
              profile?.timings?.readingPhase?.deviation || 10000,
              10000, 45000
            );
            const readingS = Math.min(Math.max(readingMs / 1000, 15), 45);
            const pauses = Math.max(1, Math.floor(readingS / 2.2));

            await api.scroll.read(null, {
              pauses,
              scrollAmount: api.randomInRange(600, 1200),
              variableSpeed: true,
              backScroll: true,
            });

            await api.wait(api.randomInRange(1000, 2000));
          })(),
          new Promise((_, reject) =>
            setTimeout(() => reject(new Error("Pageview timeout")), taskTimeoutMs)
          ),
        ]);

        return createSuccessResult('pageview', {
          url: targetUrl,
          referrer: ctx.referrer
        }, { startTime, sessionId: browserInfo });
      },
      { taskName: "pageview", sessionId: browserInfo }
    );
  } catch (error) {
    logger.error(`Pageview error: ${error.message}`);
    return createFailedResult('pageview', error, {
      partialData: { url: targetUrl },
      sessionId: browserInfo
    });
  }
}
