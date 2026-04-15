/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Game Unit Selection Primitives
 * Select units, box selection, and unit management for strategy games.
 * @module api/interactions/gameUnits
 */

import { getPage, isSessionActive } from "../core/context.js";
import { createLogger } from "../core/logger.js";
import { mathUtils } from "../utils/math.js";
import { SessionDisconnectedError } from "../core/errors.js";

const logger = createLogger("api/interactions/gameUnits.js");

/**
 * Select a single unit by selector or element ID
 * @param {string|number} unit - Selector or element ID
 * @returns {Promise<boolean>}
 */
export async function selectUnit(unit) {
  if (!isSessionActive()) {
    throw new SessionDisconnectedError("Browser closed.");
  }

  const page = getPage();

  logger.info(`Selecting unit: ${unit}`);

  let coords;

  if (typeof unit === "number") {
    const { getStateAgentElementMap } =
      await import("../core/context-state.js");
    const elementMap = getStateAgentElementMap();
    const element = elementMap.find((el) => el.id === unit);
    if (!element) {
      throw new Error(`Element with ID ${unit} not found`);
    }
    const locator = page.locator(element.selector);
    const box = await locator.boundingBox();
    if (!box) {
      throw new Error(`Could not find element for unit ${unit}`);
    }
    coords = { x: box.x + box.width / 2, y: box.y + box.height / 2 };
  } else if (typeof unit === "string") {
    const locator = page.locator(unit).first();
    const box = await locator.boundingBox();
    if (!box) {
      throw new Error(`Could not find unit: ${unit}`);
    }
    coords = { x: box.x + box.width / 2, y: box.y + box.height / 2 };
  } else if (typeof unit === "object" && "x" in unit && "y" in unit) {
    coords = unit;
  } else {
    throw new Error("Invalid unit input");
  }

  await page.mouse.move(coords.x, coords.y);
  await page.mouse.down();
  await page.waitForTimeout(mathUtils.randomInRange(50, 150));
  await page.mouse.up();

  logger.info(`Unit selected at (${coords.x}, ${coords.y})`);
  return true;
}

/**
 * Box selection - drag to select multiple units
 * @param {object} start - Start coordinates {x, y}
 * @param {object} end - End coordinates {x, y}
 * @returns {Promise<boolean>}
 */
export async function selectByArea(start, end) {
  if (!isSessionActive()) {
    throw new Error("SessionDisconnectedError: Browser closed.");
  }

  const page = getPage();

  logger.info(
    `Box selection from (${start.x}, ${start.y}) to (${end.x}, ${end.y})`,
  );

  await page.mouse.move(start.x, start.y);
  await page.mouse.down();
  await page.waitForTimeout(100);

  await page.mouse.move(end.x, end.y, { steps: 10 });
  await page.waitForTimeout(100);
  await page.mouse.up();

  logger.info("Box selection completed");
  return true;
}

/**
 * Select all units on screen
 * @returns {Promise<boolean>}
 */
export async function selectAll() {
  if (!isSessionActive()) {
    throw new Error("SessionDisconnectedError: Browser closed.");
  }

  const page = getPage();

  logger.info("Selecting all units (Ctrl+A)");

  const isMac = process.platform === "darwin";
  const modifier = isMac ? "Meta" : "Control";

  await page.keyboard.down(modifier);
  await page.keyboard.press("a");
  await page.keyboard.up(modifier);

  await page.waitForTimeout(200);

  return true;
}

/**
 * Deselect all units (click on empty area)
 * @param {object} options - Options
 * @returns {Promise<boolean>}
 */
export async function deselectAll(options = {}) {
  if (!isSessionActive()) {
    throw new Error("SessionDisconnectedError: Browser closed.");
  }

  const page = getPage();
  const { x = 10, y = 10 } = options;

  logger.info("Deselecting all (click empty area)");

  await page.mouse.move(x, y);
  await page.click(x, y);

  return true;
}

/**
 * Get currently selected units
 * @param {string} selectedIndicator - Selector for selected state indicator
 * @returns {Promise<Array>}
 */
