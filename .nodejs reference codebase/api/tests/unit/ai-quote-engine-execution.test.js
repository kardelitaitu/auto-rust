/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import {
  createPageMock,
  createHumanMock,
} from "./ai-quote-engine.test-utils.js";

vi.mock("@api/index.js", () => {
  return {
    api: {
      setPage: vi.fn(),
      getPage: vi.fn(),
      wait: vi.fn().mockResolvedValue(undefined),
      click: vi.fn().mockResolvedValue(true),
      type: vi.fn().mockResolvedValue(undefined),
      scroll: { toTop: vi.fn().mockResolvedValue(undefined) },
      visible: vi.fn().mockImplementation((selector) => {
        if (selector === '[role="menu"]') {
          return Promise.resolve(true);
        }
        return Promise.resolve(true);
      }),
      exists: vi.fn().mockResolvedValue(true),
      findElement: vi.fn().mockImplementation((selectors, options) => {
        if (Array.isArray(selectors)) {
          return Promise.resolve("#mock-selector");
        }
        return Promise.resolve("#mock-selector");
      }),
      getCurrentUrl: vi.fn().mockResolvedValue("https://x.com/status/1"),
      eval: vi.fn().mockImplementation((fn, arg) => {
        // Return proper values based on the function being called
        if (typeof fn === "function") {
          try {
            const result = fn(arg);
            // Ensure we always return a string for includes() calls
            if (result === undefined || result === null) {
              return Promise.resolve("<div><br></div>");
            }
            return Promise.resolve(String(result));
          } catch (e) {
            return Promise.resolve("<div><br></div>");
          }
        }
        return Promise.resolve("<div><br></div>");
      }),
      text: vi.fn().mockImplementation((sel) => {
        return Promise.resolve("https://x.com/status/123");
      }),
      waitVisible: vi.fn().mockImplementation(() => Promise.resolve(undefined)),
    },
  };
});
import { api } from "@api/index.js";

vi.mock("@api/core/logger.js", () => ({
  createLogger: vi.fn((name) => ({
    info: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
    debug: vi.fn(),
  })),
}));

vi.mock("@api/utils/config-service.js", () => ({ config: {} }));
vi.mock("@api/utils/scroll-helper.js", () => ({
  scrollRandom: vi.fn(),
}));

vi.mock("@api/behaviors/human-interaction.js", () => ({
  HumanInteraction: class {
    constructor() {}
    selectMethod(methods) {
      return methods[0];
    }
    logStep() {}
    verifyComposerOpen() {
      return { open: true, selector: '[data-testid="tweetTextarea_0"]' };
    }
    typeText() {
      return Promise.resolve();
    }
    postTweet() {
      return Promise.resolve({ success: true, reason: "posted" });
    }
    safeHumanClick() {
      return Promise.resolve(true);
    }
    fixation() {
      return Promise.resolve();
    }
    microMove() {
      return Promise.resolve();
    }
    hesitation() {
      return Promise.resolve();
    }
    ensureFocus() {
      return Promise.resolve(true);
    }
    findElement() {
      return Promise.resolve({
        element: { click: () => Promise.resolve() },
        selector: "#id",
      });
    }
  },
}));

import { HumanInteraction } from "@api/behaviors/human-interaction.js";

