/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { api } from '../index.js';
/**
 * Motor Control Module
 * Provides robust, human-like click targeting with recovery behaviors:
 * - Continuous target tracking (handles layout shifts)
 * - Visual overlap protection
 * - Micro-correction & spiraling recovery
 * - Smart selector fallback
 *
 * @module utils/motor-control
 */

import { calculateBackoffDelay } from '../utils/retry.js';

const MOTOR_CONFIG = {
    layoutShiftThreshold: 5,
    spiralSearchAttempts: 4,
    spiralOffsets: [
        { dx: 0, dy: -10 },
        { dx: 0, dy: 10 },
        { dx: -10, dy: 0 },
        { dx: 10, dy: 0 },
        { dx: -8, dy: -8 },
        { dx: 8, dy: 8 },
    ],
    retryDelay: 100,
    maxRetries: 3,
    targetTimeout: 5000,
    scrollRecoveryAmount: 150,
};

function createMotorController(options = {}) {
    const config = { ...MOTOR_CONFIG, ...options };
    // let trackedElements = new Map();

    return {
        config,

        /**
         * Get X.com specific smart selectors for common elements
         */
        getXSelectors(context) {
            switch (context) {
                case 'tweet_text':
                    return {
                        primary: '[data-testid="tweetText"]',
                        fallbacks: [
                            {
                                selector: 'article [role="group"] a[href*="/status"]',
                                reason: 'permalink_link',
                            },
                            { selector: 'time[datetime]', reason: 'timestamp' },
                            {
                                selector: 'article a[href*="/status"]',
                                reason: 'article_status_link',
                            },
                            { selector: '[lang] > div', reason: 'lang_div' },
                        ],
                    };
                case 'reply':
                    return {
                        primary: '[data-testid="reply"]',
                        fallbacks: [
                            { selector: '[aria-label="Reply"]', reason: 'aria_reply' },
                            { selector: 'svg[aria-label*="Reply"]', reason: 'svg_reply' },
                            { selector: 'button:has-text("Reply")', reason: 'text_reply' },
                        ],
                    };
                case 'retweet':
                    return {
                        primary: '[data-testid="retweet"]',
                        fallbacks: [
                            { selector: '[aria-label="Retweet"]', reason: 'aria_retweet' },
                            { selector: 'svg[aria-label*="Retweet"]', reason: 'svg_retweet' },
                            { selector: 'button:has-text("Repost")', reason: 'text_repost' },
                        ],
                    };
                case 'like':
                    return {
                        primary: '[data-testid="like"]',
                        fallbacks: [
                            { selector: '[aria-label="Like"]', reason: 'aria_like' },
                            { selector: 'svg[aria-label*="Like"]', reason: 'svg_like' },
                            { selector: 'button:has-text("Like")', reason: 'text_like' },
                        ],
                    };
                case 'bookmark':
                    return {
                        primary: '[data-testid="bookmark"]',
                        fallbacks: [
                            { selector: '[aria-label="Bookmark"]', reason: 'aria_bookmark' },
                            { selector: 'svg[aria-label*="Bookmark"]', reason: 'svg_bookmark' },
                        ],
                    };
                case 'follow':
                    return {
                        primary: '[data-testid="follow"]',
                        fallbacks: [
                            { selector: '[data-testid="followButton"]', reason: 'followButton' },
                            { selector: 'button:has-text("Follow")', reason: 'text_follow' },
                        ],
                    };
                case 'home':
                    return {
                        primary: '[aria-label="Home"]',
                        fallbacks: [
                            { selector: '[data-testid="appleLogo"]', reason: 'appleLogo' },
                            { selector: '[data-testid="logo"]', reason: 'logo' },
                            { selector: 'a[href="/home"]', reason: 'home_link' },
                        ],
                    };
                default:
                    return { primary: context, fallbacks: [] };
            }
        },

        /**
         * Smart Selector Fallback
         * Tries primary selector, then falls back to alternatives
         */
        async smartSelector(page, primary, fallbacks = [], options = {}) {
            const { logger = console } = options;

            try {
                const primaryEl = await page.$(primary);
                if (primaryEl) {
                    const isVisible = await api.visible(primaryEl).catch(() => false);
                    if (isVisible) {
                        logger.info(`[Motor] Primary selector found: ${primary}`);
                        return { selector: primary, element: primaryEl, usedFallback: false };
                    }
                }
            } catch (error) {
                logger.debug(`[Motor] Primary selector error: ${error.message}`);
            }

            for (let i = 0; i < fallbacks.length; i++) {
                const fallback = fallbacks[i];

                try {
                    const fallbackEl = await page.$(fallback.selector);
                    if (fallbackEl) {
                        const isVisible = await api.visible(fallbackEl).catch(() => false);
                        if (isVisible) {
                            logger.info(
                                `[Motor] Using fallback selector [${i + 1}/${fallbacks.length}]: ${fallback.selector}`
                            );
                            return {
                                selector: fallback.selector,
                                element: fallbackEl,
                                usedFallback: true,
                                reason: fallback.reason || 'primary_not_found',
                            };
                        }
                    }
                } catch (error) {
                    logger.debug(`[Motor] Fallback ${i + 1} error: ${error.message}`);
                }
            }

            logger.warn(`[Motor] No selector found for: ${primary}`);
            return { selector: primary, element: null, usedFallback: false, reason: 'not_found' };
        },

        async getStableTarget(page, selector, options = {}) {
            const { timeout = config.targetTimeout, scrollFirst: _scrollFirst = true } = options;

            const startTime = Date.now();
            let lastBox = null;
            let stableCount = 0;
            const stableThreshold = 3;

            while (Date.now() - startTime < timeout) {
                try {
                    const element = await page.$(selector);

                    if (!element) {
                        await api.wait(1000);
                        continue;
                    }

                    const box = await element.boundingBox();

                    if (!box) {
                        await api.wait(1000);
                        continue;
                    }

                    if (lastBox) {
                        const dx = Math.abs(box.x - lastBox.x);
                        const dy = Math.abs(box.y - lastBox.y);

                        if (dx < config.layoutShiftThreshold && dy < config.layoutShiftThreshold) {
                            stableCount++;

                            if (stableCount >= stableThreshold) {
                                return { success: true, box, stable: true };
                            }
                        } else {
                            stableCount = 0;
                        }
                    }

                    lastBox = box;
                    await api.wait(1000);
                } catch (_error) {
                    await api.wait(1000);
                }
            }

            return { success: false, reason: 'timeout', lastBox };
        },

        async checkOverlap(page, x, y) {
            try {
                const element = await page.evaluate(
                    (x, y) => {
                        return document.elementFromPoint(x, y);
                    },
                    x,
                    y
                );

                return element;
            } catch (_error) {
                return null;
            }
        },

        async findUncoveredArea(page, box, options = {}) {
            const { logger = console } = options;

            const offsets = [
                { dx: 0, dy: -50 },
                { dx: 0, dy: 50 },
                { dx: -50, dy: 0 },
                { dx: 50, dy: 0 },
                { dx: 0, dy: -100 },
                { dx: 0, dy: 100 },
            ];

            for (const offset of offsets) {
                const testX = box.x + box.width / 2 + offset.dx;
                const testY = box.y + box.height / 2 + offset.dy;

                const element = await this.checkOverlap(page, testX, testY);

                if (!element) {
                    logger.info(
                        `[Motor] Found uncovered area at (${Math.round(testX)}, ${Math.round(testY)})`
                    );
                    return { success: true, x: testX, y: testY };
                }
            }

            return { success: false, reason: 'no_uncovered_area' };
        },

        async spiralSearch(page, targetX, targetY, options = {}) {
            const { logger = console, maxAttempts = config.spiralSearchAttempts } = options;

            for (let i = 0; i < maxAttempts; i++) {
                const offset = config.spiralOffsets[i % config.spiralOffsets.length];
                const spiralX = targetX + offset.dx;
                const spiralY = targetY + offset.dy;

                logger.info(
                    `[Motor] Spiral search attempt ${i + 1}: (${Math.round(spiralX)}, ${Math.round(spiralY)})`
                );

                const element = await this.checkOverlap(page, spiralX, spiralY);

                if (!element) {
                    return { success: true, x: spiralX, y: spiralY, attempts: i + 1 };
                }
            }

            return { success: false, reason: 'spiral_failed' };
        },

        async clickWithRecovery(page, selector, options = {}) {
            const { logger = console, recovery = 'scroll', timeout = 5000 } = options;

            try {
                const stable = await this.getStableTarget(page, selector, { timeout });

                if (!stable.success) {
                    logger.warn(`[Motor] Could not stabilize target: ${selector}`);

                    if (recovery === 'scroll') {
                        await page.evaluate(() => window.scrollBy(0, 200));
                        await api.wait(1000);

                        return await this.clickWithRecovery(page, selector, {
                            ...options,
                            recovery: 'spiral',
                        });
                    }

                    return { success: false, reason: 'no_stable_target' };
                }

                const _element = await page.$(selector);
                const box = stable.box;
                const targetX = box.x + box.width / 2;
                const targetY = box.y + box.height / 2;

                const overlap = await this.checkOverlap(page, targetX, targetY);

                if (overlap) {
                    logger.warn(
                        `[Motor] Element overlapped at target point, finding alternative...`
                    );

                    const uncovered = await this.findUncoveredArea(page, box, { logger });

                    if (uncovered.success) {
                        await page.mouse.click(uncovered.x, uncovered.y);
                        return { success: true, x: uncovered.x, y: uncovered.y, recovered: true };
                    }

                    const spiral = await this.spiralSearch(page, targetX, targetY, { logger });

                    if (spiral.success) {
                        await page.mouse.click(spiral.x, spiral.y);
                        return { success: true, x: spiral.x, y: spiral.y, recovered: true };
                    }

                    return { success: false, reason: 'overlapped' };
                }

                await page.mouse.click(targetX, targetY);

                return { success: true, x: targetX, y: targetY, recovered: false };
            } catch (error) {
                logger.error(`[Motor] Click error: ${error.message}`);
                return { success: false, reason: error.message };
            }
        },

        async clickWithVerification(page, selector, options = {}) {
            const { logger = console, verifySelector = null, verifyTimeout = 500 } = options;

            const result = await this.clickWithRecovery(page, selector, { logger, ...options });

            if (result.success && verifySelector) {
                await api.wait(1000);

                try {
                    const verified = await page.waitForSelector(verifySelector, {
                        timeout: verifyTimeout,
                    });

                    if (verified) {
                        logger.info(`[Motor] Click verified: ${verifySelector} appeared`);
                        return { ...result, verified: true };
                    }
                } catch {
                    logger.warn(`[Motor] Click not verified: ${verifySelector} did not appear`);
                    return { ...result, verified: false };
                }
            }

            return result;
        },

        async scrollToElement(page, selector, options = {}) {
            const { offset = 100, smooth = true } = options;

            try {
                const element = await page.$(selector);

                if (!element) {
                    return { success: false, reason: 'no_element' };
                }

                const box = await element.boundingBox();

                if (!box) {
                    return { success: false, reason: 'no_box' };
                }

                const targetY = box.y - offset;

                await page.evaluate(
                    (y, smoothScroll) => {
                        window.scrollTo({ top: y, behavior: smoothScroll ? 'smooth' : 'auto' });
                    },
                    targetY,
                    smooth
                );

                await api.wait(1000);

                return { success: true, y: targetY };
            } catch (error) {
                return { success: false, reason: error.message };
            }
        },

        async retryWithBackoff(page, fn, options = {}) {
            const {
                maxRetries = config.maxRetries,
                baseDelay = 500,
                factor = 2,
                maxDelay = 30000,
                jitterMin = 0.9,
                jitterMax = 1.1,
            } = options;

            let lastError;

            for (let attempt = 0; attempt < maxRetries; attempt++) {
                try {
                    return await fn(attempt);
                } catch (error) {
                    lastError = error;
                    const _delay = calculateBackoffDelay(attempt, {
                        baseDelay,
                        maxDelay,
                        factor,
                        jitterMin,
                        jitterMax,
                    });

                    if (attempt < maxRetries - 1) {
                        await api.wait(1000);
                    }
                }
            }

            throw lastError;
        },

        async smartClick(page, selectorConfig, options = {}) {
            const {
                logger = console,
                context = null,
                fallbacks = [],
                verifySelector = null,
                verifyTimeout = 500,
            } = options;

            let selectors;

            if (context) {
                selectors = this.getXSelectors(context);
            } else if (selectorConfig) {
                selectors = {
                    primary: selectorConfig.primary || selectorConfig,
                    fallbacks: fallbacks,
                };
            } else {
                logger.warn(`[Motor] No context or selectorConfig provided`);
                return { success: false, reason: 'no_context_or_selector' };
            }

            logger.info(`[Motor] Smart click on: ${selectors.primary}`);

            const smartResult = await this.smartSelector(
                page,
                selectors.primary,
                selectors.fallbacks,
                { logger }
            );

            if (!smartResult.element) {
                return { success: false, reason: 'selector_not_found' };
            }

            const stableResult = await this.getStableTarget(page, smartResult.selector, {
                timeout: options.timeout || config.targetTimeout,
                scrollFirst: true,
            });

            if (!stableResult.success) {
                logger.warn(`[Motor] Target not stable, attempting scroll recovery...`);
                await page.evaluate(() => window.scrollBy(0, config.scrollRecoveryAmount));
                await api.wait(1000);

                const retryResult = await this.getStableTarget(page, smartResult.selector, {
                    timeout: 2000,
                    scrollFirst: false,
                });

                if (!retryResult.success) {
                    return { success: false, reason: 'target_not_stable' };
                }
            }

            const box = stableResult.box || smartResult.element.boundingBox();
            const targetX = box.x + box.width / 2;
            const targetY = box.y + box.height / 2;

            const overlap = await this.checkOverlap(page, targetX, targetY);

            if (overlap && overlap !== smartResult.element) {
                logger.warn(`[Motor] Element overlapped, finding alternative...`);

                const uncovered = await this.findUncoveredArea(page, box, { logger });
                if (uncovered.success) {
                    await page.mouse.click(uncovered.x, uncovered.y);
                    return {
                        success: true,
                        x: uncovered.x,
                        y: uncovered.y,
                        selector: smartResult.selector,
                        recovered: true,
                        usedFallback: smartResult.usedFallback,
                    };
                }

                const spiral = await this.spiralSearch(page, targetX, targetY, { logger });
                if (spiral.success) {
                    await page.mouse.click(spiral.x, spiral.y);
                    return {
                        success: true,
                        x: spiral.x,
                        y: spiral.y,
                        selector: smartResult.selector,
                        recovered: true,
                        usedFallback: smartResult.usedFallback,
                    };
                }

                return { success: false, reason: 'overlapped_element' };
            }

            await page.mouse.click(targetX, targetY);

            if (verifySelector) {
                await api.wait(1000);
                try {
                    await page.waitForSelector(verifySelector, { timeout: verifyTimeout });
                    logger.info(`[Motor] Click verified: ${verifySelector}`);
                    return {
                        success: true,
                        x: targetX,
                        y: targetY,
                        selector: smartResult.selector,
                        recovered: false,
                        usedFallback: smartResult.usedFallback,
                        verified: true,
                    };
                } catch {
                    logger.warn(`[Motor] Click not verified: ${verifySelector}`);
                    return {
                        success: true,
                        x: targetX,
                        y: targetY,
                        selector: smartResult.selector,
                        recovered: false,
                        usedFallback: smartResult.usedFallback,
                        verified: false,
                    };
                }
            }

            return {
                success: true,
                x: targetX,
                y: targetY,
                selector: smartResult.selector,
                recovered: false,
                usedFallback: smartResult.usedFallback,
            };
        },
    };
}

export const motorControl = {
    createMotorController,
    defaults: MOTOR_CONFIG,
};

export default motorControl;
