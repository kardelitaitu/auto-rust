import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";

// Mock dependencies
vi.mock("@api/core/context.js", () => ({
  getPage: vi.fn(),
  isSessionActive: vi.fn().mockReturnValue(true),
}));

vi.mock("@api/behaviors/timing.js", () => ({
  randomInRange: vi.fn((min, max) => (min + max) / 2),
}));

vi.mock("@api/behaviors/persona.js", () => ({
  getPersona: vi.fn().mockReturnValue({ speed: 1 }),
}));

vi.mock("@api/core/logger.js", () => ({
  createLogger: () => ({
    info: vi.fn(),
    debug: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
  }),
}));

vi.mock("@api/utils/math.js", () => ({
  mathUtils: {
    randomInRange: vi.fn((min, max) => (min + max) / 2),
    gaussian: vi.fn((mean, stddev, min, max) => mean),
  },
}));

vi.mock("@api/core/errors.js", () => ({
  SessionDisconnectedError: class SessionDisconnectedError extends Error {
    constructor(message = "Browser closed") {
      super(message);
      this.name = "SessionDisconnectedError";
    }
  },
}));

// Import after mocks
import * as keys from "@api/interactions/keys.js";
import { getPage, isSessionActive } from "@api/core/context.js";
import { getPersona } from "@api/behaviors/persona.js";

