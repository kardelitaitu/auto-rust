/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Twitter Intent Test Task
 * Tests the four new Twitter intent URL helpers: like, quote, retweet, and follow.
 *
 * Usage:
 * node main.js twitter-intents-test
 * OR with custom payload:
 * node main.js twitter-intents-test='{"likeUrl":"...","quoteUrl":"...","quoteText":"...","retweetUrl":"...","followUrl":"..."}'
 */

import { api } from "../api/index.js";
import { createLogger } from "../api/core/logger.js";

export default async function twitterIntentsTestTask(page, payload) {
  const logger = createLogger("twitter-intents-test.js");
  const browserInfo = payload.browserInfo || "unknown_profile";

  logger.info(`Starting Twitter Intents Test Task for profile: ${browserInfo}`);

  return api.withPage(page, async () => {
    try {
      await api.init(page, { logger });
      const initialURL = payload?.initialURL || "https://www.google.com";
      await api.goto(initialURL);
      await api.wait(api.randomInRange(3000, 5000));

      // 1. Test Like Intent
      const likeUrl =
        payload?.likeUrl ||
        "https://x.com/arojinle1/status/2030191547656892514";
      // logger.info(`Testing LIKE intent for: ${likeUrl}`);
      const likeResult = await api.twitter.intent.like(likeUrl);
      // logger.info(`LIKE Result: ${JSON.stringify(likeResult)}`);

      await api.wait(api.randomInRange(3000, 5000));

      // 2. Test Retweet Intent
      const retweetUrl = payload?.retweetUrl || likeUrl;
      // logger.info(`Testing RETWEET intent for: ${retweetUrl}`);
      const retweetResult = await api.twitter.intent.retweet(retweetUrl);
      // logger.info(`RETWEET Result: ${JSON.stringify(retweetResult)}`);

      await api.wait(api.randomInRange(3000, 5000));

      // 3. Test Quote Intent
      const quoteUrl = payload?.quoteUrl || likeUrl;
      const quoteText = payload?.quoteText || "wow 🚀\n\n:)";
      // logger.info(`Testing QUOTE intent for: ${quoteUrl}`);
      const quoteResult = await api.twitter.intent.quote(quoteUrl, quoteText);
      // logger.info(`QUOTE Result: ${JSON.stringify(quoteResult)}`);
      await api.wait(api.randomInRange(3000, 5000));

      // 4. Test Follow Intent
      const followUrl = payload?.followUrl || "https://x.com/arojinle1";
      // logger.info(`Testing FOLLOW intent for: ${followUrl}`);
      const followResult = await api.twitter.intent.follow(followUrl);
      // logger.info(`FOLLOW Result: ${JSON.stringify(followResult)}`);

      // 5. Test Post Intent
      const postText =
        payload?.postText || "This is a test post using url intent";
      // logger.info(`Testing POST intent with text: ${postText}`);
      const postResult = await api.twitter.intent.post(postText);
      // logger.info(`POST Result: ${JSON.stringify(postResult)}`);

      logger.info("✅ Twitter Intents End-to-End Test completed.");

      return {
        like: likeResult,
        retweet: retweetResult,
        quote: quoteResult,
        follow: followResult,
        post: postResult,
      };
    } catch (error) {
      logger.error(`Error during Twitter intents test task: ${error.message}`);
      throw error;
    }
  });
}