export async function getSelectedUnits(
  selectedIndicator = '[class*="selected"]',
) {
  if (!isSessionActive()) {
    throw new Error("SessionDisconnectedError: Browser closed.");
  }

  const page = getPage();

  const selectedLocator = page.locator(selectedIndicator);
  const count = await selectedLocator.count();

  const units = [];
  for (let i = 0; i < count; i++) {
    const el = selectedLocator.nth(i);
    const box = await el.boundingBox().catch(() => null);
    if (box) {
      units.push({
        index: i,
        x: box.x,
        y: box.y,
        width: box.width,
        height: box.height,
      });
    }
  }

  logger.info(`Found ${units.length} selected units`);
  return units;
}

/**
 * Right-click to issue command (move, attack, etc.)
 * @param {object} target - Target coordinates or selector
 * @param {string} commandType - Command type description
 * @returns {Promise<boolean>}
 */
export async function issueCommand(target, commandType = "move") {
  if (!isSessionActive()) {
    throw new Error("SessionDisconnectedError: Browser closed.");
  }

  const page = getPage();

  let coords;

  if (typeof target === "string") {
    const locator = page.locator(target).first();
    const box = await locator.boundingBox();
    if (!box) {
      throw new Error(`Could not find target: ${target}`);
    }
    coords = { x: box.x + box.width / 2, y: box.y + box.height / 2 };
  } else if (typeof target === "object" && "x" in target && "y" in target) {
    coords = target;
  } else {
    throw new Error("Invalid target");
  }

  logger.info(`Issuing ${commandType} command to (${coords.x}, ${coords.y})`);

  await page.mouse.move(coords.x, coords.y);
  await page.mouse.down({ button: "right" });
  await page.waitForTimeout(50);
  await page.mouse.up({ button: "right" });

  return true;
}

/**
 * Group select - press hotkey to select a unit group
 * @param {number} groupNumber - Group number (1-9)
 * @returns {Promise<boolean>}
 */
export async function selectGroup(groupNumber) {
  if (!isSessionActive()) {
    throw new Error("SessionDisconnectedError: Browser closed.");
  }

  if (groupNumber < 1 || groupNumber > 9) {
    throw new Error("Group number must be between 1 and 9");
  }

  const page = getPage();

  logger.info(`Selecting group ${groupNumber}`);

  await page.keyboard.press(String(groupNumber));
  await page.waitForTimeout(200);

  return true;
}

/**
 * Group assign - assign selected units to a group
 * @param {number} groupNumber - Group number (1-9)
 * @returns {Promise<boolean>}
 */
export async function assignGroup(groupNumber) {
  if (!isSessionActive()) {
    throw new Error("SessionDisconnectedError: Browser closed.");
  }

  if (groupNumber < 1 || groupNumber > 9) {
    throw new Error("Group number must be between 1 and 9");
  }

  const page = getPage();

  logger.info(`Assigning selected units to group ${groupNumber}`);

  const isMac = process.platform === "darwin";
  const modifier = isMac ? "Meta" : "Control";

  await page.keyboard.down(modifier);
  await page.keyboard.press(String(groupNumber));
  await page.keyboard.up(modifier);
  await page.waitForTimeout(200);

  return true;
}

/**
 * Idle unit selection - select idle units of a type
 * @param {string} unitType - Unit type selector (optional)
 * @returns {Promise<number>} Number of idle units found
 */
export async function selectIdleUnits(_unitType = null) {
  if (!isSessionActive()) {
    throw new Error("SessionDisconnectedError: Browser closed.");
  }

  const page = getPage();

  logger.info("Selecting idle units");

  const hotkey = "Tab";
  await page.keyboard.press(hotkey);
  await page.waitForTimeout(500);

  const count = await getSelectedUnits();
  return count.length;
}

export default {
  selectUnit,
  selectByArea,
  selectAll,
  deselectAll,
  getSelectedUnits,
  issueCommand,
  selectGroup,
  assignGroup,
  selectIdleUnits,
};
