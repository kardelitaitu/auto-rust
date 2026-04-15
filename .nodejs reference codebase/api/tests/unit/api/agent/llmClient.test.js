/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import llmClient from '@api/agent/llmClient.js';

const defaultConfig = {
    baseUrl: 'http://localhost:11434',
    model: 'llama2',
    serverType: 'ollama',
    temperature: 0.7,
    contextLength: 4096,
    maxTokens: 2000,
    timeoutMs: 60000,
    useVision: false,
    bypassHealthCheck: false,
};

vi.mock('@api/core/config.js', () => ({
    configManager: {
        init: vi.fn().mockResolvedValue(undefined),
        get: vi.fn().mockReturnValue({
            baseUrl: 'http://localhost:11434',
            model: 'llama2',
            serverType: 'ollama',
            temperature: 0.7,
            contextLength: 4096,
            maxTokens: 2000,
            timeoutMs: 60000,
            useVision: false,
            bypassHealthCheck: false,
        }),
        _getDefaults: vi.fn().mockReturnValue({
            agent: { llm: { baseUrl: 'http://localhost:11434', model: 'llama2' } },
        }),
    },
}));

describe('LLMClient', () => {
    let originalFetch;

    beforeEach(async () => {
        llmClient.config = null;
        originalFetch = global.fetch;

        const { configManager } = await import('@api/core/config.js');
        configManager.get.mockReturnValue({ ...defaultConfig });
    });

    afterEach(() => {
        global.fetch = originalFetch;
        vi.restoreAllMocks();
    });

    describe('checkAvailability', () => {
        it('should return true when response.ok is true', async () => {
            const mockResponse = {
                ok: true,
                json: vi.fn().mockResolvedValue({ data: [{ name: 'llama2' }] }),
            };
            global.fetch = vi.fn().mockResolvedValue(mockResponse);

            const result = await llmClient.checkAvailability();

            expect(result).toBe(true);
            expect(global.fetch).toHaveBeenCalledWith(
                'http://localhost:11434/models',
                expect.objectContaining({ method: 'GET' })
            );
        });

        it('should return false when response.ok is false', async () => {
            const mockResponse = {
                ok: false,
                status: 500,
                text: vi.fn().mockResolvedValue('Internal Server Error'),
            };
            global.fetch = vi.fn().mockResolvedValue(mockResponse);

            const result = await llmClient.checkAvailability();

            expect(result).toBe(false);
        });

        it('should return true on fetch error when bypassHealthCheck is true', async () => {
            const mockConfig = {
                baseUrl: 'http://localhost:11434',
                model: 'llama2',
                serverType: 'ollama',
                temperature: 0.7,
                contextLength: 4096,
                maxTokens: 2000,
                timeoutMs: 60000,
                useVision: false,
                bypassHealthCheck: true,
            };

            const { configManager } = await import('@api/core/config.js');
            configManager.get.mockReturnValue(mockConfig);

            llmClient.config = mockConfig;
            global.fetch = vi.fn().mockRejectedValue(new Error('Connection refused'));

            const result = await llmClient.checkAvailability();

            expect(result).toBe(true);
        });
    });

    describe('generateCompletion', () => {
        it('should generate completion for ollama server type', async () => {
            const mockResponse = {
                ok: true,
                json: vi.fn().mockResolvedValue({
                    choices: [{ message: { content: '{"result": "test"}' } }],
                    usage: { prompt_tokens: 10, completion_tokens: 5, total_tokens: 15 },
                }),
            };
            global.fetch = vi.fn().mockResolvedValue(mockResponse);

            const messages = [{ role: 'user', content: 'Hello' }];
            const result = await llmClient.generateCompletion(messages);

            expect(global.fetch).toHaveBeenCalledWith(
                'http://localhost:11434/api/chat',
                expect.objectContaining({
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: expect.stringContaining('"model":"llama2"'),
                })
            );
            expect(result).toEqual({ result: 'test' });
        });

        it('should generate completion for openai server type', async () => {
            const mockConfig = {
                baseUrl: 'https://api.openai.com/v1',
                model: 'gpt-4',
                serverType: 'openai',
                temperature: 0.7,
                contextLength: 4096,
                maxTokens: 2000,
                timeoutMs: 60000,
                useVision: false,
                bypassHealthCheck: false,
            };

            const { configManager } = await import('@api/core/config.js');
            configManager.get.mockReturnValue(mockConfig);

            llmClient.config = mockConfig;

            const mockResponse = {
                ok: true,
                json: vi.fn().mockResolvedValue({
                    choices: [{ message: { content: '{"response": "ok"}' } }],
                    usage: { prompt_tokens: 20, completion_tokens: 10, total_tokens: 30 },
                }),
            };
            global.fetch = vi.fn().mockResolvedValue(mockResponse);

            const messages = [{ role: 'user', content: 'Hello' }];
            const result = await llmClient.generateCompletion(messages);

            expect(global.fetch).toHaveBeenCalledWith(
                'https://api.openai.com/v1/chat/completions',
                expect.objectContaining({
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: expect.stringContaining('"model":"gpt-4"'),
                })
            );
            expect(result).toEqual({ response: 'ok' });
        });
    });

    describe('getUsageStats', () => {
        it('should return config info', async () => {
            await llmClient.init();

            const stats = llmClient.getUsageStats();

            expect(stats).toEqual({
                model: 'llama2',
                baseUrl: 'http://localhost:11434',
                useVision: false,
                isRestarting: false,
            });
        });

        it('should return config with isRestarting true', async () => {
            await llmClient.init();
            llmClient.isRestarting = true;
            llmClient.restartPromise = Promise.resolve();

            const stats = llmClient.getUsageStats();

            expect(stats.isRestarting).toBe(true);
        });
    });

    describe('error handling', () => {
        it('should throw when fetch fails without bypassHealthCheck', async () => {
            global.fetch = vi.fn().mockRejectedValue(new Error('Connection refused'));

            const result = await llmClient.checkAvailability();

            expect(result).toBe(false);
        });

        it('should handle JSON parse error in response', async () => {
            const mockResponse = {
                ok: true,
                json: vi.fn().mockResolvedValue({
                    choices: [{ message: { content: 'invalid json' } }],
                }),
            };
            global.fetch = vi.fn().mockResolvedValue(mockResponse);

            const messages = [{ role: 'user', content: 'Hello' }];

            await expect(llmClient.generateCompletion(messages)).rejects.toThrow();
        });
    });
});
