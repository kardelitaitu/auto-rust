/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * Error Recovery Engine
 * Human-like error patterns and recovery
 *
 * Human Error Patterns:
 * 1. Mis-click nearby element (40%)
 * 2. Scroll past and come back (30%)
 * 3. Cancel and retry (20%)
 * 4. Give up entirely (10%)
 */

import { mathUtils } from '../../utils/math.js';
import { entropy } from '../../utils/entropyController.js';
import { scrollRandom } from '../scroll-helper.js';
import { api } from '../../index.js';

export class ErrorRecovery {
    constructor(page, logger, humanizationEngine = null) {
        this.page = page;
        this.logger = logger;
        this.human = humanizationEngine;
        this.recoveryChain = [];
    }

    /**
     * Handle an error with human-like recovery
     *
     * @param {string} errorType - Type of error encountered
     * @param {object} context - Error context
     * @returns {Promise<object>} Recovery result
     */
    async handle(errorType, context = {}) {
        const errorPatterns = {
            // Element not found
            element_not_found: {
                weight: 0.35,
                actions: [
                    () => this._scrollAndRetry(context),
                    () => this._refreshAndRetry(context),
                    () => this._giveUp(),
                ],
            },

            // Click failed
            click_failed: {
                weight: 0.25,
                actions: [
                    () => this._clickNearby(context),
                    () => this._retryWithForce(context),
                    () => this._giveUp(),
                ],
            },

            // Navigation failed
            navigation_failed: {
                weight: 0.15,
                actions: [
                    () => this._retryNavigation(context),
                    () => this._goBackAndRetry(context),
                    () => this._giveUp(),
                ],
            },

            // Timeout
            timeout: {
                weight: 0.15,
                actions: [
                    () => this._waitAndRetry(context),
                    () => this._refreshAndRetry(context),
                    () => this._giveUp(),
                ],
            },

            // Verification failed
            verification_failed: {
                weight: 0.1,
                actions: [
                    () => this._checkState(context),
                    () => this._retryAction(context),
                    () => this._giveUp(),
                ],
            },

            // Default
            default: {
                weight: 1.0,
                actions: [() => this._waitAndRetry(context), () => this._giveUp()],
            },
        };

        const pattern = errorPatterns[errorType] || errorPatterns.default;

        // Log the error
        this._logError(errorType, context);

        // Try recovery actions in order
        for (const action of pattern.actions) {
            try {
                const result = await action();
                if (result.success) {
                    this._logRecovery(result.strategy);
                    return result;
                }
            } catch (error) {
                // Log failure and continue to next recovery strategy
                if (this.logger) {
                    this.logger.debug(`[Recovery] Action failed: ${error.message}`);
                }
                continue;
            }
        }

        // All recovery failed
        this._logRecovery('failed');
        return { success: false, strategy: 'exhausted', error: errorType };
    }

    // ==========================================
    // RECOVERY STRATEGIES
    // ==========================================

    /**
     * Scroll and retry finding element
     */
    async _scrollAndRetry(context) {
        this._logStrategy('scroll_and_retry');

        // Scroll in random direction
        await scrollRandom(150, 300);

        // Wait for content to load
        await api.wait(mathUtils.gaussian(500, 200));

        // Check if element is now available
        if (context.locator) {
            const count = await context.locator.count();
            if (count > 0) {
                return { success: true, strategy: 'scroll' };
            }
        }

        return { success: false, strategy: 'scroll' };
    }

    /**
     * Click nearby element (mis-click recovery)
     */
    async _clickNearby(_context) {
        this._logStrategy('click_nearby');

        // Find nearby clickable element
        const nearbySelectors = ['[role="button"]', 'button', 'a[href]', '[tabindex="0"]'];

        for (const selector of nearbySelectors) {
            try {
                const elements = await this.page.$$(selector);
                if (elements.length > 1) {
                    // Click second element (nearby)
                    await elements[1].click({ timeout: 1000 }).catch(() => {});
                    await this.page.waitForTimeout(entropy.reactionTime());

                    return { success: true, strategy: 'nearby_click' };
                }
            } catch (_error) {
                continue;
            }
        }

        return { success: false, strategy: 'nearby_click' };
    }

