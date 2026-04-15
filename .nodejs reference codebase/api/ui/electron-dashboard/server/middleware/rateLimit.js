/**
 * Rate limiting middleware for dashboard server
 */

import { createLogger } from "../../lib/logger.js";

const logger = createLogger("server/middleware/rateLimit.js");

/**
 * Create a rate limiting middleware.
 * @param {Object} options - Rate limit options
 * @param {number} options.windowMs - Time window in milliseconds
 * @param {number} options.maxRequests - Maximum requests per window
 * @returns {Function} - Express middleware function
 */
export function createRateLimit(options = {}) {
  const windowMs = options.windowMs || 60000;
  const maxRequests = options.maxRequests || 100;

  const requests = new Map();

  // Periodic cleanup to prevent memory leak
  const cleanupInterval = setInterval(() => {
    const cutoff = Date.now() - windowMs;
    for (const [key, timestamps] of requests.entries()) {
      const filtered = timestamps.filter((t) => t > cutoff);
      if (filtered.length === 0) {
        requests.delete(key);
      } else {
        requests.set(key, filtered);
      }
    }
  }, windowMs);

  // Allow cleanup interval to be cleared on process exit
  if (cleanupInterval.unref) {
    cleanupInterval.unref();
  }

  return (req, res, next) => {
    const key = req.ip || req.connection.remoteAddress;
    const now = Date.now();
    const windowStart = now - windowMs;

    let clientRequests = requests.get(key) || [];
    clientRequests = clientRequests.filter((t) => t > windowStart);

    if (clientRequests.length >= maxRequests) {
      logger.warn(`Rate limit exceeded for ${key}`);
      return res.status(429).json({ error: "Too many requests" });
    }

    clientRequests.push(now);
    requests.set(key, clientRequests);
    next();
  };
}
