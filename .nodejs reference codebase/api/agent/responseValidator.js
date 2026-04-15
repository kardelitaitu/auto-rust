/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Response Validator for LLM responses
 * Validates JSON structure and action parameters
 * @module api/agent/responseValidator
 */

class ResponseValidator {
  constructor() {
    this.validActions = [
      "click",
      "clickAt",
      "type",
      "wait",
      "verify",
      "done",
      "scroll",
      "navigate",
      "drag",
      "multiSelect",
      "press",
    ];

    this.requiredParams = {
      click: ["selector"],
      clickAt: ["x", "y"],
      type: ["selector", "value"],
      wait: ["value"],
      verify: ["description"],
      done: [],
      scroll: ["value"],
      navigate: ["value"],
      drag: ["selector", "target"],
      multiSelect: ["items"],
      press: ["key"],
    };

    this.selectorPatterns = {
      css: /^[#.]?[a-zA-Z][a-zA-Z0-9_-]*(\[[^\]]+\])*(\s[#.]?[a-zA-Z][a-zA-Z0-9_-]*(\[[^\]]+\])*)*$/,
      role: /^role=[a-z]+(,name="[^"]+")?$/,
      text: /^text=.+$/,
      xpath: /^\/\/.+/,
    };
  }

  /**
   * Validate LLM response
   * @param {object} response - LLM response object
   * @returns {object} Validation result { valid: boolean, errors: string[], warnings: string[] }
   */
  validate(response) {
    const _errors = [];
    const _warnings = [];

    // Check if response is an object
    if (!response || typeof response !== "object") {
      return {
        valid: false,
        errors: ["Response is not an object"],
        warnings: [],
      };
    }

    // Check if response is an array (multiple actions)
    if (Array.isArray(response)) {
      return this._validateArray(response);
    }

    // Validate single action
    return this._validateSingle(response);
  }

  /**
   * Validate array of actions
   * @private
   */
  _validateArray(actions) {
    const errors = [];
    const warnings = [];

    if (actions.length === 0) {
      return { valid: false, errors: ["Empty action array"], warnings: [] };
    }

    for (let i = 0; i < actions.length; i++) {
      const result = this._validateSingle(actions[i], i);
      errors.push(...result.errors.map((e) => `Action ${i}: ${e}`));
      warnings.push(...result.warnings.map((w) => `Action ${i}: ${w}`));
    }

    return { valid: errors.length === 0, errors, warnings };
  }

  /**
   * Validate single action
   * @private
   */
  _validateSingle(action, index = null) {
    const errors = [];
    const warnings = [];
    const prefix = index !== null ? `Action ${index}: ` : "";

    // Check for action type
    if (!action.action) {
      return {
        valid: false,
        errors: [`${prefix}Missing 'action' field`],
        warnings: [],
      };
    }

    // Check if action type is valid
    if (!this.validActions.includes(action.action)) {
      errors.push(`${prefix}Invalid action type: ${action.action}`);
      return { valid: false, errors, warnings };
    }

    // Check required parameters
    const required = this.requiredParams[action.action] || [];
    for (const param of required) {
      if (action[param] === undefined || action[param] === null) {
        errors.push(`${prefix}Missing required parameter: ${param}`);
      }
    }

    // Validate specific parameters
    if (action.selector) {
      const selectorValid = this._validateSelector(action.selector);
      if (!selectorValid.valid) {
        warnings.push(
          `${prefix}Selector may be invalid: ${selectorValid.reason}`,
        );
      }
    }

    if (action.x !== undefined && typeof action.x !== "number") {
      errors.push(`${prefix}'x' must be a number`);
    }

    if (action.y !== undefined && typeof action.y !== "number") {
      errors.push(`${prefix}'y' must be a number`);
    }

    if (
      action.value !== undefined &&
      typeof action.value !== "string" &&
      typeof action.value !== "number"
    ) {
      warnings.push(`${prefix}'value' should be a string or number`);
    }

    // Check for rationale (recommended)
    if (!action.rationale) {
      warnings.push(
        `${prefix}Missing 'rationale' (recommended for better decision tracking)`,
      );
    }

    return { valid: errors.length === 0, errors, warnings };
  }

  /**
   * Validate selector format
   * @private
   */
  _validateSelector(selector) {
    if (!selector || typeof selector !== "string") {
      return { valid: false, reason: "Selector is not a string" };
    }

    // Check for placeholder values
    const placeholders = ["...", "placeholder", "N/A"];
    if (placeholders.includes(selector)) {
      return { valid: false, reason: "Selector is a placeholder" };
    }

    // Check selector patterns
    for (const [type, pattern] of Object.entries(this.selectorPatterns)) {
      if (pattern.test(selector)) {
        return { valid: true, type };
      }
    }

    // If no pattern matches, it might still be valid (e.g., complex selectors)
    return { valid: true, type: "unknown" };
  }

  /**
   * Auto-correct common errors in response
   * @param {object} response - LLM response object
   * @returns {object} Corrected response
   */
  autoCorrect(response) {
    if (!response || typeof response !== "object") {
      return response;
    }

    if (Array.isArray(response)) {
      return response.map((r) => this._correctSingle(r));
    }

    return this._correctSingle(response);
  }

  /**
   * Auto-correct single action
   * @private
   */
  _correctSingle(action) {
    const corrected = { ...action };

    // Fix action type typos
    if (corrected.action) {
      corrected.action = this._correctActionType(corrected.action);
    }

    // Fix selector format
    if (corrected.selector && typeof corrected.selector === "string") {
      corrected.selector = this._correctSelector(corrected.selector);
    }

    // Ensure value is string for type action
    if (corrected.action === "type" && corrected.value !== undefined) {
      corrected.value = String(corrected.value);
    }

    // Ensure wait value is string
    if (corrected.action === "wait" && corrected.value !== undefined) {
      corrected.value = String(corrected.value);
    }

    // Ensure coordinates are numbers
    if (corrected.x !== undefined) {
      corrected.x = Number(corrected.x);
    }

    if (corrected.y !== undefined) {
      corrected.y = Number(corrected.y);
    }

    return corrected;
  }

  /**
   * Correct action type typos
   * @private
   */
  _correctActionType(action) {
    const actionLower = action.toLowerCase();

    // Common typos and variations
    const corrections = {
      clic: "click",
      clck: "click",
      clk: "click",
      tye: "type",
      tpye: "type",
      wiat: "wait",
      wt: "wait",
      verfy: "verify",
      vrify: "verify",
      don: "done",
      dn: "done",
      scrol: "scroll",
      scrll: "scroll",
      navgate: "navigate",
      naviagte: "navigate",
    };

    return corrections[actionLower] || action;
  }

  /**
   * Correct selector format
   * @private
   */
  _correctSelector(selector) {
    // Remove extra whitespace
    let corrected = selector.trim();

    // Add # prefix for IDs without prefix
    if (/^[a-zA-Z][a-zA-Z0-9_-]+$/.test(corrected)) {
      // Could be an ID or class, default to ID
      corrected = `#${corrected}`;
    }

    return corrected;
  }

  /**
   * Get validation summary
   * @param {object} validationResult - Result from validate()
   * @returns {string} Summary string
   */
  getSummary(validationResult) {
    if (validationResult.valid) {
      if (validationResult.warnings.length > 0) {
        return `✅ Valid with ${validationResult.warnings.length} warning(s)`;
      }
      return "✅ Valid";
    }

    return `❌ Invalid: ${validationResult.errors.length} error(s), ${validationResult.warnings.length} warning(s)`;
  }
}

const responseValidator = new ResponseValidator();

export { responseValidator };
export default responseValidator;
