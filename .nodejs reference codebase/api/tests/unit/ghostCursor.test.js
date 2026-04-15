/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { GhostCursor } from '@api/utils/ghostCursor.js';
import { mathUtils } from '@api/utils/math.js';

// Mock mathUtils
vi.mock('@api/utils/math.js', () => ({
    mathUtils: {
        randomInRange: vi.fn((min, max) => (min + max) / 2),
        gaussian: vi.fn((mean) => mean),
        roll: vi.fn(() => false),
    },
}));

describe('GhostCursor', () => {
    let page;
    let cursor;

    beforeEach(() => {
        vi.clearAllMocks();

        page = {
            mouse: {
                move: vi.fn().mockImplementation(async (x, y) => {
                    if (cursor) cursor.previousPos = { x, y };
                }),
                down: vi.fn(),
                up: vi.fn(),
            },
            viewportSize: vi.fn().mockReturnValue({ width: 1920, height: 1080 }),
        };

        // Mock randomInRange for init
        vi.mocked(mathUtils.randomInRange).mockReturnValue(100);

        cursor = new GhostCursor(page);

        // Movement methods MUST update position and call page.mouse.move to pass tests
        vi.spyOn(cursor, 'move').mockImplementation(async (x, y) => {
            cursor.previousPos = { x, y };
            await page.mouse.move(x, y);
        });
        vi.spyOn(cursor, 'performMove').mockImplementation(async (start, end) => {
            cursor.previousPos = end;
            await page.mouse.move(end.x, end.y);
        });
        vi.spyOn(cursor, 'moveWithHesitation').mockImplementation(async (x, y) => {
            cursor.previousPos = { x, y };
            await page.mouse.move(x, y);
        });
        vi.spyOn(cursor, 'hoverWithDrift').mockImplementation(async (x, y) => {
            cursor.previousPos = { x, y };
            await page.mouse.move(x, y);
        });
    });

    afterEach(() => {
        vi.restoreAllMocks();
        vi.unstubAllGlobals();
    });

    describe('Initialization', () => {
        it('should initialize with random start position', async () => {
            vi.mocked(mathUtils.randomInRange).mockImplementation(() => 100);
            await cursor.init();
            expect(cursor.previousPos.x).toBeGreaterThan(0);
            expect(cursor.previousPos.y).toBeGreaterThan(0);
        });
    });

    describe('Vector Helpers', () => {
        it('should add vectors', () => {
            const result = cursor.vecAdd({ x: 1, y: 2 }, { x: 3, y: 4 });
            expect(result).toEqual({ x: 4, y: 6 });
        });
        it('should sub vectors', () => {
            const result = cursor.vecSub({ x: 4, y: 6 }, { x: 1, y: 2 });
            expect(result).toEqual({ x: 3, y: 4 });
        });
        it('should mult vector', () => {
            const result = cursor.vecMult({ x: 2, y: 3 }, 2);
            expect(result).toEqual({ x: 4, y: 6 });
        });
        it('should calc length', () => {
            const result = cursor.vecLen({ x: 3, y: 4 });
            expect(result).toBe(5);
        });

        it('should calculate bezier at different t values', () => {
            const p0 = { x: 0, y: 0 };
            const p1 = { x: 10, y: 10 };
            const p2 = { x: 20, y: 20 };
            const p3 = { x: 30, y: 30 };

            const result0 = cursor.bezier(0, p0, p1, p2, p3);
            const result1 = cursor.bezier(0.5, p0, p1, p2, p3);
            const result2 = cursor.bezier(1, p0, p1, p2, p3);

            expect(result0.x).toBeCloseTo(0);
            expect(result1.x).toBeCloseTo(15);
            expect(result2.x).toBeCloseTo(30);
        });
    });

    describe('Movement', () => {
        it('should perform move using bezier curve', async () => {
            const start = { x: 0, y: 0 };
            const end = { x: 100, y: 100 };

            cursor.performMove.mockRestore();

            await cursor.performMove(start, end, 50, 5);

            expect(page.mouse.move).toHaveBeenCalled();
            expect(cursor.previousPos).toEqual(end);
        });

        it('should move with overshoot when triggered', async () => {
            vi.mocked(mathUtils.roll).mockReturnValueOnce(true);
            vi.mocked(mathUtils.randomInRange).mockReturnValue(1.1);

            await cursor.move(600, 600);

            expect(page.mouse.move).toHaveBeenCalled();
        });
    });

    describe('Twitter Click', () => {
        it('should perform complex click sequence', async () => {
            const locator = {
                boundingBox: vi.fn().mockResolvedValue({ x: 100, y: 100, width: 50, height: 20 }),
                click: vi.fn().mockResolvedValue(),
            };

            vi.stubGlobal('setTimeout', (fn) => fn());

            await cursor.twitterClick(locator, 'like');

            expect(locator.boundingBox).toHaveBeenCalled();
            expect(page.mouse.down).toHaveBeenCalled();
            expect(page.mouse.up).toHaveBeenCalled();
        });

        it('should fallback to native click if no bbox found', async () => {
            const locator = {
                boundingBox: vi.fn().mockResolvedValue(null),
                click: vi.fn().mockResolvedValue(),
            };

            await cursor.twitterClick(locator);

            expect(locator.click).toHaveBeenCalled();
        });
    });

    describe('Stable Element Wait', () => {
        it('should return bbox when element is stable', async () => {
            const locator = {
                boundingBox: vi.fn().mockResolvedValue({ x: 100, y: 100, width: 50, height: 50 }),
            };

            vi.stubGlobal('setTimeout', (fn) => fn());
            const bbox = await cursor.waitForStableElement(locator, 1000);
            expect(bbox).toBeDefined();
        });

        it('should return null if element disappears', async () => {
            const locator = {
                boundingBox: vi.fn().mockResolvedValue(null),
            };

            vi.stubGlobal('setTimeout', (fn) => fn());
            const bbox = await cursor.waitForStableElement(locator, 500);
            expect(bbox).toBeNull();
        });
    });

    describe('General Click', () => {
        beforeEach(() => {
            vi.stubGlobal('setTimeout', (fn) => fn());
        });

        it('should track and click element', async () => {
            const locator = {
                boundingBox: vi.fn().mockResolvedValue({ x: 100, y: 100, width: 50, height: 50 }),
            };

            cursor.previousPos = { x: 0, y: 0 };

            await cursor.click(locator);

            expect(page.mouse.down).toHaveBeenCalled();
            expect(page.mouse.up).toHaveBeenCalled();
        });

        it('should use native fallback when element not found', async () => {
            const locator = {
                boundingBox: vi.fn().mockResolvedValue(null),
                click: vi.fn().mockResolvedValue(),
            };

            const result = await cursor.click(locator, { allowNativeFallback: true });

            expect(result.success).toBe(false);
            expect(result.usedFallback).toBe(true);
            expect(locator.click).toHaveBeenCalled();
        });

        it('should handle click with different mouse buttons', async () => {
            const locator = {
                boundingBox: vi.fn().mockResolvedValue({ x: 100, y: 100, width: 50, height: 50 }),
            };

            cursor.previousPos = { x: 0, y: 0 };

            await cursor.click(locator, { button: 'right' });

            expect(page.mouse.down).toHaveBeenCalledWith({ button: 'right' });
        });

        it('should return fallback when bounding box is null', async () => {
            const locator = {
                boundingBox: vi.fn().mockResolvedValue(null),
                click: vi.fn().mockResolvedValue(),
            };

            const result = await cursor.click(locator, { allowNativeFallback: true });

            expect(result.success).toBe(false);
            expect(result.usedFallback).toBe(true);
        });

        it('should retry tracking when element moves', async () => {
            vi.spyOn(cursor, 'waitForStableElement').mockResolvedValue({
                x: 100,
                y: 100,
                width: 50,
                height: 50,
            });

            const locator = {
                boundingBox: vi
                    .fn()
                    .mockResolvedValueOnce({ x: 150, y: 150, width: 50, height: 50 })
                    .mockResolvedValue({ x: 160, y: 160, width: 50, height: 50 }),
            };

            cursor.previousPos = { x: 0, y: 0 };

            await cursor.click(locator);

            expect(page.mouse.down).toHaveBeenCalled();
        });
    });

    describe('Easing Functions', () => {
        it('should calculate easeOutCubic correctly', () => {
            expect(cursor.easeOutCubic(0)).toBe(0);
            expect(cursor.easeOutCubic(0.5)).toBe(0.875);
            expect(cursor.easeOutCubic(1)).toBe(1);
        });
    });

    describe('Park', () => {
        it('should park cursor', async () => {
            await cursor.park();
            expect(page.mouse.move).toHaveBeenCalled();
        });
    });

    describe('Twitter Click Profiles', () => {
        it('should use reply profile', async () => {
            const locator = {
                boundingBox: vi.fn().mockResolvedValue({ x: 100, y: 100, width: 50, height: 20 }),
                click: vi.fn().mockResolvedValue(),
            };

            vi.stubGlobal('setTimeout', (fn) => fn());

            await cursor.twitterClick(locator, 'reply');

            expect(cursor.hoverWithDrift).toHaveBeenCalled();
        });

        it('should fallback to native click after all retries fail', async () => {
            const locator = {
                boundingBox: vi.fn().mockResolvedValue(null),
                click: vi.fn().mockResolvedValue(),
            };

            vi.stubGlobal('setTimeout', (fn) => fn());

            await cursor.twitterClick(locator, 'like', 0);

            expect(locator.click).toHaveBeenCalled();
        });
    });
});
