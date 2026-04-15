/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Circuit Breaker for Model Health Monitoring
 * @module core/circuit-breaker
 */

import { createLogger } from "../core/logger.js";
import { getTimeoutValue } from "../utils/configLoader.js";

const logger = createLogger("circuit-breaker.js");

const STATE_CLOSED = "CLOSED";
const STATE_OPEN = "OPEN";
const STATE_HALF_OPEN = "HALF_OPEN";

class CircuitOpenError extends Error {
  constructor(modelId, waitTime) {
    super(
      `Circuit breaker OPEN for ${modelId}. Retry after ${Math.ceil(waitTime / 1000)}s`,
    );
    this.code = "CIRCUIT_OPEN";
    this.modelId = modelId;
    this.retryAfter = waitTime;
  }
}

export { CircuitOpenError };

function getKey(model, apiKey) {
  return `${model}::${apiKey || "default"}`;
}

/**
 * @class CircuitBreaker
 * @description Prevents cascading failures by monitoring model health and tripping when failure threshold reached
 */
class CircuitBreaker {
  constructor(options = {}) {
    // Set defaults synchronously first
    this.failureThreshold = options.failureThreshold ?? 50;
    this.successThreshold = options.successThreshold ?? 2;
    this.halfOpenTime = options.halfOpenTime ?? 30000;
    this.monitoringWindow = options.monitoringWindow ?? 60000;
    this.minSamples = options.minSamples ?? 5;
    this.breakers = new Map();

    // Then async load to override with config values
    this._configLoaded = false;
    this._loadConfig(options);
  }

  check(model, apiKey = null) {
    const key = getKey(model, apiKey);
    const breaker = this.getBreaker(key);

    const now = Date.now();

    switch (breaker.state) {
      case STATE_OPEN: {
        if (breaker.nextAttempt && now >= breaker.nextAttempt) {
          breaker.state = STATE_HALF_OPEN;
          logger.info(`[${key}] Circuit breaker transitioning to half-open`);
          return { allowed: true, state: "half-open" };
        }
        const retryAfter = breaker.nextAttempt
          ? breaker.nextAttempt - now
          : this.halfOpenTime;
        return { allowed: false, state: "open", retryAfter };
      }
      case STATE_HALF_OPEN:
        return { allowed: true, state: "half-open" };
      case STATE_CLOSED:
      default:
        return { allowed: true, state: "closed" };
    }
  }

  recordSuccess(model, apiKey = null) {
    const key = getKey(model, apiKey);
    const breaker = this.getBreaker(key);

    if (breaker.state === STATE_HALF_OPEN) {
      breaker.successes++;

      if (breaker.successes >= this.successThreshold) {
        breaker.state = STATE_CLOSED;
        breaker.failures = 0;
        breaker.successes = 0;
        logger.info(`[${key}] Circuit breaker closed (recovered)`);
      }
    }
  }

  recordFailure(model, apiKey = null) {
    const key = getKey(model, apiKey);
    const breaker = this.getBreaker(key);

    breaker.failures++;
    breaker.lastFailure = Date.now();

    const failureRate = this._calculateFailureRate(breaker);

    if (breaker.state === STATE_HALF_OPEN) {
      breaker.state = STATE_OPEN;
      breaker.nextAttempt = Date.now() + this.halfOpenTime;
      breaker.successes = 0;
      logger.warn(
        `[${key}] Circuit breaker reopened after failure in half-open state`,
      );
    } else if (
      breaker.state === STATE_CLOSED &&
      failureRate >= this.failureThreshold
    ) {
      breaker.state = STATE_OPEN;
      breaker.nextAttempt = Date.now() + this.halfOpenTime;
      logger.warn(
        `[${key}] Circuit breaker opened after ${breaker.failures} failures (rate: ${failureRate}%)`,
      );
    }
  }

  getState(model, apiKey = null) {
    const key = getKey(model, apiKey);
    const breaker = this.breakers.get(key);
    if (!breaker) return null;
    return {
      state: breaker.state,
      failures: breaker.failures,
      successes: breaker.successes,
      lastFailure: breaker.lastFailure,
      lastSuccess: breaker.lastSuccess,
    };
  }

  getAllStates() {
    const states = {};
    for (const [key, breaker] of this.breakers) {
      states[key] = {
        state: breaker.state,
        failures: breaker.failures,
        successes: breaker.successes,
        lastFailure: breaker.lastFailure,
      };
    }
    return states;
  }

