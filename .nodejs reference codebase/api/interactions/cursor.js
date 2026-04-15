/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Low-Level Cursor Control
 * Thin wrapper around GhostCursor's move() for direct spatial manipulation.
 * Includes trajectory sophistication for advanced path patterns.
 * Uses context-aware state for session isolation.
 *
 * @module api/cursor
 */

import { getCursor, getPage, setSessionInterval, clearSessionInterval } from '../core/context.js';
import {
    getStatePathStyle,
    setStatePathStyle,
    getStatePathOptions,
} from '../core/context-state.js';
import { getPersona } from '../behaviors/persona.js';
import { mathUtils } from '../utils/math.js';
import { createLogger } from '../core/logger.js';

const logger = createLogger('api/cursor.js');

const FIDGET_INTERVAL_NAME = 'cursor_fidgeting';

/**
 * Set cursor path style for movement.
 * @param {string} style - Path style: 'bezier' | 'arc' | 'zigzag' | 'overshoot' | 'stopped'
 * @param {object} [options] - Style-specific options
 * @param {number} [options.overshootDistance=20] - For overshoot style
 * @param {number} [options.stops=3] - For stopped style
 * @returns {void}
 */
export function setPathStyle(style, options = {}) {
    setStatePathStyle(style, options);
}

/**
 * Get current path style.
 * @returns {string}
 */
export function getPathStyle() {
    return getStatePathStyle();
}

/**
 * Move cursor to a DOM element using Bezier path.
 * Resolves selector to bounding box, calculates Gaussian target point,
 * and delegates to GhostCursor.move().
 * @param {string} selector - CSS selector to move to
 * @param {object} [options]
 * @param {string} [options.pathStyle] - Override path style for this move
 * @param {boolean} [options.correction=false] - Add correction movement after reaching target
 * @returns {Promise<void>}
 */
export async function move(selector, options = {}) {
    const page = getPage();
    const cursor = getCursor();
    const persona = getPersona();

    const locator = page.locator(selector).first();

    // Scroll element into view first
    try {
        await locator.scrollIntoViewIfNeeded();
    } catch (_e) {
        logger.debug(`Scroll into view failed for ${selector}: ${_e.message}`);
    }

    const box = await locator.boundingBox();

    // Debug: Show bounding box info
    logger.debug(`Selector: ${selector}`);
    logger.debug(`Bounding box:`, box);

    // Validate bounding box
    if (!box || !box.width || !box.height || isNaN(box.x) || isNaN(box.y)) {
        logger.debug(`Invalid bounding box, skipping movement`);
        return; // Skip movement, let cursor.click() handle it
    }

    // Use box center with random offset scaled by persona precision
    const centerX = box.x + box.width / 2;
    const centerY = box.y + box.height / 2;

    const precision = persona.precision ?? 0.8;
    const randomOffsetX = mathUtils.randomInRange(-10, 10) * (1 - precision);
    const randomOffsetY = mathUtils.randomInRange(-5, 5) * (1 - precision);

    const targetX = centerX + randomOffsetX;
    const targetY = centerY + randomOffsetY;

    // Debug: Show calculated targets
    logger.debug(`Target coordinates: (${Math.round(targetX)}, ${Math.round(targetY)})`);
    logger.debug(`Box center: (${Math.round(centerX)}, ${Math.round(centerY)})`);
    logger.debug(
        `Box dimensions: ${Math.round(box.width)}x${Math.round(box.height)} at (${Math.round(box.x)}, ${Math.round(box.y)})`
    );
    logger.debug(`Random offset: (${randomOffsetX}, ${randomOffsetY})`);
    logger.debug(
        `Target from center: (${Math.round(targetX - centerX)}, ${Math.round(targetY - centerY)})`
    );

    // Validate target coordinates
    if (isNaN(targetX) || isNaN(targetY)) {
        return; // Skip movement, let cursor.click() handle it
    }

    const style = options.pathStyle || getStatePathStyle();
    const useCorrection = options.correction || persona.microMoveChance > 0;

    // 1. Fitts's Law duration calculation: T = a + b * log2(2D / W)
    const distance = Math.sqrt(Math.pow(targetX - box.x, 2) + Math.pow(targetY - box.y, 2));
    const targetSize = Math.min(box.width, box.height);
    const id = Math.log2((2 * distance) / targetSize + 1);
    const baseDuration = (100 + 150 * id) * (1 / persona.speed);

    // Ghost 3.0: Flinch Detection (Periodic target stability check during move)
    const flinchInterval = setInterval(async () => {
        const currentBox = await locator.boundingBox().catch(() => null);
        if (currentBox) {
            const shift = Math.abs(currentBox.x - box.x) + Math.abs(currentBox.y - box.y);
            if (shift > 10) {
                // Significant movement threshold
                // Note: We don't throw here to avoid breaking GhostCursor's internal state,
                // but a more advanced implementation would cancel and recalculate.
                // For now, we log it for "Visual Guarding" awareness.
            }
        }
    }, 150);

    try {
        await _moveWithStyle(cursor, targetX, targetY, style, useCorrection, baseDuration);
    } finally {
        clearInterval(flinchInterval);
    }
}

