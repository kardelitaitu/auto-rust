/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * Multi-API Client Tester
 * Tests multiple OpenRouter API keys with fallback
 */

import 'dotenv/config';

const CONFIG = {
    apiKeys: [
        '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',
        'sk-or-v1-2aaf40d2b0c0c0e5b1b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b',
        'sk-or-v1-3bbf50d3c1d1d1f2c2c2c2c2c2c2c2c2c2c2c2c2c2c2c2c2c2c2c2c2c',
    ],
    endpoint: 'https://openrouter.ai/api/v1/chat/completions',
    model: 'arcee-ai/trinity-large-preview:free',
    timeout: 60000,
};

import { MultiOpenRouterClient } from './utils/multi-api.js';

const client = new MultiOpenRouterClient(CONFIG);

const TEST_PROMPT = `Tweet from @TechCrunch:
"Apple just announced the new iPhone 16 with AI features"

Reply from @JohnDoe: "Camera upgrades are insane"
Reply from @JaneSmith: "Finally proper AI integration"

Your take:`;

async function testSingleRequest() {
    console.log('='.repeat(60));
    console.log('MULTI-API TESTER');
    console.log('='.repeat(60));
    console.log(`API Keys: ${CONFIG.apiKeys.length}`);
    console.log(`Model: ${CONFIG.model}`);
    console.log('');

    // const startTime = Date.now();

    const result = await client.processRequest({
        messages: [
            {
                role: 'user',
                content: TEST_PROMPT,
            },
        ],
        maxTokens: 50,
        temperature: 0.7,
    });

    console.log('RESULT:');
    if (result.success) {
        console.log(`✓ Status: SUCCESS`);
        console.log(`✓ Key Used: Key${result.keyUsed + 1}/${CONFIG.apiKeys.length}`);
        console.log(`✓ Response: "${result.content}"`);
        console.log(`✓ Tokens: ${result.tokens?.total}`);
    } else {
        console.log(`✗ Status: FAILED`);
        console.log(`✗ Error: ${result.error}`);
        console.log(`✗ Keys Tried: ${result.keysTried}`);
    }

    console.log('');
    console.log('STATS:');
    console.log(client.getStats());
}

async function testMultipleRequests() {
    console.log('');
    console.log('='.repeat(60));
    console.log('LOAD TEST (5 rapid requests)');
    console.log('='.repeat(60));

    for (let i = 1; i <= 5; i++) {
        console.log(`\nRequest ${i}/5...`);

        const result = await client.processRequest({
            messages: [{ role: 'user', content: 'Say exactly: "OK"' }],
            maxTokens: 10,
        });

        if (result.success) {
            console.log(`  ✓ Key${result.keyUsed + 1}: "${result.content}"`);
        } else {
            console.log(`  ✗ Failed: ${result.error}`);
        }

        // Small delay between requests
        await new Promise((r) => setTimeout(r, 500));
    }

    console.log('\n' + '='.repeat(60));
    console.log('FINAL STATS:');
    console.log(client.getStats());
}

async function testKeyFallback() {
    console.log('');
    console.log('='.repeat(60));
    console.log('FALLBACK SIMULATION');
    console.log('='.repeat(60));
    console.log('Testing with potentially failing keys...');

    // Create client with bad keys to test fallback
    const testClient = new MultiOpenRouterClient({
        apiKeys: [
            'sk-or-v1-badkey1234567890123456789012345678901234567890', // Bad key
            'sk-or-v1-anotherbadkey123456789012345678901234567890', // Bad key
            CONFIG.apiKeys[0], // Good key
        ],
        model: CONFIG.model,
        retryDelay: 1000,
    });

    console.log('\nSending request (should fallback from Key1 -> Key2 -> Key3)...');

    const result = await testClient.processRequest({
        messages: [{ role: 'user', content: 'Say: "Fallback works"' }],
        maxTokens: 10,
    });

    if (result.success) {
        console.log(`\n✓ Fallback SUCCESS!`);
        console.log(`✓ Final Key Used: Key${result.keyUsed + 1}`);
        console.log(`✓ Response: "${result.content}"`);
    } else {
        console.log(`\n✗ Fallback failed: ${result.error}`);
    }

    console.log('\nFallback Test Stats:', testClient.getStats());
}

async function main() {
    await testSingleRequest();
    await testMultipleRequests();
    await testKeyFallback();

    console.log('\n' + '='.repeat(60));
    console.log('TEST COMPLETE');
    console.log('='.repeat(60));
}

main().catch(console.error);
