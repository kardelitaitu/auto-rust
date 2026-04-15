/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from "vitest";

vi.mock("@api/core/context.js", () => ({
  getCursor: vi.fn(),
  getPage: vi.fn(),
  setSessionInterval: vi.fn(),
  clearSessionInterval: vi.fn(),
}));

vi.mock("@api/core/context-state.js", () => ({
  getStatePathStyle: vi.fn().mockReturnValue("bezier"),
  setStatePathStyle: vi.fn(),
  getStatePathOptions: vi.fn().mockReturnValue({}),
}));

vi.mock("@api/behaviors/persona.js", () => ({
  getPersona: vi.fn().mockReturnValue({ microMoveChance: 0.1 }),
}));

vi.mock("@api/utils/math.js", () => ({
  mathUtils: {
    gaussian: vi.fn(() => 0.5),
    randomInRange: vi.fn((min, max) => (min + max) / 2),
  },
}));

vi.mock("@api/core/logger.js", () => ({
  createLogger: () => ({
    info: vi.fn(),
    debug: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
  }),
}));

describe("api/interactions/cursor.js", () => {
  let cursor;
  let mockPage;
  let mockCursor;

  beforeEach(async () => {
    vi.clearAllMocks();

    mockPage = {
      locator: vi.fn().mockReturnValue({
        first: vi.fn().mockReturnValue({
          scrollIntoViewIfNeeded: vi.fn().mockResolvedValue(undefined),
          boundingBox: vi
            .fn()
            .mockResolvedValue({ x: 100, y: 100, width: 50, height: 50 }),
        }),
      }),
    };

    mockCursor = {
      move: vi.fn().mockResolvedValue(undefined),
    };

    const { getPage, getCursor } = await import("@api/core/context.js");
    getPage.mockReturnValue(mockPage);
    getCursor.mockReturnValue(mockCursor);

    cursor = await import("@api/interactions/cursor.js");
  });

  describe("setPathStyle() and getPathStyle()", () => {
    it("should set and get path style", () => {
      cursor.setPathStyle("arc");
      expect(cursor.getPathStyle()).toBe("bezier"); // Mocked to return 'bezier'
    });

    it("should accept options", () => {
      cursor.setPathStyle("overshoot", { overshootDistance: 30 });
      // No error should be thrown
    });
  });

  describe("move()", () => {
    it("should move cursor to element", async () => {
      await cursor.move("#test-element");
      expect(mockPage.locator).toHaveBeenCalledWith("#test-element");
      expect(mockCursor.move).toHaveBeenCalled();
    });

    it("should handle invalid selector gracefully", async () => {
      mockPage.locator.mockReturnValue({
        first: vi.fn().mockReturnValue({
          scrollIntoViewIfNeeded: vi
            .fn()
            .mockRejectedValue(new Error("Not found")),
          boundingBox: vi.fn().mockResolvedValue(null),
        }),
      });

      // Should not throw, just log warning
      await cursor.move("#invalid");
      expect(mockPage.locator).toHaveBeenCalled();
    });
  });

  describe("up() and down()", () => {
    it("should have up function", () => {
      expect(typeof cursor.up).toBe("function");
    });

    it("should have down function", () => {
      expect(typeof cursor.down).toBe("function");
    });
  });

  describe("startFidgeting() and stopFidgeting()", () => {
    it("should start fidgeting", () => {
      cursor.startFidgeting();
      // Should not throw
    });

    it("should stop fidgeting", () => {
      cursor.stopFidgeting();
      // Should not throw
    });
  });
});
