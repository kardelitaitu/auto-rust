/**
 * Error Suggestions Module
 *
 * Provides actionable suggestions for resolving common errors.
 * Maps error codes to troubleshooting steps and documentation links.
 *
 * @module api/utils/error-suggestions
 */

/**
 * Error suggestion database
 * Maps error codes to actionable fixes
 */
const ERROR_SUGGESTIONS = {
  // Browser/Connection Errors
  BROWSER_NOT_FOUND: {
    message: "No browsers discovered",
    suggestions: [
      "Ensure browser is running with remote debugging enabled",
      "Chrome/Brave: Launch with --remote-debugging-port=9222",
      "ixBrowser: Enable Remote Debugging in Settings → Port 53200",
      "Check config/browserAPI.json for correct port configuration",
      "Verify firewall is not blocking the debugging port",
    ],
    docs: "docs/troubleshooting.md#browser-not-found",
    severity: "high",
  },

  CONNECTION_TIMEOUT: {
    message: "Connection timeout",
    suggestions: [
      "Check if browser is still running",
      "Verify the WebSocket endpoint is accessible",
      "Increase timeout in config/timeouts.json",
      "Check firewall settings for port access",
      "Try restarting the browser",
    ],
    docs: "docs/troubleshooting.md#connection-timeout",
    severity: "high",
  },

  SESSION_DISCONNECTED: {
    message: "Session disconnected",
    suggestions: [
      "Check if browser window was closed",
      "Verify network connection is stable",
      "Increase session timeout settings",
      "Check browser console for crash errors",
      "Try with a fresh browser instance",
    ],
    docs: "docs/troubleshooting.md#session-crashed",
    severity: "high",
  },

  SESSION_CLOSED: {
    message: "Session has been closed",
    suggestions: [
      "Ensure api.withPage() is used for all operations",
      "Check if page.close() was called prematurely",
      "Verify browser hasn't crashed",
      "Add error handling to prevent premature closure",
    ],
    docs: "docs/api.md#context-methods",
    severity: "medium",
  },

  // Context Errors
  CONTEXT_NOT_INITIALIZED: {
    message: "API context not initialized",
    suggestions: [
      "Always wrap operations in api.withPage(page, async () => { ... })",
      "Call api.init(page) before using other API methods",
      "Don't access page outside the withPage callback",
      "Check that page object is valid",
    ],
    docs: "docs/api.md#getting-started",
    severity: "high",
  },

  PAGE_CLOSED: {
    message: "Page has been closed",
    suggestions: [
      "Check if page.close() was called",
      "Verify browser didn't crash",
      "Add try/finally to ensure proper cleanup",
      "Use a fresh page instance",
    ],
    docs: "docs/troubleshooting.md#context-isolation-error",
    severity: "medium",
  },

  // Element Errors
  ELEMENT_NOT_FOUND: {
    message: "Element not found",
    suggestions: [
      "Verify selector is correct: check for typos",
      "Wait for element: await api.wait.forElement(selector)",
      "Check if element is inside an iframe",
      "Verify page has fully loaded",
      "Try alternative selectors (CSS, XPath, text)",
      "Check if element requires user interaction first",
    ],
    docs: "docs/RECIPES.md#error-handling",
    severity: "medium",
  },

  ELEMENT_TIMEOUT: {
    message: "Element not found within timeout",
    suggestions: [
      "Increase timeout: await api.wait.forElement(selector, { timeout: 10000 })",
      "Check if element requires navigation first",
      "Verify page content has loaded",
      "Check for cookie consent dialogs blocking content",
      "Try scrolling to bring element into view",
    ],
    docs: "docs/RECIPES.md#waiting",
    severity: "medium",
  },

  ELEMENT_OBSCURED: {
    message: "Element is obscured",
    suggestions: [
      "Close any popup dialogs or modals",
      "Scroll element into view: await api.scroll.toElement(selector)",
      "Wait for loading overlays to disappear",
      "Check for cookie consent blocking interaction",
      "Try clicking at specific coordinates",
    ],
    docs: "docs/troubleshooting.md",
    severity: "medium",
  },

  ELEMENT_DETACHED: {
    message: "Element has been detached from DOM",
    suggestions: [
      "Re-query the element after page changes",
      "Wait for page to stabilize before interacting",
      "Use fresh selector query instead of cached reference",
      "Check if page navigation occurred",
    ],
    docs: "docs/api.md#queries",
    severity: "low",
  },

  // Action Errors
  ACTION_FAILED: {
    message: "Action failed",
    suggestions: [
      "Retry the action with: await api.click(selector, { retries: 3 })",
      "Check element is visible and enabled",
      "Verify page hasn't navigated away",
      "Try alternative interaction method",
      "Check browser console for JavaScript errors",
    ],
    docs: "docs/RECIPES.md#error-handling",
    severity: "medium",
  },

  NAVIGATION_ERROR: {
    message: "Navigation failed",
    suggestions: [
      "Verify URL is valid and accessible",
      "Check network connection",
      "Increase navigation timeout",
      "Check if site blocks automation",
      "Try with different user agent",
    ],
    docs: "docs/RECIPES.md#navigation",
    severity: "high",
  },

  TASK_TIMEOUT: {
    message: "Task timed out",
    suggestions: [
      "Increase timeout in config/timeouts.json",
      "Check for infinite loops in task logic",
      "Verify external services are responding",
      "Add progress logging to identify bottleneck",
      "Break task into smaller steps",
    ],
    docs: "docs/configuration.md",
    severity: "high",
  },

  // LLM Errors
  LLM_TIMEOUT: {
    message: "LLM request timeout",
    suggestions: [
      "Local LLM: Ensure Ollama is running (ollama serve)",
      "Local LLM: Try a smaller/faster model",
      "Cloud LLM: Check API key is valid",
      "Cloud LLM: Verify account has credits",
      "Increase timeout in config/settings.json",
      "Enable fallback to alternative provider",
    ],
    docs: "docs/troubleshooting.md#llm-issues",
    severity: "high",
  },

  LLM_RATE_LIMIT: {
    message: "LLM rate limit exceeded",
    suggestions: [
      "Wait before retrying (exponential backoff)",
      "Reduce request frequency",
      "Enable request queuing in settings",
      "Use local LLM for simple tasks",
      "Upgrade API plan for higher limits",
    ],
    docs: "docs/configuration.md#llm",
    severity: "medium",
  },

  LLM_CIRCUIT_OPEN: {
    message: "Circuit breaker open",
    suggestions: [
      "Wait for circuit to reset (check retry-after time)",
      "Check LLM provider status",
      "Verify API credentials",
      "Switch to fallback provider",
      "Use local LLM temporarily",
    ],
    docs: "docs/architecture.md#error-recovery",
    severity: "high",
  },

  // Configuration Errors
  CONFIG_NOT_FOUND: {
    message: "Configuration not found",
    suggestions: [
      "Check config file exists: config/settings.json",
      "Verify .env file is present",
      "Run: copy .env.example .env",
      "Check environment variables are set",
      "Validate JSON syntax in config files",
    ],
    docs: "docs/configuration.md",
    severity: "high",
  },

  VALIDATION_ERROR: {
    message: "Validation failed",
    suggestions: [
      "Check input matches expected format",
      "Review error details for specific field",
      "Verify required fields are provided",
      "Check data types match schema",
    ],
    docs: "docs/api.md",
    severity: "low",
  },

  // HTTP Errors (mapped from utils/errors.js)
  HTTP_401: {
    message: "Unauthorized",
    suggestions: [
      "Verify API key is correct",
      "Check API key hasn't expired",
      "Ensure API key has required permissions",
      "Check for extra whitespace in key",
    ],
    docs: "docs/troubleshooting.md",
    severity: "high",
  },

  HTTP_403: {
    message: "Forbidden",
    suggestions: [
      "Verify account has required permissions",
      "Check if IP is blocked",
      "Ensure API key has access to this resource",
      "Contact support if issue persists",
    ],
    docs: "docs/troubleshooting.md",
    severity: "high",
  },

  HTTP_404: {
    message: "Not found",
    suggestions: [
      "Verify URL/resource exists",
      "Check for typos in URL",
      "Ensure correct API endpoint",
      "Check if resource was deleted",
    ],
    docs: "docs/troubleshooting.md",
    severity: "medium",
  },

  HTTP_429: {
    message: "Too many requests",
    suggestions: [
      "Slow down request rate",
      "Implement exponential backoff",
      "Check rate limit headers",
      "Wait before retrying",
    ],
    docs: "docs/configuration.md",
    severity: "medium",
  },

  HTTP_500: {
    message: "Internal server error",
    suggestions: [
      "Retry after a short delay",
      "Check service status page",
      "Try alternative endpoint",
      "Contact support if persistent",
    ],
    docs: "docs/troubleshooting.md",
    severity: "high",
  },

  HTTP_503: {
    message: "Service unavailable",
    suggestions: [
      "Service may be down for maintenance",
      "Retry after a few minutes",
      "Check service status page",
      "Use fallback service",
    ],
    docs: "docs/troubleshooting.md",
    severity: "high",
  },
};

