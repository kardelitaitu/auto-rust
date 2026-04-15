/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Configuration Validator
 * Provides schema-based validation for task configurations
 * @module utils/config-validator
 */

import { createLogger } from "../core/logger.js";

const logger = createLogger("config-validator.js");

/**
 * Configuration Validator
 * Validates configuration objects against predefined schemas
 */
export class ConfigValidator {
  constructor() {
    this.schemas = this.initializeSchemas();
    this.errorCodeMap = this.initializeErrorCodeMap();
    this.suggestions = this.initializeSuggestions();
  }

  /**
   * Initialize error code mapping for better error handling
   * @private
   */
  initializeErrorCodeMap() {
    return {
      "must be of type": "INVALID_TYPE",
      "must be one of": "INVALID_ENUM",
      "must be >= ": "VALUE_TOO_LOW",
      "must be <= ": "VALUE_TOO_HIGH",
      "is required but missing": "MISSING_REQUIRED",
      "Unknown section": "UNKNOWN_SECTION",
      "at least ": "ARRAY_TOO_SHORT",
      "at most ": "ARRAY_TOO_LONG",
    };
  }

  /**
   * Initialize suggestions for common mistakes
   * @private
   */
  initializeSuggestions() {
    return {
      cycles: { near: 10, suggestion: "10 cycles is typical for testing" },
      timeout: {
        near: 30000,
        suggestion: "30000ms (30s) is recommended for most operations",
      },
      minDuration: {
        near: 300,
        suggestion: "300 seconds (5 min) is a good minimum",
      },
      maxDuration: {
        near: 600,
        suggestion: "600 seconds (10 min) is a good maximum",
      },
      speed: { near: 1.0, suggestion: "speed: 1.0 is normal, 2.0 is faster" },
    };
  }

  /**
   * Get error code from error message
   * @param {string} error - Error message
   * @returns {string} Error code
   * @private
   */
  getErrorCode(error) {
    for (const [pattern, code] of Object.entries(this.errorCodeMap)) {
      if (error.includes(pattern)) {
        return code;
      }
    }
    return "VALIDATION_ERROR";
  }

  /**
   * Get suggestion for a field based on its value
   * @param {string} fieldName - Field name
   * @param {any} value - Current value
   * @returns {string|null} Suggestion or null
   * @private
   */
  getSuggestion(fieldName, value) {
    const fieldBase = fieldName.split(".").pop();

    if (this.suggestions[fieldBase] && typeof value === "number") {
      const { near, suggestion } = this.suggestions[fieldBase];
      if (Math.abs(value - near) < near * 0.5) {
        return suggestion;
      }
    }

    // Common typos
    const commonTypos = {
      timout: "timeout",
      maxDuraton: "maxDuration",
      minDuraton: "minDuration",
      enabeld: "enabled",
      dissabled: "disabled",
    };

    if (commonTypos[fieldBase]) {
      return `Did you mean '${commonTypos[fieldBase]}'?`;
    }

    return null;
  }

