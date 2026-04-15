/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Simple test script to verify core modules.
 * @module tests/test-core-modules
 */

import { createLogger } from '../utils/logger.js';
import StateManager from '../core/state-manager.js';
import IntentClassifier from '../core/intent-classifier.js';
import CloudClient from '../core/cloud-client.js';
import LocalClient from '../core/local-client.js';
import AgentConnector from '../core/agent-connector.js';
import HumanizerEngine from '../core/humanizer-engine.js';
import IdleGhosting from '../core/idle-ghosting.js';
import AuditVerifier from '../core/audit-verifier.js';

const logger = createLogger('test-core-modules.js');

/**
 * Test all core modules
 */
(async () => {
    logger.info('=== Testing Core Modules ===');

    try {
        // Test StateManager
        logger.info('[Test] StateManager...');
        const stateManager = new StateManager();
        stateManager.addBreadcrumb('test-session', {
            action: 'navigate',
            target: 'https://example.com',
            success: true,
        });
        const breadcrumbs = stateManager.getBreadcrumbs('test-session');
        logger.success(`[Test] StateManager: ${breadcrumbs.length} breadcrumb(s) recorded`);

        // Test IntentClassifier
        logger.info('[Test] IntentClassifier...');
        const classifier = new IntentClassifier();
        const classification = classifier.classify({
            action: 'click',
            payload: { target: 'button' },
        });
        logger.success(
            `[Test] IntentClassifier: Routed to ${classification.destination} (confidence: ${classification.confidenceScore}%)`
        );

        // Test CloudClient (no actual API call)
        logger.info('[Test] CloudClient...');
        const cloudClient = new CloudClient();
        cloudClient.getStats();
        logger.success(`[Test] CloudClient initialized`);

        // Test LocalClient (stub)
        logger.info('[Test] LocalClient...');
        const localClient = new LocalClient();
        const localStats = localClient.getStats();
        logger.success(
            `[Test] LocalClient (stub mode): ${localStats.stubMode ? 'STUB' : 'ACTIVE'}`
        );

        // Test HumanizerEngine
        logger.info('[Test] HumanizerEngine...');
        const humanizer = new HumanizerEngine();
        const path = humanizer.generateMousePath({ x: 100, y: 100 }, { x: 500, y: 300 });
        logger.success(
            `[Test] HumanizerEngine: Generated ${path.points.length}-point path, duration: ${path.duration}ms`
        );

        // Test IdleGhosting
        logger.info('[Test] IdleGhosting...');
        const idleGhosting = new IdleGhosting();
        const idleStats = idleGhosting.getStats();
        logger.success(`[Test] IdleGhosting: Wiggle frequency: ${idleStats.wiggleFrequency}ms`);

        // Test AuditVerifier
        logger.info('[Test] AuditVerifier...');
        const auditor = new AuditVerifier();
        auditor.getStats();
        logger.success(`[Test] AuditVerifier: Reliability metric initialized`);

        // Test AgentConnector (this integrates all the above)
        logger.info('[Test] AgentConnector...');
        const agentConnector = new AgentConnector();
        // Don't make actual request, just verify initialization
        const agentStats = agentConnector.getStats();
        logger.success(
            `[Test] AgentConnector: ${agentStats.localAvailable ? 'Local available' : 'Cloud-only mode'}`
        );

        logger.info('=== All Core Module Tests Passed ===');

        // Log summary
        logger.info('\nModule Summary:');
        logger.info(`- StateManager: ✓`);
        logger.info(`- IntentClassifier: ✓`);
        logger.info(`- CloudClient: ✓`);
        logger.info(`- LocalClient: ✓ (stub)`);
        logger.info(`- HumanizerEngine: ✓`);
        logger.info(`- IdleGhosting: ✓`);
        logger.info(`- AuditVerifier: ✓`);
        logger.info(`- AgentConnector: ✓`);

        process.exit(0);
    } catch (error) {
        logger.error('Module test failed:', error);
        process.exit(1);
    }
})();
