/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview High-Level Kinetic Actions
 * Each action auto-invokes scroll.focus() → cursor move → execute.
 * All behaviors are persona-aware.
 *
 * @module api/actions
 */

import {
  getPage,
  getCursor,
  isSessionActive,
  getEvents,
} from "../core/context.js";
import { setPreviousUrl } from "../core/context-state.js";
import { randomInRange } from "../behaviors/timing.js";
import { focus } from "./scroll.js";
import { wait } from "./wait.js";
import { getPersona } from "../behaviors/persona.js";
import { createLogger } from "../core/logger.js";
import { mathUtils } from "../utils/math.js";
import { getLocator, stringify } from "../utils/locator.js";
import {
  createPipeline,
  retryMiddleware,
  recoveryMiddleware,
} from "../core/middleware.js";
import {
  ElementObscuredError,
  SessionDisconnectedError,
} from "../core/errors.js";

export { drag } from "./drag.js";
export { clickAt } from "./clickAt.js";
export { multiSelect } from "./multiSelect.js";
export { press, typeText, hold, releaseAll } from "./keys.js";

const logger = createLogger("api/actions.js");

function safeEmitWarning(context, error) {
  try {
    const events = getEvents();
    events.emitSafe("on:error", { context, error, level: "warning" });
  } catch {
    logger.debug(`[Warning] ${context}:`, error);
  }
}

async function waitForStableBox(locator, options = {}) {
  const {
    timeoutMs = 2000,
    stableChecks = 3,
    intervalMs = 100,
    movementThreshold = 2,
  } = options;

  const start = Date.now();
  let prev = null;
  let stable = 0;

  while (Date.now() - start < timeoutMs) {
    if (!isSessionActive()) {
      throw new SessionDisconnectedError(
        "Browser closed during stability check.",
      );
    }
    const box = await locator.boundingBox().catch((e) => {
      if (e.message?.includes("SessionDisconnectedError")) throw e;
      return null;
    });
    if (!box) {
      await wait(intervalMs);
      continue;
    }

    if (prev) {
      const delta = Math.abs(box.x - prev.x) + Math.abs(box.y - prev.y);
      if (delta <= movementThreshold) {
        stable += 1;
        if (stable >= stableChecks) {
          return box;
        }
      } else {
        stable = 0;
      }
    }

    prev = box;
    await wait(intervalMs);
  }

  return prev;
}

/**
 * Checks if an element is obscured by another element.
 */
async function isObscured(locator) {
  return await locator
    .evaluate((el) => {
      const rect = el.getBoundingClientRect();
      const cx = rect.left + rect.width / 2;
      const cy = rect.top + rect.height / 2;
      const elementAtPoint = document.elementFromPoint(cx, cy);
      if (!elementAtPoint) return false;
      return !el.contains(elementAtPoint) && !elementAtPoint.contains(el);
    })
    .catch(() => false);
}

/**
 * Human-like click on a DOM element.
 * 2-Step Movement: scroll → move cursor → click
 * @param {string|import('playwright').Locator} selector - CSS selector or Locator to click
 * @param {object} [options]
 * @param {boolean} [options.recovery=true] - Automatic recovery on failure
 * @param {number} [options.maxRetries=3] - Max retry attempts
 * @param {boolean} [options.hoverBeforeClick=false] - Hover with drift before clicking
 * @param {string} [options.precision='normal'] - 'normal' or 'high'
 * @param {string} [options.button='left'] - Mouse button
 * @returns {Promise<{success: boolean, usedFallback: boolean}>}
 */
