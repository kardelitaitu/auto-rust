/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Visual Debug Helper - Shows cursor position and clicks on screen
 * Useful for debugging browser automation interactions
 * @module api/utils/visual-debug
 */

import { getPage } from "../core/context.js";
import { createLogger } from "../core/logger.js";

const logger = createLogger("api/utils/visual-debug.js");

// IDs for DOM elements
const IDS = {
  cursor: "autoai-debug-cursor",
  overlay: "autoai-debug-overlay",
  styles: "autoai-debug-styles",
  clickHistory: "autoai-debug-click-history",
};

// State
let isEnabled = false;

const STYLES_CSS = `
/* Cursor tracker */
#${IDS.cursor} {
    position: fixed !important;
    width: 30px !important;
    height: 30px !important;
    background: rgba(255, 0, 0, 0.8) !important;
    border: 3px solid red !important;
    border-radius: 50% !important;
    pointer-events: none !important;
    z-index: 2147483647 !important;
    transform: translate(-50%, -50%) !important;
    transition: left 0.05s, top 0.05s !important;
    box-shadow: 0 0 15px rgba(255, 0, 0, 0.8) !important;
    display: block !important;
    visibility: visible !important;
    opacity: 1 !important;
}

/* Click ripple effect */
.autoai-click-marker {
    position: fixed !important;
    width: 50px !important;
    height: 50px !important;
    border: 4px solid lime !important;
    border-radius: 50% !important;
    pointer-events: none !important;
    z-index: 2147483647 !important;
    transform: translate(-50%, -50%) !important;
    animation: autoaiClickPulse 0.6s ease-out forwards !important;
    display: block !important;
    visibility: visible !important;
    opacity: 1 !important;
}

@keyframes autoaiClickPulse {
    0% { 
        transform: translate(-50%, -50%) scale(0.3); 
        opacity: 1; 
        border-color: lime;
        background: rgba(0, 255, 0, 0.3);
    }
    50% { 
        transform: translate(-50%, -50%) scale(1.2); 
        opacity: 0.7; 
        border-color: yellow;
        background: rgba(255, 255, 0, 0.2);
    }
    100% { 
        transform: translate(-50%, -50%) scale(2); 
        opacity: 0; 
        border-color: orange;
        background: transparent;
    }
}

/* Info panel */
#${IDS.overlay} {
    position: fixed !important;
    bottom: 10px !important;
    right: 10px !important;
    background: #000000 !important;
    color: #0f0 !important;
    font-family: monospace !important;
    font-size: 14px !important;
    padding: 15px 20px !important;
    border-radius: 8px !important;
    z-index: 2147483647 !important;
    min-width: 220px !important;
    border: 2px solid lime !important;
    box-shadow: 0 0 20px rgba(0, 255, 0, 0.5) !important;
    pointer-events: none !important;
    display: block !important;
    visibility: visible !important;
    opacity: 1 !important;
}

#${IDS.overlay} h4 {
    margin: 0 0 8px 0;
    color: #0ff;
    font-size: 14px;
    border-bottom: 1px solid #333;
    padding-bottom: 5px;
}

#${IDS.overlay} .stat {
    display: flex;
    justify-content: space-between;
    margin: 4px 0;
}

#${IDS.overlay} .stat-label {
    color: #888;
}

#${IDS.overlay} .stat-value {
    color: #0f0;
    font-weight: bold;
}

#${IDS.clickHistory} {
    max-height: 150px;
    overflow-y: auto;
    margin-top: 8px;
    border-top: 1px solid #333;
    padding-top: 8px;
}

#${IDS.clickHistory} .click-entry {
    font-size: 11px;
    color: #aaa;
    margin: 2px 0;
    padding: 2px 4px;
    background: rgba(255, 255, 255, 0.05);
    border-radius: 3px;
}

#${IDS.clickHistory} .click-entry:last-child {
    color: #0f0;
    background: rgba(0, 255, 0, 0.1);
}
`;

/**
 * Enable visual debug overlay
 * @returns {Promise<boolean>}
 */
