/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Robust Orchestrator for the multi-browser automation framework.
 * Features: Task/group timeouts, abort signal propagation, worker health monitoring.
 * @module core/orchestrator
 */

import { EventEmitter } from "events";
import SessionManager from "./sessionManager.js";
import Discovery from "./discovery.js";
import Automator from "./automator.js";
import { api } from "../index.js";
import { createLogger } from "../core/logger.js";
import { getSettings, getTimeoutValue } from "../utils/configLoader.js";
import { validateTaskExecution, validatePayload } from "../utils/validator.js";
import { TaskTimeoutError } from "./errors.js";
import metricsCollector from "../utils/metrics.js";
import {
  TaskStatus,
  createSuccessResult,
  createFailedResult,
  createTimeoutResult,
  formatResult,
} from "./task-result.js";

const logger = createLogger("orchestrator.js");

const DEFAULT_TASK_TIMEOUT_MS = 600000;
const DEFAULT_GROUP_TIMEOUT_MS = 600000;
const DEFAULT_WORKER_WAIT_TIMEOUT_MS = 10000;
const STUCK_WORKER_THRESHOLD_MS = 120000;
const TASK_MODULE_CACHE_MAX_SIZE = 50;
const SESSION_FAILURE_SCORES_MAX_SIZE = 100;
const MEMORY_WARNING_THRESHOLD_MB = 1024;
const MEMORY_CHECK_INTERVAL_MS = 30000;

class Orchestrator extends EventEmitter {
  constructor(options = {}) {
    super();
    this.taskTimeoutMs = options.taskTimeoutMs ?? DEFAULT_TASK_TIMEOUT_MS;
    this.groupTimeoutMs = options.groupTimeoutMs ?? DEFAULT_GROUP_TIMEOUT_MS;
    this.workerWaitTimeoutMs =
      options.workerWaitTimeoutMs ?? DEFAULT_WORKER_WAIT_TIMEOUT_MS;
    this.stuckWorkerThresholdMs =
      options.stuckWorkerThresholdMs ?? STUCK_WORKER_THRESHOLD_MS;

    this.automator = new Automator();
    this.sessionManager = new SessionManager({
      stuckWorkerThresholdMs: this.stuckWorkerThresholdMs,
    });
    this.taskQueue = [];
    this.isProcessingTasks = false;
    this.processTimeout = null;
    this.maxTaskQueueSize = 500;
    this.maxTaskRetries = 2;
    this.sessionFailureScores = new Map();
    this.discovery = new Discovery();
    this.isShuttingDown = false;
    this.currentGroupStartTime = null;
    this.activeTasks = new Map();
    this.taskAbortControllers = new Map();
    this.taskModuleCache = new Map();

    this.dashboardProcess = null;
    this.dashboardSocket = null;
    this.dashboardInterval = null;
    this.memoryCheckInterval = null;

    this.globalActiveTasks = 0;
    this.maxGlobalConcurrency = 20; // Default global limit
    this.taskStaggerDelayMs = 2000; // Delay between task starts
    this.browserConcurrencyMap = {}; // Loaded from config

    this._loadConfig();
    this._startMemoryMonitoring();

    this.automator.onReconnect = async (wsEndpoint, newBrowser) => {
      await this.sessionManager.replaceBrowserByEndpoint(
        wsEndpoint,
        newBrowser,
      );
    };
  }

  _sleep(ms) {
    return new Promise((resolve) => setTimeout(resolve, ms));
  }

  _formatSessionDisplayName(endpointData) {
    const shortNames = {
      localChrome: "chrome",
      localBrave: "brave",
      localEdge: "edge",
      localVivaldi: "vivaldi",
      roxybrowser: "roxy",
      ixbrowser: "ix",
      morelogin: "more",
      undetectable: "und",
    };

    const browserType = endpointData.type || "unknown";
    const shortType = shortNames[browserType] || browserType;
    const actualPort =
      endpointData.port ||
      (endpointData.ws ? new URL(endpointData.ws).port : null);

    if (browserType.startsWith("local")) {
      return actualPort ? `${shortType}:${actualPort}` : shortType;
    }
    return endpointData.windowName
      ? `${shortType}:${endpointData.windowName}`
      : shortType;
  }

