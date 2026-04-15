/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * Test Smart Probability Logic
 * Verifies redistribution when actions hit limits
 */

import { ActionRunner } from './utils/actions/index.js';

console.log('=== Testing Smart Probability Redistribution ===\n');

const mockAgent = {
    config: {
        twitter: {
            actions: {
                reply: { probability: 0.6, enabled: true },
                quote: { probability: 0.2, enabled: true },
                like: { probability: 0.15, enabled: true },
                bookmark: { probability: 0.05, enabled: true },
                goHome: { enabled: true },
            },
        },
    },
    diveQueue: {
        canEngage: (type) => {
            const limits = {
                replies: 3,
                quotes: 1,
                likes: 5,
                bookmarks: 2,
            };
            const used = {
                replies: 0,
                quotes: 1,
                likes: 0,
                bookmarks: 0,
            };
            return used[type] < limits[type];
        },
    },
};

const runner = new ActionRunner(mockAgent);

console.log('Base probabilities (from config):');
console.log('  reply: 60%');
console.log('  quote: 20%');
console.log('  like: 15%');
console.log('  bookmark: 5%');
console.log('  Total: 100%');
console.log('');

console.log('Scenario 1: All actions available (quotes remaining: 1/1)');
let probs = runner.calculateSmartProbabilities();
console.log('Smart probabilities:', JSON.stringify(probs, null, 2));
console.log('Selected action:', runner.selectAction());
console.log('');

console.log('Scenario 2: Quote at limit (quotes used: 1/1)');
console.log('Quote should be excluded, remaining 80% redistributed:');
console.log('  reply: 60/80 = 75%');
console.log('  like: 15/80 = 18.75%');
console.log('  bookmark: 5/80 = 6.25%');
probs = runner.calculateSmartProbabilities();
console.log('Smart probabilities:', JSON.stringify(probs, null, 2));
console.log('');

console.log('Scenario 3: Only like available (simulate reply/quote/bookmark at limit)');
runner.config.reply.enabled = false;
runner.config.quote.enabled = false;
runner.config.bookmark.enabled = false;
probs = runner.calculateSmartProbabilities();
console.log('Smart probabilities:', JSON.stringify(probs, null, 2));
console.log('');

console.log('=== Test Complete ===');
