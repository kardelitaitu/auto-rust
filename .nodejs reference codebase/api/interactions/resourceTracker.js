/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Resource Tracker
 * Watch for resource changes using screenshot diff + LLM
 * @module api/interactions/resourceTracker
 */

import { getPage, isSessionActive } from "../core/context.js";
import { createLogger } from "../core/logger.js";
import { llmClient } from "../agent/llmClient.js";
import { SessionDisconnectedError } from "../core/errors.js";

const logger = createLogger("api/interactions/resourceTracker.js");

let watchInterval = null;
let lastResourceState = null;

/**
 * Extract resource values from screenshot using LLM
 * @param {string} screenshot - Base64 screenshot
 * @param {object} regions - Resource regions { gold: {x,y,w,h}, ... }
 * @returns {Promise<object>}
 */
async function extractResources(screenshot, regions) {
  try {
    const regionDescriptions = Object.entries(regions)
      .map(
        ([name, region]) =>
          `${name}: region at (${region.x},${region.y}) size ${region.width}x${region.height}`,
      )
      .join(", ");

    const prompt = {
      role: "user",
      content: [
        {
          type: "text",
          text: `Extract resource values from this game UI screenshot.
                    
Regions to extract: ${regionDescriptions}

Return JSON with resource names as keys and numeric values:
{"gold": 500, "wood": 200, "food": 100}`,
        },
        {
          type: "image_url",
          image_url: { url: `data:image/jpeg;base64,${screenshot}` },
        },
      ],
    };

    const response = await llmClient.generateCompletion([prompt]);

    const jsonMatch = response.match(/\{[\s\S]*\}/);
    if (jsonMatch) {
      return JSON.parse(jsonMatch[0]);
    }

    return {};
  } catch (e) {
    logger.warn("Resource extraction failed:", e.message);
    return {};
  }
}

/**
 * Compare two resource states and return changes
 * @param {object} oldState
 * @param {object} newState
 * @returns {object}
 */
function computeChanges(oldState, newState) {
  const changes = {};

  for (const key of Object.keys(newState)) {
    const oldVal = oldState[key] || 0;
    const newVal = newState[key] || 0;
    const diff = newVal - oldVal;

    if (diff !== 0) {
      changes[key] = { from: oldVal, to: newVal, diff };
    }
  }

  return changes;
}

/**
 * Start watching resources
 * @param {object} options - Watch options
 * @returns {Promise<void>}
 */
export async function startWatch(options = {}) {
  if (!isSessionActive()) {
    throw new SessionDisconnectedError("Browser closed.");
  }

  const page = getPage();
  const {
    regions = {},
    interval = 1000,
    onChange = null,
    useLLM = true,
  } = options;

  if (watchInterval) {
    logger.warn("Resource watch already running. Stop first.");
    return;
  }

  logger.info("Starting resource watch with interval:", interval);

  let previousResources = null;

  watchInterval = setInterval(async () => {
    try {
      const buffer = await page.screenshot({ type: "jpeg", quality: 50 });
      const screenshot = buffer.toString("base64");

      let currentResources;

      if (useLLM && Object.keys(regions).length > 0) {
        currentResources = await extractResources(screenshot, regions);
      } else {
        currentResources = await extractAllTextResources(page);
      }

      if (previousResources) {
        const changes = computeChanges(previousResources, currentResources);

        if (Object.keys(changes).length > 0) {
          logger.debug("Resource changes:", changes);

          if (onChange) {
            onChange(changes, currentResources);
          }
        }
      }

      lastResourceState = currentResources;
      previousResources = currentResources;
    } catch (e) {
      logger.debug("Resource watch error:", e.message);
    }
  }, interval);
}

/**
 * Extract resources using text content (fallback)
 * @param {object} page - Playwright page
 * @returns {Promise<object>}
 */
async function extractAllTextResources(page) {
  const resources = {};

  const resourcePatterns = [
    {
      key: "gold",
      selectors: ['[class*="gold"]', '[id*="gold"]', ':has-text("Gold")]'],
    },
    {
      key: "wood",
      selectors: ['[class*="wood"]', '[id*="wood"]', ':has-text("Wood")]'],
    },
    {
      key: "food",
      selectors: ['[class*="food"]', '[id*="food"]', ':has-text("Food")]'],
    },
  ];

  for (const pattern of resourcePatterns) {
    for (const selector of pattern.selectors) {
      try {
        const locator = page.locator(selector).first();
        const text = await locator.textContent().catch(() => "");
        const match = text.match(/\d+/);
        if (match) {
          resources[pattern.key] = parseInt(match[0], 10);
          break;
        }
      } catch {
        // Ignore parsing errors for this resource pattern
      }
    }
  }

  return resources;
}

/**
 * Stop watching resources
 * @returns {void}
 */
export function stopWatch() {
  if (watchInterval) {
    clearInterval(watchInterval);
    watchInterval = null;
    logger.info("Resource watch stopped");
  }
}

/**
 * Wait for resources to reach target values
 * @param {object} targets - Target values { gold: 500, wood: 200 }
 * @param {object} options - Options
 * @returns {Promise<boolean>}
 */
export async function waitForResources(targets, options = {}) {
  if (!isSessionActive()) {
    throw new SessionDisconnectedError("Browser closed.");
  }

  const page = getPage();
  const { timeout = 30000, regions = {}, interval = 500 } = options;

  const startTime = Date.now();

  while (Date.now() - startTime < timeout) {
    try {
      const buffer = await page.screenshot({ type: "jpeg", quality: 50 });
      const screenshot = buffer.toString("base64");

      const current = await extractResources(screenshot, regions);

      let allMet = true;
      for (const [resource, required] of Object.entries(targets)) {
        const currentVal = current[resource] || 0;
        if (currentVal < required) {
          allMet = false;
          break;
        }
      }

      if (allMet) {
        logger.info(`Resource targets met: ${JSON.stringify(targets)}`);
        return true;
      }
    } catch (e) {
      logger.debug("waitForResources error:", e.message);
    }

    await new Promise((r) => setTimeout(r, interval));
  }

  logger.warn(`Resource targets not met within ${timeout}ms`);
  return false;
}

/**
 * Alert when resources available (throws error when ready)
 * @param {object} targets - Target values
 * @param {object} options - Options
 * @returns {Promise<void>}
 */
export async function alertWhenAvailable(targets, options = {}) {
  const ready = await waitForResources(targets, options);

  if (!ready) {
    throw new Error(`Resources not available: ${JSON.stringify(targets)}`);
  }
}

/**
 * Get current resource state
 * @returns {Promise<object>}
 */
export async function getCurrent() {
  if (!isSessionActive()) {
    throw new SessionDisconnectedError("Browser closed.");
  }

  if (lastResourceState) {
    return lastResourceState;
  }

  const page = getPage();
  const buffer = await page.screenshot({ type: "jpeg", quality: 50 });
  const screenshot = buffer.toString("base64");

  return await extractResources(screenshot, {});
}

export default {
  startWatch,
  stopWatch,
  waitForResources,
  alertWhenAvailable,
  getCurrent,
};
