/**
 * Status and metrics routes
 */

import express from "express";

/**
 * Create status router.
 * @param {Object} options - Router options
 * @param {Object} options.server - DashboardServer instance
 * @returns {express.Router}
 */
export function createStatusRouter(options = {}) {
  const router = express.Router();
  const { server } = options;

  // Status endpoint
  router.get("/api/status", (req, res) => {
    res.json({
      ready: true,
      sessions: server?.latestMetrics?.sessions?.length || 0,
      queue: server?.latestMetrics?.queue?.queueLength || 0,
      timestamp: Date.now(),
    });
  });

  router.get("/api/sessions", (req, res) => {
    res.json(server?.latestMetrics?.sessions || []);
  });

  router.get("/api/queue", (req, res) => {
    res.json(server?.latestMetrics?.queue || {});
  });

  router.get("/api/metrics", (req, res) => {
    res.json(server?.latestMetrics?.metrics || {});
  });

  return router;
}
