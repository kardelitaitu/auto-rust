/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Action Engine - Executes JSON actions on the Browser (Safe Layer)
 * Supports clickAt, drag, multiSelect, press for game automation
 * @module api/agent/actionEngine
 */

import { createLogger } from "../core/logger.js";
import { GhostCursor } from "../utils/ghostCursor.js";
import { mathUtils } from "../utils/math.js";

const logger = createLogger("api/agent/actionEngine.js");

/**
 * @typedef {Object} ActionResult
 * @property {boolean} success - Whether the action succeeded
 * @property {boolean} done - Whether the agent task is complete
 * @property {string} error - Error message if failed
 */

/**
 * @typedef {Object} Action
 * @property {string} action - Action type: click, clickAt, type, press, scroll, navigate, wait, done, screenshot, drag, multiSelect
 * @property {string|number} [selector] - Element selector or ID
 * @property {string|number} [target] - Target for drag
 * @property {string} [value] - Value for type/navigate/scroll/wait
 * @property {string} [key] - Key for press action
 * @property {number} [x] - X coordinate for clickAt
 * @property {number} [y] - Y coordinate for clickAt
 * @property {string} [clickType] - Click type: single, double, long
 * @property {number} [duration] - Duration for drag or long_press in ms
 * @property {Array} [items] - Items for multiSelect
 * @property {string} [mode] - Mode for multiSelect (add, remove, range)
 */

class ActionEngine {
  /**
   * Create a new ActionEngine instance
   * @param {object} options - Configuration options
   */
  constructor(options = {}) {
    this.timeouts = {
      elementVisible: options.timeouts?.elementVisible || 5000,
      navigation: options.timeouts?.navigation || 30000,
      action: options.timeouts?.action || 10000,
    };

    this.humanization = {
      enabled: options.humanization?.enabled !== false,
      mouseMovement: options.humanization?.mouseMovement !== false,
      typingDelay: options.humanization?.typingDelay || { min: 30, max: 120 },
      hesitationChance: options.humanization?.hesitationChance || 0.1,
    };

    this.cursor = null; // Will be initialized per page
  }

  /**
   * Initialize cursor for a page
   * @param {object} page - Playwright page
   * @private
   */
  _initCursor(page) {
    if (!this.cursor) {
      this.cursor = new GhostCursor(page, logger);
    }
    return this.cursor;
  }

  /**
   * Executes a single JSON action on the page
   * @param {object} page - The Playwright page instance.
   * @param {Action} action - Action object
   * @param {string} [sessionId='unknown']
   * @returns {Promise<ActionResult>}
   */
  async execute(page, action, sessionId = "unknown") {
    if (!action || !action.action) {
      return { success: false, done: false, error: "No action specified" };
    }

    try {
      await page.bringToFront();
    } catch (_e) {
      /* ignore if already closed */
    }

    let target = action.selector || action.target || action.key;
    if (!target) {
      if (action.action === "done") target = "Task Completion";
      else if (action.action === "navigate" || action.action === "goto")
        target = "Page URL";
      else if (action.action === "wait" || action.action === "delay")
        target = "Timer";
      else if (action.action === "clickAt") {
        let x = action.x;
        let y = action.y;

        if (Array.isArray(x)) {
          if (x.length === 2 && !y) {
            y = x[1];
            x = x[0];
          } else {
            x = x[0];
          }
        }
        if (Array.isArray(y)) y = y[0];

        target = `(${x}, ${y})`;
      } else if (action.action === "drag") target = "Drag operation";
      else if (action.action === "multiSelect") target = "Multi-select";
      else target = "N/A";
    }

    if (action.rationale) {
      logger.info(`Agent Rationale: ${action.rationale}`);
    }

    logger.info(`Executing Action: ${action.action} on ${target}`);

    try {
      switch (action.action) {
        case "click":
          await this.performClick(page, action.selector);
          break;
        case "clickAt": {
          let cx = action.x;
          let cy = action.y;
          if (Array.isArray(cx)) {
            if (cx.length === 2 && !cy) {
              cy = cx[1];
              cx = cx[0];
            } else {
              cx = cx[0];
            }
          }
          if (Array.isArray(cy)) cy = cy[0];
          await this.performClickAt(
            page,
            cx,
            cy,
            action.clickType,
            action.duration,
          );
          break;
        }
        case "type":
          await this.performType(page, action.selector, action.value);
          break;
        case "press":
          await this.performPress(page, action.key || action.value);
          break;
        case "scroll":
          await this.performScroll(page, action.value);
          break;
        case "navigate":
        case "goto":
          await this.performNavigate(page, action.value);
          break;
        case "wait":
        case "delay":
          await this.performWait(page, action.value);
          break;
        case "drag":
          await this.performDrag(
            page,
            action.source,
            action.target,
            action.duration,
          );
          break;
        case "multiSelect":
          await this.performMultiSelect(page, action.items, action.mode);
          break;
        case "screenshot":
          await this.performScreenshot(page, sessionId);
          break;
        case "verify":
          // Verification is handled by the runner comparing states
          logger.info(
            `Verification step reached: ${action.description || "No description"}`,
          );
          break;
        case "done":
          logger.info("Agent indicates task completion.");
          return { done: true, success: true, error: "" };
        default:
          return {
            success: false,
            done: false,
            error: `Unknown action: ${action.action}`,
          };
      }
      return { success: true, done: false, error: "" };
    } catch (e) {
      logger.error(`Action Execution Failed: ${e.message}`);
      return { success: false, done: false, error: e.message };
    }
  }

