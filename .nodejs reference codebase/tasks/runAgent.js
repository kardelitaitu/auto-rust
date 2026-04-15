/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Task wrapper for Local Agent.
 * Allows running the agent as a standard task with configurable delays.
 * @module tasks/runAgent
 */

import Agent from '../local-agent/core/agent.js';
// import { createLogger } from '../api/core/logger.js';

// const logger = createLogger('runAgent.js');

/**
 * Runs the agent task.
 * @param {object} page - The Playwright page instance.
 * @param {string[]} args - Command line arguments (Goal is first arg)
 */
export async function run(page, args = [], config = {}) {
    const goal = args[0];
    if (!goal) {
        throw new Error('Goal is required for runAgent task (Pass it as an argument).');
    }

    const agent = new Agent(page, goal, {
        stepDelay: 2000,
        ...config,
    });

    await agent.run();
    return agent; // Return agent to allow caller to access usage stats
}
