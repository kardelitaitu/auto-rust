/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Semantic Mapping System
 * Enriches AXTree with semantic understanding for better LLM context
 * @module api/agent/semanticMapper
 */

import { createLogger } from "../core/logger.js";

const logger = createLogger("api/agent/semanticMapper.js");

class SemanticMapper {
  constructor() {
    this.elementPatterns = {
      // Navigation elements
      navigation: {
        keywords: ["menu", "nav", "sidebar", "header", "footer", "breadcrumb"],
        semanticTags: ["navigation", "structural"],
        suggestedActions: ["click", "scroll"],
      },
      // Input elements
      input: {
        keywords: ["input", "textfield", "textarea", "search", "box"],
        semanticTags: ["input", "interactive"],
        suggestedActions: ["click", "type"],
      },
      // Button elements
      button: {
        keywords: ["button", "btn", "submit", "send", "post", "tweet"],
        semanticTags: ["action", "interactive"],
        suggestedActions: ["click"],
      },
      // Link elements
      link: {
        keywords: ["link", "href", "anchor", "a "],
        semanticTags: ["navigation", "interactive"],
        suggestedActions: ["click"],
      },
      // Media elements
      media: {
        keywords: ["image", "img", "video", "media", "photo"],
        semanticTags: ["content", "visual"],
        suggestedActions: ["click", "scroll"],
      },
      // Form elements
      form: {
        keywords: ["form", "fieldset", "form-group"],
        semanticTags: ["input", "structural"],
        suggestedActions: ["click", "type"],
      },
      // List elements
      list: {
        keywords: ["list", "ul", "ol", "li", "item"],
        semanticTags: ["content", "structural"],
        suggestedActions: ["click", "scroll"],
      },
      // Card elements
      card: {
        keywords: ["card", "panel", "tile", "widget"],
        semanticTags: ["content", "container"],
        suggestedActions: ["click", "scroll"],
      },
    };

    this.actionConfidence = {
      click: {
        button: 0.95,
        link: 0.9,
        navigation: 0.85,
        card: 0.7,
        input: 0.6,
      },
      type: {
        input: 0.95,
        textarea: 0.95,
        form: 0.7,
      },
      scroll: {
        list: 0.8,
        card: 0.75,
        navigation: 0.6,
      },
    };
  }

  /**
   * Enrich AXTree with semantic information
   * @param {object} tree - AXTree snapshot
   * @returns {object} Enriched tree with semantic tags
   */
  enrichAXTree(tree) {
    if (!tree) return null;

    const enriched = this._traverseAndEnrich(tree);
    logger.info(`[SemanticMapper] Enriched AXTree with semantic tags`);
    return enriched;
  }

  /**
   * Recursively traverse and enrich tree nodes
   * @private
   */
  _traverseAndEnrich(node, depth = 0) {
    if (!node) return null;

    const enriched = { ...node };

    // Add semantic tags
    enriched.semanticTags = this._identifySemantics(node);

    // Add confidence scores for different actions
    enriched.clickConfidence = this._calculateClickConfidence(node);
    enriched.typeConfidence = this._calculateTypeConfidence(node);
    enriched.scrollConfidence = this._calculateScrollConfidence(node);

    // Add suggested actions based on semantics
    enriched.suggestedActions = this._suggestActions(node);

    // Add element category
    enriched.category = this._categorizeElement(node);

    // Process children recursively
    if (node.children && depth < 5) {
      // Limit depth
      enriched.children = node.children
        .map((child) => this._traverseAndEnrich(child, depth + 1))
        .filter(Boolean);
    }

    return enriched;
  }

  /**
   * Identify semantic tags for an element
   * @private
   */
  _identifySemantics(node) {
    const tags = new Set();
    const role = (node.role || "").toLowerCase();
    const name = (node.name || "").toLowerCase();
    const text = (node.text || "").toLowerCase();

    // Check role-based semantics
    if (role.includes("button") || role.includes("link")) {
      tags.add("interactive");
      tags.add("clickable");
    }

    if (role.includes("textbox") || role.includes("input")) {
      tags.add("input");
      tags.add("interactive");
    }

    if (role.includes("heading")) {
      tags.add("heading");
      tags.add("structural");
    }

    if (role.includes("list") || role.includes("menu")) {
      tags.add("navigation");
      tags.add("structural");
    }

    // Check name/text-based semantics
    const combinedText = `${name} ${text}`;

    for (const [_category, pattern] of Object.entries(this.elementPatterns)) {
      for (const keyword of pattern.keywords) {
        if (combinedText.includes(keyword)) {
          pattern.semanticTags.forEach((tag) => tags.add(tag));
          break;
        }
      }
    }

    // Special cases
    if (combinedText.includes("login") || combinedText.includes("sign in")) {
      tags.add("auth");
      tags.add("critical");
    }

    if (combinedText.includes("search")) {
      tags.add("search");
      tags.add("input");
    }

    if (
      combinedText.includes("submit") ||
      combinedText.includes("post") ||
      combinedText.includes("tweet")
    ) {
      tags.add("action");
      tags.add("submit");
    }

    return Array.from(tags);
  }