  async _loadConfig() {
    try {
      const settings = await getSettings();
      const orchConfig = await getTimeoutValue("orchestration", {});

      this.taskTimeoutMs = orchConfig.taskTimeoutMs ?? this.taskTimeoutMs;
      this.groupTimeoutMs = orchConfig.groupTimeoutMs ?? this.groupTimeoutMs;
      this.workerWaitTimeoutMs =
        orchConfig.workerWaitTimeoutMs ?? this.workerWaitTimeoutMs;
      this.defaultColorScheme = settings?.ui?.dashboard?.colorScheme || null;
      
      if (orchConfig.maxGlobalConcurrency) {
        this.maxGlobalConcurrency = orchConfig.maxGlobalConcurrency;
      }
      
      if (orchConfig.browserConcurrency) {
        this.browserConcurrencyMap = orchConfig.browserConcurrency;
      }
    } catch (_error) {
      logger.warn(`Failed to load orchestrator config: ${_error.message}`);
    }
  }

  async startDiscovery(options = {}) {
    logger.info("[Orchestrator] Starting browser discovery...");

    try {
      await this.sessionManager.loadConfiguration();
      await this.discovery.loadConnectors(options.browsers || []);
      const endpoints = await this.discovery.discoverBrowsers();

      if (endpoints.length === 0) {
        logger.warn("No browser endpoints discovered.");
        return;
      }

      logger.info(`Found ${endpoints.length} browser endpoints. Connecting...`);

      const connectionPromises = endpoints.map((endpointData) => {
        const wsEndpoint = endpointData.ws;
        const browserName = String(
          endpointData.windowName || endpointData.sortNum || "Unnamed Profile",
        );

        if (!wsEndpoint) {
          logger.warn(`Profile ${browserName} has no 'ws' endpoint. Skipping.`);
          return Promise.resolve(null);
        }

        return this.automator
          .connectToBrowser(wsEndpoint)
          .then((browser) => ({
            browser,
            browserName,
            wsEndpoint,
            endpointData,
          }))
          .catch((error) => {
            logger.error(
              `Failed to connect to browser profile ${browserName}: ${error.message}`,
            );
            return null;
          });
      });

      const results = await Promise.allSettled(connectionPromises);

      for (const result of results) {
        if (result.status === "fulfilled" && result.value) {
          const { browser, endpointData } = result.value;

          const displayName = this._formatSessionDisplayName(endpointData);

          this.sessionManager.addSession(
            browser,
            displayName,
            endpointData.ws,
            endpointData.type
          );
          logger.info(`[Orchestrator] Connected: ${displayName} (type: ${endpointData.type || "unknown"})`);
        }
      }

      const connectedCount = this.sessionManager.activeSessionsCount;
      logger.info(
        `[Orchestrator] Discovery complete. Connected to ${connectedCount} browsers.`,
      );

      metricsCollector.recordBrowserDiscovery(
        endpoints.length,
        connectedCount,
        endpoints.length - connectedCount,
      );

      if (connectedCount > 0) {
        this.automator.startHealthChecks();
      }
    } catch (_error) {
      logger.error("Browser discovery failed:", _error.message);
    }
  }

  addTask(taskName, payload = {}) {
    if (!taskName || typeof taskName !== "string") {
      throw new Error("Invalid task name provided");
    }

    const payloadValidation = validatePayload(payload);
    if (!payloadValidation.isValid) {
      throw new Error(
        `Invalid task payload: ${payloadValidation.errors.join(", ")}`,
      );
    }

    if (
      this.maxTaskQueueSize &&
      this.taskQueue.length >= this.maxTaskQueueSize
    ) {
      logger.warn(
        `Task queue full (max: ${this.maxTaskQueueSize}). Dropping task '${taskName}'.`,
      );
      throw new Error(`Task queue full (max: ${this.maxTaskQueueSize})`);
    }

    const effectiveTimeout = payload.timeoutMs ?? this.taskTimeoutMs;
    this.taskQueue.push({ taskName, payload, effectiveTimeout });
    logger.info(
      `[Orchestrator] Task '${taskName}' added (timeout: ${effectiveTimeout}ms). Queue: ${this.taskQueue.length}`,
    );

    if (this.processTimeout) {
      clearTimeout(this.processTimeout);
    }

    this.processTimeout = setTimeout(async () => {
      this.processTimeout = null;
      await this.processTasks();
    }, 50);
  }

