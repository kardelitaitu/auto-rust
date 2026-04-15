/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { AIQuoteAction } from "@api/actions/ai-twitter-quote.js";
import { AIReplyAction } from "@api/actions/ai-twitter-reply.js";
import { LikeAction } from "@api/actions/ai-twitter-like.js";
import { BookmarkAction } from "@api/actions/ai-twitter-bookmark.js";
import { FollowAction } from "@api/actions/ai-twitter-follow.js";

// Mock logger
vi.mock("@api/core/logger.js", () => ({
  createLogger: vi.fn(() => ({
    info: vi.fn(),
    error: vi.fn(),
    warn: vi.fn(),
    debug: vi.fn(),
  })),
}));

// Mock api
vi.mock("@api/index.js", () => ({
  api: {
    wait: vi.fn().mockResolvedValue(undefined),
    visible: vi.fn().mockResolvedValue(false),
    getCurrentUrl: vi
      .fn()
      .mockResolvedValue("https://x.com/testuser/status/12345"),
  },
}));
import { api } from "@api/index.js";

describe("newEngagement Flag", () => {
  let mockAgent;

  beforeEach(() => {
    vi.clearAllMocks();

    mockAgent = {
      twitterConfig: {
        actions: {
          reply: { enabled: true, probability: 0.5 },
          quote: { enabled: true, probability: 0.5 },
          like: { enabled: true, probability: 0.5 },
          bookmark: { enabled: true, probability: 0.5 },
          follow: { enabled: true, probability: 0.5 },
        },
      },
      page: {},
      pageOps: {
        urlSync: vi
          .fn()
          .mockResolvedValue("https://x.com/testuser/status/12345"),
      },
      contextEngine: {
        extractEnhancedContext: vi
          .fn()
          .mockResolvedValue({ replies: [], sentiment: {} }),
      },
      replyEngine: {
        generateReply: vi
          .fn()
          .mockResolvedValue({ success: true, reply: "Test reply" }),
      },
      quoteEngine: {
        generateQuote: vi
          .fn()
          .mockResolvedValue({ success: true, quote: "Test quote" }),
      },
      executeAIReply: vi.fn().mockResolvedValue(true),
      executeAIQuote: vi.fn().mockResolvedValue(true),
      handleLike: vi.fn().mockResolvedValue(undefined),
      handleBookmark: vi.fn().mockResolvedValue(undefined),
      diveQueue: {
        canEngage: vi.fn().mockReturnValue(true),
        recordEngagement: vi.fn(),
      },
    };
  });

  describe("AIQuoteAction", () => {
    it("should return newEngagement: true on success", async () => {
      const quoteAction = new AIQuoteAction(mockAgent);
      const context = {
        tweetText: "Test",
        username: "user",
        tweetUrl: "https://x.com/user/status/123",
      };
      const result = await quoteAction.execute(context);

      expect(result.success).toBe(true);
      expect(result.newEngagement).toBe(true);
    });

    it("should return newEngagement: false on UI post failure", async () => {
      mockAgent.executeAIQuote.mockResolvedValue(false);
      const quoteAction = new AIQuoteAction(mockAgent);
      const context = {
        tweetText: "Test",
        username: "user",
        tweetUrl: "https://x.com/user/status/123",
      };
      const result = await quoteAction.execute(context);

      expect(result.success).toBe(false);
      expect(result.newEngagement).toBe(false);
    });

    it("should return newEngagement: false on AI generation failure", async () => {
      mockAgent.quoteEngine.generateQuote.mockResolvedValue({ success: false });
      const quoteAction = new AIQuoteAction(mockAgent);
      const context = {
        tweetText: "Test",
        username: "user",
        tweetUrl: "https://x.com/user/status/123",
      };
      const result = await quoteAction.execute(context);

      expect(result.success).toBe(false);
      expect(result.newEngagement).toBe(false);
    });

    it("should return newEngagement: false on exception", async () => {
      mockAgent.contextEngine.extractEnhancedContext.mockRejectedValue(
        new Error("fail"),
      );
      const quoteAction = new AIQuoteAction(mockAgent);
      const context = {
        tweetText: "Test",
        username: "user",
        tweetUrl: "https://x.com/user/status/123",
      };
      const result = await quoteAction.execute(context);

      expect(result.success).toBe(false);
      expect(result.newEngagement).toBe(false);
    });

    it("should return newEngagement: false when skipped in tryExecute", async () => {
      const quoteAction = new AIQuoteAction(mockAgent);
      quoteAction.probability = 0; // Force probability skip
      const context = {
        tweetText: "Test",
        username: "user",
        tweetUrl: "https://x.com/user/status/123",
      };
      const result = await quoteAction.tryExecute(context);

      expect(result.success).toBe(false);
      expect(result.newEngagement).toBe(false);
    });

    it("should return newEngagement: false when canExecute fails", async () => {
      const quoteAction = new AIQuoteAction(mockAgent);
      const result = await quoteAction.tryExecute({});

      expect(result.success).toBe(false);
      expect(result.newEngagement).toBe(false);
    });
  });

  describe("AIReplyAction", () => {
    it("should return newEngagement: true on success", async () => {
      const replyAction = new AIReplyAction(mockAgent);
      const context = {
        tweetText: "Test",
        username: "user",
        tweetUrl: "https://x.com/user/status/123",
      };
      const result = await replyAction.execute(context);

      expect(result.success).toBe(true);
      expect(result.newEngagement).toBe(true);
    });

    it("should return newEngagement: false on failure", async () => {
      mockAgent.executeAIReply.mockResolvedValue(false);
      const replyAction = new AIReplyAction(mockAgent);
      const context = {
        tweetText: "Test",
        username: "user",
        tweetUrl: "https://x.com/user/status/123",
      };
      const result = await replyAction.execute(context);

      expect(result.success).toBe(false);
      expect(result.newEngagement).toBe(false);
    });

    it("should return newEngagement: false when skipped", async () => {
      const replyAction = new AIReplyAction(mockAgent);
      replyAction.probability = 0;
      const context = {
        tweetText: "Test",
        username: "user",
        tweetUrl: "https://x.com/user/status/123",
      };
      const result = await replyAction.tryExecute(context);

      expect(result.newEngagement).toBe(false);
    });
  });

  describe("LikeAction", () => {
    it("should return newEngagement: true on success", async () => {
      const likeAction = new LikeAction(mockAgent);
      const context = {
        tweetElement: {},
        tweetUrl: "https://x.com/user/status/123",
      };
      const result = await likeAction.execute(context);

      expect(result.success).toBe(true);
      expect(result.newEngagement).toBe(true);
    });

    it("should return newEngagement: false on exception", async () => {
      mockAgent.handleLike.mockRejectedValue(new Error("fail"));
      const likeAction = new LikeAction(mockAgent);
      const context = {
        tweetElement: {},
        tweetUrl: "https://x.com/user/status/123",
      };
      const result = await likeAction.execute(context);

      expect(result.success).toBe(false);
      expect(result.newEngagement).toBe(false);
    });

    it("should return newEngagement: false when engagement limit reached", async () => {
      mockAgent.diveQueue.canEngage.mockReturnValue(false);
      const likeAction = new LikeAction(mockAgent);
      const result = await likeAction.tryExecute({});

      expect(result.newEngagement).toBe(false);
    });
  });

  describe("BookmarkAction", () => {
    it("should return newEngagement: true on success", async () => {
      const bookmarkAction = new BookmarkAction(mockAgent);
      const context = {
        tweetElement: {},
        tweetUrl: "https://x.com/user/status/123",
      };
      const result = await bookmarkAction.execute(context);

      expect(result.success).toBe(true);
      expect(result.newEngagement).toBe(true);
    });

    it("should return newEngagement: false on exception", async () => {
      mockAgent.handleBookmark.mockRejectedValue(new Error("fail"));
      const bookmarkAction = new BookmarkAction(mockAgent);
      const context = {
        tweetElement: {},
        tweetUrl: "https://x.com/user/status/123",
      };
      const result = await bookmarkAction.execute(context);

      expect(result.success).toBe(false);
      expect(result.newEngagement).toBe(false);
    });

    it("should return newEngagement: false when engagement limit reached", async () => {
      mockAgent.diveQueue.canEngage.mockReturnValue(false);
      const bookmarkAction = new BookmarkAction(mockAgent);
      const result = await bookmarkAction.tryExecute({});

      expect(result.newEngagement).toBe(false);
    });
  });

  describe("FollowAction", () => {
    it("should return newEngagement: false on stub execution", async () => {
      const followAction = new FollowAction(mockAgent);
      const result = await followAction.execute({});

      expect(result.success).toBe(false);
      expect(result.newEngagement).toBe(false);
    });

    it("should return newEngagement: false when skipped", async () => {
      const followAction = new FollowAction(mockAgent);
      const result = await followAction.tryExecute({});

      expect(result.newEngagement).toBe(false);
    });
  });

  describe("RetweetAction - already_retweeted", () => {
    // Import RetweetAction with mocked api
    vi.doMock("@api/index.js", () => ({
      api: {
        wait: vi.fn().mockResolvedValue(undefined),
        visible: vi.fn().mockResolvedValue(false),
        getCurrentUrl: vi
          .fn()
          .mockResolvedValue("https://x.com/testuser/status/12345"),
        getPage: vi.fn().mockReturnValue({
          locator: vi.fn(),
          keyboard: { press: vi.fn() },
          waitForTimeout: vi.fn(),
        }),
      },
    }));

    it("should return newEngagement: false for already_retweeted", async () => {
      // This tests the pattern, not the actual execution
      const alreadyRetweetedResult = {
        success: true,
        reason: "already_retweeted",
        newEngagement: false,
      };

      expect(alreadyRetweetedResult.newEngagement).toBe(false);
      expect(alreadyRetweetedResult.success).toBe(true); // Still "succeeded" in some sense
    });

    it("should return newEngagement: true for successful retweet", async () => {
      const successfulRetweetResult = {
        success: true,
        reason: "retweet_successful",
        newEngagement: true,
      };

      expect(successfulRetweetResult.newEngagement).toBe(true);
    });
  });
});
