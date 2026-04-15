/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import * as contextModule from '@api/core/context.js';

vi.mock('@api/core/context.js', () => ({
    getPage: vi.fn(),
    getEvents: vi.fn().mockReturnValue({
        emitSafe: vi.fn(),
    }),
}));

vi.mock('@api/utils/math.js', () => ({
    mathUtils: {
        gaussian: vi.fn().mockReturnValue(0.85),
        randomInRange: vi.fn().mockReturnValue(50),
    },
}));

import { apply, stripCDPMarkers, check } from '@api/utils/patch.js';

describe('api/utils/patch.js', () => {
    let mockPage;
    let mockEvents;

    beforeEach(() => {
        mockEvents = {
            emitSafe: vi.fn(),
        };
        mockPage = {
            addInitScript: vi.fn().mockResolvedValue(undefined),
            evaluate: vi.fn(),
            evaluateOnNewDocument: vi.fn(),
        };
        contextModule.getPage.mockReturnValue(mockPage);
        contextModule.getEvents.mockReturnValue(mockEvents);
    });

    afterEach(() => {
        vi.clearAllMocks();
    });

    describe('apply', () => {
        it('should call addInitScript with spoof data', async () => {
            await apply();
            expect(mockPage.addInitScript).toHaveBeenCalledTimes(1);

            const callArgs = mockPage.addInitScript.mock.calls[0];
            expect(typeof callArgs[0]).toBe('function');
        });

        it('should accept custom fingerprint with all options', async () => {
            const customFingerprint = {
                languages: ['de-DE', 'de'],
                deviceMemory: 16,
                hardwareConcurrency: 12,
                maxTouchPoints: 5,
            };
            await apply(customFingerprint);

            const callArgs = mockPage.addInitScript.mock.calls[0];
            const scriptOptions = callArgs[1];
            expect(scriptOptions.languages).toEqual(['de-DE', 'de']);
            expect(scriptOptions.deviceMemory).toBe(16);
            expect(scriptOptions.hardwareConcurrency).toBe(12);
            expect(scriptOptions.maxTouchPoints).toBe(5);
        });

        it('should use defaults when no fingerprint provided', async () => {
            await apply();

            const callArgs = mockPage.addInitScript.mock.calls[0];
            const scriptOptions = callArgs[1];
            expect(scriptOptions.languages).toEqual(['en-US', 'en']);
            expect(scriptOptions.deviceMemory).toBe(8);
            expect(scriptOptions.hardwareConcurrency).toBe(8);
            expect(scriptOptions.maxTouchPoints).toBe(0);
        });

        it('should handle partial fingerprint options', async () => {
            const partialFingerprint = {
                deviceMemory: 4,
            };
            await apply(partialFingerprint);

            const callArgs = mockPage.addInitScript.mock.calls[0];
            const scriptOptions = callArgs[1];
            expect(scriptOptions.deviceMemory).toBe(4);
        });

        it('should pass spoofData to addInitScript', async () => {
            const fingerprint = {
                languages: ['fr-FR'],
                deviceMemory: 4,
                hardwareConcurrency: 4,
                maxTouchPoints: 1,
            };
            await apply(fingerprint);

            const callArgs = mockPage.addInitScript.mock.calls[0];
            expect(callArgs[1]).toEqual(fingerprint);
        });
    });

    describe('stripCDPMarkers', () => {
        it('should be exported function', () => {
            expect(typeof stripCDPMarkers).toBe('function');
        });

        it('should not throw when called', () => {
            expect(() => stripCDPMarkers()).not.toThrow();
        });
    });

    describe('check', () => {
        it('should evaluate page for detection markers - passed', async () => {
            mockPage.evaluate.mockResolvedValue({
                webdriver: false,
                cdcMarkers: false,
                passed: true,
            });

            const result = await check();

            expect(mockPage.evaluate).toHaveBeenCalled();
            expect(result).toHaveProperty('webdriver');
            expect(result).toHaveProperty('cdcMarkers');
            expect(result).toHaveProperty('passed');
            expect(result.passed).toBe(true);
        });

        it('should evaluate page for detection markers - failed', async () => {
            mockPage.evaluate.mockResolvedValue({
                webdriver: true,
                cdcMarkers: true,
                passed: false,
            });

            const result = await check();

            expect(result.webdriver).toBe(true);
            expect(result.cdcMarkers).toBe(true);
            expect(result.passed).toBe(false);
        });

        it('should return webdriver false when not detected', async () => {
            mockPage.evaluate.mockResolvedValue({
                webdriver: false,
                cdcMarkers: false,
                passed: true,
            });

            const result = await check();

            expect(result.webdriver).toBe(false);
        });

        it('should return cdcMarkers false when not detected', async () => {
            mockPage.evaluate.mockResolvedValue({
                webdriver: false,
                cdcMarkers: false,
                passed: true,
            });

            const result = await check();

            expect(result.cdcMarkers).toBe(false);
        });
    });
});
