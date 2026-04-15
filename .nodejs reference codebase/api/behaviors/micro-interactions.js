/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * Micro-Interactions Module
 * Simulates human fidgeting and subconscious behaviors:
 * - Text highlighting
 * - Random right-click
 * - Logo clicks
 * - Whitespace clicks
 *
 * @module utils/micro-interactions
 */

import { api } from '../index.js';
import { setSessionInterval, clearSessionInterval } from '../core/context.js';

const MICRO_CONFIG = {
    highlightChance: 0.03,
    rightClickChance: 0.02,
    logoClickChance: 0.05,
    whitespaceClickChance: 0.1,
    fidgetChance: 0.08,
    fidgetInterval: { min: 10000, max: 30000 },
};

function randomInRange(min, max) {
    return Math.floor(Math.random() * (max - min + 1)) + min;
}

/**
 * Creates a micro-interaction handler for human-like behaviors
 * @param {object} options - Configuration options
 * @returns {object} Handler object
 */
function createMicroInteractionHandler(options = {}) {
    const config = { ...MICRO_CONFIG, ...options };
    let fidgetInterval = null;
    let isRunning = false;

    return {
        config,

        async textHighlight(page, options = {}) {
            const { logger = console, selector = 'article [data-testid="tweetText"]' } = options;

            try {
                const textEl = await page.$(selector);
                if (!textEl) {
                    return { success: false, reason: 'no_element' };
                }

                const box = await textEl.boundingBox();
                if (!box) {
                    return { success: false, reason: 'no_box' };
                }

                const startX = box.x + 10;
                const startY = box.y + box.height / 2;
                const distance = Math.min(50, box.width);

                logger.info(`[Micro] Text highlighting: ${distance}px`);

                await page.mouse.move(startX, startY);
                await page.mouse.down();
                await page.mouse.move(startX + distance, startY, { steps: 5 });
                await api.wait(300);
                await page.mouse.up();
                await api.wait(200);
                await page.mouse.click(startX - 20, startY);

                return { success: true, type: 'highlight', distance };
            } catch (error) {
                logger.error(`[Micro] Highlight error: ${error.message}`);
                return { success: false, reason: error.message };
            }
        },

        async randomRightClick(page, options = {}) {
            const { logger = console, x = null, y = null } = options;

            try {
                const viewport = page.viewportSize() || { width: 1280, height: 720 };
                const targetX = x !== null ? x : Math.random() * viewport.width * 0.8;
                const targetY = y !== null ? y : Math.random() * viewport.height * 0.8;

                logger.info(
                    `[Micro] Random right-click at (${Math.round(targetX)}, ${Math.round(targetY)})`
                );

                await page.mouse.move(targetX, targetY);
                await page.mouse.click(targetX, targetY, { button: 'right' });
                await api.wait(1000);

                await page.mouse.click(targetX - 50, targetY, { button: 'left' });

                return { success: true, type: 'right_click', x: targetX, y: targetY };
            } catch (error) {
                logger.error(`[Micro] Right-click error: ${error.message}`);
                return { success: false, reason: error.message };
            }
        },

        async logoClick(page, options = {}) {
            const { logger = console } = options;

            try {
                const logoSelector =
                    '[data-testid="appleLogo"], [data-testid="logo"], [aria-label="Home"]';
                const logo = await page.$(logoSelector);

                if (!logo) {
                    return { success: false, reason: 'no_logo' };
                }

                await logo.click();
                await api.wait(2000);

                logger.info(`[Micro] Logo click (refresh)`);

                return { success: true, type: 'logo_click' };
            } catch (error) {
                logger.error(`[Micro] Logo click error: ${error.message}`);
                return { success: false, reason: error.message };
            }
        },

        async whitespaceClick(page, options = {}) {
            const { logger = console } = options;

            try {
                const viewport = page.viewportSize() || { width: 1280, height: 720 };
                const x = Math.random() * viewport.width * 0.3;
                const y = Math.random() * viewport.height * 0.3;

                logger.info(`[Micro] Whitespace click at (${Math.round(x)}, ${Math.round(y)})`);

                await page.mouse.move(x, y);
                await page.mouse.click(x, y);

                return { success: true, type: 'whitespace_click', x, y };
            } catch (error) {
                logger.error(`[Micro] Whitespace click error: ${error.message}`);
                return { success: false, reason: error.message };
            }
        },

        async fidget(page, options = {}) {
            const { logger = console } = options;

            if (isRunning) {
                return { success: false, reason: 'already_running' };
            }

            isRunning = true;
            const actions = [];
            const roll = Math.random();

            try {
                if (roll < 0.3) {
                    const result = await this.whitespaceClick(page, { logger });
                    actions.push(result);
                } else if (roll < 0.5) {
                    const result = await this.randomRightClick(page, { logger });
                    actions.push(result);
                } else if (roll < 0.7) {
                    const result = await this.logoClick(page, { logger });
                    actions.push(result);
                } else {
                    const dx = Math.random() * 100 - 50;
                    const dy = Math.random() * 50 - 25;
                    await page.mouse.move(dx, dy, { steps: 3 });
                    actions.push({ success: true, type: 'micro_move', dx, dy });
                }

                logger.info(`[Micro] Fidget action completed`);
                return { success: true, actions };
            } finally {
                isRunning = false;
            }
        },

        shouldFidget() {
            return Math.random() < config.fidgetChance;
        },

        getFidgetInterval() {
            return randomInRange(config.fidgetInterval.min, config.fidgetInterval.max);
        },

        startFidgetLoop(page, options = {}) {
            const { logger = console } = options;

            const doFidget = async () => {
                if (this.shouldFidget()) {
                    await this.fidget(page, { logger });
                }
            };

            fidgetInterval = setSessionInterval('fidget_loop', doFidget, this.getFidgetInterval());
            logger.info(`[Micro] Fidget loop started`);

            return fidgetInterval;
        },

        stopFidgetLoop() {
            clearSessionInterval('fidget_loop');
            const wasRunning = !!fidgetInterval;
            fidgetInterval = null;
            return wasRunning;
        },

        async executeMicroInteraction(page, options = {}) {
            const { logger = console } = options;

            const roll = Math.random();
            const actionRoll = roll;

            if (actionRoll < config.highlightChance) {
                return await this.textHighlight(page, { logger });
            } else if (actionRoll < config.highlightChance + config.rightClickChance) {
                return await this.randomRightClick(page, { logger });
            } else if (
                actionRoll <
                config.highlightChance + config.rightClickChance + config.logoClickChance
            ) {
                return await this.logoClick(page, { logger });
            } else if (
                actionRoll <
                config.highlightChance +
                    config.rightClickChance +
                    config.logoClickChance +
                    config.whitespaceClickChance
            ) {
                return await this.whitespaceClick(page, { logger });
            }

            return { success: false, reason: 'no_action' };
        },
    };
}

export const microInteractions = {
    createMicroInteractionHandler,
    defaults: MICRO_CONFIG,
};

export default microInteractions;
