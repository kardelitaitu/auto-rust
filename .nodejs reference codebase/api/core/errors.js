/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Centralized Error Definitions
 * Provides custom error classes with codes for the automation framework.
 *
 * @module api/core/errors
 */

// Re-export HTTP/API errors from utils for unified import
export {
  AppError,
  RouterError,
  ProxyError,
  RateLimitError,
  ModelError,
  BrowserError,
  TimeoutError,
  CircuitBreakerError,
  classifyHttpError,
  wrapError,
} from "../utils/errors.js";

/**
 * Base error class for all automation errors
 * Supports both: (code, message, metadata, cause) and legacy (message, code)
 */
export class AutomationError extends Error {
  /**
   * @param {string} code - Error code (or message for legacy support)
   * @param {string} [message] - Error message (or code for legacy support)
   * @param {object} [metadata={}] - Additional context
   * @param {Error} [cause] - Original error
   */
  constructor(code, message, metadata = {}, cause = null) {
    // Detect legacy signature: (message, code) where code looks like an error code
    let finalCode, finalMessage;

    if (message === undefined) {
      // Single argument: treat as message
      finalMessage = code;
      finalCode = "AUTOMATION_ERROR"; // Default for base class
    } else if (typeof message === "string" && /^[A-Z_]+$/.test(message)) {
      // Second argument looks like an error code (OLD signature: message, code)
      finalMessage = code;
      finalCode = message;
    } else {
      // NEW signature: (code, message, metadata, cause)
      finalCode = code;
      finalMessage = message;
    }

    super(finalMessage);
    this.name = this.constructor.name;
    this.code = finalCode;
    this.metadata = metadata;
    this.timestamp = new Date().toISOString();
    this.cause = cause;
    Error.captureStackTrace(this, this.constructor);
  }

  /**
   * Convert error to JSON for logging
   * @returns {object}
   */
  toJSON() {
    return {
      name: this.name,
      code: this.code,
      message: this.message,
      metadata: this.metadata,
      timestamp: this.timestamp,
      stack: this.stack,
      cause: this.cause
        ? {
            message: this.cause.message,
            stack: this.cause.stack,
          }
        : null,
    };
  }

  /**
   * Get formatted string representation
   * @returns {string}
   */
  toString() {
    const metaStr =
      Object.keys(this.metadata).length > 0
        ? ` | ${JSON.stringify(this.metadata)}`
        : "";
    return `[${this.code}] ${this.message}${metaStr}`;
  }
}

/**
 * Session-related errors
 */
export class SessionError extends AutomationError {
  constructor(code, message, metadata = {}, cause = null) {
    // Backwards compat: if only one arg, treat as message and derive code from class name
    const finalMessage = message === undefined ? code : message;
    const finalCode =
      message === undefined
        ? "SESSION_ERROR" // Derive from class name for single-arg calls
        : code;
    super(finalCode, finalMessage, metadata, cause);
    this.name = "SessionError";
  }
}

export class SessionDisconnectedError extends SessionError {
  constructor(
    message = "Session has been disconnected",
    metadata = {},
    cause = null,
  ) {
    super("SESSION_DISCONNECTED", message, metadata, cause);
    this.name = "SessionDisconnectedError";
  }
}

export class SessionClosedError extends SessionError {
  constructor(
    sessionId = "unknown",
    message = "Session has been closed",
    metadata = {},
    cause = null,
  ) {
    super("SESSION_CLOSED", message, { sessionId, ...metadata }, cause);
    this.name = "SessionClosedError";
  }
}

export class SessionNotFoundError extends SessionError {
  constructor(sessionId, metadata = {}) {
    super("SESSION_NOT_FOUND", `Session not found: ${sessionId}`, metadata);
    this.name = "SessionNotFoundError";
  }
}

export class SessionTimeoutError extends SessionError {
  constructor(message = "Session timed out", metadata = {}, cause = null) {
    super("SESSION_TIMEOUT", message, metadata, cause);
    this.name = "SessionTimeoutError";
  }
}

/**
 * Page/context errors
 */
export class ContextError extends AutomationError {
  constructor(code, message, metadata = {}, cause = null) {
    const finalMessage = message === undefined ? code : message;
    const finalCode = message === undefined ? "CONTEXT_ERROR" : code;
    super(finalCode, finalMessage, metadata, cause);
    this.name = "ContextError";
  }
}

export class ContextNotInitializedError extends ContextError {
  constructor(
    message = "API context not initialized. Use api.withPage(page, fn) first.",
    metadata = {},
    cause = null,
  ) {
    super("CONTEXT_NOT_INITIALIZED", message, metadata, cause);
    this.name = "ContextNotInitializedError";
  }
}

export class PageClosedError extends ContextError {
  constructor(message = "Page has been closed", metadata = {}, cause = null) {
    super("PAGE_CLOSED", message, metadata, cause);
    this.name = "PageClosedError";
  }
}

/**
 * Element/selector errors
 */
export class ElementError extends AutomationError {
  constructor(code, message, metadata = {}, cause = null) {
    const finalMessage = message === undefined ? code : message;
    const finalCode = message === undefined ? "ELEMENT_ERROR" : code;
    super(finalCode, finalMessage, metadata, cause);
    this.name = "ElementError";
  }
}

export class ElementNotFoundError extends ElementError {
  constructor(selector, metadata = {}) {
    super("ELEMENT_NOT_FOUND", `Element not found: ${selector}`, metadata);
    this.name = "ElementNotFoundError";
  }
}

export class ElementDetachedError extends ElementError {
  constructor(selector = "element", metadata = {}) {
    super(
      "ELEMENT_DETACHED",
      `Element has been detached from DOM: ${selector}`,
      metadata,
    );
    this.name = "ElementDetachedError";
  }
}

