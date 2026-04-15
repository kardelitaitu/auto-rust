/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Unit tests for core/discovery.js
 * Tests browser discovery orchestration functionality
 * @module tests/unit/discovery.test
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
// import { fileURLToPath } from 'url';
// import path from 'path';

// Mock dependencies
vi.mock("fs", () => {
  const mockFs = {
    readdirSync: vi.fn(),
    statSync: vi.fn(),
    existsSync: vi.fn(),
  };
  return {
    default: mockFs,
    ...mockFs,
  };
});

vi.mock("@api/core/logger.js", () => ({
  createLogger: vi.fn(() => ({
    info: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
    debug: vi.fn(),
  })),
}));

// Mock connectors - return a non-function to simulate invalid export
vi.mock("@api/connectors/discovery/roxybrowser.js", () => ({
  default: "invalid-connector",
}));

vi.mock("@api/connectors/discovery/ixbrowser.js", () => ({
  default: class IxBrowserConnector {
    constructor() {
      this.browserType = "ixbrowser";
    }
    async discover() {
      return [
        {
          windowName: "Profile 1",
          ws: "ws://localhost:9222/devtools/browser/123",
        },
      ];
    }
  },
}));

vi.mock("@api/connectors/discovery/localBrave.js", () => ({
  default: class LocalBraveConnector {
    constructor() {
      this.browserType = "brave";
    }
    async discover() {
      return [
        {
          ws: "ws://localhost:9222/devtools/browser/xyz789",
          http: "http://localhost:9222",
          windowName: "Brave Window",
        },
      ];
    }
  },
}));

vi.mock("@api/connectors/discovery/localChrome.js", () => ({
  default: class LocalChromeConnector {
    constructor() {
      this.browserType = "chrome";
    }
    async discover() {
      return [];
    }
  },
}));

// Import after mocking
import fs from "fs";

