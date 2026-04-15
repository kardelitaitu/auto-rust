/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview System-wide metrics collection and monitoring.
 * @module utils/metrics
 */

import { createLogger } from "../core/logger.js";

const logger = createLogger("metrics.js");

/**
 * @class MetricsCollector
 * @description A class for tracking system performance and statistics.
 */
class MetricsCollector {
  constructor() {
    this.metrics = {
      tasksExecuted: 0,
      tasksFailed: 0,
      totalTaskDuration: 0,
      taskDurations: [],
      browsersDiscovered: 0,
      browsersConnected: 0,
      browsersFailed: 0,
      apiCalls: 0,
      apiFailures: 0,
      totalApiResponseTime: 0,
      sessionsCreated: 0,
      sessionsActive: 0,
      sessionsClosed: 0,
      // Social action counters
      totalFollows: 0,
      totalLikes: 0,
      totalRetweets: 0,
      totalTweets: 0,
      // Twitter engagement metrics
      totalReplies: 0,
      totalQuotes: 0,
      totalBookmarks: 0,
      // Performance metrics
      totalDiveDuration: 0,
      diveCount: 0,
      diveDurations: [],
      totalAILatency: 0,
      aiRequestCount: 0,
      aiLatencies: [],
      // Error tracking
      errorsByType: {},
      startTime: Date.now(),
      lastResetTime: Date.now(),
    };

    this.taskHistory = [];
    this.maxHistorySize = 100;
    this.maxErrorTypes = 50;
  }

  /**
   * Records a task execution.
   * @param {string} taskName - The name of the task.
   * @param {number} duration - The duration of the task in milliseconds.
   * @param {boolean} success - Whether the task succeeded.
   * @param {string} sessionId - The ID of the session that executed the task.
   * @param {Error} [error=null] - The error object if the task failed.
   */
  recordTaskExecution(taskName, duration, success, sessionId, error = null) {
    this.metrics.tasksExecuted++;
    this.metrics.totalTaskDuration += duration;
    this.metrics.taskDurations.push(duration);

    if (this.metrics.taskDurations.length > 100) {
      this.metrics.taskDurations.shift();
    }

    if (!success) {
      this.metrics.tasksFailed++;
    }

    this.taskHistory.push({
      taskName,
      duration,
      success,
      sessionId,
      error: error ? error.message : null,
      startTime: new Date(Date.now() - duration).toISOString(),
      timestamp: Date.now(),
    });

    if (this.taskHistory.length > this.maxHistorySize) {
      this.taskHistory.shift();
    }

    logger.debug(
      `Task '${taskName}' recorded: ${success ? "SUCCESS" : "FAILED"}, duration: ${duration}ms`,
    );
  }

  /**
   * Records a browser discovery attempt.
   * @param {number} count - The number of browsers discovered.
   * @param {number} connected - The number of browsers that successfully connected.
   * @param {number} failed - The number of browsers that failed to connect.
   */
  recordBrowserDiscovery(count, connected, failed) {
    this.metrics.browsersDiscovered += count;
    this.metrics.browsersConnected += connected;
    this.metrics.browsersFailed += failed;

    logger.debug(
      `Browser discovery: ${count} discovered, ${connected} connected, ${failed} failed`,
    );
  }

  /**
   * Records an API call.
   * @param {number} responseTime - The response time of the API call in milliseconds.
   * @param {boolean} success - Whether the API call succeeded.
   */
  recordApiCall(responseTime, success) {
    this.metrics.apiCalls++;
    this.metrics.totalApiResponseTime += responseTime;

    if (!success) {
      this.metrics.apiFailures++;
    }
  }

  /**
   * Records a session lifecycle event.
   * @param {string} event - The type of event ('created' or 'closed').
   * @param {number} activeCount - The current number of active sessions.
   */
  recordSessionEvent(event, activeCount) {
    if (event === "created") {
      this.metrics.sessionsCreated++;
    } else if (event === "closed") {
      this.metrics.sessionsClosed++;
    }

    this.metrics.sessionsActive = activeCount;
  }

