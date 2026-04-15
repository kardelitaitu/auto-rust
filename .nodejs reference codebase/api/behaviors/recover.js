/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Error Recovery Behavior
 * Human-like error handling and recovery for automation actions.
 *
 * @module api/recover
 */

import { getPage } from '../core/context.js';
import { think, delay, randomInRange, gaussian } from './timing.js';
import { scroll } from '../interactions/scroll.js';
import { createLogger } from '../core/logger.js';

const logger = createLogger('api/recover.js');

import { getPreviousUrl } from '../core/context-state.js';

/**
 * Recover from unexpected navigation.
 * If URL changed after an action, go back to restore previous state.
 * @returns {Promise<boolean>} - True if recovery was needed and performed
 */
export async function recover() {
    const page = getPage();
    const currentUrl = page.url();
    const prevUrl = getPreviousUrl();

    if (prevUrl && currentUrl !== prevUrl) {
        logger.warn(`[Recover] URL changed from ${prevUrl} to ${currentUrl}. Restoring...`);
        await goBack();
        return true;
    }

    return false;
}

/**
 * Check if URL changed unexpectedly after an action.
 * @param {string} previousUrl - URL before action
 * @returns {Promise<boolean>}
 */
export async function urlChanged(previousUrl) {
    const page = getPage();
    const currentUrl = page.url();
    return currentUrl !== previousUrl;
}

/**
 * Go back in history - used when wrong click caused navigation.
 * @returns {Promise<void>}
 */
export async function goBack() {
    const page = getPage();
    await think(randomInRange(500, 1500)); // Brief "confusion" pause
    await page.goBack();
    await delay(randomInRange(1000, 2000)); // Wait for page to load
}

/**
 * Find element by scrolling and searching.
 * Used when element is not immediately visible.
 * @param {string|string[]} selectors - CSS selector or array of selectors to find
 * @param {object} [options]
 * @param {number} [options.maxRetries=3] - Maximum scroll/search cycles
 * @param {boolean} [options.scrollOnFail=true] - Scroll when not found
 * @returns {Promise<string|null>} - Found selector or null
 */
export async function findElement(selectors, options = {}) {
    const page = getPage();
    const { maxRetries = 3, scrollOnFail = true } = options;
    const selectorList = Array.isArray(selectors) ? selectors : [selectors];

    for (let attempt = 0; attempt < maxRetries; attempt++) {
        for (const selector of selectorList) {
            // Check if element exists
            const count = await page.locator(selector).count();
            if (count > 0) {
                const isVisible = await page
                    .locator(selector)
                    .first()
                    .isVisible()
                    .catch(() => false);
                if (isVisible) {
                    return selector;
                }
            }
        }

        // Scroll down to look for element
        if (scrollOnFail && attempt < maxRetries - 1) {
            // Entropy: Random scroll amount
            const scrollAmount = gaussian(400, 150, 200, 800);
            await scroll(scrollAmount);
            await delay(randomInRange(500, 1000));
        }
    }

    return null;
}

/**
 * Smart click with error recovery and entropy-based retries.
 * @param {string|string[]} selectors - CSS selector or array of selectors
 * @param {object} [options]
 * @param {boolean} [options.recovery=true] - Enable error recovery
 * @param {number} [options.maxRetries=3] - Max retry attempts
 * @param {boolean} [options.scrollOnFail=true] - Scroll and try again if not found
 * @param {boolean} [options.expectsNavigation=false] - Disable auto-rollback on expected navigation
 * @returns {Promise<{success: boolean, recovered: boolean, selector: string|null}>}
 */
export async function smartClick(selectors, options = {}) {
    const page = getPage();
    const {
        recovery = true,
        maxRetries = 3,
        scrollOnFail = true,
        expectsNavigation = false,
    } = options;

    const selectorList = Array.isArray(selectors) ? selectors : [selectors];
    const previousUrl = page.url();

    for (let attempt = 0; attempt < maxRetries; attempt++) {
        for (const selector of selectorList) {
            try {
                // Try to find element
                const locator = page.locator(selector).first();
                const isVisible = await locator.isVisible().catch(() => false);

                if (!isVisible) {
                    continue; // Try next selector in list
                }

                // Entropy: Gaussian Coordinate Noise
                try {
                    const box = await locator.boundingBox();
                    if (box) {
                        const x = gaussian(
                            box.x + box.width / 2,
                            box.width / 6,
                            box.x,
                            box.x + box.width
                        );
                        const y = gaussian(
                            box.y + box.height / 2,
                            box.height / 6,
                            box.y,
                            box.y + box.height
                        );

                        logger.debug(
                            `[SmartClick] Clicking at (${x}, ${y}) for ${selector} (Attempt ${attempt + 1})`
                        );
                        await page.mouse.click(x, y);
                    } else {
                        await locator.click({ timeout: 2000 });
                    }
                } catch (clickError) {
                    logger.warn(
                        `[SmartClick] Coordinate click failed, falling back to standard click: ${clickError.message}`
                    );
                    await locator.click({ timeout: 2000 });
                }

                // Check if URL changed unexpectedly
                if (recovery && !expectsNavigation) {
                    const changed = await urlChanged(previousUrl);
                    if (changed) {
                        logger.warn(
                            `[SmartClick] Unexpected navigation after click on ${selector}. Recovering...`
                        );
                        await think(randomInRange(500, 1500));
                        await goBack();
                        return { success: false, recovered: true, selector };
                    }
                }

                return { success: true, recovered: false, selector };
            } catch (error) {
                logger.warn(
                    `[SmartClick] Attempt ${attempt + 1} failed for ${selector}: ${error.message}`
                );
            }
        }

        // If no selector worked in this attempt, scroll and retry
        if (scrollOnFail && attempt < maxRetries - 1) {
            const scrollAmount = gaussian(300, 100, 100, 600);
            await scroll(scrollAmount);
            await think(randomInRange(500, 1000));
        } else if (attempt < maxRetries - 1) {
            // Exponential backoff with jitter
            const basePause = Math.pow(2, attempt) * 1000;
            const pauseTime = gaussian(basePause, basePause * 0.2);
            await think(pauseTime);
        }
    }

    return { success: false, recovered: false, selector: null };
}

/**
 * Undo last action if possible.
 * Currently supports going back in history.
 * @returns {Promise<boolean>} - True if undo was performed
 */
export async function undo() {
    // const page = getPage();
    // Note: page.history() may not be available in all Playwright versions
    // This is a simplified implementation
    await goBack();
    return true;
}
