/**
 * Integration tests for utils modules - Real Implementations
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { mathUtils } from "@api/utils/math.js";
import {
  taskConfigLoader,
  loadAiTwitterActivityConfig,
} from "@api/utils/task-config-loader.js";
import {
  PERSONAS,
  listPersonas,
  getPersonaName,
  getPersonaParam,
  setPersona,
  getPersona,
} from "@api/behaviors/persona.js";
import { gaussian, randomInRange } from "@api/behaviors/timing.js";

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
}));

vi.mock("@api/core/logger.js", () => ({
  createLogger: vi.fn(() => ({
    debug: vi.fn(),
    info: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
  })),
}));

describe("Math Utils Integration", () => {
  describe("gaussian", () => {
    it("generates normally distributed numbers around mean", () => {
      const results = [];
      for (let i = 0; i < 1000; i++) {
        results.push(mathUtils.gaussian(100, 10));
      }
      const avg = results.reduce((a, b) => a + b, 0) / results.length;
      expect(avg).toBeGreaterThan(90);
      expect(avg).toBeLessThan(110);
    });

    it("respects min bound", () => {
      for (let i = 0; i < 100; i++) {
        const result = mathUtils.gaussian(100, 50, 80);
        expect(result).toBeGreaterThanOrEqual(80);
      }
    });

    it("respects max bound", () => {
      for (let i = 0; i < 100; i++) {
        const result = mathUtils.gaussian(100, 50, undefined, 120);
        expect(result).toBeLessThanOrEqual(120);
      }
    });

    it("respects both min and max bounds", () => {
      for (let i = 0; i < 100; i++) {
        const result = mathUtils.gaussian(100, 50, 80, 120);
        expect(result).toBeGreaterThanOrEqual(80);
        expect(result).toBeLessThanOrEqual(120);
      }
    });
  });

  describe("randomInRange", () => {
    it("returns integer within range inclusive", () => {
      const min = 5;
      const max = 10;
      for (let i = 0; i < 100; i++) {
        const result = mathUtils.randomInRange(min, max);
        expect(result).toBeGreaterThanOrEqual(min);
        expect(result).toBeLessThanOrEqual(max);
        expect(Number.isInteger(result)).toBe(true);
      }
    });

    it("handles same min and max", () => {
      expect(mathUtils.randomInRange(5, 5)).toBe(5);
    });
  });

  describe("roll", () => {
    it("returns true when threshold is 1", () => {
      let allTrue = true;
      for (let i = 0; i < 100; i++) {
        if (!mathUtils.roll(1)) allTrue = false;
      }
      expect(allTrue).toBe(true);
    });

    it("returns false when threshold is 0", () => {
      let allFalse = true;
      for (let i = 0; i < 100; i++) {
        if (mathUtils.roll(0)) allFalse = false;
      }
      expect(allFalse).toBe(true);
    });

    it("returns approximately correct percentage", () => {
      const threshold = 0.5;
      let trueCount = 0;
      const total = 1000;
      for (let i = 0; i < total; i++) {
        if (mathUtils.roll(threshold)) trueCount++;
      }
      const percentage = trueCount / total;
      expect(percentage).toBeGreaterThan(0.4);
      expect(percentage).toBeLessThan(0.6);
    });
  });

  describe("sample", () => {
    it("returns random element from array", () => {
      const arr = [1, 2, 3, 4, 5];
      const picked = mathUtils.sample(arr);
      expect(arr).toContain(picked);
    });

    it("returns null for empty array", () => {
      expect(mathUtils.sample([])).toBeNull();
    });

    it("returns null for null input", () => {
      expect(mathUtils.sample(null)).toBeNull();
    });

    it("returns null for undefined input", () => {
      expect(mathUtils.sample(undefined)).toBeNull();
    });
  });

  describe("pidStep", () => {
    it("converges to target over iterations", () => {
      const state = { pos: 0, integral: 0, prevError: 0 };
      const model = { Kp: 0.1, Ki: 0.01, Kd: 0.05 };
      const target = 100;

      for (let i = 0; i < 100; i++) {
        mathUtils.pidStep(state, target, model);
      }

      expect(state.pos).toBeGreaterThan(80);
    });

    it("clamps integral to prevent windup", () => {
      const state = { pos: 0, integral: 0, prevError: 0 };
      const model = { Kp: 0.1, Ki: 10, Kd: 0 };

      for (let i = 0; i < 20; i++) {
        mathUtils.pidStep(state, 1000, model);
      }

      expect(state.integral).toBeLessThanOrEqual(10);
      expect(state.integral).toBeGreaterThanOrEqual(-10);
    });

    it("tracks previous error", () => {
      const state = { pos: 0, integral: 0, prevError: 0 };
      const model = { Kp: 1, Ki: 0, Kd: 0 };

      mathUtils.pidStep(state, 10, model);
      expect(state.prevError).toBe(10);
    });
  });
});

describe("Config Loader Integration", () => {
  it("loads AI Twitter Activity config", async () => {
    const config = await loadAiTwitterActivityConfig({});
    expect(config).toBeDefined();
  });

  it("loads AI Twitter Activity config with payload overrides", async () => {
    const config = await loadAiTwitterActivityConfig({ testMode: true });
    expect(config).toBeDefined();
  });

  it("uses taskConfigLoader instance methods", async () => {
    expect(taskConfigLoader).toBeDefined();
    expect(typeof taskConfigLoader.loadAiTwitterActivityConfig).toBe(
      "function",
    );
  });
});

describe("Persona Integration", () => {
  beforeEach(() => {
    setPersona("casual");
  });

  describe("PERSONAS", () => {
    it("exports all defined personas", () => {
      expect(PERSONAS).toBeDefined();
      expect(PERSONAS.casual).toBeDefined();
      expect(PERSONAS.efficient).toBeDefined();
      expect(PERSONAS.researcher).toBeDefined();
      expect(PERSONAS.power).toBeDefined();
      expect(PERSONAS.elderly).toBeDefined();
    });

    it("each persona has required properties", () => {
      const requiredProps = [
        "speed",
        "hoverMin",
        "hoverMax",
        "typoRate",
        "correctionRate",
        "hesitation",
      ];
      for (const [name, persona] of Object.entries(PERSONAS)) {
        for (const prop of requiredProps) {
          expect(persona).toHaveProperty(prop);
        }
        expect(typeof persona.speed).toBe("number");
        expect(persona.speed).toBeGreaterThan(0);
      }
    });

    it("persona speeds are in expected ranges", () => {
      expect(PERSONAS.elderly.speed).toBeLessThan(PERSONAS.casual.speed);
      expect(PERSONAS.casual.speed).toBeLessThan(PERSONAS.power.speed);
    });
  });

  describe("listPersonas", () => {
    it("returns array of persona names", () => {
      const names = listPersonas();
      expect(Array.isArray(names)).toBe(true);
      expect(names).toContain("casual");
      expect(names).toContain("efficient");
      expect(names).toContain("researcher");
    });
  });

  describe("setPersona", () => {
    it("sets persona by name", () => {
      setPersona("power");
      expect(getPersonaName()).toBe("power");
    });

    it("accepts overrides", () => {
      setPersona("casual", { speed: 2.0 });
      const persona = getPersona();
      expect(persona.speed).toBe(2.0);
    });
  });

  describe("getPersona", () => {
    it("returns current persona object", () => {
      setPersona("researcher");
      const persona = getPersona();
      expect(persona).toBeDefined();
      expect(persona.speed).toBeDefined();
    });
  });

  describe("getPersonaName", () => {
    it("returns name of current persona", () => {
      setPersona("efficient");
      expect(getPersonaName()).toBe("efficient");
    });
  });

  describe("getPersonaParam", () => {
    it("returns specific persona parameter", () => {
      setPersona("casual");
      expect(getPersonaParam("speed")).toBe(PERSONAS.casual.speed);
    });

    it("returns undefined for unknown param", () => {
      setPersona("casual");
      expect(getPersonaParam("nonexistent")).toBeUndefined();
    });
  });
});

describe("Timing Integration", () => {
  describe("gaussian", () => {
    it("returns gaussian distributed number", () => {
      const result = gaussian(100, 10, 50, 150);
      expect(result).toBeGreaterThanOrEqual(50);
      expect(result).toBeLessThanOrEqual(150);
    });
  });

  describe("randomInRange", () => {
    it("returns random integer in range", () => {
      const result = randomInRange(5, 10);
      expect(result).toBeGreaterThanOrEqual(5);
      expect(result).toBeLessThanOrEqual(10);
    });
  });
});

describe("Config Service Integration", () => {
  it("exports config service", async () => {
    const { config } = await import("@api/utils/config-service.js");
    expect(config).toBeDefined();
  });

  it("gets twitter activity config", async () => {
    const { getTwitterActivity } = await import("@api/utils/config-service.js");
    const activity = getTwitterActivity();
    expect(activity).toBeDefined();
  });

  it("gets engagement limits", async () => {
    const { getEngagementLimits } =
      await import("@api/utils/config-service.js");
    const limits = getEngagementLimits();
    expect(limits).toBeDefined();
  });

  it("gets timing config", async () => {
    const { getTiming } = await import("@api/utils/config-service.js");
    const timing = getTiming();
    expect(timing).toBeDefined();
  });

  it("gets humanization config", async () => {
    const { getHumanization } = await import("@api/utils/config-service.js");
    const humanization = getHumanization();
    expect(humanization).toBeDefined();
  });

  it("gets mouse config", async () => {
    const { getMouseConfig } = await import("@api/utils/config-service.js");
    const mouse = getMouseConfig();
    expect(mouse).toBeDefined();
  });

  it("gets LLM config", async () => {
    const { getLLMConfig } = await import("@api/utils/config-service.js");
    const llm = getLLMConfig();
    expect(llm).toBeDefined();
  });
});

describe("Config Cache Integration", () => {
  it("exports cache functions", async () => {
    const { getFromCache, setInCache, clearCache } =
      await import("@api/utils/config-cache.js");
    expect(typeof getFromCache).toBe("function");
    expect(typeof setInCache).toBe("function");
    expect(typeof clearCache).toBe("function");
  });

  it("can set and get from cache", async () => {
    const { setInCache, getFromCache, clearCache } =
      await import("@api/utils/config-cache.js");
    clearCache();
    setInCache("testKey", { value: 42 });
    const result = getFromCache("testKey");
    expect(result).toEqual({ value: 42 });
    clearCache();
  });

  it("returns null for missing keys", async () => {
    const { getFromCache, clearCache } =
      await import("@api/utils/config-cache.js");
    clearCache();
    expect(getFromCache("nonexistent")).toBeNull();
  });
});

describe("Config Validator Integration", () => {
  it("exports validator functions", async () => {
    const { validateConfig, validateWithReport } =
      await import("@api/utils/config-validator.js");
    expect(typeof validateConfig).toBe("function");
    expect(typeof validateWithReport).toBe("function");
  });

  it("validates valid config", async () => {
    const { validateConfig } = await import("@api/utils/config-validator.js");
    const result = validateConfig({ twitter: { enabled: true } });
    expect(result).toBeDefined();
  });
});

describe("Constants Integration", () => {
  it("exports engagement constants", async () => {
    const engagement = await import("@api/constants/engagement.js");
    expect(engagement.default).toBeDefined();
    expect(engagement.default.TWITTER_CLICK_PROFILES).toBeDefined();
  });

  it("exports timeout constants", async () => {
    const { TWITTER_TIMEOUTS } =
      await import("@api/constants/twitter-timeouts.js");
    expect(TWITTER_TIMEOUTS).toBeDefined();
  });
});

describe("Interactions - Module Exports", () => {
  it("exports actions module", async () => {
    const actions = await import("@api/interactions/actions.js");
    expect(actions.click).toBeDefined();
    expect(actions.type).toBeDefined();
    expect(actions.press).toBeDefined();
  });

  it("exports scroll module", async () => {
    const scroll = await import("@api/interactions/scroll.js");
    expect(scroll.scroll).toBeDefined();
    expect(scroll.toTop).toBeDefined();
    expect(scroll.toBottom).toBeDefined();
  });

  it("exports navigation module", async () => {
    const navigation = await import("@api/interactions/navigation.js");
    expect(navigation.goto).toBeDefined();
    expect(navigation.reload).toBeDefined();
    expect(navigation.back).toBeDefined();
    expect(navigation.forward).toBeDefined();
  });

  it("exports queries module", async () => {
    const queries = await import("@api/interactions/queries.js");
    expect(queries.text).toBeDefined();
    expect(queries.count).toBeDefined();
    expect(queries.exists).toBeDefined();
  });

  it("exports cursor module", async () => {
    const cursor = await import("@api/interactions/cursor.js");
    expect(cursor.move).toBeDefined();
    expect(cursor.up).toBeDefined();
    expect(cursor.down).toBeDefined();
  });

  it("exports wait module", async () => {
    const wait = await import("@api/interactions/wait.js");
    expect(wait.wait).toBeDefined();
    expect(wait.waitFor).toBeDefined();
    expect(wait.waitVisible).toBeDefined();
  });
});

describe("Agent Module Exports", () => {
  it("exports observer module", async () => {
    const { see } = await import("@api/agent/observer.js");
    expect(see).toBeDefined();
  });

  it("exports executor module", async () => {
    const { doAction } = await import("@api/agent/executor.js");
    expect(doAction).toBeDefined();
  });

  it("exports finder module", async () => {
    const { find } = await import("@api/agent/finder.js");
    expect(find).toBeDefined();
  });

  it("exports tokenCounter module", async () => {
    const {
      estimateTokens,
      estimateMessageTokens,
      estimateConversationTokens,
    } = await import("@api/agent/tokenCounter.js");
    expect(typeof estimateTokens).toBe("function");
    expect(typeof estimateMessageTokens).toBe("function");
    expect(typeof estimateConversationTokens).toBe("function");
  });

  it("estimates tokens correctly", async () => {
    const { estimateTokens } = await import("@api/agent/tokenCounter.js");
    const text = "Hello world";
    const tokens = estimateTokens(text);
    expect(tokens).toBeGreaterThan(0);
  });

  it("estimates message tokens", async () => {
    const { estimateMessageTokens } =
      await import("@api/agent/tokenCounter.js");
    const message = { role: "user", content: "Hello" };
    const tokens = estimateMessageTokens(message);
    expect(tokens).toBeGreaterThan(0);
  });

  it("exports vision module", async () => {
    const vision = await import("@api/agent/vision.js");
    expect(vision.screenshot).toBeDefined();
    expect(vision.getVPrepPresets).toBeDefined();
  });

  it("gets VPrep presets", async () => {
    const { getVPrepPresets } = await import("@api/agent/vision.js");
    const presets = getVPrepPresets();
    expect(presets).toBeDefined();
  });
});
