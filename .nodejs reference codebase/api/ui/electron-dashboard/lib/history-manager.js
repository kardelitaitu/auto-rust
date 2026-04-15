/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 */

import fs from "fs";
import path from "path";
import { createLogger } from "./logger.js";

const logger = createLogger("history-manager.js");

export class HistoryManager {
  constructor(historyFilePath, maxItems = 40, saveDebounceMs = 5000) {
    this.historyFilePath = historyFilePath;
    this.maxItems = maxItems;
    this.saveDebounceMs = saveDebounceMs;
    this.tasks = [];
    this.pendingSave = false;
    this.saveTimeout = null;
    this.load();
  }

  scheduleSave() {
    if (this.saveTimeout) {
      // Already have a pending save, just mark that we need another
      this.pendingSave = true;
      return;
    }

    this.pendingSave = false;
    this.saveTimeout = setTimeout(() => {
      this.saveTimeout = null;
      // Save once for the current batch
      this.save();
      // If more changes came in during debounce, schedule another
      if (this.pendingSave) {
        this.pendingSave = false;
        this.scheduleSave();
      }
    }, this.saveDebounceMs);
  }

  flushSave() {
    if (this.saveTimeout) {
      clearTimeout(this.saveTimeout);
      this.saveTimeout = null;
    }
    this.save();
  }

  load() {
    try {
      const dataDir = path.dirname(this.historyFilePath);
      if (!fs.existsSync(dataDir)) {
        fs.mkdirSync(dataDir, { recursive: true });
        logger.info(`Created data directory: ${dataDir}`);
        return;
      }

      if (fs.existsSync(this.historyFilePath)) {
        const rawData = fs.readFileSync(this.historyFilePath, "utf8");

        let data;
        try {
          data = JSON.parse(rawData);
        } catch (parseErr) {
          logger.warn(
            `Corrupted history file detected, backing up and resetting: ${parseErr.message}`,
          );
          const backupPath = this.historyFilePath + ".backup." + Date.now();
          fs.renameSync(this.historyFilePath, backupPath);
          this.tasks = [];
          this.completedTasks = 0;
          this.twitterActions = {
            likes: 0,
            retweets: 0,
            replies: 0,
            quotes: 0,
            follows: 0,
            bookmarks: 0,
            total: 0,
          };
          this.apiMetrics = {
            calls: 0,
            failures: 0,
            successRate: 100,
            avgResponseTime: 0,
          };
          this.save();
          return;
        }

        this.tasks = data.tasks || [];
        this.completedTasks = data.completedTasks || 0;
        this.twitterActions = data.twitterActions || {
          likes: 0,
          retweets: 0,
          replies: 0,
          quotes: 0,
          follows: 0,
          bookmarks: 0,
          total: 0,
        };
        this.apiMetrics = data.apiMetrics || {
          calls: 0,
          failures: 0,
          successRate: 100,
          avgResponseTime: 0,
        };
        logger.info(
          `Loaded ${this.tasks.length} tasks and cumulative metrics from ${path.basename(this.historyFilePath)}`,
        );
      }
    } catch (err) {
      logger.error("Failed to load dashboard history:", err);
      this.tasks = [];
      this.completedTasks = 0;
      this.twitterActions = {
        likes: 0,
        retweets: 0,
        replies: 0,
        quotes: 0,
        follows: 0,
        bookmarks: 0,
        total: 0,
      };
      this.apiMetrics = {
        calls: 0,
        failures: 0,
        successRate: 100,
        avgResponseTime: 0,
      };
    }
  }

  save() {
    try {
      const dataDir = path.dirname(this.historyFilePath);
      if (!fs.existsSync(dataDir)) fs.mkdirSync(dataDir, { recursive: true });

      // Maintain limit
      if (this.tasks.length > this.maxItems) {
        this.tasks = this.tasks.slice(-this.maxItems);
      }

      fs.writeFileSync(
        this.historyFilePath,
        JSON.stringify(
          {
            tasks: this.tasks,
            completedTasks: this.completedTasks || 0,
            twitterActions: this.twitterActions,
            apiMetrics: this.apiMetrics,
            updatedAt: Date.now(),
          },
          null,
          2,
        ),
      );
    } catch (err) {
      logger.error("Failed to save dashboard history:", err);
    }
  }

  /**
   * Merges or adds a task to the history.
   * @param {Object} task - The task object from Orchestrator.
   * @returns {Object[]} The updated tasks array.
   */
  addOrUpdateTask(task) {
    if (!task || (!task.id && !task.timestamp)) {
      return this.tasks;
    }

    // Field Normalization
    const tName = task.taskName || task.name || task.command || "task";
    const sessionId = task.sessionId || task.session || "anon";
    const timestamp = task.timestamp || Date.now();
    const taskId = task.id || `${sessionId}-${tName}-${timestamp}`;

    const normalizedTask = {
      ...task,
      taskName: tName,
      name: tName,
      command: tName,
      sessionId: sessionId,
      id: taskId,
      timestamp: timestamp,
    };

    // Find existing task
    const existingIndex = this.tasks.findIndex(
      (t) =>
        t.id === taskId ||
        (t.timestamp === timestamp && t.sessionId === sessionId),
    );

    if (existingIndex === -1) {
      // New task
      this.tasks.push({
        ...normalizedTask,
        addedAt: Date.now(),
      });

      // Enforce limit
      if (this.tasks.length > this.maxItems) {
        this.tasks = this.tasks.slice(-this.maxItems);
      }
    } else {
      // Update existing
      this.tasks[existingIndex] = {
        ...this.tasks[existingIndex],
        ...normalizedTask,
        updatedAt: Date.now(),
      };
    }

    this.scheduleSave();
    return this.tasks;
  }

  getTasks() {
    return this.tasks;
  }

  getCompletedTasksCount() {
    if (!this.tasks || this.tasks.length === 0) return 0;
    return this.tasks.filter((t) => t.success === true).length;
  }

  setCompletedTasksCount(count) {
    this.completedTasks = count;
    this.save();
  }

  getTwitterActions() {
    return (
      this.twitterActions || {
        likes: 0,
        retweets: 0,
        replies: 0,
        quotes: 0,
        follows: 0,
        bookmarks: 0,
        total: 0,
      }
    );
  }

  setTwitterActions(actions) {
    this.twitterActions = actions;
    this.scheduleSave();
  }

  getApiMetrics() {
    return (
      this.apiMetrics || {
        calls: 0,
        failures: 0,
        successRate: 100,
        avgResponseTime: 0,
      }
    );
  }

  setApiMetrics(metrics) {
    this.apiMetrics = metrics;
    this.scheduleSave();
  }

  clearHistory() {
    try {
      if (this.saveTimeout) {
        clearTimeout(this.saveTimeout);
        this.saveTimeout = null;
      }
      if (fs.existsSync(this.historyFilePath)) {
        fs.unlinkSync(this.historyFilePath);
        logger.info("Dashboard history file deleted");
      }
      this.tasks = [];
      this.completedTasks = 0;
      this.twitterActions = {
        likes: 0,
        retweets: 0,
        replies: 0,
        quotes: 0,
        follows: 0,
        bookmarks: 0,
        total: 0,
      };
      this.apiMetrics = {
        calls: 0,
        failures: 0,
        successRate: 100,
        avgResponseTime: 0,
      };
      this.save();
      logger.info("Dashboard history cleared");
    } catch (err) {
      logger.error("Failed to clear history:", err);
    }
  }
}