  reset(model = null, _apiKey = null) {
    if (model) {
      if (this.breakers.has(model)) {
        this.breakers.set(model, this._createBreaker());
        logger.info(`[${model}] Circuit breaker reset`);
      }
    } else {
      this.breakers.clear();
      logger.info("All circuit breakers reset");
    }
  }

  getStats() {
    let open = 0;
    let halfOpen = 0;
    let closed = 0;

    for (const breaker of this.breakers.values()) {
      if (breaker.state === STATE_OPEN) open++;
      else if (breaker.state === STATE_HALF_OPEN) halfOpen++;
      else closed++;
    }

    return {
      total: this.breakers.size,
      open,
      halfOpen,
      closed,
    };
  }

  async _loadConfig(_options = {}) {
    if (this._configLoaded) return;

    const cbConfig = await getTimeoutValue("circuitBreaker", {});

    // Only override if config provides values
    if (cbConfig.failureThreshold !== undefined)
      this.failureThreshold = cbConfig.failureThreshold;
    if (cbConfig.successThreshold !== undefined)
      this.successThreshold = cbConfig.successThreshold;
    if (cbConfig.halfOpenTime !== undefined)
      this.halfOpenTime = cbConfig.halfOpenTime;
    if (cbConfig.monitoringWindow !== undefined)
      this.monitoringWindow = cbConfig.monitoringWindow;
    if (cbConfig.minSamples !== undefined)
      this.minSamples = cbConfig.minSamples;

    this._configLoaded = true;
  }

  /**
   * Get or create a circuit breaker for a model
   * @param {string} modelId - Unique model identifier
   * @returns {object} Circuit breaker instance
   */
  getBreaker(modelId) {
    if (!this.breakers.has(modelId)) {
      this.breakers.set(modelId, this._createBreaker());
    }
    return this.breakers.get(modelId);
  }

  /**
   * Create a new breaker instance
   * @private
   */
  _createBreaker() {
    return {
      state: STATE_CLOSED,
      failures: 0,
      successes: 0,
      lastFailure: null,
      lastSuccess: null,
      nextAttempt: null,
      history: [],
    };
  }

  /**
   * Execute a function through the circuit breaker
   * @param {string} modelId - Model identifier
   * @param {Function} fn - Async function to execute
   * @param {object} options - Execution options
   * @returns {Promise<any>} Function result
   */
  async execute(modelId, fn, _options = {}) {
    const breaker = this.getBreaker(modelId);

    if (breaker.state === STATE_OPEN) {
      if (Date.now() < breaker.nextAttempt) {
        const waitTime = breaker.nextAttempt - Date.now();
        throw new CircuitOpenError(modelId, waitTime);
      } else {
        breaker.state = STATE_HALF_OPEN;
        logger.info(`[${modelId}] Circuit breaker transitioning to HALF_OPEN`);
      }
    }

    try {
      const result = await fn();
      this._recordSuccess(breaker, modelId);

      return result;
    } catch (error) {
      this._recordFailure(breaker, modelId, error);
      throw error;
    }
  }

  /**
   * Check if circuit is open
   * @private
   */
  _isOpen(breaker) {
    if (breaker.state === STATE_OPEN) {
      if (Date.now() >= breaker.nextAttempt) {
        return false;
      }
    }
    return breaker.state === STATE_OPEN;
  }

  /**
   * Record successful execution
   * @private
   */
  _recordSuccess(breaker, modelId) {
    breaker.successes++;
    breaker.lastSuccess = Date.now();
    breaker.history.push({ time: Date.now(), type: "success" });
    this._cleanupHistory(breaker);

    if (breaker.state === STATE_HALF_OPEN) {
      if (breaker.successes >= this.successThreshold) {
        breaker.state = STATE_CLOSED;
        breaker.failures = 0;
        breaker.successes = 0;
        logger.info(`[${modelId}] Circuit breaker CLOSED (recovered)`);
      }
    }
  }

  /**
   * Record failed execution
   * @private
   */
  _recordFailure(breaker, modelId, error) {
    breaker.failures++;
    breaker.lastFailure = Date.now();
    breaker.history.push({
      time: Date.now(),
      type: "failure",
      error: error.message,
    });
    this._cleanupHistory(breaker);

    const failureRate = this._calculateFailureRate(breaker);

    if (breaker.state === STATE_HALF_OPEN) {
      breaker.state = STATE_OPEN;
      breaker.nextAttempt = Date.now() + this.halfOpenTime;
      logger.warn(
        `[${modelId}] Circuit breaker OPEN (failed in HALF_OPEN, rate: ${failureRate}%)`,
      );
    } else if (
      breaker.state === STATE_CLOSED &&
      failureRate >= this.failureThreshold
    ) {
      breaker.state = STATE_OPEN;
      breaker.nextAttempt = Date.now() + this.halfOpenTime;
      logger.warn(
        `[${modelId}] Circuit breaker OPEN (failure rate: ${failureRate}%)`,
      );
    }
  }

