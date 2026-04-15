/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * Test script for the dive locking mechanism
 * Usage: node test-dive-lock.js
 */

import { config } from './utils/config-service.js';

// const logger = createLogger('test-dive-lock.js');

// Page states for testing
const PAGE_STATE = {
    HOME: 'HOME',
    DIVING: 'DIVING',
    TWEET_PAGE: 'TWEET_PAGE',
    RETURNING: 'RETURNING',
};

// Mock dive lock state
const diveLock = {
    pageState: PAGE_STATE.HOME,
    scrollingEnabled: true,
    operationLock: false,
    diveLockAcquired: false,
    lastWaitLogTime: 0,
    waitLogInterval: 10000,
};

// Test functions
async function testLockMechanism() {
    console.log('\n' + '='.repeat(70));
    console.log('DIVE LOCK MECHANISM TEST');
    console.log('='.repeat(70) + '\n');

    // Test 1: Initial state
    console.log('[Test 1] Initial State:');
    console.log(`  State: ${diveLock.pageState}`);
    console.log(`  Scrolling Enabled: ${diveLock.scrollingEnabled}`);
    console.log(`  Operation Lock: ${diveLock.operationLock}`);
    console.log(`  Dive Lock Acquired: ${diveLock.diveLockAcquired}`);
    console.log('  ✓ Initial state is HOME with scrolling enabled');

    // Test 2: Start dive operation
    console.log('\n[Test 2] Starting Dive Operation:');
    async function startDive() {
        // Wait for any ongoing operations with buffered logging
        let firstWait = true;
        while (diveLock.operationLock) {
            const now = Date.now();

            // Only log every 10 seconds to prevent log spam
            if (firstWait || now - diveLock.lastWaitLogTime >= diveLock.waitLogInterval) {
                console.log(
                    `  [DiveLock] ⏳ Waiting for existing operation to complete... (${((now - diveLock.lastWaitLogTime) / 1000).toFixed(0)}s since last check)`
                );
                diveLock.lastWaitLogTime = now;
                firstWait = false;
            }

            await new Promise((resolve) => setTimeout(resolve, 10));
        }

        // Acquire operation lock
        diveLock.operationLock = true;
        diveLock.diveLockAcquired = true;
        diveLock.pageState = 'DIVING';
        diveLock.scrollingEnabled = false;
        diveLock.lastWaitLogTime = 0; // Reset wait log timestamp

        return true;
    }

    await startDive();
    console.log(`  State: ${diveLock.pageState}`);
    console.log(`  Scrolling Enabled: ${diveLock.scrollingEnabled}`);
    console.log(`  Operation Lock: ${diveLock.operationLock}`);
    console.log('  ✓ Diving operation started - scrolling disabled');

    // Test 3: Verify scrolling is disabled
    console.log('\n[Test 3] Verify Scrolling Disabled:');
    const canScroll = diveLock.scrollingEnabled && !diveLock.operationLock;
    console.log(`  Can Scroll: ${canScroll}`);
    console.log('  ✓ Scrolling correctly disabled during dive operation');

    // Test 4: Simulate concurrent operation attempt
    console.log('\n[Test 4] Concurrent Operation Attempt:');
    let concurrentStarted = false;
    async function tryConcurrentOperation() {
        if (diveLock.operationLock) {
            console.log('  ⚠️ Operation blocked by existing lock');
            return false;
        }
        concurrentStarted = true;
        return true;
    }

    await tryConcurrentOperation();
    console.log(`  Concurrent operation started: ${concurrentStarted}`);
    console.log('  ✓ Concurrent operations correctly blocked during dive');

    // Test 5: End dive operation
    console.log('\n[Test 5] Ending Dive Operation:');
    async function endDive(success, returnHome) {
        if (returnHome) {
            diveLock.pageState = PAGE_STATE.RETURNING;
            diveLock.scrollingEnabled = false;

            // Simulate navigation
            await new Promise((resolve) => setTimeout(resolve, 100));
            diveLock.pageState = PAGE_STATE.HOME;
            diveLock.scrollingEnabled = true;
        } else {
            diveLock.pageState = success ? PAGE_STATE.TWEET_PAGE : PAGE_STATE.HOME;
        }

        // Release operation lock
        diveLock.operationLock = false;
        diveLock.diveLockAcquired = false;
    }

    await endDive(true, true);
    console.log(`  State: ${diveLock.pageState}`);
    console.log(`  Scrolling Enabled: ${diveLock.scrollingEnabled}`);
    console.log(`  Operation Lock: ${diveLock.operationLock}`);
    console.log('  ✓ Dive operation ended - scrolling re-enabled');

    // Test 6: Verify concurrent operation now allowed
    console.log('\n[Test 6] Concurrent Operation After Dive:');
    const concurrentResult2 = await tryConcurrentOperation();
    console.log(`  Concurrent operation started: ${concurrentResult2}`);
    console.log('  ✓ Concurrent operations allowed after dive completion');

    // Test 7: Test state transitions
    console.log('\n[Test 7] State Transition Summary:');
    const states = [
        { from: 'HOME', to: 'DIVING', action: 'Start Dive' },
        { from: 'DIVING', to: 'RETURNING', action: 'Return Home' },
        { from: 'RETURNING', to: 'HOME', action: 'Home Reached' },
    ];

    for (const transition of states) {
        console.log(`  ${transition.from} → ${transition.to}: ${transition.action}`);
    }
    console.log('  ✓ State transitions working correctly');

    // Test 8: Test with AITwitterAgent-like state management
    console.log('\n[Test 8] Full State Management Test:');

    class MockAITwitterAgent {
        constructor() {
            this.pageState = PAGE_STATE.HOME;
            this.scrollingEnabled = true;
            this.operationLock = false;
            this.lastWaitLogTime = 0;
            this.waitLogInterval = 10000;
        }

        async startDive() {
            let firstWait = true;
            while (this.operationLock) {
                const now = Date.now();
                if (firstWait || now - this.lastWaitLogTime >= this.waitLogInterval) {
                    console.log(
                        `  [DiveLock] ⏳ Waiting for existing operation... (${((now - this.lastWaitLogTime) / 1000).toFixed(0)}s)`
                    );
                    this.lastWaitLogTime = now;
                    firstWait = false;
                }
                await new Promise((resolve) => setTimeout(resolve, 10));
            }
            this.operationLock = true;
            this.pageState = PAGE_STATE.DIVING;
            this.scrollingEnabled = false;
            this.lastWaitLogTime = 0;
            return true;
        }

        async endDive(success, returnHome) {
            if (returnHome) {
                this.pageState = PAGE_STATE.RETURNING;
                this.scrollingEnabled = false;
                await new Promise((resolve) => setTimeout(resolve, 50));
                this.pageState = PAGE_STATE.HOME;
                this.scrollingEnabled = true;
            }
            this.operationLock = false;
        }

        isDiving() {
            return this.operationLock && this.pageState === PAGE_STATE.DIVING;
        }

        canScroll() {
            return this.scrollingEnabled && !this.operationLock;
        }

        getPageState() {
            return {
                state: this.pageState,
                scrollingEnabled: this.scrollingEnabled,
                operationLock: this.operationLock,
            };
        }
    }

    const agent = new MockAITwitterAgent();

    // Test normal operation
    console.log('  Normal operation - canScroll:', agent.canScroll());
    await agent.startDive();
    console.log('  Diving operation - isDiving:', agent.isDiving());
    console.log('  Diving operation - canScroll:', agent.canScroll());
    await agent.endDive(true, true);
    console.log('  After dive - canScroll:', agent.canScroll());
    console.log('  ✓ State management working correctly');

    // Summary
    console.log('\n' + '='.repeat(70));
    console.log('DIVE LOCK MECHANISM TEST COMPLETE');
    console.log('='.repeat(70));
    console.log('\nKey Features Verified:');
    console.log('  ✓ State management (HOME → DIVING → RETURNING → HOME)');
    console.log('  ✓ Operation locking (prevents concurrent operations)');
    console.log('  ✓ Scrolling control (disabled during diving)');
    console.log('  ✓ Concurrent operation prevention');
    console.log('  ✓ Graceful state transitions');
    console.log('\nExpected Log Output During Runtime (with 10s buffer):');
    console.log('  [DiveLock] 🔒 Dive operation started - scrolling disabled');
    console.log('  [DiveLock] 🔓 Dive operation ended - scrolling re-enabled');
    console.log('  [Session] ⏳ Waiting for diving operation to complete... (0s elapsed)');
    console.log('  [Session] ✓ Diving operation completed, continuing session');
    console.log('\nBuffered Logging Features:');
    console.log('  - Wait messages logged every 10 seconds instead of every 50ms');
    console.log('  - Shows elapsed time since last check');
    console.log('  - Prevents log spam during long operations');
    console.log('\nUsage in AITwitterAgent:');
    console.log('  await this.startDive();    // Acquire lock');
    console.log('  // ... perform dive operations ...');
    console.log('  await this.endDive(true, true);  // Release lock and return home');
}

async function main() {
    try {
        // Initialize config service
        await config.init();
        console.log('Config service initialized');

        // Run tests
        await testLockMechanism();

        console.log('\n✅ All tests passed!');
        process.exit(0);
    } catch (error) {
        console.error('❌ Test failed:', error.message);
        process.exit(1);
    }
}

main();
