/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { SessionManager } from '@api/behaviors/humanization/session.js';
import { mathUtils } from '@api/utils/math.js';
import * as scrollHelper from '@api/behaviors/scroll-helper.js';

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

// Mock dependencies
vi.mock('@api/utils/math.js', () => ({
    mathUtils: {
        randomInRange: vi.fn(),
        gaussian: vi.fn(),
        sample: vi.fn(),
    },
}));

vi.mock('@api/behaviors/scroll-helper.js', () => ({
    scrollRandom: vi.fn(),
}));

describe('SessionManager', () => {
    let sessionManager;
    let mockPage;
    let mockLogger;

    beforeEach(() => {
        const mockPageForApi = {
            isClosed: () => false,
            context: () => ({ browser: () => ({ isConnected: () => true }) }),
        };
        if (typeof api !== 'undefined' && api.getPage) api.getPage.mockReturnValue(mockPageForApi);
        vi.useFakeTimers();
        // Set default time to Monday noon (weekday, lunch)
        vi.setSystemTime(new Date('2024-01-01T12:00:00')); // Monday

        vi.clearAllMocks();

        mockPage = {
            waitForTimeout: vi.fn().mockResolvedValue(undefined),
            mouse: {
                move: vi.fn().mockResolvedValue(undefined),
            },
        };

        mockLogger = {
            log: vi.fn(),
        };

        // Default mock behaviors
        mathUtils.randomInRange.mockImplementation((min, _max) => min);
        mathUtils.gaussian.mockImplementation((mean, _dev) => mean);
        mathUtils.sample.mockImplementation((arr) => arr[0]);

        sessionManager = new SessionManager(mockPage, mockLogger);
    });

    afterEach(() => {
        vi.useRealTimers();
    });

    describe('getOptimalLength', () => {
        it('should return lunch peak config for 12pm weekday', () => {
            const config = sessionManager.getOptimalLength();
            // Lunch peak: 12 mins
            // 12 * 60 * 1000 = 720000
            expect(config.targetMs).toBe(720000);
            expect(config.reason).toContain('lunch');
            expect(config.reason).toContain('weekday');
        });

        it('should return morning peak config', () => {
            vi.setSystemTime(new Date('2024-01-01T09:00:00')); // 9 AM

            const config = sessionManager.getOptimalLength();
            // Morning peak weekday: 10 mins = 600000
            expect(config.targetMs).toBe(600000);
            expect(config.reason).toContain('morning');
        });

        it('should return weekend config', () => {
            vi.setSystemTime(new Date('2024-01-06T12:00:00')); // Saturday 12 PM

            const config = sessionManager.getOptimalLength();
            // Lunch peak is always 12 mins regardless of weekend in the code
            // Wait, logic:
            // if (hour >= 12 && hour <= 14) baseLength = 12;
            expect(config.targetMs).toBe(720000);
            expect(config.reason).toContain('weekend');
        });

        it('should return late night config', () => {
            vi.setSystemTime(new Date('2024-01-01T23:00:00')); // 11 PM

            const config = sessionManager.getOptimalLength();
            // Late night weekday: 7 mins = 420000 (baseLength = 7)
            expect(config.targetMs).toBe(420000);
            expect(config.reason).toContain('late night');
        });

        it('should return evening peak config', () => {
            vi.setSystemTime(new Date('2024-01-01T19:00:00'));

            const config = sessionManager.getOptimalLength();
            expect(config.targetMs).toBe(900000);
            expect(config.reason).toContain('evening');
        });

        it('should return normal hours config', () => {
            vi.setSystemTime(new Date('2024-01-01T16:00:00'));

            const config = sessionManager.getOptimalLength();
            expect(config.targetMs).toBe(480000);
            expect(config.reason).toContain('weekday');
        });
    });

    describe('shouldTakeBreak', () => {
        it('should return true if duration exceeds threshold', () => {
            // Target is 720000 (12 mins)
            // Threshold is 80% = 576000

            const result = sessionManager.shouldTakeBreak(600000);
            expect(result).toBe(true);
        });

        it('should return false if duration is below threshold', () => {
            const result = sessionManager.shouldTakeBreak(100000);
            expect(result).toBe(false);
        });
    });

    describe('warmup', () => {
        it('should wait for warmup steps', async () => {
            mathUtils.randomInRange.mockReturnValue(3); // 3 steps

            await sessionManager.warmup();

            expect(api.wait).toHaveBeenCalledTimes(3);
        });
    });

    describe('getBreakDuration', () => {
        it('should return random break duration', () => {
            mathUtils.randomInRange.mockReturnValue(3600000);
            const duration = sessionManager.getBreakDuration();
            expect(duration).toBe(3600000);
        });
    });

    describe('boredomPause', () => {
        it('should execute random behavior and wait', async () => {
            // sample returns first behavior (scrollRandom)

            await sessionManager.boredomPause(mockPage);

            expect(scrollHelper.scrollRandom).toHaveBeenCalled();
            expect(api.wait).toHaveBeenCalled();
            expect(mockPage.mouse.move).toHaveBeenCalled(); // Moves back at end
        });
    });

    describe('wrapUp', () => {
        it('should execute wrap up behavior', async () => {
            // Math.random < 0.5 -> behaviors[2] (Final scroll)
            vi.spyOn(Math, 'random').mockReturnValue(0.4);

            await sessionManager.wrapUp(mockPage);

            expect(scrollHelper.scrollRandom).toHaveBeenCalled();
            expect(api.wait).toHaveBeenCalled();
        });

        it('should execute bookmark behavior', async () => {
            vi.spyOn(Math, 'random').mockReturnValue(0.6);

            await sessionManager.wrapUp(mockPage);

            expect(mockPage.mouse.move).toHaveBeenCalledWith(800, 300);
            expect(api.wait).toHaveBeenCalled();
        });

        it('should execute mentions behavior', async () => {
            vi.spyOn(Math, 'random').mockReturnValue(0.9);

            await sessionManager.wrapUp(mockPage);

            expect(mockPage.mouse.move).toHaveBeenCalledWith(100, 100);
            expect(api.wait).toHaveBeenCalled();
        });
    });

    describe('getTimeUntilBreak', () => {
        it('should return remaining time until break', () => {
            const remaining = sessionManager.getTimeUntilBreak(100000);
            expect(remaining).toBe(476000);
        });

        it('should not return negative remaining time', () => {
            const remaining = sessionManager.getTimeUntilBreak(9999999);
            expect(remaining).toBe(0);
        });
    });

    describe('shouldEndSession', () => {
        it('should end when past max', () => {
            const result = sessionManager.shouldEndSession(2000000);
            expect(result).toBe(true);
        });

        it('should end based on target threshold roll', () => {
            vi.spyOn(Math, 'random').mockReturnValue(0.2);
            const result = sessionManager.shouldEndSession(700000);
            expect(result).toBe(true);
        });

        it('should end based on extended threshold roll', () => {
            vi.spyOn(Math, 'random').mockReturnValue(0.5);
            const result = sessionManager.shouldEndSession(800000);
            expect(result).toBe(false);
        });

        it('should continue when below target', () => {
            const result = sessionManager.shouldEndSession(100000);
            expect(result).toBe(false);
        });
    });

    describe('getSessionPhase', () => {
        it('should return warmup phase', () => {
            const phase = sessionManager.getSessionPhase(100000);
            expect(phase.phase).toBe('warmup');
        });

        it('should return active phase', () => {
            const phase = sessionManager.getSessionPhase(400000);
            expect(phase.phase).toBe('active');
        });

        it('should return winding down phase', () => {
            const phase = sessionManager.getSessionPhase(600000);
            expect(phase.phase).toBe('winding_down');
        });

        it('should return ending phase', () => {
            const phase = sessionManager.getSessionPhase(700000);
            expect(phase.phase).toBe('ending');
        });
    });

    describe('getActivityMultiplier', () => {
        it('should return default multiplier for unknown phase', () => {
            const multiplier = sessionManager.getActivityMultiplier('unknown');
            expect(multiplier).toBe(0.9);
        });
    });
});