  /**
   * Resolve a locator from a string selector
   * @param {object} page - Playwright page
   * @param {string} selector - Selector string
   * @returns {object} Playwright locator
   */
  getLocator(page, selector) {
    if (!selector || typeof selector !== "string") {
      throw new Error(
        `Invalid selector: "${selector}". Selector must be a non-empty string.`,
      );
    }

    // Defensive check for placeholders the model might mimic
    if (
      selector === "..." ||
      selector === "placeholder" ||
      selector === "N/A" ||
      selector.includes("-id")
    ) {
      throw new Error(
        `Model error: "${selector}" is a placeholder. Please use a real selector from the AXTree or screenshot.`,
      );
    }

    if (selector.startsWith("role=")) {
      const parts = selector.split(",").map((s) => s.trim());
      const rolePart = parts.find((p) => p.startsWith("role="));
      const namePart = parts.find((p) => p.startsWith("name="));

      if (rolePart) {
        const role = rolePart.split("=")[1];
        const options = {};
        if (namePart) {
          let name = namePart.split("=")[1];
          if (
            (name.startsWith('"') && name.endsWith('"')) ||
            (name.startsWith("'") && name.endsWith("'"))
          ) {
            name = name.slice(1, -1);
          }
          options.name = name;
        }
        return page.getByRole(role, options).first();
      }
    }

    if (selector.startsWith("text=")) {
      const text = selector.substring(5);
      return page.getByText(text).first();
    }

    return page.locator(selector).first();
  }

  /**
   * Click an element with humanized mouse movement
   * @param {object} page - Playwright page
   * @param {string} selector - Element selector
   */
  async performClick(page, selector) {
    const locator = this.getLocator(page, selector);
    await locator.waitFor({
      state: "visible",
      timeout: this.timeouts.elementVisible,
    });

    if (this.humanization.enabled && this.humanization.mouseMovement) {
      const cursor = this._initCursor(page);
      const box = await locator.boundingBox();

      if (box) {
        // Calculate target with Gaussian distribution for natural variation
        const marginX = box.width * 0.15;
        const marginY = box.height * 0.15;
        const targetX = mathUtils.gaussian(
          box.x + box.width / 2,
          box.width / 6,
          box.x + marginX,
          box.x + box.width - marginX,
        );
        const targetY = mathUtils.gaussian(
          box.y + box.height / 2,
          box.height / 6,
          box.y + marginY,
          box.y + box.height - marginY,
        );

        // Move with hesitation for natural feel
        await cursor.moveWithHesitation(targetX, targetY);

        // Small hesitation before click
        await page.waitForTimeout(mathUtils.randomInRange(50, 150));
      }
    }

    await locator.click();
  }

