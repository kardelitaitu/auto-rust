/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Task Configuration Loader
 * Centralizes configuration loading with caching, validation, and environment variable support
 * @module utils/task-config-loader
 */

import { config } from "./config-service.js";
import { getSettings } from "./configLoader.js";
import { ConfigValidator } from "./config-validator.js";
import { ConfigCache } from "./config-cache.js";
import { EnvironmentConfig } from "./environment-config.js";
import { createLogger } from "../core/logger.js";

const logger = createLogger("task-config-loader.js");

const clamp = (val, min, max) => Math.max(min, Math.min(max, val));

/**
 * Task Configuration Loader
 * Provides centralized configuration loading with caching, validation, and environment overrides
 */
export class TaskConfigLoader {
  constructor() {
    this.validator = new ConfigValidator();
    this.cache = new ConfigCache();
    this.envConfig = new EnvironmentConfig();
    this.loadCount = 0;
    this.hitCount = 0;
  }

  /**
   * Load AI Twitter Activity configuration with caching and validation
   * @param {object} payload - Task payload with optional overrides
   * @returns {Promise<object>} Unified configuration object
   */
  async loadAiTwitterActivityConfig(payload = {}) {
    const startTime = Date.now();
    const cacheKey = this.generateCacheKey(payload);

    // Check cache first
    const cached = this.cache.get(cacheKey);
    if (cached) {
      this.hitCount++;
      logger.debug(`[ConfigLoader] Cache hit for key: ${cacheKey}`);
      return cached;
    }

    this.loadCount++;
    logger.info(
      `[ConfigLoader] Loading configuration (cache miss: ${cacheKey})`,
    );

    try {
      // Load all configuration in parallel for optimal performance
      const [
        settings,
        activityConfig,
        timingConfig,
        engagementLimits,
        humanizationConfig,
      ] = await Promise.all([
        this.loadSettings(),
        config.getTwitterActivity(),
        config.getTiming(),
        config.getEngagementLimits(),
        config.getHumanization(),
      ]);

      // Build unified configuration object
      const builtConfig = this.buildConfig({
        settings,
        activityConfig,
        timingConfig,
        engagementLimits,
        humanizationConfig,
        payload,
      });

      // Apply environment variable overrides
      const envConfig = EnvironmentConfig.applyEnvOverrides(builtConfig);

      // Validate configuration
      const validationResult = this.validator.validateConfig(envConfig);
      if (!validationResult.valid) {
        const error = new Error(
          `Configuration validation failed: ${validationResult.errors.join(", ")}`,
        );
        logger.error(`[ConfigLoader] ${error.message}`);
        throw error;
      }

      // Cache the validated configuration
      this.cache.set(cacheKey, envConfig);

      const loadTime = Date.now() - startTime;
      logger.info(
        `[ConfigLoader] Configuration loaded successfully in ${loadTime}ms`,
      );
      logger.debug(
        `[ConfigLoader] Cache stats: ${this.hitCount} hits, ${this.loadCount} loads`,
      );

      return envConfig;
    } catch (error) {
      logger.error(
        `[ConfigLoader] Failed to load configuration: ${error.message}`,
      );
      throw new Error(`Configuration loading failed: ${error.message}`, {
        cause: error,
      });
    }
  }

  /**
   * Load settings with error handling
   * @private
   */
  async loadSettings() {
    try {
      return await getSettings();
    } catch (error) {
      logger.warn(
        `[ConfigLoader] Failed to load settings, using defaults: ${error.message}`,
      );
      return this.getDefaultSettings();
    }
  }

