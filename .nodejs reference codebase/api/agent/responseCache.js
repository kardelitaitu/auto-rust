/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Response Cache with Semantic Similarity
 * Caches LLM responses for similar contexts
 * @module api/agent/responseCache
 */

import { createLogger } from "../core/logger.js";

const logger = createLogger("api/agent/responseCache.js");

class ResponseCache {
  constructor() {
    this.cache = new Map();
    this.maxSize = 1000;
    this.defaultTTL = 300000; // 5 minutes
    this.similarityThreshold = 0.8;
  }

  /**
   * Generate cache key from context
   * @param {object} context - Context object { url, goal, pageType, elementHash }
   * @returns {string} Cache key
   */
  getKey(context) {
    const { url, goal, pageType, elementHash } = context;

    // Normalize URL (remove query params and hash)
    let normalizedUrl = url || "";
    try {
      const urlObj = new URL(normalizedUrl);
      normalizedUrl = `${urlObj.hostname}${urlObj.pathname}`;
    } catch {
      // Keep original if URL parsing fails
    }

    // Normalize goal (lowercase, trim)
    const normalizedGoal = (goal || "").toLowerCase().trim();

    return `${normalizedUrl}|${normalizedGoal}|${pageType || "unknown"}|${elementHash || "none"}`;
  }

  /**
   * Get cached response
   * @param {object} context - Context object
   * @returns {object|null} Cached response or null
   */
  get(context) {
    const key = this.getKey(context);
    const cached = this.cache.get(key);

    if (cached && !this._isExpired(cached)) {
      logger.debug(
        `[ResponseCache] Cache hit for key: ${key.substring(0, 50)}...`,
      );
      return cached.response;
    }

    // Try semantic similarity match
    const similar = this._findSimilar(context);
    if (similar) {
      logger.debug(
        `[ResponseCache] Semantic cache hit (similarity: ${similar.similarity.toFixed(2)})`,
      );
      return similar.response;
    }

    logger.debug(
      `[ResponseCache] Cache miss for key: ${key.substring(0, 50)}...`,
    );
    return null;
  }

  /**
   * Set cached response
   * @param {object} context - Context object
   * @param {object} response - Response to cache
   * @param {number} ttl - Time to live in ms (optional)
   */
  set(context, response, ttl = this.defaultTTL) {
    const key = this.getKey(context);

    this.cache.set(key, {
      response,
      context,
      timestamp: Date.now(),
      ttl,
      hits: 0,
    });

    // Evict oldest if over limit
    if (this.cache.size > this.maxSize) {
      this._evictOldest();
    }

    logger.debug(
      `[ResponseCache] Cached response for key: ${key.substring(0, 50)}...`,
    );
  }

  /**
   * Find semantically similar cached entry
   * @private
   */
  _findSimilar(context) {
    let bestMatch = null;
    let bestSimilarity = 0;

    for (const [_key, entry] of this.cache.entries()) {
      if (this._isExpired(entry)) continue;

      const similarity = this._calculateSimilarity(context, entry.context);

      if (
        similarity > bestSimilarity &&
        similarity >= this.similarityThreshold
      ) {
        bestSimilarity = similarity;
        bestMatch = { ...entry, similarity };
      }
    }

    if (bestMatch) {
      bestMatch.hits++;
    }

    return bestMatch;
  }

  /**
   * Calculate similarity between two contexts
   * @private
   */
  _calculateSimilarity(context1, context2) {
    let similarity = 0;
    let factors = 0;

    // URL similarity (40% weight)
    if (context1.url && context2.url) {
      const urlSim = this._stringSimilarity(context1.url, context2.url);
      similarity += urlSim * 0.4;
      factors += 0.4;
    }

    // Goal similarity (40% weight)
    if (context1.goal && context2.goal) {
      const goalSim = this._stringSimilarity(context1.goal, context2.goal);
      similarity += goalSim * 0.4;
      factors += 0.4;
    }

    // Page type match (20% weight)
    if (context1.pageType && context2.pageType) {
      const pageSim = context1.pageType === context2.pageType ? 1 : 0;
      similarity += pageSim * 0.2;
      factors += 0.2;
    }

    return factors > 0 ? similarity / factors : 0;
  }

  /**
   * Calculate string similarity (Jaccard-like)
   * @private
   */
  _stringSimilarity(str1, str2) {
    const normalize = (s) => s.toLowerCase().trim();
    const s1 = normalize(str1);
    const s2 = normalize(str2);

    if (s1 === s2) return 1;

    // Split into words
    const words1 = new Set(s1.split(/\s+/));
    const words2 = new Set(s2.split(/\s+/));

    // Calculate Jaccard similarity
    const intersection = new Set([...words1].filter((x) => words2.has(x)));
    const union = new Set([...words1, ...words2]);

    return union.size > 0 ? intersection.size / union.size : 0;
  }

  /**
   * Check if cached entry is expired
   * @private
   */
  _isExpired(entry) {
    return Date.now() - entry.timestamp > entry.ttl;
  }

  /**
   * Evict oldest entry
   * @private
   */
  _evictOldest() {
    let oldestKey = null;
    let oldestTime = Date.now();

    for (const [key, entry] of this.cache.entries()) {
      if (entry.timestamp < oldestTime) {
        oldestTime = entry.timestamp;
        oldestKey = key;
      }
    }

    if (oldestKey) {
      this.cache.delete(oldestKey);
    }
  }

  /**
   * Clear all cached entries
   */
  clear() {
    this.cache.clear();
    logger.info("[ResponseCache] Cache cleared");
  }

  /**
   * Get cache statistics
   * @returns {object} Cache statistics
   */
  getStats() {
    let totalHits = 0;
    let expiredCount = 0;

    for (const entry of this.cache.values()) {
      totalHits += entry.hits || 0;
      if (this._isExpired(entry)) expiredCount++;
    }

    return {
      size: this.cache.size,
      maxSize: this.maxSize,
      totalHits,
      expiredCount,
      hitRate: this.cache.size > 0 ? totalHits / this.cache.size : 0,
    };
  }

  /**
   * Remove expired entries
   */
  cleanup() {
    let removed = 0;

    for (const [key, entry] of this.cache.entries()) {
      if (this._isExpired(entry)) {
        this.cache.delete(key);
        removed++;
      }
    }

    logger.info(`[ResponseCache] Cleaned up ${removed} expired entries`);
    return removed;
  }

  /**
   * Set similarity threshold
   * @param {number} threshold - New threshold (0-1)
   */
  setSimilarityThreshold(threshold) {
    this.similarityThreshold = Math.max(0, Math.min(1, threshold));
    logger.info(
      `[ResponseCache] Similarity threshold set to ${this.similarityThreshold}`,
    );
  }
}

const responseCache = new ResponseCache();

export { responseCache };
export default responseCache;
