/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * API-Enhanced Retweet Test Task with Random Branching
 * Multiple retweet execution paths for varied automation patterns
 * @module tasks/retweet
 */

import { api } from "../api/index.js";
import { createLogger } from "../api/core/logger.js";
import { profileManager } from "../api/utils/profileManager.js";
import { retweetWithAPI } from "../api/actions/retweet.js";
import { ReferrerEngine } from "../api/utils/urlReferrer.js";
import { mathUtils } from "../api/utils/math.js";
import { createSuccessResult, createFailedResult } from "../api/core/task-result.js";

const DEFAULT_TIMEOUT_MS = 600000; // 10 minutes (accounts for 4-8 min branch activities)
const HYDRATION_DELAY_MS = 3000;
const OBSERVATION_DELAY_MS = 5000;
const TWEET_VISIBLE_TIMEOUT_MS = 15000;
const MAX_RETRIES = 2;

// Branch 1 browse scroll configuration
const BROWSE_SCROLL_MIN_PX = 150;
const BROWSE_SCROLL_MAX_PX = 400;
const BROWSE_BACK_SCROLL_MIN_PX = 30;
const BROWSE_BACK_SCROLL_MAX_PX = 250;
const BROWSE_PAUSE_MIN_MS = 400;
const BROWSE_PAUSE_MAX_MS = 2500;
const BROWSE_BACK_PAUSE_MIN_MS = 300;
const BROWSE_BACK_PAUSE_MAX_MS = 1200;
const BROWSE_BACK_CHANCE = 0.2;

/**
 * Retweet branch probability configuration.
 * Weights determine selection frequency (higher = more likely).
 * Total weight: 100 for easy percentage interpretation.
 */
const RETWEET_BRANCH_PROBABILITIES = {
  retweetBranch1_directRetweet: 20,
  retweetBranch2_profileVisitRetweet: 15,
  retweetBranch3_homeReadRetweet: 12,
  retweetBranch4_homeProfileRetweet: 8,
  retweetBranch5_threadReader: 10,
  retweetBranch6_likeRetweet: 10,
  retweetBranch7_notificationCheck: 8,
  retweetBranch8_overscrollReturn: 7,
  retweetBranch9_searchToRetweet: 7,
  retweetBranch10_impulseRetweet: 6,
  retweetBranch11_multiPhaseEngagement: 5,
  retweetBranch12_misclickRecovery: 2,
};

/**
 * Debug flags for branch forcing.
 * Set to true to force a specific branch (useful for testing).
 * When multiple flags are true, first match wins.
 */
const DEBUG_FORCE_BRANCH = {
  retweetBranch1_directRetweet: false,
  retweetBranch2_profileVisitRetweet: false,
  retweetBranch3_homeReadRetweet: false,
  retweetBranch4_homeProfileRetweet: false,
  retweetBranch5_threadReader: false,
  retweetBranch6_likeRetweet: false,
  retweetBranch7_notificationCheck: false,
  retweetBranch8_overscrollReturn: false,
  retweetBranch9_searchToRetweet: false,
  retweetBranch10_impulseRetweet: false,
  retweetBranch11_multiPhaseEngagement: false,
  retweetBranch12_misclickRecovery: false,
};

const RETWEET_BRANCHES = Object.entries(RETWEET_BRANCH_PROBABILITIES).map(
  ([name, weight]) => ({
    name,
    weight,
  }),
);

function selectRetweetBranch(payload, logger) {
  // Check debug force flags first
  const forcedBranch = Object.entries(DEBUG_FORCE_BRANCH).find(
    ([, forced]) => forced,
  );
  if (forcedBranch) {
    const forcedName = forcedBranch[0];
    logger.info(`[DEBUG] Forcing branch: ${forcedName}`);
    return forcedName;
  }

  // Check payload debug override
  if (payload?.debugBranch) {
    const branchName = payload.debugBranch;
    const exists = RETWEET_BRANCHES.some((b) => b.name === branchName);
    if (exists) {
      logger.info(`[DEBUG] Payload forcing branch: ${branchName}`);
      return branchName;
    }
    logger.warn(
      `[DEBUG] Invalid debugBranch: ${branchName}, using random selection`,
    );
  }

  // Weighted random selection
  const totalWeight = RETWEET_BRANCHES.reduce((sum, b) => sum + b.weight, 0);
  let random = Math.random() * totalWeight;

  for (const branch of RETWEET_BRANCHES) {
    random -= branch.weight;
    if (random <= 0) {
      return branch.name;
    }
  }

  return RETWEET_BRANCHES[0].name;
}

/**
 * Branch 1: Direct Retweet + Home Feed Reading
 * Navigate to tweet URL, browse around, retweet, then read home feed.
 * Simulates users who visit a tweet, retweet it, then browse their timeline.
 */
async function retweetBranch1_directRetweet(page, logger) {
  logger.info("[Branch1] Direct retweet - locating retweet button...");

  const tweetSelector = 'article[data-testid="tweet"]';
  const tweetElement = page.locator(tweetSelector).first();
  await tweetElement.waitFor({
    state: "visible",
    timeout: TWEET_VISIBLE_TIMEOUT_MS,
  });

  // Find the retweet button
  const retweetBtn = page.locator('[data-testid="retweet"]').first();
  await retweetBtn.waitFor({
    state: "visible",
    timeout: TWEET_VISIBLE_TIMEOUT_MS,
  });
  logger.info("[Branch1] Retweet button found.");

  // Random browse: scroll down for 10-20s simulating content scanning
  const browseTimeMs = Math.random() * 10000 + 10000; // 10-20 seconds
  const browseTimeSec = (browseTimeMs / 1000).toFixed(1);
  logger.info(`[Branch1] Browsing page for ${browseTimeSec}s...`);

  const browseStart = Date.now();
  while (Date.now() - browseStart < browseTimeMs) {
    // Random scroll down with configurable range
    const scrollDistance = mathUtils.randomInRange(
      BROWSE_SCROLL_MIN_PX,
      BROWSE_SCROLL_MAX_PX,
    );
    await api.scroll(scrollDistance);

    // Random pause between scrolls (simulates reading/scanning)
    const pauseTime = mathUtils.randomInRange(
      BROWSE_PAUSE_MIN_MS,
      BROWSE_PAUSE_MAX_MS,
    );
    await api.wait(pauseTime);

    // Occasionally scroll back up (configurable chance)
    if (Math.random() < BROWSE_BACK_CHANCE) {
      const backDistance = mathUtils.randomInRange(
        BROWSE_BACK_SCROLL_MIN_PX,
        BROWSE_BACK_SCROLL_MAX_PX,
      );
      await api.scroll.back(backDistance);

      const backPause = mathUtils.randomInRange(
        BROWSE_BACK_PAUSE_MIN_MS,
        BROWSE_BACK_PAUSE_MAX_MS,
      );
      await api.wait(backPause);
    }
  }
  logger.info(
    `[Branch1] Finished browsing, scrolling back to retweet button...`,
  );

  // Scroll to retweet button using focus2
  const scrollResult = await api.scroll.focus2(retweetBtn);
  logger.info(
    `[Branch1] focus2 result: distance=${scrollResult.distance?.toFixed(0)}px, steps=${scrollResult.steps}`,
  );
  await api.wait(mathUtils.randomInRange(600, 1000));

  // Execute retweet
  await retweetWithAPI({ page, tweetElement });

  // Navigate to home and simulate reading feed for 2-5 minutes
  logger.info("[Branch1] Retweet successful, navigating to home feed...");
  const homeReadMs = mathUtils.randomInRange(120000, 300000); // 2-5 minutes
  logger.info(
    `[Branch1] Reading home feed for ${(homeReadMs / 60000).toFixed(1)} minutes...`,
  );
  await api.twitter.home({ readDurationMs: homeReadMs });

  return { success: true, branch: "Branch1" };
}

