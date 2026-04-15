/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * @fileoverview Agent Integration Tests
 * Tests the agent connector structure and basic operations
 * @module tests/integration/agent/agent-workflow
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";

const { mockGetSettings, mockGetTimeouts, mockGetTimeoutValue } = vi.hoisted(
  () => {
    const mockGetSettings = vi.fn().mockResolvedValue({
      llm: { cloud: { enabled: false, providers: [] } },
    });

    const mockGetTimeouts = vi.fn().mockResolvedValue({
      api: { retryDelayMs: 1000, maxRetries: 2 },
      requestQueue: { maxConcurrent: 5 },
      circuitBreaker: { failureThreshold: 5, resetTimeout: 30000 },
    });

    const mockGetTimeoutValue = vi
      .fn()
      .mockImplementation((path, defaultValue) => {
        const sections = {
          requestQueue: { maxConcurrent: 5, retryDelay: 1000, maxRetries: 3 },
          circuitBreaker: {
            failureThreshold: 5,
            successThreshold: 3,
            resetTimeout: 30000,
          },
          api: { retryDelayMs: 1000, maxRetries: 2 },
        };
        return Promise.resolve(sections[path] ?? defaultValue);
      });

    return { mockGetSettings, mockGetTimeouts, mockGetTimeoutValue };
  },
);

vi.mock("@api/core/logger.js", () => ({
  createLogger: vi.fn(() => ({
    info: vi.fn(),
    success: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
    debug: vi.fn(),
  })),
}));

vi.mock("@api/core/config.js", () => ({
  configManager: {
    init: vi.fn().mockResolvedValue(undefined),
    get: vi.fn().mockReturnValue({
      baseUrl: "http://localhost:11434",
      model: "test-model",
      serverType: "ollama",
    }),
    _getDefaults: vi.fn().mockReturnValue({
      agent: { llm: { baseUrl: "http://localhost:11434" } },
    }),
  },
}));

vi.mock("@api/utils/configLoader.js", () => ({
  getSettings: mockGetSettings,
  getTimeouts: mockGetTimeouts,
  getTimeoutValue: mockGetTimeoutValue,
}));

import AgentConnector from "@api/core/agent-connector.js";

describe("Agent Connector Structure", () => {
  let agentConnector;

  beforeEach(async () => {
    vi.clearAllMocks();
    agentConnector = new AgentConnector();
  });

  afterEach(() => {
    agentConnector = null;
  });

  describe("Constructor", () => {
    it("should create an AgentConnector instance", () => {
      expect(agentConnector).toBeDefined();
      expect(agentConnector).toBeInstanceOf(AgentConnector);
    });

    it("should have localClient property", () => {
      expect(agentConnector.localClient).toBeDefined();
    });

    it("should have cloudClient property", () => {
      expect(agentConnector.cloudClient).toBeDefined();
    });

    it("should have visionInterpreter property", () => {
      expect(agentConnector.visionInterpreter).toBeDefined();
    });

    it("should have requestQueue property", () => {
      expect(agentConnector.requestQueue).toBeDefined();
    });

    it("should have circuitBreaker property", () => {
      expect(agentConnector.circuitBreaker).toBeDefined();
    });
  });

  describe("Statistics", () => {
    it("should have stats object", () => {
      expect(agentConnector.stats).toBeDefined();
    });

    it("should initialize totalRequests to 0", () => {
      expect(agentConnector.stats.totalRequests).toBe(0);
    });

    it("should initialize successfulRequests to 0", () => {
      expect(agentConnector.stats.successfulRequests).toBe(0);
    });

    it("should initialize failedRequests to 0", () => {
      expect(agentConnector.stats.failedRequests).toBe(0);
    });

    it("should track startTime", () => {
      expect(agentConnector.stats.startTime).toBeDefined();
      expect(typeof agentConnector.stats.startTime).toBe("number");
    });
  });

  describe("Method Existence", () => {
    it("should have processRequest method", () => {
      expect(typeof agentConnector.processRequest).toBe("function");
    });

    it("should have getStats method", () => {
      expect(typeof agentConnector.getStats).toBe("function");
    });

    it("should have getHealth method", () => {
      expect(typeof agentConnector.getHealth).toBe("function");
    });

    it("should have logHealth method", () => {
      expect(typeof agentConnector.logHealth).toBe("function");
    });

    it("should have handleGenerateReply method", () => {
      expect(typeof agentConnector.handleGenerateReply).toBe("function");
    });

    it("should have handleVisionRequest method", () => {
      expect(typeof agentConnector.handleVisionRequest).toBe("function");
    });
  });

  describe("Client Components", () => {
    it("should have localClient as object", () => {
      expect(typeof agentConnector.localClient).toBe("object");
    });

    it("should have cloudClient as object", () => {
      expect(typeof agentConnector.cloudClient).toBe("object");
    });

    it("should have visionInterpreter as object", () => {
      expect(typeof agentConnector.visionInterpreter).toBe("object");
    });
  });

  describe("Infrastructure Components", () => {
    it("should have requestQueue with getStats method", () => {
      expect(typeof agentConnector.requestQueue.getStats).toBe("function");
    });

    it("should have circuitBreaker with getState method", () => {
      expect(typeof agentConnector.circuitBreaker.getState).toBe("function");
    });
  });
});

describe("Agent Connector Integration", () => {
  let agentConnector;

  beforeEach(async () => {
    vi.clearAllMocks();
    agentConnector = new AgentConnector();
  });

  afterEach(() => {
    agentConnector = null;
  });

  describe("Stats Management", () => {
    it("should return stats via getStats()", () => {
      const stats = agentConnector.getStats();
      expect(stats).toBeDefined();
      expect(stats.requests).toBeDefined();
      expect(stats.requests.total).toBe(0);
      expect(stats.queue).toBeDefined();
      expect(stats.circuitBreaker).toBeDefined();
    });

    it("should return health via getHealth()", () => {
      const health = agentConnector.getHealth();
      expect(health).toBeDefined();
      expect(typeof health.healthScore).toBe("number");
      expect(health.status).toBeDefined();
    });
  });

  describe("Mock Interactions", () => {
    it("should use mocked getSettings", () => {
      expect(mockGetSettings).toHaveBeenCalled();
    });

    it("should use mocked getTimeoutValue", () => {
      expect(mockGetTimeoutValue).toHaveBeenCalled();
    });
  });

  describe("Request Queue", () => {
    it("should have request queue stats", () => {
      const queueStats = agentConnector.requestQueue.getStats();
      expect(queueStats).toBeDefined();
      expect(typeof queueStats.running).toBe("number");
      expect(typeof queueStats.queued).toBe("number");
    });
  });

  describe("Circuit Breaker", () => {
    it("should have circuit breaker state", () => {
      const state = agentConnector.circuitBreaker.getState();
      expect(state).toBeDefined();
    });
  });
});