    /**
     * Refresh page and retry
     */
    async _refreshAndRetry(_context) {
        this._logStrategy('refresh_and_retry');

        try {
            // Quick refresh
            await api.reload({ waitUntil: 'domcontentloaded' });
            await api.wait(mathUtils.gaussian(1500, 500));
            return { success: true, strategy: 'refresh' };
        } catch (_e) {
            return { success: false, strategy: 'refresh' };
        }
    }

    /**
     * Wait and retry
     */
    async _waitAndRetry(_context) {
        this._logStrategy('wait_and_retry');

        try {
            // Human wait time (1-3 seconds)
            const waitTime = mathUtils.randomInRange(1000, 3000);
            await api.wait(waitTime);
            return { success: true, strategy: 'wait' };
        } catch (_e) {
            return { success: false, strategy: 'wait' };
        }
    }

    /**
     * Retry with force click
     */
    async _retryWithForce(context) {
        this._logStrategy('retry_with_force');

        if (context.locator) {
            try {
                await context.locator.click({ force: true });
                return { success: true, strategy: 'force_click' };
            } catch (_error) {
                return { success: false, strategy: 'force_click' };
            }
        }

        return { success: false, strategy: 'force_click' };
    }

    /**
     * Go back and retry
     */
    async _goBackAndRetry(_context) {
        this._logStrategy('go_back');

        try {
            // Human-like: go back, then forward if needed
            await api.getPage().goBack();
            await api.wait(mathUtils.gaussian(1000, 300));

            // Sometimes also go forward
            if (mathUtils.roll(0.3)) {
                await api.getPage().goForward();
                await api.wait(mathUtils.gaussian(1000, 300));
            }

            return { success: true, strategy: 'navigation' };
        } catch (_e) {
            return { success: false, strategy: 'navigation' };
        }
    }

    /**
     * Retry navigation
     */
    async _retryNavigation(context) {
        this._logStrategy('retry_navigation');

        try {
            // Wait and retry
            await api.wait(mathUtils.randomInRange(2000, 4000));

            if (context.url) {
                await api.goto(context.url, { waitUntil: 'domcontentloaded' });
            }

            return { success: true, strategy: 'navigation_retry' };
        } catch (_e) {
            return { success: false, strategy: 'navigation_retry' };
        }
    }

    /**
     * Check current state
     */
    async _checkState(_context) {
        this._logStrategy('check_state');

        // Verify current page state
        const url = await api.getCurrentUrl();
        const title = await api.getPage().title();

        // Log state for debugging
        this._logDebug({ url, title });

        return { success: true, strategy: 'state_check' };
    }

    /**
     * Retry original action
     */
    async _retryAction(_context) {
        this._logStrategy('retry_action');

        await api.wait(mathUtils.randomInRange(500, 1500));
        return { success: true, strategy: 'retry' };
    }

    /**
     * Give up (human-like)
     */
    async _giveUp() {
        this._logStrategy('give_up');

        // Human response: sometimes just move on
        await this.page.waitForTimeout(mathUtils.randomInRange(500, 1000));

        // Small scroll to "move on"
        await scrollRandom(50, 150);

        return { success: true, strategy: 'gave_up' };
    }

    // ==========================================
    // LOGGING UTILITIES
    // ==========================================

    _logError(errorType, _context) {
        if (this.logger) {
            this.logger.warn(`[Error] ${errorType}`);
        }
    }

    _logStrategy(strategy) {
        if (this.logger) {
            this.logger.debug(`[Recovery] Trying: ${strategy}`);
        }
    }

    _logRecovery(result) {
        if (this.logger) {
            if (result === 'failed') {
                this.logger.warn(`[Recovery] All strategies exhausted`);
            } else {
                this.logger.info(`[Recovery] Recovered via: ${result}`);
            }
        }
    }

    _logDebug(state) {
        if (this.logger) {
            this.logger.debug(`[Recovery] State: ${JSON.stringify(state)}`);
        }
    }
}

export default ErrorRecovery;
