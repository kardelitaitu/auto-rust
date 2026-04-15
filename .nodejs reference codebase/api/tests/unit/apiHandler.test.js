/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Unit tests for utils/apiHandler.js
 * Tests HTTP request handling with retry logic
 * @module tests/unit/apiHandler.test
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";

// Mock the retry module
vi.mock("@api/utils/retry.js", () => ({
  withRetry: vi.fn(async (operation, _options) => operation()),
}));

describe("utils/apiHandler", () => {
  let ApiHandler;
  let apiHandler;
  let mockFetch;

  beforeEach(async () => {
    vi.clearAllMocks();

    // Create mock fetch
    mockFetch = vi.fn();
    vi.stubGlobal("fetch", mockFetch);

    const module = await import("../../utils/apiHandler.js");
    ApiHandler = module.ApiHandler;
    apiHandler = module.default;
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  describe("ApiHandler Class", () => {
    it("should instantiate with default values", () => {
      const handler = new ApiHandler();

      expect(handler.baseUrl).toBe("");
      expect(handler.defaultHeaders).toBeDefined();
      expect(handler.defaultHeaders["Content-Type"]).toBe("application/json");
    });

    it("should accept custom baseUrl", () => {
      const handler = new ApiHandler("https://api.example.com");

      expect(handler.baseUrl).toBe("https://api.example.com");
    });

    it("should accept custom defaultHeaders", () => {
      const handler = new ApiHandler("", { Authorization: "Bearer token" });

      expect(handler.defaultHeaders["Authorization"]).toBe("Bearer token");
      expect(handler.defaultHeaders["Content-Type"]).toBe("application/json");
    });

    it("should merge custom headers with defaults", () => {
      const handler = new ApiHandler("", { "X-Custom": "value" });

      expect(handler.defaultHeaders["Content-Type"]).toBe("application/json");
      expect(handler.defaultHeaders["X-Custom"]).toBe("value");
    });
  });

  describe("request", () => {
    it("should make GET request by default", async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        status: 200,
        headers: { get: () => "application/json" },
        json: () => Promise.resolve({ data: "test" }),
      });

      const result = await apiHandler.request("/endpoint");

      expect(mockFetch).toHaveBeenCalledWith(
        "/endpoint",
        expect.objectContaining({ method: "GET" }),
      );
      expect(result).toEqual({ data: "test" });
    });

    it("should prepend baseUrl to relative endpoints", async () => {
      const handler = new ApiHandler("https://api.example.com");

      mockFetch.mockResolvedValueOnce({
        ok: true,
        headers: { get: () => "application/json" },
        json: () => Promise.resolve({}),
      });

      await handler.request("/users");

      expect(mockFetch).toHaveBeenCalledWith(
        "https://api.example.com/users",
        expect.any(Object),
      );
    });

    it("should use absolute URL when endpoint starts with http", async () => {
      const handler = new ApiHandler("https://api.example.com");

      mockFetch.mockResolvedValueOnce({
        ok: true,
        headers: { get: () => "application/json" },
        json: () => Promise.resolve({}),
      });

      await handler.request("https://other.com/endpoint");

      expect(mockFetch).toHaveBeenCalledWith(
        "https://other.com/endpoint",
        expect.any(Object),
      );
    });

    it("should include default headers in request", async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        headers: { get: () => "application/json" },
        json: () => Promise.resolve({}),
      });

      await apiHandler.request("/endpoint");

      expect(mockFetch).toHaveBeenCalledWith(
        expect.any(String),
        expect.objectContaining({
          headers: expect.objectContaining({
            "Content-Type": "application/json",
          }),
        }),
      );
    });

    it("should allow custom headers to override defaults", async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        headers: { get: () => "application/json" },
        json: () => Promise.resolve({}),
      });

      await apiHandler.request("/endpoint", {
        headers: { "Content-Type": "application/xml" },
      });

      expect(mockFetch).toHaveBeenCalledWith(
        expect.any(String),
        expect.objectContaining({
          headers: expect.objectContaining({
            "Content-Type": "application/xml",
          }),
        }),
      );
    });

    it("should throw on HTTP error status", async () => {
      mockFetch.mockResolvedValueOnce({
        ok: false,
        status: 404,
        statusText: "Not Found",
        headers: { get: () => "application/json" },
        json: () => Promise.resolve({}),
      });

      await expect(apiHandler.request("/missing")).rejects.toThrow(
        "HTTP 404: Not Found",
      );
    });

    it("should throw on HTTP 500 error", async () => {
      mockFetch.mockResolvedValueOnce({
        ok: false,
        status: 500,
        statusText: "Internal Server Error",
        headers: { get: () => "text/plain" },
        text: () => Promise.resolve("error"),
      });

      await expect(apiHandler.request("/boom")).rejects.toThrow(
        "HTTP 500: Internal Server Error",
      );
    });

    it("should parse JSON response for application/json content-type", async () => {
      const jsonData = { message: "success", count: 42 };

      mockFetch.mockResolvedValueOnce({
        ok: true,
        status: 200,
        headers: { get: () => "application/json" },
        json: () => Promise.resolve(jsonData),
        text: vi.fn(), // Should not be called
      });

      const result = await apiHandler.request("/json");

      expect(result).toEqual(jsonData);
    });

    it("should parse text response for non-JSON content-type", async () => {
      const textData = "plain text response";

      mockFetch.mockResolvedValueOnce({
        ok: true,
        status: 200,
        headers: { get: () => "text/plain" },
        text: () => Promise.resolve(textData),
      });

      const result = await apiHandler.request("/text");

      expect(result).toBe(textData);
    });

    it("should support custom method", async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        headers: { get: () => "application/json" },
        json: () => Promise.resolve({}),
      });

      await apiHandler.request("/endpoint", { method: "PATCH" });

      expect(mockFetch).toHaveBeenCalledWith(
        expect.any(String),
        expect.objectContaining({ method: "PATCH" }),
      );
    });

    it("should support request body", async () => {
      const postData = { name: "test", value: 123 };

      mockFetch.mockResolvedValueOnce({
        ok: true,
        headers: { get: () => "application/json" },
        json: () => Promise.resolve({}),
      });

      await apiHandler.request("/endpoint", {
        method: "POST",
        body: JSON.stringify(postData),
      });

      expect(mockFetch).toHaveBeenCalledWith(
        expect.any(String),
        expect.objectContaining({
          body: JSON.stringify(postData),
        }),
      );
    });

    it("should use retry logic via withRetry", async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        headers: { get: () => "application/json" },
        json: () => Promise.resolve({ ok: true }),
      });

      await apiHandler.request("/retry-check");

      const { withRetry } = await import("@api/utils/retry.js");
      expect(withRetry).toHaveBeenCalledWith(expect.any(Function), {
        description: "API request to /retry-check",
      });
    });
  });

  describe("get", () => {
    it("should make GET request", async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        headers: { get: () => "application/json" },
        json: () => Promise.resolve({ data: "test" }),
      });

      const result = await apiHandler.get("/users");

      expect(mockFetch).toHaveBeenCalledWith(
        expect.any(String),
        expect.objectContaining({ method: "GET" }),
      );
      expect(result).toEqual({ data: "test" });
    });

    it("should pass options to request", async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        headers: { get: () => "application/json" },
        json: () => Promise.resolve({}),
      });

      await apiHandler.get("/users", { headers: { "X-Custom": "value" } });

      expect(mockFetch).toHaveBeenCalledWith(
        expect.any(String),
        expect.objectContaining({
          headers: expect.objectContaining({ "X-Custom": "value" }),
        }),
      );
    });
  });

  describe("post", () => {
    it("should make POST request with JSON body", async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        headers: { get: () => "application/json" },
        json: () => Promise.resolve({ created: true }),
      });

      const data = { name: "new item" };
      const result = await apiHandler.post("/users", data);

      expect(mockFetch).toHaveBeenCalledWith(
        expect.any(String),
        expect.objectContaining({
          method: "POST",
          body: JSON.stringify(data),
        }),
      );
      expect(result).toEqual({ created: true });
    });

    it("should pass additional options to request", async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        headers: { get: () => "application/json" },
        json: () => Promise.resolve({}),
      });

      await apiHandler.post(
        "/users",
        { name: "test" },
        { headers: { "X-Custom": "value" } },
      );

      expect(mockFetch).toHaveBeenCalledWith(
        expect.any(String),
        expect.objectContaining({
          headers: expect.objectContaining({ "X-Custom": "value" }),
        }),
      );
    });
  });

  describe("put", () => {
    it("should make PUT request with JSON body", async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        headers: { get: () => "application/json" },
        json: () => Promise.resolve({ updated: true }),
      });

      const data = { name: "updated item" };
      const result = await apiHandler.put("/users/1", data);

      expect(mockFetch).toHaveBeenCalledWith(
        expect.any(String),
        expect.objectContaining({
          method: "PUT",
          body: JSON.stringify(data),
        }),
      );
      expect(result).toEqual({ updated: true });
    });
  });

  describe("delete", () => {
    it("should make DELETE request", async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        headers: { get: () => "application/json" },
        json: () => Promise.resolve({ deleted: true }),
      });

      const result = await apiHandler.delete("/users/1");

      expect(mockFetch).toHaveBeenCalledWith(
        expect.any(String),
        expect.objectContaining({ method: "DELETE" }),
      );
      expect(result).toEqual({ deleted: true });
    });

    it("should pass options to request", async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        headers: { get: () => "application/json" },
        json: () => Promise.resolve({}),
      });

      await apiHandler.delete("/users/1", { headers: { "X-Custom": "value" } });

      expect(mockFetch).toHaveBeenCalledWith(
        expect.any(String),
        expect.objectContaining({
          method: "DELETE",
          headers: expect.objectContaining({ "X-Custom": "value" }),
        }),
      );
    });
  });

  describe("Edge Cases", () => {
    it("should handle empty baseUrl", async () => {
      const handler = new ApiHandler("");

      mockFetch.mockResolvedValueOnce({
        ok: true,
        headers: { get: () => "application/json" },
        json: () => Promise.resolve({}),
      });

      await handler.request("/endpoint");

      expect(mockFetch).toHaveBeenCalledWith("/endpoint", expect.any(Object));
    });

    it("should handle null response body", async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        status: 204,
        headers: { get: () => null },
        json: () => Promise.resolve(null),
        text: () => Promise.resolve(""),
      });

      const result = await apiHandler.request("/empty");

      // Result can be null or empty string depending on implementation
      expect(result === null || result === "").toBe(true);
    });

    it("should handle missing content-type header", async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        headers: { get: () => null },
        text: () => Promise.resolve("text response"),
      });

      const result = await apiHandler.request("/no-content-type");

      expect(result).toBe("text response");
    });

    it("should handle endpoint with query parameters", async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        headers: { get: () => "application/json" },
        json: () => Promise.resolve({}),
      });

      await apiHandler.get("/search?query=test&limit=10");

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining("/search?query=test&limit=10"),
        expect.any(Object),
      );
    });
  });

  describe("Module Exports", () => {
    it("should export default ApiHandler instance", () => {
      expect(apiHandler).toBeDefined();
      expect(typeof apiHandler.request).toBe("function");
      expect(typeof apiHandler.get).toBe("function");
      expect(typeof apiHandler.post).toBe("function");
      expect(typeof apiHandler.put).toBe("function");
      expect(typeof apiHandler.delete).toBe("function");
    });

    it("should export ApiHandler class", () => {
      expect(ApiHandler).toBeDefined();
      expect(typeof ApiHandler).toBe("function");
    });
  });
});
