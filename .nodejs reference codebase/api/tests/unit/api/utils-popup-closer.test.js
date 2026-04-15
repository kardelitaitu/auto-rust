/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import PopupCloser from '@api/utils/popup-closer.js';

vi.mock('@api/core/context.js', () => ({
    setSessionInterval: vi.fn(),
    clearSessionInterval: vi.fn(),
}));

vi.mock('@api/core/logger.js', () => ({
    createLogger: vi.fn().mockReturnValue({
        info: vi.fn(),
        debug: vi.fn(),
    }),
}));

describe('api/utils/popup-closer.js', () => {
    let mockPage;
    let mockLogger;
    let mockApi;

    beforeEach(() => {
        mockPage = {
            isClosed: vi.fn().mockReturnValue(false),
        };
        mockLogger = {
            info: vi.fn(),
            debug: vi.fn(),
        };
        mockApi = {
            exists: vi.fn().mockResolvedValue(false),
            click: vi.fn().mockResolvedValue(true),
        };
    });

    afterEach(() => {
        vi.clearAllMocks();
    });

    describe('constructor', () => {
        it('should initialize with default values', () => {
            const closer = new PopupCloser(mockPage, mockLogger);
            expect(closer.page).toBe(mockPage);
            expect(closer.logger).toBe(mockLogger);
            expect(closer.intervalMs).toBe(120000);
            expect(closer.running).toBe(false);
        });

        it('should accept custom options', () => {
            const mockLock = vi.fn();
            const closer = new PopupCloser(mockPage, mockLogger, { lock: mockLock, api: mockApi });
            expect(closer.lock).toBe(mockLock);
            expect(closer.api).toBe(mockApi);
        });
    });

    describe('start', () => {
        it('should set timer if not already set', () => {
            const closer = new PopupCloser(mockPage, mockLogger);
            closer.start();
            expect(closer.timer).not.toBeNull();
            closer.stop();
        });

        it('should not start if timer already exists', () => {
            const closer = new PopupCloser(mockPage, mockLogger);
            closer.start();
            const firstTimer = closer.timer;
            closer.start();
            expect(closer.timer).toBe(firstTimer);
            closer.stop();
        });
    });

    describe('stop', () => {
        it('should clear timer', () => {
            const closer = new PopupCloser(mockPage, mockLogger);
            closer.start();
            closer.stop();
            expect(closer.timer).toBeNull();
        });
    });

    describe('runOnce', () => {
        it('should return early if page is closed', async () => {
            mockPage.isClosed.mockReturnValue(true);
            const closer = new PopupCloser(mockPage, mockLogger);
            const result = await closer.runOnce();
            expect(result).toBeUndefined();
        });

        it('should return early if already running', async () => {
            const closer = new PopupCloser(mockPage, mockLogger);
            closer.running = true;
            const result = await closer.runOnce();
            expect(result).toBeUndefined();
        });

        it('should return early if shouldSkip returns true', async () => {
            const closer = new PopupCloser(mockPage, mockLogger, { shouldSkip: () => true });
            const result = await closer.runOnce();
            expect(result).toBeUndefined();
        });

        it('should run internal logic when page is valid', async () => {
            const closer = new PopupCloser(mockPage, mockLogger, { api: mockApi });
            const result = await closer.runOnce();
            expect(closer.running).toBe(false);
        });
    });
});
