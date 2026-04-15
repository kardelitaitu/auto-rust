/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Task module for executing weather search operations using the runAgent.
 * @module taskAgent2
 */

import { run as runAgent } from './runAgent.js';

/**
 * Executes a weather search task for Jakarta using the runAgent.
 * @param {Object} page - Playwright Page object for browser interaction
 * @param {string[]} args - Arguments to pass to the runAgent (instructions/actions)
 * @param {Object} [config={}] - Configuration object for the agent
 * @returns {Promise<void>} Resolves when the task completes
 */
export async function run(page, args, config = {}) {
    console.log('Starting taskAgent2 - Weather Search...');

    console.log('Goal: Search for weather in Jakarta');
    await runAgent(page, ["Go to Google and search for 'weather in jakarta'"], config);

    console.log('taskAgent2 complete.');
}
