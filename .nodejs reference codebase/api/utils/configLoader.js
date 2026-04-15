/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Manages loading and caching of configuration files.
 *
 * This is the core configuration loader that handles:
 * - Loading JSON config files from disk
 * - Caching to avoid repeated file reads
 * - File watching for development (hot reload)
 * - Default values fallback
 *
 * Used by: core modules (orchestrator, sessionManager, automator, cloud-client, etc.)
 *
 * @module utils/configLoader
 */

import fs from "fs/promises";
import path from "path";
import { fileURLToPath } from "url";
import { createLogger } from "../core/logger.js";

const logger = createLogger("configLoader.js");
const __dirname = path.dirname(fileURLToPath(import.meta.url));

/**
 * @class ConfigLoader
 * @description Loads, caches, and provides access to configuration files.
 */
export class ConfigLoader {
  constructor() {
    this.configDir = path.join(__dirname, "../../config");
    this.cache = new Map();
    this.watchers = new Map();
  }

  /**
   * Loads a configuration file from disk, using a cache to avoid repeated reads.
   * @param {string} filename - The name of the configuration file (without the .json extension).
   * @param {object} [defaults={}] - Default values to use if the file doesn't exist.
   * @returns {Promise<object>} A promise that resolves with the configuration object.
   */
  async loadConfig(filename, defaults = {}) {
    const cacheKey = filename;

    if (this.cache.has(cacheKey)) {
      return this.cache.get(cacheKey);
    }

    try {
      const configPath = path.join(this.configDir, `${filename}.json`);
      const data = await fs.readFile(configPath, "utf8");
      const parsedData = JSON.parse(data);

      if (parsedData === null) {
        this.cache.set(cacheKey, null);
        return null;
      }

      const config = { ...defaults, ...parsedData };

      this.cache.set(cacheKey, config);
      logger.debug(`Loaded configuration from ${filename}.json`);

      return config;
    } catch (error) {
      if (error.code === "ENOENT") {
        logger.warn(
          `Configuration file ${filename}.json not found, using defaults`,
        );
      } else {
        logger.error(`Failed to load ${filename}.json:`, error.message);
      }

      this.cache.set(cacheKey, defaults);
      return defaults;
    }
  }

  /**
   * Gets a specific configuration value using dot notation (e.g., 'session.timeoutMs').
   * @param {string} configPath - The path to the configuration value.
   * @param {any} [defaultValue=null] - The default value to return if the path doesn't exist.
   * @returns {Promise<any>} A promise that resolves with the configuration value.
   */
  async getValue(configPath, defaultValue = null) {
    const [filename, ...pathParts] = configPath.split(".");
    const config = await this.loadConfig(filename);

    let current = config;
    for (const part of pathParts) {
      if (current && typeof current === "object" && part in current) {
        current = current[part];
      } else {
        return defaultValue;
      }
    }

    return current;
  }

  /**
   * Reloads a configuration file from disk, bypassing the cache.
   * @param {string} filename - The name of the configuration file to reload.
   * @returns {Promise<object>} A promise that resolves with the reloaded configuration object.
   */
  async reloadConfig(filename) {
    this.cache.delete(filename);
    return this.loadConfig(filename);
  }

  /**
   * Clears the entire configuration cache.
   */
  clearCache() {
    this.cache.clear();
    logger.debug("Configuration cache cleared");
  }

  /**
   * Validates a configuration object against a schema.
   * @param {object} config - The configuration object to validate.
   * @param {object} schema - The validation schema.
   * @returns {{isValid: boolean, errors: string[]}} An object indicating whether the configuration is valid, and an array of any validation errors.
   */
  validateConfig(config, schema) {
    const errors = [];

    for (const [key, rules] of Object.entries(schema)) {
      if (rules.required && !(key in config)) {
        errors.push(`Required configuration key '${key}' is missing`);
        continue;
      }

      if (key in config) {
        const value = config[key];

        if (rules.type && typeof value !== rules.type) {
          errors.push(
            `Configuration key '${key}' must be of type ${rules.type}`,
          );
        }

        if (rules.type === "number") {
          if (rules.min !== undefined && value < rules.min) {
            errors.push(
              `Configuration key '${key}' must be at least ${rules.min}`,
            );
          }
          if (rules.max !== undefined && value > rules.max) {
            errors.push(
              `Configuration key '${key}' must be at most ${rules.max}`,
            );
          }
        }
      }
    }

    return {
      isValid: errors.length === 0,
      errors,
    };
  }

  /**
   * Gets the full settings configuration.
   * @returns {Promise<object>} A promise that resolves with the settings configuration object.
   */
  async getSettings() {
    return this.loadConfig("settings", {});
  }
}

const configLoader = new ConfigLoader();

/**
 * Gets the timeouts configuration.
 * @returns {Promise<object>} A promise that resolves with the timeouts configuration object.
 */
export async function getTimeouts() {
  return configLoader.loadConfig("timeouts", {
    session: { timeoutMs: 1800000, cleanupIntervalMs: 300000 },
    task: { minDelayMs: 1000, maxDelayMs: 5000, finalDelayMs: 1000 },
    automation: {
      zoom: { minWaitMs: 2000, maxWaitMs: 5000 },
      scrolling: {
        minReadDelayMs: 1000,
        maxReadDelayMs: 3500,
        minPauseMs: 500,
        maxPauseMs: 1500,
      },
    },
    api: { retryDelayMs: 2000, maxRetries: 3, pollingIntervalMs: 250 },
    browser: { connectionTimeoutMs: 10000 },
  });
}

/**
 * Gets a specific timeout value using dot notation.
 * @param {string} path - The path to the timeout value (e.g., 'session.timeoutMs').
 * @param {any} [defaultValue=null] - The default value to return if the path doesn't exist.
 * @returns {Promise<any>} A promise that resolves with the timeout value.
 */
export async function getTimeoutValue(path, defaultValue = null) {
  return configLoader.getValue(`timeouts.${path}`, defaultValue);
}

/**
 * Gets the browser API configuration.
 * @returns {Promise<object>} A promise that resolves with the browser API configuration object.
 */
export async function getBrowserAPI() {
  return configLoader.loadConfig("browserAPI", {
    default: "roxybrowser",
    profiles: {},
  });
}

/**
 * Gets the settings configuration.
 * @returns {Promise<object>} A promise that resolves with the settings configuration object.
 */
export async function getSettings() {
  return configLoader.loadConfig("settings", {});
}

export default configLoader;
