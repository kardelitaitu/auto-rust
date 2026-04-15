/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Progress Tracker for real-time agent monitoring
 * Provides WebSocket events for dashboard integration
 * @module api/agent/progressTracker
 */

import { createLogger } from "../core/logger.js";

const logger = createLogger("api/agent/progressTracker.js");

class ProgressTracker {
  constructor() {
    this.wsServer = null;
    this.metrics = {
      stepsCompleted: 0,
      actionsAttempted: 0,
      actionsSucceeded: 0,
      actionsFailed: 0,
      llmCalls: 0,
      llmFailures: 0,
      errors: 0,
      startTime: null,
      currentGoal: null,
      currentUrl: null,
    };
    this.listeners = new Set();
  }

  /**
   * Initialize with WebSocket server
   * @param {object} wsServer - WebSocket server instance
   */
  init(wsServer) {
    this.wsServer = wsServer;
    logger.info("[ProgressTracker] Initialized with WebSocket server");
  }

  /**
   * Add event listener
   * @param {function} listener - Event listener function
   */
  addListener(listener) {
    this.listeners.add(listener);
  }

  /**
   * Remove event listener
   * @param {function} listener - Event listener function
   */
  removeListener(listener) {
    this.listeners.delete(listener);
  }

  /**
   * Start tracking a new session
   * @param {string} goal - The goal being pursued
   */
  startSession(goal) {
    this.metrics = {
      stepsCompleted: 0,
      actionsAttempted: 0,
      actionsSucceeded: 0,
      actionsFailed: 0,
      llmCalls: 0,
      llmFailures: 0,
      errors: 0,
      startTime: Date.now(),
      currentGoal: goal,
      currentUrl: null,
    };

    this._emit("session:start", {
      goal,
      timestamp: Date.now(),
    });

    logger.info(`[ProgressTracker] Session started: "${goal}"`);
  }

  /**
   * Update current URL
   * @param {string} url - Current page URL
   */
  updateUrl(url) {
    this.metrics.currentUrl = url;
    this._emit("url:change", { url, timestamp: Date.now() });
  }

  /**
   * Record step completion
   * @param {number} stepNumber - Current step number
   * @param {object} action - Action that was executed
   * @param {object} result - Action result
   */
  recordStep(stepNumber, action, result) {
    this.metrics.stepsCompleted = stepNumber;
    this.metrics.actionsAttempted++;

    if (result.success) {
      this.metrics.actionsSucceeded++;
    } else {
      this.metrics.actionsFailed++;
    }

    const elapsed = Date.now() - this.metrics.startTime;

    this._emit("step:complete", {
      step: stepNumber,
      action: action.action,
      success: result.success,
      error: result.error,
      metrics: { ...this.metrics },
      elapsed,
      timestamp: Date.now(),
    });

    logger.debug(
      `[ProgressTracker] Step ${stepNumber}: ${action.action} - ${result.success ? "SUCCESS" : "FAILED"}`,
    );
  }

  /**
   * Record LLM call
   * @param {boolean} success - Whether LLM call succeeded
   * @param {number} duration - Call duration in ms
   */
  recordLlmCall(success, duration) {
    this.metrics.llmCalls++;
    if (!success) {
      this.metrics.llmFailures++;
    }

    this._emit("llm:call", {
      success,
      duration,
      totalCalls: this.metrics.llmCalls,
      failureRate:
        this.metrics.llmCalls > 0
          ? this.metrics.llmFailures / this.metrics.llmCalls
          : 0,
      timestamp: Date.now(),
    });
  }

  /**
   * Record error
   * @param {string} type - Error type
   * @param {string} message - Error message
   */
  recordError(type, message) {
    this.metrics.errors++;

    this._emit("error", {
      type,
      message,
      totalErrors: this.metrics.errors,
      timestamp: Date.now(),
    });

    logger.warn(`[ProgressTracker] Error: ${type} - ${message}`);
  }

  /**
   * Record stuck detection
   * @param {number} stepNumber - Current step
   * @param {string} reason - Stuck reason
   */
  recordStuck(stepNumber, reason) {
    this._emit("agent:stuck", {
      step: stepNumber,
      reason,
      metrics: { ...this.metrics },
      timestamp: Date.now(),
    });

    logger.warn(
      `[ProgressTracker] Agent stuck at step ${stepNumber}: ${reason}`,
    );
  }

  /**
   * Complete session tracking
   * @param {boolean} success - Whether session succeeded
   * @param {string} reason - Completion reason
   */
  completeSession(success, reason) {
    const duration = Date.now() - this.metrics.startTime;

    this._emit("session:complete", {
      success,
      reason,
      metrics: { ...this.metrics },
      duration,
      timestamp: Date.now(),
    });

    logger.info(
      `[ProgressTracker] Session ${success ? "COMPLETED" : "FAILED"}: ${reason} (${duration}ms)`,
    );
  }

  /**
   * Get current metrics
   * @returns {object} Current metrics
   */
  getMetrics() {
    return {
      ...this.metrics,
      elapsed: this.metrics.startTime ? Date.now() - this.metrics.startTime : 0,
      successRate:
        this.metrics.actionsAttempted > 0
          ? this.metrics.actionsSucceeded / this.metrics.actionsAttempted
          : 0,
      llmFailureRate:
        this.metrics.llmCalls > 0
          ? this.metrics.llmFailures / this.metrics.llmCalls
          : 0,
    };
  }

  /**
   * Emit event to all listeners
   * @private
   */
  _emit(event, data) {
    const payload = { event, data };

    // Emit to WebSocket server
    if (this.wsServer) {
      try {
        this.wsServer.emit("agent:progress", payload);
      } catch (error) {
        logger.debug("[ProgressTracker] WebSocket emit failed:", error.message);
      }
    }

    // Emit to registered listeners
    for (const listener of this.listeners) {
      try {
        listener(payload);
      } catch (error) {
        logger.debug("[ProgressTracker] Listener error:", error.message);
      }
    }
  }

  /**
   * Reset metrics
   */
  reset() {
    this.metrics = {
      stepsCompleted: 0,
      actionsAttempted: 0,
      actionsSucceeded: 0,
      actionsFailed: 0,
      llmCalls: 0,
      llmFailures: 0,
      errors: 0,
      startTime: null,
      currentGoal: null,
      currentUrl: null,
    };
  }
}

const progressTracker = new ProgressTracker();

export { progressTracker };
export default progressTracker;
