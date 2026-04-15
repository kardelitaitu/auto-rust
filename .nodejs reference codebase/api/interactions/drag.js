/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Drag Interaction
 * Supports dragging from selector/coordinates to selector/coordinates with configurable duration.
 * @module api/interactions/drag
 */

import { getPage, getCursor, isSessionActive } from "../core/context.js";
import { randomInRange } from "../behaviors/timing.js";
import { focus as _focus } from "./scroll.js";
import { wait } from "./wait.js";
import { getPersona } from "../behaviors/persona.js";
import { createLogger } from "../core/logger.js";
import { mathUtils } from "../utils/math.js";
import { getLocator as _getLocator, stringify } from "../utils/locator.js";
import {
  createPipeline,
  retryMiddleware,
  recoveryMiddleware,
} from "../core/middleware.js";
import {
  ElementObscuredError as _ElementObscuredError,
  SessionDisconnectedError,
} from "../core/errors.js";

const logger = createLogger("api/interactions/drag.js");

/**
 * @typedef {Object} DragOptions
 * @property {number} [durationMs=300] - Total drag duration from start to end
 * @property {number} [holdMs=150] - Time to hold before dragging
 * @property {'direct'|'bezier'|'arc'} [movement='bezier'] - Path style
 * @property {'left'|'right'} [button='left'] - Mouse button
 * @property {boolean} [recovery=true] - Auto-retry on failure
 * @property {number} [maxRetries=3] - Max retry attempts
 */

/**
 * Parse source/target to coordinates
 * @param {string|{x:number,y:number}|number} input - Selector, coords, or element ID
 * @param {object} page - Playwright page
 * @returns {Promise<{x:number,y:number}>}
 */
async function resolveToCoords(input, page) {
  if (!input) {
    throw new Error("Drag source/target is required");
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
    "Invalid drag source/target. Use selector string, {x, y}, or element ID",
  );
}

/**
 * Check if point is within viewport
 * @param {number} x
 * @param {number} y
 * @param {object} viewport
 * @returns {boolean}
 */
function isInViewport(x, y, viewport) {
  return x >= 0 && x <= viewport.width && y >= 0 && y <= viewport.height;
}

/**
 * Apply bezier curve interpolation for smooth movement
 * @param {object} cursor - GhostCursor instance
 * @param {number} startX
 * @param {number} startY
 * @param {number} endX
 * @param {number} endY
 * @param {number} steps - Number of intermediate points
 */
async function moveBezier(cursor, startX, startY, endX, endY, steps) {
  const controlX = (startX + endX) / 2;
  const controlY = Math.min(startY, endY) - Math.abs(endX - startX) * 0.3;

  for (let i = 1; i <= steps; i++) {
    const t = i / steps;
    const t2 = t * t;
    const t3 = t2 * t;
    const mt = 1 - t;
    const mt2 = mt * mt;
    const mt3 = mt2 * mt;

    const x =
      mt3 * startX +
      3 * mt2 * t * controlX +
      3 * mt * t2 * controlX +
      t3 * endX;
    const y =
      mt3 * startY +
      3 * mt2 * t * controlY +
      3 * mt * t2 * controlY +
      t3 * endY;

    await cursor.move(x, y);
  }
}

/**
 * Apply arc movement
 * @param {object} cursor - GhostCursor instance
 * @param {number} startX
 * @param {number} startY
 * @param {number} endX
 * @param {number} endY
 * @param {number} steps - Number of intermediate points
 */
async function moveArc(cursor, startX, startY, endX, endY, steps) {
  const midX = (startX + endX) / 2;
  const distance = Math.sqrt(
    Math.pow(endX - startX, 2) + Math.pow(endY - startY, 2),
  );
  const midY = Math.min(startY, endY) - distance * 0.2;

  for (let i = 1; i <= steps; i++) {
    const t = i / steps;
    const x =
      (1 - t) * (1 - t) * startX + 2 * (1 - t) * t * midX + t * t * endX;
    const y =
      (1 - t) * (1 - t) * startY + 2 * (1 - t) * t * midY + t * t * endY;
    await cursor.move(x, y);
  }
}

/**
 * Direct linear movement
 * @param {object} cursor - GhostCursor instance
 * @param {number} startX
 * @param {number} startY
 * @param {number} endX
 * @param {number} endY
 * @param {number} steps - Number of intermediate points
 */