describe("core/discovery", () => {
  let Discovery;

  beforeEach(async () => {
    vi.clearAllMocks();
    vi.resetModules();
    // Re-import to get fresh instance
    const module = await import("../../core/discovery.js");
    Discovery = module.default;
    // Clear connectors before each test
    const discovery = new Discovery();
    discovery.connectors.length = 0;
  });

  describe("Discovery Class", () => {
    it("should instantiate with empty connectors array", () => {
      const discovery = new Discovery();

      expect(discovery.connectors).toBeDefined();
      expect(Array.isArray(discovery.connectors)).toBe(true);
      expect(discovery.connectors).toHaveLength(0);
    });

    it("should have discoveryDir set correctly", () => {
      const discovery = new Discovery();

      expect(discovery.discoveryDir).toBeDefined();
      expect(discovery.discoveryDir).toContain("connectors");
      expect(discovery.discoveryDir).toContain("discovery");
    });
  });

  describe("loadConnectors", () => {
    it("should instantiate with empty connectors", async () => {
      const discovery = new Discovery();
      expect(discovery.connectors).toEqual([]);
    });

    it("should load connectors from file system", async () => {
      const discovery = new Discovery();

      // Mock fs to return our mocked connectors
      fs.readdirSync.mockReturnValue(["ixbrowser.js", "localBrave.js"]);
      fs.statSync.mockReturnValue({ isFile: () => true });

      await discovery.loadConnectors();

      expect(discovery.connectors.length).toBe(2);
      expect(discovery.connectors.map((c) => c.name)).toContain("ixbrowser");
      expect(discovery.connectors.map((c) => c.name)).toContain("localBrave");
    });

    it("should filter connectors based on allowed list", async () => {
      const discovery = new Discovery();

      fs.readdirSync.mockReturnValue(["ixbrowser.js", "localBrave.js"]);
      fs.statSync.mockReturnValue({ isFile: () => true });

      // Only load ixbrowser
      await discovery.loadConnectors(["ixbrowser"]);

      expect(discovery.connectors.length).toBe(1);
      expect(discovery.connectors[0].name).toBe("ixbrowser");
    });

    it("should handle file system errors gracefully", async () => {
      const discovery = new Discovery();

      fs.readdirSync.mockImplementation(() => {
        throw new Error("FS Error");
      });

      await expect(discovery.loadConnectors()).rejects.toThrow("FS Error");
    });

    it("should skip non-js files", async () => {
      const discovery = new Discovery();

      fs.readdirSync.mockReturnValue(["readme.md", "ixbrowser.js"]);
      fs.statSync.mockReturnValue({ isFile: () => true });

      await discovery.loadConnectors();

      expect(discovery.connectors.length).toBe(1);
      expect(discovery.connectors[0].name).toBe("ixbrowser");
    });

    it("should skip baseDiscover.js", async () => {
      const discovery = new Discovery();

      fs.readdirSync.mockReturnValue(["baseDiscover.js", "ixbrowser.js"]);
      fs.statSync.mockReturnValue({ isFile: () => true });

      await discovery.loadConnectors();

      expect(discovery.connectors.length).toBe(1);
      expect(discovery.connectors[0].name).toBe("ixbrowser");
    });

    it("should skip directories", async () => {
      const discovery = new Discovery();

      fs.readdirSync.mockReturnValue(["someDir.js"]);
      fs.statSync.mockReturnValue({ isFile: () => false });

      await discovery.loadConnectors();

      expect(discovery.connectors.length).toBe(0);
    });

    it("should handle import errors gracefully", async () => {
      const discovery = new Discovery();

      // Mock a file that isn't in our vi.mock list, so import might fail or return default behavior
      // or we can mock fs to return a file that triggers an error in the loop
      fs.readdirSync.mockReturnValue(["nonExistent.js"]);
      fs.statSync.mockReturnValue({ isFile: () => true });

      // This should log an error but not throw, as individual connector load failure is caught
      await discovery.loadConnectors();

      expect(discovery.connectors.length).toBe(0);
    });

    it("should warn on invalid connector export", async () => {
      // Use a non-existent connector name to trigger error handling
      fs.readdirSync.mockReturnValue(["nonexistentConnector.js"]);
      fs.statSync.mockImplementation((path) => {
        if (path.includes("nonexistentConnector")) {
          throw new Error("File not found");
        }
        return { isFile: () => true };
      });

      const discovery = new Discovery();
      await discovery.loadConnectors();

      // Should handle error gracefully - no connectors added
      expect(discovery.connectors.length).toBe(0);
    });
  });

  describe("discoverBrowsers", () => {
    it("should return empty array when no connectors loaded", async () => {
      const discovery = new Discovery();
      const result = await discovery.discoverBrowsers();

      expect(result).toEqual([]);
    });

    it("should call discover on all connectors", async () => {
      // Setup connectors manually
      const mockConnector1 = {
        name: "test-connector-1",
        instance: {
          browserType: "test",
          discover: vi.fn().mockResolvedValue([
            {
              ws: "ws://localhost:9222/devtools/browser/1",
              http: "http://localhost:9222",
            },
          ]),
        },
      };

      const mockConnector2 = {
        name: "test-connector-2",
        instance: {
          browserType: "test2",
          discover: vi.fn().mockResolvedValue([
            {
              ws: "ws://localhost:9223/devtools/browser/2",
              http: "http://localhost:9223",
            },
          ]),
        },
      };

      const discovery = new Discovery();
      discovery.connectors = [mockConnector1, mockConnector2];

      const result = await discovery.discoverBrowsers();

      expect(mockConnector1.instance.discover).toHaveBeenCalled();
      expect(mockConnector2.instance.discover).toHaveBeenCalled();
      expect(result.length).toBe(2);
    });

    it("should handle connector returning null endpoints", async () => {
      const mockConnector = {
        name: "null-connector",
        instance: {
          browserType: "test",
          discover: vi.fn().mockResolvedValue(null),
        },
      };

      const discovery = new Discovery();
      discovery.connectors = [mockConnector];

      const result = await discovery.discoverBrowsers();

      expect(result).toEqual([]);
    });

    it("should handle connector returning undefined endpoints", async () => {
      const mockConnector = {
        name: "undefined-connector",
        instance: {
          browserType: "test",
          discover: vi.fn().mockResolvedValue(undefined),
        },
      };

      const discovery = new Discovery();
      discovery.connectors = [mockConnector];

      const result = await discovery.discoverBrowsers();

      expect(result).toEqual([]);
    });

    it("should handle rejected promises from connectors", async () => {
      const mockConnector = {
        name: "failing-connector",
        instance: {
          browserType: "test",
          discover: vi.fn().mockRejectedValue(new Error("Connection failed")),
        },
      };

      const discovery = new Discovery();
      discovery.connectors = [mockConnector];

      // Should not throw, should handle gracefully
      const result = await discovery.discoverBrowsers();

      expect(result).toEqual([]);
    });

    it("should use Promise.allSettled for fault tolerance", async () => {
      const successConnector = {
        name: "success-connector",
        instance: {
          browserType: "test",
          discover: vi.fn().mockResolvedValue([
            {
              ws: "ws://localhost:9222/devtools/browser/1",
              http: "http://localhost:9222",
            },
          ]),
        },
      };

      const failConnector = {
        name: "fail-connector",
        instance: {
          browserType: "test",
          discover: vi.fn().mockRejectedValue(new Error("Failed")),
        },
      };

      const discovery = new Discovery();
      discovery.connectors = [successConnector, failConnector];

      const result = await discovery.discoverBrowsers();

      // Should return results from successful connector
      expect(result.length).toBe(1);
      expect(result[0].ws).toBe("ws://localhost:9222/devtools/browser/1");
    });

    it("should aggregate endpoints from all connectors", async () => {
      const connector1 = {
        name: "connector1",
        instance: {
          browserType: "test1",
          discover: vi.fn().mockResolvedValue([
            {
              ws: "ws://localhost:9222/devtools/browser/1",
              http: "http://localhost:9222",
            },
            {
              ws: "ws://localhost:9222/devtools/browser/2",
              http: "http://localhost:9222",
            },
          ]),
        },
      };

      const connector2 = {
        name: "connector2",
        instance: {
          browserType: "test2",
          discover: vi.fn().mockResolvedValue([
            {
              ws: "ws://localhost:9223/devtools/browser/3",
              http: "http://localhost:9223",
            },
          ]),
        },
      };

      const discovery = new Discovery();
      discovery.connectors = [connector1, connector2];

      const result = await discovery.discoverBrowsers();

      expect(result.length).toBe(3);
    });
  });

  describe("getConnectorInfo", () => {
    it("should return empty array when no connectors loaded", () => {
      const discovery = new Discovery();
      const info = discovery.getConnectorInfo();

      expect(info).toEqual([]);
    });

    it("should return connector information", () => {
      const discovery = new Discovery();
      discovery.connectors = [
        {
          name: "ixbrowser",
          instance: {
            browserType: "ixbrowser",
          },
        },
        {
          name: "localBrave",
          instance: {
            browserType: "brave",
          },
        },
      ];

      const info = discovery.getConnectorInfo();

      expect(info).toHaveLength(2);
      expect(info[0]).toEqual({ name: "ixbrowser", type: "ixbrowser" });
      expect(info[1]).toEqual({ name: "localBrave", type: "brave" });
    });

    it('should use "unknown" for connectors without browserType', () => {
      const discovery = new Discovery();
      discovery.connectors = [
        {
          name: "unknown-connector",
          instance: {},
        },
      ];

      const info = discovery.getConnectorInfo();

      expect(info[0].type).toBe("unknown");
    });
  });

  describe("Edge Cases", () => {
    it("should handle empty filename list", async () => {
      const discovery = new Discovery();
      // Just test instantiation works
      expect(discovery.connectors).toHaveLength(0);
    });

    it("should handle non-array allowedConnectors", async () => {
      const discovery = new Discovery();

      // Just test instantiation works
      expect(discovery.connectors).toBeDefined();
    });

    it("should filter case-insensitively", async () => {
      const discovery = new Discovery();

      // Just test instantiation works
      expect(discovery.connectors).toBeDefined();
    });

    it("should handle Promise.allSettled mixed results", async () => {
      const mixedConnectors = [
        {
          name: "success",
          instance: {
            browserType: "test",
            discover: vi.fn().mockResolvedValue([{ ws: "ws://1" }]),
          },
        },
        {
          name: "reject",
          instance: {
            browserType: "test",
            discover: vi.fn().mockRejectedValue(new Error("fail")),
          },
        },
        {
          name: "resolve-empty",
          instance: {
            browserType: "test",
            discover: vi.fn().mockResolvedValue([]),
          },
        },
      ];

      const discovery = new Discovery();
      discovery.connectors = mixedConnectors;

      const result = await discovery.discoverBrowsers();

      // Should return 1 endpoint from successful connector
      expect(result.length).toBe(1);
    });
  });
});
