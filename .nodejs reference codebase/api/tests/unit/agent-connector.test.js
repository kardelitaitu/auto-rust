/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import AgentConnector from '@api/core/agent-connector.js';
import { getSettings } from '@api/utils/configLoader.js';

// Hoist mocks
const mocks = vi.hoisted(() => {
    return {
        logger: {
            info: vi.fn(),
            warn: vi.fn(),
            error: vi.fn(),
            debug: vi.fn(),
            success: vi.fn(),
        },
    };
});

// Mock dependencies
vi.mock('@api/core/logger.js', () => ({
    createLogger: () => mocks.logger,
}));

vi.mock('@api/core/local-client.js', () => {
    return {
        default: vi.fn(function () {
            return {
                processRequest: vi.fn().mockResolvedValue({
                    success: true,
                    content: 'local response',
                    metadata: { routedTo: 'local' },
                }),
                sendRequest: vi.fn().mockResolvedValue({
                    success: true,
                    content: 'local response',
                    metadata: { routedTo: 'local' },
                }),
                getStats: vi.fn().mockReturnValue({}),
            };
        }),
    };
});

vi.mock('@api/core/cloud-client.js', () => {
    return {
        default: vi.fn(function () {
            return {
                processRequest: vi.fn().mockResolvedValue({
                    success: true,
                    content: 'cloud response',
                    metadata: { routedTo: 'cloud' },
                }),
                sendRequest: vi.fn().mockResolvedValue({
                    success: true,
                    content: 'cloud response',
                    metadata: { routedTo: 'cloud' },
                }),
                getStats: vi.fn().mockReturnValue({}),
                isReady: vi.fn().mockReturnValue(true),
            };
        }),
    };
});

vi.mock('@api/core/vision-interpreter.js', () => {
    return {
        default: vi.fn(function () {
            return {
                analyze: vi.fn().mockResolvedValue({
                    success: true,
                    content: 'vision response',
                    metadata: { routedTo: 'vision' },
                }),
                buildPrompt: vi.fn().mockReturnValue('vision prompt'),
                parseResponse: vi.fn().mockReturnValue({ success: true, data: {} }),
                getStats: vi.fn().mockReturnValue({}),
            };
        }),
    };
});

vi.mock('@api/core/request-queue.js', () => {
    return {
        default: vi.fn(function () {
            return {
                enqueue: vi.fn().mockImplementation(async (fn) => {
                    const result = await fn();
                    return { success: true, data: result };
                }),
                getStats: vi.fn().mockReturnValue({ running: 0, queued: 0 }),
            };
        }),
    };
});

vi.mock('@api/core/circuit-breaker.js', () => {
    return {
        default: vi.fn(function () {
            return {
                execute: vi.fn().mockImplementation(async (id, fn) => {
                    return await fn();
                }),
                getStats: vi.fn().mockReturnValue({}),
                getAllStatus: vi.fn().mockReturnValue({}),
            };
        }),
    };
});

vi.mock('@api/utils/configLoader.js', () => ({
    getSettings: vi.fn().mockResolvedValue({
        llm: {
            local: { enabled: true },
            cloud: { enabled: true },
        },
    }),
}));

