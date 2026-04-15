/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Game State Matcher
 * Wait for visual changes, element states, and resource updates in games.
 * @module api/interactions/gameState
 */

import { getPage, isSessionActive } from "../core/context.js";
import { createLogger } from "../core/logger.js";
import { SessionDisconnectedError } from "../core/errors.js";

const logger = createLogger("api/interactions/gameState.js");

/**
 * Wait for an element to reach a specific state
 * @param {string} selector - Element selector
 * @param {object} options - Wait options
 * @returns {Promise<boolean>}
 */
export async function waitForElementState(selector, options = {}) {
  if (!isSessionActive()) {
    throw new SessionDisconnectedError("Browser closed.");
  }

  const page = getPage();
  const { state = "visible", timeout = 5000, throwOnTimeout = false } = options;

  logger.debug(`Waiting for element "${selector}" to be ${state}`);

  try {
    const locator = page.locator(selector);
    await locator.waitFor({ state, timeout });
    logger.debug(`Element "${selector}" is now ${state}`);
    return true;
  } catch (e) {
    logger.warn(`Element "${selector}" did not become ${state}: ${e.message}`);
    if (throwOnTimeout) {
      throw e;
    }
    return false;
  }
}

/**
 * Wait for element to be enabled/disabled
 * @param {string} selector - Element selector
 * @param {boolean} enabled - Wait for enabled (true) or disabled (false)
 * @param {number} timeout - Timeout in ms
 * @returns {Promise<boolean>}
 */
export async function waitForEnabled(selector, enabled = true, timeout = 5000) {
  if (!isSessionActive()) {
    throw new SessionDisconnectedError("Browser closed.");
  }

  const page = getPage();
  const locator = page.locator(selector);

  const startTime = Date.now();

  while (Date.now() - startTime < timeout) {
    const isVisible = await locator.isVisible().catch(() => false);
    if (!isVisible) {
      await page.waitForTimeout(100);
      continue;
    }

    const isDisabled = await locator.isDisabled().catch(() => false);
    if (enabled ? !isDisabled : isDisabled) {
      return true;
    }

    await page.waitForTimeout(100);
  }

  logger.warn(
    `Element "${selector}" did not become ${enabled ? "enabled" : "disabled"}`,
  );
  return false;
}

/**
 * Wait for text content to match expected value
 * @param {string} selector - Element selector
 * {string} expectedText - Expected text content
 * @param {number} timeout - Timeout in ms
 * @returns {Promise<boolean>}
 */
export async function waitForText(selector, expectedText, timeout = 5000) {
  if (!isSessionActive()) {
    throw new SessionDisconnectedError("Browser closed.");
  }

  const page = getPage();
  const locator = page.locator(selector);

  const startTime = Date.now();

  while (Date.now() - startTime < timeout) {
    const isVisible = await locator.isVisible().catch(() => false);
    if (!isVisible) {
      await page.waitForTimeout(100);
      continue;
    }

    const text = await locator.textContent().catch(() => "");
    if (text && text.includes(expectedText)) {
      return true;
    }

    await page.waitForTimeout(100);
  }

  logger.warn(`Element "${selector}" text did not contain "${expectedText}"`);
  return false;
}

/**
 * Wait for value to change
 * @param {string} selector - Element selector
 * @param {string} initialValue - Initial value to compare against
 * @param {number} timeout - Timeout in ms
 * @returns {Promise<boolean>}
 */
export async function waitForValueChange(
  selector,
  initialValue,
  timeout = 5000,
) {
  if (!isSessionActive()) {
    throw new SessionDisconnectedError("Browser closed.");
  }

  const page = getPage();
  const locator = page.locator(selector);

  const startTime = Date.now();

  while (Date.now() - startTime < timeout) {
    const value = await locator
      .inputValue()
      .catch(() => locator.textContent().catch(() => ""));

    if (value !== initialValue) {
      logger.debug(`Value changed from "${initialValue}" to "${value}"`);
      return true;
    }

    await page.waitForTimeout(100);
  }

  logger.warn(`Value did not change from "${initialValue}"`);
  return false;
}

/**
 * Wait for pixel color to change at coordinates
 * @param {number} x - X coordinate
 * @param {number} y - Y coordinate
 * @param {object} options - Options
 * @returns {Promise<boolean>}
 */
export async function waitForPixelChange(x, y, options = {}) {
  if (!isSessionActive()) {
    throw new SessionDisconnectedError("Browser closed.");
  }

  const page = getPage();
  const { threshold = 10, timeout = 5000 } = options;

  const initialColor = await page.evaluate(
    ([px, py]) => {
      const canvas = document.createElement("canvas");
      const ctx = canvas.getContext("2d");
      canvas.width = 1;
      canvas.height = 1;
      ctx.drawImage(document, px, py, 1, 1, 0, 0, 1, 1);
      const [r, g, b] = ctx.getImageData(0, 0, 1, 1).data;
      return { r, g, b };
    },
    [x, y],
  );

  const startTime = Date.now();

  while (Date.now() - startTime < timeout) {
    await page.waitForTimeout(200);

    const currentColor = await page.evaluate(
      ([px, py]) => {
        const canvas = document.createElement("canvas");
        const ctx = canvas.getContext("2d");
        canvas.width = 1;
        canvas.height = 1;
        ctx.drawImage(document, px, py, 1, 1, 0, 0, 1, 1);
        const [r, g, b] = ctx.getImageData(0, 0, 1, 1).data;
        return { r, g, b };
      },
      [x, y],
    );

    const diff =
      Math.abs(currentColor.r - initialColor.r) +
      Math.abs(currentColor.g - initialColor.g) +
      Math.abs(currentColor.b - initialColor.b);

    if (diff > threshold) {
      logger.debug(`Pixel changed at (${x}, ${y})`);
      return true;
    }
  }

  logger.debug(`Pixel did not change at (${x}, ${y})`);
  return false;
}

