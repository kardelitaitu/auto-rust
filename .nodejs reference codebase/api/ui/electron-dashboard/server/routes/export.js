/**
 * Export routes for data export functionality
 */

import express from "express";
import { requireAuth } from "../middleware/auth.js";

/**
 * Create export router.
 * @param {Object} options - Router options
 * @param {Object} options.server - DashboardServer instance
 * @param {boolean} options.authEnabled - Whether authentication is enabled
 * @param {string} options.authToken - The configured auth token
 * @returns {express.Router}
 */
export function createExportRouter(options = {}) {
  const router = express.Router();
  const { server, authEnabled, authToken } = options;

  // Export endpoints - require authentication
  router.get(
    "/api/export/json",
    requireAuth(authEnabled, authToken),
    (req, res) => {
      const data = {
        sessions: server?.dashboardData?.sessions,
        tasks: server?.dashboardData?.tasks,
        twitterActions: server?.dashboardData?.twitterActions,
        apiMetrics: server?.dashboardData?.apiMetrics,
        exportedAt: new Date().toISOString(),
      };
      res.setHeader("Content-Type", "application/json");
      res.setHeader(
        "Content-Disposition",
        "attachment; filename=dashboard-export.json",
      );
      res.json(data);
    },
  );

  router.get(
    "/api/export/csv",
    requireAuth(authEnabled, authToken),
    (req, res) => {
      const tasks = server?.dashboardData?.tasks || [];
      const headers = [
        "id",
        "taskName",
        "sessionId",
        "timestamp",
        "status",
        "success",
        "duration",
      ];
      const csvRows = [headers.join(",")];

      for (const task of tasks) {
        const row = headers.map((h) => {
          const val = task[h] ?? "";
          const str = String(val).replace(/"/g, '""');
          return `"${str}"`;
        });
        csvRows.push(row.join(","));
      }

      res.setHeader("Content-Type", "text/csv");
      res.setHeader(
        "Content-Disposition",
        "attachment; filename=tasks-export.csv",
      );
      res.send(csvRows.join("\n"));
    },
  );

  return router;
}