  async processTasks() {
    if (this.isProcessingTasks || this.taskQueue.length === 0) {
      this.emit("tasksProcessed");
      return;
    }

    if (this.sessionManager.activeSessionsCount === 0) {
      logger.warn("[Orchestrator] No active sessions available.");
      // Clear queue since no sessions can process them - allows graceful exit
      if (this.taskQueue.length > 0) {
        logger.warn(`[Orchestrator] Clearing ${this.taskQueue.length} queued tasks (no sessions)`);
        this.taskQueue.length = 0;
      }
      this.isProcessingTasks = false;
      this.emit("tasksProcessed");
      return;
    }

    this.isProcessingTasks = true;
    this.currentGroupStartTime = Date.now();
    // logger.info(`[Orchestrator] Processing ${this.taskQueue.length} tasks...`);

    const tasks = [...this.taskQueue];
    this.taskQueue.length = 0;

    const sessions = this.sessionManager.getAllSessions();
    const orderedSessions = [...sessions].sort(
      (a, b) =>
        this._getSessionFailureScore(a.id) - this._getSessionFailureScore(b.id),
    );

    const allSessionPromises = orderedSessions.map((session) => {
      const tasksForSession = tasks.map((task) => ({
        ...task,
        payload: { ...task.payload },
      }));
      return this.processSharedChecklistForSession(session, tasksForSession);
    });

    const results = await Promise.allSettled(allSessionPromises);

    results.forEach((result, index) => {
      if (result.status === "rejected") {
        logger.error(
          `[${orderedSessions[index].id}] Checklist error:`,
          result.reason?.message || result.reason,
        );
      }
    });

    this.isProcessingTasks = false;
    this.currentGroupStartTime = null;
    this._cleanupStaleAbortControllers();

    this.emit("tasksProcessed");

    if (this.taskQueue.length > 0) {
      logger.info("[Orchestrator] New tasks in queue, processing...");
      this.processTasks();
    } else {
      this.emit("allTasksComplete");
    }
  }

