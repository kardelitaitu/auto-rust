/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * Cloud Client Twitter Prompt Test
 * Tests that Twitter-style prompts (systemPrompt + userPrompt) are passed correctly
 */

import CloudClient from './core/cloud-client.js';

async function testTwitterPromptFix() {
    console.log('='.repeat(60));
    console.log('CLOUD CLIENT TWITTER PROMPT TEST');
    console.log('='.repeat(60));
    console.log('');

    const client = new CloudClient();

    // Wait for config to load
    await new Promise((resolve) => setTimeout(resolve, 500));

    // Simulate Twitter reply request structure
    const twitterRequest = {
        action: 'generate_reply',
        payload: {
            systemPrompt: `You are a Twitter engagement bot. Write a short, engaging reply (under 280 chars).
Your task is to generate a reply that:
1. Is relevant to the tweet and context
2. Adds value to the conversation
3. Sounds natural and human-like
4. Avoids generic responses like "interesting" or "nice"

Reply format: Just the reply text, nothing else.`,
            userPrompt: `Tweet from @TechCrunch:
"Apple just announced the new iPhone 16 with AI features"

Reply from @JohnDoe: "Camera upgrades are insane"
Reply from @JaneSmith: "Finally proper AI integration"

Your reply:`,
            maxTokens: 100,
            temperature: 0.7,
        },
    };

    console.log('Sending Twitter-style request...');
    console.log(`System prompt length: ${twitterRequest.payload.systemPrompt.length} chars`);
    console.log(`User prompt length: ${twitterRequest.payload.userPrompt.length} chars`);
    console.log('');

    const startTime = Date.now();

    const result = await client.sendRequest(twitterRequest);

    const duration = Date.now() - startTime;

    console.log('');
    console.log('RESULT:');
    if (result.success) {
        console.log('✓ Status: SUCCESS');
        console.log(`✓ Duration: ${duration}ms`);
        console.log(`✓ Model: ${result.metadata?.model}`);
        console.log(`✓ Content: "${result.content}"`);
        console.log(`✓ Length: ${result.content?.length || 0} chars`);
    } else {
        console.log('✗ Status: FAILED');
        console.log(`✗ Error: ${result.error}`);
        console.log(`✗ Duration: ${duration}ms`);
    }

    console.log('');
    console.log('='.repeat(60));
    console.log('TEST COMPLETE');
    console.log('='.repeat(60));
}

testTwitterPromptFix().catch(console.error);