/**
 * Branch 2: Profile Visit + Retweet
 * Visit author's profile, read it, return to tweet, then retweet.
 * Simulates users who check the author before retweeting.
 */
async function retweetBranch2_profileVisitRetweet(page, logger, targetUrl) {
  logger.info("[Branch2] Profile Visit + Retweet...");

  const tweetSelector = 'article[data-testid="tweet"]';
  const tweetElement = page.locator(tweetSelector).first();
  await tweetElement.waitFor({
    state: "visible",
    timeout: TWEET_VISIBLE_TIMEOUT_MS,
  });

  // Extract and validate profile from tweet URL
  const profileMatch = targetUrl.match(/x\.com\/([^/]+)\/status\/\d+/);
  if (!profileMatch) {
    logger.warn(
      "[Branch2] Invalid tweet URL format, falling back to direct retweet",
    );
    const retweetBtn = tweetElement.locator('[data-testid="retweet"]');
    await retweetBtn.waitFor({
      state: "visible",
      timeout: TWEET_VISIBLE_TIMEOUT_MS,
    });
    await api.scroll.focus2(retweetBtn);
    await api.wait(mathUtils.randomInRange(500, 1000));
    return await retweetWithAPI({ page, tweetElement });
  }

  const profileHandle = profileMatch[1];
  logger.info(`[Branch2] Extracted profile: ${profileHandle}`);

  // Click profile link on the tweet
  const profileLinkSelector = `a[href="/${profileHandle}"]`;
  const profileLink = tweetElement.locator(profileLinkSelector).first();

  if (await api.visible(profileLink).catch(() => false)) {
    logger.info(`[Branch2] Clicking profile link: ${profileLinkSelector}`);
    await api.scroll.focus2(profileLink);
    await api.wait(mathUtils.randomInRange(500, 1000));
    await api.click(profileLink);
  } else {
    // Fallback: navigate directly to profile
    logger.info(`[Branch2] Profile link not visible, navigating directly...`);
    await api.goto(`https://x.com/${profileHandle}`, {
      waitUntil: "domcontentloaded",
      timeout: 60000,
    });
  }

  // Hydration wait and verify URL
  logger.info("[Branch2] Waiting for profile page hydration...");
  await api.wait(HYDRATION_DELAY_MS);

  const profileUrl = await api.getCurrentUrl();
  const onProfile = profileUrl.includes(`/${profileHandle}`);
  logger.info(
    `[Branch2] Current URL: ${profileUrl} (profile match: ${onProfile})`,
  );

  if (!onProfile) {
    logger.warn(
      "[Branch2] Profile page may not have loaded correctly, continuing anyway...",
    );
  }

  // Read profile (20-90 seconds)
  const profileReadTime = mathUtils.randomInRange(20000, 90000);
  logger.info(
    `[Branch2] Reading profile for ${(profileReadTime / 1000).toFixed(1)}s...`,
  );

  const profileScrollCount = Math.floor(Math.random() * 3) + 1;
  for (let i = 0; i < profileScrollCount; i++) {
    await api.scroll(mathUtils.randomInRange(200, 600));
    await api.wait(mathUtils.randomInRange(500, 2000));
  }

  // Navigate back to tweet
  logger.info("[Branch2] Navigating back to tweet...");
  await api.back();
  await api.wait(HYDRATION_DELAY_MS);

  // Verify we're back on the tweet
  const backUrl = await api.getCurrentUrl();
  logger.info(`[Branch2] Back on: ${backUrl}`);

  // Read tweet page (20-90 seconds)
  const tweetReadTime = mathUtils.randomInRange(20000, 90000);
  logger.info(
    `[Branch2] Reading tweet page for ${(tweetReadTime / 1000).toFixed(1)}s...`,
  );

  const tweetScrollCount = Math.floor(Math.random() * 2) + 1;
  for (let i = 0; i < tweetScrollCount; i++) {
    await api.scroll(mathUtils.randomInRange(200, 400));
    await api.wait(mathUtils.randomInRange(500, 1500));
  }

  // Focus on retweet button and execute
  logger.info("[Branch2] Focusing on retweet button...");
  const retweetBtn = tweetElement.locator('[data-testid="retweet"]');
  await retweetBtn.waitFor({
    state: "visible",
    timeout: TWEET_VISIBLE_TIMEOUT_MS,
  });
  await api.scroll.focus2(retweetBtn);
  await api.wait(mathUtils.randomInRange(500, 1000));

  const result = await retweetWithAPI({ page, tweetElement });
  // Navigate to home and simulate reading feed for 2-5 minutes
  if (result.success) {
    logger.info("[Branch2] Retweet successful, navigating to home feed...");
    const homeReadMs = mathUtils.randomInRange(120000, 300000); // 2-5 minutes
    logger.info(
      `[Branch2] Reading home feed for ${(homeReadMs / 60000).toFixed(1)} minutes...`,
    );
    await api.twitter.home({ readDurationMs: homeReadMs });
  }
  return result;
}

