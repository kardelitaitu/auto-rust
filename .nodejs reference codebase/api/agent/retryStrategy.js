/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Intelligent Retry Strategy
 * Provides smart retry with different approaches based on error type
 * @module api/agent/retryStrategy
 */

import { createLogger } from "../core/logger.js";

const logger = createLogger("api/agent/retryStrategy.js");

class RetryStrategy {
  constructor() {
    this.strategies = [
      "same_action_retry",
      "alternative_selector",
      "coordinate_click",
      "wait_and_retry",
      "scroll_and_retry",
      "navigate_back",
      "simplify_selector",
      "use_text_selector",
    ];

    this.strategyDescriptions = {
      same_action_retry: "Retry the same action with a small delay",
      alternative_selector: "Try a different selector for the same element",
      coordinate_click: "Use coordinates instead of selector",
      wait_and_retry: "Wait longer and retry",
      scroll_and_retry: "Scroll to make element visible, then retry",
      navigate_back: "Navigate back and try again",
      simplify_selector: "Use a simpler, more general selector",
      use_text_selector: "Use text-based selector instead of CSS",
    };
  }

  /**
   * Get next retry strategy based on error and attempt
   * @param {string} error - Error message
   * @param {object} failedAction - Failed action object
   * @param {number} attempt - Current attempt number (0-based)
   * @param {object} context - Current context
   * @returns {object} Strategy object { name, description, action }
   */
  getNextStrategy(error, failedAction, attempt, context = {}) {
    const errorType = this._classifyError(error);

    // Select strategy based on error type and attempt
    let strategyName;

    switch (errorType) {
      case "selector_not_found":
        strategyName = this._getSelectorStrategy(attempt);
        break;

      case "element_not_visible":
        strategyName = attempt === 0 ? "scroll_and_retry" : "wait_and_retry";
        break;

      case "timeout":
        strategyName = attempt === 0 ? "wait_and_retry" : "same_action_retry";
        break;

      case "action_failed":
        strategyName = this._getActionStrategy(attempt);
        break;

      case "verification_failed":
        strategyName = attempt === 0 ? "wait_and_retry" : "same_action_retry";
        break;

      default:
        // Cycle through strategies
        strategyName = this.strategies[attempt % this.strategies.length];
    }

    const action = this._generateRetryAction(
      strategyName,
      failedAction,
      context,
    );

    logger.info(
      `[RetryStrategy] Strategy for attempt ${attempt}: ${strategyName}`,
    );

    return {
      name: strategyName,
      description: this.strategyDescriptions[strategyName],
      action,
    };
  }

  /**
   * Get selector-specific strategy
   * @private
   */
  _getSelectorStrategy(attempt) {
    const selectorStrategies = [
      "alternative_selector",
      "simplify_selector",
      "use_text_selector",
      "coordinate_click",
    ];

    return selectorStrategies[attempt % selectorStrategies.length];
  }

  /**
   * Get action-specific strategy
   * @private
   */
  _getActionStrategy(attempt) {
    const actionStrategies = [
      "same_action_retry",
      "wait_and_retry",
      "alternative_selector",
      "navigate_back",
    ];

    return actionStrategies[attempt % actionStrategies.length];
  }

  /**
   * Classify error type
   * @private
   */
  _classifyError(error) {
    const errorLower = error.toLowerCase();

    if (
      errorLower.includes("selector") ||
      errorLower.includes("not found") ||
      errorLower.includes("cannot find")
    ) {
      return "selector_not_found";
    }

    if (
      errorLower.includes("not visible") ||
      errorLower.includes("hidden") ||
      errorLower.includes("obscured")
    ) {
      return "element_not_visible";
    }

    if (errorLower.includes("timeout") || errorLower.includes("timed out")) {
      return "timeout";
    }

    if (errorLower.includes("verification") || errorLower.includes("verify")) {
      return "verification_failed";
    }

    return "action_failed";
  }

