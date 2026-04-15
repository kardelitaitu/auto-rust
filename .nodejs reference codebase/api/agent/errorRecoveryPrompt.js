/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Error Recovery Prompt Generator
 * Generates error-specific prompt templates and recovery strategies
 * @module api/agent/errorRecoveryPrompt
 */

class ErrorRecoveryPrompt {
  constructor() {
    this.errorPatterns = {
      selector_not_found: {
        patterns: [
          "selector",
          "not found",
          "no element",
          "element not found",
          "cannot find",
        ],
        recovery: [
          "Try a different selector (class, id, text, role)",
          "Use coordinates with clickAt instead",
          "Scroll to make the element visible",
          "Wait for the element to load",
        ],
      },
      element_not_visible: {
        patterns: [
          "not visible",
          "hidden",
          "obscured",
          "out of view",
          "not in viewport",
        ],
        recovery: [
          "Scroll to the element first",
          "Check if element is behind another element",
          "Wait for animation to complete",
          "Try clicking at element coordinates",
        ],
      },
      timeout: {
        patterns: [
          "timeout",
          "timed out",
          "wait timed out",
          "navigation timeout",
        ],
        recovery: [
          "Increase wait time",
          "Check if page is still loading",
          "Try refreshing the page",
          "Navigate to a different page first",
        ],
      },
      action_failed: {
        patterns: [
          "action failed",
          "execution failed",
          "click failed",
          "type failed",
        ],
        recovery: [
          "Try the action again",
          "Use a different approach",
          "Check if page state changed",
          "Verify element is still present",
        ],
      },
      verification_failed: {
        patterns: ["verification failed", "verify failed", "check failed"],
        recovery: [
          "Wait longer before verifying",
          "Check different verification criteria",
          "Use visual comparison instead",
          "Skip verification and continue",
        ],
      },
      navigation_failed: {
        patterns: ["navigation failed", "goto failed", "page load failed"],
        recovery: [
          "Check URL is correct",
          "Try navigating to a simpler page first",
          "Wait for network to stabilize",
          "Use browser back button",
        ],
      },
      stuck: {
        patterns: ["stuck", "no progress", "no change", "same state"],
        recovery: [
          "Try a completely different approach",
          "Navigate to a different page",
          "Wait for external changes",
          "Reset and start over",
        ],
      },
    };
  }

  /**
   * Generate recovery prompt based on error
   * @param {string} error - Error message
   * @param {object} failedAction - Failed action object
   * @param {number} attempts - Number of attempts made
   * @param {object} context - Current context (url, goal, etc.)
   * @returns {string} Recovery prompt
   */
  generateRecoveryPrompt(error, failedAction, attempts, context = {}) {
    const errorType = this._classifyError(error);
    const pattern = this.errorPatterns[errorType];

    let prompt = "\n## ⚠️ Previous Attempt Failed\n";
    prompt += `**Error Type**: ${errorType}\n`;
    prompt += `**Error Message**: ${error}\n`;
    prompt += `**Failed Action**: ${JSON.stringify(failedAction)}\n`;
    prompt += `**Attempts**: ${attempts}\n\n`;

    if (pattern) {
      prompt += "### Recovery Strategies\n";
      for (const strategy of pattern.recovery) {
        prompt += `- ${strategy}\n`;
      }
    } else {
      prompt += "### General Recovery Strategies\n";
      prompt += "- Try a different approach\n";
      prompt += "- Wait and retry\n";
      prompt += "- Check page state\n";
      prompt += "- Use alternative selectors\n";
    }

    // Add context-specific tips
    if (context.goal) {
      prompt += "\n### Goal Reminder\n";
      prompt += `Your goal is: "${context.goal}"\n`;
      prompt += "Focus on achieving this goal with a different approach.\n";
    }

    // Add attempt-specific advice
    if (attempts >= 3) {
      prompt += "\n### ⚠️ Multiple Failures Detected\n";
      prompt += "You have failed multiple times. Consider:\n";
      prompt += "- Taking a completely different approach\n";
      prompt += "- Breaking the goal into smaller steps\n";
      prompt += "- Verifying the page is in the expected state\n";
    }

    return prompt;
  }

  /**
   * Classify error type from error message
   * @private
   */
  _classifyError(error) {
    const errorLower = error.toLowerCase();

    for (const [type, pattern] of Object.entries(this.errorPatterns)) {
      for (const keyword of pattern.patterns) {
        if (errorLower.includes(keyword)) {
          return type;
        }
      }
    }

    return "unknown";
  }

  /**
   * Generate quick recovery hint for inline use
   * @param {string} error - Error message
   * @returns {string} Quick hint
   */
  getQuickHint(error) {
    const errorType = this._classifyError(error);
    const pattern = this.errorPatterns[errorType];

    if (pattern && pattern.recovery.length > 0) {
      return `💡 Hint: ${pattern.recovery[0]}`;
    }

    return "💡 Hint: Try a different approach";
  }

  /**
   * Generate recovery actions for specific error
   * @param {string} error - Error message
   * @param {object} failedAction - Failed action object
   * @returns {Array} Array of recovery action suggestions
   */
  getRecoveryActions(error, failedAction) {
    const errorType = this._classifyError(error);
    const actions = [];

    switch (errorType) {
      case "selector_not_found":
        // Suggest alternative selectors
        if (failedAction.selector) {
          actions.push({
            action: "click",
            selector: failedAction.selector,
            rationale: "Retry with same selector",
          });
          actions.push({
            action: "click",
            selector: `text=${failedAction.selector}`,
            rationale: "Try text-based selector",
          });
        }
        break;

      case "element_not_visible":
        // Suggest scroll first
        actions.push({
          action: "scroll",
          value: "down",
          rationale: "Scroll to make element visible",
        });
        break;

      case "timeout":
        // Suggest wait
        actions.push({
          action: "wait",
          value: "2000",
          rationale: "Wait for page to stabilize",
        });
        break;

      default:
        // Generic recovery
        actions.push({
          action: "wait",
          value: "1000",
          rationale: "Wait before retrying",
        });
    }

    return actions;
  }

  /**
   * Check if error is recoverable
   * @param {string} error - Error message
   * @returns {boolean} True if error is recoverable
   */
  isRecoverable(error) {
    const _errorType = this._classifyError(error);

    // Most errors are recoverable except critical ones
    const nonRecoverable = ["fatal", "crash", "out of memory"];
    const errorLower = error.toLowerCase();

    for (const keyword of nonRecoverable) {
      if (errorLower.includes(keyword)) {
        return false;
      }
    }

    return true;
  }

  /**
   * Get max recommended attempts for error type
   * @param {string} error - Error message
   * @returns {number} Max attempts
   */
  getMaxAttempts(error) {
    const errorType = this._classifyError(error);

    const maxAttempts = {
      selector_not_found: 3,
      element_not_visible: 2,
      timeout: 2,
      action_failed: 3,
      verification_failed: 2,
      navigation_failed: 2,
      stuck: 2,
      unknown: 3,
    };

    return maxAttempts[errorType] || 3;
  }
}

const errorRecoveryPrompt = new ErrorRecoveryPrompt();

export { errorRecoveryPrompt };
export default errorRecoveryPrompt;
