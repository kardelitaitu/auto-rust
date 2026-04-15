/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Parallel Execution Engine
 * Executes independent actions in parallel for improved performance
 * @module api/agent/parallelExecutor
 */

import { createLogger } from "../core/logger.js";

const logger = createLogger("api/agent/parallelExecutor.js");

class ParallelExecutor {
  constructor() {
    this.independentActions = new Set(["wait", "screenshot", "verify"]);
    this.dependentActions = new Set([
      "click",
      "clickAt",
      "type",
      "navigate",
      "scroll",
      "drag",
      "multiSelect",
    ]);
  }

  /**
   * Execute action sequence with parallel execution for independent actions
   * @param {Array} actions - Array of action objects
   * @param {function} executeFn - Function to execute a single action
   * @returns {Promise<Array>} Array of results
   */
  async executeSequence(actions, executeFn) {
    if (!actions || actions.length === 0) {
      return [];
    }

    if (actions.length === 1) {
      // Single action, no parallelization needed
      const result = await executeFn(actions[0]);
      return [{ action: actions[0], result }];
    }

    // Separate independent and dependent actions
    const { independent, dependent } = this._categorizeActions(actions);

    logger.info(
      `[ParallelExecutor] Executing ${actions.length} actions: ${independent.length} independent, ${dependent.length} dependent`,
    );

    const results = [];

    // Execute independent actions in parallel
    if (independent.length > 0) {
      const parallelResults = await this._executeParallel(
        independent,
        executeFn,
      );
      results.push(...parallelResults);
    }

    // Execute dependent actions sequentially
    if (dependent.length > 0) {
      const sequentialResults = await this._executeSequential(
        dependent,
        executeFn,
      );
      results.push(...sequentialResults);
    }

    // Sort results by original order
    const orderedResults = this._orderByOriginalOrder(results, actions);

    return orderedResults;
  }

  /**
   * Categorize actions into independent and dependent
   * @private
   */
  _categorizeActions(actions) {
    const independent = [];
    const dependent = [];

    actions.forEach((action, index) => {
      const actionWithIndex = { ...action, _originalIndex: index };

      if (this._isIndependent(action)) {
        independent.push(actionWithIndex);
      } else {
        dependent.push(actionWithIndex);
      }
    });

    return { independent, dependent };
  }

  /**
   * Check if action is independent (doesn't affect DOM state for other actions)
   * @private
   */
  _isIndependent(action) {
    return this.independentActions.has(action.action);
  }

  /**
   * Execute actions in parallel
   * @private
   */
  async _executeParallel(actions, executeFn) {
    const promises = actions.map(async (action) => {
      try {
        const result = await executeFn(action);
        return { action, result, success: true };
      } catch (error) {
        logger.error(
          `[ParallelExecutor] Parallel action failed: ${action.action}`,
          error.message,
        );
        return {
          action,
          result: { success: false, error: error.message },
          success: false,
        };
      }
    });

    const results = await Promise.allSettled(promises);

    return results.map((r, i) => ({
      action: actions[i],
      result:
        r.status === "fulfilled"
          ? r.value.result
          : { success: false, error: "Promise rejected" },
      success: r.status === "fulfilled" ? r.value.success : false,
    }));
  }

  /**
   * Execute actions sequentially
   * @private
   */
  async _executeSequential(actions, executeFn) {
    const results = [];

    for (const action of actions) {
      try {
        const result = await executeFn(action);
        results.push({ action, result, success: result.success });
      } catch (error) {
        logger.error(
          `[ParallelExecutor] Sequential action failed: ${action.action}`,
          error.message,
        );
        results.push({
          action,
          result: { success: false, error: error.message },
          success: false,
        });
        // Stop on first failure for dependent actions
        break;
      }
    }

    return results;
  }

  /**
   * Order results by original action order
   * @private
   */
  _orderByOriginalOrder(results, originalActions) {
    const resultMap = new Map();

    results.forEach((r) => {
      const index = r.action._originalIndex;
      resultMap.set(index, r);
    });

    return originalActions
      .map((_, index) => resultMap.get(index))
      .filter(Boolean);
  }

  /**
   * Get execution statistics
   * @param {Array} results - Execution results
   * @returns {object} Statistics
   */
  getStats(results) {
    const total = results.length;
    const successful = results.filter((r) => r.success).length;
    const failed = total - successful;

    return {
      total,
      successful,
      failed,
      successRate: total > 0 ? successful / total : 0,
    };
  }

  /**
   * Check if action sequence can benefit from parallelization
   * @param {Array} actions - Array of action objects
   * @returns {boolean} True if parallelization is beneficial
   */
  canParallelize(actions) {
    if (!actions || actions.length < 2) {
      return false;
    }

    const independentCount = actions.filter((a) =>
      this._isIndependent(a),
    ).length;
    return independentCount >= 2; // At least 2 independent actions
  }

  /**
   * Estimate speedup from parallelization
   * @param {Array} actions - Array of action objects
   * @returns {object} Speedup estimate
   */
  estimateSpeedup(actions) {
    const { independent, dependent } = this._categorizeActions(actions);

    // Assume independent actions take 100ms each, dependent take 200ms each
    const sequentialTime = independent.length * 100 + dependent.length * 200;
    const parallelTime =
      Math.max(...independent.map(() => 100), 0) + dependent.length * 200;

    const speedup = sequentialTime > 0 ? sequentialTime / parallelTime : 1;

    return {
      sequentialTime,
      parallelTime,
      speedup: Math.round(speedup * 100) / 100,
      independentCount: independent.length,
      dependentCount: dependent.length,
    };
  }
}

const parallelExecutor = new ParallelExecutor();

export { parallelExecutor };
export default parallelExecutor;
