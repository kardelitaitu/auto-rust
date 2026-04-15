/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import Automator from "@api/core/automator.js";
import { chromium } from "playwright";
import { getTimeoutValue } from "@api/utils/configLoader.js";
// import { withRetry } from '@api/utils/retry.js';

vi.mock("playwright", () => ({
  chromium: {
    connectOverCDP: vi.fn(),
    launch: vi.fn(),
  },
}));
vi.mock("@api/core/logger.js", () => ({
  createLogger: () => ({
    info: vi.fn(),
    error: vi.fn(),
    warn: vi.fn(),
    debug: vi.fn(),
  }),
}));
vi.mock("@api/utils/configLoader.js", () => ({
  getTimeoutValue: vi.fn(),
}));
vi.mock("@api/utils/retry.js", () => ({
  withRetry: vi.fn((fn) => fn()),
}));

describe("Automator", () => {
  let automator;

  beforeEach(() => {
    vi.clearAllMocks();
    vi.useFakeTimers();
    automator = new Automator();
  });

  afterEach(() => {
    automator.stopHealthChecks();
    vi.useRealTimers();
  });

  describe("constructor", () => {
    it("should initialize with empty connections", () => {
      expect(automator.connections).toBeInstanceOf(Map);
      expect(automator.connections.size).toBe(0);
      expect(automator.healthCheckInterval).toBeNull();
      expect(automator.isShuttingDown).toBe(false);
    });
  });

  describe("connectToBrowser", () => {
    it("should connect and test the browser", async () => {
      const mockBrowser = {
        on: vi.fn(),
        isConnected: vi.fn().mockReturnValue(true),
        close: vi.fn().mockResolvedValue(),
      };
      chromium.connectOverCDP.mockResolvedValue(mockBrowser);
      getTimeoutValue.mockResolvedValue(5000);

      const wsEndpoint = "ws://localhost:1234";
      const browser = await automator.connectToBrowser(wsEndpoint);

      expect(browser).toBe(mockBrowser);
      expect(chromium.connectOverCDP).toHaveBeenCalledWith(wsEndpoint, {
        timeout: 5000,
      });
      expect(automator.connections.has(wsEndpoint)).toBe(true);
    });

    it("should store connection info with correct metadata", async () => {
      const mockBrowser = {
        on: vi.fn(),
        isConnected: vi.fn().mockReturnValue(true),
      };
      chromium.connectOverCDP.mockResolvedValue(mockBrowser);
      getTimeoutValue.mockResolvedValue(5000);

      const wsEndpoint = "ws://localhost:1234";
      await automator.connectToBrowser(wsEndpoint);

      const connInfo = automator.connections.get(wsEndpoint);
      expect(connInfo.browser).toBe(mockBrowser);
      expect(connInfo.endpoint).toBe(wsEndpoint);
      expect(connInfo.connectedAt).toBeDefined();
      expect(connInfo.lastHealthCheck).toBeDefined();
      expect(connInfo.healthy).toBe(true);
      expect(connInfo.reconnectAttempts).toBe(0);
    });
  });

  describe("testConnection", () => {
    it("should return true when browser is connected", async () => {
      const mockBrowser = {
        on: vi.fn(),
        isConnected: vi.fn().mockReturnValue(true),
      };
      const result = await automator.testConnection(mockBrowser);
      expect(result).toBe(true);
    });

    it("should throw when browser is disconnected", async () => {
      const mockBrowser = {
        on: vi.fn(),
        isConnected: vi.fn().mockReturnValue(false),
      };
      await expect(automator.testConnection(mockBrowser)).rejects.toThrow(
        "Browser is not connected",
      );
    });
  });

  describe("reconnect", () => {
    it("should close old and open new connection", async () => {
      const oldBrowser = {
        on: vi.fn(),
        close: vi.fn().mockResolvedValue(),
        isConnected: vi.fn().mockReturnValue(true),
      };
      const newBrowser = {
        on: vi.fn(),
        isConnected: vi.fn().mockReturnValue(true),
      };
      const wsEndpoint = "ws://localhost:1234";

      automator.connections.set(wsEndpoint, { browser: oldBrowser });
      chromium.connectOverCDP.mockResolvedValue(newBrowser);

      const reconnected = await automator.reconnect(wsEndpoint);

      expect(oldBrowser.close).toHaveBeenCalled();
      expect(reconnected).toBe(newBrowser);
      expect(automator.connections.get(wsEndpoint).browser).toBe(newBrowser);
    });

    it("should handle error when closing old connection", async () => {
      const oldBrowser = {
        on: vi.fn(),
        close: vi.fn().mockRejectedValue(new Error("Already closed")),
        isConnected: vi.fn().mockReturnValue(true),
      };
      const newBrowser = {
        on: vi.fn(),
        isConnected: vi.fn().mockReturnValue(true),
      };
      const wsEndpoint = "ws://localhost:1234";

      automator.connections.set(wsEndpoint, { browser: oldBrowser });
      chromium.connectOverCDP.mockResolvedValue(newBrowser);

      const reconnected = await automator.reconnect(wsEndpoint);
      expect(reconnected).toBe(newBrowser);
    });

    it("should reconnect when no existing connection", async () => {
      const newBrowser = {
        on: vi.fn(),
        isConnected: vi.fn().mockReturnValue(true),
      };
      const wsEndpoint = "ws://localhost:1234";

      // No connection set
      chromium.connectOverCDP.mockResolvedValue(newBrowser);

      const reconnected = await automator.reconnect(wsEndpoint);
      expect(reconnected).toBe(newBrowser);
    });
  });

  describe("attemptBackgroundReconnect", () => {
    it("should attempt reconnection", async () => {
      const mockBrowser = {
        on: vi.fn(),
        isConnected: vi.fn().mockReturnValue(false),
      };
      const wsEndpoint = "ws://localhost:1234";
      automator.connections.set(wsEndpoint, {
        browser: mockBrowser,
        reconnectAttempts: 0,
      });
      const newBrowser = {
        on: vi.fn(),
        isConnected: vi.fn().mockReturnValue(true),
      };
      chromium.connectOverCDP.mockResolvedValue(newBrowser);

      await automator.attemptBackgroundReconnect(
        wsEndpoint,
        automator.connections.get(wsEndpoint),
      );
      // Connection info is updated
    });

    it("should handle reconnection failure gracefully", async () => {
      const mockBrowser = {
        on: vi.fn(),
        isConnected: vi.fn().mockReturnValue(false),
      };
      const wsEndpoint = "ws://localhost:1234";
      automator.connections.set(wsEndpoint, {
        browser: mockBrowser,
        reconnectAttempts: 0,
      });
      chromium.connectOverCDP.mockRejectedValue(new Error("Connection failed"));

      await automator.attemptBackgroundReconnect(
        wsEndpoint,
        automator.connections.get(wsEndpoint),
      );
      // Should not throw
    });
  });

  describe("getBrowser", () => {
    it("should return browser for existing endpoint", async () => {
      const mockBrowser = {
        on: vi.fn(),
        isConnected: vi.fn().mockReturnValue(true),
      };
      const wsEndpoint = "ws://localhost:1234";
      automator.connections.set(wsEndpoint, { browser: mockBrowser });

      const browser = automator.getBrowser(wsEndpoint);
      expect(browser).toBe(mockBrowser);
    });

    it("should return null for unknown endpoint", () => {
      const browser = automator.getBrowser("unknown");
      expect(browser).toBeNull();
    });
  });

  describe("isHealthy", () => {
    it("should return healthy status", () => {
      const wsEndpoint = "ws://localhost:1234";
      automator.connections.set(wsEndpoint, { healthy: true });
      expect(automator.isHealthy(wsEndpoint)).toBe(true);
    });

    it("should return false for unhealthy", () => {
      const wsEndpoint = "ws://localhost:1234";
      automator.connections.set(wsEndpoint, { healthy: false });
      expect(automator.isHealthy(wsEndpoint)).toBe(false);
    });

    it("should return false for unknown endpoint", () => {
      expect(automator.isHealthy("unknown")).toBe(false);
    });
  });

  describe("getConnectedEndpoints", () => {
    it("should return all endpoints", () => {
      automator.connections.set("ws1", { browser: {} });
      automator.connections.set("ws2", { browser: {} });

      const endpoints = automator.getConnectedEndpoints();
      expect(endpoints).toContain("ws1");
      expect(endpoints).toContain("ws2");
    });
  });

  describe("getConnectionCount", () => {
    it("should return connection count", () => {
      automator.connections.set("ws1", { browser: {} });
      automator.connections.set("ws2", { browser: {} });

      expect(automator.getConnectionCount()).toBe(2);
    });
  });

  describe("getHealthyConnectionCount", () => {
    it("should count healthy connections", () => {
      automator.connections.set("ws1", { healthy: true });
      automator.connections.set("ws2", { healthy: false });
      automator.connections.set("ws3", { healthy: true });

      expect(automator.getHealthyConnectionCount()).toBe(2);
    });
  });

  describe("closeAll", () => {
    it("should close all browsers and clear connections", async () => {
      const mockBrowser = {
        on: vi.fn(),
        close: vi.fn().mockResolvedValue(),
      };
      automator.connections.set("ws1", {
        browser: mockBrowser,
        endpoint: "ws1",
      });
      automator.connections.set("ws2", {
        browser: mockBrowser,
        endpoint: "ws2",
      });

      await automator.closeAll();

      expect(mockBrowser.close).toHaveBeenCalledTimes(2);
      expect(automator.connections.size).toBe(0);
      expect(automator.isShuttingDown).toBe(true);
    });

    it("should handle browser without close method", async () => {
      const conn = { browser: {}, endpoint: "ws1" };
      automator.connections.set("ws1", conn);

      await automator.closeAll();
      expect(automator.connections.size).toBe(0);
    });

    it("should handle close error", async () => {
      const mockBrowser = {
        on: vi.fn(),
        close: vi.fn().mockRejectedValue(new Error("Close failed")),
        isConnected: vi.fn().mockReturnValue(true),
      };
      automator.connections.set("ws1", {
        browser: mockBrowser,
        endpoint: "ws1",
      });

      await automator.closeAll();
      expect(automator.connections.size).toBe(0);
    });
  });

  describe("checkPageResponsive", () => {
    it("should return healthy for responsive page", async () => {
      const mockPage = {
        isClosed: vi.fn().mockReturnValue(false),
        evaluate: vi.fn().mockResolvedValue({
          documentReady: "complete",
          title: "Test",
          bodyExists: true,
        }),
      };

      const result = await automator.checkPageResponsive(mockPage);
      expect(result.healthy).toBe(true);
      expect(result.title).toBe("Test");
    });

    it("should return unhealthy for closed page", async () => {
      const mockPage = { isClosed: vi.fn().mockReturnValue(true) };

      const result = await automator.checkPageResponsive(mockPage);
      expect(result.healthy).toBe(false);
      expect(result.error).toBe("Page is closed or null");
    });

    it("should return unhealthy for null page", async () => {
      const result = await automator.checkPageResponsive(null);
      expect(result.healthy).toBe(false);
    });

    it("should return unhealthy for loading page", async () => {
      const mockPage = {
        isClosed: vi.fn().mockReturnValue(false),
        evaluate: vi.fn().mockResolvedValue({
          documentReady: "loading",
          title: "",
          bodyExists: false,
        }),
      };

      const result = await automator.checkPageResponsive(mockPage);
      expect(result.healthy).toBe(false);
    });

    it("should handle evaluation error", async () => {
      const mockPage = {
        isClosed: vi.fn().mockReturnValue(false),
        evaluate: vi.fn().mockRejectedValue(new Error("Eval failed")),
      };

      const result = await automator.checkPageResponsive(mockPage);
      expect(result.healthy).toBe(false);
    });
  });

  describe("checkConnectionHealth", () => {
    it("should return health status for existing connection", async () => {
      const mockBrowser = {
        on: vi.fn(),
        isConnected: vi.fn().mockReturnValue(true),
      };
      const wsEndpoint = "ws://localhost:1234";
      automator.connections.set(wsEndpoint, {
        browser: mockBrowser,
        healthy: true,
        lastHealthCheck: Date.now(),
      });

      const result = await automator.checkConnectionHealth(wsEndpoint);
      expect(result.endpoint).toBe(wsEndpoint);
      expect(result.checks.browserConnection).toBe(true);
    });

    it("should return unhealthy for missing connection", async () => {
      const result = await automator.checkConnectionHealth("unknown");
      expect(result.healthy).toBe(false);
    });

    it("should check page when provided", async () => {
      const mockBrowser = {
        on: vi.fn(),
        isConnected: vi.fn().mockReturnValue(true),
      };
      const mockPage = {
        isClosed: vi.fn().mockReturnValue(false),
        evaluate: vi.fn().mockResolvedValue({
          documentReady: "complete",
          title: "Test",
          bodyExists: true,
        }),
      };
      const wsEndpoint = "ws://localhost:1234";
      automator.connections.set(wsEndpoint, {
        browser: mockBrowser,
        healthy: true,
        lastHealthCheck: Date.now(),
      });

      const result = await automator.checkConnectionHealth(
        wsEndpoint,
        mockPage,
      );
      expect(result.checks.page).toBeDefined();
      expect(result.checks.page.healthy).toBe(true);
    });
  });

  describe("recoverConnection", () => {
    it("should recover when browser is connected", async () => {
      const mockBrowser = {
        on: vi.fn(),
        isConnected: vi.fn().mockReturnValue(true),
      };
      const wsEndpoint = "ws://localhost:1234";
      automator.connections.set(wsEndpoint, { browser: mockBrowser });

      const result = await automator.recoverConnection(wsEndpoint);
      expect(result.successful).toBe(true);
      expect(result.steps).toContainEqual(
        expect.objectContaining({ step: "browser_check", success: true }),
      );
    });

    it("should reconnect when browser disconnected", async () => {
      const mockBrowser = {
        on: vi.fn(),
        isConnected: vi.fn().mockReturnValue(false),
      };
      const newBrowser = {
        on: vi.fn(),
        isConnected: vi.fn().mockReturnValue(true),
      };
      const wsEndpoint = "ws://localhost:1234";
      automator.connections.set(wsEndpoint, { browser: mockBrowser });
      chromium.connectOverCDP.mockResolvedValue(newBrowser);

      const result = await automator.recoverConnection(wsEndpoint);
      expect(result.steps).toContainEqual(
        expect.objectContaining({ step: "reconnect" }),
      );
    });

    it("should recover with page reload", async () => {
      const mockBrowser = {
        on: vi.fn(),
        isConnected: vi.fn().mockReturnValue(true),
      };
      const mockPage = {
        isClosed: vi.fn().mockReturnValue(false),
        reload: vi.fn().mockResolvedValue(),
      };
      const wsEndpoint = "ws://localhost:1234";
      automator.connections.set(wsEndpoint, { browser: mockBrowser });

      const result = await automator.recoverConnection(wsEndpoint, mockPage);
      expect(result.steps).toContainEqual(
        expect.objectContaining({ step: "page_reload", success: true }),
      );
    });

    it("should handle missing connection gracefully", async () => {
      const result = await automator.recoverConnection("unknown");
      expect(result).toBeDefined();
    });

    it("should handle isConnected error gracefully", async () => {
      const mockBrowser = {
        on: vi.fn(),
        isConnected: vi.fn().mockRejectedValue(new Error("Connection error")),
      };
      const newBrowser = {
        on: vi.fn(),
        isConnected: vi.fn().mockReturnValue(true),
      };
      const wsEndpoint = "ws://localhost:1234";
      automator.connections.set(wsEndpoint, { browser: mockBrowser });
      chromium.connectOverCDP.mockResolvedValue(newBrowser);

      const result = await automator.recoverConnection(wsEndpoint);
      expect(result).toBeDefined();
    });

    it("should handle page reload error in recovery", async () => {
      const mockBrowser = {
        on: vi.fn(),
        isConnected: vi.fn().mockReturnValue(true),
      };
      const mockPage = {
        isClosed: vi.fn().mockReturnValue(false),
        reload: vi.fn().mockRejectedValue(new Error("Reload failed")),
        goto: vi.fn().mockResolvedValue(),
      };
      const wsEndpoint = "ws://localhost:1234";
      automator.connections.set(wsEndpoint, { browser: mockBrowser });

      const result = await automator.recoverConnection(wsEndpoint, mockPage);
      expect(result.steps).toContainEqual(
        expect.objectContaining({ step: "page_reload", success: false }),
      );
    });

    it("should try navigate home on page reload failure", async () => {
      const mockBrowser = {
        on: vi.fn(),
        isConnected: vi.fn().mockReturnValue(true),
      };
      const mockPage = {
        isClosed: vi.fn().mockReturnValue(false),
        reload: vi.fn().mockRejectedValue(new Error("Reload failed")),
        goto: vi.fn().mockResolvedValue(),
      };
      const wsEndpoint = "ws://localhost:1234";
      automator.connections.set(wsEndpoint, { browser: mockBrowser });

      const result = await automator.recoverConnection(wsEndpoint, mockPage);
      expect(result.steps).toContainEqual(
        expect.objectContaining({ step: "navigate_home", success: true }),
      );
    });
  });

  describe("getHealthSummary", () => {
    it("should return health summary for connections", async () => {
      const mockBrowser = {
        on: vi.fn(),
        isConnected: vi.fn().mockReturnValue(true),
      };
      automator.connections.set("ws1", {
        browser: mockBrowser,
        healthy: true,
        lastHealthCheck: Date.now(),
      });

      const summary = await automator.getHealthSummary();
      expect(summary.total).toBeGreaterThan(0);
      expect(summary.connections).toBeDefined();
    });
  });

  describe("getStats", () => {
    it("should return connection statistics", async () => {
      automator.connections.set("ws1", {
        browser: {},
        endpoint: "ws1",
        healthy: true,
        connectedAt: Date.now() - 1000,
        lastHealthCheck: Date.now() - 500,
        reconnectAttempts: 0,
      });

      const stats = automator.getStats();
      expect(stats.totalConnections).toBe(1);
      expect(stats.healthyConnections).toBe(1);
      expect(stats.unhealthyConnections).toBe(0);
      expect(stats.connections).toHaveLength(1);
    });
  });

  describe("shutdown", () => {
    it("should shutdown gracefully", async () => {
      const mockBrowser = {
        on: vi.fn(),
        close: vi.fn().mockResolvedValue(),
      };
      automator.connections.set("ws1", {
        browser: mockBrowser,
        endpoint: "ws1",
      });

      await automator.shutdown();

      expect(mockBrowser.close).toHaveBeenCalled();
      expect(automator.connections.size).toBe(0);
    });
  });
});
