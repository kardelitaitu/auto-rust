/**
 * Authentication middleware for dashboard server
 */

import { createLogger } from '../../lib/logger.js';

const logger = createLogger('server/middleware/auth.js');

/**
 * Authenticate a socket event handler.
 * Returns true if auth is disabled, token matches, or no token is configured.
 * @param {Object} data - The event payload (may contain token)
 * @param {boolean} authEnabled - Whether authentication is enabled
 * @param {string} authToken - The configured auth token
 * @returns {boolean} - Whether the request is authenticated
 */
export function isAuthenticated(data, authEnabled, authToken) {
    if (!authEnabled) return true;
    if (!authToken || authToken.length === 0) {
        logger.warn('Auth enabled but no token configured - blocking all requests');
        return false;
    }

    const providedToken = data?.token || data?.authToken;
    return providedToken === authToken;
}

/**
 * Wraps a socket event handler with authentication.
 * @param {string} eventName - The event name for logging
 * @param {Function} handler - The actual handler function
 * @param {boolean} authEnabled - Whether authentication is enabled
 * @param {string} authToken - The configured auth token
 * @returns {Function} - Wrapped handler with auth check
 */
export function withAuth(eventName, handler, authEnabled, authToken) {
    return (data) => {
        if (!isAuthenticated(data, authEnabled, authToken)) {
            logger.warn(`Unauthorized ${eventName} attempt from socket`);
            return;
        }
        // Strip auth fields before passing to handler
        const { token, authToken: _, ...cleanData } = data || {};
        handler(cleanData);
    };
}

/**
 * Express middleware for HTTP endpoint authentication.
 * @param {boolean} authEnabled - Whether authentication is enabled
 * @param {string} authToken - The configured auth token
 * @returns {Function} - Express middleware function
 */
export function requireAuth(authEnabled, authToken) {
    return (req, res, next) => {
        if (!authEnabled) return next();
        const token = req.headers['x-auth-token'] || req.query.token;
        if (!token || token !== authToken) {
            logger.warn(`Unauthorized HTTP request to ${req.path} from ${req.ip}`);
            return res.status(401).json({ error: 'Unauthorized' });
        }
        next();
    };
}
