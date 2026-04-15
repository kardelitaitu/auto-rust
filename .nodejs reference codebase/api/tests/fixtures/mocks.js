/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

// Common mock objects for testing
import { vi } from 'vitest';

// Mock logger with all required methods
export const mockLogger = {
    info: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
    debug: vi.fn(),
    success: vi.fn(),
};

// Mock config object
export const mockConfig = {
    getSettings: vi.fn().mockResolvedValue({}),
    getTimeouts: vi.fn().mockResolvedValue({ api: { retryDelayMs: 1000, maxRetries: 3 } }),
    getBrowserAPI: vi.fn().mockResolvedValue({}),
};

// Create a mock logger factory
export const createMockLogger = (moduleName = 'test') => ({
    ...mockLogger,
    info: vi.fn((...args) => console.log(`[${moduleName}]`, ...args)),
    warn: vi.fn((...args) => console.warn(`[${moduleName}]`, ...args)),
    error: vi.fn((...args) => console.error(`[${moduleName}]`, ...args)),
    debug: vi.fn((...args) => console.debug(`[${moduleName}]`, ...args)),
    success: vi.fn((...args) => console.log(`[${moduleName}]`, ...args)),
});

// Mock session object
export const mockSession = {
    id: 'session-123',
    browser: {},
    windowName: 'Test Browser',
    ws: 'ws://localhost:9222/devtools',
    http: 'http://localhost:9222',
    port: 9222,
    status: 'active',
};

// Mock task object
export const mockTask = {
    id: 'task-456',
    taskName: 'testTask',
    payload: { test: 'data' },
    status: 'pending',
    sessionId: null,
};

// Mock browser info
export const mockBrowserInfo = {
    ws: 'ws://localhost:9222/devtools',
    http: 'http://localhost:9222',
    windowName: 'Test Browser',
    port: 9222,
    type: 'chrome',
};
