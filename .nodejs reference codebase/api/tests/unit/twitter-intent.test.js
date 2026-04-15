import { describe, it, expect, vi, beforeEach } from "vitest";
import { like } from "@api/twitter/intent-like.js";
import { quote } from "@api/twitter/intent-quote.js";
import { retweet } from "@api/twitter/intent-retweet.js";
import { follow } from "@api/twitter/intent-follow.js";

// Mock dependencies
vi.mock("@api/core/context.js", () => ({
  getPage: vi.fn(),
}));
vi.mock("@api/core/logger.js", () => ({
  createLogger: () => ({ info: vi.fn(), warn: vi.fn(), error: vi.fn() }),
}));
vi.mock("@api/interactions/wait.js", () => ({
  wait: vi.fn().mockResolvedValue(true),
  waitForLoadState: vi.fn().mockResolvedValue(true),
}));
vi.mock("@api/interactions/actions.js", () => ({
  click: vi.fn().mockResolvedValue(true),
}));
vi.mock("@api/interactions/navigation.js", () => ({
  goto: vi.fn().mockResolvedValue(true),
  back: vi.fn().mockResolvedValue(true),
}));
vi.mock("@api/interactions/queries.js", () => ({
  visible: vi.fn(),
}));

import { goto, back } from "@api/interactions/navigation.js";
import { visible } from "@api/interactions/queries.js";
import { click } from "@api/interactions/actions.js";

