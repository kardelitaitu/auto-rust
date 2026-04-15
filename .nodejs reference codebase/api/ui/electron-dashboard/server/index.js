/**
 * Dashboard Server - Main orchestrator module
 *
 * This module orchestrates all server components and provides the main
 * DashboardServer class that was previously in dashboard.js
 */

import { createHash } from "crypto";
import { Server } from "socket.io";
import express from "express";
import { createLogger } from "../lib/logger.js";
import path from "path";
import fs from "fs";
import { fileURLToPath } from "url";
import { createServer } from "http";
import os from "os";
import { HistoryManager } from "../lib/history-manager.js";

// Import modular components
import { quickHash } from "./utils/hashing.js";
import { sanitizeLogString, sanitizeObject } from "./utils/sanitization.js";
import { getSystemMetrics } from "./utils/metrics.js";
import {
  validateTask,
  validateSession,
  validateMetrics,
  validatePayload,
} from "./socket/validation.js";
import { isAuthenticated, withAuth, requireAuth } from "./middleware/auth.js";
import { createRateLimit } from "./middleware/rateLimit.js";
import { createHealthDashboardRouter } from "./routes/health.js";
import { createStatusRouter } from "./routes/status.js";
import { createTasksRouter } from "./routes/tasks.js";
import { createDashboardRouter } from "./routes/dashboard.js";
import { createExportRouter } from "./routes/export.js";
import { setupSocketHandlers } from "./socket/handlers.js";
import { BroadcastManager } from "./socket/broadcast.js";

// Import health monitoring from api/core/
import { getHealth } from "../../../core/health-monitor.js";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const logger = createLogger("server/index.js");
const HISTORY_FILE = path.join(
  __dirname,
  "..",
  "data",
  "dashboard-history.json",
);
const CONFIG_FILE = path.join(__dirname, "..", "config.json");

function loadConfig() {
  try {
    if (fs.existsSync(CONFIG_FILE)) {
      const data = fs.readFileSync(CONFIG_FILE, "utf8");
      return JSON.parse(data);
    }
  } catch (err) {
    console.warn("[Dashboard] Failed to load config:", err.message);
  }
  return {};
}

const config = loadConfig();
const DEFAULT_BROADCAST_MS = config?.broadcast?.intervalMs || 2000;
const DEFAULT_PING_TIMEOUT = config?.broadcast?.pingTimeout || 60000;
const DEFAULT_PING_INTERVAL = config?.broadcast?.pingInterval || 25000;
const DEFAULT_HISTORY_MAX_ITEMS = config?.history?.maxItems || 9999;
const DEFAULT_HISTORY_SAVE_DEBOUNCE = config?.history?.saveDebounceMs || 5000;

const CORS_ENABLED = config?.security?.cors?.enabled ?? true;
const CORS_ORIGINS = config?.security?.cors?.origins || [
  "http://localhost:5173",
  "http://localhost:3001",
];
const RATE_LIMIT_ENABLED = config?.security?.rateLimit?.enabled ?? true;
const RATE_LIMIT_WINDOW_MS = config?.security?.rateLimit?.windowMs || 60000;
const RATE_LIMIT_MAX_REQUESTS = config?.security?.rateLimit?.maxRequests || 100;
const SESSION_TTL_MS = config?.security?.sessionTTL || 300000;

// Authentication configuration
const AUTH_ENABLED = config?.security?.auth?.enabled ?? false;
const AUTH_TOKEN = config?.security?.auth?.token || "";
const PROTECTED_EVENTS = config?.security?.auth?.protectedEvents || [
  "clear-history",
  "send-notification",
  "push_metrics",
  "task-update",
];

/**
 * Dashboard Server class - Main orchestrator
 */
