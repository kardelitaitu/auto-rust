/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview V-PREP (Vision Pre-Processing and Resolution Enhancement Protocol)
 * Optimizes screenshots for LLM vision consumption by reducing entropy
 * while preserving critical visual information.
 *
 * @module api/utils/vision-preprocessor
 */

import sharp from "sharp";
import { createLogger } from "../core/logger.js";

const logger = createLogger("api/utils/vision-preprocessor.js");

/**
 * @typedef {Object} ROI
 * @property {number} left - Left coordinate
 * @property {number} top - Top coordinate
 * @property {number} width - Region width
 * @property {number} height - Region height
 */

/**
 * @typedef {Object} VPrepConfig
 * @property {number} [targetWidth=800] - Target width for downscaling
 * @property {number} [targetHeight] - Optional target height (maintains aspect if only width)
 * @property {boolean} [grayscale=false] - Convert to grayscale (reduces token count ~3x)
 * @property {number} [contrast=1.0] - Contrast multiplier (1.0 = normal, 1.3-1.5 recommended)
 * @property {number} [brightness=0] - Brightness adjustment (-100 to 100)
 * @property {number} [sharpness=0] - Sharpening amount (0-10, helps text readability)
 * @property {number} [noiseReduction=0] - Noise reduction strength (0-100)
 * @property {ROI} [roi] - Region of Interest to extract
 * @property {boolean} [autoROI=true] - Auto-detect ROI if not specified
 * @property {boolean} [edgeEnhance=false] - Enhance edges for better element detection
 * @property {number} [saturationBoost] - Boost color saturation (1.0 = normal, 1.3 = 30% boost)
 * @property {boolean} [maskBlue=false] - Mask blue tiles by replacing with black (for OWB)
 * @property {number} [posterize] - Reduce colors to N levels per channel (2-8, e.g. 4 = 64 colors)
 * @property {'jpeg'|'webp'|'png'} [format='jpeg'] - Output format
 * @property {number} [quality=75] - Compression quality (60-85 optimal for LLMs)
 * @property {boolean} [debug=false] - Save debug images to logs/
 */

/**
 * @typedef {Object} VPrepResult
 * @property {Buffer} buffer - Processed image buffer
 * @property {string} base64 - Base64 encoded image
 * @property {ROI} [appliedROI] - ROI that was applied
 * @property {object} stats - Processing statistics
 * @property {number} stats.originalSize - Original size in bytes
 * @property {number} stats.processedSize - Processed size in bytes
 * @property {number} stats.compressionRatio - Size reduction ratio
 * @property {number} stats.processingTime - Time spent in ms
 */

/**
 * V-PREP: Vision Pre-Processing and Resolution Enhancement Protocol
 * Optimizes images for LLM vision consumption.
 */
export class VisionPreprocessor {
  /**
   * Create a new VisionPreprocessor instance
   * @param {object} [options] - Configuration options
   */
  constructor(options = {}) {
    this.defaultConfig = {
      targetWidth: options.targetWidth || 800,
      grayscale: options.grayscale || false,
      contrast: options.contrast || 1.0,
      brightness: options.brightness || 0,
      sharpness: options.sharpness || 0,
      noiseReduction: options.noiseReduction || 0,
      autoROI: options.autoROI !== false,
      format: options.format || "jpeg",
      quality: options.quality || 75,
      debug: options.debug || false,
    };

    this.stats = {
      totalProcessed: 0,
      totalSavedBytes: 0,
      avgCompressionRatio: 0,
    };
  }