  /**
   * Records a social action (follow, like, retweet, tweet).
   * Safe: validates type and count before incrementing.
   * @param {string} type - The type of action: 'follow', 'like', 'retweet', or 'tweet'.
   * @param {number} [count=1] - The number of actions to record.
   */
  recordSocialAction(type, count = 1) {
    // Validate type
    if (typeof type !== "string") {
      logger.warn(
        `[recordSocialAction] Invalid type: expected string, got ${typeof type}`,
      );
      return;
    }

    // Validate count
    const safeCount =
      typeof count === "number" && !isNaN(count) && count > 0
        ? Math.floor(count)
        : 0;

    if (safeCount === 0) {
      return; // Nothing to record
    }

    const normalizedType = type.toLowerCase().trim();

    switch (normalizedType) {
      case "follow":
        this.metrics.totalFollows += safeCount;
        break;
      case "like":
        this.metrics.totalLikes += safeCount;
        break;
      case "retweet":
        this.metrics.totalRetweets += safeCount;
        break;
      case "tweet":
        this.metrics.totalTweets += safeCount;
        break;
      default:
        logger.warn(`[recordSocialAction] Unknown action type: '${type}'`);
        return;
    }

    //logger.debug(`Social action recorded: ${safeCount} ${normalizedType}(s). Totals: F=${this.metrics.totalFollows}, L=${this.metrics.totalLikes}, R=${this.metrics.totalRetweets}`);
  }

  /**
   * Records Twitter engagement actions (replies, quotes, bookmarks)
   * @param {string} type - The type of action: 'reply', 'quote', 'bookmark'
   * @param {number} [count=1] - The number of actions to record
   */
  recordTwitterEngagement(type, count = 1) {
    if (typeof type !== "string") {
      logger.warn(
        `[recordTwitterEngagement] Invalid type: expected string, got ${typeof type}`,
      );
      return;
    }

    const safeCount =
      typeof count === "number" && !isNaN(count) && count > 0
        ? Math.floor(count)
        : 0;

    if (safeCount === 0) {
      return;
    }

    const normalizedType = type.toLowerCase().trim();

    switch (normalizedType) {
      case "reply":
        this.metrics.totalReplies += safeCount;
        break;
      case "quote":
        this.metrics.totalQuotes += safeCount;
        break;
      case "bookmark":
        this.metrics.totalBookmarks += safeCount;
        break;
      default:
        logger.warn(`[recordTwitterEngagement] Unknown action type: '${type}'`);
        return;
    }

    logger.debug(
      `[recordTwitterEngagement] Recorded ${safeCount} ${normalizedType}(s)`,
    );
  }

  /**
   * Records the duration of a tweet dive operation
   * @param {number} durationMs - Duration in milliseconds
   */
  recordDiveDuration(durationMs) {
    if (typeof durationMs !== "number" || isNaN(durationMs) || durationMs < 0) {
      return;
    }

    this.metrics.totalDiveDuration += durationMs;
    this.metrics.diveCount++;
    this.metrics.diveDurations.push(durationMs);

    if (this.metrics.diveDurations.length > 100) {
      this.metrics.diveDurations.shift();
    }

    logger.debug(
      `[recordDiveDuration] Recorded: ${durationMs}ms (avg: ${this.getAvgDiveDuration()}ms)`,
    );
  }

  /**
   * Records AI request latency
   * @param {number} latencyMs - Latency in milliseconds
   * @param {boolean} success - Whether the request succeeded
   */
  recordAILatency(latencyMs, success = true) {
    if (typeof latencyMs !== "number" || isNaN(latencyMs) || latencyMs < 0) {
      return;
    }

    this.metrics.totalAILatency += latencyMs;
    this.metrics.aiRequestCount++;
    this.metrics.aiLatencies.push(latencyMs);

    if (this.metrics.aiLatencies.length > 100) {
      this.metrics.aiLatencies.shift();
    }

    if (!success) {
      this.recordError("ai_request_failure", "AI request failed");
    }

    logger.debug(
      `[recordAILatency] Recorded: ${latencyMs}ms (avg: ${this.getAvgAILatency()}ms)`,
    );
  }

  /**
   * Records an error with type categorization
   * @param {string} errorType - The type/category of error
   * @param {string} message - Error message
   */
  recordError(errorType, message) {
    if (!this.metrics.errorsByType[errorType]) {
      if (Object.keys(this.metrics.errorsByType).length >= this.maxErrorTypes) {
        const oldestKey = Object.keys(this.metrics.errorsByType)[0];
        delete this.metrics.errorsByType[oldestKey];
      }
      this.metrics.errorsByType[errorType] = { count: 0, messages: [] };
    }
    this.metrics.errorsByType[errorType].count++;
    if (this.metrics.errorsByType[errorType].messages.length < 5) {
      this.metrics.errorsByType[errorType].messages.push(message);
    }
    logger.debug(`[recordError] ${errorType}: ${message}`);
  }

