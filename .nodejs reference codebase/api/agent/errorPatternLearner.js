/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Error Pattern Learner
 * Learns from errors to prevent repetition
 * @module api/agent/errorPatternLearner
 */

import { createLogger } from "../core/logger.js";

const logger = createLogger("api/agent/errorPatternLearner.js");

class ErrorPatternLearner {
  constructor() {
    this.patterns = new Map();
    this.maxPatterns = 100;
    this.minOccurrences = 2; // Minimum occurrences to consider a pattern
  }

  /**
   * Record an error occurrence
   * @param {string} error - Error message
   * @param {object} context - Context { url, goal, action, pageType }
   */
  recordError(error, context = {}) {
    const patternKey = this._getKey(error, context);

    if (!this.patterns.has(patternKey)) {
      this.patterns.set(patternKey, {
        key: patternKey,
        errorType: this._classifyError(error),
        errorMessage: error,
        context: {
          url: context.url,
          goal: context.goal,
          action: context.action,
          pageType: context.pageType,
        },
        count: 0,
        firstSeen: Date.now(),
        lastSeen: Date.now(),
        occurrences: [],
      });
    }

    const pattern = this.patterns.get(patternKey);
    pattern.count++;
    pattern.lastSeen = Date.now();
    pattern.occurrences.push({
      timestamp: Date.now(),
      action: context.action,
    });

    // Keep only last 10 occurrences
    if (pattern.occurrences.length > 10) {
      pattern.occurrences = pattern.occurrences.slice(-10);
    }

    // Trim patterns if over limit
    if (this.patterns.size > this.maxPatterns) {
      this._trimPatterns();
    }

    logger.debug(
      `[ErrorPatternLearner] Recorded error pattern: ${patternKey} (count: ${pattern.count})`,
    );
  }

  /**
   * Get warning for current context
   * @param {object} context - Current context
   * @returns {string|null} Warning message or null
   */
  getWarning(context) {
    const relevantPatterns = this._findRelevantPatterns(context);

    if (relevantPatterns.length === 0) {
      return null;
    }

    const pattern = relevantPatterns[0];

    if (pattern.count >= this.minOccurrences) {
      return `⚠️ Warning: This error has occurred ${pattern.count} times before. Consider alternative approaches.`;
    }

    return null;
  }

  /**
   * Get prevention strategies for current context
   * @param {object} context - Current context
   * @returns {Array} Array of prevention strategies
   */
  getPreventionStrategies(context) {
    const relevantPatterns = this._findRelevantPatterns(context);
    const strategies = [];

    for (const pattern of relevantPatterns.slice(0, 3)) {
      const strategy = this._generatePreventionStrategy(pattern);
      if (strategy) {
        strategies.push(strategy);
      }
    }

    return strategies;
  }

  /**
   * Find relevant patterns for current context
   * @private
   */
  _findRelevantPatterns(context) {
    const relevant = [];

    for (const pattern of this.patterns.values()) {
      const relevance = this._calculateRelevance(pattern, context);

      if (relevance > 0.3) {
        // Minimum relevance threshold
        relevant.push({ ...pattern, relevance });
      }
    }

    // Sort by relevance (descending) and count (descending)
    return relevant.sort((a, b) => {
      if (b.relevance !== a.relevance) {
        return b.relevance - a.relevance;
      }
      return b.count - a.count;
    });
  }

  /**
   * Calculate relevance of a pattern to current context
   * @private
   */
  _calculateRelevance(pattern, context) {
    let relevance = 0;

    // Same URL = high relevance
    if (pattern.context.url && context.url) {
      const patternUrl = this._normalizeUrl(pattern.context.url);
      const contextUrl = this._normalizeUrl(context.url);

      if (patternUrl === contextUrl) {
        relevance += 0.5;
      } else if (
        patternUrl.includes(contextUrl) ||
        contextUrl.includes(patternUrl)
      ) {
        relevance += 0.25;
      }
    }

    // Same goal = medium relevance
    if (pattern.context.goal && context.goal) {
      const goalSimilarity = this._stringSimilarity(
        pattern.context.goal,
        context.goal,
      );
      relevance += goalSimilarity * 0.3;
    }

    // Same action = medium relevance
    if (pattern.context.action && context.action) {
      if (pattern.context.action === context.action) {
        relevance += 0.2;
      }
    }

    return Math.min(1, relevance);
  }

