/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Context Compressor
 * Compresses AXTree and generates text descriptions for LLM context
 * @module api/agent/contextCompressor
 */

import { createLogger } from "../core/logger.js";

const logger = createLogger("api/agent/contextCompressor.js");

class ContextCompressor {
  constructor() {
    this.maxTokens = 2000; // Approximate max tokens for compressed context
    this.compressionRatio = 0.3; // Target 30% of original size
  }

  /**
   * Compress AXTree to essential information
   * @param {object|string} tree - AXTree (object or JSON string)
   * @param {number} maxTokens - Maximum tokens for output
   * @returns {object} Compressed tree summary
   */
  compressAXTree(tree, _maxTokens = this.maxTokens) {
    try {
      // Parse if string
      const treeObj = typeof tree === "string" ? JSON.parse(tree) : tree;

      const summary = {
        interactiveElements: this._extractInteractive(treeObj),
        forms: this._extractForms(treeObj),
        navigation: this._extractNavigation(treeObj),
        headings: this._extractHeadings(treeObj),
        images: this._extractImages(treeObj),
        textContent: this._extractTextContent(treeObj),
      };

      // Convert to compact string representation
      const compressed = this._toCompactString(summary);

      logger.debug(
        `[ContextCompressor] Compressed AXTree: ${compressed.length} chars`,
      );

      return {
        summary,
        compressed,
        originalSize: JSON.stringify(tree).length,
        compressedSize: compressed.length,
        ratio: compressed.length / JSON.stringify(tree).length,
      };
    } catch (error) {
      logger.error(
        "[ContextCompressor] Failed to compress AXTree:",
        error.message,
      );
      return { compressed: "", error: error.message };
    }
  }

  /**
   * Extract interactive elements from tree
   * @private
   */
  _extractInteractive(tree, depth = 0) {
    const elements = [];

    if (!tree) return elements;

    const interactiveRoles = [
      "button",
      "link",
      "textbox",
      "checkbox",
      "radio",
      "menuitem",
      "tab",
      "combobox",
      "slider",
    ];

    if (interactiveRoles.includes(tree.role)) {
      elements.push({
        role: tree.role,
        name: tree.name || "",
        selector: tree.selector || "",
        value: tree.value || "",
      });
    }

    if (tree.children && depth < 5) {
      for (const child of tree.children) {
        elements.push(...this._extractInteractive(child, depth + 1));
      }
    }

    return elements;
  }

  /**
   * Extract forms from tree
   * @private
   */
  _extractForms(tree, depth = 0) {
    const forms = [];

    if (!tree) return forms;

    if (
      tree.role === "form" ||
      tree.role === "textbox" ||
      tree.role === "combobox"
    ) {
      forms.push({
        role: tree.role,
        name: tree.name || "",
        selector: tree.selector || "",
      });
    }

    if (tree.children && depth < 5) {
      for (const child of tree.children) {
        forms.push(...this._extractForms(child, depth + 1));
      }
    }

    return forms;
  }

  /**
   * Extract navigation elements from tree
   * @private
   */
  _extractNavigation(tree, depth = 0) {
    const navElements = [];

    if (!tree) return navElements;

    if (
      tree.role === "navigation" ||
      tree.role === "menubar" ||
      tree.role === "menu"
    ) {
      navElements.push({
        role: tree.role,
        name: tree.name || "",
        childCount: tree.children?.length || 0,
      });
    }

    if (tree.children && depth < 5) {
      for (const child of tree.children) {
        navElements.push(...this._extractNavigation(child, depth + 1));
      }
    }

    return navElements;
  }

  /**
   * Extract headings from tree
   * @private
   */
  _extractHeadings(tree, depth = 0) {
    const headings = [];

    if (!tree) return headings;

    if (tree.role === "heading") {
      headings.push({
        level: tree.level || 1,
        name: tree.name || "",
      });
    }

    if (tree.children && depth < 5) {
      for (const child of tree.children) {
        headings.push(...this._extractHeadings(child, depth + 1));
      }
    }

    return headings;
  }

  /**
   * Extract images from tree
   * @private
   */
  _extractImages(tree, depth = 0) {
    const images = [];

    if (!tree) return images;

    if (tree.role === "img" || tree.role === "image") {
      images.push({
        name: tree.name || "",
        selector: tree.selector || "",
      });
    }

    if (tree.children && depth < 5) {
      for (const child of tree.children) {
        images.push(...this._extractImages(child, depth + 1));
      }
    }

    return images;
  }

