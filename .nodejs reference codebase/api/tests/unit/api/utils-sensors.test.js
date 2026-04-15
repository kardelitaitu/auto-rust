/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import * as contextModule from '@api/core/context.js';

vi.mock('@api/core/context.js', () => ({
    getPage: vi.fn(),
}));

vi.mock('@api/utils/math.js', () => ({
    mathUtils: {
        gaussian: vi.fn().mockReturnValue(0.85),
        randomInRange: vi.fn().mockReturnValue(50),
    },
}));

import { injectSensors } from '@api/utils/sensors.js';
import { mathUtils } from '@api/utils/math.js';

describe('api/utils/sensors.js', () => {
    let mockPage;

    beforeEach(() => {
        mockPage = {
            addInitScript: vi.fn().mockResolvedValue(undefined),
        };
        contextModule.getPage.mockReturnValue(mockPage);
        vi.clearAllMocks();
        // Re-apply mock implementations after clearAllMocks
        mathUtils.gaussian.mockReturnValue(0.85);
        mathUtils.randomInRange.mockReturnValue(50);
    });

    afterEach(() => {
        vi.restoreAllMocks();
    });

    describe('injectSensors', () => {
        it('should call addInitScript with sensor configuration', async () => {
            await injectSensors();

            expect(mockPage.addInitScript).toHaveBeenCalledTimes(1);

            const callArgs = mockPage.addInitScript.mock.calls[0];
            const scriptFn = callArgs[0];
            const scriptOptions = callArgs[1];

            expect(typeof scriptFn).toBe('function');
            expect(scriptOptions).toHaveProperty('level');
            expect(scriptOptions).toHaveProperty('chargingTime');
            expect(scriptOptions).toHaveProperty('dischargingTime');
        });

        it('should pass valid sensor parameters', async () => {
            await injectSensors();

            const callArgs = mockPage.addInitScript.mock.calls[0];
            const scriptOptions = callArgs[1];

            expect(scriptOptions.level).toBeGreaterThanOrEqual(0.5);
            expect(scriptOptions.level).toBeLessThanOrEqual(1.0);
            expect(scriptOptions.chargingTime).toBeGreaterThanOrEqual(0);
            expect(scriptOptions.dischargingTime).toBe(Infinity);
        });

        it('should get page from context', async () => {
            await injectSensors();

            expect(contextModule.getPage).toHaveBeenCalled();
        });

        it('should use mathUtils.gaussian for battery level', async () => {
            mathUtils.gaussian.mockReturnValue(0.9);

            await injectSensors();

            expect(mathUtils.gaussian).toHaveBeenCalledWith(0.85, 0.1, 0.5, 1.0);
        });

        it('should use mathUtils.randomInRange for chargingTime', async () => {
            mathUtils.randomInRange.mockReturnValue(75);

            await injectSensors();

            expect(mathUtils.randomInRange).toHaveBeenCalledWith(0, 100);
        });

        it('should set dischargingTime to Infinity', async () => {
            await injectSensors();

            const callArgs = mockPage.addInitScript.mock.calls[0];
            const scriptOptions = callArgs[1];

            expect(scriptOptions.dischargingTime).toBe(Infinity);
        });

        it('should return undefined', async () => {
            const result = await injectSensors();

            expect(result).toBeUndefined();
        });

        it('should pass a function as first argument', async () => {
            await injectSensors();

            const callArgs = mockPage.addInitScript.mock.calls[0];
            const scriptFn = callArgs[0];

            expect(typeof scriptFn).toBe('function');
        });

        it('should handle different gaussian values', async () => {
            mathUtils.gaussian.mockReturnValue(0.7);

            await injectSensors();

            const callArgs = mockPage.addInitScript.mock.calls[0];
            const scriptOptions = callArgs[1];

            expect(scriptOptions.level).toBe(0.7);
        });

        it('should handle different randomInRange values', async () => {
            mathUtils.randomInRange.mockReturnValue(30);

            await injectSensors();

            const callArgs = mockPage.addInitScript.mock.calls[0];
            const scriptOptions = callArgs[1];

            expect(scriptOptions.chargingTime).toBe(30);
        });

        it('should have consistent function signature', async () => {
            await injectSensors();

            const callArgs = mockPage.addInitScript.mock.calls[0];
            const scriptFn = callArgs[0];
            const scriptOptions = callArgs[1];

            expect(scriptFn.length).toBe(1); // Function has 1 destructured parameter
            expect(scriptOptions).toBeDefined();
        });
    });
});
