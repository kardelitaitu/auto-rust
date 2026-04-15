/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import VLLMClient from '@api/core/vllm-client.js';

vi.mock('@api/core/logger.js', () => ({
    createLogger: () => ({
        info: vi.fn(),
        error: vi.fn(),
        warn: vi.fn(),
        debug: vi.fn(),
        success: vi.fn(),
    }),
}));

vi.mock('@api/utils/configLoader.js', () => ({
    getSettings: vi.fn().mockResolvedValue({
        llm: {
            vllm: {
                enabled: true,
                endpoint: 'http://localhost:8000/v1',
                model: 'meta-llama/Llama-3.3-70B-Instruct',
                timeout: 120000,
            },
        },
    }),
}));

describe('VLLMClient', () => {
    let client;

    beforeEach(() => {
        vi.clearAllMocks();
        client = new VLLMClient();
    });

    describe('constructor', () => {
        it('should initialize with default values', () => {
            // Prevent _loadConfig from running to test raw constructor defaults
            const loadConfigSpy = vi
                .spyOn(VLLMClient.prototype, '_loadConfig')
                .mockImplementation(() => {});
            const client = new VLLMClient();

            expect(client.config).toBeNull();
            expect(client.endpoint).toBe('');
            expect(client.model).toBe('');
            expect(client.timeout).toBe(120000);
            expect(client.isEnabled).toBe(false);

            loadConfigSpy.mockRestore();
        });

        it('should initialize with default config after loading', async () => {
            const client = new VLLMClient();
            // Wait for async config loading
            await new Promise((resolve) => setTimeout(resolve, 10));

            expect(client.endpoint).toBe('http://localhost:8000/v1');
            expect(client.model).toBe('meta-llama/Llama-3.3-70B-Instruct');
            expect(client.timeout).toBe(120000);
            expect(client.isEnabled).toBe(true);
        });

        it('should initialize with empty stats', () => {
            expect(client.stats).toEqual({
                totalRequests: 0,
                successfulRequests: 0,
                failedRequests: 0,
                totalDuration: 0,
            });
        });
    });

    describe('sendRequest', () => {
        it('should return disabled error when client is not enabled', async () => {
            const client = new VLLMClient();
            client.isEnabled = false;

            const result = await client.sendRequest({ prompt: 'test' });

            expect(result.success).toBe(false);
            expect(result.error).toBe('vLLM client disabled');
        });

        it('should return error on network failure', async () => {
            global.fetch = vi.fn().mockRejectedValue(new Error('Network error'));
            const client = new VLLMClient();
            client.isEnabled = true;
            client.endpoint = 'http://localhost:8000/v1';

            const result = await client.sendRequest({ prompt: 'test' });

            expect(result.success).toBe(false);
            expect(result.error).toBe('Network error');
        });

        it('should return error on timeout', async () => {
            const abortError = new Error('AbortError');
            abortError.name = 'AbortError';
            global.fetch = vi.fn().mockImplementation(() => {
                return new Promise((_, reject) => {
                    setTimeout(() => reject(abortError), 10);
                });
            });

            const client = new VLLMClient();
            client.isEnabled = true;
            client.endpoint = 'http://localhost:8000/v1';
            client.timeout = 5;

            const result = await client.sendRequest({ prompt: 'test' });

            expect(result.success).toBe(false);
            expect(result.error).toBe('Request timeout');
        });

        it('should return error on non-OK response', async () => {
            global.fetch = vi.fn().mockResolvedValue({
                ok: false,
                status: 500,
                text: vi.fn().mockResolvedValue('Internal Server Error'),
            });

            const client = new VLLMClient();
            client.isEnabled = true;
            client.endpoint = 'http://localhost:8000/v1';

            const result = await client.sendRequest({ prompt: 'test' });

            expect(result.success).toBe(false);
            expect(result.error).toContain('vLLM API error 500');
        });

        it('should return success with content on valid response', async () => {
            global.fetch = vi.fn().mockResolvedValue({
                ok: true,
                json: vi.fn().mockResolvedValue({
                    choices: [{ message: { content: 'Test response' } }],
                    usage: { total_tokens: 10 },
                }),
            });

            const client = new VLLMClient();
            client.isEnabled = true;
            client.endpoint = 'http://localhost:8000/v1';
            // Note: model is set from config in constructor
            await new Promise((resolve) => setTimeout(resolve, 10));

            const result = await client.sendRequest({ prompt: 'test prompt' });

            expect(result.success).toBe(true);
            expect(result.content).toBe('Test response');
            expect(result.metadata.model).toBe('meta-llama/Llama-3.3-70B-Instruct');
            expect(result.metadata.tokens).toBe(10);
        });

        it('should handle empty response content', async () => {
            global.fetch = vi.fn().mockResolvedValue({
                ok: true,
                json: vi.fn().mockResolvedValue({
                    choices: [],
                }),
            });

            const client = new VLLMClient();
            client.isEnabled = true;
            client.endpoint = 'http://localhost:8000/v1';

            const result = await client.sendRequest({ prompt: 'test' });

            expect(result.success).toBe(true);
            expect(result.content).toBe('');
        });
    });

    describe('_buildPayload', () => {
        it('should build payload with prompt', () => {
            const client = new VLLMClient();
            client.model = 'test-model';

            const payload = client._buildPayload({
                prompt: 'Hello',
                temperature: 0.5,
                maxTokens: 100,
            });

            expect(payload.model).toBe('test-model');
            expect(payload.messages).toContainEqual({ role: 'user', content: 'Hello' });
            expect(payload.temperature).toBe(0.5);
            expect(payload.max_tokens).toBe(100);
        });

        it('should build payload with system prompt', () => {
            const client = new VLLMClient();
            client.model = 'test-model';

            const payload = client._buildPayload({
                systemPrompt: 'You are a helpful assistant',
                userPrompt: 'Hello',
            });

            expect(payload.messages).toContainEqual({
                role: 'system',
                content: 'You are a helpful assistant',
            });
            expect(payload.messages).toContainEqual({ role: 'user', content: 'Hello' });
        });

        it('should use default temperature and max_tokens', () => {
            const client = new VLLMClient();
            client.model = 'test-model';

            const payload = client._buildPayload({ prompt: 'test' });

            expect(payload.temperature).toBe(0.7);
            expect(payload.max_tokens).toBe(2048);
        });
    });

    describe('testConnection', () => {
        it('should return false when client is disabled', async () => {
            const client = new VLLMClient();
            client.isEnabled = false;

            const result = await client.testConnection();

            expect(result).toBe(false);
        });

        it('should return true on successful connection', async () => {
            const client = new VLLMClient();
            client.isEnabled = true;
            vi.spyOn(client, 'sendRequest').mockResolvedValue({ success: true });

            const result = await client.testConnection();

            expect(result).toBe(true);
        });

        it('should return false on failed connection', async () => {
            const client = new VLLMClient();
            client.isEnabled = true;
            vi.spyOn(client, 'sendRequest').mockResolvedValue({ success: false, error: 'Failed' });

            const result = await client.testConnection();

            expect(result).toBe(false);
        });

        it('should handle errors during connection test', async () => {
            const client = new VLLMClient();
            client.isEnabled = true;
            vi.spyOn(client, 'sendRequest').mockRejectedValue(new Error('Unexpected error'));

            const result = await client.testConnection();

            expect(result).toBe(false);
        });
    });

    describe('stats', () => {
        it('should calculate stats correctly', () => {
            const client = new VLLMClient();
            client.stats = {
                totalRequests: 10,
                successfulRequests: 8,
                failedRequests: 2,
                totalDuration: 1000,
            };

            const stats = client.getStats();

            expect(stats.avgDuration).toBe(100);
            expect(stats.successRate).toBe('80.00%');
        });

        it('should handle zero requests in stats', () => {
            const client = new VLLMClient();

            const stats = client.getStats();

            expect(stats.avgDuration).toBe(0);
            expect(stats.successRate).toBe('0%');
        });

        it('should reset stats', () => {
            const client = new VLLMClient();
            client.stats = {
                totalRequests: 10,
                successfulRequests: 8,
                failedRequests: 2,
                totalDuration: 1000,
            };

            client.resetStats();

            expect(client.stats.totalRequests).toBe(0);
            expect(client.stats.successfulRequests).toBe(0);
            expect(client.stats.failedRequests).toBe(0);
            expect(client.stats.totalDuration).toBe(0);
        });
    });

    describe('_loadConfig error handling', () => {
        it('should handle config loading errors', async () => {
            const getSettingsSpy = await import('@api/utils/configLoader.js');
            getSettingsSpy.getSettings.mockRejectedValueOnce(new Error('Config error'));

            const client = new VLLMClient();
            await new Promise((resolve) => setTimeout(resolve, 10)); // Wait for async load

            // Should not crash, just log error
            expect(client.isEnabled).toBe(false);
        });

        it('should log when client is disabled via config', async () => {
            const getSettingsSpy = await import('@api/utils/configLoader.js');
            getSettingsSpy.getSettings.mockResolvedValueOnce({
                llm: { vllm: { enabled: false } },
            });

            const client = new VLLMClient();
            await new Promise((resolve) => setTimeout(resolve, 10));

            expect(client.isEnabled).toBe(false);
        });
    });

    describe('getStats', () => {
        it('should return stats with calculated averages', () => {
            const client = new VLLMClient();
            client.stats = {
                totalRequests: 10,
                successfulRequests: 8,
                failedRequests: 2,
                totalDuration: 1000,
            };

            const stats = client.getStats();

            expect(stats.totalRequests).toBe(10);
            expect(stats.successfulRequests).toBe(8);
            expect(stats.avgDuration).toBe(100);
            expect(stats.successRate).toBe('80.00%');
        });

        it('should handle zero requests', () => {
            const client = new VLLMClient();

            const stats = client.getStats();

            expect(stats.totalRequests).toBe(0);
            expect(stats.avgDuration).toBe(0);
            expect(stats.successRate).toBe('0%');
        });
    });

    describe('resetStats', () => {
        it('should reset all stats to zero', () => {
            const client = new VLLMClient();
            client.stats = {
                totalRequests: 100,
                successfulRequests: 50,
                failedRequests: 50,
                totalDuration: 5000,
            };

            client.resetStats();

            expect(client.stats.totalRequests).toBe(0);
            expect(client.stats.successfulRequests).toBe(0);
            expect(client.stats.failedRequests).toBe(0);
            expect(client.stats.totalDuration).toBe(0);
        });
    });
});
