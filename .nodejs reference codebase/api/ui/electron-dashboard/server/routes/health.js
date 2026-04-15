/**
 * Health Dashboard Route
 * Serves the health dashboard HTML page and provides health status API
 */

import express from 'express';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const DASHBOARD_DIR = join(__dirname, '..', 'health-dashboard');

/**
 * Create health dashboard router
 * @param {object} options - Router options
 * @param {object} options.io - Socket.io instance for client count
 * @returns {express.Router}
 */
export function createHealthDashboardRouter(options = {}) {
  const router = express.Router();
  const io = options.io;

  // Health status API endpoint
  router.get('/health', (req, res) => {
    res.json({
      status: 'ok',
      timestamp: Date.now(),
      clients: io?.sockets?.sockets?.size || 0
    });
  });

  // Serve dashboard HTML at /dashboard
  router.get('/dashboard', (req, res) => {
    res.sendFile(join(DASHBOARD_DIR, 'index.html'));
  });

  // Serve static files (CSS, JS) at /dashboard/*
  router.use('/dashboard', express.static(DASHBOARD_DIR));

  return router;
}

export default {
  createHealthDashboardRouter
};