  /**
   * Process an image buffer for optimal LLM consumption
   * @param {Buffer|string} input - Image buffer or base64 string
   * @param {VPrepConfig} [config] - Processing configuration
   * @returns {Promise<VPrepResult>} Processed result
   */
  async process(input, config = {}) {
    const startTime = Date.now();
    const cfg = { ...this.defaultConfig, ...config };

    // Convert input to buffer if needed
    let inputBuffer;
    if (typeof input === "string") {
      if (input.startsWith("data:")) {
        inputBuffer = Buffer.from(input.split(",")[1], "base64");
      } else {
        inputBuffer = Buffer.from(input, "base64");
      }
    } else {
      inputBuffer = input;
    }

    const originalSize = inputBuffer.length;

    try {
      let pipeline = sharp(inputBuffer);
      const metadata = await pipeline.metadata();

      logger.info(
        `[V-PREP] Processing: ${metadata.width}x${metadata.height}, ${formatBytes(originalSize)}`,
      );

      // 1. Auto-detect ROI if enabled and no ROI specified
      let appliedROI = cfg.roi;
      if (!appliedROI && cfg.autoROI) {
        appliedROI = await this._detectROI(pipeline, metadata);
        if (appliedROI) {
          logger.info(
            `[V-PREP] Auto-detected ROI: ${JSON.stringify(appliedROI)}`,
          );
        }
      }

      // 2. Extract ROI if specified
      if (appliedROI) {
        pipeline = pipeline.extract({
          left: appliedROI.left,
          top: appliedROI.top,
          width: appliedROI.width,
          height: appliedROI.height,
        });
      }

      // 3. Noise reduction (before other operations)
      if (cfg.noiseReduction > 0) {
        pipeline = pipeline.median(Math.max(1, cfg.noiseReduction / 10));
      }

      // 4. Resize to target dimensions
      if (cfg.targetWidth || cfg.targetHeight) {
        pipeline = pipeline.resize({
          width: cfg.targetWidth,
          height: cfg.targetHeight,
          fit: "inside",
          withoutEnlargement: true,
        });
      }

      // 5. Grayscale (reduces tokens significantly for vision models)
      if (cfg.grayscale) {
        pipeline = pipeline.grayscale();
      }

      // 6. Contrast adjustment (linear normalization)
      if (cfg.contrast !== 1.0) {
        // Improved contrast formula: preserves mid-tones better
        const offset = 128 * (1 - cfg.contrast);
        pipeline = pipeline.linear(cfg.contrast, offset);
      }

      // 7. Brightness adjustment
      if (cfg.brightness !== 0) {
        const multiplier = 1 + cfg.brightness / 100;
        pipeline = pipeline.linear(multiplier, 0);
      }

      // 7b. Saturation boost (helps distinguish blue from grey in OWB)
      if (cfg.saturationBoost && cfg.saturationBoost !== 1.0) {
        pipeline = pipeline.modulate({
          saturation: cfg.saturationBoost,
        });
      }

      // 7c. Posterize - reduce colors to discrete levels (like Photoshop posterize)
      // Makes distinct color regions (blue, grey, red) easier to identify
      if (cfg.posterize && cfg.posterize >= 2 && cfg.posterize <= 8) {
        const levels = cfg.posterize;
        const step = 255 / (levels - 1);

        // Get raw pixels, posterize, recreate pipeline
        const { data, info } = await pipeline
          .clone()
          .raw()
          .toBuffer({ resolveWithObject: true });

        const { width, height, channels } = info;

        // Posterize each channel: value = round(value/step) * step
        for (let i = 0; i < data.length; i += channels) {
          data[i] = Math.round(Math.round(data[i] / step) * step); // R
          data[i + 1] = Math.round(Math.round(data[i + 1] / step) * step); // G
          data[i + 2] = Math.round(Math.round(data[i + 2] / step) * step); // B
        }

        pipeline = sharp(data, {
          raw: { width, height, channels },
        });
      }

      // 8. Edge enhancement (helps element detection)
      if (cfg.edgeEnhance) {
        pipeline = pipeline.sharpen({
          sigma: 1.5,
          m1: 0.5,
          m2: 0.5,
        });
      }

      // 9. Additional sharpening for text readability
      if (cfg.sharpness > 0) {
        pipeline = pipeline.sharpen(cfg.sharpness);
      }

      // 10. Mask blue tiles (replace blue pixels with black)
      // This helps LLMs that struggle to distinguish blue from grey
      if (cfg.maskBlue) {
        // Get raw pixel data for manipulation
        const { data, info } = await pipeline
          .clone()
          .raw()
          .toBuffer({ resolveWithObject: true });

        const { width, height, channels } = info;

        // Process each pixel: if blue, make it black
        for (let i = 0; i < data.length; i += channels) {
          const r = data[i];
          const g = data[i + 1];
          const b = data[i + 2];

          // Blue detection: B > 100, B > R*1.3, B > G*1.2
          const isBlue = b > 100 && b > r * 1.3 && b > g * 1.2;

          if (isBlue) {
            // Replace blue with black
            data[i] = 0; // R
            data[i + 1] = 0; // G
            data[i + 2] = 0; // B
          }
        }

        // Create new pipeline from modified raw data
        pipeline = sharp(data, {
          raw: { width, height, channels },
        });
      }

      // 11. Execute pipeline and get buffer
      const format = cfg.format;
      const formatOptions =
        format === "webp"
          ? { quality: cfg.quality, effort: 4 }
          : { quality: cfg.quality, mozjpeg: true };

      const processedBuffer = await pipeline[format](formatOptions).toBuffer();

      const processingTime = Date.now() - startTime;
      const processedSize = processedBuffer.length;
      const compressionRatio = originalSize / processedSize;

      // Update stats
      this.stats.totalProcessed++;
      this.stats.totalSavedBytes += originalSize - processedSize;
      this.stats.avgCompressionRatio =
        (this.stats.avgCompressionRatio * (this.stats.totalProcessed - 1) +
          compressionRatio) /
        this.stats.totalProcessed;

      // Debug output
      if (cfg.debug) {
        await this._saveDebugImage(processedBuffer, cfg);
      }

      const result = {
        buffer: processedBuffer,
        base64: processedBuffer.toString("base64"),
        appliedROI,
        stats: {
          originalSize,
          processedSize,
          compressionRatio: compressionRatio.toFixed(2),
          processingTime,
          outputDimensions: await this._getDimensions(processedBuffer),
        },
      };

      logger.info(
        `[V-PREP] Complete: ${formatBytes(originalSize)} → ${formatBytes(processedSize)} ` +
          `(${compressionRatio.toFixed(1)}x smaller, ${processingTime}ms)`,
      );

      return result;
    } catch (error) {
      logger.error(`[V-PREP] Processing failed: ${error.message}`);

      // Fallback: return original as base64
      return {
        buffer: inputBuffer,
        base64: inputBuffer.toString("base64"),
        appliedROI: null,
        stats: {
          originalSize,
          processedSize: originalSize,
          compressionRatio: 1,
          processingTime: Date.now() - startTime,
          error: error.message,
        },
      };
    }
  }

