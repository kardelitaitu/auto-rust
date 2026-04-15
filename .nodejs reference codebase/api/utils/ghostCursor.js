/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Advanced Ghost Cursor (api/-internal)
 * Moved from utils/ghostCursor.js to break the circular dependency:
 *   utils/ghostCursor.js → api/index.js → core/context.js → utils/ghostCursor.js
 *
 * Implements human-like mouse physics: Fitts's Law, Overshoot/Correction,
 * Gaussian targeting, variable reaction timing.
 *
 * @module api/utils/ghostCursor
 */

import { mathUtils } from './math.js';
import { createLogger } from '../core/logger.js';
import { TWITTER_CLICK_PROFILES } from '../constants/engagement.js';

export class GhostCursor {
    constructor(page, logger = null) {
        this.page = page;
        this.logger = logger || createLogger('api.click');
        this.previousPos = { x: 0, y: 0 };
        this.init();
    }

    async init() {
        this.previousPos = {
            x: mathUtils.randomInRange(50, 500),
            y: mathUtils.randomInRange(50, 500),
        };
    }

    // ── Vector Arithmetic Helpers ─────────────────────────────────────
    vecAdd(a, b) {
        return { x: a.x + b.x, y: a.y + b.y };
    }
    vecSub(a, b) {
        return { x: a.x - b.x, y: a.y - b.y };
    }
    vecMult(a, s) {
        return { x: a.x * s, y: a.y * s };
    }
    vecLen(a) {
        return Math.sqrt(a.x * a.x + a.y * a.y);
    }

    /** Cubic Bezier Point */
    bezier(t, p0, p1, p2, p3) {
        const u = 1 - t;
        const tt = t * t,
            uu = u * u,
            uuu = uu * u,
            ttt = tt * t;
        return {
            x: uuu * p0.x + 3 * uu * t * p1.x + 3 * u * tt * p2.x + ttt * p3.x,
            y: uuu * p0.y + 3 * uu * t * p1.y + 3 * u * tt * p2.y + ttt * p3.y,
        };
    }

    /** EaseOutCubic — starts fast, slows down naturally */
    easeOutCubic(t) {
        return 1 - Math.pow(1 - t, 3);
    }

    /**
     * Move the mouse along a Bezier arc with variable velocity and tremor noise.
     */
    async performMove(start, end, durationMs, _steps = 30) {
        if (
            !start ||
            !end ||
            !Number.isFinite(start.x) ||
            !Number.isFinite(start.y) ||
            !Number.isFinite(end.x) ||
            !Number.isFinite(end.y)
        )
            return;

        const distance = this.vecLen(this.vecSub(end, start));
        const arcAmount = mathUtils.randomInRange(20, Math.min(200, distance * 0.5));

        const p0 = start,
            p3 = end;
        const p1 = {
            x: start.x + (end.x - start.x) * 0.3 + mathUtils.gaussian(0, arcAmount),
            y: start.y + (end.y - start.y) * 0.3 + mathUtils.gaussian(0, arcAmount),
        };
        const p2 = {
            x: start.x + (end.x - start.x) * 0.7 + mathUtils.gaussian(0, arcAmount),
            y: start.y + (end.y - start.y) * 0.7 + mathUtils.gaussian(0, arcAmount),
        };

        const startTime = Date.now();
        let loop = true;

        while (loop) {
            const elapsed = Date.now() - startTime;
            let progress = elapsed / durationMs;

            if (progress >= 1) {
                progress = 1;
                loop = false;
            }

            const easedT = this.easeOutCubic(progress);
            const pos = this.bezier(easedT, p0, p1, p2, p3);

            const tremorScale = (1 - easedT) * 1.5;
            const noisyX = pos.x + (Math.random() - 0.5) * tremorScale;
            const noisyY = pos.y + (Math.random() - 0.5) * tremorScale;

            await this.page.mouse.move(noisyX, noisyY);
            if (loop) await new Promise((r) => setTimeout(r, Math.random() * 8));
        }

        this.previousPos = end;
    }