  /**
   * Initialize validation schemas
   * @private
   */
  initializeSchemas() {
    return {
      session: {
        cycles: { type: "number", min: 1, max: 100, required: true },
        minDuration: { type: "number", min: 60, max: 3600, required: true },
        maxDuration: { type: "number", min: 120, max: 7200, required: true },
        timeout: { type: "number", min: 30000, max: 3600000, required: false },
      },
      engagement: {
        limits: {
          type: "object",
          properties: {
            replies: { type: "number", min: 0, max: 20, required: true },
            retweets: { type: "number", min: 0, max: 10, required: true },
            quotes: { type: "number", min: 0, max: 10, required: true },
            likes: { type: "number", min: 0, max: 50, required: true },
            follows: { type: "number", min: 0, max: 20, required: true },
            bookmarks: { type: "number", min: 0, max: 20, required: true },
          },
          required: true,
        },
        probabilities: {
          type: "object",
          properties: {
            reply: { type: "number", min: 0, max: 1, required: true },
            quote: { type: "number", min: 0, max: 1, required: true },
            like: { type: "number", min: 0, max: 1, required: true },
            bookmark: { type: "number", min: 0, max: 1, required: true },
          },
          required: true,
        },
      },
      timing: {
        warmup: {
          type: "object",
          properties: {
            min: { type: "number", min: 1000, max: 30000, required: true },
            max: { type: "number", min: 2000, max: 60000, required: true },
          },
          required: true,
        },
        scroll: {
          type: "object",
          properties: {
            min: { type: "number", min: 100, max: 2000, required: true },
            max: { type: "number", min: 200, max: 5000, required: true },
          },
          required: true,
        },
        read: {
          type: "object",
          properties: {
            min: { type: "number", min: 1000, max: 60000, required: true },
            max: { type: "number", min: 2000, max: 120000, required: true },
          },
          required: true,
        },
        diveRead: { type: "number", min: 5000, max: 30000, required: false },
        globalScrollMultiplier: {
          type: "number",
          min: 0.1,
          max: 5.0,
          required: false,
        },
      },
      humanization: {
        mouse: {
          type: "object",
          properties: {
            speed: { type: "number", min: 0.1, max: 5.0, required: false },
            jitter: { type: "number", min: 0, max: 20, required: false },
          },
          required: false,
        },
        typing: {
          type: "object",
          properties: {
            delay: { type: "number", min: 10, max: 1000, required: false },
            errorRate: { type: "number", min: 0, max: 0.5, required: false },
          },
          required: false,
        },
        session: {
          type: "object",
          properties: {
            minMinutes: { type: "number", min: 1, max: 60, required: false },
            maxMinutes: { type: "number", min: 2, max: 120, required: false },
          },
          required: false,
        },
      },
      ai: {
        enabled: { type: "boolean", required: false },
        localEnabled: { type: "boolean", required: false },
        visionEnabled: { type: "boolean", required: false },
        timeout: { type: "number", min: 10000, max: 600000, required: false },
      },
      llm: {
        provider: {
          type: "string",
          enum: ["openrouter", "ollama", "anthropic", "openai"],
          required: false,
        },
        model: { type: "string", required: false },
        temperature: { type: "number", min: 0, max: 2.0, required: false },
        maxTokens: { type: "number", min: 100, max: 32000, required: false },
        endpoint: { type: "string", required: false },
      },
      browser: {
        theme: {
          type: "string",
          enum: ["light", "dark", "auto"],
          required: false,
        },
        referrer: {
          type: "object",
          properties: {
            addUTM: { type: "boolean", required: false },
          },
          required: false,
        },
        headers: {
          type: "object",
          properties: {
            secFetchSite: { type: "string", required: false },
            secFetchMode: { type: "string", required: false },
          },
          required: false,
        },
      },
      monitoring: {
        queueMonitor: {
          type: "object",
          properties: {
            enabled: { type: "boolean", required: false },
            interval: {
              type: "number",
              min: 1000,
              max: 300000,
              required: false,
            },
          },
          required: false,
        },
        engagementProgress: {
          type: "object",
          properties: {
            enabled: { type: "boolean", required: false },
            showProgressBar: { type: "boolean", required: false },
          },
          required: false,
        },
      },
      system: {
        debugMode: { type: "boolean", required: false },
        performanceTracking: { type: "boolean", required: false },
        errorRecovery: {
          type: "object",
          properties: {
            maxRetries: { type: "number", min: 0, max: 10, required: false },
            retryDelay: {
              type: "number",
              min: 1000,
              max: 60000,
              required: false,
            },
            fallbackStrategies: { type: "array", required: false },
          },
          required: false,
        },
      },
    };
  }

  /**
   * Validate configuration object against schemas
   * @param {object} config - Configuration to validate
   * @returns {object} Validation result with valid flag and errors array
   */
  validateConfig(config) {
    const errors = [];

    for (const [section, schema] of Object.entries(this.schemas)) {
      if (config[section] !== undefined) {
        const sectionErrors = this.validateSection(
          config[section],
          schema,
          section,
        );
        errors.push(...sectionErrors);
      }
    }

    const result = {
      valid: errors.length === 0,
      errors,
    };

    if (!result.valid) {
      logger.warn(
        `[ConfigValidator] Validation failed with ${errors.length} errors:`,
        errors,
      );
    }

    return result;
  }

  /**
   * Validate a specific configuration section
   * @private
   */
  validateSection(data, schema, sectionName) {
    const errors = [];

    // Handle schemas that are plain objects with field definitions (no type property)
    // This is the case for session, timing, humanization, etc. schemas
    if (
      schema.cycles !== undefined ||
      schema.enabled !== undefined ||
      schema.limits !== undefined ||
      schema.warmup !== undefined ||
      schema.mouse !== undefined ||
      schema.theme !== undefined ||
      schema.queueMonitor !== undefined ||
      schema.debugMode !== undefined
    ) {
      // This is a field-based schema (not type: 'object')
      for (const [field, rules] of Object.entries(schema)) {
        if (data[field] !== undefined) {
          const fieldErrors = this.validateField(
            data[field],
            rules,
            `${sectionName}.${field}`,
          );
          errors.push(...fieldErrors);
        } else if (rules.required) {
          errors.push(`${sectionName}.${field} is required but missing`);
        }
      }
    } else if (schema.type === "object") {
      // Validate object properties
      for (const [field, rules] of Object.entries(schema.properties || {})) {
        if (data[field] !== undefined) {
          const fieldErrors = this.validateField(
            data[field],
            rules,
            `${sectionName}.${field}`,
          );
          errors.push(...fieldErrors);
        } else if (rules.required) {
          errors.push(`${sectionName}.${field} is required but missing`);
        }
      }
    } else {
      // Validate primitive fields
      const fieldErrors = this.validateField(data, schema, sectionName);
      errors.push(...fieldErrors);
    }

    return errors;
  }

