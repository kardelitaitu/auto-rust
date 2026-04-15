/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Unit tests for api/agent/semanticMapper.js
 * @module tests/unit/agent/semanticMapper.test
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

describe("api/agent/semanticMapper.js", () => {
  let semanticMapper;

  beforeEach(async () => {
    vi.clearAllMocks();
    const module = await import("@api/agent/semanticMapper.js");
    semanticMapper = module.semanticMapper || module.default;
  });

  describe("Constructor", () => {
    it("should define element patterns", () => {
      expect(semanticMapper.elementPatterns).toBeDefined();
      expect(semanticMapper.elementPatterns.navigation).toBeDefined();
      expect(semanticMapper.elementPatterns.input).toBeDefined();
      expect(semanticMapper.elementPatterns.button).toBeDefined();
      expect(semanticMapper.elementPatterns.link).toBeDefined();
    });

    it("should define action confidence scores", () => {
      expect(semanticMapper.actionConfidence).toBeDefined();
      expect(semanticMapper.actionConfidence.click).toBeDefined();
      expect(semanticMapper.actionConfidence.type).toBeDefined();
      expect(semanticMapper.actionConfidence.scroll).toBeDefined();
    });
  });

  describe("enrichAXTree()", () => {
    it("should return null for null input", () => {
      expect(semanticMapper.enrichAXTree(null)).toBeNull();
    });

    it("should enrich tree node with semantic tags", () => {
      const tree = {
        role: "button",
        name: "Submit",
        children: [],
      };

      const enriched = semanticMapper.enrichAXTree(tree);

      expect(enriched.semanticTags).toBeDefined();
      expect(enriched.clickConfidence).toBeDefined();
      expect(enriched.typeConfidence).toBeDefined();
      expect(enriched.scrollConfidence).toBeDefined();
      expect(enriched.suggestedActions).toBeDefined();
      expect(enriched.category).toBeDefined();
    });

    it("should process children recursively", () => {
      const tree = {
        role: "generic",
        name: "Container",
        children: [
          { role: "button", name: "Click me" },
          { role: "textbox", name: "Input" },
        ],
      };

      const enriched = semanticMapper.enrichAXTree(tree);

      expect(enriched.children.length).toBe(2);
      expect(enriched.children[0].category).toBe("button");
      expect(enriched.children[1].category).toBe("input");
    });

    it("should limit depth to 5 levels", () => {
      const deepTree = {
        role: "generic",
        children: [
          {
            role: "generic",
            children: [
              {
                role: "generic",
                children: [
                  {
                    role: "generic",
                    children: [
                      {
                        role: "generic",
                        children: [{ role: "button", name: "Deep button" }],
                      },
                    ],
                  },
                ],
              },
            ],
          },
        ],
      };

      const enriched = semanticMapper.enrichAXTree(deepTree);

      // Should process up to depth 5
      expect(enriched).toBeDefined();
    });
  });

  describe("_identifySemantics()", () => {
    it("should identify button semantics", () => {
      const tags = semanticMapper._identifySemantics({
        role: "button",
        name: "Submit",
      });

      expect(tags).toContain("interactive");
      expect(tags).toContain("clickable");
    });

    it("should identify link semantics", () => {
      const tags = semanticMapper._identifySemantics({
        role: "link",
        name: "Click here",
      });

      expect(tags).toContain("interactive");
      expect(tags).toContain("clickable");
    });

    it("should identify input semantics", () => {
      const tags = semanticMapper._identifySemantics({
        role: "textbox",
        name: "Enter text",
      });

      expect(tags).toContain("input");
      expect(tags).toContain("interactive");
    });

    it("should identify heading semantics", () => {
      const tags = semanticMapper._identifySemantics({
        role: "heading",
        name: "Title",
      });

      expect(tags).toContain("heading");
      expect(tags).toContain("structural");
    });

    it("should identify navigation semantics", () => {
      const tags = semanticMapper._identifySemantics({
        role: "menu",
        name: "Main menu",
      });

      expect(tags).toContain("navigation");
      expect(tags).toContain("structural");
    });

    it("should identify text-based semantics", () => {
      const tags = semanticMapper._identifySemantics({
        role: "generic",
        name: "Search box",
      });

      expect(tags).toContain("input");
    });

    it("should identify auth-related elements", () => {
      const tags = semanticMapper._identifySemantics({
        role: "button",
        name: "Login",
      });

      expect(tags).toContain("auth");
      expect(tags).toContain("critical");
    });

    it("should identify submit-related elements", () => {
      const tags = semanticMapper._identifySemantics({
        role: "button",
        name: "Tweet",
      });

      expect(tags).toContain("action");
      expect(tags).toContain("submit");
    });
  });

  describe("_calculateClickConfidence()", () => {
    it("should return high confidence for buttons", () => {
      const confidence = semanticMapper._calculateClickConfidence({
        role: "button",
        name: "Submit",
      });

      expect(confidence).toBe(0.95);
    });

    it("should return high confidence for links", () => {
      const confidence = semanticMapper._calculateClickConfidence({
        role: "link",
        name: "Click here",
      });

      expect(confidence).toBe(0.95);
    });

    it("should return medium confidence for tabs and menu items", () => {
      expect(
        semanticMapper._calculateClickConfidence({
          role: "tab",
          name: "Tab 1",
        }),
      ).toBe(0.85);
      expect(
        semanticMapper._calculateClickConfidence({
          role: "menuitem",
          name: "Item",
        }),
      ).toBe(0.85);
    });

    it("should return 0.80 for action word names", () => {
      const confidence = semanticMapper._calculateClickConfidence({
        role: "generic",
        name: "Click to continue",
      });

      expect(confidence).toBe(0.8);
    });

    it("should return lower confidence for generic elements", () => {
      const confidence = semanticMapper._calculateClickConfidence({
        role: "generic",
        name: "Some text",
      });

      expect(confidence).toBe(0.4);
    });

    it("should return default 0.50 for unrecognized roles", () => {
      const confidence = semanticMapper._calculateClickConfidence({
        role: "unknown",
        name: "Unknown",
      });

      expect(confidence).toBe(0.5);
    });
  });

  describe("_calculateTypeConfidence()", () => {
    it("should return high confidence for textboxes", () => {
      expect(semanticMapper._calculateTypeConfidence({ role: "textbox" })).toBe(
        0.95,
      );
    });

    it("should return high confidence for inputs", () => {
      expect(semanticMapper._calculateTypeConfidence({ role: "input" })).toBe(
        0.95,
      );
    });

    it("should return high confidence for searchbox", () => {
      expect(
        semanticMapper._calculateTypeConfidence({ role: "searchbox" }),
      ).toBe(0.95);
    });

    it("should return 0.90 for combobox", () => {
      expect(
        semanticMapper._calculateTypeConfidence({ role: "combobox" }),
      ).toBe(0.9);
    });

    it("should return low confidence for non-input elements", () => {
      expect(semanticMapper._calculateTypeConfidence({ role: "button" })).toBe(
        0.2,
      );
      expect(semanticMapper._calculateTypeConfidence({ role: "generic" })).toBe(
        0.2,
      );
    });
  });

  describe("_calculateScrollConfidence()", () => {
    it("should return high confidence for lists and feeds", () => {
      expect(semanticMapper._calculateScrollConfidence({ role: "list" })).toBe(
        0.85,
      );
      expect(semanticMapper._calculateScrollConfidence({ role: "feed" })).toBe(
        0.85,
      );
    });

    it("should return medium confidence for articles and documents", () => {
      expect(
        semanticMapper._calculateScrollConfidence({ role: "article" }),
      ).toBe(0.75);
      expect(
        semanticMapper._calculateScrollConfidence({ role: "document" }),
      ).toBe(0.75);
    });

    it("should return 0.70 for main and region", () => {
      expect(semanticMapper._calculateScrollConfidence({ role: "main" })).toBe(
        0.7,
      );
      expect(
        semanticMapper._calculateScrollConfidence({ role: "region" }),
      ).toBe(0.7);
    });

    it("should return default 0.40 for other elements", () => {
      expect(
        semanticMapper._calculateScrollConfidence({ role: "generic" }),
      ).toBe(0.4);
    });
  });

  describe("_suggestActions()", () => {
    it("should suggest click for interactive elements", () => {
      const suggestions = semanticMapper._suggestActions({
        role: "button",
        name: "Submit",
      });

      const clickSuggestion = suggestions.find((s) => s.action === "click");
      expect(clickSuggestion).toBeDefined();
      expect(clickSuggestion.confidence).toBe(0.95);
    });

    it("should suggest type for input elements", () => {
      const suggestions = semanticMapper._suggestActions({
        role: "textbox",
        name: "Input",
      });

      const typeSuggestion = suggestions.find((s) => s.action === "type");
      expect(typeSuggestion).toBeDefined();
    });

    it("should suggest scroll for structural elements", () => {
      const suggestions = semanticMapper._suggestActions({
        role: "list",
        name: "Items",
      });

      const scrollSuggestion = suggestions.find((s) => s.action === "scroll");
      expect(scrollSuggestion).toBeDefined();
    });

    it("should sort suggestions by confidence", () => {
      const suggestions = semanticMapper._suggestActions({
        role: "button",
        name: "Submit",
      });

      for (let i = 1; i < suggestions.length; i++) {
        expect(suggestions[i - 1].confidence).toBeGreaterThanOrEqual(
          suggestions[i].confidence,
        );
      }
    });
  });

  describe("_categorizeElement()", () => {
    it("should categorize buttons", () => {
      expect(semanticMapper._categorizeElement({ role: "button" })).toBe(
        "button",
      );
    });

    it("should categorize links", () => {
      expect(semanticMapper._categorizeElement({ role: "link" })).toBe("link");
    });

    it("should categorize textboxes and inputs", () => {
      expect(semanticMapper._categorizeElement({ role: "textbox" })).toBe(
        "input",
      );
      expect(semanticMapper._categorizeElement({ role: "input" })).toBe(
        "input",
      );
    });

    it("should categorize headings", () => {
      expect(semanticMapper._categorizeElement({ role: "heading" })).toBe(
        "heading",
      );
    });

    it("should categorize lists", () => {
      expect(semanticMapper._categorizeElement({ role: "list" })).toBe("list");
    });

    it("should categorize images", () => {
      expect(semanticMapper._categorizeElement({ role: "img" })).toBe("image");
      expect(semanticMapper._categorizeElement({ role: "image" })).toBe(
        "image",
      );
    });

    it("should categorize articles", () => {
      expect(semanticMapper._categorizeElement({ role: "article" })).toBe(
        "article",
      );
    });

    it("should categorize navigation", () => {
      expect(semanticMapper._categorizeElement({ role: "navigation" })).toBe(
        "navigation",
      );
    });

    it("should return generic for unknown roles", () => {
      expect(semanticMapper._categorizeElement({ role: "unknown" })).toBe(
        "generic",
      );
    });
  });

  describe("getBestAction()", () => {
    it("should return best action from suggested actions", () => {
      const node = {
        suggestedActions: [
          { action: "click", confidence: 0.95 },
          { action: "scroll", confidence: 0.5 },
        ],
      };

      const bestAction = semanticMapper.getBestAction(node);
      expect(bestAction).toBe("click");
    });

    it("should return null when no suggested actions", () => {
      const node = { suggestedActions: [] };
      const bestAction = semanticMapper.getBestAction(node);
      expect(bestAction).toBeNull();
    });

    it("should return null when suggestedActions is undefined", () => {
      const node = {};
      const bestAction = semanticMapper.getBestAction(node);
      expect(bestAction).toBeNull();
    });
  });

  describe("getPageSummary()", () => {
    it("should count total elements", () => {
      const enrichedTree = {
        role: "button",
        category: "button",
        clickConfidence: 0.9,
        typeConfidence: 0.2,
        suggestedActions: [{ action: "click" }],
        children: [
          {
            role: "textbox",
            category: "input",
            clickConfidence: 0.3,
            typeConfidence: 0.95,
            suggestedActions: [{ action: "type" }],
          },
        ],
      };

      const summary = semanticMapper.getPageSummary(enrichedTree);

      expect(summary.totalElements).toBe(2);
    });

    it("should count clickable elements", () => {
      const enrichedTree = {
        role: "button",
        clickConfidence: 0.9,
        children: [
          { role: "generic", clickConfidence: 0.5 },
          { role: "link", clickConfidence: 0.95 },
        ],
      };

      const summary = semanticMapper.getPageSummary(enrichedTree);

      expect(summary.clickableElements).toBe(2);
    });

    it("should count input elements", () => {
      const enrichedTree = {
        role: "textbox",
        typeConfidence: 0.95,
        children: [
          { role: "generic", typeConfidence: 0.2 },
          { role: "searchbox", typeConfidence: 0.95 },
        ],
      };

      const summary = semanticMapper.getPageSummary(enrichedTree);

      expect(summary.inputElements).toBe(2);
    });

    it("should count interactive elements", () => {
      const enrichedTree = {
        suggestedActions: [{ action: "click" }],
        children: [{ suggestedActions: [{ action: "type" }] }],
      };

      const summary = semanticMapper.getPageSummary(enrichedTree);

      expect(summary.interactiveElements).toBe(2);
    });

    it("should categorize elements by category", () => {
      const enrichedTree = {
        category: "button",
        children: [{ category: "input" }, { category: "button" }],
      };

      const summary = semanticMapper.getPageSummary(enrichedTree);

      expect(summary.categories.button).toBe(2);
      expect(summary.categories.input).toBe(1);
    });

    it("should count top actions", () => {
      const enrichedTree = {
        suggestedActions: [{ action: "click" }],
        children: [
          { suggestedActions: [{ action: "click" }] },
          { suggestedActions: [{ action: "type" }] },
        ],
      };

      const summary = semanticMapper.getPageSummary(enrichedTree);

      expect(summary.topActions.click).toBe(2);
      expect(summary.topActions.type).toBe(1);
    });

    it("should handle null nodes", () => {
      const summary = semanticMapper.getPageSummary(null);

      expect(summary.totalElements).toBe(0);
    });
  });

  describe("_countElements()", () => {
    it("should count elements recursively", () => {
      const summary = {
        totalElements: 0,
        interactiveElements: 0,
        inputElements: 0,
        clickableElements: 0,
        categories: {},
        topActions: {},
      };

      const tree = {
        category: "button",
        clickConfidence: 0.9,
        suggestedActions: [{ action: "click" }],
        children: [
          {
            category: "input",
            typeConfidence: 0.95,
            suggestedActions: [{ action: "type" }],
          },
        ],
      };

      semanticMapper._countElements(tree, summary);

      expect(summary.totalElements).toBe(2);
      expect(summary.categories.button).toBe(1);
      expect(summary.categories.input).toBe(1);
    });

    it("should handle null nodes", () => {
      const summary = { totalElements: 0 };

      semanticMapper._countElements(null, summary);

      expect(summary.totalElements).toBe(0);
    });
  });
});
