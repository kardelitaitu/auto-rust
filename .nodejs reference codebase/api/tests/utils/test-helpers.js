/**
 * Auto-AI Framework - Test Utilities
 * Shared test helpers for consistent test patterns
 * @module tests/utils/test-helpers
 */

import { vi } from "vitest";

// ═══════════════════════════════════════════════════════════════════════════════
// MOCK FACTORIES
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Creates a mock logger with all standard methods
 * @param {string} moduleName - Name for log prefix
 * @returns {Object} Mock logger object
 */
export const createMockLogger = (moduleName = "test") => ({
  info: vi.fn((...args) => console.log(`[${moduleName}]`, ...args)),
  warn: vi.fn((...args) => console.warn(`[${moduleName}]`, ...args)),
  error: vi.fn((...args) => console.error(`[${moduleName}]`, ...args)),
  debug: vi.fn((...args) => console.debug(`[${moduleName}]`, ...args)),
  success: vi.fn((...args) => console.log(`[${moduleName}] ✓`, ...args)),
});

/**
 * Creates a minimal mock logger (silent)
 * @returns {Object} Silent mock logger
 */
export const createSilentLogger = () => ({
  info: vi.fn(),
  warn: vi.fn(),
  error: vi.fn(),
  debug: vi.fn(),
  success: vi.fn(),
});

/**
 * Creates a mock Playwright page with common methods
 * @param {Object} overrides - Override default mock behaviors
 * @returns {Object} Mock page object
 */
export const createMockPage = (overrides = {}) => {
  const defaultPage = {
    // Navigation
    goto: vi.fn().mockResolvedValue(undefined),
    goBack: vi.fn().mockResolvedValue(undefined),
    goForward: vi.fn().mockResolvedValue(undefined),
    reload: vi.fn().mockResolvedValue(undefined),

    // Content
    content: vi.fn().mockResolvedValue("<html><body>Test Page</body></html>"),
    title: vi.fn().mockResolvedValue("Test Page"),
    url: vi.fn().mockReturnValue("https://example.com"),

    // Click
    click: vi.fn().mockResolvedValue(undefined),

    // Screenshot
    screenshot: vi.fn().mockResolvedValue(Buffer.from("fake-screenshot")),

    // Evaluation
    evaluate: vi.fn().mockResolvedValue(undefined),
    evaluateHandle: vi.fn().mockResolvedValue({}),

    // Wait
    waitForTimeout: vi.fn().mockResolvedValue(undefined),
    waitForLoadState: vi.fn().mockResolvedValue(undefined),
    waitForURL: vi.fn().mockResolvedValue(undefined),

    // Viewport
    viewportSize: vi.fn().mockReturnValue({ width: 1280, height: 720 }),

    // Keyboard
    keyboard: {
      press: vi.fn().mockResolvedValue(undefined),
      type: vi.fn().mockResolvedValue(undefined),
      down: vi.fn().mockResolvedValue(undefined),
      up: vi.fn().mockResolvedValue(undefined),
    },

    // Mouse
    mouse: {
      click: vi.fn().mockResolvedValue(undefined),
      dblclick: vi.fn().mockResolvedValue(undefined),
      down: vi.fn().mockResolvedValue(undefined),
      up: vi.fn().mockResolvedValue(undefined),
      move: vi.fn().mockResolvedValue(undefined),
    },

    // Locator
    locator: vi.fn().mockReturnValue(createMockLocator()),
    getByRole: vi.fn().mockReturnValue(createMockLocator()),
    getByText: vi.fn().mockReturnValue(createMockLocator()),
    getByLabel: vi.fn().mockReturnValue(createMockLocator()),
    getByPlaceholder: vi.fn().mockReturnValue(createMockLocator()),
    getByTestId: vi.fn().mockReturnValue(createMockLocator()),

    // Lifecycle
    bringToFront: vi.fn().mockResolvedValue(undefined),
    close: vi.fn().mockResolvedValue(undefined),

    // Events
    on: vi.fn(),
    off: vi.fn(),
    once: vi.fn(),

    // CDP
    context: vi.fn().mockReturnValue({
      newPage: vi.fn().mockResolvedValue(null),
      close: vi.fn().mockResolvedValue(undefined),
    }),
  };

  return { ...defaultPage, ...overrides };
};

