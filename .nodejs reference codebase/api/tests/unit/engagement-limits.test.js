/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Unit tests for Engagement Limits
 * @module tests/unit/engagement-limits.test
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import engagementLimits from '@api/utils/engagement-limits.js';

describe('engagement-limits', () => {
    let tracker;

    beforeEach(() => {
        tracker = engagementLimits.createEngagementTracker();
    });

    describe('createEngagementTracker', () => {
        it('should create tracker with default limits', () => {
            expect(tracker.limits.replies).toBe(3);
            expect(tracker.limits.retweets).toBe(1);
            expect(tracker.limits.quotes).toBe(1);
            expect(tracker.limits.likes).toBe(5);
            expect(tracker.limits.follows).toBe(2);
            expect(tracker.limits.bookmarks).toBe(2);
        });

        it('should create tracker with custom limits', () => {
            const customTracker = engagementLimits.createEngagementTracker({
                likes: 10,
                replies: 5,
            });

            expect(customTracker.limits.likes).toBe(10);
            expect(customTracker.limits.replies).toBe(5);
            expect(customTracker.limits.follows).toBeUndefined(); // Only custom limits set
        });

        it('should initialize stats to zero', () => {
            expect(tracker.stats.likes).toBe(0);
            expect(tracker.stats.replies).toBe(0);
            expect(tracker.stats.retweets).toBe(0);
        });

        it('should initialize empty history', () => {
            expect(tracker.history).toEqual([]);
        });
    });

    describe('canPerform', () => {
        it('should return true when under limit', () => {
            expect(tracker.canPerform('likes')).toBe(true);
        });

        it('should return false when at limit', () => {
            tracker.stats.likes = 5;

            expect(tracker.canPerform('likes')).toBe(false);
        });

        it('should return false when over limit', () => {
            tracker.stats.likes = 10;

            expect(tracker.canPerform('likes')).toBe(false);
        });

        it('should handle unknown actions with Infinity limit', () => {
            expect(tracker.canPerform('unknown')).toBe(true);
        });
    });

    describe('getRemaining', () => {
        it('should return correct remaining count', () => {
            tracker.stats.likes = 2;

            expect(tracker.getRemaining('likes')).toBe(3);
        });

        it('should return 0 when exhausted', () => {
            tracker.stats.likes = 5;

            expect(tracker.getRemaining('likes')).toBe(0);
        });

        it('should return Infinity for unknown actions', () => {
            expect(tracker.getRemaining('unknown')).toBe(Infinity);
        });
    });

    describe('getUsage', () => {
        it('should return usage ratio', () => {
            tracker.stats.likes = 2;

            expect(tracker.getUsage('likes')).toBe(0.4);
        });

        it('should return 0 when at limit 0', () => {
            const customTracker = engagementLimits.createEngagementTracker({ custom: 0 });

            expect(customTracker.getUsage('custom')).toBe(0);
        });
    });

    describe('getProgress', () => {
        it('should return formatted progress string', () => {
            tracker.stats.likes = 2;

            expect(tracker.getProgress('likes')).toBe('2/5');
        });

        it('should handle infinity limits', () => {
            tracker.stats.custom = 10;

            expect(tracker.getProgress('custom')).toBe('10 used');
        });
    });

    describe('getProgressPercent', () => {
        it('should return percentage with % sign', () => {
            tracker.stats.likes = 2;

            expect(tracker.getProgressPercent('likes')).toBe('40.0%');
        });
    });

    describe('isNearLimit', () => {
        it('should return true when at or above threshold', () => {
            tracker.stats.likes = 4; // 80% of 5

            expect(tracker.isNearLimit('likes', 0.8)).toBe(true);
        });

        it('should return false when below threshold', () => {
            tracker.stats.likes = 3;

            expect(tracker.isNearLimit('likes', 0.8)).toBe(false);
        });

        it('should use default threshold of 0.8', () => {
            tracker.stats.likes = 4;

            expect(tracker.isNearLimit('likes')).toBe(true);
        });
    });

    describe('isExhausted', () => {
        it('should return true when at limit', () => {
            tracker.stats.likes = 5;

            expect(tracker.isExhausted('likes')).toBe(true);
        });

        it('should return false when under limit', () => {
            tracker.stats.likes = 4;

            expect(tracker.isExhausted('likes')).toBe(false);
        });
    });

    describe('isAnyExhausted', () => {
        it('should return false when all actions have capacity', () => {
            expect(tracker.isAnyExhausted()).toBe(false);
        });

        it('should return true when all actions are exhausted', () => {
            tracker.stats.likes = 5;
            tracker.stats.replies = 3;

            expect(tracker.isAnyExhausted()).toBe(true);
        });
    });

    describe('hasRemainingCapacity', () => {
        it('should return true when some actions have capacity', () => {
            tracker.stats.likes = 5;

            expect(tracker.hasRemainingCapacity()).toBe(true);
        });

        it('should return false when all actions exhausted', () => {
            tracker.stats.likes = 5;
            tracker.stats.replies = 3;
            tracker.stats.retweets = 1;
            tracker.stats.quotes = 1;
            tracker.stats.follows = 2;
            tracker.stats.bookmarks = 2;

            expect(tracker.hasRemainingCapacity()).toBe(false);
        });
    });

    describe('record', () => {
        it('should increment stat for valid action', () => {
            tracker.record('likes');

            expect(tracker.stats.likes).toBe(1);
        });

        it('should add to history', () => {
            tracker.record('likes');

            expect(tracker.history.length).toBe(1);
            expect(tracker.history[0].action).toBe('likes');
        });

        it('should return true on successful record', () => {
            const result = tracker.record('likes');

            expect(result).toBe(true);
        });

        it('should return false when at limit', () => {
            tracker.stats.likes = 5;

            const result = tracker.record('likes');

            expect(result).toBe(false);
            expect(tracker.history.length).toBe(0);
        });

        it('should warn on unknown action', () => {
            const originalWarn = console.warn;
            console.warn = vi.fn();

            tracker.record('unknown');

            expect(console.warn).toHaveBeenCalled();
            console.warn = originalWarn;
        });
    });

    describe('recordIfAllowed', () => {
        it('should record when allowed', () => {
            tracker.recordIfAllowed('likes');

            expect(tracker.stats.likes).toBe(1);
        });

        it('should not record when not allowed', () => {
            tracker.stats.likes = 5;
            tracker.recordIfAllowed('likes');

            expect(tracker.stats.likes).toBe(5);
        });
    });

    describe('decrement', () => {
        it('should decrement stat when greater than 0', () => {
            tracker.stats.likes = 3;

            tracker.decrement('likes');

            expect(tracker.stats.likes).toBe(2);
        });

        it('should not decrement below 0', () => {
            tracker.decrement('likes');

            expect(tracker.stats.likes).toBe(0);
        });

        it('should return true when decremented', () => {
            tracker.stats.likes = 1;

            expect(tracker.decrement('likes')).toBe(true);
        });

        it('should return false when already 0', () => {
            expect(tracker.decrement('likes')).toBe(false);
        });
    });

    describe('getStatus', () => {
        it('should return status for all limited actions', () => {
            tracker.stats.likes = 2;

            const status = tracker.getStatus();

            expect(status.likes).toBeDefined();
            expect(status.likes.current).toBe(2);
            expect(status.likes.limit).toBe(5);
            expect(status.likes.remaining).toBe(3);
        });
    });

    describe('getSummary', () => {
        it('should return formatted summary', () => {
            tracker.stats.likes = 2;
            tracker.stats.replies = 1;

            const summary = tracker.getSummary();

            expect(summary).toContain('likes: 2/5');
            expect(summary).toContain('replies: 1/3');
        });

        it('should exclude infinity limits', () => {
            tracker.stats.custom = 10;

            const summary = tracker.getSummary();

            expect(summary).not.toContain('custom');
        });
    });

    describe('getUsageRate', () => {
        it('should return total usage statistics', () => {
            tracker.stats.likes = 2;
            tracker.stats.replies = 1;

            const rate = tracker.getUsageRate();

            expect(rate.used).toBe(3);
            expect(rate.limit).toBeGreaterThan(0);
            expect(typeof rate.percentage).toBe('string');
        });
    });

    describe('getRecentActions', () => {
        it('should return recent actions with default limit', () => {
            for (let i = 0; i < 5; i++) {
                tracker.record('likes');
            }

            const recent = tracker.getRecentActions();

            expect(recent.length).toBe(5);
        });

        it('should respect custom limit', () => {
            for (let i = 0; i < 5; i++) {
                tracker.record('likes');
            }

            const recent = tracker.getRecentActions(3);

            expect(recent.length).toBe(3);
        });
    });

    describe('reset', () => {
        it('should reset all stats to zero', () => {
            tracker.stats.likes = 5;
            tracker.stats.replies = 2;
            tracker.history.push({ action: 'likes' });

            tracker.reset();

            expect(tracker.stats.likes).toBe(0);
            expect(tracker.stats.replies).toBe(0);
            expect(tracker.history.length).toBe(0);
        });

        it('should preserve limits', () => {
            tracker.reset();

            expect(tracker.limits.likes).toBe(5);
        });
    });

    describe('setLimit', () => {
        it('should update existing limit', () => {
            tracker.setLimit('likes', 10);

            expect(tracker.limits.likes).toBe(10);
        });

        it('should not create new limit', () => {
            tracker.setLimit('custom', 5);

            expect(tracker.limits.custom).toBeUndefined();
        });
    });

    describe('setLimits', () => {
        it('should update multiple limits', () => {
            tracker.setLimits({ likes: 10, replies: 5 });

            expect(tracker.limits.likes).toBe(10);
            expect(tracker.limits.replies).toBe(5);
        });

        it('should preserve unspecified limits', () => {
            tracker.setLimits({ likes: 10 });

            expect(tracker.limits.retweets).toBe(1);
        });
    });
});

