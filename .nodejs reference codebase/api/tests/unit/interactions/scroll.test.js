import { describe, it, expect, vi, beforeEach } from "vitest";
import * as scroll from "@api/interactions/scroll.js";

vi.mock("@api/core/context.js", () => ({
  getPage: vi.fn(() => mockPage),
  getCursor: vi.fn(() => mockCursor),
}));

vi.mock("@api/behaviors/persona.js", () => ({
  getPersona: vi.fn(() => ({ scrollSpeed: 1.0 })),
}));

vi.mock("@api/utils/math.js", () => ({
  mathUtils: {
    randomInRange: vi.fn((min, max) => (min + max) / 2),
    gaussian: vi.fn((mean, std, min, max) => mean),
  },
}));

vi.mock("@api/core/logger.js", () => ({
  createLogger: vi.fn(() => ({
    info: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
    debug: vi.fn(),
  })),
}));

vi.mock("@api/utils/config.js", () => ({
  getSettings: vi.fn(() =>
    Promise.resolve({ twitter: { timing: { globalScrollMultiplier: 1.0 } } }),
  ),
}));

vi.mock("@api/utils/locator.js", () => ({
  getLocator: vi.fn((selector) => ({
    first: () => ({
      waitFor: vi.fn().mockResolvedValue(undefined),
      evaluate: vi
        .fn()
        .mockResolvedValue({ x: 100, y: 100, width: 200, height: 100 }),
    }),
  })),
}));

vi.mock("@api/core/errors.js", () => ({
  ValidationError: class ValidationError extends Error {
    constructor(code, message, metadata, cause) {
      super(message);
      this.name = "ValidationError";
      this.code = code;
      this.metadata = metadata;
      this.cause = cause;
    }
  },
}));

const mockPage = {
  evaluate: vi.fn().mockImplementation((fn, arg) => {
    // Handle the case where arg is passed (for the smoothScroll functions)
    if (arg && typeof arg === "object") {
      return Promise.resolve(undefined);
    }
    const fnStr = fn.toString();
    // Viewport density check
    if (fnStr.includes("querySelectorAll")) {
      return Promise.resolve({ pCount: 5, imgCount: 2, textLength: 500 });
    }
    // Scroll state (scrollY + viewportHeight)
    if (
      fnStr.includes("scrollY") &&
      fnStr.includes("innerHeight") &&
      !fnStr.includes("scrollBy")
    ) {
      return Promise.resolve({ scrollY: 100, viewportHeight: 720 });
    }
    // Viewport dimensions only
    if (
      fnStr.includes("innerWidth") &&
      fnStr.includes("innerHeight") &&
      !fnStr.includes("scrollY")
    ) {
      return Promise.resolve({ width: 1280, height: 720 });
    }
    // Scroll height
    if (fnStr.includes("scrollHeight")) {
      return Promise.resolve(5000);
    }
    // Single scrollY
    if (fnStr.includes("scrollY") && !fnStr.includes("innerHeight")) {
      return Promise.resolve(100);
    }
    // Bounding client rect
    if (fnStr.includes("getBoundingClientRect")) {
      return Promise.resolve({ x: 100, y: 100, width: 200, height: 100 });
    }
    return Promise.resolve(undefined);
  }),
  viewportSize: vi.fn(() => ({ width: 1280, height: 720 })),
  scrollBy: vi.fn(),
  scrollTo: vi.fn(),
  bringToFront: vi.fn().mockResolvedValue(undefined),
};

const mockCursor = {
  move: vi.fn().mockResolvedValue(undefined),
};