  /**
   * Extract text content from tree
   * @private
   */
  _extractTextContent(tree, depth = 0) {
    if (!tree) return "";

    let text = "";

    if (tree.name && tree.role !== "heading") {
      text += tree.name + " ";
    }

    if (tree.children && depth < 5) {
      for (const child of tree.children) {
        text += this._extractTextContent(child, depth + 1);
      }
    }

    return text.trim().substring(0, 500); // Limit to 500 chars
  }

  /**
   * Convert summary to compact string
   * @private
   */
  _toCompactString(summary) {
    let compact = "";

    // Interactive elements
    if (summary.interactiveElements.length > 0) {
      compact += `Interactive(${summary.interactiveElements.length}): `;
      compact += summary.interactiveElements
        .slice(0, 10) // Limit to 10
        .map((e) => `${e.role}${e.name ? `("${e.name}")` : ""}`)
        .join(", ");
      compact += "\n";
    }

    // Forms
    if (summary.forms.length > 0) {
      compact += `Forms(${summary.forms.length}): `;
      compact += summary.forms
        .slice(0, 5)
        .map((e) => `${e.role}${e.name ? `("${e.name}")` : ""}`)
        .join(", ");
      compact += "\n";
    }

    // Navigation
    if (summary.navigation.length > 0) {
      compact += `Navigation: ${summary.navigation.map((n) => n.role).join(", ")}\n`;
    }

    // Headings
    if (summary.headings.length > 0) {
      compact += `Headings: `;
      compact += summary.headings
        .slice(0, 5)
        .map((h) => `H${h.level}("${h.name}")`)
        .join(", ");
      compact += "\n";
    }

    // Images
    if (summary.images.length > 0) {
      compact += `Images(${summary.images.length})\n`;
    }

    // Text content preview
    if (summary.textContent) {
      compact += `Text: ${summary.textContent.substring(0, 200)}...`;
    }

    return compact.trim();
  }

  /**
   * Generate text description of screenshot (heuristic-based)
   * @param {Buffer|string} screenshot - Screenshot buffer or base64
   * @returns {string} Text description
   */
  describeScreenshot(screenshot) {
    // This is a heuristic-based description
    // In a real implementation, this could use a vision model

    const size =
      typeof screenshot === "string" ? screenshot.length : screenshot.length;

    let description = "Screenshot: ";

    // Estimate content type based on size
    if (size < 50000) {
      description += "Small page (likely minimal content or popup)";
    } else if (size < 150000) {
      description += "Medium page (standard content)";
    } else {
      description += "Large page (heavy content or images)";
    }

    return description;
  }

  /**
   * Compress conversation history
   * @param {Array} history - Conversation history
   * @param {number} maxEntries - Maximum entries to keep
   * @returns {Array} Compressed history
   */
  compressHistory(history, maxEntries = 10) {
    if (history.length <= maxEntries) {
      return history;
    }

    // Keep recent entries and summarize old ones
    const recent = history.slice(-maxEntries + 2);
    const old = history.slice(0, history.length - maxEntries + 2);

    const summary = {
      role: "system",
      content: `[Previous ${old.length} messages summarized: ${old.filter((m) => m.role === "user").length} user messages, ${old.filter((m) => m.role === "assistant").length} assistant responses]`,
    };

    return [summary, ...recent];
  }

  /**
   * Get compression statistics
   * @param {object} compressionResult - Result from compressAXTree
   * @returns {object} Statistics
   */
  getStats(compressionResult) {
    return {
      originalSize: compressionResult.originalSize,
      compressedSize: compressionResult.compressedSize,
      ratio: compressionResult.ratio,
      savings:
        compressionResult.originalSize - compressionResult.compressedSize,
      savingsPercent:
        compressionResult.originalSize > 0
          ? (
              ((compressionResult.originalSize -
                compressionResult.compressedSize) /
                compressionResult.originalSize) *
              100
            ).toFixed(1)
          : 0,
    };
  }
}

const contextCompressor = new ContextCompressor();

export { contextCompressor };
export default contextCompressor;
