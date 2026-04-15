/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { createLogger } from '../api/core/logger.js';
import { api } from '../api/index.js';

/**
 * Reply Test Task (Refactored)
 * Tests the "Reply Tweet" functionality using the new api.replyWithAI() helper.
 *
 * Usage: node main.js reply-test
 */
export default async function replyTestTask(page, payload) {
    const logger = createLogger('reply-test.js');
    const targetUrl =
        payload?.url ||
        process.env.TARGET_URL ||
        'https://x.com/SightlyGirls/status/2026039840634994891';

    logger.info(`Starting refactored reply test task...`);

    return api.withPage(page, async () => {
        try {
            await api.init(page, { logger });

            // 1. Navigate to target tweet
            logger.info(`Navigating to ${targetUrl}...`);
            await api.goto(targetUrl, {
                waitUntil: 'domcontentloaded',
                timeout: 60000,
                warmup: false,
            });
            await api.wait(3000);

            // 2. Perform Reply with AI (Integrated high-level function)
            const result = await api.replyWithAI();

            if (result.success) {
                logger.info(`✅ Reply Test PASSED via ${result.method}`);
            } else {
                logger.error(`❌ Reply Test FAILED: ${result.reason || 'unknown'}`);
            }

            // Keep page open for observation
            logger.info(`Waiting 5 seconds before finishing...`);
            await api.wait(5000);

            return result;
        } catch (error) {
            logger.error(`Error during reply test: ${error.message}`);
            throw error;
        }
    });
}
