/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { ActionRunner } from '@api/actions/advanced-index.js';

// Mock logger
vi.mock('@api/core/logger.js', () => ({
    createLogger: vi.fn(() => ({
        info: vi.fn(),
        error: vi.fn(),
        warn: vi.fn(),
        debug: vi.fn(),
    })),
}));

describe('Mutual Exclusion - ActionRunner', () => {
    let actionRunner;
    let mockAgent;
    let mockActions;

    beforeEach(() => {
        vi.clearAllMocks();

        mockAgent = {
            twitterConfig: {
                actions: {
                    reply: { probability: 0.3, enabled: true },
                    quote: { probability: 0.25, enabled: true },
                    like: { probability: 0.15, enabled: true },
                    bookmark: { probability: 0.1, enabled: true },
                    retweet: { probability: 0.2, enabled: true },
                    follow: { probability: 0, enabled: false },
                    goHome: { enabled: true },
                },
                engagement: {
                    mutualExclusion: {
                        enabled: true,
                        preventQuoteAfterRetweet: true,
                        preventRetweetAfterQuote: true,
                    },
                },
            },
            diveQueue: {
                canEngage: vi.fn().mockReturnValue(true),
            },
            // Tracking sets for mutual exclusion
            _quotedTweetIds: new Set(),
            _retweetedTweetIds: new Set(),
            _mutualExclusionConfig: {
                enabled: true,
                preventQuoteAfterRetweet: true,
                preventRetweetAfterQuote: true,
            },
        };

        mockActions = {
            reply: { execute: vi.fn(), tryExecute: vi.fn(), getStats: vi.fn() },
            quote: { execute: vi.fn(), tryExecute: vi.fn(), getStats: vi.fn() },
            like: { execute: vi.fn(), tryExecute: vi.fn(), getStats: vi.fn() },
            bookmark: { execute: vi.fn(), tryExecute: vi.fn(), getStats: vi.fn() },
            retweet: { execute: vi.fn(), tryExecute: vi.fn(), getStats: vi.fn() },
            goHome: { execute: vi.fn(), tryExecute: vi.fn(), getStats: vi.fn() },
        };

        actionRunner = new ActionRunner(mockAgent, mockActions);
    });

    describe('setCurrentTweetId', () => {
        it('should set current tweet ID', () => {
            actionRunner.setCurrentTweetId('123456789');
            expect(actionRunner.getCurrentTweetId()).toBe('123456789');
        });

        it('should allow null tweet ID', () => {
            actionRunner.setCurrentTweetId(null);
            expect(actionRunner.getCurrentTweetId()).toBeNull();
        });

        it('should overwrite previous tweet ID', () => {
            actionRunner.setCurrentTweetId('111');
            actionRunner.setCurrentTweetId('222');
            expect(actionRunner.getCurrentTweetId()).toBe('222');
        });
    });

    describe('isActionAvailable - mutual exclusion', () => {
        it('should allow quote when tweet not retweeted', () => {
            actionRunner.setCurrentTweetId('12345');
            const result = actionRunner.isActionAvailable('quote');
            expect(result).toBe(true);
        });

        it('should allow retweet when tweet not quoted', () => {
            actionRunner.setCurrentTweetId('12345');
            const result = actionRunner.isActionAvailable('retweet');
            expect(result).toBe(true);
        });

        it('should block quote when tweet already retweeted', () => {
            const tweetId = '12345';
            actionRunner.setCurrentTweetId(tweetId);
            mockAgent._retweetedTweetIds.add(tweetId);

            const result = actionRunner.isActionAvailable('quote');
            expect(result).toBe(false);
        });

        it('should block retweet when tweet already quoted', () => {
            const tweetId = '12345';
            actionRunner.setCurrentTweetId(tweetId);
            mockAgent._quotedTweetIds.add(tweetId);

            const result = actionRunner.isActionAvailable('retweet');
            expect(result).toBe(false);
        });

        it('should allow quote for different tweet when one is retweeted', () => {
            mockAgent._retweetedTweetIds.add('99999');
            actionRunner.setCurrentTweetId('12345');

            const result = actionRunner.isActionAvailable('quote');
            expect(result).toBe(true);
        });

        it('should allow retweet for different tweet when one is quoted', () => {
            mockAgent._quotedTweetIds.add('99999');
            actionRunner.setCurrentTweetId('12345');

            const result = actionRunner.isActionAvailable('retweet');
            expect(result).toBe(true);
        });

        it('should respect preventQuoteAfterRetweet config', () => {
            actionRunner.setCurrentTweetId('12345');
            mockAgent._retweetedTweetIds.add('12345');
            mockAgent._mutualExclusionConfig.preventQuoteAfterRetweet = false;

            const result = actionRunner.isActionAvailable('quote');
            expect(result).toBe(true);

            // Restore
            mockAgent._mutualExclusionConfig.preventQuoteAfterRetweet = true;
        });

        it('should respect preventRetweetAfterQuote config', () => {
            actionRunner.setCurrentTweetId('12345');
            mockAgent._quotedTweetIds.add('12345');
            mockAgent._mutualExclusionConfig.preventRetweetAfterQuote = false;

            const result = actionRunner.isActionAvailable('retweet');
            expect(result).toBe(true);

            // Restore
            mockAgent._mutualExclusionConfig.preventRetweetAfterQuote = true;
        });

        it('should bypass mutual exclusion when enabled is false', () => {
            actionRunner.setCurrentTweetId('12345');
            mockAgent._retweetedTweetIds.add('12345');
            mockAgent._mutualExclusionConfig.enabled = false;

            const result = actionRunner.isActionAvailable('quote');
            expect(result).toBe(true);

            // Restore
            mockAgent._mutualExclusionConfig.enabled = true;
        });

        it('should not check mutual exclusion for non-quote/retweet actions', () => {
            actionRunner.setCurrentTweetId('12345');
            mockAgent._retweetedTweetIds.add('12345');

            expect(actionRunner.isActionAvailable('reply')).toBe(true);
            expect(actionRunner.isActionAvailable('like')).toBe(true);
            expect(actionRunner.isActionAvailable('bookmark')).toBe(true);
        });

        it('should not check mutual exclusion when no tweet ID set', () => {
            actionRunner.setCurrentTweetId(null);
            mockAgent._retweetedTweetIds.add('12345');

            // Without tweetId, mutual exclusion check is skipped
            const result = actionRunner.isActionAvailable('quote');
            expect(result).toBe(true);
        });

        it('should use explicit tweetId parameter over currentTweetId', () => {
            actionRunner.setCurrentTweetId('11111');
            mockAgent._retweetedTweetIds.add('22222');

            // Should not block since '11111' is not in retweeted set
            const result = actionRunner.isActionAvailable('quote', '11111');
            expect(result).toBe(true);

            // Should block '22222' which is in retweeted set
            const result2 = actionRunner.isActionAvailable('quote', '22222');
            expect(result2).toBe(false);
        });
    });

    describe('calculateSmartProbabilities - mutual exclusion', () => {
        it('should redistribute when mutual exclusion blocks an action', () => {
            const tweetId = '12345';
            actionRunner.setCurrentTweetId(tweetId);
            mockAgent._retweetedTweetIds.add(tweetId);

            // Quote should be blocked by mutual exclusion
            const probs = actionRunner.calculateSmartProbabilities();

            expect(probs.quote).toBeUndefined();
            expect(probs.reply).toBeGreaterThan(0);
            expect(probs.retweet).toBeGreaterThan(0);
        });

        it('should handle both quote and retweet blocked', () => {
            const tweetId = '12345';
            actionRunner.setCurrentTweetId(tweetId);
            mockAgent._retweetedTweetIds.add(tweetId);
            mockAgent._quotedTweetIds.add(tweetId);

            const probs = actionRunner.calculateSmartProbabilities();

            expect(probs.quote).toBeUndefined();
            expect(probs.retweet).toBeUndefined();
        });
    });

    describe('selectAction - mutual exclusion', () => {
        it('should not select blocked action', () => {
            const tweetId = '12345';
            actionRunner.setCurrentTweetId(tweetId);
            mockAgent._retweetedTweetIds.add(tweetId);

            // Run multiple times to ensure quote is never selected
            for (let i = 0; i < 20; i++) {
                const action = actionRunner.selectAction();
                expect(action).not.toBe('quote');
            }
        });
    });
});
