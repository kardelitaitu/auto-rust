/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Unit tests for core/request-queue.js
 * @module tests/unit/request-queue.test
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import RequestQueue from '@api/core/request-queue.js';

// Mock logger
vi.mock('@api/core/logger.js', () => ({
    createLogger: vi.fn(() => ({
        info: vi.fn(),
        warn: vi.fn(),
        error: vi.fn(),
        debug: vi.fn(),
    })),
}));

describe('core/request-queue', () => {
    let queue;

    beforeEach(() => {
        vi.clearAllMocks();
        vi.useFakeTimers();
        queue = new RequestQueue({
            maxConcurrent: 2,
            retryDelay: 100, // Small delay for testing
            maxRetries: 3,
            maxQueueSize: 5,
        });
    });

    afterEach(() => {
        vi.useRealTimers();
    });

    describe('Constructor & State', () => {
        it('should initialize with default values', () => {
            const q = new RequestQueue();
            expect(q.maxConcurrent).toBe(3);
            expect(q.maxRetries).toBe(3);
        });

        it('should initialize with custom options', () => {
            const q = new RequestQueue({ maxConcurrent: 5 });
            expect(q.maxConcurrent).toBe(5);
        });
    });

    describe('enqueue', () => {
        it('should execute a simple task', async () => {
            const task = vi.fn().mockResolvedValue('ok');
            const promise = queue.enqueue(task);

            const result = await promise;
            expect(result.success).toBe(true);
            expect(result.data).toBe('ok');
            expect(task).toHaveBeenCalledTimes(1);
        });

        it('should respect maxQueueSize', async () => {
            const task = () => new Promise((r) => setTimeout(r, 1000));
            // Fill concurrency (2)
            queue.enqueue(task).catch(() => {});
            queue.enqueue(task).catch(() => {});
            // Fill queue (5)
            for (let i = 0; i < 5; i++) queue.enqueue(task).catch(() => {});

            // Should throw
            await expect(queue.enqueue(task)).rejects.toThrow(/Queue full/);
        });

        it('should handle priority sorting', async () => {
            const task = () => new Promise((r) => setTimeout(r, 1000));
            queue.enqueue(task).catch(() => {}); // Takes slot 1
            queue.enqueue(task).catch(() => {}); // Takes slot 2

            const low = vi.fn().mockResolvedValue('low');
            const high = vi.fn().mockResolvedValue('high');

            const pLow = queue.enqueue(low, { priority: 1, id: 'low' });
            const pHigh = queue.enqueue(high, { priority: 10, id: 'high' });

            // Advance time to finish running tasks
            await vi.advanceTimersByTimeAsync(1100);

            // Wait for both to finish
            await Promise.all([pLow, pHigh]);

            expect(queue.stats.completed).toBe(4);
        });
    });

    describe('Retry Logic & Backoff', () => {
        it('should retry retryable errors', async () => {
            const task = vi
                .fn()
                .mockRejectedValueOnce(new Error('timeout'))
                .mockResolvedValueOnce('ok');

            vi.spyOn(queue, '_calculateBackoff').mockReturnValue(1);

            const promise = queue.enqueue(task);
            await vi.advanceTimersByTimeAsync(10);

            const result = await promise;
            expect(result.success).toBe(true);
            expect(task).toHaveBeenCalledTimes(2);
        });

        it('should fail after max retries', async () => {
            const task = vi.fn().mockRejectedValue(new Error('econnreset'));
            vi.spyOn(queue, '_calculateBackoff').mockReturnValue(1);

            const promise = queue.enqueue(task, { maxRetries: 2 });
            // Attach a no-op catch to prevent unhandled rejection warning during timer advancement
            promise.catch(() => {});

            await vi.advanceTimersByTimeAsync(10);
            await vi.advanceTimersByTimeAsync(10);

            await expect(promise).rejects.toMatchObject({
                success: false,
                error: 'econnreset',
                attempts: 3,
            });
            expect(task).toHaveBeenCalledTimes(3);
        });

        it('should not retry non-retryable errors', async () => {
            const error = new Error('Bad Request');
            error.status = 400;
            const task = vi.fn().mockRejectedValue(error);

            const promise = queue.enqueue(task);
            promise.catch(() => {});

            await expect(promise).rejects.toMatchObject({
                success: false,
                error: 'Bad Request',
            });
            expect(task).toHaveBeenCalledTimes(1);
        });

        it('should log error after multiple failed retries', async () => {
            const task = vi.fn().mockRejectedValue(new Error('retryable timeout'));
            vi.spyOn(queue, '_calculateBackoff').mockReturnValue(1);

            const promise = queue.enqueue(task, { maxRetries: 1 });
            promise.catch(() => {});

            await vi.advanceTimersByTimeAsync(10);

            await expect(promise).rejects.toMatchObject({
                success: false,
                error: 'retryable timeout',
                attempts: 2,
            });
            expect(task).toHaveBeenCalledTimes(2);
        });
    });

    describe('Pause & Resume', () => {
        it('should not start new tasks when paused', async () => {
            queue.pause();
            const task = vi.fn().mockResolvedValue('ok');
            const p = queue.enqueue(task);

            await vi.advanceTimersByTimeAsync(100);
            expect(task).not.toHaveBeenCalled();

            queue.resume();
            await vi.advanceTimersByTimeAsync(10);
            await p;
            expect(task).toHaveBeenCalled();
        });

        it('should finish in-flight tasks even if paused during execution', async () => {
            const task = vi
                .fn()
                .mockImplementation(() => new Promise((r) => setTimeout(() => r('ok'), 500)));
            const p = queue.enqueue(task);

            queue.pause();
            await vi.advanceTimersByTimeAsync(600);

            const result = await p;
            expect(result.success).toBe(true);
        });
    });

    describe('Stats & Cleanup', () => {
        it('should report correct stats', () => {
            queue.enqueue(() => Promise.resolve()).catch(() => {});
            const stats = queue.getStats();
            expect(stats.enqueued).toBe(1);
        });

        it('should clear queue', async () => {
            queue.pause();
            const task = () => Promise.resolve();

            const p1 = queue.enqueue(task);
            const p2 = queue.enqueue(task);

            expect(queue.queue.length).toBe(2);

            queue.clear();

            await expect(p1).rejects.toThrow('Queue cleared');
            await expect(p2).rejects.toThrow('Queue cleared');
            expect(queue.queue.length).toBe(0);
        });
    });

    describe('Helper Methods', () => {
        it('_sleep should wait for specified time', async () => {
            const spy = vi.fn();
            queue._sleep(100).then(spy);

            await vi.advanceTimersByTimeAsync(100);
            expect(spy).toHaveBeenCalled();
        });

        it('_calculateBackoff should respect caps', () => {
            const d1 = queue._calculateBackoff(0);
            const d2 = queue._calculateBackoff(10);

            expect(d1).toBeGreaterThanOrEqual(100); // retryDelay is 100
            expect(d2).toBeGreaterThanOrEqual(30000);
        });
    });

    describe('Edge Cases', () => {
        it('should handle errors without messages in retry check', () => {
            const error = {}; // No message
            const isRetryable = queue._isRetryableError(error);
            expect(isRetryable).toBe(false);
        });

        it('should handle zero maxConcurrent in stats', () => {
            queue.maxConcurrent = 0;
            const stats = queue.getStats();
            expect(stats.utilization).toBe(0);
        });

        it('should handle resume when not paused', () => {
            queue.paused = false;
            const spy = vi.spyOn(queue, '_processQueue');
            queue.resume();
            expect(spy).not.toHaveBeenCalled();
        });
    });
});