/**
 * Branch 3: Home Browse + Read Then Retweet
 * Browse home feed first, then return to tweet, read it, and retweet.
 * Simulates users who browse timeline before engaging with a specific tweet.
 */
async function retweetBranch3_homeReadRetweet(page, logger, targetUrl) {
  logger.info("[Branch3] Home Browse + Read Then Retweet...");

  // Phase 1: Browse home feed (2-4 minutes)
  const homeReadMs = mathUtils.randomInRange(120000, 240000); // 2-4 minutes
  logger.info(
    `[Phase1] Browsing home feed for ${(homeReadMs / 60000).toFixed(1)} minutes...`,
  );
  await api.twitter.home({ readDurationMs: homeReadMs });

  // Phase 2: Navigate back to target tweet
  logger.info(`[Phase2] Returning to target tweet: ${targetUrl}`);
  await api.goto(targetUrl, { waitUntil: "domcontentloaded", timeout: 60000 });
  await api.wait(HYDRATION_DELAY_MS);

  // Phase 3: Read the tweet
  const tweetSelector = 'article[data-testid="tweet"]';
  const tweetElement = page.locator(tweetSelector).first();
  await tweetElement.waitFor({
    state: "visible",
    timeout: TWEET_VISIBLE_TIMEOUT_MS,
  });
  logger.info("[Phase3] Reading tweet...");

  // Focus on the tweet for reading
  await api.scroll.focus2(tweetElement);
  await api.wait(mathUtils.randomInRange(200, 500));

  // Simulate reading with hover and pauses
  await api.hover(tweetElement);
  const readTime = mathUtils.randomInRange(2000, 5000);
  logger.info(`[Phase3] Reading tweet for ${Math.round(readTime)}ms...`);
  await api.wait(readTime);

  // Small cursor movements while reading
  await api.mouse.wiggle({ magnitude: 5, duration: 200 });
  await api.wait(mathUtils.randomInRange(200, 500));

  // Phase 4: Focus on retweet button and execute
  logger.info("[Phase4] Focusing on retweet button...");
  const retweetBtn = tweetElement.locator('[data-testid="retweet"]');
  await retweetBtn.waitFor({
    state: "visible",
    timeout: TWEET_VISIBLE_TIMEOUT_MS,
  });
  await api.scroll.focus2(retweetBtn);
  await api.wait(mathUtils.randomInRange(300, 600));

  const result = await retweetWithAPI({ page, tweetElement });
  return result;
}

/**
 * Branch 4: Home Browse + Profile Visit + Retweet
 * Browse home feed first, then visit author's profile, return to tweet, and retweet.
 * Simulates users who browse timeline, check author profile, then retweet.
 */
async function retweetBranch4_homeProfileRetweet(page, logger, targetUrl) {
  logger.info("[Branch4] Home Browse + Profile Visit + Retweet...");

  // Phase 1: Browse home feed (2-4 minutes)
  const homeReadMs = mathUtils.randomInRange(120000, 240000); // 2-4 minutes
  logger.info(
    `[Phase1] Browsing home feed for ${(homeReadMs / 60000).toFixed(1)} minutes...`,
  );
  await api.twitter.home({ readDurationMs: homeReadMs });

  // Phase 2: Navigate back to target tweet
  logger.info(`[Phase2] Returning to target tweet: ${targetUrl}`);
  await api.goto(targetUrl, { waitUntil: "domcontentloaded", timeout: 60000 });
  await api.wait(HYDRATION_DELAY_MS);

  // Phase 3: Extract and click profile
  const tweetSelector = 'article[data-testid="tweet"]';
  const tweetElement = page.locator(tweetSelector).first();
  await tweetElement.waitFor({
    state: "visible",
    timeout: TWEET_VISIBLE_TIMEOUT_MS,
  });

  const profileMatch = targetUrl.match(/x\.com\/([^/]+)\/status\/\d+/);
  if (!profileMatch) {
    logger.warn(
      "[Phase3] Invalid tweet URL format, falling back to direct retweet",
    );
    const retweetBtn = tweetElement.locator('[data-testid="retweet"]');
    await retweetBtn.waitFor({
      state: "visible",
      timeout: TWEET_VISIBLE_TIMEOUT_MS,
    });
    await api.scroll.focus2(retweetBtn);
    await api.wait(mathUtils.randomInRange(500, 1000));
    return await retweetWithAPI({ page, tweetElement });
  }

  const profileHandle = profileMatch[1];
  logger.info(`[Phase3] Extracted profile: ${profileHandle}`);

  // Click profile link on the tweet
  const profileLinkSelector = `a[href="/${profileHandle}"]`;
  const profileLink = tweetElement.locator(profileLinkSelector).first();

  if (await api.visible(profileLink).catch(() => false)) {
    logger.info(`[Phase3] Clicking profile link: ${profileLinkSelector}`);
    await api.scroll.focus2(profileLink);
    await api.wait(mathUtils.randomInRange(500, 1000));
    await api.click(profileLink);
  } else {
    logger.info(`[Phase3] Profile link not visible, navigating directly...`);
    await api.goto(`https://x.com/${profileHandle}`, {
      waitUntil: "domcontentloaded",
      timeout: 60000,
    });
  }

  await api.wait(HYDRATION_DELAY_MS);

  // Phase 4: Read profile (5-15 seconds)
  const profileReadTime = mathUtils.randomInRange(5000, 15000);
  logger.info(
    `[Phase4] Reading profile for ${(profileReadTime / 1000).toFixed(1)}s...`,
  );

  const profileScrollCount = Math.floor(Math.random() * 3) + 1;
  for (let i = 0; i < profileScrollCount; i++) {
    await api.scroll(mathUtils.randomInRange(200, 600));
    await api.wait(mathUtils.randomInRange(500, 2000));
  }

  // Phase 5: Navigate back to tweet
  logger.info("[Phase5] Navigating back to tweet...");
  await api.back();
  await api.wait(HYDRATION_DELAY_MS);

  // Phase 6: Read tweet page (5-10 seconds)
  const tweetReadTime = mathUtils.randomInRange(5000, 10000);
  logger.info(
    `[Phase6] Reading tweet page for ${(tweetReadTime / 1000).toFixed(1)}s...`,
  );

  const tweetScrollCount = Math.floor(Math.random() * 2) + 1;
  for (let i = 0; i < tweetScrollCount; i++) {
    await api.scroll(mathUtils.randomInRange(200, 400));
    await api.wait(mathUtils.randomInRange(500, 1500));
  }

  // Phase 7: Focus on retweet button and execute
  logger.info("[Phase7] Focusing on retweet button...");
  const retweetBtn = tweetElement.locator('[data-testid="retweet"]');
  await retweetBtn.waitFor({
    state: "visible",
    timeout: TWEET_VISIBLE_TIMEOUT_MS,
  });
  await api.scroll.focus2(retweetBtn);
  await api.wait(mathUtils.randomInRange(500, 1000));

  const result = await retweetWithAPI({ page, tweetElement });
  return result;
}

