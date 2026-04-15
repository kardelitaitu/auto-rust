/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview System-wide event logging facility with rich ANSI colors.
 * @module utils/logger
 */

import fs from "fs";
import path from "path";
import { fileURLToPath } from "url";
import { EventEmitter } from "events";
import { AsyncLocalStorage } from "async_hooks";
import { randomUUID as _randomUUID } from "crypto";

export const loggerContext = new AsyncLocalStorage();

export function runWithContext(context, fn) {
  return loggerContext.run(context, fn);
}

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const LOG_FILE = path.join(__dirname, "../../logs.txt");
const LOG_FILE_JSON = path.join(__dirname, "../../logs.json");
export const logEmitter = new EventEmitter();

// Rich ANSI Color Codes (Neon/Bright variants)
const COLORS = {
  RESET: "\x1b[0m",
  BRIGHT: "\x1b[1m",
  DIM: "\x1b[2m",
  UNDERSCORE: "\x1b[4m",
  BLINK: "\x1b[5m",
  REVERSE: "\x1b[7m",
  HIDDEN: "\x1b[8m",

  // Foreground Standard
  FG_BLACK: "\x1b[30m",
  FG_RED: "\x1b[31m",
  FG_GREEN: "\x1b[32m",
  FG_YELLOW: "\x1b[33m",
  FG_BLUE: "\x1b[34m",
  FG_MAGENTA: "\x1b[35m",
  FG_CYAN: "\x1b[36m",
  FG_WHITE: "\x1b[37m",
  FG_GRAY: "\x1b[90m",

  // Foreground Bright (Neon)
  FG_BRIGHT_RED: "\x1b[91m",
  FG_BRIGHT_GREEN: "\x1b[92m",
  FG_BRIGHT_YELLOW: "\x1b[93m",
  FG_BRIGHT_BLUE: "\x1b[94m",
  FG_BRIGHT_MAGENTA: "\x1b[95m",
  FG_BRIGHT_CYAN: "\x1b[96m",
  FG_BRIGHT_WHITE: "\x1b[97m",

  // Foreground Extended (256-color)
  FG_ORANGE: "\x1b[38;5;208m",
  FG_PINK: "\x1b[38;5;205m",
  FG_PURPLE: "\x1b[38;5;129m",
  FG_TEAL: "\x1b[38;5;37m",
  FG_NAVY: "\x1b[38;5;17m",
  FG_GOLD: "\x1b[38;5;220m",
  FG_LIME: "\x1b[38;5;118m",

  // Background Standard
  BG_BLACK: "\x1b[40m",
  BG_RED: "\x1b[41m",
  BG_GREEN: "\x1b[42m",
  BG_YELLOW: "\x1b[43m",
  BG_BLUE: "\x1b[44m",
  BG_MAGENTA: "\x1b[45m",
  BG_CYAN: "\x1b[46m",
  BG_WHITE: "\x1b[47m",

  // Background Bright
  BG_GRAY: "\x1b[100m",
  BG_BRIGHT_RED: "\x1b[101m",
  BG_BRIGHT_GREEN: "\x1b[102m",
  BG_BRIGHT_YELLOW: "\x1b[103m",
  BG_BRIGHT_BLUE: "\x1b[104m",
  BG_BRIGHT_MAGENTA: "\x1b[105m",
  BG_BRIGHT_CYAN: "\x1b[106m",
  BG_BRIGHT_WHITE: "\x1b[107m",

  // Background Extended (256-color)
  BG_ORANGE: "\x1b[48;5;208m",
  BG_PINK: "\x1b[48;5;205m",
  BG_PURPLE: "\x1b[48;5;129m",
  BG_TEAL: "\x1b[48;5;37m",
  BG_NAVY: "\x1b[48;5;17m",
  BG_GOLD: "\x1b[48;5;220m",
};

// Global flag to track if log file has been initialized
let logFileInitialized = false;

// Buffered async logging - improves performance by batching writes
const LOG_BUFFER = [];
const FLUSH_INTERVAL = 1000; // Flush every 1 second
let flushTimer = null;

