/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";

vi.mock("@api/core/context.js", () => ({
  getPage: vi.fn(),
  getEvents: vi.fn(() => ({ emitSafe: vi.fn() })),
}));

import { getPage } from "@api/core/context.js";
import { apply, check, stripCDPMarkers } from "@api/utils/patch.js";

describe("api/utils/patch.js", () => {
  let mockPage;
  let originalWindow;
  let originalNavigator;

  beforeEach(() => {
    vi.clearAllMocks();
    vi.useFakeTimers();

    mockPage = {
      addInitScript: vi.fn().mockImplementation(async (script, payload) => {
        mockPage.lastInitScript = script;
        mockPage.lastInitPayload = payload;
      }),
      evaluate: vi.fn().mockResolvedValue({
        webdriver: false,
        cdcMarkers: false,
        passed: true,
      }),
    };

    getPage.mockReturnValue(mockPage);

    originalWindow = global.window;
    originalNavigator = global.navigator;
  });

  afterEach(() => {
    vi.useRealTimers();
    delete mockPage.lastInitScript;
    delete mockPage.lastInitPayload;

    if (originalWindow === undefined) {
      delete global.window;
    } else {
      global.window = originalWindow;
    }

    if (originalNavigator === undefined) {
      delete global.navigator;
    } else {
      Object.defineProperty(global, "navigator", {
        value: originalNavigator,
        configurable: true,
        writable: true,
      });
    }

    delete global.Notification;
  });

  describe("apply", () => {
    it("adds an init script with the default spoof payload", async () => {
      await apply();

      expect(mockPage.addInitScript).toHaveBeenCalledTimes(1);
      const [script, payload] = mockPage.addInitScript.mock.calls[0];

      expect(script).toEqual(expect.any(Function));
      expect(payload).toEqual({
        languages: ["en-US", "en"],
        deviceMemory: 8,
        hardwareConcurrency: 8,
        maxTouchPoints: 0,
      });
    });

    it("passes a custom fingerprint payload through to the init script", async () => {
      const fingerprint = {
        languages: ["es-ES", "es"],
        deviceMemory: 16,
        hardwareConcurrency: 12,
        maxTouchPoints: 5,
      };

      await apply(fingerprint);

      const [, payload] = mockPage.addInitScript.mock.calls[0];
      expect(payload).toEqual(fingerprint);
    });

    it("exposes the expected patching behavior in the injected script source", async () => {
      await apply();

      const [script] = mockPage.addInitScript.mock.calls[0];
      const source = String(script);

      expect(source).toContain("stripMarkers");
      expect(source).toContain("webdriver");
      expect(source).toContain("PluginArray");
      expect(source).toContain("getBattery");
      expect(source).toContain("Function.prototype.toString");
      expect(source).toContain("navigatorProxy");
    });

    it("executes the injected script against a browser-like shim", async () => {
      await apply();

      const navigatorProto = {};
      const navigatorShim = Object.create(navigatorProto);
      navigatorShim.webdriver = true;
      navigatorShim.languages = ["en"];

      global.navigator = navigatorShim;
      const originalDateNow = Date.now;
      const originalRandom = Math.random;
      Date.now = vi.fn(() => 1700000000000);
      Math.random = vi.fn(() => 0.25);
      global.window = {
        navigator: navigatorShim,
        document: {},
        chrome: null,
        PluginArray: function PluginArray() {},
        Plugin: function Plugin() {},
      };
      global.document = global.window.document;

      const initScript = mockPage.lastInitScript;
      const payload = mockPage.lastInitPayload;

      expect(initScript).toEqual(expect.any(Function));
      expect(() => initScript(payload)).not.toThrow();

      expect(global.window.navigator).toBeDefined();
      expect(global.window.navigator.getBattery()).toBeInstanceOf(Promise);
      await expect(global.window.navigator.getBattery()).resolves.toEqual(
        expect.objectContaining({
          charging: true,
          chargingTime: 0,
          dischargingTime: Infinity,
        }),
      );

      Date.now = originalDateNow;
      Math.random = originalRandom;
    });

    it("patches native-looking getters and toString behavior", async () => {
      await apply();

      const navigatorProto = {};
      const navigatorShim = Object.create(navigatorProto);
      navigatorShim.webdriver = true;
      global.navigator = navigatorShim;
      global.window = {
        navigator: navigatorShim,
        document: {},
        chrome: null,
        PluginArray: function PluginArray() {},
        Plugin: function Plugin() {},
      };
      global.document = global.window.document;

      const initScript = mockPage.lastInitScript;
      expect(initScript).toEqual(expect.any(Function));
      expect(() => initScript(mockPage.lastInitPayload)).not.toThrow();

      const languagesDescriptor = Object.getOwnPropertyDescriptor(
        Object.getPrototypeOf(global.window.navigator),
        "languages",
      );

      expect(languagesDescriptor).toBeDefined();
      expect(languagesDescriptor.get()).toEqual(["en-US", "en"]);
      expect(String(languagesDescriptor.get)).toContain("[native code]");

      const spoofedFunction = function playwrightAutomationProbe() {};
      expect(String(spoofedFunction)).toContain("[native code]");

      expect(global.window.chrome).toBeDefined();
      expect(global.window.chrome.app.getIsInstalled()).toBe(false);
      expect(global.window.chrome.app.getDetails()).toBeNull();
      expect(global.window.chrome.csi()).toEqual(
        expect.objectContaining({
          pageT: 0,
          tran: 0,
        }),
      );
      expect(global.window.chrome.loadTimes()).toEqual(
        expect.objectContaining({
          navigationType: "Other",
          wasFetchedFromCache: false,
        }),
      );
      const pluginList = global.window.navigator.plugins;
      expect(pluginList).toHaveLength(3);
      expect(pluginList[0]).toEqual(
        expect.objectContaining({
          name: "PDF Viewer",
          filename: "internal-pdf-viewer",
        }),
      );
      expect(pluginList[Symbol.iterator]).toEqual(expect.any(Function));
      expect(pluginList.item(1)).toBe(pluginList[1]);
      expect(pluginList.namedItem("ignored")).toBe(pluginList[0]);
      expect(() => pluginList.refresh()).not.toThrow();
      expect([...pluginList]).toHaveLength(3);
      expect(
        Object.prototype.hasOwnProperty.call(
          global.window.navigator,
          "languages",
        ),
      ).toBe(false);
      expect(
        Object.prototype.hasOwnProperty.call(
          global.window.navigator,
          "webdriver",
        ),
      ).toBe(false);
      expect(Object.keys(global.window.navigator)).not.toContain("webdriver");
      expect(Reflect.ownKeys(global.window.navigator)).not.toContain(
        "webdriver",
      );
      expect(
        Object.getOwnPropertyDescriptor(
          Object.getPrototypeOf(global.window.navigator),
          "languages",
        ),
      ).toEqual(
        expect.objectContaining({
          configurable: true,
          enumerable: true,
        }),
      );
    });

    it("leaves an existing chrome object untouched", async () => {
      await apply();

      const navigatorProto = {};
      const navigatorShim = Object.create(navigatorProto);
      navigatorShim.webdriver = true;
      global.navigator = navigatorShim;
      global.window = {
        navigator: navigatorShim,
        document: {},
        chrome: { alreadyPresent: true },
        PluginArray: function PluginArray() {},
        Plugin: function Plugin() {},
      };
      global.document = global.window.document;

      const initScript = mockPage.lastInitScript;
      expect(() => initScript(mockPage.lastInitPayload)).not.toThrow();
      expect(global.window.chrome).toEqual({ alreadyPresent: true });
    });

    it("handles a navigator without a prototype", async () => {
      await apply();

      const navigatorShim = Object.create(null);
      navigatorShim.webdriver = true;

      global.navigator = navigatorShim;
      global.window = {
        navigator: navigatorShim,
        document: {},
        chrome: null,
        PluginArray: function PluginArray() {},
        Plugin: function Plugin() {},
      };
      global.document = global.window.document;

      const initScript = mockPage.lastInitScript;
      expect(() => initScript(mockPage.lastInitPayload)).not.toThrow();
    });

    it("propagates page errors from addInitScript", async () => {
      mockPage.addInitScript.mockRejectedValueOnce(new Error("Page closed"));

      await expect(apply()).rejects.toThrow("Page closed");
    });
  });

  describe("stripCDPMarkers", () => {
    it("does not throw when window is missing", () => {
      delete global.window;

      expect(() => stripCDPMarkers()).not.toThrow();
    });

    it("clears known marker properties when present", () => {
      global.window = {
        cdc_adoQjvpsHSjkbJjLPRbPQ: "present",
        $cdc_asdjflasutopfhvcZLmcfl_: "present",
      };

      stripCDPMarkers();

      expect(global.window.cdc_adoQjvpsHSjkbJjLPRbPQ).toBeUndefined();
      expect(global.window.$cdc_asdjflasutopfhvcZLmcfl_).toBeUndefined();
    });

    it("ignores assignment failures gracefully", () => {
      const windowMock = {};
      Object.defineProperty(windowMock, "cdc_adoQjvpsHSjkbJjLPRbPQ", {
        configurable: true,
        get() {
          throw new Error("read failure");
        },
      });

      global.window = windowMock;

      expect(() => stripCDPMarkers()).not.toThrow();
    });

    it("leaves non-marker descriptors untouched", () => {
      const windowMock = {};
      Object.defineProperty(windowMock, "safeValue", {
        configurable: true,
        get() {
          return 1;
        },
      });

      global.window = windowMock;

      stripCDPMarkers();

      expect(
        Object.getOwnPropertyDescriptor(global.window, "safeValue").get(),
      ).toBe(1);
    });
  });

  describe("check", () => {
    it("returns the detection result from the page evaluation", async () => {
      const result = await check();

      expect(mockPage.evaluate).toHaveBeenCalledTimes(1);
      expect(result).toEqual({
        webdriver: false,
        cdcMarkers: false,
        passed: true,
      });
    });

    it("returns a failed status when webdriver is reported", async () => {
      mockPage.evaluate.mockResolvedValueOnce({
        webdriver: true,
        cdcMarkers: false,
        passed: false,
      });

      const result = await check();

      expect(result.webdriver).toBe(true);
      expect(result.passed).toBe(false);
    });

    it("returns a failed status when cdc markers are reported", async () => {
      mockPage.evaluate.mockResolvedValueOnce({
        webdriver: false,
        cdcMarkers: true,
        passed: false,
      });

      const result = await check();

      expect(result.cdcMarkers).toBe(true);
      expect(result.passed).toBe(false);
    });

    it("uses navigator and window state inside page.evaluate", async () => {
      mockPage.evaluate.mockImplementationOnce(async (fn) => {
        global.window = {
          cdc_adoQjvpsHSjkbJjLPRbPQ: undefined,
          $cdc_asdjflasutopfhvcZLmcfl_: undefined,
        };
        global.navigator = { webdriver: false };

        return fn();
      });

      const result = await check();

      expect(result).toEqual({
        webdriver: false,
        cdcMarkers: false,
        passed: true,
      });
    });

    it("propagates page evaluation errors", async () => {
      mockPage.evaluate.mockRejectedValueOnce(new Error("Eval error"));

      await expect(check()).rejects.toThrow("Eval error");
    });
  });
});
