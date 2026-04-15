/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Health Monitor - Centralized health monitoring for all system components
 * Aggregates circuit breaker status, browser connections, and system resources.
 * @module core/health-monitor
 */

import { createLogger } from "./logger.js";
import { recordStatusChange, recordProviderFailure } from "./health-alerts.js";

const logger = createLogger("health-monitor.js");

/**
 * Health status constants
 */
export const HealthStatus = {
  HEALTHY: "healthy",
  DEGRADED: "degraded",
  UNHEALTHY: "unhealthy",
  UNKNOWN: "unknown",
};

/**
 * Health Monitor Class
 *
 * Provides centralized health monitoring for:
 * - Circuit breakers (LLM providers)
 * - Browser connections
 * - System resources
 */
export class HealthMonitor {
  constructor(options = {}) {
    this.circuitBreaker = options.circuitBreaker || null;
    this.browserBreaker = options.browserBreaker || null;

    this.healthHistory = [];
    this.maxHistory = options.maxHistory || 100;
    this.checkInterval = options.checkInterval || 30000;
    this.lastCheck = null;
    this.currentHealth = {
      overall: HealthStatus.UNKNOWN,
      circuitBreakers: {},
      browsers: {},
      system: {},
    };

    this.listeners = new Map();
    this.checkTimer = null;

    if (this.checkInterval > 0) {
      this.startPeriodicChecks();
    }
  }

  /**
   * Set circuit breaker instance
   * @param {CircuitBreaker} breaker
   */
  setCircuitBreaker(breaker) {
    this.circuitBreaker = breaker;
  }

  /**
   * Set browser circuit breaker instance
   * @param {BrowserCircuitBreaker} breaker
   */
  setBrowserBreaker(breaker) {
    this.browserBreaker = breaker;
  }

  /**
   * Get overall system health
   * @returns {object} Health status
   */
  getHealth() {
    return {
      overall: this.currentHealth.overall,
      timestamp: new Date().toISOString(),
      lastCheck: this.lastCheck,
      circuitBreakers: this.getCircuitBreakerHealth(),
      browsers: this.getBrowserHealth(),
      system: this.getSystemHealth(),
    };
  }

  /**
   * Get circuit breaker health
   * @returns {object}
   */
  getCircuitBreakerHealth() {
    if (!this.circuitBreaker) {
      return {};
    }

    const health = {};
    const status = this.circuitBreaker.getAllStatus();

    for (const [model, data] of Object.entries(status)) {
      const failureRate = parseFloat(data.failureRate) || 0;

      health[model] = {
        state: data.state,
        failureRate: data.failureRate,
        status: this._calculateProviderHealth(failureRate, data.state),
        details: {
          failures: data.failures,
          successes: data.successes,
        },
      };
    }

    return health;
  }

  /**
   * Get browser connection health
   * @returns {object}
   */
  getBrowserHealth() {
    if (!this.browserBreaker) {
      return {};
    }

    const health = {};
    const states = this.browserBreaker.getAllStates();

    for (const [profileId, data] of Object.entries(states)) {
      health[profileId] = {
        state: data.state,
        status: this._calculateProviderHealth(
          (data.failures / (data.failures + data.successes || 1)) * 100,
          data.state,
        ),
        details: {
          failures: data.failures,
          successes: data.successes,
        },
      };
    }

    return health;
  }

  /**
   * Get system health (memory, CPU, etc.)
   * @returns {object}
   */
  getSystemHealth() {
    const memUsage = process.memoryUsage();
    const heapUsedPercent = (memUsage.heapUsed / memUsage.heapTotal) * 100;

    return {
      memory: {
        heapUsed: Math.round(memUsage.heapUsed / 1024 / 1024) + " MB",
        heapTotal: Math.round(memUsage.heapTotal / 1024 / 1024) + " MB",
        usagePercent: heapUsedPercent.toFixed(2) + "%",
        status:
          heapUsedPercent > 90
            ? HealthStatus.UNHEALTHY
            : heapUsedPercent > 70
              ? HealthStatus.DEGRADED
              : HealthStatus.HEALTHY,
      },
      uptime: Math.round(process.uptime()) + " seconds",
      status: HealthStatus.HEALTHY,
    };
  }

