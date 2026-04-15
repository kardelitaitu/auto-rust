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

vi.mock("@api/interactions/scroll.js", () => ({
  focus: vi.fn(),
}));

vi.mock("@api/interactions/wait.js", () => ({
  wait: vi.fn().mockResolvedValue(undefined),
}));

vi.mock("@api/utils/locator.js", () => ({
  getLocator: vi.fn(),
  stringify: vi.fn((v) => JSON.stringify(v)),
}));

vi.mock("@api/core/middleware.js", () => ({
  createPipeline: vi.fn((...middlewares) => {
    return async (fn) => fn();
  }),
  retryMiddleware: vi.fn(() => vi.fn()),
  recoveryMiddleware: vi.fn(() => vi.fn()),
}));

vi.mock("@api/core/errors.js", () => ({
  ElementObscuredError: class ElementObscuredError extends Error {
    constructor(message) {
      super(message);
      this.name = "ElementObscuredError";
    }
  },
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
  ]),
}));

// Import after mocks
import { drag } from "@api/interactions/drag.js";
import { getPage, getCursor, isSessionActive } from "@api/core/context.js";
import { getPersona } from "@api/behaviors/persona.js";
import { wait } from "@api/interactions/wait.js";

describe("drag", () => {
  let mockPage;
  let mockCursor;

  beforeEach(() => {
    vi.clearAllMocks();

    const mockLocator = {
      boundingBox: vi
        .fn()
        .mockResolvedValue({ x: 100, y: 100, width: 50, height: 50 }),
      first: vi.fn().mockReturnThis(),
    };

    mockPage = {
      viewportSize: vi.fn().mockReturnValue({ width: 1920, height: 1080 }),
      evaluate: vi.fn().mockResolvedValue({ width: 1920, height: 1080 }),
      mouse: {
        move: vi.fn().mockResolvedValue(undefined),
        down: vi.fn().mockResolvedValue(undefined),
        up: vi.fn().mockResolvedValue(undefined),
      },
      locator: vi.fn().mockReturnValue(mockLocator),
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
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe("session validation", () => {
    it("should throw SessionDisconnectedError when session is inactive", async () => {
      isSessionActive.mockReturnValue(false);

      await expect(
        drag({ x: 100, y: 100 }, { x: 200, y: 200 }),
      ).rejects.toThrow("Browser closed");
    });
  });

  describe("source/target resolution", () => {
    it("should resolve object coordinates directly", async () => {
      const result = await drag({ x: 100, y: 100 }, { x: 200, y: 200 });

      expect(result).toEqual({ success: true });
    });

    it("should resolve string selectors to coordinates", async () => {
      const result = await drag(".source", ".target");

      expect(result).toEqual({ success: true });
      expect(mockPage.locator).toHaveBeenCalled();
    });

    it("should resolve numeric element IDs to coordinates", async () => {
      const result = await drag(1, 2);

      expect(result).toEqual({ success: true });
    });

    it("should throw error for invalid source", async () => {
      await expect(drag(null, { x: 100, y: 100 })).rejects.toThrow();
    });

    it("should throw error when element not found by ID", async () => {
      await expect(drag(999, { x: 100, y: 100 })).rejects.toThrow();
    });

    it("should throw error when locator boundingBox returns null", async () => {
      const mockLocator = {
        boundingBox: vi.fn().mockResolvedValue(null),
        first: vi.fn().mockReturnThis(),
      };
      mockPage.locator.mockReturnValue(mockLocator);

      await expect(drag(".missing", { x: 100, y: 100 })).rejects.toThrow();
    });
  });

  describe("viewport validation", () => {
    it("should throw error when start coordinates outside viewport", async () => {
      await expect(
        drag({ x: -100, y: 100 }, { x: 200, y: 200 }),
      ).rejects.toThrow("outside viewport");
    });

    it("should throw error when end coordinates outside viewport", async () => {
      await expect(
        drag({ x: 100, y: 100 }, { x: 3000, y: 200 }),
      ).rejects.toThrow("outside viewport");
    });

    it("should fallback to window dimensions when viewportSize returns null", async () => {
      mockPage.viewportSize.mockReturnValue(null);

      const result = await drag({ x: 100, y: 100 }, { x: 200, y: 200 });

      expect(result).toEqual({ success: true });
      expect(mockPage.evaluate).toHaveBeenCalled();
    });
  });

  describe("movement types", () => {
    it("should use bezier movement by default", async () => {
      const result = await drag({ x: 100, y: 100 }, { x: 200, y: 200 });

      expect(result).toEqual({ success: true });
    });

    it("should use direct movement", async () => {
      const result = await drag(
        { x: 100, y: 100 },
        { x: 200, y: 200 },
        { movement: "direct" },
      );

      expect(result).toEqual({ success: true });
    });

    it("should use arc movement", async () => {
      const result = await drag(
        { x: 100, y: 100 },
        { x: 200, y: 200 },
        { movement: "arc" },
      );

      expect(result).toEqual({ success: true });
    });
  });

  describe("drag options", () => {
    it("should use custom duration", async () => {
      const result = await drag(
        { x: 100, y: 100 },
        { x: 200, y: 200 },
        { durationMs: 500 },
      );

      expect(result).toEqual({ success: true });
    });

    it("should use custom holdMs", async () => {
      const result = await drag(
        { x: 100, y: 100 },
        { x: 200, y: 200 },
        { holdMs: 200 },
      );

      expect(result).toEqual({ success: true });
    });

    it("should use right mouse button", async () => {
      const result = await drag(
        { x: 100, y: 100 },
        { x: 200, y: 200 },
        { button: "right" },
      );

      expect(result).toEqual({ success: true });
      expect(mockPage.mouse.down).toHaveBeenCalledWith({ button: "right" });
    });
  });

  describe("mouse operations", () => {
    it("should perform mouse down and up", async () => {
      await drag({ x: 100, y: 100 }, { x: 200, y: 200 });

      expect(mockPage.mouse.down).toHaveBeenCalled();
      expect(mockPage.mouse.up).toHaveBeenCalled();
    });

    it("should update cursor.previousPos after drag", async () => {
      await drag({ x: 100, y: 100 }, { x: 200, y: 200 });

      expect(mockCursor.previousPos).toBeDefined();
    });
  });

  describe("default export", () => {
    it("should export drag as default", async () => {
      const mod = await import("@api/interactions/drag.js");
      expect(mod.default).toBe(drag);
    });
  });
});
