/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { MultiOpenRouterClient } from '@api/utils/multi-api.js';

vi.mock('@api/core/logger.js', () => ({
    createLogger: vi.fn(() => ({
        info: vi.fn(),
        error: vi.fn(),
        warn: vi.fn(),
        debug: vi.fn(),
    })),
}));

describe('MultiOpenRouterClient', () => {
    let client;

    beforeEach(() => {
        vi.clearAllMocks();
        client = new MultiOpenRouterClient({
            apiKeys: ['key1', 'key2'],
            models: ['model1', 'model2'],
            timeout: 5000,
            retryDelay: 100,
        });
    });

    describe('constructor', () => {
        it('should create with default config', () => {
            const defaultClient = new MultiOpenRouterClient();
            expect(defaultClient.apiKeys).toEqual([]);
            expect(defaultClient.defaultModel).toBe('arcee-ai/trinity-large-preview:free');
            expect(defaultClient.timeout).toBe(120000);
        });

        it('should create with custom config', () => {
            expect(client.apiKeys).toEqual(['key1', 'key2']);
            expect(client.models).toEqual(['model1', 'model2']);
            expect(client.timeout).toBe(5000);
        });
    });

    describe('processRequest', () => {
        it('should make successful request with first key', async () => {
            global.fetch = vi.fn().mockResolvedValue({
                ok: true,
                json: vi.fn().mockResolvedValue({
                    choices: [{ message: { content: 'Hello' } }],
                    model: 'model1',
                    usage: { prompt_tokens: 10, completion_tokens: 5, total_tokens: 15 },
                }),
            });

            const result = await client.processRequest({
                messages: [{ role: 'user', content: 'Hi' }],
            });

            expect(result.success).toBe(true);
            expect(result.content).toBe('Hello');
            expect(result.model).toBe('model1');
            expect(result.keyUsed).toBe(0);
        });

        it('should fallback to second key on rate limit', async () => {
            global.fetch = vi
                .fn()
                .mockRejectedValueOnce(new Error('429 Rate limit'))
                .mockResolvedValueOnce({
                    ok: true,
                    json: vi.fn().mockResolvedValue({
                        choices: [{ message: { content: 'Hello' } }],
                        model: 'model2',
                    }),
                });

            const result = await client.processRequest({
                messages: [{ role: 'user', content: 'Hi' }],
            });

            expect(result.success).toBe(true);
            expect(result.keyUsed).toBe(1);
            expect(client.stats.apiKeyFallbacks).toBe(1);
        });

        it('should return failure after all keys fail', async () => {
            global.fetch = vi.fn().mockRejectedValue(new Error('Invalid API key'));

            const result = await client.processRequest({
                messages: [{ role: 'user', content: 'Hi' }],
            });

            expect(result.success).toBe(false);
            expect(result.keysTried).toBe(2);
        });

        it('should handle non-retryable errors', async () => {
            global.fetch = vi.fn().mockRejectedValue(new Error('Invalid API key'));

            const result = await client.processRequest({
                messages: [{ role: 'user', content: 'Hi' }],
            });

            expect(result.success).toBe(false);
            expect(result.error).toBe('Invalid API key');
        });
    });

    describe('callAPI', () => {
        it('should make successful API call', async () => {
            global.fetch = vi.fn().mockResolvedValue({
                ok: true,
                json: vi.fn().mockResolvedValue({
                    choices: [{ message: { content: 'Response' } }],
                    model: 'test-model',
                    usage: { prompt_tokens: 5, completion_tokens: 10, total_tokens: 15 },
                }),
            });

            const result = await client.callAPI('test-key', [{ role: 'user', content: 'Hi' }], {
                maxTokens: 100,
                temperature: 0.7,
                model: 'test-model',
            });

            expect(result.content).toBe('Response');
            expect(result.model).toBe('test-model');
            expect(result.tokens.total).toBe(15);
        });

        it('should throw on HTTP error', async () => {
            global.fetch = vi.fn().mockResolvedValue({
                ok: false,
                status: 401,
                text: vi.fn().mockResolvedValue('Unauthorized'),
            });

            await expect(client.callAPI('bad-key', [], {})).rejects.toThrow(
                'HTTP 401: Unauthorized'
            );
        });

        it('should handle empty response', async () => {
            global.fetch = vi.fn().mockResolvedValue({
                ok: true,
                json: vi.fn().mockResolvedValue({
                    choices: [],
                    model: 'test',
                }),
            });

            const result = await client.callAPI('key', [], {});
            expect(result.content).toBe('');
        });
    });

    describe('isRetryableError', () => {
        it('should return true for rate limit', () => {
            expect(client.isRetryableError(new Error('429 rate limit exceeded'))).toBe(true);
        });

        it('should return true for quota exceeded', () => {
            expect(client.isRetryableError(new Error('quota exceeded'))).toBe(true);
        });

        it('should return true for overloaded', () => {
            expect(client.isRetryableError(new Error('Server overloaded'))).toBe(true);
        });

        it('should return true for 503 error', () => {
            expect(client.isRetryableError(new Error('503 Service Unavailable'))).toBe(true);
        });

        it('should return false for other errors', () => {
            expect(client.isRetryableError(new Error('Invalid API key'))).toBe(false);
        });
    });

    describe('sleep', () => {
        it('should resolve after specified time', async () => {
            const start = Date.now();
            await client.sleep(50);
            const elapsed = Date.now() - start;
            expect(elapsed).toBeGreaterThanOrEqual(40);
        });
    });

    describe('getStats', () => {
        it('should return empty stats initially', () => {
            const stats = client.getStats();
            expect(stats.totalRequests).toBe(0);
            expect(stats.successes).toBe(0);
            expect(stats.keysConfigured).toBe(2);
        });

        it('should calculate success rate', () => {
            client.stats.totalRequests = 10;
            client.stats.successes = 8;

            const stats = client.getStats();
            expect(stats.successRate).toBe('80.0%');
        });
    });

    describe('resetStats', () => {
        it('should reset all stats', () => {
            client.stats.totalRequests = 10;
            client.stats.successes = 5;
            client.stats.failures = 5;
            client.stats.apiKeyFallbacks = 2;

            client.resetStats();

            expect(client.stats.totalRequests).toBe(0);
            expect(client.stats.successes).toBe(0);
            expect(client.stats.failures).toBe(0);
            expect(client.stats.apiKeyFallbacks).toBe(0);
        });
    });
});
