/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { followWithAPI } from "@api/actions/follow.js";

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

import { getPage } from "@api/core/context.js";
import { visible } from "@api/interactions/queries.js";
import { click } from "@api/interactions/actions.js";

describe("api/actions/follow.js", () => {
  let mockPage;

  beforeEach(() => {
    vi.clearAllMocks();

    mockPage = {
      url: vi.fn().mockReturnValue("https://x.com/user"),
      locator: vi.fn().mockReturnThis(),
      first: vi.fn().mockReturnThis(),
      textContent: vi.fn().mockResolvedValue("Follow"),
    };

    getPage.mockReturnValue(mockPage);
  });

  describe("followWithAPI", () => {
    it("should follow successfully", async () => {
      visible.mockResolvedValueOnce(false); // unfollowSel not visible
      visible.mockResolvedValueOnce(true); // followSel visible
      // after click, verification loop will see unfollowSel (now visible)
      visible.mockResolvedValue(true);

      const result = await followWithAPI({ username: "testuser" });

      expect(click).toHaveBeenCalled();
      expect(result.success).toBe(true);
      expect(result.method).toBe("followAPI");
    });

    it("should skip if already following (unfollow visible)", async () => {
      visible.mockResolvedValueOnce(true); // already following

      const result = await followWithAPI({ username: "testuser" });

      expect(click).not.toHaveBeenCalled();
      expect(result.success).toBe(true);
      expect(result.reason).toBe("already_following");
    });

    it("should skip if button shows Following", async () => {
      visible.mockResolvedValueOnce(false); // unfollowSel not visible
      visible.mockResolvedValueOnce(true); // followSel visible
      mockPage.textContent.mockResolvedValue("Following");

      const result = await followWithAPI({ username: "testuser" });

      expect(click).not.toHaveBeenCalled();
      expect(result.success).toBe(true);
      expect(result.reason).toBe("already_following");
    });

    it("should return failure if verification fails", async () => {
      // unfollowSel not visible, followSel visible
      visible.mockResolvedValueOnce(false);
      visible.mockResolvedValueOnce(true);
      // after click, verification loop will check unfollowSel (still false)
      visible.mockResolvedValue(false);
      // button text remains "Follow"
      mockPage.textContent.mockResolvedValue("Follow");

      const result = await followWithAPI({
        username: "testuser",
        maxAttempts: 1,
      });

      expect(result.success).toBe(false);
      expect(result.reason).toBe("verification_failed");
    });

    it("should handle click error", async () => {
      visible.mockResolvedValueOnce(false); // unfollowSel not visible
      visible.mockResolvedValueOnce(true); // followSel visible
      click.mockRejectedValueOnce(new Error("Click failed"));

      const result = await followWithAPI({ username: "testuser" });

      expect(result.success).toBe(false);
      expect(result.reason).toContain("Click failed");
    });
  });
});
