/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * Check if dedupe cache has empty responses
 */

import { FreeApiRouter } from './utils/free-api-router.js';

const apiKey = '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';
const model = 'arcee-ai/trinity-large-preview:free';

async function testDedupe() {
    console.log('Testing dedupe cache behavior...\n');

    // First request - should hit API
    const router1 = new FreeApiRouter({
        enabled: true,
        apiKeys: [apiKey],
        primaryModel: model,
        fallbackModels: [],
        proxyEnabled: false,
    });

    const request = {
        messages: [{ role: 'user', content: 'Reply with exactly one word: "ok"' }],
        maxTokens: 50,
        temperature: 0.7,
    };

    console.log('=== First request (should hit API) ===');
    const result1 = await router1.processRequest(request);
    console.log(`Success: ${result1.success}, Content: "${result1.content}"`);
    console.log(`From cache: ${result1.fromCache || false}`);

    // Second request with SAME messages - should hit cache
    console.log('\n=== Second request (same messages - should hit cache) ===');
    const router2 = new FreeApiRouter({
        enabled: true,
        apiKeys: [apiKey],
        primaryModel: model,
        fallbackModels: [],
        proxyEnabled: false,
    });

    const result2 = await router2.processRequest(request);
    console.log(`Success: ${result2.success}, Content: "${result2.content}"`);
    console.log(`From cache: ${result2.fromCache || false}`);

    // Check cache stats
    const stats = router2.getStats();
    console.log('\n=== Cache Stats ===');
    console.log(JSON.stringify(stats.router, null, 2));
}

testDedupe().catch(console.error);
