/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { injectSensors } from '@api/utils/sensors.js';

vi.mock('@api/core/context.js', () => ({
    getPage: vi.fn(),
}));

vi.mock('@api/utils/math.js', () => ({
    mathUtils: {
        gaussian: vi.fn().mockReturnValue(0.85),
        randomInRange: vi.fn().mockReturnValue(50),
    },
}));

describe('sensors.js', () => {
    let mockPage;
    let mockGetPage;

    beforeEach(async () => {
        vi.clearAllMocks();
        const { getPage } = await import('@api/core/context.js');
        mockGetPage = getPage;

        mockPage = {
            addInitScript: vi.fn().mockResolvedValue(undefined),
        };
        mockGetPage.mockReturnValue(mockPage);
    });

    describe('injectSensors', () => {
        it('should be a function', () => {
            expect(typeof injectSensors).toBe('function');
        });

        it('should call getPage', async () => {
            await injectSensors();
            expect(mockGetPage).toHaveBeenCalled();
        });

        it('should call page.addInitScript', async () => {
            await injectSensors();
            expect(mockPage.addInitScript).toHaveBeenCalled();
        });

        it('should call page.addInitScript with config object', async () => {
            const { mathUtils } = await import('@api/utils/math.js');

            await injectSensors();

            expect(mockPage.addInitScript).toHaveBeenCalled();
            const callArgs = mockPage.addInitScript.mock.calls[0];
            expect(callArgs).toHaveLength(2);

            // First arg is the callback function
            expect(typeof callArgs[0]).toBe('function');

            // Second arg is the config
            const config = callArgs[1];
            expect(config).toHaveProperty('level');
            expect(config).toHaveProperty('chargingTime');
            expect(config).toHaveProperty('dischargingTime');
            expect(config.dischargingTime).toBe(Infinity);
        });

        it('should use mathUtils.gaussian for battery level', async () => {
            const { mathUtils } = await import('@api/utils/math.js');

            await injectSensors();

            expect(mathUtils.gaussian).toHaveBeenCalledWith(0.85, 0.1, 0.5, 1.0);
        });

        it('should use mathUtils.randomInRange for charging time', async () => {
            const { mathUtils } = await import('@api/utils/math.js');

            await injectSensors();

            expect(mathUtils.randomInRange).toHaveBeenCalledWith(0, 100);
        });

        it('should call addInitScript exactly once', async () => {
            await injectSensors();
            expect(mockPage.addInitScript).toHaveBeenCalledTimes(1);
        });

        it('should set dischargingTime to Infinity', async () => {
            await injectSensors();

            const [, config] = mockPage.addInitScript.mock.calls[0];
            expect(config.dischargingTime).toBe(Infinity);
        });

        it('should return undefined (void function)', async () => {
            const result = await injectSensors();
            expect(result).toBeUndefined();
        });
    });
});
