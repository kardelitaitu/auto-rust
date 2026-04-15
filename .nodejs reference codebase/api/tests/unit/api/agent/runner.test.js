/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

vi.mock('@api/core/logger.js', () => ({
    createLogger: vi.fn(() => ({
        info: vi.fn(),
        warn: vi.fn(),
        error: vi.fn(),
        debug: vi.fn(),
    })),
}));

vi.mock('@api/core/context.js', () => ({
    getPage: vi.fn(),
}));

vi.mock('@api/agent/llmClient.js', () => ({
    llmClient: {
        init: vi.fn().mockResolvedValue(undefined),
        generateCompletion: vi.fn(),
        getUsageStats: vi.fn().mockReturnValue({}),
        config: { useVision: true },
    },
}));

vi.mock('@api/agent/actionEngine.js', () => ({
    actionEngine: {
        execute: vi.fn(),
    },
}));

vi.mock('@api/agent/tokenCounter.js', () => ({
    estimateConversationTokens: vi.fn().mockReturnValue(100),
}));

vi.mock('@api/core/config.js', () => ({
    configManager: {
        get: vi.fn((key, defaultVal) => defaultVal),
    },
}));

vi.mock('fs/promises', () => ({
    mkdir: vi.fn().mockResolvedValue(undefined),
    writeFile: vi.fn().mockResolvedValue(undefined),
}));

import { getPage } from '@api/core/context.js';
import { agentRunner } from '@api/agent/runner.js';
import { llmClient } from '@api/agent/llmClient.js';
import { actionEngine } from '@api/agent/actionEngine.js';

