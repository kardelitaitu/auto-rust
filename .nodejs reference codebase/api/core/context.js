/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { AsyncLocalStorage } from "node:async_hooks";
import { GhostCursor } from "../utils/ghostCursor.js";
import { getDefaultState, setContextStore } from "./context-state.js";
import { APIEvents } from "./events.js";
import { PluginManager } from "./plugins/manager.js";
import { loggerContext } from "./logger.js";
import { randomUUID } from "crypto";
import {
  ContextNotInitializedError,
  PageClosedError,
  SessionDisconnectedError,
} from "./errors.js";

const contextStore = new AsyncLocalStorage();
setContextStore(contextStore);

// Global cache for session stores, indexed by Page instance
const sessionCache = new WeakMap();

class ClipboardLock {
  constructor() {
    this._queue = Promise.resolve();
    this._release = null;
  }

  async acquire() {
    this._queue = this._queue.then(
      () =>
        new Promise((resolve) => {
          this._release = resolve;
        }),
    );
    return this._queue;
  }

  release() {
    if (this._release) {
      this._release();
      this._release = null;
    }
  }

  async runExclusive(fn) {
    await this.acquire();
    try {
      return await fn();
    } finally {
      this.release();
    }
  }
}

function createStore(page) {
  if (sessionCache.has(page)) {
    const store = sessionCache.get(page);
    if (!store._cleanupRegistered) {
      _registerPageCleanupHandler(page);
      store._cleanupRegistered = true;
    }
    return store;
  }

  const events = new APIEvents();
  const plugins = new PluginManager(events);

  const store = {
    page,
    cursor: new GhostCursor(page),
    state: getDefaultState(),
    events,
    plugins,
    intervals: new Map(),
    clipboardLock: new ClipboardLock(),
  };

  _registerPageCleanupHandler(page);
  store._cleanupRegistered = true;
  sessionCache.set(page, store);
  return store;
}

function _cleanupStore(page) {
  const store = sessionCache.get(page);
  if (!store) return;

  if (store._destroyed) return;
  store._destroyed = true;

  if (store.intervals) {
    for (const [, id] of store.intervals) clearInterval(id);
    store.intervals.clear();
  }

  if (store.events) store.events.removeAllListeners();

  if (store.plugins?.destroy) store.plugins.destroy();

  if (store.clipboardLock) {
    store.clipboardLock._queue = Promise.resolve();
    store.clipboardLock._release = null;
  }

  store.state = null;
  store.cursor = null;
  store.page = null;

  sessionCache.delete(page);
}

function _registerPageCleanupHandler(page) {
  page.on("close", async () => {
    try {
      _cleanupStore(page);
      clearContext();
    } catch (e) {
      console.error("[Context] Error during page close cleanup:", e);
    }
  });
}

/**
 * Get a session-bound interval.
 * @param {string} name - Interval identifier
 * @returns {number|null}
 */
export function getInterval(name) {
  const store = contextStore.getStore();
  return store?.intervals?.get(name) || null;
}

/**
 * Set a session-bound interval.
 * @param {string} name - Interval identifier
 * @param {Function} fn - Function to execute
 * @param {number} ms - Delay in milliseconds
 */
export function setSessionInterval(name, fn, ms) {
  const store = contextStore.getStore();
  if (!store) return;

  // Clear existing if any
  if (store.intervals.has(name)) {
    clearInterval(store.intervals.get(name));
  }

  const id = setInterval(async () => {
    try {
      // Ensure the callback runs within the specific session context
      await contextStore.run(store, async () => {
        // Only execute if the page is still alive
        if (store.page && !store.page.isClosed()) {
          await fn();
        } else {
          // Auto-cleanup if page is dead
          clearInterval(id);
          store.intervals.delete(name);
        }
      });
    } catch (e) {
      // Prevent unhandled rejections from crashing the process
      console.debug(`[SessionInterval Error] ${name}:`, e.message || e);
      // If the error is session-related, stop the interval
      if (
        e.message?.includes("SessionDisconnectedError") ||
        e.message?.includes("closed")
      ) {
        clearInterval(id);
        store.intervals.delete(name);
      }
    }
  }, ms);

  store.intervals.set(name, id);
  return id;
}

/**
 * Clear a session-bound interval.
 * @param {string} name - Interval identifier
 */
export function clearSessionInterval(name) {
  const store = contextStore.getStore();
  if (!store) return;

  const id = store.intervals.get(name);
  if (id) {
    clearInterval(id);
    store.intervals.delete(name);
  }
}

/**
 * Get the current context store.
 * @returns {object|null}
 */
/**
 * Get the current context store (session-bound).
 * @returns {object|null} The store object or null if no context is active.
 */
export function getStore() {
  return contextStore.getStore();
}

