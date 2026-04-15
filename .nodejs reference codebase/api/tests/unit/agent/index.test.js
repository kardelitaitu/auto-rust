/**
 * Auto-AI Framework - Agent Index Tests
 * Tests for api/agent/index.js exports
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';

// Mock all dependencies
vi.mock('@api/agent/observer.js', () => ({
    see: vi.fn(),
}));

vi.mock('@api/agent/executor.js', () => ({
    doAction: vi.fn(),
}));

vi.mock('@api/agent/finder.js', () => ({
    find: vi.fn(),
}));

vi.mock('@api/agent/vision.js', () => ({
    screenshot: vi.fn(),
    buildPrompt: vi.fn(),
    parseResponse: vi.fn(),
    captureAXTree: vi.fn(),
    captureState: vi.fn(),
}));

vi.mock('@api/agent/actionEngine.js', () => ({
    actionEngine: {
        execute: vi.fn(),
    },
}));

vi.mock('@api/agent/llmClient.js', () => ({
    llmClient: vi.fn(),
}));

vi.mock('@api/agent/runner.js', () => ({
    agentRunner: {
        run: vi.fn(),
        stop: vi.fn(),
    },
}));

vi.mock('@api/agent/tokenCounter.js', () => ({
    estimateTokens: vi.fn(),
    estimateMessageTokens: vi.fn(),
    estimateConversationTokens: vi.fn(),
}));

describe('api/agent/index.js - Check Imports/Context', () => {
    beforeEach(() => {
        vi.clearAllMocks();
    });

    it('should export see function from observer', async () => {
        const { see } = await import('@api/agent/index.js');
        expect(see).toBeDefined();
        expect(typeof see).toBe('function');
    });

    it('should export do function from executor', async () => {
        const { do: doAction } = await import('@api/agent/index.js');
        expect(doAction).toBeDefined();
        expect(typeof doAction).toBe('function');
    });

    it('should export find function from finder', async () => {
        const { find } = await import('@api/agent/index.js');
        expect(find).toBeDefined();
        expect(typeof find).toBe('function');
    });

    it('should export vision functions', async () => {
        const vision = await import('@api/agent/index.js');
        expect(vision.screenshot).toBeDefined();
        expect(vision.buildPrompt).toBeDefined();
        expect(vision.parseResponse).toBeDefined();
        expect(vision.captureAXTree).toBeDefined();
        expect(vision.captureState).toBeDefined();
    });

    it('should export actionEngine and executeAction', async () => {
        const { actionEngine, executeAction } = await import('@api/agent/index.js');
        expect(actionEngine).toBeDefined();
        expect(executeAction).toBeDefined();
        expect(typeof executeAction).toBe('function');
    });

    it('should export llmClient', async () => {
        const { llmClient } = await import('@api/agent/index.js');
        expect(llmClient).toBeDefined();
    });

    it('should export agentRunner and related functions', async () => {
        const agent = await import('@api/agent/index.js');
        expect(agent.agentRunner).toBeDefined();
        expect(agent.runAgent).toBeDefined();
        expect(agent.stopAgent).toBeDefined();
        expect(typeof agent.runAgent).toBe('function');
        expect(typeof agent.stopAgent).toBe('function');
    });

    it('should export token utilities', async () => {
        const tokenUtils = await import('@api/agent/index.js');
        expect(tokenUtils.estimateTokens).toBeDefined();
        expect(tokenUtils.estimateMessageTokens).toBeDefined();
        expect(tokenUtils.estimateConversationTokens).toBeDefined();
    });
});