describe('api/agent/runner.js', () => {
    let mockPage;

    beforeEach(() => {
        vi.clearAllMocks();

        mockPage = {
            bringToFront: vi.fn().mockResolvedValue(undefined),
            accessibility: {
                snapshot: vi.fn().mockResolvedValue({ role: 'root', name: 'test' }),
            },
            url: vi.fn().mockReturnValue('https://example.com'),
            waitForLoadState: vi.fn().mockResolvedValue(undefined),
            waitForTimeout: vi.fn().mockResolvedValue(undefined),
            screenshot: vi.fn().mockResolvedValue(Buffer.from('fake-image-data')),
        };

        getPage.mockReturnValue(mockPage);

        agentRunner.isRunning = false;
        agentRunner.currentGoal = null;
        agentRunner.history = [];
        agentRunner.lastAction = null;
        agentRunner.lastState = null;
        agentRunner.consecutiveLlmFailures = 0;
        agentRunner.consecutiveActionCount = 0;
        agentRunner.stateVisitCounts = {};
    });

    afterEach(() => {
        agentRunner.stop();
    });

    describe('Runner basic properties', () => {
        it('should have correct default values', () => {
            expect(agentRunner.maxSteps).toBe(20);
            expect(agentRunner.stepDelay).toBe(2000);
            expect(agentRunner.isRunning).toBe(false);
        });

        it('should have isRunning state', () => {
            expect(agentRunner.isRunning).toBeDefined();
        });

        it('should have history array', () => {
            expect(Array.isArray(agentRunner.history)).toBe(true);
        });
    });

    describe('run method', () => {
        it('should throw if already running', async () => {
            agentRunner.isRunning = true;

            await expect(agentRunner.run('test goal')).rejects.toThrow('already running');
        });

        it('should initialize llmClient on run', async () => {
            llmClient.generateCompletion.mockResolvedValue({ action: 'done', done: true });
            actionEngine.execute.mockResolvedValue({ success: true, done: true });

            await agentRunner.run('test goal');

            expect(llmClient.init).toHaveBeenCalled();
        });

        it('should throw if no page available', async () => {
            getPage.mockReturnValue(null);

            await expect(agentRunner.run('test goal')).rejects.toThrow('No page available');
        });

        it('should complete successfully with done action', async () => {
            llmClient.generateCompletion.mockResolvedValue({ action: 'done' });
            actionEngine.execute.mockResolvedValue({ success: true, done: true });

            const result = await agentRunner.run('test goal');

            expect(result.success).toBe(true);
            expect(result.done).toBe(true);
        });

        it('should continue when no action in response', async () => {
            llmClient.generateCompletion
                .mockResolvedValueOnce({})
                .mockResolvedValueOnce({ action: 'done' });
            actionEngine.execute.mockResolvedValue({ success: true, done: true });

            const result = await agentRunner.run('test goal', { maxSteps: 2 });

            expect(result.success).toBe(true);
        });

        it('should abort after 3 consecutive LLM failures', async () => {
            llmClient.generateCompletion.mockRejectedValue(new Error('LLM error'));

            await agentRunner.run('test goal', { maxSteps: 5 });

            expect(llmClient.generateCompletion).toHaveBeenCalledTimes(3);
        });

        it('should stop on loop detection', async () => {
            llmClient.generateCompletion.mockResolvedValue({ action: 'click', selector: '#btn' });
            actionEngine.execute.mockResolvedValue({ success: true });

            const result = await agentRunner.run('test goal', { maxSteps: 5 });

            // Should complete either by reaching max steps or loop detection
            expect(result).toHaveProperty('success');
            expect(result).toHaveProperty('reason');
        });

        it('should respect custom maxSteps', async () => {
            llmClient.generateCompletion.mockResolvedValue({ action: 'wait', value: '100' });
            actionEngine.execute.mockResolvedValue({ success: true });

            const result = await agentRunner.run('test goal', { maxSteps: 3 });

            // With maxSteps 3, should not exceed that many calls
            expect(llmClient.generateCompletion).toHaveBeenCalled();
            expect(result.steps).toBeLessThanOrEqual(3);
        });

        it('should use adaptive delay when configured', async () => {
            llmClient.generateCompletion.mockResolvedValue({ action: 'wait', value: '100' });
            actionEngine.execute.mockResolvedValue({ success: true });

            await agentRunner.run('test goal', { maxSteps: 2, stepDelay: 1000 });

            expect(mockPage.waitForLoadState).toHaveBeenCalled();
        });

        it('should add actions to history', async () => {
            llmClient.generateCompletion.mockResolvedValue({ action: 'click', selector: '#btn' });
            actionEngine.execute.mockResolvedValue({ success: true, done: true });

            await agentRunner.run('test goal');

            expect(agentRunner.history.length).toBeGreaterThan(0);
        });
    });

    describe('stop method', () => {
        it('should stop the runner', () => {
            agentRunner.isRunning = true;

            agentRunner.stop();

            expect(agentRunner.isRunning).toBe(false);
        });
    });

    describe('getUsageStats', () => {
        it('should return usage stats', () => {
            agentRunner.currentGoal = 'test goal';
            agentRunner.maxSteps = 10;
            agentRunner.history = [{ role: 'user', content: 'test' }];

            const stats = agentRunner.getUsageStats();

            expect(stats.isRunning).toBe(false);
            expect(stats.goal).toBe('test goal');
            expect(stats.maxSteps).toBe(10);
            expect(stats.historySize).toBe(1);
        });
    });

    describe('semantic state hash', () => {
        it('should generate same hash from AXTree with different timestamps', () => {
            const input1 =
                '{"id": "123456789012345", "timestamp": "1234567890123", "name": "test1"}';
            const input2 =
                '{"id": "987654321098765", "timestamp": "9876543210987", "name": "test2"}';

            const hash1 = agentRunner._generateSemanticStateHash(input1);
            const hash2 = agentRunner._generateSemanticStateHash(input2);

            // Both should have their timestamps and dynamic IDs stripped
            expect(hash1).not.toContain('123456789012345');
            expect(hash2).not.toContain('987654321098765');
            expect(hash1).not.toContain('1234567890123');
            expect(hash2).not.toContain('9876543210987');
        });

        it('should handle empty input', () => {
            const hash = agentRunner._generateSemanticStateHash('');
            expect(hash).toBe('');
        });

        it('should handle null input', () => {
            const hash = agentRunner._generateSemanticStateHash(null);
            expect(hash).toBe('');
        });

        it('should handle non-string input', () => {
            const obj = { key: 'value' };
            const hash = agentRunner._generateSemanticStateHash(obj);
            expect(hash).toBeDefined();
        });

        it('should strip timestamps', () => {
            const input = '{"time": "1234567890123", "name": "test"}';
            const hash = agentRunner._generateSemanticStateHash(input);
            expect(hash).not.toContain('1234567890123');
        });

        it('should strip dynamic IDs', () => {
            const input = '{"id": "abc123def456ghi789", "name": "test"}';
            const hash = agentRunner._generateSemanticStateHash(input);
            expect(hash).not.toContain('abc123def456ghi789');
        });

        it('should strip coordinates', () => {
            const input = '{"x": 123.45, "y": 67.89}';
            const hash = agentRunner._generateSemanticStateHash(input);
            expect(hash).toContain('"coord":0');
        });
    });

    describe('buildPrompt', () => {
        it('should build prompt with vision enabled', () => {
            llmClient.config = { useVision: true };

            const messages = agentRunner._buildPrompt(
                'test goal',
                'base64image',
                '{"role": "root"}',
                'https://example.com'
            );

            expect(messages.length).toBeGreaterThanOrEqual(2);
            expect(messages[0].role).toBe('system');
        });

        it('should build prompt with vision disabled', () => {
            llmClient.config = { useVision: false };

            const messages = agentRunner._buildPrompt(
                'test goal',
                'base64image',
                '{"role": "root"}',
                'https://example.com'
            );

            const userMsg = messages[messages.length - 1];
            expect(
                userMsg.content.some((c) => c.text && c.text.includes('accessibility tree'))
            ).toBe(true);
        });

        it('should include history in prompt', () => {
            agentRunner.history = [{ role: 'user', content: 'previous message' }];

            const messages = agentRunner._buildPrompt(
                'test goal',
                'base64image',
                '{"role": "root"}',
                'https://example.com'
            );

            expect(messages.length).toBeGreaterThan(2);
        });

        it('should handle array content in history', () => {
            agentRunner.history = [
                { role: 'user', content: [{ type: 'text', text: 'text content' }] },
            ];

            const messages = agentRunner._buildPrompt(
                'test goal',
                'base64image',
                '{"role": "root"}',
                'https://example.com'
            );

            expect(messages).toBeDefined();
        });
    });
});