  /**
   * Calculate confidence score for click action
   * @private
   */
  _calculateClickConfidence(node) {
    const role = (node.role || "").toLowerCase();
    const name = (node.name || "").toLowerCase();

    // High confidence for buttons and links
    if (role.includes("button") || role.includes("link")) return 0.95;

    // Medium confidence for clickable-looking elements
    if (role.includes("tab") || role.includes("menuitem")) return 0.85;

    // Check name for action words
    const actionWords = ["click", "open", "start", "go", "view", "see", "read"];
    for (const word of actionWords) {
      if (name.includes(word)) return 0.8;
    }

    // Lower confidence for generic elements
    if (role.includes("generic") || role.includes("text")) return 0.4;

    return 0.5; // Default
  }

  /**
   * Calculate confidence score for type action
   * @private
   */
  _calculateTypeConfidence(node) {
    const role = (node.role || "").toLowerCase();

    if (role.includes("textbox") || role.includes("input")) return 0.95;
    if (role.includes("combobox")) return 0.9;
    if (role.includes("searchbox")) return 0.95;

    return 0.2; // Low confidence for non-input elements
  }

  /**
   * Calculate confidence score for scroll action
   * @private
   */
  _calculateScrollConfidence(node) {
    const role = (node.role || "").toLowerCase();

    if (role.includes("list") || role.includes("feed")) return 0.85;
    if (role.includes("article") || role.includes("document")) return 0.75;
    if (role.includes("main") || role.includes("region")) return 0.7;

    return 0.4; // Default
  }

  /**
   * Suggest actions based on element semantics
   * @private
   */
  _suggestActions(node) {
    const suggestions = [];
    const tags = this._identifySemantics(node);

    if (tags.includes("interactive") || tags.includes("clickable")) {
      suggestions.push({
        action: "click",
        confidence: this._calculateClickConfidence(node),
      });
    }

    if (tags.includes("input")) {
      suggestions.push({
        action: "type",
        confidence: this._calculateTypeConfidence(node),
      });
    }

    if (tags.includes("structural") || tags.includes("content")) {
      suggestions.push({
        action: "scroll",
        confidence: this._calculateScrollConfidence(node),
      });
    }

    // Sort by confidence
    return suggestions.sort((a, b) => b.confidence - a.confidence);
  }

  /**
   * Categorize element type
   * @private
   */
  _categorizeElement(node) {
    const role = (node.role || "").toLowerCase();

    if (role.includes("button")) return "button";
    if (role.includes("link")) return "link";
    if (role.includes("textbox") || role.includes("input")) return "input";
    if (role.includes("heading")) return "heading";
    if (role.includes("list")) return "list";
    if (role.includes("img") || role.includes("image")) return "image";
    if (role.includes("article")) return "article";
    if (role.includes("navigation")) return "navigation";

    return "generic";
  }

  /**
   * Get best action for an element
   * @param {object} node - Enriched tree node
   * @returns {string|null} Best action or null
   */
  getBestAction(node) {
    if (!node.suggestedActions || node.suggestedActions.length === 0) {
      return null;
    }

    return node.suggestedActions[0].action;
  }

  /**
   * Get semantic summary of page
   * @param {object} enrichedTree - Enriched AXTree
   * @returns {object} Summary statistics
   */
  getPageSummary(enrichedTree) {
    const summary = {
      totalElements: 0,
      interactiveElements: 0,
      inputElements: 0,
      clickableElements: 0,
      categories: {},
      topActions: {},
    };

    this._countElements(enrichedTree, summary);

    return summary;
  }

  /**
   * Count elements recursively
   * @private
   */
  _countElements(node, summary) {
    if (!node) return;

    summary.totalElements++;

    if (node.category) {
      summary.categories[node.category] =
        (summary.categories[node.category] || 0) + 1;
    }

    if (node.clickConfidence > 0.7) {
      summary.clickableElements++;
    }

    if (node.typeConfidence > 0.7) {
      summary.inputElements++;
    }

    if (node.suggestedActions && node.suggestedActions.length > 0) {
      summary.interactiveElements++;
      const bestAction = node.suggestedActions[0].action;
      summary.topActions[bestAction] =
        (summary.topActions[bestAction] || 0) + 1;
    }

    if (node.children) {
      for (const child of node.children) {
        this._countElements(child, summary);
      }
    }
  }
}

const semanticMapper = new SemanticMapper();

export { semanticMapper };
export default semanticMapper;
