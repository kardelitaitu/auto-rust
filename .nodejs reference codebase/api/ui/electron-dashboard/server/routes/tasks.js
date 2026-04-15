/**
 * Task-related routes
 */

import express from 'express';

/**
 * Create tasks router.
 * @param {Object} options - Router options
 * @param {Object} options.server - DashboardServer instance
 * @returns {express.Router}
 */
export function createTasksRouter(options = {}) {
    const router = express.Router();
    const { server } = options;

    router.get('/api/tasks/recent', (req, res) => {
        res.json(server?.latestMetrics?.recentTasks || []);
    });

    router.get('/api/tasks/breakdown', (req, res) => {
        res.json(server?.latestMetrics?.taskBreakdown || {});
    });

    return router;
}