export class ElementObscuredError extends ElementError {
  constructor(selector = "element", metadata = {}) {
    super(
      "ELEMENT_OBSCURED",
      `Element is obscured by another element: ${selector}`,
      metadata,
    );
    this.name = "ElementObscuredError";
  }
}

export class ElementTimeoutError extends ElementError {
  constructor(selector, timeout, metadata = {}) {
    super(
      "ELEMENT_TIMEOUT",
      `Element not found within timeout (${timeout}ms): ${selector}`,
      metadata,
    );
    this.name = "ElementTimeoutError";
  }
}

/**
 * Action errors
 */
export class ActionError extends AutomationError {
  constructor(code, message, metadata = {}, cause = null) {
    const finalMessage = message === undefined ? code : message;
    const finalCode = message === undefined ? "ACTION_ERROR" : code;
    super(finalCode, finalMessage, metadata, cause);
    this.name = "ActionError";
  }
}

export class ActionFailedError extends ActionError {
  constructor(action, reason, metadata = {}) {
    super("ACTION_FAILED", `Action '${action}' failed: ${reason}`, {
      ...metadata,
      action,
    });
    this.name = "ActionFailedError";
    this.action = action;
  }
}

export class NavigationError extends ActionError {
  constructor(url, reason, metadata = {}) {
    super("NAVIGATION_ERROR", `Navigation to ${url} failed: ${reason}`, {
      ...metadata,
      url,
    });
    this.name = "NavigationError";
    this.url = url;
  }
}

export class TaskTimeoutError extends ActionError {
  constructor(taskName, timeout, metadata = {}) {
    super("TASK_TIMEOUT", `Task '${taskName}' timed out after ${timeout}ms`, {
      ...metadata,
      taskName,
      timeout,
    });
    this.name = "TaskTimeoutError";
    this.taskName = taskName;
    this.timeout = timeout;
  }
}

/**
 * Configuration errors
 */
export class ConfigError extends AutomationError {
  constructor(code, message, metadata = {}, cause = null) {
    const finalMessage = message === undefined ? code : message;
    const finalCode = message === undefined ? "CONFIG_ERROR" : code;
    super(finalCode, finalMessage, metadata, cause);
    this.name = "ConfigError";
  }
}

export class ConfigNotFoundError extends ConfigError {
  constructor(key, metadata = {}) {
    super("CONFIG_NOT_FOUND", `Configuration key not found: ${key}`, metadata);
    this.name = "ConfigNotFoundError";
  }
}

/**
 * LLM/AI errors
 */
export class LLMError extends AutomationError {
  constructor(code, message, metadata = {}, cause = null) {
    const finalMessage = message === undefined ? code : message;
    const finalCode = message === undefined ? "LLM_ERROR" : code;
    super(finalCode, finalMessage, metadata, cause);
    this.name = "LLMError";
  }
}

export class LLMTimeoutError extends LLMError {
  constructor(message = "LLM request timed out", metadata = {}, cause = null) {
    super("LLM_TIMEOUT", message, metadata, cause);
    this.name = "LLMTimeoutError";
  }
}

export class LLMRateLimitError extends LLMError {
  constructor(
    message = "LLM rate limit exceeded",
    metadata = {},
    cause = null,
  ) {
    super("LLM_RATE_LIMIT", message, metadata, cause);
    this.name = "LLMRateLimitError";
  }
}

export class LLMCircuitOpenError extends LLMError {
  constructor(modelId, retryAfter, metadata = {}) {
    super(
      "LLM_CIRCUIT_OPEN",
      `Circuit breaker OPEN for ${modelId}. Retry after ${Math.ceil(retryAfter / 1000)}s`,
      { ...metadata, modelId, retryAfter },
    );
    this.name = "LLMCircuitOpenError";
    // Preserve direct property access for backwards compatibility
    this.modelId = modelId;
    this.retryAfter = retryAfter;
  }
}

/**
 * Validation errors
 */
export class ValidationError extends AutomationError {
  constructor(code, message, metadata = {}, cause = null) {
    const finalMessage = message === undefined ? code : message;
    const finalCode = message === undefined ? "VALIDATION_ERROR" : code;
    super(finalCode, finalMessage, metadata, cause);
    this.name = "ValidationError";
  }
}

/**
 * Helper to check error type
 * @param {Error} error - Error to check
 * @param {string} code - Error code to match
 * @returns {boolean}
 */
export function isErrorCode(error, code) {
  return error?.code === code || error?.name === code;
}

/**
 * Helper to wrap async functions with error handling
 * @param {Function} fn - Function to wrap
 * @param {string} context - Context description for errors
 * @param {object} metadata - Additional metadata
 * @returns {Promise<any>}
 */
export async function withErrorHandling(
  fn,
  context = "operation",
  metadata = {},
) {
  try {
    return await fn();
  } catch (error) {
    if (error instanceof AutomationError) {
      throw error;
    }
    throw new ActionError(
      "OPERATION_ERROR",
      `Error during ${context}: ${error.message}`,
      metadata,
      error,
    );
  }
}

export default {
  AutomationError,
  SessionError,
  SessionClosedError,
  SessionDisconnectedError,
  SessionNotFoundError,
  SessionTimeoutError,
  ContextError,
  ContextNotInitializedError,
  PageClosedError,
  ElementError,
  ElementNotFoundError,
  ElementDetachedError,
  ElementObscuredError,
  ElementTimeoutError,
  ActionError,
  ActionFailedError,
  NavigationError,
  TaskTimeoutError,
  ConfigError,
  ConfigNotFoundError,
  LLMError,
  LLMTimeoutError,
  LLMRateLimitError,
  LLMCircuitOpenError,
  ValidationError,
  isErrorCode,
  withErrorHandling,
};