  async processSharedChecklistForSession(session, tasks, _options = {}) {
    // logger.info(
    //     `[Orchestrator][${session.id}] Starting checklist: ${tasks.length} tasks, ${session.workers.length} workers`
    // );

    let sharedContext;
    let createdNewContext = false;

    const contexts = session.browser.contexts();
    if (contexts.length > 0) {
      sharedContext = contexts[0];
    } else {
      sharedContext = await session.browser.newContext({
        colorScheme: this.defaultColorScheme || undefined,
      });
      createdNewContext = true;
    }

    try {
      const taskQueue = tasks.slice();
      let taskIndex = 0;
      const retryStack = [];

      const takeTask = () => {
        if (retryStack.length > 0) return retryStack.pop();
        if (taskIndex >= taskQueue.length) return null;
        return taskQueue[taskIndex++];
      };

      const requeueTask = (task) => retryStack.push(task);

      const workerPromises = session.workers.map(async (_worker) => {
        while (true) {
          if (this.isShuttingDown) break;

          if (!session.browser.isConnected()) {
            logger.error(`[Orchestrator][${session.id}] Browser disconnected.`);
            this.sessionManager.markSessionFailed(session.id);
            break;
          }

          if (this._isGroupTimeoutExceeded()) {
            logger.warn(
              `[Orchestrator][${session.id}] Group timeout exceeded. Stopping.`,
            );
            break;
          }

          const task = takeTask();
          if (!task) break;

          // Global Concurrency Throttling
          while (this.globalActiveTasks >= this.maxGlobalConcurrency) {
            await new Promise((resolve) => this.once("workerFreed", resolve));
            if (this.isShuttingDown) break;
          }
          if (this.isShuttingDown) break;

          const allocatedWorker = await this.sessionManager.acquireWorker(
            session.id,
            {
              timeoutMs: this.workerWaitTimeoutMs,
            },
          );
          if (!allocatedWorker) {
            logger.warn(
              `[Orchestrator][${session.id}] No idle workers. Requeuing task '${task.taskName}'`,
            );
            requeueTask(task);
            await this._sleep(5000);
            continue;
          }

          this.globalActiveTasks++;
          // Stagger task starts to prevent network spikes
          // await this._sleep(this.taskStaggerDelayMs); // Disabled: Causes massive queue artificial latency

          let page = null;
          try {
            page = await this.sessionManager.acquirePage(
              session.id,
              sharedContext,
            );

            // Optimization: Removed aggressive checkPageResponsive before EVERY task.
            // We rely on task failure natural retries to rotate dead pages instead of a proactive CDP round-trip.

            // logger.info(
            //     `[Orchestrator][${session.id}][Worker ${allocatedWorker.id}] Starting '${task.taskName}'`
            // );
            await this.executeTask(task, page, session);
            session.completedTaskCount = (session.completedTaskCount || 0) + 1;
          } catch (e) {
            logger.error(
              `[Orchestrator][${session.id}][Worker ${allocatedWorker.id}] Task error:`,
              e.message,
            );
            this._recordSessionOutcome(session.id, false);

            if (task.retriesLeft === undefined)
              task.retriesLeft = this.maxTaskRetries;
            if (task.retriesLeft > 0) {
              task.retriesLeft--;
              logger.warn(
                `[Orchestrator][${session.id}] Retrying '${task.taskName}'. Left: ${task.retriesLeft}`,
              );
              requeueTask(task);
            } else {
              logger.error(
                `[Orchestrator][${session.id}] Task '${task.taskName}' failed permanently.`,
              );
              this.emit("taskFailed", {
                sessionId: session.id,
                task,
                error: e,
              });
            }
          } finally {
            this.globalActiveTasks = Math.max(0, this.globalActiveTasks - 1);
            this.emit("workerFreed"); // Wake up any throttled loops
            if (page) await this.sessionManager.releasePage(session.id, page);
            
            // Clear context state between tasks for isolation
            if (sharedContext && !this.isShuttingDown) {
              try {
                await sharedContext.clearCookies();
                await sharedContext.clearPermissions();
              } catch (e) {
                logger.debug(`[Orchestrator][${session.id}] Context clear error: ${e.message}`);
              }
            }
            
            await this.sessionManager.releaseWorker(
              session.id,
              allocatedWorker.id,
            );
          }
        }
      });

      await Promise.all(workerPromises);
    } finally {
      if (!this.isShuttingDown && createdNewContext) {
        try {
          if (sharedContext) {
            await Promise.race([
              sharedContext.close(),
              new Promise((r) => setTimeout(r, 5000)),
            ]);
          }
        } catch (e) {
          logger.warn(
            `[Orchestrator][${session.id}] Error closing context:`,
            e.message,
          );
        }
      }
    }
  }

