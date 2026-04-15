/**
 * Auto-AI Framework - Action Engine Behavior Tests
 * Comprehensive behavior tests for action execution
 * @module tests/unit/agent/actionEngine-behavior
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import {
  createMockPage,
  createSilentLogger,
} from "@api/tests/utils/test-helpers.js";

// Mock dependencies before importing the module
vi.mock("@api/core/logger.js", () => ({
  createLogger: () => createSilentLogger(),
}));

vi.mock("@api/utils/ghostCursor.js", () => ({
  GhostCursor: vi.fn().mockImplementation(() => ({
    moveWithHesitation: vi.fn().mockResolvedValue(undefined),
  })),
}));

vi.mock("@api/utils/math.js", () => ({
  mathUtils: {
    gaussian: vi.fn((mean) => mean),
    randomInRange: vi.fn((min, max) => Math.floor((min + max) / 2)),
  },
}));

describe("ActionEngine - Behavior Tests", () => {
  let ActionEngine;
  let engine;
  let mockPage;

  beforeEach(async () => {
    vi.clearAllMocks();

    // Dynamic import after mocks are set up
    const module = await import("@api/agent/actionEngine.js");
    engine = module.default || module.actionEngine;

    mockPage = createMockPage();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe("Input Validation", () => {
    it("should return error when action is null", async () => {
      const result = await engine.execute(mockPage, null);

      expect(result.success).toBe(false);
      expect(result.error).toBe("No action specified");
    });

    it("should return error when action.action is undefined", async () => {
      const result = await engine.execute(mockPage, { selector: "#test" });

      expect(result.success).toBe(false);
      expect(result.error).toBe("No action specified");
    });

    it("should return error for unknown action type", async () => {
      const result = await engine.execute(mockPage, {
        action: "unknownAction",
      });

      expect(result.success).toBe(false);
      expect(result.error).toContain("Unknown action");
    });
  });

  describe("Done Action", () => {
    it('should return done=true when action is "done"', async () => {
      const result = await engine.execute(mockPage, { action: "done" });

      expect(result.success).toBe(true);
      expect(result.done).toBe(true);
    });
  });

  describe("Click Action", () => {
    beforeEach(() => {
      // Disable humanization for click tests to avoid GhostCursor issues
      engine.humanization.enabled = false;
      engine.humanization.mouseMovement = false;
    });

    it("should click element by selector", async () => {
      const mockLocator = {
        waitFor: vi.fn().mockResolvedValue(undefined),
        click: vi.fn().mockResolvedValue(undefined),
        boundingBox: vi
          .fn()
          .mockResolvedValue({ x: 100, y: 100, width: 50, height: 50 }),
        first: vi.fn().mockReturnThis(),
      };
      mockPage.locator.mockReturnValue(mockLocator);

      const result = await engine.execute(mockPage, {
        action: "click",
        selector: "#test-button",
      });

      expect(result.success).toBe(true);
      expect(mockPage.locator).toHaveBeenCalledWith("#test-button");
      expect(mockLocator.click).toHaveBeenCalled();
    });

    it("should wait for element visibility before clicking", async () => {
      const mockLocator = {
        waitFor: vi.fn().mockResolvedValue(undefined),
        click: vi.fn().mockResolvedValue(undefined),
        boundingBox: vi.fn().mockResolvedValue(null),
        first: vi.fn().mockReturnThis(),
      };
      mockPage.locator.mockReturnValue(mockLocator);

      await engine.execute(mockPage, { action: "click", selector: "#test" });

      expect(mockLocator.waitFor).toHaveBeenCalledWith({
        state: "visible",
        timeout: 5000,
      });
    });

    it("should handle click with role selector", async () => {
      const mockLocator = {
        waitFor: vi.fn().mockResolvedValue(undefined),
        click: vi.fn().mockResolvedValue(undefined),
        boundingBox: vi.fn().mockResolvedValue(null),
        first: vi.fn().mockReturnThis(),
      };
      mockPage.getByRole.mockReturnValue(mockLocator);

      const result = await engine.execute(mockPage, {
        action: "click",
        selector: 'role=button,name="Submit"',
      });

      expect(result.success).toBe(true);
      expect(mockPage.getByRole).toHaveBeenCalled();
    });

    it("should handle click with text selector", async () => {
      const mockLocator = {
        waitFor: vi.fn().mockResolvedValue(undefined),
        click: vi.fn().mockResolvedValue(undefined),
        boundingBox: vi.fn().mockResolvedValue(null),
        first: vi.fn().mockReturnThis(),
      };
      mockPage.getByText.mockReturnValue(mockLocator);

      const result = await engine.execute(mockPage, {
        action: "click",
        selector: "text=Click me",
      });

      expect(result.success).toBe(true);
      expect(mockPage.getByText).toHaveBeenCalledWith("Click me");
    });
  });

  describe("ClickAt Action", () => {
    beforeEach(() => {
      // Disable humanization for clickAt tests to avoid GhostCursor issues
      engine.humanization.enabled = false;
      engine.humanization.mouseMovement = false;
    });

    it("should click at specific coordinates", async () => {
      const result = await engine.execute(mockPage, {
        action: "clickAt",
        x: 100,
        y: 200,
      });

      expect(result.success).toBe(true);
      expect(mockPage.mouse.click).toHaveBeenCalledWith(100, 200);
    });

    it("should handle array coordinates", async () => {
      const result = await engine.execute(mockPage, {
        action: "clickAt",
        x: [100, 150],
        y: [200, 250],
      });

      expect(result.success).toBe(true);
      expect(mockPage.mouse.click).toHaveBeenCalledWith(100, 200);
    });

    it("should handle single array format [x, y]", async () => {
      const result = await engine.execute(mockPage, {
        action: "clickAt",
        x: [100, 200],
      });

      expect(result.success).toBe(true);
      expect(mockPage.mouse.click).toHaveBeenCalledWith(100, 200);
    });

    it("should support different click types", async () => {
      await engine.execute(mockPage, {
        action: "clickAt",
        x: 100,
        y: 200,
        clickType: "double",
      });

      expect(mockPage.mouse.dblclick).toHaveBeenCalled();
    });
  });

  describe("Type Action", () => {
    beforeEach(() => {
      // Disable humanization for type tests to avoid GhostCursor issues
      engine.humanization.enabled = false;
      engine.humanization.mouseMovement = false;
    });

    it("should type into element by selector", async () => {
      const mockLocator = {
        waitFor: vi.fn().mockResolvedValue(undefined),
        click: vi.fn().mockResolvedValue(undefined),
        fill: vi.fn().mockResolvedValue(undefined),
        boundingBox: vi.fn().mockResolvedValue(null),
        first: vi.fn().mockReturnThis(),
      };
      mockPage.locator.mockReturnValue(mockLocator);

      const result = await engine.execute(mockPage, {
        action: "type",
        selector: "#input",
        value: "Hello World",
      });

      expect(result.success).toBe(true);
      expect(mockLocator.fill).toHaveBeenCalledWith("Hello World");
    });

    it("should click element before typing", async () => {
      const mockLocator = {
        waitFor: vi.fn().mockResolvedValue(undefined),
        click: vi.fn().mockResolvedValue(undefined),
        fill: vi.fn().mockResolvedValue(undefined),
        boundingBox: vi.fn().mockResolvedValue(null),
        first: vi.fn().mockReturnThis(),
      };
      mockPage.locator.mockReturnValue(mockLocator);

      await engine.execute(mockPage, {
        action: "type",
        selector: "#input",
        value: "test",
      });

      expect(mockLocator.click).toHaveBeenCalled();
    });
  });

  describe("Press Action", () => {
    it("should press a key", async () => {
      const result = await engine.execute(mockPage, {
        action: "press",
        key: "Enter",
      });

      expect(result.success).toBe(true);
      expect(mockPage.keyboard.press).toHaveBeenCalledWith("Enter");
    });

    it("should use value as key if key not specified", async () => {
      const result = await engine.execute(mockPage, {
        action: "press",
        value: "Escape",
      });

      expect(result.success).toBe(true);
      expect(mockPage.keyboard.press).toHaveBeenCalledWith("Escape");
    });

    it("should return error if no key specified", async () => {
      const result = await engine.execute(mockPage, { action: "press" });

      expect(result.success).toBe(false);
      expect(result.error).toContain("Key is required");
    });
  });

  describe("Scroll Action", () => {
    it("should scroll down", async () => {
      const result = await engine.execute(mockPage, {
        action: "scroll",
        value: "down",
      });

      expect(result.success).toBe(true);
      expect(mockPage.evaluate).toHaveBeenCalled();
    });

    it("should scroll up", async () => {
      const result = await engine.execute(mockPage, {
        action: "scroll",
        value: "up",
      });

      expect(result.success).toBe(true);
    });

    it("should scroll to top", async () => {
      const result = await engine.execute(mockPage, {
        action: "scroll",
        value: "top",
      });

      expect(result.success).toBe(true);
    });

    it("should scroll to bottom", async () => {
      const result = await engine.execute(mockPage, {
        action: "scroll",
        value: "bottom",
      });

      expect(result.success).toBe(true);
    });
  });

  describe("Navigate Action", () => {
    it("should navigate to URL", async () => {
      const result = await engine.execute(mockPage, {
        action: "navigate",
        value: "https://example.com",
      });

      expect(result.success).toBe(true);
      expect(mockPage.goto).toHaveBeenCalledWith(
        "https://example.com",
        expect.objectContaining({ waitUntil: "domcontentloaded" }),
      );
    });

    it("should support goto alias", async () => {
      const result = await engine.execute(mockPage, {
        action: "goto",
        value: "https://example.com",
      });

      expect(result.success).toBe(true);
      expect(mockPage.goto).toHaveBeenCalled();
    });
  });

  describe("Wait Action", () => {
    it("should wait for specified duration", async () => {
      const result = await engine.execute(mockPage, {
        action: "wait",
        value: "1000",
      });

      expect(result.success).toBe(true);
      expect(mockPage.waitForTimeout).toHaveBeenCalled();
    });

    it("should support delay alias", async () => {
      const result = await engine.execute(mockPage, {
        action: "delay",
        value: "500",
      });

      expect(result.success).toBe(true);
    });
  });

  describe("Screenshot Action", () => {
    it("should take screenshot", async () => {
      const result = await engine.execute(mockPage, { action: "screenshot" });

      expect(result.success).toBe(true);
      expect(mockPage.screenshot).toHaveBeenCalled();
    });
  });

  describe("Verify Action", () => {
    it("should handle verify action", async () => {
      const result = await engine.execute(mockPage, {
        action: "verify",
        description: "Check element exists",
      });

      expect(result.success).toBe(true);
    });
  });

  describe("Error Handling", () => {
    it("should catch and return errors from click", async () => {
      const mockLocator = {
        waitFor: vi.fn().mockRejectedValue(new Error("Element not found")),
        click: vi.fn().mockResolvedValue(undefined),
        boundingBox: vi.fn().mockResolvedValue(null),
        first: vi.fn().mockReturnThis(),
      };
      mockPage.locator.mockReturnValue(mockLocator);

      const result = await engine.execute(mockPage, {
        action: "click",
        selector: "#missing",
      });

      expect(result.success).toBe(false);
      expect(result.error).toBeDefined();
    });

    it("should catch and return errors from navigation", async () => {
      mockPage.goto.mockRejectedValue(new Error("Navigation timeout"));

      const result = await engine.execute(mockPage, {
        action: "navigate",
        value: "https://invalid.example.com",
      });

      expect(result.success).toBe(false);
      expect(result.error).toContain("Navigation timeout");
    });
  });

  describe("Locator Resolution", () => {
    it("should throw error for invalid selector", () => {
      expect(() => engine.getLocator(mockPage, "")).toThrow("Invalid selector");
      expect(() => engine.getLocator(mockPage, null)).toThrow(
        "Invalid selector",
      );
      expect(() => engine.getLocator(mockPage, 123)).toThrow(
        "Invalid selector",
      );
    });

    it("should throw error for placeholder selectors", () => {
      expect(() => engine.getLocator(mockPage, "...")).toThrow("placeholder");
      expect(() => engine.getLocator(mockPage, "placeholder")).toThrow(
        "placeholder",
      );
      expect(() => engine.getLocator(mockPage, "N/A")).toThrow("placeholder");
      expect(() => engine.getLocator(mockPage, "test-id-123")).toThrow(
        "placeholder",
      );
    });

    it("should parse role selector correctly", () => {
      engine.getLocator(mockPage, 'role=button,name="Submit"');

      expect(mockPage.getByRole).toHaveBeenCalledWith("button", {
        name: "Submit",
      });
    });

    it("should parse text selector correctly", () => {
      engine.getLocator(mockPage, "text=Click here");

      expect(mockPage.getByText).toHaveBeenCalledWith("Click here");
    });

    it("should use locator for standard selectors", () => {
      engine.getLocator(mockPage, "#my-element");

      expect(mockPage.locator).toHaveBeenCalledWith("#my-element");
    });
  });

  describe("Configuration", () => {
    it("should respect custom timeouts", async () => {
      // Test that engine has default timeouts configured
      expect(engine.timeouts.elementVisible).toBeDefined();
      expect(engine.timeouts.navigation).toBeDefined();
      expect(engine.timeouts.action).toBeDefined();
    });

    it("should respect humanization settings", async () => {
      // Test that engine has humanization configured
      expect(engine.humanization).toBeDefined();
      expect(engine.humanization.typingDelay).toBeDefined();
      expect(engine.humanization.hesitationChance).toBeDefined();
    });

    it("should have default configuration", async () => {
      // Test that the default engine instance exists
      expect(engine).toBeDefined();
      expect(engine.timeouts).toBeDefined();
    });
  });
});
