/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Adaptive Timing System
 * Measures site performance and adjusts timing profiles accordingly
 * @module api/agent/adaptiveTiming
 */

import { createLogger } from "../core/logger.js";

const logger = createLogger("api/agent/adaptiveTiming.js");

class AdaptiveTiming {
  constructor() {
    this.siteProfiles = new Map();
    this.maxProfiles = 200;
    this.defaultTiming = {
      click: 100,
      type: 50,
      navigation: 2000,
      waitMultiplier: 1.0,
    };
  }

  /**
   * Measure site performance and create timing profile
   * @param {object} page - Playwright page
   * @returns {Promise<object>} Timing profile for the site
   */
  async measureSitePerformance(page) {
    try {
      const metrics = await page.evaluate(() => {
        const perf = performance.getEntriesByType("navigation")[0];
        if (!perf) return null;

        return {
          domContentLoaded:
            perf.domContentLoadedEventEnd - perf.domContentLoadedEventStart,
          loadComplete: perf.loadEventEnd - perf.loadEventStart,
          responseTime: perf.responseEnd - perf.responseStart,
          domInteractive: perf.domInteractive - perf.domLoading,
        };
      });

      if (!metrics) {
        logger.debug("Could not measure site performance, using defaults");
        return this.defaultTiming;
      }

      const profile = this._calculateTimingProfile(metrics);
      const url = page.url();

      if (this.siteProfiles.size >= this.maxProfiles) {
        const oldestKey = this.siteProfiles.keys().next().value;
        this.siteProfiles.delete(oldestKey);
      }
      this.siteProfiles.set(url, profile);
      logger.info(
        `[AdaptiveTiming] Created profile for ${url}: click=${profile.click}ms, wait=${profile.waitMultiplier.toFixed(2)}x`,
      );

      return profile;
    } catch (error) {
      logger.warn("Failed to measure site performance:", error.message);
      return this.defaultTiming;
    }
  }

  /**
   * Calculate timing profile based on performance metrics
   * @private
   */
  _calculateTimingProfile(metrics) {
    const _base = 1.0;

    // Calculate load factor (1.0 = normal, >1.0 = slow, <1.0 = fast)
    const loadFactor = Math.min((metrics.loadComplete || 1000) / 1000, 3.0);
    const responseFactor = Math.min((metrics.responseTime || 200) / 200, 2.0);

    // Combined factor (weighted average)
    const combinedFactor = loadFactor * 0.7 + responseFactor * 0.3;

    return {
      click: Math.round(100 * combinedFactor),
      type: Math.round(50 * combinedFactor),
      navigation: Math.round(2000 * combinedFactor),
      waitMultiplier: combinedFactor,
      metrics: metrics,
    };
  }

  /**
   * Get timing profile for a URL
   * @param {string} url - Page URL
   * @returns {object} Timing profile
   */
  getTimingForSite(url) {
    // Try exact match first
    if (this.siteProfiles.has(url)) {
      return this.siteProfiles.get(url);
    }

    // Try domain match
    try {
      const urlObj = new URL(url);
      const domain = urlObj.hostname;

      for (const [cachedUrl, profile] of this.siteProfiles.entries()) {
        try {
          const cachedUrlObj = new URL(cachedUrl);
          if (cachedUrlObj.hostname === domain) {
            return profile;
          }
        } catch {
          // Skip invalid URLs
        }
      }
    } catch {
      // Skip invalid URL
    }

    return this.defaultTiming;
  }

  /**
   * Get adjusted delay based on site performance
   * @param {string} url - Page URL
   * @param {string} actionType - Type of action (click, type, navigation)
   * @param {number} baseDelay - Base delay in ms
   * @returns {number} Adjusted delay in ms
   */
  getAdjustedDelay(url, actionType, baseDelay) {
    const profile = this.getTimingForSite(url);
    const actionDelay = profile[actionType] || baseDelay;
    return Math.round(actionDelay * profile.waitMultiplier);
  }

  /**
   * Clear cached profiles
   */
  clearProfiles() {
    this.siteProfiles.clear();
    logger.info("[AdaptiveTiming] Cleared all cached profiles");
  }

  /**
   * Get statistics about cached profiles
   * @returns {object} Statistics
   */
  getStats() {
    return {
      cachedSites: this.siteProfiles.size,
      defaultTiming: this.defaultTiming,
    };
  }
}

const adaptiveTiming = new AdaptiveTiming();

export { adaptiveTiming };
export default adaptiveTiming;
