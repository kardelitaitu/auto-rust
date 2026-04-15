/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Environment Configuration
 * Handles environment variable overrides for configuration
 * @module utils/environment-config
 */

import { createLogger } from "../core/logger.js";

const logger = createLogger("environment-config.js");

/**
 * Environment Configuration
 * Manages environment variable overrides for configuration
 */
export class EnvironmentConfig {
  constructor() {
    this.overrides = this.getEnvOverrides();
    this.appliedOverrides = new Map();
  }

  /**
   * Get environment variable override mappings
   * @returns {object} Environment variable to config path mappings
   */
  getEnvOverrides() {
    return {
      // Session overrides
      TWITTER_CYCLES: "session.cycles",
      TWITTER_MIN_DURATION: "session.minDuration",
      TWITTER_MAX_DURATION: "session.maxDuration",
      TWITTER_TIMEOUT_MS: "session.timeout",

      // Engagement overrides
      TWITTER_REPLY_PROBABILITY: "engagement.probabilities.reply",
      TWITTER_QUOTE_PROBABILITY: "engagement.probabilities.quote",
      TWITTER_LIKE_PROBABILITY: "engagement.probabilities.like",
      TWITTER_BOOKMARK_PROBABILITY: "engagement.probabilities.bookmark",

      // Engagement limits overrides
      TWITTER_MAX_REPLIES: "engagement.limits.replies",
      TWITTER_MAX_RETWEETS: "engagement.limits.retweets",
      TWITTER_MAX_QUOTES: "engagement.limits.quotes",
      TWITTER_MAX_LIKES: "engagement.limits.likes",
      TWITTER_MAX_FOLLOWS: "engagement.limits.follows",
      TWITTER_MAX_BOOKMARKS: "engagement.limits.bookmarks",

      // Timing overrides
      TWITTER_WARMUP_MIN: "timing.warmup.min",
      TWITTER_WARMUP_MAX: "timing.warmup.max",
      TWITTER_SCROLL_MIN: "timing.scroll.min",
      TWITTER_SCROLL_MAX: "timing.scroll.max",
      TWITTER_READ_MIN: "timing.read.min",
      TWITTER_READ_MAX: "timing.read.max",
      TWITTER_DIVE_READ: "timing.diveRead",
      GLOBAL_SCROLL_MULTIPLIER: "timing.globalScrollMultiplier",

      // Humanization overrides
      HUMAN_MOUSE_SPEED: "humanization.mouse.speed",
      HUMAN_MOUSE_JITTER: "humanization.mouse.jitter",
      HUMAN_TYPING_DELAY: "humanization.typing.delay",
      HUMAN_ERROR_RATE: "humanization.typing.errorRate",
      SESSION_MIN_MINUTES: "humanization.session.minMinutes",
      SESSION_MAX_MINUTES: "humanization.session.maxMinutes",

      // AI overrides
      AI_ENABLED: "ai.enabled",
      AI_LOCAL_ENABLED: "ai.localEnabled",
      AI_VISION_ENABLED: "ai.visionEnabled",
      AI_TIMEOUT: "ai.timeout",

      // Browser overrides
      BROWSER_THEME: "browser.theme",
      BROWSER_ADD_UTM: "browser.referrer.addUTM",
      BROWSER_SEC_FETCH_SITE: "browser.headers.secFetchSite",
      BROWSER_SEC_FETCH_MODE: "browser.headers.secFetchMode",

      // Monitoring overrides
      QUEUE_MONITOR_ENABLED: "monitoring.queueMonitor.enabled",
      QUEUE_MONITOR_INTERVAL: "monitoring.queueMonitor.interval",
      ENGAGEMENT_PROGRESS_ENABLED: "monitoring.engagementProgress.enabled",
      ENGAGEMENT_PROGRESS_SHOW_PROGRESS_BAR:
        "monitoring.engagementProgress.showProgressBar",

      // System overrides
      DEBUG_MODE: "system.debugMode",
      PERFORMANCE_TRACKING: "system.performanceTracking",
      ERROR_RECOVERY_MAX_RETRIES: "system.errorRecovery.maxRetries",
      ERROR_RECOVERY_RETRY_DELAY: "system.errorRecovery.retryDelay",
    };
  }

  /**
   * Apply environment variable overrides to configuration
   * @param {object} config - Configuration object to modify
   * @returns {object} Modified configuration with overrides applied
   */
  static applyEnvOverrides(config) {
    const overrides = new EnvironmentConfig().getEnvOverrides();
    const appliedOverrides = new Map();

    for (const [envVar, configPath] of Object.entries(overrides)) {
      const envValue = process.env[envVar];
      if (envValue !== undefined && envValue !== "") {
        try {
          const parsedValue = EnvironmentConfig.parseEnvValue(envVar, envValue);
          const oldValue = EnvironmentConfig.getNestedValue(config, configPath);

          EnvironmentConfig.setNestedValue(config, configPath, parsedValue);

          appliedOverrides.set(envVar, {
            path: configPath,
            oldValue,
            newValue: parsedValue,
            type: typeof parsedValue,
          });

          logger.info(
            `[EnvironmentConfig] Applied override: ${envVar} = ${parsedValue} (${typeof parsedValue})`,
          );
        } catch (error) {
          logger.warn(
            `[EnvironmentConfig] Failed to apply override ${envVar}: ${error.message}`,
          );
        }
      }
    }

    if (appliedOverrides.size > 0) {
      logger.info(
        `[EnvironmentConfig] Applied ${appliedOverrides.size} environment overrides`,
      );
      this.logAppliedOverrides(appliedOverrides);
    } else {
      logger.debug("[EnvironmentConfig] No environment overrides found");
    }

    return config;
  }

