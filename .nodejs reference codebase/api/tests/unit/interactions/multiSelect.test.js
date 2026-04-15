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
  getPersona: vi.fn().mockReturnValue({ precision: 0.8 }),
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
  },
}));

vi.mock("@api/utils/locator.js", () => ({
  getLocator: vi.fn(),
  stringify: vi.fn((v) => JSON.stringify(v)),
}));

vi.mock("@api/interactions/wait.js", () => ({
  wait: vi.fn().mockResolvedValue(undefined),
}));

vi.mock("@api/core/errors.js", () => ({
  SessionDisconnectedError: class SessionDisconnectedError extends Error {
    constructor(message = "Browser closed") {
      super(message);
      this.name = "SessionDisconnectedError";
    }
  },
}));

vi.mock("@api/core/context-state.js", () => ({
  getStateAgentElementMap: vi.fn().mockReturnValue([
    { id: 1, selector: "#element-1" },
    { id: 2, selector: "#element-2" },
    { id: 3, selector: "#element-3" },
  ]),
}));

// Import after mocks
import { multiSelect } from "@api/interactions/multiSelect.js";
import { getPage, isSessionActive } from "@api/core/context.js";
import { wait } from "@api/interactions/wait.js";

describe("multiSelect", () => {
  let mockPage;

  beforeEach(() => {
    vi.clearAllMocks();

    mockPage = {
      mouse: {
        move: vi.fn().mockResolvedValue(undefined),
        down: vi.fn().mockResolvedValue(undefined),
        up: vi.fn().mockResolvedValue(undefined),
      },
      keyboard: {
        down: vi.fn().mockResolvedValue(undefined),
        up: vi.fn().mockResolvedValue(undefined),
      },
      locator: vi.fn().mockReturnValue({
        first: vi.fn().mockReturnValue({
          boundingBox: vi
            .fn()
            .mockResolvedValue({ x: 100, y: 100, width: 50, height: 50 }),
        }),
      }),
      waitForTimeout: vi.fn(),
    };

    getPage.mockReturnValue(mockPage);
    isSessionActive.mockReturnValue(true);
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe("session validation", () => {
    it("should throw SessionDisconnectedError when session is inactive", async () => {
      isSessionActive.mockReturnValue(false);

      await expect(multiSelect([1, 2])).rejects.toThrow("Browser closed");
    });
  });

  describe("input validation", () => {
    it("should throw error for empty array", async () => {
      await expect(multiSelect([])).rejects.toThrow("non-empty array");
    });

    it("should throw error for non-array input", async () => {
      await expect(multiSelect("invalid")).rejects.toThrow("non-empty array");
    });

    it("should throw error for null input", async () => {
      await expect(multiSelect(null)).rejects.toThrow("non-empty array");
    });
  });

  describe("coordinate resolution", () => {
    it("should resolve object coordinates directly", async () => {
      const result = await multiSelect([{ x: 100, y: 100 }]);

      expect(result.success).toBe(true);
      expect(result.selected).toBe(1);
    });

    it("should resolve string selectors to coordinates", async () => {
      const result = await multiSelect([".element"]);

      expect(result.success).toBe(true);
      expect(result.selected).toBe(1);
    });

    it("should resolve numeric element IDs to coordinates", async () => {
      const result = await multiSelect([1, 2]);

      expect(result.success).toBe(true);
      expect(result.selected).toBe(2);
    });

    it("should throw error for invalid item type", async () => {
      await expect(multiSelect([123])).rejects.toThrow();
    });

    it("should throw error when element ID not found", async () => {
      await expect(multiSelect([999])).rejects.toThrow();
    });

    it("should throw error when locator returns null boundingBox", async () => {
      mockPage.locator.mockReturnValue({
        first: vi.fn().mockReturnValue({
          boundingBox: vi.fn().mockResolvedValue(null),
        }),
      });

      await expect(multiSelect([".missing"])).rejects.toThrow();
    });
  });

  describe("add mode", () => {
    it("should add items to selection", async () => {
      const result = await multiSelect(
        [
          { x: 100, y: 100 },
          { x: 200, y: 200 },
        ],
        { mode: "add" },
      );

      expect(result.success).toBe(true);
      expect(result.selected).toBe(2);
      expect(mockPage.keyboard.down).toHaveBeenCalledWith("Control");
      expect(mockPage.keyboard.up).toHaveBeenCalledWith("Control");
    });
  });

  describe("remove mode", () => {
    it("should remove items from selection", async () => {
      const result = await multiSelect([{ x: 100, y: 100 }], {
        mode: "remove",
      });

      expect(result.success).toBe(true);
      expect(result.selected).toBe(1);
      expect(mockPage.keyboard.down).toHaveBeenCalledWith("Control");
    });
  });

  describe("range mode", () => {
    it("should select range with Shift", async () => {
      const result = await multiSelect(
        [
          { x: 100, y: 100 },
          { x: 200, y: 200 },
        ],
        { mode: "range" },
      );

      expect(result.success).toBe(true);
      expect(result.selected).toBe(2);
      expect(mockPage.keyboard.down).toHaveBeenCalledWith("Shift");
      expect(mockPage.keyboard.up).toHaveBeenCalledWith("Shift");
    });

    it("should throw error for range with less than 2 items", async () => {
      await expect(
        multiSelect([{ x: 100, y: 100 }], { mode: "range" }),
      ).rejects.toThrow();
    });
  });

  describe("toggle mode", () => {
    it("should toggle selection", async () => {
      const result = await multiSelect([{ x: 100, y: 100 }], {
        mode: "toggle",
      });

      expect(result.success).toBe(true);
      expect(result.selected).toBe(1);
    });
  });

  describe("holdMs option", () => {
    it("should use custom holdMs between clicks", async () => {
      const result = await multiSelect(
        [
          { x: 100, y: 100 },
          { x: 200, y: 200 },
        ],
        { holdMs: 200 },
      );

      expect(result.success).toBe(true);
      expect(wait).toHaveBeenCalled();
    });
  });

  describe("error recovery", () => {
    it("should clean up modifier keys on error", async () => {
      mockPage.mouse.down.mockRejectedValue(new Error("Click failed"));

      await expect(
        multiSelect([{ x: 100, y: 100 }], { recovery: true }),
      ).rejects.toThrow();
    });

    it("should not recover when recovery is false", async () => {
      mockPage.mouse.down.mockRejectedValue(new Error("Click failed"));

      await expect(
        multiSelect([{ x: 100, y: 100 }], { recovery: false }),
      ).rejects.toThrow();
    });
  });

  describe("default export", () => {
    it("should export multiSelect as default", async () => {
      const mod = await import("@api/interactions/multiSelect.js");
      expect(mod.default).toBe(multiSelect);
    });
  });
});
