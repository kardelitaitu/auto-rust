/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 */

/**
 * @fileoverview Interval Manager - Guaranteed interval cleanup
 * Prevents interval leaks after session closure
 * @module core/interval-manager
 */

import { createLogger } from "../core/logger.js";

const logger = createLogger("interval-manager.js");

/**
 * @class IntervalManager
 * @description Manages session-bound intervals with guaranteed cleanup
 */
class IntervalManager {
  constructor() {
    this._intervals = new Map();
  }

  /**
   * Set an interval
   * @param {string} name - Interval identifier
   * @param {Function} fn - Function to execute
   * @param {number} ms - Delay in milliseconds
   * @param {object} options - Additional options
   * @returns {number} Interval ID
   */
  set(name, fn, ms, options = {}) {
    const id = setInterval(async () => {
      try {
        if (!options.continueOnClosed) {
          return;
        }
        await fn();
      } catch (e) {
        logger.debug(`[Interval ${name}] Error:`, e.message);
      }
    }, ms);

    this._intervals.set(name, id);
    logger.debug(`[IntervalManager] Set interval: ${name}`);
    return id;
  }

  /**
   * Clear a specific interval
   * @param {string} name - Interval identifier
   */
  clear(name) {
    const id = this._intervals.get(name);
    if (id) {
      clearInterval(id);
      this._intervals.delete(name);
      logger.debug(`[IntervalManager] Cleared interval: ${name}`);
    }
  }

  /**
   * Check if interval exists
   * @param {string} name - Interval identifier
   * @returns {boolean}
   */
  has(name) {
    return this._intervals.has(name);
  }

  /**
   * Get all interval names
   * @returns {string[]}
   */
  keys() {
    return Array.from(this._intervals.keys());
  }

  /**
   * Clear all intervals - MUST be called on session close
   * @returns {Promise<void>}
   */
  async clearAll() {
    const names = Array.from(this._intervals.keys());
    for (const name of names) {
      const id = this._intervals.get(name);
      if (id) {
        clearInterval(id);
        logger.debug(`[IntervalManager] Cleared: ${name}`);
      }
    }
    this._intervals.clear();
    logger.debug("[IntervalManager] All intervals cleared");
  }

  /**
   * Get interval count
   * @returns {number}
   */
  size() {
    return this._intervals.size;
  }
}

export default IntervalManager;