/**
 * Branch 5: Home Browse + Thread Reader
 * Browse home, then read thread, then retweet.
 * Simulates users who browse timeline then read full context before retweeting.
 */
async function retweetBranch5_threadReader(page, logger, targetUrl) {
  logger.info("[Branch5] Home Browse + Thread Reader...");

  // Phase 1: Browse home feed (2-4 minutes)
  const homeReadMs = mathUtils.randomInRange(120000, 240000);
  logger.info(
    `[Phase1] Browsing home feed for ${(homeReadMs / 60000).toFixed(1)} minutes...`,
  );
  await api.twitter.home({ readDurationMs: homeReadMs });

  // Phase 2: Navigate to target tweet
  logger.info("[Phase2] Returning to target tweet...");
  await api.goto(targetUrl, { waitUntil: "domcontentloaded", timeout: 60000 });
  await api.wait(HYDRATION_DELAY_MS);

  // Phase 3: Read tweet page briefly (1-2 minutes)
  const tweetReadTime = mathUtils.randomInRange(60000, 120000);
  logger.info(
    `[Phase3] Reading tweet page for ${(tweetReadTime / 1000).toFixed(0)}s...`,
  );
  const tweetScrollStart = Date.now();
  while (Date.now() - tweetScrollStart < tweetReadTime) {
    await api.scroll(mathUtils.randomInRange(200, 500));
    await api.wait(mathUtils.randomInRange(1000, 3000));
  }

  // Phase 4: Look for and read thread
  const tweetSelector = 'article[data-testid="tweet"]';
  const tweetElement = page.locator(tweetSelector).first();
  await tweetElement.waitFor({
    state: "visible",
    timeout: TWEET_VISIBLE_TIMEOUT_MS,
  });

  const showThreadBtn = tweetElement
    .locator('[data-testid="tweet-text-show-show-thread"]')
    .first();
  const threadLink = tweetElement.locator('a[href*="/status/"]').last();

  let clickedThread = false;
  if (await api.visible(showThreadBtn).catch(() => false)) {
    logger.info('[Phase4] Found "Show thread" button, clicking...');
    await api.scroll.focus2(showThreadBtn);
    await api.wait(mathUtils.randomInRange(300, 600));
    await api.click(showThreadBtn);
    clickedThread = true;
  } else if (await api.visible(threadLink).catch(() => false)) {
    logger.info("[Phase4] Found thread link, clicking...");
    await api.scroll.focus2(threadLink);
    await api.wait(mathUtils.randomInRange(300, 600));
    await api.click(threadLink);
    clickedThread = true;
  }

  if (clickedThread) {
    await api.wait(HYDRATION_DELAY_MS);

    // Read through thread (1-2 minutes)
    const threadReadTime = mathUtils.randomInRange(60000, 120000);
    logger.info(
      `[Phase4] Reading thread for ${(threadReadTime / 1000).toFixed(0)}s...`,
    );

    const scrollStart = Date.now();
    while (Date.now() - scrollStart < threadReadTime) {
      await api.scroll(mathUtils.randomInRange(200, 500));
      await api.wait(mathUtils.randomInRange(1000, 3000));
    }

    // Navigate back to original tweet
    logger.info("[Phase4] Returning to original tweet...");
    await api.goto(targetUrl, {
      waitUntil: "domcontentloaded",
      timeout: 60000,
    });
    await api.wait(HYDRATION_DELAY_MS);
  }

  // Phase 5: Focus on retweet button and execute
  logger.info("[Phase5] Focusing on retweet button...");
  const retweetBtn = tweetElement.locator('[data-testid="retweet"]');
  await retweetBtn.waitFor({
    state: "visible",
    timeout: TWEET_VISIBLE_TIMEOUT_MS,
  });
  await api.scroll.focus2(retweetBtn);
  await api.wait(mathUtils.randomInRange(500, 1000));

  const result = await retweetWithAPI({ page, tweetElement });
  return result;
}

/**
 * Branch 6: Home Browse + Like + Retweet
 * Browse home, then like and retweet.
 * Simulates multi-engagement pattern.
 */
async function retweetBranch6_likeRetweet(page, logger, targetUrl) {
  logger.info("[Branch6] Home Browse + Like + Retweet...");

  // Phase 1: Browse home feed (2-3 minutes)
  const homeReadMs = mathUtils.randomInRange(120000, 180000);
  logger.info(
    `[Phase1] Browsing home feed for ${(homeReadMs / 60000).toFixed(1)} minutes...`,
  );
  await api.twitter.home({ readDurationMs: homeReadMs });

  // Phase 2: Navigate to target tweet
  logger.info("[Phase2] Navigating to target tweet...");
  await api.goto(targetUrl, { waitUntil: "domcontentloaded", timeout: 60000 });
  await api.wait(HYDRATION_DELAY_MS);

  // Phase 3: Read tweet before engaging (1-2 minutes)
  const readTime = mathUtils.randomInRange(60000, 120000);
  logger.info(`[Phase3] Reading tweet for ${(readTime / 1000).toFixed(0)}s...`);
  const readStart = Date.now();
  while (Date.now() - readStart < readTime) {
    await api.scroll(mathUtils.randomInRange(100, 300));
    await api.wait(mathUtils.randomInRange(1000, 3000));
  }

  // Phase 4: Like the tweet
  const tweetSelector = 'article[data-testid="tweet"]';
  const tweetElement = page.locator(tweetSelector).first();
  await tweetElement.waitFor({
    state: "visible",
    timeout: TWEET_VISIBLE_TIMEOUT_MS,
  });

  const likeBtn = tweetElement.locator('[data-testid="like"]').first();
  if (await api.visible(likeBtn).catch(() => false)) {
    logger.info("[Phase4] Clicking like button...");
    await api.scroll.focus2(likeBtn);
    await api.wait(mathUtils.randomInRange(300, 600));
    await api.click(likeBtn);
    await api.wait(mathUtils.randomInRange(1000, 2000));
  } else {
    logger.info("[Phase4] Like button not found, may already be liked");
  }

  // Phase 5: Focus on retweet button and execute
  logger.info("[Phase5] Focusing on retweet button...");
  const retweetBtn = tweetElement.locator('[data-testid="retweet"]');
  await retweetBtn.waitFor({
    state: "visible",
    timeout: TWEET_VISIBLE_TIMEOUT_MS,
  });
  await api.scroll.focus2(retweetBtn);
  await api.wait(mathUtils.randomInRange(500, 1000));

  const result = await retweetWithAPI({ page, tweetElement });
  return result;
}

