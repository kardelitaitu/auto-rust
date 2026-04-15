/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Detection API Patching
 * Strips automation markers and patches runtime detection vectors.
 *
 * @module api/patch
 */

import { getPage, getEvents } from "../core/context.js";

function _safeEmitError(error, context) {
  try {
    const events = getEvents();
    events.emitSafe("on:error", { context, error });
  } catch {
    // ignore if no events available
  }
}

/**
 * Apply all detection patches to the page.
 * Should be called within api.withPage() context to inject patches into page context.
 * @param {object} [fingerprint] - Optional fingerprint data to inject
 * @returns {Promise<void>}
 */
export async function apply(fingerprint = null) {
  const page = getPage();

  const spoofData = fingerprint || {
    languages: ["en-US", "en"],
    deviceMemory: 8,
    hardwareConcurrency: 8,
    maxTouchPoints: 0,
  };

  // Add init script to patch detection APIs
  await page.addInitScript((data) => {
    // 1. Exhaustive CDP & Webdriver Marker Stripping
    const originalNavigator = navigator;

    const stripMarkers = () => {
      const markers = ["cdc_", "__webdriver", "__driver", "dom-automation-"];
      [window, document].forEach((target) => {
        for (const key in target) {
          try {
            if (markers.some((m) => key.includes(m))) {
              delete target[key];
            }
          } catch (_e) {
            console.warn("[Patch] Failed to strip marker", key, _e);
          }
        }
      });
    };
    stripMarkers();

    // 2. WebDriver: prototype-only spoof (avoid ownKeys/hasOwnProperty leaks)
    try {
      if (navigator && "webdriver" in navigator) {
        try {
          delete navigator.webdriver;
        } catch (_e) {
          console.warn("[Patch] Failed to delete webdriver", _e);
        }
      }

      const navProto =
        navigator && typeof navigator === "object"
          ? Object.getPrototypeOf(navigator)
          : null;
      if (navProto) {
        Object.defineProperty(navProto, "webdriver", {
          get: () => false,
          configurable: true,
          enumerable: false,
        });
      }
    } catch (_e) {
      console.warn("[Patch] Failed to redefine webdriver", _e);
    }

    // 3. Advanced Navigator Patch (Timing Optimized)
    try {
      const navProto =
        originalNavigator && typeof originalNavigator === "object"
          ? Object.getPrototypeOf(originalNavigator)
          : null;
      const staticSpoofs = {
        languages: data.languages || ["en-US", "en"],
        deviceMemory: data.deviceMemory || 8,
        hardwareConcurrency: data.hardwareConcurrency || 8,
        webdriver: false,
        maxTouchPoints: data.maxTouchPoints || 0,
      };

      const makeNativeGetter = (name, value) => {
        const getter = {
          [name]: function () {
            return value;
          },
        }[name];
        getter.__native_source = `function get ${name}() { [native code] }`;
        Object.defineProperty(getter, "name", {
          value: `get ${name}`,
          configurable: true,
        });
        return getter;
      };

      if (navProto && typeof navProto === "object") {
        for (const [prop, value] of Object.entries(staticSpoofs)) {
          try {
            Object.defineProperty(navProto, prop, {
              get: makeNativeGetter(prop, value),
              configurable: true,
              enumerable: prop !== "webdriver",
            });
          } catch (e) {
            console.warn("[Patch] Failed to define getter for", prop, e);
          }
        }
      }

      // Ghost 3.0: Chrome Object Hardening
      if (typeof window !== "undefined" && !window.chrome) {
        const chromeMock = {
          app: {
            isInstalled: false,
            InstallState: {
              DISABLED: "disabled",
              INSTALLED: "installed",
              NOT_INSTALLED: "not_installed",
            },
            getIsInstalled: () => false,
            getDetails: () => null,
          },
          csi: () => ({
            startE: Date.now(),
            onloadT: Date.now(),
            pageT: 0,
            tran: 0,
          }),
          loadTimes: () => ({
            requestTime: Date.now() / 1000,
            startLoadTime: Date.now() / 1000,
            commitLoadTime: Date.now() / 1000,
            finishDocumentLoadTime: Date.now() / 1000,
            finishLoadTime: Date.now() / 1000,
            firstPaintTime: Date.now() / 1000,
            firstPaintAfterLoadTime: 0,
            navigationType: "Other",
            wasFetchedFromCache: false,
            wasAlternateProtocolAvailable: false,
            wasContradictoryProxyConfig: false,
          }),
          runtime: {
            OnInstalledReason: {
              INSTALL: "install",
              UPDATE: "update",
              CHROME_UPDATE: "chrome_update",
              SHARED_MODULE_UPDATE: "shared_module_update",
            },
            OnRestartRequiredReason: {
              APP_UPDATE: "app_update",
              OS_UPDATE: "os_update",
              PERIODIC: "periodic",
            },
            PlatformOs: {
              MAC: "mac",
              WIN: "win",
              ANDROID: "android",
              CROS: "cros",
              LINUX: "linux",
              OPENBSD: "openbsd",
            },
            PlatformArch: {
              ARM: "arm",
              X86_32: "x86-32",
              X86_64: "x86-64",
              MIPS: "mips",
              MIPS64: "mips64",
            },
            PlatformNaclArch: {
              ARM: "arm",
              X86_32: "x86-32",
              X86_64: "x86-64",
              MIPS: "mips",
              MIPS64: "mips64",
            },
            RequestUpdateCheckStatus: {
              THROTTLED: "throttled",
              NO_UPDATE: "no_update",
              UPDATE_AVAILABLE: "update_available",
            },
          },
        };
        window.chrome = chromeMock;
      }

      // Still need Proxy for complex objects like plugins and battery
      const originalNavProto =
        originalNavigator && typeof originalNavigator === "object"
          ? Object.getPrototypeOf(originalNavigator)
          : Object.prototype;
      const fakeNavigator = Object.create(originalNavProto);

      // Still need Proxy for complex objects like plugins and battery
      const navigatorProxy = new Proxy(fakeNavigator, {
        get: (target, prop) => {
          if (prop === Symbol.toStringTag) return "Navigator";

          // Mask instance properties redirected to our perfect prototype getters
          if (Object.prototype.hasOwnProperty.call(staticSpoofs, prop))
            return staticSpoofs[prop];

          // Override hasOwnProperty to truly hide masked properties
          if (prop === "hasOwnProperty") {
            return (key) => {
              if (Object.prototype.hasOwnProperty.call(staticSpoofs, key))
                return false;
              return Object.prototype.hasOwnProperty.call(
                originalNavigator,
                key,
              );
            };
          }

          // Spoof complex plugins object
          if (prop === "plugins") {
            const hasWindow = typeof window !== "undefined";
            const pluginArrayProto =
              hasWindow && window.PluginArray
                ? window.PluginArray.prototype
                : Object.prototype;
            const pluginProto =
              hasWindow && window.Plugin
                ? window.Plugin.prototype
                : Object.prototype;

            // Create objects that mimic PluginArray and Plugin identity
            const pluginArray = Object.create(pluginArrayProto, {
              [Symbol.toStringTag]: { value: "PluginArray", enumerable: false },
              length: { value: 3, enumerable: true },
              item: {
                value: function (i) {
                  return this[i];
                },
                enumerable: true,
              },
              namedItem: {
                value: function () {
                  return this[0];
                },
                enumerable: true,
              },
              refresh: { value: function () {}, enumerable: true },
            });

            const createPlugin = (data) => {
              const p = Object.create(pluginProto, {
                [Symbol.toStringTag]: { value: "Plugin", enumerable: false },
                name: { value: data.name, enumerable: true },
                filename: { value: data.filename, enumerable: true },
                description: { value: data.description, enumerable: true },
                length: { value: 0, enumerable: true },
                item: {
                  value: function () {
                    return null;
                  },
                  enumerable: true,
                },
                namedItem: {
                  value: function () {
                    return null;
                  },
                  enumerable: true,
                },
              });
              return p;
            };

            pluginArray[0] = createPlugin({
              name: "PDF Viewer",
              filename: "internal-pdf-viewer",
              description: "Portable Document Format",
            });
            pluginArray[1] = createPlugin({
              name: "Chrome PDF Viewer",
              filename: "internal-pdf-viewer",
              description: "Portable Document Format",
            });
            pluginArray[2] = createPlugin({
              name: "Chromium PDF Viewer",
              filename: "internal-pdf-viewer",
              description: "Portable Document Format",
            });

            // Ghost 3.0: Make plugins iterable
            pluginArray[Symbol.iterator] = function* () {
              yield this[0];
              yield this[1];
              yield this[2];
            };
            return pluginArray;
          }

          // Ghost 3.0: Submerge Battery spoofing
          if (prop === "getBattery") {
            return () =>
              Promise.resolve({
                level: 0.85 + Math.random() * 0.1,
                charging: true,
                chargingTime: 0,
                dischargingTime: Infinity,
                addEventListener: () => {},
                removeEventListener: () => {},
                dispatchEvent: () => true,
                onchargingchange: null,
                onchargingtimechange: null,
                ondischargingtimechange: null,
                onlevelchange: null,
                [Symbol.toStringTag]: "BatteryManager",
              });
          }

          const value = originalNavigator[prop];
          if (typeof value === "function") {
            return value.bind(originalNavigator);
          }
          return value;
        },
        ownKeys: (target) => {
          const keys = Reflect.ownKeys(originalNavigator);
          // Add mandatory Proxy target keys to satisfy invariants (like platform)
          return [...new Set([...keys, ...Reflect.ownKeys(target)])].filter(
            (key) => !Object.prototype.hasOwnProperty.call(staticSpoofs, key),
          );
        },
        getOwnPropertyDescriptor: (target, prop) => {
          if (Object.prototype.hasOwnProperty.call(staticSpoofs, prop))
            return undefined;
          return (
            Reflect.getOwnPropertyDescriptor(originalNavigator, prop) ||
            Reflect.getOwnPropertyDescriptor(target, prop)
          );
        },
        getPrototypeOf: () => {
          return originalNavigator && typeof originalNavigator === "object"
            ? Object.getPrototypeOf(originalNavigator)
            : Object.prototype;
        },
      });

      // Ghost 3.0: Permissions API hardening
      if (
        typeof window !== "undefined" &&
        window.navigator &&
        window.navigator.permissions
      ) {
        const originalQuery = window.navigator.permissions.query;
        window.navigator.permissions.query = (parameters) =>
          parameters.name === "notifications"
            ? Promise.resolve({
                state:
                  Notification.permission === "default"
                    ? "prompt"
                    : Notification.permission,
              })
            : originalQuery(parameters);
      }

      // Ghost 3.0: Ensure navigator is immutable so driver cannot re-inject properties
      if (typeof window === "object" && window !== null) {
        Object.defineProperty(window, "navigator", {
          value: navigatorProxy,
          configurable: false,
          writable: false,
          enumerable: true,
        });
      }
    } catch (e) {
      console.warn("[Patch] Failed to apply static spoofs", e);
    }

    // 4. Robust Function.prototype.toString (Proxy-Aware)
    // 4. Robust Function.prototype.toString (Proxy-Aware)
    try {
      const originalToString = Function.prototype.toString;
      const patchedToString = function () {
        if (this === patchedToString)
          return "function toString() { [native code] }";
        if (this.__native_source) return this.__native_source;

        const name = this.name || "";
        const keywords = [
          "playwright",
          "puppeteer",
          "selenium",
          "automation",
          "cdc_",
        ];

        if (keywords.some((k) => name.toLowerCase().includes(k))) {
          return `function ${name}() { [native code] }`;
        }

        return originalToString.call(this);
      };

      // Patching toString itself is meta, must be careful
      Object.defineProperty(Function.prototype, "toString", {
        value: patchedToString,
        configurable: true,
        writable: true,
        enumerable: false,
      });
    } catch (e) {
      console.warn("[Patch] Failed to patch toString", e);
    }
  }, spoofData);
}

/**
 * Strip CDP markers from window object.
 * @returns {void}
 */
export function stripCDPMarkers() {
  // This is handled via addInitScript in apply()
  // Exposed as separate function for explicit calling if needed
  if (typeof window !== "undefined") {
    try {
      window.cdc_adoQjvpsHSjkbJjLPRbPQ = undefined;
      window.$cdc_asdjflasutopfhvcZLmcfl_ = undefined;
    } catch (e) {
      void e;
    }
  }
}

/**
 * Check if page passes basic detection checks.
 * Useful for testing.
 * @returns {Promise<{webdriver: boolean, cdcMarkers: boolean, passed: boolean}>}
 */
export async function check() {
  const page = getPage();

  const results = await page.evaluate(() => {
    const webdriver = navigator ? navigator.webdriver : false;
    const hasCDC = !!(
      (typeof window !== "undefined" && window.cdc_adoQjvpsHSjkbJjLPRbPQ) ||
      (typeof window !== "undefined" && window.$cdc_asdjflasutopfhvcZLmcfl_)
    );

    return {
      webdriver: webdriver === true,
      cdcMarkers: hasCDC,
      passed: !webdriver && !hasCDC,
    };
  });

  return results;
}
