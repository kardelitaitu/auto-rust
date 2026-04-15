/**
 * Simple Session Manager Integration Test
 * Basic session manager functionality
 */

import { describe, it, expect, vi } from "vitest";

vi.mock("@api/core/logger.js", () => ({
  createLogger: vi.fn(() => ({
    info: vi.fn(),
    debug: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
  })),
}));

vi.mock("@api/utils/configLoader.js", () => ({
  getSettings: vi.fn().mockResolvedValue({
    sessionManager: {
      maxSessions: 100,
      sessionTimeout: 1800000,
    },
  }),
  getTimeoutValue: vi.fn().mockResolvedValue(30000),
}));

describe("Simple Session Manager Integration", () => {
  it("should import session manager module", async () => {
    const SessionManager = await import("../../core/sessionManager.js");
    expect(SessionManager.default).toBeDefined();
  });

  it("should have session manager constructor", async () => {
    const SessionManager = (await import("../../core/sessionManager.js"))
      .default;
    const sm = new SessionManager();
    expect(sm).toBeDefined();
  });

  it("should create session manager instance", async () => {
    const SessionManager = (await import("../../core/sessionManager.js"))
      .default;
    const sessionManager = new SessionManager();
    expect(sessionManager.sessions).toBeDefined();
  });
});
