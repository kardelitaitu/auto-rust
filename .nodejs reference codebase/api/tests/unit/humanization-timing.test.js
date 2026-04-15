/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { HumanTiming } from '@api/behaviors/humanization/timing.js';

vi.mock('@api/index.js', () => ({
    api: {
        setPage: vi.fn(),
        getPage: vi.fn(),
        wait: vi.fn().mockResolvedValue(undefined),
        think: vi.fn().mockResolvedValue(undefined),
        getPersona: vi.fn().mockReturnValue({ microMoveChance: 0.1, fidgetChance: 0.05 }),
        scroll: Object.assign(vi.fn().mockResolvedValue(undefined), {
            toTop: vi.fn().mockResolvedValue(undefined),
            back: vi.fn().mockResolvedValue(undefined),
            read: vi.fn().mockResolvedValue(undefined),
        }),
        visible: vi.fn().mockResolvedValue(true),
        exists: vi.fn().mockResolvedValue(true),
        getCurrentUrl: vi.fn().mockResolvedValue('https://x.com/home'),
    },
}));
import { api } from '@api/index.js';

// Mock dependencies - must match the actual import path in timing.js
vi.mock('@api/utils/math.js', () => ({
    mathUtils: {
        randomInRange: vi.fn(),
        gaussian: vi.fn(),
        roll: vi.fn(),
    },
}));

vi.mock('@api/utils/entropyController.js', () => ({
    entropy: {
        reactionTime: vi.fn(),
    },
}));

import { mathUtils } from '@api/utils/math.js';

