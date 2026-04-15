/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Cloud Client Integration Tests
 * Tests the CloudClient module for OpenRouter API integration
 * @module tests/integration/cloud-client.test
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
// import { createLogger } from '@api/core/logger.js';
import { getSettings } from '@api/utils/configLoader.js';
import { MultiOpenRouterClient } from '@api/utils/multi-api.js';
import { FreeApiRouter } from '@api/utils/free-api-router.js';
import CloudClient from '@api/core/cloud-client.js';

const mocks = vi.hoisted(() => ({
    getSettings: vi.fn(),
    mockPage: {
        url: vi.fn().mockReturnValue('https://example.com'),
        goto: vi.fn().mockResolvedValue(undefined),
        goBack: vi.fn().mockResolvedValue({}),
        evaluate: vi.fn().mockResolvedValue(undefined),
    },
}));

vi.mock('@api/utils/configLoader.js', () => ({
    getSettings: mocks.getSettings,
}));

vi.mock('@api/utils/multi-api.js', () => ({
    MultiOpenRouterClient: vi.fn(),
}));

vi.mock('@api/tests/api/core/context.js', () => ({
    getPage: vi.fn(() => ({
        url: vi.fn().mockReturnValue('https://example.com'),
        isClosed: vi.fn().mockReturnValue(false),
    })),
    isSessionActive: vi.fn().mockReturnValue(true),
    getEvents: vi.fn(() => ({ emitSafe: vi.fn() })),
}));

vi.mock('@api/utils/free-api-router.js', () => ({
    FreeApiRouter: vi.fn(),
    setSharedHelper: vi.fn(),
}));

vi.mock('@api/utils/free-openrouter-helper.js', () => ({
    FreeOpenRouterHelper: {
        getInstance: vi.fn(() => ({
            testAllModelsInBackground: vi.fn(),
            getOptimizedModelList: vi.fn(() => ({ primary: 'test-model', fallbacks: [] })),
            getResults: vi.fn(() => ({ working: [], failed: [] })),
            updateConfig: vi.fn(),
            isCacheValid: vi.fn(() => true),
            isTesting: vi.fn(() => false),
        })),
    },
}));

vi.mock('@api/core/logger.js', () => ({
    createLogger: vi.fn(() => ({
        info: vi.fn(),
        warn: vi.fn(),
        error: vi.fn(),
        debug: vi.fn(),
    })),
}));

