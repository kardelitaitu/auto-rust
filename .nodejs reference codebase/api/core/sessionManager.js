/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Session Manager - Robust worker management with health monitoring.
 * @module core/sessionManager
 */

import fs from "fs";
import path from "path";
import { fileURLToPath } from "url";
import Database from "better-sqlite3";
import { createLogger } from "../core/logger.js";
import { getTimeoutValue, getSettings } from "../utils/configLoader.js";
import metricsCollector from "../utils/metrics.js";
import { SESSION_TIMEOUTS } from "../constants/timeouts.js";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const SESSION_DB_FILE = path.join(__dirname, "../../data/sessions.db");
const logger = createLogger("sessionManager.js");

export class SimpleSemaphore {
  constructor(permits) {
    this.permits = permits;
    this.maxPermits = permits;
    this.queue = [];
    this._lock = Promise.resolve();
  }

  async acquire(timeoutMs = null) {
    return (this._lock = this._lock.then(async () => {
      if (this.permits > 0) {
        this.permits--;
        return true;
      }

      return new Promise((resolve) => {
        const entry = { resolve, timer: null, addedAt: Date.now() };

        if (timeoutMs) {
          entry.timer = setTimeout(() => {
            const idx = this.queue.indexOf(entry);
            if (idx !== -1) {
              this.queue.splice(idx, 1);
              logger.debug(`Semaphore acquire timed out after ${timeoutMs}ms`);
              resolve(false);
            }
          }, timeoutMs);
        }

        this.queue.push(entry);
      });
    }));
  }

  release() {
    if (this.queue.length > 0) {
      const { resolve, timer } = this.queue.shift();
      if (timer) clearTimeout(timer);
      resolve(true);
    } else {
      this.permits = Math.min(this.permits + 1, this.maxPermits);
    }
  }

  get availablePermits() {
    return this.permits;
  }

  get queuedCount() {
    return this.queue.length;
  }
}

class SessionManager {
  constructor(options = {}) {
    this.sessions = [];
    this.sessionsMap = new Map();
    this.nextSessionId = 1;

    this.sessionTimeoutMs =
      options.sessionTimeoutMs || SESSION_TIMEOUTS.SESSION_TIMEOUT_MS;
    this.cleanupIntervalMs =
      options.cleanupIntervalMs || SESSION_TIMEOUTS.CLEANUP_INTERVAL_MS;
    this.workerWaitTimeoutMs =
      options.workerWaitTimeoutMs || SESSION_TIMEOUTS.WORKER_WAIT_TIMEOUT_MS;
    this.stuckWorkerThresholdMs =
      options.stuckWorkerThresholdMs ||
      SESSION_TIMEOUTS.STUCK_WORKER_THRESHOLD_MS;
    this.concurrencyPerBrowser = 5;
    this.browserConcurrencyMap = {}; // Will be set from config

    this.workerSemaphores = new Map();
    this.workerOccupancy = new Map();
    this.workerHealthCheckInterval = null;
    this.cleanupInterval = null;

    this._initDatabase();
    this._startWorkerHealthChecks();
    this.loadConfiguration();
  }

  _initDatabase() {
    try {
      const dbDir = path.dirname(SESSION_DB_FILE);
      if (!fs.existsSync(dbDir)) fs.mkdirSync(dbDir, { recursive: true });

      this.db = new Database(SESSION_DB_FILE);
      this.db.pragma("journal_mode = WAL");

      this.db.exec(`
        CREATE TABLE IF NOT EXISTS sessions (
          id TEXT PRIMARY KEY,
          browserInfo TEXT,
          wsEndpoint TEXT,
          workers TEXT,
          createdAt INTEGER,
          lastActivity INTEGER
        );
        CREATE TABLE IF NOT EXISTS metadata (
          key TEXT PRIMARY KEY,
          value TEXT
        );
      `);
      this._cleanupStaleSessions();
      logger.info(
        `[SessionManager] Database initialized at ${SESSION_DB_FILE}`,
      );
    } catch (error) {
      logger.error(`[SessionManager] Database init failed: ${error.message}`);
    }
  }

