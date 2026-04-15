/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * Async Queue Manager for AI Operations
 * Handles race conditions and provides timeout/fallback mechanisms
 * @module utils/async-queue
 */

import { createLogger } from "../core/logger.js";
import { TWITTER_TIMEOUTS } from "../constants/timeouts.js";

export class AsyncQueue {
  constructor(options = {}) {
    this.logger = options.logger || createLogger("async-queue.js");
    this.maxConcurrent = options.maxConcurrent ?? 3;
    this.maxQueueSize = options.maxQueueSize ?? 50;
    this.defaultTimeout =
      options.defaultTimeout ?? TWITTER_TIMEOUTS.QUEUE_ITEM_TIMEOUT;

    this.queue = [];
    this.active = new Map();
    this.processingPromise = null; // Use promise chain instead of boolean flag
    this.processing = false; // Boolean flag for getStatus() compatibility
    this.processedCount = 0;
    this.failedCount = 0;
    this.timedOutCount = 0;

    this.stats = {
      totalAdded: 0,
      totalCompleted: 0,
      totalFailed: 0,
      totalTimedOut: 0,
      averageWaitTime: 0,
      averageProcessingTime: 0,
    };

    this.logger.info(
      `[AsyncQueue] Initialized (maxConcurrent: ${this.maxConcurrent}, maxQueueSize: ${this.maxQueueSize}, defaultTimeout: ${this.defaultTimeout}ms)`,
    );
  }

