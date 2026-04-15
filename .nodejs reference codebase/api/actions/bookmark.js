/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { getPage } from '../core/context.js';
import { createLogger } from '../core/logger.js';
import { mathUtils } from '../utils/math.js';
import { wait } from '../interactions/wait.js';
import { visible } from '../interactions/queries.js';
import { click } from '../interactions/actions.js';
import metricsCollector from '../utils/metrics.js';

const logger = createLogger('api/bookmark.js');

/**
 * Bookmark the currently focused tweet on-page.
 *
 * Performs:
 * 1. Locate bookmark button on the active tweet article
 * 2. Already-bookmarked guard
 * 3. Human click + toast verification
 *
 * @param {object} [options]
 * @param {object} [options.tweetElement] - Optional Playwright locator for the tweet
 * @returns {Promise<{success: boolean, reason: string, method: string}>}
 */
export async function bookmarkWithAPI(options = {}) {
    getPage(); // Ensure context is set
    const { tweetElement } = options;

    logger.info(`Starting api.bookmarkWithAPI()...`);

    // X.com uses aria-label "Bookmark" / "Remove Bookmark" on the caret menu button
    const bookmarkSel = '[data-testid="bookmark"]';
    const removeBookmarkSel = '[data-testid="removeBookmark"]';

    try {
        // Already bookmarked guard
        const alreadyBookmarked = tweetElement
            ? await tweetElement
                  .locator(removeBookmarkSel)
                  .first()
                  .isVisible()
                  .catch(() => false)
            : await visible(removeBookmarkSel);

        if (alreadyBookmarked) {
            logger.info(`Tweet already bookmarked, skipping.`);
            return { success: true, reason: 'already_bookmarked', method: 'bookmarkAPI' };
        }

        // Scroll into view
        if (tweetElement) {
            await tweetElement.scrollIntoViewIfNeeded().catch(() => {});
        }
        await wait(mathUtils.randomInRange(300, 700));

        // Click with ghost cursor
        const target = tweetElement ? tweetElement.locator(bookmarkSel).first() : bookmarkSel;

        logger.info(`[bookmarkWithAPI] Clicking bookmark button (ghost cursor)...`);
        await click(target);

        await wait(mathUtils.randomInRange(600, 1200));

        // Verify via toast
        const toastSel = '[data-testid="toast"]';
        const toastVisible = await visible(toastSel);
        const nowRemovable = tweetElement
            ? await tweetElement
                  .locator(removeBookmarkSel)
                  .first()
                  .isVisible()
                  .catch(() => false)
            : await visible(removeBookmarkSel);

        if (toastVisible || nowRemovable) {
            logger.info(`✅ api.bookmarkWithAPI successful!`);
            metricsCollector.recordTwitterEngagement('bookmark', 1);
            return { success: true, reason: 'success', method: 'bookmarkAPI' };
        }

        logger.warn(`❌ api.bookmarkWithAPI: no confirmation signal detected`);
        return { success: false, reason: 'verification_failed', method: 'bookmarkAPI' };
    } catch (error) {
        logger.error(`api.bookmarkWithAPI error: ${error.message}`);
        return { success: false, reason: error.message, method: 'bookmarkAPI' };
    }
}