  async executeTask(task, page, session) {
    const startTime = Date.now();
    const taskId = `${session.id}-${task.taskName}-${startTime}`;
    let result = null;

    const abortController = new AbortController();
    this.taskAbortControllers.set(taskId, abortController);

    session.currentTaskName = task.taskName;
    session.currentProcessing = "Executing...";

    const effectiveTimeout = task.effectiveTimeout ?? this.taskTimeoutMs;

    try {
      const validation = validateTaskExecution(page, task.payload);
      if (!validation.isValid) {
        throw new Error(`Validation failed: ${validation.errors.join(", ")}`);
      }

      const taskModule = await this._importTaskModule(task.taskName);
      if (typeof taskModule.default !== "function") {
        throw new Error(`Task '${task.taskName}' missing default export`);
      }

      const augmentedPayload = {
        ...task.payload,
        browserInfo: session.browserInfo || "unknown",
        abortSignal: abortController.signal,
        taskId: taskId,
      };

      await api.withPage(page, async () => {
        await api.init(page, {
          persona: augmentedPayload.persona || "casual",
          logger: logger,
          colorScheme:
            augmentedPayload.colorScheme || this.defaultColorScheme || null,
        });

        const timeoutPromise = new Promise((_, reject) => {
          const timeoutId = setTimeout(() => {
            reject(new TaskTimeoutError(task.taskName, effectiveTimeout));
          }, effectiveTimeout);

          abortController.signal.addEventListener("abort", () => {
            clearTimeout(timeoutId);
            reject(new Error("Task cancelled"));
          });

          this.activeTasks.set(taskId, { startTime, task, abortController });
        });

        try {
          // Execute task and capture result
          const taskResult = await Promise.race([
            taskModule.default(page, augmentedPayload),
            timeoutPromise,
          ]);

          // If task returned a result, use it; otherwise create success result
          if (
            taskResult &&
            typeof taskResult === "object" &&
            taskResult.status
          ) {
            result = {
              ...taskResult,
              duration: (Date.now() - startTime) / 1000,
            };
          } else {
            result = createSuccessResult(task.taskName, taskResult || {}, {
              startTime,
              duration: (Date.now() - startTime) / 1000,
              sessionId: session.id,
            });
          }

          this._recordSessionOutcome(session.id, true);
        } finally {
          this.activeTasks.delete(taskId);
        }
      });
    } catch (err) {
      logger.error(
        `[Orchestrator] Task '${task.taskName}' error:`,
        err.message,
      );

      if (err instanceof TaskTimeoutError) {
        result = createTimeoutResult(task.taskName, effectiveTimeout, {
          startTime,
          duration: (Date.now() - startTime) / 1000,
          sessionId: session.id,
        });
        logger.warn(
          `[Orchestrator] Task '${task.taskName}' timed out after ${effectiveTimeout}ms`,
        );
        this.emit("taskTimeout", {
          sessionId: session.id,
          task,
          duration: Date.now() - startTime,
        });
      } else {
        result = createFailedResult(task.taskName, err, {
          startTime,
          duration: (Date.now() - startTime) / 1000,
          sessionId: session.id,
        });
      }
    } finally {
      this.taskAbortControllers.delete(taskId);
      session.currentTaskName = null;
      session.currentProcessing = null;
      metricsCollector.recordTaskExecution(
        task.taskName,
        Date.now() - startTime,
        result?.status === TaskStatus.SUCCESS,
        session.id,
        result?.error,
      );

      // Immediate task-update emission to Dashboard
      const taskUpdate = {
        taskName: task.taskName,
        duration: Date.now() - startTime,
        success: result?.status === TaskStatus.SUCCESS,
        status: result?.status || "unknown",
        sessionId: session.id,
        error: result?.error ? result.error.message : null,
        timestamp: Date.now,
      };

      if (this.dashboardProcess && this.dashboardProcess.connected) {
        try {
          this.dashboardProcess.send({
            type: "task-update",
            payload: taskUpdate,
          });
        } catch (_e) {
          /* ignore */
        }
      }
      if (this.dashboardSocket && this.dashboardSocket.connected) {
        this.dashboardSocket.emit("task-update", taskUpdate);
      }
    }
  }

  _isGroupTimeoutExceeded() {
    if (!this.currentGroupStartTime) return false;
    return Date.now() - this.currentGroupStartTime >= this.groupTimeoutMs;
  }

  _cleanupStaleAbortControllers() {
    const stale = [];
    for (const [taskId, controller] of this.taskAbortControllers) {
      if (controller.signal.aborted) stale.push(taskId);
    }
    stale.forEach((id) => this.taskAbortControllers.delete(id));
  }

  _getSessionFailureScore(sessionId) {
    return this.sessionFailureScores.get(sessionId) || 0;
  }

  _recordSessionOutcome(sessionId, success) {
    const current = this.sessionFailureScores.get(sessionId) || 0;
    if (success) {
      const next = Math.max(0, current - 1);
      if (next === 0) this.sessionFailureScores.delete(sessionId);
      else this.sessionFailureScores.set(sessionId, next);
    } else {
      if (this.sessionFailureScores.size >= SESSION_FAILURE_SCORES_MAX_SIZE) {
        const oldestKey = this.sessionFailureScores.keys().next().value;
        this.sessionFailureScores.delete(oldestKey);
        logger.debug(
          `[Orchestrator] Evicted oldest session failure score: ${oldestKey}`,
        );
      }
      this.sessionFailureScores.set(sessionId, current + 1);
    }
  }

  _cleanupStaleActiveTasks() {
    const staleThreshold = 86400000;
    let cleaned = 0;
    for (const [taskId, data] of this.activeTasks) {
      if (
        data.abortController.signal.aborted ||
        Date.now() - data.startTime > staleThreshold
      ) {
        this.activeTasks.delete(taskId);
        cleaned++;
      }
    }
    if (cleaned > 0) {
      logger.debug(`[Orchestrator] Cleaned ${cleaned} stale active tasks`);
    }
  }

