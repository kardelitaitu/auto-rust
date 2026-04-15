/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Keyboard Interaction
 * Unified keyboard API supporting single keys, chords, and modifier combinations.
 * @module api/interactions/keys
 */

import { getPage, isSessionActive } from "../core/context.js";
import { randomInRange as _randomInRange } from "../behaviors/timing.js";
import { getPersona } from "../behaviors/persona.js";
import { createLogger } from "../core/logger.js";
import { mathUtils } from "../utils/math.js";
import { SessionDisconnectedError } from "../core/errors.js";

const logger = createLogger("api/interactions/keys.js");

/**
 * @typedef {Object} PressOptions
 * @property {Array<'ctrl'|'shift'|'alt'|'meta'>} [modifiers=[]] - Modifier keys to hold
 * @property {number} [delay=0] - Delay between keys in ms (for chords)
 * @property {number} [repeat=1] - Times to repeat the key press
 * @property {boolean} [downAndUp=true] - Do full down+up vs just press
 */

/**
 * Normalize modifier names
 * @param {string} modifier
 * @returns {string}
 */
function normalizeModifier(modifier) {
  const map = {
    ctrl: "Control",
    control: "Control",
    shift: "Shift",
    alt: "Alt",
    meta: "Meta",
    cmd: "Meta",
    command: "Meta",
    win: "Meta",
  };
  return map[modifier.toLowerCase()] || modifier;
}

/**
 * Check if key is a modifier
 * @param {string} key
 * @returns {boolean}
 */
function isModifier(key) {
  const modifiers = ["Control", "Shift", "Alt", "Meta"];
  return modifiers.includes(key) || modifiers.includes(normalizeModifier(key));
}

/**
 * Press a single key or combination.
 *
 * @param {string|Array<string>} key - Single key, array of keys (chord), or special keys
 * @param {PressOptions|Array<'ctrl'|'shift'|'alt'|'meta'>} [options] - Options or modifiers array
 * @returns {Promise<{success: boolean}>}
 *
 * @example
 * // Single keys
 * await api.press('Enter');
 * await api.press('Escape');
 * await api.press('a');
 *
 * // With modifiers
 * await api.press('s', ['ctrl']);       // Ctrl+S
 * await api.press('c', ['ctrl', 'shift']); // Ctrl+Shift+C
 *
 * // Key chords (multiple keys pressed in sequence)
 * await api.press(['w', 'a', 's', 'd']); // WASD game movement
 * await api.press(['w', 'a', 's', 'd'], { delay: 50 });
 *
 * // Special keys
 * await api.press('ArrowUp');
 * await api.press('F1');
 * await api.press('Tab');
 * await api.press('Backspace');
 *
 * // With options
 * await api.press('Enter', { repeat: 3, delay: 100 });
 */
export async function press(key, options = {}) {
  if (!isSessionActive()) {
    throw new SessionDisconnectedError("Browser closed before press.");
  }

  const page = getPage();
  const _persona = getPersona();

  let modifiers = [];
  let delay = 0;
  let repeat = 1;
  let downAndUp = true;

  if (Array.isArray(options)) {
    modifiers = options;
  } else if (typeof options === "object") {
    modifiers = options.modifiers || [];
    delay = options.delay || 0;
    repeat = options.repeat || 1;
    downAndUp = options.downAndUp !== false;
  }

  if (!key) {
    throw new Error("Key is required for press()");
  }

  const keys = Array.isArray(key) ? key : [key];

  logger.info(
    `Pressing: ${keys.join("+")} ${modifiers.length ? "(" + modifiers.join(",") + ")" : ""}`,
  );

  try {
    const normalizedModifiers = modifiers.map(normalizeModifier);

    for (let r = 0; r < repeat; r++) {
      for (const mod of normalizedModifiers) {
        await page.keyboard.down(mod);
      }

      for (let i = 0; i < keys.length; i++) {
        const k = keys[i];

        if (isModifier(k)) {
          continue;
        }

        if (downAndUp) {
          await page.keyboard.down(k);
          await page.waitForTimeout(mathUtils.randomInRange(20, 50));
          await page.keyboard.up(k);
        } else {
          await page.keyboard.press(k);
        }

        if (i < keys.length - 1 && delay > 0) {
          await page.waitForTimeout(delay);
        }
      }

      for (const mod of normalizedModifiers) {
        await page.keyboard.up(mod);
      }

      if (r < repeat - 1) {
        await page.waitForTimeout(mathUtils.randomInRange(50, 150));
      }
    }

    logger.info(`Press completed: ${keys.join("+")}`);
    return { success: true };
  } catch (error) {
    logger.error(`Press failed: ${error.message}`);

    // Cleanup modifier keys
    await page.keyboard.up("Shift").catch(() => {});
    await page.keyboard.up("Control").catch(() => {});
    await page.keyboard.up("Alt").catch(() => {});
    await page.keyboard.up("Meta").catch(() => {});

    throw error;
  }
}

/**
 * Type text with human-like timing (uses existing api.type logic)
 * @param {string} text - Text to type
 * @param {Object} [options] - Typing options
 * @returns {Promise<void>}
 */
export async function typeText(text, options = {}) {
  if (!isSessionActive()) {
    throw new SessionDisconnectedError("Browser closed before typeText.");
  }

  const page = getPage();
  const persona = getPersona();
  const { delay = null } = options;

  const baseDelay = delay ?? Math.round(100 / persona.speed);

  logger.info(`Typing: "${text}"`);

  for (let i = 0; i < text.length; i++) {
    const char = text[i];
    await page.keyboard.type(char, { delay: 0 });

    let charDelay = mathUtils.gaussian(
      baseDelay,
      baseDelay * 0.3,
      30,
      baseDelay * 3,
    );

    if (".!?,;:".includes(char)) {
      charDelay += mathUtils.randomInRange(100, 300);
    }

    await page.waitForTimeout(charDelay);
  }

  return { success: true };
}

/**
 * Hold a key down for a duration (for games)
 * @param {string} key - Key to hold
 * @param {number} durationMs - Duration to hold in ms
 * @returns {Promise<void>}
 */
export async function hold(key, durationMs = 500) {
  if (!isSessionActive()) {
    throw new SessionDisconnectedError("Browser closed before hold.");
  }

  const page = getPage();

  logger.info(`Holding ${key} for ${durationMs}ms`);

  await page.keyboard.down(key);
  await page.waitForTimeout(durationMs);
  await page.keyboard.up(key);

  return { success: true };
}

/**
 * Release all held keys (safety function)
 * @returns {Promise<void>}
 */
export async function releaseAll() {
  const page = getPage();

  await page.keyboard.up("Shift").catch(() => {});
  await page.keyboard.up("Control").catch(() => {});
  await page.keyboard.up("Alt").catch(() => {});
  await page.keyboard.up("Meta").catch(() => {});

  logger.info("Released all modifier keys");
  return { success: true };
}

export default {
  press,
  typeText,
  hold,
  releaseAll,
};
