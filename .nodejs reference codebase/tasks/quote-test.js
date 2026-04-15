/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { createLogger } from '../api/core/logger.js';
import { api } from '../api/index.js';

/**
 * Quote Test Task (Refactored)
 * Tests the "Quote Tweet" functionality using the new api.quoteWithAI()
 *
 * Usage: node main.js quote-test
 */
export default async function quoteTestTask(page, payload) {
    const logger = createLogger('quote-test.js');
    const targetUrl =
        payload?.url ||
        process.env.TARGET_URL ||
        'https://x.com/SightlyGirls/status/2026039840634994891';

    logger.info(`Starting refactored quote test task...`);

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

            // 2. Perform Quote with AI (Integrated high-level function)
            const result = await api.quoteWithAI();

            if (result.success) {
                logger.info(`✅ Quote Test PASSED via ${result.method}`);
            } else {
                logger.error(`❌ Quote Test FAILED: ${result.reason || 'unknown'}`);
            }

            // Keep page open for observation
            logger.info(`Waiting 1 seconds before finishing...`);
            await api.wait(1000);

            return result;
        } catch (error) {
            logger.error(`Error during quote test: ${error.message}`);
            throw error;
        }
    });
}
