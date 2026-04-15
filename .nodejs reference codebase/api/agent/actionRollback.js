/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Action Rollback System
 * Captures pre-action state and allows rollback on critical failures
 * @module api/agent/actionRollback
 */

import { createLogger } from "../core/logger.js";

const logger = createLogger("api/agent/actionRollback.js");

class ActionRollback {
  constructor() {
    this.actionHistory = [];
    this.maxHistory = 50;
    this.criticalActions = new Set(["navigate", "click", "type", "drag"]);
  }

  /**
   * Capture current page state for potential rollback
   * @param {object} page - Playwright page
   * @param {object} action - Action being executed
   * @returns {Promise<object>} Pre-action state
   */
  async capturePreState(page, action) {
    try {
      const state = {
        url: page.url(),
        timestamp: Date.now(),
        action: action,
        screenshot: null,
        scrollPosition: null,
        formValues: null,
      };

      // Capture screenshot
      try {
        state.screenshot = await page.screenshot({
          type: "jpeg",
          quality: 80,
          fullPage: false,
        });
      } catch (e) {
        logger.debug("Failed to capture screenshot for rollback:", e.message);
      }

      // Capture scroll position
      try {
        state.scrollPosition = await page.evaluate(() => ({
          x: window.scrollX,
          y: window.scrollY,
        }));
      } catch (e) {
        logger.debug("Failed to capture scroll position:", e.message);
      }

      // Capture form values for type actions
      if (action.action === "type" && action.selector) {
        try {
          const value = await page.evaluate((selector) => {
            const el = document.querySelector(selector);
            return el ? el.value : null;
          }, action.selector);
          state.formValue = value;
        } catch (e) {
          logger.debug("Failed to capture form value:", e.message);
        }
      }

      return state;
    } catch (error) {
      logger.error("Failed to capture pre-state:", error.message);
      return null;
    }
  }

  /**
   * Record action with pre-state for potential rollback
   * @param {object} preState - Pre-action state
   * @param {object} action - Action that was executed
   * @param {object} result - Action result
   */
  recordAction(preState, action, result) {
    if (!preState) return;

    const record = {
      preState,
      action,
      result,
      timestamp: Date.now(),
    };

    this.actionHistory.push(record);

    // Trim history if over limit
    if (this.actionHistory.length > this.maxHistory) {
      this.actionHistory.shift();
    }

    logger.debug(
      `[Rollback] Recorded action: ${action.action} (history: ${this.actionHistory.length})`,
    );
  }

  /**
   * Check if action is critical and might need rollback
   * @param {object} action - Action to check
   * @returns {boolean} True if action is critical
   */
  isCriticalAction(action) {
    return this.criticalActions.has(action.action);
  }

  /**
   * Rollback the last action
   * @param {object} page - Playwright page
   * @returns {Promise<boolean>} True if rollback succeeded
   */
  async rollbackLast(page) {
    if (this.actionHistory.length === 0) {
      logger.warn("[Rollback] No actions to rollback");
      return false;
    }

    const lastRecord = this.actionHistory.pop();
    logger.info(`[Rollback] Rolling back action: ${lastRecord.action.action}`);

    try {
      // Restore URL if changed
      if (lastRecord.preState.url && lastRecord.preState.url !== page.url()) {
        logger.info(`[Rollback] Restoring URL: ${lastRecord.preState.url}`);
        await page.goto(lastRecord.preState.url, {
          waitUntil: "domcontentloaded",
        });
      }

      // Restore scroll position
      if (lastRecord.preState.scrollPosition) {
        await page.evaluate((pos) => {
          window.scrollTo(pos.x, pos.y);
        }, lastRecord.preState.scrollPosition);
        logger.debug("[Rollback] Restored scroll position");
      }

      // Restore form value for type actions
      if (
        lastRecord.action.action === "type" &&
        lastRecord.preState.formValue !== null
      ) {
        try {
          await page.evaluate(
            (selector, value) => {
              const el = document.querySelector(selector);
              if (el) el.value = value;
            },
            lastRecord.action.selector,
            lastRecord.preState.formValue,
          );
          logger.debug("[Rollback] Restored form value");
        } catch (e) {
          logger.debug("Failed to restore form value:", e.message);
        }
      }

      logger.info("[Rollback] Rollback completed successfully");
      return true;
    } catch (error) {
      logger.error("[Rollback] Rollback failed:", error.message);
      return false;
    }
  }

  /**
   * Rollback multiple actions
   * @param {object} page - Playwright page
   * @param {number} count - Number of actions to rollback
   * @returns {Promise<number>} Number of successfully rolled back actions
   */
  async rollbackMultiple(page, count) {
    let rolledBack = 0;

    for (let i = 0; i < count && this.actionHistory.length > 0; i++) {
      const success = await this.rollbackLast(page);
      if (success) {
        rolledBack++;
      } else {
        break; // Stop on first failure
      }
    }

    logger.info(`[Rollback] Rolled back ${rolledBack}/${count} actions`);
    return rolledBack;
  }

  /**
   * Get rollback history
   * @returns {Array} Action history
   */
  getHistory() {
    return [...this.actionHistory];
  }

  /**
   * Clear rollback history
   */
  clearHistory() {
    this.actionHistory = [];
    logger.info("[Rollback] History cleared");
  }

  /**
   * Get statistics
   * @returns {object} Rollback statistics
   */
  getStats() {
    return {
      historySize: this.actionHistory.length,
      maxHistory: this.maxHistory,
      criticalActions: Array.from(this.criticalActions),
    };
  }
}

const actionRollback = new ActionRollback();

export { actionRollback };
export default actionRollback;
