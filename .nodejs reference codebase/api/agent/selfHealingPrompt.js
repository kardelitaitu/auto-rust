/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Self-Healing Prompt Generator
 * Generates adaptive prompts based on failures
 * @module api/agent/selfHealingPrompt
 */

import { createLogger } from "../core/logger.js";

const logger = createLogger("api/agent/selfHealingPrompt.js");

class SelfHealingPrompt {
  constructor() {
    this.recentFailures = [];
    this.maxFailures = 10;
  }

  /**
   * Record a failure
   * @param {object} failure - Failure object { action, error, timestamp, context }
   */
  recordFailure(failure) {
    this.recentFailures.push({
      ...failure,
      timestamp: failure.timestamp || Date.now(),
    });

    // Keep only recent failures
    if (this.recentFailures.length > this.maxFailures) {
      this.recentFailures = this.recentFailures.slice(-this.maxFailures);
    }
  }

  /**
   * Generate healing instructions based on recent failures
   * @param {object} context - Current context
   * @returns {string} Healing instructions for prompt
   */
  generateHealingInstructions(context) {
    if (this.recentFailures.length === 0) {
      return "";
    }

    // Filter relevant failures
    const relevantFailures = this._getRelevantFailures(context);

    if (relevantFailures.length === 0) {
      return "";
    }

    let instructions = "\n## ⚠️ Recent Failures to Avoid\n";
    instructions +=
      "The following actions have failed recently. Consider alternative approaches:\n\n";

    for (const failure of relevantFailures.slice(-3)) {
      instructions += this._formatFailure(failure);
    }

    // Add general advice based on failure patterns
    instructions += "\n### General Advice\n";
    instructions += this._generateGeneralAdvice(relevantFailures);

    return instructions;
  }

  /**
   * Get relevant failures for current context
   * @private
   */
  _getRelevantFailures(context) {
    return this.recentFailures.filter((failure) => {
      // Same URL = relevant
      if (failure.context?.url && context.url) {
        const failureUrl = this._normalizeUrl(failure.context.url);
        const contextUrl = this._normalizeUrl(context.url);
        if (failureUrl === contextUrl) {
          return true;
        }
      }

      // Same goal = relevant
      if (failure.context?.goal && context.goal) {
        const similarity = this._stringSimilarity(
          failure.context.goal,
          context.goal,
        );
        if (similarity > 0.5) {
          return true;
        }
      }

      // Recent failures (last 5 minutes) are always relevant
      const age = Date.now() - failure.timestamp;
      if (age < 300000) {
        return true;
      }

      return false;
    });
  }

  /**
   * Format a failure for display
   * @private
   */
  _formatFailure(failure) {
    let formatted = `- **Action**: ${failure.action?.action || "unknown"}\n`;
    formatted += `  **Error**: ${failure.error || "Unknown error"}\n`;

    if (failure.alternative) {
      formatted += `  **Alternative**: ${failure.alternative}\n`;
    }

    formatted += "\n";
    return formatted;
  }

  /**
   * Generate general advice based on failure patterns
   * @private
   */
  _generateGeneralAdvice(failures) {
    const advice = [];
    const errorTypes = failures.map((f) => this._classifyError(f.error));

    // Count error types
    const errorCounts = {};
    for (const type of errorTypes) {
      errorCounts[type] = (errorCounts[type] || 0) + 1;
    }

    // Generate advice based on most common errors
    if (errorCounts.selector_not_found > 1) {
      advice.push(
        "Multiple selector failures detected. Try using text-based or role-based selectors.",
      );
    }

    if (errorCounts.element_not_visible > 1) {
      advice.push(
        "Elements are frequently not visible. Always scroll to elements before clicking.",
      );
    }

    if (errorCounts.timeout > 1) {
      advice.push(
        "Multiple timeouts detected. Consider increasing wait times or checking page load status.",
      );
    }

    if (failures.length >= 3) {
      advice.push(
        "Multiple failures detected. Consider taking a completely different approach.",
      );
    }

    if (advice.length === 0) {
      advice.push("Try a different approach or wait before retrying.");
    }

    return advice.map((a) => `- ${a}`).join("\n");
  }

  /**
   * Classify error type
   * @private
   */
  _classifyError(error) {
    if (!error) return "unknown";

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
   * Generate recovery prompt for specific failure
   * @param {object} failure - Failure object
   * @returns {string} Recovery prompt
   */
  generateRecoveryPrompt(failure) {
    let prompt = "\n## 🔄 Recovery Mode\n";
    prompt += `The previous action failed: ${failure.error}\n\n`;

    prompt += "### Recovery Options\n";
    prompt += "1. Try a different selector or approach\n";
    prompt += "2. Wait and retry the same action\n";
    prompt += "3. Navigate to a different page and come back\n";
    prompt +=
      "4. Simplify the action (e.g., use coordinates instead of selector)\n";

    if (failure.alternative) {
      prompt += `\n### Suggested Alternative\n`;
      prompt += `${failure.alternative}\n`;
    }

    return prompt;
  }

  /**
   * Check if in recovery mode (multiple recent failures)
   * @returns {boolean} True if in recovery mode
   */
  isInRecoveryMode() {
    const recentWindow = 60000; // 1 minute
    const recentFailures = this.recentFailures.filter(
      (f) => Date.now() - f.timestamp < recentWindow,
    );

    return recentFailures.length >= 2;
  }

  /**
   * Get failure statistics
   * @returns {object} Statistics
   */
  getStats() {
    const errorTypes = {};

    for (const failure of this.recentFailures) {
      const type = this._classifyError(failure.error);
      errorTypes[type] = (errorTypes[type] || 0) + 1;
    }

    return {
      totalFailures: this.recentFailures.length,
      errorTypes,
      inRecoveryMode: this.isInRecoveryMode(),
    };
  }

  /**
   * Clear all failures
   */
  clear() {
    this.recentFailures = [];
    logger.info("[SelfHealingPrompt] All failures cleared");
  }

  /**
   * Get recent failures
   * @param {number} count - Number of failures to return
   * @returns {Array} Recent failures
   */
  getRecentFailures(count = 5) {
    return this.recentFailures.slice(-count);
  }
}

const selfHealingPrompt = new SelfHealingPrompt();

export { selfHealingPrompt };
export default selfHealingPrompt;