describe("scroll.js", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockPage.evaluate.mockResolvedValue(100);
    mockPage.viewportSize.mockReturnValue({ width: 1280, height: 720 });
  });

  describe("scroll()", () => {
    it("should throw ValidationError for non-number input", async () => {
      await expect(scroll.scroll("invalid")).rejects.toThrow(
        "requires a finite number",
      );
    });

    it("should throw ValidationError for NaN", async () => {
      await expect(scroll.scroll(NaN)).rejects.toThrow(
        "requires a finite number",
      );
    });

    it("should throw ValidationError for Infinity", async () => {
      await expect(scroll.scroll(Infinity)).rejects.toThrow(
        "requires a finite number",
      );
    });

    it("should throw ValidationError for undefined", async () => {
      await expect(scroll.scroll(undefined)).rejects.toThrow(
        "requires a finite number",
      );
    });

    it("should throw ValidationError for null", async () => {
      await expect(scroll.scroll(null)).rejects.toThrow(
        "requires a finite number",
      );
    });

    it("should scroll down with positive distance", async () => {
      mockPage.evaluate.mockResolvedValue(undefined);
      await scroll.scroll(500);
      expect(mockPage.evaluate).toHaveBeenCalled();
    });

    it("should scroll up with negative distance", async () => {
      mockPage.evaluate.mockResolvedValue(undefined);
      await scroll.scroll(-300);
      expect(mockPage.evaluate).toHaveBeenCalled();
    });

    it("should scroll zero pixels (no-op)", async () => {
      await scroll.scroll(0);
      // scroll(0) is a valid no-op, should not throw
    });
  });

  describe("read()", () => {
    it("should throw ValidationError for negative pauses", async () => {
      await expect(scroll.read(".element", { pauses: -1 })).rejects.toThrow(
        "must be a non-negative number",
      );
    });

    it("should throw ValidationError for NaN pauses", async () => {
      await expect(scroll.read(".element", { pauses: NaN })).rejects.toThrow(
        "must be a non-negative number",
      );
    });

    it("should throw ValidationError for non-number pauses", async () => {
      await expect(
        scroll.read(".element", { pauses: "invalid" }),
      ).rejects.toThrow("must be a non-negative number");
    });

    it("should read with pauses=0 (no scrolling)", async () => {
      await scroll.read(".article", { pauses: 0 });
      // pauses=0 means no scroll loops execute
    });

    it("should read with minimal options", async () => {
      await scroll.read(".article", { pauses: 0 });
    });
  });

  describe("back()", () => {
    it("should scroll back with default distance", async () => {
      mockPage.evaluate.mockResolvedValue(undefined);
      await scroll.back();
      expect(mockPage.evaluate).toHaveBeenCalled();
    });

    it("should scroll back with custom distance", async () => {
      mockPage.evaluate.mockResolvedValue(undefined);
      await scroll.back(200);
      expect(mockPage.evaluate).toHaveBeenCalled();
    });
  });

  describe("focus()", () => {
    it("should focus on element", async () => {
      await scroll.focus(".element");
      expect(mockCursor.move).toHaveBeenCalled();
    });

    it("should focus with custom options", async () => {
      mockPage.evaluate.mockResolvedValue(undefined);
      await scroll.focus(".element", { randomness: 0.2, timeout: 3000 });
    });

    it("should handle missing bounding box", async () => {
      const { getLocator } = await import("@api/utils/locator.js");
      getLocator.mockReturnValue({
        first: () => ({
          waitFor: vi.fn().mockResolvedValue(undefined),
          evaluate: vi.fn().mockResolvedValue(null),
        }),
      });
      await scroll.focus(".missing");
      getLocator.mockReturnValue({
        first: () => ({
          waitFor: vi.fn().mockResolvedValue(undefined),
          evaluate: vi
            .fn()
            .mockResolvedValue({ x: 100, y: 100, width: 200, height: 100 }),
        }),
      });
    });

    it("should handle null viewport", async () => {
      mockPage.viewportSize.mockReturnValue(null);
      mockPage.evaluate.mockResolvedValue({ width: 1280, height: 720 });
      await scroll.focus(".element");
    });
  });

  describe("toTop()", () => {
    it("should scroll to top with default duration", async () => {
      mockPage.evaluate.mockResolvedValue(undefined);
      await scroll.toTop();
      expect(mockPage.evaluate).toHaveBeenCalled();
    });

    it("should scroll to top with custom duration", async () => {
      mockPage.evaluate.mockResolvedValue(undefined);
      await scroll.toTop(2000);
      expect(mockPage.evaluate).toHaveBeenCalled();
    });
  });

  describe("toBottom()", () => {
    it("should scroll to bottom", async () => {
      mockPage.evaluate.mockResolvedValue(5000);
      await scroll.toBottom();
      expect(mockPage.evaluate).toHaveBeenCalled();
    });
  });

  describe("focus2()", () => {
    it("should focus on element with absolute coordinates", async () => {
      mockPage.evaluate.mockResolvedValue({
        scrollY: 0,
        viewportHeight: 720,
      });
      const result = await scroll.focus2(".element");
      expect(result).toHaveProperty("success");
      expect(result).toHaveProperty("distance");
      expect(result).toHaveProperty("steps");
      expect(result).toHaveProperty("duration");
    });

    it("should return success false when element info is null", async () => {
      const { getLocator } = await import("@api/utils/locator.js");
      getLocator.mockReturnValue({
        first: () => ({
          waitFor: vi.fn().mockResolvedValue(undefined),
          evaluate: vi.fn().mockRejectedValue(new Error("Element not found")),
        }),
      });
      const result = await scroll.focus2(".missing");
      expect(result.success).toBe(false);
      getLocator.mockReturnValue({
        first: () => ({
          waitFor: vi.fn().mockResolvedValue(undefined),
          evaluate: vi
            .fn()
            .mockResolvedValue({ x: 100, y: 100, width: 200, height: 100 }),
        }),
      });
    });

    it("should skip scroll when already centered", async () => {
      mockPage.evaluate.mockResolvedValue({
        scrollY: 500,
        viewportHeight: 720,
      });
      const { getLocator } = await import("@api/utils/locator.js");
      getLocator.mockReturnValue({
        first: () => ({
          waitFor: vi.fn().mockResolvedValue(undefined),
          evaluate: vi.fn().mockResolvedValue({
            absoluteY: 800,
            height: 100,
            width: 200,
            viewportY: 100,
            viewportX: 100,
          }),
        }),
      });
      const result = await scroll.focus2(".element");
      expect(result.success).toBe(true);
      expect(result.steps).toBe(0);
    });

    it("should calculate steps based on distance", async () => {
      mockPage.evaluate.mockResolvedValue({
        scrollY: 0,
        viewportHeight: 720,
      });
      const { getLocator } = await import("@api/utils/locator.js");
      getLocator.mockReturnValue({
        first: () => ({
          waitFor: vi.fn().mockResolvedValue(undefined),
          evaluate: vi.fn().mockResolvedValue({
            absoluteY: 3000,
            height: 100,
            width: 200,
            viewportY: 100,
            viewportX: 100,
          }),
        }),
      });
      const result = await scroll.focus2(".element");
      expect(result.steps).toBeGreaterThan(0);
    });
  });
});