  /**
   * Build unified configuration object
   * @private
   */
  buildConfig({
    settings,
    activityConfig,
    timingConfig,
    engagementLimits,
    humanizationConfig,
    payload,
  }) {
    return {
      // Session configuration
      session: {
        cycles: clamp(payload.cycles ?? activityConfig.defaultCycles, 1, 100),
        minDuration: clamp(
          payload.minDuration ?? activityConfig.defaultMinDuration,
          60,
          3600,
        ),
        maxDuration: clamp(
          payload.maxDuration ?? activityConfig.defaultMaxDuration,
          120,
          7200,
        ),
        timeout:
          payload.taskTimeoutMs ??
          (activityConfig.defaultMinDuration +
            activityConfig.defaultMaxDuration) *
            1000,
      },

      // Engagement configuration
      engagement: {
        limits: engagementLimits,
        probabilities: {
          reply: settings.twitter?.reply?.probability ?? 0.5,
          quote: settings.twitter?.quote?.probability ?? 0.2,
          like: settings.twitter?.actions?.like?.probability ?? 0.15,
          bookmark: settings.twitter?.actions?.bookmark?.probability ?? 0.05,
          retweet: settings.twitter?.actions?.retweet?.probability ?? 0.2,
          follow: settings.twitter?.actions?.follow?.probability ?? 0.1,
        },
      },

      // Timing configuration
      timing: {
        warmup: {
          min: timingConfig.warmupMin ?? 2000,
          max: timingConfig.warmupMax ?? 15000,
        },
        scroll: {
          min: timingConfig.scrollMin ?? 300,
          max: timingConfig.scrollMax ?? 700,
        },
        read: {
          min: timingConfig.readMin ?? 5000,
          max: timingConfig.readMax ?? 15000,
        },
        diveRead: timingConfig.diveRead ?? 10000,
        globalScrollMultiplier: timingConfig.globalScrollMultiplier ?? 1.0,
      },

      // Humanization configuration
      humanization: {
        mouse: humanizationConfig.mouse ?? { speed: 1.0, jitter: 5 },
        typing: humanizationConfig.typing ?? { delay: 80, errorRate: 0.05 },
        session: humanizationConfig.session ?? { minMinutes: 5, maxMinutes: 9 },
      },

      // AI configuration
      ai: {
        enabled: settings.llm?.cloud?.enabled !== false,
        localEnabled: settings.llm?.local?.enabled === true,
        visionEnabled: settings.vision?.enabled ?? true,
        timeout: settings.llm?.cloud?.timeout ?? 120000,
      },

      // Browser configuration
      browser: {
        theme: payload.theme ?? "dark",
        referrer: settings.referrer ?? { addUTM: true },
        headers: {
          secFetchSite: "none",
          secFetchMode: "navigate",
        },
      },

      // Monitoring configuration
      monitoring: {
        queueMonitor: {
          enabled: settings.logging?.queueMonitor?.enabled ?? true,
          interval: settings.logging?.queueMonitor?.interval ?? 30000,
        },
        engagementProgress: {
          enabled: settings.logging?.engagementProgress?.enabled ?? true,
          showProgressBar:
            settings.logging?.engagementProgress?.showProgressBar ?? true,
        },
      },

      // System configuration
      system: {
        debugMode: process.env.DEBUG_MODE === "true" || payload.debug === true,
        performanceTracking: process.env.PERFORMANCE_TRACKING !== "false",
        errorRecovery: {
          maxRetries: 3,
          retryDelay: 5000,
          fallbackStrategies: ["reducedEngagement", "textOnly", "skipAI"],
        },
      },
    };
  }

  /**
   * Generate cache key from payload
   * @private
   */
  generateCacheKey(payload) {
    const keyData = {
      cycles: payload.cycles,
      minDuration: payload.minDuration,
      maxDuration: payload.maxDuration,
      theme: payload.theme,
      debug: payload.debug,
      DEBUG_MODE: process.env.DEBUG_MODE, // Include env var
    };
    return JSON.stringify(keyData);
  }

  /**
   * Get default settings when loading fails
   * @private
   */
  getDefaultSettings() {
    return {
      twitter: {
        reply: { probability: 0.5 },
        quote: { probability: 0.2 },
        actions: {
          like: { probability: 0.15 },
          bookmark: { probability: 0.05 },
        },
      },
      llm: {
        cloud: { enabled: true },
        local: { enabled: false },
      },
      vision: { enabled: true },
      logging: {
        queueMonitor: { enabled: true, interval: 30000 },
        engagementProgress: { enabled: true, showProgressBar: true },
      },
      system: {},
    };
  }

  /**
   * Get configuration loading statistics
   */
  getStats() {
    const stats = this.cache.getStats();
    return {
      cache: {
        hitCount: this.hitCount,
        loadCount: this.loadCount,
        hitRate:
          this.loadCount > 0
            ? (
                (this.hitCount / (this.hitCount + this.loadCount)) *
                100
              ).toFixed(2) + "%"
            : "0%",
      },
      cacheSize: stats.size,
      cacheMaxSize: stats.maxSize,
    };
  }

  /**
   * Clear configuration cache
   */
  clearCache() {
    this.cache.clear();
    this.hitCount = 0;
    this.loadCount = 0;
    logger.info("[ConfigLoader] Cache cleared");
  }
}

// Export singleton instance
export const taskConfigLoader = new TaskConfigLoader();

// Convenience function for backward compatibility
export const loadAiTwitterActivityConfig = (payload) =>
  taskConfigLoader.loadAiTwitterActivityConfig(payload);

export default TaskConfigLoader;