    /**
     * Wait for element to be stable (not animating) before clicking.
     * @param {object} locator - Playwright locator
     * @param {number} [maxWaitMs=3000]
     * @returns {Promise<object|null>}
     */
    async waitForStableElement(locator, maxWaitMs = 3000) {
        const startTime = Date.now();
        let prevBox = null,
            stableCount = 0;
        const requiredStableChecks = 3;

        while (Date.now() - startTime < maxWaitMs) {
            const bbox = await Promise.resolve(
                locator.boundingBox ? locator.boundingBox() : null
            ).catch(() => null);

            if (!bbox) return null;

            if (prevBox) {
                const delta = Math.abs(bbox.x - prevBox.x) + Math.abs(bbox.y - prevBox.y);
                if (delta < 2) {
                    stableCount++;
                    if (stableCount >= requiredStableChecks) return bbox;
                } else {
                    stableCount = 0;
                }
            }

            prevBox = bbox;
            await new Promise((r) => setTimeout(r, 100));
        }

        return prevBox;
    }

    /**
     * Profiled click with hover-hold, hesitation, and retry logic based on provided profile parameters.
     * @param {object} locator
     * @param {object} profile - Specific engagement profile
     * @param {number} [maxRetries=3]
     */
    async profiledClick(
        locator,
        profile = { hoverMin: 200, hoverMax: 800, holdMs: 80, hesitation: false, microMove: false },
        maxRetries = 3
    ) {
        let lastError = null;

        for (let attempt = 0; attempt <= maxRetries; attempt++) {
            try {
                let bbox = await this.waitForStableElement(locator, 3000);

                if (!bbox) {
                    console.warn('[GhostCursor] Twitter click: No bounding box found');
                    await locator.click({ force: true }).catch(() => {});
                    return;
                }

                const fixationDelay =
                    attempt === 0
                        ? mathUtils.randomInRange(200, 800)
                        : mathUtils.randomInRange(50, 200);
                await new Promise((r) => setTimeout(r, fixationDelay));

                const marginX = bbox.width * 0.15,
                    marginY = bbox.height * 0.15;
                const targetX = mathUtils.gaussian(
                    bbox.x + bbox.width / 2,
                    bbox.width / 6,
                    bbox.x + marginX,
                    bbox.x + bbox.width - marginX
                );
                const targetY = mathUtils.gaussian(
                    bbox.y + bbox.height / 2,
                    bbox.height / 6,
                    bbox.y + marginY,
                    bbox.y + bbox.height - marginY
                );

                await this.moveWithHesitation(targetX, targetY);
                await this.hoverWithDrift(targetX, targetY, profile.hoverMin, profile.hoverMax);

                if (profile.hesitation) {
                    await new Promise((r) => setTimeout(r, mathUtils.randomInRange(40, 120)));
                }

                if (profile.microMove) {
                    const microX = targetX + mathUtils.randomInRange(-2, 2);
                    const microY = targetY + mathUtils.randomInRange(-2, 2);
                    await this.page.mouse.move(microX, microY);
                    await new Promise((r) => setTimeout(r, mathUtils.randomInRange(20, 50)));
                }

                await this.page.mouse.down();
                await new Promise((r) => setTimeout(r, profile.holdMs));
                await this.page.mouse.up();

                this.previousPos = { x: targetX, y: targetY };
                return;
            } catch (error) {
                lastError = error;
                console.warn(
                    `[GhostCursor] Twitter click attempt ${attempt + 1}/${maxRetries + 1} failed: ${error.message}`
                );

                if (attempt < maxRetries) {
                    const delay = Math.pow(2, attempt) * 1000;
                    await new Promise((r) => setTimeout(r, delay));
                }
            }
        }

        console.warn(
            `[GhostCursor] All retries failed, using native fallback: ${lastError?.message}`
        );
        try {
            await locator.click({ force: true });
        } catch (fallbackError) {
            console.warn(`[GhostCursor] Native fallback also failed: ${fallbackError.message}`);
        }
    }

