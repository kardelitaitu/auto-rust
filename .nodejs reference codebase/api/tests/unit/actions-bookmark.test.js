/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { BookmarkAction } from '@api/actions/ai-twitter-bookmark.js';

// Mock logger
vi.mock('@api/core/logger.js', () => ({
    createLogger: vi.fn(() => ({
        info: vi.fn(),
        error: vi.fn(),
        warn: vi.fn(),
        debug: vi.fn(),
    })),
}));

describe('BookmarkAction', () => {
    let bookmarkAction;
    let mockAgent;

    beforeEach(() => {
        vi.clearAllMocks();

        mockAgent = {
            twitterConfig: {
                actions: {
                    bookmark: {
                        enabled: true,
                        probability: 0.5,
                    },
                },
            },
            handleBookmark: vi.fn().mockResolvedValue(undefined),
            diveQueue: {
                canEngage: vi.fn().mockReturnValue(true),
            },
        };

        bookmarkAction = new BookmarkAction(mockAgent);
    });

    describe('constructor', () => {
        it('should initialize with default stats', () => {
            const stats = bookmarkAction.getStats();
            expect(stats.attempts).toBe(0);
            expect(stats.successes).toBe(0);
        });

        it('should load config from agent', () => {
            expect(bookmarkAction.enabled).toBe(true);
            expect(bookmarkAction.probability).toBe(0.5);
        });

        it('should use defaults if config missing', () => {
            const agent = { twitterConfig: {} };
            const action = new BookmarkAction(agent);
            expect(action.enabled).toBe(true);
            expect(action.probability).toBe(0.05); // Default
        });
    });

    describe('canExecute', () => {
        it('should allow if conditions met', async () => {
            const result = await bookmarkAction.canExecute();
            expect(result.allowed).toBe(true);
        });

        it('should reject if agent not initialized', async () => {
            bookmarkAction.agent = null;
            const result = await bookmarkAction.canExecute();
            expect(result.allowed).toBe(false);
            expect(result.reason).toBe('agent_not_initialized');
        });

        it('should reject if engagement limit reached', async () => {
            mockAgent.diveQueue.canEngage.mockReturnValue(false);
            const result = await bookmarkAction.canExecute();
            expect(result.allowed).toBe(false);
            expect(result.reason).toBe('engagement_limit_reached');
        });
    });

    describe('execute', () => {
        it('should execute bookmark successfully', async () => {
            const context = { tweetElement: {}, tweetUrl: 'http://twitter.com/123' };
            const result = await bookmarkAction.execute(context);

            expect(mockAgent.handleBookmark).toHaveBeenCalledWith(context.tweetElement);
            expect(result.success).toBe(true);
            expect(bookmarkAction.stats.successes).toBe(1);
        });

        it('should handle exception', async () => {
            mockAgent.handleBookmark.mockRejectedValue(new Error('Bookmark failed'));

            const context = { tweetElement: {} };
            const result = await bookmarkAction.execute(context);

            expect(result.success).toBe(false);
            expect(result.reason).toBe('exception');
            expect(bookmarkAction.stats.failures).toBe(1);
        });
    });

    describe('tryExecute', () => {
        it('should skip if not allowed', async () => {
            mockAgent.diveQueue.canEngage.mockReturnValue(false);

            const result = await bookmarkAction.tryExecute();

            expect(result.executed).toBe(false);
            expect(result.reason).toBe('engagement_limit_reached');
            expect(bookmarkAction.stats.skipped).toBe(1);
        });

        it('should skip based on probability', async () => {
            bookmarkAction.probability = 0; // Never execute

            const result = await bookmarkAction.tryExecute();

            expect(result.executed).toBe(false);
            expect(result.reason).toBe('probability');
            expect(bookmarkAction.stats.skipped).toBe(1);
        });

        it('should execute if allowed and probability check passes', async () => {
            bookmarkAction.probability = 1; // Always execute

            const result = await bookmarkAction.tryExecute({ tweetElement: {} });

            expect(result.executed).toBe(true);
            expect(result.success).toBe(true);
        });
    });
});