async function moveDirect(cursor, startX, startY, endX, endY, steps) {
  for (let i = 1; i <= steps; i++) {
    const t = i / steps;
    const x = startX + (endX - startX) * t;
    const y = startY + (endY - startY) * t;
    await cursor.move(x, y);
  }
}

/**
 * Human-like drag operation from source to target.
 *
 * @param {string|{x:number,y:number}|number} source - Selector, coordinates, or element ID
 * @param {string|{x:number,y:number}|number} target - Selector, coordinates, or element ID
 * @param {DragOptions} options - Drag configuration
 * @returns {Promise<{success: boolean, usedFallback: boolean}>}
 *
 * @example
 * // Drag from selector to selector
 * await api.drag('.unit-card', '.troop-queue', { durationMs: 800 });
 *
 * // Drag from coordinates to coordinates
 * await api.drag({x: 100, y: 200}, {x: 500, y: 400}, { durationMs: 500, movement: 'direct' });
 *
 * // Using element IDs from api.agent.see()
 * await api.drag(1, 2, { durationMs: 300 });
 */
export async function drag(source, target, options = {}) {
  if (!isSessionActive()) {
    throw new SessionDisconnectedError("Browser closed before drag.");
  }

  const page = getPage();
  const cursor = getCursor();
  const persona = getPersona();

  const {
    durationMs = 300,
    holdMs = 150,
    movement = "bezier",
    button = "left",
    recovery = true,
    maxRetries = 3,
  } = options;

  const pipeline = createPipeline(
    retryMiddleware({ maxRetries: recovery ? maxRetries : 0 }),
    recoveryMiddleware({ scrollOnDetached: recovery }),
  );

  return await pipeline(
    async () => {
      // Get actual viewport - page.viewportSize() may return null, use window dimensions as fallback
      let viewport = page.viewportSize();
      if (!viewport) {
        const jsSize = await page.evaluate(() => ({
          width: window.innerWidth,
          height: window.innerHeight,
        }));
        viewport = jsSize;
      }

      logger.info(
        `Dragging from ${stringify(source)} to ${stringify(target)} (${durationMs}ms)`,
      );

      const startCoords = await resolveToCoords(source, page);
      const endCoords = await resolveToCoords(target, page);

      if (!isInViewport(startCoords.x, startCoords.y, viewport)) {
        throw new Error(
          `Start coordinates (${startCoords.x}, ${startCoords.y}) outside viewport`,
        );
      }
      if (!isInViewport(endCoords.x, endCoords.y, viewport)) {
        throw new Error(
          `End coordinates (${endCoords.x}, ${endCoords.y}) outside viewport`,
        );
      }

      const precision = persona.precision ?? 0.8;
      const jitterX = mathUtils.randomInRange(-3, 3) * (1 - precision);
      const jitterY = mathUtils.randomInRange(-3, 3) * (1 - precision);
      const targetX = endCoords.x + jitterX;
      const targetY = endCoords.y + jitterY;

      await cursor.move(startCoords.x, startCoords.y);
      await wait(randomInRange(100, 300));

      await page.mouse.down({ button });
      await wait(holdMs);

      const steps = Math.max(3, Math.floor(durationMs / 15));

      switch (movement) {
        case "direct":
          await moveDirect(
            cursor,
            startCoords.x,
            startCoords.y,
            targetX,
            targetY,
            steps,
          );
          break;
        case "arc":
          await moveArc(
            cursor,
            startCoords.x,
            startCoords.y,
            targetX,
            targetY,
            steps,
          );
          break;
        case "bezier":
        default:
          await moveBezier(
            cursor,
            startCoords.x,
            startCoords.y,
            targetX,
            targetY,
            steps,
          );
          break;
      }

      await page.mouse.up({ button });

      cursor.previousPos = { x: targetX, y: targetY };

      await wait(randomInRange(100, 300));

      logger.info(
        `Drag completed: (${Math.round(startCoords.x)},${Math.round(startCoords.y)}) → (${Math.round(targetX)},${Math.round(targetY)})`,
      );

      return { success: true };
    },
    { action: "drag", source, target, options },
  );
}

export default drag;
