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

describe('ActionRunner', () => {
    let actionRunner;
    let mockAgent;
    let mockActions;

    beforeEach(() => {
        vi.clearAllMocks();

        mockAgent = {
            twitterConfig: {
                actions: {
                    reply: { probability: 0.5, enabled: true },
                    quote: { probability: 0.2, enabled: true },
                    like: { probability: 0.2, enabled: true },
                    bookmark: { probability: 0.1, enabled: true },
                    retweet: { probability: 0, enabled: false },
                    follow: { probability: 0, enabled: false },
                    goHome: { enabled: true },
                },
            },
            diveQueue: {
                canEngage: vi.fn().mockReturnValue(true),
            },
        };

        mockActions = {
            reply: {
                execute: vi.fn().mockResolvedValue({ success: true }),
                tryExecute: vi.fn().mockResolvedValue({ executed: true }),
                getStats: vi.fn().mockReturnValue({ attempts: 1 }),
            },
            quote: { execute: vi.fn(), tryExecute: vi.fn(), getStats: vi.fn() },
            like: { execute: vi.fn(), tryExecute: vi.fn(), getStats: vi.fn() },
            bookmark: { execute: vi.fn(), tryExecute: vi.fn(), getStats: vi.fn() },
            goHome: { execute: vi.fn(), tryExecute: vi.fn(), getStats: vi.fn() },
        };

        actionRunner = new ActionRunner(mockAgent, mockActions);
    });

    describe('constructor', () => {
        it('should load config correctly', () => {
            expect(actionRunner.config.reply.probability).toBe(0.5);
            expect(actionRunner.config.reply.enabled).toBe(true);
        });

        it('should use defaults if config is missing', () => {
            const agent = { twitterConfig: {} };
            const runner = new ActionRunner(agent);

            expect(runner.config.reply.probability).toBe(0.6); // Default
            expect(runner.config.reply.enabled).toBe(true);
            expect(runner.config.follow.probability).toBe(0.1); // Default for follow
        });
    });

    describe('getEngagementType', () => {
        it('should return correct engagement types', () => {
            expect(actionRunner.getEngagementType('reply')).toBe('replies');
            expect(actionRunner.getEngagementType('quote')).toBe('quotes');
            expect(actionRunner.getEngagementType('like')).toBe('likes');
            expect(actionRunner.getEngagementType('bookmark')).toBe('bookmarks');
            expect(actionRunner.getEngagementType('follow')).toBe('follows');
            expect(actionRunner.getEngagementType('unknown')).toBe('unknown');
        });
    });

    describe('isActionAvailable', () => {
        it('should return true if action enabled and not limited', () => {
            expect(actionRunner.isActionAvailable('reply')).toBe(true);
        });

        it('should return false if action disabled', () => {
            actionRunner.config.reply.enabled = false;
            expect(actionRunner.isActionAvailable('reply')).toBe(false);
        });

        it('should return false if action limited', () => {
            mockAgent.diveQueue.canEngage.mockReturnValue(false);
            expect(actionRunner.isActionAvailable('reply')).toBe(false);
        });

        it('should return false if action config missing', () => {
            expect(actionRunner.isActionAvailable('nonexistent')).toBe(false);
        });
    });

    describe('calculateSmartProbabilities', () => {
        it('should return base probabilities if all available', () => {
            // Mock total to be 1.0 for simplicity in test setup
            // Config: 0.5, 0.2, 0.2, 0.1 => Sum 1.0

            const probs = actionRunner.calculateSmartProbabilities();

            expect(probs.reply).toBeCloseTo(0.5);
            expect(probs.quote).toBeCloseTo(0.2);
            expect(probs.like).toBeCloseTo(0.2);
            expect(probs.bookmark).toBeCloseTo(0.1);
            expect(probs.follow).toBeUndefined(); // follow is disabled in mockAgent
        });

        it('should redistribute if an action is unavailable', () => {
            // Disable reply (0.5)
            // Remaining: 0.2, 0.2, 0.1 => Sum 0.5
            // New weights should be doubled: 0.4, 0.4, 0.2

            actionRunner.config.reply.enabled = false;
            if (actionRunner.config.retweet) actionRunner.config.retweet.enabled = false;
            if (actionRunner.config.follow) actionRunner.config.follow.enabled = false; // Ensure follow is also disabled

            const probs = actionRunner.calculateSmartProbabilities();

            expect(probs.reply).toBeUndefined();
            expect(probs.quote).toBeCloseTo(0.4);
            expect(probs.like).toBeCloseTo(0.4);
            expect(probs.bookmark).toBeCloseTo(0.2);
            expect(probs.follow).toBeUndefined();
        });

        it('should handle all actions unavailable', () => {
            actionRunner.config.reply.enabled = false;
            actionRunner.config.quote.enabled = false;
            actionRunner.config.like.enabled = false;
            actionRunner.config.bookmark.enabled = false;
            if (actionRunner.config.retweet) actionRunner.config.retweet.enabled = false;
            if (actionRunner.config.follow) actionRunner.config.follow.enabled = false; // Ensure follow is also disabled

            const probs = actionRunner.calculateSmartProbabilities();
            expect(Object.keys(probs).length).toBe(0);
        });
    });

    describe('selectAction', () => {
        it('should select an action based on probabilities', () => {
            // Mock Math.random to return 0 (selects first available)
            const randomSpy = vi.spyOn(Math, 'random').mockReturnValue(0.0);

            const action = actionRunner.selectAction();

            // First one in loop is usually 'reply' based on order in calculateSmartProbabilities
            // 'reply', 'quote', 'like', 'bookmark'
            expect(action).toBe('reply');

            randomSpy.mockRestore();
        });

        it('should return null if no actions available', () => {
            vi.spyOn(actionRunner, 'calculateSmartProbabilities').mockReturnValue({});

            const action = actionRunner.selectAction();
            expect(action).toBeNull();
        });

        it('should fallback to last action if random roll exceeds cumulative (rounding errors)', () => {
            // Mock probabilities
            vi.spyOn(actionRunner, 'calculateSmartProbabilities').mockReturnValue({
                reply: 0.5,
                quote: 0.5,
            });

            // Random 0.999 -> should pick quote
            vi.spyOn(Math, 'random').mockReturnValue(0.999);

            const action = actionRunner.selectAction();
            expect(action).toBe('quote');
        });
    });

    describe('executeAction', () => {
        it('should execute known action', async () => {
            const result = await actionRunner.executeAction('reply', {});

            expect(mockActions.reply.execute).toHaveBeenCalled();
            expect(result.success).toBe(true);
        });

        it('should return error for unknown action', async () => {
            const result = await actionRunner.executeAction('unknown');

            expect(result.success).toBe(false);
            expect(result.reason).toBe('unknown_action');
        });
    });

    describe('tryExecute', () => {
        it('should try execute known action', async () => {
            const result = await actionRunner.tryExecute('reply', {});

            expect(mockActions.reply.tryExecute).toHaveBeenCalled();
            expect(result.executed).toBe(true);
        });

        it('should return error for unknown action', async () => {
            const result = await actionRunner.tryExecute('unknown');

            expect(result.success).toBe(false);
            expect(result.reason).toBe('unknown_action');
        });
    });

    describe('getStats', () => {
        it('should aggregate stats from all actions', () => {
            const stats = actionRunner.getStats();

            expect(stats).toHaveProperty('reply');
            expect(stats).toHaveProperty('quote');
            expect(stats.reply.attempts).toBe(1);
        });
    });
});