// Flush buffer on process exit to ensure no logs are lost
// Increase max listeners to avoid warning in test environments with multiple imports
process.setMaxListeners?.(Infinity);

process.on("exit", () => {
  if (LOG_BUFFER.length > 0) {
    const entries = LOG_BUFFER.splice(0, LOG_BUFFER.length);
    const textData =
      entries
        .map((e) => {
          const time = new Date(e.timestamp).toLocaleTimeString("en-US", {
            hour12: false,
          });
          return `[${time}] [${e.level}] [${e.module}] ${e.message}`;
        })
        .join("\n") + "\n";
    const jsonData = entries.map((e) => JSON.stringify(e)).join("\n") + "\n";

    fs.appendFileSync(LOG_FILE, textData, "utf8");
    fs.appendFileSync(LOG_FILE_JSON, jsonData, "utf8");
  }
});

// Pre-compile ANSI regex once for performance
const ANSI_REGEX = new RegExp(`${String.fromCharCode(27)}\\[[0-9;]*m`, "g");

/**
 * Initialize the log file (clear it)
 */
function initLogFile() {
  if (process.env.NODE_ENV === "test" || process.env.VITEST) return;
  if (!logFileInitialized) {
    try {
      fs.writeFileSync(LOG_FILE, "", "utf8");
      fs.writeFileSync(LOG_FILE_JSON, "", "utf8");
      logFileInitialized = true;
    } catch (error) {
      console.error(`Failed to initialize log files: ${error.message}`);
    }
  }
}

/**
 * Flush buffered logs to file (async)
 */
function flushLogBuffer() {
  if (process.env.NODE_ENV === "test" || process.env.VITEST) return;
  if (LOG_BUFFER.length === 0) return;

  const entries = LOG_BUFFER.splice(0, LOG_BUFFER.length);
  const textData =
    entries
      .map((e) => {
        const time = new Date(e.timestamp).toLocaleTimeString("en-US", {
          hour12: false,
        });
        return `[${time}] [${e.level}] [${e.module}] ${e.message}`;
      })
      .join("\n") + "\n";
  const jsonData = entries.map((e) => JSON.stringify(e)).join("\n") + "\n";

  fs.appendFile(LOG_FILE, textData, "utf8", (err) => {
    if (err) console.error("Failed to write log.txt buffer:", err.message);
  });

  fs.appendFile(LOG_FILE_JSON, jsonData, "utf8", (err) => {
    if (err) console.error("Failed to write log.json buffer:", err.message);
  });
}

/**
 * Write to log file (strips ANSI codes for clean text logs)
 * Uses buffered async writes for better performance
 */
function writeToLogFile(level, scriptName, message, args) {
  if (process.env.NODE_ENV === "test" || process.env.VITEST) return;
  try {
    const timestamp = new Date().toISOString();
    // Extract structured data if first arg is an object
    let structuredData = null;
    if (args.length > 0 && typeof args[0] === "object" && args[0] !== null) {
      structuredData = args[0];
    }
    // Use pre-compiled regex
    const cleanMessage = message.replace(ANSI_REGEX, "");

    let context = {};
    try {
      context = loggerContext.getStore() || {};
    } catch {
      // Ignore TLS errors in tests
    }
    const sessionId = context.sessionId || currentSessionId;
    const traceId = context.traceId;

    // Build structured log line
    const logEntry = {
      timestamp,
      level: level.toUpperCase(),
      module: scriptName,
      sessionId,
      traceId,
      message: cleanMessage,
      ...(structuredData && { data: structuredData }),
    };

    // Buffer the log entry instead of sync write
    LOG_BUFFER.push(logEntry);
    logEmitter.emit("log", logEntry);

    // Start flush timer on first entry
    if (!flushTimer) {
      flushTimer = setTimeout(() => {
        flushLogBuffer();
        flushTimer = null;
      }, FLUSH_INTERVAL);
    }
  } catch {
    // Silently fail
  }
}

/**
 * @class Logger
 * @description A high-fidelity logger with smart tag coloring.
 */
class Logger {
  constructor(scriptName) {
    this.scriptName = scriptName;
    initLogFile();
  }

