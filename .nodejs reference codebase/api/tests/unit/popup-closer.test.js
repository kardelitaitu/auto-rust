/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import PopupCloser from '@api/utils/popup-closer.js';

vi.mock('@api/core/context.js', () => ({
    setSessionInterval: vi.fn((name, fn, ms) => setInterval(fn, ms)),
    clearSessionInterval: vi.fn((name) => {}),
}));

const createMockPage = () => {
    const button = {
        count: vi.fn().mockResolvedValue(0),
        isVisible: vi.fn().mockResolvedValue(false),
        scrollIntoViewIfNeeded: vi.fn().mockResolvedValue(undefined),
        click: vi.fn().mockResolvedValue(undefined),
    };

    const locator = {
        first: vi.fn().mockReturnValue(button),
        count: vi.fn().mockResolvedValue(0),
        isVisible: vi.fn().mockResolvedValue(false),
        scrollIntoViewIfNeeded: vi.fn().mockResolvedValue(undefined),
        click: vi.fn().mockResolvedValue(undefined),
    };

    return {
        isClosed: vi.fn().mockReturnValue(false),
        getByRole: vi.fn().mockReturnValue({ first: vi.fn().mockReturnValue(button) }),
        locator: vi.fn().mockReturnValue(locator),
    };
};

describe('PopupCloser', () => {
    let page;
    let mockLogger;
    let mockApi;

    beforeEach(() => {
        page = createMockPage();
        mockLogger = {
            info: vi.fn(),
            debug: vi.fn(),
            warn: vi.fn(),
            error: vi.fn(),
        };
        mockApi = {
            exists: vi.fn().mockResolvedValue(false),
            click: vi.fn().mockResolvedValue(undefined),
            isSessionActive: vi.fn().mockReturnValue(true),
        };
    });

    afterEach(() => {
        vi.clearAllMocks();
    });

    it('skips execution when aborted', async () => {
        const controller = new AbortController();
        controller.abort();
        const closer = new PopupCloser(page, null, { signal: controller.signal, api: mockApi });

        await closer.runOnce();

        expect(mockApi.exists).not.toHaveBeenCalled();
    });

    it('uses lock when provided', async () => {
        const lock = vi.fn(async (task) => await task());
        const closer = new PopupCloser(page, null, { lock, api: mockApi });

        await closer.runOnce();

        expect(lock).toHaveBeenCalledTimes(1);
    });

    it('skips when shouldSkip returns true', async () => {
        const shouldSkip = vi.fn().mockReturnValue(true);
        const closer = new PopupCloser(page, null, { shouldSkip, api: mockApi });

        await closer.runOnce();

        expect(mockApi.exists).not.toHaveBeenCalled();
    });

    it('skips when already running', async () => {
        const closer = new PopupCloser(page, mockLogger, { api: mockApi });
        closer.running = true;

        await closer.runOnce();

        expect(mockApi.exists).not.toHaveBeenCalled();
    });

    it('skips when page is closed', async () => {
        page.isClosed = vi.fn().mockReturnValue(true);
        const closer = new PopupCloser(page, mockLogger, { api: mockApi });

        await closer.runOnce();

        expect(mockApi.exists).not.toHaveBeenCalled();
    });

    it('closes popup using getByRole button', async () => {
        mockApi.exists.mockResolvedValueOnce(true);
        const closer = new PopupCloser(page, mockLogger, { api: mockApi });
        const result = await closer.runOnce();

        expect(result).toBe(true);
        expect(mockLogger.info).toHaveBeenCalled();
        expect(mockApi.click).toHaveBeenCalled();
    });

    it('closes popup using alternative locator', async () => {
        mockApi.exists.mockResolvedValueOnce(false).mockResolvedValueOnce(true);
        const closer = new PopupCloser(page, mockLogger, { api: mockApi });
        const result = await closer.runOnce();

        expect(result).toBe(true);
        expect(mockLogger.info).toHaveBeenCalled();
        expect(mockApi.click).toHaveBeenCalled();
    });

    it('returns false when no popup found', async () => {
        const closer = new PopupCloser(page, mockLogger, { api: mockApi });
        const result = await closer.runOnce();

        expect(result).toBe(false);
    });

    it('handles errors gracefully', async () => {
        mockApi.exists = vi.fn().mockRejectedValue(new Error('Test error'));
        const closer = new PopupCloser(page, mockLogger, { api: mockApi });
        const result = await closer.runOnce();

        expect(result).toBe(false);
    });

    it('updates lastClosedAt and nextNotifyMinutes', async () => {
        mockApi.exists.mockResolvedValueOnce(true);
        const closer = new PopupCloser(page, mockLogger, { api: mockApi });
        const beforeTime = Date.now();
        await closer.runOnce();
        const afterTime = Date.now();

        expect(closer.lastClosedAt).toBeGreaterThanOrEqual(beforeTime);
        expect(closer.lastClosedAt).toBeLessThanOrEqual(afterTime);
        expect(closer.nextNotifyMinutes).toBe(2);
    });

    it('starts interval timer', () => {
        const closer = new PopupCloser(page, mockLogger, { api: mockApi });
        closer.start();

        expect(closer.timer).not.toBeNull();

        closer.stop();
    });

    it('stops interval timer', () => {
        const closer = new PopupCloser(page, mockLogger, { api: mockApi });
        closer.start();
        closer.stop();

        expect(closer.timer).toBeNull();
    });

    it('does nothing when stop called without timer', () => {
        const closer = new PopupCloser(page, mockLogger, { api: mockApi });
        closer.timer = null;
        closer.stop();
        expect(closer.timer).toBeNull();
    });

    it('does not start if timer already exists', () => {
        const closer = new PopupCloser(page, mockLogger, { api: mockApi });
        closer.timer = 'existing';
        closer.start();

        expect(closer.timer).toBe('existing');
    });

    it('handles internal run with closed page', async () => {
        page.isClosed = vi.fn().mockReturnValue(true);
        const closer = new PopupCloser(page, mockLogger);

        const result = await closer._runOnceInternal();

        expect(result).toBeUndefined();
    });

    it('handles internal run with aborted signal', async () => {
        const controller = new AbortController();
        controller.abort();
        const closer = new PopupCloser(page, mockLogger, { signal: controller.signal });

        const result = await closer._runOnceInternal();

        expect(result).toBeUndefined();
    });

    it('executes runOnce on interval', async () => {
        vi.useFakeTimers();
        const closer = new PopupCloser(page, mockLogger);
        const runOnceSpy = vi.spyOn(closer, 'runOnce').mockResolvedValue(undefined);

        closer.start();

        await vi.advanceTimersByTimeAsync(120000);

        expect(runOnceSpy).toHaveBeenCalled();

        closer.stop();
        vi.useRealTimers();
    });

    it('handles error in interval execution', async () => {
        vi.useFakeTimers();
        const closer = new PopupCloser(page, mockLogger);
        const runOnceSpy = vi
            .spyOn(closer, 'runOnce')
            .mockRejectedValue(new Error('Interval error'));

        closer.start();

        // Should not throw
        await vi.advanceTimersByTimeAsync(120000);

        expect(runOnceSpy).toHaveBeenCalled();

        closer.stop();
        vi.useRealTimers();
    });

    it('handles isVisible error for first button', async () => {
        const button = {
            count: vi.fn().mockResolvedValue(1),
            isVisible: vi.fn().mockRejectedValue(new Error('Visibility check failed')),
            first: vi.fn().mockReturnValue({
                count: vi.fn().mockResolvedValue(1),
                isVisible: vi.fn().mockRejectedValue(new Error('Visibility check failed')),
            }),
        };
        page.getByRole = vi.fn().mockReturnValue(button);

        // Mock locator to return empty so it doesn't try the second path successfully
        page.locator = vi.fn().mockReturnValue({
            first: vi.fn().mockReturnValue({
                count: vi.fn().mockResolvedValue(0),
            }),
        });

        const closer = new PopupCloser(page, mockLogger);
        const result = await closer._runOnceInternal();

        expect(result).toBe(false);
    });

    it('handles isVisible error for alternative button', async () => {
        // First button not found
        page.getByRole = vi.fn().mockReturnValue({
            first: vi.fn().mockReturnValue({
                count: vi.fn().mockResolvedValue(0),
            }),
        });

        // Alt button found but visibility check fails
        const altButton = {
            count: vi.fn().mockResolvedValue(1),
            isVisible: vi.fn().mockRejectedValue(new Error('Visibility check failed')),
            first: vi.fn().mockReturnValue({
                count: vi.fn().mockResolvedValue(1),
                isVisible: vi.fn().mockRejectedValue(new Error('Visibility check failed')),
            }),
        };
        page.locator = vi.fn().mockReturnValue(altButton);

        const closer = new PopupCloser(page, mockLogger);
        const result = await closer._runOnceInternal();

        expect(result).toBe(false);
    });
});
