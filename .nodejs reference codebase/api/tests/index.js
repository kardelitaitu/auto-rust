/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * Tests Index
 *
 * Main entry point for all tests in the project.
 * Use @tests alias to import from this directory.
 *
 * Structure:
 *   @tests/unit        - Unit tests (fast, isolated)
 *   @tests/integration - Integration tests (component interaction)
 *   @tests/edge-cases  - Edge case and boundary tests
 */

export * from './unit/index.js';
export * from './integration/index.js';
export * from './edge-cases/index.js';

/**
 * Run all tests in the project
 */
export async function runAllTests() {
    const results = {
        unit: await import('./unit/index.js').then((m) => m.runAllUnitTests()),
        integration: await import('./integration/index.js').then((m) => m.runAllIntegrationTests()),
        edgeCases: await import('./edge-cases/index.js').then((m) => m.runAllEdgeCaseTests()),
    };

    const total = results.unit.passed + results.integration.passed + results.edgeCases.passed;
    const failed = results.unit.failed + results.integration.failed + results.edgeCases.failed;

    return {
        ...results,
        total,
        failed,
        success: failed === 0,
    };
}

/**
 * Test statistics
 */
export const testStats = {
    unit: {
        count: 11, // 4 original + 7 new unit tests
        category: 'Unit Tests',
    },
    integration: {
        count: 10, // 4 original + 6 new integration tests
        category: 'Integration Tests',
    },
    edgeCases: {
        count: 4, // 2 original + 2 new edge case tests
        category: 'Edge Case Tests',
    },
    get total() {
        return this.unit.count + this.integration.count + this.edgeCases.count;
    },
};
