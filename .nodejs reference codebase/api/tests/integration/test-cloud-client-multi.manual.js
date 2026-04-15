/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * Cloud Client Multi-Key Integration Test
 * Tests that cloud-client.js properly uses multi-api.js for fallback
 */

import CloudClient from './core/cloud-client.js';

async function testCloudClientMultiKey() {
    console.log('='.repeat(60));
    console.log('CLOUD CLIENT MULTI-KEY INTEGRATION TEST');
    console.log('='.repeat(60));
    console.log('');

    const client = new CloudClient();

    // Wait for config to load
    await new Promise((resolve) => setTimeout(resolve, 500));

    console.log('Initial stats:', client.getStats());
    console.log('');

    // Test request
    console.log('Sending test request...');

    const result = await client.sendRequest({
        prompt: 'Say exactly: "OK"',
        maxTokens: 10,
        temperature: 0.7,
    });

    console.log('');
    console.log('RESULT:');
    if (result.success) {
        console.log('✓ Status: SUCCESS');
        console.log(`✓ Content: "${result.content}"`);
        console.log(`✓ Model: ${result.metadata?.model}`);
        console.log(`✓ Duration: ${result.metadata?.duration}ms`);
        if (result.metadata?.keyUsed !== undefined) {
            console.log(`✓ Key Used: Key${result.metadata.keyUsed + 1}`);
        }
        if (result.metadata?.fallbackFromKey) {
            console.log('✓ Fallback: Activated');
        }
    } else {
        console.log('✗ Status: FAILED');
        console.log(`✗ Error: ${result.error}`);
        if (result.metadata?.keysTried) {
            console.log(`✗ Keys Tried: ${result.metadata.keysTried}`);
        }
    }

    console.log('');
    console.log('FINAL STATS:');
    console.log(JSON.stringify(client.getStats(), null, 2));

    console.log('');
    console.log('='.repeat(60));
    console.log('TEST COMPLETE');
    console.log('='.repeat(60));
}

testCloudClientMultiKey().catch(console.error);