  _cleanupStaleSessions() {
    if (!this.db) return;
    try {
      const weekAgo = Date.now() - 7 * 24 * 60 * 60 * 1000;
      const result = this.db
        .prepare("DELETE FROM sessions WHERE lastActivity < ?")
        .run(weekAgo);
      if (result.changes > 0) {
        logger.info(
          `[SessionManager] Cleaned up ${result.changes} stale sessions from DB`,
        );
      }
    } catch (error) {
      logger.warn(`[SessionManager] DB cleanup failed: ${error.message}`);
    }
  }

  _getSemaphore(sessionId, permits) {
    if (!this.workerSemaphores.has(sessionId)) {
      this.workerSemaphores.set(
        sessionId,
        new SimpleSemaphore(permits || this.concurrencyPerBrowser),
      );
    }
    return this.workerSemaphores.get(sessionId);
  }

  _startWorkerHealthChecks() {
    if (this.workerHealthCheckInterval) return;

    this.workerHealthCheckInterval = setInterval(async () => {
      await this._checkStuckWorkers();
    }, SESSION_TIMEOUTS.HEALTH_CHECK_INTERVAL_MS);
  }

  async _checkStuckWorkers() {
    const now = Date.now();
    const stuckWorkers = [];

    for (const session of this.sessions) {
      if (!session.browser?.isConnected()) continue;

      for (const worker of session.workers) {
        if (worker.status === "busy" && worker.occupiedAt) {
          const elapsed = now - worker.occupiedAt;
          if (elapsed > this.stuckWorkerThresholdMs) {
            stuckWorkers.push({
              sessionId: session.id,
              workerId: worker.id,
              elapsed,
            });
          }
        }
      }
    }

    if (stuckWorkers.length > 0) {
      logger.warn(
        `[SessionManager] Found ${stuckWorkers.length} stuck workers, force releasing...`,
      );
      for (const sw of stuckWorkers) {
        await this.forceReleaseWorker(sw.sessionId, sw.workerId);
      }
    }
  }

  async loadConfiguration() {
    try {
      const timeouts = await getTimeoutValue("session", {});
      const orchConfig = await getTimeoutValue("orchestration", {});
      const settings = await getSettings();

      this.sessionTimeoutMs = timeouts.timeoutMs || this.sessionTimeoutMs;
      this.cleanupIntervalMs =
        timeouts.cleanupIntervalMs || this.cleanupIntervalMs;
      this.workerWaitTimeoutMs =
        orchConfig.workerWaitTimeoutMs ?? this.workerWaitTimeoutMs;
      this.stuckWorkerThresholdMs =
        orchConfig.workerStuckThresholdMs ?? this.stuckWorkerThresholdMs;
      this.concurrencyPerBrowser =
        orchConfig.concurrencyPerBrowser || settings.concurrencyPerBrowser || this.concurrencyPerBrowser;
      
      if (orchConfig.browserConcurrency) {
        this.browserConcurrencyMap = orchConfig.browserConcurrency;
      }

      logger.info(
        `[SessionManager] Configured: session=${this.sessionTimeoutMs}ms, defaultConcurrency=${this.concurrencyPerBrowser}, browserMap=${Object.keys(this.browserConcurrencyMap).join(", ")}`,
      );
    } catch (error) {
      logger.warn("[SessionManager] Config load failed:", error.message);
    }
  }

  get activeSessionsCount() {
    return this.sessions.filter((s) => s.browser && s.browser.isConnected())
      .length;
  }

  get idleSessionsCount() {
    return this.sessions.filter(
      (s) =>
        s.browser &&
        s.browser.isConnected() &&
        s.workers.some((w) => w.status === "idle"),
    ).length;
  }

