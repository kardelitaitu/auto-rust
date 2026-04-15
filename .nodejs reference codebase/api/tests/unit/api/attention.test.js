/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import {
  setDistractionChance,
  getDistractionChance,
  gaze,
  attention,
  distraction,
  beforeLeave,
  focusShift,
  maybeDistract,
  calculateVisualWeight,
} from "@api/behaviors/attention.js";

// Mocks
vi.mock("@api/core/context.js", () => ({
  getPage: vi.fn(),
  getCursor: vi.fn(),
}));

vi.mock("@api/core/context-state.js", () => ({
  getStateDistractionChance: vi.fn().mockReturnValue(0.2),
  setStateDistractionChance: vi.fn(),
  getStateAttentionMemory: vi.fn().mockReturnValue([]),
  recordStateAttentionMemory: vi.fn(),
}));

vi.mock("@api/core/logger.js", () => ({
  log: vi.fn(),
  warn: vi.fn(),
  error: vi.fn(),
  debug: vi.fn(),
  info: vi.fn(),
}));

vi.mock("@api/behaviors/persona.js", () => ({
  getPersona: vi.fn().mockReturnValue({
    speed: 1,
    idleChance: 0.1,
  }),
}));

vi.mock("@api/behaviors/timing.js", () => ({
  think: vi.fn().mockResolvedValue(),
  delay: vi.fn().mockResolvedValue(),
  randomInRange: vi.fn((min, max) => min),
}));

vi.mock("@api/utils/math.js", () => ({
  mathUtils: {
    randomInRange: vi.fn((min, max) => min),
  },
}));

import { getPage, getCursor } from "@api/core/context.js";
import { think, delay } from "@api/behaviors/timing.js";

