/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * Simple dive queue test
 */

import { DiveQueue } from '../../utils/async-queue.js';

async function testDiveQueue() {
    console.log('Testing DiveQueue...\n');

    const queue = new DiveQueue({
        maxConcurrent: 1,
        maxQueueSize: 10,
        defaultTimeout: 5000,
        fallbackEngagement: true,
    });

    // Test 1: Normal completion
    console.log('Test 1: Normal completion (should succeed)');
    const result1 = await queue.addDive(
        async () => {
            console.log('  - Dive function executing...');
            await new Promise((resolve) => setTimeout(resolve, 1000));
            console.log('  - Dive function completed');
            return { success: true };
        },
        async () => {
            console.log('  - Fallback engaged');
            return { fallback: true };
        },
        { timeout: 3000, taskName: 'test1' }
    );
    console.log('  Result:', result1.success ? 'SUCCESS' : 'FAILED');

    // Test 2: Timeout
    console.log('\nTest 2: Timeout (should trigger fallback)');
    const result2 = await queue.addDive(
        async () => {
            console.log('  - Dive function executing (will take 5s)...');
            await new Promise((resolve) => setTimeout(resolve, 5000));
            console.log('  - Dive function completed');
            return { success: true };
        },
        async () => {
            console.log('  - Fallback engaged');
            return { fallback: true };
        },
        { timeout: 2000, taskName: 'test2' }
    );
    console.log('  Result:', result2.fallbackUsed ? 'FALLBACK USED' : 'FAILED');

    // Test 3: Quick mode
    console.log('\nTest 3: Quick mode timeout');
    queue.enableQuickMode();
    console.log('  Quick mode enabled, defaultTimeout:', queue.defaultTimeout);

    const result3 = await queue.addDive(
        async () => {
            console.log('  - Dive function executing (will take 5s)...');
            await new Promise((resolve) => setTimeout(resolve, 5000));
            return { success: true };
        },
        async () => {
            console.log('  - Fallback engaged');
            return { fallback: true };
        },
        { timeout: queue.defaultTimeout, taskName: 'test3' }
    );
    console.log('  Result:', result3.fallbackUsed ? 'FALLBACK USED' : 'SUCCESS');

    console.log('\n=== Tests Complete ===');
}

testDiveQueue().catch(console.error);
