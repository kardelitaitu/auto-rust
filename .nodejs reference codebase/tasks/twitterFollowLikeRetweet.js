/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Twitter Follow+Like+Retweet Task
 * @module tasks/twitterFollowLikeRetweet.js
 */

// --- CONFIGURATION ---
const DEFAULT_TASK_TIMEOUT_MS = 6 * 60 * 1000; // 6 Minutes Hard Limit (More actions = more time)
const TARGET_TWEET_URL = "https://x.com/_nadiku/status/1998218314703852013";

import { createLogger } from "../api/core/logger.js";
import { AITwitterAgent } from "../utils/ai-twitterAgent.js";
import { profileManager } from "../utils/profileManager.js";
import { mathUtils } from "../utils/math.js";
import { ReferrerEngine } from "../utils/urlReferrer.js";
import metricsCollector from "../utils/metrics.js";
import { takeScreenshot } from "../utils/screenshot.js";
import { applyHumanizationPatch } from "../utils/browserPatch.js";
import { createSuccessResult, createFailedResult } from "../api/core/task-result.js";

// Helper: Extract username from tweet URL
function extractUsername(tweetUrl) {
  try {
    const url = new URL(tweetUrl);
    const pathParts = url.pathname.split("/").filter((p) => p.length > 0);
    // URL format: /username/status/tweet_id
    if (pathParts.length >= 1) {
      return "@" + pathParts[0];
    }
  } catch (_e) {
    return "(unknown)";
  }
  return "(unknown)";
}

/**
 * Executes the Follow+Like+Retweet Task.
 * @param {object} page - The Playwright page instance.
 * @param {object} payload
 * @param {string} payload.browserInfo
 * @param {number} [payload.taskTimeoutMs]
 * @param {string} [payload.profileId]
 * @param {string} [payload.targetUrl]
 */