    /**
     * Twitter-specific reply click with hover-hold and hesitation.
     * Convenience wrapper around profiledClick using TWITTER_CLICK_PROFILES.
     * @param {object} locator - Playwright locator
     * @param {string} actionType - Click profile to use ('reply', 'like', 'retweet', etc.)
     * @param {number} [maxRetries=3]
     */
    async twitterClick(locator, actionType = 'reply', maxRetries = 3) {
        const profile = TWITTER_CLICK_PROFILES[actionType] || TWITTER_CLICK_PROFILES.nav;
        return this.profiledClick(locator, profile, maxRetries);
    }

    /**
     * Move with mid-path hesitation for long distances (>400px).
     */
    async moveWithHesitation(targetX, targetY) {
        if (!Number.isFinite(targetX) || !Number.isFinite(targetY)) return;

        const start = this.previousPos || { x: 0, y: 0 };
        const distance = this.vecLen(this.vecSub({ x: targetX, y: targetY }, start));

        if (distance <= 400) {
            await this.move(targetX, targetY);
            return;
        }

        const midX = start.x + (targetX - start.x) * 0.4;
        const midY = start.y + (targetY - start.y) * 0.4;

        await this.performMove(start, { x: midX, y: midY }, 150);
        await new Promise((r) => setTimeout(r, mathUtils.randomInRange(100, 300)));
        await this.move(targetX, targetY);
    }

    /**
     * Hover with realistic micro-drift noise.
     */
    async hoverWithDrift(startX, startY, minDuration, maxDuration) {
        const duration = mathUtils.randomInRange(minDuration, maxDuration);
        const startTime = Date.now();
        const driftRange = 1;

        while (Date.now() - startTime < duration) {
            const driftX = (Math.random() - 0.5) * 2 * driftRange;
            const driftY = (Math.random() - 0.5) * 2 * driftRange;
            await this.page.mouse.move(startX + driftX, startY + driftY);

            if (Math.random() < 0.2) {
                await new Promise((r) => setTimeout(r, mathUtils.randomInRange(50, 150)));
            }
            await new Promise((r) => setTimeout(r, mathUtils.randomInRange(50, 100)));
        }

        this.previousPos = { x: startX, y: startY };
    }

    /**
     * Move cursor with overshoot and correction (Fitts's Law).
     */
    async move(targetX, targetY, _speed = undefined) {
        if (!Number.isFinite(targetX) || !Number.isFinite(targetY)) return;

        const start = this.previousPos || { x: 0, y: 0 };
        const end = { x: targetX, y: targetY };
        const pathVector = this.vecSub(end, start);
        const distance = this.vecLen(pathVector);

        const targetDuration = 250 + distance * 0.4 + mathUtils.randomInRange(-50, 50);
        const shouldOvershoot = distance > 500 && mathUtils.roll(0.2);

        if (shouldOvershoot) {
            const overshootScale = mathUtils.randomInRange(1.05, 1.15);
            const errorLateral = mathUtils.gaussian(0, 20);
            const overshootPoint = {
                x: start.x + pathVector.x * overshootScale + errorLateral,
                y: start.y + pathVector.y * overshootScale + errorLateral,
            };

            await this.performMove(start, overshootPoint, targetDuration * 0.8);
            await new Promise((r) => setTimeout(r, mathUtils.randomInRange(80, 300)));
            await this.performMove(overshootPoint, end, mathUtils.randomInRange(150, 300));
        } else {
            await this.performMove(start, end, targetDuration);
        }
    }