/**
 * Creates a mock Playwright locator
 * @param {Object} overrides - Override default mock behaviors
 * @returns {Object} Mock locator object
 */
export const createMockLocator = (overrides = {}) => {
  const defaultLocator = {
    // Actions
    click: vi.fn().mockResolvedValue(undefined),
    fill: vi.fn().mockResolvedValue(undefined),
    type: vi.fn().mockResolvedValue(undefined),
    press: vi.fn().mockResolvedValue(undefined),

    // Queries
    count: vi.fn().mockResolvedValue(1),
    first: vi.fn().mockReturnThis(),
    last: vi.fn().mockReturnThis(),
    nth: vi.fn().mockReturnThis(),

    // State
    isVisible: vi.fn().mockResolvedValue(true),
    isHidden: vi.fn().mockResolvedValue(false),
    isEnabled: vi.fn().mockResolvedValue(true),
    isDisabled: vi.fn().mockResolvedValue(false),

    // Content
    innerText: vi.fn().mockResolvedValue("Test Text"),
    innerHTML: vi.fn().mockResolvedValue("<span>Test</span>"),
    textContent: vi.fn().mockResolvedValue("Test Text"),
    getAttribute: vi.fn().mockResolvedValue("test-value"),

    // Geometry
    boundingBox: vi
      .fn()
      .mockResolvedValue({ x: 100, y: 100, width: 50, height: 50 }),

    // Wait
    waitFor: vi.fn().mockResolvedValue(undefined),

    // Filter
    filter: vi.fn().mockReturnThis(),
  };

  return { ...defaultLocator, ...overrides };
};

/**
 * Creates a mock browser session
 * @param {Object} overrides - Override default values
 * @returns {Object} Mock session object
 */
export const createMockSession = (overrides = {}) => ({
  id: `session-${Date.now()}`,
  browser: createMockBrowser(),
  windowName: "Test Browser",
  ws: "ws://localhost:9222/devtools",
  http: "http://localhost:9222",
  port: 9222,
  status: "active",
  createdAt: new Date(),
  ...overrides,
});

/**
 * Creates a mock browser instance
 * @param {Object} overrides - Override default behaviors
 * @returns {Object} Mock browser object
 */
export const createMockBrowser = (overrides = {}) => ({
  contexts: vi.fn().mockReturnValue([]),
  newContext: vi.fn().mockResolvedValue({
    newPage: vi.fn().mockResolvedValue(createMockPage()),
    close: vi.fn().mockResolvedValue(undefined),
  }),
  close: vi.fn().mockResolvedValue(undefined),
  isConnected: vi.fn().mockReturnValue(true),
  version: vi.fn().mockReturnValue("1.0.0"),
  ...overrides,
});

/**
 * Creates a mock task object
 * @param {Object} overrides - Override default values
 * @returns {Object} Mock task object
 */
export const createMockTask = (overrides = {}) => ({
  id: `task-${Date.now()}`,
  taskName: "testTask",
  payload: { test: "data" },
  status: "pending",
  sessionId: null,
  priority: 1,
  retries: 0,
  maxRetries: 3,
  createdAt: new Date(),
  ...overrides,
});

/**
 * Creates a mock LLM response
 * @param {Object} overrides - Override default values
 * @returns {Object} Mock LLM response
 */
export const createMockLLMResponse = (overrides = {}) => ({
  success: true,
  content: "Test response content",
  metadata: {
    model: "test-model",
    tokens: { prompt: 100, completion: 50 },
    latency: 250,
  },
  ...overrides,
});

// ═══════════════════════════════════════════════════════════════════════════════
// MOCK MODULES
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Creates a mock module for logger
 * @returns {Object} Mock module
 */
export const mockLoggerModule = () => ({
  createLogger: () => createSilentLogger(),
  loggerContext: {
    run: vi.fn((ctx, fn) => fn()),
    getStore: vi.fn().mockReturnValue({}),
  },
});

/**
 * Creates a mock module for config loader
 * @returns {Object} Mock module
 */
export const mockConfigLoaderModule = () => ({
  getSettings: vi.fn().mockResolvedValue({}),
  getTimeoutValue: vi.fn().mockReturnValue(1000),
  loadConfig: vi.fn().mockResolvedValue({}),
  clearCache: vi.fn(),
});

