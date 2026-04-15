/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * Request Deduplication
 * Caches identical requests within a time window
 * @module utils/request-dedupe
 */

import { createLogger } from '../core/logger.js';

const logger = createLogger('request-dedupe.js');

export class RequestDedupe {
    constructor(options = {}) {
        this.ttl = options.ttl || 30000;
        this.maxSize = options.maxSize || 1000;
        this.enabled = options.enabled !== false;

        this.cache = new Map();
        this.stats = {
            hits: 0,
            misses: 0,
            cached: 0,
            expired: 0,
        };
    }

    _generateKey(messages, model, maxTokens, temperature) {
        const content = messages.map((m) => `${m.role}:${m.content}`).join('||');
        const params = `${model}:${maxTokens}:${temperature}`;
        return this._hash(content + params);
    }

    _hash(str) {
        let hash = 0;
        for (let i = 0; i < str.length; i++) {
            const char = str.charCodeAt(i);
            hash = (hash << 5) - hash + char;
            hash = hash & hash;
        }
        return hash.toString(16);
    }

    check(messages, model, maxTokens = 100, temperature = 0.7) {
        if (!this.enabled) {
            return { hit: false, reason: 'disabled' };
        }

        if (this.cache.size >= this.maxSize) {
            logger.warn('[RequestDedupe] Cache full, oldest entry evicted');
            this._evictOldest();
        }

        const key = this._generateKey(messages, model, maxTokens, temperature);
        const entry = this.cache.get(key);

        if (!entry) {
            this.stats.misses++;
            return { hit: false, key };
        }

        if (Date.now() - entry.timestamp > this.ttl) {
            this.stats.expired++;
            this.cache.delete(key);
            return { hit: false, key, reason: 'expired' };
        }

        this.stats.hits++;
        logger.debug(`[RequestDedupe] Cache hit for key ${key.substring(0, 8)}`);
        return { hit: true, response: entry.response, key };
    }

    set(messages, model, response, maxTokens = 100, temperature = 0.7) {
        if (!this.enabled) {
            return;
        }

        // Don't cache empty responses - they pollute the cache
        if (!response || (typeof response === 'string' && response.trim().length === 0)) {
            logger.debug(`[RequestDedupe] Not caching empty response`);
            return;
        }

        const key = this._generateKey(messages, model, maxTokens, temperature);

        this.cache.set(key, {
            response,
            timestamp: Date.now(),
        });

        this.stats.cached++;
        logger.debug(`[RequestDedupe] Cached response for key ${key.substring(0, 8)}`);
    }

    _evictOldest() {
        let oldestKey = null;
        let oldestTime = Infinity;

        this.cache.forEach((entry, key) => {
            if (entry.timestamp < oldestTime) {
                oldestTime = entry.timestamp;
                oldestKey = key;
            }
        });

        if (oldestKey) {
            this.cache.delete(oldestKey);
        }
    }

    clear() {
        this.cache.clear();
        logger.info('[RequestDedupe] Cache cleared');
    }

    getStats() {
        const total = this.stats.hits + this.stats.misses;
        const hitRate = total > 0 ? ((this.stats.hits / total) * 100).toFixed(1) + '%' : '0%';

        return {
            ...this.stats,
            hitRate,
            cacheSize: this.cache.size,
            maxSize: this.maxSize,
            ttl: this.ttl,
        };
    }

    prune() {
        let pruned = 0;
        const now = Date.now();

        this.cache.forEach((entry, key) => {
            if (now - entry.timestamp > this.ttl) {
                this.cache.delete(key);
                pruned++;
            }
        });

        if (pruned > 0) {
            logger.info(`[RequestDedupe] Pruned ${pruned} expired entries`);
        }

        return pruned;
    }

    isEnabled() {
        return this.enabled;
    }

    setEnabled(enabled) {
        this.enabled = enabled;
        logger.info(`[RequestDedupe] ${enabled ? 'Enabled' : 'Disabled'}`);
    }
}

export default RequestDedupe;
