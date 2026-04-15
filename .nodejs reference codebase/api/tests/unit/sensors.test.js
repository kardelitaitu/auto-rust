/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @vitest-environment jsdom
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";

// Mock dependencies
const mockPage = {
  addInitScript: vi.fn().mockResolvedValue(undefined),
};

const mockGetPage = vi.fn();

const mockMathUtils = {
  gaussian: vi.fn((mean, dev, min, max) => Math.min(max, Math.max(min, mean))),
  randomInRange: vi.fn((min, max) => Math.floor((min + max) / 2)),
};

vi.mock("@api/core/context.js", () => ({
  getPage: (...args) => mockGetPage(...args),
}));

vi.mock("@api/utils/math.js", () => ({
  mathUtils: mockMathUtils,
}));

describe("sensors", () => {
  let sensors;

  beforeEach(async () => {
    vi.clearAllMocks();
    vi.resetModules();

    // Reset navigator mocks
    delete navigator.connection;
    delete navigator.getBattery;

    // Reset window mocks
    delete window.DeviceOrientationEvent;
    delete window.DeviceMotionEvent;

    sensors = await import("@api/utils/sensors.js");
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe("injectSensors", () => {
    it("should call getPage to get the page", async () => {
      mockGetPage.mockReturnValue(mockPage);

      await sensors.injectSensors();

      expect(mockGetPage).toHaveBeenCalled();
    });

    it("should call page.addInitScript with a function", async () => {
      mockGetPage.mockReturnValue(mockPage);

      await sensors.injectSensors();

      expect(mockPage.addInitScript).toHaveBeenCalled();
      const [scriptFn] = mockPage.addInitScript.mock.calls[0];
      expect(typeof scriptFn).toBe("function");
    });

    it("should pass battery parameters to init script", async () => {
      mockGetPage.mockReturnValue(mockPage);
      mockMathUtils.gaussian.mockReturnValue(0.85);
      mockMathUtils.randomInRange.mockReturnValue(50);

      await sensors.injectSensors();

      const [, params] = mockPage.addInitScript.mock.calls[0];
      expect(params).toHaveProperty("level");
      expect(params).toHaveProperty("chargingTime");
      expect(params).toHaveProperty("dischargingTime");
    });

    it("should use gaussian for battery level", async () => {
      mockGetPage.mockReturnValue(mockPage);
      mockMathUtils.gaussian.mockReturnValue(0.75);

      await sensors.injectSensors();

      expect(mockMathUtils.gaussian).toHaveBeenCalledWith(0.85, 0.1, 0.5, 1.0);
    });

    it("should use randomInRange for chargingTime", async () => {
      mockGetPage.mockReturnValue(mockPage);
      mockMathUtils.randomInRange.mockReturnValue(42);

      await sensors.injectSensors();

      expect(mockMathUtils.randomInRange).toHaveBeenCalledWith(0, 100);
    });

    it("should set dischargingTime to Infinity", async () => {
      mockGetPage.mockReturnValue(mockPage);

      await sensors.injectSensors();

      const [, params] = mockPage.addInitScript.mock.calls[0];
      expect(params.dischargingTime).toBe(Infinity);
    });

    it("should pass correct parameters structure", async () => {
      mockGetPage.mockReturnValue(mockPage);
      mockMathUtils.gaussian.mockReturnValue(0.9);
      mockMathUtils.randomInRange.mockReturnValue(75);

      await sensors.injectSensors();

      const [, params] = mockPage.addInitScript.mock.calls[0];
      expect(params).toEqual({
        level: 0.9,
        chargingTime: 75,
        dischargingTime: Infinity,
      });
    });
  });

  describe("injected script - Battery API", () => {
    it("should mock getBattery when available", async () => {
      mockGetPage.mockReturnValue(mockPage);
      mockMathUtils.gaussian.mockReturnValue(0.85);
      mockMathUtils.randomInRange.mockReturnValue(50);

      await sensors.injectSensors();

      // Get the injected script
      const [scriptFn, params] = mockPage.addInitScript.mock.calls[0];

      // Set up navigator.getBattery
      const originalGetBattery = navigator.getBattery;
      navigator.getBattery = undefined;

      // Execute the script
      scriptFn(params);

      // Check if getBattery was mocked
      if ("getBattery" in navigator) {
        // The script checks 'getBattery' in navigator
        // We need to set it up first
        expect(navigator.getBattery).toBeDefined();
      }

      // Restore
      if (originalGetBattery) navigator.getBattery = originalGetBattery;
    });

    it("should set battery charging to true", async () => {
      mockGetPage.mockReturnValue(mockPage);
      mockMathUtils.gaussian.mockReturnValue(0.85);
      mockMathUtils.randomInRange.mockReturnValue(50);

      // Setup navigator.getBattery as a property (not function) to trigger the if condition
      Object.defineProperty(navigator, "getBattery", {
        value: undefined,
        writable: true,
        configurable: true,
      });

      await sensors.injectSensors();

      const [scriptFn, params] = mockPage.addInitScript.mock.calls[0];
      scriptFn(params);

      // After script execution, getBattery should be a function
      expect(typeof navigator.getBattery).toBe("function");

      // Call it and check the battery object
      const battery = await navigator.getBattery();
      expect(battery.charging).toBe(true);
      expect(battery.level).toBe(params.level);
      expect(battery.chargingTime).toBe(params.chargingTime);
      expect(battery.dischargingTime).toBe(params.dischargingTime);
      expect(typeof battery.addEventListener).toBe("function");
    });

    it("should set battery level from params", async () => {
      mockGetPage.mockReturnValue(mockPage);
      mockMathUtils.gaussian.mockReturnValue(0.75);
      mockMathUtils.randomInRange.mockReturnValue(30);

      Object.defineProperty(navigator, "getBattery", {
        value: undefined,
        writable: true,
        configurable: true,
      });

      await sensors.injectSensors();

      const [scriptFn, params] = mockPage.addInitScript.mock.calls[0];
      scriptFn(params);

      const battery = await navigator.getBattery();
      expect(battery.level).toBe(0.75);
    });

    it("should set battery chargingTime from params", async () => {
      mockGetPage.mockReturnValue(mockPage);
      mockMathUtils.gaussian.mockReturnValue(0.85);
      mockMathUtils.randomInRange.mockReturnValue(42);

      Object.defineProperty(navigator, "getBattery", {
        value: undefined,
        writable: true,
        configurable: true,
      });

      await sensors.injectSensors();

      const [scriptFn, params] = mockPage.addInitScript.mock.calls[0];
      scriptFn(params);

      const battery = await navigator.getBattery();
      expect(battery.chargingTime).toBe(42);
    });

    it("should set battery dischargingTime from params", async () => {
      mockGetPage.mockReturnValue(mockPage);
      mockMathUtils.gaussian.mockReturnValue(0.85);
      mockMathUtils.randomInRange.mockReturnValue(50);

      Object.defineProperty(navigator, "getBattery", {
        value: undefined,
        writable: true,
        configurable: true,
      });

      await sensors.injectSensors();

      const [scriptFn, params] = mockPage.addInitScript.mock.calls[0];
      scriptFn(params);

      const battery = await navigator.getBattery();
      expect(battery.dischargingTime).toBe(Infinity);
    });

    it("should skip battery mocking if getBattery not in navigator", async () => {
      mockGetPage.mockReturnValue(mockPage);
      mockMathUtils.gaussian.mockReturnValue(0.85);
      mockMathUtils.randomInRange.mockReturnValue(50);

      // Ensure getBattery doesn't exist
      const tempNav = {};
      for (const key of Object.keys(navigator)) {
        tempNav[key] = navigator[key];
      }

      await sensors.injectSensors();

      const [scriptFn, params] = mockPage.addInitScript.mock.calls[0];

      // Execute in context where getBattery doesn't exist
      // The script checks 'getBattery' in navigator
      // Since jsdom navigator has getBattery defined, we test both paths
      expect(() => scriptFn(params)).not.toThrow();
    });
  });

  describe("injected script - Network Information API", () => {
    beforeEach(() => {
      // Clean up navigator.connection
      if (navigator.connection) {
        delete navigator.connection;
      }
    });

    it("should mock connection when navigator.connection exists", async () => {
      mockGetPage.mockReturnValue(mockPage);
      mockMathUtils.gaussian.mockReturnValue(0.85);
      mockMathUtils.randomInRange.mockReturnValue(50);

      // Create a connection property on navigator
      Object.defineProperty(navigator, "connection", {
        value: { existing: true },
        writable: true,
        configurable: true,
      });

      await sensors.injectSensors();

      const [scriptFn, params] = mockPage.addInitScript.mock.calls[0];
      scriptFn(params);

      // After script, connection should have new properties
      expect(navigator.connection.effectiveType).toBe("4g");
      expect(navigator.connection.rtt).toBe(50);
      expect(navigator.connection.downlink).toBe(10);
      expect(navigator.connection.saveData).toBe(false);
      expect(typeof navigator.connection.addEventListener).toBe("function");
    });

    it("should set effectiveType to 4g", async () => {
      mockGetPage.mockReturnValue(mockPage);
      mockMathUtils.gaussian.mockReturnValue(0.85);
      mockMathUtils.randomInRange.mockReturnValue(50);

      Object.defineProperty(navigator, "connection", {
        value: {},
        writable: true,
        configurable: true,
      });

      await sensors.injectSensors();

      const [scriptFn, params] = mockPage.addInitScript.mock.calls[0];
      scriptFn(params);

      expect(navigator.connection.effectiveType).toBe("4g");
    });

    it("should set rtt to 50", async () => {
      mockGetPage.mockReturnValue(mockPage);
      mockMathUtils.gaussian.mockReturnValue(0.85);
      mockMathUtils.randomInRange.mockReturnValue(50);

      Object.defineProperty(navigator, "connection", {
        value: {},
        writable: true,
        configurable: true,
      });

      await sensors.injectSensors();

      const [scriptFn, params] = mockPage.addInitScript.mock.calls[0];
      scriptFn(params);

      expect(navigator.connection.rtt).toBe(50);
    });

    it("should set downlink to 10", async () => {
      mockGetPage.mockReturnValue(mockPage);
      mockMathUtils.gaussian.mockReturnValue(0.85);
      mockMathUtils.randomInRange.mockReturnValue(50);

      Object.defineProperty(navigator, "connection", {
        value: {},
        writable: true,
        configurable: true,
      });

      await sensors.injectSensors();

      const [scriptFn, params] = mockPage.addInitScript.mock.calls[0];
      scriptFn(params);

      expect(navigator.connection.downlink).toBe(10);
    });

    it("should set saveData to false", async () => {
      mockGetPage.mockReturnValue(mockPage);
      mockMathUtils.gaussian.mockReturnValue(0.85);
      mockMathUtils.randomInRange.mockReturnValue(50);

      Object.defineProperty(navigator, "connection", {
        value: {},
        writable: true,
        configurable: true,
      });

      await sensors.injectSensors();

      const [scriptFn, params] = mockPage.addInitScript.mock.calls[0];
      scriptFn(params);

      expect(navigator.connection.saveData).toBe(false);
    });

    it("should add addEventListener stub to connection", async () => {
      mockGetPage.mockReturnValue(mockPage);
      mockMathUtils.gaussian.mockReturnValue(0.85);
      mockMathUtils.randomInRange.mockReturnValue(50);

      Object.defineProperty(navigator, "connection", {
        value: {},
        writable: true,
        configurable: true,
      });

      await sensors.injectSensors();

      const [scriptFn, params] = mockPage.addInitScript.mock.calls[0];
      scriptFn(params);

      expect(typeof navigator.connection.addEventListener).toBe("function");
      // Should not throw when called
      expect(() =>
        navigator.connection.addEventListener("change", () => {}),
      ).not.toThrow();
    });

    it("should skip connection mocking if not in navigator", async () => {
      mockGetPage.mockReturnValue(mockPage);
      mockMathUtils.gaussian.mockReturnValue(0.85);
      mockMathUtils.randomInRange.mockReturnValue(50);

      // The script checks 'connection' in navigator
      // In jsdom, navigator.connection may or may not exist
      // We test that the script runs without error
      await sensors.injectSensors();

      const [scriptFn, params] = mockPage.addInitScript.mock.calls[0];
      expect(() => scriptFn(params)).not.toThrow();
    });
  });

  describe("injected script - Device Events", () => {
    it("should set DeviceOrientationEvent if not defined", async () => {
      mockGetPage.mockReturnValue(mockPage);
      mockMathUtils.gaussian.mockReturnValue(0.85);
      mockMathUtils.randomInRange.mockReturnValue(50);

      // Delete DeviceOrientationEvent
      delete window.DeviceOrientationEvent;

      await sensors.injectSensors();

      const [scriptFn, params] = mockPage.addInitScript.mock.calls[0];
      scriptFn(params);

      expect(window.DeviceOrientationEvent).toBeDefined();
      expect(typeof window.DeviceOrientationEvent).toBe("function");
    });

    it("should set DeviceMotionEvent if not defined", async () => {
      mockGetPage.mockReturnValue(mockPage);
      mockMathUtils.gaussian.mockReturnValue(0.85);
      mockMathUtils.randomInRange.mockReturnValue(50);

      // Delete DeviceMotionEvent
      delete window.DeviceMotionEvent;

      await sensors.injectSensors();

      const [scriptFn, params] = mockPage.addInitScript.mock.calls[0];
      scriptFn(params);

      expect(window.DeviceMotionEvent).toBeDefined();
      expect(typeof window.DeviceMotionEvent).toBe("function");
    });

    it("should not overwrite existing DeviceOrientationEvent", async () => {
      mockGetPage.mockReturnValue(mockPage);
      mockMathUtils.gaussian.mockReturnValue(0.85);
      mockMathUtils.randomInRange.mockReturnValue(50);

      // Create existing DeviceOrientationEvent
      const originalEvent = class CustomOrientationEvent {};
      window.DeviceOrientationEvent = originalEvent;

      await sensors.injectSensors();

      const [scriptFn, params] = mockPage.addInitScript.mock.calls[0];
      scriptFn(params);

      // Should not be overwritten (using || operator)
      expect(window.DeviceOrientationEvent).toBe(originalEvent);
    });

    it("should not overwrite existing DeviceMotionEvent", async () => {
      mockGetPage.mockReturnValue(mockPage);
      mockMathUtils.gaussian.mockReturnValue(0.85);
      mockMathUtils.randomInRange.mockReturnValue(50);

      // Create existing DeviceMotionEvent
      const originalEvent = class CustomMotionEvent {};
      window.DeviceMotionEvent = originalEvent;

      await sensors.injectSensors();

      const [scriptFn, params] = mockPage.addInitScript.mock.calls[0];
      scriptFn(params);

      // Should not be overwritten (using || operator)
      expect(window.DeviceMotionEvent).toBe(originalEvent);
    });
  });

  describe("parameter generation", () => {
    it("should generate valid battery level within range", async () => {
      mockGetPage.mockReturnValue(mockPage);
      mockMathUtils.gaussian.mockReturnValue(0.85);
      mockMathUtils.randomInRange.mockReturnValue(50);

      await sensors.injectSensors();

      const [, params] = mockPage.addInitScript.mock.calls[0];
      expect(params.level).toBeGreaterThanOrEqual(0.5);
      expect(params.level).toBeLessThanOrEqual(1.0);
    });

    it("should generate valid chargingTime within range", async () => {
      mockGetPage.mockReturnValue(mockPage);
      mockMathUtils.gaussian.mockReturnValue(0.85);
      mockMathUtils.randomInRange.mockReturnValue(50);

      await sensors.injectSensors();

      const [, params] = mockPage.addInitScript.mock.calls[0];
      expect(params.chargingTime).toBeGreaterThanOrEqual(0);
      expect(params.chargingTime).toBeLessThanOrEqual(100);
    });

    it("should use consistent gaussian parameters", async () => {
      mockGetPage.mockReturnValue(mockPage);
      mockMathUtils.gaussian.mockReturnValue(0.85);
      mockMathUtils.randomInRange.mockReturnValue(50);

      await sensors.injectSensors();

      expect(mockMathUtils.gaussian).toHaveBeenCalledWith(
        0.85, // mean
        0.1, // deviation
        0.5, // min
        1.0, // max
      );
    });

    it("should use consistent randomInRange parameters", async () => {
      mockGetPage.mockReturnValue(mockPage);
      mockMathUtils.gaussian.mockReturnValue(0.85);
      mockMathUtils.randomInRange.mockReturnValue(50);

      await sensors.injectSensors();

      expect(mockMathUtils.randomInRange).toHaveBeenCalledWith(0, 100);
    });
  });

  describe("edge cases", () => {
    it("should handle page.addInitScript rejection", async () => {
      mockGetPage.mockReturnValue(mockPage);
      mockPage.addInitScript.mockRejectedValue(new Error("Init script failed"));

      await expect(sensors.injectSensors()).rejects.toThrow(
        "Init script failed",
      );
    });

    it("should handle gaussian returning minimum value", async () => {
      mockGetPage.mockReturnValue(mockPage);
      mockMathUtils.gaussian.mockReturnValue(0.5);
      mockMathUtils.randomInRange.mockReturnValue(0);

      await sensors.injectSensors();

      const [, params] = mockPage.addInitScript.mock.calls[0];
      expect(params.level).toBe(0.5);
    });

    it("should handle gaussian returning maximum value", async () => {
      mockGetPage.mockReturnValue(mockPage);
      mockMathUtils.gaussian.mockReturnValue(1.0);
      mockMathUtils.randomInRange.mockReturnValue(100);

      await sensors.injectSensors();

      const [, params] = mockPage.addInitScript.mock.calls[0];
      expect(params.level).toBe(1.0);
    });

    it("should handle chargingTime at minimum (0)", async () => {
      mockGetPage.mockReturnValue(mockPage);
      mockMathUtils.gaussian.mockReturnValue(0.85);
      mockMathUtils.randomInRange.mockReturnValue(0);

      await sensors.injectSensors();

      const [, params] = mockPage.addInitScript.mock.calls[0];
      expect(params.chargingTime).toBe(0);
    });

    it("should handle chargingTime at maximum (100)", async () => {
      mockGetPage.mockReturnValue(mockPage);
      mockMathUtils.gaussian.mockReturnValue(0.85);
      mockMathUtils.randomInRange.mockReturnValue(100);

      await sensors.injectSensors();

      const [, params] = mockPage.addInitScript.mock.calls[0];
      expect(params.chargingTime).toBe(100);
    });

    it("should handle undefined page gracefully", async () => {
      mockGetPage.mockReturnValue(null);

      await expect(sensors.injectSensors()).rejects.toThrow();
    });
  });

  describe("script structure", () => {
    it("should have correct function signature", async () => {
      mockGetPage.mockReturnValue(mockPage);

      await sensors.injectSensors();

      const [scriptFn, params] = mockPage.addInitScript.mock.calls[0];
      expect(typeof scriptFn).toBe("function");
      expect(params).toHaveProperty("level");
      expect(params).toHaveProperty("chargingTime");
      expect(params).toHaveProperty("dischargingTime");
    });

    it("should include all three sensor categories", async () => {
      mockGetPage.mockReturnValue(mockPage);

      await sensors.injectSensors();

      const [scriptFn] = mockPage.addInitScript.mock.calls[0];
      const fnStr = scriptFn.toString();

      // Check for Battery API
      expect(fnStr).toContain("getBattery");
      expect(fnStr).toContain("charging");

      // Check for Network Information API
      expect(fnStr).toContain("connection");
      expect(fnStr).toContain("effectiveType");

      // Check for Device Events
      expect(fnStr).toContain("DeviceOrientationEvent");
      expect(fnStr).toContain("DeviceMotionEvent");
    });

    it("should contain conditional checks for APIs", async () => {
      mockGetPage.mockReturnValue(mockPage);

      await sensors.injectSensors();

      const [scriptFn] = mockPage.addInitScript.mock.calls[0];
      const fnStr = scriptFn.toString();

      expect(fnStr).toContain("'getBattery' in navigator");
      expect(fnStr).toContain("'connection' in navigator");
    });

    it("should use Object.defineProperty for connection", async () => {
      mockGetPage.mockReturnValue(mockPage);

      await sensors.injectSensors();

      const [scriptFn] = mockPage.addInitScript.mock.calls[0];
      const fnStr = scriptFn.toString();

      expect(fnStr).toContain("Object.defineProperty");
    });
  });
});
