/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Unit Tests for utils/api-key-timeout-tracker.js
 * @module tests/unit/api-key-timeout-tracker.test
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import ApiKeyTimeoutTracker from '@api/utils/api-key-timeout-tracker.js';

vi.mock('@api/core/logger.js', () => ({
    createLogger: vi.fn(() => ({
        info: vi.fn(),
        warn: vi.fn(),
        error: vi.fn(),
        debug: vi.fn(),
    })),
}));

describe('utils/api-key-timeout-tracker', () => {
    let tracker;

    beforeEach(() => {
        tracker = new ApiKeyTimeoutTracker({
            defaultTimeout: 30000,
            quickTimeout: 20000,
            slowThreshold: 15000,
            failureThreshold: 3,
        });
    });

    describe('Constructor', () => {
        it('should initialize with default options', () => {
            const defaultTracker = new ApiKeyTimeoutTracker();
            expect(defaultTracker.defaultTimeout).toBe(30000);
            expect(defaultTracker.quickTimeout).toBe(20000);
            expect(defaultTracker.slowThreshold).toBe(15000);
            expect(defaultTracker.failureThreshold).toBe(3);
        });

        it('should initialize with custom options', () => {
            expect(tracker.defaultTimeout).toBe(30000);
            expect(tracker.quickTimeout).toBe(20000);
            expect(tracker.slowThreshold).toBe(15000);
            expect(tracker.failureThreshold).toBe(3);
            expect(tracker.keyStats).toBeInstanceOf(Map);
            expect(tracker.timeoutHistory).toEqual([]);
        });
    });

    describe('trackRequest', () => {
        it('should create new stats for unknown key', () => {
            const result = tracker.trackRequest('test-api-key-12345', 10000, true);

            expect(result.apiKey).toBe('test...2345');
            expect(result.requestCount).toBe(1);
            expect(result.successCount).toBe(1);
            expect(result.failureCount).toBe(0);
        });

        it('should track successful request', () => {
            tracker.trackRequest('key1', 10000, true);
            const stats = tracker.getKeyStats('key1');

            expect(stats.requestCount).toBe(1);
            expect(stats.successCount).toBe(1);
            expect(stats.failureCount).toBe(0);
        });

        it('should track failed request', () => {
            tracker.trackRequest('key1', 5000, false);
            const stats = tracker.getKeyStats('key1');

            expect(stats.requestCount).toBe(1);
            expect(stats.successCount).toBe(0);
            expect(stats.failureCount).toBe(1);
        });

        it('should count timeouts when duration exceeds slowThreshold', () => {
            tracker.trackRequest('key1', 20000, true); // > 15000
            const stats = tracker.getKeyStats('key1');

            expect(stats.timeoutCount).toBe(1);
        });

        it('should not count timeout when duration is below threshold', () => {
            tracker.trackRequest('key1', 10000, true); // < 15000
            const stats = tracker.getKeyStats('key1');

            expect(stats.timeoutCount).toBe(0);
        });

        it('should calculate average duration', () => {
            tracker.trackRequest('key1', 10000, true);
            tracker.trackRequest('key1', 20000, true);

            const stats = tracker.getKeyStats('key1');
            expect(stats.avgDuration).toBe(15000);
        });

        it('should limit recent durations to 20', () => {
            for (let i = 0; i < 25; i++) {
                tracker.trackRequest('key1', 10000, true);
            }

            const stats = tracker.getKeyStats('key1');
            expect(stats.recentDurations.length).toBe(20);
        });

        it('should mark key as slow when avg exceeds threshold', () => {
            tracker.trackRequest('key1', 20000, true);
            tracker.trackRequest('key1', 20000, true);

            const stats = tracker.getKeyStats('key1');
            expect(stats.isSlow).toBe(true);
        });

        it('should mark key as problematic after failure threshold', () => {
            tracker.trackRequest('key1', 5000, false);
            tracker.trackRequest('key1', 5000, false);
            tracker.trackRequest('key1', 5000, false);

            const stats = tracker.getKeyStats('key1');
            expect(stats.isProblematic).toBe(true);
        });

        it('should handle null apiKey', () => {
            const result = tracker.trackRequest(null, 5000, true);
            expect(result.apiKey).toBe('unknown');
        });

        it('should handle empty apiKey', () => {
            const result = tracker.trackRequest('', 5000, true);
            expect(result.apiKey).toBe('unknown');
        });

        it('should add entry to history', () => {
            tracker.trackRequest('key1', 5000, true);

            const history = tracker.getHistory();
            expect(history.length).toBe(1);
            expect(history[0].apiKey).toBe('key1...key1');
            expect(history[0].duration).toBe(5000);
            expect(history[0].success).toBe(true);
        });

        it('should limit history to 1000 entries', () => {
            for (let i = 0; i < 1100; i++) {
                tracker.trackRequest('key' + i, 5000, true);
            }

            const history = tracker.getHistory();
            expect(history.length).toBe(50); // default limit
        });
    });

    describe('getTimeoutForKey', () => {
        it('should return defaultTimeout for unknown key', () => {
            const timeout = tracker.getTimeoutForKey('unknown-key');
            expect(timeout).toBe(30000);
        });

        it('should return quickTimeout for problematic key', () => {
            tracker.trackRequest('key1', 5000, false);
            tracker.trackRequest('key1', 5000, false);
            tracker.trackRequest('key1', 5000, false);

            const timeout = tracker.getTimeoutForKey('key1');
            expect(timeout).toBe(20000);
        });

        it('should return calculated timeout for slow key', () => {
            tracker.trackRequest('key1', 20000, true);
            tracker.trackRequest('key1', 20000, true);

            const timeout = tracker.getTimeoutForKey('key1');
            expect(timeout).toBeLessThan(30000);
        });

        it('should return defaultTimeout for healthy key', () => {
            tracker.trackRequest('key1', 5000, true);

            const timeout = tracker.getTimeoutForKey('key1');
            expect(timeout).toBe(30000);
        });
    });

    describe('shouldSkipKey', () => {
        it('should return false for unknown key', () => {
            expect(tracker.shouldSkipKey('unknown')).toBe(false);
        });

        it('should return false for non-problematic key', () => {
            tracker.trackRequest('key1', 5000, true);
            expect(tracker.shouldSkipKey('key1')).toBe(false);
        });

        it('should return true for problematic key with few recent requests', () => {
            tracker.trackRequest('key1', 5000, false);
            tracker.trackRequest('key1', 5000, false);
            tracker.trackRequest('key1', 5000, false);

            // Not enough recent durations to recover
            expect(tracker.shouldSkipKey('key1')).toBe(true);
        });

        it('should return false for problematic key with enough recent requests', () => {
            for (let i = 0; i < 5; i++) {
                tracker.trackRequest('key1', 5000, true);
            }
            tracker.trackRequest('key1', 5000, false);
            tracker.trackRequest('key1', 5000, false);
            tracker.trackRequest('key1', 5000, false);

            // Has enough recent successful requests
            expect(tracker.shouldSkipKey('key1')).toBe(false);
        });
    });

    describe('getKeyStats', () => {
        it('should return null for unknown key', () => {
            expect(tracker.getKeyStats('unknown')).toBeNull();
        });

        it('should return stats for known key', () => {
            tracker.trackRequest('key1', 5000, true);
            const stats = tracker.getKeyStats('key1');

            expect(stats).not.toBeNull();
            expect(stats.requestCount).toBe(1);
        });
    });

    describe('getAllStats', () => {
        it('should return empty object when no keys tracked', () => {
            expect(tracker.getAllStats()).toEqual({});
        });

        it('should return stats for all tracked keys', () => {
            tracker.trackRequest('key1', 5000, true);
            tracker.trackRequest('key2', 10000, true);

            const allStats = tracker.getAllStats();
            expect(Object.keys(allStats).length).toBe(2);
            expect(allStats['key1...key1']).toBeDefined();
            expect(allStats['key2...key2']).toBeDefined();
        });

        it('should format avgDuration as string with ms', () => {
            tracker.trackRequest('key1', 5000, true);

            const allStats = tracker.getAllStats();
            expect(allStats['key1...key1'].avgDuration).toBe('5000ms');
        });
    });

    describe('getSlowKeys', () => {
        it('should return empty array when no slow keys', () => {
            tracker.trackRequest('key1', 5000, true);
            expect(tracker.getSlowKeys()).toEqual([]);
        });

        it('should return slow keys sorted by avgDuration', () => {
            tracker.trackRequest('slow-key', 25000, true);
            tracker.trackRequest('fast-key', 5000, true);

            const slowKeys = tracker.getSlowKeys();
            expect(slowKeys.length).toBe(1);
            expect(slowKeys[0].key).toBe('slow...-key');
        });
    });

    describe('getProblematicKeys', () => {
        it('should return empty array when no problematic keys', () => {
            tracker.trackRequest('key1', 5000, true);
            expect(tracker.getProblematicKeys()).toEqual([]);
        });

        it('should return problematic keys with failure count', () => {
            tracker.trackRequest('key1', 5000, false);
            tracker.trackRequest('key1', 5000, false);
            tracker.trackRequest('key1', 5000, false);

            const problematic = tracker.getProblematicKeys();
            expect(problematic.length).toBe(1);
            expect(problematic[0].failureCount).toBe(3);
        });
    });

    describe('getHistory', () => {
        it('should return default 50 entries', () => {
            for (let i = 0; i < 100; i++) {
                tracker.trackRequest('key1', 5000, true);
            }

            const history = tracker.getHistory();
            expect(history.length).toBe(50);
        });

        it('should respect custom limit', () => {
            for (let i = 0; i < 20; i++) {
                tracker.trackRequest('key1', 5000, true);
            }

            const history = tracker.getHistory(10);
            expect(history.length).toBe(10);
        });

        it('should return entries in reverse chronological order', () => {
            tracker.trackRequest('key1', 1000, true);
            tracker.trackRequest('key1', 2000, true);

            const history = tracker.getHistory();
            expect(history[0].duration).toBe(2000);
            expect(history[1].duration).toBe(1000);
        });
    });

    describe('reset', () => {
        it('should reset specific key stats', () => {
            tracker.trackRequest('key1', 5000, true);
            tracker.trackRequest('key2', 5000, true);

            tracker.reset('key1');

            expect(tracker.getKeyStats('key1')).toBeNull();
            expect(tracker.getKeyStats('key2')).not.toBeNull();
        });

        it('should reset all stats when no key provided', () => {
            tracker.trackRequest('key1', 5000, true);
            tracker.trackRequest('key2', 5000, true);

            tracker.reset();

            expect(tracker.getKeyStats('key1')).toBeNull();
            expect(tracker.getKeyStats('key2')).toBeNull();
            expect(tracker.timeoutHistory).toEqual([]);
        });

        it('should handle reset for unknown key gracefully', () => {
            expect(() => tracker.reset('unknown-key')).not.toThrow();
        });
    });

    describe('getStats', () => {
        it('should return aggregate statistics', () => {
            tracker.trackRequest('key1', 5000, true);
            tracker.trackRequest('key1', 5000, false);
            tracker.trackRequest('key1', 5000, false);
            tracker.trackRequest('key1', 5000, false);
            tracker.trackRequest('key2', 25000, true); // slow

            const stats = tracker.getStats();

            expect(stats.trackedKeys).toBe(2);
            expect(stats.totalRequests).toBe(5);
            expect(stats.totalFailures).toBe(3);
            expect(stats.slowKeys).toBe(1);
            expect(stats.problematicKeys).toBe(1);
        });

        it('should return zeros for empty tracker', () => {
            const stats = tracker.getStats();

            expect(stats.trackedKeys).toBe(0);
            expect(stats.totalRequests).toBe(0);
            expect(stats.totalFailures).toBe(0);
            expect(stats.slowKeys).toBe(0);
            expect(stats.problematicKeys).toBe(0);
        });
    });

    describe('Edge Cases', () => {
        it('should handle very short apiKey', () => {
            const result = tracker.trackRequest('ab', 5000, true);
            // For keys shorter than 8 chars, it takes all from start and tries end
            expect(result.apiKey).toBe('ab...ab');
        });

        it('should handle apiKey shorter than 8 characters', () => {
            const result = tracker.trackRequest('1234567', 5000, true);
            // 4 chars from start + ... + last 4 chars
            expect(result.apiKey).toBe('1234...4567');
        });

        it('should handle negative duration', () => {
            const result = tracker.trackRequest('key1', -1000, true);
            expect(result.avgDuration).toBe(-1000);
        });

        it('should handle zero duration', () => {
            const result = tracker.trackRequest('key1', 0, true);
            expect(result.avgDuration).toBe(0);
        });

        it('should handle custom thresholds', () => {
            const customTracker = new ApiKeyTimeoutTracker({
                slowThreshold: 5000,
                failureThreshold: 1,
            });

            customTracker.trackRequest('key1', 3000, true); // < 5000, not slow
            customTracker.trackRequest('key1', 3000, false); // 1 failure

            const stats = customTracker.getKeyStats('key1');
            expect(stats.isSlow).toBe(false);
            expect(stats.isProblematic).toBe(true);
        });
    });
});
