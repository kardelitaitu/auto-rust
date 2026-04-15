/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Smart History Management
 * Manages conversation history with relevance scoring and compression
 * @module api/agent/historyManager
 */

import { createLogger } from "../core/logger.js";

const logger = createLogger("api/agent/historyManager.js");

class HistoryManager {
  constructor() {
    this.history = [];
    this.maxSize = 50;
    this.compressionThreshold = 30; // Start compressing after this many entries
  }

  /**
   * Add entry to history
   * @param {object} entry - History entry { role, content, url, goal, success, timestamp }
   */
  add(entry) {
    const historyEntry = {
      ...entry,
      timestamp: entry.timestamp || Date.now(),
      relevance: this._calculateRelevance(entry),
    };

    this.history.push(historyEntry);

    // Trim if over limit
    if (this.history.length > this.maxSize) {
      this._trim();
    }

    // Compress if over threshold
    if (this.history.length > this.compressionThreshold) {
      this._compress();
    }
  }

  /**
   * Get relevant history entries for current context
   * @param {object} context - Current context { url, goal, pageType }
   * @param {number} limit - Maximum number of entries to return
   * @returns {Array} Relevant history entries
   */
  getRelevant(context, limit = 4) {
    if (this.history.length === 0) {
      return [];
    }

    // Score each entry for relevance
    const scored = this.history.map((entry) => ({
      entry,
      score: this._scoreRelevance(entry, context),
    }));

    // Sort by score (descending) and return top entries
    return scored
      .sort((a, b) => b.score - a.score)
      .slice(0, limit)
      .map((s) => s.entry);
  }

  /**
   * Calculate initial relevance score for an entry
   * @private
   */
  _calculateRelevance(entry) {
    let relevance = 0.5; // Base relevance

    // Successful actions are more relevant
    if (entry.success) relevance += 0.2;

    // Recent entries are more relevant
    const age = Date.now() - entry.timestamp;
    relevance += Math.max(0, 1 - age / 300000); // 5 minute decay

    return Math.min(1, relevance);
  }

  /**
   * Score relevance of an entry for current context
   * @private
   */
  _scoreRelevance(entry, context) {
    let score = 0;

    // Same URL = higher relevance
    if (entry.url && context.url) {
      const entryUrl = this._normalizeUrl(entry.url);
      const contextUrl = this._normalizeUrl(context.url);
      if (entryUrl === contextUrl) {
        score += 0.3;
      } else if (
        entryUrl.includes(contextUrl) ||
        contextUrl.includes(entryUrl)
      ) {
        score += 0.15;
      }
    }

    // Same goal = higher relevance
    if (entry.goal && context.goal) {
      const goalSimilarity = this._stringSimilarity(entry.goal, context.goal);
      score += goalSimilarity * 0.3;
    }

    // Recent = higher relevance
    const age = Date.now() - entry.timestamp;
    score += Math.max(0, 1 - age / 300000) * 0.2; // 5 minute decay

    // Successful actions = higher relevance
    if (entry.success) {
      score += 0.1;
    }

    // Failed actions with recovery = higher relevance
    if (!entry.success && entry.recovery) {
      score += 0.05;
    }

    return Math.min(1, score);
  }

  /**
   * Normalize URL for comparison
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
   * Trim history to max size (remove least relevant)
   * @private
   */
  _trim() {
    // Sort by relevance (ascending) and remove oldest/least relevant
    this.history.sort((a, b) => a.relevance - b.relevance);
    this.history = this.history.slice(-this.maxSize);
  }

  /**
   * Compress history by summarizing old entries
   * @private
   */
  _compress() {
    const oldEntries = this.history.slice(0, this.history.length - 20);
    const recentEntries = this.history.slice(-20);

    // Create summary of old entries
    const summary = this._createSummary(oldEntries);

    // Replace old entries with summary
    this.history = [summary, ...recentEntries];

    logger.debug(
      `[HistoryManager] Compressed ${oldEntries.length} entries into summary`,
    );
  }

  /**
   * Create summary of history entries
   * @private
   */
  _createSummary(entries) {
    const successful = entries.filter((e) => e.success).length;
    const failed = entries.filter((e) => !e.success).length;
    const goals = [...new Set(entries.map((e) => e.goal).filter(Boolean))];
    const urls = [
      ...new Set(entries.map((e) => this._normalizeUrl(e.url)).filter(Boolean)),
    ];

    return {
      role: "system",
      content: `[History Summary: ${entries.length} previous actions, ${successful} successful, ${failed} failed. Goals attempted: ${goals.join(", ")}. Pages visited: ${urls.join(", ")}]`,
      timestamp: entries[0]?.timestamp || Date.now(),
      relevance: 0.3,
      isSummary: true,
    };
  }

  /**
   * Clear all history
   */
  clear() {
    this.history = [];
    logger.info("[HistoryManager] History cleared");
  }

  /**
   * Get history statistics
   * @returns {object} Statistics
   */
  getStats() {
    const successful = this.history.filter((e) => e.success).length;
    const failed = this.history.filter((e) => !e.success).length;
    const summaries = this.history.filter((e) => e.isSummary).length;

    return {
      total: this.history.length,
      successful,
      failed,
      summaries,
      successRate:
        this.history.length > 0 ? successful / this.history.length : 0,
    };
  }

  /**
   * Export history for debugging
   * @returns {Array} History entries
   */
  export() {
    return [...this.history];
  }
}

const historyManager = new HistoryManager();

export { historyManager };
export default historyManager;