  /**
   * Applies distinct colors to valid bracketed tags
   * Priorities:
   * 1. [Agent:...] -> Green
   * 2. [Brave/Chrome/...] -> Cyan
   * 3. [*.js/Task] -> Magenta
   * 4. [Metrics] -> Blue
   * 5. Others -> Yellow/White
   */
  colorizeTags(text) {
    return text.replace(/\[(.*?)\]/g, (match, content) => {
      let color = COLORS.FG_YELLOW; // Default

      if (content.match(/Agent:|User:/i)) {
        color = COLORS.FG_BRIGHT_GREEN;
      } else if (content.match(/Brave|Chrome|Firefox|ixbrowser|Edge/i)) {
        color = COLORS.FG_BRIGHT_CYAN;
      } else if (content.match(/js|Task|Module|main/i)) {
        // Removed dot requirement for robustness
        color = COLORS.FG_ORANGE;
      } else if (content.match(/Metrics|Stats/i)) {
        color = COLORS.FG_BRIGHT_YELLOW;
      }

      return `${COLORS.RESET}${color}[${content}]${COLORS.RESET}`;
    });
  }

  /**
   * Helper to format console output
   */
  _log(level, color, icon, message, ...args) {
    // 1. Timestamp (Dim Gray)
    const time = new Date().toLocaleTimeString("en-US", { hour12: false });
    const timeStr = `${COLORS.DIM}${time}${COLORS.RESET}`;

    // 2. Level/Icon
    const levelStr = `${color}${icon}${COLORS.RESET}`;

    // 3. Script Name (Pre-processing)
    const context = loggerContext.getStore() || {};
    const sessionId = context.sessionId || currentSessionId;
    const taskName = context.taskName;
    const scriptName = this.scriptName;

    let displayScript = "";

    // Step 1: Add Session ID (e.g. [roxy:0001])
    if (sessionId) {
      displayScript += `[${sessionId}]`;
    }

    // Step 2: Add Task Name (e.g. [followback])
    if (taskName) {
      // Avoid redundancy if taskName is same as sessionId
      if (taskName !== sessionId) {
        // Remove .js suffix for display if present
        const cleanTask = taskName.replace(/\.js$/, "");
        displayScript += `[${cleanTask}]`;
      }
    }

    // Step 3: Add Script/Module Name (e.g. [api.click])
    // Clean scriptName: remove surrounding brackets if any for normalized check
    const cleanScriptName = scriptName
      .replace(/^\[|\]$/g, "")
      .replace(/\.js$/, "");
    if (
      cleanScriptName &&
      cleanScriptName !== taskName &&
      cleanScriptName !== sessionId &&
      cleanScriptName !== "api/actions" && // Generic fallback suppression
      cleanScriptName !== "api/interactions/actions"
    ) {
      displayScript += `[${cleanScriptName}]`;
    }

    // Fallback if absolutely everything is empty
    if (!displayScript) {
      displayScript = "[system]";
    }

    // 4. Message Coloring
    const msgColor =
      level === "ERROR"
        ? COLORS.FG_RED
        : level === "WARN"
          ? COLORS.FG_YELLOW
          : COLORS.RESET;

    // Apply Smart Coloring to Script Name
    const coloredScript = this.colorizeTags(displayScript);

    // Process Message
    // Remove space if message starts with a tag (Tight Packing)
    let separator = " ";
    if (message.trim().startsWith("[")) {
      separator = "";
    }

    // Colorize tags within the RAW message first (to avoid parsing ANSI codes as tags)
    let coloredInnerMessage = message.replace(/\[(.*?)\]/g, (match) => {
      const coloredTag = this.colorizeTags(match);
      // Re-apply msgColor after the tag's RESET so the rest of the string stays colored
      return `${coloredTag}${msgColor}`;
    });

    // Highlight words with '@' (e.g. @User, email@addr)
    coloredInnerMessage = coloredInnerMessage.replace(/(\S*@\S*)/g, (match) => {
      // Don't colorize if it looks like an ANSI code or is inside one
      if (match.includes("\x1b")) return match;
      return `${COLORS.BG_YELLOW}${COLORS.FG_BLACK}${match}${COLORS.RESET}${msgColor}`;
    });

    // Highlight quoted text ("...", '...') and parentheses (...) with distinct text colors
    coloredInnerMessage = coloredInnerMessage.replace(
      /(".*?"|'.*?'|\(.*?\))/g,
      (match) => {
        if (match.includes("\x1b")) return match;

        if (match.startsWith('"')) {
          return `${COLORS.FG_BRIGHT_YELLOW}${match}${COLORS.RESET}${msgColor}`;
        } else if (match.startsWith("'")) {
          return `${COLORS.FG_BRIGHT_GREEN}${match}${COLORS.RESET}${msgColor}`;
        } else if (match.startsWith("(")) {
          return `${COLORS.FG_BRIGHT_MAGENTA}${match}${COLORS.RESET}${msgColor}`;
        }
        return match;
      },
    );

    // Highlight URLs (http, https, ws, wss, ftp, ftps or www.) in orange
    coloredInnerMessage = coloredInnerMessage.replace(
      /((?:(?:https?|wss?|ftp|ftps):\/\/|www\.)[^\s\]]+)/gi,
      (match) => {
        if (match.includes("\x1b")) return match;
        return `${COLORS.FG_ORANGE}${match}${COLORS.RESET}${msgColor}`;
      },
    );