describe('engagementLimits helpers', () => {
    describe('formatLimitStatus', () => {
        it('should format status with emojis', () => {
            const tracker = engagementLimits.createEngagementTracker({ likes: 5 });
            tracker.stats.likes = 5; // Exhausted

            const status = engagementLimits.formatLimitStatus(tracker);

            expect(status).toContain('🚫');
            expect(status).toContain('likes');
        });

        it('should show warning emoji when near limit', () => {
            const tracker = engagementLimits.createEngagementTracker({ likes: 10 });
            tracker.stats.likes = 8; // 80%

            const status = engagementLimits.formatLimitStatus(tracker);

            expect(status).toContain('⚠️');
        });

        it('should show check emoji when under limit', () => {
            const tracker = engagementLimits.createEngagementTracker({ likes: 10 });
            tracker.stats.likes = 2;

            const status = engagementLimits.formatLimitStatus(tracker);

            expect(status).toContain('✅');
        });
    });

    describe('shouldSkipAction', () => {
        it('should skip action based on random roll', () => {
            const modifiers = {};

            vi.spyOn(Math, 'random').mockReturnValue(0.1);
            const result = engagementLimits.shouldSkipAction('likes', 'active', modifiers);

            expect(result).toBe(false);
        });

        it('should skip action when roll exceeds modifier', () => {
            const modifiers = { active: { likes: 0.3 } };

            vi.spyOn(Math, 'random').mockReturnValue(0.5);
            const result = engagementLimits.shouldSkipAction('likes', 'active', modifiers);

            expect(result).toBe(true);
        });
    });

    describe('getSmartActionProbability', () => {
        it('should calculate probability based on remaining capacity', () => {
            const tracker = engagementLimits.createEngagementTracker({ likes: 10 });
            tracker.stats.likes = 5;

            const modifiers = { active: { likes: 1.0 } };
            const prob = engagementLimits.getSmartActionProbability(
                'likes',
                tracker,
                'active',
                modifiers
            );

            expect(prob).toBeLessThan(1.0);
            expect(prob).toBeGreaterThan(0);
        });

        it('should return minimum probability when exhausted', () => {
            const tracker = engagementLimits.createEngagementTracker({ likes: 10 });
            tracker.stats.likes = 10;

            const modifiers = { active: { likes: 1.0 } };
            const prob = engagementLimits.getSmartActionProbability(
                'likes',
                tracker,
                'active',
                modifiers
            );

            expect(prob).toBe(0.1); // Minimum 0.1
        });
    });

    describe('DEFAULT_LIMITS', () => {
        it('should have sensible defaults', () => {
            expect(engagementLimits.defaults.replies).toBe(3);
            expect(engagementLimits.defaults.likes).toBe(5);
            expect(engagementLimits.defaults.follows).toBe(2);
        });
    });

    describe('CRITICAL_THRESHOLDS', () => {
        it('should use 0.8 as default threshold', () => {
            expect(engagementLimits.thresholds.likes).toBe(0.8);
            expect(engagementLimits.thresholds.replies).toBe(0.8);
        });
    });
});
