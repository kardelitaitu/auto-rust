/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Health Alerts System
 * Manages alerts for health status changes and threshold violations
 * @module core/health-alerts
 */

import { createLogger } from './logger.js';

const logger = createLogger('health-alerts.js');

/**
 * Alert severity levels
 */
export const AlertSeverity = {
  INFO: 'info',
  WARNING: 'warning',
  CRITICAL: 'critical'
};

/**
 * Health Alerts Manager
 */
export class HealthAlertsManager {
  constructor(options = {}) {
    this.alerts = [];
    this.maxAlerts = options.maxAlerts || 50;
    this.webhookUrl = options.webhookUrl;
    this.webhookEnabled = !!options.webhookUrl;
    this.listeners = new Map();
    
    // Alert thresholds
    this.thresholds = {
      providerFailureRate: options.providerFailureRate || 50,
      memoryUsage: options.memoryUsage || 90,
      consecutiveFailures: options.consecutiveFailures || 5
    };
    
    // Track consecutive failures per provider
    this.failureCounts = new Map();
  }

  /**
   * Record health status change
   * @param {string} previous - Previous status
   * @param {string} current - Current status
   * @param {object} health - Full health data
   */
  recordStatusChange(previous, current, health) {
    if (previous === current) return;
    
    const severity = current === 'unhealthy' ? AlertSeverity.CRITICAL :
                     current === 'degraded' ? AlertSeverity.WARNING :
                     AlertSeverity.INFO;
    
    const alert = {
      id: `status-${Date.now()}`,
      type: 'status_change',
      severity,
      message: `System health changed: ${previous} → ${current}`,
      previous,
      current,
      health,
      timestamp: new Date()
    };
    
    this.addAlert(alert);
  }

  /**
   * Record provider failure
   * @param {string} providerId - Provider identifier
   * @param {number} failureRate - Current failure rate
   */
  recordProviderFailure(providerId, failureRate) {
    // Track consecutive failures
    const count = (this.failureCounts.get(providerId) || 0) + 1;
    this.failureCounts.set(providerId, count);
    
    // Check thresholds
    if (failureRate >= this.thresholds.providerFailureRate) {
      const alert = {
        id: `provider-${providerId}-${Date.now()}`,
        type: 'provider_failure',
        severity: AlertSeverity.CRITICAL,
        message: `Provider ${providerId} failure rate exceeded threshold (${failureRate}%)`,
        providerId,
        failureRate,
        consecutiveFailures: count,
        timestamp: new Date()
      };
      
      this.addAlert(alert);
    } else if (failureRate >= this.thresholds.providerFailureRate * 0.8) {
      const alert = {
        id: `provider-${providerId}-${Date.now()}`,
        type: 'provider_warning',
        severity: AlertSeverity.WARNING,
        message: `Provider ${providerId} failure rate elevated (${failureRate}%)`,
        providerId,
        failureRate,
        timestamp: new Date()
      };
      
      this.addAlert(alert);
    } else {
      // Reset consecutive count on success
      this.failureCounts.set(providerId, 0);
    }
  }

  /**
   * Record memory warning
   * @param {number} usagePercent - Memory usage percentage
   */
  recordMemoryWarning(usagePercent) {
    if (usagePercent >= this.thresholds.memoryUsage) {
      const alert = {
        id: `memory-${Date.now()}`,
        type: 'memory_critical',
        severity: AlertSeverity.CRITICAL,
        message: `Memory usage critical (${usagePercent}%)`,
        usagePercent,
        timestamp: new Date()
      };
      
      this.addAlert(alert);
    } else if (usagePercent >= this.thresholds.memoryUsage * 0.9) {
      const alert = {
        id: `memory-${Date.now()}`,
        type: 'memory_warning',
        severity: AlertSeverity.WARNING,
        message: `Memory usage elevated (${usagePercent}%)`,
        usagePercent,
        timestamp: new Date()
      };
      
      this.addAlert(alert);
    }
  }