/**
 * Wait for popup to appear or disappear
 * @param {string} popupSelector - Popup selector
 * @param {boolean} appear - Wait for appear (true) or disappear (false)
 * @param {number} timeout - Timeout in ms
 * @returns {Promise<boolean>}
 */
export async function waitForPopup(
  popupSelector,
  appear = true,
  timeout = 5000,
) {
  if (!isSessionActive()) {
    throw new SessionDisconnectedError("Browser closed.");
  }

  const page = getPage();
  const locator = page.locator(popupSelector);

  if (appear) {
    try {
      await locator.waitFor({ state: "visible", timeout });
      return true;
    } catch {
      return false;
    }
  } else {
    const startTime = Date.now();

    while (Date.now() - startTime < timeout) {
      const isVisible = await locator.isVisible().catch(() => false);
      if (!isVisible) {
        return true;
      }
      await page.waitForTimeout(100);
    }

    return false;
  }
}

/**
 * Wait for resource values (helper for game resource tracking)
 * @param {object} resources - Resource requirements { gold: 500, wood: 200 }
 * @param {string} resourceSelector - Selector for resource display
 * @param {number} timeout - Timeout in ms
 * @returns {Promise<boolean>}
 */
export async function waitForResources(
  resources,
  resourceSelector = '[class*="resource"]',
  timeout = 30000,
) {
  if (!isSessionActive()) {
    throw new SessionDisconnectedError("Browser closed.");
  }

  const page = getPage();
  const startTime = Date.now();

  while (Date.now() - startTime < timeout) {
    let allMet = true;

    for (const [resourceName, requiredAmount] of Object.entries(resources)) {
      const selector = `${resourceSelector}:has-text("${resourceName}")`;
      const locator = page.locator(selector);

      const isVisible = await locator.isVisible().catch(() => false);
      if (!isVisible) {
        allMet = false;
        break;
      }

      const text = await locator.textContent().catch(() => "0");
      const currentAmount = parseInt(text.replace(/\D/g, ""), 10) || 0;

      if (currentAmount < requiredAmount) {
        allMet = false;
        break;
      }
    }

    if (allMet) {
      logger.info(`Resource requirements met: ${JSON.stringify(resources)}`);
      return true;
    }

    await page.waitForTimeout(500);
  }

  logger.warn(`Resource requirements not met within ${timeout}ms`);
  return false;
}

/**
 * Get current game state snapshot
 * @param {object} options - Options
 * @returns {Promise<object>}
 */
export async function getGameState(options = {}) {
  if (!isSessionActive()) {
    throw new SessionDisconnectedError("Browser closed.");
  }

  const page = getPage();
  const { screenshot = true } = options;

  const state = {
    url: page.url(),
    timestamp: Date.now(),
  };

  if (screenshot) {
    try {
      const buffer = await page.screenshot({ type: "jpeg", quality: 60 });
      state.screenshot = buffer.toString("base64");
    } catch (e) {
      logger.warn("Screenshot failed:", e.message);
    }
  }

  try {
    const axTree = await page.accessibility.snapshot();
    state.axTree = axTree;
  } catch (e) {
    logger.warn("AXTree failed:", e.message);
  }

  return state;
}

/**
 * Extract gold amount from game UI
 * @param {object} options - Options
 * @returns {Promise<number>} Current gold amount (falls back to 180 if unavailable)
 */
export async function extractGold(options = {}) {
  if (!isSessionActive()) {
    throw new SessionDisconnectedError("Browser closed.");
  }

  const page = getPage();
  const { defaultValue = 180, timeout = 2000 } = options;

  const goldSelectors = [
    '[class*="gold"]',
    "#gold",
    '[id*="gold"]',
    ':text-matches("Gold", "i")',
  ];

  const startTime = Date.now();

  while (Date.now() - startTime < timeout) {
    for (const selector of goldSelectors) {
      try {
        const locator = page.locator(selector).first();
        const isVisible = await locator.isVisible().catch(() => false);
        if (!isVisible) continue;

        const text = await locator.textContent().catch(() => "");
        if (!text) continue;

        const match = text.match(/\d[\d,]*/);
        if (match) {
          const gold = parseInt(match[0].replace(/,/g, ""), 10);
          if (!isNaN(gold) && gold > 0) {
            logger.debug(`Extracted gold: ${gold}`);
            return gold;
          }
        }
      } catch {
        continue;
      }
    }

    await page.waitForTimeout(100);
  }

  logger.debug(`Gold extraction failed, using default: ${defaultValue}`);
  return defaultValue;
}

export default {
  waitForElementState,
  waitForEnabled,
  waitForText,
  waitForValueChange,
  waitForPixelChange,
  waitForPopup,
  waitForResources,
  getGameState,
  extractGold,
};