  /**
   * Gets average dive duration in milliseconds
   * @returns {number} Average duration or 0 if no dives recorded
   */
  getAvgDiveDuration() {
    if (this.metrics.diveCount === 0) return 0;
    return Math.round(this.metrics.totalDiveDuration / this.metrics.diveCount);
  }

  /**
   * Gets average AI latency in milliseconds
   * @returns {number} Average latency or 0 if no requests recorded
   */
  getAvgAILatency() {
    if (this.metrics.aiRequestCount === 0) return 0;
    return Math.round(
      this.metrics.totalAILatency / this.metrics.aiRequestCount,
    );
  }

  /**
   * Gets Twitter-specific engagement metrics
   * @returns {object} Engagement statistics
   */
  getTwitterEngagementMetrics() {
    const totalEngagements =
      this.metrics.totalLikes +
      this.metrics.totalReplies +
      this.metrics.totalQuotes +
      this.metrics.totalRetweets +
      this.metrics.totalBookmarks +
      this.metrics.totalFollows;

    return {
      actions: {
        likes: this.metrics.totalLikes,
        replies: this.metrics.totalReplies,
        quotes: this.metrics.totalQuotes,
        retweets: this.metrics.totalRetweets,
        bookmarks: this.metrics.totalBookmarks,
        follows: this.metrics.totalFollows,
        tweets: this.metrics.totalTweets,
        total: totalEngagements,
      },
      performance: {
        avgDiveDuration: this.getAvgDiveDuration(),
        diveCount: this.metrics.diveCount,
        avgAILatency: this.getAvgAILatency(),
        aiRequestCount: this.metrics.aiRequestCount,
      },
      errors: { ...this.metrics.errorsByType },
    };
  }

  /**
   * Gets comprehensive statistics about the system.
   * @returns {object} An object containing system statistics.
   */
  getStats() {
    const now = Date.now();
    const uptime = now - this.metrics.startTime;
    const timeSinceReset = now - this.metrics.lastResetTime;

    const successRate =
      this.metrics.tasksExecuted > 0
        ? (
            ((this.metrics.tasksExecuted - this.metrics.tasksFailed) /
              this.metrics.tasksExecuted) *
            100
          ).toFixed(2)
        : 0;

    const avgTaskDuration =
      this.metrics.tasksExecuted > 0
        ? (this.metrics.totalTaskDuration / this.metrics.tasksExecuted).toFixed(
            2,
          )
        : 0;

    const avgApiResponseTime =
      this.metrics.apiCalls > 0
        ? (this.metrics.totalApiResponseTime / this.metrics.apiCalls).toFixed(2)
        : 0;

    const apiSuccessRate =
      this.metrics.apiCalls > 0
        ? (
            ((this.metrics.apiCalls - this.metrics.apiFailures) /
              this.metrics.apiCalls) *
            100
          ).toFixed(2)
        : 0;

    const sortedDurations = [...this.metrics.taskDurations].sort(
      (a, b) => a - b,
    );
    const p50 = this.getPercentile(sortedDurations, 50);
    const p95 = this.getPercentile(sortedDurations, 95);
    const p99 = this.getPercentile(sortedDurations, 99);

    return {
      system: {
        uptime,
        uptimeFormatted: this.formatDuration(uptime),
        timeSinceReset,
        timeSinceResetFormatted: this.formatDuration(timeSinceReset),
      },
      tasks: {
        executed: this.metrics.tasksExecuted,
        failed: this.metrics.tasksFailed,
        succeeded: this.metrics.tasksExecuted - this.metrics.tasksFailed,
        successRate: parseFloat(successRate),
        avgDuration: parseFloat(avgTaskDuration),
        totalDuration: this.metrics.totalTaskDuration,
        durationPercentiles: {
          p50,
          p95,
          p99,
        },
      },
      browsers: {
        discovered: this.metrics.browsersDiscovered,
        connected: this.metrics.browsersConnected,
        failed: this.metrics.browsersFailed,
        connectionRate:
          this.metrics.browsersDiscovered > 0
            ? (
                (this.metrics.browsersConnected /
                  this.metrics.browsersDiscovered) *
                100
              ).toFixed(2)
            : 0,
      },
      api: {
        calls: this.metrics.apiCalls,
        failures: this.metrics.apiFailures,
        successRate: parseFloat(apiSuccessRate),
        avgResponseTime: parseFloat(avgApiResponseTime),
      },
      sessions: {
        created: this.metrics.sessionsCreated,
        active: this.metrics.sessionsActive,
        closed: this.metrics.sessionsClosed,
      },
      social: {
        follows: this.metrics.totalFollows,
        likes: this.metrics.totalLikes,
        retweets: this.metrics.totalRetweets,
        tweets: this.metrics.totalTweets,
      },
      twitter: this.getTwitterEngagementMetrics(),
    };
  }

