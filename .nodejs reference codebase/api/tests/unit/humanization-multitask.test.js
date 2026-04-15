/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { MultitaskEngine } from '@api/behaviors/humanization/multitask.js';
import { mathUtils } from '@api/utils/math.js';
import { scrollRandom } from '@api/behaviors/scroll-helper.js';

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

vi.mock('@api/utils/math.js', () => ({
    mathUtils: {
        roll: vi.fn(),
        randomInRange: vi.fn(),
        gaussian: vi.fn(),
        sample: vi.fn(),
    },
}));
vi.mock('@api/behaviors/scroll-helper.js', () => ({
    scrollRandom: vi.fn().mockResolvedValue(undefined),
}));

describe('MultitaskEngine', () => {
    let multitaskEngine;
    let mockPage;
    let mockLogger;

    beforeEach(() => {
        const mockPageForApi = {
            isClosed: () => false,
            context: () => ({ browser: () => ({ isConnected: () => true }) }),
        };
        if (typeof api !== 'undefined' && api.getPage) api.getPage.mockReturnValue(mockPageForApi);
        mockPage = {
            mouse: {
                move: vi.fn().mockResolvedValue(undefined),
            },
            waitForTimeout: vi.fn().mockResolvedValue(undefined),
        };
        mockLogger = {
            log: vi.fn(),
        };
        multitaskEngine = new MultitaskEngine(mockPage, mockLogger);

        mathUtils.randomInRange.mockReturnValue(100);
        mathUtils.roll.mockReturnValue(false);
    });

    afterEach(() => {
        vi.clearAllMocks();
        vi.restoreAllMocks();
    });

    describe('constructor', () => {
        it('should initialize with page and logger', () => {
            expect(multitaskEngine.page).toBe(mockPage);
            expect(multitaskEngine.logger).toBe(mockLogger);
            expect(multitaskEngine.activities).toHaveLength(5);
        });
    });

    describe('execute', () => {
        it('should execute notifications activity', async () => {
            vi.spyOn(multitaskEngine, '_weightedRandom').mockReturnValue({ name: 'notifications' });
            vi.spyOn(multitaskEngine, 'checkNotifications').mockResolvedValue({
                success: true,
                activity: 'notifications',
            });

            const result = await multitaskEngine.execute();

            expect(multitaskEngine.checkNotifications).toHaveBeenCalled();
            expect(result).toEqual({ success: true, activity: 'notifications' });
        });

        it('should execute trending activity', async () => {
            vi.spyOn(multitaskEngine, '_weightedRandom').mockReturnValue({ name: 'trending' });
            vi.spyOn(multitaskEngine, 'glanceTrending').mockResolvedValue({
                success: true,
                activity: 'trending',
            });

            const result = await multitaskEngine.execute();

            expect(multitaskEngine.glanceTrending).toHaveBeenCalled();
            expect(result).toEqual({ success: true, activity: 'trending' });
        });

        it('should execute position activity', async () => {
            vi.spyOn(multitaskEngine, '_weightedRandom').mockReturnValue({ name: 'position' });
            vi.spyOn(multitaskEngine, 'shiftPosition').mockResolvedValue({
                success: true,
                activity: 'position_shift',
            });

            const result = await multitaskEngine.execute();

            expect(multitaskEngine.shiftPosition).toHaveBeenCalled();
            expect(result).toEqual({ success: true, activity: 'position_shift' });
        });

        it('should execute mentions activity', async () => {
            vi.spyOn(multitaskEngine, '_weightedRandom').mockReturnValue({ name: 'mentions' });
            vi.spyOn(multitaskEngine, 'glanceMentions').mockResolvedValue({
                success: true,
                activity: 'mentions',
            });

            const result = await multitaskEngine.execute();

            expect(multitaskEngine.glanceMentions).toHaveBeenCalled();
            expect(result).toEqual({ success: true, activity: 'mentions' });
        });

        it('should execute idle activity', async () => {
            vi.spyOn(multitaskEngine, '_weightedRandom').mockReturnValue({ name: 'idle' });
            vi.spyOn(multitaskEngine, 'pureIdle').mockResolvedValue({
                success: true,
                activity: 'idle',
            });

            const result = await multitaskEngine.execute();

            expect(multitaskEngine.pureIdle).toHaveBeenCalled();
            expect(result).toEqual({ success: true, activity: 'idle' });
        });

        it('should default to shiftPosition for unknown activity', async () => {
            vi.spyOn(multitaskEngine, '_weightedRandom').mockReturnValue({ name: 'unknown' });
            vi.spyOn(multitaskEngine, 'shiftPosition').mockResolvedValue({
                success: true,
                activity: 'position_shift',
            });

            const result = await multitaskEngine.execute();

            expect(multitaskEngine.shiftPosition).toHaveBeenCalled();
            expect(result).toEqual({ success: true, activity: 'position_shift' });
        });
    });

    describe('checkNotifications', () => {
        it('should move to notification area and back', async () => {
            await multitaskEngine.checkNotifications();
            expect(mockPage.mouse.move).toHaveBeenCalled();
            expect(api.wait).toHaveBeenCalled();
        });
    });

    describe('glanceTrending', () => {
        it('should move to trending area and back', async () => {
            await multitaskEngine.glanceTrending();
            expect(mockPage.mouse.move).toHaveBeenCalled();
            expect(api.wait).toHaveBeenCalled();
        });

        it('should scroll if roll is true', async () => {
            mathUtils.roll.mockReturnValue(true);
            await multitaskEngine.glanceTrending();
            expect(scrollRandom).toHaveBeenCalledWith(30, 80);
        });
    });

    describe('shiftPosition', () => {
        it('should move mouse randomly', async () => {
            await multitaskEngine.shiftPosition();
            expect(mockPage.mouse.move).toHaveBeenCalled();
            expect(api.wait).toHaveBeenCalled();
        });
    });

    describe('glanceMentions', () => {
        it('should move to mentions area and back', async () => {
            await multitaskEngine.glanceMentions();
            expect(mockPage.mouse.move).toHaveBeenCalled();
            expect(api.wait).toHaveBeenCalled();
        });
    });

    describe('pureIdle', () => {
        it('should wait without moving mouse', async () => {
            await multitaskEngine.pureIdle();
            expect(mockPage.mouse.move).not.toHaveBeenCalled();
            expect(api.wait).toHaveBeenCalled();
        });
    });

    describe('quickCheck', () => {
        it('should move mouse slightly', async () => {
            await multitaskEngine.quickCheck();
            expect(mockPage.mouse.move).toHaveBeenCalled();
            expect(api.wait).toHaveBeenCalled();
        });
    });

    describe('simulateNotification', () => {
        it('should check notifications and return status', async () => {
            vi.spyOn(multitaskEngine, 'checkNotifications').mockResolvedValue(undefined);
            mathUtils.roll.mockReturnValue(true);

            const result = await multitaskEngine.simulateNotification();

            expect(multitaskEngine.checkNotifications).toHaveBeenCalled();
            expect(result).toEqual({
                success: true,
                activity: 'notification_check',
                sawNotification: true,
            });
        });
    });

    describe('_moveToArea', () => {
        it('should move mouse in steps', async () => {
            mathUtils.randomInRange.mockReturnValue(2);
            await multitaskEngine._moveToArea(100, 100, 'test');
            expect(mockPage.mouse.move).toHaveBeenCalled();
            expect(mockPage.mouse.move.mock.calls.length).toBeGreaterThanOrEqual(2);
        });
    });

    describe('_weightedRandom', () => {
        it('should return an activity based on random value', () => {
            vi.spyOn(Math, 'random').mockReturnValue(0.1);
            const activity = multitaskEngine._weightedRandom();
            expect(activity.name).toBe('notifications');
        });

        it('should return last activity if random is high', () => {
            vi.spyOn(Math, 'random').mockReturnValue(0.99);
            const activity = multitaskEngine._weightedRandom();
            expect(activity.name).toBe('idle');
        });

        it('should return fallback activity (first item) if loop completes without selection', () => {
            vi.spyOn(Math, 'random').mockReturnValue(2.0);
            const activity = multitaskEngine._weightedRandom();
            expect(activity.name).toBe('notifications');
        });
    });
});