  /**
   * Add a task to the queue
   * @param {Function} taskFn - Async function to execute
   * @param {object} options - Task options
   * @returns {Promise} Result of the task
   */
  async add(taskFn, options = {}) {
    const id = `task_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
    const timeout = options.timeout ?? this.defaultTimeout;
    const priority = options.priority ?? 0;
    const taskName = options.name || "unnamed";

    // Check queue size with atomic access
    const currentQueueSize = this.queue.length + this.active.size;
    if (currentQueueSize >= this.maxQueueSize) {
      this.logger.warn(
        `[AsyncQueue] Queue full, rejecting task: ${taskName} (current: ${currentQueueSize}, max: ${this.maxQueueSize})`,
      );
      return { success: false, reason: "queue_full", taskName: id };
    }

    // Create promise that will be resolved/rejected when task completes
    const taskPromise = new Promise((resolve, reject) => {
      const startTime = Date.now();
      this.queue.push({
        id,
        taskFn,
        timeout, // Ensure timeout is properly stored
        priority,
        taskName,
        resolve,
        reject,
        enqueueTime: startTime,
      });
      this.stats.totalAdded++;

      // Trigger queue processing (non-blocking synchronous launcher)
      try {
        this._processQueue();
      } catch (err) {
        this.logger.error(
          `[AsyncQueue] Queue processing error: ${err.message}`,
        );
      }
    });

    return taskPromise;
  }

  /**
   * Process items in the queue
   * This acts as a synchronous launcher/re-entrancy guard to start executing items
   */
  _processQueue() {
    if (this.processing) return;
    this.processing = true;

    try {
      while (this.queue.length > 0 && this.active.size < this.maxConcurrent) {
        // Sort by priority (higher priority first)
        this.queue.sort((a, b) => b.priority - a.priority);

        const item = this.queue.shift();
        const startTime = Date.now();

        if (item.taskName !== "unnamed") {
          this.logger.debug(
            `[AsyncQueue] Starting task: ${item.taskName} (queue: ${this.queue.length}, active: ${this.active.size}/${this.maxConcurrent})`,
          );
        }

        // Track active task
        this.active.set(item.id, {
          ...item,
          startTime,
        });

        // Process task with timeout
        let timeoutId;
        const timeoutPromise = new Promise((_, reject) => {
          timeoutId = setTimeout(() => {
            if (item.taskName !== "unnamed") {
              this.logger.warn(
                `[AsyncQueue] ⚠ Task timeout reached: ${item.timeout}ms for ${item.taskName}`,
              );
            }
            reject(new Error("timeout"));
          }, item.timeout);
        });

        // NON BLOCKING: Do not await inside the loop
        Promise.race([this._executeTask(item), timeoutPromise])
          .then((result) => {
            clearTimeout(timeoutId);

            const processingTime = Date.now() - startTime;
            this.stats.totalCompleted++;
            this._updateAverageStats("processing", processingTime);

            if (item.taskName !== "unnamed") {
              this.logger.info(
                `[AsyncQueue] ✓ Completed task: ${item.taskName} in ${processingTime}ms`,
              );
            }
            item.resolve({
              success: true,
              result,
              taskName: item.taskName,
              processingTime,
            });
          })
          .catch((error) => {
            const isTimeout = error.message === "timeout";
            const processingTime = Date.now() - startTime;

            // Get just the first line of the error to avoid huge Playwright traces
            const shortError = error.message
              ? error.message.split("\n")[0]
              : "Unknown error";
            const isExpectedTimeout =
              shortError.includes("Timeout") ||
              shortError.includes(
                "Target page, context or browser has been closed",
              );

            if (isTimeout) {
              this.stats.totalTimedOut++;
              this.timedOutCount++;
              if (item.taskName !== "unnamed") {
                this.logger.warn(
                  `[AsyncQueue] ⚠ Task timed out: ${item.taskName} after ${processingTime}ms`,
                );
              }
            } else {
              this.stats.totalFailed++;
              this.failedCount++;

              if (item.taskName === "unnamed" && isExpectedTimeout) {
                // Expected timeout behavior for background tasks, log silently
                this.logger.debug(
                  `[AsyncQueue] ⚡ Background task expected timeout: ${shortError}`,
                );
              } else {
                this.logger.error(
                  `[AsyncQueue] ✗ Task failed: ${item.taskName} - ${shortError}`,
                );
              }
            }

            item.resolve({
              success: false,
              reason: isTimeout ? "timeout" : "error",
              error: error.message,
              taskName: item.taskName,
              processingTime,
            });
          })
          .finally(() => {
            if (timeoutId) {
              clearTimeout(timeoutId);
            }
            this.active.delete(item.id);
            this.processedCount++;
            this._processQueue();
          });
      }
    } finally {
      this.processing = false;
    }
  }

  /**
   * Execute a task with timeout
   */
  async _executeTask(item) {
    return await item.taskFn();
  }

  /**
   * Create a timeout promise using the item's timeout value
   */
  _createTimeout(item) {
    return new Promise((_, reject) => {
      setTimeout(() => {
        this.logger.warn(
          `[AsyncQueue] Task timeout reached: ${item.timeout}ms for ${item.taskName}`,
        );
        reject(new Error("timeout"));
      }, item.timeout);
    });
  }

  /**
   * Update average statistics
   */
  _updateAverageStats(type, value) {
    const count = this.stats.totalCompleted + this.stats.totalFailed;
    if (count <= 1) {
      if (type === "processing") {
        this.stats.averageProcessingTime = value;
      }
      return;
    }

    if (type === "processing") {
      const total = this.stats.averageProcessingTime * (count - 1) + value;
      this.stats.averageProcessingTime = total / count;
    }
  }

  /**
   * Get queue status
   */
  getStatus() {
    return {
      queueLength: this.queue.length,
      activeCount: this.active.size,
      maxConcurrent: this.maxConcurrent,
      isProcessing: this.processing,
      processed: this.processedCount,
      failed: this.failedCount,
      timedOut: this.timedOutCount,
    };
  }

  /**
   * Get detailed statistics
   */
  getStats() {
    return {
      ...this.stats,
      queueStatus: this.getStatus(),
      utilizationPercent:
        this.active.size > 0
          ? Math.round((this.active.size / this.maxConcurrent) * 100)
          : 0,
    };
  }

  /**
   * Clear the queue
   */
  clear() {
    const dropped = this.queue.length;
    this.queue = [];

    this.logger.info(`[AsyncQueue] Cleared queue (dropped ${dropped} tasks)`);

    return { dropped };
  }

  /**
   * Check if queue is healthy
   */
  isHealthy() {
    return this.failedCount < 5 && this.timedOutCount < 10;
  }

  /**
   * Alias for isHealthy() for external API compatibility
   */
  isQueueHealthy() {
    return this.isHealthy();
  }

  /**
   * Get health status
   */
  getHealth() {
    return {
      healthy: this.isHealthy(),
      failedCount: this.failedCount,
      timedOutCount: this.timedOutCount,
      queueLength: this.queue.length,
    };
  }

  /**
   * Shutdown the queue - clears all pending tasks
   */
  shutdown() {
    this.logger.info(`[AsyncQueue] Shutting down...`);
    this.clear();
    // Since we can't easily cancel running taskPromises, we rely on
    // their internal timeouts or page closures.
    // Clearing the queue prevents new tasks from starting.
  }
}

/**
 * Dive Queue - Specialized queue for tweet dive operations
 * Extends AsyncQueue with dive-specific features
 */
export class DiveQueue extends AsyncQueue {
  constructor(options = {}) {
    // Use passed timeout or default, but ensure it's reasonable
    const timeout = options.defaultTimeout ?? TWITTER_TIMEOUTS.DIVE_TIMEOUT;
    const finalTimeout = Math.max(10000, Math.min(timeout, 300000)); // Clamp between 10s and 5min

    super({
      maxConcurrent: 1, // Force sequential processing - only 1 dive at a time
      maxQueueSize: options.maxQueueSize ?? 30,
      defaultTimeout: finalTimeout,
      logger: options.logger, // Pass logger to parent
    });

    this.fallbackEngagement = options.fallbackEngagement ?? false;
    this.quickMode = false;

    // Engagement tracking
    this.engagementLimits = {
      replies: options.replies ?? 3,
      retweets: options.retweets ?? 1,
      quotes: options.quotes ?? 1,
      likes: options.likes ?? 5,
      follows: options.follows ?? 2,
      bookmarks: options.bookmarks ?? 2,
    };

    this.engagementCounters = {
      replies: 0,
      retweets: 0,
      quotes: 0,
      likes: 0,
      follows: 0,
      bookmarks: 0,
    };

    this.logger.info(
      `[DiveQueue] Initialized with engagement limits: ${JSON.stringify(this.engagementLimits)}`,
    );
  }

  /**
   * Check if engagement limit allows action (synchronous - optimized for performance)
   */
  canEngage(action) {
    const limit = this.engagementLimits[action] ?? Infinity;
    const current = this.engagementCounters[action] ?? 0;
    return current < limit;
  }

  /**
   * Record engagement action (synchronous - optimized for performance)
   */
  recordEngagement(action) {
    if (Object.prototype.hasOwnProperty.call(this.engagementCounters, action)) {
      const limit = this.engagementLimits[action] ?? Infinity;
      const current = this.engagementCounters[action];

      if (current < limit) {
        this.engagementCounters[action]++;
        this.logger.debug(
          `[DiveQueue] ✓ Engagement recorded: ${action} (${current + 1}/${limit})`,
        );
        return true;
      } else {
        this.logger.debug(
          `[DiveQueue] ⚠ Engagement limit reached: ${action} (${current}/${limit})`,
        );
        return false;
      }
    }
    return false;
  }

  /**
   * Get engagement progress (synchronous - optimized for performance)
   */
  getEngagementProgress() {
    const progress = {};
    for (const action of Object.keys(this.engagementLimits)) {
      progress[action] = {
        current: this.engagementCounters[action],
        limit: this.engagementLimits[action],
        remaining: Math.max(
          0,
          this.engagementLimits[action] - this.engagementCounters[action],
        ),
        percentUsed: Math.round(
          (this.engagementCounters[action] / this.engagementLimits[action]) *
            100,
        ),
      };
    }
    return progress;
  }

  /**
   * Update engagement limits at runtime
   */
  updateEngagementLimits(newLimits) {
    this.engagementLimits = { ...this.engagementLimits, ...newLimits };
    this.logger.info(
      `[DiveQueue] Updated engagement limits: ${JSON.stringify(this.engagementLimits)}`,
    );
  }

  /**
   * Add dive task with fallback support
   */
  async addDive(diveFn, fallbackFn, options = {}) {
    const taskName = options.taskName || `dive_${Date.now()}`;
    const timeout = options.timeout ?? this.defaultTimeout;

    this.logger.debug(
      `[DiveQueue] addDive called: ${taskName}, timeout: ${timeout}ms`,
    );

    return this.add(
      async () => {
        try {
          this.logger.debug(`[DiveQueue] Starting dive function: ${taskName}`);

          // Create dive promise with proper timeout closure
          const divePromise = diveFn();

          // Create timeout promise with correct closure variables
          let timeoutId;
          const timeoutPromise = new Promise((_, reject) => {
            timeoutId = setTimeout(() => {
              this.logger.warn(
                `[DiveQueue] Dive timeout reached: ${timeout}ms for ${taskName}`,
              );
              reject(new Error("dive_timeout"));
            }, timeout);
          });

          try {
            const result = await Promise.race([divePromise, timeoutPromise]);
            clearTimeout(timeoutId); // Clear timeout on success

            this.logger.debug(
              `[DiveQueue] Dive function completed: ${taskName}`,
            );
            return {
              success: true,
              result,
              fallbackUsed: false,
              taskName,
            };
          } catch (error) {
            clearTimeout(timeoutId); // Clear timeout on error
            throw error;
          }
        } catch (error) {
          this.logger.warn(
            `[DiveQueue] Dive failed: ${error.message}, checking fallback...`,
          );

          // Check if we should use fallback
          if (this.fallbackEngagement && fallbackFn) {
            try {
              const fallbackResult = await fallbackFn();
              return {
                success: false,
                fallbackUsed: true,
                fallbackResult,
                error: error.message,
                taskName,
              };
            } catch (fallbackError) {
              this.logger.error(
                `[DiveQueue] Fallback also failed: ${fallbackError.message}`,
              );
              return {
                success: false,
                fallbackUsed: false,
                error: `${error.message}; fallback: ${fallbackError.message}`,
                taskName,
              };
            }
          }

          return {
            success: false,
            fallbackUsed: false,
            error: error.message,
            taskName,
          };
        }
      },
      {
        ...options,
        name: taskName,
        timeout: timeout, // Explicitly pass timeout to base add method
      },
    );
  }

  /**
   * Enable quick mode (reduced timeouts for faster fallback)
   */
  enableQuickMode() {
    this.quickMode = true;
    this.defaultTimeout = 15000; // Still faster than normal 20-30s, but enough for AI
    this.logger.info(
      `[DiveQueue] Quick mode enabled (timeout: ${this.defaultTimeout}ms)`,
    );
  }

  /**
   * Disable quick mode
   */
  disableQuickMode() {
    this.quickMode = false;
    this.defaultTimeout = 20000; // Normal timeout
    this.logger.info(
      `[DiveQueue] Quick mode disabled (timeout: ${this.defaultTimeout}ms)`,
    );
  }

  /**
   * Get comprehensive queue status including engagement
   */
  getFullStatus() {
    return {
      queue: this.getStatus(),
      engagement: this.getEngagementProgress(),
      quickMode: this.quickMode,
      health: this.getHealth(),
    };
  }

  /**
   * Reset engagement counters
   */
  resetEngagement() {
    for (const key of Object.keys(this.engagementCounters)) {
      this.engagementCounters[key] = 0;
    }
    this.logger.info(`[DiveQueue] Engagement counters reset`);
  }
}

export default AsyncQueue;