    // Wrap the message in the base color
    const coloredMessageFinal = `${msgColor}${coloredInnerMessage}${COLORS.RESET}`;

    try {
      console.log(
        `${timeStr} ${levelStr} ${coloredScript}${separator}${coloredMessageFinal}`,
        ...args,
      );
    } catch (_e) {
      // In some environments (like Vitest threads), console.log can throw after MessagePort is closed
    }

    // Write clean version to file
    writeToLogFile(level, this.scriptName, message, args);
  }

  /**
   * Logs an informational message.
   */
  info(message, ...args) {
    this._log("INFO", COLORS.FG_CYAN, "🔵", message, ...args);
  }

  /**
   * Logs a success message.
   */
  success(message, ...args) {
    this._log("SUCCESS", COLORS.FG_GREEN, "🟢", message, ...args);
  }

  /**
   * Logs an error message.
   */
  error(message, ...args) {
    this._log("ERROR", COLORS.FG_RED, "🔴", message, ...args);
  }

  /**
   * Logs a warning message.
   */
  warn(message, ...args) {
    this._log("WARN", COLORS.FG_YELLOW, "🟡", message, ...args);
  }

  /**
   * Logs a debug message.
   */
  debug(message, ...args) {
    this._log("DEBUG", COLORS.FG_GRAY, "⚪", message, ...args);
  }
}

/**
 * Creates a new Logger instance.
 */
export function createLogger(scriptName) {
  return new Logger(scriptName);
}

// Session tracking for structured logging
let currentSessionId = null;
let sessionStartTime = null;
let sessionBrowserInfo = null;

export const sessionLogger = {
  startSession(sessionId, browserInfo = null) {
    currentSessionId = sessionId;
    sessionStartTime = Date.now();
    sessionBrowserInfo = browserInfo;
    const logger = createLogger("session.js");
    logger.info(
      `[Session] Started: ${sessionId} ${browserInfo ? `[${browserInfo}]` : ""}`,
    );
    return { sessionId, browserInfo, startTime: sessionStartTime };
  },

  endSession() {
    const sessionId = currentSessionId;
    const duration = sessionStartTime ? Date.now() - sessionStartTime : 0;
    const logger = createLogger("session.js");
    logger.info(`[Session] Ended: ${sessionId} (duration: ${duration}ms)`);
    currentSessionId = null;
    sessionStartTime = null;
    sessionBrowserInfo = null;
    return { sessionId, duration };
  },

  getSessionId() {
    return currentSessionId;
  },

  getSessionInfo() {
    return {
      sessionId: currentSessionId,
      browserInfo: sessionBrowserInfo,
      startTime: sessionStartTime,
      duration: sessionStartTime ? Date.now() - sessionStartTime : 0,
    };
  },

  setCurrentSessionId(sessionId) {
    currentSessionId = sessionId;
  },
};

export default Logger;

