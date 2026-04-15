/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { api } from '../../index.js';
/**
 * Human Scroll Engine
 * Human-like scrolling patterns with bursts, pauses, and micro-adjustments
 *
 * Human Scroll Patterns:
 * 1. Quick burst (100-300px)
 * 2. Pause to "look" (200-800ms)
 * 3. Sometimes scroll back slightly (20-50px)
 * 4. Repeat 2-5 times
 * 5. Longer pause (1-3s)
 */

import { mathUtils } from '../../utils/math.js';
import { entropy as _entropy } from '../../utils/entropyController.js';
import { scrollRandom } from '../scroll-helper.js';

export class HumanScroll {
    constructor(page, logger) {
        this.page = page;
        this.logger = logger;
        this.agent = null;
    }

    setAgent(agent) {
        this.agent = agent;
    }

    /**
     * Main scroll method - use this instead of page.mouse.wheel()
     *
     * @param {string} direction - 'up', 'down', or 'random'
     * @param {string} intensity - 'light', 'normal', 'heavy'
     */
    async execute(direction = 'down', intensity = 'normal') {
        const intensityConfig = {
            light: { bursts: 2, pxMin: 50, pxMax: 150 },
            normal: { bursts: 3, pxMin: 100, pxMax: 300 },
            heavy: { bursts: 5, pxMax: 500 },
        };

        const config = intensityConfig[intensity] || intensityConfig.normal;
        const burstCount = config.bursts || mathUtils.randomInRange(2, 4);

        // Determine scroll direction and amount
        let scrollAmount;
        switch (direction) {
            case 'up':
                scrollAmount = -mathUtils.randomInRange(config.pxMin || 100, config.pxMax || 300);
                break;
            case 'down':
                scrollAmount = mathUtils.randomInRange(config.pxMin || 100, config.pxMax || 300);
                break;
            case 'random':
            default:
                scrollAmount =
                    Math.random() > 0.5
                        ? mathUtils.randomInRange(100, 300)
                        : -mathUtils.randomInRange(50, 150);
                break;
        }

        // Execute scroll burst pattern
        for (let i = 0; i < burstCount; i++) {
            // Random variation in scroll amount
            const variation = (Math.random() - 0.5) * 0.2; // ±10%
            const adjustedScroll = Math.round(scrollAmount * (1 + variation));

            await scrollRandom(adjustedScroll, adjustedScroll);

            // Pause between bursts (looking time)
            if (i < burstCount - 1) {
                const _pauseTime = mathUtils.gaussian(400, 200);
                await api.wait(1000);

                // Occasionally scroll back slightly (re-reading)
                if (mathUtils.roll(0.2) && i > 0) {
                    await scrollRandom(20, 50);
                    await api.wait(1000);
                }
            }
        }

        // Final pause (processing time)
        const _endPause =
            intensity === 'light' ? mathUtils.gaussian(800, 300) : mathUtils.gaussian(1500, 500);
        await api.wait(1000);

        if (this.agent) {
            this.agent.log(
                `[Scroll] ${direction} (${burstCount} bursts, ${Math.abs(scrollAmount)}px)`
            );
        }
    }

    /**
     * Scroll to element with human-like approach
     */
    async toElement(locator, _context = 'view') {
        try {
            const element = await locator.first();
            if (!element) return;

            const box = await element.boundingBox();
            if (!box) return;

            // Calculate distance to center
            const viewportHeight = await this.page.evaluate(() => window.innerHeight);
            const targetY = box.y + box.height / 2;
            const centerY = viewportHeight / 2;
            const distance = targetY - centerY;

            if (Math.abs(distance) < 100) {
                // Already close, just micro-adjust
                await scrollRandom(Math.abs(distance * 0.5), Math.abs(distance * 0.5));
            } else {
                // Human approach: quick-jump → slow-approach
                const approachCount = 2;

                for (let i = 0; i < approachCount; i++) {
                    const remaining = distance * ((approachCount - i) / approachCount);
                    const scrollAmount = remaining * 0.6; // Overshoot slightly

                    await scrollRandom(Math.abs(scrollAmount), Math.abs(scrollAmount));
                    await api.wait(1000);
                }

                // Fine-tuning
                await scrollRandom(Math.abs(distance * 0.1), Math.abs(distance * 0.1));
            }

            // Pause to "look" at element
            await api.wait(1000);
        } catch (_e) {
            // Fallback to direct scroll
            await scrollRandom(200, 200);
        }
    }

    /**
     * Micro-adjustments during "reading"
     */
    async microAdjustments() {
        const adjustments = mathUtils.randomInRange(2, 4);

        for (let i = 0; i < adjustments; i++) {
            // Tiny random movements
            await scrollRandom(-30, 30);
            await api.wait(1000);
        }
    }

    /**
     * Quick scroll to "check what's new"
     */
    async quickCheck() {
        await this.execute('down', 'light');
    }

    /**
     * Deep scroll (exploring feed)
     */
    async deepScroll() {
        const sessions = mathUtils.randomInRange(3, 5);

        for (let i = 0; i < sessions; i++) {
            await this.execute('down', 'normal');

            // Occasionally go back up slightly
            if (mathUtils.roll(0.3)) {
                await scrollRandom(50, 100);
                await api.wait(1000);
            }
        }
    }

    /**
     * Scroll to top (refresh)
     */
    async scrollToTop() {
        // Quick jumps up
        for (let i = 0; i < 3; i++) {
            await scrollRandom(500, 500);
            await api.wait(1000);
        }

        // Fine adjustment
        await scrollRandom(100, 100);
        await api.wait(1000);
    }
}

export default HumanScroll;