export class DashboardServer {
  constructor(port = 3001, broadcastIntervalMs = DEFAULT_BROADCAST_MS) {
    this.port = port;
    this.server = null;
    this.io = null;
    this.latestMetrics = {
      sessions: [],
      queue: { queueLength: 0, maxQueueSize: 500 },
      metrics: { system: { uptime: 0 } },
      recentTasks: [],
      taskBreakdown: {},
      system: {
        cpu: { usage: 0, cores: 0 },
        memory: { total: 0, used: 0, free: 0, percent: 0 },
        platform: "Unknown",
        hostname: "Unknown",
        uptime: 0,
      },
    };

    this.historyManager = new HistoryManager(
      HISTORY_FILE,
      DEFAULT_HISTORY_MAX_ITEMS,
      DEFAULT_HISTORY_SAVE_DEBOUNCE,
    );

    // Independent dashboard data stores (persist across orchestrator restarts)
    this.dashboardData = {
      sessions: [],
      sessionHistory: [],
      tasks: this.historyManager.getTasks(),
      queue: { queueLength: 0, activeTaskCount: 0 },
      metrics: {},
      recentTasks: this.historyManager.getTasks().slice(-40),
      twitterActions: this.historyManager.getTwitterActions(),
      apiMetrics: this.historyManager.getApiMetrics(),
      browserMetrics: { discovered: 0, connected: 0 },
      queueHistory: [],
      errors: [],
      firstDataTime: Date.now(),
    };

    // Initialize latestMetrics with loaded history data
    this.latestMetrics = {
      sessions: [],
      queue: { queueLength: 0, maxQueueSize: 500 },
      metrics: { system: { uptime: 0 } },
      recentTasks: this.historyManager.getTasks().slice(-40),
      taskBreakdown: {},
      system: {
        cpu: { usage: 0, cores: 0 },
        memory: { total: 0, used: 0, free: 0, percent: 0 },
        platform: "Unknown",
        hostname: "Unknown",
        uptime: 0,
      },
    };

    this.cumulativeMetrics = {
      engineUptimeMs: 0,
      sessionUptimeMs: 0,
      clientConnectTime: 0,
      completedTasks: this.historyManager.getCompletedTasksCount(),
      startTime: Date.now(),
    };

    // Track last seen metrics to calculate deltas from cumulative engine data
    this.lastSeenMetrics = {
      twitter: { actions: {} },
      api: { calls: 0, failures: 0 },
      tasks: { executed: 0, failed: 0 },
    };

    this.broadcastManager = null;
    this.BROADCAST_MS = broadcastIntervalMs;
    this.isShuttingDown = false;

    this.lastActiveCheck = Date.now();
    this.firstClientConnected = false;
  }

  /**
   * Update metrics with new data.
   * @param {Object} payload - Metrics payload
   */
  updateMetrics(payload) {
    if (!payload) return;

    // Merge sessions
    if (payload.sessions) {
      this.latestMetrics.sessions = payload.sessions;
      this.dashboardData.sessions = payload.sessions;
    }

    // Merge queue
    if (payload.queue) {
      this.latestMetrics.queue = {
        ...this.latestMetrics.queue,
        ...payload.queue,
      };
      this.dashboardData.queue = payload.queue;
    }

    // Merge metrics
    if (payload.metrics) {
      this.latestMetrics.metrics = {
        ...this.latestMetrics.metrics,
        ...payload.metrics,
      };
      this.dashboardData.metrics = payload.metrics;
    }

    // Merge recent tasks
    if (payload.recentTasks) {
      this.latestMetrics.recentTasks = payload.recentTasks;
    }

    // Merge task breakdown
    if (payload.taskBreakdown) {
      this.latestMetrics.taskBreakdown = payload.taskBreakdown;
    }

    // Merge cumulative metrics
    if (payload.cumulative) {
      this.cumulativeMetrics = {
        ...this.cumulativeMetrics,
        ...payload.cumulative,
      };
    }

    // Merge errors
    if (payload.errors) {
      this.dashboardData.errors = [
        ...this.dashboardData.errors,
        ...payload.errors,
      ].slice(-100);
    }
  }