export async function click(selector, options = {}) {
  if (!isSessionActive()) {
    throw new SessionDisconnectedError("Browser closed before click.");
  }

  const page = getPage();
  setPreviousUrl(page.url());

  const {
    recovery = true,
    maxRetries = 3,
    ensureStable = true,
    hoverBeforeClick = false,
    precision = "normal",
    button = "left",
  } = options;

  try {
    const pipeline = createPipeline(
      retryMiddleware({
        maxRetries:
          options.maxRetries !== undefined
            ? maxRetries
            : recovery
              ? maxRetries
              : 0,
      }),
      recoveryMiddleware({
        scrollOnDetached: recovery,
        retryOnObscured: recovery,
      }),
    );

    return await pipeline(
      async () => {
        const _page = getPage();
        const cursor = getCursor();
        const persona = getPersona();

        // STEP 1: Scroll to Golden View
        await focus(selector, { timeout: 5000 }).catch((e) =>
          safeEmitWarning("click:focus", e),
        );
        await wait(randomInRange(200, 800));

        // STEP 2: Pre-click stability & Visibility
        const locator = getLocator(selector).first();
        if (ensureStable) {
          await waitForStableBox(locator, { timeoutMs: 2000 }).catch((e) => {
            if (e.message?.includes("SessionDisconnectedError")) throw e;
            return null;
          });
        }

        // Ghost 3.0: Visual-Semantic Guard (Obstruction check)
        if (!options.force && (await isObscured(locator))) {
          throw new ElementObscuredError(
            `Element "${stringify(selector)}" appears obscured.`,
          );
        }

        // STEP 3: Move cursor
        try {
          if (typeof selector === "string") {
            await cursor.move(selector);
          } else {
            // If it's a locator, we move to its box
            const box = await locator.boundingBox();
            if (box) {
              await cursor.move(box.x + box.width / 2, box.y + box.height / 2);
            }
          }
        } catch (e) {
          safeEmitWarning("click:cursor-move", e);
        }
        await wait(mathUtils.randomInRange(100, 400));

        // STEP 4: Kinetic Action
        const result = await cursor.click(locator, {
          allowNativeFallback: true,
          hoverBeforeClick,
          hoverMinMs: persona.hoverMin,
          hoverMaxMs: persona.hoverMax,
          precision,
          button,
        });

        await wait(mathUtils.randomInRange(200, 600)); // Post-click observation
        return result;
      },
      { action: "click", selector, options },
    );
  } catch (e) {
    console.error("CRITICAL ERROR in api.click:", e);
    if (e.message && e.message.includes("SessionDisconnectedError")) {
      console.error("DEBUG INFO:", {
        isClosed: getPage().isClosed(),
        isConnected: getPage().context().browser()?.isConnected(),
        stack: e.stack,
      });
    }
    throw e;
  }
}

/**
 * Human-like typing into a DOM element.
 * Scrolls to Golden View, focuses element, types character-by-character
 * with persona-driven typo injection and correction.
 * @param {string|import('playwright').Locator} selector - CSS selector or Locator of the input/textarea
 * @param {string} text - Text to type
 * @param {object} [options]
 * @param {number} [options.typoRate] - Override persona typo rate
 * @param {number} [options.correctionRate] - Override persona correction rate
 * @param {boolean} [options.clearFirst=false] - Clear field before typing
 * @param {number} [options.timeoutMs=5000] - Timeout for waiting for element to be attached
 * @param {boolean} [options.recovery=true] - Automatic recovery on failure
 * @returns {Promise<void>}
 */
export async function type(selector, text, options = {}) {
  if (!isSessionActive()) {
    throw new SessionDisconnectedError("Browser closed before type.");
  }

  // Ensure text is immutable copy to prevent race conditions in concurrent sessions
  const textToType = text != null ? String(text) : "";

  const { recovery = true } = options;

  const pipeline = createPipeline(
    retryMiddleware({ maxRetries: recovery ? 2 : 0 }),
    recoveryMiddleware(),
  );

  return await pipeline(
    async () => {
      const page = getPage();
      const persona = getPersona();
      const {
        typoRate = persona.typoRate,
        correctionRate = persona.correctionRate,
        clearFirst = false,
      } = options;

      // Golden View: scroll + cursor to element
      await focus(selector).catch((e) => safeEmitWarning("type:focus", e));

      const locator = getLocator(selector).first();
      // waitForSelector only works with strings, so we use locator.waitFor
      await locator
        .waitFor({ state: "attached", timeout: 3000 })
        .catch((e) => safeEmitWarning("type:waitFor", e));
      await waitForStableBox(locator, { timeoutMs: 2000 }).catch(() => null);

      // Ghost 3.0: Visual-Semantic Guard
      if (!options.force && (await isObscured(locator))) {
        throw new ElementObscuredError(
          `Input "${stringify(selector)}" appears obscured.`,
        );
      }

      // Focus the element
      await locator
        .click({ timeout: 3000 })
        .catch((e) => safeEmitWarning("type:click", e));

      // Clear if requested
      if (clearFirst) {
        const isMac = process.platform === "darwin";
        const modifier = isMac ? "Meta" : "Control";
        await page.keyboard.press(`${modifier}+A`);
        await page.keyboard.press("Backspace");
        await wait(mathUtils.randomInRange(100, 300));
      }

      // Type character by character with humanization
      const baseDelay = Math.round(100 / persona.speed); // ms per character

      for (let i = 0; i < textToType.length; i++) {
        const char = textToType[i];

        // Typo injection
        if (mathUtils.roll(typoRate)) {
          // Type wrong character
          const wrongChar = _getAdjacentKey(char);
          await page.keyboard.type(wrongChar, { delay: 0 });
          await wait(mathUtils.randomInRange(50, 200));

          // Maybe correct the typo
          if (mathUtils.roll(correctionRate)) {
            await page.keyboard.press("Backspace");
            await wait(mathUtils.randomInRange(80, 250));
            await page.keyboard.type(char, { delay: 0 });
          }
        } else {
          await page.keyboard.type(char, { delay: 0 });
        }

        // Inter-character delay with Gaussian distribution
        let charDelay = mathUtils.gaussian(
          baseDelay,
          baseDelay * 0.3,
          30,
          baseDelay * 3,
        );

        // Punctuation pause
        if (".!?,;:".includes(char)) {
          charDelay += mathUtils.randomInRange(100, 300);
        }

        // Hesitation (persona-driven)
        if (mathUtils.roll(persona.hesitation * 0.5)) {
          charDelay += mathUtils.randomInRange(200, persona.hesitationDelay);
        }

        await wait(charDelay);
      }
    },
    { action: "type", selector, options },
  );
}

