/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

const mocked = vi.hoisted(() => ({
    processRequest: vi.fn(),
    logger: {
        info: vi.fn(),
        success: vi.fn(),
        error: vi.fn(),
    },
}));

vi.mock('@api/core/agent-connector.js', () => ({
    default: class {
        constructor() {
            this.processRequest = mocked.processRequest;
        }
    },
}));

vi.mock('@api/core/logger.js', () => ({
    createLogger: vi.fn(() => mocked.logger),
}));

vi.mock('@api/utils/math.js', () => ({
    mathUtils: {
        randomInRange: vi.fn(() => 1234),
    },
}));

describe('AITools', () => {
    let AITools;
    let tools;

    beforeEach(async () => {
        vi.resetModules();
        vi.clearAllMocks();
        const module = await import('../../utils/ai-tools.js');
        AITools = module.AITools;
        tools = new AITools({ timeout: 50 });
    });

    afterEach(() => {
        vi.useRealTimers();
    });

    it('initializes with config and stats', () => {
        expect(tools.config.timeout).toBe(50);
        expect(tools.stats.totalRequests).toBe(0);
        expect(mocked.logger.info).toHaveBeenCalledWith('[AITools] Initialized');
    });

    it('defaults timeout when not provided', () => {
        const fresh = new AITools();
        expect(fresh.config.timeout).toBe(30000);
    });

    it('queues requests through processRequest', async () => {
        const spy = vi.spyOn(tools, 'processRequest').mockResolvedValue({ success: true });
        const result = await tools.queueRequest({ action: 'ping', payload: {} });
        expect(spy).toHaveBeenCalled();
        expect(result.success).toBe(true);
    });

    it('processes requests successfully', async () => {
        vi.useFakeTimers();
        vi.setSystemTime(new Date('2026-02-14T00:00:00Z'));
        mocked.processRequest.mockResolvedValue({
            content: 'ok',
            metadata: { routedTo: 'cloud', model: 'gpt' },
        });

        const result = await tools.processRequest({ action: 'act', payload: { foo: 'bar' } });

        expect(result.success).toBe(true);
        expect(result.data).toBe('ok');
        expect(result.raw).toBe('ok');
        expect(result.metadata.provider).toBe('cloud');
        expect(result.metadata.model).toBe('gpt');
        expect(tools.stats.totalRequests).toBe(1);
        expect(tools.stats.successfulRequests).toBe(1);

        const callArg = mocked.processRequest.mock.calls[0][0];
        expect(callArg.action).toBe('act');
        expect(callArg.payload.foo).toBe('bar');
        expect(callArg.payload.sessionId).toBe(callArg.sessionId);
    });

    it('prefers response data over content', async () => {
        mocked.processRequest.mockResolvedValue({
            data: { ok: true },
            content: 'fallback',
        });
        const result = await tools.processRequest({ action: 'act', payload: {} });
        expect(result.data).toEqual({ ok: true });
    });

    it('handles request failures', async () => {
        mocked.processRequest.mockRejectedValue(new Error('fail'));
        const result = await tools.processRequest({ action: 'act', payload: {} });
        expect(result.success).toBe(false);
        expect(result.error).toBe('fail');
        expect(tools.stats.failedRequests).toBe(1);
    });

    it('times out when request exceeds timeout', async () => {
        mocked.processRequest.mockImplementation(() => new Promise(() => {}));
        vi.useFakeTimers();
        const promise = tools.sendWithTimeout({ action: 'act', payload: {} });
        promise.catch(() => {});
        await vi.advanceTimersByTimeAsync(51);
        await expect(promise).rejects.toThrow('Request timeout after 50ms');
    });

    it('clears timeout on success', async () => {
        mocked.processRequest.mockResolvedValue({ content: 'ok' });
        const response = await tools.sendWithTimeout({ action: 'act', payload: {} });
        expect(response.content).toBe('ok');
    });

    it('skips clearTimeout when timeout id is undefined', async () => {
        const setTimeoutSpy = vi.spyOn(global, 'setTimeout').mockImplementation(() => undefined);
        mocked.processRequest.mockResolvedValue({ content: 'ok' });
        const response = await tools.sendWithTimeout({ action: 'act', payload: {} });
        setTimeoutSpy.mockRestore();
        expect(response.content).toBe('ok');
    });

    it('builds generateReply requests', async () => {
        const spy = vi.spyOn(tools, 'queueRequest').mockResolvedValue({ success: true });
        await tools.generateReply('Hello', 'user', {
            systemPrompt: 'sys',
            maxTokens: 10,
            temperature: 0.1,
        });

        const callArg = spy.mock.calls[0][0];
        expect(callArg.action).toBe('generate_reply');
        expect(callArg.payload.systemPrompt).toBe('sys');
        expect(callArg.payload.userPrompt).toContain('Tweet from @user');
        expect(callArg.payload.maxTokens).toBe(10);
        expect(callArg.payload.temperature).toBe(0.1);
    });

    it('uses default prompt when none provided', async () => {
        const spy = vi.spyOn(tools, 'queueRequest').mockResolvedValue({ success: true });
        await tools.generateReply('Hello', 'user');
        const callArg = spy.mock.calls[0][0];
        expect(callArg.payload.systemPrompt).toContain('neutral, casual Twitter user');
    });

    it('builds analyzeTweet requests with context', async () => {
        vi.useFakeTimers();
        vi.setSystemTime(new Date('2026-02-14T00:00:00Z'));
        const spy = vi.spyOn(tools, 'queueRequest').mockResolvedValue({ success: true });
        await tools.analyzeTweet('Hello', 'user');
        const callArg = spy.mock.calls[0][0];
        expect(callArg.action).toBe('analyze_tweet');
        expect(callArg.payload.context.textLength).toBe(5);
        expect(callArg.payload.context.timestamp).toBe(Date.now());
    });

    it('builds classifyTweet requests', async () => {
        const spy = vi.spyOn(tools, 'queueRequest').mockResolvedValue({ success: true });
        await tools.classifyTweet('Hello');
        const callArg = spy.mock.calls[0][0];
        expect(callArg.action).toBe('classify_content');
        expect(callArg.payload.categories).toContain('spam');
    });

    it('builds conversation reply requests with defaults', async () => {
        const spy = vi.spyOn(tools, 'queueRequest').mockResolvedValue({ success: true });
        await tools.generateConversationReply([{ role: 'user', content: 'hi' }]);
        const callArg = spy.mock.calls[0][0];
        expect(callArg.action).toBe('generate_conversation');
        expect(callArg.payload.systemPrompt).toContain('friendly conversationalist');
    });

    it('generates stable session id format', () => {
        vi.useFakeTimers();
        vi.setSystemTime(new Date('2026-02-14T00:00:00Z'));
        const id = tools.getSessionId();
        expect(id).toBe(`session_${Date.now()}_1234`);
    });

    it('updates average response time', () => {
        tools.stats.successfulRequests = 1;
        tools.updateAvgResponseTime(10);
        expect(tools.stats.avgResponseTime).toBe(10);
        tools.stats.successfulRequests = 2;
        tools.updateAvgResponseTime(30);
        expect(tools.stats.avgResponseTime).toBe(20);
    });

    it('returns formatted stats', () => {
        tools.stats.totalRequests = 2;
        tools.stats.successfulRequests = 1;
        tools.stats.avgResponseTime = 12.4;
        const stats = tools.getStats();
        expect(stats.successRate).toBe('50.0%');
        expect(stats.avgResponseTime).toBe('12ms');
    });

    it('returns zero success rate when no requests', () => {
        tools.stats.totalRequests = 0;
        tools.stats.successfulRequests = 0;
        const stats = tools.getStats();
        expect(stats.successRate).toBe('0%');
    });

    it('updates configuration', () => {
        tools.updateConfig({ timeout: 100 });
        expect(tools.config.timeout).toBe(100);
    });

    it('ignores undefined timeout updates', () => {
        tools.updateConfig({});
        expect(tools.config.timeout).toBe(50);
    });

    it('reports health status', async () => {
        const ok = await tools.isHealthy();
        expect(ok).toBe(true);
        Object.defineProperty(tools, 'connector', {
            get() {
                throw new Error('boom');
            },
        });
        const bad = await tools.isHealthy();
        expect(bad).toBe(false);
    });

    it('resets statistics', () => {
        tools.stats.totalRequests = 5;
        tools.resetStats();
        expect(tools.stats.totalRequests).toBe(0);
        expect(tools.stats.successfulRequests).toBe(0);
    });
});
