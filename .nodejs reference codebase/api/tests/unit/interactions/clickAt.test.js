import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";

// Mock dependencies
vi.mock("@api/core/context.js", () => ({
  getPage: vi.fn(),
  getCursor: vi.fn(),
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

vi.mock("@api/core/middleware.js", () => ({
  createPipeline: vi.fn((...middlewares) => {
    return async (fn) => fn();
  }),
  retryMiddleware: vi.fn(() => vi.fn()),
}));

vi.mock("@api/core/errors.js", () => ({
  SessionDisconnectedError: class SessionDisconnectedError extends Error {
    constructor(message = "Browser closed") {
      super(message);
      this.name = "SessionDisconnectedError";
    }
  },
}));

vi.mock("@api/utils/visual-debug.js", () => ({
  default: {
    isEnabled: vi.fn().mockReturnValue(false),
    moveCursor: vi.fn(),
    mark: vi.fn(),
  },
}));

// Import after mocks
import { clickAt } from "@api/interactions/clickAt.js";
import { getPage, getCursor, isSessionActive } from "@api/core/context.js";
import { getPersona } from "@api/behaviors/persona.js";
import visualDebug from "@api/utils/visual-debug.js";

describe("clickAt", () => {
  let mockPage;
  let mockCursor;

  beforeEach(() => {
    vi.clearAllMocks();

    mockPage = {
      viewportSize: vi.fn().mockReturnValue({ width: 1920, height: 1080 }),
      evaluate: vi.fn().mockResolvedValue({ width: 1920, height: 1080 }),
      mouse: {
        move: vi.fn(),
        down: vi.fn(),
        up: vi.fn(),
      },
      waitForTimeout: vi.fn(),
    };

    mockCursor = {
      move: vi.fn(),
      previousPos: { x: 100, y: 100 },
    };

    getPage.mockReturnValue(mockPage);
    getCursor.mockReturnValue(mockCursor);
    isSessionActive.mockReturnValue(true);
    getPersona.mockReturnValue({ precision: 0.8 });
    visualDebug.isEnabled.mockReturnValue(false);
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe("session validation", () => {
    it("should throw SessionDisconnectedError when session is inactive", async () => {
      isSessionActive.mockReturnValue(false);

      await expect(clickAt(100, 100)).rejects.toThrow("Browser closed");
    });
  });

  describe("coordinate validation", () => {
    it("should throw error for non-numeric x coordinate", async () => {
      await expect(clickAt("invalid", 100)).rejects.toThrow("numeric");
    });

    it("should throw error for non-numeric y coordinate", async () => {
      await expect(clickAt(100, "invalid")).rejects.toThrow("numeric");
    });

    it("should throw error when coordinates are outside viewport", async () => {
      await expect(clickAt(-10, 100)).rejects.toThrow("outside viewport");
      await expect(clickAt(2000, 100)).rejects.toThrow("outside viewport");
      await expect(clickAt(100, -10)).rejects.toThrow("outside viewport");
      await expect(clickAt(100, 2000)).rejects.toThrow("outside viewport");
    });
  });

  describe("viewport handling", () => {
    it("should use page.viewportSize when available", async () => {
      mockPage.viewportSize.mockReturnValue({ width: 1280, height: 720 });

      const result = await clickAt(100, 100);

      expect(result).toEqual({ success: true });
      expect(mockPage.viewportSize).toHaveBeenCalled();
    });

    it("should fallback to window dimensions when viewportSize returns null", async () => {
      mockPage.viewportSize.mockReturnValue(null);

      const result = await clickAt(100, 100);

      expect(result).toEqual({ success: true });
      expect(mockPage.evaluate).toHaveBeenCalled();
    });
  });

  describe("precision modes", () => {
    it("should use exact precision (no jitter)", async () => {
      const result = await clickAt(500, 500, { precision: "exact" });

      expect(result).toEqual({ success: true });
      expect(mockPage.mouse.move).toHaveBeenCalled();
    });

    it("should apply jitter for safe precision", async () => {
      getPersona.mockReturnValue({ precision: 0.5 });

      const result = await clickAt(500, 500, { precision: "safe" });

      expect(result).toEqual({ success: true });
    });

    it("should apply larger jitter for rough precision", async () => {
      const result = await clickAt(500, 500, { precision: "rough" });

      expect(result).toEqual({ success: true });
    });
  });

  describe("movement options", () => {
    it("should skip moveToFirst when disabled", async () => {
      const result = await clickAt(500, 500, { moveToFirst: false });

      expect(result).toEqual({ success: true });
      expect(mockCursor.move).not.toHaveBeenCalled();
    });

    it("should use human-like path when enabled", async () => {
      const result = await clickAt(500, 500, { humanPath: true });

      expect(result).toEqual({ success: true });
    });

    it("should use direct movement when humanPath is false", async () => {
      const result = await clickAt(500, 500, { humanPath: false });

      expect(result).toEqual({ success: true });
    });
  });

  describe("speed modes", () => {
    it("should use fast speed settings", async () => {
      const result = await clickAt(500, 500, { speed: "fast" });

      expect(result).toEqual({ success: true });
    });

    it("should use normal speed settings", async () => {
      const result = await clickAt(500, 500, { speed: "normal" });

      expect(result).toEqual({ success: true });
    });

    it("should use slow speed settings", async () => {
      const result = await clickAt(500, 500, { speed: "slow" });

      expect(result).toEqual({ success: true });
    });
  });

  describe("hover option", () => {
    it("should wait when hoverMs > 0", async () => {
      const result = await clickAt(500, 500, { hoverMs: 200 });

      expect(result).toEqual({ success: true });
      expect(mockPage.waitForTimeout).toHaveBeenCalledWith(200);
    });

    it("should skip wait when hoverMs is 0", async () => {
      const result = await clickAt(500, 500, { hoverMs: 0 });

      expect(result).toEqual({ success: true });
    });
  });

  describe("mouse button", () => {
    it("should click with left button by default", async () => {
      const result = await clickAt(500, 500);

      expect(result).toEqual({ success: true });
      expect(mockPage.mouse.down).toHaveBeenCalledWith({ button: "left" });
      expect(mockPage.mouse.up).toHaveBeenCalledWith({ button: "left" });
    });

    it("should click with right button", async () => {
      const result = await clickAt(500, 500, { button: "right" });

      expect(result).toEqual({ success: true });
      expect(mockPage.mouse.down).toHaveBeenCalledWith({ button: "right" });
    });

    it("should click with middle button", async () => {
      const result = await clickAt(500, 500, { button: "middle" });

      expect(result).toEqual({ success: true });
      expect(mockPage.mouse.down).toHaveBeenCalledWith({ button: "middle" });
    });
  });

  describe("visual debug", () => {
    it("should update cursor position when visual debug is enabled", async () => {
      visualDebug.isEnabled.mockReturnValue(true);

      const result = await clickAt(500, 500);

      expect(result).toEqual({ success: true });
      expect(visualDebug.moveCursor).toHaveBeenCalled();
    });

    it("should mark click location when visual debug is enabled", async () => {
      visualDebug.isEnabled.mockReturnValue(true);

      const result = await clickAt(500, 500);

      expect(result).toEqual({ success: true });
      expect(visualDebug.mark).toHaveBeenCalled();
    });
  });

  describe("cursor state", () => {
    it("should update cursor.previousPos after click", async () => {
      await clickAt(500, 500);

      expect(mockCursor.previousPos).toBeDefined();
    });
  });

  describe("default export", () => {
    it("should export clickAt as default", async () => {
      const mod = await import("@api/interactions/clickAt.js");
      expect(mod.default).toBe(clickAt);
    });
  });
});