/**
 * Human-like hover on a DOM element.
 * Scrolls to Golden View, moves cursor, and drifts.
 * @param {string|import('playwright').Locator} selector - CSS selector or Locator to hover
 * @param {object} [options]
 * @param {number} [options.duration] - Override hover duration
 * @param {boolean} [options.recovery=true] - Automatic recovery on failure
 * @returns {Promise<void>}
 */
export async function hover(selector, options = {}) {
  if (!isSessionActive()) {
    throw new SessionDisconnectedError("Browser closed before hover.");
  }
  const { recovery = true } = options;

  const pipeline = createPipeline(
    retryMiddleware({ maxRetries: recovery ? 2 : 0 }),
    recoveryMiddleware(),
  );

  return await pipeline(
    async () => {
      const _page = getPage();
      const cursor = getCursor();
      const persona = getPersona();

      await focus(selector).catch((e) => safeEmitWarning("hover:focus", e));

      const locator = getLocator(selector).first();
      const box = await locator.boundingBox();
      if (!box)
        throw new Error(`Target not found for hover: ${stringify(selector)}`);

      const targetX = mathUtils.gaussian(box.x + box.width / 2, box.width / 6);
      const targetY = mathUtils.gaussian(
        box.y + box.height / 2,
        box.height / 6,
      );

      const hoverDuration =
        options.duration ||
        mathUtils.randomInRange(persona.hoverMin, persona.hoverMax);
      await cursor.hoverWithDrift(
        targetX,
        targetY,
        hoverDuration,
        hoverDuration + 200,
      );
    },
    { action: "hover", selector, options },
  );
}

/**
 * Right-click on a DOM element.
 * @param {string|import('playwright').Locator} selector - CSS selector or Locator
 * @param {object} [options] - Same as click() options
 * @returns {Promise<{success: boolean, usedFallback: boolean}>}
 */
export async function rightClick(selector, options = {}) {
  return click(selector, { ...options, button: "right" });
}

// ─── Internal ────────────────────────────────────────────────────────────────

/** QWERTY keyboard adjacency map for typo simulation */
const ADJACENT_KEYS = {
  a: "sq",
  b: "vn",
  c: "xv",
  d: "sf",
  e: "wr",
  f: "dg",
  g: "fh",
  h: "gj",
  i: "uo",
  j: "hk",
  k: "jl",
  l: "k;",
  m: "n,",
  n: "bm",
  o: "ip",
  p: "o[",
  q: "wa",
  r: "et",
  s: "ad",
  t: "ry",
  u: "yi",
  v: "cb",
  w: "qe",
  x: "zc",
  y: "tu",
  z: "xs",
};

/**
 * Get a random adjacent key for typo simulation.
 * @param {string} char
 * @returns {string}
 */
function _getAdjacentKey(char) {
  const lower = char.toLowerCase();
  const adjacents = ADJACENT_KEYS[lower];
  if (!adjacents) return char;
  const picked = adjacents[Math.floor(Math.random() * adjacents.length)];
  return char === char.toUpperCase() ? picked.toUpperCase() : picked;
}
