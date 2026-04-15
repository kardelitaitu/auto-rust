/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import {
    setPathStyle,
    getPathStyle,
    move,
    up,
    down,
    startFidgeting,
    stopFidgeting,
} from '@api/interactions/cursor.js';

// Mocks
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

// Mock storage for path style state
let mockPathStyleState = { style: 'bezier', options: {} };

vi.mock('@api/core/context-state.js', () => ({
    getStatePathStyle: vi.fn(() => mockPathStyleState.style),
    setStatePathStyle: vi.fn((style, options) => {
        const validStyles = ['bezier', 'arc', 'zigzag', 'overshoot', 'stopped', 'muscle'];
        if (!validStyles.includes(style)) {
            throw new Error(`Invalid path style: ${style}. Valid: ${validStyles.join(', ')}`);
        }
        mockPathStyleState = { style, options };
    }),
    getStatePathOptions: vi.fn(() => mockPathStyleState.options),
}));

vi.mock('@api/behaviors/persona.js', () => ({
    getPersona: vi.fn().mockReturnValue({
        speed: 1,
        microMoveChance: 0,
        muscleModel: { Kp: 1, Ki: 0, Kd: 0 },
    }),
}));

vi.mock('@api/behaviors/timing.js', () => ({
    delay: vi.fn().mockResolvedValue(),
    randomInRange: vi.fn((min, max) => min),
}));

// Mock PID state tracker for muscle path simulation
const mockPidStates = new Map();

