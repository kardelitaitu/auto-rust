/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * Session Timeout Constants
 * Centralized timeout values for session management
 * @module constants/session-timeouts
 */

export const SESSION_TIMEOUTS = {
  /** Session timeout: 30 minutes */
  SESSION_TIMEOUT_MS: 30 * 60 * 1000,

  /** Cleanup interval: 5 minutes */
  CLEANUP_INTERVAL_MS: 5 * 60 * 1000,

  /** Worker wait timeout: 30 seconds */
  WORKER_WAIT_TIMEOUT_MS: 30000,

  /** Stuck worker threshold: 10 minutes */
  STUCK_WORKER_THRESHOLD_MS: 600000,

  /** Page close timeout: 5 seconds */
  PAGE_CLOSE_TIMEOUT_MS: 5000,

  /** Health check interval: 30 seconds */
  HEALTH_CHECK_INTERVAL_MS: 30000,
};

export function importSessionTimeouts(settings = {}) {
  return {
    ...SESSION_TIMEOUTS,
    ...settings.timeouts?.session,
  };
}

export default SESSION_TIMEOUTS;