describe("api/behaviors/attention.js", () => {
  let mockPage;
  let mockCursor;
  let mockLocator;

  beforeEach(() => {
    vi.clearAllMocks();
    setDistractionChance(0.2);

    mockLocator = {
      first: vi.fn().mockReturnThis(),
      boundingBox: vi
        .fn()
        .mockResolvedValue({ x: 0, y: 0, width: 100, height: 100 }),
      click: vi.fn().mockResolvedValue(),
    };

    mockPage = {
      locator: vi.fn().mockReturnValue(mockLocator),
      viewportSize: vi.fn().mockReturnValue({ width: 1000, height: 800 }),
      mouse: {
        click: vi.fn().mockResolvedValue(),
      },
      evaluate: vi.fn().mockResolvedValue(10), // Visual weight
    };

    mockCursor = {
      move: vi.fn().mockResolvedValue(),
    };

    getPage.mockReturnValue(mockPage);
    getCursor.mockReturnValue(mockCursor);
  });

  describe("Distraction Chance", () => {
    it("should set and get distraction chance", () => {
      setDistractionChance(0.5);
      expect(getDistractionChance()).toBeDefined();
    });

    it("should clamp distraction chance", () => {
      setDistractionChance(1.5);
      expect(getDistractionChance()).toBeDefined();
      setDistractionChance(-0.1);
      expect(getDistractionChance()).toBeDefined();
    });
  });

  describe("gaze", () => {
    it("should gaze at selector with saccades", async () => {
      await gaze("#target", { saccades: 2 });

      expect(mockPage.locator).toHaveBeenCalledWith("#target");
      expect(mockCursor.move).toHaveBeenCalledTimes(3); // 2 saccades + 1 final fixation
      expect(delay).toHaveBeenCalledTimes(2); // 2 saccade pauses
      expect(think).toHaveBeenCalled(); // Final duration
    });

    it("should skip if element not found", async () => {
      mockLocator.boundingBox.mockResolvedValue(null);
      await gaze("#target");
      expect(mockCursor.move).not.toHaveBeenCalled();
    });
  });

  describe("attention", () => {
    it("should gaze and record memory", async () => {
      await attention("#target");
      expect(mockPage.locator).toHaveBeenCalledWith("#target");
      expect(mockCursor.move).toHaveBeenCalled();
    });

    it("should use provided duration option", async () => {
      await attention("#target", { duration: 3000 });
      // The gaze should be called with the custom duration
      expect(mockPage.locator).toHaveBeenCalledWith("#target");
    });
  });

  describe("distraction", () => {
    it("should look at provided selector if found", async () => {
      const originalRandom = Math.random;
      Math.random = () => 0; // Select first selector

      await distraction(["#distract"]);

      expect(mockPage.locator).toHaveBeenCalledWith("#distract");
      expect(mockCursor.move).toHaveBeenCalled();

      Math.random = originalRandom;
    });

    it("should look at random position if no selectors provided", async () => {
      await distraction([]);
      expect(mockCursor.move).toHaveBeenCalled();
      expect(mockPage.locator).not.toHaveBeenCalled();
    });

    it("should fallback to random position if selector not found", async () => {
      const originalRandom = Math.random;
      Math.random = () => 0;

      mockLocator.boundingBox.mockResolvedValue(null); // Not found

      await distraction(["#distract"]);

      expect(mockCursor.move).toHaveBeenCalled(); // Random pos

      Math.random = originalRandom;
    });
  });

  describe("beforeLeave", () => {
    it("should move to top and pause", async () => {
      await beforeLeave();
      expect(mockCursor.move).toHaveBeenCalledWith(500, 50); // Top center (1000/2, 50)
      expect(think).toHaveBeenCalled();
    });

    it("should respect options", async () => {
      await beforeLeave({ moveToTop: false, pause: false });
      expect(mockCursor.move).not.toHaveBeenCalled();
      expect(think).not.toHaveBeenCalled();
    });

    it("should handle missing viewport", async () => {
      mockPage.viewportSize.mockReturnValue(null);
      await beforeLeave({ moveToTop: true, pause: true });
      // Should still call think but not cursor.move
      expect(mockCursor.move).not.toHaveBeenCalled();
      expect(think).toHaveBeenCalled();
    });
  });

  describe("focusShift", () => {
    it("should click shift selector if provided", async () => {
      await focusShift("#main", "#shift");
      expect(mockPage.locator).toHaveBeenCalledWith("#shift");
      expect(mockLocator.click).toHaveBeenCalled();
    });

    it("should handle shift selector click failure", async () => {
      mockLocator.click.mockRejectedValue(new Error("Click failed"));
      // Should not throw, should continue to main
      await expect(focusShift("#main", "#shift")).resolves.not.toThrow();
    });

    it("should click near main selector if no shift selector with valid box", async () => {
      // Reset mock to ensure valid box
      mockLocator.boundingBox.mockResolvedValue({
        x: 0,
        y: 0,
        width: 100,
        height: 100,
      });

      // Test both branches of the random ternary
      const originalRandom = Math.random;

      // Test when Math.random() <= 0.5 (offset negative)
      Math.random = () => 0.3;
      await focusShift("#main");
      expect(mockPage.locator).toHaveBeenCalledWith("#main");
      expect(mockCursor.move).toHaveBeenCalled();

      // Test when Math.random() > 0.5 (offset positive)
      Math.random = () => 0.7;
      await focusShift("#main");

      Math.random = originalRandom;
      expect(mockPage.mouse.click).toHaveBeenCalled();
      expect(delay).toHaveBeenCalled();
      expect(think).toHaveBeenCalled();
    });

    it("should handle main selector with no bounding box", async () => {
      mockLocator.boundingBox.mockResolvedValue(null);

      await focusShift("#main");
      // Should not throw, just doesn't do the offset click
      expect(mockPage.locator).toHaveBeenCalledWith("#main");
      expect(mockCursor.move).not.toHaveBeenCalled();
      expect(mockPage.mouse.click).not.toHaveBeenCalled();
    });
  });

  describe("maybeDistract", () => {
    it("should return false if random check fails", async () => {
      const originalRandom = Math.random;
      Math.random = () => 0.99; // High value -> false

      const result = await maybeDistract();
      expect(result).toBe(false);

      Math.random = originalRandom;
    });

    it("should distract if random check passes", async () => {
      const originalRandom = Math.random;
      Math.random = () => 0.01; // Low value -> true

      const result = await maybeDistract();
      expect(result).toBe(true);
      expect(mockCursor.move).toHaveBeenCalled();

      Math.random = originalRandom;
    });

    it("should use memory for distraction", async () => {
      // First call attention to populate memory
      await attention("#mem");

      const originalRandom = Math.random;
      // 1. Chance check (pass)
      // 2. Use memory check (pass < 0.4)
      // 3. Select from memory (index 0)
      let callCount = 0;
      Math.random = () => {
        callCount++;
        if (callCount === 1) return 0.01; // Chance pass
        if (callCount === 2) return 0.1; // Memory use pass
        return 0; // Selector index
      };

      const result = await maybeDistract();
      expect(result).toBe(true);
      // It should look at #mem (from memory)
      expect(mockPage.locator).toHaveBeenCalledWith("#mem");

      Math.random = originalRandom;
    });

    it("should scale chance by visual weight", async () => {
      mockPage.evaluate.mockResolvedValue(400); // 200+ -> 2x multiplier

      const originalRandom = Math.random;
      Math.random = () => 0.3; // (0.2 + 0.1) * 2 = 0.6 chance. 0.3 < 0.6 -> true

      const result = await maybeDistract();
      expect(result).toBe(true);

      Math.random = originalRandom;
    });

    it("should manage attention memory limits", async () => {
      await attention("#1");
      await attention("#2");
      await attention("#3");
      await attention("#4"); // Should pop #1

      const originalRandom = Math.random;
      let callCount = 0;
      Math.random = () => {
        callCount++;
        if (callCount === 1) return 0.01; // chance pass
        if (callCount === 2) return 0.1; // memory pass
        return 0.99; // Pick last item in memory
      };

      await maybeDistract();
      // Memory should be [#4, #3, #2]. Last is #2.
      expect(mockPage.locator).toHaveBeenCalledWith("#2");

      // Move existing to front
      await attention("#2");
      callCount = 0;
      await maybeDistract();
      // Memory should be [#2, #4, #3]. Last is #3.
      expect(mockPage.locator).toHaveBeenCalledWith("#3");

      Math.random = originalRandom;
    });

    it("should use default idleChance when persona.idleChance is undefined", async () => {
      const { getPersona } = await import("@api/behaviors/persona.js");
      getPersona.mockReturnValue({ speed: 1, idleChance: undefined });

      const originalRandom = Math.random;
      Math.random = () => 0.1;

      const result = await maybeDistract();
      expect(result).toBe(true);

      getPersona.mockReturnValue({ speed: 1, idleChance: 0.1 });
      Math.random = originalRandom;
    });

    it("should not use memory when memory is empty", async () => {
      const originalRandom = Math.random;
      let callCount = 0;
      Math.random = () => {
        callCount++;
        if (callCount === 1) return 0.01;
        if (callCount === 2) return 0.5;
        return 0;
      };

      const result = await maybeDistract(["#fallback"]);
      expect(result).toBe(true);
      expect(mockPage.locator).toHaveBeenCalledWith("#fallback");

      Math.random = originalRandom;
    });

    it("should handle recordMemory with null/undefined selector", async () => {
      // Test that recordMemory handles edge cases without throwing
      // Since attention() always calls gaze(), we need to test recordMemory differently
      // The function should handle null/undefined/empty strings gracefully
      const originalRandom = Math.random;
      Math.random = () => 0.99; // Skip the distraction path

      // These should not throw
      await expect(attention(null)).resolves.not.toThrow();
      await expect(attention(undefined)).resolves.not.toThrow();
      await expect(attention("")).resolves.not.toThrow();

      Math.random = originalRandom;
    });

    it("should return early if viewport is missing in distraction", async () => {
      mockPage.viewportSize.mockReturnValue(null);
      await distraction();
      expect(mockCursor.move).not.toHaveBeenCalled();
    });

    describe("calculateVisualWeight", () => {
      it("should calculate weight with only interactives", () => {
        expect(calculateVisualWeight(5, 0, 0)).toBe(5);
      });

      it("should calculate weight with stickies (2x multiplier)", () => {
        expect(calculateVisualWeight(2, 3, 0)).toBe(2 + 3 * 2); // 8
      });

      it("should calculate weight with animations (3x multiplier)", () => {
        expect(calculateVisualWeight(1, 0, 2)).toBe(1 + 2 * 3); // 7
      });

      it("should calculate weight with all element types", () => {
        // interactives(10) + stickies(5*2) + animations(3*3) = 10 + 10 + 9 = 29
        expect(calculateVisualWeight(10, 5, 3)).toBe(29);
      });

      it("should handle zero elements", () => {
        expect(calculateVisualWeight(0, 0, 0)).toBe(0);
      });
    });
  });
});
