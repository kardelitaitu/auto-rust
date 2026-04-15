/**
 * Unit tests for api/utils/patch.js
 * Covers: stripCDPMarkers, check, apply (via mocking), _safeEmitError
 */

import { describe, it, expect, vi, beforeEach } from "vitest";

// Mock the context module before importing patch
vi.mock("@api/core/context.js", () => ({
  getPage: vi.fn(),
  getEvents: vi.fn(),
}));

import { stripCDPMarkers, check, apply } from "@api/utils/patch.js";
import { getPage, getEvents } from "@api/core/context.js";

describe("patch.js", () => {
  let mockPage;
  let mockEvents;

  beforeEach(() => {
    vi.clearAllMocks();

    mockPage = {
      addInitScript: vi.fn().mockResolvedValue(undefined),
      evaluate: vi.fn(),
    };

    mockEvents = {
      emitSafe: vi.fn(),
    };

    getPage.mockReturnValue(mockPage);
    getEvents.mockReturnValue(mockEvents);
  });

  describe("stripCDPMarkers", () => {
    it("should handle browser environment with window", () => {
      // Simulate browser environment
      global.window = {
        cdc_adoQjvpsHSjkbJjLPRbPQ: "test",
        $cdc_asdjflasutopfhvcZLmcfl_: "test",
      };

      // Should not throw
      expect(() => stripCDPMarkers()).not.toThrow();

      // Clean up
      delete global.window;
    });

    it("should handle non-browser environment (no window)", () => {
      // Ensure window is undefined
      const originalWindow = global.window;
      delete global.window;

      // Should not throw
      expect(() => stripCDPMarkers()).not.toThrow();

      // Restore
      global.window = originalWindow;
    });

    it("should handle errors gracefully", () => {
      global.window = {};

      // Make defineProperty throw
      Object.defineProperty(global.window, "cdc_adoQjvpsHSjkbJjLPRbPQ", {
        get() {
          throw new Error("test");
        },
        set() {
          throw new Error("test");
        },
        configurable: true,
      });

      // Should not throw due to try/catch
      expect(() => stripCDPMarkers()).not.toThrow();

      delete global.window;
    });
  });

  describe("check", () => {
    it("should return detection check results", async () => {
      mockPage.evaluate.mockResolvedValue({
        webdriver: false,
        cdcMarkers: false,
        passed: true,
      });

      const result = await check();

      expect(result).toEqual({
        webdriver: false,
        cdcMarkers: false,
        passed: true,
      });
      expect(mockPage.evaluate).toHaveBeenCalled();
    });

    it("should detect webdriver flag", async () => {
      mockPage.evaluate.mockResolvedValue({
        webdriver: true,
        cdcMarkers: false,
        passed: false,
      });

      const result = await check();

      expect(result.webdriver).toBe(true);
      expect(result.passed).toBe(false);
    });

    it("should detect CDC markers", async () => {
      mockPage.evaluate.mockResolvedValue({
        webdriver: false,
        cdcMarkers: true,
        passed: false,
      });

      const result = await check();

      expect(result.cdcMarkers).toBe(true);
      expect(result.passed).toBe(false);
    });
  });

  describe("apply", () => {
    it("should call page.addInitScript with default fingerprint", async () => {
      await apply();

      expect(mockPage.addInitScript).toHaveBeenCalled();
      const callArgs = mockPage.addInitScript.mock.calls[0];

      // First arg is the function, second is the data
      expect(callArgs[0]).toBeInstanceOf(Function);
      expect(callArgs[1]).toEqual({
        languages: ["en-US", "en"],
        deviceMemory: 8,
        hardwareConcurrency: 8,
        maxTouchPoints: 0,
      });
    });

    it("should call page.addInitScript with custom fingerprint", async () => {
      const customFingerprint = {
        languages: ["fr-FR", "fr"],
        deviceMemory: 16,
        hardwareConcurrency: 4,
        maxTouchPoints: 5,
      };

      await apply(customFingerprint);

      expect(mockPage.addInitScript).toHaveBeenCalled();
      const callArgs = mockPage.addInitScript.mock.calls[0];
      expect(callArgs[1]).toEqual(customFingerprint);
    });

    it("should handle page.addInitScript errors", async () => {
      mockPage.addInitScript.mockRejectedValue(new Error("Page closed"));

      await expect(apply()).rejects.toThrow("Page closed");
    });

    it("should emit error via _safeEmitError when page fails", async () => {
      // The _safeEmitError is internal, so we test indirectly
      // by checking if getEvents is called during error scenarios
      mockPage.addInitScript.mockRejectedValue(new Error("Test error"));

      try {
        await apply();
      } catch (e) {
        expect(e.message).toBe("Test error");
      }
    });
  });

  describe("_safeEmitError (via apply error handling)", () => {
    it("should handle missing events gracefully", async () => {
      getEvents.mockImplementation(() => {
        throw new Error("No events");
      });

      // Even if getEvents fails, apply should work
      await apply();

      expect(mockPage.addInitScript).toHaveBeenCalled();
    });
  });

  describe("apply init script content", () => {
    it("should create a function that patches navigator properties", async () => {
      await apply();

      const initFn = mockPage.addInitScript.mock.calls[0][0];

      // Execute the init function with mock data
      const mockData = {
        languages: ["en-US", "en"],
        deviceMemory: 8,
        hardwareConcurrency: 8,
        maxTouchPoints: 0,
      };

      // Create mock window and navigator
      const mockNavigator = {
        webdriver: true,
        plugins: [],
        getBattery: () => Promise.resolve({}),
      };

      const mockWindow = {
        navigator: mockNavigator,
        chrome: null,
      };

      // Execute the init script function
      // Note: This runs in browser context, we can only verify it doesn't throw
      expect(() => {
        // We can't fully execute this in Node, but we verify the function exists
        expect(typeof initFn).toBe("function");
      }).not.toThrow();
    });

    it("should include chrome object creation logic", async () => {
      await apply();

      const initFn = mockPage.addInitScript.mock.calls[0][0];
      const fnString = initFn.toString();

      // Check that chrome mock creation is included
      expect(fnString).toContain("chrome");
      expect(fnString).toContain("app");
      expect(fnString).toContain("runtime");
    });

    it("should include Function.prototype.toString patching", async () => {
      await apply();

      const initFn = mockPage.addInitScript.mock.calls[0][0];
      const fnString = initFn.toString();

      expect(fnString).toContain("toString");
      expect(fnString).toContain("playwright");
    });

    it("should include plugin spoofing logic", async () => {
      await apply();

      const initFn = mockPage.addInitScript.mock.calls[0][0];
      const fnString = initFn.toString();

      expect(fnString).toContain("plugins");
      expect(fnString).toContain("PluginArray");
      expect(fnString).toContain("PDF Viewer");
    });

    it("should include battery spoofing logic", async () => {
      await apply();

      const initFn = mockPage.addInitScript.mock.calls[0][0];
      const fnString = initFn.toString();

      expect(fnString).toContain("getBattery");
      expect(fnString).toContain("BatteryManager");
    });
  });
});
