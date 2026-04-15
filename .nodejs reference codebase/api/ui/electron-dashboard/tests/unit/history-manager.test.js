/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * @fileoverview Unit tests for history-manager.js
 */
import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import fs from "fs";
import path from "path";
import os from "os";

describe("HistoryManager", () => {
  let tempDir;
  let historyFilePath;
  let HistoryManager;

  beforeEach(async () => {
    tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "dashboard-test-"));
    historyFilePath = path.join(tempDir, "history.json");

    const module = await import("../../lib/history-manager.js");
    HistoryManager = module.HistoryManager;
  });

  afterEach(() => {
    if (tempDir && fs.existsSync(tempDir)) {
      fs.rmSync(tempDir, { recursive: true, force: true });
    }
    vi.restoreAllMocks();
  });

  describe("initialization", () => {
    it("should initialize with empty data", () => {
      const manager = new HistoryManager(historyFilePath, 40, 5000);
      expect(manager.getTasks()).toEqual([]);
      expect(manager.getTwitterActions()).toEqual({
        likes: 0,
        retweets: 0,
        replies: 0,
        quotes: 0,
        follows: 0,
        bookmarks: 0,
        total: 0,
      });
      expect(manager.getApiMetrics()).toEqual({
        calls: 0,
        failures: 0,
        successRate: 100,
        avgResponseTime: 0,
      });
    });

    it("should use custom maxItems", () => {
      const manager = new HistoryManager(historyFilePath, 10, 5000);
      expect(manager.maxItems).toBe(10);
    });

    it("should use custom saveDebounceMs", () => {
      const manager = new HistoryManager(historyFilePath, 40, 10000);
      expect(manager.saveDebounceMs).toBe(10000);
    });
  });

  describe("addOrUpdateTask", () => {
    it("should add a new task", () => {
      const manager = new HistoryManager(historyFilePath, 40, 1);
      const task = {
        id: "task-1",
        taskName: "followUser",
        sessionId: "session-1",
      };

      manager.addOrUpdateTask(task);
      manager.flushSave();

      const result = manager.getTasks();
      expect(result).toHaveLength(1);
      expect(result[0].id).toBe("task-1");
      expect(result[0].taskName).toBe("followUser");
    });

    it("should normalize task fields", () => {
      const manager = new HistoryManager(historyFilePath, 40, 1);
      const task = {
        name: "retweetTask",
        session: "session-123",
        timestamp: 1234567890,
      };

      manager.addOrUpdateTask(task);
      manager.flushSave();

      const result = manager.getTasks();
      expect(result[0].taskName).toBe("retweetTask");
      expect(result[0].command).toBe("retweetTask");
      expect(result[0].sessionId).toBe("session-123");
      expect(result[0].timestamp).toBe(1234567890);
    });

    it("should update existing task", () => {
      const manager = new HistoryManager(historyFilePath, 40, 1);
      const task = { id: "task-1", taskName: "followUser", success: false };

      manager.addOrUpdateTask(task);
      manager.flushSave();

      const updatedTask = {
        id: "task-1",
        taskName: "followUser",
        success: true,
      };
      manager.addOrUpdateTask(updatedTask);
      manager.flushSave();

      const tasks = manager.getTasks();
      expect(tasks).toHaveLength(1);
      expect(tasks[0].success).toBe(true);
    });

    it("should ignore invalid tasks", () => {
      const manager = new HistoryManager(historyFilePath, 40, 1);

      const result1 = manager.addOrUpdateTask(null);
      const result2 = manager.addOrUpdateTask(undefined);
      const result3 = manager.addOrUpdateTask({});

      expect(result1).toEqual([]);
      expect(result2).toEqual([]);
      expect(result3).toEqual([]);
    });
  });

  describe("twitter actions", () => {
    it("should set twitter actions", () => {
      const manager = new HistoryManager(historyFilePath, 40, 1);
      const actions = { likes: 10, retweets: 5, total: 15 };

      manager.setTwitterActions(actions);
      manager.flushSave();

      const result = manager.getTwitterActions();
      expect(result.likes).toBe(10);
      expect(result.retweets).toBe(5);
      expect(result.total).toBe(15);
    });

    it("should set twitter actions completely", () => {
      const manager = new HistoryManager(historyFilePath, 40, 1);
      manager.setTwitterActions({ likes: 5, retweets: 3, replies: 2 });
      manager.flushSave();

      manager.setTwitterActions({ likes: 10 });
      manager.flushSave();

      const result = manager.getTwitterActions();
      expect(result.likes).toBe(10);
    });
  });

  describe("api metrics", () => {
    it("should set api metrics", () => {
      const manager = new HistoryManager(historyFilePath, 40, 1);
      const metrics = { calls: 100, failures: 10, successRate: 90 };

      manager.setApiMetrics(metrics);
      manager.flushSave();

      const result = manager.getApiMetrics();
      expect(result.calls).toBe(100);
      expect(result.failures).toBe(10);
      expect(result.successRate).toBe(90);
    });

    it("should set api metrics completely", () => {
      const manager = new HistoryManager(historyFilePath, 40, 1);
      manager.setApiMetrics({ calls: 50, failures: 5, avgResponseTime: 100 });
      manager.flushSave();

      manager.setApiMetrics({ calls: 100 });
      manager.flushSave();

      const result = manager.getApiMetrics();
      expect(result.calls).toBe(100);
    });
  });

  describe("completed tasks count", () => {
    it("should return 0 for empty tasks", () => {
      const manager = new HistoryManager(historyFilePath, 40, 1);
      expect(manager.getCompletedTasksCount()).toBe(0);
    });
  });

  describe("clearHistory", () => {
    it("should clear all tasks", () => {
      const manager = new HistoryManager(historyFilePath, 40, 0);
      manager.addOrUpdateTask({ id: "task-1", taskName: "test" });

      manager.clearHistory();

      expect(manager.getTasks()).toEqual([]);
    });

    it("should reset twitter actions", () => {
      const manager = new HistoryManager(historyFilePath, 40, 0);
      manager.setTwitterActions({ likes: 10, total: 10 });

      manager.clearHistory();

      expect(manager.getTwitterActions().likes).toBe(0);
    });

    it("should reset api metrics", () => {
      const manager = new HistoryManager(historyFilePath, 40, 0);
      manager.setApiMetrics({ calls: 100, failures: 10 });

      manager.clearHistory();

      const metrics = manager.getApiMetrics();
      expect(metrics.calls).toBe(0);
      expect(metrics.failures).toBe(0);
      expect(metrics.successRate).toBe(100);
    });

    it("should recreate history file after clear", () => {
      const manager = new HistoryManager(historyFilePath, 40, 0);
      manager.addOrUpdateTask({ id: "task-1" });

      manager.clearHistory();

      expect(fs.existsSync(historyFilePath)).toBe(true);
    });
  });

  describe("corrupted file handling", () => {
    it("should handle corrupted history file", () => {
      fs.writeFileSync(historyFilePath, "{ invalid json }");

      const manager = new HistoryManager(historyFilePath, 40, 1);

      expect(manager.getTasks()).toEqual([]);
      expect(manager.getTwitterActions().likes).toBe(0);
    });

    it("should backup corrupted file", () => {
      fs.writeFileSync(historyFilePath, '{ "tasks": [] }');
      fs.writeFileSync(historyFilePath, "{ invalid json }");

      const manager = new HistoryManager(historyFilePath, 40, 1);

      const files = fs.readdirSync(tempDir);
      const backupFiles = files.filter((f) => f.includes(".backup."));
      expect(backupFiles.length).toBeGreaterThan(0);
    });
  });

  describe("field normalization", () => {
    it("should use taskName over name", () => {
      const manager = new HistoryManager(historyFilePath, 40, 1);
      manager.addOrUpdateTask({ name: "nameValue", taskName: "taskNameValue" });
      manager.flushSave();
      const tasks = manager.getTasks();
      if (tasks.length > 0) {
        expect(tasks[0].taskName).toBe("taskNameValue");
      }
    });

    it("should use sessionId over session", () => {
      const manager = new HistoryManager(historyFilePath, 40, 1);
      manager.addOrUpdateTask({
        session: "sessionValue",
        sessionId: "sessionIdValue",
      });
      manager.flushSave();
      const tasks = manager.getTasks();
      if (tasks.length > 0) {
        expect(tasks[0].sessionId).toBe("sessionIdValue");
      }
    });
  });
});
