/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Attention Modeling
 * Simulates human attention and focus patterns - gaze, distractions, exit intent.
 * Uses context-aware state for session isolation.
 *
 * @module api/attention
 */

import { getPage, getCursor } from '../core/context.js';
import { getPersona } from './persona.js';
import { think, delay, randomInRange } from './timing.js';
import { mathUtils } from '../utils/math.js';
import {
    getStateDistractionChance,
    setStateDistractionChance,
    getStateAttentionMemory,
    recordStateAttentionMemory,
} from '../core/context-state.js';

/**
 * Set probability of distraction.
 * @param {number} chance - 0.0 to 1.0
 */
export function setDistractionChance(chance) {
    setStateDistractionChance(chance);
}

/**
 * Get current distraction chance.
 * @returns {number}
 */
export function getDistractionChance() {
    return getStateDistractionChance();
}

/**
 * Gaze - move mouse to area and "look" before acting.
 * Simulates human attention: look at area first, then proceed.
 * includes Saccadic Bouncing (fixating on interest points).
 * @param {string} selector - CSS selector to gaze at
 * @param {object} [options]
 * @param {number} [options.duration=1500] - How long to gaze in ms
 * @param {number} [options.saccades=2] - Number of saccadic jumps
 * @returns {Promise<void>}
 */
export async function gaze(selector, options = {}) {
    const page = getPage();
    const cursor = getCursor();
    const persona = getPersona();
    const { duration = randomInRange(1000, 2000), saccades = 2 } = options;

    // Move cursor to element
    const locator = page.locator(selector).first();
    const box = await locator.boundingBox();
    if (!box) return;

    // 1. Initial Saccadic Bouncing (Looking at "Interest Points")
    for (let i = 0; i < saccades; i++) {
        // Pick a random "Interest Point" (corners or edges)
        const offsetX = mathUtils.randomInRange(box.width * 0.1, box.width * 0.9);
        const offsetY = mathUtils.randomInRange(box.height * 0.1, box.height * 0.9);

        await cursor.move(box.x + offsetX, box.y + offsetY);
        await delay(randomInRange(50, 150)); // Fixation pause
    }

    // 2. Final Gaze Fixation
    const targetX = box.x + box.width / 2;
    const targetY = box.y + box.height / 2;
    await cursor.move(targetX, targetY);

    // Gaze duration - "looking at" the element
    const adjustedDuration = duration / persona.speed;
    await think(adjustedDuration);
}

/**
 * Attention - gaze at element, then optionally perform action.
 * This is the main entry point that combines gaze with optional action.
 * @param {string} selector - CSS selector
 * @param {object} [options]
 * @param {number} [options.duration=1500] - Gaze duration
 * @param {boolean} [options.act=true] - Whether to perform action after gaze
 * @returns {Promise<void>}
 */
export async function attention(selector, options = {}) {
    const { duration = randomInRange(1000, 2000) } = options;

    // Record in memory for future gazes
    recordStateAttentionMemory(selector);

    // First gaze at the area
    await gaze(selector, { duration });

    // Optionally perform action (caller will do this)
    // This is here for API completeness
}

/**
 * Move to random element on page - simulates distraction.
 * @param {string[]} [selectors] - Array of possible selectors to look at
 * @returns {Promise<void>}
 */
export async function distraction(selectors = []) {
    const page = getPage();
    const cursor = getCursor();
    const viewport = page.viewportSize();

    if (!viewport) return;

    if (selectors.length > 0) {
        // Pick random selector from list
        const selector = selectors[Math.floor(Math.random() * selectors.length)];
        try {
            const locator = page.locator(selector).first();
            const box = await locator.boundingBox();
            if (box) {
                await cursor.move(box.x + box.width / 2, box.y + box.height / 2);
                await think(randomInRange(500, 1500));
                return;
            }
        } catch {
            // Selector not found, continue to random position
        }
    }

    // Fallback: random position on page
    const randomX = randomInRange(0, viewport.width);
    const randomY = randomInRange(0, viewport.height);
    await cursor.move(randomX, randomY);
    await think(randomInRange(500, 1500));
}

