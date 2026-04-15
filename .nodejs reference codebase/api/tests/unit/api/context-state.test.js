/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import {
  setContextStore,
  getContextState,
  setContextState,
  getStateSection,
  updateStateSection,
  setStatePersona,
  getStatePersona,
  getStatePersonaName,
  setStatePathStyle,
  getStatePathStyle,
  getStatePathOptions,
  setStateDistractionChance,
  getStateDistractionChance,
  getStateAttentionMemory,
  recordStateAttentionMemory,
  getStateIdle,
  setStateIdle,
  getPreviousUrl,
  setPreviousUrl,
  getStateAgentElementMap,
  setStateAgentElementMap,
  getAutoBanners,
  setAutoBanners,
  getMuteAudio,
  setMuteAudio,
} from "@api/core/context-state.js";

// Mock api/behaviors/persona.js
vi.mock("@api/tests/behaviors/persona.js", () => ({
  PERSONAS: {
    casual: { muscleModel: { Kp: 1, Ki: 1, Kd: 1 } },
    focused: { muscleModel: { Kp: 2, Ki: 2, Kd: 2 } },
  },
}));

describe("api/core/context-state.js", () => {
  let mockStore;

  beforeEach(() => {
    mockStore = {
      state: null,
      getStore: () => mockStore,
    };
    setContextStore(mockStore);
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  describe("Context State Management", () => {
    it("should return default state when store is empty", () => {
      const state = getContextState();
      expect(state).toBeDefined();
      expect(state.persona.name).toBe("casual");
      expect(state.pathStyle.style).toBe("bezier");
    });

    it("should set and get state from store", () => {
      const newState = { test: true };
      setContextState(newState);
      expect(mockStore.state).toEqual(newState);
      expect(getContextState()).toEqual(newState);
    });

    it("should get specific section", () => {
      const section = getStateSection("persona");
      expect(section.name).toBe("casual");
    });

    it("should update specific section", () => {
      updateStateSection("persona", { name: "updated" });
      const section = getStateSection("persona");
      expect(section.name).toBe("updated");
      // Ensure other properties are preserved (if they existed in default)
      expect(section.sessionStartTime).toBeDefined();
    });
  });

  describe("Persona State", () => {
    it("should set valid persona", () => {
      setStatePersona("focused");
      expect(getStatePersona()).toBeDefined();
      const state = getContextState();
      expect(state.persona.name).toBe("focused");
    });

    it("should throw on invalid persona", () => {
      expect(() => setStatePersona("invalid")).toThrow("Unknown persona");
    });

    it("should get persona name", () => {
      setStatePersona("focused");
      expect(getStatePersonaName()).toBe("focused");
    });

    it("should apply biometric randomization", () => {
      // Mock Math.random to return a fixed value that results in non-zero drift
      const originalRandom = Math.random;
      Math.random = () => 0.1;

      setStatePersona("casual");
      const config = getStatePersona();

      // Original casual has Kp: 1
      // With random 0.1:
      // drift = 0.1
      // factor = 1 + (0.1 * 0.2 - 0.1) = 1 + (0.02 - 0.1) = 1 - 0.08 = 0.92
      expect(config.muscleModel.Kp).not.toBe(1);

      Math.random = originalRandom;
    });
  });

  describe("Path Style State", () => {
    it("should set valid path style", () => {
      setStatePathStyle("zigzag", { speed: 10 });
      expect(getStatePathStyle()).toBe("zigzag");
      const state = getContextState();
      expect(state.pathStyle.options).toEqual({ speed: 10 });
    });

    it("should get path options", () => {
      setStatePathStyle("overshoot", { tension: 0.5 });
      expect(getStatePathOptions()).toEqual({ tension: 0.5 });
    });

    it("should throw on invalid path style", () => {
      expect(() => setStatePathStyle("invalid")).toThrow("Invalid path style");
    });
  });

  describe("Attention State", () => {
    it("should set distraction chance", () => {
      setStateDistractionChance(0.5);
      expect(getStateDistractionChance()).toBe(0.5);
    });

    it("should clamp distraction chance", () => {
      setStateDistractionChance(1.5);
      expect(getStateDistractionChance()).toBe(1);

      setStateDistractionChance(-0.5);
      expect(getStateDistractionChance()).toBe(0);
    });

    it("should manage attention memory", () => {
      const memory = getStateAttentionMemory();
      expect(Array.isArray(memory)).toBe(true);

      recordStateAttentionMemory(".btn-1");
      recordStateAttentionMemory(".btn-2");
      recordStateAttentionMemory(".btn-3");

      let current = getStateAttentionMemory();
      expect(current).toEqual([".btn-3", ".btn-2", ".btn-1"]);

      // Should move existing to front
      recordStateAttentionMemory(".btn-1");
      current = getStateAttentionMemory();
      expect(current).toEqual([".btn-1", ".btn-3", ".btn-2"]);

      // Should cap at 3
      recordStateAttentionMemory(".btn-4");
      current = getStateAttentionMemory();
      expect(current).toEqual([".btn-4", ".btn-1", ".btn-3"]);
      expect(current.length).toBe(3);
    });

    it("should handle invalid memory recording", () => {
      const initial = [...getStateAttentionMemory()];
      recordStateAttentionMemory(null);
      recordStateAttentionMemory(undefined);
      recordStateAttentionMemory("");
      expect(getStateAttentionMemory()).toEqual(initial);
    });
  });

  describe("Idle State", () => {
    it("should set and get idle state", () => {
      const idleState = { isRunning: true, fidgetInterval: 1000 };
      setStateIdle(idleState);
      expect(getStateIdle()).toEqual(idleState);
    });
  });

  describe("Session State", () => {
    it("should set and get previous URL", () => {
      const url = "https://example.com";
      setPreviousUrl(url);
      expect(getPreviousUrl()).toBe(url);
    });
  });

  describe("Agent State", () => {
    it("should set and get agent element map", () => {
      const elementMap = [{ selector: ".item", text: "One" }];
      setStateAgentElementMap(elementMap);
      expect(getStateAgentElementMap()).toEqual(elementMap);
    });

    it("should preserve an empty agent element map", () => {
      setStateAgentElementMap([]);
      expect(getStateAgentElementMap()).toEqual([]);
    });
  });

  describe("Automation State", () => {
    it("should set and get auto banner state", () => {
      setAutoBanners(true);
      expect(getAutoBanners()).toBe(true);

      setAutoBanners(false);
      expect(getAutoBanners()).toBe(false);
    });
  });

  describe("Audio State", () => {
    it("should set and get mute audio state", () => {
      setMuteAudio(true);
      expect(getMuteAudio()).toBe(true);

      setMuteAudio(0);
      expect(getMuteAudio()).toBe(false);
    });
  });

  describe("Branch Coverage", () => {
    it("should handle setContextState when store is missing", () => {
      setContextStore(null);
      expect(() => setContextState({ test: true })).not.toThrow();
    });

    it("should use default state in getStateSection when section is missing in current state", () => {
      setContextStore(mockStore);
      setContextState({ someOtherSection: {} }); // state exists but requested section missing
      const section = getStateSection("persona");
      expect(section).toBeDefined();
      expect(section.name).toBe("casual");
    });

    it("should fallback to casual in setStatePersona when name is custom and no overrides", () => {
      vi.spyOn(Math, "random").mockReturnValue(0.5); // no drift: 1 + (0.5 * 0.2 - 0.1) = 1 + 0 = 1
      setContextStore(mockStore);
      setStatePersona("custom");
      expect(getStatePersonaName()).toBe("custom");
      expect(getStatePersona().muscleModel.Kp).toBe(0.1); // from casual (Kp: 0.1)
      vi.restoreAllMocks();
    });
  });
});
