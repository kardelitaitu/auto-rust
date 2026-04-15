/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Barrel export for common utilities.
 * Re-exports frequently used utilities for convenient importing.
 *
 * Note: This is a selective barrel - only includes utilities that are:
 * 1. Pure functions with no side effects on import
 * 2. Not typically mocked in tests
 * 3. Safe from circular dependency issues
 *
 * For other utilities, import directly from their specific modules.
 *
 * @module api/utils
 */

// Math utilities (pure functions, widely used)
export { mathUtils } from "./math.js";

// Human timing utilities (pure functions, no side effects)
export { humanTiming } from "./timing.js";

// Locator utilities (pure functions for Playwright locators)
export { getLocator, stringify } from "./locator.js";

// Default export for convenience
import { mathUtils } from "./math.js";
import { humanTiming } from "./timing.js";

export default {
  mathUtils,
  humanTiming,
};
