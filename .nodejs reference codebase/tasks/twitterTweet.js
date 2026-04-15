/**
 * @fileoverview Twitter Tweet Task using Unified API
 * Full feature parity with twitterTweet.js using api/ modules.
 * @module tasks/api-twitterTweet
 */

import fs from "fs";
import path from "path";
import { fileURLToPath } from "url";
import { api } from "../api/index.js";
import { profileManager } from "../utils/profileManager.js";
import { mathUtils } from "../utils/mathUtils.js";
import metricsCollector from "../utils/metrics.js";
import { createLogger } from "../utils/logger.js";
import { takeScreenshot } from "../utils/screenshot.js";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

// --- CONFIGURATION ---
const TWEET_SOURCE_FILE = path.join(__dirname, "twitterTweet.txt");
const DEFAULT_TASK_TIMEOUT_MS = 6 * 60 * 1000; // 6 Minutes (Robustness takes time)

/**
 * Executes the Tweet Task using Unified API.
 * @param {object} page - The Playwright page instance.
 * @param {object} payload
 * @param {string} [payload.profileId]
 * @param {number} [payload.taskTimeoutMs]
 */
export default async function apiTwitterTweetTask(page, payload) {
  const browserInfo = payload.browserInfo || "unknown_profile";
  const logger = createLogger(`api-twitterTweet [${browserInfo}]`);
  const taskTimeoutMs = payload.taskTimeoutMs || DEFAULT_TASK_TIMEOUT_MS;

  logger.info(`[api-twitterTweet] Starting Unified API Tweet Task...`);

  try {
    await Promise.race([
      (async () => {
        // 1. Initialize API Context
        await api.init(page, { logger });
        // Note: Page context should be set via orchestrator's api.withPage()

        let profile;
        if (payload.profileId) {
          try {
            profile = profileManager.getById(payload.profileId);
          } catch (_e) {
            profile = profileManager.getStarter();
          }
        } else {
          profile = profileManager.getStarter();
        }

        // Persona Mapping
        const personaName = profile.persona || "casual";
        await api.setPersona(personaName);
        logger.info(`[api-twitterTweet] Using persona: ${personaName}`);

        if (profile.theme) {
          await api.emulateMedia({ colorScheme: profile.theme });
        }

        // 2. Navigate Home with Warmup
        logger.info(`[api-twitterTweet] Navigating Home...`);
        await api.goto("https://x.com", {
          waitUntil: "domcontentloaded",
          warmup: true,
          warmupMouse: true,
          warmupPause: true,
        });

        // 3. Warm-up Reading (30-45s)
        // logger.info(`[api-twitterTweet] Warm-up reading phase...`);
        const warmupTime = mathUtils.randomInRange(30000, 45000);
        const startTime = Date.now();
        while (Date.now() - startTime < warmupTime) {
          await api.scroll.read("body", { pauses: 1 });
          await api.think(mathUtils.randomInRange(2000, 5000));
        }

        // 4. Prepare Tweet Content
        // logger.info(`[api-twitterTweet] Preparing tweet content...`);
        let tweetLine;
        try {
          if (!fs.existsSync(TWEET_SOURCE_FILE)) {
            throw new Error(`Source file not found: ${TWEET_SOURCE_FILE}`);
          }
          const content = fs.readFileSync(TWEET_SOURCE_FILE, "utf-8");
          const lines = content
            .split(/\r?\n/)
            .filter((line) => line.trim().length > 0);

          if (lines.length === 0) {
            throw new Error(`Source file is empty: ${TWEET_SOURCE_FILE}`);
          }

          tweetLine = lines[0];
          const remainingLines = lines.slice(1);

          // Decode multi-line tweets
          tweetLine = tweetLine.replace(/\\n/g, "\n");

          fs.writeFileSync(
            TWEET_SOURCE_FILE,
            remainingLines.join("\n"),
            "utf-8",
          );
          // logger.info(`[api-twitterTweet] Picked tweet: "${tweetLine.substring(0, 30).replace(/\n/g, ' ')}..."`);
        } catch (fileError) {
          logger.error(`[api-twitterTweet] File Error: ${fileError.message}`);
          throw fileError;
        }

        // 5. Open Composer
        // logger.info(`[api-twitterTweet] Opening Composer...`);
        const sideNavTweetBtn = 'a[data-testid="SideNav_NewTweet_Button"]';
        const inlineComposeInput = 'div[data-testid="tweetTextarea_0"]';

        let composerOpen = false;
        // Strategy 1: Side Nav Button (if visible)
        if (await api.visible(sideNavTweetBtn)) {
          await api.click(sideNavTweetBtn);
        } else {
          // Strategy 2: Keyboard Shortcut
          // logger.info(`[api-twitterTweet] Using keyboard shortcut 'n'...`);
          await page.keyboard.press("n");
        }

        // Wait for input area
        try {
          await api.waitVisible(inlineComposeInput, { timeout: 10000 });
          composerOpen = true;
          // logger.info(`[api-twitterTweet] Composer area visible.`);
        } catch (_e) {
          logger.warn(
            `[api-twitterTweet] Composer did not appear via standard UI. Retrying 'n'...`,
          );
          await page.keyboard.press("n");
          await api
            .waitVisible(inlineComposeInput, { timeout: 5000 })
            .then(() => (composerOpen = true))
            .catch(() => {});
        }

        if (!composerOpen) {
          throw new Error("Fatal: Could not open tweet composer.");
        }

        // 6. Type Content
        // logger.info(`[api-twitterTweet] Typing content...`);
        // Use api.type which handles Golden View focus, typos, and human timing
        await api.type(inlineComposeInput, tweetLine, { clearFirst: false });

        // Add a trailing space (common human behavior)
        await page.keyboard.type(" ", {
          delay: mathUtils.randomInRange(50, 150),
        });
        await api.think(mathUtils.randomInRange(1500, 3500)); // Review pause

        // 7. Post
        // logger.info(`[api-twitterTweet] Clicking Post button...`);
        const postBtn =
          'button[data-testid="tweetButton"], button[data-testid="tweetButtonInline"]';

        const clickResult = await api.click(postBtn, { recovery: true });

        if (clickResult.success) {
          // logger.info(`[api-twitterTweet] Post click executed.`);
          // Wait for button to disappear or success state
          try {
            await api.waitHidden(postBtn, { timeout: 8000 });
            logger.info(`[api-twitterTweet] ✅ Tweet successful!`);
            metricsCollector.recordSocialAction("tweet", 1);
            await takeScreenshot(page, `api-tweet-success`);
          } catch (_e) {
            logger.warn(
              `[api-twitterTweet] Post button still visible, checking URL or modal state...`,
            );
          }
        } else {
          throw new Error("Failed to click Post button.");
        }

        // 8. Cool-down Reading (1-2 mins)
        // logger.info(`[api-twitterTweet] Cool-down reading (1-2 mins)...`);
        const cooldownTime = mathUtils.randomInRange(60000, 120000);
        const cdStart = Date.now();
        while (Date.now() - cdStart < cooldownTime) {
          await api.scroll.read("body", { pauses: 1 });
          await api.think(mathUtils.randomInRange(3000, 8000));
        }

        logger.info(`[api-twitterTweet] Task finished successfully.`);
      })(),
      new Promise((_, reject) =>
        setTimeout(
          () => reject(new Error("Global Task Timeout")),
          taskTimeoutMs,
        ),
      ),
    ]);
  } catch (error) {
    logger.error(`[api-twitterTweet] Fatal Task Error: ${error.message}`);
  } finally {
    try {
      if (page && !page.isClosed()) {
        await Promise.race([
          page.close(),
          new Promise((r) => setTimeout(r, 5000)),
        ]);
      }
    } catch (_ce) {
      void _ce;
    }
  }
}
