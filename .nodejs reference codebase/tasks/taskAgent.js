/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { run as runAgent } from './runAgent.js';
import { takeScreenshot } from '../utils/screenshot.js';

export async function run(page, args, config = {}) {
    console.log('Starting runAgent.js...');

    const goals = [
        "Navigate to https://www.google.com/ and wait for the search bar type 'boneka kayu', press Enter",
        'Navigate to https://www.google.com/, DO NOT interact with the search bar',
    ];

    for (const [index, goal] of goals.entries()) {
        if (index > 0) {
            console.log('\n🔄 Resetting browser to about:blank to force navigation...');
            await page.goto('about:blank');
        }
        console.log(`\n--- Executing Goal: ${goal} ---`);
        const agent = await runAgent(page, [goal], config);

        // Auto-Screenshot
        console.log(`📸 Capturing Post-Task Screenshot for Goal ${index + 1}...`);
        const sessionId = config.sessionId || 'unknown';
        await takeScreenshot(page, sessionId, `Task-${index + 1}`);

        // Log Context Usage
        const usage = agent.getUsageStats();
        console.log(`\n📊 Context Usage for Goal ${index + 1}:`);
        console.log(`   - Steps: ${usage.steps}/${usage.maxSteps}`);
        console.log(`   - Estimated Tokens: ${usage.estimatedTokens.toLocaleString()}`);
        console.log(`   - History Size: ${usage.historySize} messages`);
    }

    console.log('Workflow complete.');
}
