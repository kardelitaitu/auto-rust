/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Universal Agent Task
 * The entry point for autonomous goal execution.
 * Usage: node main.js agent --goal="Your goal here"
 */

import { createLogger } from '../api/core/logger.js';
import AgentCortex from '../core/agent-cortex.js';
import VisionPackager from '../core/vision-packager.js';
import HumanizerEngine from '../core/humanizer-engine.js';
import { takeScreenshot } from '../utils/screenshot.js';
import { scrollRandom } from '../utils/scroll-helper.js';

const logger = createLogger('agent.js');

async function agent(page, payload) {
    const { browserInfo = 'unknown', goal = 'Browse blindly', steps = [] } = payload;
    logger.info(`[${browserInfo}] 🧠 STARTING UNIVERSAL AGENT. Goal: "${goal}"`);

    // Initialize Core Modules
    const cortex = new AgentCortex(browserInfo, goal, steps);
    const visionPackager = new VisionPackager();
    const humanizer = new HumanizerEngine();

    let turn = 0;
    const MAX_STEPS = 15;

    try {
        while (turn < MAX_STEPS) {
            turn++;
            logger.info(`[${browserInfo}] --- TURN ${turn} ---`);

            // 1. Observe (Vision Only)
            logger.info(`[${browserInfo}] 👀 Observing...`);

            let visionPacket;
            try {
                visionPacket = await visionPackager.captureWithROI(
                    page,
                    `${browserInfo}_${Date.now()}`
                );
            } catch (e) {
                logger.error(`[${browserInfo}] Vision failed: ${e.message}`);
                break;
            }

            // 2. Orient (Planner)
            const semanticTree = {}; // Mock empty tree.
            const actionPlan = await cortex.planNextStep(visionPacket, semanticTree);

            // 3. CHECK TERMINATION
            // Check if the plan itself is a terminate action or if first action is terminate
            if (
                actionPlan.type === 'terminate' ||
                (actionPlan.actions && actionPlan.actions[0]?.type === 'terminate')
            ) {
                const reason =
                    actionPlan.reason || actionPlan.actions?.[0]?.reason || actionPlan.description;
                logger.success(`[${browserInfo}] 🏁 GOAL ACHIEVED: ${reason}`);
                break;
            }

            // 4. EXECUTE ALL ACTIONS IN PLAN
            // The LLM plans multiple actions (e.g. click, type, press)
            // We execute them all sequentially before re-planning
            const actionsToExecute = actionPlan.actions || [actionPlan];

            for (const action of actionsToExecute) {
                let success = true;
                let resultMessage = 'Executed';

                try {
                    logger.info(
                        `[${browserInfo}] ▶ ACT: ${action.type} (${action.description || 'no description'})`
                    );

                    if (action.type === 'click') {
                        const { x, y, description } = action;

                        if (x === undefined || y === null) {
                            logger.error(`[${browserInfo}] ❌ Click missing coordinates`);
                            cortex.recordResult(action, false, 'Missing coordinates');
                            continue;
                        }

                        logger.info(`[${browserInfo}] 🖱️ Clicking at (${x}, ${y}): ${description}`);

                        // Human-like mouse movement to coordinates
                        const start = { x: Math.random() * 100, y: Math.random() * 100 };
                        const target = { x: parseFloat(x), y: parseFloat(y) };

                        const path = humanizer.generateMousePath(start, target);
                        for (const pt of path.points) {
                            await page.mouse.move(pt.x, pt.y);
                        }

                        // Click
                        await page.mouse.down();
                        await page.waitForTimeout(Math.random() * 100 + 50);
                        await page.mouse.up();

                        // Wait for focus to settle
                        await page.waitForTimeout(300);

                        logger.success(`[${browserInfo}] ✓ Clicked at (${x}, ${y})`);
                    } else if (action.type === 'type') {
                        const { text, description } = action;
                        logger.info(
                            `[${browserInfo}] ⌨️ Typing "${text}": ${description || 'input'}`
                        );

                        // Type at current cursor position (after clicking)
                        // const timings = humanizer.generateKeystrokeTiming(text);
                        for (const char of text) {
                            await page.keyboard.type(char);
                            await page.waitForTimeout(Math.random() * 100 + 30);
                        }
                        logger.success(`[${browserInfo}] ✓ Typed "${text}"`);
                    } else if (action.type === 'press') {
                        const { key } = action;
                        const keyName = key || 'Enter';
                        logger.info(`[${browserInfo}] ⌨️ Pressing key: ${keyName}`);

                        await page.keyboard.press(keyName);
                        await page.waitForTimeout(Math.random() * 200 + 100);
                        logger.success(`[${browserInfo}] ✓ Pressed ${keyName}`);
                    } else if (action.type === 'navigate') {
                        await page.goto(action.url, { waitUntil: 'domcontentloaded' });
                        logger.info(`[${browserInfo}] ⏳ Stabilization Wait (2000ms)...`);
                        await page.waitForTimeout(2000);
                    } else if (action.type === 'wait') {
                        await page.waitForTimeout(action.duration || 2000);
                    } else if (action.type === 'scroll') {
                        await scrollRandom(
                            page,
                            action.direction === 'up' ? -300 : 300,
                            action.direction === 'up' ? -300 : 300
                        );
                        await page.waitForTimeout(500);
                    } else {
                        logger.warn(`[${browserInfo}] Unknown interaction: ${action.type}`);
                    }
                } catch (actErr) {
                    success = false;
                    resultMessage = actErr.message;
                    logger.error(`[${browserInfo}] ❌ Action Failed: ${actErr.message}`);
                }

                // Record result
                cortex.recordResult(action, success, resultMessage);

                // Wait between actions
                await page.waitForTimeout(500);
            }

            // 5. WAIT (Stabilize after all actions)
            await page.waitForTimeout(1000);
        }

        if (turn >= MAX_STEPS) {
            logger.warn(`[${browserInfo}] 🛑 Reached Max Steps (${MAX_STEPS}). Aborting.`);
        }

        // Final Audit
        await takeScreenshot(page, browserInfo, '-Final');
    } catch (err) {
        logger.error(`[${browserInfo}] CRITICAL AGENT FAILURE:`, err);
        throw err;
    }
}

export default agent;