  /**
   * Parse environment variable value to appropriate type
   * @param {string} envVar - Environment variable name
   * @param {string} value - Environment variable value
   * @returns {any} Parsed value
   */
  static parseEnvValue(envVar, value) {
    // Handle boolean values
    if (value.toLowerCase() === "true") return true;
    if (value.toLowerCase() === "false") return false;

    // Handle numeric values
    if (!isNaN(value)) {
      const numValue = parseFloat(value);
      if (!isNaN(numValue)) {
        return numValue;
      }
    }

    // Handle array values (comma-separated)
    if (value.includes(",")) {
      return value.split(",").map((item) => item.trim());
    }

    // Default to string
    return value;
  }

  /**
   * Get nested value from object using dot notation path
   * @param {object} obj - Object to search
   * @param {string} path - Dot notation path (e.g., 'session.cycles')
   * @returns {any} Value at path or undefined
   */
  static getNestedValue(obj, path) {
    return path
      .split(".")
      .reduce((current, key) => current && current[key], obj);
  }

  /**
   * Set nested value in object using dot notation path
   * @param {object} obj - Object to modify
   * @param {string} path - Dot notation path (e.g., 'session.cycles')
   * @param {any} value - Value to set
   */
  static setNestedValue(obj, path, value) {
    const keys = path.split(".");
    let current = obj;

    for (let i = 0; i < keys.length - 1; i++) {
      const key = keys[i];
      if (!(key in current)) {
        current[key] = {};
      }
      current = current[key];
    }

    current[keys[keys.length - 1]] = value;
  }

  /**
   * Log applied overrides for debugging
   * @param {Map} appliedOverrides - Map of applied overrides
   */
  static logAppliedOverrides(appliedOverrides) {
    logger.debug("[EnvironmentConfig] Applied overrides:");
    for (const [envVar, details] of appliedOverrides.entries()) {
      logger.debug(
        `  ${envVar}: ${details.oldValue} → ${details.newValue} (${details.type})`,
      );
    }
  }

  /**
   * Get list of all supported environment variables
   * @returns {Array} Array of environment variable names
   */
  getSupportedEnvVars() {
    return Object.keys(this.overrides);
  }

  /**
   * Check if environment variable is supported
   * @param {string} envVar - Environment variable name
   * @returns {boolean} True if supported
   */
  isSupportedEnvVar(envVar) {
    return envVar in this.overrides;
  }

  /**
   * Get configuration path for environment variable
   * @param {string} envVar - Environment variable name
   * @returns {string|null} Configuration path or null if not found
   */
  getConfigPath(envVar) {
    return this.overrides[envVar] || null;
  }

  /**
   * Get current environment variable values
   * @returns {object} Object with current environment variable values
   */
  getCurrentEnvValues() {
    const currentValues = {};

    for (const envVar of Object.keys(this.overrides)) {
      const value = process.env[envVar];
      if (value !== undefined) {
        currentValues[envVar] = value;
      }
    }

    return currentValues;
  }

  /**
   * Validate environment variable values
   * @returns {object} Validation result with errors
   */
  validateEnvValues() {
    const errors = [];
    const currentValues = this.getCurrentEnvValues();

    for (const [envVar, value] of Object.entries(currentValues)) {
      try {
        this.parseEnvValue(envVar, value);
      } catch (error) {
        errors.push({
          envVar,
          value,
          error: error.message,
        });
      }
    }

    return {
      valid: errors.length === 0,
      errors,
    };
  }

  /**
   * Generate environment variable documentation
   * @returns {string} Documentation string
   */
  generateDocumentation() {
    let docs = "# Environment Variable Overrides\n\n";
    docs +=
      "The following environment variables can be used to override configuration settings:\n\n";

    const sections = {};

    // Group by section
    for (const [envVar, configPath] of Object.entries(this.overrides)) {
      const section = configPath.split(".")[0];
      if (!sections[section]) {
        sections[section] = [];
      }
      sections[section].push({ envVar, configPath });
    }

    // Generate documentation for each section
    for (const [section, vars] of Object.entries(sections)) {
      docs += `## ${section.toUpperCase()}\n\n`;

      for (const { envVar, configPath } of vars.sort((a, b) =>
        a.envVar.localeCompare(b.envVar),
      )) {
        docs += `- **${envVar}**: Overrides \`${configPath}\`\n`;
      }
      docs += "\n";
    }

    return docs;
  }
}

// Export singleton instance
export const environmentConfig = new EnvironmentConfig();

// Convenience functions for backward compatibility
export const applyEnvOverrides = (config) =>
  EnvironmentConfig.applyEnvOverrides(config);
export const getSupportedEnvVars = () =>
  environmentConfig.getSupportedEnvVars();
export const validateEnvValues = () => environmentConfig.validateEnvValues();

// Re-export envLoader utilities for unified import
export {
  getEnv,
  getRequiredEnv,
  resolveEnvVars,
  resolveEnvVarsInObject,
  getNodeEnv,
  isProduction,
  isDevelopment,
  getLogLevel,
  validateRequiredEnvVars,
} from "./envLoader.js";

export default EnvironmentConfig;
