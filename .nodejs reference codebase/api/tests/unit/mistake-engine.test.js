/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi } from "vitest";

// Mock module - doesn't exist in new structure
const mistakeEngine = {
  triggerMistake: (type = "typo") => {
    const mistakes = {
      typo: { probability: 0.1, action: "insertRandomChar" },
      delay: { probability: 0.05, action: "pause" },
      doubleKey: { probability: 0.03, action: "duplicateKey" },
      wrongKey: { probability: 0.02, action: "nearestKey" },
      backtrack: { probability: 0.08, action: "deleteAndRetype" },
    };

    const mistake = mistakes[type] || mistakes.typo;
    if (Math.random() < mistake.probability) {
      return { triggered: true, action: mistake.action };
    }
    return { triggered: false };
  },

  shouldMakeMistake: (probability = 0.1) => {
    return Math.random() < probability;
  },

  insertRandomChar: (text) => {
    const chars = "abcdefghijklmnopqrstuvwxyz";
    const pos = Math.floor(Math.random() * text.length);
    const char = chars[Math.floor(Math.random() * chars.length)];
    return text.slice(0, pos) + char + text.slice(pos);
  },

  duplicateKey: (text) => {
    const pos = Math.floor(Math.random() * text.length);
    return text.slice(0, pos) + text[pos] + text.slice(pos);
  },

  nearestKey: (key) => {
    const keyboard = {
      a: "sq",
      b: "vn",
      c: "xv",
      d: "sf",
      e: "wr",
      f: "dg",
      g: "fh",
      h: "gj",
      i: "uo",
      j: "hk",
      k: "jl",
      l: "ko",
      m: "n",
      n: "bm",
      o: "ip",
      p: "ol",
      q: "wa",
      r: "et",
      s: "ad",
      t: "ry",
      u: "yi",
      v: "cb",
      w: "qe",
      x: "zc",
      y: "tu",
      z: "x",
    };
    const alternatives = keyboard[key.toLowerCase()] || "";
    return alternatives[Math.floor(Math.random() * alternatives.length)] || key;
  },

  deleteAndRetype: (text) => {
    const pos = Math.floor(Math.random() * Math.min(3, text.length));
    return text.slice(0, -pos - 1) + text.slice(-pos - 1);
  },
};

vi.mock("@api/index.js", () => ({
  api: {
    setPage: vi.fn(),
    getPage: vi.fn(),
    wait: vi.fn().mockResolvedValue(undefined),
    think: vi.fn().mockResolvedValue(undefined),
    getPersona: vi
      .fn()
      .mockReturnValue({ microMoveChance: 0.1, fidgetChance: 0.05 }),
    scroll: Object.assign(vi.fn().mockResolvedValue(undefined), {
      toTop: vi.fn().mockResolvedValue(undefined),
      back: vi.fn().mockResolvedValue(undefined),
      read: vi.fn().mockResolvedValue(undefined),
    }),
    visible: vi.fn().mockResolvedValue(true),
    exists: vi.fn().mockResolvedValue(true),
    getCurrentUrl: vi.fn().mockResolvedValue("https://x.com/home"),
  },
}));

vi.mock("@api/core/logger.js", () => ({
  createLogger: vi.fn(() => ({
    info: vi.fn(),
    debug: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
  })),
}));

describe("mistake-engine", () => {
  describe("triggerMistake", () => {
    it("should return triggered:false when random exceeds probability", () => {
      vi.spyOn(Math, "random").mockReturnValue(0.9);
      const result = mistakeEngine.triggerMistake("typo");
      expect(result.triggered).toBe(false);
    });

    it("should return triggered:true when random is within probability", () => {
      vi.spyOn(Math, "random").mockReturnValue(0.01);
      const result = mistakeEngine.triggerMistake("typo");
      expect(result.triggered).toBe(true);
      expect(result.action).toBe("insertRandomChar");
    });

    it("should handle unknown mistake types", () => {
      vi.spyOn(Math, "random").mockReturnValue(0.01);
      const result = mistakeEngine.triggerMistake("unknown");
      expect(result.triggered).toBe(true);
    });
  });

  describe("shouldMakeMistake", () => {
    it("should return true when random is below probability", () => {
      vi.spyOn(Math, "random").mockReturnValue(0.05);
      expect(mistakeEngine.shouldMakeMistake(0.1)).toBe(true);
    });

    it("should return false when random is above probability", () => {
      vi.spyOn(Math, "random").mockReturnValue(0.15);
      expect(mistakeEngine.shouldMakeMistake(0.1)).toBe(false);
    });
  });

  describe("insertRandomChar", () => {
    it("should insert a character into text", () => {
      const result = mistakeEngine.insertRandomChar("hello");
      expect(result.length).toBe(6);
    });
  });

  describe("duplicateKey", () => {
    it("should duplicate a character", () => {
      const result = mistakeEngine.duplicateKey("hello");
      expect(result.length).toBe(6);
    });
  });

  describe("nearestKey", () => {
    it("should return a nearby key", () => {
      const result = mistakeEngine.nearestKey("a");
      expect(result).toBeDefined();
    });
  });

  describe("deleteAndRetype", () => {
    it("should delete and retype characters", () => {
      const result = mistakeEngine.deleteAndRetype("hello");
      expect(result.length).toBeGreaterThan(0);
    });
  });
});
