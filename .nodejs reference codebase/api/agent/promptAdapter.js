/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Prompt Adapter for dynamic prompt generation
 * Adapts prompts based on page type and context
 * @module api/agent/promptAdapter
 */

import { createLogger } from "../core/logger.js";

const logger = createLogger("api/agent/promptAdapter.js");

class PromptAdapter {
  constructor() {
    this.pageTypes = {
      form: {
        keywords: [
          "input",
          "textarea",
          "select",
          "button",
          "submit",
          "form",
          "login",
          "register",
          "signup",
        ],
        rules: [
          "Always fill required fields before submitting",
          "Check for validation errors after submission",
          "Use type action for text inputs, click for buttons",
        ],
        examples: [
          {
            action: "type",
            selector: "#email",
            value: "user@example.com",
            rationale: "Fill email field",
          },
          {
            action: "click",
            selector: "#submit-btn",
            rationale: "Submit the form",
          },
        ],
      },
      navigation: {
        keywords: [
          "nav",
          "menu",
          "sidebar",
          "header",
          "footer",
          "link",
          "breadcrumb",
        ],
        rules: [
          "Click navigation links to explore",
          "Use scroll to see more menu items",
          "Verify page changed after navigation",
        ],
        examples: [
          {
            action: "click",
            selector: 'nav a[href="/products"]',
            rationale: "Navigate to products page",
          },
          {
            action: "verify",
            description: "Check if products page loaded",
            rationale: "Confirm navigation",
          },
        ],
      },
      content: {
        keywords: [
          "article",
          "post",
          "content",
          "text",
          "paragraph",
          "heading",
          "image",
        ],
        rules: [
          "Scroll to read more content",
          "Click on articles to read full content",
          "Use verify to confirm content loaded",
        ],
        examples: [
          { action: "scroll", value: "down", rationale: "Read more content" },
          {
            action: "click",
            selector: ".read-more",
            rationale: "Open full article",
          },
        ],
      },
      game: {
        keywords: [
          "game",
          "canvas",
          "score",
          "level",
          "player",
          "unit",
          "build",
          "train",
        ],
        rules: [
          "Use clickAt for canvas-based games",
          "Wait for animations to complete",
          "Verify actions before proceeding",
        ],
        examples: [
          {
            action: "clickAt",
            x: 450,
            y: 320,
            rationale: "Click on game unit",
          },
          { action: "wait", value: "1000", rationale: "Wait for animation" },
        ],
      },
      social: {
        keywords: [
          "tweet",
          "post",
          "share",
          "like",
          "follow",
          "comment",
          "reply",
          "retweet",
        ],
        rules: [
          "Compose messages carefully",
          "Verify post was published",
          "Check character limits",
        ],
        examples: [
          {
            action: "type",
            selector: ".tweet-box",
            value: "Hello world!",
            rationale: "Compose tweet",
          },
          {
            action: "click",
            selector: ".tweet-btn",
            rationale: "Publish tweet",
          },
        ],
      },
      ecommerce: {
        keywords: [
          "product",
          "cart",
          "checkout",
          "price",
          "buy",
          "add to cart",
          "order",
        ],
        rules: [
          "Add items to cart before checkout",
          "Fill shipping information",
          "Review order before confirming",
        ],
        examples: [
          {
            action: "click",
            selector: ".add-to-cart",
            rationale: "Add product to cart",
          },
          {
            action: "click",
            selector: ".checkout-btn",
            rationale: "Proceed to checkout",
          },
        ],
      },
    };
  }

  /**
   * Detect page type from AXTree and URL
   * @param {string} axTree - AXTree JSON string
   * @param {string} url - Current page URL
   * @returns {string} Page type (form, navigation, content, game, social, ecommerce, unknown)
   */
  detectPageType(axTree, url) {
    try {
      // Parse AXTree if it's a string
      const tree = typeof axTree === "string" ? JSON.parse(axTree) : axTree;

      // Extract text content from tree
      const textContent = this._extractTextContent(tree).toLowerCase();

      // Check URL for clues
      const urlLower = url.toLowerCase();

      // Score each page type
      const scores = {};

      for (const [type, config] of Object.entries(this.pageTypes)) {
        scores[type] = 0;

        // Check keywords in text content
        for (const keyword of config.keywords) {
          if (textContent.includes(keyword)) {
            scores[type] += 1;
          }
        }

        // Check URL for type indicators
        if (urlLower.includes(type)) {
          scores[type] += 2;
        }
      }

      // Find highest score
      let maxScore = 0;
      let detectedType = "unknown";

      for (const [type, score] of Object.entries(scores)) {
        if (score > maxScore) {
          maxScore = score;
          detectedType = type;
        }
      }

      // If no clear match, default to content
      if (maxScore === 0) {
        detectedType = "content";
      }

      logger.info(
        `[PromptAdapter] Detected page type: ${detectedType} (score: ${maxScore})`,
      );
      return detectedType;
    } catch (error) {
      logger.warn("[PromptAdapter] Failed to detect page type:", error.message);
      return "unknown";
    }
  }

