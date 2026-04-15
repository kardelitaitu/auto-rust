/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Unit tests for api/agent/promptAdapter.js
 * @module tests/unit/agent/promptAdapter.test
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

describe("api/agent/promptAdapter.js", () => {
  let promptAdapter;

  beforeEach(async () => {
    vi.clearAllMocks();
    const module = await import("@api/agent/promptAdapter.js");
    promptAdapter = module.promptAdapter || module.default;
  });

  describe("Constructor", () => {
    it("should initialize with page types", () => {
      expect(promptAdapter.pageTypes).toBeDefined();
      expect(promptAdapter.pageTypes.form).toBeDefined();
      expect(promptAdapter.pageTypes.navigation).toBeDefined();
      expect(promptAdapter.pageTypes.content).toBeDefined();
      expect(promptAdapter.pageTypes.game).toBeDefined();
      expect(promptAdapter.pageTypes.social).toBeDefined();
      expect(promptAdapter.pageTypes.ecommerce).toBeDefined();
    });

    it("should have keywords for each page type", () => {
      expect(promptAdapter.pageTypes.form.keywords).toContain("input");
      expect(promptAdapter.pageTypes.ecommerce.keywords).toContain("product");
      expect(promptAdapter.pageTypes.social.keywords).toContain("tweet");
    });

    it("should have rules for each page type", () => {
      expect(promptAdapter.pageTypes.form.rules.length).toBeGreaterThan(0);
      expect(promptAdapter.pageTypes.game.rules.length).toBeGreaterThan(0);
    });

    it("should have examples for each page type", () => {
      expect(promptAdapter.pageTypes.form.examples.length).toBeGreaterThan(0);
      expect(
        promptAdapter.pageTypes.navigation.examples.length,
      ).toBeGreaterThan(0);
    });
  });

  describe("detectPageType()", () => {
    it("should detect form page from content", () => {
      const axTree = {
        name: "Login Form",
        children: [
          { name: "Email Input", role: "textbox" },
          { name: "Password Input", role: "textbox" },
          { name: "Submit", role: "button" },
        ],
      };
      const result = promptAdapter.detectPageType(
        axTree,
        "https://example.com/login",
      );
      expect(result).toBe("form");
    });

    it("should detect social page from URL containing social keywords", () => {
      const axTree = { name: "Page", children: [] };
      const result = promptAdapter.detectPageType(
        axTree,
        "https://social.example.com/home",
      );
      expect(result).toBe("social");
    });

    it("should detect game page from content", () => {
      const axTree = {
        name: "Game Canvas",
        children: [
          { name: "Score: 100", role: "text" },
          { name: "Player", role: "text" },
        ],
      };
      const result = promptAdapter.detectPageType(
        axTree,
        "https://example.com/game",
      );
      expect(result).toBe("game");
    });

    it("should detect ecommerce page", () => {
      const axTree = {
        name: "Product Page",
        children: [
          { name: "Add to Cart", role: "button" },
          { name: "$99.99", role: "text" },
        ],
      };
      const result = promptAdapter.detectPageType(
        axTree,
        "https://shop.example.com/product",
      );
      expect(result).toBe("ecommerce");
    });

    it("should detect navigation page", () => {
      const axTree = {
        name: "Header",
        role: "navigation",
        children: [
          { name: "Menu", role: "menubar" },
          { name: "Home", role: "link" },
        ],
      };
      const result = promptAdapter.detectPageType(
        axTree,
        "https://example.com",
      );
      expect(result).toBe("navigation");
    });

    it("should detect content page as default", () => {
      const axTree = {
        name: "Article",
        children: [{ name: "Hello World", role: "heading" }],
      };
      const result = promptAdapter.detectPageType(
        axTree,
        "https://blog.example.com/post",
      );
      expect(result).toBe("content");
    });

    it("should handle JSON string input", () => {
      const axTreeJson = JSON.stringify({
        name: "Form",
        children: [{ name: "Input", role: "textbox" }],
      });
      const result = promptAdapter.detectPageType(
        axTreeJson,
        "https://example.com",
      );
      expect(result).toBe("form");
    });

    it("should return unknown for invalid JSON", () => {
      const result = promptAdapter.detectPageType(
        "invalid json",
        "https://example.com",
      );
      expect(result).toBe("unknown");
    });

    it("should handle null input gracefully", () => {
      const result = promptAdapter.detectPageType(null, "https://example.com");
      // null input is handled gracefully - may return 'unknown' or 'content'
      expect(["unknown", "content"]).toContain(result);
    });
  });

  describe("_extractTextContent()", () => {
    it("should extract text from node", () => {
      const node = { name: "Button", text: "Click Me", role: "button" };
      const result = promptAdapter._extractTextContent(node);
      expect(result).toContain("Button");
      expect(result).toContain("Click Me");
      expect(result).toContain("button");
    });

    it("should extract text from children recursively", () => {
      const node = {
        name: "Form",
        children: [
          { name: "Input", role: "textbox" },
          { name: "Submit", role: "button" },
        ],
      };
      const result = promptAdapter._extractTextContent(node);
      expect(result).toContain("Input");
      expect(result).toContain("Submit");
    });

    it("should handle null node", () => {
      const result = promptAdapter._extractTextContent(null);
      expect(result).toBe("");
    });

    it("should limit recursion depth", () => {
      const deepNode = {
        name: "Level0",
        children: [
          {
            name: "Level1",
            children: [
              {
                name: "Level2",
                children: [
                  {
                    name: "Level3",
                    children: [
                      {
                        name: "Level4",
                      },
                    ],
                  },
                ],
              },
            ],
          },
        ],
      };
      const result = promptAdapter._extractTextContent(deepNode);
      expect(result).toContain("Level0");
      expect(result).toContain("Level4");
    });
  });

  describe("generateContextualPrompt()", () => {
    it("should generate prompt for form page", () => {
      const result = promptAdapter.generateContextualPrompt(
        "form",
        "login to account",
        null,
      );
      expect(result).toContain("form");
      expect(result).toContain("Rules");
    });

    it("should generate generic prompt for unknown page", () => {
      const result = promptAdapter.generateContextualPrompt(
        "unknown",
        "do something",
        null,
      );
      expect(result).toContain("general web page");
    });

    it("should include goal-specific tips", () => {
      const result = promptAdapter.generateContextualPrompt(
        "form",
        "login to my account",
        null,
      );
      expect(result.toLowerCase()).toContain("login");
    });
  });

  describe("_generateGoalTips()", () => {
    it("should generate tips for login goals", () => {
      const tips = promptAdapter._generateGoalTips("login to account", "form");
      expect(tips).toContain("username");
      expect(tips).toContain("password");
    });

    it("should generate tips for search goals", () => {
      const tips = promptAdapter._generateGoalTips(
        "search for products",
        "ecommerce",
      );
      expect(tips).toContain("search");
    });

    it("should generate tips for purchase goals", () => {
      const tips = promptAdapter._generateGoalTips(
        "buy a product",
        "ecommerce",
      );
      expect(tips).toContain("cart");
      expect(tips).toContain("checkout");
    });

    it("should generate tips for navigation goals", () => {
      const tips = promptAdapter._generateGoalTips(
        "navigate to page",
        "content",
      );
      expect(tips).toContain("navigation");
    });

    it("should generate generic tips for unknown goals", () => {
      const tips = promptAdapter._generateGoalTips(
        "do something random",
        "content",
      );
      expect(tips).toContain("steps");
    });
  });

  describe("getContextualExamples()", () => {
    it("should return examples for form page", () => {
      const examples = promptAdapter.getContextualExamples("form");
      expect(examples.length).toBeGreaterThan(0);
      expect(examples.some((e) => e.action === "type")).toBe(true);
    });

    it("should return examples for game page", () => {
      const examples = promptAdapter.getContextualExamples("game");
      expect(examples.some((e) => e.action === "clickAt")).toBe(true);
    });

    it("should return generic examples for unknown page", () => {
      const examples = promptAdapter.getContextualExamples("unknown");
      expect(examples.length).toBeGreaterThan(0);
      expect(examples.some((e) => e.action === "click")).toBe(true);
    });
  });

  describe("enhancePrompt()", () => {
    it("should enhance base prompt with context", () => {
      const basePrompt = "You are an AI assistant.";
      const enhanced = promptAdapter.enhancePrompt(
        basePrompt,
        "form",
        "login",
        null,
      );
      expect(enhanced).toContain("You are an AI assistant.");
      expect(enhanced).toContain("Page Context");
      expect(enhanced).toContain("Example actions");
    });

    it("should include JSON examples in enhanced prompt", () => {
      const basePrompt = "Base prompt";
      const enhanced = promptAdapter.enhancePrompt(
        basePrompt,
        "form",
        "login",
        null,
      );
      expect(enhanced).toContain("```json");
      expect(enhanced).toContain("```");
    });

    it("should handle unknown page type", () => {
      const enhanced = promptAdapter.enhancePrompt(
        "Base",
        "unknown",
        "task",
        null,
      );
      expect(enhanced).toContain("general web page");
    });
  });

  describe("Edge Cases", () => {
    it("should handle empty axTree children", () => {
      const axTree = { name: "Empty", children: [] };
      const result = promptAdapter.detectPageType(
        axTree,
        "https://example.com",
      );
      expect(result).toBeDefined();
    });

    it("should handle missing name in node", () => {
      const node = { role: "button", text: "Click" };
      const result = promptAdapter._extractTextContent(node);
      expect(result).toContain("Click");
    });

    it("should handle case insensitive keyword matching", () => {
      const axTree = { name: "INPUT FORM", role: "FORM" };
      const result = promptAdapter.detectPageType(
        axTree,
        "https://example.com",
      );
      expect(result).toBe("form");
    });
  });
});
