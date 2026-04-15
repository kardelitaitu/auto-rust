/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Idle Simulation
 * Simulates human presence during idle periods - mouse wiggle, occasional scrolling.
 * Uses context-aware state for session isolation.
 *
 * @module api/idle
 */

import { getPage, getCursor, setSessionInterval, clearSessionInterval } from '../core/context.js';
import { getStateIdle, setStateIdle } from '../core/context-state.js';
import { randomInRange } from './timing.js';
import { createLogger } from '../core/logger.js';

const logger = createLogger('api/idle.js');

const IDLE_INTERVAL_NAME = 'idle_simulation';

/**
 * Start idle ghosting - random micro-movements when idle.
 * @param {object} [options]
 * @param {boolean} [options.wiggle=true] - Cursor micro-movements
 * @param {boolean} [options.scroll=true] - Occasional scrolling
 * @param {number} [options.frequency=3000] - Movement interval in ms
 * @param {number} [options.magnitude=5] - Movement range in pixels
 * @returns {void}
 */
export function start(options = {}) {
    const { wiggle = true, scroll = true, frequency = 3000, magnitude = 5 } = options;

    const state = getStateIdle();
    if (state.isRunning) {
        return; // Already running
    }

    const page = getPage();
    const cursor = getCursor();

    const intervalId = setSessionInterval(
        IDLE_INTERVAL_NAME,
        async () => {
            try {
                if (wiggle) {
                    // Mouse wiggle
                    const currentPos = cursor.previousPos || { x: 0, y: 0 };
                    const deltaX = randomInRange(-magnitude, magnitude);
                    const deltaY = randomInRange(-magnitude, magnitude);

                    await cursor.move(
                        Math.max(0, currentPos.x + deltaX),
                        Math.max(0, currentPos.y + deltaY)
                    );
                }

                if (scroll && Math.random() > 0.7) {
                    // Occasional micro-scroll (30% chance)
                    const scrollAmount = randomInRange(-50, 50);
                    try {
                        await page.mouse.wheel(0, scrollAmount);
                    } catch (_wheelError) {
                        // Fallback to JS scroll if CDP wheel fails
                        logger.debug(
                            `[Idle] CDP wheel failed, using window.scrollBy(${scrollAmount})`
                        );
                        await page.evaluate((dy) => window.scrollBy(0, dy), scrollAmount);
                    }
                }
            } catch {
                // Ignore errors during idle
            }
        },
        frequency
    );

    setStateIdle({
        isRunning: true,
        fidgetInterval: intervalId, // Store ID in state for visibility
    });
}

/**
 * Background heartbeat for long-running idle periods.
 */
export function startHeartbeat(options = {}) {
    return start({ frequency: 30000, magnitude: 3, ...options });
}

/**
 * Stop idle ghosting.
 * @returns {void}
 */
export function stop() {
    clearSessionInterval(IDLE_INTERVAL_NAME);
    setStateIdle({ isRunning: false, fidgetInterval: null });
}

/**
 * Check if idle simulation is running.
 * @returns {boolean}
 */
export function isRunning() {
    return getStateIdle().isRunning;
}

/**
 * Perform a single idle wiggle.
 * @param {number} [magnitude=5] - Movement range in pixels
 * @returns {Promise<void>}
 */
export async function wiggle(magnitude = 5) {
    const cursor = getCursor();
    const currentPos = cursor.previousPos || { x: 0, y: 0 };

    const deltaX = randomInRange(-magnitude, magnitude);
    const deltaY = randomInRange(-magnitude, magnitude);

    await cursor.move(Math.max(0, currentPos.x + deltaX), Math.max(0, currentPos.y + deltaY));
}

/**
 * Perform a single idle scroll.
 * @param {number} [distance=30] - Scroll distance
 * @returns {Promise<void>}
 */
export async function idleScroll(distance = 30) {
    const page = getPage();
    const direction = Math.random() > 0.5 ? 1 : -1;
    await page.mouse.wheel(0, distance * direction);
}
