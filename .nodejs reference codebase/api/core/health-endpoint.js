/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Health Endpoint - Express middleware for health monitoring
 * Provides REST API endpoints for health checks and monitoring
 * @module core/health-endpoint
 */

import {
  healthMonitor,
  getHealth,
  getRecommendedProvider,
} from "./health-monitor.js";
import { getHealthAlerts } from "./health-alerts.js";
import { createLogger } from "./logger.js";

const logger = createLogger("health-endpoint.js");

/**
 * Create Express health endpoints
 *
 * @param {object} app - Express application
 * @param {object} options - Endpoint options
 * @param {string} options.basePath - Base path for health endpoints (default: '/api')
 * @param {boolean} options.authRequired - Require authentication (default: false)
 * @param {Function} options.authMiddleware - Auth middleware function
 *
 * @example
 * import { createHealthEndpoints } from './api/core/health-endpoint.js';
 *
 * const app = express();
 * createHealthEndpoints(app, { basePath: '/api' });
 *
 * // Endpoints created:
 * // GET /api/health - Current health status
 * // GET /api/health/history - Health history
 * // GET /api/health/provider/:id - Provider-specific health
 * // GET /api/health/recommended - Recommended provider
 */
export function createHealthEndpoints(app, options = {}) {
  const basePath = options.basePath || "/api";
  const healthPath = `${basePath}/health`;

  logger.info(`Health endpoints created at ${healthPath}`);

  /**
   * GET /api/health
   * Get current system health status
   */
  app.get(healthPath, (req, res) => {
    try {
      const health = healthMonitor.toJSON();

      // Set status code based on health
      if (health.status === "unhealthy") {
        res.status(503);
      } else if (health.status === "degraded") {
        res.setHeader("X-Health-Status", "degraded");
      }

      res.json(health);
    } catch (error) {
      logger.error("Health endpoint error:", error.message);
      res.status(500).json({
        status: "error",
        error: error.message,
      });
    }
  });

  /**
   * GET /api/health/history
   * Get health history
   * Query params: limit (default: 10)
   */
  app.get(`${healthPath}/history`, (req, res) => {
    try {
      const limit = parseInt(req.query.limit) || 10;
      const history = healthMonitor.getHistory();

      res.json({
        total: history.length,
        limit,
        data: history.slice(-limit),
      });
    } catch (error) {
      logger.error("Health history endpoint error:", error.message);
      res.status(500).json({
        status: "error",
        error: error.message,
      });
    }
  });

  /**
   * GET /api/health/provider/:id
   * Get provider-specific health
   */
  app.get(`${healthPath}/provider/:id`, (req, res) => {
    try {
      const providerId = req.params.id;
      const health = healthMonitor.getCircuitBreakerHealth();
      const providerHealth = health[providerId];

      if (!providerHealth) {
        return res.status(404).json({
          status: "not_found",
          error: `Provider ${providerId} not found`,
        });
      }

      res.json({
        id: providerId,
        ...providerHealth,
      });
    } catch (error) {
      logger.error("Provider health endpoint error:", error.message);
      res.status(500).json({
        status: "error",
        error: error.message,
      });
    }
  });

  /**
   * GET /api/health/recommended
   * Get recommended provider based on health
   */
  app.get(`${healthPath}/recommended`, (req, res) => {
    try {
      const recommended = getRecommendedProvider();

      if (!recommended) {
        return res.json({
          status: "unknown",
          message: "No healthy providers available",
          recommended: null,
        });
      }

      const health = healthMonitor.getCircuitBreakerHealth();
      res.json({
        status: "ok",
        recommended,
        health: health[recommended],
      });
    } catch (error) {
      logger.error("Recommended provider endpoint error:", error.message);
      res.status(500).json({
        status: "error",
        error: error.message,
      });
    }
  });

  /**
   * GET /api/health/summary
   * Get brief health summary (for monitoring dashboards)
   */
  app.get(`${healthPath}/summary`, (req, res) => {
    try {
      const health = getHealth();
      const cbHealth = healthMonitor.getCircuitBreakerHealth();
      const browserHealth = healthMonitor.getBrowserHealth();

      // Count statuses
      const providerStats = { healthy: 0, degraded: 0, unhealthy: 0 };
      for (const h of Object.values(cbHealth)) {
        providerStats[h.status]++;
      }

      const browserStats = { healthy: 0, degraded: 0, unhealthy: 0 };
      for (const h of Object.values(browserHealth)) {
        browserStats[h.status]++;
      }

      res.json({
        overall: health.overall,
        timestamp: health.timestamp,
        providers: providerStats,
        browsers: browserStats,
        system: health.system.memory.status,
      });
    } catch (error) {
      logger.error("Health summary endpoint error:", error.message);
      res.status(500).json({
        status: "error",
        error: error.message,
      });
    }
  });

  /**
   * GET /api/health/alerts
   * Get recent health alerts
   */
  app.get(`${healthPath}/alerts`, (req, res) => {
    try {
      const limit = parseInt(req.query.limit) || 20;
      const alertsManager = getHealthAlerts();
      
      if (!alertsManager) {
        return res.json({
          status: 'ok',
          alerts: [],
          message: 'Alerts not initialized'
        });
      }
      
      const alerts = alertsManager.getAlerts(limit);
      res.json({
        status: 'ok',
        alerts,
        total: alerts.length
      });
    } catch (error) {
      logger.error("Health alerts endpoint error:", error.message);
      res.status(500).json({
        status: 'error',
        error: error.message
      });
    }
  });

  return healthPath;
}

/**
 * Health check middleware for protecting routes
 *
 * Returns 503 if system is unhealthy
 *
 * @param {object} options - Middleware options
 * @param {boolean} options.allowDegraded - Allow degraded status (default: true)
 *
 * @example
 * // Protect critical routes
 * app.post('/api/automation', healthCheckMiddleware(), runAutomation);
 */
export function healthCheckMiddleware(options = {}) {
  const { allowDegraded = true } = options;

  return (req, res, next) => {
    const health = getHealth();

    if (health.overall === "unhealthy") {
      return res.status(503).json({
        status: "unhealthy",
        message: "System is currently unhealthy",
        recommendations: healthMonitor.toJSON().recommendations,
      });
    }

    if (!allowDegraded && health.overall === "degraded") {
      return res.status(503).json({
        status: "degraded",
        message: "System is degraded",
        recommendations: healthMonitor.toJSON().recommendations,
      });
    }

    next();
  };
}

export default {
  createHealthEndpoints,
  healthCheckMiddleware,
};