  /**
   * Gets the recent task history.
   * @param {number} [limit=10] - The number of recent tasks to return.
   * @returns {object[]} An array of recent task execution objects.
   */
  getRecentTasks(limit = 10) {
    return this.taskHistory.slice(-limit).reverse();
  }

  /**
   * Gets task statistics grouped by task name.
   * @returns {object} An object containing statistics for each task.
   */
  getTaskBreakdown() {
    const breakdown = {};

    for (const task of this.taskHistory) {
      if (!breakdown[task.taskName]) {
        breakdown[task.taskName] = {
          executions: 0,
          successes: 0,
          failures: 0,
          totalDuration: 0,
          durations: [],
        };
      }

      const stats = breakdown[task.taskName];
      stats.executions++;
      stats.totalDuration += task.duration;
      stats.durations.push(task.duration);

      if (task.success) {
        stats.successes++;
      } else {
        stats.failures++;
      }
    }

    for (const [_taskName, stats] of Object.entries(breakdown)) {
      stats.avgDuration = (stats.totalDuration / stats.executions).toFixed(2);
      stats.successRate = ((stats.successes / stats.executions) * 100).toFixed(
        2,
      );
      delete stats.durations;
    }

    return breakdown;
  }

  /**
   * Calculates a percentile from a sorted array.
   * @param {number[]} sortedArray - A sorted array of numbers.
   * @param {number} percentile - The percentile to calculate (0-100).
   * @returns {number} The percentile value.
   * @private
   */
  getPercentile(sortedArray, percentile) {
    if (sortedArray.length === 0) return 0;

    const index = Math.ceil((percentile / 100) * sortedArray.length) - 1;
    return sortedArray[Math.max(0, index)];
  }

  /**
   * Formats a duration in milliseconds to a human-readable string.
   * @param {number} ms - The duration in milliseconds.
   * @returns {string} The formatted duration string.
   * @private
   */
  formatDuration(ms) {
    if (!ms || ms < 1) return "0ms";

    const seconds = Math.floor(ms / 1000);
    const minutes = Math.floor(seconds / 60);
    const hours = Math.floor(minutes / 60);
    const days = Math.floor(hours / 24);

    if (days > 0) return `${days}d ${hours % 24}h ${minutes % 60}m`;
    if (hours > 0) return `${hours}h ${minutes % 60}m ${seconds % 60}s`;
    if (minutes > 0) return `${minutes}m ${seconds % 60}s`;
    if (seconds > 0) return `${seconds}s`;
    return `${Math.floor(ms)}ms`; // Show ms for sub-second durations
  }

  /**
   * Resets all metrics.
   */
  reset() {
    const previousStats = this.getStats();

    this.metrics = {
      tasksExecuted: 0,
      tasksFailed: 0,
      totalTaskDuration: 0,
      taskDurations: [],
      browsersDiscovered: 0,
      browsersConnected: 0,
      browsersFailed: 0,
      apiCalls: 0,
      apiFailures: 0,
      totalApiResponseTime: 0,
      sessionsCreated: 0,
      sessionsActive: 0,
      sessionsClosed: 0,
      // Social action counters
      totalFollows: 0,
      totalLikes: 0,
      totalRetweets: 0,
      totalTweets: 0,
      // Twitter engagement metrics
      totalReplies: 0,
      totalQuotes: 0,
      totalBookmarks: 0,
      // Performance metrics
      totalDiveDuration: 0,
      diveCount: 0,
      diveDurations: [],
      totalAILatency: 0,
      aiRequestCount: 0,
      aiLatencies: [],
      // Error tracking
      errorsByType: {},
      startTime: this.metrics.startTime,
      lastResetTime: Date.now(),
    };

    this.taskHistory = [];

    logger.info("Metrics reset", previousStats);
  }

