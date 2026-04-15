/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from "vitest";

vi.mock("@api/core/logger.js", () => ({
  createLogger: () => ({
    info: vi.fn(),
    debug: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
  }),
}));

describe("api/utils/retry.js", () => {
  let withRetry;

  beforeEach(async () => {
    vi.clearAllMocks();
    const module = await import("@api/utils/retry.js");
    withRetry = module.withRetry || module.default;
  });

  describe("withRetry", () => {
    it("should be defined", () => {
      expect(withRetry).toBeDefined();
    });

    it("should execute function successfully", async () => {
      const fn = vi.fn().mockResolvedValue("success");
      const result = await withRetry(fn);
      expect(result).toBe("success");
      expect(fn).toHaveBeenCalledTimes(1);
    });

    it("should retry on failure", async () => {
      const fn = vi
        .fn()
        .mockRejectedValueOnce(new Error("fail"))
        .mockResolvedValueOnce("success");
      const result = await withRetry(fn, { maxRetries: 2 });
      expect(result).toBe("success");
      expect(fn).toHaveBeenCalledTimes(2);
    });

    it("should throw after max retries", async () => {
      const fn = vi.fn().mockRejectedValue(new Error("always fails"));
      await expect(withRetry(fn, { maxRetries: 2 })).rejects.toThrow(
        "always fails",
      );
      expect(fn).toHaveBeenCalledTimes(2);
    });

    it("should accept custom delay", async () => {
      const fn = vi.fn().mockResolvedValue("success");
      await withRetry(fn, { delay: 10 });
      expect(fn).toHaveBeenCalledTimes(1);
    });
  });
});
