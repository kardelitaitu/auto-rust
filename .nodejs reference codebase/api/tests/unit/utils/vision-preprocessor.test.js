/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from "vitest";

const mockToBuffer = vi
  .fn()
  .mockResolvedValue(Buffer.from("processed-image-data"));

const mockPipeline = {
  metadata: vi.fn().mockResolvedValue({ width: 800, height: 600 }),
  extract: vi.fn().mockReturnThis(),
  resize: vi.fn().mockReturnThis(),
  grayscale: vi.fn().mockReturnThis(),
  linear: vi.fn().mockReturnThis(),
  sharpen: vi.fn().mockReturnThis(),
  median: vi.fn().mockReturnThis(),
  jpeg: vi.fn().mockReturnThis(),
  webp: vi.fn().mockReturnThis(),
  raw: vi.fn().mockReturnThis(),
  toBuffer: mockToBuffer,
  modulate: vi.fn().mockReturnThis(),
  clone: vi.fn().mockReturnThis(),
};

vi.mock("sharp", () => ({
  default: vi.fn(() => mockPipeline),
}));

vi.mock("@api/core/logger.js", () => ({
  createLogger: vi.fn(() => ({
    info: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
    debug: vi.fn(),
    success: vi.fn(),
  })),
}));

vi.mock("fs/promises", () => ({
  mkdir: vi.fn().mockResolvedValue(undefined),
  writeFile: vi.fn().mockResolvedValue(undefined),
}));

vi.mock("path", async () => {
  const actual = await vi.importActual("path");
  return actual;
});