  _evictTaskModuleCache() {
    if (this.taskModuleCache.size >= TASK_MODULE_CACHE_MAX_SIZE) {
      const oldestKey = this.taskModuleCache.keys().next().value;
      this.taskModuleCache.delete(oldestKey);
      logger.debug(
        `[Orchestrator] Evicted oldest task module from cache: ${oldestKey}`,
      );
    }
  }

  _getMemoryUsage() {
    const usage = process.memoryUsage();
    return {
      heapUsed: Math.round(usage.heapUsed / 1024 / 1024),
      heapTotal: Math.round(usage.heapTotal / 1024 / 1024),
      external: Math.round(usage.external / 1024 / 1024),
      rss: Math.round(usage.rss / 1024 / 1024),
    };
  }

  _checkMemoryHealth() {
    this._cleanupStaleActiveTasks();
    const mem = this._getMemoryUsage();
    if (mem.heapUsed > MEMORY_WARNING_THRESHOLD_MB) {
      logger.warn(
        `[Orchestrator] High memory usage: ${mem.heapUsed}MB heap used (threshold: ${MEMORY_WARNING_THRESHOLD_MB}MB). ` +
          `Caches: taskModuleCache=${this.taskModuleCache.size}, ` +
          `sessionFailureScores=${this.sessionFailureScores.size}, ` +
          `activeTasks=${this.activeTasks.size}`,
      );
    }
    return mem;
  }

  _startMemoryMonitoring() {
    if (this.memoryCheckInterval) return;

    this.memoryCheckInterval = setInterval(() => {
      this._checkMemoryHealth();
    }, MEMORY_CHECK_INTERVAL_MS);

    const mem = this._getMemoryUsage();
    if (mem.heapUsed > MEMORY_WARNING_THRESHOLD_MB) {
      logger.warn(
        `[Orchestrator] High memory at startup: ${mem.heapUsed}MB heap used`,
      );
    }
  }

  _stopMemoryMonitoring() {
    if (this.memoryCheckInterval) {
      clearInterval(this.memoryCheckInterval);
      this.memoryCheckInterval = null;
      logger.info("[Orchestrator] Memory monitoring stopped");
    }
  }

  async _importTaskModule(taskName) {
    if (this.taskModuleCache.has(taskName)) {
      return this.taskModuleCache.get(taskName);
    }

    const baseName = taskName.replace(".js", "").split("/").pop();
    const possiblePaths = [
      `../tasks/${baseName}.js`,
      `../../tasks/${baseName}.js`,
      `../tasks/${taskName}.js`,
      `../../tasks/${taskName}.js`,
    ];

    for (const path of possiblePaths) {
      try {
        const mod = await import(path);
        this._evictTaskModuleCache();
        this.taskModuleCache.set(taskName, mod);
        return mod;
      } catch (_e) {
        /* ignore */
      }
    }

    // Case-insensitive fallback: scan the tasks/ directory for a matching filename
    try {
      const { readdir } = await import("fs/promises");
      const { resolve, join } = await import("path");
      const { pathToFileURL } = await import("url");
      const tasksDir = resolve(process.cwd(), "tasks");
      const files = await readdir(tasksDir);
      const lowerBase = baseName.toLowerCase();
      const match = files.find(
        (f) =>
          f.toLowerCase().replace(".js", "") === lowerBase && f.endsWith(".js"),
      );
      if (match) {
        const resolvedPath = join(tasksDir, match);
        const fileUrl = pathToFileURL(resolvedPath).href;
        logger.debug(
          `[Orchestrator] Task '${taskName}' resolved via case-insensitive lookup → ${match}`,
        );
        const mod = await import(fileUrl);
        this._evictTaskModuleCache();
        this.taskModuleCache.set(taskName, mod);
        return mod;
      }
    } catch (_e) {
      logger.warn(
        `[Orchestrator] Case-insensitive task lookup failed for '${taskName}': ${_e.message}`,
      );
    }

    throw new Error(`Task module '${taskName}' not found`);
  }