/**
 * Exit intent - move to navigation area before leaving.
 * Simulates user moving to menu/top bar before closing or navigating away.
 * @param {object} [options]
 * @param {boolean} [options.moveToTop=true] - Move cursor to top of page
 * @param {boolean} [options.pause=true] - Pause at top
 * @returns {Promise<void>}
 */
export async function beforeLeave(options = {}) {
    const page = getPage();
    const cursor = getCursor();
    const { moveToTop = true, pause = true } = options;

    if (moveToTop) {
        // Move to top of page (navigation area)
        const viewport = page.viewportSize();
        if (viewport) {
            await cursor.move(viewport.width / 2, 50); // Top center
        }
    }

    // Pause to simulate "deciding to leave"
    if (pause) {
        await think(randomInRange(1000, 3000));
    }
}

/**
 * Focus shift - click something else before main target.
 * Simulates human behavior of clicking nearby element first.
 * @param {string} mainSelector - Main target selector
 * @param {string} [shiftSelector] - Optional shift target (nearby element)
 * @returns {Promise<void>}
 */
export async function focusShift(mainSelector, shiftSelector = null) {
    const page = getPage();
    const cursor = getCursor();

    // If shiftSelector provided, click it first
    if (shiftSelector) {
        try {
            await page.locator(shiftSelector).first().click();
            await think(randomInRange(300, 800));
        } catch {
            // Shift click failed, continue
        }
    } else {
        // Click near main element (shift focus)
        const locator = page.locator(mainSelector).first();
        const box = await locator.boundingBox();
        if (box) {
            // Click slightly offset from main target
            const offsetX = box.width * 0.3 * (Math.random() > 0.5 ? 1 : -1);
            const targetX = box.x + box.width / 2 + offsetX;
            const targetY = box.y + box.height / 2;

            await cursor.move(targetX, targetY);
            await delay(100);
            await page.mouse.click(targetX, targetY);
            await think(randomInRange(200, 600));
        }
    }
}

/**
 * Calculate "Visual Weight" of the current viewport.
 * Counts interactive elements and potential distractions.
 * @returns {Promise<number>} - Weighted complexity score
 */
export async function getVisualWeight() {
    const page = getPage();
    /* c8 ignore next */
    return await page.evaluate(() => {
        const interactives = document.querySelectorAll('button, a, input, select, [role="button"]');
        const stickies = document.querySelectorAll('[style*="fixed"], [style*="sticky"]');
        const animations = document.querySelectorAll(
            'video, canvas, [class*="anim"], [class*="spin"]'
        );

        return interactives.length + stickies.length * 2 + animations.length * 3;
    });
}

/**
 * Calculate visual weight from element counts.
 * Exported for testing.
 * @param {number} interactivesCount - Number of interactive elements
 * @param {number} stickiesCount - Number of sticky/fixed elements
 * @param {number} animationsCount - Number of animated elements
 * @returns {number} - Weighted complexity score
 */
export function calculateVisualWeight(interactivesCount, stickiesCount, animationsCount) {
    return interactivesCount + stickiesCount * 2 + animationsCount * 3;
}

/**
 * Randomly decide whether to get distracted.
 * Checks persona and distraction chance, scaled by Visual Weight.
 * @param {string[]} [selectors] - Optional selectors to potentially look at
 * @returns {Promise<boolean>} - True if distraction occurred
 */
export async function maybeDistract(selectors = []) {
    const persona = getPersona();
    const weight = await getVisualWeight();
    const distractionChance = getStateDistractionChance();
    const attentionMemory = getStateAttentionMemory();

    // Scale chance by visual weight (more noise = more likely to look away)
    // weight 0-50 = 1x, 50-200 = 1.5x, 200+ = 2x
    const weightMultiplier = Math.min(2, 1 + weight / 200);
    const chance = (distractionChance + (persona.idleChance || 0)) * weightMultiplier;

    if (Math.random() < chance) {
        // Use memory to potentially look back at something
        const useMemory = Math.random() < 0.4 && attentionMemory.length > 0;
        const targetSelectors = useMemory ? [...selectors, ...attentionMemory] : selectors;

        await distraction(targetSelectors);
        return true;
    }

    return false;
}
