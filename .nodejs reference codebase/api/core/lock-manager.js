/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 */

/**
 * @fileoverview Lock Manager - Thread-safe locking for session operations
 * Prevents race conditions in concurrent session access
 * @module core/lock-manager
 */

import { createLogger } from "../core/logger.js";

const logger = createLogger("lock-manager.js");

/**
 * @class LockManager
 * @description Provides async locking mechanism to prevent race conditions
 */
class LockManager {
  constructor() {
    this._locks = new Map();
  }

  /**
   * Acquire a lock
   * @param {string} key - Lock identifier
   * @returns {Promise<{release: Function}>} Release function
   */
  async acquire(key) {
    while (this._locks.has(key)) {
      const waitPromise = this._locks.get(key);
      await waitPromise;
    }

    let releaseFn;
    const waitPromise = new Promise((resolve) => {
      releaseFn = resolve;
    });

    this._locks.set(key, waitPromise);

    return {
      release: async () => {
        this._locks.delete(key);
        releaseFn();
      },
    };
  }

  /**
   * Check if a lock is held
   * @param {string} key - Lock identifier
   * @returns {boolean}
   */
  isLocked(key) {
    return this._locks.has(key);
  }

  /**
   * Release all locks - for cleanup
   */
  async releaseAll() {
    const promises = Array.from(this._locks.values());
    for (const promise of promises) {
      promise.then((release) => release());
    }
    this._locks.clear();
    logger.debug("All locks released");
  }
}

export default LockManager;