/**
 * Branch 7: Home + Notification Check + Retweet
 * Browse home, check notifications, then retweet.
 * Simulates users who check notifications before engaging.
 */
async function retweetBranch7_notificationCheck(page, logger, targetUrl) {
  logger.info("[Branch7] Home + Notification Check + Retweet...");

  // Phase 1: Browse home feed (1-2 minutes)
  const homeReadMs = mathUtils.randomInRange(60000, 120000);
  logger.info(
    `[Phase1] Browsing home feed for ${(homeReadMs / 60000).toFixed(1)} minutes...`,
  );
  await api.twitter.home({ readDurationMs: homeReadMs });

  // Phase 2: Check notifications (1-2 minutes)
  logger.info("[Phase2] Checking notifications...");
  await api.goto("https://x.com/notifications", {
    waitUntil: "domcontentloaded",
    timeout: 60000,
  });
  await api.wait(HYDRATION_DELAY_MS);

  const notifReadTime = mathUtils.randomInRange(60000, 120000);
  logger.info(
    `[Phase2] Browsing notifications for ${(notifReadTime / 1000).toFixed(0)}s...`,
  );

  const scrollStart = Date.now();
  while (Date.now() - scrollStart < notifReadTime) {
    await api.scroll(mathUtils.randomInRange(200, 400));
    await api.wait(mathUtils.randomInRange(1000, 3000));
  }

  // Phase 3: Navigate to target tweet
  logger.info("[Phase3] Navigating to target tweet...");
  await api.goto(targetUrl, { waitUntil: "domcontentloaded", timeout: 60000 });
  await api.wait(HYDRATION_DELAY_MS);

  // Phase 4: Read tweet (1-2 minutes)
  const tweetReadTime = mathUtils.randomInRange(60000, 120000);
  logger.info(
    `[Phase4] Reading tweet for ${(tweetReadTime / 1000).toFixed(0)}s...`,
  );
  const tweetReadStart = Date.now();
  while (Date.now() - tweetReadStart < tweetReadTime) {
    await api.scroll(mathUtils.randomInRange(100, 300));
    await api.wait(mathUtils.randomInRange(1000, 3000));
  }

  // Phase 5: Focus and retweet
  const tweetSelector = 'article[data-testid="tweet"]';
  const tweetElement = page.locator(tweetSelector).first();
  await tweetElement.waitFor({
    state: "visible",
    timeout: TWEET_VISIBLE_TIMEOUT_MS,
  });

  logger.info("[Phase5] Focusing on retweet button...");
  const retweetBtn = tweetElement.locator('[data-testid="retweet"]');
  await retweetBtn.waitFor({
    state: "visible",
    timeout: TWEET_VISIBLE_TIMEOUT_MS,
  });
  await api.scroll.focus2(retweetBtn);
  await api.wait(mathUtils.randomInRange(500, 1000));

  const result = await retweetWithAPI({ page, tweetElement });
  return result;
}

/**
 * Branch 8: Home Browse + Overscroll & Return
 * Browse home, then overscroll tweet, return, and retweet.
 * Simulates users who browse timeline then accidentally scroll too far.
 */
async function retweetBranch8_overscrollReturn(page, logger, targetUrl) {
  logger.info("[Branch8] Home Browse + Overscroll & Return...");

  // Phase 1: Browse home feed (2-3 minutes)
  const homeReadMs = mathUtils.randomInRange(120000, 180000);
  logger.info(
    `[Phase1] Browsing home feed for ${(homeReadMs / 60000).toFixed(1)} minutes...`,
  );
  await api.twitter.home({ readDurationMs: homeReadMs });

  // Phase 2: Navigate to target tweet
  logger.info("[Phase2] Navigating to target tweet...");
  await api.goto(targetUrl, { waitUntil: "domcontentloaded", timeout: 60000 });
  await api.wait(HYDRATION_DELAY_MS);

  // Phase 3: Read tweet briefly (1-2 minutes)
  const readTime = mathUtils.randomInRange(60000, 120000);
  logger.info(`[Phase3] Reading tweet for ${(readTime / 1000).toFixed(0)}s...`);
  const readStart = Date.now();
  while (Date.now() - readStart < readTime) {
    await api.scroll(mathUtils.randomInRange(100, 300));
    await api.wait(mathUtils.randomInRange(1000, 3000));
  }

  // Phase 4: Overscroll past the tweet
  const tweetSelector = 'article[data-testid="tweet"]';
  const tweetElement = page.locator(tweetSelector).first();
  await tweetElement.waitFor({
    state: "visible",
    timeout: TWEET_VISIBLE_TIMEOUT_MS,
  });

  logger.info("[Phase4] Overscrolling past tweet...");
  const overScrollCount = Math.floor(Math.random() * 3) + 2;
  for (let i = 0; i < overScrollCount; i++) {
    await api.scroll(mathUtils.randomInRange(300, 600));
    await api.wait(mathUtils.randomInRange(500, 1500));
  }

  // Pause to "realize" we scrolled too far
  logger.info("[Phase4] Pausing to realize overscroll...");
  await api.wait(mathUtils.randomInRange(2000, 4000));

  // Scroll back up to tweet
  logger.info("[Phase4] Scrolling back to tweet...");
  await api.scroll.focus2(tweetElement);
  await api.wait(mathUtils.randomInRange(500, 1000));

  // Phase 5: Focus on retweet button and execute
  logger.info("[Phase5] Focusing on retweet button...");
  const retweetBtn = tweetElement.locator('[data-testid="retweet"]');
  await retweetBtn.waitFor({
    state: "visible",
    timeout: TWEET_VISIBLE_TIMEOUT_MS,
  });
  await api.scroll.focus2(retweetBtn);
  await api.wait(mathUtils.randomInRange(500, 1000));

  const result = await retweetWithAPI({ page, tweetElement });
  return result;
}

