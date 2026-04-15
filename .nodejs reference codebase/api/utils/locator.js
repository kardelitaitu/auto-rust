/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Locator Utilities
 * Helpers for handling both string selectors and Playwright Locators.
 * @module api/utils/locator
 */

import { getPage } from '../core/context.js';

/**
 * Normalizes an input into a Playwright Locator.
 * @param {string|import('playwright').Locator} selectorOrLocator - String selector or Locator instance
 * @returns {import('playwright').Locator}
 */
export function getLocator(selectorOrLocator) {
    if (!selectorOrLocator) {
        throw new Error('Selector or Locator is required');
    }

    if (typeof selectorOrLocator === 'string') {
        return getPage().locator(selectorOrLocator);
    }

    return selectorOrLocator;
}

/**
 * Returns a string representation of the selector or locator for logging.
 * @param {string|import('playwright').Locator} selectorOrLocator
 * @returns {string}
 */
export function stringify(selectorOrLocator) {
    if (typeof selectorOrLocator === 'string') {
        return selectorOrLocator;
    }
    // Locators don't have a simple way to get their original selector string
    // but we can try to use their toString or a placeholder.
    return selectorOrLocator.toString() || '[Locator]';
}