export async function enable() {
  const page = getPage();
  if (!page) {
    logger.warn("[VisualDebug] No page available");
    return false;
  }

  try {
    // Inject styles and create elements in one evaluate call
    await page.evaluate(
      ({ ids, css }) => {
        // Add styles
        if (!document.getElementById(ids.styles)) {
          const style = document.createElement("style");
          style.id = ids.styles;
          style.textContent = css;
          document.head.appendChild(style);
        }

        // Create cursor
        if (!document.getElementById(ids.cursor)) {
          const cursor = document.createElement("div");
          cursor.id = ids.cursor;
          document.body.appendChild(cursor);
        }

        // Create info panel
        if (!document.getElementById(ids.overlay)) {
          const overlay = document.createElement("div");
          overlay.id = ids.overlay;
          overlay.innerHTML = `
                    <h4>🐛 Auto-AI Debug</h4>
                    <div class="stat">
                        <span class="stat-label">Cursor:</span>
                        <span class="stat-value" id="cursor-pos">-</span>
                    </div>
                    <div class="stat">
                        <span class="stat-label">Clicks:</span>
                        <span class="stat-value" id="click-count">0</span>
                    </div>
                    <div class="stat">
                        <span class="stat-label">Last Click:</span>
                        <span class="stat-value" id="last-click">-</span>
                    </div>
                    <div id="${ids.clickHistory}"></div>
                `;
          document.body.appendChild(overlay);
        }

        // Track mouse movement
        if (!window.__autoaiMouseTracking) {
          window.__autoaiMouseTracking = true;
          document.addEventListener("mousemove", (e) => {
            const cursor = document.getElementById(ids.cursor);
            if (cursor) {
              cursor.style.left = e.clientX + "px";
              cursor.style.top = e.clientY + "px";
            }

            const posDisplay = document.getElementById("cursor-pos");
            if (posDisplay) {
              posDisplay.textContent = `${Math.round(e.clientX)}, ${Math.round(e.clientY)}`;
            }
          });
        }

        // Track clicks and pointer events
        if (!window.__autoaiClickTracking) {
          window.__autoaiClickTracking = true;
          window.__autoaiClickCount = 0;
          window.__autoaiClickHistory = [];

          // Helper function to update cursor position (called from external code)
          window.__autoaiMoveCursor = (x, y) => {
            const cursor = document.getElementById(ids.cursor);
            if (cursor) {
              cursor.style.left = x + "px";
              cursor.style.top = y + "px";
            }

            const posDisplay = document.getElementById("cursor-pos");
            if (posDisplay) {
              posDisplay.textContent = `${Math.round(x)}, ${Math.round(y)}`;
            }
          };

          // Helper function to record click
          window.__autoaiRecordClick = (x, y, source) => {
            window.__autoaiClickCount++;

            // Move cursor to click position
            window.__autoaiMoveCursor(x, y);

            // Create click marker
            const marker = document.createElement("div");
            marker.className = "autoai-click-marker";
            marker.style.left = x + "px";
            marker.style.top = y + "px";
            document.body.appendChild(marker);
            setTimeout(() => marker.remove(), 600);

            // Update display
            const countDisplay = document.getElementById("click-count");
            if (countDisplay) {
              countDisplay.textContent = window.__autoaiClickCount;
            }

            const lastClickDisplay = document.getElementById("last-click");
            if (lastClickDisplay) {
              lastClickDisplay.textContent = `${Math.round(x)}, ${Math.round(y)}`;
            }

            // Add to history
            const time = new Date().toLocaleTimeString();
            window.__autoaiClickHistory.unshift({
              x: Math.round(x),
              y: Math.round(y),
              time,
              source,
            });
            if (window.__autoaiClickHistory.length > 10) {
              window.__autoaiClickHistory.pop();
            }

            // Update history display
            const historyContainer = document.getElementById(ids.clickHistory);
            if (historyContainer) {
              historyContainer.innerHTML = window.__autoaiClickHistory
                .map(
                  (c) =>
                    `<div class="click-entry">${c.time}: (${c.x}, ${c.y}) [${c.source}]</div>`,
                )
                .join("");
            }

            console.log(
              `[AutoAI Debug] Click #${window.__autoaiClickCount} at (${Math.round(x)}, ${Math.round(y)}) via ${source}`,
            );
          };

          // Track DOM clicks
          document.addEventListener("click", (e) => {
            window.__autoaiRecordClick(e.clientX, e.clientY, "click");
          });

          // Track pointer events (more reliable for canvas)
          document.addEventListener("pointerdown", (e) => {
            window.__autoaiRecordClick(e.clientX, e.clientY, "pointer");
          });
        }
      },
      { ids: IDS, css: STYLES_CSS },
    );

    isEnabled = true;
    logger.info("[VisualDebug] Enabled - cursor and clicks will be visualized");
    return true;
  } catch (e) {
    logger.error(`[VisualDebug] Failed to enable: ${e.message}`);
    return false;
  }
}

