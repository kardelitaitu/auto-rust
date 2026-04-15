/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * Rate Limit Tracker
 * Monitors API key usage and rate limits
 * @module utils/rate-limit-tracker
 */

import { createLogger } from '../core/logger.js';

const logger = createLogger('rate-limit-tracker.js');

export class RateLimitTracker {
    constructor(options = {}) {
        this.cacheDuration = options.cacheDuration || 60000;
        this.warningThreshold = options.warningThreshold || 0.2;

        this.cache = new Map();
        this.requestHistory = new Map();
    }

    async checkKey(apiKey) {
        const cached = this.cache.get(apiKey);

        if (cached && Date.now() - cached.timestamp < this.cacheDuration) {
            return cached.data;
        }

        try {
            const data = await this._fetchKeyInfo(apiKey);
            this.cache.set(apiKey, { data, timestamp: Date.now() });
            return data;
        } catch (error) {
            logger.warn(`[RateLimitTracker] Failed to fetch key info: ${error.message}`);
            return null;
        }
    }

    async _fetchKeyInfo(apiKey) {
        const response = await fetch('https://openrouter.ai/api/v1/key', {
            method: 'GET',
            headers: {
                Authorization: `Bearer ${apiKey}`,
                'Content-Type': 'application/json',
            },
        });

        if (!response.ok) {
            throw new Error(`HTTP ${response.status}`);
        }

        return await response.json();
    }

    trackRequest(apiKey, model) {
        if (!apiKey) return;

        const history = this.requestHistory.get(apiKey) || {
            requests: [],
            total: 0,
        };

        history.requests.push({
            timestamp: Date.now(),
            model,
        });

        history.total++;

        this.requestHistory.set(apiKey, history);
    }

    getRemaining(apiKey) {
        const cached = this.cache.get(apiKey);
        if (!cached || !cached.data) {
            return null;
        }

        const limitRemaining = cached.data.data?.limit_remaining;
        return typeof limitRemaining === 'number' ? limitRemaining : null;
    }

    getIsFreeTier(apiKey) {
        const cached = this.cache.get(apiKey);
        if (!cached || !cached.data) {
            return null;
        }

        return cached.data.data?.is_free_tier || false;
    }

    getUsageToday(apiKey) {
        const cached = this.cache.get(apiKey);
        if (!cached || !cached.data) {
            return null;
        }

        return cached.data.data?.usage_daily || 0;
    }

    getWarningStatus(apiKey) {
        const remaining = this.getRemaining(apiKey);
        const isFreeTier = this.getIsFreeTier(apiKey);

        if (remaining === null) {
            return 'unknown';
        }

        if (remaining <= 0) {
            return 'exhausted';
        }

        if (isFreeTier && remaining < 10) {
            return 'critical';
        }

        if (remaining < 50) {
            return 'warning';
        }

        return 'ok';
    }

    getRequestRate(apiKey, windowMs = 60000) {
        const history = this.requestHistory.get(apiKey);
        if (!history) {
            return 0;
        }

        const now = Date.now();
        const recent = history.requests.filter((r) => now - r.timestamp < windowMs);
        return recent.length;
    }

    async refreshKey(apiKey) {
        this.cache.delete(apiKey);
        return await this.checkKey(apiKey);
    }

    invalidateCache() {
        this.cache.clear();
        logger.info('[RateLimitTracker] Cache cleared');
    }

    getCacheStatus() {
        const status = {};
        this.cache.forEach((value, key) => {
            status[this._maskKey(key)] = {
                age: Date.now() - value.timestamp,
                remaining: value.data?.data?.limit_remaining,
            };
        });
        return status;
    }

    _maskKey(key) {
        if (!key) return 'null';
        if (key.length < 8) return '***';
        return `${key.substring(0, 6)}...${key.substring(key.length - 4)}`;
    }

    async getAllKeyStatus(apiKeys) {
        const statuses = await Promise.all(
            apiKeys.map((key) => ({
                key: this._maskKey(key),
                remaining: this.getRemaining(key),
                usageToday: this.getUsageToday(key),
                warning: this.getWarningStatus(key),
                isFreeTier: this.getIsFreeTier(key),
            }))
        );

        return statuses;
    }

    getStats() {
        let totalRequests = 0;
        for (const history of this.requestHistory.values()) {
            totalRequests += history.total;
        }

        return {
            cachedKeys: this.cache.size,
            trackedKeys: this.requestHistory.size,
            totalRequests,
        };
    }
}

export default RateLimitTracker;
