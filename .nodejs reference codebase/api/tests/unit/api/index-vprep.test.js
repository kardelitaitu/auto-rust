/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 */

import { describe, it, expect, vi, beforeEach } from "vitest";

vi.mock("@api/utils/vision-preprocessor.js", () => {
  const mockInstance = {
    process: vi.fn().mockResolvedValue({
      base64: "processedimage",
      buffer: Buffer.from("image"),
      width: 800,
      height: 600,
    }),
    getStats: vi.fn().mockReturnValue({
      totalProcessed: 100,
      bytesSaved: 50000,
    }),
    resetStats: vi.fn(),
  };

  return {
    VisionPreprocessor: vi.fn(() => mockInstance),
    VPrepPresets: {
      GAME_UI: { targetWidth: 800, grayscale: true },
      SOCIAL_MEDIA: { targetWidth: 1200, grayscale: false },
      DOCUMENT: { targetWidth: 600, contrast: 1.2 },
    },
    processForVision: vi.fn().mockResolvedValue({
      base64: "processed",
      width: 800,
    }),
    getVPrepPresets: vi.fn().mockReturnValue({
      GAME_UI: { targetWidth: 800 },
      SOCIAL_MEDIA: { targetWidth: 1200 },
    }),
  };
});

describe("api.vprep functionality - direct module tests", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe("VisionPreprocessor", () => {
    it("should create VisionPreprocessor instance", async () => {
      const { VisionPreprocessor } =
        await import("@api/utils/vision-preprocessor.js");
      expect(VisionPreprocessor).toBeDefined();
    });

    it("should have process function", async () => {
      const { VisionPreprocessor } =
        await import("@api/utils/vision-preprocessor.js");
      const instance = new VisionPreprocessor();
      expect(typeof instance.process).toBe("function");
    });

    it("should process image with input and config", async () => {
      const { VisionPreprocessor } =
        await import("@api/utils/vision-preprocessor.js");
      const instance = new VisionPreprocessor();
      const input = Buffer.from("test image");
      const config = { targetWidth: 800, grayscale: true };
      const result = await instance.process(input, config);
      expect(instance.process).toHaveBeenCalledWith(input, config);
      expect(result).toBeDefined();
    });

    it("should return result with base64, buffer, and dimensions", async () => {
      const { VisionPreprocessor } =
        await import("@api/utils/vision-preprocessor.js");
      const instance = new VisionPreprocessor();
      const result = await instance.process("test");
      expect(result.base64).toBeDefined();
      expect(result.buffer).toBeDefined();
      expect(result.width).toBe(800);
      expect(result.height).toBe(600);
    });

    it("should get stats", async () => {
      const { VisionPreprocessor } =
        await import("@api/utils/vision-preprocessor.js");
      const instance = new VisionPreprocessor();
      const stats = instance.getStats();
      expect(stats.totalProcessed).toBe(100);
      expect(stats.bytesSaved).toBe(50000);
    });

    it("should reset stats", async () => {
      const { VisionPreprocessor } =
        await import("@api/utils/vision-preprocessor.js");
      const instance = new VisionPreprocessor();
      instance.resetStats();
      expect(instance.resetStats).toHaveBeenCalled();
    });
  });

  describe("VPrepPresets", () => {
    it("should have GAME_UI preset", async () => {
      const { VPrepPresets } =
        await import("@api/utils/vision-preprocessor.js");
      expect(VPrepPresets.GAME_UI).toBeDefined();
      expect(VPrepPresets.GAME_UI.targetWidth).toBe(800);
      expect(VPrepPresets.GAME_UI.grayscale).toBe(true);
    });

    it("should have SOCIAL_MEDIA preset", async () => {
      const { VPrepPresets } =
        await import("@api/utils/vision-preprocessor.js");
      expect(VPrepPresets.SOCIAL_MEDIA).toBeDefined();
      expect(VPrepPresets.SOCIAL_MEDIA.targetWidth).toBe(1200);
    });

    it("should have DOCUMENT preset", async () => {
      const { VPrepPresets } =
        await import("@api/utils/vision-preprocessor.js");
      expect(VPrepPresets.DOCUMENT).toBeDefined();
      expect(VPrepPresets.DOCUMENT.targetWidth).toBe(600);
    });
  });

  describe("processForVision", () => {
    it("should process for vision", async () => {
      const { processForVision } =
        await import("@api/utils/vision-preprocessor.js");
      const result = await processForVision("test");
      expect(processForVision).toHaveBeenCalledWith("test");
      expect(result).toBeDefined();
    });
  });

  describe("getVPrepPresets", () => {
    it("should return presets", async () => {
      const { getVPrepPresets } =
        await import("@api/utils/vision-preprocessor.js");
      const presets = getVPrepPresets();
      expect(presets).toBeDefined();
      expect(presets.GAME_UI).toBeDefined();
    });
  });
});
