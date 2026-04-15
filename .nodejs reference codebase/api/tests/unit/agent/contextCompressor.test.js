/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Unit tests for api/agent/contextCompressor.js
 * @module tests/unit/agent/contextCompressor.test
 */

import { describe, it, expect, vi, beforeEach } from "vitest";

// Mock logger
vi.mock("@api/core/logger.js", () => ({
  createLogger: vi.fn(() => ({
    info: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
    debug: vi.fn(),
  })),
}));

describe("api/agent/contextCompressor.js", () => {
  let contextCompressor;

  beforeEach(async () => {
    vi.clearAllMocks();
    const module = await import("@api/agent/contextCompressor.js");
    contextCompressor = module.contextCompressor || module.default;
  });

  describe("Constructor", () => {
    it("should initialize with default values", () => {
      expect(contextCompressor.maxTokens).toBe(2000);
      expect(contextCompressor.compressionRatio).toBe(0.3);
    });
  });

  describe("compressAXTree()", () => {
    it("should compress a simple tree", () => {
      const tree = {
        role: "root",
        name: "Page",
        children: [
          { role: "button", name: "Submit", selector: "#submit" },
          { role: "textbox", name: "Email", selector: "#email" },
        ],
      };

      const result = contextCompressor.compressAXTree(tree);

      expect(result.summary).toBeDefined();
      expect(result.compressed).toBeDefined();
      expect(result.originalSize).toBeGreaterThan(0);
      expect(result.compressedSize).toBeGreaterThan(0);
      expect(result.ratio).toBeGreaterThan(0);
    });

    it("should handle JSON string input", () => {
      const treeJson = JSON.stringify({
        role: "root",
        children: [{ role: "button", name: "Click" }],
      });

      const result = contextCompressor.compressAXTree(treeJson);
      expect(result.summary).toBeDefined();
    });

    it("should handle invalid JSON gracefully", () => {
      const result = contextCompressor.compressAXTree("invalid json");
      expect(result.error).toBeDefined();
      expect(result.compressed).toBe("");
    });

    it("should extract interactive elements", () => {
      const tree = {
        role: "root",
        children: [
          { role: "button", name: "OK", selector: "#ok" },
          { role: "link", name: "Home", selector: 'a[href="/"]' },
          { role: "textbox", selector: "#search" },
          { role: "checkbox", name: "Remember me" },
        ],
      };

      const result = contextCompressor.compressAXTree(tree);
      expect(result.summary.interactiveElements.length).toBe(4);
    });

    it("should extract forms", () => {
      const tree = {
        role: "root",
        children: [
          { role: "form", name: "Login Form" },
          { role: "textbox", name: "Username" },
          { role: "textbox", name: "Password" },
        ],
      };

      const result = contextCompressor.compressAXTree(tree);
      expect(result.summary.forms.length).toBe(3);
    });

    it("should extract navigation", () => {
      const tree = {
        role: "navigation",
        name: "Main Nav",
        children: [
          { role: "menuitem", name: "Home" },
          { role: "menuitem", name: "About" },
        ],
      };

      const result = contextCompressor.compressAXTree(tree);
      expect(result.summary.navigation.length).toBe(1);
    });

    it("should extract headings", () => {
      const tree = {
        role: "root",
        children: [
          { role: "heading", name: "Welcome", level: 1 },
          { role: "heading", name: "Section", level: 2 },
        ],
      };

      const result = contextCompressor.compressAXTree(tree);
      expect(result.summary.headings.length).toBe(2);
      expect(result.summary.headings[0].level).toBe(1);
    });

    it("should extract images", () => {
      const tree = {
        role: "root",
        children: [
          { role: "img", name: "Logo", selector: "#logo" },
          { role: "image", name: "Banner" },
        ],
      };

      const result = contextCompressor.compressAXTree(tree);
      expect(result.summary.images.length).toBe(2);
    });

    it("should extract text content", () => {
      const tree = {
        role: "root",
        name: "Page Title",
        children: [{ role: "paragraph", name: "Some text content here" }],
      };

      const result = contextCompressor.compressAXTree(tree);
      expect(result.summary.textContent).toContain("Page Title");
      expect(result.summary.textContent).toContain("Some text content here");
    });
  });

  describe("_extractInteractive()", () => {
    it("should return empty array for null", () => {
      const result = contextCompressor._extractInteractive(null);
      expect(result).toEqual([]);
    });

    it("should extract interactive roles", () => {
      const tree = {
        role: "button",
        name: "Click Me",
        selector: "#btn",
      };

      const result = contextCompressor._extractInteractive(tree);
      expect(result.length).toBe(1);
      expect(result[0].role).toBe("button");
    });

    it("should limit depth to 5", () => {
      const deepTree = {
        role: "root",
        children: [
          {
            children: [
              {
                children: [
                  {
                    children: [
                      {
                        children: [
                          {
                            children: [{ role: "button", name: "Deep" }],
                          },
                        ],
                      },
                    ],
                  },
                ],
              },
            ],
          },
        ],
      };

      const result = contextCompressor._extractInteractive(deepTree);
      // Should not find the deeply nested button
      expect(result.length).toBe(0);
    });
  });

  describe("_extractForms()", () => {
    it("should extract form elements", () => {
      const tree = {
        role: "form",
        name: "Contact Form",
        selector: "form#contact",
      };

      const result = contextCompressor._extractForms(tree);
      expect(result.length).toBe(1);
      expect(result[0].role).toBe("form");
    });

    it("should extract textboxes as forms", () => {
      const tree = { role: "textbox", name: "Input" };
      const result = contextCompressor._extractForms(tree);
      expect(result.length).toBe(1);
    });
  });

  describe("_extractNavigation()", () => {
    it("should extract navigation elements", () => {
      const tree = {
        role: "navigation",
        name: "Main Nav",
        children: [{ role: "menuitem" }],
      };

      const result = contextCompressor._extractNavigation(tree);
      expect(result.length).toBe(1);
      expect(result[0].childCount).toBe(1);
    });

    it("should handle navigation without children", () => {
      const tree = { role: "menubar" };
      const result = contextCompressor._extractNavigation(tree);
      expect(result[0].childCount).toBe(0);
    });
  });

  describe("_extractHeadings()", () => {
    it("should extract headings with levels", () => {
      const tree = {
        role: "heading",
        name: "Title",
        level: 2,
      };

      const result = contextCompressor._extractHeadings(tree);
      expect(result.length).toBe(1);
      expect(result[0].level).toBe(2);
    });

    it("should default level to 1 if missing", () => {
      const tree = { role: "heading", name: "Title" };
      const result = contextCompressor._extractHeadings(tree);
      expect(result[0].level).toBe(1);
    });
  });

  describe("_extractImages()", () => {
    it("should extract img role", () => {
      const tree = { role: "img", name: "Logo", selector: "#logo" };
      const result = contextCompressor._extractImages(tree);
      expect(result.length).toBe(1);
    });

    it("should extract image role", () => {
      const tree = { role: "image", name: "Banner" };
      const result = contextCompressor._extractImages(tree);
      expect(result.length).toBe(1);
    });
  });

  describe("_extractTextContent()", () => {
    it("should return empty string for null", () => {
      const result = contextCompressor._extractTextContent(null);
      expect(result).toBe("");
    });

    it("should extract text from nodes", () => {
      const tree = {
        role: "root",
        name: "Page",
        children: [{ role: "paragraph", name: "Content" }],
      };

      const result = contextCompressor._extractTextContent(tree);
      expect(result).toContain("Page");
      expect(result).toContain("Content");
    });

    it("should skip heading names", () => {
      const tree = {
        role: "root",
        name: "Main",
        children: [{ role: "heading", name: "Title" }],
      };

      const result = contextCompressor._extractTextContent(tree);
      expect(result).toContain("Main");
      expect(result).not.toContain("Title");
    });

    it("should limit to 500 characters", () => {
      const longText = "A".repeat(600);
      const tree = { role: "paragraph", name: longText };
      const result = contextCompressor._extractTextContent(tree);
      expect(result.length).toBeLessThanOrEqual(500);
    });
  });

  describe("_toCompactString()", () => {
    it("should format interactive elements", () => {
      const summary = {
        interactiveElements: [
          { role: "button", name: "Submit" },
          { role: "textbox", name: "Email" },
        ],
        forms: [],
        navigation: [],
        headings: [],
        images: [],
        textContent: "",
      };

      const result = contextCompressor._toCompactString(summary);
      expect(result).toContain("Interactive(2)");
      expect(result).toContain("button");
      expect(result).toContain("textbox");
    });

    it("should format forms", () => {
      const summary = {
        interactiveElements: [],
        forms: [{ role: "form", name: "Login" }],
        navigation: [],
        headings: [],
        images: [],
        textContent: "",
      };

      const result = contextCompressor._toCompactString(summary);
      expect(result).toContain("Forms(1)");
    });

    it("should format navigation", () => {
      const summary = {
        interactiveElements: [],
        forms: [],
        navigation: [{ role: "navigation" }],
        headings: [],
        images: [],
        textContent: "",
      };

      const result = contextCompressor._toCompactString(summary);
      expect(result).toContain("Navigation");
    });

    it("should format headings", () => {
      const summary = {
        interactiveElements: [],
        forms: [],
        navigation: [],
        headings: [{ level: 1, name: "Title" }],
        images: [],
        textContent: "",
      };

      const result = contextCompressor._toCompactString(summary);
      expect(result).toContain("Headings");
      expect(result).toContain("H1");
    });

    it("should format images", () => {
      const summary = {
        interactiveElements: [],
        forms: [],
        navigation: [],
        headings: [],
        images: [{ name: "logo" }, { name: "banner" }],
        textContent: "",
      };

      const result = contextCompressor._toCompactString(summary);
      expect(result).toContain("Images(2)");
    });

    it("should include text content preview", () => {
      const summary = {
        interactiveElements: [],
        forms: [],
        navigation: [],
        headings: [],
        images: [],
        textContent: "This is sample text content",
      };

      const result = contextCompressor._toCompactString(summary);
      expect(result).toContain("Text:");
      expect(result).toContain("This is sample text content");
    });
  });

  describe("describeScreenshot()", () => {
    it("should describe small screenshot", () => {
      const smallBuffer = Buffer.alloc(30000);
      const result = contextCompressor.describeScreenshot(smallBuffer);
      expect(result).toContain("Small page");
    });

    it("should describe medium screenshot", () => {
      const mediumBuffer = Buffer.alloc(100000);
      const result = contextCompressor.describeScreenshot(mediumBuffer);
      expect(result).toContain("Medium page");
    });

    it("should describe large screenshot", () => {
      const largeBuffer = Buffer.alloc(200000);
      const result = contextCompressor.describeScreenshot(largeBuffer);
      expect(result).toContain("Large page");
    });

    it("should handle base64 string input", () => {
      const base64 = "A".repeat(30000);
      const result = contextCompressor.describeScreenshot(base64);
      expect(result).toContain("Small page");
    });
  });

  describe("compressHistory()", () => {
    it("should return history as-is if under limit", () => {
      const history = [
        { role: "user", content: "Hello" },
        { role: "assistant", content: "Hi" },
      ];

      const result = contextCompressor.compressHistory(history, 10);
      expect(result).toEqual(history);
    });

    it("should summarize old messages when over limit", () => {
      const history = Array(15)
        .fill(null)
        .map((_, i) => ({
          role: i % 2 === 0 ? "user" : "assistant",
          content: `Message ${i}`,
        }));

      const result = contextCompressor.compressHistory(history, 10);
      expect(result.length).toBeLessThan(15);
      expect(result[0].role).toBe("system");
      expect(result[0].content).toContain("summarized");
    });

    it("should include recent messages after summary", () => {
      const history = Array(15)
        .fill(null)
        .map((_, i) => ({
          role: "user",
          content: `Message ${i}`,
        }));

      const result = contextCompressor.compressHistory(history, 10);
      // Should have summary + recent messages
      expect(result[result.length - 1].content).toContain("Message 14");
    });
  });

  describe("getStats()", () => {
    it("should calculate compression statistics", () => {
      const compressionResult = {
        originalSize: 1000,
        compressedSize: 300,
        ratio: 0.3,
      };

      const stats = contextCompressor.getStats(compressionResult);
      expect(stats.savings).toBe(700);
      expect(stats.savingsPercent).toBe("70.0");
    });

    it("should handle zero original size", () => {
      const stats = contextCompressor.getStats({
        originalSize: 0,
        compressedSize: 0,
        ratio: 0,
      });

      expect(stats.savingsPercent).toBe(0);
    });
  });

  describe("Edge Cases", () => {
    it("should handle empty tree", () => {
      const result = contextCompressor.compressAXTree({});
      expect(result.summary).toBeDefined();
    });

    it("should handle tree with no children", () => {
      const tree = { role: "root", name: "Page" };
      const result = contextCompressor.compressAXTree(tree);
      expect(result.summary.interactiveElements).toEqual([]);
    });
  });
});
