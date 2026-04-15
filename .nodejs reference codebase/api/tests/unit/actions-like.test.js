/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { LikeAction } from '@api/actions/ai-twitter-like.js';

// Mock logger
vi.mock('@api/core/logger.js', () => ({
    createLogger: vi.fn(() => ({
        info: vi.fn(),
        error: vi.fn(),
        warn: vi.fn(),
        debug: vi.fn(),
    })),
}));

describe('LikeAction', () => {
    let likeAction;
    let mockAgent;

    beforeEach(() => {
        vi.clearAllMocks();

        mockAgent = {
            twitterConfig: {
                actions: {
                    like: {
                        enabled: true,
                        probability: 0.5,
                    },
                },
            },
            handleLike: vi.fn().mockResolvedValue(undefined),
            diveQueue: {
                canEngage: vi.fn().mockReturnValue(true),
            },
        };

        likeAction = new LikeAction(mockAgent);
    });

    describe('constructor', () => {
        it('should load config', () => {
            expect(likeAction.enabled).toBe(true);
            expect(likeAction.probability).toBe(0.5);
        });

        it('should use defaults if config missing', () => {
            const agent = { twitterConfig: {} };
            const action = new LikeAction(agent);
            expect(action.enabled).toBe(true);
            expect(action.probability).toBe(0.15); // Default
        });
    });

    describe('canExecute', () => {
        it('should allow if conditions met', async () => {
            const result = await likeAction.canExecute();
            expect(result.allowed).toBe(true);
        });

        it('should reject if engagement limit reached', async () => {
            mockAgent.diveQueue.canEngage.mockReturnValue(false);
            const result = await likeAction.canExecute();
            expect(result.allowed).toBe(false);
            expect(result.reason).toBe('engagement_limit_reached');
        });
    });

    describe('execute', () => {
        it('should execute like successfully', async () => {
            const context = { tweetElement: {}, tweetUrl: 'url' };
            const result = await likeAction.execute(context);

            expect(mockAgent.handleLike).toHaveBeenCalled();
            expect(result.success).toBe(true);
        });

        it('should handle exception', async () => {
            mockAgent.handleLike.mockRejectedValue(new Error('Like failed'));

            const result = await likeAction.execute({});

            expect(result.success).toBe(false);
            expect(result.reason).toBe('exception');
        });
    });

    describe('tryExecute', () => {
        it('should execute if allowed and probability check passes', async () => {
            likeAction.probability = 1.0;
            const result = await likeAction.tryExecute({});
            expect(result.executed).toBe(true);
        });

        it('should skip based on probability', async () => {
            likeAction.probability = 0.0;
            const result = await likeAction.tryExecute({});
            expect(result.executed).toBe(false);
            expect(result.reason).toBe('probability');
        });
    });

    describe('stats', () => {
        it('should track stats correctly', async () => {
            await likeAction.execute({});

            const stats = likeAction.getStats();
            expect(stats.attempts).toBe(1);
            expect(stats.successes).toBe(1);
            expect(stats.successRate).toBe('100.0%');
        });

        it('should reset stats', async () => {
            await likeAction.execute({});
            likeAction.resetStats();

            const stats = likeAction.getStats();
            expect(stats.attempts).toBe(0);
            expect(stats.successes).toBe(0);
        });
    });
});
