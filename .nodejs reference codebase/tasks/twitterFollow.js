/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Twitter Follow Task - Returns structured result
 * @module tasks/twitterFollow.js
 */
import { createLogger } from "../api/core/logger.js";
import { TwitterAgent } from "../api/twitter/twitterAgent.js";
import { profileManager } from "../api/utils/profileManager.js";
import { mathUtils } from "../api/utils/math.js";
import { ReferrerEngine } from "../api/utils/urlReferrer.js";
import metricsCollector from "../api/utils/metrics.js";
import { api } from "../api/index.js";
import { takeScreenshot } from "../api/utils/screenshot.js";
import {
  createSuccessResult,
  createFailedResult,
} from "../api/core/task-result.js";

// Helper: Extract username from tweet URL
function extractUsername(tweetUrl) {
  try {
    const url = new URL(tweetUrl);
    const pathParts = url.pathname.split("/").filter((p) => p.length > 0);
    if (pathParts.length >= 1) {
      return "@" + pathParts[0];
    }
  } catch (_e) {
    return "(unknown)";
  }
  return "(unknown)";
}

/**
 * Executes the Follow Task.
 * @param {object} page - The Playwright page instance.
 * @param {any} payload
 * @returns {Promise<Object>} Task result with status, data, and error
 */
export default async function twitterFollowTask(page, payload) {
  const startTime = Date.now();
  const browserInfo = payload.browserInfo || "unknown_profile";
  const logger = createLogger(`twitterFollowTask [${browserInfo}]`);
  const DEFAULT_TASK_TIMEOUT_MS = 3 * 60 * 1000;
  const taskTimeoutMs = payload.taskTimeoutMs || DEFAULT_TASK_TIMEOUT_MS;
  const TARGET_TWEET_URL = "https://x.com/_nadiku/status/1998218314703852013";

  logger.info(
    `[twitterFollow] Initializing (Timeout: ${(taskTimeoutMs / 1000 / 60).toFixed(1)}m)`,
  );

  let agent;
  let sessionStart;
  let followedUsername = null;

  try {
    await Promise.race([
      (async () => {
        sessionStart = Date.now();

        // 1. Initialize Agent
        let profile = payload.profileId
          ? profileManager.getById(payload.profileId) ||
            profileManager.getStarter()
          : profileManager.getStarter();

        agent = new TwitterAgent(page, profile, logger);
        sessionStart = agent.sessionStart;

        if (profile.theme)
          await api.emulateMedia({ colorScheme: profile.theme });

        // 2. Warm-up jitter
        await api.wait(mathUtils.randomInRange(2000, 8000));

        // 3. Navigation
        const targetUrl = payload.targetUrl || payload.url || TARGET_TWEET_URL;
        if (!targetUrl || targetUrl.length < 5) {
          return createFailedResult("twitterFollow", "No target URL provided");
        }

        const engine = new ReferrerEngine({ addUTM: false });
        const ctx = engine.generateContext(targetUrl);

        try {
          await api.goto(ctx.targetWithParams, {
            waitUntil: "domcontentloaded",
            timeout: 90000,
            referer: ctx.referrer || undefined,
          });
        } catch (navError) {
          if (navError.message.includes("ERR_TOO_MANY_REDIRECTS")) {
            return createFailedResult("twitterFollow", navError, {
              partialData: { targetUrl },
            });
          }
          await api.goto(targetUrl, {
            waitUntil: "domcontentloaded",
            timeout: 60000,
          });
        }

        await api.waitVisible('article[data-testid="tweet"]', {
          timeout: 60000,
        });

        // 4. Simulate Reading
        const originalProbs = { ...agent.config.probabilities };
        agent.config.probabilities = {
          refresh: 0,
          profileDive: 0,
          tweetDive: 0,
          idle: 0.8,
        };
        agent.config.timings.readingPhase = {
          mean: mathUtils.randomInRange(5000, 10000),
          deviation: 1000,
        };
        await agent.simulateReading();
        agent.config.probabilities = originalProbs;

        // 5. Navigate to Profile
        const safeUsername = extractUsername(targetUrl).replace("@", "");
        const handleSelector = `article[data-testid="tweet"] a[href="/${safeUsername}"]`;

        if (await api.visible(handleSelector)) {
          await agent.humanClick(
            page.locator(handleSelector).first(),
            "Profile Link",
          );
          await api.wait(3000);
        }

        // 6. Read Profile
        agent.config.probabilities = {
          ...agent.config.probabilities,
          idle: 0.8,
        };
        agent.config.timings.readingPhase = { mean: 15000, deviation: 5000 };
        await agent.simulateReading();

        // 7. Follow
        const followResult = await agent.robustFollow(
          "[twitterFollow]",
          `https://x.com/${safeUsername}`,
        );

        if (followResult.success && followResult.attempts > 0) {
          followedUsername = extractUsername(targetUrl);
          logger.info(`[twitterFollow] ✅ Followed '${followedUsername}'`);
          metricsCollector.recordSocialAction("follow", 1);
          await api.wait(mathUtils.randomInRange(2000, 4000));
          await takeScreenshot(page, `Follow-${followedUsername}`);
        } else if (followResult.fatal) {
          return createFailedResult(
            "twitterFollow",
            new Error(`Follow failed: ${followResult.reason}`),
            { partialData: { username: safeUsername } },
          );
        }

        // 8. Return Home
        await agent.navigateHome();
        if (!(await agent.checkLoginState())) {
          logger.warn("[twitterFollow] ⚠️ Potential logout detected");
        }

        agent.config.timings.readingPhase = { mean: 90000, deviation: 30000 };
        await agent.simulateReading();

        return createSuccessResult(
          "twitterFollow",
          {
            followed: followedUsername,
            targetUrl,
            attempts: followResult.attempts,
          },
          { startTime, sessionId: browserInfo },
        );
      })(),
      new Promise((_, reject) =>
        setTimeout(
          () =>
            reject(
              new Error(`Task timeout (${(taskTimeoutMs / 1000).toFixed(0)}s)`),
            ),
          taskTimeoutMs,
        ),
      ),
    ]);
  } catch (error) {
    logger.error(`[twitterFollow] Error: ${error.message}`);
    return createFailedResult("twitterFollow", error, {
      partialData: { followed: followedUsername },
      sessionId: browserInfo,
    });
  } finally {
    if (sessionStart) {
      logger.info(
        `[Metrics] Duration: ${((Date.now() - sessionStart) / 1000 / 60).toFixed(1)}m`,
      );
    }
    try {
      if (page && !page.isClosed()) await page.close();
    } catch {
      // Ignore close errors
    }
  }
}