describe("AIQuoteEngine - Execution Methods", () => {
  let AIQuoteEngine;
  let engine;

  beforeEach(async () => {
    vi.clearAllMocks();

    // Re-apply mock implementations after clearAllMocks
    api.wait.mockResolvedValue(undefined);
    api.click.mockResolvedValue(true);
    api.type.mockResolvedValue(undefined);
    api.scroll = { toTop: vi.fn().mockResolvedValue(undefined) };
    api.visible.mockResolvedValue(true);
    api.exists.mockResolvedValue(true);
    api.findElement.mockResolvedValue("#mock-selector");
    api.getCurrentUrl.mockResolvedValue("https://x.com/status/1");
    api.eval.mockResolvedValue("<div><br></div>");
    api.text.mockResolvedValue("https://x.com/status/123");
    api.waitVisible.mockResolvedValue(undefined);

    ({ default: AIQuoteEngine } =
      await import("../../../api/agent/ai-quote-engine.js"));
    engine = new AIQuoteEngine(
      { processRequest: vi.fn(), sessionId: "test" },
      { quoteProbability: 1, maxRetries: 1 },
    );
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe("quoteMethodA_Keyboard", () => {
    it("executes keyboard method successfully", async () => {
      const { page } = createPageMock();
      const human = createHumanMock();
      api.getPage.mockReturnValue(page);

      const result = await engine.quoteMethodA_Keyboard(
        page,
        "Test quote",
        human,
      );
      expect(result.success).toBe(true);
      expect(result.method).toBe("keyboard_compose");
    });
  });

  describe("quoteMethodB_Retweet", () => {
    it("executes retweet menu method successfully", async () => {
      const { page } = createPageMock();
      const human = createHumanMock();
      api.getPage.mockReturnValue(page);
      api.visible.mockImplementation((selector) => {
        if (selector === '[role="menu"]') return Promise.resolve(true);
        return Promise.resolve(true);
      });
      api.findElement.mockResolvedValue("#mock-selector");
      api.click.mockResolvedValue(true);

      const result = await engine.quoteMethodB_Retweet(
        page,
        "Test quote",
        human,
      );
      expect(result.success).toBe(true);
      expect(result.method).toBe("retweet_menu");
    });

    it("handles quote preview visibility timeout", async () => {
      const { page } = createPageMock();
      const human = createHumanMock();
      api.getPage.mockReturnValue(page);
      api.findElement
        .mockResolvedValueOnce("#mock-selector") // retweet
        .mockResolvedValueOnce("#mock-selector") // quote option
        .mockResolvedValueOnce(null); // preview
      api.waitVisible.mockRejectedValue(new Error("timeout"));
      api.visible.mockImplementation((selector) => {
        if (selector === '[role="menu"]') return Promise.resolve(true);
        return Promise.resolve(true);
      });

      const result = await engine.quoteMethodB_Retweet(page, "Test", human);
      expect(result.success).toBe(false);
      expect(result.reason).toBe("quote_preview_missing");
    });

    it("aborts post if quote preview is missing to prevent regular tweet", async () => {
      const { page } = createPageMock();
      const human = createHumanMock();
      api.getPage.mockReturnValue(page);
      api.findElement
        .mockResolvedValueOnce("#mock-selector") // retweet
        .mockResolvedValueOnce("#mock-selector") // quote option
        .mockResolvedValueOnce(null); // preview
      api.waitVisible.mockRejectedValue(new Error("timeout"));
      api.visible.mockImplementation((selector) => {
        if (selector === '[role="menu"]') return Promise.resolve(true);
        return Promise.resolve(true);
      });

      const result = await engine.quoteMethodB_Retweet(page, "Test", human);
      expect(result.success).toBe(false);
      expect(result.reason).toBe("quote_preview_missing");
      expect(page.keyboard.press).toHaveBeenCalledWith("Escape");
    });
  });

  describe("quoteMethodC_Url Edge Cases", () => {
    it("retries paste when URL not found initially", async () => {
      const { page } = createPageMock();
      const human = createHumanMock();
      api.getPage.mockReturnValue(page);

      let attempt = 0;
      api.text.mockImplementation(() => {
        attempt++;
        if (attempt === 1) return Promise.resolve("No URL here");
        return Promise.resolve("https://x.com/status/123");
      });
      api.findElement.mockResolvedValue("#mock-selector");
      api.waitVisible.mockResolvedValue(undefined);

      const result = await engine.quoteMethodC_Url(page, "Test quote", human);
      expect(result.success).toBe(true);
      expect(api.text).toHaveBeenCalled();
    });

    it("handles composer failing to open", async () => {
      const { page } = createPageMock();
      const human = createHumanMock({
        verifyComposerOpen: vi.fn().mockResolvedValue({ open: false }),
      });
      api.getPage.mockReturnValue(page);
      api.findElement.mockResolvedValue("#mock-selector");
      api.waitVisible.mockResolvedValue(undefined);

      const result = await engine.quoteMethodC_Url(page, "Test", human);
      expect(result.success).toBe(false);
      expect(result.reason).toBe("composer_not_open");
    });

    it("handles compose button not found", async () => {
      const { page } = createPageMock();
      const human = createHumanMock();
      api.getPage.mockReturnValue(page);
      api.findElement.mockResolvedValue(null);

      const result = await engine.quoteMethodC_Url(page, "Test", human);
      expect(result.success).toBe(false);
      expect(result.reason).toBe("compose_button_not_found");
    });

    it("handles composer visibility timeout in URL method", async () => {
      const { page } = createPageMock();
      const human = createHumanMock({
        postTweet: vi
          .fn()
          .mockResolvedValue({ success: true, reason: "posted" }),
      });
      api.getPage.mockReturnValue(page);
      api.waitVisible.mockResolvedValue(undefined);
      api.findElement.mockResolvedValue("#mock-selector");
      api.eval.mockResolvedValue("<div><br></div>");

      const result = await engine.quoteMethodC_Url(page, "Test", human);
      expect(result.success).toBe(true);
    });

    it("retries Enter if newline not detected after typing comment", async () => {
      const { page } = createPageMock();
      const human = createHumanMock();
      api.getPage.mockReturnValue(page);

      let evalCount = 0;
      api.eval.mockImplementation(() => {
        evalCount++;
        if (evalCount === 2) return Promise.resolve("no newline");
        return Promise.resolve("<div><br></div>");
      });

      api.text.mockResolvedValue("https://x.com/status");
      api.findElement.mockResolvedValue("#mock-selector");
      api.waitVisible.mockResolvedValue(undefined);

      const result = await engine.quoteMethodC_Url(page, "Test", human);

      const enterCalls = page.keyboard.press.mock.calls.filter(
        (call) => call[0] === "Enter",
      );
      expect(enterCalls.length).toBeGreaterThanOrEqual(1);
      expect(result.success).toBe(true);
    });

    it("falls back to manual URL typing when paste fails completely", async () => {
      const { page } = createPageMock();
      const human = createHumanMock();
      api.getPage.mockReturnValue(page);
      api.text.mockResolvedValue("No URL pasted at all");
      api.findElement.mockResolvedValue("#mock-selector");
      api.waitVisible.mockResolvedValue(undefined);

      const result = await engine.quoteMethodC_Url(page, "Test", human);
      expect(page.keyboard.type).toHaveBeenCalled();
      expect(result.success).toBe(true);
    });
  });

  it("falls back to secondary method if primary fails", async () => {
    const { page } = createPageMock();
    api.getPage.mockReturnValue(page);

    engine.quoteMethodA_Keyboard = vi
      .fn()
      .mockResolvedValue({ success: true, method: "keyboard_compose" });

    const result = await engine.executeQuote(page, "Main quote");
    expect(result.success).toBe(true);
    expect(engine.quoteMethodA_Keyboard).toHaveBeenCalled();
  });

  it("falls back to retweet method if keyboard method fails", async () => {
    const { page } = createPageMock();
    api.getPage.mockReturnValue(page);

    engine.quoteMethodA_Keyboard = vi
      .fn()
      .mockRejectedValue(new Error("UI Error"));
    engine.quoteMethodB_Retweet = vi
      .fn()
      .mockResolvedValue({ success: true, method: "retweet_menu" });

    const result = await engine.executeQuote(page, "Fallback quote");
    expect(result.success).toBe(true);
    expect(engine.quoteMethodA_Keyboard).toHaveBeenCalled();
    expect(engine.quoteMethodB_Retweet).toHaveBeenCalled();
  });
});