/**
 * Branch 9: Home Browse + Search to Retweet
 * Browse home, search for content, then retweet.
 * Simulates users who search for specific topics before engaging.
 */
async function retweetBranch9_searchToRetweet(page, logger, targetUrl) {
  logger.info("[Branch9] Home Browse + Search to Retweet...");

  // Phase 1: Browse home feed (1-2 minutes)
  const homeReadMs = mathUtils.randomInRange(60000, 120000);
  logger.info(
    `[Phase1] Browsing home feed for ${(homeReadMs / 60000).toFixed(1)} minutes...`,
  );
  await api.twitter.home({ readDurationMs: homeReadMs });

  // Phase 2: Search for content (1-2 minutes)
  logger.info("[Phase2] Navigating to search...");
  await api.goto("https://x.com/search", {
    waitUntil: "domcontentloaded",
    timeout: 60000,
  });
  await api.wait(HYDRATION_DELAY_MS);

  const keywordMatch = targetUrl.match(/x\.com\/([^/]+)/);
  const searchKeyword = keywordMatch?.[1] || "tech";

  logger.info(`[Phase2] Searching for: ${searchKeyword}`);
  const searchBox = page
    .locator('[data-testid="SearchBox_Search_Input"]')
    .first();
  if (await api.visible(searchBox).catch(() => false)) {
    await api.click(searchBox);
    await api.wait(mathUtils.randomInRange(300, 600));
    await api.type(searchBox, searchKeyword);
    await api.press(searchBox, "Enter");
    await api.wait(HYDRATION_DELAY_MS);

    // Browse search results (1-2 minutes)
    const searchReadTime = mathUtils.randomInRange(60000, 120000);
    logger.info(
      `[Phase2] Browsing search results for ${(searchReadTime / 1000).toFixed(0)}s...`,
    );
    const scrollStart = Date.now();
    while (Date.now() - scrollStart < searchReadTime) {
      await api.scroll(mathUtils.randomInRange(200, 400));
      await api.wait(mathUtils.randomInRange(1000, 3000));
    }
  }

  // Phase 3: Navigate to target tweet
  logger.info("[Phase3] Navigating to target tweet...");
  await api.goto(targetUrl, { waitUntil: "domcontentloaded", timeout: 60000 });
  await api.wait(HYDRATION_DELAY_MS);

  // Phase 4: Read tweet (1-2 minutes)
  const tweetReadTime = mathUtils.randomInRange(60000, 120000);
  logger.info(
    `[Phase4] Reading tweet for ${(tweetReadTime / 1000).toFixed(0)}s...`,
  );
  const tweetReadStart = Date.now();
  while (Date.now() - tweetReadStart < tweetReadTime) {
    await api.scroll(mathUtils.randomInRange(100, 300));
    await api.wait(mathUtils.randomInRange(1000, 3000));
  }

  // Phase 5: Focus and retweet
  const tweetSelector = 'article[data-testid="tweet"]';
  const tweetElement = page.locator(tweetSelector).first();
  await tweetElement.waitFor({
    state: "visible",
    timeout: TWEET_VISIBLE_TIMEOUT_MS,
  });

  logger.info("[Phase5] Focusing on retweet button...");
  const retweetBtn = tweetElement.locator('[data-testid="retweet"]');
  await retweetBtn.waitFor({
    state: "visible",
    timeout: TWEET_VISIBLE_TIMEOUT_MS,
  });
  await api.scroll.focus2(retweetBtn);
  await api.wait(mathUtils.randomInRange(500, 1000));

  const result = await retweetWithAPI({ page, tweetElement });
  return result;
}

/**
 * Branch 10: Home Browse + Impulse Retweet
 * Browse home, then quick retweet.
 * Simulates impulse/quick reaction retweets.
 */
async function retweetBranch10_impulseRetweet(page, logger, targetUrl) {
  logger.info("[Branch10] Home Browse + Impulse Retweet...");

  // Phase 1: Browse home feed (2-3 minutes)
  const homeReadMs = mathUtils.randomInRange(120000, 180000);
  logger.info(
    `[Phase1] Browsing home feed for ${(homeReadMs / 60000).toFixed(1)} minutes...`,
  );
  await api.twitter.home({ readDurationMs: homeReadMs });

  // Phase 2: Navigate to target tweet
  logger.info("[Phase2] Navigating to target tweet...");
  await api.goto(targetUrl, { waitUntil: "domcontentloaded", timeout: 60000 });
  await api.wait(HYDRATION_DELAY_MS);

  // Phase 3: Quick reaction (minimal pause - 30s-1 min)
  const tweetSelector = 'article[data-testid="tweet"]';
  const tweetElement = page.locator(tweetSelector).first();
  await tweetElement.waitFor({
    state: "visible",
    timeout: TWEET_VISIBLE_TIMEOUT_MS,
  });

  logger.info("[Phase3] Quick reaction...");
  await api.wait(mathUtils.randomInRange(1000, 3000));

  // Quick focus and retweet
  const retweetBtn = tweetElement.locator('[data-testid="retweet"]');
  await retweetBtn.waitFor({
    state: "visible",
    timeout: TWEET_VISIBLE_TIMEOUT_MS,
  });
  await api.scroll.focus2(retweetBtn);
  await api.wait(mathUtils.randomInRange(200, 500));

  const result = await retweetWithAPI({ page, tweetElement });
  return result;
}

/**
 * Branch 11: Notifications + Home + Tweet + Retweet + Profile
 * Check notifications, browse home, read tweet, retweet, then browse author's profile.
 * Simulates thorough user engagement pattern.
 */
