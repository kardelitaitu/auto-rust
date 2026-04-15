/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Goal Decomposition Engine
 * Breaks down complex goals into manageable sub-goals
 * @module api/agent/goalDecomposer
 */

import { createLogger } from "../core/logger.js";

const logger = createLogger("api/agent/goalDecomposer.js");

class GoalDecomposer {
  constructor() {
    this.goalPatterns = {
      login: {
        keywords: ["login", "sign in", "log in", "authenticate"],
        steps: [
          {
            subGoal: "Navigate to login page",
            expectedOutcome: "Login form visible",
          },
          {
            subGoal: "Enter username/email",
            expectedOutcome: "Username field filled",
          },
          {
            subGoal: "Enter password",
            expectedOutcome: "Password field filled",
          },
          { subGoal: "Click submit button", expectedOutcome: "User logged in" },
        ],
      },
      purchase: {
        keywords: ["buy", "purchase", "order", "checkout", "add to cart"],
        steps: [
          {
            subGoal: "Find the product",
            expectedOutcome: "Product page visible",
          },
          {
            subGoal: "Add product to cart",
            expectedOutcome: "Product in cart",
          },
          {
            subGoal: "Go to checkout",
            expectedOutcome: "Checkout page visible",
          },
          {
            subGoal: "Enter shipping details",
            expectedOutcome: "Shipping form filled",
          },
          { subGoal: "Complete payment", expectedOutcome: "Order confirmed" },
        ],
      },
      post: {
        keywords: ["post", "tweet", "publish", "share", "write"],
        steps: [
          {
            subGoal: "Navigate to compose area",
            expectedOutcome: "Compose form visible",
          },
          {
            subGoal: "Enter message content",
            expectedOutcome: "Content typed",
          },
          { subGoal: "Add media if needed", expectedOutcome: "Media attached" },
          {
            subGoal: "Click publish/post button",
            expectedOutcome: "Content published",
          },
        ],
      },
      search: {
        keywords: ["search", "find", "look for", "locate"],
        steps: [
          {
            subGoal: "Find search input",
            expectedOutcome: "Search field visible",
          },
          { subGoal: "Enter search query", expectedOutcome: "Query typed" },
          { subGoal: "Execute search", expectedOutcome: "Results displayed" },
          {
            subGoal: "Find target in results",
            expectedOutcome: "Target located",
          },
        ],
      },
      register: {
        keywords: ["register", "sign up", "create account"],
        steps: [
          {
            subGoal: "Navigate to registration page",
            expectedOutcome: "Registration form visible",
          },
          { subGoal: "Fill required fields", expectedOutcome: "Form filled" },
          {
            subGoal: "Accept terms if required",
            expectedOutcome: "Terms accepted",
          },
          {
            subGoal: "Submit registration",
            expectedOutcome: "Account created",
          },
        ],
      },
    };
  }

  /**
   * Decompose a complex goal into sub-goals
   * @param {string} goal - The goal to decompose
   * @param {string} currentUrl - Current page URL
   * @returns {Promise<object>} Decomposition result
   */
  async decompose(goal, currentUrl) {
    logger.info(`[GoalDecomposer] Decomposing goal: "${goal}"`);

    // Check if goal matches known pattern
    const pattern = this._matchPattern(goal);
    if (pattern) {
      logger.info(`[GoalDecomposer] Matched pattern: ${pattern.name}`);
      return {
        pattern: pattern.name,
        steps: pattern.steps,
        currentStep: 0,
        totalSteps: pattern.steps.length,
      };
    }

    // Use heuristic decomposition for unknown patterns
    logger.info(
      `[GoalDecomposer] No pattern match, using heuristic decomposition`,
    );
    return this._heuristicDecompose(goal, currentUrl);
  }

  /**
   * Match goal against known patterns
   * @private
   */
  _matchPattern(goal) {
    const goalLower = goal.toLowerCase();

    for (const [name, pattern] of Object.entries(this.goalPatterns)) {
      for (const keyword of pattern.keywords) {
        if (goalLower.includes(keyword)) {
          return { name, steps: pattern.steps };
        }
      }
    }

    return null;
  }

  /**
   * Heuristic decomposition for unknown goals
   * @private
   */
  _heuristicDecompose(goal, _currentUrl) {
    // Simple heuristic: break goal into action-oriented steps
    const steps = [];

    // Check for common action words
    const actionWords = {
      click: "Click on the target element",
      type: "Enter the required text",
      fill: "Fill in the form",
      select: "Select the option",
      navigate: "Navigate to the target page",
      wait: "Wait for the page to load",
      scroll: "Scroll to find the element",
    };

    const goalLower = goal.toLowerCase();
    let hasAction = false;

    for (const [word, description] of Object.entries(actionWords)) {
      if (goalLower.includes(word)) {
        steps.push({
          subGoal: description,
          expectedOutcome: "Action completed",
        });
        hasAction = true;
      }
    }

    // If no specific actions found, create generic steps
    if (!hasAction) {
      steps.push(
        {
          subGoal: "Analyze the current page state",
          expectedOutcome: "Page understood",
        },
        {
          subGoal: "Locate target element or area",
          expectedOutcome: "Target found",
        },
        {
          subGoal: "Perform the required action",
          expectedOutcome: "Action completed",
        },
        {
          subGoal: "Verify the action succeeded",
          expectedOutcome: "Goal achieved",
        },
      );
    }

    return {
      pattern: "heuristic",
      steps,
      currentStep: 0,
      totalSteps: steps.length,
    };
  }

  /**
   * Get next sub-goal from decomposition
   * @param {object} decomposition - Decomposition result
   * @returns {object|null} Next sub-goal or null if complete
   */
  getNextStep(decomposition) {
    if (decomposition.currentStep >= decomposition.totalSteps) {
      return null;
    }

    return {
      ...decomposition.steps[decomposition.currentStep],
      stepNumber: decomposition.currentStep + 1,
      totalSteps: decomposition.totalSteps,
    };
  }

  /**
   * Advance to next step
   * @param {object} decomposition - Decomposition result
   * @returns {object} Updated decomposition
   */
  advanceStep(decomposition) {
    return {
      ...decomposition,
      currentStep: decomposition.currentStep + 1,
    };
  }

  /**
   * Check if decomposition is complete
   * @param {object} decomposition - Decomposition result
   * @returns {boolean} True if all steps completed
   */
  isComplete(decomposition) {
    return decomposition.currentStep >= decomposition.totalSteps;
  }

  /**
   * Get progress percentage
   * @param {object} decomposition - Decomposition result
   * @returns {number} Progress percentage (0-100)
   */
  getProgress(decomposition) {
    if (decomposition.totalSteps === 0) return 100;
    return Math.round(
      (decomposition.currentStep / decomposition.totalSteps) * 100,
    );
  }
}

const goalDecomposer = new GoalDecomposer();

export { goalDecomposer };
export default goalDecomposer;