export default async function twitterFollowLikeRetweetTask(page, payload) {
  const _startTime = process.hrtime.bigint();
  const browserInfo = payload.browserInfo || "unknown_profile";
  const logger = createLogger(`twitterFollowLikeRetweet [${browserInfo}]`);
  const taskTimeoutMs = payload.taskTimeoutMs || DEFAULT_TASK_TIMEOUT_MS;

  let agent = null;

  const startupJitter = Math.floor(Math.random() * 6000);
  // logger.info(`[followLikeRetweet] Warming up for ${startupJitter}ms...`);
  await page.waitForTimeout(startupJitter);

  try {
    // Wrap execution in a Promise.race to enforce hard time limit
    await Promise.race([
      (async () => {
        // 1. Initialize Agent
        let profile;
        if (payload && payload.profileId) {
          try {
            profile = profileManager.getById(payload.profileId);
          } catch (_e) {
            profile = profileManager.getStarter();
          }
        } else {
          profile = profileManager.getStarter();
        }

        agent = new AITwitterAgent(page, profile, logger);

        // Enforce Theme
        if (profile.theme) {
          await page.emulateMedia({ colorScheme: profile.theme });
        }

        // 2. Apply Humanization
        await applyHumanizationPatch(page, logger);

        // 3. WARM-UP JITTER (Decouple browser launch from action)
        const wakeUpTime = mathUtils.randomInRange(2000, 8000);
        // logger.info(
        //     `[Startup] Warming up (Human Jitter) for ${(wakeUpTime / 1000).toFixed(1)}s...`
        // );
        await page.waitForTimeout(wakeUpTime);

        // 4. Initial Login Check - SKIPPED to preserve Referrer Mechanism
        // We rely on the target page navigation to handle auth states naturally

        // 5. Navigation with Advanced Referrer Engine
        // logger.info(`[followLikeRetweet] Initializing Referrer Engine...`);

        const targetUrl =
          (payload && (payload.targetUrl || payload.url)) || TARGET_TWEET_URL;

        if (!targetUrl || targetUrl.length < 5) {
          logger.error(
            `[followLikeRetweet] No targetUrl provided in payload and TARGET_TWEET_URL is invalid.`,
          );
          return;
        }

        // Use ReferrerEngine for natural referer (browser sets Sec-Fetch headers automatically)
        const engine = new ReferrerEngine({ addUTM: false });
        const ctx = engine.generateContext(targetUrl);
        // logger.info(`[followLikeRetweet][Anti-Sybil] Engine Strategy: ${ctx.strategy}`);
        // logger.info(
        //     `[followLikeRetweet][Anti-Sybil] Referrer: ${ctx.referrer || '(Direct Traffic - No Referrer)'}`
        // );

        // Navigate to target
        // logger.info(`[followLikeRetweet] Navigating to Target Tweet: ${targetUrl}`);
        await page.goto(ctx.targetWithParams, {
          waitUntil: "domcontentloaded",
          timeout: 90000,
          referer: ctx.referrer || undefined,
        });

        // Explicitly wait for the tweet to be visible
        try {
          // logger.info(`[followLikeRetweet] Waiting for tweet content to load...`);
          await page.waitForSelector('article[data-testid="tweet"]', {
            state: "visible",
            timeout: 60000,
          });
        } catch (e) {
          logger.warn(
            `[followLikeRetweet] Timed out waiting for tweet selector. Page might be incomplete.`,
          );
          throw new Error(
            "Fatal: Tweet selector timeout - likely stuck or no internet",
            {
              cause: e,
            },
          );
        }

        // 6. Simulate Reading (Tweet Thread)
        // Use agent.simulateReading() but force it to be purely reading (disable diving) by overriding probabilities temporarily
        const originalProbs = { ...agent.config.probabilities };
        agent.config.probabilities = {
          refresh: 0,
          profileDive: 0,
          tweetDive: 0,
          idle: 0.8,
          likeTweetAfterDive: 0,
          bookmarkAfterDive: 0,
          followOnProfile: 0,
        };

        const tweetReadTime = mathUtils.randomInRange(5000, 10000);
        // logger.info(
        //     `[followLikeRetweet] Reading Tweet for ${(tweetReadTime / 1000).toFixed(1)}s...`
        // );

        // Manually trigger a read phase (hack: overwrite config timings slightly to match desired duration)
        agent.config.timings.readingPhase = {
          mean: tweetReadTime,
          deviation: 1000,
        };
        await agent.simulateReading();

        // Restore config
        agent.config.probabilities = originalProbs;

        // 7. Click on Profile
        // logger.info(`[followLikeRetweet] Finding and clicking profile...`);

        // Extract names used for targeting
        const targetUsername = extractUsername(targetUrl).toLowerCase();
        const safeUsername = targetUsername.replace("@", "");

        // Potential Selectors
        const handleSelector = `article[data-testid="tweet"] a[href="/${safeUsername}"]`;
        const avatarSelector = `article[data-testid="tweet"] [data-testid="Tweet-User-Avatar"]`;
        const fallbackSelector =
          'article[data-testid="tweet"] div[data-testid="User-Name"] a[href^="/"]';

        // Attempt 1: Click specific Handle/Name link
        let targetEl = page.locator(handleSelector).first();
        if (!(await targetEl.count())) {
          targetEl = page.locator(fallbackSelector).first();
        }

        if (await targetEl.isVisible()) {
          await agent.humanClick(targetEl, "Profile Link (Handle/Name)");
          await page.waitForTimeout(3000); // Wait for nav
        }

        // Verification & Retry
        if (page.url().includes("/status/")) {
          logger.warn(
            `[followLikeRetweet] ⚠️ Navigation check: Still on status page. Retrying with Avatar...`,
          );

          // Attempt 2: Click Avatar
          const avatarEl = page.locator(avatarSelector).first();
          if (await avatarEl.isVisible()) {
            await agent.humanClick(avatarEl, "Profile Avatar");
            await page.waitForTimeout(3000);
          }
        }

        // Final Verify
        if (!page.url().includes("/status/")) {
          // We likely navigated!
          await page.waitForLoadState("domcontentloaded");
        } else {
          logger.warn(
            `[followLikeRetweet] 🛑 Failed to navigate to profile. Still on status page: ${page.url()}`,
          );
        }

        // 8. Simulate Reading on Profile
        // logger.info(`[followLikeRetweet] Reading Profile...`);

        // Disable diving while reading profile to avoid navigating away
        const probsBeforeProfile = { ...agent.config.probabilities };
        agent.config.probabilities = {
          ...probsBeforeProfile,
          refresh: 0,
          profileDive: 0,
          tweetDive: 0,
          idle: 0.8,
        };

        agent.config.timings.readingPhase = { mean: 15000, deviation: 5000 }; // 15s avg
        await agent.simulateReading();

        // Restore
        agent.config.probabilities = probsBeforeProfile;

        // 9. Follow (Using Robust Follow from TwitterAgent)
        // logger.info(`[followLikeRetweet] Executing follow action...`);
        // Construct explicit profile URL just in case robust follow needs to reload
        const profileUrl = `https://x.com/${safeUsername}`;
        const followResult = await agent.robustFollow(
          "[followLikeRetweet]",
          profileUrl,
        );

        if (followResult.success && followResult.attempts > 0) {
          const username = extractUsername(targetUrl);
          logger.info(
            `[followLikeRetweet] ✅✅ Followed '\x1b[94m${username}\x1b[0m' Successfully ✅✅✅`,
          );

          // Report follow to global metrics IMMEDIATELY
          metricsCollector.recordSocialAction("follow", 1);

          // Post-follow delay: Human "satisfaction" reaction time before leaving
          const postFollowDelay = mathUtils.randomInRange(2000, 4000);
          // logger.info(
          //     `[followLikeRetweet] Lingering on profile for ${(postFollowDelay / 1000).toFixed(1)}s...`
          // );
          await page.waitForTimeout(postFollowDelay);
        } else if (
          !followResult.success &&
          typeof followResult.reason === "string" &&
          followResult.reason.includes("reload")
        ) {
          // If follow failed fatally, we might want to throw error, or just continue to like/retweet?
          // twitterFollow task throws error. But here we have potential Partial Success (Follow fail, but Like might work).
          // However, if follow failed fatally (e.g. reload failed), page is likely broken.
          // Let's log heavily but TRY to continue to return to tweet.
          logger.warn(
            `[followLikeRetweet] ⚠️ Follow failed fatally (reload): ${followResult.reason}. Attempting to continue...`,
          );
        }

        // 10. Go Back to Tweet (CDP)
        // logger.info('[followLikeRetweet] Going back to Tweet URL...');
        try {
          const client = await page.context().newCDPSession(page);
          await client.send("Runtime.evaluate", {
            expression: "window.history.back()",
          });
          // Wait for tweet to verify we are back (url should contain /status/)
          await page.waitForURL("**/status/**", { timeout: 15000 });
          // logger.info('[followLikeRetweet] ✅ Navigated back to tweet.');
          await page.waitForTimeout(mathUtils.randomInRange(2000, 4000));
        } catch (e) {
          logger.warn(
            `[followLikeRetweet] CDP Back failed: ${e.message}. Fallback to page.goBack().`,
          );
          try {
            await page.goBack();
            await page.waitForTimeout(mathUtils.randomInRange(2000, 4000));
          } catch (e2) {
            logger.error(
              `[followLikeRetweet] Navigation back failed completely: ${e2.message}. Logic may drift.`,
            );
          }
        }

        // Helper for Golden Zone Scrolling
        const goldenZone = async (locator) => {
          try {
            const box = await locator.boundingBox();
            if (box) {
              let viewport = page.viewportSize();
              let vHeight = 0;
              if (viewport) {
                vHeight = viewport.height;
              } else {
                vHeight = await page.evaluate(() => window.innerHeight);
              }

              // Target: 30% down from top (avoid sticky header)
              const targetY = vHeight * 0.3;
              const currentScroll = await page.evaluate(() => window.scrollY);
              const scrollY = currentScroll + box.y - targetY;
              await page.evaluate((y) => window.scrollTo(0, y), scrollY);
              await page.waitForTimeout(mathUtils.randomInRange(500, 1000)); // Settle
            }
          } catch (e) {
            logger.warn(
              `[followLikeRetweet][goldenZone] Scroll failed: ${e.message}`,
            );
          }
        };

        // 11. Retweet (Hardened)
        // logger.info(`[followLikeRetweet] Checking Retweet status...`);
        const retweetBtnSelector = 'button[data-testid="retweet"]';
        const unretweetBtnSelector = 'button[data-testid="unretweet"]';
        const retweetConfirmSelector = 'div[data-testid="retweetConfirm"]';

        // STRICT ID CHECK: Ensure we are acting on the correct tweet content
        // (This prevents acting on a 'Who to follow' card or ad if navigation drifted)
        // We assume we are on the status page, so the primary tweet should match.

        // Atomic Retry Loop for Retweet
        for (let attempt = 1; attempt <= 3; attempt++) {
          const unretweetBtn = page.locator(unretweetBtnSelector).first();
          if (await unretweetBtn.isVisible()) {
            logger.info("[followLikeRetweet] ✅ Already retweeted post.");
            break;
          }

          const retweetBtn = page.locator(retweetBtnSelector).first();
          if (
            (await retweetBtn.count()) > 0 &&
            (await retweetBtn.isVisible())
          ) {
            if (attempt === 1) await goldenZone(retweetBtn);

            // logger.info(
            //     `[followLikeRetweet] Clicking Retweet button (Attempt ${attempt})...`
            // );
            try {
              await agent.humanClick(retweetBtn, "Retweet Button");
              await page.waitForTimeout(mathUtils.randomInRange(800, 1500));

              const confirmBtn = page.locator(retweetConfirmSelector).first();
              if (await confirmBtn.isVisible()) {
                // logger.info('[followLikeRetweet] Confirming Retweet...');
                await agent.humanClick(confirmBtn, "Retweet Confirm");
                await page.waitForTimeout(mathUtils.randomInRange(2000, 3000));
              } else {
                logger.warn(
                  `[followLikeRetweet] Retweet confirm menu not found.`,
                );
                // If menu didn't appear, likely click failed. Retry loop handles this.
                continue;
              }

              // Verify success
              if (
                await unretweetBtn
                  .isVisible({ timeout: 5000 })
                  .catch(() => false)
              ) {
                logger.info(`[followLikeRetweet] ✅ Retweeted Successfully`);
                metricsCollector.recordSocialAction("retweet", 1); // Record Immediately
                await takeScreenshot(page, `Retweet-${targetUsername}`);
                break;
              }
            } catch (e) {
              logger.warn(
                `[followLikeRetweet] Retweet attempt ${attempt} failed: ${e.message}`,
              );
            }
          } else {
            logger.warn(`[followLikeRetweet] Retweet button not found.`);
            break; // No button, no point retrying
          }
          await page.waitForTimeout(2000);
        }

        // 12. Like (Hardened)
        // logger.info(`[followLikeRetweet] Checking Like status...`);
        const likeButton = page
          .locator('button[data-testid="like"][role="button"]')
          .first();
        const unlikeButton = page
          .locator('button[data-testid="unlike"][role="button"]')
          .first();

        // Atomic Retry Loop for Like
        for (let attempt = 1; attempt <= 3; attempt++) {
          if (await unlikeButton.isVisible()) {
            logger.info("[followLikeRetweet] ✅ Already liked post.");
            break;
          }

          if (
            (await likeButton.count()) > 0 &&
            (await likeButton.isVisible())
          ) {
            if (attempt === 1) await goldenZone(likeButton);

            try {
              // logger.info(
              //     `[followLikeRetweet] Clicking Like button (Attempt ${attempt})...`
              // );
              await agent.humanClick(likeButton, "Like Button");

              // Wait for state change
              await page.waitForTimeout(mathUtils.randomInRange(1000, 2000));

              // Verify
              if (await unlikeButton.isVisible().catch(() => false)) {
                logger.info(`[followLikeRetweet] ✅ Liked Successfully`);
                metricsCollector.recordSocialAction("like", 1); // Record Immediately
                await takeScreenshot(page, `Like-${targetUsername}`);
                await page.waitForTimeout(mathUtils.randomInRange(1000, 3000));
                break;
              }
            } catch (e) {
              logger.warn(
                `[followLikeRetweet] Like attempt ${attempt} failed: ${e.message}`,
              );
            }
          } else {
            logger.warn(`[followLikeRetweet] Like button not found.`);
            break;
          }
          await page.waitForTimeout(2000); // Backoff before retry
        }

        // 13. Return Home & "Cool Down" Reading
        // logger.info(`[followLikeRetweet] Navigating Home for cool-down...`);
        await agent.navigateHome();

        // Check login state here as requested (verifying session health at end)
        if (!(await agent.checkLoginState())) {
          logger.warn(
            "[followLikeRetweet] ⚠️ Potential logout detected after task completion.",
          );
        }

        // Read feed for a bit (1-2 mins)
        agent.config.timings.readingPhase = { mean: 60000, deviation: 20000 };
        // Allow mild interaction (likes/retweets) during cool down
        agent.config.probabilities.tweetDive = 0.1;

        await agent.simulateReading();

        logger.info(`[followLikeRetweet] Task completed.`);
        
        return createSuccessResult('twitterFollowLikeRetweet', {
          targetUrl,
          targetUsername: safeUsername
        }, { startTime: Date.now(), sessionId: browserInfo });
      })(),
      new Promise((_, reject) =>
        setTimeout(
          () =>
            reject(
              new Error(
                `Strict Task Time Limit Exceeded (${(taskTimeoutMs / 1000).toFixed(0)}s)`,
              ),
            ),
          taskTimeoutMs,
        ),
      ),
    ]);
  } catch (error) {
    if (error.message.includes("Target page, context or browser has been closed")) {
      logger.warn(`[followLikeRetweet] Task interrupted`);
      return createFailedResult('twitterFollowLikeRetweet', error, { sessionId: browserInfo });
    } else {
      logger.error(`[followLikeRetweet] Error: ${error.message}`);
      return createFailedResult('twitterFollowLikeRetweet', error, { sessionId: browserInfo });
    }
  } finally {
    const sessionStart = agent?.sessionStart || null;
    if (sessionStart) {
      logger.info(`[Metrics] Duration: ${((Date.now() - sessionStart) / 1000 / 60).toFixed(1)}m`);
    }

    try {
      if (page && !page.isClosed()) await page.close();
    } catch {
      // Ignore close errors
    }
  }
}
