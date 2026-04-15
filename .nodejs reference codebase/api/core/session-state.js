/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 */

/**
 * @fileoverview Session State - Immutable session storage
 * Prevents cross-session state contamination
 * @module core/session-state
 */

import { createLogger } from "../core/logger.js";

const logger = createLogger("session-state.js");

/**
 * Deep clone to prevent external mutation
 * @param {any} obj
 * @returns {any}
 */
function deepClone(obj) {
  if (obj === null || typeof obj !== "object") {
    return obj;
  }
  if (obj instanceof Date) {
    return new Date(obj.getTime());
  }
  if (obj instanceof Array) {
    return obj.map(deepClone);
  }
  if (obj instanceof Map) {
    const cloned = new Map();
    for (const [key, value] of obj) {
      cloned.set(key, deepClone(value));
    }
    return cloned;
  }
  if (obj instanceof Set) {
    const cloned = new Set();
    for (const value of obj) {
      cloned.add(deepClone(value));
    }
    return cloned;
  }
  const cloned = {};
  for (const key in obj) {
    if (Object.prototype.hasOwnProperty.call(obj, key)) {
      cloned[key] = deepClone(obj[key]);
    }
  }
  return cloned;
}

/**
 * @class SessionState
 * @description Immutable session storage with freeze capability
 */
class SessionState {
  constructor() {
    this._data = new Map();
    this._frozen = false;
  }

  /**
   * Get a value
   * @param {string} key
   * @returns {any}
   */
  get(key) {
    if (this._frozen) {
      throw new Error("Session state is frozen");
    }
    const value = this._data.get(key);
    return value !== undefined ? deepClone(value) : undefined;
  }

  /**
   * Set a value (cloned)
   * @param {string} key
   * @param {any} value
   */
  set(key, value) {
    if (this._frozen) {
      throw new Error("Session state is frozen");
    }
    this._data.set(key, deepClone(value));
  }

  /**
   * Delete a key
   * @param {string} key
   */
  delete(key) {
    if (this._frozen) {
      throw new Error("Session state is frozen");
    }
    this._data.delete(key);
  }

  /**
   * Check if key exists
   * @param {string} key
   * @returns {boolean}
   */
  has(key) {
    return this._data.has(key);
  }

  /**
   * Clear all data
   */
  clear() {
    if (this._frozen) {
      throw new Error("Session state is frozen");
    }
    this._data.clear();
  }

  /**
   * Freeze state - for cleanup
   */
  freeze() {
    this._frozen = true;
    this._data.clear();
    logger.debug("[SessionState] State frozen and cleared");
  }

  /**
   * Check if frozen
   * @returns {boolean}
   */
  isFrozen() {
    return this._frozen;
  }

  /**
   * Get all keys
   * @returns {string[]}
   */
  keys() {
    return Array.from(this._data.keys());
  }

  /**
   * Get state size
   * @returns {number}
   */
  size() {
    return this._data.size;
  }

  /**
   * Export state as plain object
   * @returns {object}
   */
  toJSON() {
    const obj = {};
    for (const [key, value] of this._data) {
      obj[key] = deepClone(value);
    }
    return obj;
  }
}

export default SessionState;