/**
 * Executes a function within an isolated page context.
 * Best practice for orchestrating multiple concurrent agents.
 *
 * @param {import('playwright').Page} page - Playwright page instance
 * @param {Function} asyncFn - Async function to execute with this page bound
 * @param {object} [options] - Context options
 * @param {string} [options.taskName] - Task name for logging context
 * @param {string} [options.sessionId] - Session ID for logging context
 * @returns {Promise<any>}
 */
export async function withPage(page, asyncFn, options = {}) {
  if (!page)
    throw new Error("withPage requires a valid Playwright page instance.");

  // Logging context integration
  const existingLoggerContext = loggerContext.getStore();
  const sessionId =
    options.sessionId ||
    existingLoggerContext?.sessionId ||
    `session-${randomUUID().slice(0, 8)}`;
  const traceId = existingLoggerContext?.traceId || randomUUID();
  const taskName = options.taskName || existingLoggerContext?.taskName;

  const runWithLogger = (fn) =>
    loggerContext.run({ sessionId, traceId, taskName }, fn);

  // If we are already in a context for THIS page, just continue but with potentially updated logger context
  const existingStore = getStore();
  if (existingStore && existingStore.page === page) {
    return await runWithLogger(asyncFn);
  }

  const store = createStore(page);
  return runWithLogger(() => contextStore.run(store, asyncFn));
}

/**
 * Check if the current session is active (page not closed, browser connected).
 * @returns {boolean} True if session is active.
 */
export function isSessionActive() {
  const store = contextStore.getStore();
  if (!store || !store.page) return false;
  return (
    !store.page.isClosed() &&
    store.page.context().browser()?.isConnected() !== false
  );
}

/**
 * Verify session health and throw descriptive error if dead.
 * @throws {ContextNotInitializedError}
 * @throws {PageClosedError}
 * @throws {SessionDisconnectedError}
 */
/**
 * Verify session health and throw descriptive error if dead.
 * @throws {ContextNotInitializedError}
 * @throws {PageClosedError}
 * @throws {SessionDisconnectedError}
 */
export function checkSession() {
  const store = contextStore.getStore();
  if (!store || !store.page) {
    throw new ContextNotInitializedError();
  }
  if (store.page.isClosed()) {
    throw new PageClosedError();
  }
  if (store.page.context().browser()?.isConnected() === false) {
    throw new SessionDisconnectedError();
  }
}

/**
 * Get the active Playwright page.
 * @returns {import('playwright').Page}
 * @throws {Error} If page context is uninitialized or dead
 */
export function getPage() {
  checkSession();
  return contextStore.getStore().page;
}

/**
 * Evaluate a function in the browser context of the current page.
 * @param {Function} pageFunction - Function to evaluate
 * @param {...any} args - Arguments to pass to the function
 * @returns {Promise<any>} Result of the evaluation
 */
export async function evalPage(pageFunction, ...args) {
  const page = getPage();
  return page.evaluate(pageFunction, ...args);
}

/**
 * Get the GhostCursor tied to the current page.
 * @returns {GhostCursor}
 * @throws {Error} If page context is uninitialized or dead
 */
export function getCursor() {
  checkSession();
  return contextStore.getStore().cursor;
}

/**
 * Get the APIEvents instance tied to the current page.
 * @returns {APIEvents}
 * @throws {Error} If page context is uninitialized or dead
 */
export function getEvents() {
  checkSession();
  return contextStore.getStore().events;
}

/**
 * Get the PluginManager instance tied to the current page.
 * @returns {PluginManager}
 * @throws {Error} If page context is uninitialized or dead
 */
export function getPlugins() {
  checkSession();
  return contextStore.getStore().plugins;
}

/**
 * Get the clipboard lock for atomic clipboard operations.
 * Use this to prevent race conditions when multiple sessions write to clipboard.
 * @returns {Promise<{acquire: Function, release: Function, runExclusive: Function}>}
 */
export function getClipboardLock() {
  const store = contextStore.getStore();
  if (!store) {
    throw new ContextNotInitializedError();
  }
  return store.clipboardLock;
}

/**
 * Tear down the current context.
 * Useful for forceful cleanup, though AsyncLocalStorage garbage collects automatically.
 */
/**
 * Tear down the current context.
 * Useful for forceful cleanup, though AsyncLocalStorage garbage collects automatically.
 */
export function clearContext() {
  const store = contextStore.getStore();
  if (store && store.intervals) {
    for (const [_name, id] of store.intervals) {
      clearInterval(id);
    }
    store.intervals.clear();
  }
  // enterWith(null) clears the current execution tree's store
  contextStore.enterWith(null);
}

/**
 * Destroy a session and clean up all associated resources.
 * @param {import('playwright').Page} page - The page to destroy session for
 */
export function destroySession(page) {
  if (page) {
    _cleanupStore(page);
  }
  clearContext();
}
