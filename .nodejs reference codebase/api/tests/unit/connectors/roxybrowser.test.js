/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import RoxybrowserDiscover from "@api/connectors/discovery/roxybrowser.js";

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
    get: vi.fn(),
  },
}));

describe("RoxybrowserDiscover", () => {
  let discover;
  let mockApiHandler;

  beforeEach(async () => {
    vi.clearAllMocks();
    const apiHandler = await import("@api/utils/apiHandler.js");
    mockApiHandler = apiHandler.default;
    discover = new RoxybrowserDiscover();
  });

  describe("Constructor", () => {
    it("should initialize with default API URL", () => {
      expect(discover.browserType).toBe("roxybrowser");
      expect(discover.apiBaseUrl).toBe("http://127.0.0.1:50000/");
    });

    it("should add trailing slash to API URL if missing", async () => {
      const { getEnv } = await import("@api/utils/envLoader.js");
      getEnv.mockReturnValueOnce("http://custom:51111");

      const customDiscover = new RoxybrowserDiscover();
      expect(customDiscover.apiBaseUrl).toBe("http://custom:51111/");
    });

    it("should use custom API URL from environment", async () => {
      const { getEnv } = await import("@api/utils/envLoader.js");
      getEnv.mockImplementation((key, defaultValue) => {
        if (key === "ROXYBROWSER_API_URL") return "http://custom:53333/";
        if (key === "ROXYBROWSER_API_KEY") return "test-key";
        return defaultValue;
      });

      const customDiscover = new RoxybrowserDiscover();
      expect(customDiscover.apiBaseUrl).toBe("http://custom:53333/");
    });
  });

  describe("discover()", () => {
    it("should return empty array when API base URL is not configured", async () => {
      discover.apiBaseUrl = "";
      const result = await discover.discover();
      expect(result).toEqual([]);
    });

    it("should return empty array when API key is not configured", async () => {
      discover.API_KEY = "";
      const result = await discover.discover();
      expect(result).toEqual([]);
    });

    it("should return empty array when API returns non-zero code", async () => {
      mockApiHandler.get.mockResolvedValue({
        code: 1,
        msg: "Error",
        data: [],
      });

      const result = await discover.discover();
      expect(result).toEqual([]);
    });

    it("should return empty array when API returns null data", async () => {
      mockApiHandler.get.mockResolvedValue({
        code: 0,
        msg: "Success",
        data: null,
      });

      const result = await discover.discover();
      expect(result).toEqual([]);
    });

    it("should return empty array when API returns non-array data", async () => {
      mockApiHandler.get.mockResolvedValue({
        code: 0,
        msg: "Success",
        data: { some: "object" },
      });

      const result = await discover.discover();
      expect(result).toEqual([]);
    });

    it("should return empty array when API returns empty array", async () => {
      mockApiHandler.get.mockResolvedValue({
        code: 0,
        msg: "Success",
        data: [],
      });

      const result = await discover.discover();
      expect(result).toEqual([]);
    });

    it("should return empty array when API throws error", async () => {
      mockApiHandler.get.mockRejectedValue(new Error("Network error"));

      const result = await discover.discover();
      expect(result).toEqual([]);
    });

    it("should return profiles when API returns valid data", async () => {
      mockApiHandler.get.mockResolvedValue({
        code: 0,
        msg: "Success",
        data: [
          {
            id: "profile-123",
            name: "Test Profile",
            ws: "ws://127.0.0.1:9222/devtools/browser",
            http: "http://127.0.0.1:9222/json",
            port: 9222,
            sortNum: 1,
            windowName: "Test Window",
            userAgent: "Mozilla/5.0",
            browserVersion: "120.0",
          },
        ],
      });

      const result = await discover.discover();

      expect(result).toHaveLength(1);
      expect(result[0]).toMatchObject({
        id: "profile-123",
        name: "Test Profile",
        type: "roxybrowser",
        ws: "ws://127.0.0.1:9222/devtools/browser",
        http: "http://127.0.0.1:9222/json",
        windowName: "Test Window",
        sortNum: 1,
        port: 9222,
        userAgent: "Mozilla/5.0",
        browserVersion: "120.0",
      });
    });

    it("should use sortNum for id when id is missing", async () => {
      mockApiHandler.get.mockResolvedValue({
        code: 0,
        msg: "Success",
        data: [
          {
            ws: "ws://127.0.0.1:9222/devtools/browser",
            http: "http://127.0.0.1:9222/json",
            sortNum: 42,
          },
        ],
      });

      const result = await discover.discover();

      expect(result).toHaveLength(1);
      expect(result[0].id).toBe("roxybrowser-42");
    });

    it("should use windowName for name when name is missing", async () => {
      mockApiHandler.get.mockResolvedValue({
        code: 0,
        msg: "Success",
        data: [
          {
            ws: "ws://127.0.0.1:9222/devtools/browser",
            http: "http://127.0.0.1:9222/json",
            sortNum: 1,
            windowName: "My Window",
          },
        ],
      });

      const result = await discover.discover();

      expect(result[0].name).toBe("My Window");
    });

    it("should use index when sortNum is missing", async () => {
      mockApiHandler.get.mockResolvedValue({
        code: 0,
        msg: "Success",
        data: [
          { ws: "ws://1:1/devtools", http: "http://1:1/json" },
          { ws: "ws://2:2/devtools", http: "http://2:2/json" },
        ],
      });

      const result = await discover.discover();

      expect(result[0].sortNum).toBe(0);
      expect(result[1].sortNum).toBe(1);
    });

    it("should skip profiles without ws and http", async () => {
      mockApiHandler.get.mockResolvedValue({
        code: 0,
        msg: "Success",
        data: [
          {
            id: "1",
            ws: "ws://127.0.0.1:9222/devtools/browser",
            http: "http://127.0.0.1:9222/json",
          },
          {
            id: "2",
            ws: null,
            http: null,
          },
        ],
      });

      const result = await discover.discover();

      expect(result).toHaveLength(1);
      expect(result[0].id).toBe("1");
    });

    it("should handle multiple profiles", async () => {
      mockApiHandler.get.mockResolvedValue({
        code: 0,
        msg: "Success",
        data: [
          {
            id: "1",
            ws: "ws://1:1/devtools",
            http: "http://1:1/json",
            sortNum: 1,
          },
          {
            id: "2",
            ws: "ws://2:2/devtools",
            http: "http://2:2/json",
            sortNum: 2,
          },
          {
            id: "3",
            ws: "ws://3:3/devtools",
            http: "http://3:3/json",
            sortNum: 3,
          },
        ],
      });

      const result = await discover.discover();

      expect(result).toHaveLength(3);
      expect(result.map((p) => p.id)).toEqual(["1", "2", "3"]);
    });
  });
});