  /**
   * Merge task data into dashboard.
   * @param {Object} task - Task to merge
   */
  mergeTaskData(task) {
    if (!task || !task.id) return;

    const existingIndex = this.dashboardData.tasks.findIndex(
      (t) => t.id === task.id,
    );
    if (existingIndex >= 0) {
      this.dashboardData.tasks[existingIndex] = {
        ...this.dashboardData.tasks[existingIndex],
        ...task,
      };
    } else {
      this.dashboardData.tasks.push(task);
    }

    // Keep only last 1000 tasks
    if (this.dashboardData.tasks.length > 1000) {
      this.dashboardData.tasks = this.dashboardData.tasks.slice(-1000);
    }
  }

  /**
   * Start the dashboard server.
   */
  async start() {
    // Check if port is available
    const net = await import("net");
    const isPortAvailable = await new Promise((resolve) => {
      const testServer = net.createServer();
      testServer.once("error", () => resolve(false));
      testServer.once("listening", () => {
        testServer.close();
        resolve(true);
      });
      testServer.listen(this.port);
    });
    if (!isPortAvailable) {
      const err = new Error("Port already in use");
      err.code = "EADDRINUSE";
      throw err;
    }

    try {
      const expressApp = express();

      if (RATE_LIMIT_ENABLED) {
        const rateLimit = createRateLimit({
          windowMs: RATE_LIMIT_WINDOW_MS,
          maxRequests: RATE_LIMIT_MAX_REQUESTS,
        });
        expressApp.use(rateLimit);
      }

      this.server = createServer(expressApp);
      // Always restrict origins - never use wildcard '*' for security
      const corsOrigin = CORS_ENABLED
        ? CORS_ORIGINS
        : ["http://localhost:3001"];
      this.io = new Server(this.server, {
        cors: {
          origin: corsOrigin,
          methods: ["GET", "POST"],
        },
        maxHttpBufferSize: 1e6,
        pingTimeout: DEFAULT_PING_TIMEOUT,
        pingInterval: DEFAULT_PING_INTERVAL,
      });

      logger.info(
        `Dashboard server starting on port ${this.port} (${this.BROADCAST_MS}ms broadcast)`,
      );

      // Setup routes
      expressApp.use(createHealthDashboardRouter({ io: this.io }));
      expressApp.use(createStatusRouter({ server: this }));
      expressApp.use(createTasksRouter({ server: this }));
      expressApp.use(
        createDashboardRouter({
          server: this,
          authEnabled: AUTH_ENABLED,
          authToken: AUTH_TOKEN,
        }),
      );
      expressApp.use(
        createExportRouter({
          server: this,
          authEnabled: AUTH_ENABLED,
          authToken: AUTH_TOKEN,
        }),
      );

      // Serve static React build if exists
      const rendererPath = path.join(__dirname, "..", "renderer");
      const distPath = path.join(rendererPath, "dist");

      if (fs.existsSync(distPath)) {
        expressApp.use(express.static(distPath));
        expressApp.get("*", (req, res) => {
          res.sendFile(path.join(distPath, "index.html"));
        });
      }

      // Setup socket handlers
      setupSocketHandlers({
        io: this.io,
        server: this,
        authEnabled: AUTH_ENABLED,
        authToken: AUTH_TOKEN,
      });

      // Setup broadcast manager
      this.broadcastManager = new BroadcastManager({
        io: this.io,
        server: this,
        broadcastIntervalMs: this.BROADCAST_MS,
        sessionTTL: SESSION_TTL_MS,
      });

      return new Promise((resolve, reject) => {
        this.server.listen(this.port, () => {
          logger.info(`Dashboard server listening on port ${this.port}`);
          resolve();
        });

        this.server.on("error", (error) => {
          logger.error("Dashboard server error:", error);
          reject(error);
        });
      });
    } catch (error) {
      logger.error("Failed to start dashboard server:", error);
      throw error;
    }
  }

  /**
   * Start broadcast interval.
   */
  startBroadcast() {
    if (this.broadcastManager) {
      this.broadcastManager.start();
    }
  }

