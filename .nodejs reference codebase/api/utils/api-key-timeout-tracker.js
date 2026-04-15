/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * API Key Timeout Tracker
 * Tracks response times per API key for quick timeout management
 * @module utils/api-key-timeout-tracker
 */

import { createLogger } from '../core/logger.js';

const logger = createLogger('api-key-timeout-tracker.js');

export class ApiKeyTimeoutTracker {
    constructor(options = {}) {
        this.defaultTimeout = options.defaultTimeout || 30000;
        this.quickTimeout = options.quickTimeout || 20000;
        this.slowThreshold = options.slowThreshold || 15000;
        this.failureThreshold = options.failureThreshold || 3;

        this.keyStats = new Map();
        this.timeoutHistory = [];
    }

    trackRequest(apiKey, duration, success) {
        const key = this._getKeyIdentifier(apiKey);
        let stats = this.keyStats.get(key);

        if (!stats) {
            stats = {
                apiKey: key,
                requestCount: 0,
                successCount: 0,
                failureCount: 0,
                timeoutCount: 0,
                avgDuration: 0,
                recentDurations: [],
                isSlow: false,
                isProblematic: false,
                lastRequest: null,
            };
            this.keyStats.set(key, stats);
        }

        stats.requestCount++;
        stats.lastRequest = Date.now();

        if (success) {
            stats.successCount++;
            stats.recentDurations.push(duration);
        } else {
            stats.failureCount++;
        }

        if (duration > this.slowThreshold) {
            stats.timeoutCount++;
        }

        if (stats.recentDurations.length > 20) {
            stats.recentDurations.shift();
        }

        stats.avgDuration = this._calculateAvg(stats.recentDurations);

        stats.isSlow = stats.avgDuration > this.slowThreshold;
        stats.isProblematic = stats.failureCount >= this.failureThreshold;

        this._addToHistory({
            apiKey: key,
            duration,
            success,
            timestamp: Date.now(),
        });

        return stats;
    }

    _calculateAvg(durations) {
        if (durations.length === 0) return 0;
        return durations.reduce((a, b) => a + b, 0) / durations.length;
    }

    _getKeyIdentifier(apiKey) {
        if (!apiKey) return 'unknown';
        return `${apiKey.substring(0, 4)}...${apiKey.substring(apiKey.length - 4)}`;
    }

    _addToHistory(entry) {
        this.timeoutHistory.push(entry);
        if (this.timeoutHistory.length > 1000) {
            this.timeoutHistory.shift();
        }
    }

    getTimeoutForKey(apiKey) {
        const key = this._getKeyIdentifier(apiKey);
        const stats = this.keyStats.get(key);

        if (!stats) {
            return this.defaultTimeout;
        }

        if (stats.isProblematic) {
            return this.quickTimeout;
        }

        if (stats.isSlow) {
            return Math.min(this.quickTimeout, stats.avgDuration * 0.8);
        }

        return this.defaultTimeout;
    }

    shouldSkipKey(apiKey) {
        const key = this._getKeyIdentifier(apiKey);
        const stats = this.keyStats.get(key);

        if (!stats) {
            return false;
        }

        return stats.isProblematic && stats.recentDurations.length < 3;
    }

    getKeyStats(apiKey) {
        const key = this._getKeyIdentifier(apiKey);
        return this.keyStats.get(key) || null;
    }

    getAllStats() {
        const stats = {};
        this.keyStats.forEach((value, key) => {
            stats[key] = {
                requestCount: value.requestCount,
                successCount: value.successCount,
                failureCount: value.failureCount,
                avgDuration: value.avgDuration.toFixed(0) + 'ms',
                isSlow: value.isSlow,
                isProblematic: value.isProblematic,
                recommendedTimeout: this.getTimeoutForKey(value.apiKey) + 'ms',
            };
        });
        return stats;
    }

    getSlowKeys() {
        const slow = [];
        this.keyStats.forEach((stats, key) => {
            if (stats.isSlow) {
                slow.push({
                    key,
                    avgDuration: stats.avgDuration.toFixed(0) + 'ms',
                    requestCount: stats.requestCount,
                });
            }
        });
        return slow.sort((a, b) => b.avgDuration - a.avgDuration);
    }

    getProblematicKeys() {
        const problematic = [];
        this.keyStats.forEach((stats, key) => {
            if (stats.isProblematic) {
                problematic.push({
                    key,
                    failureCount: stats.failureCount,
                    successCount: stats.successCount,
                });
            }
        });
        return problematic;
    }

    getHistory(limit = 50) {
        return this.timeoutHistory.slice(-limit).reverse();
    }

    reset(apiKey = null) {
        if (apiKey) {
            const key = this._getKeyIdentifier(apiKey);
            this.keyStats.delete(key);
            logger.info(`[ApiKeyTimeoutTracker] Reset stats for ${key}`);
        } else {
            this.keyStats.clear();
            this.timeoutHistory = [];
            logger.info('[ApiKeyTimeoutTracker] Reset all stats');
        }
    }

    getStats() {
        let totalRequests = 0;
        let totalFailures = 0;
        let slowKeys = 0;
        let problematicKeys = 0;

        this.keyStats.forEach((stats) => {
            totalRequests += stats.requestCount;
            totalFailures += stats.failureCount;
            if (stats.isSlow) slowKeys++;
            if (stats.isProblematic) problematicKeys++;
        });

        return {
            trackedKeys: this.keyStats.size,
            totalRequests,
            totalFailures,
            slowKeys,
            problematicKeys,
            avgTimeout: this.defaultTimeout + 'ms',
            quickTimeout: this.quickTimeout + 'ms',
        };
    }
}

export default ApiKeyTimeoutTracker;
