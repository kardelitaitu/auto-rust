/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Session Persistence Store
 * Stores session data and learned patterns in SQLite for cross-session learning
 * @module api/agent/sessionStore
 */

import { createLogger } from "../core/logger.js";

const logger = createLogger("api/agent/sessionStore.js");

class SessionStore {
  constructor() {
    this.db = null;
    this.enabled = false;
    this._initDatabase();
  }

  async _initDatabase() {
    try {
      const Database = (await import("better-sqlite3")).default;
      this.db = new Database("data/agent-sessions.db");
      this._createTables();
      this.enabled = true;
      logger.info("[SessionStore] Database initialized successfully");
    } catch (error) {
      logger.warn(
        "[SessionStore] Database not available, using in-memory fallback:",
        error.message,
      );
      this.db = null;
      this.enabled = false;
      this._initFallbackStorage();
    }
  }

  _initFallbackStorage() {
    this.memoryStore = {
      sessions: [],
      actionPatterns: [],
      learnedSelectors: [],
    };
  }

  _createTables() {
    if (!this.db) return;

    this.db.exec(`
            CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                goal TEXT,
                url TEXT,
                success INTEGER,
                steps INTEGER,
                duration_ms INTEGER,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );
            
            CREATE TABLE IF NOT EXISTS action_patterns (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                url_pattern TEXT,
                element_selector TEXT,
                action_type TEXT,
                success_count INTEGER DEFAULT 0,
                failure_count INTEGER DEFAULT 0,
                last_used DATETIME,
                UNIQUE(url_pattern, element_selector, action_type)
            );
            
            CREATE TABLE IF NOT EXISTS learned_selectors (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                url_pattern TEXT,
                element_description TEXT,
                selector TEXT,
                confidence REAL DEFAULT 0.5,
                times_used INTEGER DEFAULT 0,
                last_used DATETIME,
                UNIQUE(url_pattern, element_description)
            );
            
            CREATE INDEX IF NOT EXISTS idx_patterns_url ON action_patterns(url_pattern);
            CREATE INDEX IF NOT EXISTS idx_selectors_url ON learned_selectors(url_pattern);
        `);
  }

  /**
   * Record a session result
   * @param {object} session - Session data
   */
  recordSession(session) {
    if (!this.enabled) {
      logger.debug("[SessionStore] Database disabled, skipping session record");
      return;
    }

    const { id, goal, url, success, steps, durationMs } = session;

    if (this.db) {
      try {
        const stmt = this.db.prepare(`
                    INSERT OR REPLACE INTO sessions (id, goal, url, success, steps, duration_ms)
                    VALUES (?, ?, ?, ?, ?, ?)
                `);
        stmt.run(id, goal, url, success ? 1 : 0, steps, durationMs);
        logger.debug(`[SessionStore] Recorded session: ${id}`);
      } catch (error) {
        logger.error("[SessionStore] Failed to record session:", error.message);
      }
    } else {
      this.memoryStore.sessions.push(session);
    }
  }

  /**
   * Record action success/failure
   * @param {string} url - Page URL
   * @param {string} selector - Element selector
   * @param {string} actionType - Action type (click, type, etc.)
   * @param {boolean} success - Whether action succeeded
   */
  recordAction(url, selector, actionType, success) {
    if (!this.enabled) return;

    const urlPattern = this._extractUrlPattern(url);

    if (this.db) {
      try {
        const stmt = this.db.prepare(`
                    INSERT INTO action_patterns (url_pattern, element_selector, action_type, success_count, failure_count, last_used)
                    VALUES (?, ?, ?, ?, ?, CURRENT_TIMESTAMP)
                    ON CONFLICT(url_pattern, element_selector, action_type) DO UPDATE SET
                        success_count = success_count + excluded.success_count,
                        failure_count = failure_count + excluded.failure_count,
                        last_used = CURRENT_TIMESTAMP
                `);
        stmt.run(
          urlPattern,
          selector,
          actionType,
          success ? 1 : 0,
          success ? 0 : 1,
        );
      } catch (error) {
        logger.error("[SessionStore] Failed to record action:", error.message);
      }
    } else {
      this.memoryStore.actionPatterns.push({
        urlPattern,
        selector,
        actionType,
        success,
      });
    }
  }

