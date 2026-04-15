/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Click at Coordinates
 * Raw coordinate clicking for game canvas elements and dynamically positioned targets.
 * @module api/interactions/clickAt
 */

import { getPage, getCursor, isSessionActive } from "../core/context.js";
import { randomInRange as _randomInRange } from "../behaviors/timing.js";
import { getPersona } from "../behaviors/persona.js";
import { createLogger } from "../core/logger.js";
import { mathUtils } from "../utils/math.js";
import { createPipeline, retryMiddleware } from "../core/middleware.js";
import { SessionDisconnectedError } from "../core/errors.js";
import visualDebug from "../utils/visual-debug.js";

const logger = createLogger("api/interactions/clickAt.js");

/**
 * @typedef {Object} ClickAtOptions
 * @property {'left'|'right'|'middle'} [button='left'] - Mouse button
 * @property {boolean} [moveToFirst=true] - Move cursor to position before clicking
 * @property {boolean} [humanPath=true] - Use human-like movement path
 * @property {'exact'|'safe'|'rough'} [precision='safe'] - Click precision
 * @property {number} [hoverMs=0] - Hover duration before clicking
 * @property {boolean} [recovery=true] - Auto-retry on failure
 * @property {number} [maxRetries=3] - Max retry attempts
 * @property {'fast'|'normal'|'slow'} [speed='normal'] - Movement and click speed
 */

/**
 * Apply bezier curve for human-like movement
 * @param {object} cursor - GhostCursor instance
 * @param {number} startX
 * @param {number} startY
 * @param {number} targetX
 * @param {number} targetY
 * @param {number} steps
 */
async function moveWithPath(cursor, startX, startY, targetX, targetY, steps) {
  const controlX = (startX + targetX) / 2;
  const controlY = Math.min(startY, targetY) - Math.abs(targetX - startX) * 0.2;

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
      t3 * targetX;
    const y =
      mt3 * startY +
      3 * mt2 * t * controlY +
      3 * mt * t2 * controlY +
      t3 * targetY;

    await cursor.move(x, y);
  }
}

/**
 * Human-like click at specific viewport coordinates.
 * Useful for game canvas elements or dynamically positioned elements without stable selectors.
 *
 * @param {number} x - Viewport X coordinate
 * @param {number} y - Viewport Y coordinate
 * @param {ClickAtOptions} options - Click configuration
 * @returns {Promise<{success: boolean}>}
 *
 * @example
 * // Basic click at coordinates
 * await api.clickAt(450, 320);
 *
 * // Right-click
 * await api.clickAt(450, 320, { button: 'right' });
 *
 * // Click without moving cursor first (instant)
 * await api.clickAt(450, 320, { moveToFirst: false });
 *
 * // High precision click
 * await api.clickAt(450, 320, { precision: 'exact', hoverMs: 200 });
 */
export async function clickAt(x, y, options = {}) {
  if (!isSessionActive()) {
    throw new SessionDisconnectedError("Browser closed before clickAt.");
  }

  const page = getPage();
  const cursor = getCursor();
  const persona = getPersona();

  const {
    button = "left",
    moveToFirst = true,
    humanPath = true,
    precision = "safe",
    hoverMs = 0,
    recovery = true,
    maxRetries = 3,
    speed = "normal", // 'fast', 'normal', 'slow'
  } = options;

  // Speed multipliers
  const speedConfig = {
    fast: { moveMultiplier: 0.3, clickDelay: [10, 30], disableHumanPath: true },
    normal: {
      moveMultiplier: 1.0,
      clickDelay: [50, 150],
      disableHumanPath: false,
    },
    slow: {
      moveMultiplier: 2.0,
      clickDelay: [100, 300],
      disableHumanPath: false,
    },
  };
  const speedSettings = speedConfig[speed] || speedConfig.normal;

  if (typeof x !== "number" || typeof y !== "number") {
    throw new Error("clickAt requires numeric x and y coordinates");
  }

  const pipeline = createPipeline(
    retryMiddleware({ maxRetries: recovery ? maxRetries : 0 }),
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

      if (x < 0 || x > viewport.width || y < 0 || y > viewport.height) {
        throw new Error(
          `Coordinates (${x}, ${y}) outside viewport (${viewport.width}x${viewport.height})`,
        );
      }

      logger.info(`Clicking at (${x}, ${y}) with button: ${button}`);

      let targetX = x;
      let targetY = y;

      if (precision === "rough") {
        targetX = x + mathUtils.randomInRange(-10, 10);
        targetY = y + mathUtils.randomInRange(-10, 10);
      } else if (precision === "safe") {
        const personaPrecision = persona.precision ?? 0.8;
        targetX = x + mathUtils.randomInRange(-3, 3) * (1 - personaPrecision);
        targetY = y + mathUtils.randomInRange(-3, 3) * (1 - personaPrecision);
      }

      if (moveToFirst) {
        const startPos = cursor.previousPos || {
          x: viewport.width / 2,
          y: viewport.height / 2,
        };
        const useHumanPath = humanPath && !speedSettings.disableHumanPath;

        if (useHumanPath) {
          const distance = Math.sqrt(
            Math.pow(targetX - startPos.x, 2) +
              Math.pow(targetY - startPos.y, 2),
          );
          const baseDuration =
            (100 + 0.3 * distance) * speedSettings.moveMultiplier;
          const steps = Math.max(2, Math.floor(baseDuration / 20));

          await moveWithPath(
            cursor,
            startPos.x,
            startPos.y,
            targetX,
            targetY,
            steps,
          );
        } else {
          await cursor.move(targetX, targetY);
        }
      }

      if (hoverMs > 0) {
        await page.waitForTimeout(hoverMs);
      }

      // Perform click using Playwright's native mouse
      await page.mouse.move(targetX, targetY);

      // Update debug cursor position after move
      if (visualDebug.isEnabled()) {
        await visualDebug.moveCursor(Math.round(targetX), Math.round(targetY));
      }

      await page.mouse.down({ button });
      await page.waitForTimeout(
        mathUtils.randomInRange(
          speedSettings.clickDelay[0],
          speedSettings.clickDelay[1],
        ),
      );
      await page.mouse.up({ button });

      cursor.previousPos = { x: targetX, y: targetY };

      logger.info(
        `ClickAt completed at (${Math.round(targetX)}, ${Math.round(targetY)})`,
      );

      // Visual debug marker (only if enabled)
      if (visualDebug.isEnabled()) {
        await visualDebug.mark(
          Math.round(targetX),
          Math.round(targetY),
          "CLICK",
        );
      }

      return { success: true };
    },
    { action: "clickAt", x, y, options },
  );
}

export default clickAt;
