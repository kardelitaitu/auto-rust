/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * Quick model test script
 * Tests a few models to see which ones return content
 */

import { FreeApiRouter } from './utils/free-api-router.js';

const modelsToTest = [
    'arcee-ai/trinity-large-preview:free',
    'tngtech/deepseek-r1t2-chimera:free',
    'meta-llama/llama-3.3-70b-instruct:free',
    'nousresearch/hermes-3-llama-3.1-405b:free',
    'google/gemma-3-27b-it:free',
];

const apiKey = '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';

async function testModel(model) {
    console.log(`\nTesting: ${model}`);
    console.log('-'.repeat(50));

    const router = new FreeApiRouter({
        enabled: true,
        apiKeys: [apiKey],
        primaryModel: model,
        fallbackModels: [],
        proxyEnabled: false,
    });

    const request = {
        messages: [{ role: 'user', content: 'Reply with exactly one word: "hello"' }],
        maxTokens: 50,
        temperature: 0.7,
    };

    try {
        const startTime = Date.now();
        const result = await router.processRequest(request);
        const duration = Date.now() - startTime;

        console.log(`Duration: ${duration}ms`);
        console.log(`Success: ${result.success}`);

        if (result.success) {
            const content = result.content || '';
            console.log(`Content length: ${content.length} chars`);
            console.log(`Content: "${content.substring(0, 100)}..."`);
        } else {
            console.log(`Error: ${result.error}`);
        }

        // Also show model used
        if (result.model) {
            console.log(`Model used: ${result.model}`);
        }
    } catch (error) {
        console.log(`Exception: ${error.message}`);
    }
}

async function main() {
    console.log('=== Quick Model Test ===\n');

    for (const model of modelsToTest) {
        await testModel(model);
        // Wait between tests to avoid rate limits
        await new Promise((resolve) => setTimeout(resolve, 2000));
    }

    console.log('\n=== Done ===');
}

main().catch(console.error);
