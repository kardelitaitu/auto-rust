/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { start, stop, isRunning, startHeartbeat, wiggle, idleScroll } from '@api/behaviors/idle.js';

// Mocks
// Shared state for mocking context-state.js
const mockIdleState = { isRunning: false, fidgetInterval: null };
const mockIntervals = new Map();

vi.mock('@api/core/context.js', () => ({
    getPage: vi.fn(),
    getCursor: vi.fn(),
    clearContext: vi.fn(),
    setSessionInterval: vi.fn((name, fn, ms) => {
        const id = setInterval(fn, ms);
        mockIntervals.set(name, id);
        return id;
    }),
    clearSessionInterval: vi.fn((name) => {
        const id = mockIntervals.get(name);
        if (id) {
            clearInterval(id);
            mockIntervals.delete(name);
        }
    }),
}));

vi.mock('@api/core/context-state.js', () => ({
    getStateIdle: vi.fn(() => mockIdleState),
    setStateIdle: vi.fn((state) => Object.assign(mockIdleState, state)),
}));

vi.mock('@api/behaviors/timing.js', () => ({
    randomInRange: vi.fn((min, max) => max), // Return max for deterministic movement
}));

vi.mock('@api/core/logger.js', () => ({
    createLogger: () => ({
        debug: vi.fn(),
        warn: vi.fn(),
        error: vi.fn(),
    }),
}));

import {
    getPage,
    getCursor,
    setSessionInterval,
    clearSessionInterval,
    clearContext,
} from '@api/core/context.js';
import { randomInRange } from '@api/behaviors/timing.js';
import { getStateIdle, setStateIdle } from '@api/core/context-state.js';

describe('api/behaviors/idle.js', () => {
    let mockPage;
    let mockCursor;

    beforeEach(() => {
        vi.clearAllMocks();
        vi.useFakeTimers();

        mockPage = {
            mouse: {
                wheel: vi.fn().mockResolvedValue(),
            },
            evaluate: vi.fn().mockResolvedValue(),
        };

        mockCursor = {
            move: vi.fn().mockResolvedValue(),
            previousPos: { x: 100, y: 100 },
        };

        getPage.mockReturnValue(mockPage);
        getCursor.mockReturnValue(mockCursor);
        mockIdleState.isRunning = false;
        mockIdleState.fidgetInterval = null;
        mockIntervals.clear();

        stop();
    });

    afterEach(() => {
        stop();
        vi.useRealTimers();
        clearContext();
    });

    describe('start/stop', () => {
        it('should start idle loop', () => {
            start({ frequency: 1000 });
            expect(isRunning()).toBe(true);

            // Advance time to trigger interval
            vi.advanceTimersByTime(1100);

            expect(mockCursor.move).toHaveBeenCalled();
        });

        it('should not start if already running', () => {
            start();
            const firstInterval = setInterval; // Not accessible directly, but we can verify behavior

            start(); // Should return early

            expect(isRunning()).toBe(true);
        });

        it('should stop idle loop', () => {
            start({ frequency: 1000 });
            stop();
            expect(isRunning()).toBe(false);

            mockCursor.move.mockClear();
            vi.advanceTimersByTime(1100);
            expect(mockCursor.move).not.toHaveBeenCalled();
        });
    });

    describe('Idle Behavior', () => {
        it('should wiggle cursor', async () => {
            start({ frequency: 1000, wiggle: true, scroll: false });

            vi.advanceTimersByTime(1100);

            // With max randomInRange, delta should be magnitude (5)
            // New pos: 100 + 5, 100 + 5
            expect(mockCursor.move).toHaveBeenCalledWith(105, 105);
        });

        it('should scroll occasionally', async () => {
            const originalRandom = Math.random;
            Math.random = () => 0.8; // > 0.7 threshold for scroll

            start({ frequency: 1000, wiggle: false, scroll: true });

            vi.advanceTimersByTime(1100);

            expect(mockPage.mouse.wheel).toHaveBeenCalled();

            Math.random = originalRandom;
        });

        it('should fallback to window.scrollBy on wheel error', async () => {
            const originalRandom = Math.random;
            Math.random = () => 0.8;

            mockPage.mouse.wheel.mockRejectedValue(new Error('Wheel failed'));

            start({ frequency: 1000, wiggle: false, scroll: true });

            // Wait for async interval callback
            await vi.advanceTimersByTimeAsync(1100);

            expect(mockPage.evaluate).toHaveBeenCalled(); // window.scrollBy

            Math.random = originalRandom;
        });
    });

    describe('startHeartbeat', () => {
        it('should start with heartbeat defaults', () => {
            startHeartbeat();
            expect(isRunning()).toBe(true);
            // Heartbeat frequency is 30000
            vi.advanceTimersByTime(1000);
            expect(mockCursor.move).not.toHaveBeenCalled();

            vi.advanceTimersByTime(30000);
            expect(mockCursor.move).toHaveBeenCalled();
        });
    });

    describe('Single Actions', () => {
        it('should perform single wiggle', async () => {
            await wiggle(10);
            expect(mockCursor.move).toHaveBeenCalledWith(110, 110);
        });

        it('should perform single idle scroll', async () => {
            await idleScroll(50);
            expect(mockPage.mouse.wheel).toHaveBeenCalled();
        });
    });
});