  /**
   * Extract text content from AXTree recursively
   * @private
   */
  _extractTextContent(node, depth = 0) {
    if (!node) return "";

    let text = "";

    if (node.name) text += node.name + " ";
    if (node.text) text += node.text + " ";
    if (node.role) text += node.role + " ";

    if (node.children && depth < 5) {
      for (const child of node.children) {
        text += this._extractTextContent(child, depth + 1);
      }
    }

    return text;
  }

  /**
   * Generate contextual prompt additions based on page type
   * @param {string} pageType - Detected page type
   * @param {string} goal - Current goal
   * @param {string} axTree - AXTree JSON string
   * @returns {string} Prompt additions
   */
  generateContextualPrompt(pageType, goal, _axTree) {
    const config = this.pageTypes[pageType];

    if (!config) {
      return "\n## Page Context\nThis appears to be a general web page. Use standard web interaction patterns.\n";
    }

    let prompt = "\n## Page Context\n";
    prompt += `This appears to be a **${pageType}** page.\n\n`;

    // Add rules
    prompt += "### Rules for this page type:\n";
    for (const rule of config.rules) {
      prompt += `- ${rule}\n`;
    }

    // Add goal-specific tips
    prompt += "\n### Goal-specific tips:\n";
    prompt += this._generateGoalTips(goal, pageType);

    return prompt;
  }

  /**
   * Generate goal-specific tips
   * @private
   */
  _generateGoalTips(goal, _pageType) {
    const goalLower = goal.toLowerCase();
    let tips = "";

    // Login-related goals
    if (goalLower.includes("login") || goalLower.includes("sign in")) {
      tips += "- Look for username/email and password fields\n";
      tips += "- Fill credentials before clicking submit\n";
      tips +=
        "- Verify login succeeded by checking for user profile or dashboard\n";
    }

    // Search-related goals
    if (goalLower.includes("search") || goalLower.includes("find")) {
      tips += "- Locate the search input field first\n";
      tips += "- Enter search query and press Enter or click search button\n";
      tips += "- Scroll through results to find the target\n";
    }

    // Purchase-related goals
    if (
      goalLower.includes("buy") ||
      goalLower.includes("purchase") ||
      goalLower.includes("order")
    ) {
      tips += "- Add item to cart first\n";
      tips += "- Proceed to checkout\n";
      tips += "- Fill shipping and payment information\n";
      tips += "- Review order before confirming\n";
    }

    // Navigation goals
    if (
      goalLower.includes("navigate") ||
      goalLower.includes("go to") ||
      goalLower.includes("open")
    ) {
      tips += "- Look for navigation links or buttons\n";
      tips += "- Use URL bar if direct navigation is needed\n";
      tips += "- Verify page changed after navigation\n";
    }

    // Generic tips if no specific match
    if (!tips) {
      tips += "- Break down the goal into smaller steps\n";
      tips += "- Verify each action before proceeding\n";
      tips += "- Use scroll to explore more content\n";
    }

    return tips;
  }

  /**
   * Get contextual examples for page type
   * @param {string} pageType - Detected page type
   * @returns {Array} Array of example actions
   */
  getContextualExamples(pageType) {
    const config = this.pageTypes[pageType];

    if (!config) {
      // Return generic examples
      return [
        { action: "click", selector: "button", rationale: "Click a button" },
        {
          action: "type",
          selector: "input",
          value: "text",
          rationale: "Type text",
        },
        { action: "scroll", value: "down", rationale: "Scroll down" },
      ];
    }

    return config.examples;
  }

  /**
   * Generate full prompt with context
   * @param {string} basePrompt - Base system prompt
   * @param {string} pageType - Detected page type
   * @param {string} goal - Current goal
   * @param {string} axTree - AXTree JSON string
   * @returns {string} Enhanced prompt
   */
  enhancePrompt(basePrompt, pageType, goal, axTree) {
    const contextualPrompt = this.generateContextualPrompt(
      pageType,
      goal,
      axTree,
    );
    const examples = this.getContextualExamples(pageType);

    let enhanced = basePrompt;

    // Add contextual section
    enhanced += contextualPrompt;

    // Add examples
    if (examples.length > 0) {
      enhanced += "\n### Example actions for this page type:\n";
      enhanced += "```json\n";
      enhanced += JSON.stringify(examples, null, 2);
      enhanced += "\n```\n";
    }

    return enhanced;
  }
}

const promptAdapter = new PromptAdapter();

export { promptAdapter };
export default promptAdapter;
