/**
 * Essential Integration Tests
 * Core flows that browser automation depends on
 */

import { describe, it, expect, vi } from "vitest";

vi.mock("@api/core/context.js", () => ({
  getPage: vi.fn(() => ({
    evaluate: vi.fn(() =>
      Promise.resolve({ loadTime: 100, firstPaint: 50, lcp: 100, lag: 5 }),
    ),
  })),
  isSessionActive: vi.fn(() => true),
  checkSession: vi.fn(() => true),
  clearContext: vi.fn(),
  getCursor: vi.fn(),
  evalPage: vi.fn(),
  withPage: vi.fn(async (page, fn) => fn(page)),
}));

vi.mock("@api/core/logger.js", () => ({
  createLogger: vi.fn(() => ({
    debug: vi.fn(),
    info: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
  })),
}));

describe("Essential Context Exports", () => {
  it("has context functions", async () => {
    const context = await import("@api/core/context.js");
    expect(context.clearContext).toBeDefined();
    expect(context.isSessionActive).toBeDefined();
    expect(context.checkSession).toBeDefined();
  });
});

describe("Essential Configuration Flow", () => {
  it("loads and returns config", async () => {
    const { getTwitterActivity, getEngagementLimits } =
      await import("@api/utils/config-service.js");

    const activity = getTwitterActivity();
    const limits = getEngagementLimits();

    expect(activity).toBeDefined();
    expect(limits).toBeDefined();
  });

  it("loads timeout settings", async () => {
    const { getTimeouts } = await import("@api/utils/configLoader.js");
    const timeouts = await getTimeouts();
    expect(timeouts).toBeDefined();
  });
});

describe("Essential Persona Flow", () => {
  it("sets and gets persona", async () => {
    const { setPersona, getPersona, getPersonaName } =
      await import("@api/behaviors/persona.js");

    setPersona("power");
    expect(getPersonaName()).toBe("power");

    const persona = getPersona();
    expect(persona.speed).toBeDefined();
  });

  it("lists available personas", async () => {
    const { listPersonas } = await import("@api/behaviors/persona.js");
    const names = listPersonas();
    expect(names).toContain("casual");
    expect(names).toContain("power");
  });
});

describe("Essential Timing Flow", () => {
  it("generates random delay", async () => {
    const { randomInRange } = await import("@api/behaviors/timing.js");
    const delay = randomInRange(100, 500);
    expect(delay).toBeGreaterThanOrEqual(100);
    expect(delay).toBeLessThanOrEqual(500);
  });

  it("generates gaussian distribution", async () => {
    const { gaussian } = await import("@api/behaviors/timing.js");
    const value = gaussian(100, 10, 50, 150);
    expect(value).toBeGreaterThanOrEqual(50);
    expect(value).toBeLessThanOrEqual(150);
  });
});

describe("Essential Math Exports", () => {
  it("has math utilities", async () => {
    const mathUtils = await import("@api/utils/math.js");
    expect(mathUtils.mathUtils.randomInRange).toBeDefined();
    expect(mathUtils.mathUtils.roll).toBeDefined();
    expect(mathUtils.mathUtils.sample).toBeDefined();
    expect(mathUtils.mathUtils.gaussian).toBeDefined();
  });
});

describe("Essential Cache Flow", () => {
  it("sets and gets from cache", async () => {
    const { setInCache, getFromCache, clearCache } =
      await import("@api/utils/config-cache.js");

    clearCache();
    setInCache("key1", { data: "value" });

    const result = getFromCache("key1");
    expect(result).toEqual({ data: "value" });
  });

  it("returns null for missing cache key", async () => {
    const { getFromCache, clearCache } =
      await import("@api/utils/config-cache.js");
    clearCache();
    expect(getFromCache("nonexistent")).toBeNull();
  });
});

describe("Essential Validation Flow", () => {
  it("validates config structure", async () => {
    const { validateConfig } = await import("@api/utils/config-validator.js");
    const result = validateConfig({ twitter: { enabled: true } });
    expect(result).toBeDefined();
  });
});

describe("Essential Agent Token Counting", () => {
  it("estimates token count", async () => {
    const { estimateTokens } = await import("@api/agent/tokenCounter.js");
    const tokens = estimateTokens("Hello world");
    expect(tokens).toBeGreaterThan(0);
  });
});

describe("Essential Interactions Export", () => {
  it("has navigation functions", async () => {
    const nav = await import("@api/interactions/navigation.js");
    expect(nav.goto).toBeDefined();
    expect(nav.reload).toBeDefined();
  });

  it("has query functions", async () => {
    const queries = await import("@api/interactions/queries.js");
    expect(queries.text).toBeDefined();
    expect(queries.count).toBeDefined();
  });

  it("has wait functions", async () => {
    const wait = await import("@api/interactions/wait.js");
    expect(wait.wait).toBeDefined();
    expect(wait.waitFor).toBeDefined();
  });

  it("has action functions", async () => {
    const actions = await import("@api/interactions/actions.js");
    expect(actions.click).toBeDefined();
    expect(actions.type).toBeDefined();
  });
});

describe("Essential Twitter Intent Exports", () => {
  it("has intent functions", async () => {
    const like = await import("@api/twitter/intent-like.js");
    const follow = await import("@api/twitter/intent-follow.js");
    const retweet = await import("@api/twitter/intent-retweet.js");

    expect(like.like).toBeDefined();
    expect(follow.follow).toBeDefined();
    expect(retweet.retweet).toBeDefined();
  });
});

describe("Essential Error Classes", () => {
  it("creates error instances", async () => {
    const { ValidationError, AutomationError, SessionError } =
      await import("@api/core/errors.js");

    const valError = new ValidationError("test");
    expect(valError.message).toBe("test");

    const autoError = new AutomationError("fail");
    expect(autoError.message).toBe("fail");

    const sessError = new SessionError("timeout");
    expect(sessError.message).toBe("timeout");
  });
});

describe("Essential Middleware Flow", () => {
  it("creates pipeline", async () => {
    const { createPipeline } = await import("@api/core/middleware.js");
    expect(typeof createPipeline).toBe("function");
  });
});

describe("Essential Hooks Flow", () => {
  it("has hook functions", async () => {
    const hooks = await import("@api/core/hooks.js");
    expect(hooks.createHookWrapper).toBeDefined();
    expect(hooks.withErrorHook).toBeDefined();
  });
});

describe("Essential Plugins Flow", () => {
  it("has plugin management", async () => {
    const plugins = await import("@api/core/plugins/index.js");
    expect(plugins.registerPlugin).toBeDefined();
    expect(plugins.listPlugins).toBeDefined();
  });
});
