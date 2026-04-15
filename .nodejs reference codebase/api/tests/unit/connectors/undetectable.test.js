/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import UndetectableDiscover from "@api/connectors/discovery/undetectable.js";

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

describe("UndetectableDiscover", () => {
  let discover;
  let mockApiHandler;

  beforeEach(async () => {
    vi.clearAllMocks();
    const apiHandler = await import("@api/utils/apiHandler.js");
    mockApiHandler = apiHandler.default;
    discover = new UndetectableDiscover();
  });

  describe("Constructor", () => {
    it("should initialize with default API URL", () => {
      expect(discover.browserType).toBe("undetectable");
      expect(discover.apiBaseUrl).toBe("http://127.0.0.1:25325/");
    });

    it("should add trailing slash to API URL if missing", async () => {
      const { getEnv } = await import("@api/utils/envLoader.js");
      getEnv.mockReturnValueOnce("http://custom:25444");

      const customDiscover = new UndetectableDiscover();
      expect(customDiscover.apiBaseUrl).toBe("http://custom:25444/");
    });

    it("should preserve trailing slash if already present", async () => {
      const { getEnv } = await import("@api/utils/envLoader.js");
      getEnv.mockReturnValueOnce("http://custom:25444/");

      const customDiscover = new UndetectableDiscover();
      expect(customDiscover.apiBaseUrl).toBe("http://custom:25444/");
    });
  });

  describe("discover()", () => {
    it("should return empty array when API base URL is not configured", async () => {
      discover.apiBaseUrl = "";
      const result = await discover.discover();
      expect(result).toEqual([]);
    });

    it("should return empty array when API returns non-zero code", async () => {
      mockApiHandler.get.mockResolvedValue({
        code: 1,
        msg: "Error",
        data: null,
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

    it("should return empty array when API returns empty data object", async () => {
      mockApiHandler.get.mockResolvedValue({
        code: 0,
        msg: "Success",
        data: {},
      });

      const result = await discover.discover();
      expect(result).toEqual([]);
    });

    it("should return empty array when no profiles have websocket_link", async () => {
      mockApiHandler.get.mockResolvedValue({
        code: 0,
        msg: "Success",
        data: {
          "profile-1": { name: "Profile 1", websocket_link: "" },
          "profile-2": { name: "Profile 2" },
        },
      });

      const result = await discover.discover();
      expect(result).toEqual([]);
    });

    it("should return empty array when API throws error", async () => {
      mockApiHandler.get.mockRejectedValue(new Error("Network error"));

      const result = await discover.discover();
      expect(result).toEqual([]);
    });

    it("should return profiles when API returns valid running profiles", async () => {
      mockApiHandler.get.mockResolvedValue({
        code: 0,
        msg: "Success",
        data: {
          "uuid-123": {
            name: "Test Profile",
            websocket_link: "ws://127.0.0.1:9222/devtools/browser/uuid",
            cloud_id: "cloud-1",
            cloud_group: "group-1",
          },
        },
      });

      const result = await discover.discover();

      expect(result).toHaveLength(1);
      expect(result[0]).toMatchObject({
        id: "uuid-123",
        name: "Test Profile",
        type: "undetectable",
        ws: "ws://127.0.0.1:9222/devtools/browser/uuid",
        http: "http://127.0.0.1:9222",
        windowName: "Test Profile",
        sortNum: 0,
        port: 9222,
        cloud_id: "cloud-1",
        cloud_group: "group-1",
      });
    });

    it("should use default name when profile name is missing", async () => {
      mockApiHandler.get.mockResolvedValue({
        code: 0,
        msg: "Success",
        data: {
          "uuid-abc-12345": {
            websocket_link: "ws://127.0.0.1:9222/devtools/browser/uuid",
          },
        },
      });

      const result = await discover.discover();

      expect(result).toHaveLength(1);
      expect(result[0].name).toBe("Undetectable-uuid-abc");
      expect(result[0].windowName).toBe("Undetectable-0");
    });

    it("should handle multiple profiles", async () => {
      mockApiHandler.get.mockResolvedValue({
        code: 0,
        msg: "Success",
        data: {
          "uuid-1": {
            name: "Profile 1",
            websocket_link: "ws://1.1.1.1:9001/devtools/browser/1",
          },
          "uuid-2": {
            name: "Profile 2",
            websocket_link: "ws://2.2.2.2:9002/devtools/browser/2",
          },
          "uuid-3": {
            name: "Profile 3",
            websocket_link: "ws://3.3.3.3:9003/devtools/browser/3",
          },
        },
      });

      const result = await discover.discover();

      expect(result).toHaveLength(3);
      expect(result.map((p) => p.id)).toEqual(["uuid-1", "uuid-2", "uuid-3"]);
      expect(result.map((p) => p.port)).toEqual([9001, 9002, 9003]);
    });

    it("should filter out profiles without websocket_link", async () => {
      mockApiHandler.get.mockResolvedValue({
        code: 0,
        msg: "Success",
        data: {
          "uuid-1": {
            name: "Profile 1",
            websocket_link: "ws://1.1.1.1:9001/devtools/browser/1",
          },
          "uuid-2": { name: "Profile 2", websocket_link: "" },
          "uuid-3": { name: "Profile 3", websocket_link: "   " },
          "uuid-4": { name: "Profile 4" },
        },
      });

      const result = await discover.discover();

      expect(result).toHaveLength(1);
      expect(result[0].id).toBe("uuid-1");
    });

    it("should use index for sortNum", async () => {
      mockApiHandler.get.mockResolvedValue({
        code: 0,
        msg: "Success",
        data: {
          "uuid-a": {
            name: "Profile A",
            websocket_link: "ws://1.1.1.1:9001/devtools/browser/a",
          },
          "uuid-b": {
            name: "Profile B",
            websocket_link: "ws://2.2.2.2:9002/devtools/browser/b",
          },
        },
      });

      const result = await discover.discover();

      expect(result[0].sortNum).toBe(0);
      expect(result[1].sortNum).toBe(1);
    });
  });
});
