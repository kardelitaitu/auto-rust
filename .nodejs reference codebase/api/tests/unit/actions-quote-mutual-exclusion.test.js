/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { AIQuoteAction } from '@api/actions/ai-twitter-quote.js';

// Mock logger
vi.mock('@api/core/logger.js', () => ({
    createLogger: vi.fn(() => ({
        info: vi.fn(),
        error: vi.fn(),
        warn: vi.fn(),
        debug: vi.fn(),
    })),
}));

describe('AIQuoteAction - Tweet ID Tracking', () => {
    let quoteAction;
    let mockAgent;

    beforeEach(() => {
        vi.clearAllMocks();

        mockAgent = {
            twitterConfig: {
                actions: {
                    quote: {
                        enabled: true,
                        probability: 0.5,
                    },
                },
            },
            contextEngine: {
                extractEnhancedContext: vi.fn().mockResolvedValue({ replies: [], sentiment: {} }),
            },
            quoteEngine: {
                generateQuote: vi.fn().mockResolvedValue({ success: true, quote: 'Nice quote!' }),
            },
            executeAIQuote: vi.fn().mockResolvedValue(true),
            diveQueue: {
                canEngage: vi.fn().mockReturnValue(true),
                recordEngagement: vi.fn(),
            },
            page: {},
            // Tracking sets for mutual exclusion
            _quotedTweetIds: new Set(),
            _retweetedTweetIds: new Set(),
        };

        quoteAction = new AIQuoteAction(mockAgent);
    });

    describe('execute - tweet ID tracking', () => {
        it('should add tweet ID to _quotedTweetIds after successful quote', async () => {
            const context = {
                tweetText: 'Test tweet',
                username: 'testuser',
                tweetUrl: 'https://x.com/testuser/status/123456789',
            };

            await quoteAction.execute(context);

            expect(mockAgent._quotedTweetIds.has('123456789')).toBe(true);
        });

        it('should not add tweet ID on failed quote', async () => {
            mockAgent.executeAIQuote.mockResolvedValue(false);

            const context = {
                tweetText: 'Test tweet',
                username: 'testuser',
                tweetUrl: 'https://x.com/testuser/status/123456789',
            };

            await quoteAction.execute(context);

            expect(mockAgent._quotedTweetIds.has('123456789')).toBe(false);
        });

        it('should not add tweet ID on AI generation failure', async () => {
            mockAgent.quoteEngine.generateQuote.mockResolvedValue({
                success: false,
                reason: 'generation_failed',
            });

            const context = {
                tweetText: 'Test tweet',
                username: 'testuser',
                tweetUrl: 'https://x.com/testuser/status/123456789',
            };

            await quoteAction.execute(context);

            expect(mockAgent._quotedTweetIds.has('123456789')).toBe(false);
        });

        it('should handle invalid tweet URL gracefully', async () => {
            const context = {
                tweetText: 'Test tweet',
                username: 'testuser',
                tweetUrl: 'https://x.com/testuser/invalid',
            };

            const result = await quoteAction.execute(context);

            // Should still succeed, just not track tweet ID
            expect(result.success).toBe(true);
            expect(mockAgent._quotedTweetIds.size).toBe(0);
        });

        it('should handle missing tweetUrl gracefully', async () => {
            const context = {
                tweetText: 'Test tweet',
                username: 'testuser',
                tweetUrl: '',
            };

            const result = await quoteAction.execute(context);

            // Should still succeed, just not track tweet ID
            expect(result.success).toBe(true);
            expect(mockAgent._quotedTweetIds.size).toBe(0);
        });

        it('should handle missing _quotedTweetIds set gracefully', async () => {
            delete mockAgent._quotedTweetIds;

            const context = {
                tweetText: 'Test tweet',
                username: 'testuser',
                tweetUrl: 'https://x.com/testuser/status/123456789',
            };

            const result = await quoteAction.execute(context);

            // Should still succeed without error
            expect(result.success).toBe(true);
        });

        it('should track multiple tweet IDs independently', async () => {
            const context1 = {
                tweetText: 'Test tweet 1',
                username: 'user1',
                tweetUrl: 'https://x.com/user1/status/111111111',
            };

            const context2 = {
                tweetText: 'Test tweet 2',
                username: 'user2',
                tweetUrl: 'https://x.com/user2/status/222222222',
            };

            await quoteAction.execute(context1);
            await quoteAction.execute(context2);

            expect(mockAgent._quotedTweetIds.has('111111111')).toBe(true);
            expect(mockAgent._quotedTweetIds.has('222222222')).toBe(true);
            expect(mockAgent._quotedTweetIds.size).toBe(2);
        });

        it('should handle same tweet ID (idempotent)', async () => {
            const context = {
                tweetText: 'Test tweet',
                username: 'testuser',
                tweetUrl: 'https://x.com/testuser/status/123456789',
            };

            await quoteAction.execute(context);
            await quoteAction.execute(context);

            // Set should deduplicate
            expect(mockAgent._quotedTweetIds.size).toBe(1);
        });
    });
});
