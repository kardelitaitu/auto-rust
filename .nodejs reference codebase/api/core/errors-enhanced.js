/**
 * Auto-AI Framework - Enhanced Error Classes
 *
 * Extended error types with actionable suggestions for resolution.
 * Builds upon the base errors.js with suggestion support.
 *
 * @module api/core/errors-enhanced
 */

import {
  getSuggestionsForError,
  formatSuggestions,
} from "../utils/error-suggestions.js";

// Re-export all base errors
export * from "./errors.js";

/**
 * Enhanced AutomationError with suggestions
 */
export class EnhancedAutomationError extends Error {
  constructor(code, message, options = {}) {
    super(message);
    this.name = this.constructor.name;
    this.code = code;
    this.metadata = options.metadata || {};
    this.timestamp = new Date().toISOString();
    this.cause = options.cause || null;
    this.suggestions = options.suggestions || null;
    this.docs = options.docs || null;
    this.severity = options.severity || "medium";

    Error.captureStackTrace(this, this.constructor);
  }

  /**
   * Get formatted error with suggestions
   * @returns {string}
   */
  toString() {
    const metaStr =
      Object.keys(this.metadata).length > 0
        ? ` | ${JSON.stringify(this.metadata)}`
        : "";

    let result = `[${this.code}] ${this.message}${metaStr}`;

    if (this.suggestions && this.suggestions.length > 0) {
      result += "\n\nSuggestions:";
      this.suggestions.forEach((s, i) => {
        result += `\n  ${i + 1}. ${s}`;
      });
    }

    if (this.docs) {
      result += `\n\nDocumentation: ${this.docs}`;
    }

    return result;
  }