  /**
   * Generate retry action based on strategy
   * @private
   */
  _generateRetryAction(strategyName, failedAction, context) {
    const action = { ...failedAction };

    switch (strategyName) {
      case "same_action_retry":
        // Just add a small delay
        return {
          action: "wait",
          value: "500",
          rationale: "Wait before retrying same action",
        };

      case "alternative_selector":
        // Try to generate alternative selector
        if (action.selector) {
          action.selector = this._generateAlternativeSelector(action.selector);
          action.rationale = `Retry with alternative selector: ${action.selector}`;
        }
        return action;

      case "coordinate_click":
        // Convert to coordinate click if we have coordinates
        if (context.lastClickCoords) {
          return {
            action: "clickAt",
            x: context.lastClickCoords.x,
            y: context.lastClickCoords.y,
            rationale: "Retry using coordinates instead of selector",
          };
        }
        // Fallback to wait
        return {
          action: "wait",
          value: "1000",
          rationale: "Wait for element to become available",
        };

      case "wait_and_retry":
        return {
          action: "wait",
          value: "2000",
          rationale: "Wait longer for page to stabilize",
        };

      case "scroll_and_retry":
        return {
          action: "scroll",
          value: "down",
          rationale: "Scroll to make element visible",
        };

      case "navigate_back":
        return {
          action: "navigate",
          value: context.previousUrl || "/",
          rationale: "Navigate back and try again",
        };

      case "simplify_selector":
        if (action.selector) {
          action.selector = this._simplifySelector(action.selector);
          action.rationale = `Retry with simplified selector: ${action.selector}`;
        }
        return action;

      case "use_text_selector":
        if (action.selector) {
          // Extract text from selector or use action name
          const text =
            this._extractTextFromSelector(action.selector) || action.rationale;
          action.selector = `text=${text}`;
          action.rationale = `Retry using text selector: ${text}`;
        }
        return action;

      default:
        return action;
    }
  }

  /**
   * Generate alternative selector
   * @private
   */
  _generateAlternativeSelector(selector) {
    // If ID selector, try class
    if (selector.startsWith("#")) {
      const id = selector.substring(1);
      return `.${id}`;
    }

    // If class selector, try ID
    if (selector.startsWith(".")) {
      const className = selector.substring(1);
      return `#${className}`;
    }

    // If complex selector, try simpler version
    if (selector.includes(" ")) {
      return selector.split(" ").pop();
    }

    return selector;
  }

  /**
   * Simplify selector
   * @private
   */
  _simplifySelector(selector) {
    // Remove attribute selectors
    let simplified = selector.replace(/\[[^\]]+\]/g, "");

    // Remove :nth-child, :first, etc.
    simplified = simplified.replace(/:[a-z-]+\([^)]*\)/g, "");
    simplified = simplified.replace(/:[a-z-]+/g, "");

    // Clean up extra spaces
    simplified = simplified.replace(/\s+/g, " ").trim();

    return simplified || selector;
  }

  /**
   * Extract text from selector
   * @private
   */
  _extractTextFromSelector(selector) {
    // Try to extract meaningful text
    const parts = selector.split(/[#.:[\]\s]+/);
    const meaningful = parts.filter((p) => p.length > 2 && !p.match(/^\d+$/));
    return meaningful[0] || "";
  }

  /**
   * Check if should retry
   * @param {number} attempt - Current attempt number
   * @param {number} maxAttempts - Maximum attempts allowed
   * @param {string} error - Error message
   * @returns {boolean} True if should retry
   */
  shouldRetry(attempt, maxAttempts = 3, error = "") {
    if (attempt >= maxAttempts) {
      return false;
    }

    // Don't retry certain fatal errors
    const fatalErrors = ["out of memory", "crash", "fatal"];
    const errorLower = error.toLowerCase();

    for (const fatal of fatalErrors) {
      if (errorLower.includes(fatal)) {
        return false;
      }
    }

    return true;
  }

  /**
   * Get delay before retry
   * @param {number} attempt - Current attempt number
   * @param {string} strategyName - Strategy name
   * @returns {number} Delay in milliseconds
   */
  getRetryDelay(attempt, strategyName) {
    // Base delay with exponential backoff
    let delay = 1000 * Math.pow(2, attempt);

    // Adjust based on strategy
    switch (strategyName) {
      case "wait_and_retry":
        delay *= 2; // Longer wait
        break;
      case "scroll_and_retry":
        delay = 500; // Short delay for scroll
        break;
      case "same_action_retry":
        delay = 500; // Short delay
        break;
    }

    // Cap at 10 seconds
    return Math.min(delay, 10000);
  }

  /**
   * Get strategy statistics
   * @returns {object} Statistics
   */
  getStats() {
    return {
      totalStrategies: this.strategies.length,
      strategies: this.strategies,
    };
  }
}

const retryStrategy = new RetryStrategy();

export { retryStrategy };
export default retryStrategy;
