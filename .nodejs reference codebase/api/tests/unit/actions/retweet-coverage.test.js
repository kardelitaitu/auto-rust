/**
 * Unit tests for api/actions/retweet.js
 * Tests the retweet functionality
 */

import { describe, it, expect, vi, beforeEach } from "vitest";

// Mock all dependencies - use correct relative paths from test file
vi.mock("@api/core/context.js", () => ({
  getPage: vi.fn(),
}));

vi.mock("@api/core/logger.js", () => ({
  createLogger: vi.fn(() => ({
    info: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
  })),
}));

vi.mock("@api/utils/math.js", () => ({
  mathUtils: {
    randomInRange: vi.fn((min, max) => Math.floor((min + max) / 2)),
  },
}));

vi.mock("@api/interactions/wait.js", () => ({
  wait: vi.fn().mockResolvedValue(undefined),
}));

vi.mock("@api/interactions/queries.js", () => ({
  visible: vi.fn().mockResolvedValue(false),
}));

vi.mock("@api/interactions/actions.js", () => ({
  click: vi.fn().mockResolvedValue(undefined),
}));

vi.mock("@api/utils/metrics.js", () => ({
  default: {
    recordSocialAction: vi.fn(),
  },
}));

import { retweetWithAPI } from "@api/actions/retweet.js";
import { getPage } from "@api/core/context.js";
import { mathUtils } from "@api/utils/math.js";
import { wait } from "@api/interactions/wait.js";
import { visible } from "@api/interactions/queries.js";
import { click } from "@api/interactions/actions.js";
import metricsCollector from "@api/utils/metrics.js";

