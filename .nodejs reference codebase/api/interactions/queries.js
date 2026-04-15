/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Read-Only DOM Queries
 * Simple extraction functions with zero humanization or entropy.
 *
 * @module api/queries
 */

import { getPage, isSessionActive } from '../core/context.js';
import { getLocator } from '../utils/locator.js';

/**
 * Internal helper to run a query only if the session is alive.
 * Returns defaultValue if session is dead.
 */
async function safeQuery(fn, defaultValue = null) {
    if (!isSessionActive()) return defaultValue;
    try {
        return await fn();
    } catch (_e) {
        return defaultValue;
    }
}

/**
 * Extract innerText from an element.
 * @param {string|import('playwright').Locator} selector - CSS selector or Locator
 * @returns {Promise<string>}
 */
export async function text(selector) {
    return safeQuery(async () => {
        return await getLocator(selector).first().innerText();
    }, '');
}

/**
 * Extract a DOM attribute value.
 * @param {string|import('playwright').Locator} selector - CSS selector or Locator
 * @param {string} name - Attribute name
 * @returns {Promise<string|null>}
 */
export async function attr(selector, name) {
    return safeQuery(async () => {
        return await getLocator(selector).first().getAttribute(name);
    }, null);
}

/**
 * Check if an element is visible in the layout.
 * @param {string|import('playwright').Locator} selector - CSS selector or Locator
 * @returns {Promise<boolean>}
 */
export async function visible(selector) {
    return safeQuery(async () => {
        return await getLocator(selector).first().isVisible();
    }, false);
}

/**
 * Count matching elements.
 * @param {string|import('playwright').Locator} selector - CSS selector or Locator
 * @returns {Promise<number>}
 */
export async function count(selector) {
    return safeQuery(async () => {
        return await getLocator(selector).count();
    }, 0);
}

/**
 * Check if at least one matching element exists in the DOM.
 * @param {string|import('playwright').Locator} selector - CSS selector or Locator
 * @returns {Promise<boolean>}
 */
export async function exists(selector) {
    return (await count(selector)) > 0;
}

export async function currentUrl() {
    return safeQuery(async () => {
        const page = getPage();
        return page.url();
    }, '');
}
