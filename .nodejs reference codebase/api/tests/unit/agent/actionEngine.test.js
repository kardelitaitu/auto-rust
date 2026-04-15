import { describe, it, expect, vi, beforeEach } from "vitest";

vi.mock("@api/core/logger.js", () => ({
  createLogger: vi.fn(() => ({
    info: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
    debug: vi.fn(),
  })),
}));

vi.mock("@api/utils/ghostCursor.js", () => ({
  GhostCursor: vi.fn().mockImplementation(() => ({
    moveWithHesitation: vi.fn().mockResolvedValue(undefined),
  })),
}));

vi.mock("@api/utils/math.js", () => ({
  mathUtils: {
    gaussian: vi.fn((mean) => mean),
    randomInRange: vi.fn((min, max) => (min + max) / 2),
  },
}));

const createMockPage = () => ({
  bringToFront: vi.fn().mockResolvedValue(undefined),
  locator: vi.fn().mockReturnValue({
    first: vi.fn().mockReturnValue({
      waitFor: vi.fn().mockResolvedValue(undefined),
      click: vi.fn().mockResolvedValue(undefined),
      fill: vi.fn().mockResolvedValue(undefined),
      boundingBox: vi
        .fn()
        .mockResolvedValue({ x: 100, y: 100, width: 200, height: 50 }),
    }),
  }),
  getByRole: vi.fn().mockReturnValue({
    first: vi.fn().mockReturnValue({
      waitFor: vi.fn().mockResolvedValue(undefined),
      click: vi.fn().mockResolvedValue(undefined),
    }),
  }),
  getByText: vi.fn().mockReturnValue({
    first: vi.fn().mockReturnValue({
      waitFor: vi.fn().mockResolvedValue(undefined),
      click: vi.fn().mockResolvedValue(undefined),
    }),
  }),
  keyboard: {
    press: vi.fn().mockResolvedValue(undefined),
    type: vi.fn().mockResolvedValue(undefined),
    down: vi.fn().mockResolvedValue(undefined),
    up: vi.fn().mockResolvedValue(undefined),
  },
  mouse: {
    click: vi.fn().mockResolvedValue(undefined),
    dblclick: vi.fn().mockResolvedValue(undefined),
    down: vi.fn().mockResolvedValue(undefined),
    up: vi.fn().mockResolvedValue(undefined),
    move: vi.fn().mockResolvedValue(undefined),
  },
  evaluate: vi.fn().mockResolvedValue(undefined),
  waitForTimeout: vi.fn().mockResolvedValue(undefined),
  goto: vi.fn().mockResolvedValue(undefined),
  screenshot: vi.fn().mockResolvedValue(Buffer.from("screenshot")),
  url: vi.fn().mockReturnValue("https://example.com"),
  viewportSize: vi.fn().mockReturnValue({ width: 1280, height: 720 }),
});