describe("retweet.js", () => {
  let mockPage;

  beforeEach(() => {
    vi.clearAllMocks();

    mockPage = {
      keyboard: {
        press: vi.fn().mockResolvedValue(undefined),
      },
    };

    getPage.mockReturnValue(mockPage);
    mathUtils.randomInRange.mockImplementation((min, max) =>
      Math.floor((min + max) / 2),
    );
  });

  describe("retweetWithAPI", () => {
    it("should return success when already retweeted", async () => {
      visible.mockImplementation((sel) => {
        if (sel === '[data-testid="unretweet"]') return Promise.resolve(true);
        return Promise.resolve(false);
      });

      const result = await retweetWithAPI();

      expect(result.success).toBe(true);
      expect(result.reason).toBe("already_retweeted");
    });

    it("should return success when already retweeted (with tweetElement)", async () => {
      const mockTweetElement = {
        locator: vi.fn().mockReturnValue({
          first: vi.fn().mockReturnValue({
            isVisible: vi.fn().mockResolvedValue(true),
          }),
        }),
        scrollIntoViewIfNeeded: vi.fn().mockResolvedValue(undefined),
      };

      const result = await retweetWithAPI({ tweetElement: mockTweetElement });

      expect(result.success).toBe(true);
      expect(result.reason).toBe("already_retweeted");
    });

    it("should scroll tweet into view when provided", async () => {
      visible.mockResolvedValue(false);
      const mockTweetElement = {
        locator: vi.fn().mockReturnValue({
          first: vi.fn().mockReturnValue({
            isVisible: vi.fn().mockResolvedValue(false),
          }),
        }),
        scrollIntoViewIfNeeded: vi.fn().mockResolvedValue(undefined),
      };

      await retweetWithAPI({ tweetElement: mockTweetElement });

      expect(mockTweetElement.scrollIntoViewIfNeeded).toHaveBeenCalled();
    });

    it("should click retweet button", async () => {
      visible
        .mockResolvedValueOnce(false) // not already retweeted
        .mockResolvedValueOnce(true) // confirm menu visible
        .mockResolvedValueOnce(true); // verification successful

      await retweetWithAPI();

      expect(click).toHaveBeenCalledWith('[data-testid="retweet"]');
    });

    it("should return error when confirm menu not found", async () => {
      visible
        .mockResolvedValueOnce(false) // not already retweeted
        .mockResolvedValueOnce(false); // confirm menu not visible

      const result = await retweetWithAPI();

      expect(result.success).toBe(false);
      expect(result.reason).toBe("confirm_menu_not_found");
    });

    it("should press Escape when confirm menu not found", async () => {
      visible
        .mockResolvedValueOnce(false) // not already retweeted
        .mockResolvedValueOnce(false); // confirm menu not visible

      await retweetWithAPI();

      expect(mockPage.keyboard.press).toHaveBeenCalledWith("Escape");
    });

    it("should click confirm retweet button", async () => {
      visible
        .mockResolvedValueOnce(false) // not already retweeted
        .mockResolvedValueOnce(true) // confirm menu visible
        .mockResolvedValueOnce(true); // verification successful

      await retweetWithAPI();

      expect(click).toHaveBeenCalledWith('[data-testid="retweetConfirm"]');
    });

    it("should verify retweet was successful", async () => {
      visible
        .mockResolvedValueOnce(false) // not already retweeted
        .mockResolvedValueOnce(true) // confirm menu visible
        .mockResolvedValueOnce(true); // unretweet button visible (verification)

      const result = await retweetWithAPI();

      expect(result.success).toBe(true);
      expect(result.reason).toBe("success");
    });

    it("should return verification_failed when unretweet not visible after click", async () => {
      visible
        .mockResolvedValueOnce(false) // not already retweeted
        .mockResolvedValueOnce(true) // confirm menu visible
        .mockResolvedValueOnce(false); // verification failed

      const result = await retweetWithAPI();

      expect(result.success).toBe(false);
      expect(result.reason).toBe("verification_failed");
    });

    it("should record metrics on success", async () => {
      visible
        .mockResolvedValueOnce(false) // not already retweeted
        .mockResolvedValueOnce(true) // confirm menu visible
        .mockResolvedValueOnce(true); // verification successful

      await retweetWithAPI();

      expect(metricsCollector.recordSocialAction).toHaveBeenCalledWith(
        "retweet",
        1,
      );
    });

    it("should not record metrics on failure", async () => {
      visible
        .mockResolvedValueOnce(false) // not already retweeted
        .mockResolvedValueOnce(false); // confirm menu not visible

      await retweetWithAPI();

      expect(metricsCollector.recordSocialAction).not.toHaveBeenCalled();
    });

    it("should handle errors gracefully", async () => {
      click.mockRejectedValue(new Error("Click failed"));

      const result = await retweetWithAPI();

      expect(result.success).toBe(false);
      expect(result.reason).toBe("Click failed");
    });

    it("should use tweetElement locators when provided", async () => {
      visible.mockResolvedValue(false);
      const mockTweetElement = {
        locator: vi.fn().mockReturnValue({
          first: vi.fn().mockReturnValue({
            isVisible: vi
              .fn()
              .mockResolvedValueOnce(false) // already retweeted check
              .mockResolvedValueOnce(true), // verification check
          }),
        }),
        scrollIntoViewIfNeeded: vi.fn().mockResolvedValue(undefined),
      };

      await retweetWithAPI({ tweetElement: mockTweetElement });

      // Should have used tweetElement.locator for selectors
      expect(mockTweetElement.locator).toHaveBeenCalled();
    });

    it("should wait between actions", async () => {
      visible
        .mockResolvedValueOnce(false) // not already retweeted
        .mockResolvedValueOnce(true) // confirm menu visible
        .mockResolvedValueOnce(true); // verification successful

      await retweetWithAPI();

      // Should have multiple wait calls for timing
      expect(wait).toHaveBeenCalled();
    });

    it("should handle scrollIntoViewIfNeeded failure", async () => {
      visible.mockResolvedValue(false);
      const mockTweetElement = {
        locator: vi.fn().mockReturnValue({
          first: vi.fn().mockReturnValue({
            isVisible: vi
              .fn()
              .mockResolvedValueOnce(false)
              .mockResolvedValueOnce(true),
          }),
        }),
        scrollIntoViewIfNeeded: vi
          .fn()
          .mockRejectedValue(new Error("scroll failed")),
      };

      // Should not throw due to catch(() => {})
      const result = await retweetWithAPI({ tweetElement: mockTweetElement });

      // Flow continues despite scroll failure
      expect(click).toHaveBeenCalled();
    });
  });
});
