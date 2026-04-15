/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 */

/**
 * @fileoverview Session Container - Complete session lifecycle management
 * Provides guaranteed cleanup and state isolation
 * @module core/session-container
 */

import IntervalManager from "./interval-manager.js";
import LockManager from "./lock-manager.js";
import SessionState from "./session-state.js";
import { createLogger } from "../core/logger.js";
import {
  SessionClosedError,
  SessionDisconnectedError,
} from "../core/errors.js";

const logger = createLogger("session-container.js");

/**
 * @class SessionContainer
 * @description Complete session lifecycle with guaranteed cleanup
 */
class SessionContainer {
  /**
   * @param {string} sessionId
   * @param {object} page - Playwright page
   * @param {object} options
   */
  constructor(sessionId, page, options = {}) {
    this.sessionId = sessionId;
    this.page = page;
    this._closed = false;
    this._createdAt = Date.now();

    this.intervals = new IntervalManager();
    this.locks = new LockManager();
    this.state = new SessionState();

    logger.info(`[SessionContainer] Created: ${sessionId}`);
  }

  /**
   * Verify session is still active - call before every operation
   * @throws {SessionClosedError}
   * @throws {SessionDisconnectedError}
   */
  verify() {
    if (this._closed) {
      throw new SessionClosedError(this.sessionId);
    }
    if (this.page && this.page.isClosed()) {
      throw new SessionDisconnectedError(this.sessionId);
    }
  }

  /**
   * Check if session is closed
   * @returns {boolean}
   */
  isClosed() {
    return this._closed;
  }

  /**
   * Check if page is connected
   * @returns {boolean}
   */
  isConnected() {
    return this.page && !this.page.isClosed();
  }

  /**
   * Get session uptime
   * @returns {number}
   */
  uptime() {
    return Date.now() - this._createdAt;
  }

  /**
   * Acquire a lock for this session
   * @param {string} key
   * @returns {Promise<{release: Function}>}
   */
  async acquireLock(key) {
    return this.locks.acquire(`${this.sessionId}:${key}`);
  }

  /**
   * Set a session-bound interval
   * @param {string} name
   * @param {Function} fn
   * @param {number} ms
   */
  setInterval(name, fn, ms) {
    this.verify();
    return this.intervals.set(name, fn, ms);
  }

  /**
   * Clear a session-bound interval
   * @param {string} name
   */
  clearInterval(name) {
    this.intervals.clear(name);
  }

  /**
   * Store state
   * @param {string} key
   * @param {any} value
   */
  setState(key, value) {
    this.verify();
    this.state.set(key, value);
  }

  /**
   * Get state
   * @param {string} key
   * @returns {any}
   */
  getState(key) {
    this.verify();
    return this.state.get(key);
  }

  /**
   * Close session - GUARANTEED cleanup
   * MUST be called when session ends
   * @returns {Promise<void>}
   */
  async close() {
    if (this._closed) {
      logger.debug(`[SessionContainer] Already closed: ${this.sessionId}`);
      return;
    }

    this._closed = true;
    logger.info(`[SessionContainer] Closing: ${this.sessionId}`);

    try {
      await this.intervals.clearAll();
    } catch (e) {
      logger.error(`[SessionContainer] Interval cleanup error:`, e.message);
    }

    try {
      await this.locks.releaseAll();
    } catch (e) {
      logger.error(`[SessionContainer] Lock release error:`, e.message);
    }

    this.state.freeze();

    if (this.page && !this.page.isClosed()) {
      try {
        await this.page.close();
      } catch (e) {
        logger.debug(`[SessionContainer] Page close error:`, e.message);
      }
    }

    logger.info(`[SessionContainer] Closed: ${this.sessionId}`);
  }

  /**
   * Get status
   * @returns {object}
   */
  getStatus() {
    return {
      sessionId: this.sessionId,
      closed: this._closed,
      connected: this.isConnected(),
      uptime: this.uptime(),
      intervalsCount: this.intervals.size(),
      stateKeys: this.state.keys(),
    };
  }
}

export default SessionContainer;
