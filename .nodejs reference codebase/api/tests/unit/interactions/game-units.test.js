import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";

// Mock dependencies
vi.mock("@api/core/context.js", () => ({
  getPage: vi.fn(),
  isSessionActive: vi.fn().mockReturnValue(true),
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
    { id: 1, selector: "#unit-1" },
    { id: 2, selector: "#unit-2" },
  ]),
}));

// Import after mocks
import * as gameUnits from "@api/interactions/game-units.js";
import { getPage, isSessionActive } from "@api/core/context.js";

describe("game-units", () => {
  let mockPage;

  beforeEach(() => {
    vi.clearAllMocks();

    const mockLocator = {
      boundingBox: vi
        .fn()
        .mockResolvedValue({ x: 100, y: 100, width: 50, height: 50 }),
      first: vi.fn().mockReturnThis(),
      nth: vi.fn().mockReturnThis(),
      count: vi.fn().mockResolvedValue(3),
    };

    mockPage = {
      mouse: {
        move: vi.fn().mockResolvedValue(undefined),
        down: vi.fn().mockResolvedValue(undefined),
        up: vi.fn().mockResolvedValue(undefined),
      },
      keyboard: {
        down: vi.fn().mockResolvedValue(undefined),
        up: vi.fn().mockResolvedValue(undefined),
        press: vi.fn().mockResolvedValue(undefined),
      },
      locator: vi.fn().mockReturnValue(mockLocator),
      waitForTimeout: vi.fn(),
      click: vi.fn().mockResolvedValue(undefined),
    };

    getPage.mockReturnValue(mockPage);
    isSessionActive.mockReturnValue(true);
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe("session validation", () => {
    it("should throw SessionDisconnectedError when session is inactive for selectUnit", async () => {
      isSessionActive.mockReturnValue(false);

      await expect(gameUnits.selectUnit(".unit")).rejects.toThrow(
        "Browser closed",
      );
    });

    it("should throw SessionDisconnectedError when session is inactive for selectByArea", async () => {
      isSessionActive.mockReturnValue(false);

      await expect(
        gameUnits.selectByArea({ x: 10, y: 10 }, { x: 100, y: 100 }),
      ).rejects.toThrow("SessionDisconnectedError");
    });

    it("should throw SessionDisconnectedError when session is inactive for selectAll", async () => {
      isSessionActive.mockReturnValue(false);

      await expect(gameUnits.selectAll()).rejects.toThrow(
        "SessionDisconnectedError",
      );
    });

    it("should throw SessionDisconnectedError when session is inactive for deselectAll", async () => {
      isSessionActive.mockReturnValue(false);

      await expect(gameUnits.deselectAll()).rejects.toThrow(
        "SessionDisconnectedError",
      );
    });

    it("should throw SessionDisconnectedError when session is inactive for getSelectedUnits", async () => {
      isSessionActive.mockReturnValue(false);

      await expect(gameUnits.getSelectedUnits()).rejects.toThrow(
        "SessionDisconnectedError",
      );
    });

    it("should throw SessionDisconnectedError when session is inactive for issueCommand", async () => {
      isSessionActive.mockReturnValue(false);

      await expect(gameUnits.issueCommand({ x: 10, y: 10 })).rejects.toThrow(
        "SessionDisconnectedError",
      );
    });

    it("should throw SessionDisconnectedError when session is inactive for selectGroup", async () => {
      isSessionActive.mockReturnValue(false);

      await expect(gameUnits.selectGroup(1)).rejects.toThrow(
        "SessionDisconnectedError",
      );
    });

    it("should throw SessionDisconnectedError when session is inactive for assignGroup", async () => {
      isSessionActive.mockReturnValue(false);

      await expect(gameUnits.assignGroup(1)).rejects.toThrow(
        "SessionDisconnectedError",
      );
    });

    it("should throw SessionDisconnectedError when session is inactive for selectIdleUnits", async () => {
      isSessionActive.mockReturnValue(false);

      await expect(gameUnits.selectIdleUnits()).rejects.toThrow(
        "SessionDisconnectedError",
      );
    });
  });

  describe("selectUnit", () => {
    it("should select unit by string selector", async () => {
      const result = await gameUnits.selectUnit(".unit");

      expect(result).toBe(true);
      expect(mockPage.mouse.move).toHaveBeenCalled();
    });

    it("should select unit by numeric ID", async () => {
      const result = await gameUnits.selectUnit(1);

      expect(result).toBe(true);
    });

    it("should select unit by coordinate object", async () => {
      const result = await gameUnits.selectUnit({ x: 100, y: 100 });

      expect(result).toBe(true);
    });

    it("should throw error for invalid input", async () => {
      await expect(gameUnits.selectUnit(null)).rejects.toThrow();
    });

    it("should throw error when element ID not found", async () => {
      await expect(gameUnits.selectUnit(999)).rejects.toThrow();
    });

    it("should throw error when locator boundingBox returns null", async () => {
      const mockLocator = {
        boundingBox: vi.fn().mockResolvedValue(null),
        first: vi.fn().mockReturnThis(),
      };
      mockPage.locator.mockReturnValue(mockLocator);

      await expect(gameUnits.selectUnit(".missing")).rejects.toThrow();
    });
  });

  describe("selectByArea", () => {
    it("should perform box selection", async () => {
      const result = await gameUnits.selectByArea(
        { x: 10, y: 10 },
        { x: 100, y: 100 },
      );

      expect(result).toBe(true);
      expect(mockPage.mouse.move).toHaveBeenCalled();
      expect(mockPage.mouse.down).toHaveBeenCalled();
      expect(mockPage.mouse.up).toHaveBeenCalled();
    });
  });

  describe("selectAll", () => {
    it("should press Ctrl+A on non-Mac", async () => {
      const originalPlatform = process.platform;
      Object.defineProperty(process, "platform", { value: "linux" });

      const result = await gameUnits.selectAll();

      expect(result).toBe(true);
      expect(mockPage.keyboard.down).toHaveBeenCalledWith("Control");
      expect(mockPage.keyboard.press).toHaveBeenCalledWith("a");

      Object.defineProperty(process, "platform", { value: originalPlatform });
    });

    it("should press Cmd+A on Mac", async () => {
      const originalPlatform = process.platform;
      Object.defineProperty(process, "platform", { value: "darwin" });

      const result = await gameUnits.selectAll();

      expect(result).toBe(true);
      expect(mockPage.keyboard.down).toHaveBeenCalledWith("Meta");

      Object.defineProperty(process, "platform", { value: originalPlatform });
    });
  });

  describe("deselectAll", () => {
    it("should click empty area at default position", async () => {
      const result = await gameUnits.deselectAll();

      expect(result).toBe(true);
      expect(mockPage.click).toHaveBeenCalled();
    });

    it("should click empty area at custom position", async () => {
      const result = await gameUnits.deselectAll({ x: 50, y: 50 });

      expect(result).toBe(true);
    });
  });

  describe("getSelectedUnits", () => {
    it("should return array of selected units", async () => {
      const result = await gameUnits.getSelectedUnits();

      expect(Array.isArray(result)).toBe(true);
    });

    it("should handle empty selection", async () => {
      mockPage.locator.mockReturnValue({
        first: vi.fn().mockReturnValue({
          boundingBox: vi.fn().mockResolvedValue(null),
        }),
        nth: vi.fn().mockReturnValue({
          boundingBox: vi.fn().mockResolvedValue(null),
        }),
        count: vi.fn().mockResolvedValue(0),
      });

      const result = await gameUnits.getSelectedUnits();

      expect(result).toEqual([]);
    });
  });

  describe("issueCommand", () => {
    it("should issue command to coordinates", async () => {
      const result = await gameUnits.issueCommand({ x: 100, y: 100 });

      expect(result).toBe(true);
      expect(mockPage.mouse.down).toHaveBeenCalledWith({ button: "right" });
    });

    it("should issue command to selector", async () => {
      const result = await gameUnits.issueCommand(".target");

      expect(result).toBe(true);
    });

    it("should throw error for invalid target", async () => {
      await expect(gameUnits.issueCommand(123)).rejects.toThrow(
        "Invalid target",
      );
    });

    it("should throw error when target selector not found", async () => {
      mockPage.locator.mockReturnValue({
        first: vi.fn().mockReturnValue({
          boundingBox: vi.fn().mockResolvedValue(null),
        }),
      });

      await expect(gameUnits.issueCommand(".missing")).rejects.toThrow(
        "Could not find",
      );
    });
  });

  describe("selectGroup", () => {
    it("should select group 1-9", async () => {
      for (let i = 1; i <= 9; i++) {
        const result = await gameUnits.selectGroup(i);
        expect(result).toBe(true);
      }
      expect(mockPage.keyboard.press).toHaveBeenCalledTimes(9);
    });

    it("should throw error for group < 1", async () => {
      await expect(gameUnits.selectGroup(0)).rejects.toThrow("between 1 and 9");
    });

    it("should throw error for group > 9", async () => {
      await expect(gameUnits.selectGroup(10)).rejects.toThrow(
        "between 1 and 9",
      );
    });
  });

  describe("assignGroup", () => {
    it("should assign group 1-9 on non-Mac", async () => {
      const originalPlatform = process.platform;
      Object.defineProperty(process, "platform", { value: "linux" });

      const result = await gameUnits.assignGroup(1);

      expect(result).toBe(true);
      expect(mockPage.keyboard.down).toHaveBeenCalledWith("Control");

      Object.defineProperty(process, "platform", { value: originalPlatform });
    });

    it("should assign group 1-9 on Mac", async () => {
      const originalPlatform = process.platform;
      Object.defineProperty(process, "platform", { value: "darwin" });

      const result = await gameUnits.assignGroup(1);

      expect(result).toBe(true);
      expect(mockPage.keyboard.down).toHaveBeenCalledWith("Meta");

      Object.defineProperty(process, "platform", { value: originalPlatform });
    });

    it("should throw error for group < 1", async () => {
      await expect(gameUnits.assignGroup(0)).rejects.toThrow("between 1 and 9");
    });

    it("should throw error for group > 9", async () => {
      await expect(gameUnits.assignGroup(10)).rejects.toThrow(
        "between 1 and 9",
      );
    });
  });

  describe("selectIdleUnits", () => {
    it("should press Tab to select idle units", async () => {
      const result = await gameUnits.selectIdleUnits();

      expect(typeof result).toBe("number");
      expect(mockPage.keyboard.press).toHaveBeenCalledWith("Tab");
    });
  });

  describe("default export", () => {
    it("should export all functions as default object", async () => {
      const mod = await import("@api/interactions/game-units.js");

      expect(mod.default).toHaveProperty("selectUnit");
      expect(mod.default).toHaveProperty("selectByArea");
      expect(mod.default).toHaveProperty("selectAll");
      expect(mod.default).toHaveProperty("deselectAll");
      expect(mod.default).toHaveProperty("getSelectedUnits");
      expect(mod.default).toHaveProperty("issueCommand");
      expect(mod.default).toHaveProperty("selectGroup");
      expect(mod.default).toHaveProperty("assignGroup");
      expect(mod.default).toHaveProperty("selectIdleUnits");
    });
  });
});