  /**
   * Validate a single field against its rules
   * @private
   */
  validateField(value, rules, fieldName) {
    const errors = [];

    // Check type
    if (rules.type && !this.checkType(value, rules.type)) {
      errors.push(
        `${fieldName} must be of type ${rules.type}, got ${typeof value}`,
      );
      return errors;
    }

    // Check enum values
    if (rules.enum && !rules.enum.includes(value)) {
      errors.push(
        `${fieldName} must be one of [${rules.enum.join(", ")}], got "${value}"`,
      );
    }

    // Check numeric constraints
    if (rules.type === "number") {
      if (rules.min !== undefined && value < rules.min) {
        errors.push(`${fieldName} must be >= ${rules.min}, got ${value}`);
      }
      if (rules.max !== undefined && value > rules.max) {
        errors.push(`${fieldName} must be <= ${rules.max}, got ${value}`);
      }
    }

    // Check array constraints
    if (rules.type === "array" && Array.isArray(value)) {
      if (rules.minItems !== undefined && value.length < rules.minItems) {
        errors.push(
          `${fieldName} must have at least ${rules.minItems} items, got ${value.length}`,
        );
      }
      if (rules.maxItems !== undefined && value.length > rules.maxItems) {
        errors.push(
          `${fieldName} must have at most ${rules.maxItems} items, got ${value.length}`,
        );
      }
    }

    return errors;
  }

  /**
   * Check if value matches expected type
   * @private
   */
  checkType(value, expectedType) {
    switch (expectedType) {
      case "string":
        return typeof value === "string";
      case "number":
        return typeof value === "number" && !isNaN(value);
      case "boolean":
        return typeof value === "boolean";
      case "object":
        return (
          typeof value === "object" && value !== null && !Array.isArray(value)
        );
      case "array":
        return Array.isArray(value);
      default:
        return typeof value === expectedType;
    }
  }

  /**
   * Validate specific configuration section
   * @param {object} data - Data to validate
   * @param {string} section - Section name (e.g., 'session', 'engagement')
   * @returns {object} Validation result
   */
  validateSectionConfig(data, section) {
    if (!this.schemas[section]) {
      return {
        valid: false,
        errors: [`Unknown section: ${section}`],
      };
    }

    const errors = this.validateSection(data, this.schemas[section], section);
    return {
      valid: errors.length === 0,
      errors,
    };
  }

  /**
   * Get validation schema for a specific section
   * @param {string} section - Section name
   * @returns {object|null} Schema object or null if not found
   */
  getSchema(section) {
    return this.schemas[section] || null;
  }

  /**
   * Validate configuration with detailed reporting
   * @param {object} config - Configuration to validate
   * @returns {object} Detailed validation report
   */
  validateWithReport(config) {
    const startTime = Date.now();
    const result = this.validateConfig(config);
    const duration = Date.now() - startTime;

    const report = {
      valid: result.valid,
      errors: result.errors,
      errorCount: result.errors.length,
      duration,
      sections: this.getSectionValidationReport(config),
    };

    logger.debug(
      `[ConfigValidator] Validation completed in ${duration}ms, ${result.errors.length} errors`,
    );

    return report;
  }

  /**
   * Get validation report for each section
   * @private
   */
  getSectionValidationReport(config) {
    const report = {};

    for (const [section, schema] of Object.entries(this.schemas)) {
      if (config[section] !== undefined) {
        const sectionErrors = this.validateSection(
          config[section],
          schema,
          section,
        );
        report[section] = {
          valid: sectionErrors.length === 0,
          errors: sectionErrors,
          errorCount: sectionErrors.length,
        };
      } else {
        report[section] = {
          valid: true,
          errors: [],
          errorCount: 0,
          skipped: true,
        };
      }
    }

    return report;
  }
}

// Export singleton instance
export const configValidator = new ConfigValidator();

// Convenience functions for backward compatibility
export const validateConfig = (config) =>
  configValidator.validateConfig(config);
export const validateWithReport = (config) =>
  configValidator.validateWithReport(config);

export default ConfigValidator;
