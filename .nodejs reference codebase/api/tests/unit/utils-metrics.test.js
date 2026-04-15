/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';

vi.mock('@api/core/logger.js', () => ({
    createLogger: () => ({
        info: vi.fn(),
        debug: vi.fn(),
        warn: vi.fn(),
        error: vi.fn(),
    }),
}));

describe('api/utils/metrics.js', () => {
    let MetricsCollector;
    let metricsCollector;

    beforeEach(async () => {
        vi.clearAllMocks();
        const module = await import('@api/utils/metrics.js');
        MetricsCollector = module.MetricsCollector;
        metricsCollector = module.default;
    });

    describe('MetricsCollector class', () => {
        it('should be defined', () => {
            expect(MetricsCollector).toBeDefined();
        });

        it('should create instance with default metrics', () => {
            const collector = new MetricsCollector();
            expect(collector.metrics.tasksExecuted).toBe(0);
            expect(collector.metrics.tasksFailed).toBe(0);
            expect(collector.metrics.apiCalls).toBe(0);
        });
    });

    describe('metricsCollector singleton', () => {
        it('should be defined', () => {
            expect(metricsCollector).toBeDefined();
        });

        it('should have recordTaskExecution method', () => {
            expect(typeof metricsCollector.recordTaskExecution).toBe('function');
        });

        it('should have recordApiCall method', () => {
            expect(typeof metricsCollector.recordApiCall).toBe('function');
        });

        it('should have recordSocialAction method', () => {
            expect(typeof metricsCollector.recordSocialAction).toBe('function');
        });

        it('should have getStats method', () => {
            expect(typeof metricsCollector.getStats).toBe('function');
        });

        it('should have reset method', () => {
            expect(typeof metricsCollector.reset).toBe('function');
        });

        it('should record task execution', () => {
            metricsCollector.recordTaskExecution('test-task', 100, true, 'session1');
            // Task was recorded - no error thrown
        });

        it('should record API call', () => {
            metricsCollector.recordApiCall(50, true);
            // API call was recorded - no error thrown
        });

        it('should record social action', () => {
            metricsCollector.recordSocialAction('like');
            // Social action was recorded - no error thrown
        });

        it('should reset all metrics', () => {
            metricsCollector.recordTaskExecution('test-task', 100, true, 'session1');
            metricsCollector.reset();
            // Reset completed - no error thrown
        });

        it('should return stats object', () => {
            const stats = metricsCollector.getStats();
            expect(stats).toBeDefined();
            expect(typeof stats).toBe('object');
        });

        it('should export to JSON', () => {
            const json = metricsCollector.exportToJSON();
            expect(json).toBeDefined();
        });
    });
});