/**
 * Internal: Move with specified path style and biological jitter.
 */
async function _moveWithStyle(cursor, targetX, targetY, style, useCorrection, duration) {
    const start = cursor.previousPos || { x: 0, y: 0 };
    const distance = Math.sqrt(Math.pow(targetX - start.x, 2) + Math.pow(targetY - start.y, 2));
    const persona = getPersona();
    const pathOptions = getStatePathOptions();

    // Physiological Jitter (Micro-steps with Gaussian noise)
    const applyJitter = async (x, y) => {
        const tremorAmplitude = 0.8; // px
        const jitterX = mathUtils.gaussian(0, tremorAmplitude);
        const jitterY = mathUtils.gaussian(0, tremorAmplitude);
        await cursor.move(x + jitterX, y + jitterY);
    };

    const runStandard = async () => {
        const steps = Math.max(2, Math.floor(duration / 40));
        for (let i = 1; i <= steps; i++) {
            const progress = i / steps;
            const easedProgress = 1 - Math.pow(1 - progress, 2);
            const baseX = start.x + (targetX - start.x) * easedProgress;
            const baseY = start.y + (targetY - start.y) * easedProgress;
            await applyJitter(baseX, baseY);
        }
    };

    switch (style) {
        case 'arc':
            await _moveArc(cursor, start, targetX, targetY, distance, applyJitter);
            break;
        case 'zigzag':
            await _moveZigzag(cursor, start, targetX, targetY, distance, applyJitter);
            break;
        case 'overshoot':
            await _moveOvershoot(
                cursor,
                start,
                targetX,
                targetY,
                distance,
                applyJitter,
                pathOptions
            );
            break;
        case 'stopped':
            await _moveStopped(cursor, start, targetX, targetY, distance, applyJitter, pathOptions);
            break;
        case 'muscle':
            await _moveMuscle(cursor, targetX, targetY, persona, applyJitter);
            break;
        case 'bezier':
            await runStandard();
            break;
        default:
            await runStandard();
            break;
    }

    // Final snap to target to ensure accuracy
    await cursor.move(targetX, targetY);

    // Optional correction movement after reaching target (scaled by precision)
    if (useCorrection) {
        const accuracy = persona.precision ?? 0.8;
        const correctionX = targetX + mathUtils.randomInRange(-3, 3) * (1 - accuracy);
        const correctionY = targetY + mathUtils.randomInRange(-3, 3) * (1 - accuracy);
        await cursor.move(correctionX, correctionY);
    }
}

/**
 * Arc path - curved movement.
 */
async function _moveArc(cursor, start, targetX, targetY, distance, applyJitter) {
    // Calculate arc control point
    const midX = (start.x + targetX) / 2;
    const midY = (start.y + targetY) / 2 - distance * 0.3 * (Math.random() > 0.5 ? 1 : -1);

    // Two-step arc via control point
    // const midPoint = { x: midX, y: midY };
    await applyJitter(midX, midY);
    await applyJitter(targetX, targetY);
}

/**
 * Zigzag path - slight back-and-forth.
 */
async function _moveZigzag(cursor, start, targetX, targetY, distance, applyJitter) {
    const steps = 4;
    const zigzagAmount = distance * 0.1;

    for (let i = 1; i < steps; i++) {
        const progress = i / steps;
        const baseX = start.x + (targetX - start.x) * progress;
        const baseY = start.y + (targetY - start.y) * progress;

        // Perpendicular offset for zigzag
        const perpX = (-(targetY - start.y) / distance) * zigzagAmount * (i % 2 === 0 ? 1 : -1);
        const perpY = ((targetX - start.x) / distance) * zigzagAmount * (i % 2 === 0 ? 1 : -1);

        await applyJitter(baseX + perpX, baseY + perpY);
    }
}

/**
 * Overshoot path - go past target, come back.
 */
async function _moveOvershoot(cursor, start, targetX, targetY, distance, applyJitter, pathOptions) {
    const overshootScale = 1 + (pathOptions.overshootDistance || 20) / 100;
    const overshootX = start.x + (targetX - start.x) * overshootScale;
    const overshootY = start.y + (targetY - start.y) * overshootScale;

    // Move to overshoot point
    await applyJitter(overshootX, overshootY);

    // Brief pause
    await new Promise((r) => setTimeout(r, mathUtils.randomInRange(50, 150)));

    // Move back to actual target
    await applyJitter(targetX, targetY);
}

/**
 * Stopped path - micro-stops along the way.
 */
