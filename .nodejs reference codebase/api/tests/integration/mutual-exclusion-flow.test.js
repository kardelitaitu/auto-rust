/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { ActionRunner } from '@api/actions/advanced-index.js';
import { AIQuoteAction } from '@api/actions/ai-twitter-quote.js';
import { RetweetAction } from '@api/actions/ai-twitter-retweet.js';

// Mock logger
vi.mock('@api/core/logger.js', () => ({
    createLogger: vi.fn(() => ({
        info: vi.fn(),
        error: vi.fn(),
        warn: vi.fn(),
        debug: vi.fn(),
    })),
}));

// Mock api
vi.mock('@api/index.js', () => ({
    api: {
        wait: vi.fn().mockResolvedValue(undefined),
        visible: vi.fn().mockResolvedValue(false),
        getCurrentUrl: vi.fn().mockResolvedValue('https://x.com/testuser/status/12345'),
    },
}));
import { api } from '@api/index.js';

describe('Mutual Exclusion - Integration Flow', () => {
    let actionRunner;
    let quoteAction;
    let retweetAction;
    let mockAgent;
    let mockPage;
    let mockActions;

    beforeEach(() => {
        vi.clearAllMocks();

        mockPage = {
            locator: vi.fn(),
            keyboard: { press: vi.fn().mockResolvedValue(undefined) },
        };

        mockAgent = {
            twitterConfig: {
                actions: {
                    reply: { probability: 0.2, enabled: true },
                    quote: { probability: 0.2, enabled: true },
                    like: { probability: 0.2, enabled: true },
                    bookmark: { probability: 0.2, enabled: true },
                    retweet: { probability: 0.2, enabled: true },
                    follow: { probability: 0, enabled: false },
                    goHome: { enabled: true },
                },
            },
            page: mockPage,
            pageOps: {
                urlSync: vi.fn().mockResolvedValue('https://x.com/testuser/status/12345'),
            },
            diveQueue: {
                canEngage: vi.fn().mockReturnValue(true),
                recordEngagement: vi.fn(),
            },
            // Tracking sets for mutual exclusion
            _quotedTweetIds: new Set(),
            _retweetedTweetIds: new Set(),
            _mutualExclusionConfig: {
                enabled: true,
                preventQuoteAfterRetweet: true,
                preventRetweetAfterQuote: true,
            },
            // Quote action dependencies
            contextEngine: {
                extractEnhancedContext: vi.fn().mockResolvedValue({ replies: [], sentiment: {} }),
            },
            quoteEngine: {
                generateQuote: vi.fn().mockResolvedValue({ success: true, quote: 'Test quote' }),
            },
            executeAIQuote: vi.fn().mockResolvedValue(true),
            // Retweet action dependencies
            humanClick: vi.fn().mockResolvedValue(undefined),
        };

        // Create action instances
        quoteAction = new AIQuoteAction(mockAgent);
        retweetAction = new RetweetAction(mockAgent);

        mockActions = {
            reply: { execute: vi.fn(), tryExecute: vi.fn(), getStats: vi.fn() },
            quote: quoteAction,
            like: { execute: vi.fn(), tryExecute: vi.fn(), getStats: vi.fn() },
            bookmark: { execute: vi.fn(), tryExecute: vi.fn(), getStats: vi.fn() },
            retweet: retweetAction,
            goHome: { execute: vi.fn(), tryExecute: vi.fn(), getStats: vi.fn() },
        };

        actionRunner = new ActionRunner(mockAgent, mockActions);
    });

    describe('full flow: quote then retweet blocked', () => {
        it('should prevent retweet after quote on same tweet', async () => {
            const tweetId = '12345';
            const tweetUrl = `https://x.com/testuser/status/${tweetId}`;

            // Set current tweet ID
            actionRunner.setCurrentTweetId(tweetId);

            // Step 1: Verify both actions are initially available
            expect(actionRunner.isActionAvailable('quote')).toBe(true);
            expect(actionRunner.isActionAvailable('retweet')).toBe(true);

            // Step 2: Execute quote action (simulate success)
            mockAgent._quotedTweetIds.add(tweetId);

            // Step 3: Verify mutual exclusion kicks in
            expect(actionRunner.isActionAvailable('quote')).toBe(true); // Can still quote (no double-quote prevention)
            expect(actionRunner.isActionAvailable('retweet')).toBe(false); // Blocked by mutual exclusion
        });

        it('should prevent quote after retweet on same tweet', async () => {
            const tweetId = '67890';
            const tweetUrl = `https://x.com/testuser/status/${tweetId}`;

            // Set current tweet ID
            actionRunner.setCurrentTweetId(tweetId);

            // Step 1: Verify both actions are initially available
            expect(actionRunner.isActionAvailable('quote')).toBe(true);
            expect(actionRunner.isActionAvailable('retweet')).toBe(true);

            // Step 2: Execute retweet action (simulate success)
            mockAgent._retweetedTweetIds.add(tweetId);

            // Step 3: Verify mutual exclusion kicks in
            expect(actionRunner.isActionAvailable('quote')).toBe(false); // Blocked by mutual exclusion
            expect(actionRunner.isActionAvailable('retweet')).toBe(true); // Can still retweet (no double-retweet prevention)
        });
    });

    describe('different tweets are independent', () => {
        it('should not block retweet on tweet B after quote on tweet A', async () => {
            const tweetIdA = '11111';
            const tweetIdB = '22222';

            // Quote tweet A
            mockAgent._quotedTweetIds.add(tweetIdA);

            // Check tweet B
            actionRunner.setCurrentTweetId(tweetIdB);
            expect(actionRunner.isActionAvailable('retweet')).toBe(true);
        });

        it('should not block quote on tweet B after retweet on tweet A', async () => {
            const tweetIdA = '11111';
            const tweetIdB = '22222';

            // Retweet tweet A
            mockAgent._retweetedTweetIds.add(tweetIdA);

            // Check tweet B
            actionRunner.setCurrentTweetId(tweetIdB);
            expect(actionRunner.isActionAvailable('quote')).toBe(true);
        });
    });

    describe('action selection respects mutual exclusion', () => {
        it('should never select quote when tweet is retweeted', async () => {
            const tweetId = '12345';
            actionRunner.setCurrentTweetId(tweetId);
            mockAgent._retweetedTweetIds.add(tweetId);

            // Run many times to ensure quote is never selected
            const selectedActions = new Set();
            for (let i = 0; i < 50; i++) {
                const action = actionRunner.selectAction();
                selectedActions.add(action);
            }

            expect(selectedActions.has('quote')).toBe(false);
        });

        it('should never select retweet when tweet is quoted', async () => {
            const tweetId = '12345';
            actionRunner.setCurrentTweetId(tweetId);
            mockAgent._quotedTweetIds.add(tweetId);

            // Run many times to ensure retweet is never selected
            const selectedActions = new Set();
            for (let i = 0; i < 50; i++) {
                const action = actionRunner.selectAction();
                selectedActions.add(action);
            }

            expect(selectedActions.has('retweet')).toBe(false);
        });

        it('should allow both actions when mutual exclusion disabled', async () => {
            mockAgent._mutualExclusionConfig.enabled = false;
            const tweetId = '12345';
            actionRunner.setCurrentTweetId(tweetId);

            // Add to both tracking sets
            mockAgent._retweetedTweetIds.add(tweetId);
            mockAgent._quotedTweetIds.add(tweetId);

            // Both should be available now
            expect(actionRunner.isActionAvailable('quote')).toBe(true);
            expect(actionRunner.isActionAvailable('retweet')).toBe(true);

            // Restore
            mockAgent._mutualExclusionConfig.enabled = true;
        });
    });

    describe('config propagation', () => {
        it('should use config from settings', () => {
            expect(mockAgent._mutualExclusionConfig.enabled).toBe(true);
            expect(mockAgent._mutualExclusionConfig.preventQuoteAfterRetweet).toBe(true);
            expect(mockAgent._mutualExclusionConfig.preventRetweetAfterQuote).toBe(true);
        });

        it('should respect partial config', () => {
            mockAgent._mutualExclusionConfig.preventQuoteAfterRetweet = false;

            const tweetId = '12345';
            actionRunner.setCurrentTweetId(tweetId);
            mockAgent._retweetedTweetIds.add(tweetId);

            // Quote should be allowed since preventQuoteAfterRetweet is false
            expect(actionRunner.isActionAvailable('quote')).toBe(true);
            // Retweet should still work
            expect(actionRunner.isActionAvailable('retweet')).toBe(true);

            // Restore
            mockAgent._mutualExclusionConfig.preventQuoteAfterRetweet = true;
        });
    });

    describe('edge cases', () => {
        it('should handle empty tracking sets', () => {
            actionRunner.setCurrentTweetId('12345');

            expect(actionRunner.isActionAvailable('quote')).toBe(true);
            expect(actionRunner.isActionAvailable('retweet')).toBe(true);
        });

        it('should handle null tweet ID', () => {
            actionRunner.setCurrentTweetId(null);

            // Should not check mutual exclusion when no tweet ID
            expect(actionRunner.isActionAvailable('quote')).toBe(true);
            expect(actionRunner.isActionAvailable('retweet')).toBe(true);
        });

        it('should handle undefined tracking sets', () => {
            delete mockAgent._quotedTweetIds;
            delete mockAgent._retweetedTweetIds;

            actionRunner.setCurrentTweetId('12345');

            // Should not throw error
            expect(actionRunner.isActionAvailable('quote')).toBe(true);
            expect(actionRunner.isActionAvailable('retweet')).toBe(true);
        });

        it('should handle undefined mutualExclusionConfig', () => {
            delete mockAgent._mutualExclusionConfig;

            const tweetId = '12345';
            actionRunner.setCurrentTweetId(tweetId);
            mockAgent._retweetedTweetIds.add(tweetId);

            // Should not check mutual exclusion when config is undefined
            expect(actionRunner.isActionAvailable('quote')).toBe(true);
        });
    });
});