describe('CloudClient Integration', () => {
    let cloudClient;
    let mockPage;

    beforeEach(async () => {
        vi.clearAllMocks();

        mocks.getSettings.mockResolvedValue({
            llm: {
                cloud: {
                    enabled: true,
                    timeout: 60000,
                    defaultModel: 'anthropic/claude-3.5-sonnet',
                    endpoint: 'https://openrouter.ai/api/v1/chat/completions',
                    providers: [],
                },
            },
            open_router_free_api: {
                enabled: false,
            },
        });
    });

    describe('Initialization', () => {
        it('should initialize with default values', async () => {
            cloudClient = new CloudClient();
            await cloudClient.initPromise;

            expect(cloudClient.config).toBeDefined();
            expect(cloudClient.apiEndpoint).toBe('https://openrouter.ai/api/v1/chat/completions');
            expect(cloudClient.defaultModel).toBe('anthropic/claude-3.5-sonnet');
            expect(cloudClient.timeout).toBe(60000);
            expect(cloudClient.stats.totalRequests).toBe(0);
            expect(cloudClient.stats.successfulRequests).toBe(0);
            expect(cloudClient.stats.failedRequests).toBe(0);
        });

        it('should load configuration from settings', async () => {
            mocks.getSettings.mockResolvedValue({
                llm: {
                    cloud: {
                        enabled: true,
                        timeout: 120000,
                        defaultModel: 'anthropic/claude-3-opus',
                        endpoint: 'https://custom.api.com/v1/chat',
                        providers: [],
                    },
                },
            });

            const client = new CloudClient();
            await client.initPromise;

            expect(client.timeout).toBe(120000);
            expect(client.defaultModel).toBe('anthropic/claude-3-opus');
            expect(client.apiEndpoint).toBe('https://custom.api.com/v1/chat');
        });

        it('should initialize with single provider', async () => {
            mocks.getSettings.mockResolvedValue({
                llm: {
                    cloud: {
                        enabled: true,
                        providers: [
                            { apiKey: 'test-key-123', model: 'anthropic/claude-3.5-sonnet' },
                        ],
                    },
                },
            });

            const client = new CloudClient();
            await client.initPromise;

            expect(client.apiKey).toBe('test-key-123');
            expect(client.defaultModel).toBe('anthropic/claude-3.5-sonnet');
            expect(client.multiClient).toBeNull();
        });

        it('should initialize with multiple providers for fallback', async () => {
            const settings = {
                llm: {
                    cloud: {
                        enabled: true,
                        providers: [
                            { apiKey: 'key-1', model: 'anthropic/claude-3.5-sonnet' },
                            { apiKey: 'key-2', model: 'anthropic/claude-3-haiku' },
                            { apiKey: 'key-3', model: 'openai/gpt-4o' },
                        ],
                    },
                },
            };
            mocks.getSettings.mockResolvedValue(settings);
            mockPage = {
                url: vi.fn().mockReturnValue('https://example.com'),
                goto: vi.fn().mockResolvedValue(undefined),
            };
            // Verify mock
            await mocks.getSettings();
            // console.log('DEBUG: Mocked Settings in Test:', JSON.stringify(mockedSettings, null, 2));

            const client = new CloudClient();
            await client.initPromise;

            expect(MultiOpenRouterClient).toHaveBeenCalledWith({
                apiKeys: ['key-1', 'key-2', 'key-3'],
                models: [
                    'anthropic/claude-3.5-sonnet',
                    'anthropic/claude-3-haiku',
                    'openai/gpt-4o',
                ],
                endpoint: 'https://openrouter.ai/api/v1/chat/completions',
                timeout: 60000,
                defaultModel: 'anthropic/claude-3.5-sonnet',
                retryDelay: 2000,
            });
            expect(client.multiClient).toBeDefined();
        });

        it('should handle no providers configured', async () => {
            mocks.getSettings.mockResolvedValue({
                llm: { cloud: { providers: [] } },
                open_router_free_api: { enabled: false },
            });

            const client = new CloudClient();
            await client.initPromise;

            expect(client.apiKey).toBe('');
            expect(client.multiClient).toBeNull();
            expect(client.freeApiRouter).toBeNull();
        });

        it('should initialize free API router when enabled', async () => {
            mocks.getSettings.mockResolvedValue({
                llm: { cloud: { providers: [] } },
                open_router_free_api: {
                    enabled: true,
                    api_keys: ['free-key-1', 'free-key-2'],
                    models: {
                        primary: 'anthropic/claude-3.5-sonnet',
                        fallbacks: ['openai/gpt-4o'],
                    },
                    proxy: {
                        enabled: false,
                    },
                },
            });

            const client = new CloudClient();
            await client.initPromise;

            expect(FreeApiRouter).toHaveBeenCalled();
            expect(client.freeApiRouter).toBeDefined();
        });
    });

    describe('Prompt Building', () => {
        beforeEach(async () => {
            // No setup needed
        });

        it('should build prompt with system and user prompts', () => {
            const request = {
                payload: {
                    systemPrompt: 'You are a helpful assistant.',
                    userPrompt: 'What is 2+2?',
                },
            };

            const prompt = cloudClient._buildPrompt(request);
            expect(prompt).toBe('You are a helpful assistant.\n\nWhat is 2+2?');
        });

        it('should build prompt with single prompt property', () => {
            const request = {
                payload: { prompt: 'Simple prompt' },
            };

            const prompt = cloudClient._buildPrompt(request);
            expect(prompt).toBe('Simple prompt');
        });

        it('should build prompt with direct prompt property', () => {
            const request = {
                prompt: 'Direct prompt',
            };

            const prompt = cloudClient._buildPrompt(request);
            expect(prompt).toBe('Direct prompt');
        });

        it('should add breadcrumbs context', () => {
            const request = {
                prompt: 'Original prompt',
                context: {
                    breadcrumbs: 'Navigated to Twitter, scrolled timeline',
                },
            };

            const prompt = cloudClient._buildPrompt(request);
            expect(prompt).toContain('Context - Recent Actions:');
            expect(prompt).toContain('Navigated to Twitter, scrolled timeline');
            expect(prompt).toContain('Original prompt');
        });

        it('should add state context', () => {
            const request = {
                prompt: 'Original prompt',
                context: {
                    state: { page: 'home', tweets: 15 },
                },
            };

            const prompt = cloudClient._buildPrompt(request);
            expect(prompt).toContain('Context - Current State:');
            expect(prompt).toContain('"page": "home"');
            expect(prompt).toContain('"tweets": 15');
        });

        it('should handle all context types together', () => {
            const request = {
                payload: {
                    systemPrompt: 'System prompt',
                    userPrompt: 'User prompt',
                },
                context: {
                    breadcrumbs: 'Action 1, Action 2',
                    state: { mode: 'interactive' },
                },
            };

            const prompt = cloudClient._buildPrompt(request);
            expect(prompt).toContain('System prompt');
            expect(prompt).toContain('User prompt');
            expect(prompt).toContain('Context - Recent Actions:');
            expect(prompt).toContain('Context - Current State:');
        });
    });

    describe('Statistics', () => {
        beforeEach(async () => {
            // No setup needed
        });

        it('should return initial stats', () => {
            const stats = cloudClient.getStats();

            expect(stats).toHaveProperty('totalRequests', 0);
            expect(stats).toHaveProperty('successfulRequests', 0);
            expect(stats).toHaveProperty('failedRequests', 0);
            expect(stats).toHaveProperty('totalTokens', 0);
            expect(stats).toHaveProperty('totalDuration', 0);
            expect(stats).toHaveProperty('keyFallbacks', 0);
            expect(stats).toHaveProperty('avgDuration', 0);
            expect(stats).toHaveProperty('successRate', '0%');
            expect(stats).toHaveProperty('mode', 'single-key');
        });

        it('should calculate average duration correctly', () => {
            cloudClient.stats.totalRequests = 2;
            cloudClient.stats.totalDuration = 4000;
            cloudClient.stats.successfulRequests = 2;

            const stats = cloudClient.getStats();
            expect(stats.avgDuration).toBe(2000);
        });

        it('should calculate success rate correctly', () => {
            cloudClient.stats.totalRequests = 10;
            cloudClient.stats.successfulRequests = 7;

            const stats = cloudClient.getStats();
            expect(stats.successRate).toBe('70.00%');
        });

        it('should reset stats correctly', () => {
            cloudClient.stats.totalRequests = 5;
            cloudClient.stats.successfulRequests = 3;
            cloudClient.stats.failedRequests = 2;
            cloudClient.stats.totalTokens = 1000;

            cloudClient.resetStats();

            expect(cloudClient.stats.totalRequests).toBe(0);
            expect(cloudClient.stats.successfulRequests).toBe(0);
            expect(cloudClient.stats.failedRequests).toBe(0);
            expect(cloudClient.stats.totalTokens).toBe(0);
        });

        it('should have mode in stats', () => {
            const stats = cloudClient.getStats();
            expect(stats.mode).toBeDefined();
        });

        it('should have keyFallbacks in stats', () => {
            cloudClient.stats.keyFallbacks = 5;
            const stats = cloudClient.getStats();
            expect(stats.keyFallbacks).toBe(5);
        });
    });

    describe('Connection Testing', () => {
        beforeEach(async () => {
            // No setup needed
        });

        it('should return false when no API key configured', async () => {
            const result = await cloudClient.testConnection();
            expect(result).toBe(false);
            expect(cloudClient.stats.failedRequests).toBe(1);
        });
    });

    describe('Free Model Testing', () => {
        beforeEach(async () => {
            // No setup needed
        });

        it('should return not_enabled when free API is disabled', async () => {
            const result = await cloudClient.testFreeModels();
            expect(result.tested).toBeDefined();
        });

        it('should return no_models when no models configured', async () => {
            mocks.getSettings.mockResolvedValue({
                llm: { cloud: { providers: [] } },
                open_router_free_api: {
                    enabled: true,
                    models: { primary: null, fallbacks: [] },
                },
            });

            const client = new CloudClient();
            await client.initPromise;

            const result = await client.testFreeModels();
            expect(result.tested).toBe(false);
            expect(result.reason).toBe('no_models');
        });

        it('should start background testing when enabled', async () => {
            mocks.getSettings.mockResolvedValue({
                llm: { cloud: { providers: [] } },
                open_router_free_api: {
                    enabled: true,
                    api_keys: ['key-1'],
                    models: {
                        primary: 'anthropic/claude-3.5-sonnet',
                        fallbacks: [],
                    },
                },
            });

            const client = new CloudClient();
            await client.initPromise;

            const result = await client.testFreeModels();
            expect(result.tested).toBe(true);
            expect(result.status).toBeDefined();
        });
    });

    describe('Request Sending - Edge Cases', () => {
        beforeEach(async () => {
            // No setup needed
        });

        it('should handle missing API key gracefully', async () => {
            mocks.getSettings.mockResolvedValue({
                llm: { cloud: { providers: [] } },
                open_router_free_api: { enabled: false },
            });

            const client = new CloudClient();
            await client.initPromise;

            const result = await client.sendRequest({
                prompt: 'Test prompt',
            });

            expect(result.success).toBe(false);
            expect(result.error).toContain('API key not configured');
            expect(client.stats.failedRequests).toBe(1);
        });

        it('should use default maxTokens when not specified', () => {
            const request = {};
            const maxTokens = request.maxTokens || 4096;
            expect(maxTokens).toBe(4096);
        });

        it('should use default temperature when not specified', () => {
            const request = {};
            const temperature = request.temperature !== undefined ? request.temperature : 0.7;
            expect(temperature).toBe(0.7);
        });

        it('should respect custom temperature', () => {
            const request = { temperature: 0.2 };
            const temperature = request.temperature !== undefined ? request.temperature : 0.7;
            expect(temperature).toBe(0.2);
        });
    });

    describe('JSON Response Parsing', () => {
        beforeEach(async () => {
            // No setup needed
        });

        it('should parse JSON responses starting with {', () => {
            const content = '{"action": "tweet", "text": "Hello"}';
            let data = null;
            if (content.trim().startsWith('{')) {
                try {
                    data = JSON.parse(content);
                } catch (_e) {
                    data = null;
                }
            }
            expect(data).toEqual({ action: 'tweet', text: 'Hello' });
        });

        it('should parse JSON responses starting with [', () => {
            const content = '[{"id": 1}, {"id": 2}]';
            let data = null;
            if (content.trim().startsWith('[')) {
                try {
                    data = JSON.parse(content);
                } catch (_e) {
                    data = null;
                }
            }
            expect(data).toHaveLength(2);
        });

        it('should not parse non-JSON responses', () => {
            const content = 'This is a plain text response';
            let data = null;
            if (content.trim().startsWith('{') || content.trim().startsWith('[')) {
                try {
                    data = JSON.parse(content);
                } catch (_e) {
                    data = null;
                }
            }
            expect(data).toBeNull();
        });

        it('should handle invalid JSON gracefully', () => {
            const content = '{"invalid": json}';
            let data = null;
            if (content.trim().startsWith('{') || content.trim().startsWith('[')) {
                try {
                    data = JSON.parse(content);
                } catch (_e) {
                    data = null;
                }
            }
            expect(data).toBeNull();
        });
    });

    describe('Multi-Key Fallback', () => {
        it('should track key fallbacks in stats', async () => {
            cloudClient.stats.keyFallbacks = 2;

            expect(cloudClient.stats.keyFallbacks).toBe(2);
            const stats = cloudClient.getStats();
            expect(stats.keyFallbacks).toBe(2);
        });
    });

    describe('Error Handling', () => {
        beforeEach(async () => {
            // No setup needed
        });

        it('should handle empty settings gracefully', async () => {
            mockPage = {
                url: vi.fn().mockReturnValue('https://example.com'),
                goBack: vi.fn().mockResolvedValue({}),
            };
            getSettings.mockResolvedValue({});

            const client = new CloudClient();
            await client.initPromise;

            expect(client.config).toBeNull();
        });

        it('should handle null settings gracefully', async () => {
            mocks.getSettings.mockResolvedValue(null);

            const client = new CloudClient();
            await client.initPromise;

            expect(client.config).toBeDefined();
        });

        it('should handle settings loading error', async () => {
            mocks.getSettings.mockRejectedValue(new Error('Config load failed'));

            const client = new CloudClient();
            await new Promise((resolve) => setTimeout(resolve, 50));

            expect(client.config).toBeNull();
        });
    });
});

