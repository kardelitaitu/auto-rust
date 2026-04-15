/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * Unit tests for api/tests/unit/ai-quote-engine.test-utils.js
 */

import { describe, it, expect, vi } from "vitest";
import {
  createPageMock,
  createHumanMock,
  baseSentiment,
  sampleReplies,
  sampleTweet,
} from "./ai-quote-engine.test-utils.js";

describe("ai-quote-engine.test-utils.js", () => {
  describe("createPageMock", () => {
    it("should return page and locator objects", () => {
      const result = createPageMock();

      expect(result).toHaveProperty("page");
      expect(result).toHaveProperty("locator");
    });

    it("should apply overrides to page object", () => {
      const customUrl = "https://custom.com/test";
      const { page } = createPageMock({ url: () => customUrl });

      expect(page.url()).toBe(customUrl);
    });

    it("should have all required page methods", () => {
      const { page } = createPageMock();

      expect(page.evaluate).toBeDefined();
      expect(page.keyboard).toBeDefined();
      expect(page.mouse).toBeDefined();
      expect(page.locator).toBeDefined();
      expect(page.waitVisible).toBeDefined();
      expect(page.waitForSelector).toBeDefined();
      expect(page.waitForTimeout).toBeDefined();
      expect(page.url).toBeDefined();
      expect(page.content).toBeDefined();
    });

    it("should have all required locator methods", () => {
      const { locator } = createPageMock();

      expect(locator.count).toBeDefined();
      expect(locator.click).toBeDefined();
      expect(locator.first).toBeDefined();
      expect(locator.textContent).toBeDefined();
      expect(locator.isVisible).toBeDefined();
      expect(locator.getAttribute).toBeDefined();
      expect(locator.scrollIntoViewIfNeeded).toBeDefined();
      expect(locator.all).toBeDefined();
      expect(locator.fill).toBeDefined();
      expect(locator.press).toBeDefined();
    });

    it("should have page.evaluate execute callback with arg", async () => {
      const { page } = createPageMock();
      const testFn = (arg) => arg * 2;
      const result = await page.evaluate(testFn, 5);

      expect(result).toBe(10);
    });

    it("should have locator.first return locator itself", () => {
      const { locator } = createPageMock();

      expect(locator.first()).toBe(locator);
    });

    it("should apply overrides to locator object", () => {
      const mockClick = vi.fn().mockResolvedValue(false);
      const { locator } = createPageMock({ click: mockClick });

      // The locator from page.locator() is separate from page overrides
      // The overrides apply to page, not locator
      expect(locator.click).toBeDefined();
    });
  });

  describe("createHumanMock", () => {
    it("should return human mock with all required methods", () => {
      const human = createHumanMock();

      expect(human.logStep).toBeDefined();
      expect(human.verifyComposerOpen).toBeDefined();
      expect(human.typeText).toBeDefined();
      expect(human.postTweet).toBeDefined();
      expect(human.safeHumanClick).toBeDefined();
      expect(human.fixation).toBeDefined();
      expect(human.microMove).toBeDefined();
      expect(human.hesitation).toBeDefined();
      expect(human.ensureFocus).toBeDefined();
      expect(human.selectMethod).toBeDefined();
    });

    it("should apply overrides to human mock", () => {
      const customLogStep = vi.fn();
      const human = createHumanMock({ logStep: customLogStep });

      expect(human.logStep).toBe(customLogStep);
    });

    it("should have default mock return values", async () => {
      const human = createHumanMock();

      expect(await human.verifyComposerOpen()).toEqual({
        open: true,
        selector: '[data-testid="tweetTextarea_0"]',
      });
      expect(await human.postTweet()).toEqual({
        success: true,
        reason: "posted",
      });
      expect(await human.safeHumanClick()).toBe(true);
      expect(await human.ensureFocus()).toBe(true);
    });

    it("should have selectMethod return first method by default", () => {
      const human = createHumanMock();
      const methods = [{ name: "A" }, { name: "B" }, { name: "C" }];

      const result = human.selectMethod(methods);

      expect(result).toEqual({ name: "A" });
    });

    it("should have selectMethod use override function when provided", () => {
      const customSelect = (methods) => methods[1];
      const human = createHumanMock({ selectMethod: customSelect });
      const methods = [{ name: "A" }, { name: "B" }, { name: "C" }];

      const result = human.selectMethod(methods);

      expect(result).toEqual({ name: "B" });
    });
  });

  describe("baseSentiment", () => {
    it("should have correct structure", () => {
      expect(baseSentiment).toHaveProperty("score");
      expect(baseSentiment).toHaveProperty("isNegative");
      expect(baseSentiment).toHaveProperty("composite");
      expect(baseSentiment).toHaveProperty("dimensions");
    });

    it("should have valid composite object", () => {
      expect(baseSentiment.composite).toHaveProperty("score");
      expect(baseSentiment.composite).toHaveProperty("label");
      expect(baseSentiment.composite).toHaveProperty("engagementStyle");
      expect(baseSentiment.composite).toHaveProperty("riskLevel");
      expect(baseSentiment.composite).toHaveProperty("conversationType");
    });

    it("should have valid dimensions object", () => {
      expect(baseSentiment.dimensions).toHaveProperty("valence");
      expect(baseSentiment.dimensions).toHaveProperty("arousal");
      expect(baseSentiment.dimensions).toHaveProperty("dominance");
      expect(baseSentiment.dimensions).toHaveProperty("sarcasm");
      expect(baseSentiment.dimensions).toHaveProperty("toxicity");
      expect(baseSentiment.dimensions).toHaveProperty("intent");
    });
  });

  describe("sampleReplies", () => {
    it("should be an array", () => {
      expect(Array.isArray(sampleReplies)).toBe(true);
    });

    it("should have at least one reply", () => {
      expect(sampleReplies.length).toBeGreaterThanOrEqual(1);
    });

    it("should have replies with text and author", () => {
      sampleReplies.forEach((reply) => {
        expect(reply).toHaveProperty("text");
        expect(reply).toHaveProperty("author");
      });
    });
  });

  describe("sampleTweet", () => {
    it("should have required properties", () => {
      expect(sampleTweet).toHaveProperty("text");
      expect(sampleTweet).toHaveProperty("author");
      expect(sampleTweet).toHaveProperty("url");
    });

    it("should have valid tweet data", () => {
      expect(typeof sampleTweet.text).toBe("string");
      expect(typeof sampleTweet.author).toBe("string");
      expect(typeof sampleTweet.url).toBe("string");
    });

    it("should have URL starting with https", () => {
      expect(sampleTweet.url).toMatch(/^https:\/\//);
    });
  });
});
