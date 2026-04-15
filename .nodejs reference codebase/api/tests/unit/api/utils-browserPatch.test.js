/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { applyHumanizationPatch } from "@api/utils/browserPatch.js";

function setGlobalNavigator(value) {
  Object.defineProperty(global, "navigator", {
    value,
    writable: true,
    configurable: true,
  });
}

function installBrowserShim({
  ua = "Mozilla/5.0 (Windows NT 10.0; Win64; x64)",
  hostname,
} = {}) {
  const originalNavigator = global.navigator;
  const originalDocument = global.document;
  const originalCanvas = global.HTMLCanvasElement;
  const originalWindow = global.window;

  const originalToDataURL = vi.fn().mockReturnValue("original");
  const context = {
    fillStyle: "initial",
    fillRect: vi.fn(),
  };

  setGlobalNavigator({
    userAgent: ua,
    platform: "Win32",
  });
  global.document = {
    hidden: true,
    visibilityState: "hidden",
    addEventListener: vi.fn(),
  };
  global.HTMLCanvasElement = {
    prototype: {
      toDataURL: originalToDataURL,
      getContext: vi.fn(() => context),
    },
  };
  global.window = {
    location: hostname ? { hostname } : undefined,
  };

  return {
    context,
    originalToDataURL,
    restore() {
      setGlobalNavigator(originalNavigator);
      global.document = originalDocument;
      global.HTMLCanvasElement = originalCanvas;
      global.window = originalWindow;
    },
  };
}

describe("api/utils/browserPatch.js", () => {
  let mockPage;
  let mockLogger;

  beforeEach(() => {
    vi.clearAllMocks();

    mockLogger = {
      info: vi.fn(),
    };

    mockPage = {
      addInitScript: vi.fn(),
    };
  });

  describe("applyHumanizationPatch", () => {
    it("should add init script to page", async () => {
      mockPage.addInitScript.mockImplementation(async (fn) => {
        const shim = installBrowserShim();
        try {
          await fn();
        } finally {
          shim.restore();
        }
      });

      await applyHumanizationPatch(mockPage);

      expect(mockPage.addInitScript).toHaveBeenCalled();
    });

    it("should add init script with function", async () => {
      await applyHumanizationPatch(mockPage);

      const call = mockPage.addInitScript.mock.calls[0][0];
      expect(typeof call).toBe("function");
    });

    it("should call logger info when logger provided", async () => {
      await applyHumanizationPatch(mockPage, mockLogger);
      expect(mockLogger.info).toHaveBeenCalled();
    });

    it("should not call logger info when no logger provided", async () => {
      await applyHumanizationPatch(mockPage);
      expect(mockLogger.info).not.toHaveBeenCalled();
    });

    it("should work without logger parameter", async () => {
      await expect(applyHumanizationPatch(mockPage)).resolves.not.toThrow();
    });

    it("should handle page without addInitScript gracefully", async () => {
      await expect(applyHumanizationPatch({})).rejects.toThrow();
    });

    it("should spoof platform for Windows UA", async () => {
      const shim = installBrowserShim({
        ua: "Mozilla/5.0 (Windows NT 10.0; Win64; x64)",
      });
      let observedPlatform;
      mockPage.addInitScript.mockImplementation(async (fn) => {
        try {
          await fn();
          observedPlatform = global.navigator.platform;
        } finally {
          shim.restore();
        }
      });

      await applyHumanizationPatch(mockPage);

      expect(observedPlatform).toBe("Win32");
    });

    it("should spoof platform for Mac UA", async () => {
      const shim = installBrowserShim({
        ua: "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)",
      });
      let observedPlatform;
      mockPage.addInitScript.mockImplementation(async (fn) => {
        try {
          await fn();
          observedPlatform = global.navigator.platform;
        } finally {
          shim.restore();
        }
      });

      await applyHumanizationPatch(mockPage);

      expect(observedPlatform).toBe("MacIntel");
    });

    it("should spoof platform for Linux UA", async () => {
      const shim = installBrowserShim({
        ua: "Mozilla/5.0 (X11; Linux x86_64)",
      });
      let observedPlatform;
      mockPage.addInitScript.mockImplementation(async (fn) => {
        try {
          await fn();
          observedPlatform = global.navigator.platform;
        } finally {
          shim.restore();
        }
      });

      await applyHumanizationPatch(mockPage);

      expect(observedPlatform).toBe("Linux x86_64");
    });

    it("should preserve canvas behavior on x.com", async () => {
      const shim = installBrowserShim({ hostname: "x.com" });
      mockPage.addInitScript.mockImplementation(async (fn) => {
        try {
          await fn();
        } finally {
          // keep the canvas call below against the patched prototype
        }
      });

      await applyHumanizationPatch(mockPage);
      const result = global.HTMLCanvasElement.prototype.toDataURL.call({
        width: 10,
        height: 10,
        getContext: vi.fn(() => null),
      });

      shim.restore();

      expect(result).toBe("original");
    });

    it("should preserve canvas behavior on twitter.com", async () => {
      const shim = installBrowserShim({ hostname: "twitter.com" });
      mockPage.addInitScript.mockImplementation(async (fn) => {
        try {
          await fn();
        } finally {
          // keep the canvas call below against the patched prototype
        }
      });

      await applyHumanizationPatch(mockPage);
      const result = global.HTMLCanvasElement.prototype.toDataURL.call({
        width: 10,
        height: 10,
        getContext: vi.fn(() => null),
      });

      shim.restore();

      expect(result).toBe("original");
    });

    it("should spoof visibility state and webdriver", async () => {
      const shim = installBrowserShim();
      let observedHidden;
      let observedState;
      let observedWebdriver;
      mockPage.addInitScript.mockImplementation(async (fn) => {
        try {
          await fn();
          observedHidden = global.document.hidden;
          observedState = global.document.visibilityState;
          observedWebdriver = global.navigator.webdriver;
        } finally {
          shim.restore();
        }
      });

      await applyHumanizationPatch(mockPage);

      expect(observedHidden).toBe(false);
      expect(observedState).toBe("visible");
      expect(observedWebdriver).toBe(false);
    });
  });
});