    /**
     * "Parks" the mouse in a safe zone to avoid obscuring content.
     */
    async park() {
        try {
            const vp = this.page.viewportSize();
            if (!vp) return;

            const side = mathUtils.roll(0.5) ? 'left' : 'right';
            const margin = vp.width * 0.1;
            const targetX =
                side === 'left'
                    ? mathUtils.randomInRange(0, margin)
                    : mathUtils.randomInRange(vp.width - margin, vp.width);
            const targetY = mathUtils.randomInRange(0, vp.height);
            const current = this.previousPos || { x: 0, y: 0 };
            const dist = Math.sqrt(
                Math.pow(targetX - current.x, 2) + Math.pow(targetY - current.y, 2)
            );

            await this.performMove(current, { x: targetX, y: targetY }, Math.max(800, dist * 0.8));
        } catch (_e) {
            // Ignore viewport error
        }
    }

    /**
     * Human-like click with dynamic tracking loop.
     * Uses `visible` from interactions/queries directly (no circular api import).
     */
    async click(selector, options = {}) {
        const {
            allowNativeFallback = false,
            maxStabilityWaitMs = 2000,
            preClickStabilityMs: _preClickStabilityMs = 300,
            label = '',
            hoverBeforeClick = false,
            hoverMinMs = 120,
            hoverMaxMs = 280,
            precision = 'normal',
            button = 'left',
        } = options;
        const labelSuffix = label ? ` [${label}]` : '';

        let bbox = await this.waitForStableElement(selector, maxStabilityWaitMs);
        if (!bbox) {
            if (allowNativeFallback && selector.click)
                await selector.click({ force: true, button }).catch(() => {});
            return { success: false, usedFallback: true };
        }

        const maxTrackingAttempts = 3;
        let attempt = 0;

        while (attempt < maxTrackingAttempts) {
            attempt++;

            let marginFactor = 0.15,
                sigmaDivisor = 6;
            if (precision === 'high') {
                marginFactor = 0.35;
                sigmaDivisor = 12;
            }

            const marginX = bbox.width * marginFactor;
            const marginY = bbox.height * marginFactor;
            const targetX = mathUtils.gaussian(
                bbox.x + bbox.width / 2,
                bbox.width / sigmaDivisor,
                bbox.x + marginX,
                bbox.x + bbox.width - marginX
            );
            const targetY = mathUtils.gaussian(
                bbox.y + bbox.height / 2,
                bbox.height / sigmaDivisor,
                bbox.y + marginY,
                bbox.y + bbox.height - marginY
            );

            const targetStr = `[${Math.round(targetX)}, ${Math.round(targetY)}]`;
            await this.moveWithHesitation(targetX, targetY);
            await new Promise((r) => setTimeout(r, mathUtils.randomInRange(100, 400)));

            const newBox = await selector.boundingBox();
            if (!newBox) return { success: false, usedFallback: false };

            const currentPos = this.previousPos;
            const insideX = currentPos.x >= newBox.x && currentPos.x <= newBox.x + newBox.width;
            const insideY = currentPos.y >= newBox.y && currentPos.y <= newBox.y + newBox.height;

            if (insideX && insideY) {
                const finalX = currentPos.x + mathUtils.randomInRange(-1, 1);
                const finalY = currentPos.y + mathUtils.randomInRange(-1, 1);
                const holdTime = mathUtils.gaussian(60, 20, 20, 150);

                try {
                    if (hoverBeforeClick) {
                        await this.hoverWithDrift(finalX, finalY, hoverMinMs, hoverMaxMs);
                    }
                    await this.page.mouse.move(finalX, finalY);
                    await this.page.mouse.down({ button });
                    await new Promise((r) => setTimeout(r, holdTime));
                    await this.page.mouse.up({ button });
                    this.logger.info(
                        `Mouse Cursor moved to ${targetStr}, clicked [${Math.round(finalX)}, ${Math.round(finalY)}]${labelSuffix}`
                    );
                    return { success: true, usedFallback: false, x: finalX, y: finalY };
                } catch {
                    break;
                }
            } else {
                bbox = newBox;
            }
        }

        if (allowNativeFallback && selector.click) {
            try {
                await selector.click({ force: true, button });
            } catch {
                /* ignore */
            }
            return { success: false, usedFallback: true };
        }
        return { success: false, usedFallback: false };
    }
}