  /**
   * Add alert to the list
   * @param {object} alert - Alert object
   */
  addAlert(alert) {
    this.alerts.unshift(alert);
    
    // Trim to max size
    if (this.alerts.length > this.maxAlerts) {
      this.alerts = this.alerts.slice(0, this.maxAlerts);
    }
    
    logger.warn(`Alert: ${alert.message}`);
    
    // Notify listeners
    this.notifyListeners(alert);
    
    // Send webhook if enabled
    if (this.webhookEnabled) {
      this.sendWebhook(alert);
    }
  }

  /**
   * Get recent alerts
   * @param {number} limit - Number of alerts to return
   * @returns {Array}
   */
  getAlerts(limit = 20) {
    return this.alerts.slice(0, limit);
  }

  /**
   * Get alerts by severity
   * @param {string} severity - Severity level
   * @returns {Array}
   */
  getAlertsBySeverity(severity) {
    return this.alerts.filter(a => a.severity === severity);
  }

  /**
   * Clear all alerts
   */
  clear() {
    this.alerts = [];
    this.failureCounts.clear();
    logger.info('All alerts cleared');
  }

  /**
   * Register alert listener
   * @param {string} id - Listener identifier
   * @param {Function} callback - Callback function
   */
  onAlert(id, callback) {
    this.listeners.set(id, callback);
  }

  /**
   * Remove alert listener
   * @param {string} id - Listener identifier
   */
  removeListener(id) {
    this.listeners.delete(id);
  }

  /**
   * Notify all listeners of new alert
   * @param {object} alert - Alert object
   */
  notifyListeners(alert) {
    for (const callback of this.listeners.values()) {
      try {
        callback(alert);
      } catch (error) {
        logger.error(`Alert listener error: ${error.message}`);
      }
    }
  }

  /**
   * Send alert to webhook
   * @param {object} alert - Alert object
   */
  async sendWebhook(alert) {
    if (!this.webhookUrl) return;
    
    try {
      await fetch(this.webhookUrl, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          type: 'health_alert',
          alert: {
            id: alert.id,
            severity: alert.severity,
            message: alert.message,
            timestamp: alert.timestamp.toISOString(),
            data: alert
          }
        })
      });
      logger.debug(`Webhook sent for alert: ${alert.id}`);
    } catch (error) {
      logger.error(`Failed to send webhook: ${error.message}`);
    }
  }

  /**
   * Get alert statistics
   * @returns {object}
   */
  getStats() {
    return {
      total: this.alerts.length,
      bySeverity: {
        info: this.alerts.filter(a => a.severity === AlertSeverity.INFO).length,
        warning: this.alerts.filter(a => a.severity === AlertSeverity.WARNING).length,
        critical: this.alerts.filter(a => a.severity === AlertSeverity.CRITICAL).length
      },
      byType: this.getAlertTypes()
    };
  }

  /**
   * Get alert type counts
   * @returns {object}
   */
  getAlertTypes() {
    const types = {};
    for (const alert of this.alerts) {
      types[alert.type] = (types[alert.type] || 0) + 1;
    }
    return types;
  }
}

/**
 * Default alerts manager instance
 */
let healthAlertsManager = null;

/**
 * Initialize health alerts manager
 * @param {object} options - Manager options
 * @returns {HealthAlertsManager}
 */
export function initHealthAlerts(options = {}) {
  if (healthAlertsManager) {
    return healthAlertsManager;
  }
  
  healthAlertsManager = new HealthAlertsManager(options);
  return healthAlertsManager;
}

/**
 * Get health alerts manager instance
 * @returns {HealthAlertsManager|null}
 */
export function getHealthAlerts() {
  return healthAlertsManager;
}

/**
 * Record status change (convenience function)
 * @param {string} previous - Previous status
 * @param {string} current - Current status
 * @param {object} health - Health data
 */
export function recordStatusChange(previous, current, health) {
  if (healthAlertsManager) {
    healthAlertsManager.recordStatusChange(previous, current, health);
  }
}

/**
 * Record provider failure (convenience function)
 * @param {string} providerId - Provider identifier
 * @param {number} failureRate - Failure rate
 */
export function recordProviderFailure(providerId, failureRate) {
  if (healthAlertsManager) {
    healthAlertsManager.recordProviderFailure(providerId, failureRate);
  }
}

export default {
  HealthAlertsManager,
  AlertSeverity,
  initHealthAlerts,
  getHealthAlerts,
  recordStatusChange,
  recordProviderFailure
};
