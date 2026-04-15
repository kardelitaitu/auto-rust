/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Confidence Scorer for LLM responses
 * Scores response confidence based on various factors
 * @module api/agent/confidenceScorer
 */

import { sessionStore } from "./sessionStore.js";

class ConfidenceScorer {
  constructor() {
    this.commonPatterns = {
      click: [
        "button",
        "link",
        "submit",
        "login",
        "sign",
        "start",
        "open",
        "view",
        "read",
        "see",
        "go",
        "navigate",
        "select",
        "choose",
      ],
      type: [
        "input",
        "field",
        "box",
        "text",
        "enter",
        "fill",
        "write",
        "username",
        "password",
        "email",
        "search",
        "query",
      ],
      scroll: [
        "more",
        "down",
        "up",
        "top",
        "bottom",
        "page",
        "content",
        "article",
        "post",
        "feed",
        "list",
        "results",
      ],
      wait: [
        "loading",
        "animation",
        "transition",
        "appear",
        "show",
        "complete",
        "finish",
        "ready",
        "stable",
      ],
    };
  }

  /**
   * Score response confidence
   * @param {object} response - LLM response object
   * @param {object} context - Current context { url, goal, pageType, axTree }
   * @returns {number} Confidence score (0-1)
   */
  score(response, context = {}) {
    let confidence = 0.5; // Base confidence

    // Factor 1: Response structure quality
    confidence += this._scoreStructure(response) * 0.15;

    // Factor 2: Selector quality
    confidence += this._scoreSelector(response) * 0.15;

    // Factor 3: Rationale quality
    confidence += this._scoreRationale(response) * 0.1;

    // Factor 4: Pattern matching
    confidence += this._scorePatternMatch(response, context) * 0.15;

    // Factor 5: Historical success rate
    confidence += this._scoreHistoricalSuccess(response, context) * 0.2;

    // Factor 6: Context relevance
    confidence += this._scoreContextRelevance(response, context) * 0.15;

    // Clamp to 0-1 range
    return Math.max(0, Math.min(1, confidence));
  }

  /**
   * Score response structure quality
   * @private
   */
  _scoreStructure(response) {
    let score = 0;

    // Handle null/undefined response
    if (!response) return score;

    // Has action field
    if (response.action) score += 0.3;

    // Has required fields for action type
    const requiredFields = this._getRequiredFields(response.action);
    const hasAllRequired = requiredFields.every(
      (field) => response[field] !== undefined,
    );
    if (hasAllRequired) score += 0.4;

    // No extra/unknown fields
    const validFields = [
      "action",
      "selector",
      "x",
      "y",
      "value",
      "key",
      "description",
      "rationale",
      "clickType",
      "duration",
      "items",
      "mode",
    ];
    const extraFields = Object.keys(response).filter(
      (f) => !validFields.includes(f),
    );
    if (extraFields.length === 0) score += 0.3;

    return score;
  }

  /**
   * Score selector quality
   * @private
   */
  _scoreSelector(response) {
    if (!response || !response.selector) return 0.5; // Neutral if no selector

    const selector = response.selector;
    let score = 0;

    // Specific selectors are better
    if (selector.startsWith("#"))
      score += 0.4; // ID
    else if (selector.startsWith("."))
      score += 0.3; // Class
    else if (selector.includes("["))
      score += 0.2; // Attribute
    else if (selector.includes("text="))
      score += 0.2; // Text
    else if (selector.includes("role=")) score += 0.3; // Role

    // Length indicates specificity
    if (selector.length > 10) score += 0.2;
    if (selector.length > 20) score += 0.1;

    // Not a placeholder
    const placeholders = ["...", "placeholder", "N/A"];
    if (!placeholders.includes(selector)) score += 0.3;

    return Math.min(1, score);
  }