  async waitForTasksToComplete(options = {}) {
    const timeoutMs = options.timeoutMs ?? this.groupTimeoutMs;
    const { timeoutMs: _waitTimeout, onProgress: _onProgress } = options;

    if (this.taskQueue.length === 0 && !this.isProcessingTasks) {
      logger.info("[Orchestrator] Queue empty, resolving immediately");
      return { completed: 0, timedOut: 0, failed: 0, total: 0 };
    }

    return new Promise((resolve, _reject) => {
      const startTime = Date.now();
      let settled = false;

      const checkCompletion = () => {
        if (settled) return;

        if (this.taskQueue.length === 0 && !this.isProcessingTasks) {
          settled = true;
          this.removeListener("tasksProcessed", checkCompletion);
          this.removeListener("allTasksComplete", onComplete);
          logger.info("[Orchestrator] All tasks completed");
          resolve({
            completed: true,
            timedOut: false,
            duration: Date.now() - startTime,
          });
        }
      };

      const onComplete = () => {
        if (settled) return;
        settled = true;
        this.removeListener("tasksProcessed", checkCompletion);
        this.removeListener("allTasksComplete", onComplete);
        logger.info("[Orchestrator] All tasks complete event received");
        resolve({
          completed: true,
          timedOut: false,
          duration: Date.now() - startTime,
        });
      };

      this.on("tasksProcessed", checkCompletion);
      this.on("allTasksComplete", onComplete);

      checkCompletion();

      const _timeoutId = setTimeout(() => {
        if (settled) return;
        settled = true;
        this.removeListener("tasksProcessed", checkCompletion);
        this.removeListener("allTasksComplete", onComplete);

        this._forceCancelAllTasks();

        logger.warn(
          `[Orchestrator] waitForTasksToComplete timed out after ${timeoutMs}ms`,
        );
        resolve({ completed: false, timedOut: true, duration: timeoutMs });
      }, timeoutMs);
    });
  }

  _forceCancelAllTasks() {
    logger.info("[Orchestrator] Force cancelling all active tasks...");
    for (const [_taskId, controller] of this.taskAbortControllers) {
      if (!controller.signal.aborted) {
        controller.abort();
      }
    }
    this.activeTasks.clear();
  }

  async shutdown(force = false) {
    this.isShuttingDown = true;

    if (this.processTimeout) clearTimeout(this.processTimeout);

    if (!force) {
      const result = await this.waitForTasksToComplete({ timeoutMs: 30000 });
      if (result.timedOut) {
        logger.warn(
          "[Orchestrator] Shutdown: tasks still running, forcing cancel",
        );
        this._forceCancelAllTasks();
      }
    } else {
      this._forceCancelAllTasks();
    }

    this._stopMemoryMonitoring();

    if (this.sessionManager) await this.sessionManager.shutdown();
    await this.stopDashboard();
    await this.automator.shutdown();

    this.logMetrics();
    await metricsCollector.generateJsonReport();

    logger.info("[Orchestrator] Orchestrator shutdown complete");
  }