/**
 * Creates a mock module for metrics
 * @returns {Object} Mock module
 */
export const mockMetricsModule = () => ({
  default: {
    recordBrowserDiscovery: vi.fn(),
    recordTaskExecution: vi.fn(),
    getStats: vi.fn().mockReturnValue({}),
    logStats: vi.fn(),
    generateJsonReport: vi.fn().mockResolvedValue(undefined),
    metrics: { startTime: Date.now(), lastResetTime: Date.now() },
  },
});

// ═══════════════════════════════════════════════════════════════════════════════
// ASSERTION HELPERS
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Asserts that a result is successful
 * @param {Object} expect - Vitest expect function
 * @param {Object} result - Result object to check
 * @param {string} message - Optional failure message
 */
export const expectSuccess = (expect, result, message = "Expected success") => {
  expect(result).toBeDefined();
  expect(result.success).toBe(true);
  if (message) {
    expect(result.error).toBeUndefined();
  }
};

/**
 * Asserts that a result is an error
 * @param {Object} expect - Vitest expect function
 * @param {Object} result - Result object to check
 * @param {string} expectedError - Expected error message (optional)
 */
export const expectError = (expect, result, expectedError = null) => {
  expect(result).toBeDefined();
  expect(result.success).toBe(false);
  expect(result.error).toBeDefined();
  if (expectedError) {
    expect(result.error).toContain(expectedError);
  }
};

/**
 * Asserts that a mock function was called with expected args
 * @param {Object} expect - Vitest expect function
 * @param {Function} mockFn - Mock function to check
 * @param {...any} expectedArgs - Expected arguments
 */
export const expectCalledWith = (expect, mockFn, ...expectedArgs) => {
  expect(mockFn).toHaveBeenCalled();
  expect(mockFn).toHaveBeenCalledWith(...expectedArgs);
};

/**
 * Asserts that a promise rejects with expected error
 * @param {Object} expect - Vitest expect function
 * @param {Function} fn - Async function to test
 * @param {string|RegExp} expectedError - Expected error message/pattern
 */
export const expectRejects = async (expect, fn, expectedError) => {
  await expect(fn()).rejects.toThrow(expectedError);
};

// ═══════════════════════════════════════════════════════════════════════════════
// TEST DATA GENERATORS
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Generates random string
 * @param {number} length - String length
 * @returns {string} Random string
 */
export const randomString = (length = 10) => {
  const chars = "abcdefghijklmnopqrstuvwxyz0123456789";
  return Array.from(
    { length },
    () => chars[Math.floor(Math.random() * chars.length)],
  ).join("");
};

/**
 * Generates random URL
 * @returns {string} Random URL
 */
export const randomUrl = () =>
  `https://${randomString(8)}.example.com/${randomString(6)}`;

/**
 * Generates random email
 * @returns {string} Random email
 */
export const randomEmail = () => `${randomString(8)}@example.com`;

// ═══════════════════════════════════════════════════════════════════════════════
// TIMING UTILITIES
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Waits for a specified duration
 * @param {number} ms - Milliseconds to wait
 */
export const wait = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

/**
 * Advances fake timers and waits for pending promises
 * @param {number} ms - Milliseconds to advance
 */
export const advanceTimers = async (ms) => {
  vi.advanceTimersByTime(ms);
  await wait(0); // Allow promises to resolve
};

// ═══════════════════════════════════════════════════════════════════════════════
// SETUP/TEARDOWN HELPERS
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Standard beforeEach setup for tests
 * @param {Object} options - Setup options
 * @returns {Object} Test context
 */
export const setupTest = (options = {}) => {
  const context = {
    logger: createSilentLogger(),
    mockPage: createMockPage(),
    ...options,
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  return context;
};

export default {
  // Factories
  createMockLogger,
  createSilentLogger,
  createMockPage,
  createMockLocator,
  createMockSession,
  createMockBrowser,
  createMockTask,
  createMockLLMResponse,

  // Mock modules
  mockLoggerModule,
  mockConfigLoaderModule,
  mockMetricsModule,

  // Assertions
  expectSuccess,
  expectError,
  expectCalledWith,
  expectRejects,

  // Generators
  randomString,
  randomUrl,
  randomEmail,

  // Timing
  wait,
  advanceTimers,

  // Setup
  setupTest,
};
