/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Trampoline / Warmup System
 * Simulates human behavior before state transitions to establish believable session baseline.
 *
 * @module api/warmup
 */

import { getPage, getCursor } from '../core/context.js';
import { think, delay, randomInRange } from './timing.js';
import { createLogger } from '../core/logger.js';

const logger = createLogger('api/warmup.js');

/**
 * Random mouse movement across viewport.
 * Simulates user "waking up" the mouse before action.
 * @param {number} [duration=1500] - Movement duration in ms
 * @returns {Promise<void>}
 */
export async function randomMouse(duration = 1500) {
    const page = getPage();
    const cursor = getCursor();
    const viewport = page.viewportSize();

    if (!viewport) return;

    const startTime = Date.now();
    const moves = randomInRange(3, 8);

    for (let i = 0; i < moves && Date.now() - startTime < duration; i++) {
        const targetX = randomInRange(0, viewport.width);
        const targetY = randomInRange(0, viewport.height);

        await cursor.move(targetX, targetY);
        await delay(randomInRange(100, 300));
    }
}

/**
 * Fake reading scroll simulation.
 * Scrolls through content as if reading.
 * @param {number} [scrolls=3] - Number of scroll movements
 * @returns {Promise<void>}
 */
export async function fakeRead(scrolls = 3) {
    const page = getPage();

    for (let i = 0; i < scrolls; i++) {
        const scrollAmount = randomInRange(200, 500);
        const direction = Math.random() > 0.3 ? 1 : -1; // Mostly down

        try {
            await page.mouse.wheel(0, scrollAmount * direction);
        } catch (_e) {
            // Fallback if CDP wheel fails
            logger.debug(
                `[Warmup] CDP wheel failed, using window.scrollBy(${scrollAmount * direction})`
            );
            await page.evaluate((dy) => window.scrollBy(0, dy), scrollAmount * direction);
        }
        await delay(randomInRange(500, 1500));
    }
}

/**
 * Random pause to simulate cognitive decision-making.
 * @param {number} [min=2000] - Minimum delay in ms
 * @param {number} [max=5000] - Maximum delay in ms
 * @returns {Promise<void>}
 */
export async function pause(min = 2000, max = 5000) {
    await think(randomInRange(min, max));
}

/**
 * Complete pre-navigation warmup routine.
 * Master execution wrapper - call before navigating to a new URL.
 * @param {string} url - Target URL (for reference, not navigated yet)
 * @param {object} [options]
 * @param {boolean} [options.mouse=true] - Enable random mouse movement
 * @param {boolean} [options.fakeRead=false] - Enable fake reading scroll
 * @param {boolean} [options.pause=true] - Enable decision pause
 * @returns {Promise<void>}
 */
export async function beforeNavigate(url, options = {}) {
    const { mouse = true, fakeRead: shouldFakeRead = false, pause: doPause = true } = options;

    // 1. Random mouse movement (1-2 seconds)
    if (mouse) {
        await randomMouse(randomInRange(1000, 2000));
    }

    // 2. Optional: Fake reading scroll
    if (shouldFakeRead) {
        await fakeRead(randomInRange(2, 5));
    }

    // 3. Random pause - "deciding to click" (2-5 seconds)
    if (doPause) {
        await pause(2000, 5000);
    }

    // Navigation will be performed by the caller (e.g., navigation.js)
}

export default {
    randomMouse,
    fakeRead,
    pause,
    beforeNavigate,
};