async function retweetBranch11_multiPhaseEngagement(page, logger, targetUrl) {
  logger.info("[Branch11] Home + Notifications + Tweet + Retweet + Profile...");

  // Phase 1: Browse home feed first (1-2 minutes)
  const homeReadMs = mathUtils.randomInRange(60000, 120000);
  logger.info(
    `[Phase1] Browsing home feed for ${(homeReadMs / 1000).toFixed(0)}s...`,
  );
  await api.twitter.home({ readDurationMs: homeReadMs });

  // Phase 2: Check notifications (1-2 minutes)
  logger.info("[Phase2] Checking notifications...");
  await api.goto("https://x.com/notifications", {
    waitUntil: "domcontentloaded",
    timeout: 60000,
  });
  await api.wait(HYDRATION_DELAY_MS);

  const notifReadTime = mathUtils.randomInRange(60000, 120000);
  logger.info(
    `[Phase2] Browsing notifications for ${(notifReadTime / 1000).toFixed(0)}s...`,
  );
  const notifStart = Date.now();
  while (Date.now() - notifStart < notifReadTime) {
    await api.scroll(mathUtils.randomInRange(200, 400));
    await api.wait(mathUtils.randomInRange(1000, 3000));
  }

  // Phase 3: Navigate to target tweet and read (1-2 minutes)
  logger.info("[Phase3] Navigating to target tweet...");
  await api.goto(targetUrl, { waitUntil: "domcontentloaded", timeout: 60000 });
  await api.wait(HYDRATION_DELAY_MS);

  const tweetReadTime = mathUtils.randomInRange(60000, 120000);
  logger.info(
    `[Phase3] Reading tweet for ${(tweetReadTime / 1000).toFixed(0)}s...`,
  );
  const tweetReadStart = Date.now();
  while (Date.now() - tweetReadStart < tweetReadTime) {
    await api.scroll(mathUtils.randomInRange(100, 300));
    await api.wait(mathUtils.randomInRange(1000, 3000));
  }

  // Phase 4: Retweet
  const tweetSelector = 'article[data-testid="tweet"]';
  const tweetElement = page.locator(tweetSelector).first();
  await tweetElement.waitFor({
    state: "visible",
    timeout: TWEET_VISIBLE_TIMEOUT_MS,
  });

  logger.info("[Phase4] Retweeting...");
  const retweetBtn = tweetElement.locator('[data-testid="retweet"]');
  await retweetBtn.waitFor({
    state: "visible",
    timeout: TWEET_VISIBLE_TIMEOUT_MS,
  });
  await api.scroll.focus2(retweetBtn);
  await api.wait(mathUtils.randomInRange(500, 1000));

  const result = await retweetWithAPI({ page, tweetElement });

  // Phase 5: Navigate to author's profile and browse (1-2 minutes)
  if (result.success) {
    logger.info("[Phase5] Navigating to author profile...");

    const profileMatch = targetUrl.match(/x\.com\/([^/]+)\/status\/\d+/);
    if (profileMatch) {
      const profileHandle = profileMatch[1];
      const profileUrl = `https://x.com/${profileHandle}`;

      logger.info(`[Phase5] Visiting profile: ${profileUrl}`);
      await api.goto(profileUrl, {
        waitUntil: "domcontentloaded",
        timeout: 60000,
      });
      await api.wait(HYDRATION_DELAY_MS);

      // Browse profile (1-2 minutes)
      const profileReadTime = mathUtils.randomInRange(60000, 120000);
      logger.info(
        `[Phase5] Browsing profile for ${(profileReadTime / 1000).toFixed(0)}s...`,
      );
      const profileStart = Date.now();
      while (Date.now() - profileStart < profileReadTime) {
        await api.scroll(mathUtils.randomInRange(200, 500));
        await api.wait(mathUtils.randomInRange(1000, 3000));
      }
    }
  }

  return result;
}

/**
 * Branch 12: Home Browse + Misclick Recovery
 * Browse home, then misclick, recover, and retweet.
 * Simulates users who browse then make human errors.
 */
async function retweetBranch12_misclickRecovery(page, logger, targetUrl) {
  logger.info("[Branch12] Home Browse + Misclick Recovery...");

  // Phase 1: Browse home feed (2-3 minutes)
  const homeReadMs = mathUtils.randomInRange(120000, 180000);
  logger.info(
    `[Phase1] Browsing home feed for ${(homeReadMs / 60000).toFixed(1)} minutes...`,
  );
  await api.twitter.home({ readDurationMs: homeReadMs });

  // Phase 2: Navigate to target tweet
  logger.info("[Phase2] Navigating to target tweet...");
  await api.goto(targetUrl, { waitUntil: "domcontentloaded", timeout: 60000 });
  await api.wait(HYDRATION_DELAY_MS);

  // Phase 3: Read tweet (1-2 minutes)
  const readTime = mathUtils.randomInRange(60000, 120000);
  logger.info(`[Phase3] Reading tweet for ${(readTime / 1000).toFixed(0)}s...`);
  const readStart = Date.now();
  while (Date.now() - readStart < readTime) {
    await api.scroll(mathUtils.randomInRange(100, 300));
    await api.wait(mathUtils.randomInRange(1000, 3000));
  }

  // Phase 4: Simulate misclick
  const tweetSelector = 'article[data-testid="tweet"]';
  const tweetElement = page.locator(tweetSelector).first();
  await tweetElement.waitFor({
    state: "visible",
    timeout: TWEET_VISIBLE_TIMEOUT_MS,
  });

  const likeBtn = tweetElement.locator('[data-testid="like"]').first();
  const replyBtn = tweetElement.locator('[data-testid="reply"]').first();

  const misclickTarget = Math.random() > 0.5 ? likeBtn : replyBtn;
  if (await api.visible(misclickTarget).catch(() => false)) {
    logger.info("[Phase4] Simulating misclick...");
    await api.scroll.focus2(misclickTarget);
    await api.wait(mathUtils.randomInRange(200, 500));
    await api.click(misclickTarget);
    await api.wait(mathUtils.randomInRange(500, 1000));

    logger.info("[Phase4] Recovering from misclick...");
    await page.keyboard.press("Escape");
    await api.wait(mathUtils.randomInRange(1000, 2000));
  }

  // Phase 5: Now correctly click retweet
  logger.info("[Phase5] Now clicking retweet correctly...");
  const retweetBtn = tweetElement.locator('[data-testid="retweet"]');
  await retweetBtn.waitFor({
    state: "visible",
    timeout: TWEET_VISIBLE_TIMEOUT_MS,
  });
  await api.scroll.focus2(retweetBtn);
  await api.wait(mathUtils.randomInRange(500, 1000));

  const result = await retweetWithAPI({ page, tweetElement });
  await api.wait(mathUtils.randomInRange(3000, 6000)); // wait for 3-6 seconds after retweet
  return result;
}

/**
 * Retweet Test Task
 * Navigates to a specific tweet/profile and executes the retweet function to verify robustness.
 * @param {object} page - Playwright page instance
 * @param {object} payload - Task payload
 * @returns {Promise<void>}
 */
