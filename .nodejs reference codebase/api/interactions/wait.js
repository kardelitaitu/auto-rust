/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Synchronization Helpers
 * Wait for time, selectors, visibility, and hidden states.
 *
 * @module api/wait
 */

import { getPage } from "../core/context.js";
import { getLocator } from "../utils/locator.js";
import { ValidationError, ElementTimeoutError } from "../core/errors.js";

/**
 * Wait for a duration with Gaussian jitter (±15%).
 * @param {number} ms - Base duration in milliseconds
 * @returns {Promise<void>}
 * @throws {ValidationError} If ms is not a positive number
 */
export async function wait(ms) {
  if (typeof ms !== "number" || Number.isNaN(ms) || ms < 0) {
    throw new ValidationError(
      "VALIDATION_ERROR",
      `wait() requires a positive number, got: ${ms}`,
    );
  }
  const jitter = ms * 0.15 * (Math.random() - 0.5) * 2;
  await new Promise((r) => setTimeout(r, Math.max(0, Math.round(ms + jitter))));
}

/**
 * Wait for a duration with abort signal support.
 * @param {number} ms - Base duration in milliseconds
 * @param {AbortSignal} [signal] - Optional abort signal
 * @returns {Promise<void>}
 * @throws {ValidationError} If ms is not a positive number
 * @throws {Error} If signal is aborted
 */
export async function waitWithAbort(ms, signal) {
  if (typeof ms !== "number" || Number.isNaN(ms) || ms < 0) {
    throw new ValidationError(
      "VALIDATION_ERROR",
      `waitWithAbort() requires a positive number, got: ${ms}`,
    );
  }

  if (signal && signal.aborted) {
    throw signal.reason || new Error("Aborted");
  }

  return new Promise((resolve, reject) => {
    const abortHandler = () => {
      clearTimeout(timeoutId);
      reject(signal.reason || new Error("Aborted"));
    };

    if (signal) {
      signal.addEventListener("abort", abortHandler, { once: true });
    }

    const jitter = ms * 0.15 * (Math.random() - 0.5) * 2;
    const timeoutId = setTimeout(
      () => {
        if (signal) {
          signal.removeEventListener("abort", abortHandler);
        }
        resolve();
      },
      Math.max(0, Math.round(ms + jitter)),
    );
  });
}

/**
 * Wait for a selector or a predicate function.
 * @param {string|import('playwright').Locator|Function} selectorOrPredicate - CSS selector, Locator, or Predicate function
 * @param {object} [options]
 * @param {number} [options.timeout=10000] - Max wait time in ms
 * @param {string} [options.state='attached'] - For selectors: attached, visible, hidden, detached
 * @param {number} [options.polling=100] - For predicates: interval between checks in ms
 * @returns {Promise<void>}
 */
export async function waitFor(selectorOrPredicate, options = {}) {
  const { timeout = 10000, state = "attached", polling = 100 } = options;

  if (typeof selectorOrPredicate === "function") {
    const startTime = Date.now();
    while (Date.now() - startTime < timeout) {
      try {
        if (await selectorOrPredicate()) return;
      } catch (_e) {
        // Ignore errors during polling (e.g. page crashes, temporary disconnects)
      }
      await new Promise((r) => setTimeout(r, polling));
    }
    throw new ElementTimeoutError("predicate", timeout);
  }

  const locator = getLocator(selectorOrPredicate);
  await locator.waitFor({ state, timeout });
}

/**
 * Wait for a selector to become visible.
 * @param {string|import('playwright').Locator} selector - CSS selector or Locator
 * @param {object} [options]
 * @param {number} [options.timeout=10000] - Max wait time in ms
 * @returns {Promise<void>}
 */
export async function waitVisible(selector, options = {}) {
  const { timeout = 10000 } = options;
  const locator = getLocator(selector).first();
  await locator.waitFor({ state: "visible", timeout });
}

/**
 * Wait for a selector to become hidden or detached.
 * @param {string|import('playwright').Locator} selector - CSS selector or Locator
 * @param {object} [options]
 * @param {number} [options.timeout=10000] - Max wait time in ms
 * @returns {Promise<void>}
 */
export async function waitHidden(selector, options = {}) {
  const { timeout = 10000 } = options;
  const locator = getLocator(selector).first();
  await locator.waitFor({ state: "hidden", timeout });
}

export async function waitForLoadState(state = "networkidle", options = {}) {
  const page = getPage();
  const { timeout = 10000 } = options;
  await page.waitForLoadState(state, { timeout });
}

export async function waitForURL(urlOrPredicate, options = {}) {
  const page = getPage();
  const { timeout = 10000, waitUntil } = options;
  await page.waitForURL(urlOrPredicate, { timeout, waitUntil });
}