describe("Twitter Intent Helpers", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    visible.mockResolvedValue(true); // Default to confirm button being visible
  });

  describe("intent-like", () => {
    it("should navigate to like intent and confirm", async () => {
      const url = "https://x.com/user/status/123456";
      const res = await like(url);

      expect(res.success).toBe(true);
      expect(goto).toHaveBeenCalledWith(
        "https://x.com/intent/like?tweet_id=123456",
      );
      expect(click).toHaveBeenCalledWith(
        '[data-testid="confirmationSheetConfirm"]',
      );
      expect(back).toHaveBeenCalled();
    });

    it("should return invalid URL if tweet ID is not parseable", async () => {
      const url = "https://x.com/someUser/timeline";
      const res = await like(url);
      expect(res.success).toBe(false);
      expect(res.reason).toBe("invalid_tweet_url");
      expect(goto).not.toHaveBeenCalled();
    });

    it("should return confirm_button_not_found if visible returns false", async () => {
      visible.mockResolvedValue(false);
      const url = "https://x.com/user/status/123456";
      const res = await like(url);
      expect(res.success).toBe(false);
      expect(res.reason).toBe("confirm_button_not_found");
      expect(goto).toHaveBeenCalledWith(
        "https://x.com/intent/like?tweet_id=123456",
      );
      expect(click).not.toHaveBeenCalled();
    });

    it("should catch unhandled errors and return success: false", async () => {
      goto.mockRejectedValueOnce(new Error("Navigation failed"));
      const res = await like("https://x.com/user/status/123456");
      expect(res.success).toBe(false);
      expect(res.reason).toBe("unhandled_error");
      expect(res.error).toBe("Navigation failed");
    });

    it("should trigger timeout if operation takes more than 20s", async () => {
      vi.useFakeTimers({ legacy: true });
      goto.mockImplementationOnce(
        () => new Promise((resolve) => setTimeout(resolve, 30000)),
      );

      const promise = like("https://x.com/user/status/123456");
      await vi.advanceTimersByTimeAsync(21000);

      const res = await promise;
      expect(res.success).toBe(false);
      expect(res.reason).toBe("timeout");
      vi.useRealTimers();
    });
  });

  describe("intent-retweet", () => {
    it("should navigate to retweet intent and confirm", async () => {
      const url = "https://x.com/user/status/123456";
      const res = await retweet(url);

      expect(res.success).toBe(true);
      expect(goto).toHaveBeenCalledWith(
        "https://x.com/intent/retweet?tweet_id=123456",
      );
      expect(click).toHaveBeenCalledWith(
        '[data-testid="confirmationSheetConfirm"]',
      );
      expect(back).toHaveBeenCalled();
    });

    it("should return error on invalid URL pattern", async () => {
      const res = await retweet("invalid-url");
      expect(res.success).toBe(false);
      expect(res.reason).toBe("invalid_tweet_url");
    });

    it("should catch unhandled errors and return success: false", async () => {
      goto.mockRejectedValueOnce(new Error("Interaction failed"));
      const res = await retweet("https://x.com/user/status/123");
      expect(res.success).toBe(false);
      expect(res.reason).toBe("unhandled_error");
    });

    it("should trigger timeout on slow navigation", async () => {
      vi.useFakeTimers({ legacy: true });
      goto.mockImplementationOnce(
        () => new Promise((resolve) => setTimeout(resolve, 30000)),
      );
      const promise = retweet("https://x.com/user/status/123");
      await vi.advanceTimersByTimeAsync(21000);
      const res = await promise;
      expect(res.success).toBe(false);
      expect(res.reason).toBe("timeout");
      vi.useRealTimers();
    });
  });

  describe("intent-quote", () => {
    it("should navigate to quote intent and click tweet button", async () => {
      const url = "https://x.com/user/status/123456";
      const text = "Check this out!";
      const res = await quote(url, text);

      expect(res.success).toBe(true);
      expect(goto).toHaveBeenCalledWith(
        "https://x.com/intent/tweet?url=https%3A%2F%2Fx.com%2Fuser%2Fstatus%2F123456&text=Check%20this%20out!",
      );
      expect(click).toHaveBeenCalledWith('[data-testid="tweetButton"]');
      expect(back).toHaveBeenCalled();
    });

    it("should handle multiline text correctly", async () => {
      const url = "https://x.com/user/status/123";
      const text = "Line 1\nLine 2";
      await quote(url, text);
      expect(goto).toHaveBeenCalledWith(
        expect.stringContaining("text=Line%201%0ALine%202"),
      );
    });

    it("should fail if missing parameters", async () => {
      const res = await quote("https://x.com/user/status/123");
      expect(res.success).toBe(false);
      expect(res.reason).toBe("missing_parameters");
    });

    it("should return tweet_button_not_found if button is not visible", async () => {
      visible.mockResolvedValue(false);
      const res = await quote("https://x.com/user/status/123456", "text");
      expect(res.success).toBe(false);
      expect(res.reason).toBe("tweet_button_not_found");
    });

    it("should catch unhandled errors and return success: false", async () => {
      goto.mockRejectedValueOnce(new Error("Quote failed"));
      const res = await quote("https://x.com/user/status/123", "text");
      expect(res.success).toBe(false);
      expect(res.reason).toBe("unhandled_error");
    });

    it("should trigger timeout on slow quote", async () => {
      vi.useFakeTimers({ legacy: true });
      goto.mockImplementationOnce(
        () => new Promise((resolve) => setTimeout(resolve, 30000)),
      );
      const promise = quote("https://x.com/user/status/123", "text");
      await vi.advanceTimersByTimeAsync(21000);
      const res = await promise;
      expect(res.success).toBe(false);
      expect(res.reason).toBe("timeout");
      vi.useRealTimers();
    });
  });

  describe("intent-follow", () => {
    it("should navigate to follow intent and confirm", async () => {
      const url = "https://x.com/username";
      const res = await follow(url);

      expect(res.success).toBe(true);
      expect(goto).toHaveBeenCalledWith(
        "https://x.com/intent/follow?screen_name=username",
      );
      expect(click).toHaveBeenCalledWith(
        '[data-testid="confirmationSheetConfirm"]',
      );
      expect(back).toHaveBeenCalled();
    });

    it("should handle complex URLs and extract username", async () => {
      const res = await follow("https://x.com/testuser/status/123456");
      expect(res.success).toBe(true);
      expect(goto).toHaveBeenCalledWith(
        "https://x.com/intent/follow?screen_name=testuser",
      );
    });

    it("should return invalid_username on intent URL structure", async () => {
      const res = await follow("https://x.com/intent/like?tweet_id=123");
      expect(res.success).toBe(false);
      expect(res.reason).toBe("invalid_username");
    });

    it("should fail on invalid URL format completely", async () => {
      const res = await follow("just-random-text");
      expect(res.success).toBe(false);
      // It parses as a URL now due to http prepend strategy, but returns invalid_username
      expect(res.reason).toBe("invalid_username");
    });

    it("should catch unhandled errors and return success: false", async () => {
      goto.mockRejectedValueOnce(new Error("Follow failed"));
      const res = await follow("https://x.com/user");
      expect(res.success).toBe(false);
      expect(res.reason).toBe("unhandled_error");
    });

    it("should trigger timeout on slow follow", async () => {
      vi.useFakeTimers({ legacy: true });
      goto.mockImplementationOnce(
        () => new Promise((resolve) => setTimeout(resolve, 30000)),
      );
      const promise = follow("https://x.com/user");
      await vi.advanceTimersByTimeAsync(21000);
      const res = await promise;
      expect(res.success).toBe(false);
      expect(res.reason).toBe("timeout");
      vi.useRealTimers();
    });
  });
});
