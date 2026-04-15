/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";

vi.mock("@api/core/logger.js", () => ({
  createLogger: () => ({
    info: vi.fn(),
    debug: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
  }),
}));

// Mock sharp module
const mockSharp = vi.fn((buffer) => ({
  resize: vi.fn().mockReturnThis(),
  grayscale: vi.fn().mockReturnThis(),
  raw: vi.fn().mockReturnThis(),
  composite: vi.fn().mockReturnThis(),
  threshold: vi.fn().mockReturnThis(),
  toBuffer: vi.fn().mockResolvedValue(Buffer.alloc(256 * 256, 0)),
}));

vi.mock("sharp", () => ({
  default: mockSharp,
}));

describe("api/agent/visualDiff.js", () => {
  let visualDiffEngine;

  beforeEach(async () => {
    vi.clearAllMocks();
    vi.resetModules();

    vi.doMock("sharp", () => ({
      default: mockSharp,
    }));

    const module = await import("../../../agent/visualDiff.js");
    visualDiffEngine = module.visualDiffEngine || module.default;
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe("visualDiffEngine", () => {
    it("should be defined", () => {
      expect(visualDiffEngine).toBeDefined();
    });

    it("should have all required methods", () => {
      expect(typeof visualDiffEngine.compareScreenshots).toBe("function");
      expect(typeof visualDiffEngine.identifyChangedRegions).toBe("function");
      expect(typeof visualDiffEngine._fallbackComparison).toBe("function");
      expect(typeof visualDiffEngine._findChangeRegions).toBe("function");
      expect(typeof visualDiffEngine._floodFill).toBe("function");
    });

    it("should initialize with sharp module", () => {
      expect(visualDiffEngine.sharp).toBeDefined();
    });
  });

  describe("compareScreenshots", () => {
    it("should compare two screenshots using sharp", async () => {
      const preBuffer = Buffer.from("pre-screenshot");
      const postBuffer = Buffer.from("post-screenshot");

      const result = await visualDiffEngine.compareScreenshots(
        preBuffer,
        postBuffer,
      );

      expect(result).toHaveProperty("changed");
      expect(result).toHaveProperty("diffRatio");
      expect(result).toHaveProperty("diffPixels");
      expect(result).toHaveProperty("significantChange");
      expect(result).toHaveProperty("method");
    });

    it("should use custom threshold and minPixels options", async () => {
      const preBuffer = Buffer.from("pre");
      const postBuffer = Buffer.from("post");

      const result = await visualDiffEngine.compareScreenshots(
        preBuffer,
        postBuffer,
        {
          threshold: 0.5,
          minPixels: 500,
        },
      );

      expect(result).toBeDefined();
    });

    it("should return fallback comparison when sharp fails", async () => {
      // Simulate sharp failure
      mockSharp.mockImplementationOnce(() => {
        throw new Error("Sharp not available");
      });

      const preBuffer = Buffer.from("pre");
      const postBuffer = Buffer.from("post");

      const result = await visualDiffEngine.compareScreenshots(
        preBuffer,
        postBuffer,
      );

      expect(result.method).toBe("fallback");
    });
  });

  describe("_fallbackComparison", () => {
    it("should compare buffers by length difference", () => {
      const preBuffer = Buffer.alloc(100, 0);
      const postBuffer = Buffer.alloc(200, 0);

      const result = visualDiffEngine._fallbackComparison(
        preBuffer,
        postBuffer,
      );

      expect(result.method).toBe("fallback");
      expect(result.diffPixels).toBe(100);
      expect(result.diffRatio).toBe(0.5);
    });

    it("should handle null buffers", () => {
      const result = visualDiffEngine._fallbackComparison(
        null,
        Buffer.from("test"),
      );

      expect(result.changed).toBe(false);
      expect(result.diffRatio).toBe(0);
    });

    it("should return no change for identical buffers", () => {
      const preBuffer = Buffer.alloc(100, 1);
      const postBuffer = Buffer.alloc(100, 1);

      const result = visualDiffEngine._fallbackComparison(
        preBuffer,
        postBuffer,
      );

      expect(result.changed).toBe(false);
      expect(result.diffPixels).toBe(0);
    });

    it("should handle undefined buffers", () => {
      const result = visualDiffEngine._fallbackComparison(undefined, undefined);

      expect(result.changed).toBe(false);
    });
  });

  describe("identifyChangedRegions", () => {
    it("should identify changed regions using sharp", async () => {
      // Mock to return some changed pixels
      mockSharp.mockReturnValueOnce({
        resize: vi.fn().mockReturnThis(),
        grayscale: vi.fn().mockReturnThis(),
        raw: vi.fn().mockReturnThis(),
        composite: vi.fn().mockReturnThis(),
        threshold: vi.fn().mockReturnThis(),
        toBuffer: vi.fn().mockResolvedValue(createDiffBuffer()),
      });

      const preBuffer = Buffer.from("pre");
      const postBuffer = Buffer.from("post");

      const result = await visualDiffEngine.identifyChangedRegions(
        preBuffer,
        postBuffer,
      );

      expect(Array.isArray(result)).toBe(true);
    });

    it("should return empty array when sharp is not available", async () => {
      visualDiffEngine.sharp = null;

      const result = await visualDiffEngine.identifyChangedRegions(
        Buffer.from("pre"),
        Buffer.from("post"),
      );

      expect(result).toEqual([]);
    });

    it("should return empty array on sharp failure", async () => {
      mockSharp.mockImplementationOnce(() => {
        throw new Error("Processing failed");
      });

      const result = await visualDiffEngine.identifyChangedRegions(
        Buffer.from("pre"),
        Buffer.from("post"),
      );

      expect(result).toEqual([]);
    });
  });

  describe("_findChangeRegions", () => {
    it("should return array from diff buffer", () => {
      const diffBuffer = Buffer.alloc(256 * 256, 0);

      const regions = visualDiffEngine._findChangeRegions(diffBuffer);

      expect(Array.isArray(regions)).toBe(true);
    });

    it("should ignore small regions (less than 50 pixels)", () => {
      const diffBuffer = Buffer.alloc(256 * 256, 0);

      // Create a tiny region (less than 50 pixels)
      diffBuffer[10 * 256 + 10] = 255;
      diffBuffer[10 * 256 + 11] = 255;

      const regions = visualDiffEngine._findChangeRegions(diffBuffer);

      expect(regions.length).toBe(0);
    });
  });

  describe("_floodFill", () => {
    it("should fill connected region and return bounding box", () => {
      const buffer = Buffer.alloc(256 * 256, 0);
      const visited = new Set();

      // Create a small region
      for (let y = 5; y < 10; y++) {
        for (let x = 5; x < 10; x++) {
          buffer[y * 256 + x] = 255;
        }
      }

      const region = visualDiffEngine._floodFill(
        buffer,
        5,
        5,
        256,
        256,
        visited,
      );

      expect(region).toHaveProperty("minX");
      expect(region).toHaveProperty("maxX");
      expect(region).toHaveProperty("minY");
      expect(region).toHaveProperty("maxY");
      expect(region).toHaveProperty("pixelCount");
      expect(region.pixelCount).toBe(25); // 5x5 region
    });

    it("should handle out of bounds coordinates", () => {
      const buffer = Buffer.alloc(256 * 256, 0);
      const visited = new Set();

      const region = visualDiffEngine._floodFill(
        buffer,
        -1,
        -1,
        256,
        256,
        visited,
      );

      expect(region.pixelCount).toBe(0);
    });

    it("should skip zero-value pixels", () => {
      const buffer = Buffer.alloc(256 * 256, 0);
      const visited = new Set();

      // Only set one pixel to non-zero
      buffer[5 * 256 + 5] = 255;

      const region = visualDiffEngine._floodFill(
        buffer,
        5,
        5,
        256,
        256,
        visited,
      );

      expect(region.pixelCount).toBe(1);
    });
  });

  describe("default export", () => {
    it("should export visualDiffEngine as default", async () => {
      const mod = await import("../../../agent/visualDiff.js");
      expect(mod.default).toBe(visualDiffEngine);
    });
  });
});

// Helper function to create a diff buffer with some changes
function createDiffBuffer() {
  const buffer = Buffer.alloc(256 * 256, 0);

  // Add some changed pixels in a region
  for (let y = 10; y < 15; y++) {
    for (let x = 10; x < 15; x++) {
      buffer[y * 256 + x] = 255;
    }
  }

  return buffer;
}
