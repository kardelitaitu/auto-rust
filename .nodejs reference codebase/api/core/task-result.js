/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Task result types and utilities
 * Provides structured result reporting for task execution
 * @module core/task-result
 */

/**
 * Task execution status
 * @enum {string}
 */
export const TaskStatus = {
  /** Task completed successfully */
  SUCCESS: "success",
  /** Task failed with error */
  FAILED: "failed",
  /** Task timed out */
  TIMEOUT: "timeout",
  /** Task was skipped (e.g., dependency failed) */
  SKIPPED: "skipped",
  /** Task was cancelled by user */
  CANCELLED: "cancelled",
};

/**
 * @typedef {Object} TaskError
 * @property {string} name - Error name/type
 * @property {string} message - Error message
 * @property {string} [code] - Error code (e.g., 'RATE_LIMIT', 'ELEMENT_NOT_FOUND')
 * @property {boolean} [recoverable] - Whether the error can be recovered from
 * @property {number} [retryAfter] - Seconds to wait before retrying (if applicable)
 */

/**
 * @typedef {Object} TaskResult
 * @property {string} taskName - Name of the task
 * @property {TaskStatus} status - Execution status
 * @property {any} [data] - Task-specific result data
 * @property {TaskError} [error] - Error details if failed
 * @property {number} startTime - Start timestamp (ms)
 * @property {number} endTime - End timestamp (ms)
 * @property {number} duration - Duration in seconds
 * @property {string} [sessionId] - Browser session identifier
 * @property {object} [metrics] - Performance metrics
 */

/**
 * Create a successful task result
 * @param {string} taskName - Task name
 * @param {any} data - Result data
 * @param {object} [options] - Additional options
 * @returns {TaskResult}
 */
export function createSuccessResult(taskName, data, options = {}) {
  const now = Date.now();
  return {
    taskName,
    status: TaskStatus.SUCCESS,
    data,
    startTime: options.startTime || now,
    endTime: now,
    duration: options.duration || 0,
    sessionId: options.sessionId,
    metrics: options.metrics,
  };
}

/**
 * Create a failed task result
 * @param {string} taskName - Task name
 * @param {Error|string} error - Error object or message
 * @param {object} [options] - Additional options
 * @returns {TaskResult}
 */
export function createFailedResult(taskName, error, options = {}) {
  const now = Date.now();
  const errorObj =
    typeof error === "string"
      ? { name: "TaskError", message: error }
      : {
          name: error.name || "TaskError",
          message: error.message,
          code: error.code,
          recoverable: isRecoverableError(error),
        };

  if (error.retryAfter) {
    errorObj.retryAfter = error.retryAfter;
  }

  return {
    taskName,
    status: TaskStatus.FAILED,
    error: errorObj,
    startTime: options.startTime || now,
    endTime: now,
    duration: options.duration || 0,
    sessionId: options.sessionId,
    data: options.partialData, // Include any partial results
  };
}

/**
 * Create a timeout task result
 * @param {string} taskName - Task name
 * @param {number} timeoutMs - Timeout duration
 * @param {object} [options] - Additional options
 * @returns {TaskResult}
 */
export function createTimeoutResult(taskName, timeoutMs, options = {}) {
  const now = Date.now();
  return {
    taskName,
    status: TaskStatus.TIMEOUT,
    error: {
      name: "TaskTimeoutError",
      message: `Task exceeded time limit of ${timeoutMs}ms`,
      code: "TIMEOUT",
      recoverable: true,
      retryAfter: 30,
    },
    startTime: options.startTime || now,
    endTime: now,
    duration: options.duration || 0,
    sessionId: options.sessionId,
    data: options.partialData, // Include any partial results
  };
}

/**
 * Create a skipped task result
 * @param {string} taskName - Task name
 * @param {string} reason - Reason for skipping
 * @param {object} [options] - Additional options
 * @returns {TaskResult}
 */
export function createSkippedResult(taskName, reason, options = {}) {
  const now = Date.now();
  return {
    taskName,
    status: TaskStatus.SKIPPED,
    error: {
      name: "TaskSkipped",
      message: reason,
      code: "SKIPPED",
    },
    startTime: options.startTime || now,
    endTime: now,
    duration: 0,
    sessionId: options.sessionId,
  };
}

/**
 * Check if an error is recoverable
 * @param {Error} error - Error to check
 * @returns {boolean}
 */
function isRecoverableError(error) {
  const recoverableCodes = [
    "RATE_LIMIT",
    "NETWORK_ERROR",
    "TIMEOUT",
    "CIRCUIT_OPEN",
    "SESSION_DISCONNECTED",
    "ELEMENT_NOT_FOUND",
    "NAVIGATION_ERROR",
  ];

  const nonRecoverableCodes = [
    "INVALID_REQUEST",
    "AUTHENTICATION_ERROR",
    "PERMISSION_DENIED",
    "NOT_FOUND",
    "INVALID_ARGUMENT",
    "FATAL",
  ];

  if (error.code) {
    return (
      recoverableCodes.includes(error.code) &&
      !nonRecoverableCodes.includes(error.code)
    );
  }

  // Default: network-like errors are recoverable
  return (
    error.message.includes("network") ||
    error.message.includes("timeout") ||
    error.message.includes("ECONNREFUSED")
  );
}

/**
 * Format task result for logging
 * @param {TaskResult} result - Task result
 * @returns {string}
 */
export function formatResult(result) {
  const statusIcon =
    {
      [TaskStatus.SUCCESS]: "✅",
      [TaskStatus.FAILED]: "❌",
      [TaskStatus.TIMEOUT]: "⏱️",
      [TaskStatus.SKIPPED]: "⏭️",
      [TaskStatus.CANCELLED]: "🚫",
    }[result.status] || "❓";

  const duration = result.duration ? ` (${result.duration.toFixed(1)}s)` : "";

  if (result.status === TaskStatus.SUCCESS) {
    return `${statusIcon} ${result.taskName}${duration}`;
  }

  if (result.error) {
    return `${statusIcon} ${result.taskName}${duration}: ${result.error.message}`;
  }

  return `${statusIcon} ${result.taskName}${duration}`;
}

/**
 * Summarize multiple task results
 * @param {TaskResult[]} results - Array of task results
 * @returns {object} Summary statistics
 */
export function summarizeResults(results) {
  const summary = {
    total: results.length,
    success: 0,
    failed: 0,
    timeout: 0,
    skipped: 0,
    cancelled: 0,
    totalDuration: 0,
  };

  for (const result of results) {
    summary.totalDuration += result.duration || 0;

    switch (result.status) {
      case TaskStatus.SUCCESS:
        summary.success++;
        break;
      case TaskStatus.FAILED:
        summary.failed++;
        break;
      case TaskStatus.TIMEOUT:
        summary.timeout++;
        break;
      case TaskStatus.SKIPPED:
        summary.skipped++;
        break;
      case TaskStatus.CANCELLED:
        summary.cancelled++;
        break;
    }
  }

  summary.successRate =
    summary.total > 0
      ? ((summary.success / summary.total) * 100).toFixed(1) + "%"
      : "0%";

  summary.averageDuration =
    summary.total > 0
      ? (summary.totalDuration / summary.total).toFixed(1) + "s"
      : "0s";

  return summary;
}

export default {
  TaskStatus,
  createSuccessResult,
  createFailedResult,
  createTimeoutResult,
  createSkippedResult,
  formatResult,
  summarizeResults,
};