  addSession(browser, browserInfo, wsEndpoint, browserType = null) {
    const id = browserInfo || `session-${this.nextSessionId++}`;
    const now = Date.now();

    const existing =
      this.sessionsMap.get(id) ||
      (wsEndpoint ? this._findByEndpoint(wsEndpoint) : null);
    if (existing) {
      logger.info(`[SessionManager] Session ${id} exists, updating browser`);
      existing.browser = browser;
      existing.wsEndpoint = wsEndpoint || existing.wsEndpoint;
      existing.lastActivity = now;
      return id;
    }

    // Determine concurrency based on browser type from config
    let effectiveConcurrency = this.concurrencyPerBrowser;
    if (browserType && this.browserConcurrencyMap[browserType]) {
      effectiveConcurrency = this.browserConcurrencyMap[browserType];
    }

    const workers = Array.from(
      { length: effectiveConcurrency },
      (_, i) => ({
        id: i,
        status: "idle",
        occupiedAt: null,
        acquiredBy: null,
      }),
    );

    const session = {
      id,
      browser,
      browserInfo,
      browserType,
      wsEndpoint: wsEndpoint || browserInfo,
      workers,
      createdAt: now,
      lastActivity: now,
      managedPages: new Set(),
      sharedContext: null,
    };

    this.sessions.push(session);
    this.sessionsMap.set(id, session);
    this._getSemaphore(id, this.concurrencyPerBrowser);

    logger.info(
      `[SessionManager] Session added: ${id} (${this.concurrencyPerBrowser} workers)`,
    );
    this.saveSessionState();
    metricsCollector.recordSessionEvent("created", this.sessions.length);
    return id;
  }

  _findByEndpoint(wsEndpoint) {
    for (const session of this.sessions) {
      if (session.wsEndpoint === wsEndpoint) return session;
    }
    return null;
  }

  async replaceBrowserByEndpoint(wsEndpoint, newBrowser) {
    const session = this.sessions.find((s) => s.wsEndpoint === wsEndpoint);
    if (session) {
      logger.info(`[SessionManager] Replacing browser for ${session.id}`);
      session.browser = newBrowser;
      session.lastActivity = Date.now();
      await this.saveSessionState();
      return true;
    }
    return false;
  }

  async markSessionFailed(sessionId) {
    logger.warn(
      `[SessionManager] Session ${sessionId} marked failed, removing`,
    );
    return await this.removeSession(sessionId);
  }

  async removeSession(sessionId) {
    const session = this.sessionsMap.get(sessionId);
    if (session) {
      const index = this.sessions.indexOf(session);
      try {
        await this.closeManagedPages(session);
      } catch (e) {
        logger.warn(
          `[SessionManager] Error closing pages for ${sessionId}:`,
          e.message,
        );
      }
      try {
        await this.closeSessionBrowser(session);
      } catch (e) {
        logger.warn(
          `[SessionManager] Error closing browser for ${sessionId}:`,
          e.message,
        );
      }
      this.workerSemaphores.delete(sessionId);
      this.sessionsMap.delete(sessionId);
      for (const worker of session.workers || []) {
        this.workerOccupancy.delete(`${sessionId}:${worker.id}`);
      }
      if (index !== -1) this.sessions.splice(index, 1);
      this.saveSessionState();
      metricsCollector.recordSessionEvent("closed", this.sessions.length);
      logger.info(`[SessionManager] Session removed: ${sessionId}`);
      return true;
    }
    return false;
  }

  registerPage(sessionId, page) {
    const session = this.sessionsMap.get(sessionId);
    if (session) session.managedPages.add(page);
  }

  unregisterPage(sessionId, page) {
    const session = this.sessionsMap.get(sessionId);
    if (session) session.managedPages.delete(page);
  }

  _cleanupStalePageRefs(sessionId) {
    const session = this.sessionsMap.get(sessionId);
    if (!session || !session.managedPages) return;

    for (const page of session.managedPages) {
      try {
        if (page.isClosed()) {
          session.managedPages.delete(page);
        }
      } catch (_e) {
        session.managedPages.delete(page);
      }
    }
  }

  async acquirePage(sessionId, context) {
    const session = this.sessionsMap.get(sessionId);
    if (!session || !context) return null;

    this._cleanupStalePageRefs(sessionId);

    const page = await context.newPage();
    this.registerPage(sessionId, page);
    return page;
  }

