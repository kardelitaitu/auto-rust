/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { MetricsCollector } from '@api/utils/metrics.js';

// Mock logger
vi.mock('@api/core/logger.js', () => ({
    createLogger: vi.fn(() => ({
        info: vi.fn(),
        warn: vi.fn(),
        error: vi.fn(),
        debug: vi.fn(),
    })),
}));

// Mock fs/promises for generateJsonReport
const mockWriteFile = vi.fn();
vi.mock('fs/promises', () => ({
    writeFile: mockWriteFile,
}));

describe('MetricsCollector', () => {
    let collector;

    beforeEach(() => {
        collector = new MetricsCollector();
        vi.clearAllMocks();
    });

    describe('Initialization', () => {
        it('should initialize with zero values', () => {
            const stats = collector.getStats();
            expect(stats.tasks.executed).toBe(0);
            expect(stats.social.likes).toBe(0);
            expect(collector.taskHistory).toEqual([]);
        });
    });

    describe('Task Recording', () => {
        it('should record successful tasks', () => {
            collector.recordTaskExecution('test-task', 100, true, 'session-1');
            const stats = collector.getStats();
            expect(stats.tasks.executed).toBe(1);
            expect(stats.tasks.succeeded).toBe(1);
            expect(stats.tasks.failed).toBe(0);
            expect(collector.taskHistory.length).toBe(1);
        });

        it('should record failed tasks', () => {
            collector.recordTaskExecution('test-task', 100, false, 'session-1', new Error('fail'));
            const stats = collector.getStats();
            expect(stats.tasks.executed).toBe(1);
            expect(stats.tasks.succeeded).toBe(0);
            expect(stats.tasks.failed).toBe(1);
            expect(collector.taskHistory[0].error).toBe('fail');
        });

        it('should limit task history size', () => {
            collector.maxHistorySize = 5;
            for (let i = 0; i < 10; i++) {
                collector.recordTaskExecution(`task-${i}`, 100, true, 's1');
            }
            expect(collector.taskHistory.length).toBe(5);
            expect(collector.taskHistory[4].taskName).toBe('task-9');
        });
    });

    describe('Social Actions', () => {
        it('should record valid social actions', () => {
            collector.recordSocialAction('like', 1);
            collector.recordSocialAction('follow', 2);
            collector.recordSocialAction('retweet', 1);
            collector.recordSocialAction('tweet', 1);

            const stats = collector.getStats();
            expect(stats.social.likes).toBe(1);
            expect(stats.social.follows).toBe(2);
            expect(stats.social.retweets).toBe(1);
            expect(stats.social.tweets).toBe(1);
        });

        it('should ignore invalid types', () => {
            collector.recordSocialAction('invalid', 1);
            const stats = collector.getStats();
            expect(stats.social.likes).toBe(0);
        });

        it('should handle count validation', () => {
            collector.recordSocialAction('like', -1); // Invalid count
            collector.recordSocialAction('like', 'string'); // Invalid count
            expect(collector.getStats().social.likes).toBe(0);
        });
    });

    describe('Twitter Engagement', () => {
        it('should record twitter specific engagements', () => {
            collector.recordTwitterEngagement('reply', 1);
            collector.recordTwitterEngagement('quote', 1);
            collector.recordTwitterEngagement('bookmark', 1);

            const stats = collector.getTwitterEngagementMetrics();
            expect(stats.actions.replies).toBe(1);
            expect(stats.actions.quotes).toBe(1);
            expect(stats.actions.bookmarks).toBe(1);
        });
    });

    describe('Performance Metrics', () => {
        it('should calculate averages for dive durations', () => {
            collector.recordDiveDuration(100);
            collector.recordDiveDuration(200);
            expect(collector.getAvgDiveDuration()).toBe(150);
        });

        it('should calculate averages for AI latency', () => {
            collector.recordAILatency(100);
            collector.recordAILatency(300);
            expect(collector.getAvgAILatency()).toBe(200);
        });
    });

    describe('Reporting', () => {
        it('should generate stats object correctly', () => {
            collector.recordTaskExecution('t1', 100, true, 's1');
            const stats = collector.getStats();
            expect(stats.tasks.successRate).toBe(100);
            expect(stats.tasks.avgDuration).toBe(100);
        });

        it('should reset metrics', () => {
            collector.recordTaskExecution('t1', 100, true, 's1');
            collector.reset();
            const stats = collector.getStats();
            expect(stats.tasks.executed).toBe(0);
            expect(collector.taskHistory.length).toBe(0);
        });

        it('should generate JSON report', async () => {
            await collector.generateJsonReport();
        });

        it('should export to JSON', () => {
            collector.recordTaskExecution('t1', 100, true, 's1');
            const json = collector.exportToJSON();
            expect(json).toContain('stats');
            expect(json).toContain('recentTasks');
        });

        it('should log stats', () => {
            collector.recordTaskExecution('t1', 100, true, 's1');
            expect(() => collector.logStats()).not.toThrow();
        });
    });

    describe('Browser Discovery', () => {
        it('should record browser discovery', () => {
            collector.recordBrowserDiscovery(5, 3, 2);
            const stats = collector.getStats();
            expect(stats.browsers.discovered).toBe(5);
            expect(stats.browsers.connected).toBe(3);
            expect(stats.browsers.failed).toBe(2);
        });
    });

    describe('API Calls', () => {
        it('should record successful API call', () => {
            collector.recordApiCall(150, true);
            const stats = collector.getStats();
            expect(stats.api.calls).toBe(1);
            expect(stats.api.failures).toBe(0);
            expect(stats.api.avgResponseTime).toBe(150);
        });

        it('should record failed API call', () => {
            collector.recordApiCall(150, false);
            const stats = collector.getStats();
            expect(stats.api.calls).toBe(1);
            expect(stats.api.failures).toBe(1);
        });
    });

    describe('Session Events', () => {
        it('should record session created', () => {
            collector.recordSessionEvent('created', 1);
            const stats = collector.getStats();
            expect(stats.sessions.created).toBe(1);
            expect(stats.sessions.active).toBe(1);
        });

        it('should record session closed', () => {
            collector.recordSessionEvent('created', 1);
            collector.recordSessionEvent('closed', 0);
            const stats = collector.getStats();
            expect(stats.sessions.closed).toBe(1);
            expect(stats.sessions.active).toBe(0);
        });
    });

    describe('Dive Duration', () => {
        it('should handle invalid dive duration', () => {
            collector.recordDiveDuration(-1);
            collector.recordDiveDuration(NaN);
            collector.recordDiveDuration('invalid');
            expect(collector.getAvgDiveDuration()).toBe(0);
        });
    });

    describe('AI Latency', () => {
        it('should handle invalid AI latency', () => {
            collector.recordAILatency(-1);
            collector.recordAILatency(NaN);
            collector.recordAILatency('invalid');
            expect(collector.getAvgAILatency()).toBe(0);
        });

        it('should record failed AI request', () => {
            collector.recordAILatency(100, false);
            const twitterStats = collector.getTwitterEngagementMetrics();
            expect(twitterStats.errors.ai_request_failure).toBeDefined();
        });
    });

    describe('Error Recording', () => {
        it('should record errors by type', () => {
            collector.recordError('test_error', 'Test message 1');
            collector.recordError('test_error', 'Test message 2');
            collector.recordError('test_error', 'Test message 3');
            collector.recordError('test_error', 'Test message 4');
            collector.recordError('test_error', 'Test message 5');
            collector.recordError('test_error', 'Test message 6'); // Should be ignored

            const twitterStats = collector.getTwitterEngagementMetrics();
            expect(twitterStats.errors.test_error.count).toBe(6);
            expect(twitterStats.errors.test_error.messages.length).toBe(5); // Max 5 messages
        });
    });

    describe('Task Breakdown', () => {
        it('should get task breakdown', () => {
            collector.recordTaskExecution('task1', 100, true, 's1');
            collector.recordTaskExecution('task1', 200, false, 's1');
            collector.recordTaskExecution('task2', 150, true, 's1');

            const breakdown = collector.getTaskBreakdown();
            expect(breakdown.task1.executions).toBe(2);
            expect(breakdown.task1.successes).toBe(1);
            expect(breakdown.task1.failures).toBe(1);
            expect(breakdown.task2.executions).toBe(1);
        });
    });

    describe('Recent Tasks', () => {
        it('should get recent tasks with limit', () => {
            for (let i = 0; i < 15; i++) {
                collector.recordTaskExecution(`task${i}`, 100, true, 's1');
            }

            const recent = collector.getRecentTasks(5);
            expect(recent.length).toBe(5);
            expect(recent[0].taskName).toBe('task14');
        });
    });

    describe('Twitter Engagement Edge Cases', () => {
        it('should handle invalid engagement type', () => {
            collector.recordTwitterEngagement('invalid', 1);
            collector.recordTwitterEngagement('reply', -1);
            collector.recordTwitterEngagement('reply', NaN);

            const stats = collector.getTwitterEngagementMetrics();
            expect(stats.actions.replies).toBe(0);
            expect(stats.errors).toBeDefined();
        });
    });

    describe('Social Action Edge Cases', () => {
        it('should handle invalid action type', () => {
            collector.recordSocialAction(123, 1);
            collector.recordSocialAction('like', 0);

            const stats = collector.getStats();
            expect(stats.social.likes).toBe(0);
        });
    });

    describe('Percentile Calculation', () => {
        it('should calculate percentiles', () => {
            collector.recordTaskExecution('t1', 10, true, 's1');
            collector.recordTaskExecution('t1', 20, true, 's1');
            collector.recordTaskExecution('t1', 30, true, 's1');
            collector.recordTaskExecution('t1', 40, true, 's1');
            collector.recordTaskExecution('t1', 50, true, 's1');

            const stats = collector.getStats();
            expect(stats.tasks.durationPercentiles.p50).toBe(30);
            expect(stats.tasks.durationPercentiles.p95).toBe(50);
            expect(stats.tasks.durationPercentiles.p99).toBe(50);
        });

        it('should handle empty durations', () => {
            const p50 = collector.getPercentile([], 50);
            expect(p50).toBe(0);
        });
    });

    describe('Duration Formatting', () => {
        it('should format milliseconds', () => {
            expect(collector.formatDuration(500)).toBe('500ms');
        });

        it('should format seconds', () => {
            expect(collector.formatDuration(5000)).toBe('5s');
        });

        it('should format minutes', () => {
            expect(collector.formatDuration(120000)).toBe('2m 0s');
        });

        it('should format hours', () => {
            expect(collector.formatDuration(3600000)).toBe('1h 0m 0s');
        });

        it('should format days', () => {
            expect(collector.formatDuration(90000000)).toBe('1d 1h 0m');
        });

        it('should handle edge cases', () => {
            expect(collector.formatDuration(0)).toBe('0ms');
            expect(collector.formatDuration(null)).toBe('0ms');
            expect(collector.formatDuration(-1)).toBe('0ms');
        });
    });
});
