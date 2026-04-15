/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from "vitest";

// Hoist mocks to ensure they are available in vi.mock factory
const mocks = vi.hoisted(() => ({
  HttpsProxyAgent: vi.fn().mockImplementation(function (url) {
    this.url = url;
  }),
  logger: {
    debug: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
  },
  fetch: vi.fn(),
}));

// Mock logger
vi.mock("@api/core/logger.js", () => ({
  createLogger: vi.fn(() => mocks.logger),
}));

// Mock https-proxy-agent for both static and dynamic imports
vi.mock("https-proxy-agent", () => ({
  HttpsProxyAgent: mocks.HttpsProxyAgent,
}));

// Mock global fetch
global.fetch = mocks.fetch;

describe("proxy-agent.js", () => {
  let ProxyAgent;
  let createProxyAgent;

  beforeEach(async () => {
    vi.clearAllMocks();
    vi.resetModules();
    mocks.fetch.mockReset();
    mocks.fetch.mockResolvedValue({ ok: true });
    const module = await import("../../utils/proxy-agent.js");
    ProxyAgent = module.default;
    createProxyAgent = module.createProxyAgent;
  });

  describe("createProxyAgent", () => {
    it("should create a ProxyAgent instance with valid URL", () => {
      const agent = createProxyAgent("http://proxy.example.com:8080");
      expect(agent).toBeDefined();
      expect(agent.proxyUrl).toBe("http://proxy.example.com:8080");
      expect(agent.host).toBe("proxy.example.com");
      expect(agent.port).toBe("8080");
    });

    it("should handle empty proxyUrl", () => {
      const agent = createProxyAgent("");
      expect(agent).toBeDefined();
      expect(agent.host).toBeUndefined();
    });

    it("should handle null proxyUrl", () => {
      const agent = createProxyAgent(null);
      expect(agent).toBeDefined();
      expect(agent.host).toBeUndefined();
    });

    it("should parse proxy URL with credentials", () => {
      const agent = createProxyAgent("http://user:pass@proxy.example.com:8080");
      expect(agent.host).toBe("proxy.example.com");
      expect(agent.port).toBe("8080");
      expect(agent.username).toBe("user");
      expect(agent.password).toBe("pass");
    });

    it("should handle proxy URL without credentials", () => {
      const agent = createProxyAgent("http://proxy.example.com:8080");
      expect(agent.username).toBeNull();
      expect(agent.password).toBeNull();
    });

    it("should handle proxy URL without port", () => {
      const agent = createProxyAgent("http://proxy.example.com");
      expect(agent.host).toBe("proxy.example.com");
      expect(agent.port).toBe("");
    });

    it("should decode URL-encoded credentials", () => {
      const agent = createProxyAgent(
        "http://user%40domain:pass%21@proxy.example.com:8080",
      );
      expect(agent.username).toBe("user@domain");
      expect(agent.password).toBe("pass!");
    });

    it("should return cached agent if exists", async () => {
      const agent = createProxyAgent("http://proxy.example.com:8080");
      agent.agent = { test: "agent" };
      const result = await agent.getAgent();
      expect(result).toEqual({ test: "agent" });
    });

    it("should handle undefined proxyUrl in constructor", () => {
      const agent = createProxyAgent(undefined);
      expect(agent).toBeDefined();
      expect(agent.host).toBeUndefined();
    });
  });

  describe("getAgent", () => {
    it("should create HttpsProxyAgent with auth", async () => {
      const proxyUrl = "http://user:pass@proxy.example.com:8080";
      const agent = new ProxyAgent(proxyUrl);
      const httpAgent = await agent.getAgent();

      expect(mocks.HttpsProxyAgent).toHaveBeenCalledWith(
        "http://user:pass@proxy.example.com:8080",
      );
      expect(httpAgent).toBeDefined();
      expect(agent.agent).toBe(httpAgent);
    });

    it("should create HttpsProxyAgent without auth", async () => {
      const proxyUrl = "http://proxy.example.com:8080";
      const agent = new ProxyAgent(proxyUrl);
      const httpAgent = await agent.getAgent();

      // Verify agent was created successfully
      expect(httpAgent).toBeDefined();
      expect(httpAgent.url).toBe("http://proxy.example.com:8080");
    });

    it("should return null and log warning if HttpsProxyAgent creation fails", async () => {
      mocks.HttpsProxyAgent.mockImplementationOnce(function () {
        throw new Error("Failed to create agent");
      });

      const agent = new ProxyAgent("http://proxy.example.com:8080");
      const httpAgent = await agent.getAgent();

      expect(httpAgent).toBeNull();
      expect(mocks.logger.warn).toHaveBeenCalledWith(
        expect.stringContaining("Failed to create proxy agent"),
      );
    });
  });

  describe("fetchWithProxy", () => {
    it("should use fetch without agent if no proxyUrl provided", async () => {
      await ProxyAgent.fetchWithProxy(
        "https://example.com",
        { method: "GET" },
        null,
      );

      expect(mocks.fetch).toHaveBeenCalledWith("https://example.com", {
        method: "GET",
      });
      expect(mocks.HttpsProxyAgent).not.toHaveBeenCalled();
    });

    it("should use fetch with agent if proxyUrl provided", async () => {
      const proxyUrl = "http://proxy.example.com:8080";
      const agent = new ProxyAgent(proxyUrl);

      // Ensure getAgent succeeds
      const httpAgent = await agent.getAgent();
      expect(httpAgent).toBeDefined();

      await ProxyAgent.fetchWithProxy(
        "https://example.com",
        { method: "GET" },
        proxyUrl,
      );

      // Verify fetch was called with agent
      expect(mocks.fetch).toHaveBeenCalledWith(
        "https://example.com",
        expect.objectContaining({
          method: "GET",
          agent: expect.anything(),
        }),
      );
    });

    it("should fall back to direct connection if agent creation fails", async () => {
      mocks.HttpsProxyAgent.mockImplementationOnce(function () {
        throw new Error("Failed");
      });

      const proxyUrl = "http://proxy.example.com:8080";
      await ProxyAgent.fetchWithProxy(
        "https://example.com",
        { method: "GET" },
        proxyUrl,
      );

      expect(mocks.fetch).toHaveBeenCalledWith("https://example.com", {
        method: "GET",
      });
      // In this case, getAgent returns null via catch, so logger.warn('Failed to create...') is called.
      // Then fetchWithProxy sees null agent, and calls logger.warn('Falling back...')

      // Let's verify 'Falling back...' is called, which implies direct connection fallback
      expect(mocks.logger.warn).toHaveBeenCalledWith(
        expect.stringContaining("Falling back to direct connection"),
      );
    });
  });
});
