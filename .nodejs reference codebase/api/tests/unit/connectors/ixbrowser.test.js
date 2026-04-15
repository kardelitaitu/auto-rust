/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import IxbrowserDiscover from "@api/connectors/discovery/ixbrowser.js";

vi.mock("@api/core/logger.js", () => ({
  createLogger: vi.fn(() => ({
    info: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
  })),
}));

vi.mock("@api/utils/envLoader.js", () => ({
  getEnv: vi.fn((key, defaultValue) => defaultValue),
}));

vi.mock("@api/utils/apiHandler.js", () => ({
  default: {
    post: vi.fn(),
  },
}));

describe("IxbrowserDiscover", () => {
  let discover;
  let mockApiHandler;

  beforeEach(async () => {
    vi.clearAllMocks();
    const apiHandler = await import("@api/utils/apiHandler.js");
    mockApiHandler = apiHandler.default;
    discover = new IxbrowserDiscover();
  });

  describe("Constructor", () => {
    it("should initialize with default API URL", () => {
      expect(discover.browserType).toBe("ixbrowser");
      expect(discover.apiBaseUrl).toBe("http://127.0.0.1:53200");
    });

    it("should use custom API URL from environment", async () => {
      const { getEnv } = await import("@api/utils/envLoader.js");
      getEnv.mockReturnValueOnce("http://custom:53300");

      const customDiscover = new IxbrowserDiscover();
      expect(customDiscover.apiBaseUrl).toBe("http://custom:53300");
    });
  });

  describe("discover()", () => {
    it("should return empty array when API base URL is not configured", async () => {
      discover.apiBaseUrl = "";
      const result = await discover.discover();
      expect(result).toEqual([]);
    });

    it("should return empty array when API returns error code", async () => {
      mockApiHandler.post.mockResolvedValue({
        error: { code: 1, message: "API Error" },
        data: [],
      });

      const result = await discover.discover();
      expect(result).toEqual([]);
    });

    it("should return empty array when API returns empty data array", async () => {
      mockApiHandler.post.mockResolvedValue({
        error: { code: 0, message: "Success" },
        data: [],
      });

      const result = await discover.discover();
      expect(result).toEqual([]);
    });

    it("should return empty array when API returns empty data object", async () => {
      mockApiHandler.post.mockResolvedValue({
        error: { code: 0, message: "Success" },
        data: {},
      });

      const result = await discover.discover();
      expect(result).toEqual([]);
    });

    it("should return empty array when API throws error", async () => {
      mockApiHandler.post.mockRejectedValue(new Error("Network error"));

      const result = await discover.discover();
      expect(result).toEqual([]);
    });

    it("should return profiles when API returns valid data as array", async () => {
      mockApiHandler.post.mockResolvedValue({
        error: { code: 0, message: "Success" },
        data: [
          {
            profile_id: "123",
            ws: "ws://127.0.0.1:9222/devtools/browser",
            debugging_address: "127.0.0.1:9222",
            debugging_port: 9222,
            pid: 1234,
            open_time: 1234567890,
          },
        ],
      });

      const result = await discover.discover();

      expect(result).toHaveLength(1);
      expect(result[0]).toMatchObject({
        id: "ixbrowser-123",
        name: "ixBrowser Profile 123",
        type: "ixbrowser",
        ws: "ws://127.0.0.1:9222/devtools/browser",
        http: "http://127.0.0.1:9222/json",
        windowName: "ixBrowser-123",
        sortNum: "123",
        port: 9222,
        pid: 1234,
        openTime: 1234567890,
      });
    });

    it("should use debugging_port to construct ws URL when ws is missing", async () => {
      mockApiHandler.post.mockResolvedValue({
        error: { code: 0, message: "Success" },
        data: [
          {
            profile_id: "456",
            debugging_address: "127.0.0.1:9333",
            debugging_port: 9333,
            pid: 5678,
            open_time: 1234567890,
          },
        ],
      });

      const result = await discover.discover();

      expect(result).toHaveLength(1);
      expect(result[0].ws).toBe("ws://127.0.0.1:9333/devtools/browser");
      expect(result[0].port).toBe(9333);
    });

    it("should skip profiles without ws and debugging_port", async () => {
      mockApiHandler.post.mockResolvedValue({
        error: { code: 0, message: "Success" },
        data: [
          {
            profile_id: "123",
            ws: "ws://127.0.0.1:9222/devtools/browser",
            debugging_address: "127.0.0.1:9222",
            debugging_port: 9222,
          },
          {
            profile_id: "456",
            debugging_address: "127.0.0.1:9333",
          },
        ],
      });

      const result = await discover.discover();

      expect(result).toHaveLength(1);
      expect(result[0].id).toBe("ixbrowser-123");
    });

    it("should handle data as object with profile entries", async () => {
      mockApiHandler.post.mockResolvedValue({
        error: { code: 0, message: "Success" },
        data: {
          profile_1: {
            profile_id: "789",
            ws: "ws://127.0.0.1:9444/devtools/browser",
            debugging_address: "127.0.0.1:9444",
            debugging_port: 9444,
          },
        },
      });

      const result = await discover.discover();

      expect(result).toHaveLength(1);
      expect(result[0].id).toBe("ixbrowser-789");
    });

    it("should handle multiple profiles", async () => {
      mockApiHandler.post.mockResolvedValue({
        error: { code: 0, message: "Success" },
        data: [
          {
            profile_id: "1",
            ws: "ws://1:1/devtools",
            debugging_address: "1:1",
            debugging_port: 1,
          },
          {
            profile_id: "2",
            ws: "ws://2:2/devtools",
            debugging_address: "2:2",
            debugging_port: 2,
          },
          {
            profile_id: "3",
            ws: "ws://3:3/devtools",
            debugging_address: "3:3",
            debugging_port: 3,
          },
        ],
      });

      const result = await discover.discover();

      expect(result).toHaveLength(3);
      expect(result.map((p) => p.id)).toEqual([
        "ixbrowser-1",
        "ixbrowser-2",
        "ixbrowser-3",
      ]);
    });
  });
});