  /**
   * Auto-detect ROI based on visual saliency
   * @private
   */
  async _detectROI(pipeline, _metadata) {
    try {
      // Get raw pixel data for analysis
      const { data, info } = await pipeline
        .raw()
        .toBuffer({ resolveWithObject: true });

      const { width, height, channels } = info;

      // Simple saliency detection: find region with highest contrast
      const blockSize = 100;
      let maxSaliency = 0;
      let bestBlock = null;

      for (let y = 0; y < height - blockSize; y += blockSize / 2) {
        for (let x = 0; x < width - blockSize; x += blockSize / 2) {
          const saliency = this._calculateBlockSaliency(
            data,
            x,
            y,
            blockSize,
            width,
            channels,
          );

          if (saliency > maxSaliency) {
            maxSaliency = saliency;
            bestBlock = { x, y };
          }
        }
      }

      if (bestBlock && maxSaliency > 10) {
        // Return a larger region around the salient block
        const padding = blockSize;
        const left = Math.max(0, bestBlock.x - padding);
        const top = Math.max(0, bestBlock.y - padding);
        const roiWidth = Math.min(width - left, blockSize * 3);
        const roiHeight = Math.min(height - top, blockSize * 3);
        return {
          left,
          top,
          width: roiWidth,
          height: roiHeight,
        };
      }

      return null;
    } catch (error) {
      logger.debug(`[V-PREP] ROI detection failed: ${error.message}`);
      return null;
    }
  }

