/**
 * Protected dashboard data route
 */

import express from 'express';
import { requireAuth } from '../middleware/auth.js';

/**
 * Create dashboard router.
 * @param {Object} options - Router options
 * @param {Object} options.server - DashboardServer instance
 * @param {boolean} options.authEnabled - Whether authentication is enabled
 * @param {string} options.authToken - The configured auth token
 * @returns {express.Router}
 */
export function createDashboardRouter(options = {}) {
    const router = express.Router();
    const { server, authEnabled, authToken } = options;

    // Protected endpoint - require authentication when enabled
    router.get('/api/dashboard/data', requireAuth(authEnabled, authToken), (req, res) => {
        res.json(server?.dashboardData || {});
    });

    return router;
}
