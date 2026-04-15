/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach, beforeAll } from 'vitest';
import { logger } from '@api/core/logger.js';

// Define mock variables at module level
let LocalClient;
let VLLMClient;
let OllamaClient;

// Mock dependencies
vi.mock('@api/core/logger.js', () => {
    const mockLogger = {
        info: vi.fn(),
        warn: vi.fn(),
        error: vi.fn(),
        debug: vi.fn(),
        success: vi.fn(),
    };
    return {
        logger: mockLogger,
        createLogger: () => mockLogger,
    };
});

vi.mock('@api/utils/configLoader.js', () => ({
    getSettings: vi.fn(),
}));

// Mock client classes
vi.mock('@api/core/vllm-client.js', () => {
    return {
        __esModule: true,
        default: vi.fn(),
    };
});

vi.mock('@api/core/ollama-client.js', () => {
    return {
        __esModule: true,
        default: vi.fn(),
    };
});

import { getSettings } from '@api/utils/configLoader.js';

describe('LocalClient', () => {
    let client;
    let mockLoggerInstance;

    beforeAll(async () => {
        // Import modules after mocking
        const localClientModule = await import('../../core/local-client.js');
        LocalClient = localClientModule.default;

        const vllmModule = await import('@api/core/vllm-client.js');
        VLLMClient = vllmModule.default;

        const ollamaModule = await import('@api/core/ollama-client.js');
        OllamaClient = ollamaModule.default;
    });

    beforeEach(() => {
        vi.clearAllMocks();

        // Reset mock implementations to defaults
        VLLMClient.mockImplementation(function () {
            return {
                sendRequest: vi.fn(),
                resetStats: vi.fn(),
            };
        });

        OllamaClient.mockImplementation(function () {
            return {
                initialize: vi.fn(),
                generate: vi.fn(),
                resetStats: vi.fn(),
            };
        });

        // Default settings
        getSettings.mockResolvedValue({
            llm: {
                local: { enabled: true },
                vllm: { enabled: true },
            },
        });

        mockLoggerInstance = logger;
    });

    describe('Initialization', () => {
        it('should initialize both if enabled', async () => {
            getSettings.mockResolvedValue({
                llm: {
                    local: { enabled: true },
                    vllm: { enabled: true },
                },
            });

            client = new LocalClient();
            await new Promise((resolve) => setTimeout(resolve, 20)); // Allow async config to load

            expect(getSettings).toHaveBeenCalled();
            expect(client.vllmEnabled).toBe(true);
            expect(client.vllmClient).toBeTruthy();
            expect(client.ollamaEnabled).toBe(true); // ollamaEnabled logic: local.enabled !== false. Here local.enabled is true. Wait, logic is ollamaEnabled = localConfig.enabled !== false.
            // In test setup: local: { enabled: true }. So ollamaEnabled should be true?
            // Wait, previous test expectation was false?
            // Let's check logic in local-client.js:
            // this.ollamaEnabled = localConfig.enabled !== false;
            // If localConfig.enabled is true, then true !== false is TRUE.
            // Why did I expect false before?
            // Ah, in previous test it was `expect(client.ollamaEnabled).toBe(false)`.
            // Maybe I misread the logic or previous test had different settings.
            // Actually, if local.enabled is true, ollamaEnabled is true.
            // Let's check `local-client.js` logic again.
            // const localConfig = settings.llm?.local || {};
            // this.ollamaEnabled = localConfig.enabled !== false;
            // If settings has local: { enabled: true }, then ollamaEnabled is true.
            // I will correct the expectation to toBe(true).
        });

        it('should log if all clients disabled', async () => {
            getSettings.mockResolvedValue({
                llm: {
                    local: { enabled: false },
                    vllm: { enabled: false },
                },
            });

            client = new LocalClient();
            await new Promise((resolve) => setTimeout(resolve, 20));

            expect(mockLoggerInstance.info).toHaveBeenCalledWith(
                expect.stringContaining('All local clients are disabled')
            );
        });

        it('should handle config loading error', async () => {
            getSettings.mockRejectedValue(new Error('Config error'));

            client = new LocalClient();
            await new Promise((resolve) => setTimeout(resolve, 20));

            expect(mockLoggerInstance.error).toHaveBeenCalledWith(
                expect.stringContaining('Failed to load config'),
                'Config error'
            );
        });
    });

    describe('sendRequest', () => {
        it('should fail if all clients disabled', async () => {
            getSettings.mockResolvedValue({
                llm: {
                    local: { enabled: false },
                    vllm: { enabled: false },
                },
            });

            client = new LocalClient();
            const result = await client.sendRequest({ prompt: 'test' });

            expect(result.success).toBe(false);
            expect(result.error).toBe('All local clients disabled');
        });

        it('should route to vLLM first if enabled and successful', async () => {
            getSettings.mockResolvedValue({
                llm: {
                    local: { enabled: true },
                    vllm: { enabled: true },
                },
            });

            const mockVllmInstance = {
                sendRequest: vi
                    .fn()
                    .mockResolvedValue({ success: true, content: 'vllm response', metadata: {} }),
                resetStats: vi.fn(),
            };
            VLLMClient.mockImplementation(function () {
                return mockVllmInstance;
            });

            client = new LocalClient();

            // sendRequest awaits _loadConfig internally
            const result = await client.sendRequest({ prompt: 'test' });

            expect(VLLMClient).toHaveBeenCalled();
            expect(mockVllmInstance.sendRequest).toHaveBeenCalledWith({ prompt: 'test' });
            expect(result.success).toBe(true);
            expect(result.content).toBe('vllm response');
            expect(result.metadata.routedTo).toBe('vllm');
            expect(client.stats.vllmRequests).toBe(1);
            expect(client.stats.successfulRequests).toBe(1);
        });

        it('should fallback to Ollama if vLLM fails', async () => {
            getSettings.mockResolvedValue({
                llm: {
                    local: { enabled: true },
                    vllm: { enabled: true },
                },
            });

            const mockVllmInstance = {
                sendRequest: vi.fn().mockResolvedValue({ success: false, error: 'vllm error' }),
                resetStats: vi.fn(),
            };
            VLLMClient.mockImplementation(function () {
                return mockVllmInstance;
            });

            const mockOllamaInstance = {
                initialize: vi.fn(),
                generate: vi
                    .fn()
                    .mockResolvedValue({ success: true, content: 'ollama response', metadata: {} }),
                resetStats: vi.fn(),
            };
            OllamaClient.mockImplementation(function () {
                return mockOllamaInstance;
            });

            client = new LocalClient();
            const result = await client.sendRequest({ prompt: 'test' });

            expect(mockVllmInstance.sendRequest).toHaveBeenCalled();
            expect(mockLoggerInstance.warn).toHaveBeenCalledWith(
                expect.stringContaining('vLLM failed')
            );
            expect(mockOllamaInstance.generate).toHaveBeenCalledWith({ prompt: 'test' });
            expect(result.success).toBe(true);
            expect(result.content).toBe('ollama response');
            expect(result.metadata.routedTo).toBe('ollama');
        });

        it('should fallback to Ollama if vLLM throws exception', async () => {
            getSettings.mockResolvedValue({
                llm: {
                    local: { enabled: true },
                    vllm: { enabled: true },
                },
            });

            const mockVllmInstance = {
                sendRequest: vi.fn().mockRejectedValue(new Error('vLLM exception')),
                resetStats: vi.fn(),
            };
            VLLMClient.mockImplementation(function () {
                return mockVllmInstance;
            });

            const mockOllamaInstance = {
                initialize: vi.fn(),
                generate: vi
                    .fn()
                    .mockResolvedValue({ success: true, content: 'ollama response', metadata: {} }),
                resetStats: vi.fn(),
            };
            OllamaClient.mockImplementation(function () {
                return mockOllamaInstance;
            });

            client = new LocalClient();
            const result = await client.sendRequest({ prompt: 'test' });

            expect(mockLoggerInstance.warn).toHaveBeenCalledWith(
                expect.stringContaining('vLLM exception')
            );
            expect(mockOllamaInstance.generate).toHaveBeenCalled();
            expect(result.success).toBe(true);
        });

        it('should fail if both vLLM and Ollama fail', async () => {
            getSettings.mockResolvedValue({
                llm: {
                    local: { enabled: true },
                    vllm: { enabled: true },
                },
            });

            const mockVllmInstance = {
                sendRequest: vi.fn().mockResolvedValue({ success: false, error: 'vllm error' }),
                resetStats: vi.fn(),
            };
            VLLMClient.mockImplementation(function () {
                return mockVllmInstance;
            });

            const mockOllamaInstance = {
                initialize: vi.fn(),
                generate: vi.fn().mockResolvedValue({ success: false, error: 'ollama error' }),
                resetStats: vi.fn(),
            };
            OllamaClient.mockImplementation(function () {
                return mockOllamaInstance;
            });

            client = new LocalClient();
            const result = await client.sendRequest({ prompt: 'test' });

            expect(result.success).toBe(false);
            expect(result.error).toBe('ollama error');
            expect(client.stats.failedRequests).toBe(1);
            expect(mockLoggerInstance.error).toHaveBeenCalledWith(
                expect.stringContaining('All local providers failed')
            );
        });

        it('should fail if Ollama throws exception', async () => {
            getSettings.mockResolvedValue({
                llm: {
                    local: { enabled: true },
                    vllm: { enabled: false },
                },
            });

            const mockOllamaInstance = {
                initialize: vi.fn(),
                generate: vi.fn().mockRejectedValue(new Error('Ollama exception')),
                resetStats: vi.fn(),
            };
            OllamaClient.mockImplementation(function () {
                return mockOllamaInstance;
            });

            client = new LocalClient();
            const result = await client.sendRequest({ prompt: 'test' });

            expect(result.success).toBe(false);
            expect(mockLoggerInstance.warn).toHaveBeenCalledWith(
                expect.stringContaining('Ollama exception')
            );
            expect(mockLoggerInstance.error).toHaveBeenCalledWith(
                expect.stringContaining('All local providers failed')
            );
        });
    });

    describe('getStats', () => {
        it('should return stats', () => {
            client = new LocalClient();
            const stats = client.getStats();
            expect(stats).toHaveProperty('vllmRequests');
            expect(stats).toHaveProperty('ollamaRequests');
        });
    });

    describe('resetStats', () => {
        it('should reset internal and client stats', async () => {
            getSettings.mockResolvedValue({
                llm: {
                    local: { enabled: true },
                    vllm: { enabled: true },
                },
            });

            const mockVllmInstance = {
                sendRequest: vi.fn(),
                resetStats: vi.fn(),
            };
            VLLMClient.mockImplementation(function () {
                return mockVllmInstance;
            });

            const mockOllamaInstance = {
                initialize: vi.fn(),
                generate: vi.fn(),
                resetStats: vi.fn(),
            };
            OllamaClient.mockImplementation(function () {
                return mockOllamaInstance;
            });

            client = new LocalClient();
            // Wait for init
            await new Promise((resolve) => setTimeout(resolve, 20));

            client.stats.vllmRequests = 5;
            client.resetStats();

            expect(client.stats.vllmRequests).toBe(0);
            expect(mockVllmInstance.resetStats).toHaveBeenCalled();
            expect(mockOllamaInstance.resetStats).toHaveBeenCalled();
        });
    });
});