describe('HumanTiming', () => {
    let humanTiming;
    let mockPage;
    let mockLogger;

    beforeEach(() => {
        const mockPageForApi = {
            isClosed: () => false,
            context: () => ({ browser: () => ({ isConnected: () => true }) }),
        };
        if (typeof api !== 'undefined' && api.getPage) api.getPage.mockReturnValue(mockPageForApi);
        vi.useFakeTimers();
        vi.setSystemTime(new Date('2024-01-01T12:00:00')); // Noon
        vi.clearAllMocks();

        mockPage = {
            waitForTimeout: vi.fn().mockResolvedValue(undefined),
        };

        mockLogger = {
            log: vi.fn(),
        };

        mathUtils.gaussian.mockImplementation((mean) => mean);
        mathUtils.randomInRange.mockImplementation((min) => min);
        mathUtils.roll.mockReturnValue(false);

        humanTiming = new HumanTiming(mockPage, mockLogger);
    });

    afterEach(() => {
        vi.useRealTimers();
    });

    describe('getThinkTime', () => {
        it('should return default time for unknown action', () => {
            const time = humanTiming.getThinkTime('unknown');
            expect(time).toBe(500); // baseTimes.default.gaussian = 500
        });

        it('should return specific time for known action (like)', () => {
            const time = humanTiming.getThinkTime('like');
            expect(time).toBe(800); // baseTimes.like.gaussian = 800
        });

        it('should increase time for interesting content', () => {
            const normalTime = humanTiming.getThinkTime('read_tweet');
            const interestingTime = humanTiming.getThinkTime('read_tweet', { interesting: true });

            expect(interestingTime).toBeGreaterThan(normalTime);
            expect(interestingTime).toBe(Math.round(normalTime * 1.3));
        });

        it('should decrease time for boring content', () => {
            const normalTime = humanTiming.getThinkTime('read_tweet');
            const boringTime = humanTiming.getThinkTime('read_tweet', { boring: true });

            expect(boringTime).toBeLessThan(normalTime);
            expect(boringTime).toBe(Math.round(normalTime * 0.7));
        });

        it('should adjust time based on time of day (morning)', () => {
            vi.setSystemTime(new Date('2024-01-01T08:00:00'));
            const time = humanTiming.getThinkTime('general');
            expect(time).toBe(1200); // 1000 * 1.2
        });

        it('should adjust time based on time of day (late night)', () => {
            vi.setSystemTime(new Date('2024-01-01T23:00:00'));
            const time = humanTiming.getThinkTime('general');
            expect(time).toBe(800); // 1000 * 0.8
        });

        it('should clamp to min/max values', () => {
            mathUtils.gaussian.mockReturnValue(100000);
            const time = humanTiming.getThinkTime('like');
            expect(time).toBe(1500); // max for like
        });

        it('should apply fatigue variation at higher cycle counts', () => {
            const time = humanTiming.getThinkTime('general', { cycleCount: 30 });
            expect(time).toBe(900); // 1000 * 0.9
        });
    });

    describe('getNaturalPause', () => {
        it('should return transition pause by default', () => {
            const pause = humanTiming.getNaturalPause();
            expect(pause).toBe(500);
        });

        it('should return specific pause type', () => {
            const pause = humanTiming.getNaturalPause('micro');
            expect(pause).toBe(180);
        });

        it('should return default pause for unknown context', () => {
            const pause = humanTiming.getNaturalPause('unknown');
            expect(pause).toBe(350);
        });
    });

    describe('sessionRampUp', () => {
        it('should wait for ramp up steps', async () => {
            await humanTiming.sessionRampUp();
            expect(api.wait).toHaveBeenCalledTimes(2);
        });
    });

    describe('getFatigueMultiplier', () => {
        it('should return 1.0 for low cycle count', () => {
            expect(humanTiming.getFatigueMultiplier(5)).toBe(1.0);
        });

        it('should return lower multiplier for high cycle count', () => {
            expect(humanTiming.getFatigueMultiplier(100)).toBe(0.8);
            expect(humanTiming.getFatigueMultiplier(101)).toBe(0.8);
        });

        it('should return mid-range multipliers', () => {
            expect(humanTiming.getFatigueMultiplier(20)).toBe(0.95);
            expect(humanTiming.getFatigueMultiplier(40)).toBe(0.9);
            expect(humanTiming.getFatigueMultiplier(70)).toBe(0.85);
        });
    });

    describe('getTypingDelay', () => {
        it('should be slower for first few chars', () => {
            mathUtils.randomInRange.mockReturnValue(200);
            const delay = humanTiming.getTypingDelay(1, 100);
            expect(delay).toBe(200);
        });

        it('should handle middle chars normal path', () => {
            mathUtils.randomInRange.mockReturnValue(50);
            vi.spyOn(Math, 'random').mockReturnValue(0.5); // Normal typing (0.1 <= x < 0.85)
            const delay = humanTiming.getTypingDelay(50, 100);
            expect(delay).toBe(50);
            expect(mathUtils.randomInRange).toHaveBeenCalledWith(30, 100);
        });

        it('should occasionally pause while typing', () => {
            mathUtils.randomInRange.mockReturnValue(300);
            vi.spyOn(Math, 'random').mockReturnValue(0.05); // Pause path (< 0.1)
            const delay = humanTiming.getTypingDelay(50, 100);
            expect(delay).toBe(300);
            expect(mathUtils.randomInRange).toHaveBeenCalledWith(200, 400);
        });

        it('should type fast when variation is high', () => {
            mathUtils.randomInRange.mockReturnValue(25);
            vi.spyOn(Math, 'random').mockReturnValue(0.95); // Fast path (>= 0.85)
            const delay = humanTiming.getTypingDelay(50, 100);
            expect(delay).toBe(25);
            expect(mathUtils.randomInRange).toHaveBeenCalledWith(20, 50);
        });

        it('should slow down at the end', () => {
            mathUtils.randomInRange.mockReturnValue(80);
            const delay = humanTiming.getTypingDelay(90, 100);
            expect(delay).toBe(80);
            expect(mathUtils.randomInRange).toHaveBeenCalledWith(50, 150);
        });
    });

    describe('getHoverTime', () => {
        it('should return hover time for default action', () => {
            const hover = humanTiming.getHoverTime();
            expect(hover).toBe(350);
        });
    });

    describe('getReadingTime', () => {
        it('should compute reading time with multipliers', () => {
            vi.spyOn(Math, 'random').mockReturnValue(0.5); // variation = 0.7 + 0.5 * 0.6 = 1.0
            // wpm = 220. wordCount = 220. minutes = 1. baseMs = 60000.
            // type = 'thread', mult = 1.5. adjustedMs = 90000. variation = 1.0.
            const time = humanTiming.getReadingTime(220, 'thread');
            expect(time).toBe(90000);
        });
    });

    describe('withJitter', () => {
        it('should apply jitter around base', () => {
            const value = humanTiming.withJitter(1000, 0.1);
            expect(value).toBe(1000);
        });
    });

    describe('humanBackoff', () => {
        it('should return capped value when attempt exceeds max', () => {
            const value = humanTiming.humanBackoff(6, 1000, 5);
            expect(value).toBe(5000);
        });

        it('should compute backoff within cap', () => {
            // 1.5^2 * 1000 = 2.25 * 1000 = 2250
            const value = humanTiming.humanBackoff(2, 1000, 5);
            expect(value).toBe(2250);
        });
    });

    describe('random', () => {
        it('should return random range value', () => {
            const value = humanTiming.random(10, 20);
            expect(value).toBe(10);
        });
    });
});
