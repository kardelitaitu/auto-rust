/**
 * Auto-AI Framework - Simplified Logger
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 *
 * Simple, performant logger with ANSI colors and file output.
 */

import fs from "fs";
import path from "path";
import { fileURLToPath } from "url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const LOG_DIR = path.join(__dirname, "..", "logs");
const LOG_FILE = path.join(LOG_DIR, "dashboard.log");

// Simple ANSI colors
const COLORS = {
  reset: "\x1b[0m",
  red: "\x1b[31m",
  green: "\x1b[32m",
  yellow: "\x1b[33m",
  blue: "\x1b[34m",
  gray: "\x1b[90m",
};

// Log level colors
const LEVEL_COLORS = {
  debug: COLORS.gray,
  info: COLORS.blue,
  warn: COLORS.yellow,
  error: COLORS.red,
};

// Ensure log directory exists
let logFileInitialized = false;
function initLogFile() {
  if (process.env.NODE_ENV === "test" || process.env.VITEST) return;
  if (!logFileInitialized) {
    try {
      if (!fs.existsSync(LOG_DIR)) {
        fs.mkdirSync(LOG_DIR, { recursive: true });
      }
      logFileInitialized = true;
    } catch (error) {
      // Silently fail - logging to file is optional
    }
  }
}

/**
 * Write log entry to file (sync, but only in production)
 */
function writeToFile(level, module, message, ...args) {
  if (!logFileInitialized) initLogFile();
  if (!logFileInitialized) return;

  try {
    const timestamp = new Date().toISOString();
    const argsStr =
      args.length > 0
        ? " " +
          args
            .map((a) => (typeof a === "object" ? JSON.stringify(a) : String(a)))
            .join(" ")
        : "";
    const line = `[${timestamp}] [${level.toUpperCase()}] [${module}] ${message}${argsStr}\n`;
    fs.appendFileSync(LOG_FILE, line, "utf8");
  } catch (e) {
    // Ignore file write errors
  }
}

/**
 * Create a logger instance for a module.
 * @param {string} moduleName - Name of the module (usually filename)
 * @returns {Object} Logger with debug, info, warn, error methods
 */
export function createLogger(moduleName) {
  const prefix = `[${moduleName}]`;

  const log = (level, message, ...args) => {
    const color = LEVEL_COLORS[level] || COLORS.reset;
    const timestamp = new Date().toLocaleTimeString("en-US", { hour12: false });

    // Console output with color
    const argsStr =
      args.length > 0
        ? " " +
          args
            .map((a) => (typeof a === "object" ? JSON.stringify(a) : String(a)))
            .join(" ")
        : "";

    console.log(
      `${COLORS.gray}${timestamp}${COLORS.reset} ${color}${level.toUpperCase().padEnd(5)}${COLORS.reset} ${COLORS.blue}${prefix}${COLORS.reset} ${message}${argsStr}`,
    );

    // File output (async-ish, non-blocking)
    if (level !== "debug") {
      writeToFile(level, moduleName, message, ...args);
    }
  };

  return {
    debug: (msg, ...args) => log("debug", msg, ...args),
    info: (msg, ...args) => log("info", msg, ...args),
    warn: (msg, ...args) => log("warn", msg, ...args),
    error: (msg, ...args) => log("error", msg, ...args),
  };
}

// Default export for convenience
export default createLogger;
