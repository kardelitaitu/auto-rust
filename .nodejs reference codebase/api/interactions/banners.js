/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Cookie Banner and Consent Handling
 * @module api/banners
 */

import { getPage } from "../core/context.js";
import { delay, randomInRange } from "../behaviors/timing.js";

/**
 * Common selectors for cookie consent and "Accept" buttons.
 * Grouped by common text patterns.
 */
const CONSENT_SELECTORS = [
  // Text-based (Playwright :has-text)
  'button:has-text("Accept all")',
  'button:has-text("Accept All")',
  'button:has-text("I agree")',
  'button:has-text("I Agree")',
  'button:has-text("Allow all")',
  'button:has-text("Allow All")',
  'button:has-text("Agree")',
  'button:has-text("Accept")',
  'button:has-text("OK")',
  'button:has-text("Understood")',
  'a:has-text("Accept all")',
  'a:has-text("Accept All")',
  'button [text*="Accept"]',

  // Common ID/Class patterns
  "#onetrust-accept-btn-handler",
  "#consent-accept",
  ".cookie-consent-accept",
  ".accept-cookies",
  '[aria-label*="Accept all"]',
  '[aria-label*="Got it"]',
  '[aria-label*="Accept"]',
  '[aria-label*="OK"]',
  '[aria-label*="Yes]',
  '[aria-label*="Accept cookies"]',
];

/**
 * Tries to find and click a cookie consent button on the page.
 * @param {object} [options]
 * @param {number} [options.timeout=2000] - Max time to look for a banner
 * @param {boolean} [options.waitAfter=true] - Wait a bit after clicking
 * @returns {Promise<boolean>} True if a banner was found and clicked
 */
export async function handleBanners(options = {}) {
  const page = getPage();
  const { timeout: _timeout = 2000, waitAfter = true } = options;

  // logger.debug('Checking for cookie banners...');

  for (const selector of CONSENT_SELECTORS) {
    try {
      const loc = page.locator(selector).first();
      if (await loc.isVisible({ timeout: 100 })) {
        //logger.info(`Banner detected! Clicking: ${selector}`);

        // Human-like delay before clicking
        await delay(randomInRange(500, 1500));

        await loc.click({ force: true }).catch(() => {});

        if (waitAfter) {
          await delay(randomInRange(1000, 2000));
        }

        return true;
      }
    } catch (_e) {
      // Ignore visibility/timeout errors for individual selectors
    }
  }

  return false;
}
