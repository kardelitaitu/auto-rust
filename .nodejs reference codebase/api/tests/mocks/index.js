/**
 * Auto-AI Framework - Standardized Mock Modules
 * Centralized mock definitions for consistent testing
 * @module tests/mocks
 */

import { vi } from 'vitest';
import { createSilentLogger, createMockPage, createMockLocator } from '../utils/test-helpers.js';

// ═══════════════════════════════════════════════════════════════════════════════
// CORE MODULE MOCKS
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Mock for @api/core/logger.js
 */
export const loggerMock = {
    createLogger: () => createSilentLogger(),
    loggerContext: {
        run: vi.fn((ctx, fn) => fn()),
        getStore: vi.fn().mockReturnValue({}),
    },
};

/**
 * Mock for @api/core/context.js
 */
export const contextMock = {
    withPage: vi.fn(async (page, fn) => fn()),
    getContext: vi.fn().mockReturnValue({ page: createMockPage() }),
    setContext: vi.fn(),
    runWithContext: vi.fn(async (ctx, fn) => fn()),
};

/**
 * Mock for @api/core/context-state.js
 */
export const contextStateMock = {
    getStateAgentElementMap: vi.fn().mockReturnValue([]),
    getState: vi.fn().mockReturnValue({}),
    setState: vi.fn(),
};

/**
 * Mock for @api/core/orchestrator.js
 */
export const orchestratorMock = {
    default: vi.fn(function () {
        return {
            startDiscovery: vi.fn().mockResolvedValue(undefined),
            addTask: vi.fn().mockResolvedValue(undefined),
            processTasks: vi.fn().mockResolvedValue(undefined),
            shutdown: vi.fn().mockResolvedValue(undefined),
            sessionManager: {
                activeSessionsCount: 1,
                getAllSessions: vi.fn().mockReturnValue([]),
                createWorker: vi.fn().mockResolvedValue({}),
            },
            on: vi.fn(),
            emit: vi.fn(),
        };
    }),
};

/**
 * Mock for @api/core/sessionManager.js
 */
export const sessionManagerMock = {
    default: vi.fn(function () {
        return {
            loadConfiguration: vi.fn().mockResolvedValue(undefined),
            addSession: vi.fn(),
            getAllSessions: vi.fn().mockReturnValue([]),
            shutdown: vi.fn().mockResolvedValue(undefined),
            markSessionFailed: vi.fn(),
            acquireWorker: vi.fn(),
            releaseWorker: vi.fn(),
            acquirePage: vi.fn(),
            releasePage: vi.fn(),
            replaceBrowserByEndpoint: vi.fn(),
            get activeSessionsCount() {
                return 0;
            },
        };
    }),
};

/**
 * Mock for @api/core/discovery.js
 */
export const discoveryMock = {
    default: vi.fn(function () {
        return {
            loadConnectors: vi.fn().mockResolvedValue(undefined),
            discoverBrowsers: vi.fn().mockResolvedValue([]),
        };
    }),
};

/**
 * Mock for @api/core/automator.js
 */
export const automatorMock = {
    default: vi.fn(function () {
        return {
            connectToBrowser: vi.fn(),
            startHealthChecks: vi.fn(),
            shutdown: vi.fn().mockResolvedValue(undefined),
            checkPageResponsive: vi.fn().mockResolvedValue({ healthy: true }),
        };
    }),
};

// ═══════════════════════════════════════════════════════════════════════════════
// UTILITY MODULE MOCKS
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Mock for @api/utils/configLoader.js
 */
export const configLoaderMock = {
    getSettings: vi.fn().mockResolvedValue({}),
    getTimeoutValue: vi.fn().mockReturnValue(1000),
    loadConfig: vi.fn().mockResolvedValue({}),
    clearCache: vi.fn(),
    ConfigLoader: vi.fn(function () {
        return {
            getSettings: vi.fn().mockResolvedValue({}),
            getTimeoutValue: vi.fn().mockReturnValue(1000),
            loadConfig: vi.fn().mockResolvedValue({}),
            clearCache: vi.fn(),
        };
    }),
};

/**
 * Mock for @api/utils/metrics.js
 */
export const metricsMock = {
    default: {
        recordBrowserDiscovery: vi.fn(),
        recordTaskExecution: vi.fn(),
        getStats: vi.fn().mockReturnValue({}),
        logStats: vi.fn(),
        generateJsonReport: vi.fn().mockResolvedValue(undefined),
        metrics: { startTime: Date.now(), lastResetTime: Date.now() },
    },
};

/**
 * Mock for @api/utils/math.js
 */
export const mathMock = {
    mathUtils: {
        gaussian: vi.fn((mean, _dev) => mean),
        randomInRange: vi.fn((min, max) => Math.floor((min + max) / 2)),
        roll: vi.fn(() => false),
        sample: vi.fn((array) => (array?.length ? array[0] : null)),
        pidStep: vi.fn((state, target) => {
            state.pos = target;
            return state.pos;
        }),
    },
};

/**
 * Mock for @api/utils/ghostCursor.js
 */