  /**
   * Check if a specific provider is healthy
   * @param {string} providerId - Provider identifier
   * @returns {boolean}
   */
  isProviderHealthy(providerId) {
    if (!this.circuitBreaker) return true;

    const health = this.circuitBreaker.getHealth(providerId);
    if (!health) return true;

    return (
      health.status === HealthStatus.HEALTHY ||
      health.status === HealthStatus.DEGRADED
    );
  }

  /**
   * Get recommended provider (lowest failure rate, healthy status)
   * @returns {string|null} Recommended provider ID
   */
  getRecommendedProvider() {
    if (!this.circuitBreaker) return null;

    const health = this.getCircuitBreakerHealth();

    const healthyProviders = Object.entries(health)
      .filter(
        ([_, data]) =>
          data.status === HealthStatus.HEALTHY ||
          data.status === HealthStatus.DEGRADED,
      )
      .sort((a, b) => {
        const rateA = parseFloat(a[1].failureRate) || 0;
        const rateB = parseFloat(b[1].failureRate) || 0;
        return rateA - rateB;
      });

    if (healthyProviders.length > 0) {
      return healthyProviders[0][0];
    }

    return null;
  }

  /**
   * Get health history
   * @returns {Array}
   */
  getHistory() {
    return this.healthHistory;
  }

  /**
   * Start periodic health checks
   */
  startPeriodicChecks() {
    if (this.checkTimer) {
      clearInterval(this.checkTimer);
    }

    this.checkTimer = setInterval(() => {
      this.performHealthCheck();
    }, this.checkInterval);

    logger.debug(
      `Health monitoring started (interval: ${this.checkInterval}ms)`,
    );
  }

  /**
   * Stop periodic health checks
   */
  stopPeriodicChecks() {
    if (this.checkTimer) {
      clearInterval(this.checkTimer);
      this.checkTimer = null;
      logger.debug("Health monitoring stopped");
    }
  }

  /**
   * Perform health check and update status
   */
  performHealthCheck() {
    const previousHealth = this.currentHealth.overall;

    this.currentHealth = {
      overall: this._calculateOverallHealth(),
      circuitBreakers: this.getCircuitBreakerHealth(),
      browsers: this.getBrowserHealth(),
      system: this.getSystemHealth(),
    };

    this.lastCheck = new Date().toISOString();

    // Record status change alert
    if (previousHealth !== HealthStatus.UNKNOWN && previousHealth !== this.currentHealth.overall) {
      recordStatusChange(previousHealth, this.currentHealth.overall, this.currentHealth);
    }

    // Record provider failures
    const cbHealth = this.getCircuitBreakerHealth();
    for (const [providerId, data] of Object.entries(cbHealth)) {
      const failureRate = parseFloat(data.failureRate) || 0;
      if (failureRate > 0) {
        recordProviderFailure(providerId, failureRate);
      }
    }

    this.healthHistory.push({
      timestamp: this.lastCheck,
      health: this.currentHealth,
    });

    if (this.healthHistory.length > this.maxHistory) {
      this.healthHistory.shift();
    }

    if (previousHealth !== this.currentHealth.overall) {
      this._notifyHealthChange(previousHealth, this.currentHealth.overall);
    }
  }

  /**
   * Register health change listener
   * @param {string} id - Listener identifier
   * @param {Function} callback - Callback function
   */
  onHealthChange(id, callback) {
    this.listeners.set(id, callback);
  }

  /**
   * Remove health change listener
   * @param {string} id - Listener identifier
   */
  removeListener(id) {
    this.listeners.delete(id);
  }

  /**
   * Get health as JSON for API response
   * @returns {object}
   */
  toJSON() {
    return {
      status: this.currentHealth.overall,
      timestamp: this.lastCheck,
      providers: this.getCircuitBreakerHealth(),
      browsers: this.getBrowserHealth(),
      system: this.getSystemHealth(),
      recommendations: this._getRecommendations(),
    };
  }

