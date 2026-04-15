/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Memory Injector
 * Injects learned patterns from session store into prompts
 * @module api/agent/memoryInjector
 */

import { createLogger } from "../core/logger.js";
import { sessionStore } from "./sessionStore.js";

const logger = createLogger("api/agent/memoryInjector.js");

class MemoryInjector {
  constructor() {
    this.maxPatterns = 5;
    this.minConfidence = 0.6;
  }

  /**
   * Inject learned patterns into prompt
   * @param {object} context - Current context { url, goal, pageType }
   * @returns {string} Memory injection string for prompt
   */
  async injectMemory(context) {
    try {
      const patterns = await this._getRelevantPatterns(context);

      if (patterns.length === 0) {
        return "";
      }

      let memory = "\n## 🧠 Learned Patterns\n";
      memory +=
        "Based on previous sessions, here are patterns that worked:\n\n";

      for (const pattern of patterns) {
        memory += this._formatPattern(pattern);
      }

      memory +=
        "\n**Tip**: These patterns have worked before. Consider using them if applicable.\n";

      logger.debug(`[MemoryInjector] Injected ${patterns.length} patterns`);
      return memory;
    } catch (error) {
      logger.error("[MemoryInjector] Failed to inject memory:", error.message);
      return "";
    }
  }

  /**
   * Get relevant patterns from session store
   * @private
   */
  async _getRelevantPatterns(context) {
    const patterns = [];

    // Get patterns for this URL
    if (context.url) {
      const urlPatterns = sessionStore.getPatternsByUrl(context.url);
      patterns.push(...urlPatterns);
    }

    // Get patterns for this goal type
    if (context.goal) {
      const goalType = this._extractGoalType(context.goal);
      const goalPatterns = sessionStore.getPatternsByGoalType(goalType);
      patterns.push(...goalPatterns);
    }

    // Get patterns for this page type
    if (context.pageType) {
      const pagePatterns = sessionStore.getPatternsByPageType(context.pageType);
      patterns.push(...pagePatterns);
    }

    // Deduplicate and filter by confidence
    const uniquePatterns = this._deduplicatePatterns(patterns);
    const filteredPatterns = uniquePatterns.filter(
      (p) => p.confidence >= this.minConfidence,
    );

    // Sort by confidence (descending) and limit
    return filteredPatterns
      .sort((a, b) => b.confidence - a.confidence)
      .slice(0, this.maxPatterns);
  }

  /**
   * Extract goal type from goal string
   * @private
   */
  _extractGoalType(goal) {
    const goalLower = goal.toLowerCase();

    if (goalLower.includes("login") || goalLower.includes("sign in"))
      return "login";
    if (goalLower.includes("search") || goalLower.includes("find"))
      return "search";
    if (goalLower.includes("buy") || goalLower.includes("purchase"))
      return "purchase";
    if (goalLower.includes("navigate") || goalLower.includes("go to"))
      return "navigation";
    if (goalLower.includes("fill") || goalLower.includes("form")) return "form";
    if (goalLower.includes("click") || goalLower.includes("press"))
      return "interaction";

    return "general";
  }

  /**
   * Deduplicate patterns
   * @private
   */
  _deduplicatePatterns(patterns) {
    const seen = new Set();
    const unique = [];

    for (const pattern of patterns) {
      const key = `${pattern.selector}|${pattern.action}`;
      if (!seen.has(key)) {
        seen.add(key);
        unique.push(pattern);
      }
    }

    return unique;
  }

  /**
   * Format a pattern for display
   * @private
   */
  _formatPattern(pattern) {
    let formatted = `- **${pattern.action}**`;

    if (pattern.selector) {
      formatted += ` on \`${pattern.selector}\``;
    }

    if (pattern.description) {
      formatted += `: ${pattern.description}`;
    }

    formatted += ` (${Math.round(pattern.confidence * 100)}% success, used ${pattern.timesUsed}x)\n`;

    return formatted;
  }

  /**
   * Inject quick tips based on common mistakes
   * @param {Array} recentErrors - Recent error messages
   * @returns {string} Quick tips string
   */
  injectQuickTips(recentErrors) {
    if (!recentErrors || recentErrors.length === 0) {
      return "";
    }

    const tips = [];

    // Analyze errors for common patterns
    for (const error of recentErrors.slice(-3)) {
      const tip = this._getTipForError(error);
      if (tip && !tips.includes(tip)) {
        tips.push(tip);
      }
    }

    if (tips.length === 0) {
      return "";
    }

    let tipStr = "\n## 💡 Quick Tips\n";
    for (const tip of tips) {
      tipStr += `- ${tip}\n`;
    }

    return tipStr;
  }

  /**
   * Get tip for specific error
   * @private
   */
  _getTipForError(error) {
    const errorLower = error.toLowerCase();

    if (errorLower.includes("selector") || errorLower.includes("not found")) {
      return "Try using a different selector (class, id, text, or role-based)";
    }

    if (errorLower.includes("not visible") || errorLower.includes("hidden")) {
      return "Scroll to the element first or wait for it to become visible";
    }

    if (errorLower.includes("timeout")) {
      return "Increase wait time or check if page is still loading";
    }

    if (errorLower.includes("verification")) {
      return "Try verifying with a different method (URL, visual, or AXTree)";
    }

    return null;
  }

  /**
   * Get success patterns for a specific action type
   * @param {string} actionType - Action type (click, type, etc.)
   * @param {string} url - Current URL
   * @returns {Array} Successful patterns
   */
  getSuccessPatterns(actionType, url) {
    const patterns = sessionStore.getSuccessfulPatterns(url, actionType);
    return patterns.slice(0, 3); // Return top 3
  }

  /**
   * Check if we have learned patterns for current context
   * @param {object} context - Current context
   * @returns {boolean} True if patterns exist
   */
  hasPatterns(context) {
    const patterns = sessionStore.getPatternsByUrl(context.url || "");
    return patterns.length > 0;
  }
}

const memoryInjector = new MemoryInjector();

export { memoryInjector };
export default memoryInjector;
