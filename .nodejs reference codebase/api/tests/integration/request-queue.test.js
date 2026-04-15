/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Unit tests for RequestQueue
 * @module tests/integration/request-queue.test
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import RequestQueue from '@api/core/request-queue.js';

vi.mock('@api/utils/configLoader.js', () => ({
    getTimeoutValue: vi.fn().mockResolvedValue({}),
}));

describe('RequestQueue', () => {
    let queue;

    beforeEach(() => {
        queue = new RequestQueue({
            maxConcurrent: 2,
            maxRetries: 2,
            retryDelay: 50,
        });
    });

    describe('Basic Operations', () => {
        it('should enqueue and execute tasks', async () => {
            const result = await queue.enqueue(async () => 'success');
            expect(result.success).toBe(true);
            expect(result.data).toBe('success');
        });

        it('should track statistics', async () => {
            await queue.enqueue(async () => 'task1');
            await queue.enqueue(async () => 'task2');

            const stats = queue.getStats();
            expect(stats.enqueued).toBe(2);
            expect(stats.completed).toBe(2);
        });
    });

    describe('Concurrency Control', () => {
        it('should limit concurrent execution', async () => {
            let running = 0;
            let maxRunning = 0;

            const tasks = Array(5)
                .fill()
                .map(() =>
                    queue.enqueue(async () => {
                        running++;
                        maxRunning = Math.max(maxRunning, running);
                        await new Promise((r) => setTimeout(r, 50));
                        running--;
                        return 'done';
                    })
                );

            await Promise.all(tasks);
            expect(maxRunning).toBeLessThanOrEqual(2);
        });
    });
});
