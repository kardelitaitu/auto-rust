/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Random Scrolling Utility
 * Re-implemented to support tasks using the Unified API.
 * @module api/utils/randomScrolling
 */

import { api } from '../index.js';
import { mathUtils } from './math.js';

/**
 * Creates a random scroller function for a page.
 * @param {object} page - Playwright page instance (ignored, uses api context)
 * @returns {function(number): Promise<void>} A function that performs random scrolling for a duration in seconds.
 */
export function createRandomScroller(_page) {
    /**
     * Performs random scrolling for a specified duration.
     * @param {number} durationSeconds - Duration to scroll in seconds.
     */
    return async function randomScrolling(durationSeconds) {
        const startTime = Date.now();
        const endTime = startTime + durationSeconds * 1000;

        while (Date.now() < endTime) {
            // Use api.scroll.read for human-like behavior
            // We do 1 cycle at a time to check for duration limit
            await api.scroll.read(undefined, {
                pauses: 1,
                scrollAmount: mathUtils.randomInRange(300, 700),
                variableSpeed: true,
                backScroll: Math.random() > 0.8,
            });

            // Small extra pause between read cycles
            await api.wait(mathUtils.randomInRange(1000, 3000));
        }
    };
}

export default createRandomScroller;
