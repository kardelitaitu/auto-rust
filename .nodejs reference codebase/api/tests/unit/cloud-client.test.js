/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import CloudClient from '@api/core/cloud-client.js';
import { MultiOpenRouterClient } from '@api/utils/multi-api.js';
import { FreeOpenRouterHelper } from '@api/utils/free-openrouter-helper.js';
import { FreeApiRouter } from '@api/utils/free-api-router.js';
import { createLogger } from '@api/core/logger.js';

// Setup hoisted mocks
const mocks = vi.hoisted(() => {
    return {
        logger: {
            info: vi.fn(),
            warn: vi.fn(),
            error: vi.fn(),
            debug: vi.fn(),
            success: vi.fn(),
        },
        getSettings: vi.fn(),
        settings: {
            llm: {
                cloud: {
                    enabled: true,
                    providers: [{ apiKey: 'test-key', model: 'test-model' }],
                    timeout: 5000,
                },
            },
            open_router_free_api: {
                enabled: false,
            },
        },
    };
});

// Mock dependencies
vi.mock('@api/core/logger.js', () => ({
    createLogger: () => mocks.logger,
    logger: mocks.logger,
}));

vi.mock('@api/utils/configLoader.js', () => ({
    getSettings: mocks.getSettings,
}));

vi.mock('@api/utils/multi-api.js', () => {
    const MockMultiClient = vi.fn();
    MockMultiClient.prototype.processRequest = vi.fn();
    MockMultiClient.prototype.getStats = vi.fn().mockReturnValue({});
    return { MultiOpenRouterClient: MockMultiClient };
});

vi.mock('@api/utils/free-api-router.js', () => {
    const MockFreeRouter = vi.fn();
    MockFreeRouter.prototype.processRequest = vi.fn();
    MockFreeRouter.prototype.isReady = vi.fn().mockReturnValue(false);
    MockFreeRouter.prototype.getStats = vi.fn().mockReturnValue({});
    MockFreeRouter.prototype.getSessionInfo = vi.fn().mockReturnValue({});
    MockFreeRouter.prototype.getModelsInfo = vi
        .fn()
        .mockReturnValue({ testedWorking: [], allConfigured: [] });
    MockFreeRouter.prototype.syncWithHelper = vi.fn();

    return {
        FreeApiRouter: MockFreeRouter,
        setSharedHelper: vi.fn(),
    };
});

vi.mock('@api/utils/free-openrouter-helper.js', () => {
    const MockHelper = vi.fn();
    MockHelper.getInstance = vi.fn();
    MockHelper.prototype.testAllModelsInBackground = vi.fn();
    MockHelper.prototype.getResults = vi.fn().mockReturnValue({ working: [] });
    MockHelper.prototype.updateConfig = vi.fn();
    MockHelper.prototype.isTesting = vi.fn().mockReturnValue(false);
    MockHelper.prototype.getOptimizedModelList = vi
        .fn()
        .mockReturnValue({ primary: 'p', fallbacks: [] });

    // Static mock setup if needed, but getInstance captures it
    return { FreeOpenRouterHelper: MockHelper };
});

