/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

// Mock Playwright Page for testing
import { vi } from 'vitest';

export const createMockPage = (overrides = {}) => ({
    goto: vi.fn().mockResolvedValue({}),
    waitForSelector: vi.fn().mockResolvedValue({}),
    waitForLoadState: vi.fn().mockResolvedValue({}),
    click: vi.fn().mockResolvedValue({}),
    dblclick: vi.fn().mockResolvedValue({}),
    rightClick: vi.fn().mockResolvedValue({}),
    hover: vi.fn().mockResolvedValue({}),
    type: vi.fn().mockResolvedValue({}),
    fill: vi.fn().mockResolvedValue({}),
    press: vi.fn().mockResolvedValue({}),
    evaluate: vi.fn().mockResolvedValue({}),
    evaluateHandle: vi.fn().mockResolvedValue({}),
    $$: vi.fn().mockResolvedValue([]),
    $: vi.fn().mockResolvedValue(null),
    innerText: vi.fn().mockResolvedValue(''),
    innerHTML: vi.fn().mockResolvedValue(''),
    textContent: vi.fn().mockResolvedValue(''),
    getAttribute: vi.fn().mockResolvedValue(null),
    isVisible: vi.fn().mockResolvedValue(true),
    isEnabled: vi.fn().mockResolvedValue(true),
    isDisabled: vi.fn().mockResolvedValue(false),
    screenshot: vi.fn().mockResolvedValue(Buffer.from('')),
    addScriptTag: vi.fn().mockResolvedValue({}),
    addStyleTag: vi.fn().mockResolvedValue({}),
    exposeFunction: vi.fn().mockResolvedValue({}),
    route: vi.fn(),
    unroute: vi.fn(),
    waitForResponse: vi.fn().mockResolvedValue({}),
    waitForRequest: vi.fn().mockResolvedValue({}),
    waitForTimeout: vi.fn().mockResolvedValue({}),
    bringToFront: vi.fn().mockResolvedValue({}),
    close: vi.fn().mockResolvedValue({}),
    reload: vi.fn().mockResolvedValue({}),
    url: vi.fn().mockReturnValue('https://example.com'),
    title: vi.fn().mockReturnValue('Test Page'),
    content: vi.fn().mockReturnValue('<html><body>Test</body></html>'),
    ...overrides,
});

// Mock Playwright browser context
export const createMockBrowserContext = (overrides = {}) => ({
    newPage: vi.fn().mockResolvedValue(createMockPage()),
    browser: vi.fn(),
    pages: vi.fn().mockResolvedValue([]),
    close: vi.fn().mockResolvedValue({}),
    ...overrides,
});

// Mock Playwright browser
export const createMockBrowser = (overrides = {}) => ({
    newContext: vi.fn().mockResolvedValue(createMockBrowserContext()),
    close: vi.fn().mockResolvedValue({}),
    version: vi.fn().mockReturnValue('1.0.0'),
    contexts: vi.fn().mockResolvedValue([]),
    ...overrides,
});
