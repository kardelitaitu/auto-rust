/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from "vitest";

vi.mock("@api/index.js", () => ({
  api: {
    wait: vi.fn().mockResolvedValue(undefined),
    think: vi.fn().mockResolvedValue(undefined),
    scroll: {
      toTop: vi.fn().mockResolvedValue(undefined),
      read: vi.fn().mockResolvedValue(undefined),
      back: vi.fn().mockResolvedValue(undefined),
    },
  },
}));
import { api } from "@api/index.js";

vi.mock("@api/core/logger.js", () => ({
  createLogger: vi.fn(() => ({
    info: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
    debug: vi.fn(),
  })),
}));

const createPageMock = (options = {}) => {
  const page = {
    _document: options.document,
    _window: options.window,
    _HTMLElement: options.HTMLElement,
    evaluate: vi.fn((fn, arg) => {
      if (typeof fn !== "function") return fn;
      const prevDocument = global.document;
      const prevWindow = global.window;
      const prevHTMLElement = global.HTMLElement;
      global.HTMLElement = page._HTMLElement || class {};
      global.document = page._document || {
        querySelectorAll: () => [],
        querySelector: () => null,
        body: { scrollHeight: 1000 },
      };
      global.window = page._window || {
        innerHeight: 800,
        scrollBy: vi.fn(),
        scrollTo: vi.fn(),
      };
      let result;
      try {
        result = fn(arg);
      } finally {
        global.document = prevDocument;
        global.window = prevWindow;
        global.HTMLElement = prevHTMLElement;
      }
      return result;
    }),
    waitForTimeout: vi.fn().mockResolvedValue(),
    $$: vi.fn().mockResolvedValue([]),
    $: vi.fn().mockResolvedValue(null),
    screenshot: vi.fn().mockResolvedValue("screenshot"),
  };
  return page;
};

const createElementFactory =
  (HTMLElementClass) =>
  (text = "", attributes = {}) => {
    const el = new HTMLElementClass();
    el.innerText = text;
    el.getAttribute = (name) => attributes[name] ?? null;
    el.querySelectorAll = () => [];
    el.querySelector = () => null;
    el.getBoundingClientRect = () => ({ height: 200, x: 0, y: 0, width: 100 });
    return el;
  };

