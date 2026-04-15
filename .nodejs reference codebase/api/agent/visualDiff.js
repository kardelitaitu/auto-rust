/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Visual Diff Engine for screenshot comparison
 * Uses sharp for efficient image processing and change detection
 * @module api/agent/visualDiff
 */

import { createLogger } from "../core/logger.js";

const logger = createLogger("api/agent/visualDiff.js");

class VisualDiffEngine {
  constructor() {
    this.sharp = null;
    this._initSharp();
  }

  async _initSharp() {
    try {
      const sharpModule = await import("sharp");
      this.sharp = sharpModule.default;
      logger.info("Sharp module loaded successfully");
    } catch (error) {
      logger.warn(
        "Sharp module not available, using fallback comparison:",
        error.message,
      );
      this.sharp = null;
    }
  }

  /**
   * Compare two screenshots and detect changes
   * @param {Buffer} preBuffer - Pre-action screenshot buffer
   * @param {Buffer} postBuffer - Post-action screenshot buffer
   * @param {object} options - Comparison options
   * @returns {Promise<object>} Comparison results
   */
  async compareScreenshots(preBuffer, postBuffer, options = {}) {
    const { threshold = 0.1, minPixels = 100 } = options;

    if (!this.sharp) {
      return this._fallbackComparison(preBuffer, postBuffer);
    }

    try {
      // Resize for faster comparison
      const preResized = await this.sharp(preBuffer)
        .resize(256, 256, { fit: "fill" })
        .grayscale()
        .raw()
        .toBuffer();

      const postResized = await this.sharp(postBuffer)
        .resize(256, 256, { fit: "fill" })
        .grayscale()
        .raw()
        .toBuffer();

      // Calculate pixel difference
      let diffPixels = 0;
      for (let i = 0; i < preResized.length; i++) {
        const diff = Math.abs(preResized[i] - postResized[i]);
        if (diff > 30) diffPixels++; // Threshold for "different"
      }

      const totalPixels = 256 * 256;
      const diffRatio = diffPixels / totalPixels;

      return {
        changed: diffRatio > threshold,
        diffRatio,
        diffPixels,
        significantChange: diffPixels > minPixels,
        method: "sharp",
      };
    } catch (error) {
      logger.error("Sharp comparison failed:", error.message);
      return this._fallbackComparison(preBuffer, postBuffer);
    }
  }

  /**
   * Fallback comparison using buffer length difference
   * @private
   */
  _fallbackComparison(preBuffer, postBuffer) {
    if (!preBuffer || !postBuffer) {
      return {
        changed: false,
        diffRatio: 0,
        diffPixels: 0,
        significantChange: false,
        method: "fallback",
      };
    }

    const diff = Math.abs(preBuffer.length - postBuffer.length);
    const diffRatio = diff / Math.max(preBuffer.length, postBuffer.length);

    return {
      changed: diff > 100,
      diffRatio,
      diffPixels: diff,
      significantChange: diff > 100,
      method: "fallback",
    };
  }

  /**
   * Identify changed regions in screenshots
   * @param {Buffer} preBuffer - Pre-action screenshot buffer
   * @param {Buffer} postBuffer - Post-action screenshot buffer
   * @returns {Promise<Array>} Array of changed region bounding boxes
   */
  async identifyChangedRegions(preBuffer, postBuffer) {
    if (!this.sharp) {
      return [];
    }

    try {
      // Create diff overlay
      const diff = await this.sharp(preBuffer)
        .composite([
          {
            input: postBuffer,
            blend: "difference",
          },
        ])
        .threshold(30)
        .raw()
        .toBuffer();

      // Find bounding boxes of changed regions
      return this._findChangeRegions(diff);
    } catch (error) {
      logger.error("Region identification failed:", error.message);
      return [];
    }
  }

  /**
   * Find bounding boxes of changed regions in diff buffer
   * @private
   */
  _findChangeRegions(diffBuffer) {
    const width = 256;
    const height = 256;
    const regions = [];
    const visited = new Set();

    for (let y = 0; y < height; y++) {
      for (let x = 0; x < width; x++) {
        const idx = y * width + x;
        if (diffBuffer[idx] > 0 && !visited.has(idx)) {
          const region = this._floodFill(
            diffBuffer,
            x,
            y,
            width,
            height,
            visited,
          );
          if (region.pixelCount > 50) {
            // Minimum region size
            regions.push(region);
          }
        }
      }
    }

    return regions;
  }

  /**
   * Flood fill to find connected region
   * @private
   */
  _floodFill(buffer, startX, startY, width, height, visited) {
    const stack = [{ x: startX, y: startY }];
    let minX = startX,
      maxX = startX,
      minY = startY,
      maxY = startY;
    let pixelCount = 0;

    while (stack.length > 0) {
      const { x, y } = stack.pop();
      const idx = y * width + x;

      if (x < 0 || x >= width || y < 0 || y >= height) continue;
      if (visited.has(idx) || buffer[idx] === 0) continue;

      visited.add(idx);
      pixelCount++;

      minX = Math.min(minX, x);
      maxX = Math.max(maxX, x);
      minY = Math.min(minY, y);
      maxY = Math.max(maxY, y);

      stack.push({ x: x + 1, y });
      stack.push({ x: x - 1, y });
      stack.push({ x, y: y + 1 });
      stack.push({ x, y: y - 1 });
    }

    return { minX, maxX, minY, maxY, pixelCount };
  }
}

const visualDiffEngine = new VisualDiffEngine();

export { visualDiffEngine };
export default visualDiffEngine;