  /**
   * Score rationale quality
   * @private
   */
  _scoreRationale(response) {
    if (!response || !response.rationale) return 0;

    const rationale = response.rationale.toLowerCase();
    let score = 0;

    // Length indicates thoughtfulness
    if (rationale.length > 10) score += 0.3;
    if (rationale.length > 30) score += 0.3;
    if (rationale.length > 50) score += 0.2;

    // Contains action verb
    const actionVerbs = [
      "click",
      "type",
      "scroll",
      "wait",
      "verify",
      "navigate",
      "select",
    ];
    if (actionVerbs.some((verb) => rationale.includes(verb))) score += 0.2;

    return Math.min(1, score);
  }

  /**
   * Score pattern match
   * @private
   */
  _scorePatternMatch(response, _context) {
    if (!response || !response.action || !response.rationale) return 0.5;

    const rationale = response.rationale.toLowerCase();
    const patterns = this.commonPatterns[response.action] || [];

    if (patterns.length === 0) return 0.5;

    const matches = patterns.filter((pattern) => rationale.includes(pattern));
    return matches.length / patterns.length;
  }

  /**
   * Score historical success rate
   * @private
   */
  _scoreHistoricalSuccess(response, context) {
    if (!response || !context.url || !response.selector) return 0.5;

    const successRate = sessionStore.getActionSuccessRate(
      context.url,
      response.selector,
      response.action,
    );

    return successRate;
  }

  /**
   * Score context relevance
   * @private
   */
  _scoreContextRelevance(response, context) {
    let score = 0.5;

    // Handle null/undefined response
    if (!response) return score;

    // Goal relevance
    if (context.goal && response.rationale) {
      const goalWords = context.goal.toLowerCase().split(" ");
      const rationaleWords = response.rationale.toLowerCase();
      const matches = goalWords.filter((word) => rationaleWords.includes(word));
      score += (matches.length / goalWords.length) * 0.3;
    }

    // Page type relevance
    if (context.pageType) {
      const pageTypeActions = {
        form: ["click", "type"],
        navigation: ["click", "scroll"],
        content: ["scroll", "click"],
        game: ["clickAt", "wait"],
        social: ["click", "type", "scroll"],
        ecommerce: ["click", "scroll"],
      };

      const expectedActions = pageTypeActions[context.pageType] || [];
      if (expectedActions.includes(response.action)) {
        score += 0.2;
      }
    }

    return Math.min(1, score);
  }

  /**
   * Get required fields for action type
   * @private
   */
  _getRequiredFields(action) {
    const requirements = {
      click: ["selector"],
      clickAt: ["x", "y"],
      type: ["selector", "value"],
      wait: ["value"],
      verify: ["description"],
      done: [],
      scroll: ["value"],
      navigate: ["value"],
    };

    return requirements[action] || [];
  }

  /**
   * Check if confidence is below threshold
   * @param {number} confidence - Confidence score
   * @param {number} threshold - Threshold (default 0.6)
   * @returns {boolean} True if should re-prompt
   */
  shouldReprompt(confidence, threshold = 0.6) {
    return confidence < threshold;
  }

  /**
   * Get confidence level description
   * @param {number} confidence - Confidence score
   * @returns {string} Confidence level
   */
  getConfidenceLevel(confidence) {
    if (confidence >= 0.8) return "high";
    if (confidence >= 0.6) return "medium";
    if (confidence >= 0.4) return "low";
    return "very_low";
  }

  /**
   * Get confidence summary
   * @param {number} confidence - Confidence score
   * @returns {string} Summary string
   */
  getSummary(confidence) {
    const level = this.getConfidenceLevel(confidence);
    const percentage = Math.round(confidence * 100);

    const icons = {
      high: "🟢",
      medium: "🟡",
      low: "🟠",
      very_low: "🔴",
    };

    return `${icons[level]} Confidence: ${percentage}% (${level})`;
  }
}

const confidenceScorer = new ConfidenceScorer();

export { confidenceScorer };
export default confidenceScorer;
