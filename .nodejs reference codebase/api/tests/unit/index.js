/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * Unit Tests Index
 *
 * This file re-exports all unit tests for aggregated testing.
 * Import paths using @tests alias:
 *   import { describe, it, expect } from 'vitest';
 *   import * as unitTests from '@tests/unit';
 */

import aiTwitterAgent from './ai-twitterAgent.test.js';
import aiTwitterActivity from './ai-twitter-activity.test.js';
import asyncQueue from './async-queue.test.js';
import configService from './config-service.test.js';
import engagementLimits from './engagement-limits.test.js';
import humanInteraction from './human-interaction.test.js';
// These are manual/debug scripts, not vitest tests:
// import smartProb from './test-smart-prob.manual.js';
// import actionConfig from './test-action-config.manual.js';
// import actions from './test-actions.manual.js';
// import simpleDive from './test-simple-dive.manual.js';
// import humanMethods from './test-human-methods.manual.js';
// import modularMethods from './test-modular-methods.manual.js';
// import replyMethod from './test-reply-method.manual.js';

export {
    aiTwitterActivity,
    aiTwitterAgent,
    asyncQueue,
    configService,
    engagementLimits,
    humanInteraction,
    // smartProb,
    // actionConfig,
    // actions,
    // simpleDive,
    // humanMethods,
    // modularMethods,
    // replyMethod,
};

/**
 * Run all unit tests
 */
export async function runAllUnitTests() {
    const results = {
        passed: 0,
        failed: 0,
        tests: [],
    };

    const testModules = [
        { name: 'ai-twitter-activity', module: aiTwitterActivity },
        { name: 'ai-twitter-agent', module: aiTwitterAgent },
        { name: 'async-queue', module: asyncQueue },
        { name: 'config-service', module: configService },
        { name: 'engagement-limits', module: engagementLimits },
        { name: 'human-interaction', module: humanInteraction },
        // Manual/debug scripts - not vitest tests:
        // { name: 'smart-prob', module: smartProb },
        // { name: 'action-config', module: actionConfig },
        // { name: 'actions', module: actions },
        // { name: 'simple-dive', module: simpleDive },
        // { name: 'human-methods', module: humanMethods },
        // { name: 'modular-methods', module: modularMethods },
        // { name: 'reply-method', module: replyMethod },
    ];

    for (const { name, module: _module } of testModules) {
        try {
            results.tests.push({ name, status: 'loaded' });
            results.passed++;
        } catch (error) {
            results.tests.push({ name, status: 'failed', error: error.message });
            results.failed++;
        }
    }

    return results;
}
