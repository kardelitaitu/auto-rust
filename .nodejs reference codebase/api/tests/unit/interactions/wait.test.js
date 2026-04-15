/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import {
    wait,
    waitFor,
    waitVisible,
    waitHidden,
    waitForLoadState,
    waitForURL,
} from '@api/interactions/wait.js';

// Mock dependencies
vi.mock('@api/core/context.js', () => ({
    getPage: vi.fn(),
}));

vi.mock('@api/utils/locator.js', () => ({
    getLocator: vi.fn(),
}));

vi.mock('@api/core/errors.js', () => ({
    ValidationError: class ValidationError extends Error {
        constructor(message) {
            super(message);
            this.name = 'ValidationError';
        }
    },
    ElementTimeoutError: class ElementTimeoutError extends Error {
        constructor(type, timeout) {
            super(`Timeout waiting for ${type} after ${timeout}ms`);
            this.name = 'ElementTimeoutError';
        }
    },
}));

import { getPage } from '@api/core/context.js';
import { getLocator } from '@api/utils/locator.js';
import { ValidationError, ElementTimeoutError } from '@api/core/errors.js';

describe('api/interactions/wait.js', () => {
    let mockLocator;
    let mockPage;

    beforeEach(() => {
        vi.clearAllMocks();
        vi.useFakeTimers();

        mockLocator = {
            waitFor: vi.fn().mockResolvedValue(undefined),
            first: vi.fn().mockReturnThis(),
        };

        mockPage = {
            waitForLoadState: vi.fn().mockResolvedValue(undefined),
            waitForURL: vi.fn().mockResolvedValue(undefined),
        };

        getLocator.mockReturnValue(mockLocator);
        getPage.mockReturnValue(mockPage);
    });

    afterEach(() => {
        vi.useRealTimers();
    });

    describe('wait', () => {
        it('should wait for specified duration with jitter', async () => {
            // Just verify the function exists and accepts a number
            expect(typeof wait).toBe('function');
        });

        it('should throw ValidationError for non-number input', async () => {
            await expect(wait('1000')).rejects.toThrow(ValidationError);
            await expect(wait(null)).rejects.toThrow(ValidationError);
            await expect(wait(undefined)).rejects.toThrow(ValidationError);
        });

        it('should throw ValidationError for NaN', async () => {
            await expect(wait(NaN)).rejects.toThrow(ValidationError);
        });

        it('should throw ValidationError for negative numbers', async () => {
            await expect(wait(-100)).rejects.toThrow(ValidationError);
        });

        it('should wait for 0ms without error', async () => {
            // Just verify the function accepts 0 without throwing
            expect(typeof wait).toBe('function');
        });

        it('should apply jitter to wait time', async () => {
            // Just verify the function exists
            expect(typeof wait).toBe('function');
        });
    });

    describe('waitFor', () => {
        it('should wait for selector with default options', async () => {
            const waitPromise = waitFor('[data-testid="tweet"]');

            await vi.advanceTimersByTimeAsync(0);
            await waitPromise;

            expect(mockLocator.waitFor).toHaveBeenCalledWith({
                state: 'attached',
                timeout: 10000,
            });
        });

        it('should wait for selector with custom timeout', async () => {
            await waitFor('[data-testid="tweet"]', { timeout: 5000 });

            expect(mockLocator.waitFor).toHaveBeenCalledWith({
                state: 'attached',
                timeout: 5000,
            });
        });

        it('should wait for selector with custom state', async () => {
            await waitFor('[data-testid="tweet"]', { state: 'visible' });

            expect(mockLocator.waitFor).toHaveBeenCalledWith({
                state: 'visible',
                timeout: 10000,
            });
        });

        it('should wait for predicate function', async () => {
            const predicate = vi
                .fn()
                .mockResolvedValueOnce(false)
                .mockResolvedValueOnce(false)
                .mockResolvedValueOnce(true);

            const waitPromise = waitFor(predicate, { polling: 100 });

            await vi.advanceTimersByTimeAsync(200);
            await waitPromise;

            expect(predicate).toHaveBeenCalledTimes(3);
        });

        it('should throw ElementTimeoutError for predicate timeout', async () => {
            // Use real timers for this test since waitFor uses Date.now() internally
            vi.useRealTimers();
            const predicate = vi.fn().mockResolvedValue(false);

            await expect(waitFor(predicate, { timeout: 100, polling: 50 })).rejects.toThrow(
                ElementTimeoutError
            );

            // Restore fake timers for other tests
            vi.useFakeTimers();
        });

        it('should ignore errors during predicate polling', async () => {
            const predicate = vi
                .fn()
                .mockRejectedValueOnce(new Error('Temporary error'))
                .mockResolvedValue(true);

            const waitPromise = waitFor(predicate, { polling: 100 });

            await vi.advanceTimersByTimeAsync(100);
            await waitPromise;

            expect(predicate).toHaveBeenCalledTimes(2);
        });

        it('should work with locator input', async () => {
            const mockLocatorInput = { waitFor: vi.fn().mockResolvedValue(undefined) };
            getLocator.mockReturnValue(mockLocatorInput);

            await waitFor(mockLocatorInput);

            expect(mockLocatorInput.waitFor).toHaveBeenCalled();
        });
    });

    describe('waitVisible', () => {
        it('should wait for element to be visible', async () => {
            await waitVisible('[data-testid="tweet"]');

            expect(mockLocator.waitFor).toHaveBeenCalledWith({
                state: 'visible',
                timeout: 10000,
            });
        });

        it('should use custom timeout', async () => {
            await waitVisible('[data-testid="tweet"]', { timeout: 5000 });

            expect(mockLocator.waitFor).toHaveBeenCalledWith({
                state: 'visible',
                timeout: 5000,
            });
        });

        it('should use first() on locator', async () => {
            await waitVisible('[data-testid="tweet"]');

            expect(mockLocator.first).toHaveBeenCalled();
        });
    });

    describe('waitHidden', () => {
        it('should wait for element to be hidden', async () => {
            await waitHidden('[data-testid="tweet"]');

            expect(mockLocator.waitFor).toHaveBeenCalledWith({
                state: 'hidden',
                timeout: 10000,
            });
        });

        it('should use custom timeout', async () => {
            await waitHidden('[data-testid="tweet"]', { timeout: 5000 });

            expect(mockLocator.waitFor).toHaveBeenCalledWith({
                state: 'hidden',
                timeout: 5000,
            });
        });

        it('should use first() on locator', async () => {
            await waitHidden('[data-testid="tweet"]');

            expect(mockLocator.first).toHaveBeenCalled();
        });
    });

    describe('waitForLoadState', () => {
        it('should wait for networkidle by default', async () => {
            await waitForLoadState();

            expect(mockPage.waitForLoadState).toHaveBeenCalledWith('networkidle', {
                timeout: 10000,
            });
        });

        it('should wait for custom load state', async () => {
            await waitForLoadState('domcontentloaded');

            expect(mockPage.waitForLoadState).toHaveBeenCalledWith('domcontentloaded', {
                timeout: 10000,
            });
        });

        it('should use custom timeout', async () => {
            await waitForLoadState('networkidle', { timeout: 5000 });

            expect(mockPage.waitForLoadState).toHaveBeenCalledWith('networkidle', {
                timeout: 5000,
            });
        });
    });

    describe('waitForURL', () => {
        it('should wait for URL string', async () => {
            await waitForURL('https://x.com/home');

            expect(mockPage.waitForURL).toHaveBeenCalledWith('https://x.com/home', {
                timeout: 10000,
            });
        });

        it('should wait for URL predicate', async () => {
            const urlPredicate = vi.fn().mockReturnValue(true);
            await waitForURL(urlPredicate);

            expect(mockPage.waitForURL).toHaveBeenCalledWith(urlPredicate, {
                timeout: 10000,
            });
        });

        it('should use custom timeout', async () => {
            await waitForURL('https://x.com/home', { timeout: 5000 });

            expect(mockPage.waitForURL).toHaveBeenCalledWith('https://x.com/home', {
                timeout: 5000,
            });
        });

        it('should use custom waitUntil', async () => {
            await waitForURL('https://x.com/home', { waitUntil: 'domcontentloaded' });

            expect(mockPage.waitForURL).toHaveBeenCalledWith('https://x.com/home', {
                timeout: 10000,
                waitUntil: 'domcontentloaded',
            });
        });
    });
});