describe("vision-preprocessor.js", () => {
  let VisionPreprocessor;
  let sharp;

  beforeEach(async () => {
    vi.clearAllMocks();
    mockToBuffer.mockResolvedValue(Buffer.from("processed-image-data"));

    const module = await import("@api/utils/vision-preprocessor.js");
    VisionPreprocessor = module.VisionPreprocessor;

    sharp = (await import("sharp")).default;
  });

  describe("VisionPreprocessor", () => {
    it("should be a class", () => {
      expect(typeof VisionPreprocessor).toBe("function");
    });

    it("should create instance with default config", () => {
      const preprocessor = new VisionPreprocessor();
      expect(preprocessor.defaultConfig.targetWidth).toBe(800);
      expect(preprocessor.defaultConfig.grayscale).toBe(false);
      expect(preprocessor.defaultConfig.contrast).toBe(1.0);
      expect(preprocessor.defaultConfig.format).toBe("jpeg");
      expect(preprocessor.defaultConfig.quality).toBe(75);
    });

    it("should create instance with custom config", () => {
      const preprocessor = new VisionPreprocessor({
        targetWidth: 1024,
        grayscale: true,
        contrast: 1.5,
        format: "webp",
        quality: 80,
      });
      expect(preprocessor.defaultConfig.targetWidth).toBe(1024);
      expect(preprocessor.defaultConfig.grayscale).toBe(true);
      expect(preprocessor.defaultConfig.contrast).toBe(1.5);
      expect(preprocessor.defaultConfig.format).toBe("webp");
      expect(preprocessor.defaultConfig.quality).toBe(80);
    });

    it("should initialize stats correctly", () => {
      const preprocessor = new VisionPreprocessor();
      expect(preprocessor.stats.totalProcessed).toBe(0);
      expect(preprocessor.stats.totalSavedBytes).toBe(0);
      expect(preprocessor.stats.avgCompressionRatio).toBe(0);
    });

    it("should have process method", () => {
      const preprocessor = new VisionPreprocessor();
      expect(typeof preprocessor.process).toBe("function");
    });

    it("should process buffer input", async () => {
      const preprocessor = new VisionPreprocessor();
      const inputBuffer = Buffer.from("test-image-data");

      // Mock toBuffer to return proper object with jpeg
      mockToBuffer.mockResolvedValue(Buffer.from("processed-image-data"));

      const result = await preprocessor.process(inputBuffer);

      expect(result).toHaveProperty("buffer");
      expect(result).toHaveProperty("base64");
      expect(result).toHaveProperty("stats");
      expect(result.stats).toHaveProperty("originalSize");
      expect(result.stats).toHaveProperty("processedSize");
      expect(result.stats).toHaveProperty("compressionRatio");
      expect(result.stats).toHaveProperty("processingTime");
    });

    it("should process base64 string input", async () => {
      const preprocessor = new VisionPreprocessor();
      const base64Input = Buffer.from("test-image-data").toString("base64");

      const result = await preprocessor.process(base64Input);

      expect(result).toHaveProperty("buffer");
      expect(result).toHaveProperty("base64");
    });

    it("should process data URL input", async () => {
      const preprocessor = new VisionPreprocessor();
      const dataUrl =
        "data:image/png;base64," +
        Buffer.from("test-image-data").toString("base64");

      const result = await preprocessor.process(dataUrl);

      expect(result).toHaveProperty("buffer");
      expect(result).toHaveProperty("base64");
    });

    it("should call sharp with input buffer", async () => {
      const preprocessor = new VisionPreprocessor();
      const inputBuffer = Buffer.from("test-image-data");

      await preprocessor.process(inputBuffer);

      expect(sharp).toHaveBeenCalledWith(inputBuffer);
    });

    it("should get metadata from pipeline", async () => {
      const preprocessor = new VisionPreprocessor();
      const inputBuffer = Buffer.from("test-image-data");

      await preprocessor.process(inputBuffer);

      expect(mockPipeline.metadata).toHaveBeenCalled();
    });

    it("should call jpeg with quality options for jpeg format", async () => {
      const preprocessor = new VisionPreprocessor();
      const inputBuffer = Buffer.from("test-image-data");

      await preprocessor.process(inputBuffer, { format: "jpeg", quality: 80 });

      expect(mockPipeline.jpeg).toHaveBeenCalledWith({
        quality: 80,
        mozjpeg: true,
      });
    });

    it("should call webp with quality options for webp format", async () => {
      const preprocessor = new VisionPreprocessor();
      const inputBuffer = Buffer.from("test-image-data");

      await preprocessor.process(inputBuffer, { format: "webp", quality: 85 });

      expect(mockPipeline.webp).toHaveBeenCalledWith({
        quality: 85,
        effort: 4,
      });
    });

    it("should apply grayscale when enabled", async () => {
      const preprocessor = new VisionPreprocessor();
      const inputBuffer = Buffer.from("test-image-data");

      await preprocessor.process(inputBuffer, { grayscale: true });

      expect(mockPipeline.grayscale).toHaveBeenCalled();
    });

    it("should apply contrast adjustment when specified", async () => {
      const preprocessor = new VisionPreprocessor();
      const inputBuffer = Buffer.from("test-image-data");

      await preprocessor.process(inputBuffer, { contrast: 1.5 });

      expect(mockPipeline.linear).toHaveBeenCalled();
    });

    it("should apply brightness adjustment when specified", async () => {
      const preprocessor = new VisionPreprocessor();
      const inputBuffer = Buffer.from("test-image-data");

      await preprocessor.process(inputBuffer, { brightness: 20 });

      expect(mockPipeline.linear).toHaveBeenCalled();
    });

    it("should apply noise reduction when specified", async () => {
      const preprocessor = new VisionPreprocessor();
      const inputBuffer = Buffer.from("test-image-data");

      await preprocessor.process(inputBuffer, { noiseReduction: 50 });

      expect(mockPipeline.median).toHaveBeenCalled();
    });

    it("should apply resize when targetWidth specified", async () => {
      const preprocessor = new VisionPreprocessor();
      const inputBuffer = Buffer.from("test-image-data");

      await preprocessor.process(inputBuffer, { targetWidth: 600 });

      expect(mockPipeline.resize).toHaveBeenCalled();
    });

    it("should have _detectROI method", () => {
      const preprocessor = new VisionPreprocessor();
      expect(typeof preprocessor._detectROI).toBe("function");
    });

    it("should have _calculateBlockSaliency method", () => {
      const preprocessor = new VisionPreprocessor();
      expect(typeof preprocessor._calculateBlockSaliency).toBe("function");
    });

    it("should calculate block saliency correctly", () => {
      const preprocessor = new VisionPreprocessor();
      const data = Buffer.alloc(100 * 100 * 3, 128);

      const saliency = preprocessor._calculateBlockSaliency(
        data,
        0,
        0,
        100,
        100,
        3,
      );

      expect(typeof saliency).toBe("number");
      expect(saliency).toBeGreaterThanOrEqual(0);
    });

    it("should update stats after successful processing", async () => {
      const preprocessor = new VisionPreprocessor();
      const inputBuffer = Buffer.from("test-image-data");

      await preprocessor.process(inputBuffer);

      expect(preprocessor.stats.totalProcessed).toBe(1);
      // totalSavedBytes can be negative if processed > original
      expect(preprocessor.stats.totalSavedBytes).toBeDefined();
    });

    it("should return fallback on processing error", async () => {
      mockPipeline.metadata.mockRejectedValueOnce(new Error("Sharp error"));

      const preprocessor = new VisionPreprocessor();
      const inputBuffer = Buffer.from("test-image-data");

      const result = await preprocessor.process(inputBuffer);

      expect(result).toHaveProperty("buffer");
      expect(result).toHaveProperty("base64");
      expect(result.stats).toHaveProperty("error");
    });

    it("should apply explicit ROI extraction", async () => {
      const preprocessor = new VisionPreprocessor();
      const inputBuffer = Buffer.from("test-image-data");

      await preprocessor.process(inputBuffer, {
        roi: { left: 10, top: 20, width: 100, height: 80 },
        autoROI: false,
      });

      expect(mockPipeline.extract).toHaveBeenCalledWith({
        left: 10,
        top: 20,
        width: 100,
        height: 80,
      });
    });

    it("should skip auto ROI when disabled", async () => {
      const preprocessor = new VisionPreprocessor();
      const inputBuffer = Buffer.from("test-image-data");

      await preprocessor.process(inputBuffer, { autoROI: false });

      expect(mockPipeline.raw).not.toHaveBeenCalled();
    });

    it("should call debug save when enabled", async () => {
      const preprocessor = new VisionPreprocessor();
      const saveSpy = vi
        .spyOn(preprocessor, "_saveDebugImage")
        .mockResolvedValue(undefined);
      const inputBuffer = Buffer.from("test-image-data");

      await preprocessor.process(inputBuffer, { debug: true, autoROI: false });

      expect(saveSpy).toHaveBeenCalled();
    });

    it("should reuse singleton instance and allow resetStats", async () => {
      const first = await import("@api/utils/vision-preprocessor.js");
      const a = first.getVisionPreprocessor({ targetWidth: 500 });
      const b = first.getVisionPreprocessor({ targetWidth: 1000 });

      expect(a).toBe(b);
      a.resetStats();
      expect(a.getStats().totalProcessed).toBe(0);
    });

    it("should process using preset config without throwing", async () => {
      const preprocessor = new VisionPreprocessor();
      const inputBuffer = Buffer.from("test-image-data");

      await preprocessor.process(inputBuffer, {
        posterize: 4,
        maskBlue: true,
        saturationBoost: 1.2,
        autoROI: false,
      });

      expect(mockPipeline.clone).toHaveBeenCalled();
    });

    it("should ignore ROI detection failures and continue", async () => {
      const preprocessor = new VisionPreprocessor();
      const inputBuffer = Buffer.from("test-image-data");
      vi.spyOn(preprocessor, "_detectROI").mockResolvedValueOnce(null);

      const result = await preprocessor.process(inputBuffer, { autoROI: true });

      expect(result).toHaveProperty("buffer");
    });

    it("should detect an ROI when saliency is high", async () => {
      const preprocessor = new VisionPreprocessor();
      vi.spyOn(preprocessor, "_calculateBlockSaliency").mockReturnValue(42);

      const data = Buffer.alloc(120 * 120 * 3, 128);
      const pipeline = {
        raw: vi.fn().mockReturnThis(),
        toBuffer: vi.fn().mockResolvedValue({
          data,
          info: { width: 120, height: 120, channels: 3 },
        }),
      };

      const roi = await preprocessor._detectROI(pipeline, {
        width: 120,
        height: 120,
      });

      expect(roi).toEqual({
        left: 0,
        top: 0,
        width: 120,
        height: 120,
      });
    });

    it("should calculate block saliency for grayscale data", () => {
      const preprocessor = new VisionPreprocessor();
      const data = Buffer.alloc(10 * 10, 128);

      const saliency = preprocessor._calculateBlockSaliency(
        data,
        0,
        0,
        10,
        10,
        1,
      );

      expect(saliency).toBe(0);
    });

    it("should return null when _detectROI raw extraction fails", async () => {
      const preprocessor = new VisionPreprocessor();
      const pipeline = {
        raw: vi.fn().mockReturnThis(),
        toBuffer: vi.fn().mockRejectedValue(new Error("raw failed")),
      };

      const roi = await preprocessor._detectROI(pipeline, {
        width: 800,
        height: 600,
      });

      expect(roi).toBeNull();
    });

    it("should return zero dimensions when _getDimensions fails", async () => {
      const preprocessor = new VisionPreprocessor();
      mockPipeline.metadata.mockRejectedValueOnce(new Error("metadata failed"));
      const result = await preprocessor._getDimensions(
        Buffer.from("bad-buffer"),
      );

      expect(result).toEqual({ width: 0, height: 0 });
    });

    it("should return dimensions when _getDimensions succeeds", async () => {
      const preprocessor = new VisionPreprocessor();
      const result = await preprocessor._getDimensions(
        Buffer.from("ok-buffer"),
      );

      expect(result).toEqual({ width: 800, height: 600 });
    });

    it("should handle edge enhancement option", async () => {
      const preprocessor = new VisionPreprocessor();
      const inputBuffer = Buffer.from("test-image-data");

      await preprocessor.process(inputBuffer, { edgeEnhance: true });

      expect(mockPipeline.sharpen).toHaveBeenCalled();
    });

    it("should handle sharpness option", async () => {
      const preprocessor = new VisionPreprocessor();
      const inputBuffer = Buffer.from("test-image-data");

      await preprocessor.process(inputBuffer, { sharpness: 5 });

      expect(mockPipeline.sharpen).toHaveBeenCalledWith(5);
    });

    it("should save debug image successfully", async () => {
      const preprocessor = new VisionPreprocessor();
      const debugSpy = vi.spyOn(console, "debug").mockImplementation(() => {});

      await preprocessor._saveDebugImage(Buffer.from("debug"), {
        format: "jpeg",
      });

      expect(debugSpy).not.toHaveBeenCalled();
      debugSpy.mockRestore();
    });

    it("should warn and continue when debug image save fails", async () => {
      const preprocessor = new VisionPreprocessor();
      const fs = await import("fs/promises");
      const mkdirSpy = vi
        .spyOn(fs, "mkdir")
        .mockRejectedValueOnce(new Error("save failed"));

      await expect(
        preprocessor._saveDebugImage(Buffer.from("debug"), { format: "jpeg" }),
      ).resolves.toBeUndefined();

      expect(mkdirSpy).toHaveBeenCalled();
      mkdirSpy.mockRestore();
    });

    it("should process via the singleton helper", async () => {
      const module = await import("@api/utils/vision-preprocessor.js");
      const spy = vi
        .spyOn(module.getVisionPreprocessor(), "process")
        .mockResolvedValueOnce({
          buffer: Buffer.from("x"),
          base64: "eA==",
          stats: {},
        });

      await module.processForVision(Buffer.from("input"), { autoROI: false });

      expect(spy).toHaveBeenCalled();
      spy.mockRestore();
    });

    it("should create a fresh singleton after module reset", async () => {
      vi.resetModules();
      const module = await import("@api/utils/vision-preprocessor.js");

      const first = module.getVisionPreprocessor({ targetWidth: 640 });
      const second = module.getVisionPreprocessor({ targetWidth: 1200 });

      expect(first).toBe(second);
      expect(first.defaultConfig.targetWidth).toBe(640);
    });

    it("should expose expected preset values", async () => {
      const module = await import("@api/utils/vision-preprocessor.js");
      expect(module.VPrepPresets.FAST.quality).toBe(70);
      expect(module.VPrepPresets.GAME_UI.edgeEnhance).toBe(true);
      expect(module.VPrepPresets.TEXT_HEAVY.sharpness).toBe(1.5);
      expect(module.VPrepPresets.TOKEN_SAVING.grayscale).toBe(true);
      expect(module.VPrepPresets.OWB_BLUE_OPTIMIZED.maskBlue).toBe(true);
      expect(module.VPrepPresets.DEBUG.debug).toBe(true);
      expect(module.VPrepPresets.OWB_GAME.autoROI).toBe(false);
    });
  });
});
