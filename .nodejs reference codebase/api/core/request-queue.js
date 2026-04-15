/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Request Queue with Concurrency Limits and Retry Logic
 * @module core/request-queue
 */

import { createLogger } from '../core/logger.js';
import { calculateBackoffDelay } from '../utils/retry.js';
import { getTimeoutValue } from '../utils/configLoader.js';

const logger = createLogger('request-queue.js');

/**
 * @class RequestQueue
 * @description Manages queued requests with concurrency control and exponential backoff retry
 */
class RequestQueue {
    constructor(options = {}) {
        // Set defaults synchronously first
        this.maxConcurrent = options.maxConcurrent ?? 3;
        this.retryDelay = options.retryDelay ?? 1000;
        this.maxRetries = options.maxRetries ?? 3;
        this.maxQueueSize = options.maxQueueSize ?? 100;
        this.intervalMs = options.intervalMs ?? 0;

        this.running = 0;
        this.queue = [];
        this.pending = new Map();
        this.lastStartAt = 0;
        this.stats = {
            enqueued: 0,
            dequeued: 0,
            completed: 0,
            failed: 0,
            retried: 0,
        };

        // Then async load to override with config values
        this._configLoaded = false;
        this._loadConfig(options);
    }

    async _loadConfig(_options = {}) {
        if (this._configLoaded) return;

        const qConfig = await getTimeoutValue('requestQueue', {});

        if (qConfig.maxConcurrent !== undefined) this.maxConcurrent = qConfig.maxConcurrent;
        if (qConfig.retryDelay !== undefined) this.retryDelay = qConfig.retryDelay;
        if (qConfig.maxRetries !== undefined) this.maxRetries = qConfig.maxRetries;
        if (qConfig.maxQueueSize !== undefined) this.maxQueueSize = qConfig.maxQueueSize;
        if (qConfig.intervalMs !== undefined) this.intervalMs = qConfig.intervalMs;

        this._configLoaded = true;
    }

