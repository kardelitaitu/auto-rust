/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Memory Profiler
 * Tracks and analyzes memory usage for browser automation sessions.
 * @module api/utils/memory-profiler
 */

import { createLogger } from '../core/logger.js';

const logger = createLogger('api/utils/memory-profiler.js');

const DEFAULT_INTERVAL_MS = 30000;
const SNAPSHOT_LIMIT = 10;

class MemoryProfiler {
    #trackingInterval = null;
    #snapshots = [];
    #baseline = null;
    #enabled = false;
    #intervalMs = DEFAULT_INTERVAL_MS;

    constructor() {
        this.#baseline = this.getUsage();
    }

    getUsage() {
        const usage = process.memoryUsage();
        return {
            heapUsed: Math.round(usage.heapUsed / 1024 / 1024),
            heapTotal: Math.round(usage.heapTotal / 1024 / 1024),
            external: Math.round(usage.external / 1024 / 1024),
            rss: Math.round(usage.rss / 1024 / 1024),
            timestamp: Date.now(),
        };
    }

    getSnapshot() {
        const current = this.getUsage();
        const delta = this.#baseline
            ? {
                  heapUsed: current.heapUsed - this.#baseline.heapUsed,
                  heapTotal: current.heapTotal - this.#baseline.heapTotal,
                  external: current.external - this.#baseline.external,
                  rss: current.rss - this.#baseline.rss,
              }
            : null;

        const snapshot = { current, delta, timestamp: Date.now() };

        this.#snapshots.push(snapshot);
        if (this.#snapshots.length > SNAPSHOT_LIMIT) {
            this.#snapshots.shift();
        }

        return snapshot;
    }

    startTracking(intervalMs = DEFAULT_INTERVAL_MS) {
        if (this.#enabled) {
            logger.warn('Memory tracking already started');
            return;
        }

        this.#intervalMs = intervalMs;
        this.#enabled = true;
        this.#baseline = this.getUsage();
        this.#snapshots = [];

        this.#trackingInterval = setInterval(() => {
            const snapshot = this.getSnapshot();
            logger.info(
                `[memory] heap: ${snapshot.current.heapUsed}MB (${snapshot.delta?.heapUsed >= 0 ? '+' : ''}${snapshot.delta?.heapUsed}MB) rss: ${snapshot.current.rss}MB`
            );
        }, this.#intervalMs);

        logger.info(`Memory tracking started (interval: ${this.#intervalMs}ms)`);
    }

    stopTracking() {
        if (!this.#enabled) {
            return;
        }

        this.#enabled = false;
        if (this.#trackingInterval) {
            clearInterval(this.#trackingInterval);
            this.#trackingInterval = null;
        }
        logger.info('Memory tracking stopped');
    }

    getSnapshots() {
        return [...this.#snapshots];
    }

    resetBaseline() {
        this.#baseline = this.getUsage();
        logger.info('Memory baseline reset');
    }

    detectLeaks(thresholdMB = 50) {
        if (this.#snapshots.length < 2) {
            return { hasLeak: false, reason: 'Insufficient snapshots' };
        }

        const first = this.#snapshots[0];
        const last = this.#snapshots[this.#snapshots.length - 1];
        const growth = last.current.heapUsed - first.current.heapUsed;

        if (growth > thresholdMB) {
            return {
                hasLeak: true,
                growthMB: growth,
                thresholdMB,
                snapshotCount: this.#snapshots.length,
            };
        }

        return { hasLeak: false, growthMB: growth };
    }

    get isTracking() {
        return this.#enabled;
    }

    dispose() {
        this.stopTracking();
        this.#snapshots = [];
        this.#baseline = null;
    }
}

const globalProfiler = new MemoryProfiler();

export const memory = {
    getUsage: () => globalProfiler.getUsage(),
    getSnapshot: () => globalProfiler.getSnapshot(),
    startTracking: (intervalMs) => globalProfiler.startTracking(intervalMs),
    stopTracking: () => globalProfiler.stopTracking(),
    getSnapshots: () => globalProfiler.getSnapshots(),
    resetBaseline: () => globalProfiler.resetBaseline(),
    detectLeaks: (thresholdMB) => globalProfiler.detectLeaks(thresholdMB),
    get isTracking() {
        return globalProfiler.isTracking;
    },
    dispose: () => globalProfiler.dispose(),
};

export { MemoryProfiler };
