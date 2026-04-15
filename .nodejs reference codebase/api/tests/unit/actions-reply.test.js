/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { AIReplyAction } from '@api/actions/ai-twitter-reply.js';

// Mock logger
vi.mock('@api/core/logger.js', () => ({
    createLogger: vi.fn(() => ({
        info: vi.fn(),
        error: vi.fn(),
        warn: vi.fn(),
        debug: vi.fn(),
    })),
}));

describe('AIReplyAction', () => {
    let replyAction;
    let mockAgent;

    beforeEach(() => {
        vi.clearAllMocks();

        mockAgent = {
            twitterConfig: {
                actions: {
                    reply: {
                        enabled: true,
                        probability: 0.5,
                    },
                },
            },
            contextEngine: {
                extractEnhancedContext: vi.fn().mockResolvedValue({ replies: [], sentiment: {} }),
            },
            replyEngine: {
                generateReply: vi.fn().mockResolvedValue({ success: true, reply: 'Nice reply!' }),
            },
            executeAIReply: vi.fn().mockResolvedValue(true),
            diveQueue: {
                canEngage: vi.fn().mockReturnValue(true),
                recordEngagement: vi.fn(),
            },
            page: {},
        };

        replyAction = new AIReplyAction(mockAgent);
    });

    describe('constructor', () => {
        it('should load config', () => {
            expect(replyAction.enabled).toBe(true);
            expect(replyAction.probability).toBe(0.5);
        });

        it('should use defaults when twitterConfig missing', () => {
            const agentNoConfig = { twitterConfig: null };
            const action = new AIReplyAction(agentNoConfig);
            expect(action.enabled).toBe(true);
            expect(action.probability).toBe(0.6);
        });

        it('should use defaults when actions config missing', () => {
            const agentPartialConfig = { twitterConfig: {} };
            const action = new AIReplyAction(agentPartialConfig);
            expect(action.enabled).toBe(true);
            expect(action.probability).toBe(0.6);
        });

        it('should handle missing agent', () => {
            const action = new AIReplyAction(null);
            expect(action.agent).toBe(null);
        });
    });

    describe('canExecute', () => {
        it('should allow if all data present', async () => {
            const context = { tweetText: 'text', username: 'user', tweetUrl: 'url' };
            const result = await replyAction.canExecute(context);
            expect(result.allowed).toBe(true);
        });

        it('should reject if data missing', async () => {
            const result = await replyAction.canExecute({});
            expect(result.allowed).toBe(false);
            expect(result.reason).toBe('no_tweet_text');
        });

        it('should reject if agent not initialized', async () => {
            replyAction.agent = null;
            const context = { tweetText: 'text', username: 'user', tweetUrl: 'url' };
            const result = await replyAction.canExecute(context);
            expect(result.allowed).toBe(false);
            expect(result.reason).toBe('agent_not_initialized');
        });

        it('should reject if username missing', async () => {
            const context = { tweetText: 'text', username: '', tweetUrl: 'url' };
            const result = await replyAction.canExecute(context);
            expect(result.allowed).toBe(false);
            expect(result.reason).toBe('no_username');
        });

        it('should reject if diveQueue limit reached', async () => {
            mockAgent.diveQueue.canEngage.mockReturnValue(false);
            const context = { tweetText: 'text', username: 'user', tweetUrl: 'url' };
            const result = await replyAction.canExecute(context);
            expect(result.allowed).toBe(false);
            expect(result.reason).toBe('engagement_limit_reached');
        });

        it('should reject if action disabled', async () => {
            replyAction.enabled = false;
            const context = { tweetText: 'text', username: 'user', tweetUrl: 'url' };
            const result = await replyAction.canExecute(context);
            expect(result.allowed).toBe(false);
            expect(result.reason).toBe('action_disabled');
        });
    });

    describe('execute', () => {
        const context = { tweetText: 'text', username: 'user', tweetUrl: 'url' };

        it('should generate and post reply successfully', async () => {
            const result = await replyAction.execute(context);

            expect(mockAgent.contextEngine.extractEnhancedContext).toHaveBeenCalled();
            expect(mockAgent.replyEngine.generateReply).toHaveBeenCalled();
            expect(mockAgent.executeAIReply).toHaveBeenCalledWith('Nice reply!');
            expect(result.success).toBe(true);
        });

        it('should use pre-calculated context if provided', async () => {
            const contextWithContext = {
                tweetText: 'text',
                username: 'user',
                tweetUrl: 'url',
                enhancedContext: { replies: [{ id: 1 }], sentiment: { overall: 'positive' } },
            };

            const result = await replyAction.execute(contextWithContext);

            expect(mockAgent.contextEngine.extractEnhancedContext).not.toHaveBeenCalled();
            expect(result.success).toBe(true);
        });

        it('should handle AI generation failure', async () => {
            mockAgent.replyEngine.generateReply.mockResolvedValue({
                success: false,
                reason: 'Too controversial',
            });

            const result = await replyAction.execute(context);

            expect(result.success).toBe(false);
            expect(result.reason).toBe('Too controversial');
            expect(mockAgent.executeAIReply).not.toHaveBeenCalled();
        });

        it('should handle exception', async () => {
            mockAgent.contextEngine.extractEnhancedContext.mockRejectedValue(
                new Error('Context failed')
            );

            const result = await replyAction.execute(context);

            expect(result.success).toBe(false);
            expect(result.reason).toBe('exception');
        });
    });

    describe('tryExecute', () => {
        it('should skip if canExecute returns false', async () => {
            const result = await replyAction.tryExecute({});
            expect(result.executed).toBe(false);
            expect(result.success).toBe(false);
            expect(replyAction.stats.skipped).toBe(1);
        });

        it('should skip based on probability', async () => {
            replyAction.probability = 0;
            const context = { tweetText: 'text', username: 'user', tweetUrl: 'url' };
            const result = await replyAction.tryExecute(context);
            expect(result.executed).toBe(false);
            expect(result.reason).toBe('probability');
            expect(replyAction.stats.skipped).toBe(1);
        });

        it('should execute if probability check passes', async () => {
            replyAction.probability = 1;
            const context = { tweetText: 'text', username: 'user', tweetUrl: 'url' };
            const result = await replyAction.tryExecute(context);
            expect(result.executed).toBe(true);
            expect(result.success).toBe(true);
        });
    });

    describe('stats', () => {
        const context = { tweetText: 'text', username: 'user', tweetUrl: 'url' };

        it('should track stats correctly', async () => {
            await replyAction.execute(context);

            const stats = replyAction.getStats();
            expect(stats.attempts).toBe(1);
            expect(stats.successes).toBe(1);
            expect(stats.successRate).toBe('100.0%');
        });

        it('should reset stats', async () => {
            await replyAction.execute(context);
            replyAction.resetStats();

            const stats = replyAction.getStats();
            expect(stats.attempts).toBe(0);
            expect(stats.successes).toBe(0);
        });
    });
});
