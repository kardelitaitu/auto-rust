/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * Model Performance Tracker
 * Tracks success/failure rates and latency per model
 * @module utils/model-perf-tracker
 */

import { createLogger } from '../core/logger.js';

const logger = createLogger('model-perf-tracker.js');

export class ModelPerfTracker {
    constructor(options = {}) {
        this.windowSize = options.windowSize || 100;
        this.minSamples = options.minSamples || 5;

        this.stats = new Map();
        this.history = [];
    }

    trackSuccess(model, duration, apiKey = null) {
        const key = this._getKey(model, apiKey);
        let stat = this.stats.get(key);

        if (!stat) {
            stat = this._createStat(model, apiKey);
            this.stats.set(key, stat);
        }

        stat.successes++;
        stat.total++;
        stat.recentSuccesses++;
        stat.recentTotal++;

        stat.avgDuration = this._calculateAvg(stat.avgDuration, stat.successes, duration);

        stat.recentDurations.push(duration);
        if (stat.recentDurations.length > this.windowSize) {
            stat.recentDurations.shift();
        }

        stat.recentAvgDuration = this._calculateRecentAvg(stat.recentDurations);

        stat.lastSuccess = Date.now();

        this._addToHistory({ model, apiKey, success: true, duration });

        logger.debug(
            `[ModelPerfTracker] ${key} success (${stat.successes}/${stat.total}, ${stat.recentAvgDuration.toFixed(0)}ms avg)`
        );
    }

    trackFailure(model, error, apiKey = null) {
        const key = this._getKey(model, apiKey);
        let stat = this.stats.get(key);

        if (!stat) {
            stat = this._createStat(model, apiKey);
            this.stats.set(key, stat);
        }

        stat.failures++;
        stat.total++;
        stat.recentFailures++;
        stat.recentTotal++;

        stat.lastFailure = Date.now();
        stat.lastError = error.substring(0, 100);

        this._addToHistory({ model, apiKey, success: false, error });

        logger.debug(
            `[ModelPerfTracker] ${key} failure (${stat.failures}/${stat.total}, error: ${error.substring(0, 50)}...)`
        );
    }

    _createStat(model, apiKey) {
        return {
            model,
            apiKey: apiKey ? this._maskKey(apiKey) : null,
            successes: 0,
            failures: 0,
            total: 0,
            avgDuration: 0,
            recentSuccesses: 0,
            recentFailures: 0,
            recentTotal: 0,
            recentDurations: [],
            recentAvgDuration: 0,
            lastSuccess: null,
            lastFailure: null,
            lastError: null,
        };
    }

    _getKey(model, apiKey) {
        return apiKey ? `${model}::${apiKey}` : model;
    }

    _maskKey(key) {
        if (!key) return 'null';
        if (key.length < 8) return '***';
        return `${key.substring(0, 4)}...${key.substring(key.length - 4)}`;
    }

    _calculateAvg(currentAvg, count, newValue) {
        return (currentAvg * count + newValue) / (count + 1);
    }

    _calculateRecentAvg(durations) {
        if (durations.length === 0) return 0;
        return durations.reduce((a, b) => a + b, 0) / durations.length;
    }

    _addToHistory(entry) {
        this.history.push({
            ...entry,
            timestamp: Date.now(),
        });

        if (this.history.length > 1000) {
            this.history.shift();
        }
    }

    getModelStats(model, apiKey = null) {
        const key = this._getKey(model, apiKey);
        return this.stats.get(key) || null;
    }

    getAllStats() {
        const result = {};
        this.stats.forEach((stat, key) => {
            result[key] = {
                model: stat.model,
                successes: stat.successes,
                failures: stat.failures,
                total: stat.total,
                successRate:
                    stat.total > 0 ? ((stat.successes / stat.total) * 100).toFixed(1) + '%' : 'N/A',
                avgDuration: stat.avgDuration.toFixed(0) + 'ms',
                recentSuccessRate:
                    stat.recentTotal > 0
                        ? ((stat.recentSuccesses / stat.recentTotal) * 100).toFixed(1) + '%'
                        : 'N/A',
                recentAvgDuration: stat.recentAvgDuration.toFixed(0) + 'ms',
                lastSuccess: stat.lastSuccess,
                lastFailure: stat.lastFailure,
            };
        });
        return result;
    }

    getBestModel(models, apiKey = null) {
        let best = null;
        let bestScore = -1;

        for (const model of models) {
            const stat = this.getModelStats(model, apiKey);

            if (!stat) {
                continue;
            }

            if (stat.total < this.minSamples) {
                continue;
            }

            const recentSuccessRate = stat.recentSuccesses / Math.max(stat.recentTotal, 1);
            const recentAvgDuration = stat.recentAvgDuration || 10000;

            const score = recentSuccessRate * 100 - recentAvgDuration / 1000;

            if (score > bestScore) {
                bestScore = score;
                best = model;
            }
        }

        return best;
    }

    getWorstModel(models, apiKey = null) {
        let worst = null;
        let worstScore = Infinity;

        for (const model of models) {
            const stat = this.getModelStats(model, apiKey);

            if (!stat) {
                continue;
            }

            if (stat.total < this.minSamples) {
                continue;
            }

            const recentSuccessRate = stat.recentSuccesses / Math.max(stat.recentTotal, 1);
            const recentAvgDuration = stat.recentAvgDuration || 10000;

            const score = recentSuccessRate * 100 + recentAvgDuration / 1000;

            if (score < worstScore) {
                worstScore = score;
                worst = model;
            }
        }

        return worst;
    }

    getSortedModels(apiKey = null) {
        const models = [];

        this.stats.forEach((stat, key) => {
            if (apiKey && !key.endsWith(`::${apiKey}`)) {
                return;
            }

            models.push({
                model: stat.model,
                successRate: stat.total > 0 ? stat.successes / stat.total : 0,
                avgDuration: stat.avgDuration,
                total: stat.total,
            });
        });

        return models.sort((a, b) => b.successRate - a.successRate);
    }

    getHistory(limit = 50) {
        return this.history.slice(-limit).reverse();
    }

    reset(model = null, apiKey = null) {
        if (model) {
            const key = this._getKey(model, apiKey);
            this.stats.delete(key);
            logger.info(`[ModelPerfTracker] Reset stats for ${key}`);
        } else {
            this.stats.clear();
            this.history = [];
            logger.info('[ModelPerfTracker] Reset all stats');
        }
    }

    getStats() {
        let totalSuccesses = 0;
        let totalFailures = 0;
        let totalDuration = 0;

        this.stats.forEach((stat) => {
            totalSuccesses += stat.successes;
            totalFailures += stat.failures;
            totalDuration += stat.avgDuration * stat.total;
        });

        const total = totalSuccesses + totalFailures;

        return {
            modelsTracked: this.stats.size,
            totalRequests: total,
            totalSuccesses,
            totalFailures,
            overallSuccessRate:
                total > 0 ? ((totalSuccesses / total) * 100).toFixed(1) + '%' : 'N/A',
            avgDuration: total > 0 ? (totalDuration / total).toFixed(0) + 'ms' : 'N/A',
            historySize: this.history.length,
        };
    }
}

export default ModelPerfTracker;