  async releasePage(sessionId, page) {
    const session = this.sessionsMap.get(sessionId);
    if (!session) return;

    let isPageClosed = false;
    try {
      if (!page || (typeof page.isClosed === "function" && page.isClosed())) {
        isPageClosed = true;
      }
    } catch (_e) {
      isPageClosed = true;
    }

    if (isPageClosed) {
      this.unregisterPage(sessionId, page);
      return;
    }

    await Promise.race([
      page.close().catch((e) => {
        logger.debug(`[SessionManager] Page close error:`, e.message);
      }),
      new Promise((r) => setTimeout(r, SESSION_TIMEOUTS.PAGE_CLOSE_TIMEOUT_MS)),
    ]);
    this.unregisterPage(sessionId, page);
  }

  async acquireWorker(sessionId, options = {}) {
    const session = this.sessionsMap.get(sessionId);
    if (!session) return null;

    const sem = this._getSemaphore(sessionId);
    const timeoutMs = options.timeoutMs ?? this.workerWaitTimeoutMs;

    try {
      if (!(await sem.acquire(timeoutMs))) {
        logger.warn(`[SessionManager] Acquire timeout for ${sessionId}`);
        return null;
      }
    } catch (err) {
      logger.error(
        `[SessionManager] Semaphore error for ${sessionId}:`,
        err.message,
      );
      return null;
    }

    const worker = session.workers.find((w) => w.status === "idle");
    if (!worker) {
      sem.release();
      return null;
    }

    worker.status = "busy";
    worker.occupiedAt = Date.now();
    worker.acquiredBy = `session-${Date.now()}`;
    this.workerOccupancy.set(`${sessionId}:${worker.id}`, {
      startTime: worker.occupiedAt,
    });

    return worker;
  }

  async releaseWorker(sessionId, workerId) {
    const session = this.sessionsMap.get(sessionId);
    if (!session) return;

    const worker = session.workers.find((w) => w.id === workerId);
    if (worker && worker.status === "busy") {
      worker.status = "idle";
      const duration = Date.now() - (worker.occupiedAt || 0);
      worker.occupiedAt = null;
      worker.acquiredBy = null;
      session.lastActivity = Date.now();
      this.workerOccupancy.delete(`${sessionId}:${workerId}`);
      this._getSemaphore(sessionId).release();
      logger.debug(
        `[SessionManager] Released worker ${workerId} in ${sessionId} (${duration}ms)`,
      );
    }
  }

  async forceReleaseWorker(sessionId, workerId) {
    const session = this.sessionsMap.get(sessionId);
    if (!session) return false;

    const worker = session.workers.find((w) => w.id === workerId);
    if (worker) {
      worker.status = "idle";
      worker.occupiedAt = null;
      worker.acquiredBy = null;
      this.workerOccupancy.delete(`${sessionId}:${workerId}`);
      this._getSemaphore(sessionId).release();
      logger.warn(
        `[SessionManager] Force released worker ${workerId} in ${sessionId}`,
      );
      return true;
    }
    return false;
  }

  getWorkerHealth(sessionId) {
    const session = this.sessionsMap.get(sessionId);
    if (!session) return null;

    const now = Date.now();
    let busy = 0,
      idle = 0,
      stuck = 0;
    const workers = session.workers.map((w) => {
      const isStuck =
        w.status === "busy" &&
        w.occupiedAt &&
        now - w.occupiedAt > this.stuckWorkerThresholdMs;
      if (isStuck) stuck++;
      else if (w.status === "busy") busy++;
      else idle++;
      return {
        id: w.id,
        status: w.status,
        occupiedAt: w.occupiedAt,
        elapsed: w.occupiedAt ? now - w.occupiedAt : 0,
        isStuck,
      };
    });

    return {
      sessionId,
      total: workers.length,
      busy,
      idle,
      stuck,
      workers,
    };
  }

  getAllWorkerHealth() {
    return this.sessions.map((s) => this.getWorkerHealth(s.id)).filter(Boolean);
  }

