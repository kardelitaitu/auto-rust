/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, beforeEach, vi } from "vitest";
import Orchestrator from "@api/core/orchestrator.js";

vi.mock("@api/core/sessionManager.js", () => ({
  default: class {
    constructor() {
      this.getAllSessions = vi.fn();
      Object.defineProperty(this, "activeSessionsCount", {
        configurable: true,
        get: vi.fn(() => 0),
      });
    }
  },
}));
vi.mock("@api/core/discovery.js", () => ({ default: class {} }));
vi.mock("@api/core/automator.js", () => ({ default: class {} }));
vi.mock("@api/core/logger.js", () => ({
  loggerContext: {
    run: vi.fn((ctx, fn) => fn()),
    getStore: vi.fn(),
  },
  createLogger: vi.fn(() => ({
    info: vi.fn(),
    debug: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
  })),
}));
vi.mock("@api/utils/configLoader.js", () => ({
  ConfigLoader: class ConfigLoader {},
  default: {
    loadConfig: vi.fn(),
    getValue: vi.fn(),
    getSettings: vi.fn(),
  },
  getSettings: vi.fn().mockResolvedValue({}),
  getTimeoutValue: vi.fn((_path, def) => Promise.resolve(def)),
}));
vi.mock("@api/utils/validator.js", () => ({
  validateTaskExecution: vi.fn(() => ({ isValid: true })),
  validatePayload: vi.fn(() => ({ isValid: true })),
}));

describe("Orchestrator dispatch integration", () => {
  let orchestrator;

  beforeEach(() => {
    vi.clearAllMocks();
    orchestrator = new Orchestrator();
  });

  it("shares tasks when centralized mode is enabled", async () => {
    const sessionA = { id: "A" };
    const sessionB = { id: "B" };

    Object.defineProperty(orchestrator.sessionManager, "activeSessionsCount", {
      configurable: true,
      get: vi.fn(() => 2),
    });
    orchestrator.sessionManager.getAllSessions.mockReturnValue([
      sessionA,
      sessionB,
    ]);

    const tasks = [
      { taskName: "taskA", payload: {} },
      { taskName: "taskB", payload: {} },
    ];
    orchestrator.taskQueue = [...tasks];

    const processSpy = vi
      .spyOn(orchestrator, "processSharedChecklistForSession")
      .mockResolvedValue();
    await orchestrator.processTasks();

    expect(processSpy).toHaveBeenCalledTimes(2);
    const firstTasks = processSpy.mock.calls[0][1];
    const secondTasks = processSpy.mock.calls[1][1];
    expect(firstTasks).not.toBe(secondTasks);
    expect(firstTasks).toEqual(tasks);
    expect(secondTasks).toEqual(tasks);
  });

  it("falls back to centralized assignment when mode is non-centralized", async () => {
    const sessionA = { id: "A" };
    const sessionB = { id: "B" };

    Object.defineProperty(orchestrator.sessionManager, "activeSessionsCount", {
      configurable: true,
      get: vi.fn(() => 2),
    });
    orchestrator.sessionManager.getAllSessions.mockReturnValue([
      sessionA,
      sessionB,
    ]);

    const tasks = [
      { taskName: "taskA", payload: {} },
      { taskName: "taskB", payload: {} },
    ];
    orchestrator.taskQueue = [...tasks];
    orchestrator.taskDispatchMode = "broadcast";

    const processSpy = vi
      .spyOn(orchestrator, "processSharedChecklistForSession")
      .mockResolvedValue();

    await orchestrator.processTasks();

    expect(processSpy).toHaveBeenCalledTimes(2);
    const firstTasks = processSpy.mock.calls[0][1];
    const secondTasks = processSpy.mock.calls[1][1];
    expect(firstTasks).not.toBe(secondTasks);
    expect(firstTasks).toEqual(tasks);
    expect(secondTasks).toEqual(tasks);
  });
});