  /**
   * Calculate provider health status
   * @private
   */
  _calculateProviderHealth(failureRate, state) {
    if (state === "OPEN") return HealthStatus.UNHEALTHY;
    if (state === "HALF_OPEN") return HealthStatus.DEGRADED;
    if (failureRate > 50) return HealthStatus.UNHEALTHY;
    if (failureRate > 20) return HealthStatus.DEGRADED;
    return HealthStatus.HEALTHY;
  }

  /**
   * Calculate overall system health
   * @private
   */
  _calculateOverallHealth() {
    const cbHealth = this.getCircuitBreakerHealth();
    const browserHealth = this.getBrowserHealth();
    const systemHealth = this.getSystemHealth();

    const hasUnhealthyProvider = Object.values(cbHealth).some(
      (h) => h.status === HealthStatus.UNHEALTHY,
    );
    const hasUnhealthyBrowser = Object.values(browserHealth).some(
      (h) => h.status === HealthStatus.UNHEALTHY,
    );
    const systemUnhealthy =
      systemHealth.memory.status === HealthStatus.UNHEALTHY;

    if (hasUnhealthyProvider || hasUnhealthyBrowser || systemUnhealthy) {
      return HealthStatus.DEGRADED;
    }

    const hasDegradedProvider = Object.values(cbHealth).some(
      (h) => h.status === HealthStatus.DEGRADED,
    );
    const hasDegradedBrowser = Object.values(browserHealth).some(
      (h) => h.status === HealthStatus.DEGRADED,
    );

    if (hasDegradedProvider || hasDegradedBrowser) {
      return HealthStatus.DEGRADED;
    }

    return HealthStatus.HEALTHY;
  }

  /**
   * Get recommendations based on current health
   * @private
   */
  _getRecommendations() {
    const recommendations = [];
    const cbHealth = this.getCircuitBreakerHealth();
    const browserHealth = this.getBrowserHealth();

    for (const [provider, health] of Object.entries(cbHealth)) {
      if (health.status === HealthStatus.UNHEALTHY) {
        recommendations.push({
          type: "warning",
          component: "provider",
          id: provider,
          message: `Provider ${provider} is unhealthy (${health.failureRate} failure rate)`,
          action: "Consider switching to backup provider",
        });
      }
    }

    for (const [profileId, health] of Object.entries(browserHealth)) {
      if (health.status === HealthStatus.UNHEALTHY) {
        recommendations.push({
          type: "warning",
          component: "browser",
          id: profileId,
          message: `Browser profile ${profileId} is unhealthy`,
          action: "Consider restarting browser profile",
        });
      }
    }

    return recommendations;
  }

  /**
   * Notify listeners of health change
   * @private
   */
  _notifyHealthChange(previous, current) {
    logger.info(`Health changed: ${previous} → ${current}`);

    for (const callback of this.listeners.values()) {
      try {
        callback(previous, current, this.currentHealth);
      } catch (error) {
        logger.error("Health listener error:", error.message);
      }
    }
  }

  /**
   * Cleanup
   */
  destroy() {
    this.stopPeriodicChecks();
    this.listeners.clear();
    this.healthHistory = [];
  }
}

/**
 * Default health monitor instance (singleton)
 */
export const healthMonitor = new HealthMonitor({
  checkInterval: 30000,
  maxHistory: 100,
});

/**
 * Get current health (convenience function)
 * @returns {object}
 */
export function getHealth() {
  return healthMonitor.getHealth();
}

/**
 * Check if provider is healthy (convenience function)
 * @param {string} providerId
 * @returns {boolean}
 */
export function isProviderHealthy(providerId) {
  return healthMonitor.isProviderHealthy(providerId);
}

/**
 * Get recommended provider (convenience function)
 * @returns {string|null}
 */
export function getRecommendedProvider() {
  return healthMonitor.getRecommendedProvider();
}

export default {
  HealthMonitor,
  HealthStatus,
  healthMonitor,
  getHealth,
  isProviderHealthy,
  getRecommendedProvider,
};
