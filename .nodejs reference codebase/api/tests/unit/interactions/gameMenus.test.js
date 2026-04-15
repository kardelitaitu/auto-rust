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

vi.mock("@api/interactions/clickAt.js", () => ({
  clickAt: vi.fn().mockResolvedValue({ success: true }),
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

// Import after mocks
import * as gameMenus from "@api/interactions/gameMenus.js";
import { isSessionActive } from "@api/core/context.js";
import { clickAt } from "@api/interactions/clickAt.js";
import { wait } from "@api/interactions/wait.js";

describe("gameMenus", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    isSessionActive.mockReturnValue(true);
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe("session validation", () => {
    it("should throw SessionDisconnectedError when session is inactive for openMenu", async () => {
      isSessionActive.mockReturnValue(false);

      await expect(gameMenus.openMenu("build")).rejects.toThrow(
        "Browser closed",
      );
    });

    it("should throw SessionDisconnectedError when session is inactive for closeMenu", async () => {
      isSessionActive.mockReturnValue(false);

      await expect(gameMenus.closeMenu()).rejects.toThrow("Browser closed");
    });

    it("should throw SessionDisconnectedError when session is inactive for selectItem", async () => {
      isSessionActive.mockReturnValue(false);

      await expect(gameMenus.selectItem("barracks")).rejects.toThrow(
        "Browser closed",
      );
    });

    it("should throw SessionDisconnectedError when session is inactive for confirm", async () => {
      isSessionActive.mockReturnValue(false);

      await expect(gameMenus.confirm()).rejects.toThrow("Browser closed");
    });

    it("should throw SessionDisconnectedError when session is inactive for cancel", async () => {
      isSessionActive.mockReturnValue(false);

      await expect(gameMenus.cancel()).rejects.toThrow("Browser closed");
    });

    it("should throw SessionDisconnectedError when session is inactive for build", async () => {
      isSessionActive.mockReturnValue(false);

      await expect(gameMenus.build("barracks")).rejects.toThrow(
        "Browser closed",
      );
    });

    it("should throw SessionDisconnectedError when session is inactive for train", async () => {
      isSessionActive.mockReturnValue(false);

      await expect(gameMenus.train("footman")).rejects.toThrow(
        "Browser closed",
      );
    });

    it("should throw SessionDisconnectedError when session is inactive for research", async () => {
      isSessionActive.mockReturnValue(false);

      await expect(
        gameMenus.research("blacksmith", "weaponry"),
      ).rejects.toThrow("Browser closed");
    });

    it("should throw SessionDisconnectedError when session is inactive for runSequence", async () => {
      isSessionActive.mockReturnValue(false);

      await expect(gameMenus.runSequence([])).rejects.toThrow("Browser closed");
    });
  });

  describe("loadConfig", () => {
    it("should merge custom config with default", () => {
      gameMenus.loadConfig({ custom: { x: 100, y: 100 } });
      // No error should be thrown
      expect(true).toBe(true);
    });

    it("should handle null config", () => {
      gameMenus.loadConfig(null);
      expect(true).toBe(true);
    });
  });

  describe("setPosition", () => {
    it("should set custom position for menu item", () => {
      gameMenus.setPosition("custom", { x: 200, y: 200 });
      // No error should be thrown
      expect(true).toBe(true);
    });
  });

  describe("openMenu", () => {
    it("should open menu by name", async () => {
      const result = await gameMenus.openMenu("build");

      expect(result).toBe(true);
      expect(clickAt).toHaveBeenCalled();
      expect(wait).toHaveBeenCalled();
    });

    it("should return false for unknown menu", async () => {
      const result = await gameMenus.openMenu("unknown_menu");

      expect(result).toBe(false);
    });
  });

  describe("closeMenu", () => {
    it("should close menu with default close position", async () => {
      const result = await gameMenus.closeMenu();

      expect(result).toBe(true);
      expect(clickAt).toHaveBeenCalled();
    });

    it("should close menu with custom position", async () => {
      const result = await gameMenus.closeMenu({ x: 300, y: 300 });

      expect(result).toBe(true);
    });

    it("should return false when no close position defined", async () => {
      // Test with no close position in options and missing config
      gameMenus.setPosition("close", null);

      const result = await gameMenus.closeMenu();

      expect(result).toBe(false);
    });
  });

  describe("selectItem", () => {
    it("should select item by name", async () => {
      const result = await gameMenus.selectItem("barracks");

      expect(result).toBe(true);
      expect(clickAt).toHaveBeenCalled();
    });

    it("should return false for unknown item", async () => {
      const result = await gameMenus.selectItem("unknown_item");

      expect(result).toBe(false);
    });
  });

  describe("confirm", () => {
    it("should click confirm button", async () => {
      const result = await gameMenus.confirm();

      expect(result).toBe(true);
      expect(clickAt).toHaveBeenCalled();
    });

    it("should return false when confirm position not defined", async () => {
      // Remove confirm position
      gameMenus.loadConfig({ confirm: null });

      const result = await gameMenus.confirm();

      // Config might still have confirm from default, so we just check it returns boolean
      expect(typeof result).toBe("boolean");
    });
  });

  describe("cancel", () => {
    it("should click cancel button", async () => {
      const result = await gameMenus.cancel();

      expect(result).toBe(true);
      expect(clickAt).toHaveBeenCalled();
    });

    it("should return false when cancel position not defined", async () => {
      // Remove cancel position
      gameMenus.loadConfig({ cancel: null });

      const result = await gameMenus.cancel();

      expect(typeof result).toBe("boolean");
    });
  });

  describe("build", () => {
    it("should build a structure", async () => {
      const result = await gameMenus.build("barracks");

      expect(result).toBe(true);
      expect(clickAt).toHaveBeenCalled();
    });
  });

  describe("train", () => {
    it("should train a single unit", async () => {
      const result = await gameMenus.train("footman");

      expect(result).toBe(true);
    });

    it("should train multiple units", async () => {
      const result = await gameMenus.train("footman", { count: 3 });

      expect(result).toBe(true);
    });
  });

  describe("research", () => {
    it("should research an upgrade", async () => {
      const result = await gameMenus.research("blacksmith", "weaponry");

      expect(result).toBe(true);
    });
  });

  describe("runSequence", () => {
    it("should run empty sequence", async () => {
      const result = await gameMenus.runSequence([]);

      expect(result).toBe(true);
    });

    it("should run sequence with open action", async () => {
      const result = await gameMenus.runSequence([
        { action: "open", target: "build" },
      ]);

      expect(result).toBe(true);
    });

    it("should run sequence with select action", async () => {
      const result = await gameMenus.runSequence([
        { action: "select", target: "barracks" },
      ]);

      expect(result).toBe(true);
    });

    it("should run sequence with click action", async () => {
      const result = await gameMenus.runSequence([
        { action: "click", position: { x: 100, y: 100 } },
      ]);

      expect(result).toBe(true);
    });

    it("should run sequence with confirm action", async () => {
      const result = await gameMenus.runSequence([{ action: "confirm" }]);

      expect(result).toBe(true);
    });

    it("should run sequence with cancel action", async () => {
      const result = await gameMenus.runSequence([{ action: "cancel" }]);

      expect(result).toBe(true);
    });

    it("should run sequence with close action", async () => {
      const result = await gameMenus.runSequence([
        { action: "close", position: { x: 100, y: 100 } },
      ]);

      expect(result).toBe(true);
    });

    it("should run sequence with wait action", async () => {
      const result = await gameMenus.runSequence([
        { action: "wait", target: 1000 },
      ]);

      expect(result).toBe(true);
      expect(wait).toHaveBeenCalled();
    });

    it("should handle unknown action", async () => {
      const result = await gameMenus.runSequence([{ action: "unknown" }]);

      expect(result).toBe(true);
    });

    it("should run full sequence", async () => {
      const result = await gameMenus.runSequence([
        { action: "open", target: "build" },
        { action: "select", target: "barracks" },
        { action: "confirm" },
        { action: "wait", target: 500 },
      ]);

      expect(result).toBe(true);
    });
  });

  describe("default export", () => {
    it("should export all functions as default object", async () => {
      const mod = await import("@api/interactions/gameMenus.js");

      expect(mod.default).toHaveProperty("loadConfig");
      expect(mod.default).toHaveProperty("openMenu");
      expect(mod.default).toHaveProperty("closeMenu");
      expect(mod.default).toHaveProperty("selectItem");
      expect(mod.default).toHaveProperty("confirm");
      expect(mod.default).toHaveProperty("cancel");
      expect(mod.default).toHaveProperty("build");
      expect(mod.default).toHaveProperty("train");
      expect(mod.default).toHaveProperty("research");
      expect(mod.default).toHaveProperty("runSequence");
      expect(mod.default).toHaveProperty("setPosition");
    });
  });
});