/**
 * Get suggestions for an error code
 *
 * @param {string} errorCode - The error code to get suggestions for
 * @returns {object|null} Suggestion object or null if not found
 */
export function getSuggestions(errorCode) {
  return ERROR_SUGGESTIONS[errorCode] || null;
}

/**
 * Get suggestions from an error instance
 *
 * @param {Error} error - Error object (may have code property)
 * @returns {object|null} Suggestion object or null if not found
 */
export function getSuggestionsForError(error) {
  const code = error?.code || error?.name;
  if (!code) return null;

  // Try exact match first
  let suggestions = ERROR_SUGGESTIONS[code];
  if (suggestions) return suggestions;

  // Try matching by error name
  suggestions = ERROR_SUGGESTIONS[error.name];
  if (suggestions) return suggestions;

  // Try matching HTTP errors
  if (error.status) {
    return ERROR_SUGGESTIONS[`HTTP_${error.status}`];
  }

  return null;
}

/**
 * Format suggestions as a string for display
 *
 * @param {object} suggestions - Suggestion object from getSuggestions()
 * @returns {string} Formatted suggestions
 */
export function formatSuggestions(suggestions) {
  if (!suggestions) return "No suggestions available";

  const lines = [
    `Issue: ${suggestions.message}`,
    "",
    "Suggestions:",
    ...suggestions.suggestions.map((s, i) => `  ${i + 1}. ${s}`),
    "",
    `Documentation: ${suggestions.docs}`,
    `Severity: ${suggestions.severity}`,
  ];

  return lines.join("\n");
}

/**
 * Add custom suggestions for a specific error code
 *
 * @param {string} errorCode - The error code
 * @param {object} suggestion - Suggestion object
 */
export function addSuggestion(errorCode, suggestion) {
  ERROR_SUGGESTIONS[errorCode] = suggestion;
}

/**
 * Get all error codes with suggestions
 *
 * @returns {string[]} Array of error codes
 */
export function getKnownErrorCodes() {
  return Object.keys(ERROR_SUGGESTIONS);
}

export default {
  getSuggestions,
  getSuggestionsForError,
  formatSuggestions,
  addSuggestion,
  getKnownErrorCodes,
  ERROR_SUGGESTIONS,
};
