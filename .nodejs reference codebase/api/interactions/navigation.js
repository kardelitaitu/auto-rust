/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Page Navigation (State Transitions)
 *
 * @module api/navigation
 */

import { getPage } from '../core/context.js';
import { setPreviousUrl } from '../core/context-state.js';
import { getAutoBanners } from '../core/context-state.js';
import { beforeNavigate, randomMouse, fakeRead, pause } from '../behaviors/warmup.js';
import { delay, randomInRange } from '../behaviors/timing.js';
import { getPersona } from '../behaviors/persona.js';
import { createLogger } from '../core/logger.js';
import { getPluginManager } from '../core/plugins/index.js';
import { handleBanners } from './banners.js';

const logger = createLogger('api/navigation.js');

export { beforeNavigate, randomMouse, fakeRead, pause };

/**
 * Navigate to a URL.
 * Automatically triggers warmup.beforeNavigate first.
 * @param {string} url - Target URL
 * @param {object} [options]
 * @param {string} [options.waitUntil='domcontentloaded'] - Load event to wait for
 * @param {number} [options.timeout=30000] - Navigation timeout
 * @param {string} [options.resolveOnSelector] - Resolve navigation once this selector is visible
 * @param {boolean} [options.warmup=true] - Enable pre-navigation warmup
 * @param {boolean} [options.warmupMouse=true] - Warmup: random mouse
 * @param {boolean} [options.warmupFakeRead=false] - Warmup: fake reading
 * @param {boolean} [options.warmupPause=true] - Warmup: decision pause
 * @param {boolean} [options.autoBanners=true] - Auto-handle cookie banners after load
 * @returns {Promise<void>}
 */
export async function goto(url, options = {}) {
    const page = getPage();
    setPreviousUrl(page.url());

    const config = getPersona();

    const {
        waitUntil = 'domcontentloaded',
        timeout = config.timeouts?.navigation || 30000,
        resolveOnSelector = null,
        warmup = true,
        warmupMouse = true,
        warmupFakeRead = false,
        warmupPause = true,
        autoBanners = getAutoBanners(),
    } = options;

    // Auto-warmup before navigation
    if (warmup) {
        await beforeNavigate(url, {
            mouse: warmupMouse,
            fakeRead: warmupFakeRead,
            pause: warmupPause,
        });
    }

    // Navigate to URL
    const gotoOptions = { waitUntil, timeout };
    if (options.referer) {
        gotoOptions.referer = options.referer;
    }
    const navPromise = page.goto(url, gotoOptions);

    if (resolveOnSelector) {
        const selectorPromise = page
            .waitForSelector(resolveOnSelector, { state: 'visible', timeout })
            .catch(() => null);
        // Race the full navigation against the specific selector availability
        await Promise.race([navPromise, selectorPromise]);
        logger.debug(`[Navigation] Quick-resolved via selector: ${resolveOnSelector}`);
    } else {
        await navPromise;
    }

    // Post-navigation: initial scroll to center (human signature)
    await delay(randomInRange(500, 1500));
    try {
        if (Math.random() > 0.3) {
            await page.mouse.wheel(0, randomInRange(100, 300));
        }
    } catch (_e) {
        // Fallback
        await page.evaluate(() => window.scrollBy(0, 100)).catch(() => {});
    }

    // Auto-handle cookie banners
    if (autoBanners) {
        await handleBanners().catch(() => {});
    }

    // Evaluate dynamic plugins
    try {
        getPluginManager().evaluateUrl(page.url());
    } catch (_e) {
        /* ignore */
    }
}

export async function setExtraHTTPHeaders(headers = {}) {
    const page = getPage();
    await page.setExtraHTTPHeaders(headers);
}

/**
 * Reload the current page.
 * @param {object} [options] - Playwright reload options
 * @returns {Promise<void>}
 */
export async function reload(options = {}) {
    const page = getPage();
    await page.reload(options);
}

/**
 * Go back in browser history.
 * page.goBack() can hang or return null (no history / SPA state change).
 * This helper makes the call bounded and returns whether a navigation response occurred.
 * @param {object} [options]
 * @param {number} [options.timeout=2500] - Max time to wait for goBack
 * @param {string} [options.waitUntil='domcontentloaded'] - Navigation event to wait for
 * @returns {Promise<boolean>} True if Playwright returned a navigation response, else false
 */
export async function back(options = {}) {
    const page = getPage();
    const { timeout = 2500, waitUntil = 'domcontentloaded' } = options;
    const response = await page.goBack({ timeout, waitUntil }).catch(() => null);
    return Boolean(response);
}

/**
 * Go forward in browser history.
 * @returns {Promise<void>}
 */
export async function forward() {
    const page = getPage();
    await page.goForward();
}