// =========================================================================
// BUFFERED LOGGER - Reduces I/O overhead for high-frequency logging
// =========================================================================

/**
 * @class BufferedLogger
 * @description Buffers log entries and flushes them periodically or when threshold is reached
 *              Reduces console I/O overhead during high-activity periods
 */
class BufferedLogger {
  /**
   * Create a buffered logger
   * @param {object} options - Configuration options
   */
  constructor(options = {}) {
    this.flushInterval = options.flushInterval || 5000; // Default: 5 seconds
    this.maxBufferSize = options.maxBufferSize || 100; // Default: 100 entries
    this.minBufferSize = options.minBufferSize || 10; // Default: 10 entries

    this.buffer = [];
    this.timer = null;
    this.logger = createLogger("BufferedLogger");
    this.module = options.module || "buffered";
    this.enabled = options.enabled !== false;

    if (this.enabled) {
      this._startTimer();
    }
  }

  /**
   * Start the flush timer
   */
  _startTimer() {
    if (this.timer) return;
    this.timer = setInterval(() => this.flush(), this.flushInterval);
  }

  /**
   * Stop the flush timer
   */
  _stopTimer() {
    if (this.timer) {
      clearInterval(this.timer);
      this.timer = null;
    }
  }

  /**
   * Add entry to buffer
   * @param {string} level - Log level
   * @param {string} message - Log message
   * @param {object} data - Optional data
   */
  _add(level, message, data = null) {
    if (!this.enabled) return;

    this.buffer.push({
      timestamp: Date.now(),
      level,
      message,
      data,
    });

    // Flush if buffer is full
    if (this.buffer.length >= this.maxBufferSize) {
      this.flush();
    }
  }

  /**
   * Flush buffer to console and file
   */
  flush() {
    if (this.buffer.length === 0) return;

    // Get base logger for actual output
    const baseLogger = createLogger(this.module);
    const entriesToFlush = [...this.buffer];
    this.buffer = [];

    // Group entries by level for cleaner output
    const grouped = entriesToFlush.reduce((acc, entry) => {
      if (!acc[entry.level]) acc[entry.level] = [];
      acc[entry.level].push(entry);
      return acc;
    }, {});

    // Output grouped entries
    for (const [level, entries] of Object.entries(grouped)) {
      if (entries.length === 1) {
        // Single entry - log normally
        const entry = entries[0];
        baseLogger[level.toLowerCase()](
          `[Buffer:${entries.length}] ${entry.message}`,
        );
      } else {
        // Multiple entries - summarize
        baseLogger[level.toLowerCase()](
          `[Buffer:${entries.length}] ${entries[0].message} (+${entries.length - 1} more)`,
        );
      }
    }
  }

  /**
   * Log info level message
   */
  info(message, data = null) {
    this._add("INFO", message, data);
  }

  /**
   * Log success level message
   */
  success(message, data = null) {
    this._add("SUCCESS", message, data);
  }

  /**
   * Log warn level message
   */
  warn(message, data = null) {
    this._add("WARN", message, data);
  }

  /**
   * Log error level message (flushes immediately)
   */
  error(message, data = null) {
    this._add("ERROR", message, data);
    this.flush(); // Errors should be logged immediately
  }

  /**
   * Log debug level message
   */
  debug(message, data = null) {
    this._add("DEBUG", message, data);
  }

  /**
   * Get buffer statistics
   */
  getStats() {
    return {
      bufferSize: this.buffer.length,
      maxBufferSize: this.maxBufferSize,
      flushInterval: this.flushInterval,
      enabled: this.enabled,
    };
  }

  /**
   * Clear buffer without logging
   */
  clear() {
    this.buffer = [];
  }

  /**
   * Shutdown - flush remaining and cleanup
   */
  shutdown() {
    this._stopTimer();
    this.flush();
  }
}

/**
 * Create a buffered logger instance
 * @param {string} moduleName - Module name for logging
 * @param {object} options - Configuration options
 * @returns {BufferedLogger}
 */
export function createBufferedLogger(moduleName, options = {}) {
  return new BufferedLogger({ ...options, module: moduleName });
}

export { BufferedLogger };
