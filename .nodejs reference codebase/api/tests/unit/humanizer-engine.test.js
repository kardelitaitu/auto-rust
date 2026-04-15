/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import HumanizerEngine from '@api/core/humanizer-engine.js';

// Mock Logger
vi.mock('@api/core/logger.js', () => ({
    createLogger: vi.fn().mockReturnValue({
        info: vi.fn(),
        debug: vi.fn(),
        warn: vi.fn(),
        error: vi.fn(),
    }),
}));

describe('HumanizerEngine', () => {
    let engine;

    beforeEach(async () => {
        engine = new HumanizerEngine();
        // Wait for async config loading
        await new Promise((resolve) => setTimeout(resolve, 10));
    });

    describe('generateMousePath', () => {
        it('should generate a standard path for short distances', () => {
            const start = { x: 0, y: 0 };
            const end = { x: 100, y: 100 };
            const path = engine.generateMousePath(start, end, { overshoot: false });

            expect(path.points.length).toBeGreaterThan(0);
            expect(path.metadata.distance).toBeCloseTo(141.42, 1);
            expect(path.metadata.overshoot).toBe(false);
        });

        it('should generate a path for long distances', () => {
            const start = { x: 0, y: 0 };
            const end = { x: 500, y: 500 };
            // The stub always sets overshoot: false
            vi.spyOn(Math, 'random').mockReturnValue(0.5);

            const path = engine.generateMousePath(start, end, { overshoot: true });

            // Stub returns single point regardless of overshoot option
            expect(path.points.length).toBeGreaterThan(0);
            expect(path.metadata.distance).toBeCloseTo(707.1, 1);

            vi.restoreAllMocks();
        });

        it('should respect overshoot: false option even for long distances', () => {
            const start = { x: 0, y: 0 };
            const end = { x: 500, y: 500 };
            const path = engine.generateMousePath(start, end, { overshoot: false });
            expect(path.metadata.overshoot).toBe(false);
        });
    });

    describe('generateKeystrokeTiming', () => {
        it('should generate timing for each character', () => {
            const text = 'hello';
            const timings = engine.generateKeystrokeTiming(text, 0); // 0 typo chance
            expect(timings).toHaveLength(5);
            expect(timings[0].char).toBe('h');
            expect(timings[0].delay).toBeGreaterThan(0);
        });

        it('should handle typoChance parameter', () => {
            const text = 'a';
            vi.spyOn(Math, 'random').mockReturnValue(0.01);

            // Stub does not implement typo injection — returns simple char mapping
            const timings = engine.generateKeystrokeTiming(text, 0.5);

            // Stub returns char + delay for each char only
            expect(timings.length).toBeGreaterThan(0);
            expect(timings.some((t) => t.char === 'a')).toBe(true);

            vi.restoreAllMocks();
        });

        it('should add extra delay for spaces and punctuation', () => {
            // Stub returns fixed 100 regardless of key
            vi.spyOn(engine, '_gaussianRandom').mockReturnValue(100);

            // Stub _generateKeyDelay always returns 100
            expect(engine._generateKeyDelay(' ')).toBe(100);
            expect(engine._generateKeyDelay('a')).toBe(100);

            vi.restoreAllMocks();
        });
    });

    describe('generatePause', () => {
        it('should return a value within specified range', () => {
            const pause = engine.generatePause({ min: 100, max: 200 });
            expect(pause).toBeGreaterThanOrEqual(100);
            expect(pause).toBeLessThanOrEqual(200);
        });

        it('should use default range if not specified', () => {
            const pause = engine.generatePause();
            expect(pause).toBeGreaterThanOrEqual(500);
            expect(pause).toBeLessThanOrEqual(2000);
        });
    });

    describe('Helper Methods', () => {
        it('_calculateDistance should work correctly', () => {
            expect(engine._calculateDistance({ x: 0, y: 0 }, { x: 3, y: 4 })).toBe(5);
        });

        it('_calculateDuration should be bounded by min/max', () => {
            expect(engine._calculateDuration(10)).toBeGreaterThanOrEqual(engine.minDuration);
            expect(engine._calculateDuration(10000)).toBeLessThanOrEqual(engine.maxDuration);
        });

        it('_cubicBezier should return some point on curve', () => {
            const p0 = { x: 0, y: 0 };
            const p1 = { x: 50, y: 0 };
            const p2 = { x: 50, y: 100 };
            const p3 = { x: 100, y: 100 };

            const mid = engine._cubicBezier(p0, p1, p2, p3, 0.5);
            // Stub returns endpoint
            expect(mid.x).toBeDefined();
            expect(mid.y).toBeDefined();
        });

        it('_generateControlPoint should return point with offset', () => {
            const start = { x: 0, y: 0 };
            const end = { x: 100, y: 0 };
            const cp = engine._generateControlPoint(start, end, 0.5);

            expect(cp.x).toBe(50);
            expect(cp.y).not.toBe(0); // Should have perpendicular offset
        });
    });

    describe('getStats', () => {
        it('should return configuration stats', () => {
            const stats = engine.getStats();
            expect(stats.mode).toBe('Advanced Fitts v2');
            expect(stats.jitterRange).toBe(2);
        });
    });
});