describe('AgentConnector', () => {
    let connector;
    let localClientMock;
    let cloudClientMock;
    let visionInterpreterMock;
    let requestQueueMock;
    let circuitBreakerMock;

    beforeEach(async () => {
        vi.clearAllMocks();
        vi.mocked(getSettings).mockResolvedValue({
            llm: {
                local: { enabled: true },
                vllm: { enabled: false },
                cloud: { enabled: true },
            },
        });

        // Re-import to get fresh mocks if needed (but class is instantiated in beforeEach)
        const _LocalClient = (await import('@api/core/local-client.js')).default;
        const _CloudClient = (await import('@api/core/cloud-client.js')).default;
        const _VisionInterpreter = (await import('@api/core/vision-interpreter.js')).default;
        const _RequestQueue = (await import('@api/core/request-queue.js')).default;
        const _CircuitBreaker = (await import('@api/core/circuit-breaker.js')).default;

        connector = new AgentConnector();

        // Get mock instances from the connector properties
        localClientMock = connector.localClient;
        cloudClientMock = connector.cloudClient;
        visionInterpreterMock = connector.visionInterpreter;
        requestQueueMock = connector.requestQueue;
        circuitBreakerMock = connector.circuitBreaker;
    });

    describe('Initialization', () => {
        it('should initialize all dependencies', () => {
            expect(connector.localClient).toBeDefined();
            expect(connector.cloudClient).toBeDefined();
            expect(connector.visionInterpreter).toBeDefined();
            expect(connector.requestQueue).toBeDefined();
            expect(connector.circuitBreaker).toBeDefined();
            expect(connector.stats).toEqual(
                expect.objectContaining({
                    totalRequests: 0,
                    successfulRequests: 0,
                    failedRequests: 0,
                })
            );
        });
    });

    describe('processRequest', () => {
        it('should route basic request through queue and circuit breaker', async () => {
            const request = {
                action: 'generate_reply',
                payload: { text: 'hello' },
                sessionId: 'test-session',
            };

            const result = await connector.processRequest(request);

            expect(requestQueueMock.enqueue).toHaveBeenCalled();
            expect(circuitBreakerMock.execute).toHaveBeenCalled();
            expect(result.success).toBe(true);
            expect(connector.stats.totalRequests).toBe(1);
            expect(connector.stats.successfulRequests).toBe(1);
        });

        it('should handle vision requests', async () => {
            const request = {
                action: 'analyze_page_with_vision',
                payload: { vision: true },
                sessionId: 'test-session',
            };

            await connector.processRequest(request);

            expect(connector.stats.visionRequests).toBe(1);
            // Vision interpreter should be called inside _executeWithCircuitBreaker -> logic
            // But we need to verify routing logic in _executeWithCircuitBreaker tests or check calls here
        });

        it('should update stats on failure', async () => {
            requestQueueMock.enqueue.mockRejectedValue(new Error('Queue Error'));

            const request = {
                action: 'fail',
                payload: {},
                sessionId: 'test-session',
            };

            await expect(connector.processRequest(request)).rejects.toThrow('Queue Error');

            expect(connector.stats.failedRequests).toBe(1);
        });
    });

    describe('Routing Logic', () => {
        it('should handle direct result from request queue', async () => {
            const request = {
                action: 'generate_reply',
                payload: { text: 'hello' },
                sessionId: 'test-session',
            };

            // Mock enqueue returning result directly (not wrapped in data)
            requestQueueMock.enqueue.mockResolvedValue({
                success: true,
                content: 'direct',
                metadata: { routedTo: 'local' },
            });

            const result = await connector.processRequest(request);

            expect(result.content).toBe('direct');
            expect(result.success).toBe(true);
        });

        it('should route vision action to handleVisionRequest', async () => {
            const request = {
                action: 'analyze_page_with_vision',
                payload: { goal: 'test', semanticTree: {}, vision: 'base64' },
                sessionId: 'test-session',
            };

            // Spy on handleVisionRequest
            const spy = vi.spyOn(connector, 'handleVisionRequest');

            // Mock VisionInterpreter behavior
            visionInterpreterMock.buildPrompt.mockReturnValue('vision prompt');
            localClientMock.sendRequest.mockResolvedValue({
                success: true,
                content: '{"action":"click"}',
                metadata: { model: 'local-vision' },
            });
            visionInterpreterMock.parseResponse.mockReturnValue({
                success: true,
                data: { action: 'click' },
            });

            await connector.processRequest(request);

            expect(spy).toHaveBeenCalledWith(request);
            expect(visionInterpreterMock.buildPrompt).toHaveBeenCalled();
            expect(localClientMock.sendRequest).toHaveBeenCalled(); // via _sendToLocal
            expect(visionInterpreterMock.parseResponse).toHaveBeenCalled();
        });

        it('should route generate_reply to handleGenerateReply', async () => {
            const request = {
                action: 'generate_reply',
                payload: { prompt: 'test' },
                sessionId: 'test-session',
            };

            const spy = vi.spyOn(connector, 'handleGenerateReply');

            await connector.processRequest(request);

            expect(spy).toHaveBeenCalledWith(request);
        });

        it('should route unknown action to cloud', async () => {
            const request = {
                action: 'unknown_action',
                payload: { prompt: 'test' },
                sessionId: 'test-session',
            };

            const spy = vi.spyOn(connector, '_sendToCloud');

            await connector.processRequest(request);

            expect(spy).toHaveBeenCalled();
            expect(cloudClientMock.sendRequest).toHaveBeenCalled();
        });
    });

    describe('handleGenerateReply', () => {
        it('should use local provider if enabled', async () => {
            // ConfigLoader mock returns local.enabled=true by default
            const request = {
                action: 'generate_reply',
                payload: { systemPrompt: 'sys', userPrompt: 'user' },
                sessionId: 'test-session',
            };

            localClientMock.sendRequest.mockResolvedValue({ success: true, content: 'reply' });

            const result = await connector.handleGenerateReply(request);

            expect(result.success).toBe(true);
            expect(localClientMock.sendRequest).toHaveBeenCalled();
            expect(cloudClientMock.sendRequest).not.toHaveBeenCalled();
            expect(result.metadata.routedTo).toBe('local');
        });

        it('should fallback to cloud if local providers disabled', async () => {
            vi.mocked(getSettings).mockResolvedValueOnce({
                llm: { local: { enabled: false }, vllm: { enabled: false } },
            });

            const request = {
                action: 'generate_reply',
                payload: { systemPrompt: 'sys', userPrompt: 'user' },
                sessionId: 'test-session',
            };

            await connector.handleGenerateReply(request);

            expect(cloudClientMock.sendRequest).toHaveBeenCalled();
            expect(localClientMock.sendRequest).not.toHaveBeenCalled();
        });

        it('should fallback to cloud if local fails', async () => {
            localClientMock.sendRequest.mockResolvedValue({ success: false, error: 'local error' });
            cloudClientMock.sendRequest.mockResolvedValue({
                success: true,
                content: 'cloud reply',
            });

            const request = {
                action: 'generate_reply',
                payload: { systemPrompt: 'sys', userPrompt: 'user' },
                sessionId: 'test-session',
            };

            const result = await connector.handleGenerateReply(request);

            expect(result.success).toBe(true);
            expect(localClientMock.sendRequest).toHaveBeenCalled();
            expect(cloudClientMock.sendRequest).toHaveBeenCalled();
            expect(result.metadata.fallbackFromLocal).toBe(true);
        });

        it('should handle vision timeout and fallback to text-only local', async () => {
            const request = {
                action: 'generate_reply',
                payload: {
                    systemPrompt: 'sys',
                    userPrompt: 'user',
                    vision: 'base64',
                    context: { hasScreenshot: true },
                },
                sessionId: 'test-session',
            };

            // First call (Vision) fails with timeout
            localClientMock.sendRequest
                .mockResolvedValueOnce({ success: false, error: 'Request timed out' })
                // Second call (Text-only) succeeds
                .mockResolvedValueOnce({ success: true, content: 'text reply' });

            const result = await connector.handleGenerateReply(request);

            expect(result.success).toBe(true);
            expect(result.metadata.visionEnabled).toBe(false);
            expect(result.metadata.fallbackFromVision).toBe(true);
            expect(localClientMock.sendRequest).toHaveBeenCalledTimes(2);
        });

        it('should handle all providers failing', async () => {
            localClientMock.sendRequest.mockResolvedValue({ success: false, error: 'local fail' });
            cloudClientMock.sendRequest.mockResolvedValue({ success: false, error: 'cloud fail' });

            const request = {
                action: 'generate_reply',
                payload: { systemPrompt: 'sys', userPrompt: 'user' },
                sessionId: 'test-session',
            };

            const result = await connector.handleGenerateReply(request);

            expect(result.success).toBe(false);
            expect(result.error).toBe('cloud fail');
        });

        it('should handle settings load failure', async () => {
            vi.mocked(getSettings).mockRejectedValueOnce(new Error('Config error'));

            const request = {
                action: 'generate_reply',
                payload: { systemPrompt: 'sys', userPrompt: 'user' },
                sessionId: 'test-session',
            };

            // Should default to cloud if settings fail
            await connector.handleGenerateReply(request);
            expect(cloudClientMock.sendRequest).toHaveBeenCalled();
        });

        it('should use vLLM if enabled', async () => {
            const { getSettings } = await import('@api/utils/configLoader.js');
            vi.mocked(getSettings).mockResolvedValueOnce({
                llm: { vllm: { enabled: true }, local: { enabled: false } },
            });

            const request = {
                action: 'generate_reply',
                payload: { systemPrompt: 'sys', userPrompt: 'user' },
                sessionId: 'test-session',
            };

            await connector.handleGenerateReply(request);
            expect(localClientMock.sendRequest).toHaveBeenCalled();
        });
        it('should handle exception during cloud fallback', async () => {
            localClientMock.sendRequest.mockResolvedValue({ success: false, error: 'local fail' });
            cloudClientMock.sendRequest.mockRejectedValue(new Error('Cloud crash'));

            const request = {
                action: 'generate_reply',
                payload: { systemPrompt: 'sys', userPrompt: 'user' },
                sessionId: 'test-session',
            };

            const result = await connector.handleGenerateReply(request);

            expect(result.success).toBe(false);
            expect(result.error).toBe('Cloud crash');
        });

        it('should handle successful vision request', async () => {
            const request = {
                action: 'generate_reply',
                payload: {
                    systemPrompt: 'sys',
                    userPrompt: 'user',
                    vision: 'base64',
                    context: { hasScreenshot: true },
                },
                sessionId: 'test-session',
            };

            localClientMock.sendRequest.mockResolvedValue({
                success: true,
                content: 'vision reply',
                metadata: { routedTo: 'local-vision', model: 'llava' },
            });

            const result = await connector.handleGenerateReply(request);

            expect(result.success).toBe(true);
            expect(result.metadata.visionEnabled).toBe(true);
            expect(result.content).toBe('vision reply');
        });

        it('should handle non-timeout vision failure', async () => {
            const request = {
                action: 'generate_reply',
                payload: {
                    systemPrompt: 'sys',
                    userPrompt: 'user',
                    vision: 'base64',
                    context: { hasScreenshot: true },
                },
                sessionId: 'test-session',
            };

            localClientMock.sendRequest
                .mockResolvedValueOnce({ success: false, error: 'Bad request' }) // Non-timeout error
                .mockResolvedValueOnce({ success: true, content: 'text reply' });

            const result = await connector.handleGenerateReply(request);

            expect(result.success).toBe(true);
            expect(result.metadata.fallbackFromVision).toBe(true);
            expect(localClientMock.sendRequest).toHaveBeenCalledTimes(2);
        });

        it('should handle vision exception', async () => {
            const request = {
                action: 'generate_reply',
                payload: {
                    systemPrompt: 'sys',
                    userPrompt: 'user',
                    vision: 'base64',
                    context: { hasScreenshot: true },
                },
                sessionId: 'test-session',
            };

            // Vision throws exception
            localClientMock.sendRequest
                .mockRejectedValueOnce(new Error('Vision crash'))
                .mockResolvedValueOnce({ success: true, content: 'text reply' });

            const result = await connector.handleGenerateReply(request);

            expect(result.success).toBe(true);
            expect(result.metadata.fallbackFromVision).toBe(true);
            expect(localClientMock.sendRequest).toHaveBeenCalledTimes(2);
        });

        it('should handle local text exception', async () => {
            localClientMock.sendRequest.mockRejectedValue(new Error('Local crash'));
            cloudClientMock.sendRequest.mockResolvedValue({
                success: true,
                content: 'cloud reply',
            });

            const request = {
                action: 'generate_reply',
                payload: { systemPrompt: 'sys', userPrompt: 'user' },
                sessionId: 'test-session',
            };

            const result = await connector.handleGenerateReply(request);

            expect(result.success).toBe(true);
            expect(result.metadata.routedTo).toBe('cloud');
            expect(result.metadata.fallbackFromLocal).toBe(true);
        });

        it('should handle undefined metadata in vision response', async () => {
            const request = {
                action: 'generate_reply',
                payload: {
                    systemPrompt: 'sys',
                    userPrompt: 'user',
                    vision: 'base64',
                    context: { hasScreenshot: true },
                },
                sessionId: 'test-session',
            };

            localClientMock.sendRequest.mockResolvedValue({
                success: true,
                content: 'vision reply',
                // No metadata
            });

            const result = await connector.handleGenerateReply(request);
            expect(result.metadata.routedTo).toBe('local');
        });

        it('should handle all providers failing with vision context', async () => {
            localClientMock.sendRequest.mockResolvedValue({ success: false, error: 'local fail' });
            cloudClientMock.sendRequest.mockResolvedValue({ success: false, error: 'cloud fail' });

            const request = {
                action: 'generate_reply',
                payload: {
                    systemPrompt: 'sys',
                    userPrompt: 'user',
                    vision: 'base64',
                    context: { hasScreenshot: true },
                },
                sessionId: 'test-session',
            };

            const result = await connector.handleGenerateReply(request);

            expect(result.success).toBe(false);
            expect(result.metadata.fallbackFromVision).toBe(true);
        });
    });

    describe('Direct Cloud Routing', () => {
        it('should handle cloud failure response when routing unknown action', async () => {
            const request = { action: 'unknown_action', payload: {}, sessionId: 'test' };
            cloudClientMock.sendRequest.mockResolvedValue({ success: false, error: 'cloud error' });

            const result = await connector.processRequest(request);
            expect(result.success).toBe(false);
            expect(result.error).toBe('cloud error');
        });

        it('should handle missing error message in cloud failure', async () => {
            const request = { action: 'unknown_action', payload: {}, sessionId: 'test' };
            cloudClientMock.sendRequest.mockResolvedValue({ success: false }); // No error message

            const result = await connector.processRequest(request);
            expect(result.error).toBe('Unknown cloud error');
        });

        it('should handle cloud exception when routing unknown action', async () => {
            const request = { action: 'unknown_action', payload: {}, sessionId: 'test' };
            cloudClientMock.sendRequest.mockRejectedValue(new Error('Network error'));

            const result = await connector.processRequest(request);
            expect(result.success).toBe(false);
            expect(result.error).toBe('Network error');
        });
    });

    describe('Coverage Edge Cases', () => {
        it('should default routedTo to local if metadata missing', async () => {
            const request = {
                action: 'generate_reply',
                payload: {
                    systemPrompt: 'sys',
                    userPrompt: 'user',
                    vision: 'base64',
                    context: { hasScreenshot: true },
                },
                sessionId: 'test-session',
            };

            localClientMock.sendRequest.mockResolvedValue({
                success: true,
                content: 'vision reply',
                metadata: {}, // No routedTo
            });

            const result = await connector.handleGenerateReply(request);
            expect(result.metadata.routedTo).toBe('local');
        });

        it('should default to Unknown vision error if error missing', async () => {
            const request = {
                action: 'generate_reply',
                payload: {
                    systemPrompt: 'sys',
                    userPrompt: 'user',
                    vision: 'base64',
                    context: { hasScreenshot: true },
                },
                sessionId: 'test-session',
            };

            localClientMock.sendRequest
                .mockResolvedValueOnce({ success: false }) // No error message
                .mockResolvedValueOnce({ success: true, content: 'text' });

            await connector.handleGenerateReply(request);
            expect(localClientMock.sendRequest).toHaveBeenCalledTimes(2);
        });

        it('should default to Unknown cloud error if error missing', async () => {
            localClientMock.sendRequest.mockResolvedValue({ success: false, error: 'local fail' });
            cloudClientMock.sendRequest.mockResolvedValue({ success: false }); // No error message

            const request = {
                action: 'generate_reply',
                payload: { systemPrompt: 'sys', userPrompt: 'user' },
                sessionId: 'test-session',
            };

            const result = await connector.handleGenerateReply(request);
            expect(result.error).toBe('Unknown cloud error');
        });

        it('should handle vision parsing failure', async () => {
            const request = {
                action: 'analyze_page_with_vision',
                payload: { goal: 'test', semanticTree: {}, vision: 'base64' },
                sessionId: 'test-session',
            };

            // Mock VisionInterpreter to return success: false
            visionInterpreterMock.parseResponse.mockReturnValue({
                success: false,
                error: 'Parse error',
            });

            // Also need to ensure local client returns success so parsing is attempted
            localClientMock.sendRequest.mockResolvedValue({
                success: true,
                content: 'vision raw content',
            });

            const result = await connector.handleVisionRequest(request);

            expect(result.success).toBe(true);
            expect(result.data).toBeNull();
            expect(result.metadata.parsedSuccessfully).toBe(false);
        });
    });

    describe('Health Monitoring', () => {
        it('should report degraded status when queue is not empty', () => {
            requestQueueMock.getStats.mockReturnValue({ running: 2, queued: 1 });
            circuitBreakerMock.getAllStatus.mockReturnValue({});

            // Need to populate stats with some requests
            connector.stats.totalRequests = 10;
            connector.stats.successfulRequests = 10;

            const health = connector.getHealth();
            expect(health.checks.queue.status).toBe('degraded');
        });

        it('should report degraded status when success rate is low', () => {
            requestQueueMock.getStats.mockReturnValue({ running: 0, queued: 0 });
            circuitBreakerMock.getAllStatus.mockReturnValue({});

            connector.stats.totalRequests = 10;
            connector.stats.successfulRequests = 5; // 50% success rate

            const health = connector.getHealth();
            expect(health.checks.requests.status).toBe('degraded');
        });

        it('should handle HALF_OPEN circuit breaker state', () => {
            requestQueueMock.getStats.mockReturnValue({ running: 0, queued: 0 });
            circuitBreakerMock.getAllStatus.mockReturnValue({
                'test-model': { state: 'HALF_OPEN' },
            });

            connector.stats.totalRequests = 10;
            connector.stats.successfulRequests = 10;

            const health = connector.getHealth();
            expect(health.checks.circuitBreaker.status).toBe('recovering');
        });

        it('should handle OPEN circuit breaker state', () => {
            requestQueueMock.getStats.mockReturnValue({ running: 0, queued: 0 });
            circuitBreakerMock.getAllStatus.mockReturnValue({
                'test-model': { state: 'OPEN' },
            });

            connector.stats.totalRequests = 10;
            connector.stats.successfulRequests = 10;

            const health = connector.getHealth();
            expect(health.checks.circuitBreaker.status).toBe('degraded');
        });

        it('should log health report', () => {
            const spy = vi.spyOn(console, 'log').mockImplementation(() => {});
            connector.logHealth();
            expect(spy).toHaveBeenCalled();
            spy.mockRestore();
        });
    });
});
