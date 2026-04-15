/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { createLogger } from '../api/core/logger.js';
import { api } from '../api/index.js';
import { mathUtils } from '../api/utils/mathUtils.js';

/**
 * Follow Test Task
 * Tests the full follow flow:
 *   1. Navigate to a tweet URL
 *   2. Extract the author's profile URL from the tweet URL
 *   3. Navigate to the profile
 *   4. Call followWithAPI()
 *
 * Usage: node main.js follow-test
 * Custom tweet: node main.js follow-test --url https://x.com/SomeUser/status/123456
 */
export default async function followTestTask(page, payload) {
    const logger = createLogger('follow-test.js');

    // Default to a known tweet — override with payload.url or env var
    const tweetUrl =
        payload?.url ||
        process.env.TARGET_URL ||
        'https://x.com/JTomCorVi/status/2028699457823342755';

    // Derive profile URL: strip /status/{id} from tweet URL
    const profileUrl = tweetUrl.replace(/\/status\/\d+.*/, '');
    const username = profileUrl.replace(/https?:\/\/x\.com\//, '');

    logger.info(`[follow-test] Tweet  : ${tweetUrl}`);
    logger.info(`[follow-test] Profile: ${profileUrl} (@${username})`);

    return api.withPage(page, async () => {
        try {
            await api.init(page, { logger });

            // 0. Human warmup — simulate user picking up the browser
            const warmup = mathUtils.randomInRange(2000, 5000);
            logger.info(`[follow-test] Warmup: waiting ${(warmup / 1000).toFixed(1)}s...`);
            await api.wait(warmup);

            // 1. Navigate to the tweet first (simulates arriving via feed dive)
            logger.info(`[follow-test] Navigating to tweet...`);
            await api.goto(tweetUrl, {
                waitUntil: 'domcontentloaded',
                timeout: 30000,
                warmup: false,
            });
            await api.wait(mathUtils.randomInRange(2500, 4500));

            // 2. Navigate to profile — click author username link (ghost cursor)
            logger.info(`[follow-test] Navigating to profile: ${profileUrl}`);
            const authorSel = `[data-testid="User-Name"] a[href="/${username}"]`;
            try {
                logger.info(`[follow-test] Clicking author name link (ghost cursor)...`);
                await api.click(authorSel);
            } catch (_e) {
                logger.info(
                    `[follow-test] Click failed (${_e.message}) api.click FAILED ,no api.goto fallback for stealth`
                );
            }
            await api.wait(mathUtils.randomInRange(2000, 4000));

            // 3. Execute follow
            logger.info(`[follow-test] Calling followWithAPI(@${username})...`);
            const result = await api.followWithAPI({ username });

            if (result.success) {
                logger.info(
                    `✅ Follow Test PASSED  reason=${result.reason}  method=${result.method}`
                );
            } else {
                logger.error(
                    `❌ Follow Test FAILED  reason=${result.reason}  method=${result.method}`
                );
            }

            // 4. Keep page open briefly for observation
            logger.info(`[follow-test] Waiting 5s before finishing...`);
            await api.wait(5000);

            return result;
        } catch (error) {
            logger.error(`[follow-test] Error: ${error.message}`);
            throw error;
        }
    });
}