  /**
   * Generate prevention strategy for a pattern
   * @private
   */
  _generatePreventionStrategy(pattern) {
    const errorType = pattern.errorType;

    switch (errorType) {
      case "selector_not_found":
        return {
          pattern: pattern.key,
          strategy:
            "Use more specific selectors or try text/role-based selectors",
          confidence: Math.min(0.9, 0.5 + pattern.count * 0.1),
        };

      case "element_not_visible":
        return {
          pattern: pattern.key,
          strategy: "Scroll to element before clicking or wait for visibility",
          confidence: Math.min(0.9, 0.5 + pattern.count * 0.1),
        };

      case "timeout":
        return {
          pattern: pattern.key,
          strategy: "Increase wait time or check page load status",
          confidence: Math.min(0.9, 0.5 + pattern.count * 0.1),
        };

      case "verification_failed":
        return {
          pattern: pattern.key,
          strategy: "Use multiple verification methods or skip verification",
          confidence: Math.min(0.9, 0.5 + pattern.count * 0.1),
        };

      default:
        return {
          pattern: pattern.key,
          strategy: "Try alternative approach or wait and retry",
          confidence: Math.min(0.9, 0.5 + pattern.count * 0.1),
        };
    }
  }

  /**
   * Generate pattern key
   * @private
   */
  _getKey(error, context) {
    const errorType = this._classifyError(error);
    const url = context.url ? this._normalizeUrl(context.url) : "unknown";
    const action = context.action || "unknown";

    return `${errorType}|${url}|${action}`;
  }

  /**
   * Classify error type
   * @private
   */
  _classifyError(error) {
    const errorLower = error.toLowerCase();

    if (errorLower.includes("selector") || errorLower.includes("not found")) {
      return "selector_not_found";
    }

    if (errorLower.includes("not visible") || errorLower.includes("hidden")) {
      return "element_not_visible";
    }

    if (errorLower.includes("timeout") || errorLower.includes("timed out")) {
      return "timeout";
    }

    if (errorLower.includes("verification") || errorLower.includes("verify")) {
      return "verification_failed";
    }

    return "unknown";
  }

  /**
   * Normalize URL
   * @private
   */
  _normalizeUrl(url) {
    try {
      const urlObj = new URL(url);
      return `${urlObj.hostname}${urlObj.pathname}`;
    } catch {
      return url.toLowerCase();
    }
  }

  /**
   * Calculate string similarity
   * @private
   */
  _stringSimilarity(str1, str2) {
    const normalize = (s) => s.toLowerCase().trim();
    const s1 = normalize(str1);
    const s2 = normalize(str2);

    if (s1 === s2) return 1;

    const words1 = new Set(s1.split(/\s+/));
    const words2 = new Set(s2.split(/\s+/));

    const intersection = new Set([...words1].filter((x) => words2.has(x)));
    const union = new Set([...words1, ...words2]);

    return union.size > 0 ? intersection.size / union.size : 0;
  }

  /**
   * Trim old patterns
   * @private
   */
  _trimPatterns() {
    // Sort by last seen (oldest first)
    const sorted = [...this.patterns.entries()].sort((a, b) => {
      return a[1].lastSeen - b[1].lastSeen;
    });

    // Remove oldest 20%
    const toRemove = Math.floor(sorted.length * 0.2);
    for (let i = 0; i < toRemove; i++) {
      this.patterns.delete(sorted[i][0]);
    }

    logger.debug(`[ErrorPatternLearner] Trimmed ${toRemove} old patterns`);
  }

  /**
   * Get all patterns
   * @returns {Array} Array of patterns
   */
  getAllPatterns() {
    return [...this.patterns.values()];
  }

  /**
   * Get pattern statistics
   * @returns {object} Statistics
   */
  getStats() {
    const patterns = [...this.patterns.values()];
    const byType = {};

    for (const pattern of patterns) {
      byType[pattern.errorType] = (byType[pattern.errorType] || 0) + 1;
    }

    return {
      totalPatterns: patterns.length,
      byType,
      minOccurrences: this.minOccurrences,
    };
  }

  /**
   * Clear all patterns
   */
  clear() {
    this.patterns.clear();
    logger.info("[ErrorPatternLearner] All patterns cleared");
  }

  /**
   * Export patterns for persistence
   * @returns {Array} Exportable patterns
   */
  export() {
    return [...this.patterns.values()].map((p) => ({
      key: p.key,
      errorType: p.errorType,
      errorMessage: p.errorMessage,
      context: p.context,
      count: p.count,
      firstSeen: p.firstSeen,
      lastSeen: p.lastSeen,
    }));
  }

  /**
   * Import patterns from persistence
   * @param {Array} data - Imported patterns
   */
  import(data) {
    for (const item of data) {
      this.patterns.set(item.key, {
        ...item,
        occurrences: [],
      });
    }

    logger.info(`[ErrorPatternLearner] Imported ${data.length} patterns`);
  }
}

const errorPatternLearner = new ErrorPatternLearner();

export { errorPatternLearner };
export default errorPatternLearner;