  async saveSessionState() {
    if (!this.db) return;
    try {
      const upsert = this.db.prepare(
        `INSERT OR REPLACE INTO sessions (id, browserInfo, wsEndpoint, workers, createdAt, lastActivity) VALUES (?, ?, ?, ?, ?, ?)`,
      );
      const transaction = this.db.transaction((sessions) => {
        for (const s of sessions) {
          upsert.run(
            s.id,
            s.browserInfo,
            s.wsEndpoint,
            JSON.stringify(s.workers),
            s.createdAt,
            s.lastActivity,
          );
        }
      });
      transaction(this.sessions);

      const updateMeta = this.db.prepare(
        "INSERT OR REPLACE INTO metadata (key, value) VALUES (?, ?)",
      );
      updateMeta.run("nextSessionId", this.nextSessionId.toString());
      updateMeta.run("savedAt", Date.now().toString());
    } catch (e) {
      logger.error(`[SessionManager] Save state failed: ${e.message}`);
    }
  }

  async loadSessionState() {
    if (!this.db) return null;
    try {
      const rows = this.db.prepare("SELECT * FROM sessions").all();
      const meta = this.db
        .prepare("SELECT * FROM metadata")
        .all()
        .reduce((acc, row) => ({ ...acc, [row.key]: row.value }), {});
      logger.info(`[SessionManager] Loaded ${rows.length} sessions from DB`);
      return { rows, meta };
    } catch (e) {
      logger.error(`[SessionManager] Load state failed: ${e.message}`);
      return null;
    }
  }

  async closeManagedPages(session) {
    if (!session?.managedPages) return;
    for (const page of session.managedPages) {
      try {
        if (!page.isClosed()) {
          await Promise.race([
            page.close(),
            new Promise((r) =>
              setTimeout(r, SESSION_TIMEOUTS.PAGE_CLOSE_TIMEOUT_MS),
            ),
          ]);
        }
      } catch (_e) {
        /* ignore close error */
      }
    }
    session.managedPages.clear();
  }

  async closeSessionBrowser(session) {
    if (session?.browser && session.browser.isConnected()) {
      try {
        await Promise.race([
          session.browser.close(),
          new Promise((r) =>
            setTimeout(r, SESSION_TIMEOUTS.PAGE_CLOSE_TIMEOUT_MS),
          ),
        ]);
      } catch (_e) {
        /* ignore close error */
      }
    }
  }

  /**
   * Stop the cleanup timer (for test cleanup)
   */
  stopCleanupTimer() {
    if (this.cleanupInterval) {
      clearInterval(this.cleanupInterval);
      this.cleanupInterval = null;
    }
  }

  /**
   * Clean up timed out sessions
   * @returns {Promise<number>} Number of sessions removed
   */
  async cleanupTimedOutSessions() {
    const now = Date.now();
    const initialCount = this.sessions.length;
    this.sessions = this.sessions.filter(
      (s) => now - s.lastActivity < this.sessionTimeoutMs,
    );
    return initialCount - this.sessions.length;
  }

  async shutdown() {
    logger.info("[SessionManager] SessionManager shutting down...");

    if (this.workerHealthCheckInterval) {
      clearInterval(this.workerHealthCheckInterval);
      this.workerHealthCheckInterval = null;
    }

    for (const session of this.sessions) {
      await this.closeManagedPages(session);
      await this.closeSessionBrowser(session);
    }

    this.sessions = [];
    this.workerSemaphores.clear();
    this.workerOccupancy.clear();

    if (this.db) {
      this.db.close();
      this.db = null;
    }

    logger.info("[SessionManager] SessionManager shutdown complete");
  }

  getAllSessions() {
    return this.sessions;
  }

  getSession(sessionId) {
    return this.sessionsMap.get(sessionId);
  }

  getIdleSession() {
    return this.sessions.find(
      (s) =>
        s.browser &&
        s.browser.isConnected() &&
        s.workers.some((w) => w.status === "idle"),
    );
  }
}

export default SessionManager;
