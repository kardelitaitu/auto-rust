/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Game Menu Automation
 * Generic menu operations using coordinates for strategy games
 * @module api/interactions/gameMenus
 */

import { getPage as _getPage, isSessionActive } from "../core/context.js";
import { createLogger } from "../core/logger.js";
import { mathUtils as _mathUtils } from "../utils/math.js";
import { clickAt } from "./clickAt.js";
import { wait } from "./wait.js";
import { SessionDisconnectedError } from "../core/errors.js";

const logger = createLogger("api/interactions/gameMenus.js");

let menuConfig = {
  build: { button: { x: 50, y: 500 } },
  barracks: { x: 150, y: 250 },
  house: { x: 200, y: 250 },
  farm: { x: 250, y: 250 },
  footman: { x: 100, y: 300 },
  archer: { x: 150, y: 300 },
  confirm: { x: 400, y: 450 },
  cancel: { x: 300, y: 450 },
  close: { x: 600, y: 50 },
  research: { x: 300, y: 400 },
  blacksmith: { x: 200, y: 350 },
  weaponry: { x: 100, y: 200 },
  armor: { x: 150, y: 200 },
};

/**
 * Load menu config from file
 * @param {object} customConfig - Custom config to merge
 */
export function loadConfig(customConfig) {
  if (customConfig) {
    menuConfig = { ...menuConfig, ...customConfig };
    logger.info("Menu config loaded");
  }
}

/**
 * Get menu position from config
 * @param {string} key - Menu item key
 * @returns {object|null}
 */
function getPosition(key) {
  const pos = menuConfig[key];
  if (!pos) {
    return null;
  }
  return pos.button || pos;
}

/**
 * Open a menu
 * @param {string} menuName - Menu name (build, research, etc.)
 * @returns {Promise<boolean>}
 */
export async function openMenu(menuName) {
  if (!isSessionActive()) {
    throw new SessionDisconnectedError("Browser closed.");
  }

  const pos = getPosition(menuName);
  if (!pos) {
    logger.warn(`Menu "${menuName}" not found in config`);
    return false;
  }

  logger.info(`Opening menu: ${menuName}`);
  await clickAt(pos.x, pos.y);
  await wait(300);

  return true;
}

/**
 * Close current menu
 * @param {string} [position] - Optional close button position
 * @returns {Promise<boolean>}
 */
export async function closeMenu(position = null) {
  if (!isSessionActive()) {
    throw new SessionDisconnectedError("Browser closed.");
  }

  const pos = position || getPosition("close");
  if (!pos) {
    logger.warn("No close position defined");
    return false;
  }

  logger.info("Closing menu");
  await clickAt(pos.x, pos.y);
  await wait(200);

  return true;
}

/**
 * Select an item from menu
 * @param {string} itemName - Item name (barracks, footman, etc.)
 * @returns {Promise<boolean>}
 */
export async function selectItem(itemName) {
  if (!isSessionActive()) {
    throw new SessionDisconnectedError("Browser closed.");
  }

  const pos = getPosition(itemName);
  if (!pos) {
    logger.warn(`Item "${itemName}" not found in config`);
    return false;
  }

  logger.info(`Selecting item: ${itemName}`);
  await clickAt(pos.x, pos.y);
  await wait(200);

  return true;
}

/**
 * Confirm action
 * @returns {Promise<boolean>}
 */
export async function confirm() {
  if (!isSessionActive()) {
    throw new SessionDisconnectedError("Browser closed.");
  }

  const pos = getPosition("confirm");
  if (!pos) {
    logger.warn("Confirm position not defined");
    return false;
  }

  logger.info("Confirming action");
  await clickAt(pos.x, pos.y);
  await wait(300);

  return true;
}

/**
 * Cancel action
 * @returns {Promise<boolean>}
 */
export async function cancel() {
  if (!isSessionActive()) {
    throw new SessionDisconnectedError("Browser closed.");
  }

  const pos = getPosition("cancel");
  if (!pos) {
    logger.warn("Cancel position not defined");
    return false;
  }

  logger.info("Cancelling action");
  await clickAt(pos.x, pos.y);
  await wait(200);

  return true;
}

/**
 * Build a structure
 * @param {string} structureName - Structure name (barracks, house, etc.)
 * @param {object} options - Options
 * @returns {Promise<boolean>}
 */
export async function build(structureName, _options = {}) {
  if (!isSessionActive()) {
    throw new SessionDisconnectedError("Browser closed.");
  }

  logger.info(`Building structure: ${structureName}`);

  await openMenu("build");
  await selectItem(structureName);
  await confirm();

  logger.info(`Building ${structureName}`);
  return true;
}

/**
 * Train a unit
 * @param {string} unitName - Unit name (footman, archer, etc.)
 * @param {object} options - Options { count: 5, queue: 1 }
 * @returns {Promise<boolean>}
 */
export async function train(unitName, options = {}) {
  if (!isSessionActive()) {
    throw new SessionDisconnectedError("Browser closed.");
  }

  const { count = 1, queue: _queue = 1 } = options;

  logger.info(`Training unit: ${unitName} (count: ${count})`);

  await openMenu("build");
  await selectItem(unitName);

  if (count > 1) {
    for (let i = 1; i < count; i++) {
      await clickAt(
        menuConfig[unitName]?.x || 100,
        (menuConfig[unitName]?.y || 300) + i * 30,
      );
    }
  }

  await confirm();
  await wait(500);

  logger.info(`Training ${count} ${unitName}(s)`);
  return true;
}

/**
 * Research an upgrade
 * @param {string} building - Building name (blacksmith, etc.)
 * @param {string} upgrade - Upgrade name (weaponry, armor, etc.)
 * @returns {Promise<boolean>}
 */
export async function research(building, upgrade) {
  if (!isSessionActive()) {
    throw new SessionDisconnectedError("Browser closed.");
  }

  logger.info(`Researching ${upgrade} at ${building}`);

  await openMenu("research");
  await selectItem(building);
  await selectItem(upgrade);
  await confirm();

  logger.info(`Researching ${upgrade}`);
  return true;
}

/**
 * Generic menu sequence
 * @param {Array} sequence - Array of menu actions
 * @returns {Promise<boolean>}
 */
export async function runSequence(sequence) {
  if (!isSessionActive()) {
    throw new SessionDisconnectedError("Browser closed.");
  }

  logger.info(`Running menu sequence: ${sequence.length} steps`);

  for (const step of sequence) {
    const { action, target, position } = step;

    switch (action) {
      case "open":
        await openMenu(target);
        break;
      case "select":
        await selectItem(target);
        break;
      case "click":
        if (position) {
          await clickAt(position.x, position.y);
        }
        break;
      case "confirm":
        await confirm();
        break;
      case "cancel":
        await cancel();
        break;
      case "close":
        await closeMenu(position);
        break;
      case "wait":
        await wait(target);
        break;
      default:
        logger.warn(`Unknown action: ${action}`);
    }
  }

  logger.info("Menu sequence completed");
  return true;
}

/**
 * Set custom menu positions
 * @param {string} key - Menu item key
 * @param {object} position - { x, y }
 */
export function setPosition(key, position) {
  menuConfig[key] = position;
  logger.debug(`Set position for "${key}":`, position);
}

export default {
  loadConfig,
  openMenu,
  closeMenu,
  selectItem,
  confirm,
  cancel,
  build,
  train,
  research,
  runSequence,
  setPosition,
};
