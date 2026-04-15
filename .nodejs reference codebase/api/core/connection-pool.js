/**
 * Browser Connection Pool
 *
 * Manages a pool of browser connections for efficient reuse.
 * Reduces connection overhead and improves performance.
 *
 * @module api/core/connection-pool
 */

import { createLogger } from "./logger.js";

const logger = createLogger("connection-pool");

/**
 * Connection states
 */
const ConnectionState = {
  AVAILABLE: "available",
  IN_USE: "in_use",
  UNHEALTHY: "unhealthy",
  CLOSED: "closed",
};

/**
 * Connection wrapper
 * @private
 */
class Connection {
  constructor(browser, wsEndpoint, options = {}) {
    this.browser = browser;
    this.wsEndpoint = wsEndpoint;
    this.state = ConnectionState.AVAILABLE;
    this.createdAt = Date.now();
    this.lastUsed = Date.now();
    this.useCount = 0;
    this.maxUses = options.maxUses || 100;
    this.idleTimeout = options.idleTimeout || 300000; // 5 min
  }

  async acquire() {
    if (this.state !== ConnectionState.AVAILABLE) {
      return false;
    }

    // Check if connection is too old
    if (Date.now() - this.createdAt > this.idleTimeout) {
      logger.debug("Connection expired, marking unhealthy");
      this.state = ConnectionState.UNHEALTHY;
      return false;
    }

    // Check if connection has been used too many times
    if (this.useCount >= this.maxUses) {
      logger.debug("Connection reached max uses, marking unhealthy");
      this.state = ConnectionState.UNHEALTHY;
      return false;
    }

    // Verify browser is still connected
    try {
      const contexts = this.browser.contexts();
      if (!contexts || contexts.length === 0) {
        // Browser may be disconnected
        this.state = ConnectionState.UNHEALTHY;
        return false;
      }
    } catch (error) {
      logger.debug("Browser connection check failed:", error.message);
      this.state = ConnectionState.UNHEALTHY;
      return false;
    }

    this.state = ConnectionState.IN_USE;
    this.lastUsed = Date.now();
    this.useCount++;
    return true;
  }

  async release() {
    if (this.state === ConnectionState.IN_USE) {
      this.state = ConnectionState.AVAILABLE;
    }
  }

  async close() {
    this.state = ConnectionState.CLOSED;
    try {
      await this.browser.close();
    } catch (error) {
      logger.warn("Error closing browser:", error.message);
    }
  }

  isHealthy() {
    return (
      this.state === ConnectionState.AVAILABLE ||
      this.state === ConnectionState.IN_USE
    );
  }
}

/**
 * Connection Pool
 *
 * Manages a pool of browser connections for reuse.
 */
export class ConnectionPool {
  /**
   * Create connection pool
   * @param {object} options - Pool options
   * @param {number} options.minSize - Minimum connections to maintain
   * @param {number} options.maxSize - Maximum connections
   * @param {number} options.acquireTimeout - Timeout for acquiring connection
   * @param {number} options.idleTimeout - Time before idle connection expires
   * @param {Function} options.createConnection - Function to create new connection
   */
  constructor(options = {}) {
    this.minSize = options.minSize || 1;
    this.maxSize = options.maxSize || 10;
    this.acquireTimeout = options.acquireTimeout || 30000;
    this.idleTimeout = options.idleTimeout || 300000;
    this.createConnectionFn = options.createConnection;

    /** @private */
    this.connections = [];
    this.waitingQueue = [];
    this.isClosed = false;
    this.stats = {
      created: 0,
      acquired: 0,
      released: 0,
      failed: 0,
    };

    // Initialize minimum connections
    this._initializeMinConnections();
  }

  /**
   * Get a connection from the pool
   * @returns {Promise<object>} Connection with browser instance
   */
  async getConnection() {
    if (this.isClosed) {
      throw new Error("Connection pool is closed");
    }

    // Try to get available connection
    for (const conn of this.connections) {
      if (await conn.acquire()) {
        logger.debug("Connection acquired from pool");
        this.stats.acquired++;
        return {
          browser: conn.browser,
          wsEndpoint: conn.wsEndpoint,
          release: () => this.releaseConnection(conn),
        };
      }
    }

    // No available connections, try to create new one
    if (this.connections.length < this.maxSize) {
      const conn = await this._createNewConnection();
      if (conn) {
        this.connections.push(conn);
        await conn.acquire();
        this.stats.acquired++;
        return {
          browser: conn.browser,
          wsEndpoint: conn.wsEndpoint,
          release: () => this.releaseConnection(conn),
        };
      }
    }

    // Pool is full, wait for available connection
    logger.debug("Pool exhausted, waiting for connection...");

    return new Promise((resolve, reject) => {
      const timeout = setTimeout(() => {
        reject(new Error("Connection pool timeout"));
        this.stats.failed++;
      }, this.acquireTimeout);

      this.waitingQueue.push({ resolve, reject, timeout });
    });
  }

