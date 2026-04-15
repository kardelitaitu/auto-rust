/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';

describe('api/utils/math', () => {
    let mathModule;

    beforeEach(async () => {
        vi.resetModules();
        mathModule = await import('../../../api/utils/math.js');
    });

    describe('mathUtils.gaussian', () => {
        it('should generate a number around the mean', () => {
            vi.spyOn(Math, 'random')
                .mockImplementationOnce(() => 0.5)
                .mockImplementationOnce(() => 0.5);

            const result = mathModule.mathUtils.gaussian(100, 10);

            expect(typeof result).toBe('number');
        });

        it('should apply min boundary', () => {
            vi.spyOn(Math, 'random')
                .mockImplementationOnce(() => 0.01)
                .mockImplementationOnce(() => 0.5);

            const result = mathModule.mathUtils.gaussian(100, 10, 50);

            expect(result).toBeGreaterThanOrEqual(50);
        });

        it('should apply max boundary', () => {
            vi.spyOn(Math, 'random')
                .mockImplementationOnce(() => 0.99)
                .mockImplementationOnce(() => 0.5);

            const result = mathModule.mathUtils.gaussian(100, 10, 0, 150);

            expect(result).toBeLessThanOrEqual(150);
        });

        it('should clamp result between min and max', () => {
            vi.spyOn(Math, 'random')
                .mockImplementationOnce(() => 0.01)
                .mockImplementationOnce(() => 0.99);

            const result = mathModule.mathUtils.gaussian(100, 50, 25, 75);

            expect(result).toBeGreaterThanOrEqual(25);
            expect(result).toBeLessThanOrEqual(75);
        });

        it('should return integer result', () => {
            vi.spyOn(Math, 'random')
                .mockImplementationOnce(() => 0.5)
                .mockImplementationOnce(() => 0.5);

            const result = mathModule.mathUtils.gaussian(100, 10);

            expect(Number.isInteger(result)).toBe(true);
        });
    });

    describe('mathUtils.randomInRange', () => {
        it('should return integer within range', () => {
            vi.spyOn(Math, 'random').mockReturnValue(0.5);

            const result = mathModule.mathUtils.randomInRange(10, 20);

            expect(typeof result).toBe('number');
            expect(Number.isInteger(result)).toBe(true);
        });

        it('should return min when random returns 0', () => {
            vi.spyOn(Math, 'random').mockReturnValue(0);

            const result = mathModule.mathUtils.randomInRange(5, 10);

            expect(result).toBe(5);
        });

        it('should return max when random returns close to 1', () => {
            vi.spyOn(Math, 'random').mockReturnValue(0.9999);

            const result = mathModule.mathUtils.randomInRange(5, 10);

            expect(result).toBe(10);
        });
    });

    describe('mathUtils.roll', () => {
        it('should return true when random is below threshold', () => {
            vi.spyOn(Math, 'random').mockReturnValue(0.3);

            const result = mathModule.mathUtils.roll(0.5);

            expect(result).toBe(true);
        });

        it('should return false when random is above threshold', () => {
            vi.spyOn(Math, 'random').mockReturnValue(0.8);

            const result = mathModule.mathUtils.roll(0.5);

            expect(result).toBe(false);
        });

        it('should always return true when threshold is 1', () => {
            vi.spyOn(Math, 'random').mockReturnValue(0.99);

            const result = mathModule.mathUtils.roll(1);

            expect(result).toBe(true);
        });

        it('should always return false when threshold is 0', () => {
            vi.spyOn(Math, 'random').mockReturnValue(0.01);

            const result = mathModule.mathUtils.roll(0);

            expect(result).toBe(false);
        });
    });

    describe('mathUtils.sample', () => {
        it('should return random element from array', () => {
            vi.spyOn(Math, 'random').mockReturnValue(0.5);

            const arr = ['a', 'b', 'c'];
            const result = mathModule.mathUtils.sample(arr);

            expect(arr).toContain(result);
        });

        it('should return null for empty array', () => {
            const result = mathModule.mathUtils.sample([]);

            expect(result).toBeNull();
        });

        it('should return null for null input', () => {
            const result = mathModule.mathUtils.sample(null);

            expect(result).toBeNull();
        });

        it('should return null for undefined input', () => {
            const result = mathModule.mathUtils.sample(undefined);

            expect(result).toBeNull();
        });

        it('should return only element for single-item array', () => {
            vi.spyOn(Math, 'random').mockReturnValue(0.5);

            const result = mathModule.mathUtils.sample(['only']);

            expect(result).toBe('only');
        });
    });

    describe('mathUtils.pidStep', () => {
        it('should calculate pid output', () => {
            const state = { pos: 10, integral: 0, prevError: 0 };
            const model = { Kp: 1, Ki: 0.1, Kd: 0.01 };

            const result = mathModule.mathUtils.pidStep(state, 20, model);

            expect(typeof result).toBe('number');
            expect(state.prevError).toBe(10);
        });

        it('should clamp integral term', () => {
            const state = { pos: 0, integral: 0, prevError: 0 };
            const model = { Kp: 0, Ki: 100, Kd: 0 }; // High Ki to cause clamping

            mathModule.mathUtils.pidStep(state, 100, model);

            expect(state.integral).toBeLessThanOrEqual(10);
            expect(state.integral).toBeGreaterThanOrEqual(-10);
        });

        it('should use default dt when not provided', () => {
            const state = { pos: 10, integral: 0, prevError: 0 };
            const model = { Kp: 1, Ki: 0, Kd: 0 };

            const result1 = mathModule.mathUtils.pidStep(state, 20, model);
            const result2 = mathModule.mathUtils.pidStep(state, 20, model, 0.1);

            expect(result1).toBe(result2);
        });

        it('should update state.pos', () => {
            const state = { pos: 10, integral: 0, prevError: 0 };
            const model = { Kp: 1, Ki: 0, Kd: 0 };

            mathModule.mathUtils.pidStep(state, 20, model);

            expect(state.pos).not.toBe(10);
        });

        it('should handle missing prevError', () => {
            const state = { pos: 10, integral: 0 };
            const model = { Kp: 1, Ki: 0, Kd: 1 };

            const result = mathModule.mathUtils.pidStep(state, 20, model);

            expect(typeof result).toBe('number');
        });
    });
});