describe("ai-context-engine", () => {
  let AIContextEngine;
  let engine;

  beforeEach(async () => {
    vi.clearAllMocks();
    ({ AIContextEngine } = await import("../../agent/ai-context-engine.js"));
    engine = new AIContextEngine();
  });

  it("analyzes sentiment as positive and negative", () => {
    const positive = engine.analyzeSentiment("love this amazing update");
    const negative = engine.analyzeSentiment("hate this terrible update");
    expect(positive.overall).toBe("positive");
    expect(negative.overall).toBe("negative");
  });

  it("detects promotional tone", () => {
    const tone = engine.detectTone(
      "limited time offer, check out the link in bio",
    );
    expect(tone.primary).toBe("promotional");
    expect(tone.confidence).toBeGreaterThan(0);
  });

  it("classifies conversation types", () => {
    const question = engine.classifyConversation("why is this happening?", []);
    const discussion = engine.classifyConversation("new idea", [
      { text: "agree with this" },
      { text: "disagree, but interesting" },
      { text: "i think it helps" },
      { text: "agree totally" },
    ]);
    expect(question).toBe("question");
    expect(discussion).toBe("discussion");
  });

  it("parses numbers and engagement levels", () => {
    expect(engine.parseNumber("1.2K")).toBe(1200);
    expect(engine.parseNumber("2M")).toBe(2000000);
    expect(engine.parseNumber("1,234")).toBe(1234);
    expect(engine.calculateEngagementLevel(null)).toBe("unknown");
    expect(
      engine.calculateEngagementLevel({
        likes: 0,
        retweets: 0,
        replies: 0,
        views: 200000,
      }),
    ).toBe("viral");
    expect(
      engine.calculateEngagementLevel({
        likes: 800,
        retweets: 200,
        replies: 5,
        views: 5000,
      }),
    ).toBe("high");
    expect(
      engine.calculateEngagementLevel({
        likes: 50,
        retweets: 60,
        replies: 5,
        views: 500,
      }),
    ).toBe("medium");
    expect(
      engine.calculateEngagementLevel({
        likes: 7,
        retweets: 2,
        replies: 3,
        views: 200,
      }),
    ).toBe("low");
    expect(
      engine.calculateEngagementLevel({
        likes: 1,
        retweets: 1,
        replies: 1,
        views: 50,
      }),
    ).toBe("minimal");
  });

  it("extracts metrics from page", async () => {
    const FakeHTMLElement = class {};
    const createEl = createElementFactory(FakeHTMLElement);
    const document = {
      querySelectorAll: () => [
        createEl("", { "aria-label": "12 likes" }),
        createEl("", { "aria-label": "3 retweets" }),
        createEl("", { "aria-label": "1 reply" }),
        createEl("", { "aria-label": "1.2K views" }),
        createEl("", { "aria-label": "5 bookmarks" }),
      ],
    };
    const page = createPageMock({ document, HTMLElement: FakeHTMLElement });
    const metrics = await engine.extractMetrics(page);
    expect(metrics.likes).toBe(12);
    expect(metrics.retweets).toBe(3);
    expect(metrics.replies).toBe(1);
    expect(metrics.views).toBe(1200);
    expect(metrics.bookmarks).toBe(5);
  });

  it("detects tweet images and captures screenshots", async () => {
    const FakeHTMLElement = class {};
    const document = {
      querySelector: () => ({
        querySelector: (selector) =>
          selector.includes("tweetPhoto") ? {} : null,
      }),
    };
    const page = createPageMock({
      document,
      HTMLElement: FakeHTMLElement,
    });
    page.$ = vi
      .fn()
      .mockResolvedValue({ screenshot: vi.fn().mockResolvedValue("jpeg") });
    const hasImage = await engine.checkForTweetImage(page);
    const shot = await engine.captureTweetScreenshot(page);
    expect(hasImage).toBe(true);
    expect(shot).toBe("jpeg");
  });

  it("returns false when image check evaluation fails", async () => {
    const page = createPageMock();
    page.evaluate = vi.fn(() => {
      throw new Error("fail");
    });
    const hasImage = await engine.checkForTweetImage(page);
    expect(hasImage).toBe(false);
  });

  it("returns null when screenshot capture fails", async () => {
    const page = createPageMock();
    page.screenshot = vi.fn().mockRejectedValue(new Error("fail"));
    page.evaluate = vi.fn(() => {
      throw new Error("fail");
    });
    const shot = await engine.captureTweetScreenshot(page);
    expect(shot).toBeNull();
  });

  it("extracts replies and authors from multiple strategies", async () => {
    const FakeHTMLElement = class {};
    const createEl = createElementFactory(FakeHTMLElement);
    const userEl = createEl("@author123");
    const replyEl = createEl("Nice reply");
    const mentionEl = createEl("Hello @user1");
    const paragraphEl = createEl("Deep reply text");
    const articleEl = {
      querySelectorAll: () => [paragraphEl],
    };
    const document = {
      body: { scrollHeight: 2000 },
      querySelectorAll: (selector) => {
        if (selector.includes("User-Name")) return [userEl];
        if (selector === "article") return [articleEl, articleEl];
        return [mentionEl, replyEl];
      },
      querySelector: () => null,
    };
    const article = {
      evaluate: vi.fn(async (fn) =>
        fn({ getBoundingClientRect: () => ({ height: 150 }) }),
      ),
      $: vi.fn(async (selector) => {
        if (
          selector.includes("tweetText") ||
          selector.includes('[dir="auto"]')
        ) {
          return { innerText: vi.fn().mockResolvedValue("Reply from article") };
        }
        return {
          getAttribute: vi.fn().mockResolvedValue("/valid_user"),
        };
      }),
      $$: vi.fn().mockResolvedValue([]),
      innerText: vi.fn().mockResolvedValue("@valid_user reply"),
    };
    const page = createPageMock({
      document,
      HTMLElement: FakeHTMLElement,
      $$: vi.fn().mockResolvedValue([article]),
      $: vi.fn().mockResolvedValue(null),
    });
    const replies = await engine.extractRepliesSmart(page);
    expect(replies.length).toBeGreaterThan(0);
    expect(
      replies.some(
        (r) => r.text.includes("Reply") || r.text.includes("Deep reply"),
      ),
    ).toBe(true);
  });

  it("extracts reply and author from article", async () => {
    const article = {
      $: vi.fn(async (selector) => {
        if (
          selector.includes("tweetText") ||
          selector.includes('[dir="auto"]')
        ) {
          return { innerText: vi.fn().mockResolvedValue("Short reply text") };
        }
        return { getAttribute: vi.fn().mockResolvedValue("/user1234") };
      }),
      $$: vi.fn().mockResolvedValue([]),
      innerText: vi.fn().mockResolvedValue("@user1234 reply"),
    };
    const data = await engine.extractReplyFromArticle(article);
    expect(data.author).toBe("user1234");
    expect(data.text).toContain("Short reply text");
  });

  it("returns null when article reply text is too short", async () => {
    const article = {
      $: vi.fn(async () => ({ innerText: vi.fn().mockResolvedValue("hi") })),
      $$: vi.fn().mockResolvedValue([]),
      innerText: vi.fn().mockResolvedValue("@user1234 hi"),
    };
    const data = await engine.extractReplyFromArticle(article);
    expect(data).toBeNull();
  });

  it("extracts author from visible text context", async () => {
    const FakeHTMLElement = class {};
    const createEl = createElementFactory(FakeHTMLElement);
    const authorEl = createEl("Name @visible_user");
    const document = {
      querySelectorAll: () => [authorEl],
    };
    const page = createPageMock({ document, HTMLElement: FakeHTMLElement });
    const author = await engine.extractAuthorFromVisibleText(
      page,
      "Some reply",
    );
    expect(author).toBe("visible_user");
  });

  it("returns unknown when visible author extraction fails", async () => {
    const page = createPageMock();
    page.evaluate = vi.fn(() => {
      throw new Error("fail");
    });
    const author = await engine.extractAuthorFromVisibleText(
      page,
      "Some reply",
    );
    expect(author).toBe("unknown");
  });

  it("skips numeric-only replies in article extraction", async () => {
    const article = {
      $: vi.fn(async (selector) => {
        if (
          selector.includes("tweetText") ||
          selector.includes('[dir="auto"]')
        ) {
          return { innerText: vi.fn().mockResolvedValue("1,234") };
        }
        return { getAttribute: vi.fn().mockResolvedValue("/user5678") };
      }),
      $$: vi.fn().mockResolvedValue([]),
      innerText: vi.fn().mockResolvedValue("@user5678"),
    };
    const data = await engine.extractReplyFromArticle(article);
    expect(data).toBeNull();
  });

  it("extracts timeline replies", async () => {
    const page = createPageMock();
    page.$$ = vi
      .fn()
      .mockResolvedValue([
        { innerText: vi.fn().mockResolvedValue("Main tweet") },
        { innerText: vi.fn().mockResolvedValue("Reply one @user") },
        { innerText: vi.fn().mockResolvedValue("Reply two") },
      ]);
    const replies = await engine.extractFromTimeline(page);
    expect(replies.length).toBe(2);
  });

  it("skips metrics extraction when disabled", async () => {
    const page = createPageMock();
    engine.config.includeMetrics = false;
    const metricsSpy = vi.spyOn(engine, "extractMetrics");
    vi.spyOn(engine, "extractRepliesSmart").mockResolvedValue([]);
    vi.spyOn(engine, "analyzeSentiment").mockReturnValue({
      overall: "neutral",
      score: 0,
    });
    vi.spyOn(engine, "detectTone").mockReturnValue({
      primary: "serious",
      confidence: 0,
    });
    vi.spyOn(engine, "classifyConversation").mockReturnValue("general");
    vi.spyOn(engine, "analyzeReplySentiment").mockReturnValue({
      overall: "neutral",
      positive: 0,
      negative: 0,
    });
    vi.spyOn(engine, "checkForTweetImage").mockResolvedValue(false);
    const result = await engine.extractEnhancedContext(
      page,
      "url",
      "tweet text",
      "author",
    );
    expect(metricsSpy).not.toHaveBeenCalled();
    expect(result.metrics).toBeNull();
    expect(result.engagementLevel).toBe("unknown");
  });

  it("returns zero metrics when evaluation fails", async () => {
    const page = createPageMock();
    page.evaluate = vi.fn(() => {
      throw new Error("metrics fail");
    });
    const metrics = await engine.extractMetrics(page);
    expect(metrics).toEqual({
      likes: 0,
      retweets: 0,
      replies: 0,
      views: 0,
      bookmarks: 0,
    });
  });

  it("captures screenshot using viewport fallback", async () => {
    const page = createPageMock();
    page.$ = vi.fn().mockResolvedValue(null);
    page.screenshot = vi.fn().mockResolvedValue("full");
    const shot = await engine.captureTweetScreenshot(page);
    expect(shot).toBe("full");
    expect(page.screenshot).toHaveBeenCalled();
  });

  it("extracts author from article text fallback", async () => {
    const article = {
      $: vi.fn().mockResolvedValue(null),
      $$: vi.fn().mockResolvedValue([]),
      innerText: vi.fn().mockResolvedValue("@visible_name hello"),
    };
    const author = await engine.extractAuthorFromArticle(article);
    expect(author).toBe("visible_name");
  });

  it("extracts author from display name when header invalid", async () => {
    const headerLink = { getAttribute: vi.fn().mockResolvedValue("/1234") };
    const nameEl = { innerText: vi.fn().mockResolvedValue("Name @validname") };
    const article = {
      $: vi.fn(async (selector) => {
        if (selector.includes("User-Name")) return nameEl;
        return headerLink;
      }),
      $$: vi.fn().mockResolvedValue([]),
    };
    const author = await engine.extractAuthorFromArticle(article);
    expect(author).toBe("validname");
  });

  it("extracts author from link scan fallback", async () => {
    const links = [
      { getAttribute: vi.fn().mockResolvedValue("/123") },
      { getAttribute: vi.fn().mockResolvedValue("/valid_user") },
    ];
    const article = {
      $: vi.fn().mockResolvedValue(null),
      $$: vi.fn().mockResolvedValue(links),
    };
    const author = await engine.extractAuthorFromArticle(article);
    expect(author).toBe("valid_user");
  });

  it("returns empty replies when smart extraction fails", async () => {
    const page = createPageMock();
    page.evaluate = vi.fn(() => {
      throw new Error("fail");
    });
    const replies = await engine.extractRepliesSmart(page);
    expect(replies).toEqual([]);
  });

  it("extracts deep replies when other strategies are empty", async () => {
    const FakeHTMLElement = class {};
    const paragraphEl = new FakeHTMLElement();
    paragraphEl.innerText = "Deep reply text";
    const articleEl = {
      querySelectorAll: () => [paragraphEl],
    };
    const document = {
      body: { scrollHeight: 1600 },
      querySelectorAll: (selector) => {
        if (selector === "article") return [articleEl, articleEl];
        return [];
      },
    };
    const page = createPageMock({
      document,
      window: { innerHeight: 800, scrollBy: vi.fn(), scrollTo: vi.fn() },
      HTMLElement: FakeHTMLElement,
    });
    page.$$ = vi.fn().mockResolvedValue([]);
    page.evaluate = vi.fn((fn, arg) => {
      if (typeof fn !== "function") return fn;
      const source = fn.toString();
      if (source.includes("window.innerHeight")) return 800;
      if (source.includes("document.body.scrollHeight")) return 1600;
      if (
        source.includes("window.scrollBy") ||
        source.includes("window.scrollTo")
      )
        return undefined;
      // Strategy 1 & 2: Scroll loop extraction (selectors)
      if (
        source.includes("selectors") &&
        source.includes("for (const selector")
      ) {
        return [];
      }
      // Strategy 3: Deep DOM extraction via articles.forEach
      if (source.includes("articles.forEach")) {
        return ["Deep reply text"];
      }
      // Strategy 3 fallback: paragraphs.forEach
      if (source.includes("paragraphs.forEach")) {
        return ["Deep reply text"];
      }
      // Fallback querySelectorAll for tweetText
      if (source.includes("querySelectorAll") && source.includes("tweetText")) {
        return [];
      }
      // Fallback querySelectorAll for article (non-scroll strategies)
      if (
        source.includes("querySelectorAll") &&
        source.includes("article") &&
        !source.includes("articles.forEach")
      ) {
        return ["Deep reply text"];
      }
      return fn(arg);
    });
    const replies = await engine.extractRepliesSmart(page);
    expect(replies.length).toBeGreaterThan(0);
    expect(replies[0].text).toContain("Deep reply");
  });

  it("builds prompt without metrics or replies", () => {
    const prompt = engine.buildEnhancedPrompt(
      {
        tweetText: "hello world",
        author: "user",
        replies: [],
        sentiment: { overall: "neutral", score: 0 },
        tone: { primary: "serious", confidence: 0 },
        conversationType: "general",
        replySentiment: { overall: "neutral", positive: 0, negative: 0 },
        engagementLevel: "unknown",
        metrics: null,
      },
      "system",
    );
    expect(prompt).toContain("Tweet from: @user");
    expect(prompt).toContain("Conversation type: general");
  });

  it("handles empty replies sentiment", () => {
    const summary = engine.analyzeReplySentiment([]);
    expect(summary.distribution.positive).toBe(0);
    expect(summary.distribution.neutral).toBe(0);
    expect(summary.distribution.negative).toBe(0);
  });
  it("summarizes reply sentiment distribution", () => {
    const summary = engine.analyzeReplySentiment([
      { text: "love this" },
      { text: "hate this" },
      { text: "cool" },
    ]);
    expect(
      summary.distribution.positive +
        summary.distribution.negative +
        summary.distribution.neutral,
    ).toBe(3);
  });

  it("builds enhanced prompts with context", () => {
    const prompt = engine.buildEnhancedPrompt(
      {
        tweetText: "hello world",
        author: "user",
        replies: [{ author: "a", text: "nice" }],
        sentiment: { overall: "neutral", score: 0 },
        tone: { primary: "serious", confidence: 1 },
        conversationType: "general",
        replySentiment: { overall: "neutral", positive: 0, negative: 0 },
        engagementLevel: "low",
        metrics: { likes: 1, retweets: 0, replies: 1 },
      },
      "system",
    );
    expect(prompt).toContain("Tweet from: @user");
    expect(prompt).toContain("Engagement: low");
  });

  it("extracts enhanced context with metrics and screenshot", async () => {
    const page = createPageMock();
    const metrics = {
      likes: 10,
      retweets: 2,
      replies: 1,
      views: 100,
      bookmarks: 0,
    };
    const replies = [{ author: "a", text: "Nice" }];
    vi.spyOn(engine, "extractMetrics").mockResolvedValue(metrics);
    vi.spyOn(engine, "extractRepliesSmart").mockResolvedValue(replies);
    vi.spyOn(engine, "analyzeSentiment").mockReturnValue({
      overall: "positive",
      score: 0.4,
    });
    vi.spyOn(engine, "detectTone").mockReturnValue({ primary: "serious" });
    vi.spyOn(engine, "classifyConversation").mockReturnValue("general");
    vi.spyOn(engine, "analyzeReplySentiment").mockReturnValue({
      overall: "neutral",
    });
    vi.spyOn(engine, "checkForTweetImage").mockResolvedValue(true);
    vi.spyOn(engine, "captureTweetScreenshot").mockResolvedValue("shot");
    const result = await engine.extractEnhancedContext(
      page,
      "url",
      "tweet text",
      "author",
    );
    expect(result.metrics).toEqual(metrics);
    expect(result.replies.length).toBe(1);
    expect(result.screenshot).toBe("shot");
    expect(result.engagementLevel).toBe("low");
  });

  it("returns base context when extraction fails", async () => {
    const page = createPageMock();
    vi.spyOn(engine, "extractMetrics").mockRejectedValue(new Error("fail"));
    const result = await engine.extractEnhancedContext(
      page,
      "url",
      "tweet text",
      "author",
    );
    expect(result.url).toBe("url");
    expect(result.replies).toEqual([]);
    expect(result.metrics).toBeNull();
  });
});