  /**
   * Type into an element with humanized keystroke timing
   * @param {object} page - Playwright page
   * @param {string} selector - Element selector
   * @param {string} value - Text to type
   */
  async performType(page, selector, value) {
    const locator = this.getLocator(page, selector);
    await locator.waitFor({
      state: "visible",
      timeout: this.timeouts.elementVisible,
    });

    // Humanized mouse movement to element
    if (this.humanization.enabled && this.humanization.mouseMovement) {
      const cursor = this._initCursor(page);
      const box = await locator.boundingBox();

      if (box) {
        const targetX = box.x + box.width / 2;
        const targetY = box.y + box.height / 2;
        await cursor.moveWithHesitation(targetX, targetY);
        await page.waitForTimeout(mathUtils.randomInRange(50, 150));
      }
    }

    await locator.click();

    // Humanized typing with variable delays
    if (this.humanization.enabled) {
      for (const char of value) {
        await page.keyboard.type(char);
        await page.waitForTimeout(
          mathUtils.randomInRange(
            this.humanization.typingDelay.min,
            this.humanization.typingDelay.max,
          ),
        );

        // Occasional longer pause (thinking)
        if (Math.random() < this.humanization.hesitationChance) {
          await page.waitForTimeout(mathUtils.randomInRange(100, 300));
        }
      }
    } else {
      await locator.fill(value);
    }
  }

  /**
   * Press a key
   * @param {object} page - Playwright page
   * @param {string} key - Key to press
   */
  async performPress(page, key) {
    if (!key) throw new Error("Key is required for press action");
    logger.info(`Pressing key: ${key}`);
    await page.keyboard.press(key);
  }

  /**
   * Scroll the page
   * @param {object} page - Playwright page
   * @param {string} directionOrValue - Direction: up, down, top, bottom or pixel value
   */
  async performScroll(page, directionOrValue) {
    if (directionOrValue === "down") {
      await page.evaluate(() => window.scrollBy(0, 500));
    } else if (directionOrValue === "up") {
      await page.evaluate(() => window.scrollBy(0, -500));
    } else if (directionOrValue === "bottom") {
      await page.evaluate(() => window.scrollTo(0, document.body.scrollHeight));
    } else if (directionOrValue === "top") {
      await page.evaluate(() => window.scrollTo(0, 0));
    } else if (directionOrValue === "done") {
      await page.evaluate(() => window.scrollTo(0, document.body.scrollHeight));
    }
  }

  /**
   * Navigate to a URL
   * @param {object} page - Playwright page
   * @param {string} url - URL to navigate to
   */
  async performNavigate(page, url) {
    if (!url) throw new Error("URL is required for navigate action");

    if (!/^https?:\/\//i.test(url)) {
      url = "https://" + url;
    }

    logger.info(`Navigating to: ${url}`);
    await page.goto(url, { waitUntil: "domcontentloaded", timeout: 30000 });
  }

  /**
   * Wait for a specified time
   * @param {object} page - Playwright page
   * @param {string} value - Wait time in milliseconds
   */
  async performWait(page, value) {
    const ms = parseInt(value, 10);
    if (isNaN(ms)) {
      throw new Error(`Invalid wait time: ${value}`);
    }
    logger.info(`Waiting for ${ms}ms...`);
    await page.waitForTimeout(ms);
  }

  /**
   * Take a screenshot
   * @param {object} page - Playwright page
   * @param {string} sessionId - Session ID for filename
   */
  async performScreenshot(page, sessionId) {
    logger.info("Taking screenshot...");
    const timestamp = Date.now();
    const filename = `screenshot-${sessionId}-${timestamp}.png`;
    await page.screenshot({ path: `./screenshot/${filename}` });
  }