  /**
   * Exports the current metrics to a JSON string.
   * @returns {string} A JSON string of the current metrics.
   */
  exportToJSON() {
    return JSON.stringify(
      {
        stats: this.getStats(),
        recentTasks: this.getRecentTasks(20),
        taskBreakdown: this.getTaskBreakdown(),
      },
      null,
      2,
    );
  }

  /**
   * Generates and saves a JSON report of the current run.
   * @returns {Promise<void>}
   */
  async generateJsonReport() {
    logger.info("Generating JSON summary report...");
    try {
      const fs = await import("fs/promises");
      const path = await import("path");

      const report = {
        summary: {
          ...this.getStats(),
          runEndTime: new Date().toISOString(),
        },
        tasks: this.taskHistory.map((task) => ({
          ...task,
          // Ensure error is serializable
          error:
            task.error instanceof Error
              ? { message: task.error.message, stack: task.error.stack }
              : task.error,
        })),
      };

      const reportPath = path.join(process.cwd(), "run-summary.json");
      await fs.writeFile(reportPath, JSON.stringify(report, null, 2));
      logger.info(`Successfully generated JSON report at ${reportPath}`);
    } catch (error) {
      logger.error("Failed to generate JSON report:", error);
    }
  }

  /**
   * Logs the current statistics to the console.
   */
  logStats() {
    const stats = this.getStats();
    const BRIGHT_YELLOW = "\x1b[93m";
    const BRIGHT_GREEN = "\x1b[92m";
    const BRIGHT_CYAN = "\x1b[96m";
    const BRIGHT_MAGENTA = "\x1b[95m";
    const BRIGHT_BLUE = "\x1b[94m";
    const BRIGHT_RED = "\x1b[91m";
    const BRIGHT_WHITE = "\x1b[97m";
    const DIM = "\x1b[2m";
    const RESET = "\x1b[0m";
    const ORANGE = "\x1b[38;5;208m";

    const time = new Date().toLocaleTimeString("en-US", { hour12: false });
    const prefix = `${DIM}${time}${RESET} ${BRIGHT_YELLOW}⭐${RESET} ${ORANGE}[metrics.js]${RESET}`;

    console.log(
      `${prefix} ${BRIGHT_YELLOW}================== System Metrics ==================${RESET}`,
    );
    console.log(
      `${prefix} ${BRIGHT_YELLOW}Uptime       : ${BRIGHT_CYAN}${stats.system.uptimeFormatted}${RESET}`,
    );
    console.log(
      `${prefix} ${BRIGHT_YELLOW}Tasks        : ${BRIGHT_WHITE}${stats.tasks.executed} executed, ${stats.tasks.succeeded} succeeded, ${stats.tasks.failed} failed ${BRIGHT_GREEN}(${stats.tasks.successRate}% success rate)${RESET}`,
    );
    console.log(
      `${prefix} ${BRIGHT_YELLOW}Twitter      : ${BRIGHT_GREEN}${stats.twitter.actions.total}${BRIGHT_YELLOW} total (f=${stats.twitter.actions.follows}, l=${stats.twitter.actions.likes}, r=${stats.twitter.actions.retweets}, rp=${stats.twitter.actions.replies}, q=${stats.twitter.actions.quotes}, bm=${stats.twitter.actions.bookmarks}, t=${stats.twitter.actions.tweets})${RESET}`,
    );
    console.log(
      `${prefix} ${BRIGHT_YELLOW}Browsers     : ${BRIGHT_MAGENTA}${stats.browsers.connected}/${stats.browsers.discovered} connected ${BRIGHT_GREEN}(${stats.browsers.connectionRate}%)${RESET}`,
    );
    console.log(
      `${prefix} ${BRIGHT_YELLOW}Sessions     : ${BRIGHT_BLUE}${stats.sessions.active} active, ${stats.sessions.created} total created${RESET}`,
    );
    console.log(
      `${prefix} ${BRIGHT_YELLOW}API          : ${BRIGHT_RED}${stats.api.calls} calls, ${stats.api.avgResponseTime}ms avg response time ${BRIGHT_GREEN}(${stats.api.successRate}% success rate)${RESET}`,
    );
    console.log(
      `${prefix} ${BRIGHT_YELLOW}Avg Duration : ${ORANGE}${this.formatDuration(stats.tasks.avgDuration)}${RESET}`,
    );
    console.log(
      `${prefix} ${BRIGHT_YELLOW}====================================================${RESET}`,
    );
  }
}

const metricsCollector = new MetricsCollector();

export default metricsCollector;
export { MetricsCollector };