  /**
   * Convert to JSON for logging
   * @returns {object}
   */
  toJSON() {
    return {
      name: this.name,
      code: this.code,
      message: this.message,
      metadata: this.metadata,
      timestamp: this.timestamp,
      suggestions: this.suggestions,
      docs: this.docs,
      severity: this.severity,
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
   * Get auto-generated suggestions based on error code
   * @returns {string[]}
   */
  getAutoSuggestions() {
    const suggestionData = getSuggestionsForError(this);
    if (suggestionData) {
      return suggestionData.suggestions;
    }
    return this.suggestions || [];
  }

  /**
   * Print error with suggestions to console
   */
  print() {
    console.error(this.toString());

    // Add auto-suggestions if available
    const autoSuggestions = this.getAutoSuggestions();
    if (
      autoSuggestions.length > 0 &&
      (!this.suggestions || this.suggestions.length === 0)
    ) {
      console.error("\nSuggestions:");
      autoSuggestions.forEach((s, i) => {
        console.error(`  ${i + 1}. ${s}`);
      });
    }
  }
}

/**
 * Enhanced Element Error with suggestions
 */
export class EnhancedElementNotFoundError extends EnhancedAutomationError {
  constructor(selector, options = {}) {
    super("ELEMENT_NOT_FOUND", `Element not found: ${selector}`, {
      ...options,
      metadata: { selector, ...options.metadata },
      suggestions: options.suggestions || [
        "Verify selector is correct: check for typos",
        "Wait for element: await api.wait.forElement(selector)",
        "Check if element is inside an iframe",
        "Verify page has fully loaded",
        "Try alternative selectors (CSS, XPath, text)",
      ],
      docs: "docs/RECIPES.md#error-handling",
      severity: "medium",
    });
    this.name = "EnhancedElementNotFoundError";
    this.selector = selector;
  }
}

/**
 * Enhanced Context Not Initialized Error
 */
export class EnhancedContextNotInitializedError extends EnhancedAutomationError {
  constructor(options = {}) {
    super(
      "CONTEXT_NOT_INITIALIZED",
      "API context not initialized. Use api.withPage(page, fn) first.",
      {
        ...options,
        suggestions: options.suggestions || [
          "Wrap operations in: await api.withPage(page, async () => { ... })",
          "Call api.init(page) before using other API methods",
          "Don't access page outside the withPage callback",
          "Check that page object is valid",
        ],
        docs: "docs/api.md#getting-started",
        severity: "high",
      },
    );
    this.name = "EnhancedContextNotInitializedError";
  }
}

/**
 * Enhanced Browser Not Found Error
 */
export class EnhancedBrowserNotFoundError extends EnhancedAutomationError {
  constructor(options = {}) {
    super(
      "BROWSER_NOT_FOUND",
      "No browsers discovered. Ensure remote debugging is enabled.",
      {
        ...options,
        suggestions: options.suggestions || [
          "Chrome/Brave: Launch with --remote-debugging-port=9222",
          "ixBrowser: Enable Remote Debugging in Settings → Port 53200",
          "Check config/browserAPI.json for correct port",
          "Verify firewall is not blocking the debugging port",
          "Restart browser and try again",
        ],
        docs: "docs/troubleshooting.md#browser-not-found",
        severity: "high",
      },
    );
    this.name = "EnhancedBrowserNotFoundError";
  }
}

/**
 * Enhanced LLM Timeout Error
 */
export class EnhancedLLMTimeoutError extends EnhancedAutomationError {
  constructor(message = "LLM request timeout", options = {}) {
    super("LLM_TIMEOUT", message, {
      ...options,
      suggestions: options.suggestions || [
        "Local LLM: Ensure Ollama is running (ollama serve)",
        "Local LLM: Try a smaller/faster model",
        "Cloud LLM: Check API key is valid",
        "Cloud LLM: Verify account has credits",
        "Increase timeout in config/settings.json",
        "Enable fallback to alternative provider",
      ],
      docs: "docs/troubleshooting.md#llm-issues",
      severity: "high",
    });
    this.name = "EnhancedLLMTimeoutError";
  }
}

/**
 * Enhanced Navigation Error
 */
export class EnhancedNavigationError extends EnhancedAutomationError {
  constructor(url, reason, options = {}) {
    super("NAVIGATION_ERROR", `Navigation to ${url} failed: ${reason}`, {
      ...options,
      metadata: { url, reason, ...options.metadata },
      suggestions: options.suggestions || [
        "Verify URL is valid and accessible",
        "Check network connection",
        "Increase navigation timeout",
        "Check if site blocks automation",
        "Try with different user agent",
      ],
      docs: "docs/RECIPES.md#navigation",
      severity: "high",
    });
    this.name = "EnhancedNavigationError";
    this.url = url;
    this.reason = reason;
  }
}

/**
 * Enhanced Action Failed Error
 */
export class EnhancedActionFailedError extends EnhancedAutomationError {
  constructor(action, reason, options = {}) {
    super("ACTION_FAILED", `Action '${action}' failed: ${reason}`, {
      ...options,
      metadata: { action, reason, ...options.metadata },
      suggestions: options.suggestions || [
        "Retry the action with: { retries: 3 }",
        "Check element is visible and enabled",
        "Verify page hasn't navigated away",
        "Try alternative interaction method",
        "Check browser console for JavaScript errors",
      ],
      docs: "docs/RECIPES.md#error-handling",
      severity: "medium",
    });
    this.name = "EnhancedActionFailedError";
    this.action = action;
    this.reason = reason;
  }
}

/**
 * Enhanced Connection Timeout Error
 */
export class EnhancedConnectionTimeoutError extends EnhancedAutomationError {
  constructor(endpoint, timeout, options = {}) {
    super(
      "CONNECTION_TIMEOUT",
      `Connection timeout after ${timeout}ms: ${endpoint}`,
      {
        ...options,
        metadata: { endpoint, timeout, ...options.metadata },
        suggestions: options.suggestions || [
          "Check if browser is still running",
          "Verify the WebSocket endpoint is accessible",
          "Increase timeout in config/timeouts.json",
          "Check firewall settings for port access",
          "Try restarting the browser",
        ],
        docs: "docs/troubleshooting.md#connection-timeout",
        severity: "high",
      },
    );
    this.name = "EnhancedConnectionTimeoutError";
    this.endpoint = endpoint;
    this.timeout = timeout;
  }
}

/**
 * Enhanced Rate Limit Error
 */
export class EnhancedRateLimitError extends EnhancedAutomationError {
  constructor(message = "Rate limit exceeded", options = {}) {
    super("RATE_LIMIT", message, {
      ...options,
      suggestions: options.suggestions || [
        "Slow down request rate",
        "Implement exponential backoff",
        "Check rate limit headers",
        "Wait before retrying",
        "Consider upgrading API plan",
      ],
      docs: "docs/configuration.md",
      severity: "medium",
      retryAfter: options.retryAfter || null,
    });
    this.name = "EnhancedRateLimitError";
    this.retryAfter = options.retryAfter;
  }
}

/**
 * Create enhanced error from base error
 *
 * @param {Error} error - Base error to enhance
 * @param {object} options - Additional options
 * @returns {EnhancedAutomationError}
 */
export function enhanceError(error, options = {}) {
  // If already enhanced, return as-is
  if (error instanceof EnhancedAutomationError) {
    return error;
  }

  // Try to get suggestions for the error
  const suggestionData = getSuggestionsForError(error);

  // Create appropriate enhanced error based on type
  if (suggestionData) {
    return new EnhancedAutomationError(
      error.code || error.name || "UNKNOWN_ERROR",
      error.message,
      {
        ...options,
        cause: error,
        suggestions: options.suggestions || suggestionData.suggestions,
        docs: options.docs || suggestionData.docs,
        severity: options.severity || suggestionData.severity,
      },
    );
  }

  // Generic enhancement
  return new EnhancedAutomationError(
    error.code || error.name || "UNKNOWN_ERROR",
    error.message,
    {
      ...options,
      cause: error,
    },
  );
}

/**
 * Wrap async function with enhanced error handling
 *
 * @param {Function} fn - Async function to wrap
 * @param {string} context - Context description
 * @param {object} options - Error options
 * @returns {Promise<any>}
 */
export async function withEnhancedError(
  fn,
  context = "operation",
  options = {},
) {
  try {
    return await fn();
  } catch (error) {
    throw enhanceError(error, {
      ...options,
      metadata: { context, ...options.metadata },
    });
  }
}

export default {
  EnhancedAutomationError,
  EnhancedElementNotFoundError,
  EnhancedContextNotInitializedError,
  EnhancedBrowserNotFoundError,
  EnhancedLLMTimeoutError,
  EnhancedNavigationError,
  EnhancedActionFailedError,
  EnhancedConnectionTimeoutError,
  EnhancedRateLimitError,
  enhanceError,
  withEnhancedError,
};
