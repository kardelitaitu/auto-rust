/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { randomMouse, fakeRead, pause, beforeNavigate } from '@api/behaviors/warmup.js';

// Mocks
vi.mock('@api/core/context.js', () => ({
    getPage: vi.fn(),
    getCursor: vi.fn(),
}));

vi.mock('@api/behaviors/timing.js', () => ({
    think: vi.fn().mockResolvedValue(),
    delay: vi.fn().mockResolvedValue(),
    randomInRange: vi.fn((min, max) => min), // Return min for predictable loops
}));

vi.mock('@api/core/logger.js', () => ({
    createLogger: () => ({
        debug: vi.fn(),
        warn: vi.fn(),
        error: vi.fn(),
    }),
}));

import { getPage, getCursor } from '@api/core/context.js';
import { think, delay, randomInRange } from '@api/behaviors/timing.js';

describe('api/behaviors/warmup.js', () => {
    let mockPage;
    let mockCursor;

    beforeEach(() => {
        vi.clearAllMocks();
        vi.useFakeTimers();

        mockPage = {
            viewportSize: vi.fn().mockReturnValue({ width: 1000, height: 800 }),
            mouse: {
                wheel: vi.fn().mockResolvedValue(),
            },
            evaluate: vi.fn().mockResolvedValue(),
        };

        mockCursor = {
            move: vi.fn().mockResolvedValue(),
        };

        getPage.mockReturnValue(mockPage);
        getCursor.mockReturnValue(mockCursor);
    });

    afterEach(() => {
        vi.useRealTimers();
    });

    describe('randomMouse', () => {
        it('should move cursor randomly', async () => {
            // randomInRange(3, 8) -> 3 moves
            const promise = randomMouse(1000);

            // Advance timers to simulate time passing for loop condition
            // Loop condition: i < moves && (Date.now() - startTime) < duration
            // We need to advance time inside the loop?
            // But randomMouse is async awaiting delay.
            // So we can advance timers.

            await vi.advanceTimersByTimeAsync(1000);
            await promise;

            expect(mockCursor.move).toHaveBeenCalled();
        });

        it('should stop if duration exceeded', async () => {
            // moves = 3, duration = 100
            const nowSpy = vi.spyOn(Date, 'now');
            nowSpy
                .mockReturnValueOnce(1000) // startTime
                .mockReturnValueOnce(1050) // i=0: inside moves, inside duration (1050-1000 < 100)
                .mockReturnValueOnce(1150); // i=1: inside moves, OUTSIDE duration (1150-1000 > 100)

            const promise = randomMouse(100);
            await promise;

            expect(mockCursor.move).toHaveBeenCalledTimes(1);
            nowSpy.mockRestore();
        });

        it('should skip if no viewport', async () => {
            mockPage.viewportSize.mockReturnValue(null);
            await randomMouse();
            expect(mockCursor.move).not.toHaveBeenCalled();
        });
    });

    describe('fakeRead', () => {
        it('should scroll page', async () => {
            await fakeRead(3);

            expect(mockPage.mouse.wheel).toHaveBeenCalledTimes(3);
            expect(delay).toHaveBeenCalledTimes(3);
        });

        it('should fallback to window.scrollBy on wheel error', async () => {
            mockPage.mouse.wheel.mockRejectedValue(new Error('Wheel failed'));

            await fakeRead(1);

            expect(mockPage.evaluate).toHaveBeenCalled(); // window.scrollBy
        });

        it('should scroll up if random is low', async () => {
            const originalRandom = Math.random;
            Math.random = () => 0.1; // <= 0.3 -> direction -1

            await fakeRead(1);

            // Get the second argument of the first call to mouse.wheel (the deltaY)
            const deltaY = mockPage.mouse.wheel.mock.calls[0][1];
            expect(deltaY).toBeLessThan(0);

            Math.random = originalRandom;
        });
    });

    describe('pause', () => {
        it('should call think', async () => {
            await pause(1000, 2000);
            expect(think).toHaveBeenCalled();
        });
    });

    describe('beforeNavigate', () => {
        it('should perform warmup sequence', async () => {
            // Default: mouse=true, fakeRead=false, pause=true
            await beforeNavigate('http://example.com');

            expect(mockCursor.move).toHaveBeenCalled(); // randomMouse
            expect(mockPage.mouse.wheel).not.toHaveBeenCalled(); // fakeRead false
            expect(think).toHaveBeenCalled(); // pause
        });

        it('should respect options', async () => {
            await beforeNavigate('http://example.com', {
                mouse: false,
                fakeRead: true,
                pause: false,
            });

            expect(mockCursor.move).not.toHaveBeenCalled();
            expect(mockPage.mouse.wheel).toHaveBeenCalled();
            expect(think).not.toHaveBeenCalled();
        });
    });
});