  /**
   * Click at specific coordinates with humanized mouse movement
   * @param {object} page - Playwright page
   * @param {number} x - X coordinate
   * @param {number} y - Y coordinate
   * @param {string} clickType - single, double, long
   * @param {number} duration - Duration for long press
   */
  async performClickAt(page, x, y, clickType = "single", duration = 500) {
    if (x === undefined || y === undefined) {
      throw new Error("clickAt requires x and y coordinates");
    }

    // Safeguard against LLMs returning arrays like "x": [496, 312]
    const finalX = Array.isArray(x) ? x[0] : x;
    const finalY = Array.isArray(y) ? y[0] : y;

    if (typeof finalX !== "number" || typeof finalY !== "number") {
      throw new Error(
        `clickAt requires numeric coordinates. Got x: ${typeof finalX}, y: ${typeof finalY}`,
      );
    }

    logger.info(`Clicking (${clickType}) at (${finalX}, ${finalY})`);

    // Humanized mouse movement before click
    if (this.humanization.enabled && this.humanization.mouseMovement) {
      const cursor = this._initCursor(page);
      await cursor.moveWithHesitation(finalX, finalY);
      await page.waitForTimeout(mathUtils.randomInRange(50, 150));
    }

    if (clickType === "double") {
      await page.mouse.dblclick(finalX, finalY);
    } else if (clickType === "long") {
      await page.mouse.down();
      await page.waitForTimeout(duration || 100);
      await page.mouse.up();
    } else {
      await page.mouse.click(finalX, finalY);
    }
  }

  /**
   * Perform drag operation
   * @param {object} page - Playwright page
   * @param {object|number} source - Source selector or element ID
   * @param {object|number} target - Target selector or element ID
   * @param {number} duration - Duration in ms
   */
  async performDrag(page, source, target, duration = 500) {
    logger.info(
      `Dragging from ${JSON.stringify(source)} to ${JSON.stringify(target)}`,
    );

    const startCoords = await this._resolveCoords(page, source);
    const endCoords = await this._resolveCoords(page, target);

    await page.mouse.move(startCoords.x, startCoords.y);
    await page.mouse.down();
    await page.waitForTimeout(150);

    const steps = Math.max(3, Math.floor(duration / 15));
    for (let i = 1; i <= steps; i++) {
      const t = i / steps;
      const x = startCoords.x + (endCoords.x - startCoords.x) * t;
      const y = startCoords.y + (endCoords.y - startCoords.y) * t;
      await page.mouse.move(x, y);
    }

    await page.mouse.up();
    logger.info("Drag completed");
  }

  /**
   * Perform multi-select
   * @param {object} page - Playwright page
   * @param {Array} items - Items to select
   * @param {string} mode - Selection mode: add, remove, range
   */
  async performMultiSelect(page, items, mode = "add") {
    if (!items || !Array.isArray(items)) {
      throw new Error("multiSelect requires an array of items");
    }
    logger.info(`Multi-selecting ${items.length} items in mode: ${mode}`);

    const useCtrl = mode === "add" || mode === "remove";

    if (mode === "range" && items.length >= 2) {
      const startCoords = await this._resolveCoords(page, items[0]);
      const endCoords = await this._resolveCoords(
        page,
        items[items.length - 1],
      );

      await page.mouse.move(startCoords.x, startCoords.y);
      await page.mouse.down();
      await page.keyboard.down("Shift");
      await page.mouse.move(endCoords.x, endCoords.y);
      await page.mouse.up();
      await page.keyboard.up("Shift");
    } else {
      if (useCtrl) {
        await page.keyboard.down("Control");
      }

      for (const item of items) {
        const coords = await this._resolveCoords(page, item);
        await page.mouse.move(coords.x, coords.y);
        await page.mouse.down();
        await page.mouse.up();
      }

      if (useCtrl) {
        await page.keyboard.up("Control");
      }
    }
  }

  /**
   * Resolve element or coordinates
   * @private
   */
  async _resolveCoords(page, input) {
    if (!input) {
      throw new Error("Input is required");
    }

    if (typeof input === "object" && "x" in input && "y" in input) {
      return input;
    }

    if (typeof input === "number") {
      const { getStateAgentElementMap } =
        await import("../core/context-state.js");
      const elementMap = getStateAgentElementMap();
      const element = elementMap.find((el) => el.id === input);
      if (!element) {
        throw new Error(`Element with ID ${input} not found`);
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

    throw new Error("Invalid input for coordinates");
  }
}

const actionEngine = new ActionEngine();

export { actionEngine };
export default actionEngine;
