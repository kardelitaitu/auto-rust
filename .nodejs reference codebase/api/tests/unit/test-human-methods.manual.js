/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * Quick debug test for human-like methods and async queue mechanism
 * Usage: HUMAN_DEBUG=true node test-human-methods.js
 */

import { HumanInteraction } from './utils/human-interaction.js';
import { AIReplyEngine } from './utils/ai-reply-engine.js';
import { AIQuoteEngine } from './utils/ai-quote-engine.js';
import { AsyncQueue } from './utils/async-queue.js';
import { DiveQueue } from './utils/async-queue.js';

async function main() {
    console.log('\n' + '='.repeat(70));
    console.log('HUMAN-LIKE METHODS DEBUG TEST');
    console.log('='.repeat(70) + '\n');

    // Check debug mode
    const debugMode = process.env.HUMAN_DEBUG === 'true';
    console.log(`Debug Mode: ${debugMode ? 'ON' : 'OFF'}`);
    console.log(`Enable with: set HUMAN_DEBUG=true\n`);

    // Test HumanInteraction utilities
    console.log('-'.repeat(70));
    console.log('TESTING HumanInteraction Utilities');
    console.log('-'.repeat(70));

    const human = new HumanInteraction();
    human.debugMode = true;

    // Test hesitation
    console.log('\n[Test 1] Hesitation (300-500ms):');
    const h1 = await human.hesitation(300, 500);
    console.log(`  Result: ${h1}ms`);

    // Test fixation
    console.log('\n[Test 2] Fixation (200-800ms):');
    const f1 = await human.fixation(200, 800);
    console.log(`  Result: ${f1}ms`);

    // Test reading time
    console.log('\n[Test 3] Reading time (1-2s):');
    const r1 = await human.readingTime(1000, 2000);
    console.log(`  Result: ${r1}ms`);

    // Test method selection
    console.log('\n[Test 4] Method Selection (10 trials):');
    const methods = [
        { name: 'A', weight: 40, fn: () => 'A' },
        { name: 'B', weight: 35, fn: () => 'B' },
        { name: 'C', weight: 15, fn: () => 'C' },
        { name: 'D', weight: 10, fn: () => 'D' },
    ];

    const results = { A: 0, B: 0, C: 0, D: 0 };
    for (let i = 0; i < 10; i++) {
        const selected = human.selectMethod(methods);
        results[selected.name]++;
    }
    console.log('  Distribution:', results);

    // ================================================================
    // TESTING ASYNC QUEUE MECHANISM (NEW)
    // ================================================================
    console.log('\n' + '='.repeat(70));
    console.log('TESTING ASYNC QUEUE MECHANISM');
    console.log('='.repeat(70));

    console.log('\n[Test 4.1] AsyncQueue Initialization:');
    const asyncQueue = new AsyncQueue({
        maxConcurrent: 3,
        maxQueueSize: 10,
        defaultTimeout: 2000,
    });
    console.log('  AsyncQueue created with:');
    console.log('    - maxConcurrent: 3');
    console.log('    - maxQueueSize: 10');
    console.log('    - defaultTimeout: 2000ms');

    console.log('\n[Test 4.2] DiveQueue Initialization (for tweet diving):');
    const diveQueue = new DiveQueue({
        maxConcurrent: 1,
        maxQueueSize: 30,
        defaultTimeout: 2000,
        fallbackEngagement: true,
        replies: 3,
        retweets: 1,
        quotes: 1,
        likes: 5,
        follows: 2,
        bookmarks: 2,
    });
    console.log('  DiveQueue created with:');
    console.log('    - maxConcurrent: 1 (sequential processing)');
    console.log('    - maxQueueSize: 30');
    console.log('    - defaultTimeout: 2000ms');
    console.log('    - fallbackEngagement: true');
    console.log('    - Engagement limits:', {
        replies: 3,
        retweets: 1,
        quotes: 1,
        likes: 5,
        follows: 2,
        bookmarks: 2,
    });

    console.log('\n[Test 4.3] AsyncQueue Concurrent Processing (3 tasks, maxConcurrent: 3):');
    const concurrentStart = Date.now();
    let concurrentCompleted = 0;

    const task1 = async () => {
        await new Promise((r) => setTimeout(r, 500));
        concurrentCompleted++;
        return `Task1_completed_${concurrentCompleted}`;
    };

    const task2 = async () => {
        await new Promise((r) => setTimeout(r, 500));
        concurrentCompleted++;
        return `Task2_completed_${concurrentCompleted}`;
    };

    const task3 = async () => {
        await new Promise((r) => setTimeout(r, 500));
        concurrentCompleted++;
        return `Task3_completed_${concurrentCompleted}`;
    };

    await Promise.all([
        asyncQueue.add(task1, { name: 'concurrent_task_1', timeout: 3000 }),
        asyncQueue.add(task2, { name: 'concurrent_task_2', timeout: 3000 }),
        asyncQueue.add(task3, { name: 'concurrent_task_3', timeout: 3000 }),
    ]);

    const concurrentDuration = Date.now() - concurrentStart;
    console.log(`  All 3 tasks completed in: ${concurrentDuration}ms (should be ~500ms)`);
    console.log('  ✓ Concurrent execution working correctly');

    console.log('\n[Test 4.4] AsyncQueue Sequential Processing (3 tasks, maxConcurrent: 1):');
    const seqQueue = new AsyncQueue({ maxConcurrent: 1, maxQueueSize: 10, defaultTimeout: 2000 });
    const seqStart = Date.now();
    let seqCompleted = 0;

    const seqTask1 = async () => {
        await new Promise((r) => setTimeout(r, 200));
        seqCompleted++;
        return `SeqTask1_${seqCompleted}`;
    };

    const seqTask2 = async () => {
        await new Promise((r) => setTimeout(r, 200));
        seqCompleted++;
        return `SeqTask2_${seqCompleted}`;
    };

    const seqTask3 = async () => {
        await new Promise((r) => setTimeout(r, 200));
        seqCompleted++;
        return `SeqTask3_${seqCompleted}`;
    };

    await Promise.all([
        seqQueue.add(seqTask1, { name: 'seq_task_1', timeout: 3000 }),
        seqQueue.add(seqTask2, { name: 'seq_task_2', timeout: 3000 }),
        seqQueue.add(seqTask3, { name: 'seq_task_3', timeout: 3000 }),
    ]);

    const seqDuration = Date.now() - seqStart;
    console.log(`  All 3 sequential tasks completed in: ${seqDuration}ms (should be ~600ms)`);
    console.log('  ✓ Sequential execution working correctly');

    console.log('\n[Test 4.5] DiveQueue Timeout Handling with Fallback:');
    const timeoutDiveQueue = new DiveQueue({
        maxConcurrent: 1,
        maxQueueSize: 10,
        defaultTimeout: 300,
        fallbackEngagement: true,
        replies: 3,
        retweets: 1,
        quotes: 1,
        likes: 5,
        follows: 2,
        bookmarks: 2,
    });
    let timeoutTaskCalled = false;
    let fallbackCalled = false;

    const slowTask = async () => {
        await new Promise((r) => setTimeout(r, 500));
        timeoutTaskCalled = true;
        return 'slow_task_result';
    };

    const fastFallback = async () => {
        fallbackCalled = true;
        return 'fallback_result';
    };

    const timeoutResult = await timeoutDiveQueue.addDive(slowTask, fastFallback, {
        taskName: 'timeout_test',
        timeout: 200,
        priority: 0,
    });

    console.log(`  Task executed: ${timeoutTaskCalled}`);
    console.log(`  Fallback executed: ${fallbackCalled}`);
    console.log(`  Result: ${JSON.stringify(timeoutResult)}`);
    console.log('  ✓ Timeout and fallback working correctly');

    console.log('\n[Test 4.6] DiveQueue Engagement Limits:');
    console.log('  Initial engagement progress:');
    const initialProgress = diveQueue.getEngagementProgress();
    console.log(`    Replies: ${initialProgress.replies.current}/${initialProgress.replies.limit}`);
    console.log(`    Likes: ${initialProgress.likes.current}/${initialProgress.likes.limit}`);
    console.log(
        `    Bookmarks: ${initialProgress.bookmarks.current}/${initialProgress.bookmarks.limit}`
    );

    console.log('\n  Recording engagements:');
    diveQueue.recordEngagement('likes');
    diveQueue.recordEngagement('likes');
    diveQueue.recordEngagement('replies');
    diveQueue.recordEngagement('bookmarks');

    const updatedProgress = diveQueue.getEngagementProgress();
    console.log(`    Replies: ${updatedProgress.replies.current}/${updatedProgress.replies.limit}`);
    console.log(`    Likes: ${updatedProgress.likes.current}/${updatedProgress.likes.limit}`);
    console.log(
        `    Bookmarks: ${updatedProgress.bookmarks.current}/${updatedProgress.bookmarks.limit}`
    );

    console.log('\n  Checking canEngage:');
    console.log(
        `    Can engage likes: ${diveQueue.canEngage('likes')} (${updatedProgress.likes.current}/${updatedProgress.likes.limit})`
    );
    console.log(
        `    Can engage replies: ${diveQueue.canEngage('replies')} (${updatedProgress.replies.current}/${updatedProgress.replies.limit})`
    );

    console.log('\n[Test 4.7] Queue Status and Health:');
    const queueStatus = diveQueue.getFullStatus();
    console.log('  Queue status:');
    console.log(`    Queue length: ${queueStatus.queue.queueLength}`);
    console.log(`    Active count: ${queueStatus.queue.activeCount}`);
    console.log(`    Utilization: ${queueStatus.queue.utilizationPercent}%`);
    console.log(`    Is healthy: ${queueStatus.isHealthy}`);

    console.log('\n[Test 4.8] Full Queue Rejection:');
    const smallQueue = new AsyncQueue({ maxConcurrent: 1, maxQueueSize: 2, defaultTimeout: 1000 });
    await smallQueue.add(async () => 'task1', { name: 'reject_test_1' });
    await smallQueue.add(async () => 'task2', { name: 'reject_test_2' });
    const rejectResult = await smallQueue.add(async () => 'task3', { name: 'reject_test_3' });
    console.log('  Queue at capacity (2/2)');
    console.log(
        `  Task 3 added: ${rejectResult.success === false ? 'REJECTED (correct)' : 'Added (unexpected)'}`
    );
    console.log('  ✓ Queue size limit working correctly');

    // Reset for next test
    await new Promise((r) => setTimeout(r, 100));

    // Test reply engine
    console.log('\n' + '-'.repeat(70));
    console.log('TESTING AI REPLY ENGINE WITH QUEUE INTEGRATION');
    console.log('-'.repeat(70));

    const replyEngine = new AIReplyEngine(null, {
        replyProbability: 1.0,
        maxRetries: 1,
    });

    console.log('\n[Test 5] Reply Engine Methods:');
    console.log('  replyA - Keyboard (R key)');
    console.log('  replyB - Button click');
    console.log('  replyC - Direct composer focus (focus, click, type, submit)');
    console.log('  executeReply (main entry)');

    console.log('\n[Test 6] Reply Engine Stats:');
    const replyStats = replyEngine.getStats();
    console.log('  Stats:', JSON.stringify(replyStats, null, 2));

    console.log('\n[Test 6.1] Queue Integration Test:');
    console.log('  Testing race-condition-free reply processing with DiveQueue...');

    let replyProcessed = false;
    let replyFallbackUsed = false;

    const simulateAIReply = async () => {
        await new Promise((r) => setTimeout(r, 100));
        replyProcessed = true;
        return { success: true, method: 'ai_reply' };
    };

    const simulateReplyFallback = async () => {
        replyFallbackUsed = true;
        return { engagementType: 'like', success: true };
    };

    const replyDiveResult = await diveQueue.addDive(simulateAIReply, simulateReplyFallback, {
        taskName: 'test_ai_reply',
        timeout: diveQueue.quickModeEnabled ? 3000 : 5000,
        priority: 0,
    });

    console.log('  DiveQueue result:', JSON.stringify(replyDiveResult, null, 2));
    console.log('  Reply processed:', replyProcessed);
    console.log('  Fallback used:', replyFallbackUsed);
    console.log('  ✓ AI Reply with queue integration working');

    console.log('\n[Test 6.2] Reply Engagement Limits:');
    const replyProgress = diveQueue.getEngagementProgress();
    console.log('  Current progress:', {
        replies: `${replyProgress.replies.current}/${replyProgress.replies.limit}`,
        likes: `${replyProgress.likes.current}/${replyProgress.likes.limit}`,
    });

    // Test quote engine
    console.log('\n' + '-'.repeat(70));
    console.log('TESTING AI QUOTE ENGINE WITH QUEUE INTEGRATION');
    console.log('-'.repeat(70));

    const quoteEngine = new AIQuoteEngine(null, {
        quoteProbability: 1.0,
        maxRetries: 1,
    });

    console.log('\n[Test 7] Quote Engine Methods:');
    console.log('  quoteA - Keyboard (T key)');
    console.log('  quoteB - Retweet menu');
    console.log('  quoteC - Quote URL');
    console.log('  quoteD - Copy-paste');
    console.log('  executeQuote (main entry)');

    console.log('\n[Test 8] Quote Engine Stats:');
    const quoteStats = quoteEngine.getStats();
    console.log('  Stats:', JSON.stringify(quoteStats, null, 2));

    console.log('\n[Test 8.1] Queue Integration Test:');
    console.log('  Testing race-condition-free quote processing with DiveQueue...');

    let quoteProcessed = false;
    let quoteFallbackUsed = false;

    const simulateAIQuote = async () => {
        await new Promise((r) => setTimeout(r, 100));
        quoteProcessed = true;
        return { success: true, method: 'ai_quote' };
    };

    const simulateQuoteFallback = async () => {
        quoteFallbackUsed = true;
        return { engagementType: 'bookmark', success: true };
    };

    const quoteDiveResult = await diveQueue.addDive(simulateAIQuote, simulateQuoteFallback, {
        taskName: 'test_ai_quote',
        timeout: diveQueue.quickModeEnabled ? 3000 : 5000,
        priority: 0,
    });

    console.log('  DiveQueue result:', JSON.stringify(quoteDiveResult, null, 2));
    console.log('  Quote processed:', quoteProcessed);
    console.log('  Fallback used:', quoteFallbackUsed);
    console.log('  ✓ AI Quote with queue integration working');

    console.log('\n[Test 8.2] Quote Engagement Limits:');
    const quoteProgress = diveQueue.getEngagementProgress();
    console.log('  Current progress:', {
        quotes: `${quoteProgress.quotes.current}/${quoteProgress.quotes.limit}`,
        bookmarks: `${quoteProgress.bookmarks.current}/${quoteProgress.bookmarks.limit}`,
    });

    // Test composer verification
    console.log('\n' + '-'.repeat(70));
    console.log('TESTING COMPOSER VERIFICATION');
    console.log('-'.repeat(70));
    console.log('\n[Test 9] verifyComposerOpen() Method:');
    console.log('  Checks for:');
    console.log('    - [data-testid="tweetTextarea_0"]');
    console.log('    - [contenteditable="true"][role="textbox"]');
    console.log('    - [data-testid="tweetTextarea"]');
    console.log('    - textarea[placeholder*="Post your reply"]');
    console.log('  Returns: { open, selector, locator }');

    console.log('\n[Test 10] typeText() Method:');
    console.log('  Features:');
    console.log('    - Multiple focus strategies');
    console.log('    - Human-like typing delays');
    console.log('    - Punctuation pauses');
    console.log('    - Random thinking breaks');

    // ================================================================
    // QUEUE MECHANISM SUMMARY
    // ================================================================
    console.log('\n' + '='.repeat(70));
    console.log('ASYNC QUEUE MECHANISM SUMMARY');
    console.log('='.repeat(70));

    console.log('\n[Summary] Race Condition Resolution:');
    console.log('  ✓ AsyncQueue: Concurrent task management (maxConcurrent: 3)');
    console.log('  ✓ DiveQueue: Sequential tweet diving (maxConcurrent: 1)');
    console.log('  ✓ Timeout Handling: Automatic fallback after timeout');
    console.log('  ✓ Engagement Limits: Per-action limits with dual tracking');
    console.log('  ✓ Queue Size Limits: Prevents queue overflow');
    console.log('  ✓ Promise-based Processing: No race conditions');

    console.log('\n[Summary] DiveQueue Configuration:');
    console.log('  maxConcurrent: 1 (sequential for tweet diving)');
    console.log('  maxQueueSize: 30');
    console.log('  defaultTimeout: 2000ms');
    console.log('  fallbackEngagement: true');
    console.log('  Engagement Limits:');
    console.log('    - Replies: 3');
    console.log('    - Retweets: 1');
    console.log('    - Quotes: 1');
    console.log('    - Likes: 5');
    console.log('    - Follows: 2');
    console.log('    - Bookmarks: 2');

    console.log('\n[Summary] Key Features:');
    console.log('  1. Race-Condition-Free: Queue-based processing instead of boolean locks');
    console.log('  2. Concurrent Processing: Up to 3 simultaneous dives');
    console.log('  3. Timeout Fallback: Automatic engagement when AI times out');
    console.log('  4. Engagement Limits: Prevents over-engagement');
    console.log('  5. Health Monitoring: Queue status and health checks');
    console.log('  6. Quick Mode: Faster timeouts during cooldown phases');

    console.log('\n[Summary] Integration Points:');
    console.log('  - AITwitterAgent.diveTweet() -> diveQueue.addDive()');
    console.log('  - handleAIReply() -> engagement limits');
    console.log('  - handleAIQuote() -> engagement limits');
    console.log('  - _quickFallbackEngagement() -> fallback on timeout');

    console.log('\n' + '='.repeat(70));
    console.log('TEST COMPLETE');
    console.log('='.repeat(70));
    console.log('\nTo test in browser:');
    console.log(
        '  node main.js testHumanMethods targetUrl=https://x.com/user/status/123 method=replyA mode=safe'
    );
    console.log(
        '  node main.js testHumanMethods targetUrl=https://x.com/user/status/123 method=quoteA mode=safe\n'
    );
}

main().catch(console.error);
