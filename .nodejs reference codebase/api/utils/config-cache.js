/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Configuration Cache
 * Provides LRU caching for configuration objects with TTL support
 * @module utils/config-cache
 */

import { createLogger } from '../core/logger.js';

const logger = createLogger('config-cache.js');

/**
 * Configuration Cache
 * Implements LRU caching with TTL for configuration objects
 */
export class ConfigCache {
    constructor(options = {}) {
        this.ttl = options.ttl || 5 * 60 * 1000; // 5 minutes default
        this.maxSize = options.maxSize || 100;
        this.cache = new Map();
        this.accessOrder = new Map(); // LRU tracking
        this.hitCount = 0;
        this.missCount = 0;
        this.evictionCount = 0;

        logger.debug(`[ConfigCache] Initialized with TTL: ${this.ttl}ms, maxSize: ${this.maxSize}`);
    }

    /**
     * Get value from cache
     * @param {string} key - Cache key
     * @returns {any|null} Cached value or null if not found/expired
     */
    get(key) {
        const entry = this.cache.get(key);

        if (!entry) {
            this.missCount++;
            return null;
        }

        const now = Date.now();

        // Check TTL
        if (now - entry.timestamp > this.ttl) {
            this.cache.delete(key);
            this.accessOrder.delete(key);
            this.evictionCount++;
            this.missCount++;
            logger.debug(`[ConfigCache] Entry expired: ${key}`);
            return null;
        }

        // Update LRU order
        this.accessOrder.set(key, now);
        this.hitCount++;

        logger.debug(`[ConfigCache] Cache hit: ${key}`);
        return entry.value;
    }

    /**
     * Set value in cache
     * @param {string} key - Cache key
     * @param {any} value - Value to cache
     */
    set(key, value) {
        const now = Date.now();

        // Check if key already exists
        if (this.cache.has(key)) {
            this.cache.set(key, { value, timestamp: now });
            this.accessOrder.set(key, now);
            logger.debug(`[ConfigCache] Updated existing entry: ${key}`);
            return;
        }

        // Check if cache is full and evict LRU entry
        if (this.cache.size >= this.maxSize) {
            this.evictLRU();
        }

        // Add new entry
        this.cache.set(key, { value, timestamp: now });
        this.accessOrder.set(key, now);

        logger.debug(`[ConfigCache] Added new entry: ${key}`);
    }

    /**
     * Evict least recently used entry
     * @private
     */
    evictLRU() {
        if (this.cache.size === 0) return;

        let oldestKey = null;
        let oldestTime = Infinity;

        for (const [key, timestamp] of this.accessOrder.entries()) {
            if (timestamp < oldestTime) {
                oldestTime = timestamp;
                oldestKey = key;
            }
        }

        if (oldestKey) {
            this.cache.delete(oldestKey);
            this.accessOrder.delete(oldestKey);
            this.evictionCount++;
            logger.debug(`[ConfigCache] Evicted LRU entry: ${oldestKey}`);
        }
    }

    /**
     * Delete entry from cache
     * @param {string} key - Cache key
     * @returns {boolean} True if entry was deleted
     */
    delete(key) {
        const deleted = this.cache.delete(key);
        if (deleted) {
            this.accessOrder.delete(key);
            logger.debug(`[ConfigCache] Deleted entry: ${key}`);
        }
        return deleted;
    }

    /**
     * Clear all entries from cache
     */
    clear() {
        const size = this.cache.size;
        this.cache.clear();
        this.accessOrder.clear();
        logger.info(`[ConfigCache] Cleared ${size} entries`);
    }

    /**
     * Check if key exists in cache
     * @param {string} key - Cache key
     * @returns {boolean} True if key exists and is not expired
     */
    has(key) {
        return this.get(key) !== null;
    }

    /**
     * Get cache statistics
     * @returns {object} Cache statistics
     */
    getStats() {
        const totalRequests = this.hitCount + this.missCount;
        const hitRate =
            totalRequests > 0 ? ((this.hitCount / totalRequests) * 100).toFixed(2) : '0.00';

        return {
            size: this.cache.size,
            maxSize: this.maxSize,
            ttl: this.ttl,
            hitCount: this.hitCount,
            missCount: this.missCount,
            evictionCount: this.evictionCount,
            hitRate: `${hitRate}%`,
            totalRequests,
        };
    }

    /**
     * Get cache entries (for debugging)
     * @returns {Array} Array of cache entries with metadata
     */
    getEntries() {
        const now = Date.now();
        const entries = [];

        for (const [key, entry] of this.cache.entries()) {
            entries.push({
                key,
                value: entry.value,
                age: now - entry.timestamp,
                expires: entry.timestamp + this.ttl,
            });
        }

        return entries.sort((a, b) => b.age - a.age);
    }

    /**
     * Clean expired entries
     * @returns {number} Number of entries cleaned
     */
    cleanExpired() {
        const now = Date.now();
        let cleaned = 0;

        for (const [key, entry] of this.cache.entries()) {
            if (now - entry.timestamp > this.ttl) {
                this.cache.delete(key);
                this.accessOrder.delete(key);
                cleaned++;
            }
        }

        if (cleaned > 0) {
            logger.info(`[ConfigCache] Cleaned ${cleaned} expired entries`);
        }

        return cleaned;
    }

    /**
     * Update cache TTL
     * @param {number} newTTL - New TTL in milliseconds
     */
    updateTTL(newTTL) {
        this.ttl = newTTL;
        logger.info(`[ConfigCache] Updated TTL to ${newTTL}ms`);
    }

    /**
     * Update cache max size
     * @param {number} newMaxSize - New maximum cache size
     */
    updateMaxSize(newMaxSize) {
        this.maxSize = newMaxSize;

        // Evict excess entries if needed
        while (this.cache.size > this.maxSize) {
            this.evictLRU();
        }

        logger.info(`[ConfigCache] Updated max size to ${newMaxSize}`);
    }

    /**
     * Get memory usage estimate
     * @returns {object} Memory usage statistics
     */
    getMemoryUsage() {
        let estimatedSize = 0;

        for (const [key, entry] of this.cache.entries()) {
            // Estimate size of key and value
            estimatedSize += key.length * 2; // UTF-16 characters
            estimatedSize += JSON.stringify(entry.value).length * 2; // Rough estimate
        }

        return {
            entries: this.cache.size,
            estimatedBytes: estimatedSize,
            estimatedKB: (estimatedSize / 1024).toFixed(2),
            estimatedMB: (estimatedSize / (1024 * 1024)).toFixed(4),
        };
    }
}

// Export singleton instance with default options
export const configCache = new ConfigCache();
logger.info(`[ConfigCache] Service initialized (TTL: ${configCache.ttl}ms)`);

// Convenience functions for backward compatibility
export const getFromCache = (key) => configCache.get(key);
export const setInCache = (key, value) => configCache.set(key, value);
export const clearCache = () => configCache.clear();
export const getCacheStats = () => configCache.getStats();

export default ConfigCache;