  /**
   * Release a connection back to the pool
   * @param {Connection} connection - Connection to release
   */
  async releaseConnection(connection) {
    await connection.release();
    this.stats.released++;

    // Process waiting queue
    if (this.waitingQueue.length > 0) {
      const waiter = this.waitingQueue.shift();
      clearTimeout(waiter.timeout);

      if (await connection.acquire()) {
        this.stats.acquired++;
        waiter.resolve({
          browser: connection.browser,
          wsEndpoint: connection.wsEndpoint,
          release: () => this.releaseConnection(connection),
        });
      } else {
        // Connection not healthy, try next waiter
        this.waitingQueue.unshift(waiter);
      }
    }

    logger.debug("Connection released to pool");
  }

  /**
   * Close all connections and shut down pool
   */
  async close() {
    this.isClosed = true;

    // Reject all waiting requests
    for (const waiter of this.waitingQueue) {
      clearTimeout(waiter.timeout);
      waiter.reject(new Error("Connection pool closed"));
    }
    this.waitingQueue = [];

    // Close all connections
    const closePromises = this.connections.map((conn) => conn.close());
    await Promise.all(closePromises);
    this.connections = [];

    logger.info("Connection pool closed");
  }

  /**
   * Get pool statistics
   * @returns {object}
   */
  stats() {
    const available = this.connections.filter(
      (c) => c.state === ConnectionState.AVAILABLE,
    ).length;
    const inUse = this.connections.filter(
      (c) => c.state === ConnectionState.IN_USE,
    ).length;
    const unhealthy = this.connections.filter(
      (c) => c.state === ConnectionState.UNHEALTHY,
    ).length;

    return {
      ...this.stats,
      total: this.connections.length,
      available,
      inUse,
      unhealthy,
      waiting: this.waitingQueue.length,
      maxSize: this.maxSize,
      minSize: this.minSize,
    };
  }

  /**
   * Remove unhealthy connections
   */
  async cleanup() {
    const healthyConnections = [];

    for (const conn of this.connections) {
      if (conn.isHealthy()) {
        healthyConnections.push(conn);
      } else {
        logger.debug("Removing unhealthy connection");
        await conn.close();
      }
    }

    this.connections = healthyConnections;

    // Replenish to min size
    while (this.connections.length < this.minSize) {
      const conn = await this._createNewConnection();
      if (conn) {
        this.connections.push(conn);
      } else {
        break;
      }
    }
  }

  /** @private */
  async _initializeMinConnections() {
    for (let i = 0; i < this.minSize; i++) {
      const conn = await this._createNewConnection();
      if (conn) {
        this.connections.push(conn);
      }
    }
    logger.debug(`Initialized ${this.connections.length} connections`);
  }

  /** @private */
  async _createNewConnection() {
    if (!this.createConnectionFn) {
      return null;
    }

    try {
      const { browser, wsEndpoint } = await this.createConnectionFn();
      const conn = new Connection(browser, wsEndpoint, {
        idleTimeout: this.idleTimeout,
      });
      this.stats.created++;
      logger.debug("Created new connection");
      return conn;
    } catch (error) {
      logger.error("Failed to create connection:", error.message);
      this.stats.failed++;
      return null;
    }
  }
}

/**
 * Default connection pool (singleton)
 */
export const defaultPool = new ConnectionPool({
  minSize: 1,
  maxSize: 5,
  acquireTimeout: 30000,
  idleTimeout: 300000,
  createConnectionFn: null, // Set via setCreateConnection
});

/**
 * Set the connection creation function for default pool
 * @param {Function} fn - Function that returns { browser, wsEndpoint }
 */
export function setCreateConnection(fn) {
  defaultPool.createConnectionFn = fn;
}

/**
 * Get connection from default pool
 * @returns {Promise<object>}
 */
export async function getConnection() {
  return defaultPool.getConnection();
}

/**
 * Release connection to default pool
 * @param {object} connection - Connection to release
 */
export async function releaseConnection(connection) {
  return defaultPool.releaseConnection(connection);
}

/**
 * Create a new connection pool
 * @param {object} options - Pool options
 * @returns {ConnectionPool}
 */
export function createPool(options = {}) {
  return new ConnectionPool(options);
}

export default {
  ConnectionPool,
  Connection,
  ConnectionState,
  defaultPool,
  setCreateConnection,
  getConnection,
  releaseConnection,
  createPool,
};