  /**
   * Resume broadcast after error pause.
   */
  resumeBroadcast() {
    if (this.broadcastManager) {
      this.broadcastManager.resume();
    }
  }

  /**
   * Stop broadcast interval.
   */
  stopBroadcast() {
    if (this.broadcastManager) {
      this.broadcastManager.stop();
    }
  }

  /**
   * Collect metrics for broadcasting.
   * @returns {Object} - Metrics object
   */
  collectMetrics() {
    if (this.broadcastManager) {
      return this.broadcastManager.collectMetrics();
    }
    return {
      error: "Broadcast manager not initialized",
      timestamp: Date.now(),
    };
  }

  /**
   * Send metrics to a specific socket.
   * @param {Object} socket - Socket to send metrics to
   */
  sendMetrics(socket) {
    if (this.broadcastManager) {
      this.broadcastManager.sendMetrics(socket);
    }
  }

  /**
   * Emit event to all connected clients.
   * @param {string} event - Event name
   * @param {Object} data - Event data
   */
  emit(event, data) {
    if (this.io) {
      this.io.emit(event, { timestamp: Date.now(), ...data });
    }
  }

  /**
   * Stop the dashboard server.
   */
  async stop() {
    this.isShuttingDown = true;

    if (this.broadcastManager) {
      this.broadcastManager.stop();
    }

    // Close socket.io first to disconnect clients and clear internal timers
    if (this.io) {
      try {
        this.io.close();
        logger.info("Socket.io closed");
      } catch (error) {
        logger.warn("Error closing Socket.io:", error.message);
      }
      this.io = null;
    }

    // Await server closure
    if (this.server) {
      try {
        await new Promise((resolve, reject) => {
          this.server.close((err) => {
            if (err) reject(err);
            else resolve();
          });
        });
        logger.info("Dashboard server stopped");
      } catch (error) {
        logger.error("Error stopping dashboard server:", error.message);
      }
      this.server = null;
    }
  }

  /**
   * Get current system metrics (CPU, memory, etc.)
   * @returns {Object} System metrics
   */
  getSystemMetrics() {
    if (this.broadcastManager) {
      const metrics = this.broadcastManager.collectMetrics();
      return metrics.system || {};
    }
    return {
      cpu: { usage: 0, cores: 0 },
      memory: { total: 0, used: 0, free: 0, percent: 0 },
    };
  }

  /**
   * Backward compatibility getters for broadcast manager properties
   */
  get broadcastPaused() {
    return this.broadcastManager?.broadcastPaused ?? false;
  }

  set broadcastPaused(value) {
    if (this.broadcastManager) {
      this.broadcastManager.broadcastPaused = value;
    }
  }

  get broadcastInterval() {
    return this.broadcastManager?.broadcastInterval ?? null;
  }

  get consecutiveErrors() {
    return this.broadcastManager?.consecutiveErrors ?? 0;
  }

  set consecutiveErrors(value) {
    if (this.broadcastManager) {
      this.broadcastManager.consecutiveErrors = value;
    }
  }

  get maxConsecutiveErrors() {
    return this.broadcastManager?.maxConsecutiveErrors ?? 5;
  }

  set maxConsecutiveErrors(value) {
    if (this.broadcastManager) {
      this.broadcastManager.maxConsecutiveErrors = value;
    }
  }
}

// Re-export validation and auth functions for backward compatibility
export {
  isAuthenticated,
  withAuth,
  sanitizeLogString,
  sanitizeObject,
  validateTask,
  validateSession,
  validateMetrics,
  validatePayload,
};

// Helper to start the server standalone (e.g., from Electron main process)
export async function startStandaloneServer(port = 3001) {
  const server = new DashboardServer(port);
  try {
    await server.start();
    logger.info(`Standalone Dashboard Server started on port ${port}`);
    return server;
  } catch (err) {
    if (err.code === "EADDRINUSE") {
      logger.warn(
        `Port ${port} already in use. Dashboard server likely already running.`,
      );
      return null;
    }
    throw err;
  }
}