  /**
   * Calculate failure rate percentage
   * @private
   */
  _calculateFailureRate(breaker) {
    const windowStart = Date.now() - this.monitoringWindow;
    const recentHistory = breaker.history.filter((h) => h.time >= windowStart);

    if (recentHistory.length === 0 || recentHistory.length < this.minSamples)
      return 0;

    const failures = recentHistory.filter((h) => h.type === "failure").length;
    return Math.round((failures / recentHistory.length) * 100);
  }

  /**
   * Clean up old history entries
   * @private
   */
  _cleanupHistory(breaker) {
    const windowStart = Date.now() - this.monitoringWindow;
    breaker.history = breaker.history.filter((h) => h.time >= windowStart);

    if (breaker.history.length > 100) {
      breaker.history = breaker.history.slice(-100);
    }
  }

  /**
   * Get health status of a model
   * @param {string} modelId - Model identifier
   * @returns {object}
   */
  getHealth(modelId) {
    const breaker = this.breakers.get(modelId);

    if (!breaker) {
      return { status: "unknown", modelId };
    }

    const failureRate = this._calculateFailureRate(breaker);
    const windowStart = Date.now() - this.monitoringWindow;
    const recentHistory = breaker.history.filter((h) => h.time >= windowStart);

    return {
      status: breaker.state,
      modelId,
      failureRate,
      recentOperations: recentHistory.length,
      lastSuccess: breaker.lastSuccess,
      lastFailure: breaker.lastFailure,
      nextAttempt: breaker.nextAttempt,
    };
  }

  /**
   * Get status of all breakers
   * @returns {object}
   */
  getAllStatus() {
    const status = {};

    for (const [modelId, breaker] of this.breakers) {
      const failureRate = this._calculateFailureRate(breaker);
      status[modelId] = {
        state: breaker.state,
        failureRate: `${failureRate}%`,
        failures: breaker.failures,
        successes: breaker.successes,
      };
    }

    return status;
  }

  /**
   * Reset all breakers
   */
  resetAll() {
    this.breakers.clear();
    logger.info("All circuit breakers reset");
  }

  /**
   * Force open a breaker (for maintenance)
   * @param {string} modelId - Model identifier
   */
  forceOpen(modelId) {
    const breaker = this.getBreaker(modelId);
    breaker.state = STATE_OPEN;
    breaker.nextAttempt = Date.now() + this.halfOpenTime;
    logger.info(`[${modelId}] Circuit breaker FORCED OPEN`);
  }

  /**
   * Force close a breaker (for recovery)
   * @param {string} modelId - Model identifier
   */
  forceClose(modelId) {
    const breaker = this.getBreaker(modelId);
    breaker.state = STATE_CLOSED;
    breaker.failures = 0;
    breaker.successes = 0;
    logger.info(`[${modelId}] Circuit breaker FORCED CLOSED`);
  }
}

export default CircuitBreaker;

/**
 * @class BrowserCircuitBreaker
 * @description Circuit breaker specifically for browser connections
 *               Monitors connection failures per profile/endpoint
 */
class BrowserCircuitBreaker {
  constructor(options = {}) {
    this.failureThreshold = options.failureThreshold ?? 5;
    this.successThreshold = options.successThreshold ?? 3;
    this.halfOpenTime = options.halfOpenTime ?? 30000;
    this.enabled = options.enabled ?? true;
    this.breakers = new Map();
    this._configLoaded = false;
  }

  /**
   * Check if connection is allowed
   * @param {string} profileId - Browser profile identifier
   * @returns {{allowed: boolean, state: string, retryAfter?: number}}
   */
  check(profileId) {
    if (!this.enabled) {
      return { allowed: true, state: "disabled" };
    }

    const breaker = this._getOrCreate(profileId);
    const now = Date.now();

    switch (breaker.state) {
      case STATE_OPEN:
        if (breaker.nextAttempt && now >= breaker.nextAttempt) {
          breaker.state = STATE_HALF_OPEN;
          logger.info(
            `[Browser:${profileId}] Circuit breaker transitioning to half-open`,
          );
          return { allowed: true, state: "half-open" };
        }
        return {
          allowed: false,
          state: "open",
          retryAfter: breaker.nextAttempt
            ? breaker.nextAttempt - now
            : this.halfOpenTime,
        };
      case STATE_HALF_OPEN:
        return { allowed: true, state: "half-open" };
      case STATE_CLOSED:
      default:
        return { allowed: true, state: "closed" };
    }
  }