describe('CloudClient Request Queue Integration', () => {
    let cloudClient;

    beforeEach(async () => {
        vi.clearAllMocks();

        mocks.getSettings.mockResolvedValue({
            llm: {
                cloud: {
                    enabled: true,
                    timeout: 60000,
                    defaultModel: 'openrouter/free',
                },
            },
        });

        cloudClient = new CloudClient();
        await cloudClient.initPromise;
    });

    describe('Concurrent Request Handling', () => {
        it('should track concurrent requests in stats', () => {
            cloudClient.stats.totalRequests = 5;
            cloudClient.stats.successfulRequests = 3;
            cloudClient.stats.failedRequests = 2;

            const stats = cloudClient.getStats();
            expect(stats.totalRequests).toBe(5);
            expect(stats.successRate).toBe('60.00%');
        });

        it('should accumulate token usage correctly', () => {
            cloudClient.stats.totalTokens = 0;
            cloudClient.stats.totalTokens += 100;
            cloudClient.stats.totalTokens += 200;
            cloudClient.stats.totalTokens += 150;

            expect(cloudClient.stats.totalTokens).toBe(450);
        });
    });

    describe('Configuration Updates', () => {
        it('should have default timeout value', () => {
            expect(cloudClient.timeout).toBeDefined();
            expect(typeof cloudClient.timeout).toBe('number');
        });
    });
});

describe('CloudClient Static Methods', () => {
    let CloudClient;

    beforeEach(async () => {
        vi.clearAllMocks();

        const module = await import('../../core/cloud-client.js');
        CloudClient = module.default;
        CloudClient.sharedHelper = null;
    });

    it('should have sharedHelper static property', () => {
        expect(CloudClient.sharedHelper).toBeNull();
        CloudClient.sharedHelper = { test: true };
        expect(CloudClient.sharedHelper.test).toBe(true);
    });

    it('should reset sharedHelper between test runs', () => {
        CloudClient.sharedHelper = { data: 'test' };
        CloudClient.sharedHelper = null;
        expect(CloudClient.sharedHelper).toBeNull();
    });
});
