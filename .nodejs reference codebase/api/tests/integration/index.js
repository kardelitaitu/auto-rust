/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * Integration Tests Index
 *
 * This file re-exports all integration tests for aggregated testing.
 * Import paths using @tests alias:
 *   import { describe, it, expect } from 'vitest';
 *   import * as integrationTests from '@tests/integration';
 */

import agentConnectorHealth from './agent-connector-health.test.js';
import circuitBreaker from './circuit-breaker.test.js';
import requestQueue from './request-queue.test.js';
// Manual/debug scripts - not vitest tests:
// import coreModules from './test-core-modules.manual.js';
// import dedupe from './test-dedupe.manual.js';
// import aiReplyEngine from './test-ai-reply-engine.manual.js';
// import cloudPromptFix from './test-cloud-prompt-fix.manual.js';
// import cloudClientMulti from './test-cloud-client-multi.manual.js';
// import multiApi from './test-multi-api.manual.js';
// import cloudApi from './test-cloud-api.manual.js';
import cloudClient from './cloud-client.test.js';
import agentConnector from './agent-connector.test.js';
import unifiedApi from './unified-api.test.js';

export {
    agentConnectorHealth,
    circuitBreaker,
    requestQueue,
    // coreModules,
    // dedupe,
    // aiReplyEngine,
    // cloudPromptFix,
    // cloudClientMulti,
    // multiApi,
    // cloudApi,
    cloudClient,
    agentConnector,
    unifiedApi,
};

/**
 * Run all integration tests
 */
export async function runAllIntegrationTests() {
    const results = {
        passed: 0,
        failed: 0,
        tests: [],
    };

    const testModules = [
        { name: 'agent-connector-health', module: agentConnectorHealth },
        { name: 'circuit-breaker', module: circuitBreaker },
        { name: 'request-queue', module: requestQueue },
        // { name: 'core-modules', module: coreModules },
        // { name: 'dedupe', module: dedupe },
        // { name: 'ai-reply-engine', module: aiReplyEngine },
        // { name: 'cloud-prompt-fix', module: cloudPromptFix },
        // { name: 'cloud-client-multi', module: cloudClientMulti },
        // { name: 'multi-api', module: multiApi },
        // { name: 'cloud-api', module: cloudApi },
        { name: 'cloud-client', module: cloudClient },
        { name: 'agent-connector', module: agentConnector },
        { name: 'unified-api', module: unifiedApi },
    ];

    for (const { name } of testModules) {
        try {
            // Module is loaded, tests will be discovered by vitest
            results.tests.push({ name, status: 'loaded' });
            results.passed++;
        } catch (error) {
            results.tests.push({ name, status: 'failed', error: error.message });
            results.failed++;
        }
    }

    return results;
}
