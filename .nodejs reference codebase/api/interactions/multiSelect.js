/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Multi-Select Interaction
 * Shift-click range selection and Ctrl-click add/remove for strategy games.
 * @module api/interactions/multiSelect
 */

import { getPage, isSessionActive } from "../core/context.js";
import { randomInRange as _randomInRange } from "../behaviors/timing.js";
import { getPersona } from "../behaviors/persona.js";
import { createLogger } from "../core/logger.js";
import { mathUtils } from "../utils/math.js";
import { getLocator as _getLocator, stringify } from "../utils/locator.js";
import { wait } from "./wait.js";
import { SessionDisconnectedError } from "../core/errors.js";

const logger = createLogger("api/interactions/multiSelect.js");

/**
 * @typedef {Object} MultiSelectOptions
 * @property {'add'|'remove'|'range'|'toggle'} [mode='add'] - Selection mode
 * @property {number} [holdMs=100] - Delay between clicks
 * @property {boolean} [recovery=true] - Auto-retry on failure
 */

/**
 * Get element coordinates from various input types
 * @param {string|{x:number,y:number}|number} input
 * @returns {Promise<{x:number,y:number}>}
 */
async function resolveToCoords(input) {
  const page = getPage();

  if (!input) {
    throw new Error("Multi-select item is required");
  }

  if (typeof input === "object" && "x" in input && "y" in input) {
    return { x: input.x, y: input.y };
  }

  if (typeof input === "number") {
    const { getStateAgentElementMap } =
      await import("../core/context-state.js");
    const elementMap = getStateAgentElementMap();
    const element = elementMap.find((el) => el.id === input);
    if (!element) {
      throw new Error(
        `Element with ID ${input} not found. Call api.agent.see() first.`,
      );
    }
    input = element.selector;
  }

  if (typeof input === "string") {
    const locator = page.locator(input).first();
    const box = await locator.boundingBox();
    if (!box) {
      throw new Error(`Could not find element: ${input}`);
    }
    return {
      x: box.x + box.width / 2,
      y: box.y + box.height / 2,
    };
  }

  throw new Error(
    "Invalid multi-select item. Use selector string, {x, y}, or element ID",
  );
}

/**
 * Perform a click with modifier keys held
 * @param {number} x - X coordinate
 * @param {number} y - Y coordinate
 * @param {boolean} shift - Hold Shift
 * @param {boolean} ctrl - Hold Control
 * @param {boolean} alt - Hold Alt
 */
async function clickWithModifiers(
  x,
  y,
  shift = false,
  ctrl = false,
  alt = false,
) {
  const page = getPage();

  if (shift) await page.keyboard.down("Shift");
  if (ctrl) await page.keyboard.down("Control");
  if (alt) await page.keyboard.down("Alt");

  await page.mouse.move(x, y);
  await page.mouse.down();
  await page.waitForTimeout(mathUtils.randomInRange(50, 150));
  await page.mouse.up();

  if (alt) await page.keyboard.up("Alt");
  if (ctrl) await page.keyboard.up("Control");
  if (shift) await page.keyboard.up("Shift");
}

/**
 * Multi-select elements using Shift or Ctrl modifiers.
 * Useful for strategy game unit selection.
 *
 * @param {Array<string|{x:number,y:number}|number>} items - Elements to select
 * @param {MultiSelectOptions} options - Selection configuration
 * @returns {Promise<{success: boolean, selected: number}>}
 *
 * @example
 * // First get element IDs from agent view
 * const view = await api.agent.see();
 * // Shows: [1] button: "Unit A", [2] button: "Unit B", [3] button: "Unit C"...
 *
 * // Add individual units with Ctrl
 * await api.multiSelect([1, 3, 5], { mode: 'add' });
 *
 * // Select range with Shift (click first, then shift-click last)
 * await api.multiSelect([1, 10], { mode: 'range' });
 *
 * // Remove from selection with Ctrl
 * await api.multiSelect([2, 4], { mode: 'remove' });
 *
 * // Toggle selection
 * await api.multiSelect([1, 2, 3], { mode: 'toggle' });
 */
export async function multiSelect(items, options = {}) {
  if (!isSessionActive()) {
    throw new SessionDisconnectedError("Browser closed before multiSelect.");
  }

  if (!Array.isArray(items) || items.length === 0) {
    throw new Error("multiSelect requires a non-empty array of items");
  }

  const page = getPage();
  const _persona = getPersona();

  const { mode = "add", holdMs = 100, recovery = true } = options;

  logger.info(`Multi-selecting ${items.length} items in mode: ${mode}`);

  let selectedCount = 0;

  try {
    if (mode === "range") {
      if (items.length < 2) {
        throw new Error(
          "Range selection requires at least 2 items (start and end)",
        );
      }

      const startCoords = await resolveToCoords(items[0]);
      const endCoords = await resolveToCoords(items[items.length - 1]);

      logger.info(
        `Range: ${stringify(items[0])} → ${stringify(items[items.length - 1])}`,
      );

      await page.mouse.move(startCoords.x, startCoords.y);
      await page.mouse.down();
      await wait(mathUtils.randomInRange(50, 150));

      await page.keyboard.down("Shift");
      await page.mouse.move(endCoords.x, endCoords.y);
      await page.mouse.up();
      await page.keyboard.up("Shift");

      selectedCount = items.length;
    } else {
      const useCtrl = mode === "add" || mode === "remove" || mode === "toggle";
      const ctrlPressed = { current: false };

      for (let i = 0; i < items.length; i++) {
        const item = items[i];

        if (useCtrl && mode !== "toggle") {
          if (!ctrlPressed.current) {
            await page.keyboard.down("Control");
            ctrlPressed.current = true;
          }
        }

        const coords = await resolveToCoords(item);

        logger.debug(
          `Clicking item ${i + 1}/${items.length}: (${coords.x}, ${coords.y})`,
        );

        if (mode === "toggle") {
          await clickWithModifiers(coords.x, coords.y, false, true, false);
        } else {
          await page.mouse.move(coords.x, coords.y);
          await page.mouse.down();
          await page.waitForTimeout(mathUtils.randomInRange(50, 150));
          await page.mouse.up();
        }

        selectedCount++;

        if (i < items.length - 1) {
          await wait(holdMs + mathUtils.randomInRange(-30, 30));
        }
      }

      if (ctrlPressed.current) {
        await page.keyboard.up("Control");
      }
    }

    logger.info(`Multi-select completed. Selected: ${selectedCount}`);

    return { success: true, selected: selectedCount };
  } catch (error) {
    logger.error(`Multi-select failed: ${error.message}`);

    if (recovery) {
      logger.info("Attempting recovery...");
      await page.keyboard.up("Shift").catch(() => {});
      await page.keyboard.up("Control").catch(() => {});
      await page.keyboard.up("Alt").catch(() => {});
      throw error;
    }

    throw error;
  }
}

export default multiSelect;
