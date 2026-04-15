/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

vi.mock('@api/utils/proxy-agent.js', () => ({
    createProxyAgent: vi.fn(),
}));

vi.mock('@api/core/logger.js', () => ({
    createLogger: vi.fn().mockReturnValue({
        info: vi.fn(),
        warn: vi.fn(),
        debug: vi.fn(),
        error: vi.fn(),
    }),
}));

import { FreeOpenRouterHelper } from '@api/utils/free-openrouter-helper.js';

describe('api/utils/free-openrouter-helper.js', () => {
    beforeEach(() => {
        FreeOpenRouterHelper.resetInstance();
    });

    afterEach(() => {
        FreeOpenRouterHelper.resetInstance();
    });

    describe('singleton pattern', () => {
        it('should return same instance on multiple getInstance calls', () => {
            const instance1 = FreeOpenRouterHelper.getInstance();
            const instance2 = FreeOpenRouterHelper.getInstance();
            expect(instance1).toBe(instance2);
        });

        it('should create new instance if reset', () => {
            const instance1 = FreeOpenRouterHelper.getInstance();
            FreeOpenRouterHelper.resetInstance();
            const instance2 = FreeOpenRouterHelper.getInstance();
            expect(instance1).not.toBe(instance2);
        });
    });

    describe('constructor', () => {
        it('should initialize with default values', () => {
            const helper = new FreeOpenRouterHelper();
            expect(helper.apiKeys).toEqual([]);
            expect(helper.models).toEqual([]);
            expect(helper.proxy).toBeNull();
            expect(helper.endpoint).toBe('https://openrouter.ai/api/v1/chat/completions');
            expect(helper.testTimeout).toBe(15000);
        });

        it('should accept custom options', () => {
            const options = {
                apiKeys: ['key1', 'key2'],
                models: ['model1', 'model2'],
                proxy: 'http://proxy:8080',
                testTimeout: 30000,
            };
            const helper = new FreeOpenRouterHelper(options);
            expect(helper.apiKeys).toEqual(['key1', 'key2']);
            expect(helper.models).toEqual(['model1', 'model2']);
            expect(helper.proxy).toBe('http://proxy:8080');
            expect(helper.testTimeout).toBe(30000);
        });
    });

    describe('getInstance', () => {
        it('should accept options on first call', () => {
            const helper = FreeOpenRouterHelper.getInstance({ apiKeys: ['test'] });
            expect(helper.apiKeys).toEqual(['test']);
        });
    });
});