  /**
   * Get success rate for an action pattern
   * @param {string} url - Page URL
   * @param {string} selector - Element selector
   * @param {string} actionType - Action type
   * @returns {number} Success rate (0-1)
   */
  getActionSuccessRate(url, selector, actionType) {
    if (!this.enabled) return 0.5; // Default neutral

    const urlPattern = this._extractUrlPattern(url);

    if (this.db) {
      try {
        const stmt = this.db.prepare(`
                    SELECT success_count, failure_count FROM action_patterns
                    WHERE url_pattern = ? AND element_selector = ? AND action_type = ?
                `);
        const row = stmt.get(urlPattern, selector, actionType);

        if (row) {
          const total = row.success_count + row.failure_count;
          return total > 0 ? row.success_count / total : 0.5;
        }
      } catch (error) {
        logger.error(
          "[SessionStore] Failed to get success rate:",
          error.message,
        );
      }
    }

    return 0.5;
  }

  /**
   * Learn a selector for an element description
   * @param {string} url - Page URL
   * @param {string} description - Element description
   * @param {string} selector - Working selector
   * @param {boolean} success - Whether selector worked
   */
  learnSelector(url, description, selector, success) {
    if (!this.enabled) return;

    const urlPattern = this._extractUrlPattern(url);

    if (this.db) {
      try {
        const stmt = this.db.prepare(`
                    INSERT INTO learned_selectors (url_pattern, element_description, selector, confidence, times_used, last_used)
                    VALUES (?, ?, ?, ?, 1, CURRENT_TIMESTAMP)
                    ON CONFLICT(url_pattern, element_description) DO UPDATE SET
                        selector = excluded.selector,
                        confidence = CASE 
                            WHEN excluded.confidence > confidence THEN excluded.confidence 
                            ELSE confidence * 0.9
                        END,
                        times_used = times_used + 1,
                        last_used = CURRENT_TIMESTAMP
                `);
        stmt.run(urlPattern, description, selector, success ? 0.9 : 0.3);
      } catch (error) {
        logger.error("[SessionStore] Failed to learn selector:", error.message);
      }
    } else {
      this.memoryStore.learnedSelectors.push({
        urlPattern,
        description,
        selector,
        success,
      });
    }
  }

  /**
   * Get best selector for an element description
   * @param {string} url - Page URL
   * @param {string} description - Element description
   * @returns {string|null} Best selector or null
   */
  getBestSelector(url, description) {
    if (!this.enabled) return null;

    const urlPattern = this._extractUrlPattern(url);

    if (this.db) {
      try {
        const stmt = this.db.prepare(`
                    SELECT selector, confidence FROM learned_selectors
                    WHERE url_pattern = ? AND element_description = ?
                    ORDER BY confidence DESC, times_used DESC
                    LIMIT 1
                `);
        const row = stmt.get(urlPattern, description);

        if (row && row.confidence > 0.6) {
          logger.debug(
            `[SessionStore] Found learned selector: ${row.selector} (confidence: ${row.confidence})`,
          );
          return row.selector;
        }
      } catch (error) {
        logger.error(
          "[SessionStore] Failed to get best selector:",
          error.message,
        );
      }
    }

    return null;
  }

  /**
   * Extract URL pattern from full URL
   * @private
   */
  _extractUrlPattern(url) {
    try {
      const urlObj = new URL(url);
      return `${urlObj.hostname}${urlObj.pathname.split("/").slice(0, 2).join("/")}`;
    } catch {
      return url;
    }
  }

  /**
   * Get session statistics
   * @returns {object} Statistics
   */
  getStats() {
    if (!this.enabled) {
      return { enabled: false };
    }

    if (this.db) {
      try {
        const sessionCount = this.db
          .prepare("SELECT COUNT(*) as count FROM sessions")
          .get().count;
        const patternCount = this.db
          .prepare("SELECT COUNT(*) as count FROM action_patterns")
          .get().count;
        const selectorCount = this.db
          .prepare("SELECT COUNT(*) as count FROM learned_selectors")
          .get().count;

        return {
          enabled: true,
          sessions: sessionCount,
          patterns: patternCount,
          selectors: selectorCount,
        };
      } catch (error) {
        logger.error("[SessionStore] Failed to get stats:", error.message);
      }
    }

    return {
      enabled: true,
      sessions: this.memoryStore.sessions.length,
      patterns: this.memoryStore.actionPatterns.length,
      selectors: this.memoryStore.learnedSelectors.length,
    };
  }

  /**
   * Clear all stored data
   */
  clear() {
    if (this.db) {
      try {
        this.db.exec(
          "DELETE FROM sessions; DELETE FROM action_patterns; DELETE FROM learned_selectors;",
        );
        logger.info("[SessionStore] Cleared all data");
      } catch (error) {
        logger.error("[SessionStore] Failed to clear data:", error.message);
      }
    } else {
      this._initFallbackStorage();
    }
  }
}

const sessionStore = new SessionStore();

export { sessionStore };
export default sessionStore;