describe("actionEngine.js", () => {
  let engine;
  let mockPage;

  beforeEach(async () => {
    vi.clearAllMocks();
    const module = await import("@api/agent/actionEngine.js");
    engine = module.actionEngine;
    mockPage = createMockPage();
  });

  describe("constructor", () => {
    it("should have default timeouts", () => {
      expect(engine.timeouts.elementVisible).toBe(5000);
      expect(engine.timeouts.navigation).toBe(30000);
      expect(engine.timeouts.action).toBe(10000);
    });

    it("should have default humanization settings", () => {
      expect(engine.humanization.enabled).toBe(true);
      expect(engine.humanization.mouseMovement).toBe(true);
    });

    it("should have cursor initially null", () => {
      expect(engine.cursor).toBeNull();
    });
  });

  describe("execute()", () => {
    it("should return error for no action", async () => {
      const result = await engine.execute(mockPage, null);
      expect(result.success).toBe(false);
      expect(result.error).toBe("No action specified");
    });

    it("should return error for action without action type", async () => {
      const result = await engine.execute(mockPage, { selector: "#btn" });
      expect(result.success).toBe(false);
      expect(result.error).toBe("No action specified");
    });

    it("should execute click action", async () => {
      const result = await engine.execute(mockPage, {
        action: "click",
        selector: "#btn",
      });
      expect(result.success).toBe(true);
    });

    it("should execute clickAt action", async () => {
      const result = await engine.execute(mockPage, {
        action: "clickAt",
        x: 100,
        y: 200,
      });
      expect(result.success).toBe(true);
    });

    it("should execute clickAt with array coordinates", async () => {
      const result = await engine.execute(mockPage, {
        action: "clickAt",
        x: [100, 150],
        y: [200, 250],
      });
      expect(result.success).toBe(true);
    });

    it("should execute type action", async () => {
      const result = await engine.execute(mockPage, {
        action: "type",
        selector: "#input",
        value: "hello",
      });
      expect(result.success).toBe(true);
    });

    it("should execute press action", async () => {
      const result = await engine.execute(mockPage, {
        action: "press",
        key: "Enter",
      });
      expect(result.success).toBe(true);
    });

    it("should execute press with value", async () => {
      const result = await engine.execute(mockPage, {
        action: "press",
        value: "Escape",
      });
      expect(result.success).toBe(true);
    });

    it("should execute scroll action", async () => {
      const result = await engine.execute(mockPage, {
        action: "scroll",
        value: "down",
      });
      expect(result.success).toBe(true);
    });

    it("should execute navigate action", async () => {
      const result = await engine.execute(mockPage, {
        action: "navigate",
        value: "https://example.com",
      });
      expect(result.success).toBe(true);
    });

    it("should execute goto action", async () => {
      const result = await engine.execute(mockPage, {
        action: "goto",
        value: "https://example.com",
      });
      expect(result.success).toBe(true);
    });

    it("should execute wait action", async () => {
      const result = await engine.execute(mockPage, {
        action: "wait",
        value: "1000",
      });
      expect(result.success).toBe(true);
    });

    it("should execute delay action", async () => {
      const result = await engine.execute(mockPage, {
        action: "delay",
        value: "500",
      });
      expect(result.success).toBe(true);
    });

    it("should return done for done action", async () => {
      const result = await engine.execute(mockPage, { action: "done" });
      expect(result.success).toBe(true);
      expect(result.done).toBe(true);
    });

    it("should return error for unknown action", async () => {
      const result = await engine.execute(mockPage, { action: "unknown" });
      expect(result.success).toBe(false);
      expect(result.error).toContain("Unknown action");
    });

    it("should handle verify action", async () => {
      const result = await engine.execute(mockPage, {
        action: "verify",
        description: "Check element",
      });
      expect(result.success).toBe(true);
    });

    it("should handle rationale logging", async () => {
      const result = await engine.execute(mockPage, {
        action: "click",
        selector: "#btn",
        rationale: "Click the button",
      });
      expect(result.success).toBe(true);
    });

    it("should handle action execution error", async () => {
      mockPage.locator.mockReturnValue({
        first: vi.fn().mockReturnValue({
          waitFor: vi.fn().mockRejectedValue(new Error("Element not found")),
        }),
      });
      const result = await engine.execute(mockPage, {
        action: "click",
        selector: "#missing",
      });
      expect(result.success).toBe(false);
      expect(result.error).toBe("Element not found");
    });
  });

  describe("getLocator()", () => {
    it("should throw for empty selector", () => {
      expect(() => engine.getLocator(mockPage, "")).toThrow("Invalid selector");
    });

    it("should throw for non-string selector", () => {
      expect(() => engine.getLocator(mockPage, null)).toThrow(
        "Invalid selector",
      );
    });

    it("should throw for placeholder selectors", () => {
      expect(() => engine.getLocator(mockPage, "...")).toThrow("placeholder");
      expect(() => engine.getLocator(mockPage, "placeholder")).toThrow(
        "placeholder",
      );
      expect(() => engine.getLocator(mockPage, "N/A")).toThrow("placeholder");
    });

    it("should handle role= selector", () => {
      const locator = engine.getLocator(
        mockPage,
        'role=button,name="Click me"',
      );
      expect(mockPage.getByRole).toHaveBeenCalled();
    });

    it("should handle text= selector", () => {
      const locator = engine.getLocator(mockPage, "text=Hello World");
      expect(mockPage.getByText).toHaveBeenCalled();
    });

    it("should use locator for CSS selectors", () => {
      const locator = engine.getLocator(mockPage, "#submit");
      expect(mockPage.locator).toHaveBeenCalled();
    });
  });

  describe("performClick()", () => {
    it("should click element", async () => {
      await engine.performClick(mockPage, "#btn");
      expect(mockPage.locator).toHaveBeenCalled();
    });

    it("should work with humanization disabled", async () => {
      engine.humanization.enabled = false;
      await engine.performClick(mockPage, "#btn");
      expect(mockPage.locator).toHaveBeenCalled();
    });
  });

  describe("performType()", () => {
    it("should type into element", async () => {
      await engine.performType(mockPage, "#input", "test");
      expect(mockPage.locator).toHaveBeenCalled();
    });

    it("should type with humanization disabled", async () => {
      engine.humanization.enabled = false;
      await engine.performType(mockPage, "#input", "test");
      expect(mockPage.locator).toHaveBeenCalled();
    });
  });

  describe("performPress()", () => {
    it("should press a key", async () => {
      await engine.performPress(mockPage, "Enter");
      expect(mockPage.keyboard.press).toHaveBeenCalledWith("Enter");
    });

    it("should throw for missing key", async () => {
      await expect(engine.performPress(mockPage, null)).rejects.toThrow(
        "Key is required",
      );
    });
  });

  describe("performScroll()", () => {
    it("should scroll down", async () => {
      await engine.performScroll(mockPage, "down");
      expect(mockPage.evaluate).toHaveBeenCalled();
    });

    it("should scroll up", async () => {
      await engine.performScroll(mockPage, "up");
      expect(mockPage.evaluate).toHaveBeenCalled();
    });

    it("should scroll to top", async () => {
      await engine.performScroll(mockPage, "top");
      expect(mockPage.evaluate).toHaveBeenCalled();
    });

    it("should scroll to bottom", async () => {
      await engine.performScroll(mockPage, "bottom");
      expect(mockPage.evaluate).toHaveBeenCalled();
    });

    it("should scroll to done (bottom)", async () => {
      await engine.performScroll(mockPage, "done");
      expect(mockPage.evaluate).toHaveBeenCalled();
    });
  });

  describe("performNavigate()", () => {
    it("should navigate to URL", async () => {
      await engine.performNavigate(mockPage, "https://example.com");
      expect(mockPage.goto).toHaveBeenCalled();
    });

    it("should add https if missing", async () => {
      await engine.performNavigate(mockPage, "example.com");
      expect(mockPage.goto).toHaveBeenCalledWith(
        "https://example.com",
        expect.any(Object),
      );
    });

    it("should throw for missing URL", async () => {
      await expect(engine.performNavigate(mockPage, null)).rejects.toThrow(
        "URL is required",
      );
    });
  });

  describe("performWait()", () => {
    it("should wait for specified time", async () => {
      await engine.performWait(mockPage, "1000");
      expect(mockPage.waitForTimeout).toHaveBeenCalledWith(1000);
    });

    it("should throw for invalid wait time", async () => {
      await expect(engine.performWait(mockPage, "invalid")).rejects.toThrow(
        "Invalid wait time",
      );
    });
  });

  describe("performClickAt()", () => {
    it("should click at coordinates", async () => {
      await engine.performClickAt(mockPage, 100, 200);
      expect(mockPage.mouse.click).toHaveBeenCalled();
    });

    it("should double click", async () => {
      await engine.performClickAt(mockPage, 100, 200, "double");
      expect(mockPage.mouse.dblclick).toHaveBeenCalled();
    });

    it("should long press", async () => {
      await engine.performClickAt(mockPage, 100, 200, "long", 500);
      expect(mockPage.mouse.down).toHaveBeenCalled();
      expect(mockPage.mouse.up).toHaveBeenCalled();
    });

    it("should handle array coordinates", async () => {
      await engine.performClickAt(mockPage, [100, 150], [200, 250]);
      expect(mockPage.mouse.click).toHaveBeenCalled();
    });

    it("should throw for missing coordinates", async () => {
      await expect(
        engine.performClickAt(mockPage, undefined, 200),
      ).rejects.toThrow("requires x and y");
    });

    it("should throw for non-numeric coordinates", async () => {
      await expect(engine.performClickAt(mockPage, "x", "y")).rejects.toThrow(
        "requires numeric",
      );
    });
  });

  describe("performDrag()", () => {
    it("should perform drag operation", async () => {
      const source = { x: 100, y: 100 };
      const target = { x: 200, y: 200 };
      await engine.performDrag(mockPage, source, target, 500);
      expect(mockPage.mouse.move).toHaveBeenCalled();
      expect(mockPage.mouse.down).toHaveBeenCalled();
      expect(mockPage.mouse.up).toHaveBeenCalled();
    });
  });

  describe("performMultiSelect()", () => {
    it("should perform multi-select", async () => {
      const items = [
        { x: 100, y: 100 },
        { x: 200, y: 200 },
      ];
      await engine.performMultiSelect(mockPage, items, "add");
      expect(mockPage.keyboard.down).toHaveBeenCalledWith("Control");
      expect(mockPage.keyboard.up).toHaveBeenCalledWith("Control");
    });

    it("should perform range selection", async () => {
      const items = [
        { x: 100, y: 100 },
        { x: 200, y: 200 },
      ];
      await engine.performMultiSelect(mockPage, items, "range");
      expect(mockPage.keyboard.down).toHaveBeenCalledWith("Shift");
    });

    it("should throw for non-array items", async () => {
      await expect(engine.performMultiSelect(mockPage, null)).rejects.toThrow(
        "requires an array",
      );
    });
  });
});