async function _moveStopped(cursor, start, targetX, targetY, distance, applyJitter, pathOptions) {
    const stops = pathOptions.stops || 3;

    for (let i = 1; i <= stops; i++) {
        const progress = i / stops;
        const x = start.x + (targetX - start.x) * progress;
        const y = start.y + (targetY - start.y) * progress;

        await applyJitter(x, y);

        // Brief stop at each point
        if (i < stops) {
            await new Promise((r) => setTimeout(r, mathUtils.randomInRange(30, 80)));
        }
    }
}

/**
 * Muscle path - PID-driven biologically-modeled movement.
 * Creates unique acceleration/deceleration signatures.
 */
async function _moveMuscle(cursor, targetX, targetY, persona) {
    const start = cursor.previousPos || { x: 0, y: 0 };
    const model = persona.muscleModel || { Kp: 0.8, Ki: 0.01, Kd: 0.2 };

    // State for X and Y controllers
    const stateX = { pos: start.x, integral: 0, prevError: 0 };
    const stateY = { pos: start.y, integral: 0, prevError: 0 };

    const maxSteps = 20; // Safety break
    const tolerance = 2.0; // px

    for (let i = 0; i < maxSteps; i++) {
        const nextX = mathUtils.pidStep(stateX, targetX, model);
        const nextY = mathUtils.pidStep(stateY, targetY, model);

        // Bounds check - prevent going off-screen
        const boundedX = Math.max(0, Math.min(1280, nextX));
        const boundedY = Math.max(0, Math.min(720, nextY));

        logger.debug(
            `Muscle Step ${i}: Moving to (${Math.round(boundedX)}, ${Math.round(boundedY)}) distance: ${Math.round(Math.sqrt(Math.pow(targetX - stateX.pos, 2) + Math.pow(targetY - stateY.pos, 2)))}px`
        );

        // Direct mouse move with bounds
        await cursor.page.mouse.move(boundedX, boundedY);
        cursor.previousPos = { x: boundedX, y: boundedY };

        const distToTarget = Math.sqrt(
            Math.pow(targetX - boundedX, 2) + Math.pow(targetY - boundedY, 2)
        );
        if (distToTarget < tolerance) {
            logger.debug(`Muscle Target reached within ${tolerance}px tolerance`);
            logger.debug(`Muscle Final distance from target: ${Math.round(distToTarget)}px`);
            logger.debug(`Muscle Target was (${Math.round(targetX)}, ${Math.round(targetY)})`);
            logger.debug(
                `Muscle Final position (${Math.round(boundedX)}, ${Math.round(boundedY)})`
            );
            logger.debug(
                `Muscle X diff: ${Math.round(targetX - boundedX)}px, Y diff: ${Math.round(targetY - boundedY)}px`
            );
            break;
        }

        // Minimal delay for 2-3 second target
        const stepDelay = Math.max(1, 3 / persona.speed);
        await new Promise((r) => setTimeout(r, stepDelay));
    }
}

/**
 * Move cursor up by relative pixels.
 * @param {number} distance - Pixels to move up
 * @returns {Promise<void>}
 */
export async function up(distance) {
    const cursor = getCursor();
    const currentPos = cursor.previousPos || { x: 0, y: 0 };
    const targetY = Math.max(0, currentPos.y - distance);
    await cursor.move(currentPos.x, targetY);
}

/**
 * Move cursor down by relative pixels.
 * @param {number} distance - Pixels to move down
 * @returns {Promise<void>}
 */
export async function down(distance) {
    const cursor = getCursor();
    const currentPos = cursor.previousPos || { x: 0, y: 0 };
    const targetY = currentPos.y + distance;
    await cursor.move(currentPos.x, targetY);
}

/**
 * Start background micro-fidgeting (idle tremors).
 * Performs subtle 1-2px movements when the cursor is idle.
 * @returns {void}
 */
export function startFidgeting() {
    setSessionInterval(
        FIDGET_INTERVAL_NAME,
        async () => {
            try {
                const cursor = getCursor();
                if (!cursor) return;

                const currentPos = cursor.previousPos || { x: 0, y: 0 };
                const tremorAmplitude = 1.0; // 1px breathing

                const jitterX = mathUtils.gaussian(0, tremorAmplitude);
                const jitterY = mathUtils.gaussian(0, tremorAmplitude);

                // Subtle move without updating previousPos if possible,
                // but GhostCursor usually updates it. That's fine.
                await cursor.move(currentPos.x + jitterX, currentPos.y + jitterY);
            } catch (_e) {
                // Ignore (likely page navigation or context loss)
            }
        },
        mathUtils.randomInRange(3000, 8000)
    ); // Every few seconds
}

/**
 * Stop micro-fidgeting.
 * @returns {void}
 */
export function stopFidgeting() {
    clearSessionInterval(FIDGET_INTERVAL_NAME);
}
