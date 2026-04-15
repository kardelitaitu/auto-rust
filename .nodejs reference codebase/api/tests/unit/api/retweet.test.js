/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { retweetWithAPI } from "@api/actions/retweet.js";

// Mock dependencies
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
    randomInRange: vi.fn((min, max) => (min + max) / 2),
  },
}));

vi.mock("@api/interactions/wait.js", () => ({
  wait: vi.fn().mockResolvedValue(undefined),
}));

vi.mock("@api/interactions/queries.js", () => ({
  visible: vi.fn(),
}));

vi.mock("@api/interactions/actions.js", () => ({
  click: vi.fn().mockResolvedValue(undefined),
}));

vi.mock("@api/utils/metrics.js", () => ({
  default: {
    recordSocialAction: vi.fn(),
  },
}));

import { getPage } from "@api/core/context.js";
import { visible } from "@api/interactions/queries.js";
import { click } from "@api/interactions/actions.js";
import metricsCollector from "@api/utils/metrics.js";

describe("api/actions/retweet.js", () => {
  let mockPage;

  beforeEach(() => {
    vi.clearAllMocks();

    mockPage = {
      url: vi.fn().mockReturnValue("https://x.com/home"),
      keyboard: {
        press: vi.fn().mockResolvedValue(undefined),
      },
    };

    getPage.mockReturnValue(mockPage);
  });

  describe("retweetWithAPI", () => {
    it("should retweet successfully", async () => {
      visible.mockResolvedValueOnce(false); // not already retweeted
      visible.mockResolvedValueOnce(true); // confirm menu visible
      visible.mockResolvedValueOnce(true); // now unretweetable

      const result = await retweetWithAPI();

      expect(click).toHaveBeenCalledTimes(2);
      expect(result.success).toBe(true);
      expect(result.method).toBe("retweetAPI");
      expect(metricsCollector.recordSocialAction).toHaveBeenCalledWith(
        "retweet",
        1,
      );
    });

    it("should skip if already retweeted", async () => {
      visible.mockResolvedValueOnce(true); // already retweeted

      const result = await retweetWithAPI();

      expect(click).not.toHaveBeenCalled();
      expect(result.success).toBe(true);
      expect(result.reason).toBe("already_retweeted");
    });

    it("should return failure if confirm menu not found", async () => {
      visible.mockResolvedValueOnce(false); // not already retweeted
      visible.mockResolvedValueOnce(false); // confirm menu not found

      const result = await retweetWithAPI();

      expect(mockPage.keyboard.press).toHaveBeenCalledWith("Escape");
      expect(result.success).toBe(false);
      expect(result.reason).toBe("confirm_menu_not_found");
    });

    it("should return failure if verification fails", async () => {
      visible.mockResolvedValueOnce(false); // not already retweeted
      visible.mockResolvedValueOnce(true); // confirm menu visible
      visible.mockResolvedValueOnce(false); // verification fails

      const result = await retweetWithAPI();

      expect(result.success).toBe(false);
      expect(result.reason).toBe("verification_failed");
    });

    it("should handle click error", async () => {
      visible.mockResolvedValueOnce(false);
      click.mockRejectedValueOnce(new Error("Click failed"));

      const result = await retweetWithAPI();

      expect(result.success).toBe(false);
      expect(result.reason).toContain("Click failed");
    });

    it("should use tweetElement when provided", async () => {
      const tweetElement = {
        scrollIntoViewIfNeeded: vi.fn().mockResolvedValue(undefined),
        locator: vi.fn().mockReturnValue({
          first: vi.fn().mockReturnValue({
            isVisible: vi
              .fn()
              .mockResolvedValueOnce(false)
              .mockResolvedValueOnce(true),
          }),
        }),
      };

      visible.mockResolvedValueOnce(true); // confirm menu visible
      visible.mockResolvedValueOnce(true); // verification succeeds

      const result = await retweetWithAPI({ tweetElement });

      expect(tweetElement.scrollIntoViewIfNeeded).toHaveBeenCalled();
      expect(tweetElement.locator).toHaveBeenCalledWith(
        '[data-testid="retweet"]',
      );
      expect(result.success).toBe(true);
    });
  });
});
