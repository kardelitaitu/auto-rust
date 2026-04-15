/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Visual Debug Utility Tests
 * Tests the visual debugging overlay functionality
 * @module tests/unit/utils/visual-debug
 */

import { describe, it, expect } from "vitest";

describe("Visual Debug Utility", () => {
  describe("Module Exports", () => {
    it("should export enable function", async () => {
      const visualDebug = await import("../../../utils/visual-debug.js");
      expect(typeof visualDebug.enable).toBe("function");
    });

    it("should export disable function", async () => {
      const visualDebug = await import("../../../utils/visual-debug.js");
      expect(typeof visualDebug.disable).toBe("function");
    });

    it("should export toggle function", async () => {
      const visualDebug = await import("../../../utils/visual-debug.js");
      expect(typeof visualDebug.toggle).toBe("function");
    });

    it("should export mark function", async () => {
      const visualDebug = await import("../../../utils/visual-debug.js");
      expect(typeof visualDebug.mark).toBe("function");
    });

    it("should export moveCursor function", async () => {
      const visualDebug = await import("../../../utils/visual-debug.js");
      expect(typeof visualDebug.moveCursor).toBe("function");
    });

    it("should export isEnabledDebug function", async () => {
      const visualDebug = await import("../../../utils/visual-debug.js");
      expect(typeof visualDebug.isEnabledDebug).toBe("function");
    });

    it("should export getState function", async () => {
      const visualDebug = await import("../../../utils/visual-debug.js");
      expect(typeof visualDebug.getState).toBe("function");
    });
  });

  describe("isEnabledDebug()", () => {
    it("should return a boolean", async () => {
      const visualDebug = await import("../../../utils/visual-debug.js");
      const result = visualDebug.isEnabledDebug();
      expect(typeof result).toBe("boolean");
    });

    it("should return false initially", async () => {
      const visualDebug = await import("../../../utils/visual-debug.js");
      const result = visualDebug.isEnabledDebug();
      expect(result).toBe(false);
    });
  });

  describe("getState()", () => {
    it("should return state object", async () => {
      const visualDebug = await import("../../../utils/visual-debug.js");
      const state = visualDebug.getState();

      expect(state).toBeDefined();
      expect(typeof state).toBe("object");
    });

    it("should have enabled property", async () => {
      const visualDebug = await import("../../../utils/visual-debug.js");
      const state = visualDebug.getState();

      expect(state).toHaveProperty("enabled");
      expect(typeof state.enabled).toBe("boolean");
    });

    it("should have cursor element ID", async () => {
      const visualDebug = await import("../../../utils/visual-debug.js");
      const state = visualDebug.getState();

      expect(state).toHaveProperty("cursor");
      expect(typeof state.cursor).toBe("string");
    });

    it("should have overlay element ID", async () => {
      const visualDebug = await import("../../../utils/visual-debug.js");
      const state = visualDebug.getState();

      expect(state).toHaveProperty("overlay");
      expect(typeof state.overlay).toBe("string");
    });

    it("should have styles element ID", async () => {
      const visualDebug = await import("../../../utils/visual-debug.js");
      const state = visualDebug.getState();

      expect(state).toHaveProperty("styles");
      expect(typeof state.styles).toBe("string");
    });
  });

  describe("Module Structure", () => {
    it("should be importable without errors", async () => {
      const visualDebug = await import("../../../utils/visual-debug.js");
      expect(visualDebug).toBeDefined();
    });

    it("should have default export with all functions", async () => {
      const visualDebug = await import("../../../utils/visual-debug.js");
      expect(visualDebug.default).toBeDefined();
      expect(typeof visualDebug.default.enable).toBe("function");
      expect(typeof visualDebug.default.disable).toBe("function");
      expect(typeof visualDebug.default.toggle).toBe("function");
      expect(typeof visualDebug.default.mark).toBe("function");
      expect(typeof visualDebug.default.moveCursor).toBe("function");
      expect(typeof visualDebug.default.isEnabled).toBe("function");
      expect(typeof visualDebug.default.getState).toBe("function");
    });
  });

  describe("Element ID Constants", () => {
    it("should have consistent element IDs", async () => {
      const visualDebug = await import("../../../utils/visual-debug.js");
      const state = visualDebug.getState();

      // Verify IDs follow expected pattern
      expect(state.cursor).toContain("autoai-debug");
      expect(state.overlay).toContain("autoai-debug");
      expect(state.styles).toContain("autoai-debug");
    });
  });
});
