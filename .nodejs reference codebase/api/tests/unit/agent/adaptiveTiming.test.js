/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { adaptiveTiming } from "@api/agent/adaptiveTiming.js";

vi.mock("@api/core/logger.js", () => ({
  createLogger: () => ({
    info: vi.fn(),
    debug: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
  }),
}));

describe("api/agent/adaptiveTiming.js", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    adaptiveTiming.clearProfiles();
  });

  describe("adaptiveTiming object", () => {
    it("should be defined", () => {
      expect(adaptiveTiming).toBeDefined();
    });

    it("should have getAdjustedDelay method", () => {
      expect(typeof adaptiveTiming.getAdjustedDelay).toBe("function");
    });

    it("should have getTimingForSite method", () => {
      expect(typeof adaptiveTiming.getTimingForSite).toBe("function");
    });

    it("should have clearProfiles method", () => {
      expect(typeof adaptiveTiming.clearProfiles).toBe("function");
    });

    it("should have measureSitePerformance method", () => {
      expect(typeof adaptiveTiming.measureSitePerformance).toBe("function");
    });

    it("should have getStats method", () => {
      expect(typeof adaptiveTiming.getStats).toBe("function");
    });
  });

  describe("constructor", () => {
    it("should initialize with empty siteProfiles map", () => {
      expect(adaptiveTiming.siteProfiles).toBeInstanceOf(Map);
    });

    it("should have default timing values", () => {
      expect(adaptiveTiming.defaultTiming).toEqual({
        click: 100,
        type: 50,
        navigation: 2000,
        waitMultiplier: 1.0,
      });
    });
  });

  describe("measureSitePerformance", () => {
    it("should return timing profile when page has performance data", async () => {
      const mockPage = {
        evaluate: vi.fn().mockResolvedValue({
          domContentLoaded: 100,
          loadComplete: 500,
          responseTime: 150,
          domInteractive: 300,
        }),
        url: vi.fn().mockReturnValue("https://example.com"),
      };

      const profile = await adaptiveTiming.measureSitePerformance(mockPage);

      expect(profile).toHaveProperty("click");
      expect(profile).toHaveProperty("type");
      expect(profile).toHaveProperty("navigation");
      expect(profile).toHaveProperty("waitMultiplier");
      expect(profile).toHaveProperty("metrics");
      expect(mockPage.evaluate).toHaveBeenCalled();
    });

    it("should return default timing when performance data is null", async () => {
      const mockPage = {
        evaluate: vi.fn().mockResolvedValue(null),
        url: vi.fn().mockReturnValue("https://example.com"),
      };

      const profile = await adaptiveTiming.measureSitePerformance(mockPage);

      expect(profile).toEqual(adaptiveTiming.defaultTiming);
    });

    it("should return default timing when page.evaluate throws", async () => {
      const mockPage = {
        evaluate: vi.fn().mockRejectedValue(new Error("Evaluation failed")),
        url: vi.fn().mockReturnValue("https://example.com"),
      };

      const profile = await adaptiveTiming.measureSitePerformance(mockPage);

      expect(profile).toEqual(adaptiveTiming.defaultTiming);
    });

    it("should cache profile for the site URL", async () => {
      const mockPage = {
        evaluate: vi.fn().mockResolvedValue({
          domContentLoaded: 100,
          loadComplete: 500,
          responseTime: 150,
          domInteractive: 300,
        }),
        url: vi.fn().mockReturnValue("https://example.com"),
      };

      await adaptiveTiming.measureSitePerformance(mockPage);

      expect(adaptiveTiming.siteProfiles.has("https://example.com")).toBe(true);
    });
  });

  describe("_calculateTimingProfile", () => {
    it("should calculate timing based on metrics", () => {
      const metrics = {
        domContentLoaded: 100,
        loadComplete: 1000,
        responseTime: 200,
        domInteractive: 300,
      };

      const profile = adaptiveTiming._calculateTimingProfile(metrics);

      expect(profile.click).toBe(100);
      expect(profile.type).toBe(50);
      expect(profile.navigation).toBe(2000);
      expect(profile.waitMultiplier).toBe(1.0);
      expect(profile.metrics).toBe(metrics);
    });

    it("should increase timings for slow sites", () => {
      const metrics = {
        domContentLoaded: 200,
        loadComplete: 3000,
        responseTime: 600,
        domInteractive: 500,
      };

      const profile = adaptiveTiming._calculateTimingProfile(metrics);

      expect(profile.click).toBeGreaterThan(100);
      expect(profile.type).toBeGreaterThan(50);
      expect(profile.navigation).toBeGreaterThan(2000);
      expect(profile.waitMultiplier).toBeGreaterThan(1.0);
    });

    it("should decrease timings for fast sites", () => {
      const metrics = {
        domContentLoaded: 50,
        loadComplete: 200,
        responseTime: 50,
        domInteractive: 100,
      };

      const profile = adaptiveTiming._calculateTimingProfile(metrics);

      expect(profile.click).toBeLessThan(100);
      expect(profile.type).toBeLessThan(50);
      expect(profile.navigation).toBeLessThan(2000);
      expect(profile.waitMultiplier).toBeLessThan(1.0);
    });

    it("should cap load factor at 3.0", () => {
      const metrics = {
        loadComplete: 10000,
        responseTime: 200,
      };

      const profile = adaptiveTiming._calculateTimingProfile(metrics);

      // Load factor capped at 3.0, response factor at 1.0, combined = 3.0*0.7 + 1.0*0.3 = 2.4
      expect(profile.waitMultiplier).toBeLessThanOrEqual(2.5);
    });

    it("should cap response factor at 2.0", () => {
      const metrics = {
        loadComplete: 1000,
        responseTime: 5000,
      };

      const profile = adaptiveTiming._calculateTimingProfile(metrics);

      expect(profile.waitMultiplier).toBeLessThanOrEqual(1.3);
    });

    it("should handle missing loadComplete metric", () => {
      const metrics = {
        responseTime: 200,
      };

      const profile = adaptiveTiming._calculateTimingProfile(metrics);

      expect(profile).toHaveProperty("click");
      expect(profile).toHaveProperty("waitMultiplier");
    });

    it("should handle missing responseTime metric", () => {
      const metrics = {
        loadComplete: 1000,
      };

      const profile = adaptiveTiming._calculateTimingProfile(metrics);

      expect(profile).toHaveProperty("click");
      expect(profile).toHaveProperty("waitMultiplier");
    });
  });

  describe("getTimingForSite", () => {
    it("should return cached profile for exact URL match", () => {
      const testUrl = "https://example.com/page";
      const testProfile = {
        click: 150,
        type: 75,
        navigation: 3000,
        waitMultiplier: 1.5,
      };
      adaptiveTiming.siteProfiles.set(testUrl, testProfile);

      const result = adaptiveTiming.getTimingForSite(testUrl);

      expect(result).toBe(testProfile);
    });

    it("should return profile for domain match", () => {
      const cachedUrl = "https://example.com/page1";
      const testUrl = "https://example.com/page2";
      const testProfile = {
        click: 150,
        type: 75,
        navigation: 3000,
        waitMultiplier: 1.5,
      };
      adaptiveTiming.siteProfiles.set(cachedUrl, testProfile);

      const result = adaptiveTiming.getTimingForSite(testUrl);

      expect(result).toBe(testProfile);
    });

    it("should return default timing for unknown URL", () => {
      const result = adaptiveTiming.getTimingForSite("https://unknown.com");

      expect(result).toEqual(adaptiveTiming.defaultTiming);
    });

    it("should return default timing for invalid URL", () => {
      const result = adaptiveTiming.getTimingForSite("not-a-valid-url");

      expect(result).toEqual(adaptiveTiming.defaultTiming);
    });

    it("should handle cached invalid URLs gracefully", () => {
      adaptiveTiming.siteProfiles.set("invalid-url", { click: 100 });
      const result = adaptiveTiming.getTimingForSite("https://example.com");

      expect(result).toEqual(adaptiveTiming.defaultTiming);
    });
  });

  describe("getAdjustedDelay", () => {
    it("should return adjusted delay for click action", () => {
      const testUrl = "https://example.com";
      const testProfile = {
        click: 100,
        type: 50,
        navigation: 2000,
        waitMultiplier: 1.5,
      };
      adaptiveTiming.siteProfiles.set(testUrl, testProfile);

      const delay = adaptiveTiming.getAdjustedDelay(testUrl, "click", 100);

      expect(delay).toBe(150);
    });

    it("should return adjusted delay for type action", () => {
      const testUrl = "https://example.com";
      const testProfile = {
        click: 100,
        type: 50,
        navigation: 2000,
        waitMultiplier: 2.0,
      };
      adaptiveTiming.siteProfiles.set(testUrl, testProfile);

      const delay = adaptiveTiming.getAdjustedDelay(testUrl, "type", 50);

      expect(delay).toBe(100);
    });

    it("should return adjusted delay for navigation action", () => {
      const testUrl = "https://example.com";
      const testProfile = {
        click: 100,
        type: 50,
        navigation: 2000,
        waitMultiplier: 1.0,
      };
      adaptiveTiming.siteProfiles.set(testUrl, testProfile);

      const delay = adaptiveTiming.getAdjustedDelay(
        testUrl,
        "navigation",
        2000,
      );

      expect(delay).toBe(2000);
    });

    it("should return baseDelay for unknown action type", () => {
      const testUrl = "https://example.com";
      const testProfile = {
        click: 100,
        type: 50,
        navigation: 2000,
        waitMultiplier: 1.0,
      };
      adaptiveTiming.siteProfiles.set(testUrl, testProfile);

      const delay = adaptiveTiming.getAdjustedDelay(testUrl, "unknown", 300);

      expect(delay).toBe(300);
    });

    it("should use default timing for unknown site", () => {
      const delay = adaptiveTiming.getAdjustedDelay(
        "https://unknown.com",
        "click",
        100,
      );

      expect(delay).toBe(100);
    });
  });

  describe("clearProfiles", () => {
    it("should clear all cached profiles", () => {
      adaptiveTiming.siteProfiles.set("https://example.com", { click: 100 });
      adaptiveTiming.siteProfiles.set("https://test.com", { click: 200 });

      expect(adaptiveTiming.siteProfiles.size).toBe(2);

      adaptiveTiming.clearProfiles();

      expect(adaptiveTiming.siteProfiles.size).toBe(0);
    });
  });

  describe("getStats", () => {
    it("should return correct statistics", () => {
      adaptiveTiming.siteProfiles.set("https://example.com", { click: 100 });
      adaptiveTiming.siteProfiles.set("https://test.com", { click: 200 });

      const stats = adaptiveTiming.getStats();

      expect(stats.cachedSites).toBe(2);
      expect(stats.defaultTiming).toEqual(adaptiveTiming.defaultTiming);
    });

    it("should return zero cached sites when empty", () => {
      const stats = adaptiveTiming.getStats();

      expect(stats.cachedSites).toBe(0);
    });
  });
});
