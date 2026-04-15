/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';

vi.mock('@api/index.js', () => ({
    api: {
        setPage: vi.fn(),
        getPage: vi.fn(),
        wait: vi.fn().mockResolvedValue(undefined),
        exists: vi.fn().mockResolvedValue(true),
        visible: vi.fn().mockImplementation(async (el) => {
            if (el && typeof el.isVisible === 'function') return await el.isVisible();
            return false;
        }),
        getCurrentUrl: vi.fn().mockResolvedValue('https://x.com/testuser/status/123456789'),
    },
}));
import { api } from '@api/index.js';
import { RetweetAction } from '@api/actions/ai-twitter-retweet.js';

vi.mock('@api/core/logger.js', () => ({
    createLogger: vi.fn(() => ({
        info: vi.fn(),
        error: vi.fn(),
        warn: vi.fn(),
        debug: vi.fn(),
    })),
}));

describe('RetweetAction', () => {
    let retweetAction;
    let mockAgent;
    let mockPage;
    let mockTweetElement;

    beforeEach(() => {
        vi.clearAllMocks();
        mockPage = {
            waitForTimeout: vi.fn().mockResolvedValue(undefined),
            locator: vi.fn(),
            keyboard: { press: vi.fn().mockResolvedValue(undefined) },
            isClosed: vi.fn().mockReturnValue(false),
            context: vi.fn().mockReturnValue({
                browser: vi.fn().mockReturnValue({ isConnected: vi.fn().mockReturnValue(true) }),
            }),
        };
        api.getPage.mockReturnValue(mockPage);
        mockAgent = {
            twitterConfig: {
                actions: { retweet: { enabled: true, probability: 0.5, strategy: 'click' } },
            },
            page: mockPage,
            humanClick: vi.fn().mockResolvedValue(undefined),
            diveQueue: { canEngage: vi.fn().mockReturnValue(true), recordEngagement: vi.fn() },
            scrollToGoldenZone: null,
        };
        mockTweetElement = {
            locator: vi.fn(),
            scrollIntoViewIfNeeded: vi.fn().mockResolvedValue(undefined),
        };
        retweetAction = new RetweetAction(mockAgent);
    });

    describe('constructor', () => {
        it('should load config correctly', () => {
            expect(retweetAction.enabled).toBe(true);
            expect(retweetAction.probability).toBe(0.5);
        });
        it('should use defaults if config missing', () => {
            mockAgent.twitterConfig = {};
            const action = new RetweetAction(mockAgent);
            expect(action.enabled).toBe(true);
            expect(action.probability).toBe(0.2);
        });
        it('should use defaults if actions missing', () => {
            mockAgent.twitterConfig = { actions: {} };
            const action = new RetweetAction(mockAgent);
            expect(action.enabled).toBe(true);
            expect(action.probability).toBe(0.2);
        });
        it('should set engagementType', () => {
            expect(retweetAction.engagementType).toBe('retweets');
        });
    });

    describe('loadConfig', () => {
        it('should load custom probability', () => {
            mockAgent.twitterConfig = { actions: { retweet: { enabled: true, probability: 0.8 } } };
            const action = new RetweetAction(mockAgent);
            expect(action.probability).toBe(0.8);
        });
        it('should disable when false', () => {
            mockAgent.twitterConfig = { actions: { retweet: { enabled: false } } };
            const action = new RetweetAction(mockAgent);
            expect(action.enabled).toBe(false);
        });
    });

    describe('canExecute', () => {
        it('should return false if no agent', async () => {
            const action = new RetweetAction(null);
            const result = await action.canExecute();
            expect(result.allowed).toBe(false);
            expect(result.reason).toBe('agent_not_initialized');
        });
        it('should return false if limit reached', async () => {
            mockAgent.diveQueue.canEngage.mockReturnValue(false);
            const result = await retweetAction.canExecute();
            expect(result.allowed).toBe(false);
            expect(result.reason).toBe('engagement_limit_reached');
        });
        it('should return true normally', async () => {
            const result = await retweetAction.canExecute();
            expect(result.allowed).toBe(true);
        });
        it('should return true without diveQueue', async () => {
            const agentNoQueue = { twitterConfig: mockAgent.twitterConfig, page: mockPage };
            const action = new RetweetAction(agentNoQueue);
            const result = await action.canExecute();
            expect(result.allowed).toBe(true);
        });
    });

    describe('handleRetweet - already retweeted', () => {
        it('should return success if unretweet button visible', async () => {
            const mockLocator = {
                first: vi.fn().mockReturnValue({
                    isVisible: vi.fn().mockResolvedValue(true),
                    count: vi.fn().mockResolvedValue(1),
                }),
            };
            mockTweetElement.locator.mockImplementation((selector) => {
                if (selector.includes('unretweet')) return mockLocator;
                return { first: vi.fn() };
            });
            const result = await retweetAction.handleRetweet(mockTweetElement);
            expect(result.success).toBe(true);
            expect(result.reason).toBe('already_retweeted');
        });

        it('should return success if aria-label Reposted', async () => {
            const mockUnretweet = {
                first: vi.fn().mockReturnValue({
                    isVisible: vi.fn().mockResolvedValue(false),
                    count: vi.fn().mockResolvedValue(0),
                }),
            };
            const mockAria = {
                first: vi.fn().mockReturnValue({ isVisible: vi.fn().mockResolvedValue(true) }),
            };
            mockTweetElement.locator.mockImplementation((selector) => {
                if (selector.includes('unretweet')) return mockUnretweet;
                if (selector.includes('Reposted')) return mockAria;
                return { first: vi.fn() };
            });
            const result = await retweetAction.handleRetweet(mockTweetElement);
            expect(result.success).toBe(true);
            expect(result.reason).toBe('already_retweeted');
        });

        it('should call scrollToGoldenZone when present', async () => {
            mockAgent.scrollToGoldenZone = vi.fn().mockResolvedValue(undefined);
            const mockLocator = {
                first: vi.fn().mockReturnValue({
                    isVisible: vi.fn().mockResolvedValue(false),
                    count: vi.fn().mockResolvedValue(1),
                }),
            };
            mockTweetElement.locator.mockImplementation((selector) => {
                if (selector.includes('unretweet')) return mockLocator;
                return { first: vi.fn() };
            });
            await retweetAction.handleRetweet(mockTweetElement);
            expect(mockAgent.scrollToGoldenZone).toHaveBeenCalled();
        });

        it('should handle scroll error gracefully', async () => {
            mockTweetElement.scrollIntoViewIfNeeded = vi
                .fn()
                .mockRejectedValue(new Error('scroll failed'));
            const mockLocator = {
                first: vi.fn().mockReturnValue({ isVisible: vi.fn().mockResolvedValue(false) }),
            };
            mockTweetElement.locator.mockImplementation((selector) => {
                if (selector.includes('unretweet')) return mockLocator;
                return { first: vi.fn() };
            });
            const result = await retweetAction.handleRetweet(mockTweetElement);
            expect(result).toBeDefined();
        });
    });

    describe('handleRetweet - button not found', () => {
        it('should return failure if button not found', async () => {
            const mockUnretweet = {
                first: vi.fn().mockReturnValue({
                    isVisible: vi.fn().mockResolvedValue(false),
                    count: vi.fn().mockResolvedValue(0),
                }),
            };
            const mockRetweet = {
                first: vi.fn().mockReturnValue({ count: vi.fn().mockResolvedValue(0) }),
            };
            const mockAria = {
                first: vi.fn().mockReturnValue({ isVisible: vi.fn().mockResolvedValue(false) }),
            };
            mockTweetElement.locator.mockImplementation((selector) => {
                if (selector.includes('unretweet')) return mockUnretweet;
                if (selector.includes('retweet') && !selector.includes('Confirm'))
                    return mockRetweet;
                if (selector.includes('Repost') || selector.includes('Retweet')) return mockAria;
                return { first: vi.fn() };
            });
            const result = await retweetAction.handleRetweet(mockTweetElement);
            expect(result.success).toBe(false);
            expect(result.reason).toBe('retweet_button_not_found');
        });
    });

    describe('handleRetweet - menu handling', () => {
        it('should handle menu appearing and confirm', async () => {
            mockAgent.scrollToGoldenZone = vi.fn().mockResolvedValue(undefined);
            // unretweet not visible, retweet count > 0 to skip aria-label fallback
            const mockUnretweet = {
                first: vi.fn().mockReturnValue({
                    isVisible: vi.fn().mockResolvedValue(false),
                    count: vi.fn().mockResolvedValue(0),
                    waitFor: vi.fn().mockResolvedValue(undefined),
                }),
            };
            // count > 0 to skip aria-label fallback
            const mockRetweet = {
                first: vi.fn().mockReturnValue({
                    count: vi.fn().mockResolvedValue(1),
                    isVisible: vi.fn().mockResolvedValue(true),
                }),
            };
            // aria-label for Reposted should NOT match (return false)
            const mockReposted = {
                first: vi.fn().mockReturnValue({
                    isVisible: vi.fn().mockResolvedValue(false),
                }),
            };
            mockTweetElement.locator.mockImplementation((selector) => {
                if (selector.includes('unretweet')) return mockUnretweet;
                if (selector.includes('retweet') && !selector.includes('Confirm'))
                    return mockRetweet;
                if (selector.includes('Reposted')) return mockReposted;
                return { first: vi.fn() };
            });
            const mockConfirm = {
                first: vi.fn().mockReturnValue({
                    waitFor: vi.fn().mockResolvedValue(undefined),
                    isVisible: vi.fn().mockResolvedValue(true),
                }),
            };
            mockPage.locator.mockImplementation((selector) => {
                if (selector.includes('Confirm')) return mockConfirm;
                return { first: vi.fn() };
            });
            const result = await retweetAction.handleRetweet(mockTweetElement);
            expect(mockAgent.humanClick).toHaveBeenCalledTimes(2);
            expect(result.success).toBe(true);
        });

        it('should handle menu timeout', async () => {
            const mockUnretweet = {
                first: vi.fn().mockReturnValue({
                    isVisible: vi.fn().mockResolvedValue(false),
                    count: vi.fn().mockResolvedValue(0),
                }),
            };
            const mockRetweet = {
                first: vi.fn().mockReturnValue({
                    count: vi.fn().mockResolvedValue(1),
                    isVisible: vi.fn().mockResolvedValue(true),
                }),
            };
            const mockReposted = {
                first: vi.fn().mockReturnValue({
                    isVisible: vi.fn().mockResolvedValue(false),
                }),
            };
            mockTweetElement.locator.mockImplementation((selector) => {
                if (selector.includes('unretweet')) return mockUnretweet;
                if (selector.includes('retweet') && !selector.includes('Confirm'))
                    return mockRetweet;
                if (selector.includes('Reposted')) return mockReposted;
                return { first: vi.fn() };
            });
            mockPage.locator.mockImplementation((selector) => {
                if (selector.includes('Confirm')) {
                    return {
                        first: vi.fn().mockReturnValue({
                            waitFor: vi.fn().mockRejectedValue(new Error('timeout')),
                        }),
                    };
                }
                return { first: vi.fn() };
            });
            const result = await retweetAction.handleRetweet(mockTweetElement);
            expect(result.success).toBe(false);
            expect(result.reason).toBe('retweet_menu_failed');
            expect(mockPage.keyboard.press).toHaveBeenCalledWith('Escape');
        });
    });

    describe('handleRetweet - verification', () => {
        it('should verify success', async () => {
            const mockUnretweet = {
                first: vi.fn().mockReturnValue({
                    isVisible: vi.fn().mockResolvedValue(false),
                    count: vi.fn().mockResolvedValue(0),
                    waitFor: vi.fn().mockResolvedValue(undefined),
                }),
            };
            // count > 0 to skip aria-label fallback
            const mockRetweet = {
                first: vi.fn().mockReturnValue({
                    count: vi.fn().mockResolvedValue(1),
                    isVisible: vi.fn().mockResolvedValue(true),
                }),
            };
            // aria-label Reposted should return false
            const mockReposted = {
                first: vi.fn().mockReturnValue({
                    isVisible: vi.fn().mockResolvedValue(false),
                }),
            };
            mockTweetElement.locator.mockImplementation((selector) => {
                if (selector.includes('unretweet')) return mockUnretweet;
                if (selector.includes('retweet') && !selector.includes('Confirm'))
                    return mockRetweet;
                if (selector.includes('Reposted')) return mockReposted;
                return { first: vi.fn() };
            });
            const mockConfirm = {
                first: vi.fn().mockReturnValue({
                    waitFor: vi.fn().mockResolvedValue(undefined),
                    isVisible: vi.fn().mockResolvedValue(true),
                }),
            };
            mockPage.locator.mockImplementation((selector) => {
                if (selector.includes('Confirm')) return mockConfirm;
                return { first: vi.fn() };
            });
            const result = await retweetAction.handleRetweet(mockTweetElement);
            expect(result.success).toBe(true);
            expect(result.reason).toBe('retweet_successful');
            expect(mockAgent.diveQueue.recordEngagement).toHaveBeenCalledWith('retweets');
        });

        it('should handle verification failure', async () => {
            const mockUnretweet = {
                first: vi.fn().mockReturnValue({
                    isVisible: vi.fn().mockResolvedValue(false),
                    count: vi.fn().mockResolvedValue(0),
                    waitFor: vi.fn().mockRejectedValue(new Error('timeout')),
                }),
            };
            const mockRetweet = {
                first: vi.fn().mockReturnValue({
                    count: vi.fn().mockResolvedValue(1),
                    isVisible: vi.fn().mockResolvedValue(true),
                }),
            };
            const mockReposted = {
                first: vi.fn().mockReturnValue({
                    isVisible: vi.fn().mockResolvedValue(false),
                }),
            };
            mockTweetElement.locator.mockImplementation((selector) => {
                if (selector.includes('unretweet')) return mockUnretweet;
                if (selector.includes('retweet') && !selector.includes('Confirm'))
                    return mockRetweet;
                if (selector.includes('Reposted')) return mockReposted;
                return { first: vi.fn() };
            });
            const mockConfirm = {
                first: vi.fn().mockReturnValue({
                    waitFor: vi.fn().mockResolvedValue(undefined),
                    isVisible: vi.fn().mockResolvedValue(true),
                }),
            };
            mockPage.locator.mockImplementation((selector) => {
                if (selector.includes('Confirm')) return mockConfirm;
                return { first: vi.fn() };
            });
            const result = await retweetAction.handleRetweet(mockTweetElement);
            expect(result.success).toBe(false);
            expect(result.reason).toBe('retweet_verification_failed');
        });

        it('should work without diveQueue', async () => {
            delete mockAgent.diveQueue;
            const mockUnretweet = {
                first: vi.fn().mockReturnValue({
                    isVisible: vi.fn().mockResolvedValue(false),
                    count: vi.fn().mockResolvedValue(0),
                    waitFor: vi.fn().mockResolvedValue(undefined),
                }),
            };
            const mockRetweet = {
                first: vi.fn().mockReturnValue({
                    count: vi.fn().mockResolvedValue(1),
                    isVisible: vi.fn().mockResolvedValue(true),
                }),
            };
            const mockReposted = {
                first: vi.fn().mockReturnValue({
                    isVisible: vi.fn().mockResolvedValue(false),
                }),
            };
            mockTweetElement.locator.mockImplementation((selector) => {
                if (selector.includes('unretweet')) return mockUnretweet;
                if (selector.includes('retweet') && !selector.includes('Confirm'))
                    return mockRetweet;
                if (selector.includes('Reposted')) return mockReposted;
                return { first: vi.fn() };
            });
            const mockConfirm = {
                first: vi.fn().mockReturnValue({
                    waitFor: vi.fn().mockResolvedValue(undefined),
                    isVisible: vi.fn().mockResolvedValue(true),
                }),
            };
            mockPage.locator.mockImplementation((selector) => {
                if (selector.includes('Confirm')) return mockConfirm;
                return { first: vi.fn() };
            });
            const result = await retweetAction.handleRetweet(mockTweetElement);
            expect(result.success).toBe(true);
        });
    });

    describe('handleRetweet - errors', () => {
        it('should handle unexpected errors', async () => {
            mockTweetElement.locator.mockImplementation(() => {
                throw new Error('Unexpected');
            });
            const result = await retweetAction.handleRetweet(mockTweetElement);
            expect(result.success).toBe(false);
            expect(result.reason).toContain('error:');
        });
    });

    describe('execute', () => {
        it('should increment stats on success', async () => {
            vi.spyOn(retweetAction, 'handleRetweet').mockResolvedValue({
                success: true,
                reason: 'success',
            });
            const result = await retweetAction.execute({
                tweetElement: mockTweetElement,
                tweetUrl: 'url',
            });
            expect(retweetAction.stats.attempts).toBe(1);
            expect(retweetAction.stats.successes).toBe(1);
            expect(result.success).toBe(true);
        });
        it('should increment failures', async () => {
            vi.spyOn(retweetAction, 'handleRetweet').mockResolvedValue({
                success: false,
                reason: 'failed',
            });
            const result = await retweetAction.execute({ tweetElement: mockTweetElement });
            expect(retweetAction.stats.failures).toBe(1);
            expect(result.success).toBe(false);
        });
        it('should handle exceptions', async () => {
            vi.spyOn(retweetAction, 'handleRetweet').mockRejectedValue(new Error('Crash'));
            const result = await retweetAction.execute({ tweetElement: mockTweetElement });
            expect(retweetAction.stats.failures).toBe(1);
            expect(result.reason).toBe('exception');
        });
        it('should include engagementType', async () => {
            vi.spyOn(retweetAction, 'handleRetweet').mockResolvedValue({ success: true });
            const result = await retweetAction.execute({ tweetElement: mockTweetElement });
            expect(result.engagementType).toBe('retweets');
        });
    });

    describe('tryExecute', () => {
        it('should skip when canExecute false', async () => {
            vi.spyOn(retweetAction, 'canExecute').mockResolvedValue({ allowed: false });
            const result = await retweetAction.tryExecute();
            expect(retweetAction.stats.skipped).toBe(1);
            expect(result.executed).toBe(false);
        });
        it('should skip based on probability', async () => {
            retweetAction.probability = 0;
            vi.spyOn(retweetAction, 'canExecute').mockResolvedValue({ allowed: true });
            const result = await retweetAction.tryExecute();
            expect(result.reason).toBe('probability');
        });
        it('should execute when probability passes', async () => {
            retweetAction.probability = 1;
            vi.spyOn(retweetAction, 'canExecute').mockResolvedValue({ allowed: true });
            vi.spyOn(retweetAction, 'execute').mockResolvedValue({ success: true });
            const result = await retweetAction.tryExecute();
            expect(result.success).toBe(true);
        });
    });

    describe('getStats', () => {
        it('should return correct stats', () => {
            retweetAction.stats.attempts = 10;
            retweetAction.stats.successes = 7;
            retweetAction.stats.failures = 2;
            retweetAction.stats.skipped = 1;
            const stats = retweetAction.getStats();
            expect(stats.attempts).toBe(10);
            expect(stats.successes).toBe(7);
            expect(stats.failures).toBe(2);
            expect(stats.skipped).toBe(1);
            expect(stats.successRate).toBe('70.0%');
            expect(stats.engagementType).toBe('retweets');
        });
        it('should handle zero', () => {
            const stats = retweetAction.getStats();
            expect(stats.successRate).toBe('0.0%');
        });
    });

    describe('resetStats', () => {
        it('should reset all stats', () => {
            retweetAction.stats.attempts = 10;
            retweetAction.stats.successes = 5;
            retweetAction.stats.failures = 3;
            retweetAction.stats.skipped = 2;
            retweetAction.resetStats();
            expect(retweetAction.stats.attempts).toBe(0);
            expect(retweetAction.stats.successes).toBe(0);
            expect(retweetAction.stats.failures).toBe(0);
            expect(retweetAction.stats.skipped).toBe(0);
        });
    });
});
