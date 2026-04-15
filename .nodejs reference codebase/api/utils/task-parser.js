/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Task CLI Argument Parser
 * Parses CLI arguments into task groups for sequential execution.
 * @module utils/task-parser
 */

import path from "path";
import fs from "fs";
import { fileURLToPath } from "url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

export const TASK_SEPARATOR = "then";
const URL_PROTOCOLS = ["http://", "https://"];
const TASKS_DIR = path.resolve(__dirname, "../../tasks");

/**
 * Get tasks directory path
 * @returns {string} Absolute path to tasks directory
 */
export function getTasksDir() {
  return TASKS_DIR;
}

/**
 * Format URL value - add https:// if missing
 * @param {string} value - URL value to format
 * @returns {string} Formatted URL
 */
export function formatUrl(value) {
  if (!value) return value;

  const trimmed = value.trim();

  if (URL_PROTOCOLS.some((p) => trimmed.startsWith(p))) {
    return trimmed;
  }

  const beforePort = trimmed.split(":")[0];
  if (trimmed.includes(".") || beforePort === "localhost") {
    return "https://" + trimmed;
  }

  return trimmed;
}

/**
 * Check if a task file exists
 * @param {string} name - Task name (with or without .js extension)
 * @returns {boolean} True if task file exists
 */
export function taskExists(name) {
  const taskName = name.endsWith(".js") ? name : `${name}.js`;
  const taskPath = path.join(TASKS_DIR, taskName);

  try {
    return fs.existsSync(taskPath);
  } catch {
    return false;
  }
}

/**
 * Determine if a value is numeric
 * @param {string} value - Value to check
 * @returns {boolean} True if value is purely numeric
 */
function isNumeric(value) {
  return /^\d+$/.test(value);
}

/**
 * Parse CLI args into task groups for sequential execution
 *
 * @param {string[]} args - Raw CLI arguments (e.g., ['follow=x.com', 'then', 'retweet=y.com'])
 * @returns {Object} { groups: [[{ name, payload }]], taskCount: number }
 *
 * @example
 * parseTaskArgs(['follow=x.com', 'follow=y.com', 'then', 'retweet=z.com'])
 * // Returns:
 * // {
 * //   groups: [
 * //     [{ name: 'follow', payload: { url: 'https://x.com' } },
 * //      { name: 'follow', payload: { url: 'https://y.com' } }],
 * //     [{ name: 'retweet', payload: { url: 'https://z.com' } }]
 * //   ],
 * //   taskCount: 3
 * // }
 */
export function parseTaskArgs(args) {
  const groups = [];
  let currentGroup = [];
  let currentTask = null;
  let currentPayload = {};

  if (!args || args.length === 0) {
    return { groups: [], taskCount: 0 };
  }

  for (let i = 0; i < args.length; i++) {
    const arg = args[i];
    const normalized = arg.toLowerCase();

    if (normalized === TASK_SEPARATOR) {
      if (currentGroup.length > 0 || currentTask) {
        if (currentTask) {
          currentGroup.push({ name: currentTask, payload: currentPayload });
          currentTask = null;
          currentPayload = {};
        }
        groups.push(currentGroup);
        currentGroup = [];
      }
      continue;
    }

    const firstEqualIndex = arg.indexOf("=");

    if (firstEqualIndex > 0) {
      const key = arg.substring(0, firstEqualIndex);
      let value = arg.substring(firstEqualIndex + 1);

      if (value.startsWith('"') && value.endsWith('"')) {
        value = value.slice(1, -1);
      }

      const shorthandTaskName = key.endsWith(".js") ? key.slice(0, -3) : key;

      if (!currentTask) {
        const isNumericValue = isNumeric(value);
        currentTask = shorthandTaskName;
        currentPayload = isNumericValue
          ? { value: parseInt(value, 10) }
          : { url: formatUrl(value) };
      } else if (key === currentTask) {
        currentGroup.push({ name: currentTask, payload: currentPayload });
        const isNumericValue = isNumeric(value);
        currentTask = shorthandTaskName;
        currentPayload = isNumericValue
          ? { value: parseInt(value, 10) }
          : { url: formatUrl(value) };
      } else {
        let paramValue = value;
        if (key === "url") {
          paramValue = formatUrl(value);
        }
        currentPayload[key] = paramValue;
      }
    } else {
      if (currentTask) {
        currentGroup.push({ name: currentTask, payload: currentPayload });
      }
      currentTask = arg.endsWith(".js") ? arg.slice(0, -3) : arg;
      currentPayload = {};
    }
  }

  if (currentTask) {
    currentGroup.push({ name: currentTask, payload: currentPayload });
  }

  if (currentGroup.length > 0) {
    groups.push(currentGroup);
  }

  const taskCount = groups.reduce((sum, group) => sum + group.length, 0);

  return { groups, taskCount };
}

/**
 * Format parsed task groups for display
 * @param {Object} parseResult - Result from parseTaskArgs
 * @returns {string} Formatted string for logging
 */
export function formatParseResult(parseResult) {
  const { groups, taskCount } = parseResult;

  if (taskCount === 0) {
    return "No tasks";
  }

  const parts = [];

  for (let i = 0; i < groups.length; i++) {
    const group = groups[i];

    if (groups.length > 1) {
      parts.push(`Group ${i + 1}: ${group.map((t) => t.name).join(", ")}`);
    } else {
      parts.push(group.map((t) => t.name).join(", "));
    }
  }

  return `${taskCount} task(s) [${parts.join(" | ")}]`;
}
