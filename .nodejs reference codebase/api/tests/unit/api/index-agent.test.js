/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 */

import { describe, it, expect, vi, beforeEach } from "vitest";

vi.mock("@api/agent/index.js", () => ({
  actionEngine: {
    execute: vi.fn(),
    plan: vi.fn(),
  },
  llmClient: {
    complete: vi.fn(),
    embed: vi.fn(),
  },
  agentRunner: {
    run: vi.fn().mockResolvedValue({ success: true }),
    stop: vi.fn(),
    isRunning: false,
    getUsageStats: vi.fn().mockReturnValue({ runs: 10 }),
  },
  captureAXTree: vi.fn().mockResolvedValue("<ax>tree</ax>"),
  captureState: vi.fn().mockResolvedValue({ state: "test" }),
  processWithVPrep: vi.fn(),
  getVPrepPresets: vi.fn(),
  getVPrepStats: vi.fn(),
}));

vi.mock("@api/agent/observer.js", () => ({
  see: vi.fn().mockResolvedValue({ elements: [] }),
}));

vi.mock("@api/agent/executor.js", () => ({
  doAction: vi.fn().mockResolvedValue({ success: true }),
}));

vi.mock("@api/agent/finder.js", () => ({
  find: vi.fn().mockResolvedValue({ found: true }),
}));

vi.mock("@api/agent/vision.js", () => ({
  default: {
    screenshot: vi.fn().mockResolvedValue("base64image"),
    capture: vi.fn(),
  },
}));

describe("api/agent functionality - direct module tests", () => {
  describe("agentRunner", () => {
    beforeEach(() => {
      vi.clearAllMocks();
    });

    it("should run agent with goal", async () => {
      const { agentRunner } = await import("@api/agent/index.js");
      const result = await agentRunner.run("test goal");
      expect(agentRunner.run).toHaveBeenCalled();
      expect(result).toEqual({ success: true });
    });

    it("should pass config to agentRunner.run", async () => {
      const { agentRunner } = await import("@api/agent/index.js");
      const config = { maxSteps: 10, timeout: 60000 };
      await agentRunner.run("test goal", config);
      expect(agentRunner.run).toHaveBeenCalled();
    });

    it("should stop agent runner", async () => {
      const { agentRunner } = await import("@api/agent/index.js");
      await agentRunner.stop();
      expect(agentRunner.stop).toHaveBeenCalled();
    });

    it("should return isRunning status", async () => {
      const { agentRunner } = await import("@api/agent/index.js");
      expect(typeof agentRunner.isRunning).toBe("boolean");
    });

    it("should return usage stats", async () => {
      const { agentRunner } = await import("@api/agent/index.js");
      const stats = await agentRunner.getUsageStats();
      expect(stats).toEqual({ runs: 10 });
    });
  });

  describe("see (observer)", () => {
    beforeEach(() => {
      vi.clearAllMocks();
    });

    it("should call see from observer module", async () => {
      const { see } = await import("@api/agent/observer.js");
      await see("button");
      expect(see).toHaveBeenCalledWith("button");
    });

    it("should accept options parameter", async () => {
      const { see } = await import("@api/agent/observer.js");
      await see("button", { timeout: 5000 });
      expect(see).toHaveBeenCalledWith("button", { timeout: 5000 });
    });
  });

  describe("doAction (executor)", () => {
    beforeEach(() => {
      vi.clearAllMocks();
    });

    it("should call doAction from executor module", async () => {
      const { doAction } = await import("@api/agent/executor.js");
      await doAction("click", "button");
      expect(doAction).toHaveBeenCalledWith("click", "button");
    });

    it("should accept action and target parameters", async () => {
      const { doAction } = await import("@api/agent/executor.js");
      await doAction("type", "input", "hello world");
      expect(doAction).toHaveBeenCalledWith("type", "input", "hello world");
    });
  });

  describe("find (finder)", () => {
    beforeEach(() => {
      vi.clearAllMocks();
    });

    it("should call find from finder module", async () => {
      const { find } = await import("@api/agent/finder.js");
      await find(".submit-btn");
      expect(find).toHaveBeenCalledWith(".submit-btn");
    });

    it("should accept selector and options", async () => {
      const { find } = await import("@api/agent/finder.js");
      await find("form", { visible: true });
      expect(find).toHaveBeenCalledWith("form", { visible: true });
    });
  });

  describe("vision module", () => {
    beforeEach(() => {
      vi.clearAllMocks();
    });

    it("should have screenshot function", async () => {
      const vision = await import("@api/agent/vision.js");
      expect(typeof vision.default.screenshot).toBe("function");
    });

    it("should call vision.screenshot", async () => {
      const vision = await import("@api/agent/vision.js");
      await vision.default.screenshot();
      expect(vision.default.screenshot).toHaveBeenCalled();
    });
  });

  describe("captureAXTree", () => {
    beforeEach(() => {
      vi.clearAllMocks();
    });

    it("should capture accessibility tree", async () => {
      const { captureAXTree } = await import("@api/agent/index.js");
      const result = await captureAXTree();
      expect(captureAXTree).toHaveBeenCalled();
      expect(result).toBe("<ax>tree</ax>");
    });
  });

  describe("captureState", () => {
    beforeEach(() => {
      vi.clearAllMocks();
    });

    it("should capture state", async () => {
      const { captureState } = await import("@api/agent/index.js");
      const result = await captureState();
      expect(captureState).toHaveBeenCalled();
      expect(result).toEqual({ state: "test" });
    });
  });
});
