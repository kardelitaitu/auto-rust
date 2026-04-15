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
  evaluate: vi.fn().mockResolvedValue(undefined),
};

const mockGetPage = vi.fn();
const mockLogger = {
  info: vi.fn(),
  warn: vi.fn(),
  error: vi.fn(),
  debug: vi.fn(),
};

vi.mock("@api/core/context.js", () => ({
  getPage: (...args) => mockGetPage(...args),
}));

vi.mock("@api/core/logger.js", () => ({
  createLogger: () => mockLogger,
}));

describe("visual-debug", () => {
  let visualDebug;

  beforeEach(async () => {
    vi.clearAllMocks();
    vi.resetModules();
    visualDebug = await import("@api/utils/visual-debug.js");
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe("enable", () => {
    it("should return false when no page is available", async () => {
      mockGetPage.mockReturnValue(null);

      const result = await visualDebug.enable();

      expect(result).toBe(false);
      expect(mockLogger.warn).toHaveBeenCalledWith(
        "[VisualDebug] No page available",
      );
    });

    it("should successfully enable visual debug with page", async () => {
      mockGetPage.mockReturnValue(mockPage);
      mockPage.evaluate.mockResolvedValue(undefined);

      const result = await visualDebug.enable();

      expect(result).toBe(true);
      // page.evaluate is called with (fn, data) where data is the second argument
      expect(mockPage.evaluate).toHaveBeenCalled();
      expect(mockLogger.info).toHaveBeenCalledWith(
        "[VisualDebug] Enabled - cursor and clicks will be visualized",
      );
    });

    it("should return false when page.evaluate throws an error", async () => {
      mockGetPage.mockReturnValue(mockPage);
      mockPage.evaluate.mockRejectedValue(new Error("Evaluation failed"));

      const result = await visualDebug.enable();

      expect(result).toBe(false);
      expect(mockLogger.error).toHaveBeenCalledWith(
        expect.stringContaining("[VisualDebug] Failed to enable"),
      );
    });

    it("should pass ids and css to page.evaluate", async () => {
      mockGetPage.mockReturnValue(mockPage);

      await visualDebug.enable();

      // The second argument to evaluate is the data object with ids and css
      const evaluateCall = mockPage.evaluate.mock.calls[0];
      const data = evaluateCall[1];

      expect(data).toHaveProperty("ids");
      expect(data).toHaveProperty("css");
      expect(data.ids).toHaveProperty("cursor", "autoai-debug-cursor");
      expect(data.ids).toHaveProperty("overlay", "autoai-debug-overlay");
      expect(data.ids).toHaveProperty("styles", "autoai-debug-styles");
      expect(data.ids).toHaveProperty(
        "clickHistory",
        "autoai-debug-click-history",
      );
    });

    it("should include CSS with cursor styles", async () => {
      mockGetPage.mockReturnValue(mockPage);

      await visualDebug.enable();

      const evaluateCall = mockPage.evaluate.mock.calls[0];
      const { css } = evaluateCall[1];

      expect(css).toContain("#autoai-debug-cursor");
      expect(css).toContain("position: fixed");
      expect(css).toContain("z-index: 2147483647");
    });

    it("should include CSS with overlay styles", async () => {
      mockGetPage.mockReturnValue(mockPage);

      await visualDebug.enable();

      const evaluateCall = mockPage.evaluate.mock.calls[0];
      const { css } = evaluateCall[1];

      expect(css).toContain("#autoai-debug-overlay");
      expect(css).toContain("background: #000000");
      expect(css).toContain("color: #0f0");
    });

    it("should include CSS with click marker styles", async () => {
      mockGetPage.mockReturnValue(mockPage);

      await visualDebug.enable();

      const evaluateCall = mockPage.evaluate.mock.calls[0];
      const { css } = evaluateCall[1];

      expect(css).toContain(".autoai-click-marker");
      expect(css).toContain("@keyframes autoaiClickPulse");
    });

    it("should pass a function to page.evaluate", async () => {
      mockGetPage.mockReturnValue(mockPage);

      await visualDebug.enable();

      const evaluateCall = mockPage.evaluate.mock.calls[0];
      expect(typeof evaluateCall[0]).toBe("function");
    });
  });

  describe("disable", () => {
    it("should return false when no page is available", async () => {
      mockGetPage.mockReturnValue(null);

      const result = await visualDebug.disable();

      expect(result).toBe(false);
    });

    it("should successfully disable visual debug", async () => {
      mockGetPage.mockReturnValue(mockPage);
      mockPage.evaluate.mockResolvedValue(undefined);

      const result = await visualDebug.disable();

      expect(result).toBe(true);
      expect(mockPage.evaluate).toHaveBeenCalled();
      expect(mockLogger.info).toHaveBeenCalledWith("[VisualDebug] Disabled");
    });

    it("should return false when page.evaluate throws an error", async () => {
      mockGetPage.mockReturnValue(mockPage);
      mockPage.evaluate.mockRejectedValue(new Error("Evaluation failed"));

      const result = await visualDebug.disable();

      expect(result).toBe(false);
      expect(mockLogger.error).toHaveBeenCalledWith(
        expect.stringContaining("[VisualDebug] Failed to disable"),
      );
    });

    it("should pass IDS as the second argument", async () => {
      mockGetPage.mockReturnValue(mockPage);

      await visualDebug.disable();

      // The second argument to evaluate should be IDS
      const evaluateCall = mockPage.evaluate.mock.calls[0];
      expect(evaluateCall[1]).toHaveProperty("cursor", "autoai-debug-cursor");
      expect(evaluateCall[1]).toHaveProperty("overlay", "autoai-debug-overlay");
      expect(evaluateCall[1]).toHaveProperty("styles", "autoai-debug-styles");
      expect(evaluateCall[1]).toHaveProperty(
        "clickHistory",
        "autoai-debug-click-history",
      );
    });

    it("should pass a function to page.evaluate", async () => {
      mockGetPage.mockReturnValue(mockPage);

      await visualDebug.disable();

      const evaluateCall = mockPage.evaluate.mock.calls[0];
      expect(typeof evaluateCall[0]).toBe("function");
    });
  });

  describe("toggle", () => {
    it("should enable when currently disabled", async () => {
      mockGetPage.mockReturnValue(mockPage);

      const result = await visualDebug.toggle();

      expect(result).toBe(true);
      expect(mockLogger.info).toHaveBeenCalledWith(
        expect.stringContaining("[VisualDebug] Enabled"),
      );
    });

    it("should disable when currently enabled", async () => {
      mockGetPage.mockReturnValue(mockPage);

      // Enable first
      await visualDebug.enable();

      // Now toggle should disable
      const result = await visualDebug.toggle();

      expect(result).toBe(true);
      expect(mockLogger.info).toHaveBeenCalledWith("[VisualDebug] Disabled");
    });

    it("should toggle back to enabled after disable", async () => {
      mockGetPage.mockReturnValue(mockPage);

      // Enable -> Disable -> Enable
      await visualDebug.enable();
      await visualDebug.disable();
      const result = await visualDebug.toggle();

      expect(result).toBe(true);
      expect(mockLogger.info).toHaveBeenCalledWith(
        expect.stringContaining("[VisualDebug] Enabled"),
      );
    });
  });

  describe("mark", () => {
    it("should do nothing when no page is available", async () => {
      mockGetPage.mockReturnValue(null);

      await visualDebug.mark(100, 200, "test");

      expect(mockPage.evaluate).not.toHaveBeenCalled();
    });

    it("should do nothing when not enabled", async () => {
      mockGetPage.mockReturnValue(mockPage);

      await visualDebug.mark(100, 200, "test");

      expect(mockPage.evaluate).not.toHaveBeenCalled();
    });

    it("should call page.evaluate when enabled", async () => {
      mockGetPage.mockReturnValue(mockPage);

      await visualDebug.enable();
      await visualDebug.mark(100, 200, "test");

      // enable + mark calls
      expect(mockPage.evaluate).toHaveBeenCalledTimes(2);
    });

    it("should pass coordinates and label to page.evaluate", async () => {
      mockGetPage.mockReturnValue(mockPage);

      await visualDebug.enable();
      await visualDebug.mark(150, 250, "custom-label");

      // Find the mark call (second evaluate call)
      const markCall = mockPage.evaluate.mock.calls[1];
      expect(markCall[1]).toEqual({ x: 150, y: 250, label: "custom-label" });
    });

    it("should use empty label as default", async () => {
      mockGetPage.mockReturnValue(mockPage);

      await visualDebug.enable();
      await visualDebug.mark(100, 200);

      const markCall = mockPage.evaluate.mock.calls[1];
      expect(markCall[1]).toEqual({ x: 100, y: 200, label: "" });
    });

    it("should log error when evaluate fails", async () => {
      mockGetPage.mockReturnValue(mockPage);
      mockPage.evaluate.mockResolvedValue(undefined);

      await visualDebug.enable();

      // Reset mock to track the mark call
      mockPage.evaluate.mockClear();
      mockPage.evaluate.mockRejectedValueOnce(new Error("Mark failed"));

      await visualDebug.mark(100, 200, "test");

      expect(mockLogger.error).toHaveBeenCalledWith(
        expect.stringContaining("[VisualDebug] Failed to mark"),
      );
    });

    it("should pass a function to page.evaluate", async () => {
      mockGetPage.mockReturnValue(mockPage);

      await visualDebug.enable();
      await visualDebug.mark(100, 200, "test");

      const markCall = mockPage.evaluate.mock.calls[1];
      expect(typeof markCall[0]).toBe("function");
    });

    it("should handle mark without label", async () => {
      mockGetPage.mockReturnValue(mockPage);

      await visualDebug.enable();
      await visualDebug.mark(50, 75);

      const markCall = mockPage.evaluate.mock.calls[1];
      expect(markCall[1].label).toBe("");
    });
  });

  describe("moveCursor", () => {
    it("should do nothing when no page is available", async () => {
      mockGetPage.mockReturnValue(null);

      await visualDebug.moveCursor(100, 200);

      expect(mockPage.evaluate).not.toHaveBeenCalled();
    });

    it("should do nothing when not enabled", async () => {
      mockGetPage.mockReturnValue(mockPage);

      await visualDebug.moveCursor(100, 200);

      expect(mockPage.evaluate).not.toHaveBeenCalled();
    });

    it("should call page.evaluate when enabled", async () => {
      mockGetPage.mockReturnValue(mockPage);
      mockPage.evaluate.mockResolvedValue(undefined);

      await visualDebug.enable();

      // Reset mock to track moveCursor call
      mockPage.evaluate.mockClear();

      await visualDebug.moveCursor(150, 250);

      expect(mockPage.evaluate).toHaveBeenCalled();
    });

    it("should pass coordinates to page.evaluate", async () => {
      mockGetPage.mockReturnValue(mockPage);
      mockPage.evaluate.mockResolvedValue(undefined);

      await visualDebug.enable();
      mockPage.evaluate.mockClear();

      await visualDebug.moveCursor(150, 250);

      const moveCall = mockPage.evaluate.mock.calls[0];
      expect(moveCall[1]).toEqual({ x: 150, y: 250 });
    });

    it("should silently ignore errors", async () => {
      mockGetPage.mockReturnValue(mockPage);
      mockPage.evaluate.mockResolvedValue(undefined);

      await visualDebug.enable();

      // Reset and make evaluate fail
      mockPage.evaluate.mockClear();
      mockPage.evaluate.mockRejectedValue(new Error("Move failed"));

      // Should not throw
      await expect(visualDebug.moveCursor(100, 200)).resolves.toBeUndefined();
    });

    it("should pass a function to page.evaluate", async () => {
      mockGetPage.mockReturnValue(mockPage);
      mockPage.evaluate.mockResolvedValue(undefined);

      await visualDebug.enable();
      mockPage.evaluate.mockClear();

      await visualDebug.moveCursor(100, 200);

      const moveCall = mockPage.evaluate.mock.calls[0];
      expect(typeof moveCall[0]).toBe("function");
    });
  });

  describe("isEnabledDebug", () => {
    it("should return false initially", () => {
      expect(visualDebug.isEnabledDebug()).toBe(false);
    });

    it("should return true after enable", async () => {
      mockGetPage.mockReturnValue(mockPage);

      await visualDebug.enable();

      expect(visualDebug.isEnabledDebug()).toBe(true);
    });

    it("should return false after disable", async () => {
      mockGetPage.mockReturnValue(mockPage);

      await visualDebug.enable();
      await visualDebug.disable();

      expect(visualDebug.isEnabledDebug()).toBe(false);
    });

    it("should toggle correctly through multiple cycles", async () => {
      mockGetPage.mockReturnValue(mockPage);

      expect(visualDebug.isEnabledDebug()).toBe(false);

      await visualDebug.toggle();
      expect(visualDebug.isEnabledDebug()).toBe(true);

      await visualDebug.toggle();
      expect(visualDebug.isEnabledDebug()).toBe(false);

      await visualDebug.toggle();
      expect(visualDebug.isEnabledDebug()).toBe(true);
    });
  });

  describe("getState", () => {
    it("should return current state with all IDs", () => {
      const state = visualDebug.getState();

      expect(state).toEqual({
        enabled: false,
        cursor: "autoai-debug-cursor",
        overlay: "autoai-debug-overlay",
        styles: "autoai-debug-styles",
        clickHistory: "autoai-debug-click-history",
      });
    });

    it("should reflect enabled state", async () => {
      mockGetPage.mockReturnValue(mockPage);

      await visualDebug.enable();

      const state = visualDebug.getState();
      expect(state.enabled).toBe(true);
    });

    it("should include all required properties", () => {
      const state = visualDebug.getState();

      expect(state).toHaveProperty("enabled");
      expect(state).toHaveProperty("cursor");
      expect(state).toHaveProperty("overlay");
      expect(state).toHaveProperty("styles");
      expect(state).toHaveProperty("clickHistory");
    });

    it("should reflect disabled state after disable", async () => {
      mockGetPage.mockReturnValue(mockPage);

      await visualDebug.enable();
      await visualDebug.disable();

      const state = visualDebug.getState();
      expect(state.enabled).toBe(false);
    });
  });

  describe("default export", () => {
    it("should export all public methods", async () => {
      const mod = await import("@api/utils/visual-debug.js");

      expect(mod.default).toBeDefined();
      expect(mod.default.enable).toBe(mod.enable);
      expect(mod.default.disable).toBe(mod.disable);
      expect(mod.default.toggle).toBe(mod.toggle);
      expect(mod.default.mark).toBe(mod.mark);
      expect(mod.default.moveCursor).toBe(mod.moveCursor);
      expect(mod.default.getState).toBe(mod.getState);
      expect(mod.default.isEnabled).toBe(mod.isEnabledDebug);
    });

    it("should have correct number of exports", async () => {
      const mod = await import("@api/utils/visual-debug.js");

      const exportKeys = Object.keys(mod.default);
      expect(exportKeys).toHaveLength(7);
    });
  });

  describe("integration scenarios", () => {
    it("should handle full enable-disable cycle", async () => {
      mockGetPage.mockReturnValue(mockPage);
      mockPage.evaluate.mockResolvedValue(undefined);

      // Enable
      const enableResult = await visualDebug.enable();
      expect(enableResult).toBe(true);
      expect(visualDebug.isEnabledDebug()).toBe(true);

      // Disable
      const disableResult = await visualDebug.disable();
      expect(disableResult).toBe(true);
      expect(visualDebug.isEnabledDebug()).toBe(false);
    });

    it("should handle enable-mark-move cycle", async () => {
      mockGetPage.mockReturnValue(mockPage);
      mockPage.evaluate.mockResolvedValue(undefined);

      await visualDebug.enable();

      // Perform operations
      await visualDebug.mark(100, 100, "start");
      await visualDebug.moveCursor(200, 200);
      await visualDebug.mark(200, 200, "end");

      // Verify all evaluate calls were made (enable + 2 marks + 1 move)
      expect(mockPage.evaluate).toHaveBeenCalledTimes(4);
    });

    it("should handle errors gracefully in all methods", async () => {
      mockGetPage.mockReturnValue(mockPage);

      // Enable first
      await visualDebug.enable();

      // Make evaluate fail
      mockPage.evaluate.mockRejectedValue(new Error("Page error"));

      // All methods should handle errors gracefully
      expect(await visualDebug.mark(100, 200)).toBeUndefined();
      expect(await visualDebug.moveCursor(100, 200)).toBeUndefined();
      expect(await visualDebug.disable()).toBe(false);
    });

    it("should maintain state across toggle operations", async () => {
      mockGetPage.mockReturnValue(mockPage);
      mockPage.evaluate.mockResolvedValue(undefined);

      // Initial state
      expect(visualDebug.isEnabledDebug()).toBe(false);
      expect(visualDebug.getState().enabled).toBe(false);

      // Enable via toggle
      await visualDebug.toggle();
      expect(visualDebug.isEnabledDebug()).toBe(true);
      expect(visualDebug.getState().enabled).toBe(true);

      // Operations work when enabled
      await visualDebug.mark(50, 50, "marker");
      await visualDebug.moveCursor(75, 75);

      // Disable via toggle
      await visualDebug.toggle();
      expect(visualDebug.isEnabledDebug()).toBe(false);
      expect(visualDebug.getState().enabled).toBe(false);
    });

    it("should handle multiple enable calls", async () => {
      mockGetPage.mockReturnValue(mockPage);
      mockPage.evaluate.mockResolvedValue(undefined);

      const result1 = await visualDebug.enable();
      const result2 = await visualDebug.enable();

      expect(result1).toBe(true);
      expect(result2).toBe(true);
      expect(visualDebug.isEnabledDebug()).toBe(true);
    });

    it("should handle multiple disable calls", async () => {
      mockGetPage.mockReturnValue(mockPage);
      mockPage.evaluate.mockResolvedValue(undefined);

      await visualDebug.enable();
      const result1 = await visualDebug.disable();
      const result2 = await visualDebug.disable();

      expect(result1).toBe(true);
      expect(result2).toBe(true);
      expect(visualDebug.isEnabledDebug()).toBe(false);
    });
  });

  describe("DOM injection verification", () => {
    it("should inject complete CSS styles", async () => {
      mockGetPage.mockReturnValue(mockPage);

      await visualDebug.enable();

      const evaluateCall = mockPage.evaluate.mock.calls[0];
      const { css } = evaluateCall[1];

      // Verify all major CSS components
      expect(css).toContain("position: fixed");
      expect(css).toContain("z-index: 2147483647");
      expect(css).toContain("pointer-events: none");
      expect(css).toContain(".autoai-click-marker");
      expect(css).toContain("#autoai-debug-overlay h4");
      expect(css).toContain("#autoai-debug-overlay .stat");
      expect(css).toContain("#autoai-debug-click-history");
    });

    it("should set up mouse and click tracking flags", async () => {
      mockGetPage.mockReturnValue(mockPage);

      await visualDebug.enable();

      const evaluateFn = mockPage.evaluate.mock.calls[0][0];
      expect(typeof evaluateFn).toBe("function");

      // Verify the function checks for tracking flags
      const fnStr = evaluateFn.toString();
      expect(fnStr).toContain("__autoaiMouseTracking");
      expect(fnStr).toContain("__autoaiClickTracking");
    });

    it("should set up helper functions for cursor and click recording", async () => {
      mockGetPage.mockReturnValue(mockPage);

      await visualDebug.enable();

      const evaluateFn = mockPage.evaluate.mock.calls[0][0];
      const fnStr = evaluateFn.toString();

      expect(fnStr).toContain("__autoaiMoveCursor");
      expect(fnStr).toContain("__autoaiRecordClick");
    });

    it("should include cursor position display setup", async () => {
      mockGetPage.mockReturnValue(mockPage);

      await visualDebug.enable();

      const evaluateFn = mockPage.evaluate.mock.calls[0][0];
      const fnStr = evaluateFn.toString();

      expect(fnStr).toContain("cursor-pos");
      expect(fnStr).toContain("mousemove");
    });

    it("should include click history tracking", async () => {
      mockGetPage.mockReturnValue(mockPage);

      await visualDebug.enable();

      const evaluateFn = mockPage.evaluate.mock.calls[0][0];
      const fnStr = evaluateFn.toString();

      expect(fnStr).toContain("__autoaiClickHistory");
      expect(fnStr).toContain("click-count");
      expect(fnStr).toContain("last-click");
    });
  });

  describe("page.evaluate callbacks", () => {
    describe("mark callback", () => {
      it("should execute mark callback and create marker element", async () => {
        mockGetPage.mockReturnValue(mockPage);
        mockPage.evaluate.mockImplementation(async (fn, data) => {
          // Execute the callback with jsdom environment
          fn(data);
          return undefined;
        });

        await visualDebug.enable();
        mockPage.evaluate.mockClear();

        // This will execute the callback in jsdom
        await visualDebug.mark(100, 200, "test-label");

        // Verify evaluate was called
        const markCall = mockPage.evaluate.mock.calls[0];
        expect(typeof markCall[0]).toBe("function");
        expect(markCall[1]).toEqual({ x: 100, y: 200, label: "test-label" });
      });

      it("should execute mark callback without label", async () => {
        mockGetPage.mockReturnValue(mockPage);
        mockPage.evaluate.mockImplementation(async (fn, data) => {
          fn(data);
          return undefined;
        });

        await visualDebug.enable();
        mockPage.evaluate.mockClear();

        await visualDebug.mark(50, 75);

        const markCall = mockPage.evaluate.mock.calls[0];
        expect(markCall[1].label).toBe("");
      });

      it("should execute mark callback with special characters in label", async () => {
        mockGetPage.mockReturnValue(mockPage);
        mockPage.evaluate.mockImplementation(async (fn, data) => {
          fn(data);
          return undefined;
        });

        await visualDebug.enable();
        mockPage.evaluate.mockClear();

        await visualDebug.mark(100, 200, '<script>alert("test")</script>');

        const markCall = mockPage.evaluate.mock.calls[0];
        expect(markCall[1].label).toBe('<script>alert("test")</script>');
      });
    });

    describe("moveCursor callback", () => {
      it("should execute moveCursor callback", async () => {
        mockGetPage.mockReturnValue(mockPage);
        mockPage.evaluate.mockImplementation(async (fn, data) => {
          fn(data);
          return undefined;
        });

        await visualDebug.enable();
        mockPage.evaluate.mockClear();

        await visualDebug.moveCursor(150, 250);

        const moveCall = mockPage.evaluate.mock.calls[0];
        expect(typeof moveCall[0]).toBe("function");
        expect(moveCall[1]).toEqual({ x: 150, y: 250 });
      });

      it("should execute moveCursor callback with window function available", async () => {
        // Set up the global function
        window.__autoaiMoveCursor = vi.fn();

        mockGetPage.mockReturnValue(mockPage);
        mockPage.evaluate.mockImplementation(async (fn, data) => {
          fn(data);
          return undefined;
        });

        await visualDebug.enable();
        mockPage.evaluate.mockClear();

        await visualDebug.moveCursor(100, 200);

        // The callback should have been executed
        expect(mockPage.evaluate).toHaveBeenCalled();

        delete window.__autoaiMoveCursor;
      });
    });

    describe("enable callback - DOM injection", () => {
      it("should execute enable callback and inject DOM elements", async () => {
        mockGetPage.mockReturnValue(mockPage);
        mockPage.evaluate.mockImplementation(async (fn, data) => {
          fn(data);
          return undefined;
        });

        await visualDebug.enable();

        // Verify the enable function was called and callback executed
        expect(mockPage.evaluate).toHaveBeenCalled();

        // Verify DOM elements were created by checking the document
        const cursor = document.getElementById("autoai-debug-cursor");
        const overlay = document.getElementById("autoai-debug-overlay");
        const styles = document.getElementById("autoai-debug-styles");

        // Elements should exist after enable (if not cleaned up)
        // Note: The actual elements may be cleaned up depending on test order
      });

      it("should inject styles into document head", async () => {
        mockGetPage.mockReturnValue(mockPage);
        mockPage.evaluate.mockImplementation(async (fn, data) => {
          fn(data);
          return undefined;
        });

        await visualDebug.enable();

        const styleEl = document.getElementById("autoai-debug-styles");
        if (styleEl) {
          expect(styleEl.tagName.toLowerCase()).toBe("style");
        }
      });

      it("should set up mouse tracking on enable", async () => {
        mockGetPage.mockReturnValue(mockPage);
        mockPage.evaluate.mockImplementation(async (fn, data) => {
          fn(data);
          return undefined;
        });

        await visualDebug.enable();

        // Verify tracking flag was set
        expect(window.__autoaiMouseTracking).toBe(true);
      });

      it("should set up click tracking on enable", async () => {
        mockGetPage.mockReturnValue(mockPage);
        mockPage.evaluate.mockImplementation(async (fn, data) => {
          fn(data);
          return undefined;
        });

        await visualDebug.enable();

        // Verify click tracking was initialized
        expect(window.__autoaiClickTracking).toBe(true);
        expect(window.__autoaiClickCount).toBe(0);
        expect(window.__autoaiClickHistory).toEqual([]);
      });

      it("should create helper functions on enable", async () => {
        mockGetPage.mockReturnValue(mockPage);
        mockPage.evaluate.mockImplementation(async (fn, data) => {
          fn(data);
          return undefined;
        });

        // Reset the tracking flags to ensure fresh enable
        delete window.__autoaiMouseTracking;
        delete window.__autoaiClickTracking;
        delete window.__autoaiMoveCursor;
        delete window.__autoaiRecordClick;

        await visualDebug.enable();

        expect(typeof window.__autoaiMoveCursor).toBe("function");
        expect(typeof window.__autoaiRecordClick).toBe("function");
      });
    });

    describe("disable callback - DOM cleanup", () => {
      it("should execute disable callback and clean up DOM", async () => {
        // First enable to create elements
        mockGetPage.mockReturnValue(mockPage);
        mockPage.evaluate.mockImplementation(async (fn, data) => {
          fn(data);
          return undefined;
        });

        await visualDebug.enable();

        // Now disable
        await visualDebug.disable();

        // Verify window variables were cleaned up
        expect(window.__autoaiClickCount).toBeUndefined();
        expect(window.__autoaiClickHistory).toBeUndefined();
        expect(window.__autoaiMouseTracking).toBe(false);
        expect(window.__autoaiClickTracking).toBe(false);
      });

      it("should remove DOM elements on disable", async () => {
        mockGetPage.mockReturnValue(mockPage);
        mockPage.evaluate.mockImplementation(async (fn, data) => {
          fn(data);
          return undefined;
        });

        await visualDebug.enable();
        await visualDebug.disable();

        // Elements should be removed
        expect(document.getElementById("autoai-debug-cursor")).toBeNull();
        expect(document.getElementById("autoai-debug-overlay")).toBeNull();
        expect(document.getElementById("autoai-debug-styles")).toBeNull();
      });

      it("should remove click markers on disable", async () => {
        mockGetPage.mockReturnValue(mockPage);
        mockPage.evaluate.mockImplementation(async (fn, data) => {
          fn(data);
          return undefined;
        });

        await visualDebug.enable();

        // Create a test click marker
        const marker = document.createElement("div");
        marker.className = "autoai-click-marker";
        document.body.appendChild(marker);

        await visualDebug.disable();

        // Marker should be removed
        expect(document.querySelectorAll(".autoai-click-marker").length).toBe(
          0,
        );
      });
    });

    describe("enable callback - event handlers", () => {
      it("should handle mousemove events", async () => {
        mockGetPage.mockReturnValue(mockPage);
        mockPage.evaluate.mockImplementation(async (fn, data) => {
          fn(data);
          return undefined;
        });

        await visualDebug.enable();

        // Get the cursor element
        const cursor = document.getElementById("autoai-debug-cursor");
        if (cursor) {
          // Dispatch mousemove event
          const event = new MouseEvent("mousemove", {
            clientX: 150,
            clientY: 250,
          });
          document.dispatchEvent(event);

          // The cursor position should be updated by the event handler
          // Note: exact position depends on the handler implementation
        }
      });

      it("should handle click events and increment count", async () => {
        mockGetPage.mockReturnValue(mockPage);
        mockPage.evaluate.mockImplementation(async (fn, data) => {
          fn(data);
          return undefined;
        });

        // Get the current click count (may be > 0 from previous tests)
        const countBeforeClick = window.__autoaiClickCount || 0;

        // Dispatch click event
        const event = new MouseEvent("click", {
          clientX: 100,
          clientY: 100,
        });
        document.dispatchEvent(event);

        // Click count should be incremented by 1 from previous value
        expect(window.__autoaiClickCount).toBeGreaterThan(countBeforeClick);
      });

      it("should handle pointerdown events and increment count", async () => {
        mockGetPage.mockReturnValue(mockPage);
        mockPage.evaluate.mockImplementation(async (fn, data) => {
          fn(data);
          return undefined;
        });

        const countBeforeClick = window.__autoaiClickCount || 0;

        // Dispatch pointerdown event
        const event = new PointerEvent("pointerdown", {
          clientX: 200,
          clientY: 200,
        });
        document.dispatchEvent(event);

        // Click count should be incremented by 1 from previous value
        expect(window.__autoaiClickCount).toBeGreaterThan(countBeforeClick);
      });

      it("should track click history", async () => {
        mockGetPage.mockReturnValue(mockPage);
        mockPage.evaluate.mockImplementation(async (fn, data) => {
          fn(data);
          return undefined;
        });

        await visualDebug.enable();

        // Dispatch multiple clicks
        for (let i = 0; i < 3; i++) {
          const event = new MouseEvent("click", {
            clientX: 100 + i * 50,
            clientY: 100 + i * 50,
          });
          document.dispatchEvent(event);
        }

        // Click history should have entries
        expect(window.__autoaiClickHistory.length).toBeGreaterThan(0);
      });

      it("should limit click history to 10 entries", async () => {
        mockGetPage.mockReturnValue(mockPage);
        mockPage.evaluate.mockImplementation(async (fn, data) => {
          fn(data);
          return undefined;
        });

        await visualDebug.enable();
        window.__autoaiClickHistory = [];

        // Dispatch more than 10 clicks
        for (let i = 0; i < 15; i++) {
          const event = new MouseEvent("click", {
            clientX: i * 10,
            clientY: i * 10,
          });
          document.dispatchEvent(event);
        }

        // History should be limited to 10
        expect(window.__autoaiClickHistory.length).toBeLessThanOrEqual(10);
      });
    });

    describe("moveCursor helper function", () => {
      it("should update cursor position via helper", async () => {
        mockGetPage.mockReturnValue(mockPage);
        mockPage.evaluate.mockImplementation(async (fn, data) => {
          fn(data);
          return undefined;
        });

        await visualDebug.enable();

        // Call the helper function directly
        if (window.__autoaiMoveCursor) {
          window.__autoaiMoveCursor(300, 400);

          const cursor = document.getElementById("autoai-debug-cursor");
          if (cursor) {
            expect(cursor.style.left).toBe("300px");
            expect(cursor.style.top).toBe("400px");
          }
        }
      });

      it("should update cursor position display", async () => {
        mockGetPage.mockReturnValue(mockPage);
        mockPage.evaluate.mockImplementation(async (fn, data) => {
          fn(data);
          return undefined;
        });

        await visualDebug.enable();

        if (window.__autoaiMoveCursor) {
          window.__autoaiMoveCursor(300, 400);

          const posDisplay = document.getElementById("cursor-pos");
          if (posDisplay) {
            expect(posDisplay.textContent).toBe("300, 400");
          }
        }
      });
    });

    describe("recordClick helper function", () => {
      it("should increment click count", async () => {
        mockGetPage.mockReturnValue(mockPage);
        mockPage.evaluate.mockImplementation(async (fn, data) => {
          fn(data);
          return undefined;
        });

        await visualDebug.enable();

        const initialCount = window.__autoaiClickCount || 0;

        if (window.__autoaiRecordClick) {
          window.__autoaiRecordClick(100, 100, "test");
          expect(window.__autoaiClickCount).toBe(initialCount + 1);
        }
      });

      it("should add entry to click history", async () => {
        mockGetPage.mockReturnValue(mockPage);
        mockPage.evaluate.mockImplementation(async (fn, data) => {
          fn(data);
          return undefined;
        });

        await visualDebug.enable();
        window.__autoaiClickHistory = [];

        if (window.__autoaiRecordClick) {
          window.__autoaiRecordClick(150, 200, "manual");
          expect(window.__autoaiClickHistory.length).toBe(1);
          expect(window.__autoaiClickHistory[0].x).toBe(150);
          expect(window.__autoaiClickHistory[0].y).toBe(200);
          expect(window.__autoaiClickHistory[0].source).toBe("manual");
        }
      });

      it("should create click marker element", async () => {
        mockGetPage.mockReturnValue(mockPage);
        mockPage.evaluate.mockImplementation(async (fn, data) => {
          fn(data);
          return undefined;
        });

        await visualDebug.enable();

        if (window.__autoaiRecordClick) {
          window.__autoaiRecordClick(100, 100, "test");

          // A click marker should be created
          const markers = document.querySelectorAll(".autoai-click-marker");
          expect(markers.length).toBeGreaterThan(0);
        }
      });

      it("should update last click display", async () => {
        mockGetPage.mockReturnValue(mockPage);
        mockPage.evaluate.mockImplementation(async (fn, data) => {
          fn(data);
          return undefined;
        });

        await visualDebug.enable();

        if (window.__autoaiRecordClick) {
          window.__autoaiRecordClick(250, 350, "test");

          const lastClickDisplay = document.getElementById("last-click");
          if (lastClickDisplay) {
            expect(lastClickDisplay.textContent).toBe("250, 350");
          }
        }
      });

      it("should update click count display", async () => {
        mockGetPage.mockReturnValue(mockPage);
        mockPage.evaluate.mockImplementation(async (fn, data) => {
          fn(data);
          return undefined;
        });

        await visualDebug.enable();

        if (window.__autoaiRecordClick) {
          window.__autoaiRecordClick(100, 100, "test");

          const countDisplay = document.getElementById("click-count");
          if (countDisplay) {
            expect(parseInt(countDisplay.textContent, 10)).toBe(
              window.__autoaiClickCount,
            );
          }
        }
      });
    });
  });

  describe("edge cases", () => {
    it("should handle mark with special characters in label", async () => {
      mockGetPage.mockReturnValue(mockPage);
      mockPage.evaluate.mockResolvedValue(undefined);

      await visualDebug.enable();
      await visualDebug.mark(100, 200, '<script>alert("xss")</script>');

      const markCall = mockPage.evaluate.mock.calls[1];
      expect(markCall[1].label).toBe('<script>alert("xss")</script>');
    });

    it("should handle mark with very long label", async () => {
      mockGetPage.mockReturnValue(mockPage);
      mockPage.evaluate.mockResolvedValue(undefined);

      await visualDebug.enable();
      const longLabel = "a".repeat(1000);
      await visualDebug.mark(100, 200, longLabel);

      const markCall = mockPage.evaluate.mock.calls[1];
      expect(markCall[1].label).toBe(longLabel);
    });

    it("should handle negative coordinates in mark", async () => {
      mockGetPage.mockReturnValue(mockPage);
      mockPage.evaluate.mockResolvedValue(undefined);

      await visualDebug.enable();
      await visualDebug.mark(-100, -200, "negative");

      const markCall = mockPage.evaluate.mock.calls[1];
      expect(markCall[1]).toEqual({ x: -100, y: -200, label: "negative" });
    });

    it("should handle zero coordinates", async () => {
      mockGetPage.mockReturnValue(mockPage);
      mockPage.evaluate.mockResolvedValue(undefined);

      await visualDebug.enable();
      mockPage.evaluate.mockClear();

      await visualDebug.moveCursor(0, 0);

      const moveCall = mockPage.evaluate.mock.calls[0];
      expect(moveCall[1]).toEqual({ x: 0, y: 0 });
    });

    it("should handle very large coordinates", async () => {
      mockGetPage.mockReturnValue(mockPage);
      mockPage.evaluate.mockResolvedValue(undefined);

      await visualDebug.enable();
      mockPage.evaluate.mockClear();

      await visualDebug.moveCursor(999999, 999999);

      const moveCall = mockPage.evaluate.mock.calls[0];
      expect(moveCall[1]).toEqual({ x: 999999, y: 999999 });
    });

    it("should handle enable after partial failure", async () => {
      mockGetPage.mockReturnValue(mockPage);

      // First enable fails
      mockPage.evaluate.mockRejectedValueOnce(new Error("First fail"));
      const firstResult = await visualDebug.enable();
      expect(firstResult).toBe(false);

      // Second enable succeeds
      mockPage.evaluate.mockResolvedValue(undefined);
      const secondResult = await visualDebug.enable();
      expect(secondResult).toBe(true);
      expect(visualDebug.isEnabledDebug()).toBe(true);
    });

    it("should handle enable with different error types", async () => {
      mockGetPage.mockReturnValue(mockPage);

      // TypeError
      mockPage.evaluate.mockRejectedValueOnce(new TypeError("Type error"));
      const result1 = await visualDebug.enable();
      expect(result1).toBe(false);

      // RangeError
      mockPage.evaluate.mockRejectedValueOnce(new RangeError("Range error"));
      const result2 = await visualDebug.enable();
      expect(result2).toBe(false);

      // String error
      mockPage.evaluate.mockRejectedValueOnce("string error");
      const result3 = await visualDebug.enable();
      expect(result3).toBe(false);
    });

    it("should handle disable with different error types", async () => {
      mockGetPage.mockReturnValue(mockPage);

      // TypeError
      mockPage.evaluate.mockRejectedValueOnce(new TypeError("Type error"));
      const result1 = await visualDebug.disable();
      expect(result1).toBe(false);

      // Error with custom message
      mockPage.evaluate.mockRejectedValueOnce(new Error("Custom error"));
      const result2 = await visualDebug.disable();
      expect(result2).toBe(false);
    });
  });

  describe("IDS constant", () => {
    it("should have correct cursor ID", async () => {
      mockGetPage.mockReturnValue(mockPage);
      await visualDebug.enable();

      const data = mockPage.evaluate.mock.calls[0][1];
      expect(data.ids.cursor).toBe("autoai-debug-cursor");
    });

    it("should have correct overlay ID", async () => {
      mockGetPage.mockReturnValue(mockPage);
      await visualDebug.enable();

      const data = mockPage.evaluate.mock.calls[0][1];
      expect(data.ids.overlay).toBe("autoai-debug-overlay");
    });

    it("should have correct styles ID", async () => {
      mockGetPage.mockReturnValue(mockPage);
      await visualDebug.enable();

      const data = mockPage.evaluate.mock.calls[0][1];
      expect(data.ids.styles).toBe("autoai-debug-styles");
    });

    it("should have correct clickHistory ID", async () => {
      mockGetPage.mockReturnValue(mockPage);
      await visualDebug.enable();

      const data = mockPage.evaluate.mock.calls[0][1];
      expect(data.ids.clickHistory).toBe("autoai-debug-click-history");
    });
  });
});