  /**
   * Calculate saliency score for a block (variance-based)
   * @private
   */
  _calculateBlockSaliency(data, startX, startY, blockSize, width, channels) {
    let sum = 0;
    let sumSq = 0;
    let count = 0;

    for (let y = startY; y < startY + blockSize; y++) {
      for (let x = startX; x < startX + blockSize; x++) {
        const idx = (y * width + x) * channels;
        const gray =
          channels === 1
            ? data[idx]
            : data[idx] * 0.299 + data[idx + 1] * 0.587 + data[idx + 2] * 0.114;
        sum += gray;
        sumSq += gray * gray;
        count++;
      }
    }

    const mean = sum / count;
    const variance = sumSq / count - mean * mean;
    return Math.sqrt(variance);
  }

  /**
   * Get image dimensions from buffer
   * @private
   */
  async _getDimensions(buffer) {
    try {
      const metadata = await sharp(buffer).metadata();
      return { width: metadata.width, height: metadata.height };
    } catch {
      return { width: 0, height: 0 };
    }
  }

  /**
   * Save debug image to logs directory
   * @private
   */
  async _saveDebugImage(buffer, config) {
    try {
      const fs = await import("fs/promises");
      const path = await import("path");

      const logsDir = path.resolve(process.cwd(), "logs");
      await fs.mkdir(logsDir, { recursive: true });

      const filename = `vprep-debug-${Date.now()}.${config.format}`;
      const filepath = path.join(logsDir, filename);

      await fs.writeFile(filepath, buffer);
      logger.debug(`[V-PREP] Debug image saved: ${filepath}`);
    } catch (error) {
      logger.warn(`[V-PREP] Failed to save debug image: ${error.message}`);
    }
  }

  /**
   * Get processor statistics
   * @returns {object} Statistics
   */
  getStats() {
    return { ...this.stats };
  }

  /**
   * Reset statistics
   */
  resetStats() {
    this.stats = {
      totalProcessed: 0,
      totalSavedBytes: 0,
      avgCompressionRatio: 0,
    };
  }
}

/**
 * Format bytes to human-readable string
 * @private
 */
function formatBytes(bytes) {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
}

// Singleton instance for convenience
let defaultInstance = null;

/**
 * Get or create the default VisionPreprocessor instance
 * @param {VPrepConfig} [options] - Configuration options
 * @returns {VisionPreprocessor}
 */
export function getVisionPreprocessor(options = {}) {
  if (!defaultInstance) {
    defaultInstance = new VisionPreprocessor(options);
  }
  return defaultInstance;
}

/**
 * Quick process function using default instance
 * @param {Buffer|string} input - Image to process
 * @param {VPrepConfig} [config] - Processing options
 * @returns {Promise<VPrepResult>}
 */
export async function processForVision(input, config = {}) {
  return getVisionPreprocessor().process(input, config);
}

/**
 * Preset configurations for common use cases
 */