export default async function retweetTestTask(page, payload) {
  const logger = createLogger("retweet.js");
  const browserInfo = payload.browserInfo || "unknown_profile";

  logger.info("Starting retweet.js task...");

  return await api.withPage(
    page,
    async () => {
      // Initialize API context
      await api.init(page, {
        logger,
        patch: false,
        humanizationPatch: true,
        autoInitNewPages: true,
        colorScheme: "dark",
        sensors: false,
      });

      const targetUrl = payload?.url || "https://x.com"; // Default to X.com if no payload URL is provided
      logger.info(`Target URL: ${targetUrl}`);

      // Profile resolution
      const profile = profileManager.getStarter() || {
        id: "test-user",
        type: "test",
      };
      logger.info(`Profile: ${profile.id} (${profile.type || "unknown"})`);

      // Timeout configuration
      const hardTimeoutMs = payload.taskTimeoutMs || DEFAULT_TIMEOUT_MS;

      let timeoutId;
      try {
        await Promise.race([
          // Main execution with retry logic
          (async () => {
            for (let attempt = 0; attempt <= MAX_RETRIES; attempt++) {
              try {
                if (attempt > 0) {
                  const delay = Math.pow(2, attempt) * 1000;
                  logger.info(
                    `Retry ${attempt}/${MAX_RETRIES} in ${delay}ms...`,
                  );
                  await api.wait(delay);
                }

                // Generate referrer context for natural navigation
                const referrerEngine = new ReferrerEngine({ addUTM: true });
                const refCtx = referrerEngine.generateContext(targetUrl);
                logger.info(
                  `[Referrer] Strategy: ${refCtx.strategy}, Referrer: ${refCtx.referrer || "(direct)"}`,
                );

                // Navigate to target URL with referrer
                logger.info(`Navigating to ${targetUrl}...`);
                await api.goto(targetUrl, {
                  waitUntil: "domcontentloaded",
                  timeout: 60000,
                  referer: refCtx.referrer || undefined,
                });
                logger.info("Navigation complete.");

                // Hydration delay
                logger.info(`Waiting ${HYDRATION_DELAY_MS}ms for hydration...`);
                await api.wait(HYDRATION_DELAY_MS);

                // Select and execute retweet branch
                const selectedBranch = selectRetweetBranch(payload, logger);
                logger.info(`Selected branch: ${selectedBranch}`);

                let result;
                switch (selectedBranch) {
                  case "retweetBranch1_directRetweet":
                    result = await retweetBranch1_directRetweet(
                      page,
                      logger,
                      targetUrl,
                    );
                    break;
                  case "retweetBranch2_profileVisitRetweet":
                    result = await retweetBranch2_profileVisitRetweet(
                      page,
                      logger,
                      targetUrl,
                    );
                    break;
                  case "retweetBranch3_homeReadRetweet":
                    result = await retweetBranch3_homeReadRetweet(
                      page,
                      logger,
                      targetUrl,
                    );
                    break;
                  case "retweetBranch4_homeProfileRetweet":
                    result = await retweetBranch4_homeProfileRetweet(
                      page,
                      logger,
                      targetUrl,
                    );
                    break;
                  case "retweetBranch5_threadReader":
                    result = await retweetBranch5_threadReader(
                      page,
                      logger,
                      targetUrl,
                    );
                    break;
                  case "retweetBranch6_likeRetweet":
                    result = await retweetBranch6_likeRetweet(
                      page,
                      logger,
                      targetUrl,
                    );
                    break;
                  case "retweetBranch7_notificationCheck":
                    result = await retweetBranch7_notificationCheck(
                      page,
                      logger,
                      targetUrl,
                    );
                    break;
                  case "retweetBranch8_overscrollReturn":
                    result = await retweetBranch8_overscrollReturn(
                      page,
                      logger,
                      targetUrl,
                    );
                    break;
                  case "retweetBranch9_searchToRetweet":
                    result = await retweetBranch9_searchToRetweet(
                      page,
                      logger,
                      targetUrl,
                    );
                    break;
                  case "retweetBranch10_impulseRetweet":
                    result = await retweetBranch10_impulseRetweet(
                      page,
                      logger,
                      targetUrl,
                    );
                    break;
                  case "retweetBranch11_multiPhaseEngagement":
                    result = await retweetBranch11_multiPhaseEngagement(
                      page,
                      logger,
                      targetUrl,
                    );
                    break;
                  case "retweetBranch12_misclickRecovery":
                    result = await retweetBranch12_misclickRecovery(
                      page,
                      logger,
                      targetUrl,
                    );
                    break;
                  default:
                    result = await retweetBranch1_directRetweet(page, logger);
                }

                logger.info(
                  `Retweet execution finished. Success: ${result.success}`,
                );

                // Observation delay
                logger.info(
                  `Waiting ${OBSERVATION_DELAY_MS}ms before finishing...`,
                );
                await api.wait(OBSERVATION_DELAY_MS);

                if (result.success) {
                  logger.info(
                    `✅ Retweet Test PASSED. Branch: ${selectedBranch}, Reason: ${result.reason}`,
                  );
                  return createSuccessResult('retweet', {
                    branch: selectedBranch,
                    reason: result.reason,
                    targetUrl
                  }, { startTime: Date.now(), sessionId: browserInfo });
                } else {
                  logger.error(
                    `❌ Retweet Test FAILED. Branch: ${selectedBranch}, Reason: ${result.reason}`,
                  );
                  return createFailedResult('retweet', result.reason, {
                    partialData: { branch: selectedBranch, targetUrl }
                  });
                }
              } catch (innerError) {
                logger.warn(
                  `Attempt ${attempt + 1} failed: ${innerError.message}`,
                );

                // Check for login page
                const loginSelector = '[data-testid="login"]';
                const loginCount = await page.locator(loginSelector).count();
                if (loginCount > 0) {
                  logger.error("Login page detected. Please log in first.");
                }

                if (attempt === MAX_RETRIES) throw innerError;
              }
            }
          })(),
          // Timeout promise
          new Promise((_, reject) => {
            timeoutId = setTimeout(() => {
              const timeoutError = new Error("Retweet task timeout");
              reject(timeoutError);
            }, hardTimeoutMs);
          }),
        ]);
      } finally {
        if (timeoutId) clearTimeout(timeoutId);

        // Cleanup
        api.clearContext();
        logger.info("retweet.js task completed.");
      }
    },
    { sessionId: browserInfo },
  );
}