  async startDashboard(port = 3001) {
    try {
      const fs = await import("fs");
      const path = await import("path");
      const http = await import("http");
      const { getSettings } = await import("../utils/configLoader.js");
      const { fork } = await import("child_process");
      const { io } = await import("socket.io-client");

      const uiPath = path.join(
        process.cwd(),
        "api",
        "ui",
        "electron-dashboard",
        "dashboard.js",
      );

      const settings = await getSettings();
      const broadcastIntervalMs =
        settings?.ui?.dashboard?.broadcastIntervalMs || 5000;

      // 1. Check if dashboard is already running
      const isPortBusy = await new Promise((resolve) => {
        const srv = http.createServer();
        srv.once("error", (err) => {
          if (err.code === "EADDRINUSE") resolve(true);
          else resolve(false);
        });
        srv.once("listening", () => {
          srv.close();
          resolve(false);
        });
        srv.listen(port);
      });

      if (isPortBusy) {
        logger.debug(
          `[Orchestrator] Dashboard port ${port} busy. Attempting Socket.io connection...`,
        );
        this.dashboardSocket = io(`http://localhost:${port}`, {
          reconnection: true,
        });
        this.dashboardSocket.on("connect", () => {
          logger.info("[Orchestrator] Connected to dashboard via Socket.io");
        });
        this.dashboardSocket.on("disconnect", () => {
          logger.warn("[Orchestrator] Disconnected from dashboard");
        });
      } else if (fs.existsSync(uiPath)) {
        this.dashboardProcess = fork(uiPath, [], {
          env: {
            ...process.env,
            PORT: port,
            BROADCAST_MS: broadcastIntervalMs,
          },
          detached: true,
          stdio: "ignore",
        });
        this.dashboardProcess.unref(); // Parent can exit without killing child

        this.dashboardProcess.on("error", (err) => {
          logger.error("[Orchestrator] Dashboard Process Error:", err.message);
        });

        this.dashboardProcess.on("exit", (code) => {
          if (code !== 0 && !this.isShuttingDown) {
            logger.warn(
              `[Orchestrator] Dashboard process exited with code ${code}`,
            );
          }
          this.dashboardProcess = null;
        });

        logger.info(
          `[Orchestrator] Persistent dashboard process spawned on port ${port}`,
        );
      }

      // 2. Start the metrics push interval (unified for IPC and Socket)
      if (this.dashboardInterval) clearInterval(this.dashboardInterval);
      this.dashboardInterval = setInterval(() => {
        const payload = {
          sessions: this.getSessionMetrics(),
          queue: this.getQueueStatus(),
          metrics: this.getMetrics(),
          recentTasks: this.getRecentTasks(20),
          taskBreakdown: this.getTaskBreakdown(),
        };

        // Push via IPC if we are the parent
        if (this.dashboardProcess && this.dashboardProcess.connected) {
          try {
            this.dashboardProcess.send({ type: "metrics_tick", payload });
          } catch (_e) {
            /* ignore */
          }
        }

        // Push via Socket if we are a secondary orchestrator or if IPC failed
        if (this.dashboardSocket && this.dashboardSocket.connected) {
          this.dashboardSocket.emit("push_metrics", payload);
        }
      }, broadcastIntervalMs);
    } catch (error) {
      logger.warn(
        "[Orchestrator] Failed to start/connect dashboard:",
        error.message,
      );
    }
  }

  async stopDashboard() {
    if (this.dashboardInterval) {
      clearInterval(this.dashboardInterval);
      this.dashboardInterval = null;
    }

    if (this.dashboardSocket) {
      this.dashboardSocket.removeAllListeners();
      this.dashboardSocket.disconnect();
      this.dashboardSocket = null;
    }

    if (this.dashboardProcess) {
      this.dashboardProcess.removeAllListeners();
    }
    this.dashboardProcess = null;
  }

  getSessionMetrics() {
    try {
      const sessions = this.sessionManager.getAllSessions() || [];
      return sessions.map((session) => ({
        id: session.id,
        name: session.displayName || session.id,
        status: session.browser?.isConnected() ? "online" : "offline",
        activeWorkers: (session.workers || []).filter(
          (w) => w?.status === "busy",
        ).length,
        totalWorkers: (session.workers || []).length,
        completedTasks: session.completedTaskCount || 0,
        taskName: session.currentTaskName || null,
        processing: session.currentProcessing || null,
        browserType: session.browserType || null,
      }));
    } catch (_error) {
      return [];
    }
  }

  getQueueStatus() {
    return {
      queueLength: this.taskQueue.length,
      isProcessing: this.isProcessingTasks,
      activeTaskCount: this.activeTasks.size,
      maxQueueSize: this.maxTaskQueueSize,
      groupElapsedTime: this.currentGroupStartTime
        ? Date.now() - this.currentGroupStartTime
        : 0,
    };
  }

  getMetrics() {
    try {
      const stats = metricsCollector.getStats() || {};
      return {
        ...stats,
        startTime: metricsCollector.metrics?.startTime,
        lastResetTime: metricsCollector.metrics?.lastResetTime,
      };
    } catch (_error) {
      return {};
    }
  }

  getWorkerHealth(sessionId) {
    return this.sessionManager.getWorkerHealth(sessionId);
  }

  getActiveTasks() {
    return Array.from(this.activeTasks.entries()).map(([id, data]) => ({
      taskId: id,
      taskName: data.task.taskName,
      startTime: data.startTime,
      elapsed: Date.now() - data.startTime,
    }));
  }

  getRecentTasks(limit = 10) {
    return metricsCollector.getRecentTasks
      ? metricsCollector.getRecentTasks(limit)
      : [];
  }

  getTaskBreakdown() {
    return metricsCollector.getTaskBreakdown
      ? metricsCollector.getTaskBreakdown()
      : {};
  }

  logMetrics() {
    metricsCollector?.logStats?.();
  }
}

export default Orchestrator;
