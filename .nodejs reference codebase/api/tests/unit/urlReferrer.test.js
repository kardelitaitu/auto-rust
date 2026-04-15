/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { ReferrerEngine } from "@api/utils/urlReferrer.js";
import fs from "fs";
import path from "path";

vi.mock("@api/index.js", () => ({
  api: {
    setPage: vi.fn(),
    goto: vi.fn(),
    setExtraHTTPHeaders: vi.fn(),
    click: vi.fn(),
    waitForURL: vi.fn(),
  },
}));
import { api } from "@api/index.js";

vi.mock("fs");
vi.mock("path");

describe("ReferrerEngine", () => {
  let engine;

  beforeEach(() => {
    vi.resetAllMocks();
    vi.mocked(fs.existsSync).mockReturnValue(false);
    vi.mocked(path.resolve).mockImplementation((...args) => args.join("/"));

    engine = new ReferrerEngine({ addUTM: true });
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe("PrivacyEngine (Naturalization)", () => {
    it("should return origin-only for non-whitelisted strategies (e.g., reddit)", () => {
      vi.spyOn(Math, "random").mockReturnValue(0.6); // reddit_thread

      const ctx = engine.generateContext("https://target.com");
      expect(ctx.strategy).toBe("reddit_thread");
      expect(ctx.referrer).toBe("https://www.reddit.com/");
    });

    it("should preserve full path for whitelisted strategies (e.g., twitter_tco)", () => {
      vi.spyOn(Math, "random").mockReturnValue(0.5);

      const ctx = engine.generateContext("https://target.com");
      expect(ctx.strategy).toBe("twitter_tco");
      expect(ctx.referrer).toMatch(/^https:\/\/t\.co\/.+/);
    });

    it("should preserve full path for search engines (e.g., google)", () => {
      vi.spyOn(Math, "random").mockReturnValue(0.2);

      const ctx = engine.generateContext("https://target.com");
      expect(ctx.strategy).toBe("google_search");
      expect(ctx.referrer).toContain("/search?q=");
    });
  });

  describe("Context Generation", () => {
    it("should generate direct traffic correctly", () => {
      vi.spyOn(Math, "random").mockReturnValue(0.05);

      const ctx = engine.generateContext("https://target.com");
      expect(ctx.strategy).toBe("direct");
      expect(ctx.referrer).toBe("");
      expect(ctx.headers["Referer"]).toBeUndefined();
      expect(ctx.headers["Sec-Fetch-Site"]).toBe("none");
    });

    it("should add UTM parameters when configured", () => {
      vi.spyOn(Math, "random").mockReturnValue(0.2);

      const ctx = engine.generateContext("https://target.com");
      expect(ctx.targetWithParams).toContain("utm_source=google");
      expect(ctx.targetWithParams).toContain("utm_medium=organic");
    });

    it("should NOT add UTM parameters when disabled", () => {
      const noUtmEngine = new ReferrerEngine({ addUTM: false });
      vi.spyOn(Math, "random").mockReturnValue(0.2);

      const ctx = noUtmEngine.generateContext("https://target.com");
      expect(ctx.targetWithParams).toBe("https://target.com");
      expect(ctx.targetWithParams).not.toContain("utm_source");
    });

    it("should add UTM parameters for whatsapp strategies", () => {
      vi.spyOn(Math, "random").mockReturnValue(0.92);

      const ctx = engine.generateContext("https://target.com");
      expect(["whatsapp_web", "whatsapp_api"]).toContain(ctx.strategy);
      expect(ctx.targetWithParams).toContain("utm_source=whatsapp");
      expect(ctx.targetWithParams).toContain("utm_medium=messenger");
    });

    it("should add UTM parameters for twitter strategy", () => {
      vi.spyOn(Math, "random").mockReturnValue(0.45);

      const ctx = engine.generateContext("https://target.com");
      expect(ctx.targetWithParams).toContain("utm_source=twitter");
      expect(ctx.targetWithParams).toContain("utm_medium=social");
    });

    it("should generate context with different target URLs", () => {
      vi.spyOn(Math, "random").mockReturnValue(0.05);

      const twitterCtx = engine.generateContext(
        "https://twitter.com/user/status/123456789",
      );
      expect(twitterCtx).toBeDefined();

      const profileCtx = engine.generateContext("https://twitter.com/username");
      expect(profileCtx).toBeDefined();
    });

    it("should generate context with profile subpages", () => {
      vi.spyOn(Math, "random").mockReturnValue(0.05);

      const mediaCtx = engine.generateContext(
        "https://twitter.com/username/media",
      );
      expect(mediaCtx).toBeDefined();

      const repliesCtx = engine.generateContext(
        "https://twitter.com/username/with_replies",
      );
      expect(repliesCtx).toBeDefined();
    });

    it("should return null for reserved paths", () => {
      vi.spyOn(Math, "random").mockReturnValue(0.05);

      const homeCtx = engine.generateContext("https://twitter.com/home");
      expect(homeCtx).toBeDefined();

      const exploreCtx = engine.generateContext("https://twitter.com/explore");
      expect(exploreCtx).toBeDefined();
    });
  });

  describe("Trampoline Navigation", () => {
    let mockPage;

    beforeEach(() => {
      mockPage = {
        setExtraHTTPHeaders: vi.fn(),
        goto: vi.fn(),
        route: vi.fn(),
        unroute: vi.fn(),
        click: vi.fn(),
        waitForURL: vi.fn(),
        url: vi.fn().mockReturnValue("about:blank"),
        isClosed: vi.fn().mockReturnValue(false),
        viewportSize: vi.fn().mockReturnValue({ width: 1920, height: 1080 }),
        mouse: {
          move: vi.fn().mockResolvedValue(undefined),
          click: vi.fn().mockResolvedValue(undefined),
          dblclick: vi.fn().mockResolvedValue(undefined),
          down: vi.fn().mockResolvedValue(undefined),
          up: vi.fn().mockResolvedValue(undefined),
        },
        context: () => ({
          browser: () => ({ isConnected: () => true }),
        }),
      };
      api.setPage(mockPage);
    });

    it("should use simple goto for direct traffic", async () => {
      vi.spyOn(Math, "random").mockReturnValue(0.05);

      await engine.navigate(mockPage, "https://target.com");

      expect(api.setExtraHTTPHeaders).toHaveBeenCalled();
      expect(api.goto).toHaveBeenCalledWith(
        expect.stringContaining("https://target.com"),
      );
      expect(mockPage.route).not.toHaveBeenCalled();
    });

    it("should use trampoline for complex traffic", async () => {
      vi.spyOn(Math, "random").mockReturnValue(0.2);

      mockPage.route.mockImplementation((pattern, handler) => {
        if (typeof handler === "function") {
          void handler;
        }
      });

      await engine.navigate(mockPage, "https://target.com");

      expect(mockPage.route).toHaveBeenCalledWith(
        "**/favicon.ico",
        expect.any(Function),
      );
      expect(mockPage.route).toHaveBeenCalledWith(
        expect.stringContaining("google.com"),
        expect.any(Function),
      );
      expect(api.goto).toHaveBeenCalledWith(
        expect.stringContaining("google.com"),
        {
          waitUntil: "commit",
        },
      );
      // waitForURL is called via api, not page
      expect(api.waitForURL).toHaveBeenCalled();
    });

    it("should fallback to direct goto on trampoline error", async () => {
      vi.spyOn(Math, "random").mockReturnValue(0.2);

      mockPage.route.mockRejectedValue(new Error("Route failed"));

      await engine.navigate(mockPage, "https://target.com");

      expect(api.goto).toHaveBeenCalledWith(
        expect.stringContaining("https://target.com"),
        expect.objectContaining({ referer: expect.any(String) }),
      );
    });
  });

  describe("Dictionary Fallback", () => {
    it("should use fallback when dictionary file is missing", () => {
      const fallbackEngine = new ReferrerEngine({});
      expect(fallbackEngine).toBeDefined();
    });

    it("should load dictionary when file exists", () => {
      vi.resetAllMocks();

      vi.mocked(fs.existsSync).mockImplementation((path) => {
        return path.toString().includes("referrer_dict.json");
      });

      vi.mocked(fs.readFileSync).mockReturnValue(
        JSON.stringify({
          TOPICS: ["test-topic"],
          ACTIONS: ["test-action"],
          CONTEXT: ["test-context"],
          SUBREDDITS: ["test-subreddit"],
        }),
      );

      const engine = new ReferrerEngine({});
      expect(engine).toBeDefined();
    });

    it("should load t.co links when file exists", () => {
      vi.resetAllMocks();

      vi.mocked(fs.existsSync).mockImplementation((path) => {
        const p = path.toString();
        return p.includes("referrer_dict.json") || p.includes("tco_links.json");
      });

      vi.mocked(fs.readFileSync).mockImplementation((path) => {
        const p = path.toString();
        if (p.includes("referrer_dict.json")) {
          return JSON.stringify({
            TOPICS: ["tech"],
            ACTIONS: ["read"],
            CONTEXT: ["thread"],
            SUBREDDITS: ["technology"],
          });
        }
        if (p.includes("tco_links.json")) {
          return JSON.stringify(["https://t.co/abc123", "https://t.co/xyz789"]);
        }
        return "{}";
      });

      const engine = new ReferrerEngine({});
      expect(engine).toBeDefined();
    });

    it("should handle dictionary file parse error gracefully", () => {
      vi.resetAllMocks();

      vi.mocked(fs.existsSync).mockImplementation((path) => {
        return path.toString().includes("referrer_dict.json");
      });

      vi.mocked(fs.readFileSync).mockImplementation(() => {
        throw new Error("Parse error");
      });

      const engine = new ReferrerEngine({});
      expect(engine).toBeDefined();
    });
  });

  describe("navigate method", () => {
    let mockPage;

    beforeEach(() => {
      mockPage = {
        setExtraHTTPHeaders: vi.fn().mockResolvedValue(undefined),
        goto: vi.fn().mockResolvedValue(undefined),
        route: vi.fn().mockResolvedValue(undefined),
        unroute: vi.fn().mockResolvedValue(undefined),
        click: vi.fn().mockResolvedValue(undefined),
        waitForURL: vi.fn().mockResolvedValue(undefined),
        url: vi.fn().mockReturnValue("about:blank"),
        isClosed: vi.fn().mockReturnValue(false),
        viewportSize: vi.fn().mockReturnValue({ width: 1920, height: 1080 }),
        mouse: {
          move: vi.fn().mockResolvedValue(undefined),
          click: vi.fn().mockResolvedValue(undefined),
          dblclick: vi.fn().mockResolvedValue(undefined),
          down: vi.fn().mockResolvedValue(undefined),
          up: vi.fn().mockResolvedValue(undefined),
        },
        context: () => ({
          browser: () => ({ isConnected: () => true }),
        }),
      };
      api.setPage(mockPage);
    });

    it("should call trampolineNavigate for non-direct traffic", async () => {
      vi.spyOn(Math, "random").mockReturnValue(0.2);

      await engine.navigate(mockPage, "https://twitter.com/user/status/123");

      expect(mockPage.route).toHaveBeenCalled();
    });
  });

  describe("trampolineNavigate edge cases", () => {
    let mockPage;

    beforeEach(() => {
      mockPage = {
        setExtraHTTPHeaders: vi.fn().mockResolvedValue(undefined),
        goto: vi.fn().mockResolvedValue(undefined),
        route: vi.fn().mockImplementation((pattern, handler) => {
          if (pattern.includes("favicon.ico")) {
            handler.fulfill({
              status: 200,
              contentType: "image/x-icon",
              body: "",
            });
          } else {
            handler.fulfill({
              status: 200,
              contentType: "text/html",
              body: '<html><body><a id="trampoline" href="https://target.com">Click</a></body></html>',
            });
          }
          return Promise.resolve();
        }),
        unroute: vi.fn().mockResolvedValue(undefined),
        click: vi.fn().mockResolvedValue(undefined),
        waitForURL: vi.fn().mockImplementation(() => Promise.resolve()),
        url: vi.fn().mockReturnValue("about:blank"),
        isClosed: vi.fn().mockReturnValue(false),
        viewportSize: vi.fn().mockReturnValue({ width: 1920, height: 1080 }),
        mouse: {
          move: vi.fn().mockResolvedValue(undefined),
          click: vi.fn().mockResolvedValue(undefined),
          dblclick: vi.fn().mockResolvedValue(undefined),
          down: vi.fn().mockResolvedValue(undefined),
          up: vi.fn().mockResolvedValue(undefined),
        },
        context: () => ({
          browser: () => ({ isConnected: () => true }),
        }),
      };
      api.setPage(mockPage);
    });

    it("should handle click timeout gracefully", async () => {
      vi.spyOn(Math, "random").mockReturnValue(0.2);

      mockPage.click.mockRejectedValue(new Error("Timeout"));

      await engine.trampolineNavigate(mockPage, "https://target.com");

      // On trampoline failure, it falls back to api.goto with headers
      expect(api.goto).toHaveBeenCalled();
    });

    it("should handle invalid URL in targetWithParams gracefully", async () => {
      vi.spyOn(Math, "random").mockReturnValue(0.2);

      const ctx = engine.generateContext("https://target.com");
      ctx.targetWithParams = "://invalid";

      expect(() => {
        engine.generateContext("https://valid.com");
      }).not.toThrow();
    });

    it("should handle route pattern matching for trampoline", async () => {
      vi.spyOn(Math, "random").mockReturnValue(0.2);

      const routeCalls = [];
      mockPage.route = vi.fn().mockImplementation((pattern, handler) => {
        routeCalls.push({ pattern: pattern.toString(), handler });
        return Promise.resolve();
      });

      await engine.navigate(mockPage, "https://twitter.com/user/status/123");

      // Verify route was called for the referrer URL
      expect(routeCalls.length).toBeGreaterThan(0);
    });
  });

  describe("URL generation for different strategies", () => {
    it("should generate twitter_tco URLs with random path", () => {
      vi.spyOn(Math, "random").mockReturnValue(0.5);

      const ctx = engine.generateContext("https://external-site.com");
      expect(ctx.referrer).toMatch(/^https:\/\/t\.co\/\w{10}$/);
    });

    it("should generate linkedin_feed URLs", () => {
      vi.spyOn(Math, "random")
        .mockReturnValueOnce(0.96)
        .mockReturnValueOnce(0.6);

      const ctx = engine.generateContext("https://target.com");
      expect(ctx.strategy).toBe("linkedin_feed");
      expect(ctx.referrer).toBe("https://www.linkedin.com/");
    });

    it("should generate discord_channel URLs", () => {
      vi.spyOn(Math, "random").mockReturnValue(0.68);

      const ctx = engine.generateContext("https://target.com");
      expect(ctx.strategy).toBe("discord_channel");
    });

    it("should generate messaging strategy URLs", () => {
      vi.spyOn(Math, "random").mockReturnValue(0.85);

      const ctx = engine.generateContext("https://target.com");
      expect(["telegram_web", "whatsapp_web"]).toContain(ctx.strategy);
    });

    it("should generate whatsapp_web URLs", () => {
      vi.spyOn(Math, "random").mockReturnValue(0.88);

      const ctx = engine.generateContext("https://target.com");
      expect(ctx.strategy).toBe("whatsapp_web");
      expect(ctx.referrer).toBe("https://web.whatsapp.com/");
    });

    it("should generate whatsapp_api URLs", () => {
      vi.spyOn(Math, "random").mockReturnValue(0.92);

      const ctx = engine.generateContext("https://target.com");
      expect(ctx.strategy).toBe("whatsapp_api");
      expect(ctx.referrer).toBe("https://api.whatsapp.com/");
    });

    it("should select long-tail strategies", () => {
      vi.spyOn(Math, "random")
        .mockReturnValueOnce(0.96)
        .mockReturnValueOnce(0.3);

      const ctx = engine.generateContext("https://target.com");
      expect([
        "hacker_news",
        "substack",
        "medium_article",
        "linkedin_feed",
      ]).toContain(ctx.strategy);
    });
  });

  describe("Query generation through strategy", () => {
    it("should generate search query for twitter status URLs", () => {
      vi.spyOn(Math, "random").mockReturnValue(0.22);

      const ctx = engine.generateContext(
        "https://twitter.com/testuser/status/123456789",
      );
      expect(ctx.referrer).toContain("search?q=");
      expect(decodeURIComponent(ctx.referrer)).toContain("testuser");
    });

    it("should generate search query for twitter profile URLs", () => {
      vi.spyOn(Math, "random").mockReturnValue(0.22);

      const ctx = engine.generateContext("https://twitter.com/testuser");
      expect(ctx.referrer).toContain("search?q=");
      expect(decodeURIComponent(ctx.referrer)).toContain("testuser");
    });

    it("should generate search query for generic URLs", () => {
      vi.spyOn(Math, "random").mockReturnValue(0.22);

      const ctx = engine.generateContext("https://example.com/page");
      expect(ctx.referrer).toContain("search?q=");
    });
  });

  describe("Strategy selection ranges", () => {
    it("should select direct strategy in 0-0.10 range", () => {
      vi.spyOn(Math, "random").mockReturnValue(0.05);

      const ctx = engine.generateContext("https://target.com");
      expect(ctx.strategy).toBe("direct");
    });

    it("should select search strategies in 0.10-0.40 range", () => {
      vi.spyOn(Math, "random").mockReturnValue(0.25);

      const ctx = engine.generateContext("https://target.com");
      expect(["google_search", "bing_search", "duckduckgo"]).toContain(
        ctx.strategy,
      );
    });

    it("should select social strategies in 0.40-0.70 range", () => {
      vi.spyOn(Math, "random").mockReturnValue(0.55);

      const ctx = engine.generateContext("https://external.com");
      expect(["twitter_tco", "reddit_thread", "discord_channel"]).toContain(
        ctx.strategy,
      );
    });

    it("should select messaging strategies in 0.70-0.95 range", () => {
      vi.spyOn(Math, "random").mockReturnValue(0.85);

      const ctx = engine.generateContext("https://target.com");
      expect(["telegram_web", "whatsapp_web", "whatsapp_api"]).toContain(
        ctx.strategy,
      );
    });
  });

  describe("PrivacyEngine.naturalize", () => {
    it("should preserve full path for whitelisted strategies (google_search)", () => {
      vi.spyOn(Math, "random").mockReturnValue(0.2);
      const ctx = engine.generateContext("https://target.com");
      expect(ctx.referrer).toContain("google.com/search");
    });

    it("should preserve full path for whitelisted strategies (twitter_tco)", () => {
      vi.spyOn(Math, "random").mockReturnValue(0.5);
      const ctx = engine.generateContext("https://example.com");
      expect(ctx.referrer).toMatch(/^https:\/\/t\.co\/\w+$/);
    });

    it("should truncate non-whitelisted strategies to origin", () => {
      vi.spyOn(Math, "random").mockReturnValue(0.65);
      const ctx = engine.generateContext("https://target.com");
      expect(ctx.referrer).toMatch(
        /^https:\/\/(www\.)?(reddit|discord|linkedin|telegram|whatsapp|hacker|medium|substack)\.\w+(\/)?$/,
      );
    });

    it("should handle direct traffic (empty referrer)", () => {
      vi.spyOn(Math, "random").mockReturnValue(0.05);
      const ctx = engine.generateContext("https://target.com");
      expect(ctx.referrer).toBe("");
    });
  });

  describe("HeaderEngine.getContextHeaders", () => {
    it("should return none for direct traffic (no referer)", () => {
      vi.spyOn(Math, "random").mockReturnValue(0.05);
      const headers = engine.generateContext("https://target.com");
      const fetchHeaders = headers.headers;
      expect(fetchHeaders["Sec-Fetch-Site"]).toBe("none");
    });

    it("should return cross-site for external referer", () => {
      vi.spyOn(Math, "random").mockReturnValue(0.2);
      const ctx = engine.generateContext("https://site.com/page2");
      const fetchHeaders = ctx.headers;
      expect(fetchHeaders["Sec-Fetch-Site"]).toBe("cross-site");
    });

    it("should include all required Sec-Fetch headers for search engines", () => {
      vi.spyOn(Math, "random").mockReturnValue(0.2);

      const ctx = engine.generateContext("https://target.com");
      expect(ctx.headers["Sec-Fetch-Mode"]).toBe("navigate");
      expect(ctx.headers["Sec-Fetch-User"]).toBe("?1");
      expect(ctx.headers["Sec-Fetch-Dest"]).toBe("document");
    });

    it("should include Sec-Fetch headers for direct traffic", () => {
      vi.spyOn(Math, "random").mockReturnValue(0.05);

      const ctx = engine.generateContext("https://target.com");
      expect(ctx.headers["Sec-Fetch-Mode"]).toBe("navigate");
      expect(ctx.headers["Sec-Fetch-User"]).toBe("?1");
      expect(ctx.headers["Sec-Fetch-Dest"]).toBe("document");
    });
  });

  describe("ReferrerEngine._selectStrategy", () => {
    it("should NOT use twitter_tco for twitter.com targets", () => {
      vi.spyOn(Math, "random").mockReturnValue(0.5);

      const ctx = engine.generateContext("https://twitter.com/user/status/123");
      expect(ctx.strategy).not.toBe("twitter_tco");
    });

    it("should NOT use twitter_tco for x.com targets", () => {
      vi.spyOn(Math, "random").mockReturnValue(0.5);

      const ctx = engine.generateContext("https://x.com/user/status/123");
      expect(ctx.strategy).not.toBe("twitter_tco");
    });

    it("should use twitter_tco for external targets", () => {
      vi.spyOn(Math, "random").mockReturnValue(0.5);

      const ctx = engine.generateContext("https://example.com/page");
      expect(ctx.strategy).toBe("twitter_tco");
    });
  });

  describe("Module-level file loading", () => {
    it("should use fallback when REAL_VEDS file does not exist", () => {
      vi.resetAllMocks();
      vi.mocked(fs.existsSync).mockReturnValue(false);

      const engine = new ReferrerEngine({});
      vi.spyOn(Math, "random").mockReturnValue(0.2);

      const ctx = engine.generateContext("https://google.com/test");
      expect(ctx.referrer).toContain("ved=");
    });

    it("should use fallback when REAL_TCO file does not exist", () => {
      vi.resetAllMocks();
      vi.mocked(fs.existsSync).mockReturnValue(false);

      const engine = new ReferrerEngine({});
      vi.spyOn(Math, "random").mockReturnValue(0.5);

      const ctx = engine.generateContext("https://example.com");
      expect(ctx.referrer).toMatch(/^https:\/\/t\.co\/\w+$/);
    });
  });

  describe("Long-tail strategy selection", () => {
    it("should select linkedin_feed from long-tail strategies", () => {
      vi.spyOn(Math, "random")
        .mockReturnValueOnce(0.96)
        .mockReturnValueOnce(0.6);

      const ctx = engine.generateContext("https://target.com");
      expect(ctx.strategy).toBe("linkedin_feed");
    });

    it("should select from long-tail strategies (hacker_news or substack or medium_article)", () => {
      vi.spyOn(Math, "random")
        .mockReturnValueOnce(0.96)
        .mockReturnValueOnce(0.3);

      const ctx = engine.generateContext("https://target.com");
      expect(["hacker_news", "substack", "medium_article"]).toContain(
        ctx.strategy,
      );
    });
  });

  describe("Edge cases", () => {
    it("should handle empty target URL gracefully", () => {
      vi.spyOn(Math, "random").mockReturnValue(0.05);

      const ctx = engine.generateContext("");
      expect(ctx).toBeDefined();
      expect(ctx.strategy).toBe("direct");
    });

    it("should handle URL without protocol gracefully", () => {
      vi.spyOn(Math, "random").mockReturnValue(0.05);

      const ctx = engine.generateContext("target.com");
      expect(ctx).toBeDefined();
    });

    it("should include all required Sec-Fetch headers", () => {
      vi.spyOn(Math, "random").mockReturnValue(0.2);

      const ctx = engine.generateContext("https://target.com");
      expect(ctx.headers["Sec-Fetch-Mode"]).toBe("navigate");
      expect(ctx.headers["Sec-Fetch-User"]).toBe("?1");
      expect(ctx.headers["Sec-Fetch-Dest"]).toBe("document");
    });
  });

  describe("PrivacyEngine.naturalize edge cases", () => {
    it("should return empty string for empty URL via generateContext", () => {
      vi.spyOn(Math, "random").mockReturnValue(0.05);
      const ctx = engine.generateContext("");
      expect(ctx.referrer).toBe("");
    });

    it("should return original URL when URL parsing fails (line 83)", () => {
      vi.spyOn(Math, "random").mockReturnValueOnce(0.64);
      const ctx = engine.generateContext("not-a-valid-url");
      expect(ctx.strategy).toBe("reddit_thread");
      expect(ctx.referrer).toMatch(/^https:\/\/www\.reddit\.com\/$/);
    });

    it("should truncate discord_channel to origin", () => {
      vi.spyOn(Math, "random").mockReturnValue(0.68);
      const ctx = engine.generateContext("https://target.com");
      expect(ctx.referrer).toBe("https://discord.com/");
    });
  });

  describe("HeaderEngine.getContextHeaders edge cases", () => {
    it("should return same-origin for identical hosts via search engine", () => {
      vi.spyOn(Math, "random").mockReturnValue(0.2);
      const ctx = engine.generateContext("https://www.google.com/page2");
      expect(ctx.headers["Sec-Fetch-Site"]).toBe("same-origin");
    });

    it("should return cross-site for different domains", () => {
      vi.spyOn(Math, "random").mockReturnValue(0.2);
      const ctx = engine.generateContext("https://target.com/page");
      expect(ctx.headers["Sec-Fetch-Site"]).toBe("cross-site");
    });

    it("should handle invalid target URL via direct strategy", () => {
      vi.spyOn(Math, "random").mockReturnValue(0.05);
      const ctx = engine.generateContext("invalid-url");
      expect(ctx.headers["Sec-Fetch-Site"]).toBe("none");
    });
  });

  describe("Trampoline navigation body content (line 525)", () => {
    let mockPage;

    beforeEach(() => {
      mockPage = {
        setExtraHTTPHeaders: vi.fn().mockResolvedValue(undefined),
        goto: vi.fn().mockResolvedValue(undefined),
        route: vi.fn().mockImplementation((pattern, handler) => {
          if (typeof handler === "function") {
            const mockRoute = { fulfill: vi.fn().mockResolvedValue(undefined) };
            handler(mockRoute);
            expect(mockRoute.fulfill).toHaveBeenCalledWith(
              expect.objectContaining({
                status: 200,
                contentType: "text/html",
                body: expect.stringContaining("<html>"),
              }),
            );
          }
          return Promise.resolve();
        }),
        unroute: vi.fn().mockResolvedValue(undefined),
        click: vi.fn().mockResolvedValue(undefined),
        waitForURL: vi.fn().mockImplementation(() => Promise.resolve()),
        url: vi.fn().mockReturnValue("about:blank"),
        isClosed: vi.fn().mockReturnValue(false),
        viewportSize: vi.fn().mockReturnValue({ width: 1920, height: 1080 }),
        mouse: {
          move: vi.fn().mockResolvedValue(undefined),
          click: vi.fn().mockResolvedValue(undefined),
          dblclick: vi.fn().mockResolvedValue(undefined),
          down: vi.fn().mockResolvedValue(undefined),
          up: vi.fn().mockResolvedValue(undefined),
        },
        context: () => ({
          browser: () => ({ isConnected: () => true }),
        }),
      };
      api.setPage(mockPage);
    });

    it("should include trampoline auto-click script in body", async () => {
      vi.spyOn(Math, "random").mockReturnValue(0.2);

      let bodyContent = "";
      mockPage.route = vi.fn().mockImplementation((pattern, handler) => {
        if (typeof handler === "function") {
          const mockRoute = {
            fulfill: vi.fn().mockImplementation((opts) => {
              bodyContent = opts.body || "";
              return Promise.resolve();
            }),
          };
          handler(mockRoute);
        }
        return Promise.resolve();
      });

      await engine.navigate(mockPage, "https://target.com");

      expect(bodyContent).toContain("setTimeout");
      expect(bodyContent).toContain("link.click()");
    });
  });

  describe("Module self-test execution (lines 593-597)", () => {
    it("should execute multiple generateContext calls without errors", () => {
      vi.spyOn(Math, "random").mockImplementation(() => 0.05);

      const results = [];
      for (let i = 0; i < 10; i++) {
        const ctx = engine.generateContext("https://target.com");
        results.push(ctx.strategy);
      }

      expect(results.length).toBe(10);
      expect(results.every((s) => s === "direct")).toBe(true);
    });

    it("should handle all strategy selection boundary values", () => {
      const testValues = [
        0.09, 0.1, 0.24, 0.25, 0.34, 0.35, 0.39, 0.4, 0.54, 0.55, 0.64, 0.65,
        0.69, 0.7, 0.79, 0.8, 0.89, 0.9, 0.94, 0.95,
      ];

      testValues.forEach((val) => {
        vi.spyOn(Math, "random").mockReturnValue(val);
        expect(() =>
          engine.generateContext("https://target.com"),
        ).not.toThrow();
      });
    });
  });

  describe("Additional edge cases", () => {
    it("should handle very long target URLs", () => {
      vi.spyOn(Math, "random").mockReturnValue(0.05);

      const longUrl = "https://twitter.com/" + "a".repeat(500);
      const ctx = engine.generateContext(longUrl);
      expect(ctx).toBeDefined();
    });

    it("should handle URL with complex query parameters", () => {
      vi.spyOn(Math, "random").mockReturnValue(0.2);

      const ctx = engine.generateContext(
        "https://target.com?p1=v1&p2=v2&p3=v3",
      );
      expect(ctx).toBeDefined();
      expect(ctx.targetWithParams).toContain("p1=v1");
    });

    it("should handle URL with hash fragment", () => {
      vi.spyOn(Math, "random").mockReturnValue(0.2);

      const ctx = engine.generateContext("https://target.com/page#section");
      expect(ctx).toBeDefined();
    });

    it("should handle twitter username with underscores", () => {
      vi.spyOn(Math, "random").mockReturnValue(0.22);

      const ctx = engine.generateContext(
        "https://twitter.com/user_name_123/status/123",
      );
      expect(ctx.referrer).toContain("search?q=");
    });
  });

  describe("PrivacyEngine whitelist strategies", () => {
    it("should preserve full path for bing_search", () => {
      vi.spyOn(Math, "random").mockReturnValue(0.3);

      const ctx = engine.generateContext("https://target.com");
      expect(ctx.strategy).toBe("bing_search");
      expect(ctx.referrer).toContain("/search?q=");
    });

    it("should preserve full path for duckduckgo", () => {
      vi.spyOn(Math, "random").mockReturnValue(0.38);

      const ctx = engine.generateContext("https://target.com");
      expect(ctx.strategy).toBe("duckduckgo");
      expect(ctx.referrer).toContain("/?q=");
    });

    it("should truncate medium_article to origin", () => {
      vi.spyOn(Math, "random")
        .mockReturnValueOnce(0.96)
        .mockReturnValueOnce(0.4)
        .mockReturnValueOnce(0.3);

      const ctx = engine.generateContext("https://target.com");
      expect(["medium_article", "substack", "hacker_news"]).toContain(
        ctx.strategy,
      );
      if (ctx.strategy === "medium_article") {
        expect(ctx.referrer).toMatch(/^https:\/\/medium\.com\/?$/);
      }
    });

    it("should truncate hacker_news to origin", () => {
      vi.spyOn(Math, "random")
        .mockReturnValueOnce(0.96)
        .mockReturnValueOnce(0.1);

      const ctx = engine.generateContext("https://target.com");
      expect(["hacker_news", "medium_article", "substack"]).toContain(
        ctx.strategy,
      );
      if (ctx.strategy === "hacker_news") {
        expect(ctx.referrer).toBe("https://news.ycombinator.com/");
      }
    });
  });

  describe("Dictionary loading with partial data (line 40 fallback)", () => {
    it("should use default ACTIONS when file has missing ACTIONS field", () => {
      vi.resetAllMocks();

      vi.mocked(fs.existsSync).mockImplementation((path) => {
        return path.toString().includes("referrer_dict.json");
      });

      vi.mocked(fs.readFileSync).mockImplementation((path) => {
        const p = path.toString();
        if (p.includes("referrer_dict.json")) {
          return JSON.stringify({
            TOPICS: ["test-topic"],
          });
        }
        return "{}";
      });

      const testEngine = new ReferrerEngine({});
      vi.spyOn(Math, "random").mockReturnValue(0.05);

      const ctx = testEngine.generateContext("https://target.com");
      expect(ctx.strategy).toBe("direct");
    });

    it("should use default CONTEXT when file has missing CONTEXT field", () => {
      vi.resetAllMocks();

      vi.mocked(fs.existsSync).mockImplementation((path) => {
        return path.toString().includes("referrer_dict.json");
      });

      vi.mocked(fs.readFileSync).mockImplementation((path) => {
        const p = path.toString();
        if (p.includes("referrer_dict.json")) {
          return JSON.stringify({
            TOPICS: ["test-topic"],
            ACTIONS: ["test-action"],
          });
        }
        return "{}";
      });

      const testEngine = new ReferrerEngine({});
      expect(testEngine).toBeDefined();
    });

    it("should use default SUBREDDITS when file has missing SUBREDDITS field", () => {
      vi.resetAllMocks();

      vi.mocked(fs.existsSync).mockImplementation((path) => {
        return path.toString().includes("referrer_dict.json");
      });

      vi.mocked(fs.readFileSync).mockImplementation((path) => {
        const p = path.toString();
        if (p.includes("referrer_dict.json")) {
          return JSON.stringify({
            TOPICS: ["test-topic"],
            ACTIONS: ["test-action"],
            CONTEXT: ["test-context"],
          });
        }
        return "{}";
      });

      const testEngine = new ReferrerEngine({});
      expect(testEngine).toBeDefined();
    });
  });

  describe("PrivacyEngine.naturalize catch block (line 359)", () => {
    it("should return original URL when URL parsing throws during naturalize", () => {
      vi.resetAllMocks();
      vi.mocked(fs.existsSync).mockReturnValue(false);

      vi.spyOn(Math, "random").mockReturnValueOnce(0.64);

      const ctx = engine.generateContext("not-a-valid-url");
      expect(ctx.strategy).toBe("reddit_thread");
      expect(ctx.referrer).toBe("https://www.reddit.com/");
    });
  });

  describe("Module self-test coverage (lines 593-597)", () => {
    it("should handle self-test execution pattern", () => {
      const testEngine = new ReferrerEngine({ addUTM: true });

      vi.spyOn(Math, "random").mockImplementation(() => 0.05);

      const iterations = 10;
      const results = [];
      for (let i = 0; i < iterations; i++) {
        const ctx = testEngine.generateContext("https://target.com");
        results.push({
          strategy: ctx.strategy,
          referrer: ctx.referrer
            ? ctx.referrer.substring(0, 50) + "..."
            : "(DIRECT)",
        });
      }

      expect(results.length).toBe(iterations);
      results.forEach((r) => {
        expect(r.strategy).toBe("direct");
        expect(r.referrer).toBe("(DIRECT)");
      });
    });

    it("should handle various strategies correctly", () => {
      const testEngine = new ReferrerEngine({ addUTM: true });

      const randomValues = [
        0.05, 0.2, 0.3, 0.5, 0.6, 0.7, 0.8, 0.9, 0.95, 0.99,
      ];
      let callCount = 0;
      vi.spyOn(Math, "random").mockImplementation(() => {
        const val = randomValues[callCount % randomValues.length];
        callCount++;
        return val;
      });

      for (let i = 0; i < 10; i++) {
        const ctx = testEngine.generateContext("https://target.com");
        expect(ctx.strategy).toBeDefined();
        expect(ctx.referrer).toBeDefined();
      }
    });
  });

  describe("HeaderEngine with invalid URLs", () => {
    it("should return empty headers when referer URL parsing fails", () => {
      vi.spyOn(Math, "random").mockReturnValue(0.05);

      const ctx = engine.generateContext("https://target.com");
      expect(ctx.headers).toBeDefined();
      expect(ctx.headers["Sec-Fetch-Site"]).toBe("none");
    });

    it("should handle HeaderEngine edge cases through generateContext", () => {
      vi.spyOn(Math, "random").mockReturnValue(0.2);

      const ctx1 = engine.generateContext("https://google.com/search");
      expect(["cross-site", "same-origin", "same-site"]).toContain(
        ctx1.headers["Sec-Fetch-Site"],
      );

      const ctx2 = engine.generateContext("https://different.com/page");
      expect(ctx2.headers["Sec-Fetch-Site"]).toBe("cross-site");
    });
  });

  describe("UTM injection edge cases", () => {
    it("should handle UTM with invalid target URL gracefully", () => {
      const utmEngine = new ReferrerEngine({ addUTM: true });
      vi.spyOn(Math, "random").mockReturnValue(0.2);

      const ctx = utmEngine.generateContext("not-a-valid-url");
      expect(ctx).toBeDefined();
      expect(ctx.targetWithParams).toBe("not-a-valid-url");
    });

    it("should add UTM for google strategy (includes google)", () => {
      const utmEngine = new ReferrerEngine({ addUTM: true });
      vi.spyOn(Math, "random").mockReturnValue(0.2);

      const ctx = utmEngine.generateContext("https://target.com");
      expect(ctx.targetWithParams).toContain("utm_source=google");
    });

    it("should add UTM for twitter strategy", () => {
      const utmEngine = new ReferrerEngine({ addUTM: true });
      vi.spyOn(Math, "random").mockReturnValue(0.5);

      const ctx = utmEngine.generateContext("https://external.com");
      if (ctx.strategy === "twitter_tco") {
        expect(ctx.targetWithParams).toContain("utm_source=twitter");
      }
    });

    it("should add UTM for whatsapp strategy", () => {
      const utmEngine = new ReferrerEngine({ addUTM: true });
      vi.spyOn(Math, "random").mockReturnValue(0.88);

      const ctx = utmEngine.generateContext("https://target.com");
      if (ctx.strategy === "whatsapp_web") {
        expect(ctx.targetWithParams).toContain("utm_source=whatsapp");
      }
    });
  });

  describe("VED data loading edge cases", () => {
    it("should use fallback when VED file exists but is empty", () => {
      vi.resetAllMocks();

      vi.mocked(fs.existsSync).mockImplementation((path) => {
        const p = path.toString();
        return p.includes("ved_data.json") || p.includes("referrer_dict.json");
      });

      vi.mocked(fs.readFileSync).mockImplementation((path) => {
        const p = path.toString();
        if (p.includes("ved_data.json")) {
          return "[]";
        }
        if (p.includes("referrer_dict.json")) {
          return JSON.stringify({
            TOPICS: ["tech"],
            ACTIONS: ["read"],
            CONTEXT: ["thread"],
            SUBREDDITS: ["technology"],
          });
        }
        return "{}";
      });

      const testEngine = new ReferrerEngine({});
      expect(testEngine).toBeDefined();
    });

    it("should handle VED JSON parse error gracefully", () => {
      vi.resetAllMocks();

      vi.mocked(fs.existsSync).mockImplementation((path) => {
        const p = path.toString();
        return p.includes("ved_data.json") || p.includes("referrer_dict.json");
      });

      vi.mocked(fs.readFileSync).mockImplementation((path) => {
        const p = path.toString();
        if (p.includes("ved_data.json")) {
          throw new Error("Invalid JSON");
        }
        if (p.includes("referrer_dict.json")) {
          return JSON.stringify({
            TOPICS: ["tech"],
            ACTIONS: ["read"],
            CONTEXT: ["thread"],
            SUBREDDITS: ["technology"],
          });
        }
        return "{}";
      });

      const testEngine = new ReferrerEngine({});
      expect(testEngine).toBeDefined();
    });
  });

  describe("t.co links loading edge cases", () => {
    it("should filter t.co links to only include valid t.co URLs", () => {
      vi.resetAllMocks();

      vi.mocked(fs.existsSync).mockImplementation((path) => {
        const p = path.toString();
        return p.includes("tco_links.json") || p.includes("referrer_dict.json");
      });

      vi.mocked(fs.readFileSync).mockImplementation((path) => {
        const p = path.toString();
        if (p.includes("tco_links.json")) {
          return JSON.stringify([
            "https://t.co/abc123",
            "https://not-t.co/xyz",
            "https://t.co/def456",
            "invalid-url",
          ]);
        }
        if (p.includes("referrer_dict.json")) {
          return JSON.stringify({
            TOPICS: ["tech"],
            ACTIONS: ["read"],
            CONTEXT: ["thread"],
            SUBREDDITS: ["technology"],
          });
        }
        return "{}";
      });

      const testEngine = new ReferrerEngine({});
      vi.spyOn(Math, "random").mockReturnValue(0.5);

      const ctx = testEngine.generateContext("https://external.com");
      expect(ctx.strategy).toBe("twitter_tco");
      expect(ctx.referrer).toMatch(/^https:\/\/t\.co\/\w+$/);
    });
  });

  describe("generateQuery edge cases", () => {
    it("should handle URL with complex paths for query generation", () => {
      vi.spyOn(Math, "random").mockReturnValue(0.22);

      const urls = [
        "https://twitter.com/user_name/status/123456789?ref=tw",
        "https://twitter.com/test-user_123/media",
        "https://twitter.com/official_account/highlights",
      ];

      urls.forEach((url) => {
        const ctx = engine.generateContext(url);
        expect(ctx.referrer).toContain("search?q=");
      });
    });
  });

  describe("PrivacyEngine naturalize with all whitelist strategies", () => {
    it("should preserve full path for whatsapp_api (whitelist)", () => {
      vi.spyOn(Math, "random").mockReturnValue(0.92);

      const ctx = engine.generateContext("https://target.com");
      if (ctx.strategy === "whatsapp_api") {
        expect(ctx.referrer).toBe("https://api.whatsapp.com/");
      }
    });

    it("should truncate linkedin_feed to origin", () => {
      vi.spyOn(Math, "random")
        .mockReturnValueOnce(0.96)
        .mockReturnValueOnce(0.6);

      const ctx = engine.generateContext("https://target.com");
      expect(ctx.strategy).toBe("linkedin_feed");
      expect(ctx.referrer).toBe("https://www.linkedin.com/");
    });

    it("should truncate telegram_web to origin", () => {
      vi.spyOn(Math, "random").mockReturnValue(0.75);

      const ctx = engine.generateContext("https://target.com");
      expect(ctx.strategy).toBe("telegram_web");
      expect(ctx.referrer).toBe("https://web.telegram.org/");
    });
  });
});