export const ghostCursorMock = {
    GhostCursor: vi.fn(function () {
        return {
            move: vi.fn().mockResolvedValue(undefined),
            moveWithHesitation: vi.fn().mockResolvedValue(undefined),
            click: vi.fn().mockResolvedValue({ success: true, usedFallback: false }),
            rightClick: vi.fn().mockResolvedValue({ success: true, usedFallback: false }),
            hoverWithDrift: vi.fn().mockResolvedValue(undefined),
            park: vi.fn().mockResolvedValue(undefined),
        };
    }),
};

/**
 * Mock for @api/utils/validator.js
 */
export const validatorMock = {
    validateTaskExecution: vi.fn(() => ({ isValid: true })),
    validatePayload: vi.fn(() => ({ isValid: true })),
};

// ═══════════════════════════════════════════════════════════════════════════════
// AGENT MODULE MOCKS
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Mock for @api/agent/llmClient.js
 */
export const llmClientMock = {
    llmClient: {
        send: vi.fn().mockResolvedValue({
            success: true,
            content: 'Test LLM response',
            metadata: { model: 'test-model' },
        }),
        classify: vi.fn().mockResolvedValue({ complexity: 5, confidence: 0.8 }),
        config: null,
    },
};

/**
 * Mock for @api/agent/actionEngine.js
 */
export const actionEngineMock = {
    actionEngine: {
        execute: vi.fn().mockResolvedValue({ success: true }),
    },
    default: {
        execute: vi.fn().mockResolvedValue({ success: true }),
    },
};

/**
 * Mock for @api/agent/vision.js
 */
export const visionMock = {
    screenshot: vi.fn().mockResolvedValue(Buffer.from('fake-screenshot')),
    buildPrompt: vi.fn().mockReturnValue('Test prompt'),
    parseResponse: vi.fn().mockReturnValue({ success: true, data: {} }),
    captureAXTree: vi.fn().mockResolvedValue({}),
    captureState: vi.fn().mockResolvedValue({}),
};

// ═══════════════════════════════════════════════════════════════════════════════
// INTERACTION MOCKS
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Mock for @api/interactions/wait.js
 */
export const waitMock = {
    wait: vi.fn().mockResolvedValue(undefined),
    waitWithAbort: vi.fn().mockResolvedValue(undefined),
    waitFor: vi.fn().mockResolvedValue(undefined),
    waitVisible: vi.fn().mockResolvedValue(undefined),
    waitHidden: vi.fn().mockResolvedValue(undefined),
    waitForLoadState: vi.fn().mockResolvedValue(undefined),
    waitForURL: vi.fn().mockResolvedValue(undefined),
};

/**
 * Mock for @api/behaviors/timing.js
 */
export const timingMock = {
    think: vi.fn().mockResolvedValue(undefined),
    delay: vi.fn().mockResolvedValue(undefined),
};

// ═══════════════════════════════════════════════════════════════════════════════
// MOCK FACTORY FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Creates a vi.mock() call for a module
 * @param {string} modulePath - Path to module
 * @param {Object} mockExports - Mock exports
 * @returns {Function} Mock setup function
 */
export const createModuleMock = (modulePath, mockExports) => {
    return () => mockExports;
};

/**
 * Returns mock configurations for use with vi.mock()
 * Note: vi.mock() must be called at module level, not inside functions
 * @param {Array} mocks - Array of [modulePath, mockFactory] pairs
 * @returns {Array} The input array (for chaining)
 */
export const applyMocks = (mocks) => {
    // Return the mocks array - callers should use vi.mock() at module level
    // Example usage in test file:
    //   import { loggerMock, contextMock } from './mocks';
    //   vi.mock('@api/core/logger.js', () => loggerMock);
    //   vi.mock('@api/core/context.js', () => contextMock);
    return mocks;
};

// ═══════════════════════════════════════════════════════════════════════════════
// DEFAULT MOCK PRESETS
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Standard mock preset for core modules
 */
export const coreMocks = () => {
    vi.mock('@api/core/logger.js', () => loggerMock);
    vi.mock('@api/core/context.js', () => contextMock);
    vi.mock('@api/core/context-state.js', () => contextStateMock);
};

/**
 * Standard mock preset for utility modules
 */
export const utilMocks = () => {
    vi.mock('@api/utils/configLoader.js', () => configLoaderMock);
    vi.mock('@api/utils/metrics.js', () => metricsMock);
    vi.mock('@api/utils/math.js', () => mathMock);
    vi.mock('@api/utils/ghostCursor.js', () => ghostCursorMock);
    vi.mock('@api/utils/validator.js', () => validatorMock);
};

/**
 * Standard mock preset for all common modules
 */
export const allMocks = () => {
    coreMocks();
    utilMocks();
};

export default {
    // Core mocks
    loggerMock,
    contextMock,
    contextStateMock,
    orchestratorMock,
    sessionManagerMock,
    discoveryMock,
    automatorMock,

    // Utility mocks
    configLoaderMock,
    metricsMock,
    mathMock,
    ghostCursorMock,
    validatorMock,

    // Agent mocks
    llmClientMock,
    actionEngineMock,
    visionMock,

    // Interaction mocks
    waitMock,
    timingMock,

    // Factories
    createModuleMock,
    applyMocks,

    // Presets
    coreMocks,
    utilMocks,
    allMocks,
};