    /**
     * Add a request to the queue
     * @param {Function} taskFn - Async function to execute
     * @param {object} options - Queue options
     * @returns {Promise<object>} Result or error
     */
    async enqueue(taskFn, options = {}) {
        const id = `req_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
        const priority = options.priority || 0;
        const QUEUE_WAIT_TIMEOUT_MS = 90000; // 90 seconds max wait in queue

        if (this.queue.length >= this.maxQueueSize) {
            logger.warn(
                `[RequestQueue] Queue full, rejecting task ${id} (size: ${this.queue.length})`
            );
            throw new Error(`Queue full (max: ${this.maxQueueSize})`);
        }

        this.stats.enqueued++;
        const enqueueTime = Date.now();
        logger.debug(
            `[RequestQueue] Enqueued task ${id}, queue size: ${this.queue.length + 1}, running: ${this.running}`
        );

        return new Promise((resolve, reject) => {
            // Add queue wait timeout
            const queueTimeoutId = setTimeout(() => {
                // Check if still in queue
                const idx = this.queue.findIndex((item) => item.id === id);
                if (idx !== -1) {
                    this.queue.splice(idx, 1);
                    this.stats.failed++;
                    logger.warn(
                        `[RequestQueue] Task ${id} timed out waiting in queue after ${Date.now() - enqueueTime}ms`
                    );
                    reject(new Error(`Queue wait timeout after ${QUEUE_WAIT_TIMEOUT_MS}ms`));
                }
            }, QUEUE_WAIT_TIMEOUT_MS);

            const wrappedResolve = (val) => {
                clearTimeout(queueTimeoutId);
                resolve(val);
            };
            const wrappedReject = (err) => {
                clearTimeout(queueTimeoutId);
                reject(err);
            };

            this.queue.push({
                id,
                taskFn,
                priority,
                retries: 0,
                resolve: wrappedResolve,
                reject: wrappedReject,
                createdAt: enqueueTime,
                options,
            });

            this.queue.sort((a, b) => b.priority - a.priority);
            this._processQueue();
        });
    }

    /**
     * Process next item in queue
     * @private
     */
    _processQueue() {
        if (this.paused) return;

        while (this.running < this.maxConcurrent && this.queue.length > 0) {
            const item = this.queue.shift();
            logger.debug(
                `[RequestQueue] Dequeueing task ${item.id}, queue now has ${this.queue.length} items`
            );
            this._executeTask(item);
        }

        if (this.queue.length > 0 && this.running >= this.maxConcurrent) {
            logger.debug(
                `[RequestQueue] Queue blocked: ${this.queue.length} waiting, ${this.running} running, max=${this.maxConcurrent}`
            );
        }
    }

    /**
     * Execute a queued task with retry logic
     * @private
     */
    async _executeTask(item) {
        this.running++;
        const taskId = item.id;
        logger.debug(
            `[RequestQueue] Task ${taskId} starting (running: ${this.running}/${this.maxConcurrent}, queued: ${this.queue.length})`
        );

        let startTime = Date.now();

        try {
            await this._waitForInterval();
            startTime = Date.now();
            const result = await this._executeWithRetry(item);
            this.stats.completed++;
            logger.debug(`[RequestQueue] Task ${taskId} completed successfully`);
            item.resolve({
                success: true,
                data: result,
                duration: Date.now() - startTime,
                attempts: item.retries + 1,
            });
        } catch (error) {
            this.stats.failed++;
            logger.debug(`[RequestQueue] Task ${taskId} failed: ${error.message}`);
            item.reject({
                success: false,
                error: error.message,
                attempts: item.retries + 1,
                duration: Date.now() - startTime,
            });
        } finally {
            this.running--;
            logger.debug(`[RequestQueue] Task ${taskId} finished, running: ${this.running}`);
            this._processQueue();
        }
    }

    /**
     * Execute task with exponential backoff retry
     * @private
     */
    async _executeWithRetry(item) {
        const { taskFn, options } = item;
        const maxRetries = options.maxRetries || this.maxRetries;

        for (let attempt = 0; attempt <= maxRetries; attempt++) {
            item.retries = attempt;

            try {
                return await taskFn();
            } catch (error) {
                if (this._isFatalError(error)) {
                    throw error;
                }

                const isRetryable = this._isRetryableError(error);
                const shouldRetry = attempt < maxRetries && isRetryable;

                if (!shouldRetry) {
                    throw error;
                }

                this.stats.retried++;
                const delay = this._calculateBackoff(attempt);

                logger.warn(
                    `[${item.id}] Attempt ${attempt + 1} failed: ${error.message}. Retrying in ${delay}ms`
                );

                await this._sleep(delay);
            }
        }

        throw new Error(`Max retries (${maxRetries}) exceeded`);
    }

    /**
     * Check if error is retryable
     * @private
     */
    _isRetryableError(error) {
        const retryableMessages = [
            'timeout',
            'econnreset',
            'econnrefused',
            'etimedout',
            'socket hang up',
            'network error',
            'temporary failure',
            'service unavailable',
            '429',
            '503',
            '502',
            'ENOTFOUND',
        ];

        const errorMessage = (error.message || '').toLowerCase();
        return retryableMessages.some((msg) => errorMessage.includes(msg));
    }

    _isFatalError(error) {
        if (!error) {
            return false;
        }
        if (error.code === 'CIRCUIT_OPEN') {
            return true;
        }
        return error.fatal === true;
    }

    /**
     * Calculate exponential backoff delay
     * @private
     */
    _calculateBackoff(attempt) {
        return calculateBackoffDelay(attempt, {
            baseDelay: this.retryDelay,
            maxDelay: 30000,
            factor: 2,
            jitterMin: 1,
            jitterMax: 1.3,
        });
    }

    /**
     * Sleep utility
     * @private
     */
    _sleep(ms) {
        return new Promise((resolve) => setTimeout(resolve, ms));
    }

    async _waitForInterval() {
        if (!this.intervalMs || this.intervalMs <= 0) {
            this.lastStartAt = Date.now();
            return;
        }

        const now = Date.now();
        const elapsed = this.lastStartAt ? now - this.lastStartAt : this.intervalMs;
        if (elapsed < this.intervalMs) {
            await this._sleep(this.intervalMs - elapsed);
        }
        this.lastStartAt = Date.now();
    }

    /**
     * Get queue statistics
     * @returns {object}
     */
    getStats() {
        return {
            ...this.stats,
            running: this.running,
            queued: this.queue.length,
            utilization: this.maxConcurrent > 0 ? this.running / this.maxConcurrent : 0,
        };
    }

    /**
     * Clear all pending items
     */
    clear() {
        const count = this.queue.length;
        this.queue.forEach((item) => {
            item.reject(new Error('Queue cleared'));
        });
        this.queue = [];

        logger.info(`Cleared ${count} queued items`);
    }

    /**
     * Pause queue processing
     */
    pause() {
        this.paused = true;
        logger.info('Queue processing paused');
    }

    /**
     * Resume queue processing
     */
    resume() {
        if (!this.paused) return;
        this.paused = false;
        this._processQueue();
        logger.info('Queue processing resumed');
    }
}

export default RequestQueue;