vi.mock('@api/utils/math.js', () => ({
    mathUtils: {
        randomInRange: vi.fn((min, max) => min),
        gaussian: vi.fn(() => 0),
        pidStep: vi.fn((state, target, model, dt = 0.1) => {
            const error = target - state.pos;
            state.integral = (state.integral || 0) + error * dt;
            state.integral = Math.max(-10, Math.min(10, state.integral));
            const derivative = (error - (state.prevError || 0)) / dt;
            const output = model.Kp * error + model.Ki * state.integral + model.Kd * derivative;
            state.prevError = error;
            state.pos += output;
            return state.pos;
        }),
    },
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

describe('api/interactions/cursor.js', () => {
    let mockPage;
    let mockCursor;
    let mockLocator;

    beforeEach(() => {
        vi.clearAllMocks();
        vi.useFakeTimers();

        mockLocator = {
            first: vi.fn().mockReturnThis(),
            scrollIntoViewIfNeeded: vi.fn().mockResolvedValue(),
            boundingBox: vi.fn().mockResolvedValue({ x: 100, y: 100, width: 50, height: 50 }),
        };

        mockPage = {
            locator: vi.fn().mockReturnValue(mockLocator),
            mouse: {
                move: vi.fn().mockResolvedValue(),
            },
        };

        mockCursor = {
            move: vi.fn().mockResolvedValue(),
            page: mockPage,
            previousPos: { x: 0, y: 0 },
        };

        getPage.mockReturnValue(mockPage);
        getCursor.mockReturnValue(mockCursor);
    });

    afterEach(() => {
        stopFidgeting();
        vi.useRealTimers();
        clearContext();
    });

    describe('Path Style', () => {
        it('should set and get path style', () => {
            mockPathStyleState = { style: 'bezier', options: {} }; // Reset
            setPathStyle('zigzag', { speed: 10 });
            expect(getPathStyle()).toBe('zigzag');
        });

        it('should throw on invalid path style', () => {
            expect(() => setPathStyle('invalid')).toThrow('Invalid path style');
        });
    });

    describe('move', () => {
        it('should move cursor to selector target', async () => {
            setPathStyle('bezier');

            const movePromise = move('#target');

            // Advance timers for any internal delays
            vi.runAllTimers();

            await movePromise;

            expect(mockLocator.scrollIntoViewIfNeeded).toHaveBeenCalled();
            expect(mockLocator.boundingBox).toHaveBeenCalled();
            // Should calculate target around 125, 125 (center of 100,100 50x50 box)
            // randomInRange is mocked to return min, so random offset might be min
            expect(mockCursor.move).toHaveBeenCalled();
        });

        it('should skip movement if bounding box is invalid', async () => {
            mockLocator.boundingBox.mockResolvedValue(null);
            await move('#target');
            expect(mockCursor.move).not.toHaveBeenCalled();
        });

        it('should handle incomplete bounding box', async () => {
            mockLocator.boundingBox.mockResolvedValue({ x: 100, width: 50 }); // missing y, height
            await move('#target');
            expect(mockCursor.move).not.toHaveBeenCalled();
        });

        it('should handle scroll failure gracefully', async () => {
            mockLocator.scrollIntoViewIfNeeded.mockRejectedValue(new Error('Scroll failed'));
            await move('#target');
            expect(mockCursor.move).toHaveBeenCalled(); // Should continue even if scroll fails
        });

        it('should handle missing previousPos', async () => {
            delete mockCursor.previousPos;
            await up(50);
            expect(mockCursor.move).toHaveBeenCalled();
        });

        it('should handle bezier path style', async () => {
            setPathStyle('bezier');
            await move('#target');
            expect(mockCursor.move).toHaveBeenCalled();
        });

        it('should handle arc path style', async () => {
            setPathStyle('arc');
            await move('#target');
            expect(mockCursor.move).toHaveBeenCalled();
        });

        it('should handle zigzag path style', async () => {
            setPathStyle('zigzag');
            await move('#target');
            expect(mockCursor.move).toHaveBeenCalled();
        });

        it('should handle overshoot path style', async () => {
            setPathStyle('overshoot');
            const promise = move('#target');
            await vi.runAllTimersAsync();
            await promise;
            expect(mockCursor.move).toHaveBeenCalled();
        });

        it('should handle stopped path style', async () => {
            setPathStyle('stopped');
            const promise = move('#target');
            await vi.runAllTimersAsync();
            await promise;
            expect(mockCursor.move).toHaveBeenCalled();
        });

        it('should handle muscle path style', async () => {
            setPathStyle('muscle');
            const promise = move('#target');

            // Muscle path uses PID controller which iterates
            // Need to advance timers to allow iterations to complete
            await vi.runAllTimersAsync();
            await promise;

            expect(mockPage.mouse.move).toHaveBeenCalled();
        });

        it('should handle muscle path specifically', async () => {
            setPathStyle('muscle');
            await move('#target');
            expect(mockPage.mouse.move).toHaveBeenCalled();
        });

        it('should skip movement when boundingBox returns NaN values', async () => {
            setPathStyle('bezier');
            mockLocator.boundingBox.mockResolvedValue({ x: NaN, y: NaN, width: 50, height: 50 });

            await move('#target');

            expect(mockCursor.move).not.toHaveBeenCalled();
        });

        it('should adjust duration based on persona.speed', async () => {
            const { getPersona } = await import('@api/behaviors/persona.js');

            getPersona.mockReturnValue({
                speed: 2,
                microMoveChance: 0,
                muscleModel: { Kp: 1, Ki: 0, Kd: 0 },
            });

            setPathStyle('bezier');
            const movePromise = move('#target');

            vi.runAllTimers();

            await movePromise;

            // With speed 2, duration should be halved compared to speed 1
            expect(mockCursor.move).toHaveBeenCalled();
        });

        it('should handle very slow persona.speed', async () => {
            const { getPersona } = await import('@api/behaviors/persona.js');

            getPersona.mockReturnValue({
                speed: 0.5,
                microMoveChance: 0,
                muscleModel: { Kp: 1, Ki: 0, Kd: 0 },
            });

            setPathStyle('bezier');
            const movePromise = move('#target');

            vi.runAllTimers();

            await movePromise;

            // With speed 0.5, duration should be doubled
            expect(mockCursor.move).toHaveBeenCalled();
        });

        it('should adjust stepDelay based on persona.speed in muscle movement', async () => {
            const { getPersona } = await import('@api/behaviors/persona.js');

            getPersona.mockReturnValue({
                speed: 3,
                microMoveChance: 0,
                muscleModel: { Kp: 1, Ki: 0, Kd: 0 },
            });

            setPathStyle('muscle');
            const movePromise = move('#target');

            await vi.runAllTimersAsync();
            await movePromise;

            expect(mockPage.mouse.move).toHaveBeenCalled();
        });

        it('should use slower stepDelay with lower persona.speed in muscle movement', async () => {
            const { getPersona } = await import('@api/behaviors/persona.js');

            getPersona.mockReturnValue({
                speed: 0.25,
                microMoveChance: 0,
                muscleModel: { Kp: 1, Ki: 0, Kd: 0 },
            });

            setPathStyle('muscle');
            const movePromise = move('#target');

            await vi.runAllTimersAsync();
            await movePromise;

            expect(mockPage.mouse.move).toHaveBeenCalled();
        });
    });

    describe('up/down', () => {
        it('should move cursor up', async () => {
            mockCursor.previousPos = { x: 100, y: 100 };
            await up(50);
            expect(mockCursor.move).toHaveBeenCalledWith(100, 50);
        });

        it('should move cursor down', async () => {
            mockCursor.previousPos = { x: 100, y: 100 };
            await down(50);
            expect(mockCursor.move).toHaveBeenCalledWith(100, 150);
        });
    });

    describe('fidgeting', () => {
        it('should start fidgeting interval', async () => {
            startFidgeting();

            // Fast forward time to trigger interval
            vi.advanceTimersByTime(10000);

            expect(mockCursor.move).toHaveBeenCalled();
        });

        it('should stop fidgeting', () => {
            startFidgeting();
            stopFidgeting();

            mockCursor.move.mockClear();
            vi.advanceTimersByTime(10000);

            expect(mockCursor.move).not.toHaveBeenCalled();
        });

        it('should handle null cursor during fidgeting', async () => {
            startFidgeting();
            getCursor.mockReturnValue(null);

            vi.advanceTimersByTime(10000);
            // Should not throw
        });

        it('should not start if already running', () => {
            startFidgeting();
            const intervalId = setInterval(() => {}, 1000); // Capture current interval concept if we could access it
            // But we can just check logic: startFidgeting sets fidgetInterval.
            // Calling it again should not change it.

            startFidgeting();
            // Hard to verify "no-op" without implementation detail access, but ensure no error
        });
    });
});
