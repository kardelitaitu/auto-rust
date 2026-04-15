/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Scroll Helper - Now uses Unified API
 * Migrated to use api.scroll() for all scrolling operations
 *
 * Usage:
 *   // BEFORE: await scrollWheel(page, 300);
 *   // AFTER:  await scrollWheel(300);
 *
 * @module utils/scroll-helper
 */

import { api } from '../index.js';

/**
 * Wrapper for page.mouse.wheel() with global multiplier
 * @param {number} deltaY - Vertical scroll amount (positive=down, negative=up)
 * @param {object} options - { delay: number }
 */
export async function scrollWheel(deltaY, options = {}) {
    if (options.delay > 0) {
        await api.wait(options.delay);
    }
    await api.scroll(deltaY);
}

/**
 * Scroll down with multiplier
 * @param {number} amount - Pixels to scroll down
 * @param {object} options - { delay: number }
 */
export async function scrollDown(amount, options = {}) {
    if (options.delay > 0) {
        await api.wait(options.delay);
    }
    await api.scroll(amount);
}

/**
 * Scroll up with multiplier
 * @param {number} amount - Pixels to scroll up
 * @param {object} options - { delay: number }
 */
export async function scrollUp(amount, options = {}) {
    if (options.delay > 0) {
        await api.wait(options.delay);
    }
    await api.scroll(-amount);
}

/**
 * Random scroll with multiplier
 * @param {number} min - Minimum scroll amount
 * @param {number} max - Maximum scroll amount
 * @param {object} options - { delay: number }
 */
export async function scrollRandom(min, max, options = {}) {
    if (options.delay > 0) {
        await api.wait(options.delay);
    }
    const { mathUtils } = await import('../utils/math.js');
    const amount = mathUtils.randomInRange(min, max);
    await api.scroll(amount);
}

/**
 * Scroll to top of page
 * @param {object} options - { behavior: 'auto'|'smooth' }
 */
export async function scrollToTop() {
    await api.scroll.toTop();
}

/**
 * Scroll to bottom of page
 * @param {object} options - { behavior: 'auto'|'smooth' }
 */
export async function scrollToBottom() {
    await api.scroll.toBottom();
}

/**
 * Quick scroll helper - handles both positive and negative
 * @param {number} amount - Scroll amount (positive=down, negative=up)
 */
export async function scroll(amount) {
    await api.scroll(amount);
}

/**
 * Get current scroll multiplier
 * Note: Multiplier now managed by API persona
 * @returns {number} Current multiplier
 */
export function getScrollMultiplier() {
    // Now managed by API persona system
    return 1.0;
}

/**
 * Scroll to element (golden view)
 * @param {string} selector - CSS selector
 * @param {object} options - scroll options
 */
export async function scrollToElement(selector, options = {}) {
    await api.scroll.focus(selector, options);
}
