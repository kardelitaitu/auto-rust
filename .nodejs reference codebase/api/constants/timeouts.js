/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Consolidated Timeout Constants
 * Barrel file that re-exports all timeout constants from domain-specific files.
 * Use this for importing multiple timeout categories or accessing all timeouts.
 *
 * @module constants/timeouts
 */

// Re-export domain-specific timeouts
export { SESSION_TIMEOUTS, importSessionTimeouts } from "./session-timeouts.js";

export { TWITTER_TIMEOUTS, importTimeouts } from "./twitter-timeouts.js";

// Import for consolidated export
import { SESSION_TIMEOUTS } from "./session-timeouts.js";
import { TWITTER_TIMEOUTS } from "./twitter-timeouts.js";

/**
 * Combined timeout constants for all domains.
 * Useful when you need access to multiple timeout categories.
 */
export const ALL_TIMEOUTS = {
  session: SESSION_TIMEOUTS,
  twitter: TWITTER_TIMEOUTS,
};

/**
 * Import all timeouts with settings overrides.
 * @param {Object} settings - Settings object with timeout overrides
 * @returns {Object} Combined timeout configuration
 */
export function importAllTimeouts(settings = {}) {
  return {
    session: {
      ...SESSION_TIMEOUTS,
      ...settings.timeouts?.session,
    },
    twitter: {
      ...TWITTER_TIMEOUTS,
      ...settings.timeouts?.twitter,
    },
  };
}

export default ALL_TIMEOUTS;