describe("keys", () => {
  let mockPage;

  beforeEach(() => {
    vi.clearAllMocks();

    // Re-setup getPersona mock after clearAllMocks
    getPersona.mockReturnValue({ speed: 1 });

    mockPage = {
      keyboard: {
        down: vi.fn().mockResolvedValue(undefined),
        up: vi.fn().mockResolvedValue(undefined),
        press: vi.fn().mockResolvedValue(undefined),
        type: vi.fn().mockResolvedValue(undefined),
      },
      waitForTimeout: vi.fn(),
    };

    getPage.mockReturnValue(mockPage);
    isSessionActive.mockReturnValue(true);
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe("session validation", () => {
    it("should throw SessionDisconnectedError when session is inactive for press", async () => {
      isSessionActive.mockReturnValue(false);

      await expect(keys.press("a")).rejects.toThrow("Browser closed");
    });

    it("should throw SessionDisconnectedError when session is inactive for typeText", async () => {
      isSessionActive.mockReturnValue(false);

      await expect(keys.typeText("hello")).rejects.toThrow("Browser closed");
    });

    it("should throw SessionDisconnectedError when session is inactive for hold", async () => {
      isSessionActive.mockReturnValue(false);

      await expect(keys.hold("a")).rejects.toThrow("Browser closed");
    });
  });

  describe("press", () => {
    it("should press a single key", async () => {
      const result = await keys.press("a");

      expect(result).toEqual({ success: true });
      expect(mockPage.keyboard.down).toHaveBeenCalledWith("a");
      expect(mockPage.keyboard.up).toHaveBeenCalledWith("a");
    });

    it("should press Enter key", async () => {
      const result = await keys.press("Enter");

      expect(result).toEqual({ success: true });
    });

    it("should press Escape key", async () => {
      const result = await keys.press("Escape");

      expect(result).toEqual({ success: true });
    });

    it("should press key with modifier array", async () => {
      const result = await keys.press("s", ["ctrl"]);

      expect(result).toEqual({ success: true });
      expect(mockPage.keyboard.down).toHaveBeenCalledWith("Control");
    });

    it("should press key with multiple modifiers", async () => {
      const result = await keys.press("c", ["ctrl", "shift"]);

      expect(result).toEqual({ success: true });
      expect(mockPage.keyboard.down).toHaveBeenCalledWith("Control");
      expect(mockPage.keyboard.down).toHaveBeenCalledWith("Shift");
    });

    it("should press key with options object", async () => {
      const result = await keys.press("a", { repeat: 2 });

      expect(result).toEqual({ success: true });
    });

    it("should press key with delay option", async () => {
      const result = await keys.press("a", { delay: 100 });

      expect(result).toEqual({ success: true });
    });

    it("should press key with downAndUp false", async () => {
      const result = await keys.press("a", { downAndUp: false });

      expect(result).toEqual({ success: true });
      expect(mockPage.keyboard.press).toHaveBeenCalledWith("a");
    });

    it("should press key chord", async () => {
      const result = await keys.press(["w", "a", "s", "d"]);

      expect(result).toEqual({ success: true });
    });

    it("should press key chord with delay", async () => {
      const result = await keys.press(["w", "a", "s", "d"], { delay: 50 });

      expect(result).toEqual({ success: true });
    });

    it("should normalize modifier names", async () => {
      const result = await keys.press("s", ["control"]);

      expect(result).toEqual({ success: true });
      expect(mockPage.keyboard.down).toHaveBeenCalledWith("Control");
    });

    it("should normalize cmd to Meta", async () => {
      const result = await keys.press("v", ["cmd"]);

      expect(result).toEqual({ success: true });
      expect(mockPage.keyboard.down).toHaveBeenCalledWith("Meta");
    });

    it("should normalize command to Meta", async () => {
      const result = await keys.press("v", ["command"]);

      expect(result).toEqual({ success: true });
      expect(mockPage.keyboard.down).toHaveBeenCalledWith("Meta");
    });

    it("should normalize win to Meta", async () => {
      const result = await keys.press("v", ["win"]);

      expect(result).toEqual({ success: true });
      expect(mockPage.keyboard.down).toHaveBeenCalledWith("Meta");
    });

    it("should throw error for empty key", async () => {
      await expect(keys.press("")).rejects.toThrow("Key is required");
    });

    it("should throw error for null key", async () => {
      await expect(keys.press(null)).rejects.toThrow("Key is required");
    });

    it("should clean up modifier keys on error", async () => {
      mockPage.keyboard.down.mockRejectedValue(new Error("Test error"));

      await expect(keys.press("a", ["shift"])).rejects.toThrow("Test error");
      expect(mockPage.keyboard.up).toHaveBeenCalledWith("Shift");
    });
  });

  describe("typeText", () => {
    it("should type text", async () => {
      const result = await keys.typeText("hello");

      expect(result).toEqual({ success: true });
      expect(mockPage.keyboard.type).toHaveBeenCalled();
    });

    it("should type text with custom delay", async () => {
      const result = await keys.typeText("hello", { delay: 50 });

      expect(result).toEqual({ success: true });
    });

    it("should add extra delay after punctuation", async () => {
      const result = await keys.typeText("hello.");

      expect(result).toEqual({ success: true });
    });

    it("should handle empty string", async () => {
      const result = await keys.typeText("");

      expect(result).toEqual({ success: true });
    });
  });

  describe("hold", () => {
    it("should hold a key for duration", async () => {
      const result = await keys.hold("a", 500);

      expect(result).toEqual({ success: true });
      expect(mockPage.keyboard.down).toHaveBeenCalledWith("a");
      expect(mockPage.keyboard.up).toHaveBeenCalledWith("a");
      expect(mockPage.waitForTimeout).toHaveBeenCalledWith(500);
    });

    it("should hold with default duration", async () => {
      const result = await keys.hold("a");

      expect(result).toEqual({ success: true });
      expect(mockPage.waitForTimeout).toHaveBeenCalledWith(500);
    });

    it("should hold special keys", async () => {
      const result = await keys.hold("ArrowUp", 1000);

      expect(result).toEqual({ success: true });
    });
  });

  describe("releaseAll", () => {
    it("should release all modifier keys", async () => {
      const result = await keys.releaseAll();

      expect(result).toEqual({ success: true });
      expect(mockPage.keyboard.up).toHaveBeenCalledWith("Shift");
      expect(mockPage.keyboard.up).toHaveBeenCalledWith("Control");
      expect(mockPage.keyboard.up).toHaveBeenCalledWith("Alt");
      expect(mockPage.keyboard.up).toHaveBeenCalledWith("Meta");
    });
  });

  describe("default export", () => {
    it("should export all functions as default object", async () => {
      const mod = await import("@api/interactions/keys.js");

      expect(mod.default).toHaveProperty("press");
      expect(mod.default).toHaveProperty("typeText");
      expect(mod.default).toHaveProperty("hold");
      expect(mod.default).toHaveProperty("releaseAll");
    });
  });
});
