/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from "vitest";

vi.mock("@api/core/context.js", () => ({
  getPage: vi.fn(),
}));

vi.mock("@api/core/logger.js", () => ({
  createLogger: () => ({
    info: vi.fn(),
    debug: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
  }),
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
  visible: vi.fn().mockResolvedValue(false),
}));

vi.mock("@api/interactions/actions.js", () => ({
  click: vi.fn().mockResolvedValue(undefined),
}));

vi.mock("@api/utils/metrics.js", () => ({
  default: {
    recordTwitterEngagement: vi.fn(),
  },
}));

describe("api/actions/follow.js", () => {
  let follow;
  let mockPage;

  beforeEach(async () => {
    vi.clearAllMocks();

    mockPage = {
      locator: vi.fn().mockReturnValue({
        first: vi.fn().mockReturnValue({
          textContent: vi.fn().mockResolvedValue("Follow"),
          click: vi.fn().mockResolvedValue(undefined),
        }),
        count: vi.fn().mockResolvedValue(1),
      }),
      url: vi.fn().mockReturnValue("https://x.com/testuser"),
    };

    const { getPage } = await import("@api/core/context.js");
    getPage.mockReturnValue(mockPage);

    follow = await import("@api/actions/follow.js");
  });

  describe("followWithAPI()", () => {
    it("should follow a user", async () => {
      const result = await follow.followWithAPI({ username: "testuser" });
      expect(result).toHaveProperty("success");
    });

    it("should detect already following", async () => {
      const { visible } = await import("@api/interactions/queries.js");
      visible.mockResolvedValue(true);

      const result = await follow.followWithAPI({ username: "testuser" });
      expect(result.reason).toBe("already_following");
    });

    it("should accept options", async () => {
      const result = await follow.followWithAPI({
        username: "testuser",
        maxAttempts: 3,
      });
      expect(result).toHaveProperty("success");
    });

    it("should handle missing username", async () => {
      const result = await follow.followWithAPI();
      expect(result).toHaveProperty("success");
    });
  });

  describe("module exports", () => {
    it("should export followWithAPI", () => {
      expect(typeof follow.followWithAPI).toBe("function");
    });
  });
});