  /**
   * Record successful connection
   * @param {string} profileId
   */
  recordSuccess(profileId) {
    if (!this.enabled) return;

    const breaker = this._getOrCreate(profileId);

    if (breaker.state === STATE_HALF_OPEN) {
      breaker.successes++;
      if (breaker.successes >= this.successThreshold) {
        breaker.state = STATE_CLOSED;
        breaker.failures = 0;
        breaker.successes = 0;
        logger.info(
          `[Browser:${profileId}] Circuit breaker CLOSED (recovered)`,
        );
      }
    } else if (breaker.state === STATE_CLOSED) {
      breaker.failures = Math.max(0, breaker.failures - 1);
    }
  }

  /**
   * Record failed connection
   * @param {string} profileId
   */
  recordFailure(profileId) {
    if (!this.enabled) return;

    const breaker = this._getOrCreate(profileId);
    breaker.failures++;
    breaker.lastFailure = Date.now();

    if (breaker.state === STATE_HALF_OPEN) {
      breaker.state = STATE_OPEN;
      breaker.nextAttempt = Date.now() + this.halfOpenTime;
      breaker.successes = 0;
      logger.warn(
        `[Browser:${profileId}] Circuit breaker reopened after failure in half-open`,
      );
    } else if (
      breaker.state === STATE_CLOSED &&
      breaker.failures >= this.failureThreshold
    ) {
      breaker.state = STATE_OPEN;
      breaker.nextAttempt = Date.now() + this.halfOpenTime;
      logger.warn(
        `[Browser:${profileId}] Circuit breaker OPEN after ${breaker.failures} failures`,
      );
    }
  }

  /**
   * Execute function with circuit breaker protection
   * @param {string} profileId
   * @param {Function} fn
   * @returns {Promise<any>}
   */
  async execute(profileId, fn) {
    const check = this.check(profileId);

    if (!check.allowed) {
      throw new CircuitOpenError(profileId, check.retryAfter);
    }

    try {
      const result = await fn();
      this.recordSuccess(profileId);
      return result;
    } catch (error) {
      this.recordFailure(profileId);
      throw error;
    }
  }

  /**
   * Get state for a specific profile
   * @param {string} profileId
   * @returns {object|null}
   */
  getState(profileId) {
    const breaker = this.breakers.get(profileId);
    if (!breaker) return null;
    return {
      state: breaker.state,
      failures: breaker.failures,
      successes: breaker.successes,
      lastFailure: breaker.lastFailure,
    };
  }

  /**
   * Get all breaker states
   * @returns {object}
   */
  getAllStates() {
    const states = {};
    for (const [id, breaker] of this.breakers) {
      states[id] = {
        state: breaker.state,
        failures: breaker.failures,
        successes: breaker.successes,
      };
    }
    return states;
  }

  /**
   * Reset breaker for a profile
   * @param {string} profileId
   */
  reset(profileId) {
    if (this.breakers.has(profileId)) {
      this.breakers.set(profileId, this._createBreaker());
      logger.info(`[Browser:${profileId}] Circuit breaker reset`);
    }
  }

  /**
   * Reset all breakers
   */
  resetAll() {
    this.breakers.clear();
    logger.info("All browser circuit breakers reset");
  }

  /**
   * Get stats
   * @returns {object}
   */
  getStats() {
    let open = 0,
      halfOpen = 0,
      closed = 0;
    for (const breaker of this.breakers.values()) {
      if (breaker.state === STATE_OPEN) open++;
      else if (breaker.state === STATE_HALF_OPEN) halfOpen++;
      else closed++;
    }
    return { total: this.breakers.size, open, halfOpen, closed };
  }

  _getOrCreate(profileId) {
    if (!this.breakers.has(profileId)) {
      this.breakers.set(profileId, this._createBreaker());
    }
    return this.breakers.get(profileId);
  }

  _createBreaker() {
    return {
      state: STATE_CLOSED,
      failures: 0,
      successes: 0,
      lastFailure: null,
      nextAttempt: null,
    };
  }
}

export { BrowserCircuitBreaker };