/**
 * Disable visual debug overlay
 * @returns {Promise<boolean>}
 */
export async function disable() {
  const page = getPage();
  if (!page) return false;

  try {
    await page.evaluate((ids) => {
      document.getElementById(ids.cursor)?.remove();
      document.getElementById(ids.overlay)?.remove();
      document.getElementById(ids.styles)?.remove();
      document
        .querySelectorAll(".autoai-click-marker")
        .forEach((el) => el.remove());

      delete window.__autoaiClickCount;
      delete window.__autoaiClickHistory;
      window.__autoaiMouseTracking = false;
      window.__autoaiClickTracking = false;
    }, IDS);

    isEnabled = false;
    logger.info("[VisualDebug] Disabled");
    return true;
  } catch (e) {
    logger.error(`[VisualDebug] Failed to disable: ${e.message}`);
    return false;
  }
}

/**
 * Toggle visual debug overlay
 * @returns {Promise<boolean>}
 */
export async function toggle() {
  if (isEnabled) {
    return await disable();
  } else {
    return await enable();
  }
}

/**
 * Log a custom marker at specific coordinates
 * @param {number} x - X coordinate
 * @param {number} y - Y coordinate
 * @param {string} label - Optional label
 * @returns {Promise<void>}
 */
export async function mark(x, y, label = "") {
  const page = getPage();
  if (!page || !isEnabled) return;

  try {
    await page.evaluate(
      ({ x, y, label }) => {
        const marker = document.createElement("div");
        marker.style.cssText = `
                position: fixed;
                left: ${x}px;
                top: ${y}px;
                width: 50px;
                height: 50px;
                border: 3px solid cyan;
                border-radius: 50%;
                pointer-events: none;
                z-index: 999997;
                transform: translate(-50%, -50%);
                display: flex;
                align-items: center;
                justify-content: center;
                font-size: 10px;
                color: cyan;
                background: rgba(0, 255, 255, 0.1);
            `;
        if (label) {
          marker.textContent = label;
        }
        document.body.appendChild(marker);
        setTimeout(() => marker.remove(), 2000);
      },
      { x, y, label },
    );
  } catch (e) {
    logger.error(`[VisualDebug] Failed to mark: ${e.message}`);
  }
}

/**
 * Move the debug cursor to a new position
 * @param {number} x - X coordinate
 * @param {number} y - Y coordinate
 * @returns {Promise<void>}
 */
export async function moveCursor(x, y) {
  const page = getPage();
  if (!page || !isEnabled) return;

  try {
    await page.evaluate(
      ({ x, y }) => {
        if (window.__autoaiMoveCursor) {
          window.__autoaiMoveCursor(x, y);
        }
      },
      { x, y },
    );
  } catch (_e) {
    // Ignore errors
  }
}

/**
 * Check if debug is enabled
 * @returns {boolean}
 */
export function isEnabledDebug() {
  return isEnabled;
}

/**
 * Get current debug state
 * @returns {object}
 */
export function getState() {
  return {
    enabled: isEnabled,
    ...IDS,
  };
}

export default {
  enable,
  disable,
  toggle,
  mark,
  moveCursor,
  getState,
  isEnabled: isEnabledDebug,
};