describe('CloudClient', () => {
    let client;
    const logger = createLogger();

    beforeEach(async () => {
        vi.clearAllMocks();
        // Setup default mock implementation
        mocks.getSettings.mockResolvedValue(mocks.settings);

        // Reset static state
        CloudClient.sharedHelper = null;

        // Mock global fetch
        global.fetch = vi.fn();

        client = new CloudClient();
        // Wait for async initialization (constructor calls _loadConfig) to settle
        await client.initPromise;
    });

    afterEach(() => {
        vi.clearAllMocks();
        // Reset static helper
        CloudClient.sharedHelper = null;
    });

    describe('Initialization', () => {
        it('should initialize with default config', async () => {
            await client._loadConfig();
            expect(client.apiKey).toBe('test-key');
            expect(client.defaultModel).toBe('test-model');
            expect(client.multiClient).toBeNull();
        });

        it('should initialize multi-client if multiple providers configured', async () => {
            const multiSettings = {
                llm: {
                    cloud: {
                        enabled: true,
                        providers: [
                            { provider: 'openrouter', apiKey: 'k1', model: 'm1' },
                            { provider: 'anthropic', apiKey: 'k2', model: 'm2' },
                        ],
                    },
                },
            };

            // Mock getSettings for this test
            mocks.getSettings.mockResolvedValue(multiSettings);

            // Re-initialize client to load new config
            client = new CloudClient();
            await client.initPromise;

            expect(MultiOpenRouterClient).toHaveBeenCalled();
            expect(client.multiClient).toBeDefined();
        });

        it('should initialize free api router if enabled', async () => {
            const freeSettings = {
                llm: { cloud: { enabled: true } },
                open_router_free_api: {
                    enabled: true,
                    models: { primary: 'p' },
                    api_keys: ['k'],
                },
            };

            mocks.getSettings.mockResolvedValue(freeSettings);

            // Setup helper mock for singleton BEFORE creating client
            const mockHelperInstance = {
                testAllModelsInBackground: vi.fn(),
                getOptimizedModelList: vi.fn().mockReturnValue({ primary: 'p', fallbacks: [] }),
                getResults: vi.fn(),
                isTesting: vi.fn().mockReturnValue(false),
            };
            FreeOpenRouterHelper.getInstance.mockReturnValue(mockHelperInstance);

            // Initialize client (will load config and use helper)
            client = new CloudClient();
            await client.initPromise;

            expect(FreeApiRouter).toHaveBeenCalled();
            expect(client.freeApiRouter).toBeDefined();
        });
    });

    describe('sendRequest', () => {
        it('should use fetch in single-key mode', async () => {
            // Setup success response
            global.fetch.mockResolvedValue({
                ok: true,
                json: async () => ({
                    choices: [{ message: { content: '{"status":"ok"}' } }],
                    usage: { total_tokens: 10 },
                }),
            });

            await client._loadConfig(); // Ensure config loaded

            const request = {
                prompt: 'test prompt',
            };

            const response = await client.sendRequest(request);

            expect(global.fetch).toHaveBeenCalledWith(
                expect.stringContaining('openrouter.ai'),
                expect.objectContaining({
                    method: 'POST',
                    headers: expect.objectContaining({
                        Authorization: 'Bearer test-key',
                    }),
                })
            );

            expect(response.success).toBe(true);
            expect(response.data).toEqual({ status: 'ok' });
            expect(client.stats.successfulRequests).toBe(1);
        });

        it('should handle fetch errors', async () => {
            global.fetch.mockResolvedValue({
                ok: false,
                status: 500,
                text: async () => 'Internal Server Error',
            });

            await client._loadConfig();

            const response = await client.sendRequest({ prompt: 'test' });

            expect(response.success).toBe(false);
            expect(response.error).toContain('500');
            expect(client.stats.failedRequests).toBe(1);
        });

        it('should use multi-client if active', async () => {
            // Setup multi-provider settings to ensure _loadConfig creates/uses multiClient
            const multiSettings = {
                llm: {
                    cloud: {
                        enabled: true,
                        providers: [
                            { provider: 'openrouter', apiKey: 'k1', model: 'm1' },
                            { provider: 'anthropic', apiKey: 'k2', model: 'm2' },
                        ],
                    },
                },
            };

            mocks.getSettings.mockResolvedValue(multiSettings);

            // Re-initialize client to load new config
            client = new CloudClient();
            await client.initPromise;

            // Now mock the processRequest on the created multiClient instance
            // Since client.multiClient is a new instance created by _loadConfig, we need to grab it
            const mockMultiClientInstance = client.multiClient;
            expect(mockMultiClientInstance).toBeDefined();

            mockMultiClientInstance.processRequest.mockResolvedValue({
                success: true,
                content: 'response',
                tokens: { total: 10 },
                keyUsed: 0,
            });

            const response = await client.sendRequest({ prompt: 'test' });

            expect(mockMultiClientInstance.processRequest).toHaveBeenCalled();
            expect(global.fetch).not.toHaveBeenCalled();
            expect(response.success).toBe(true);
        });

        it('should use free router if ready', async () => {
            // Setup free router
            client.freeApiRouter = new FreeApiRouter({});
            client.freeApiRouter.isReady.mockReturnValue(true);
            client.freeApiRouter.processRequest.mockResolvedValue({
                success: true,
                content: 'free response',
                model: 'free-model',
            });

            const response = await client.sendRequest({ prompt: 'test' });

            expect(client.freeApiRouter.processRequest).toHaveBeenCalled();
            expect(response.success).toBe(true);
            expect(response.metadata.mode).toBe('free-api-router');
        });
    });

    describe('_buildPrompt', () => {
        it('should handle direct prompt property', () => {
            const prompt = client._buildPrompt({ prompt: 'direct' });
            expect(prompt).toBe('direct');
        });

        it('should handle payload.prompt', () => {
            const prompt = client._buildPrompt({ payload: { prompt: 'payload' } });
            expect(prompt).toBe('payload');
        });

        it('should handle system + user prompt', () => {
            const prompt = client._buildPrompt({
                payload: {
                    systemPrompt: 'system',
                    userPrompt: 'user',
                },
            });
            expect(prompt).toBe('system\n\nuser');
        });

        it('should append context', () => {
            const prompt = client._buildPrompt({
                prompt: 'base',
                context: {
                    breadcrumbs: 'bread',
                    state: { foo: 'bar' },
                },
            });

            expect(prompt).toContain('Context - Recent Actions:\nbread');
            expect(prompt).toContain('Context - Current State:\n{');
            expect(prompt).toContain('base');
        });

        it('should handle context with only breadcrumbs', () => {
            const prompt = client._buildPrompt({
                prompt: 'base',
                context: {
                    breadcrumbs: 'bread',
                },
            });
            expect(prompt).toContain('Context - Recent Actions:\nbread');
            expect(prompt).not.toContain('Context - Current State:');
        });

        it('should handle context with only state', () => {
            const prompt = client._buildPrompt({
                prompt: 'base',
                context: {
                    state: { foo: 'bar' },
                },
            });
            expect(prompt).not.toContain('Context - Recent Actions:');
            expect(prompt).toContain('Context - Current State:\n{');
        });

        it('should handle empty request gracefully', () => {
            const prompt = client._buildPrompt({});
            expect(prompt).toBe('');
        });
    });

    describe('testConnection', () => {
        it('should return true on success', async () => {
            vi.spyOn(client, 'sendRequest').mockResolvedValue({ success: true });

            const result = await client.testConnection();

            expect(result).toBe(true);
            expect(mocks.logger.success).toHaveBeenCalled();
        });

        it('should return false on failure', async () => {
            vi.spyOn(client, 'sendRequest').mockResolvedValue({ success: false, error: 'fail' });

            const result = await client.testConnection();

            expect(result).toBe(false);
            expect(mocks.logger.error).toHaveBeenCalled();
        });
    });

    describe('Statistics and Accessors', () => {
        it('should return basic stats in single-key mode', async () => {
            const stats = client.getStats();
            expect(stats.mode).toBe('single-key');
            expect(stats.totalRequests).toBe(0);
            expect(stats.successRate).toBe('0%');
        });

        it('should return multi-client stats if active', () => {
            client.multiClient = {
                getStats: vi.fn().mockReturnValue({ total: 1 }),
            };
            const stats = client.getStats();
            expect(stats.mode).toBe('multi-key-fallback');
            expect(stats.multiClientStats).toEqual({ total: 1 });
        });

        it('should return free-router stats if ready', () => {
            client.freeApiRouter = {
                isReady: () => true,
                getStats: vi.fn().mockReturnValue({ total: 5 }),
                getSessionInfo: vi.fn().mockReturnValue({ model: 'test' }),
            };
            const stats = client.getStats();
            expect(stats.mode).toBe('free-api-router');
            expect(stats.freeApiRouterStats).toEqual({ total: 5 });
        });

        it('should return correct success rate', () => {
            client.stats.totalRequests = 10;
            client.stats.successfulRequests = 5;
            client.stats.totalDuration = 1000;

            const stats = client.getStats();
            expect(stats.successRate).toBe('50.00%');
            expect(stats.avgDuration).toBe(100);
        });

        it('should expose multi-client instance', () => {
            client.multiClient = { foo: 'bar' };
            expect(client.getMultiClient()).toEqual({ foo: 'bar' });
        });

        it('should reset stats', () => {
            client.stats.totalRequests = 10;
            client.resetStats();
            expect(client.stats.totalRequests).toBe(0);
        });
    });

    describe('Error Handling', () => {
        it('should handle request timeout (AbortError)', async () => {
            await client._loadConfig();

            // Mock fetch to reject with AbortError
            const abortError = new Error('Aborted');
            abortError.name = 'AbortError';
            global.fetch.mockRejectedValue(abortError);

            const response = await client.sendRequest({ prompt: 'test' });

            expect(response.success).toBe(false);
            expect(response.error).toContain('Request timeout');
        });

        it('should handle unexpected fetch errors', async () => {
            await client._loadConfig();

            global.fetch.mockRejectedValue(new Error('Network Error'));

            const response = await client.sendRequest({ prompt: 'test' });

            expect(response.success).toBe(false);
            expect(response.error).toBe('Network Error');
        });

        it('should throw error if _makeRequest called while multi-client active', async () => {
            client.multiClient = {};
            await expect(client._makeRequest({})).rejects.toThrow('multi-client is active');
        });
    });

    describe('Free Router Error Handling', () => {
        beforeEach(() => {
            // Setup client with free router stub
            client.freeApiRouter = {
                isReady: () => true,
                syncWithHelper: vi.fn(),
                getModelsInfo: vi.fn().mockReturnValue({ testedWorking: [], allConfigured: [] }),
                processRequest: vi.fn(),
            };
            // Prevent _loadConfig from polluting logs or resetting state
            client._loadConfig = vi.fn().mockResolvedValue();
        });

        it('should handle failed response from free router', async () => {
            client.freeApiRouter.processRequest.mockResolvedValue({
                success: false,
                error: 'Free tier exhausted',
                modelsTried: ['m1', 'm2'],
            });

            const response = await client.sendRequest({ prompt: 'test' });

            expect(response.success).toBe(false);
            expect(response.error).toBe('Free tier exhausted');
            expect(response.metadata.modelsTried).toEqual(['m1', 'm2']);
        });

        it('should handle exception from free router', async () => {
            client.freeApiRouter.processRequest.mockRejectedValue(new Error('Router fatal error'));

            const response = await client.sendRequest({ prompt: 'test' });

            expect(response.success).toBe(false);
            expect(response.error).toBe('Router fatal error');
        });

        it('should handle invalid JSON from free router', async () => {
            client.freeApiRouter.processRequest.mockResolvedValue({
                success: true,
                content: '{ invalid json',
                model: 'test-model',
            });

            const response = await client.sendRequest({ prompt: 'test' });

            expect(response.success).toBe(true);
            expect(response.data).toBeNull();
            expect(mocks.logger.debug).toHaveBeenCalledWith(
                expect.stringContaining('not valid JSON')
            );
        });

        it('should handle valid JSON from free router', async () => {
            client.freeApiRouter.processRequest.mockResolvedValue({
                success: true,
                content: '{"answer":42}',
                model: 'test-model',
            });

            const response = await client.sendRequest({ prompt: 'test' });

            expect(response.success).toBe(true);
            expect(response.data).toEqual({ answer: 42 });
        });
    });

    describe('Coverage Edge Cases', () => {
        it('should handle multi-client failure', async () => {
            // Mock multi-client to fail
            const multiClient = {
                processRequest: vi.fn().mockResolvedValue({
                    success: false,
                    error: 'All keys failed',
                    keysTried: 3,
                }),
                getStats: vi.fn(),
            };
            client.multiClient = multiClient;

            // Prevent _loadConfig
            client._loadConfig = vi.fn().mockResolvedValue();

            const response = await client.sendRequest({ prompt: 'test' });
            expect(response.success).toBe(false);
            expect(response.error).toBe('All keys failed');
        });

        it('should handle multi-client exception', async () => {
            const multiClient = {
                processRequest: vi.fn().mockRejectedValue(new Error('Fatal multi error')),
                getStats: vi.fn(),
            };
            client.multiClient = multiClient;
            client._loadConfig = vi.fn().mockResolvedValue();

            const response = await client.sendRequest({ prompt: 'test' });
            expect(response.success).toBe(false);
            expect(response.error).toBe('Fatal multi error');
        });

        it('should log tested working models info in free router', async () => {
            client.freeApiRouter = {
                isReady: () => true,
                syncWithHelper: vi.fn(),
                getModelsInfo: vi.fn().mockReturnValue({
                    testedWorking: ['m1'],
                    allConfigured: ['m1', 'm2'],
                }),
                processRequest: vi.fn().mockResolvedValue({ success: true, content: '{}' }),
            };
            client._loadConfig = vi.fn().mockResolvedValue();

            await client.sendRequest({ prompt: 'test' });
            expect(mocks.logger.info).toHaveBeenCalledWith(
                expect.stringContaining('Using 1/2 tested working models')
            );
        });

        it('should execute testFreeModels successfully', async () => {
            // Mock settings to enable free api
            const settings = {
                open_router_free_api: {
                    enabled: true,
                    models: { primary: 'p', fallbacks: ['f'] },
                    api_keys: ['k1'],
                },
            };
            // We need to mock getSettings for this specific test
            // Since it's hoisted, we can't easily change it for one test unless we mocked it to return a variable
            // But we can rely on `getSettings` mock being a vi.fn() so we can override implementation
            const { getSettings } = await import('@api/utils/configLoader.js');
            getSettings.mockResolvedValue(settings);

            // Also mock FreeOpenRouterHelper
            CloudClient.sharedHelper = null; // Ensure new instance or mock logic

            // Mock helper instance
            const mockHelper = {
                testAllModelsInBackground: vi
                    .fn()
                    .mockResolvedValue({ testedWorking: ['p'], failed: [] }),
                isTesting: () => false,
                isReady: () => false,
                getResults: vi.fn().mockReturnValue({ testedWorking: [], failed: [] }),
                getModelsInfo: vi.fn().mockReturnValue({ testedWorking: [], allConfigured: [] }),
                updateConfig: vi.fn(),
                isCacheValid: () => true, // Ensure we don't trigger restart logic that might fail
            };
            // Set shared helper directly to avoid race conditions with _loadConfig
            CloudClient.sharedHelper = mockHelper;

            console.log('Starting testFreeModels');
            const result = await client.testFreeModels();
            if (!result.tested) console.log('FAILURE REASON:', result);
            expect(result.tested).toBe(true);
        });

        it('should handle testFreeModels when disabled', async () => {
            const settings = { open_router_free_api: { enabled: false } };
            const { getSettings } = await import('@api/utils/configLoader.js');
            getSettings.mockResolvedValueOnce(settings);

            const result = await client.testFreeModels();
            expect(result.tested).toBe(false);
            expect(result.reason).toBe('not_enabled');
        });

        it('should handle testFreeModels with no models', async () => {
            const settings = {
                open_router_free_api: {
                    enabled: true,
                    models: { primary: null, fallbacks: [] },
                },
            };
            const { getSettings } = await import('@api/utils/configLoader.js');
            getSettings.mockResolvedValueOnce(settings);

            const result = await client.testFreeModels();
            expect(result.tested).toBe(false);
            expect(result.reason).toBe('no_models');
        });

        it('should handle testFreeModels error', async () => {
            const { getSettings } = await import('@api/utils/configLoader.js');
            getSettings.mockRejectedValueOnce(new Error('Config load failed'));

            const result = await client.testFreeModels();
            expect(result.tested).toBe(false);
            expect(result.error).toBe('Config load failed');
        });

        it('should handle key fallbacks correctly in multi-client', async () => {
            // Spy on prototype to ensure we catch the call
            const spy = vi
                .spyOn(CloudClient.prototype, '_loadConfig')
                .mockImplementation(async () => {});

            // Re-instantiate client to ensure clean state (optional but safe)
            client = new CloudClient();
            client.multiClient = {
                processRequest: vi.fn().mockResolvedValue({
                    success: true,
                    content: '{}',
                    keyUsed: 1, // Indicates fallback happened
                    tokens: { total: 10 },
                }),
                getStats: vi.fn(),
            };

            await client.sendRequest({ prompt: 'test' });

            expect(client.stats.keyFallbacks).toBe(1);
            expect(mocks.logger.info).toHaveBeenCalledWith(expect.stringContaining('fallback(s)'));

            spy.mockRestore();
        });

        it('should handle invalid JSON in multi-client response', async () => {
            // Spy on prototype
            const spy = vi
                .spyOn(CloudClient.prototype, '_loadConfig')
                .mockImplementation(async () => {});

            client = new CloudClient();
            client.multiClient = {
                processRequest: vi.fn().mockResolvedValue({
                    success: true,
                    content: '{ invalid json',
                    keyUsed: 0,
                }),
                getStats: vi.fn(),
            };

            const result = await client.sendRequest({ prompt: 'test' });

            expect(result.success).toBe(true);
            expect(result.data).toBeNull();
            expect(mocks.logger.debug).toHaveBeenCalledWith(
                expect.stringContaining('not valid JSON')
            );

            spy.mockRestore();
        });

        it('should return error if no API key configured', async () => {
            // Spy on prototype
            const spy = vi
                .spyOn(CloudClient.prototype, '_loadConfig')
                .mockImplementation(async () => {});

            client = new CloudClient();
            client.apiKey = '';
            client.multiClient = null;

            const result = await client.sendRequest({ prompt: 'test' });

            expect(result.success).toBe(false);
            expect(result.error).toContain('OpenRouter API key not configured');
            expect(mocks.logger.error).toHaveBeenCalledWith(
                expect.stringContaining('not configured')
            );

            spy.mockRestore();
        });

        it('should handle invalid JSON in single-key mode', async () => {
            // Spy on prototype
            const spy = vi
                .spyOn(CloudClient.prototype, '_loadConfig')
                .mockImplementation(async () => {});

            client = new CloudClient();
            client.apiKey = 'test-key';
            client.multiClient = null;

            // Mock _makeRequest to return invalid JSON
            client._makeRequest = vi.fn().mockResolvedValue({
                choices: [
                    {
                        message: { content: '{ invalid json' },
                    },
                ],
                usage: { total_tokens: 10 },
            });

            const result = await client.sendRequest({ prompt: 'test' });

            expect(result.success).toBe(true);
            expect(result.data).toBeNull();
            expect(mocks.logger.debug).toHaveBeenCalledWith(
                expect.stringContaining('not valid JSON')
            );

            spy.mockRestore();
        });

        it('should handle no providers configured in _loadConfig', async () => {
            mocks.getSettings.mockResolvedValue({
                llm: {
                    cloud: {
                        enabled: true, // Ensure enabled is true so it checks for providers
                        providers: [], // Empty providers
                    },
                },
                open_router_free_api: { enabled: false },
            });

            client = new CloudClient();
            // Wait for constructor _loadConfig
            await client.initPromise;

            expect(client.apiKey).toBe('');
            expect(mocks.logger.warn).toHaveBeenCalledWith(
                expect.stringContaining('No cloud providers configured')
            );
        });

        it('should create new shared helper if not exists in testFreeModels', async () => {
            CloudClient.sharedHelper = null;

            // Mock settings to enable free api
            const configLoader = await import('@api/utils/configLoader.js');
            configLoader.getSettings.mockResolvedValue({
                open_router_free_api: {
                    enabled: true,
                    models: { primary: 'test-model' },
                    api_keys: ['key1'],
                },
            });

            await client.testFreeModels();

            expect(mocks.logger.info).toHaveBeenCalledWith(
                expect.stringContaining('Creating new FreeOpenRouterHelper singleton')
            );
        });

        it('should restart tests if cache is stale in testFreeModels', async () => {
            // Setup existing helper with stale cache
            CloudClient.sharedHelper = {
                updateConfig: vi.fn(),
                isCacheValid: vi.fn().mockReturnValue(false),
                testAllModelsInBackground: vi.fn(),
                isTesting: vi.fn().mockReturnValue(false),
                getResults: vi.fn().mockReturnValue({ tested: true }),
            };

            // Mock settings
            const configLoader = await import('@api/utils/configLoader.js');
            configLoader.getSettings.mockResolvedValue({
                open_router_free_api: {
                    enabled: true,
                    models: { primary: 'test-model' },
                },
            });

            await client.testFreeModels();

            expect(mocks.logger.info).toHaveBeenCalledWith(expect.stringContaining('Cache stale'));
            expect(CloudClient.sharedHelper.testAllModelsInBackground).toHaveBeenCalled();
        });

        it('should reuse shared helper if already exists in _loadConfig', async () => {
            // 1. Reuse cache case
            CloudClient.sharedHelper = {
                getResults: vi.fn().mockReturnValue({ testDuration: 100, working: [], total: 1 }),
                isTesting: vi.fn().mockReturnValue(false),
                updateConfig: vi.fn(),
                testAllModelsInBackground: vi.fn(),
                getOptimizedModelList: vi.fn().mockReturnValue({ primary: 'p', fallbacks: [] }),
            };

            const configLoader = await import('@api/utils/configLoader.js');
            configLoader.getSettings.mockResolvedValue({
                open_router_free_api: {
                    enabled: true,
                    models: { primary: 'test-model' },
                },
            });

            client = new CloudClient();
            await new Promise((resolve) => setTimeout(resolve, 10));

            expect(mocks.logger.info).toHaveBeenCalledWith(expect.stringContaining('Reuse cache'));

            // 2. Tests already running case
            vi.clearAllMocks();
            CloudClient.sharedHelper = {
                getResults: vi.fn().mockReturnValue({ testDuration: 0 }),
                isTesting: vi.fn().mockReturnValue(true),
                updateConfig: vi.fn(),
                testAllModelsInBackground: vi.fn(),
                getOptimizedModelList: vi.fn().mockReturnValue({ primary: 'p', fallbacks: [] }),
            };

            client = new CloudClient();
            await new Promise((resolve) => setTimeout(resolve, 10));

            expect(mocks.logger.info).toHaveBeenCalledWith(
                expect.stringContaining('Tests already running')
            );

            // 3. Update config case
            vi.clearAllMocks();
            CloudClient.sharedHelper = {
                getResults: vi.fn().mockReturnValue({ testDuration: 0 }),
                isTesting: vi.fn().mockReturnValue(false),
                updateConfig: vi.fn(),
                testAllModelsInBackground: vi.fn(),
                getOptimizedModelList: vi.fn().mockReturnValue({ primary: 'p', fallbacks: [] }),
            };

            client = new CloudClient();
            await new Promise((resolve) => setTimeout(resolve, 10));

            expect(CloudClient.sharedHelper.updateConfig).toHaveBeenCalled();
            expect(CloudClient.sharedHelper.testAllModelsInBackground).toHaveBeenCalled();
        });

        it('should handle errors in _loadConfig', async () => {
            const { getSettings } = await import('@api/utils/configLoader.js');
            vi.mocked(getSettings).mockRejectedValueOnce(new Error('Config error'));

            // Re-instantiate to trigger _loadConfig
            const CloudClient = (await import('../../core/cloud-client.js')).default;
            client = new CloudClient();

            // Allow async init to complete
            await new Promise((resolve) => setTimeout(resolve, 10));

            // Should still function with defaults
            expect(client.apiKey).toBeDefined();
        });

        it('should handle missing content in free router response', async () => {
            // Mock settings to enable free router
            const { getSettings } = await import('@api/utils/configLoader.js');
            vi.mocked(getSettings).mockResolvedValue({
                llm: {
                    cloud: {
                        enabled: true,
                        provider: 'openrouter',
                        free: { enabled: true },
                    },
                },
            });

            // Re-instantiate
            const CloudClient = (await import('../../core/cloud-client.js')).default;
            client = new CloudClient();
            await new Promise((resolve) => setTimeout(resolve, 10));

            // Mock free router success but empty content
            const { FreeApiRouter: _FreeApiRouter } =
                await import('@api/utils/free-api-router.js');
            const mockRouter = {
                processRequest: vi
                    .fn()
                    .mockResolvedValue({ success: true, content: null, model: 'test' }),
                getStats: vi.fn().mockReturnValue({}),
                isReady: vi.fn().mockReturnValue(true),
                getModelsInfo: vi.fn().mockReturnValue({ testedWorking: [], allConfigured: [] }),
            };
            // We can't easily mock the constructor return value of FreeApiRouter since it's destructured
            // But we can manually inject it into the client instance
            client.freeApiRouter = mockRouter;
            client.useFreeRouter = true;
            client.multiClient = null; // Ensure multi-client is off
            client.config = { enabled: true }; // Prevent auto-reload

            const result = await client.sendRequest({ payload: { prompt: 'test' } });
            expect(result.success).toBe(true);
            expect(result.content).toBe('');
        });

        it('should handle root prompt property (Format 3)', async () => {
            // Mock settings for single key mode
            const { getSettings } = await import('@api/utils/configLoader.js');
            vi.mocked(getSettings).mockResolvedValue({
                llm: {
                    cloud: {
                        enabled: true,
                        provider: 'openrouter',
                        // Provide valid providers array for _loadConfig
                        providers: [{ apiKey: 'test-key', model: 'test-model' }],
                    },
                },
            });

            const CloudClient = (await import('../../core/cloud-client.js')).default;
            client = new CloudClient();
            await new Promise((resolve) => setTimeout(resolve, 10));

            // Ensure single key mode properties are set
            client.multiClient = null;
            client.useFreeRouter = false;
            // client.apiKey should be set by _loadConfig from providers[0].apiKey

            // Spy on fetch
            global.fetch = vi.fn().mockResolvedValue({
                ok: true,
                json: async () => ({ choices: [{ message: { content: 'response' } }] }),
            });

            const result = await client.sendRequest({ prompt: 'root prompt' });
            expect(result.success).toBe(true);

            expect(global.fetch).toHaveBeenCalledWith(
                expect.any(String),
                expect.objectContaining({
                    body: expect.stringContaining('root prompt'),
                })
            );
        });

        it('should handle system and user prompt in payload (Format 1)', async () => {
            // Mock settings
            const { getSettings } = await import('@api/utils/configLoader.js');
            vi.mocked(getSettings).mockResolvedValue({
                llm: {
                    cloud: {
                        enabled: true,
                        provider: 'openrouter',
                        providers: [{ apiKey: 'test-key', model: 'test-model' }],
                    },
                },
            });

            const CloudClient = (await import('../../core/cloud-client.js')).default;
            client = new CloudClient();
            await new Promise((resolve) => setTimeout(resolve, 10));
            client.multiClient = null;

            global.fetch = vi.fn().mockResolvedValue({
                ok: true,
                json: async () => ({ choices: [{ message: { content: 'response' } }] }),
            });

            await client.sendRequest({
                payload: {
                    systemPrompt: 'system instruction',
                    userPrompt: 'user query',
                },
            });

            expect(global.fetch).toHaveBeenCalledWith(
                expect.any(String),
                expect.objectContaining({
                    body: expect.stringContaining('system instruction\\n\\nuser query'),
                })
            );
        });

        it('should handle single prompt in payload (Format 2)', async () => {
            // Mock settings
            const { getSettings } = await import('@api/utils/configLoader.js');
            vi.mocked(getSettings).mockResolvedValue({
                llm: {
                    cloud: {
                        enabled: true,
                        provider: 'openrouter',
                        providers: [{ apiKey: 'test-key', model: 'test-model' }],
                    },
                },
            });

            const CloudClient = (await import('../../core/cloud-client.js')).default;
            client = new CloudClient();
            await new Promise((resolve) => setTimeout(resolve, 10));
            client.multiClient = null;

            global.fetch = vi.fn().mockResolvedValue({
                ok: true,
                json: async () => ({ choices: [{ message: { content: 'response' } }] }),
            });

            await client.sendRequest({
                payload: {
                    prompt: 'single prompt',
                },
            });

            expect(global.fetch).toHaveBeenCalledWith(
                expect.any(String),
                expect.objectContaining({
                    body: expect.stringContaining('single prompt'),
                })
            );
        });

        it('should handle direct prompt property (Format 3)', async () => {
            // Mock settings
            const { getSettings } = await import('@api/utils/configLoader.js');
            vi.mocked(getSettings).mockResolvedValue({
                llm: {
                    cloud: {
                        enabled: true,
                        provider: 'openrouter',
                        providers: [{ apiKey: 'test-key', model: 'test-model' }],
                    },
                },
            });

            const CloudClient = (await import('../../core/cloud-client.js')).default;
            client = new CloudClient();
            await new Promise((resolve) => setTimeout(resolve, 10));
            client.multiClient = null;

            global.fetch = vi.fn().mockResolvedValue({
                ok: true,
                json: async () => ({ choices: [{ message: { content: 'response' } }] }),
            });

            await client.sendRequest({
                prompt: 'direct prompt',
            });

            expect(global.fetch).toHaveBeenCalledWith(
                expect.any(String),
                expect.objectContaining({
                    body: expect.stringContaining('direct prompt'),
                })
            );
        });

        it('should inject context breadcrumbs', async () => {
            // Mock settings
            const { getSettings } = await import('@api/utils/configLoader.js');
            vi.mocked(getSettings).mockResolvedValue({
                llm: {
                    cloud: {
                        enabled: true,
                        provider: 'openrouter',
                        providers: [{ apiKey: 'test-key', model: 'test-model' }],
                    },
                },
            });

            const CloudClient = (await import('../../core/cloud-client.js')).default;
            client = new CloudClient();
            await new Promise((resolve) => setTimeout(resolve, 10));
            client.multiClient = null;

            global.fetch = vi.fn().mockResolvedValue({
                ok: true,
                json: async () => ({ choices: [{ message: { content: 'response' } }] }),
            });

            await client.sendRequest({
                prompt: 'main prompt',
                context: {
                    breadcrumbs: 'user clicked button',
                },
            });

            expect(global.fetch).toHaveBeenCalledWith(
                expect.any(String),
                expect.objectContaining({
                    body: expect.stringContaining(
                        'Context - Recent Actions:\\nuser clicked button\\n\\nmain prompt'
                    ),
                })
            );
        });

        it('should inject context state', async () => {
            // Mock settings
            const { getSettings } = await import('@api/utils/configLoader.js');
            vi.mocked(getSettings).mockResolvedValue({
                llm: {
                    cloud: {
                        enabled: true,
                        provider: 'openrouter',
                        providers: [{ apiKey: 'test-key', model: 'test-model' }],
                    },
                },
            });

            const CloudClient = (await import('../../core/cloud-client.js')).default;
            client = new CloudClient();
            await new Promise((resolve) => setTimeout(resolve, 10));
            client.multiClient = null;

            global.fetch = vi.fn().mockResolvedValue({
                ok: true,
                json: async () => ({ choices: [{ message: { content: 'response' } }] }),
            });

            const state = { url: 'http://example.com' };
            await client.sendRequest({
                prompt: 'main prompt',
                context: {
                    state,
                },
            });

            expect(global.fetch).toHaveBeenCalledWith(
                expect.any(String),
                expect.objectContaining({
                    body: expect.stringContaining('Context - Current State:'),
                })
            );
            // Check for JSON stringified state
            // Note: JSON.stringify adds quotes and formatting, so strict matching might be tricky depending on whitespace
            // But we can check for keys
            expect(global.fetch).toHaveBeenCalledWith(
                expect.any(String),
                expect.objectContaining({
                    body: expect.stringContaining('http://example.com'),
                })
            );
        });

        it('should handle empty prompt gracefully', async () => {
            // Mock settings
            const { getSettings } = await import('@api/utils/configLoader.js');
            vi.mocked(getSettings).mockResolvedValue({
                llm: {
                    cloud: {
                        enabled: true,
                        provider: 'openrouter',
                        providers: [{ apiKey: 'test-key', model: 'test-model' }],
                    },
                },
            });

            const CloudClient = (await import('../../core/cloud-client.js')).default;
            client = new CloudClient();
            await new Promise((resolve) => setTimeout(resolve, 10));
            client.multiClient = null;

            global.fetch = vi.fn().mockResolvedValue({
                ok: true,
                json: async () => ({ choices: [{ message: { content: 'response' } }] }),
            });

            await client.sendRequest({
                payload: {},
            });

            // Should send request with empty content or handled gracefully
            // The code sends whatever _buildPrompt returns. If empty string, it sends empty string.
            expect(global.fetch).toHaveBeenCalledWith(
                expect.any(String),
                expect.objectContaining({
                    body: expect.stringContaining('"content":""'), // prompt becomes content in messages
                })
            );
        });

        it('should handle context breadcrumbs and state', async () => {
            // Mock settings for single key mode
            const { getSettings } = await import('@api/utils/configLoader.js');
            vi.mocked(getSettings).mockResolvedValue({
                llm: {
                    cloud: {
                        enabled: true,
                        provider: 'openrouter',
                        providers: [{ apiKey: 'test-key', model: 'test-model' }],
                    },
                },
            });

            const CloudClient = (await import('../../core/cloud-client.js')).default;
            client = new CloudClient();
            await new Promise((resolve) => setTimeout(resolve, 10));

            // Ensure single key mode properties are set
            client.multiClient = null;
            client.useFreeRouter = false;

            // Spy on fetch
            global.fetch = vi.fn().mockResolvedValue({
                ok: true,
                json: async () => ({ choices: [{ message: { content: 'response' } }] }),
            });

            const result = await client.sendRequest({
                payload: { prompt: 'test' },
                context: {
                    breadcrumbs: 'clicked button',
                    state: { loggedIn: true },
                },
            });
            expect(result.success).toBe(true);

            expect(global.fetch).toHaveBeenCalledWith(
                expect.any(String),
                expect.objectContaining({
                    body: expect.stringContaining('Context - Recent Actions:\\nclicked button'),
                })
            );
            expect(global.fetch).toHaveBeenCalledWith(
                expect.any(String),
                expect.objectContaining({
                    body: expect.stringContaining('Context - Current State:\\n{'),
                })
            );
        });

        it('should handle context with only breadcrumbs', async () => {
            // Mock settings for single key mode
            const { getSettings } = await import('@api/utils/configLoader.js');
            vi.mocked(getSettings).mockResolvedValue({
                llm: {
                    cloud: {
                        enabled: true,
                        provider: 'openrouter',
                        providers: [{ apiKey: 'test-key', model: 'test-model' }],
                    },
                },
            });

            const CloudClient = (await import('../../core/cloud-client.js')).default;
            client = new CloudClient();
            await new Promise((resolve) => setTimeout(resolve, 10));
            client.multiClient = null;

            global.fetch = vi.fn().mockResolvedValue({
                ok: true,
                json: async () => ({ choices: [{ message: { content: 'response' } }] }),
            });

            await client.sendRequest({
                payload: { prompt: 'test' },
                context: { breadcrumbs: 'clicked button' },
            });

            expect(global.fetch).toHaveBeenCalledWith(
                expect.any(String),
                expect.objectContaining({
                    body: expect.stringContaining('Context - Recent Actions:\\nclicked button'),
                })
            );
            expect(global.fetch).toHaveBeenCalledWith(
                expect.any(String),
                expect.not.objectContaining({
                    body: expect.stringContaining('Context - Current State'),
                })
            );
        });

        it('should handle context with only state', async () => {
            // Mock settings for single key mode
            const { getSettings } = await import('@api/utils/configLoader.js');
            vi.mocked(getSettings).mockResolvedValue({
                llm: {
                    cloud: {
                        enabled: true,
                        provider: 'openrouter',
                        providers: [{ apiKey: 'test-key', model: 'test-model' }],
                    },
                },
            });

            const CloudClient = (await import('../../core/cloud-client.js')).default;
            client = new CloudClient();
            await new Promise((resolve) => setTimeout(resolve, 10));
            client.multiClient = null;

            global.fetch = vi.fn().mockResolvedValue({
                ok: true,
                json: async () => ({ choices: [{ message: { content: 'response' } }] }),
            });

            await client.sendRequest({
                payload: { prompt: 'test' },
                context: { state: { foo: 'bar' } },
            });

            expect(global.fetch).toHaveBeenCalledWith(
                expect.any(String),
                expect.objectContaining({
                    body: expect.stringContaining('Context - Current State'),
                })
            );
            expect(global.fetch).toHaveBeenCalledWith(
                expect.any(String),
                expect.not.objectContaining({
                    body: expect.stringContaining('Context - Recent Actions'),
                })
            );
        });

        describe('Additional Coverage Gaps', () => {
            it('should handle multi-client response with missing token count', async () => {
                const spy = vi
                    .spyOn(CloudClient.prototype, '_loadConfig')
                    .mockImplementation(async () => {});
                client = new CloudClient();
                client.multiClient = {
                    processRequest: vi.fn().mockResolvedValue({
                        success: true,
                        content: 'response',
                        tokens: null, // No tokens
                    }),
                    getStats: vi.fn(),
                };
                client.stats.totalTokens = 0;

                await client.sendRequest({ prompt: 'test' });
                expect(client.stats.totalTokens).toBe(0);
                spy.mockRestore();
            });

            it('should handle multi-client response with null content', async () => {
                const spy = vi
                    .spyOn(CloudClient.prototype, '_loadConfig')
                    .mockImplementation(async () => {});
                client = new CloudClient();
                client.multiClient = {
                    processRequest: vi.fn().mockResolvedValue({
                        success: true,
                        content: null,
                    }),
                    getStats: vi.fn(),
                };

                const result = await client.sendRequest({ prompt: 'test' });
                expect(result.success).toBe(true);
                expect(result.content).toBe('');
                expect(result.data).toBeNull();
                spy.mockRestore();
            });

            it('should pass custom temperature to free router', async () => {
                client.freeApiRouter = {
                    isReady: () => true,
                    syncWithHelper: vi.fn(),
                    getModelsInfo: vi
                        .fn()
                        .mockReturnValue({ testedWorking: [], allConfigured: [] }),
                    processRequest: vi
                        .fn()
                        .mockResolvedValue({ success: true, content: 'response' }),
                };
                client._loadConfig = vi.fn().mockResolvedValue();

                await client.sendRequest({ prompt: 'test', temperature: 0.9 });

                expect(client.freeApiRouter.processRequest).toHaveBeenCalledWith(
                    expect.objectContaining({ temperature: 0.9 })
                );
            });

            it('should handle single-key response with null content', async () => {
                // Mock single key settings
                client.apiKey = 'key';
                client.multiClient = null;
                client.freeApiRouter = { isReady: () => false };
                client._loadConfig = vi.fn().mockResolvedValue();

                global.fetch = vi.fn().mockResolvedValue({
                    ok: true,
                    json: async () => ({ choices: [{ message: { content: null } }] }),
                });

                const result = await client.sendRequest({ prompt: 'test' });
                expect(result.content).toBe('');
            });

            it('should handle single-key response with usage but no tokens', async () => {
                client.apiKey = 'key';
                client.multiClient = null;
                client.freeApiRouter = { isReady: () => false };
                client._loadConfig = vi.fn().mockResolvedValue();
                client.stats.totalTokens = 0;

                global.fetch = vi.fn().mockResolvedValue({
                    ok: true,
                    json: async () => ({
                        choices: [{ message: { content: 'resp' } }],
                        usage: {}, // Empty usage
                    }),
                });

                await client.sendRequest({ prompt: 'test' });
                expect(client.stats.totalTokens).toBe(0);
            });

            it('should use custom temperature in single-key mode', async () => {
                client.apiKey = 'key';
                client.multiClient = null;
                client.freeApiRouter = { isReady: () => false };
                client._loadConfig = vi.fn().mockResolvedValue();

                global.fetch = vi.fn().mockResolvedValue({
                    ok: true,
                    json: async () => ({ choices: [{ message: { content: 'resp' } }] }),
                });

                await client.sendRequest({ prompt: 'test', temperature: 0.5 });

                expect(global.fetch).toHaveBeenCalledWith(
                    expect.any(String),
                    expect.objectContaining({
                        body: expect.stringContaining('"temperature":0.5'),
                    })
                );
            });

            it('should handle multi-client response with tokens object but no total', async () => {
                const spy = vi
                    .spyOn(CloudClient.prototype, '_loadConfig')
                    .mockImplementation(async () => {});
                client = new CloudClient();
                client.multiClient = {
                    processRequest: vi.fn().mockResolvedValue({
                        success: true,
                        content: 'response',
                        tokens: {}, // Empty tokens object
                    }),
                    getStats: vi.fn(),
                };
                client.stats.totalTokens = 0;

                await client.sendRequest({ prompt: 'test' });
                expect(client.stats.totalTokens).toBe(0);
                spy.mockRestore();
            });

            it('should handle testFreeModels with null settings', async () => {
                const { getSettings } = await import('@api/utils/configLoader.js');
                vi.mocked(getSettings).mockResolvedValue(null);

                const result = await client.testFreeModels();
                expect(result.tested).toBe(false);
                expect(result.reason).toBe('not_enabled');
            });

            it('should handle testFreeModels with proxy enabled', async () => {
                CloudClient.sharedHelper = null;
                const settings = {
                    open_router_free_api: {
                        enabled: true,
                        models: { primary: 'p' },
                        proxy: { enabled: true, list: ['p1'] },
                    },
                };
                const { getSettings } = await import('@api/utils/configLoader.js');
                vi.mocked(getSettings).mockResolvedValue(settings);

                // Mock helper instance creation to verify args
                const { FreeOpenRouterHelper } =
                    await import('@api/utils/free-openrouter-helper.js');
                const mockHelper = {
                    testAllModelsInBackground: vi.fn(),
                    isTesting: () => false,
                    getResults: () => ({}),
                    getOptimizedModelList: () => ({ primary: 'p', fallbacks: [] }),
                    updateConfig: vi.fn(),
                    isCacheValid: () => true,
                };
                vi.mocked(FreeOpenRouterHelper.getInstance).mockReturnValue(mockHelper);

                await client.testFreeModels();

                expect(FreeOpenRouterHelper.getInstance).toHaveBeenCalledWith(
                    expect.objectContaining({
                        proxy: ['p1'],
                    })
                );
            });

            it('should return correct status from testFreeModels', async () => {
                // Case 1: Testing in progress
                CloudClient.sharedHelper = {
                    isTesting: () => true,
                    getResults: () => ({}),
                    updateConfig: vi.fn(),
                    isCacheValid: () => true,
                    testAllModelsInBackground: vi.fn(),
                };

                const settings = {
                    open_router_free_api: { enabled: true, models: { primary: 'p' } },
                };
                const { getSettings } = await import('@api/utils/configLoader.js');
                vi.mocked(getSettings).mockResolvedValue(settings);

                const result1 = await client.testFreeModels();
                expect(result1.status).toBe('testing_in_progress');

                // Case 2: Not testing
                CloudClient.sharedHelper.isTesting = () => false;
                const result2 = await client.testFreeModels();
                expect(result2.status).toBe('using_cached_results');
            });

            it('should use default model if provider has no model in single-key mode', async () => {
                const settings = {
                    llm: {
                        cloud: {
                            providers: [{ apiKey: 'k' }], // No model
                        },
                    },
                };
                const { getSettings } = await import('@api/utils/configLoader.js');
                vi.mocked(getSettings).mockResolvedValue(settings);

                client = new CloudClient();
                await new Promise((resolve) => setTimeout(resolve, 10));

                expect(client.defaultModel).toBe('openrouter/free'); // Default
            });

            it('should fallback to config models if optimized list empty', async () => {
                const settings = {
                    open_router_free_api: {
                        enabled: true,
                        models: { primary: 'p', fallbacks: ['f'] },
                        api_keys: ['k'],
                    },
                };
                const { getSettings } = await import('@api/utils/configLoader.js');
                vi.mocked(getSettings).mockResolvedValue(settings);

                // Mock helper to return empty optimized list
                const mockHelper = {
                    testAllModelsInBackground: vi.fn(),
                    getOptimizedModelList: vi
                        .fn()
                        .mockReturnValue({ primary: null, fallbacks: [] }),
                    getResults: vi.fn(),
                    isTesting: () => false,
                    updateConfig: vi.fn(),
                };
                const { FreeOpenRouterHelper } =
                    await import('@api/utils/free-openrouter-helper.js');
                vi.mocked(FreeOpenRouterHelper.getInstance).mockReturnValue(mockHelper);

                client = new CloudClient();
                await new Promise((resolve) => setTimeout(resolve, 10));

                // Verify FreeApiRouter initialized with config models
                expect(FreeApiRouter).toHaveBeenCalledWith(
                    expect.objectContaining({
                        primaryModel: 'p',
                        fallbackModels: ['f'],
                    })
                );
            });

            it('should use optimized fallbacks if available', async () => {
                CloudClient.sharedHelper = null;
                const settings = {
                    open_router_free_api: {
                        enabled: true,
                        models: { primary: 'p', fallbacks: ['f'] },
                        api_keys: ['k'],
                    },
                };
                const { getSettings } = await import('@api/utils/configLoader.js');
                vi.mocked(getSettings).mockResolvedValue(settings);

                const mockHelper = {
                    testAllModelsInBackground: vi.fn(),
                    getOptimizedModelList: vi
                        .fn()
                        .mockReturnValue({ primary: 'opt-p', fallbacks: ['opt-f'] }),
                    getResults: vi.fn(),
                    isTesting: () => false,
                    updateConfig: vi.fn(),
                };
                const { FreeOpenRouterHelper } =
                    await import('@api/utils/free-openrouter-helper.js');
                vi.mocked(FreeOpenRouterHelper.getInstance).mockReturnValue(mockHelper);

                client = new CloudClient();
                await new Promise((resolve) => setTimeout(resolve, 10));

                expect(FreeApiRouter).toHaveBeenCalledWith(
                    expect.objectContaining({
                        primaryModel: 'opt-p',
                        fallbackModels: ['opt-f'],
                    })
                );
            });

            it('should return multi-client instance', () => {
                client.multiClient = { id: 'test' };
                expect(client.getMultiClient()).toEqual({ id: 'test' });
            });

            it('should reset stats', () => {
                client.stats.totalRequests = 10;
                client.resetStats();
                expect(client.stats.totalRequests).toBe(0);
                expect(logger.info).toHaveBeenCalledWith(
                    expect.stringContaining('Statistics reset')
                );
            });

            it('should throw error in _makeRequest if multi-client is active', async () => {
                client.multiClient = {};
                await expect(client._makeRequest({})).rejects.toThrow(
                    '_makeRequest called but multi-client is active'
                );
            });

            it('should return no_models if testFreeModels called with empty model list', async () => {
                const { getSettings } = await import('@api/utils/configLoader.js');
                vi.mocked(getSettings).mockResolvedValue({
                    open_router_free_api: {
                        enabled: true,
                        models: { primary: '', fallbacks: [] },
                    },
                });

                const result = await client.testFreeModels();
                expect(result.tested).toBe(false);
                expect(result.reason).toBe('no_models');
            });

            it('should handle errors in testFreeModels', async () => {
                const { getSettings } = await import('@api/utils/configLoader.js');
                vi.mocked(getSettings).mockRejectedValue(new Error('Config error'));

                const result = await client.testFreeModels();
                expect(result.tested).toBe(false);
                expect(result.error).toBe('Config error');
            });

            it('should reuse cached results in _loadConfig', async () => {
                CloudClient.sharedHelper = {
                    getResults: () => ({ testDuration: 100, working: [], total: 1 }),
                    isTesting: () => false,
                    updateConfig: vi.fn(),
                    testAllModelsInBackground: vi.fn(),
                };

                const settings = {
                    open_router_free_api: {
                        enabled: true,
                        models: { primary: 'p' },
                        api_keys: ['k'],
                    },
                };
                const { getSettings } = await import('@api/utils/configLoader.js');
                vi.mocked(getSettings).mockResolvedValue(settings);

                client = new CloudClient();
                await new Promise((resolve) => setTimeout(resolve, 10));

                expect(logger.info).toHaveBeenCalledWith(expect.stringContaining('Reuse cache'));
            });

            it('should log tests running in _loadConfig', async () => {
                CloudClient.sharedHelper = {
                    getResults: () => ({ testDuration: 0 }),
                    isTesting: () => true,
                    updateConfig: vi.fn(),
                    testAllModelsInBackground: vi.fn(),
                };

                const settings = {
                    open_router_free_api: {
                        enabled: true,
                        models: { primary: 'p' },
                        api_keys: ['k'],
                    },
                };
                const { getSettings } = await import('@api/utils/configLoader.js');
                vi.mocked(getSettings).mockResolvedValue(settings);

                client = new CloudClient();
                await new Promise((resolve) => setTimeout(resolve, 10));

                expect(logger.info).toHaveBeenCalledWith(
                    expect.stringContaining('Tests already running')
                );
            });

            it('should trigger abort controller on timeout', async () => {
                vi.useFakeTimers();
                const abortSpy = vi.spyOn(AbortController.prototype, 'abort');

                // Mock fetch to hang until aborted
                global.fetch.mockImplementation((url, options) => {
                    return new Promise((resolve, reject) => {
                        if (options.signal) {
                            if (options.signal.aborted) {
                                const error = new Error('Aborted');
                                error.name = 'AbortError';
                                return reject(error);
                            }
                            options.signal.addEventListener('abort', () => {
                                const error = new Error('Aborted');
                                error.name = 'AbortError';
                                reject(error);
                            });
                        }
                    });
                });

                client.timeout = 1000;
                const requestPromise = client._makeRequest({});

                // Fast forward time past timeout
                vi.advanceTimersByTime(1001);

                await expect(requestPromise).rejects.toThrow('Request timeout');
                expect(abortSpy).toHaveBeenCalled();

                vi.useRealTimers();
                abortSpy.mockRestore();
            });
        });
    });
});
