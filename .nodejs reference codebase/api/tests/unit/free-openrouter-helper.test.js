/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Unit Tests for FreeOpenRouterHelper
 * @module tests/unit/free-openrouter-helper.test
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

vi.mock('@api/core/logger.js', () => ({
    createLogger: () => ({
        info: vi.fn(),
        warn: vi.fn(),
        debug: vi.fn(),
        error: vi.fn(),
    }),
}));

vi.mock('@api/utils/proxy-agent.js', () => ({
    createProxyAgent: vi.fn().mockReturnValue({
        getAgent: vi.fn().mockResolvedValue(null),
    }),
}));

global.fetch = vi.fn();

const { FreeOpenRouterHelper } = await import('../../utils/free-openrouter-helper.js');

describe('FreeOpenRouterHelper', () => {
    beforeEach(() => {
        vi.clearAllMocks();
        FreeOpenRouterHelper.resetInstance();
    });

    afterEach(() => {
        FreeOpenRouterHelper.resetInstance();
    });

    describe('getInstance', () => {
        it('should return singleton instance', () => {
            const instance1 = FreeOpenRouterHelper.getInstance();
            const instance2 = FreeOpenRouterHelper.getInstance();
            expect(instance1).toBe(instance2);
        });

        it('should pass options to constructor', () => {
            const instance = FreeOpenRouterHelper.getInstance({
                apiKeys: ['key1'],
                models: ['model1'],
            });
            expect(instance.apiKeys).toEqual(['key1']);
            expect(instance.models).toEqual(['model1']);
        });
    });

    describe('resetInstance', () => {
        it('should reset singleton to null', () => {
            const instance1 = FreeOpenRouterHelper.getInstance();
            FreeOpenRouterHelper.resetInstance();
            const instance2 = FreeOpenRouterHelper.getInstance();
            expect(instance1).not.toBe(instance2);
        });
    });

    describe('constructor', () => {
        it('should initialize with default options', () => {
            const helper = new FreeOpenRouterHelper();
            expect(helper.apiKeys).toEqual([]);
            expect(helper.models).toEqual([]);
            expect(helper.proxy).toBeNull();
            expect(helper.testTimeout).toBe(15000);
            expect(helper.currentKeyIndex).toBe(0);
            expect(helper.results).toBeNull();
            expect(helper.testing).toBe(false);
        });

        it('should accept custom options', () => {
            const helper = new FreeOpenRouterHelper({
                apiKeys: ['key1', 'key2'],
                models: ['model1', 'model2'],
                proxy: ['proxy1:8080:user:pass'],
                testTimeout: 20000,
                batchSize: 3,
            });
            expect(helper.apiKeys).toEqual(['key1', 'key2']);
            expect(helper.models).toEqual(['model1', 'model2']);
            expect(helper.proxy).toEqual(['proxy1:8080:user:pass']);
            expect(helper.testTimeout).toBe(20000);
            expect(helper.batchSize).toBe(3);
        });
    });

    describe('_maskKey', () => {
        it('should mask long keys', () => {
            const helper = new FreeOpenRouterHelper();
            expect(helper._maskKey('longkey123')).toBe('longke...y123');
        });

        it('should return null for null key', () => {
            const helper = new FreeOpenRouterHelper();
            expect(helper._maskKey(null)).toBe('null');
        });

        it('should return *** for short keys', () => {
            const helper = new FreeOpenRouterHelper();
            expect(helper._maskKey('short')).toBe('***');
        });
    });

    describe('_getNextApiKey', () => {
        it('should return null for empty apiKeys', () => {
            const helper = new FreeOpenRouterHelper();
            expect(helper._getNextApiKey()).toBeNull();
        });

        it('should return first key and increment index', () => {
            const helper = new FreeOpenRouterHelper({ apiKeys: ['key1', 'key2'] });
            expect(helper._getNextApiKey()).toBe('key1');
            expect(helper.currentKeyIndex).toBe(1);
        });

        it('should rotate through keys', () => {
            const helper = new FreeOpenRouterHelper({ apiKeys: ['key1', 'key2'] });
            expect(helper._getNextApiKey()).toBe('key1');
            expect(helper._getNextApiKey()).toBe('key2');
            expect(helper._getNextApiKey()).toBe('key1');
        });
    });

    describe('_selectProxy', () => {
        it('should return null when proxy is empty', () => {
            const helper = new FreeOpenRouterHelper();
            expect(helper._selectProxy()).toBeNull();
        });

        it('should return null when proxy is null', () => {
            const helper = new FreeOpenRouterHelper({ proxy: null });
            expect(helper._selectProxy()).toBeNull();
        });

        it('should return proxy from list', () => {
            const helper = new FreeOpenRouterHelper({ proxy: ['proxy1:8080', 'proxy2:9090'] });
            const result = helper._selectProxy();
            expect(['proxy1:8080', 'proxy2:9090']).toContain(result);
        });
    });

    describe('_parseProxy', () => {
        it('should parse valid proxy string', () => {
            const helper = new FreeOpenRouterHelper();
            const result = helper._parseProxy('host:port:user:pass');
            expect(result).toEqual({
                host: 'host',
                port: 'port',
                username: 'user',
                password: 'pass',
            });
        });

        it('should return null for invalid format', () => {
            const helper = new FreeOpenRouterHelper();
            expect(helper._parseProxy('invalid')).toBeNull();
        });

        it('should return null for null input', () => {
            const helper = new FreeOpenRouterHelper();
            expect(helper._parseProxy(null)).toBeNull();
        });
    });

    describe('updateConfig', () => {
        it('should update apiKeys', () => {
            const helper = new FreeOpenRouterHelper();
            helper.updateConfig(['newKey'], null);
            expect(helper.apiKeys).toEqual(['newKey']);
        });

        it('should update models and reset results', () => {
            const helper = new FreeOpenRouterHelper();
            helper.results = { working: ['model1'] };
            helper.updateConfig(null, ['newModel']);
            expect(helper.models).toEqual(['newModel']);
            expect(helper.results).toBeNull();
        });
    });

    describe('getResults', () => {
        it('should return null when no results', () => {
            const helper = new FreeOpenRouterHelper();
            expect(helper.getResults()).toBeNull();
        });

        it('should return results when available', () => {
            const helper = new FreeOpenRouterHelper();
            helper.results = { working: ['model1'] };
            expect(helper.getResults()).toEqual({ working: ['model1'] });
        });

        it('should mark stale cache results', () => {
            const helper = new FreeOpenRouterHelper();
            helper.results = { working: ['model1'], testDuration: 100 };
            helper.cacheTimestamp = Date.now() - 400000;
            const result = helper.getResults();
            expect(result.stale).toBe(true);
        });
    });

    describe('isCacheValid', () => {
        it('should return false when no results', () => {
            const helper = new FreeOpenRouterHelper();
            expect(helper.isCacheValid()).toBe(false);
        });

        it('should return true when cache is valid', () => {
            const helper = new FreeOpenRouterHelper();
            helper.results = { working: ['model1'] };
            helper.cacheTimestamp = Date.now();
            expect(helper.isCacheValid()).toBe(true);
        });

        it('should return false when cache is expired', () => {
            const helper = new FreeOpenRouterHelper();
            helper.results = { working: ['model1'] };
            helper.cacheTimestamp = Date.now() - 400000;
            expect(helper.isCacheValid()).toBe(false);
        });
    });

    describe('getCacheAge', () => {
        it('should return null when no timestamp', () => {
            const helper = new FreeOpenRouterHelper();
            expect(helper.getCacheAge()).toBeNull();
        });

        it('should return age in ms', () => {
            const helper = new FreeOpenRouterHelper();
            helper.cacheTimestamp = Date.now() - 5000;
            const age = helper.getCacheAge();
            expect(age).toBeGreaterThanOrEqual(5000);
        });
    });

    describe('isTesting', () => {
        it('should return false initially', () => {
            const helper = new FreeOpenRouterHelper();
            expect(helper.isTesting()).toBe(false);
        });

        it('should return true when testing', () => {
            const helper = new FreeOpenRouterHelper();
            helper.testing = true;
            expect(helper.isTesting()).toBe(true);
        });
    });

    describe('getQuickStatus', () => {
        it('should return idle when not testing and no results', () => {
            const helper = new FreeOpenRouterHelper();
            expect(helper.getQuickStatus()).toEqual({ status: 'idle' });
        });

        it('should return testing status when testing', () => {
            const helper = new FreeOpenRouterHelper();
            helper.testing = true;
            helper.models = ['m1', 'm2'];
            helper.results = { working: ['m1'] };
            const status = helper.getQuickStatus();
            expect(status.status).toBe('testing');
        });

        it('should return done status when results available', () => {
            const helper = new FreeOpenRouterHelper();
            helper.results = {
                working: ['m1'],
                failed: ['m2'],
                total: 2,
                testDuration: 1000,
            };
            const status = helper.getQuickStatus();
            expect(status.status).toBe('done');
            expect(status.working).toBe(1);
            expect(status.failed).toBe(1);
        });
    });

    describe('getOptimizedModelList', () => {
        it('should return empty when no results', () => {
            const helper = new FreeOpenRouterHelper();
            expect(helper.getOptimizedModelList()).toEqual({ primary: null, fallbacks: [] });
        });

        it('should return first working as primary', () => {
            const helper = new FreeOpenRouterHelper();
            helper.results = { working: ['m1', 'm2'] };
            const result = helper.getOptimizedModelList();
            expect(result.primary).toBe('m1');
            expect(result.fallbacks).toEqual(['m2']);
        });

        it('should use specified primary if in working list', () => {
            const helper = new FreeOpenRouterHelper();
            helper.results = { working: ['m1', 'm2'] };
            const result = helper.getOptimizedModelList('m2');
            expect(result.primary).toBe('m2');
            expect(result.fallbacks).toEqual(['m1']);
        });

        it('should use first working if specified primary not in list', () => {
            const helper = new FreeOpenRouterHelper();
            helper.results = { working: ['m1', 'm2'] };
            const result = helper.getOptimizedModelList('m3');
            expect(result.primary).toBe('m1');
        });
    });

    describe('waitForTests', () => {
        it('should return immediately if not testing', async () => {
            const helper = new FreeOpenRouterHelper();
            const result = await helper.waitForTests(100);
            expect(result).toBeNull();
        });

        it('should wait for tests to complete', async () => {
            const helper = new FreeOpenRouterHelper();
            helper.testing = true;
            helper.results = { working: ['m1'] };

            setTimeout(() => {
                helper.testing = false;
            }, 10);

            const result = await helper.waitForTests(100);
            expect(result).toEqual({ working: ['m1'] });
        });

        it('should timeout waiting for tests', async () => {
            const helper = new FreeOpenRouterHelper();
            helper.testing = true;
            helper.results = null;

            const result = await helper.waitForTests(10);
            expect(result).toBeNull();
        });
    });

    describe('testAllModelsInBackground', () => {
        it('should return cached results if valid', async () => {
            const helper = new FreeOpenRouterHelper({
                models: ['model1'],
                apiKeys: ['key1'],
            });
            helper.results = { working: ['model1'], testDuration: 100 };
            helper.cacheTimestamp = Date.now();

            const result = await helper.testAllModelsInBackground();
            expect(result).toEqual({ working: ['model1'], testDuration: 100 });
        });

        it('should return early if no models configured', async () => {
            const helper = new FreeOpenRouterHelper({
                models: [],
                apiKeys: ['key1'],
            });

            const result = await helper.testAllModelsInBackground();
            expect(result.working).toEqual([]);
            expect(result.total).toBe(0);
        });

        it('should return early if no API keys configured', async () => {
            const helper = new FreeOpenRouterHelper({
                models: ['model1'],
                apiKeys: [],
            });

            const result = await helper.testAllModelsInBackground();
            expect(result.working).toEqual([]);
            expect(result.total).toBe(0);
        });

        it('should wait for existing test lock', async () => {
            const helper = new FreeOpenRouterHelper({
                models: ['model1'],
                apiKeys: ['key1'],
            });
            helper.testing = true;
            const pendingPromise = Promise.resolve({ working: ['model1'], total: 1 });
            helper.testLock = pendingPromise;

            const result = await helper.testAllModelsInBackground();
            expect(result.working).toEqual([]);
        });
    });

    describe('_testModel', () => {
        it('should test model successfully', async () => {
            const helper = new FreeOpenRouterHelper({
                testTimeout: 5000,
            });

            global.fetch = vi.fn().mockResolvedValue({
                ok: true,
                json: async () => ({
                    choices: [{ message: { content: 'ok' } }],
                }),
            });

            const result = await helper._testModel('test/model', 'test-key');
            expect(result.success).toBe(true);
            expect(result.duration).toBeGreaterThanOrEqual(0);
        });

        it('should fail for unexpected response', async () => {
            const helper = new FreeOpenRouterHelper({
                testTimeout: 5000,
            });

            global.fetch = vi.fn().mockResolvedValue({
                ok: true,
                json: async () => ({
                    choices: [{ message: { content: 'unexpected' } }],
                }),
            });

            const result = await helper._testModel('test/model', 'test-key');
            expect(result.success).toBe(false);
        });

        it('should fail for HTTP error', async () => {
            const helper = new FreeOpenRouterHelper({
                testTimeout: 5000,
            });

            global.fetch = vi.fn().mockResolvedValue({
                ok: false,
                status: 500,
                text: async () => 'Server error',
            });

            const result = await helper._testModel('test/model', 'test-key');
            expect(result.success).toBe(false);
            expect(result.error).toContain('500');
        });

        it('should fail on network error', async () => {
            const helper = new FreeOpenRouterHelper({
                testTimeout: 5000,
            });

            global.fetch = vi.fn().mockRejectedValue(new Error('Network error'));

            const result = await helper._testModel('test/model', 'test-key');
            expect(result.success).toBe(false);
            expect(result.error).toContain('Network error');
        });

        it('should use proxy when configured', async () => {
            const helper = new FreeOpenRouterHelper({
                testTimeout: 5000,
                proxy: ['proxyhost:8080:user:pass'],
            });

            global.fetch = vi.fn().mockResolvedValue({
                ok: true,
                json: async () => ({
                    choices: [{ message: { content: 'ok' } }],
                }),
            });

            const result = await helper._testModel('test/model', 'test-key');
            expect(result.success).toBe(true);
        });
    });

    describe('Cache handling', () => {
        it('should return fresh cache results', () => {
            const helper = new FreeOpenRouterHelper();
            helper.results = { working: ['m1'], testDuration: 100 };
            helper.cacheTimestamp = Date.now();

            const result = helper.getResults();
            expect(result.stale).toBeUndefined();
        });

        it('should handle undefined results', () => {
            const helper = new FreeOpenRouterHelper();
            helper.results = undefined;
            helper.cacheTimestamp = undefined;

            const result = helper.getResults();
            expect(result).toBeFalsy();
        });
    });

    describe('Coverage Gap Tests', () => {
        beforeEach(() => {
            vi.clearAllMocks();
            global.fetch = vi.fn();
        });

        describe('_selectProxy', () => {
            it('should return null when proxy is empty string', () => {
                const helper = new FreeOpenRouterHelper({ proxy: '' });
                expect(helper._selectProxy()).toBeNull();
            });

            it('should return null when proxy is null', () => {
                const helper = new FreeOpenRouterHelper({ proxy: null });
                expect(helper._selectProxy()).toBeNull();
            });

            it('should return one from proxy list', () => {
                const helper = new FreeOpenRouterHelper({
                    proxy: ['proxy1:8080', 'proxy2:9090', 'proxy3:7070'],
                });
                const result = helper._selectProxy();
                expect(['proxy1:8080', 'proxy2:9090', 'proxy3:7070']).toContain(result);
            });
        });

        describe('_parseProxy', () => {
            it('should return null when not 4 parts', () => {
                const helper = new FreeOpenRouterHelper();
                expect(helper._parseProxy('host:port:user')).toBeNull();
                expect(helper._parseProxy('host:port')).toBeNull();
                expect(helper._parseProxy('host')).toBeNull();
            });

            it('should return host, port, username, password for valid format', () => {
                const helper = new FreeOpenRouterHelper();
                const result = helper._parseProxy('myhost:3128:myuser:mypass');
                expect(result).toEqual({
                    host: 'myhost',
                    port: '3128',
                    username: 'myuser',
                    password: 'mypass',
                });
            });
        });

        describe('updateConfig', () => {
            it('should update only apiKeys when models is null', () => {
                const helper = new FreeOpenRouterHelper({ models: ['model1'] });
                helper.updateConfig(['newKey'], null);
                expect(helper.apiKeys).toEqual(['newKey']);
                expect(helper.models).toEqual(['model1']);
            });

            it('should update only models and reset results when apiKeys is null', () => {
                const helper = new FreeOpenRouterHelper({ apiKeys: ['key1'] });
                helper.results = { working: ['model1'] };
                helper.updateConfig(null, ['newModel']);
                expect(helper.models).toEqual(['newModel']);
                expect(helper.results).toBeNull();
            });

            it('should not update when empty arrays passed', () => {
                const helper = new FreeOpenRouterHelper({
                    apiKeys: ['existingKey'],
                    models: ['existingModel'],
                });
                helper.updateConfig([], []);
                expect(helper.apiKeys).toEqual(['existingKey']);
                expect(helper.models).toEqual(['existingModel']);
            });
        });

        describe('getCacheAge', () => {
            it('should return null when no timestamp', () => {
                const helper = new FreeOpenRouterHelper();
                expect(helper.getCacheAge()).toBeNull();
            });

            it('should return age in ms when timestamp exists', () => {
                const helper = new FreeOpenRouterHelper();
                const now = Date.now();
                helper.cacheTimestamp = now - 3000;
                const age = helper.getCacheAge();
                expect(age).toBeGreaterThanOrEqual(3000);
                expect(age).toBeLessThanOrEqual(now - helper.cacheTimestamp + 10);
            });
        });

        describe('isTesting', () => {
            it('should return true when testing', () => {
                const helper = new FreeOpenRouterHelper();
                helper.testing = true;
                expect(helper.isTesting()).toBe(true);
            });

            it('should return false when not testing', () => {
                const helper = new FreeOpenRouterHelper();
                helper.testing = false;
                expect(helper.isTesting()).toBe(false);
            });
        });

        describe('getQuickStatus', () => {
            it('should return idle when no results', () => {
                const helper = new FreeOpenRouterHelper();
                expect(helper.getQuickStatus()).toEqual({ status: 'idle' });
            });

            it('should return done status when results exist', () => {
                const helper = new FreeOpenRouterHelper();
                helper.results = {
                    working: ['m1'],
                    failed: [],
                    total: 1,
                    testDuration: 500,
                };
                const status = helper.getQuickStatus();
                expect(status.status).toBe('done');
                expect(status.working).toBe(1);
            });

            it('should return testing status when testing', () => {
                const helper = new FreeOpenRouterHelper();
                helper.testing = true;
                helper.models = ['m1', 'm2', 'm3'];
                helper.results = null;
                const status = helper.getQuickStatus();
                expect(status.status).toBe('testing');
            });
        });

        describe('getOptimizedModelList', () => {
            it('should return null primary when no working models', () => {
                const helper = new FreeOpenRouterHelper();
                helper.results = { working: [] };
                const result = helper.getOptimizedModelList();
                expect(result.primary).toBeNull();
                expect(result.fallbacks).toEqual([]);
            });

            it('should use specified primary if in working list', () => {
                const helper = new FreeOpenRouterHelper();
                helper.results = { working: ['model-a', 'model-b', 'model-c'] };
                const result = helper.getOptimizedModelList('model-b');
                expect(result.primary).toBe('model-b');
                expect(result.fallbacks).toEqual(['model-a', 'model-c']);
            });

            it('should use first working if specified primary not in list', () => {
                const helper = new FreeOpenRouterHelper();
                helper.results = { working: ['model-a', 'model-b'] };
                const result = helper.getOptimizedModelList('model-z');
                expect(result.primary).toBe('model-a');
                expect(result.fallbacks).toEqual(['model-b']);
            });
        });

        describe('stale cache', () => {
            it('should getResults return stale: true when cache expired', () => {
                const helper = new FreeOpenRouterHelper();
                helper.results = { working: ['m1'], testDuration: 100 };
                helper.cacheTimestamp = Date.now() - 400001;
                const result = helper.getResults();
                expect(result.stale).toBe(true);
            });
        });

        describe('background testing early exit', () => {
            it('should return early when no models configured', async () => {
                const helper = new FreeOpenRouterHelper({
                    models: [],
                    apiKeys: ['key1'],
                });

                const result = await helper.testAllModelsInBackground();
                expect(result.total).toBe(0);
                expect(result.working).toEqual([]);
            });

            it('should return early when no API keys configured', async () => {
                const helper = new FreeOpenRouterHelper({
                    models: ['model1'],
                    apiKeys: [],
                });

                const result = await helper.testAllModelsInBackground();
                expect(result.total).toBe(0);
                expect(result.working).toEqual([]);
            });
        });

        describe('constructor options', () => {
            it('should accept custom batchSize', () => {
                const helper = new FreeOpenRouterHelper({ batchSize: 10 });
                expect(helper.batchSize).toBe(10);
            });

            it('should accept custom testTimeout', () => {
                const helper = new FreeOpenRouterHelper({ testTimeout: 30000 });
                expect(helper.testTimeout).toBe(30000);
            });
        });
    });

    describe('Additional Coverage Tests - _testModel edge cases', () => {
        beforeEach(() => {
            vi.clearAllMocks();
            global.fetch = vi.fn();
        });

        it('should handle response with OK uppercase', async () => {
            const helper = new FreeOpenRouterHelper({ testTimeout: 5000 });

            global.fetch = vi.fn().mockResolvedValue({
                ok: true,
                json: async () => ({
                    choices: [{ message: { content: 'OK' } }],
                }),
            });

            const result = await helper._testModel('test/model', 'test-key');
            expect(result.success).toBe(true);
        });

        it('should handle response with Ok mixed case', async () => {
            const helper = new FreeOpenRouterHelper({ testTimeout: 5000 });

            global.fetch = vi.fn().mockResolvedValue({
                ok: true,
                json: async () => ({
                    choices: [{ message: { content: 'Ok' } }],
                }),
            });

            const result = await helper._testModel('test/model', 'test-key');
            expect(result.success).toBe(true);
        });

        it('should handle empty choices array', async () => {
            const helper = new FreeOpenRouterHelper({ testTimeout: 5000 });

            global.fetch = vi.fn().mockResolvedValue({
                ok: true,
                json: async () => ({
                    choices: [],
                }),
            });

            const result = await helper._testModel('test/model', 'test-key');
            expect(result.success).toBe(false);
            expect(result.error).toBe('Unexpected response');
        });

        it('should handle missing message in choices', async () => {
            const helper = new FreeOpenRouterHelper({ testTimeout: 5000 });

            global.fetch = vi.fn().mockResolvedValue({
                ok: true,
                json: async () => ({
                    choices: [{}],
                }),
            });

            const result = await helper._testModel('test/model', 'test-key');
            expect(result.success).toBe(false);
            expect(result.error).toBe('Unexpected response');
        });

        it('should handle missing content in message', async () => {
            const helper = new FreeOpenRouterHelper({ testTimeout: 5000 });

            global.fetch = vi.fn().mockResolvedValue({
                ok: true,
                json: async () => ({
                    choices: [{ message: {} }],
                }),
            });

            const result = await helper._testModel('test/model', 'test-key');
            expect(result.success).toBe(false);
            expect(result.error).toBe('Unexpected response');
        });

        it('should handle abort error gracefully', async () => {
            const helper = new FreeOpenRouterHelper({ testTimeout: 5000 });

            const abortError = new Error('Aborted');
            abortError.name = 'AbortError';

            global.fetch = vi.fn().mockRejectedValue(abortError);

            const result = await helper._testModel('test/model', 'test-key');
            expect(result.success).toBe(false);
            expect(result.error).toContain('Aborted');
        });

        it('should handle proxy without username/password', async () => {
            const helper = new FreeOpenRouterHelper({
                testTimeout: 5000,
                proxy: ['proxyhost:8080'],
            });

            global.fetch = vi.fn().mockResolvedValue({
                ok: true,
                json: async () => ({
                    choices: [{ message: { content: 'ok' } }],
                }),
            });

            const result = await helper._testModel('test/model', 'test-key');
            expect(result.success).toBe(true);
        });

        it('should handle HTTP 401 error', async () => {
            const helper = new FreeOpenRouterHelper({ testTimeout: 5000 });

            global.fetch = vi.fn().mockResolvedValue({
                ok: false,
                status: 401,
                text: async () => 'Unauthorized',
            });

            const result = await helper._testModel('test/model', 'test-key');
            expect(result.success).toBe(false);
            expect(result.error).toContain('401');
        });

        it('should handle HTTP 429 rate limit error', async () => {
            const helper = new FreeOpenRouterHelper({ testTimeout: 5000 });

            global.fetch = vi.fn().mockResolvedValue({
                ok: false,
                status: 429,
                text: async () => 'Rate limited',
            });

            const result = await helper._testModel('test/model', 'test-key');
            expect(result.success).toBe(false);
            expect(result.error).toContain('429');
        });

        it('should handle HTTP 403 forbidden error', async () => {
            const helper = new FreeOpenRouterHelper({ testTimeout: 5000 });

            global.fetch = vi.fn().mockResolvedValue({
                ok: false,
                status: 403,
                text: async () => 'Forbidden',
            });

            const result = await helper._testModel('test/model', 'test-key');
            expect(result.success).toBe(false);
            expect(result.error).toContain('403');
        });

        it('should handle non-JSON response gracefully', async () => {
            const helper = new FreeOpenRouterHelper({ testTimeout: 5000 });

            global.fetch = vi.fn().mockResolvedValue({
                ok: true,
                json: async () => {
                    throw new Error('Invalid JSON');
                },
            });

            const result = await helper._testModel('test/model', 'test-key');
            expect(result.success).toBe(false);
            expect(result.error).toContain('Invalid JSON');
        });
    });

    describe('Additional Coverage - testAllModelsInBackground scenarios', () => {
        beforeEach(() => {
            vi.clearAllMocks();
            global.fetch = vi.fn();
        });

        it('should handle waitForTests with testLock but not testing', async () => {
            const helper = new FreeOpenRouterHelper();
            helper.testing = false;
            helper.testLock = Promise.resolve({ working: ['m1'] });
            helper.results = { working: ['m1'] };

            const result = await helper.waitForTests(100);
            expect(result).toEqual({ working: ['m1'] });
        });

        it('should handle waitForTests timeout', async () => {
            const helper = new FreeOpenRouterHelper();
            helper.testing = true;
            helper.testLock = new Promise(() => {});

            const result = await helper.waitForTests(10);
            expect(result).toBeNull();
        });
    });

    describe('Additional Coverage - Batch processing and proxy scenarios', () => {
        beforeEach(() => {
            vi.clearAllMocks();
            global.fetch = vi.fn();
        });

        it('should process batch with delay between batches', async () => {
            const helper = new FreeOpenRouterHelper({
                models: ['model1', 'model2', 'model3', 'model4'],
                apiKeys: ['key1'],
                batchSize: 2,
            });

            global.fetch = vi.fn().mockResolvedValue({
                ok: true,
                json: async () => ({
                    choices: [{ message: { content: 'ok' } }],
                }),
            });

            helper.testAllModelsInBackground();
            await helper.waitForTests(100);

            expect(helper.results.working.length).toBe(4);
        }, 15000);

        it('should rotate API keys correctly across multiple calls', async () => {
            const helper = new FreeOpenRouterHelper({
                models: ['model1', 'model2', 'model3', 'model4'],
                apiKeys: ['key1', 'key2'],
                batchSize: 4,
            });

            global.fetch = vi.fn().mockResolvedValue({
                ok: true,
                json: async () => ({
                    choices: [{ message: { content: 'ok' } }],
                }),
            });

            helper.testAllModelsInBackground();
            await helper.waitForTests(100);

            expect(helper.currentKeyIndex).toBe(4);
        }, 15000);

        it('should handle empty proxy list', async () => {
            const helper = new FreeOpenRouterHelper({
                models: ['model1'],
                apiKeys: ['key1'],
                proxy: [],
            });

            global.fetch = vi.fn().mockResolvedValue({
                ok: true,
                json: async () => ({
                    choices: [{ message: { content: 'ok' } }],
                }),
            });

            const result = await helper._testModel('model1', 'key1');
            expect(result.success).toBe(true);
        });

        it('should handle proxy creation failure gracefully', async () => {
            const { createProxyAgent } = await import('@api/utils/proxy-agent.js');
            createProxyAgent.mockRejectedValueOnce(new Error('Proxy error'));

            const helper = new FreeOpenRouterHelper({
                models: ['model1'],
                apiKeys: ['key1'],
                proxy: ['proxyhost:8080:user:pass'],
            });

            global.fetch = vi.fn().mockResolvedValue({
                ok: true,
                json: async () => ({
                    choices: [{ message: { content: 'ok' } }],
                }),
            });

            const result = await helper._testModel('model1', 'key1');
            expect(result.success).toBe(true);
        });
    });

    describe('Cache validation edge cases', () => {
        it('should return stale results with cacheAge', () => {
            const helper = new FreeOpenRouterHelper();
            helper.results = { working: ['m1'], testDuration: 100 };
            helper.cacheTimestamp = Date.now() - 350000;

            const result = helper.getResults();
            expect(result.stale).toBe(true);
            expect(result.cacheAge).toBeGreaterThan(300000);
        });

        it('should return exact TTL boundary as valid', () => {
            const helper = new FreeOpenRouterHelper();
            helper.results = { working: ['m1'], testDuration: 100 };
            helper.cacheTimestamp = Date.now() - 299999;

            expect(helper.isCacheValid()).toBe(true);
        });

        it('should return expired at exactly TTL', () => {
            const helper = new FreeOpenRouterHelper();
            helper.results = { working: ['m1'], testDuration: 100 };
            helper.cacheTimestamp = Date.now() - 300001;

            expect(helper.isCacheValid()).toBe(false);
        });
    });

    describe('getResults edge cases', () => {
        it('should return stale results without clearing them', () => {
            const helper = new FreeOpenRouterHelper();
            helper.results = { working: ['m1'], testDuration: 100 };
            helper.cacheTimestamp = Date.now() - 400000;

            const result = helper.getResults();
            expect(result.stale).toBe(true);
            expect(helper.results).not.toBeNull();
        });

        it('should return fresh results without stale flag', () => {
            const helper = new FreeOpenRouterHelper();
            helper.results = { working: ['m1'], testDuration: 100 };
            helper.cacheTimestamp = Date.now();

            const result = helper.getResults();
            expect(result.stale).toBeUndefined();
        });
    });

    describe('getQuickStatus edge cases', () => {
        it('should show testing progress correctly', () => {
            const helper = new FreeOpenRouterHelper();
            helper.testing = true;
            helper.models = ['m1', 'm2', 'm3', 'm4', 'm5'];
            helper.results = { working: ['m1', 'm2'] };

            const status = helper.getQuickStatus();
            expect(status.status).toBe('testing');
            expect(status.progress).toBe('2/5');
        });

        it('should show done status with all fields', () => {
            const helper = new FreeOpenRouterHelper();
            helper.results = {
                working: ['m1', 'm2'],
                failed: ['m3'],
                total: 3,
                testDuration: 5000,
            };

            const status = helper.getQuickStatus();
            expect(status.status).toBe('done');
            expect(status.working).toBe(2);
            expect(status.failed).toBe(1);
            expect(status.total).toBe(3);
            expect(status.duration).toBe(5000);
        });
    });

    describe('getOptimizedModelList edge cases', () => {
        it('should handle empty working array', () => {
            const helper = new FreeOpenRouterHelper();
            helper.results = { working: [] };

            const result = helper.getOptimizedModelList();
            expect(result.primary).toBeNull();
            expect(result.fallbacks).toEqual([]);
        });

        it('should handle single working model', () => {
            const helper = new FreeOpenRouterHelper();
            helper.results = { working: ['only-model'] };

            const result = helper.getOptimizedModelList();
            expect(result.primary).toBe('only-model');
            expect(result.fallbacks).toEqual([]);
        });

        it('should filter out primary from fallbacks', () => {
            const helper = new FreeOpenRouterHelper();
            helper.results = { working: ['m1', 'm2', 'm3'] };

            const result = helper.getOptimizedModelList('m1');
            expect(result.primary).toBe('m1');
            expect(result.fallbacks).toEqual(['m2', 'm3']);
        });
    });

    describe('Additional timeout and abort scenarios', () => {
        beforeEach(() => {
            vi.clearAllMocks();
            global.fetch = vi.fn();
        });

        it('should handle abort error gracefully', async () => {
            const helper = new FreeOpenRouterHelper({ testTimeout: 5000 });

            const abortError = new Error('Aborted');
            abortError.name = 'AbortError';

            global.fetch = vi.fn().mockRejectedValue(abortError);

            const result = await helper._testModel('test/model', 'test-key');
            expect(result.success).toBe(false);
            expect(result.error).toContain('Aborted');
        });

        it('should clear timeout on fetch error', async () => {
            vi.useFakeTimers();

            const helper = new FreeOpenRouterHelper({
                testTimeout: 10000,
            });

            global.fetch = vi.fn().mockRejectedValue(new Error('Fetch failed'));

            const result = await helper._testModel('model1', 'key1');
            expect(result.success).toBe(false);
            expect(result.error).toContain('Fetch failed');

            vi.useRealTimers();
        });
    });

    describe('Edge cases for branch coverage', () => {
        beforeEach(() => {
            vi.clearAllMocks();
            global.fetch = vi.fn();
        });

        it('should use proxy with username in URL', async () => {
            const helper = new FreeOpenRouterHelper({
                testTimeout: 5000,
                proxy: ['myproxy:8080:user1:pass1'],
            });

            global.fetch = vi.fn().mockResolvedValue({
                ok: true,
                json: async () => ({
                    choices: [{ message: { content: 'ok' } }],
                }),
            });

            const result = await helper._testModel('model1', 'key1');
            expect(result.success).toBe(true);
        });

        it('should use proxy without username in URL', async () => {
            const helper = new FreeOpenRouterHelper({
                testTimeout: 5000,
                proxy: ['myproxy:8080'],
            });

            global.fetch = vi.fn().mockResolvedValue({
                ok: true,
                json: async () => ({
                    choices: [{ message: { content: 'ok' } }],
                }),
            });

            const result = await helper._testModel('model1', 'key1');
            expect(result.success).toBe(true);
        });

        it('should handle httpAgent being set from proxy', async () => {
            const { createProxyAgent } = await import('@api/utils/proxy-agent.js');

            const mockAgent = { some: 'agent' };
            createProxyAgent.mockResolvedValue({
                getAgent: vi.fn().mockResolvedValue(mockAgent),
            });

            const helper = new FreeOpenRouterHelper({
                testTimeout: 5000,
                proxy: ['myproxy:8080:user:pass'],
            });

            global.fetch = vi.fn().mockResolvedValue({
                ok: true,
                json: async () => ({
                    choices: [{ message: { content: 'ok' } }],
                }),
            });

            const result = await helper._testModel('model1', 'key1');
            expect(result.success).toBe(true);
            expect(createProxyAgent).toHaveBeenCalled();
        });

        it('should log proxy when proxy array is not empty', async () => {
            const helper = new FreeOpenRouterHelper({
                models: ['model1'],
                apiKeys: ['key1'],
                proxy: ['proxy1:8080'],
            });

            global.fetch = vi.fn().mockResolvedValue({
                ok: true,
                json: async () => ({
                    choices: [{ message: { content: 'ok' } }],
                }),
            });

            helper.testAllModelsInBackground();
            await helper.waitForTests(100);
        }, 15000);

        it('should stop testing mid-batch and return early', async () => {
            const helper = new FreeOpenRouterHelper({
                models: ['model1', 'model2', 'model3'],
                apiKeys: ['key1'],
                batchSize: 5,
            });

            let callCount = 0;
            global.fetch = vi.fn().mockImplementation(() => {
                callCount++;
                if (callCount >= 1) {
                    helper.testing = false;
                }
                return Promise.resolve({
                    ok: true,
                    json: async () => ({
                        choices: [{ message: { content: 'ok' } }],
                    }),
                });
            });

            helper.testAllModelsInBackground();
            await helper.waitForTests(100);

            expect(helper.testing).toBe(false);
        }, 15000);

        it('should handle model test failure in batch processing', async () => {
            const helper = new FreeOpenRouterHelper({
                models: ['model1', 'model2'],
                apiKeys: ['key1'],
                batchSize: 2,
            });

            let callCount = 0;
            global.fetch = vi.fn().mockImplementation(() => {
                callCount++;
                if (callCount === 1) {
                    return Promise.resolve({
                        ok: false,
                        status: 500,
                        text: async () => 'Server Error',
                    });
                }
                return Promise.resolve({
                    ok: true,
                    json: async () => ({
                        choices: [{ message: { content: 'ok' } }],
                    }),
                });
            });

            helper.testAllModelsInBackground();
            await helper.waitForTests(100);

            expect(helper.results.failed.length).toBeGreaterThan(0);
            expect(helper.results.working.length).toBe(1);
        }, 15000);

        it('should handle short error message in batch', async () => {
            const helper = new FreeOpenRouterHelper({
                models: ['model1'],
                apiKeys: ['key1'],
                batchSize: 1,
            });

            global.fetch = vi.fn().mockResolvedValue({
                ok: false,
                status: 400,
                text: async () => 'Bad',
            });

            helper.testAllModelsInBackground();
            await helper.waitForTests(100);

            expect(helper.results.failed.length).toBe(1);
            expect(helper.results.failed[0].error).toBe('HTTP 400');
        }, 15000);

        it('should handle long error message in batch (truncated)', async () => {
            const helper = new FreeOpenRouterHelper({
                models: ['model1'],
                apiKeys: ['key1'],
                batchSize: 1,
            });

            const longError = 'This is a very long error message that exceeds thirty characters';
            global.fetch = vi.fn().mockResolvedValue({
                ok: false,
                status: 500,
                text: async () => longError,
            });

            helper.testAllModelsInBackground();
            await helper.waitForTests(100);

            expect(helper.results.failed.length).toBe(1);
            expect(helper.results.failed[0].error.length).toBeLessThanOrEqual(30);
        }, 15000);

        it('should recurse when testing without lock', async () => {
            const helper = new FreeOpenRouterHelper({
                models: ['model1'],
                apiKeys: ['key1'],
            });

            helper.testing = true;
            helper.testLock = null;

            global.fetch = vi.fn().mockResolvedValue({
                ok: true,
                json: async () => ({
                    choices: [{ message: { content: 'ok' } }],
                }),
            });

            helper.testAllModelsInBackground();
            await helper.waitForTests(100);

            expect(helper.testing).toBe(true);
        }, 15000);
    });
});