export const VPrepPresets = {
  /** Minimal processing - just resize and compress */
  FAST: {
    targetWidth: 800,
    quality: 70,
  },

  /** Game UI - enhanced edges, moderate compression */
  GAME_UI: {
    targetWidth: 960,
    contrast: 1.2,
    edgeEnhance: true,
    quality: 75,
  },

  /** Text-heavy content - high sharpness */
  TEXT_HEAVY: {
    targetWidth: 1000,
    contrast: 1.3,
    sharpness: 1.5,
    quality: 80,
  },

  /** Token optimization - grayscale, aggressive compression */
  TOKEN_SAVING: {
    targetWidth: 640,
    grayscale: true,
    contrast: 1.2,
    quality: 60,
  },

  /** Debug - save images, higher quality */
  DEBUG: {
    targetWidth: 1280,
    quality: 90,
    debug: true,
  },

  /**
   * OWB (Open World Browser) - Territory conquest game
   * Optimized for: hex maps, unit numbers, resource bars, quest panels
   * Screenshot analysis: 1280x720 with dark theme, pink/red borders
   * Uses half viewport dimensions for better LLM accuracy
   * ROI detection disabled to use full target dimensions
   * Updated: Lower contrast and brightness to preserve blue color visibility
   */
  OWB_GAME: {
    targetWidth: 800,
    targetHeight: 480,
    contrast: 1.1,
    brightness: 0,
    sharpness: 1.2,
    edgeEnhance: true,
    quality: 60,
    autoROI: false,
    posterize: 8,
  },

  /**
   * OWB Blue-Optimized - Masks blue tiles for LLM clarity
   * Completely removes blue from image by replacing with black
   * LLM only sees grey tiles with numbers - no blue confusion possible
   * Uses PNG format for crisp text on dark background
   */
  OWB_BLUE_OPTIMIZED: {
    targetWidth: 800,
    targetHeight: 480,
    contrast: 1.15, // Slight contrast boost for text readability
    brightness: -5, // Slight darken to make white text pop
    saturationBoost: 1.2,
    sharpness: 1.0, // Sharpen for text clarity
    edgeEnhance: false,
    maskBlue: true, // KEY: Replace blue tiles with black
    format: "png",
    quality: 100,
    autoROI: false,
  },

  /**
   * OWB Resource Bar Focus
   * Optimized for reading resource counts at top of screen
   * ROI: { left: 0, top: 0, width: 1280, height: 50 }
   */
  OWB_RESOURCES: {
    targetWidth: 1200,
    contrast: 1.4,
    sharpness: 1.5,
    quality: 85,
    roi: { left: 0, top: 0, width: 1280, height: 50 },
  },

  /**
   * OWB Quest Panel Focus
   * Optimized for reading quest text on right side
   * ROI: { left: 850, top: 150, width: 400, height: 300 }
   */
  OWB_QUESTS: {
    targetWidth: 600,
    contrast: 1.3,
    sharpness: 1.3,
    quality: 85,
    roi: { left: 850, top: 150, width: 400, height: 300 },
  },

  /**
   * OWB Map Only - Strategic view without UI chrome
   * ROI: Main hex map area excluding top/right/bottom bars
   */
  OWB_MAP: {
    targetWidth: 900,
    contrast: 1.2,
    edgeEnhance: true,
    sharpness: 1.0,
    quality: 75,
    roi: { left: 0, top: 55, width: 920, height: 600 },
  },

  /**
   * OWB Token Saver - Reduced size, preserves color coding
   * Colors: Blue=ours, Red=enemy, Purple=virus, Gray=neutral
   * Uses lower quality and smaller size to reduce tokens while keeping colors
   */
  OWB_TOKEN_SAVING: {
    targetWidth: 720, // Smaller = fewer tokens
    contrast: 1.2, // Slight boost for readability
    sharpness: 0.6, // Minimal sharpening
    quality: 55, // Aggressive JPEG compression
    // Note: We keep colors! The JPEG compression naturally reduces color depth
    // which saves tokens without losing the critical color coding
  },

  /**
   * OWB High Fidelity - Preserves text readability
   * Use this if posterized version makes text unreadable
   * No posterization, higher quality, sharper text
   */
  OWB_HIGH_FIDELITY: {
    targetWidth: 640,
    targetHeight: 360,
    contrast: 1.15, // Slightly higher for text pop
    brightness: 0,
    sharpness: 1.5, // Higher sharpness for text clarity
    edgeEnhance: true, // Enable edge enhancement for text
    quality: 90, // Higher quality to preserve text
    autoROI: false,
    // No posterize - preserves all colors for maximum text readability
  },
};

export default VisionPreprocessor;
