/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * Cloud API Tester
 * Tests OpenRouter API with arcee-ai/trinity-large-preview:free model
 */

import 'dotenv/config';

const API_KEY = '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';
const ENDPOINT = 'https://openrouter.ai/api/v1/chat/completions';
const MODEL = 'arcee-ai/trinity-large-preview:free';

const TEST_PROMPT = `Tweet from @TechCrunch:
"Apple just announced the new iPhone 16 with AI features"

Reply from @JohnDoe: "Camera upgrades are insane"
Reply from @JaneSmith: "Finally proper AI integration"
Reply from @TechGuru: "Best iPhone upgrade in years"

Your take (read the replies above and add something that fits the conversation):`;

async function testCloudAPI() {
    console.log('='.repeat(60));
    console.log('CLOUD API TESTER');
    console.log('='.repeat(60));
    console.log(`Model: ${MODEL}`);
    console.log(`Endpoint: ${ENDPOINT}`);
    console.log('');

    const startTime = Date.now();

    try {
        console.log('Sending request...');
        console.log('');

        const response = await fetch(ENDPOINT, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
                Authorization: `Bearer ${API_KEY}`,
                'HTTP-Referer': 'http://localhost:3000',
                'X-Title': 'Auto-AI Twitter Bot',
            },
            body: JSON.stringify({
                model: MODEL,
                messages: [
                    {
                        role: 'system',
                        content: `You are a real Twitter user. Read the tweet and replies, then add YOUR take that fits the conversation. Keep it 1-2 sentences.`,
                    },
                    {
                        role: 'user',
                        content: TEST_PROMPT,
                    },
                ],
                max_tokens: 100,
                temperature: 0.7,
            }),
        });

        const endTime = Date.now();
        const duration = (endTime - startTime) / 1000;

        console.log(`Response time: ${duration.toFixed(2)}s`);
        console.log(`Status: ${response.status} ${response.statusText}`);
        console.log('');

        if (!response.ok) {
            const errorText = await response.text();
            console.log('ERROR:');
            console.log(errorText);
            return;
        }

        const data = await response.json();

        console.log('RESPONSE:');
        console.log('-'.repeat(60));
        console.log(data.choices[0]?.message?.content || 'No content');
        console.log('-'.repeat(60));
        console.log('');

        console.log('USAGE:');
        const usage = data.usage;
        if (usage) {
            console.log(`  Prompt tokens: ${usage.prompt_tokens}`);
            console.log(`  Completion tokens: ${usage.completion_tokens}`);
            console.log(`  Total tokens: ${usage.total_tokens}`);
        }
        console.log('');

        console.log('COMPLETE:');
        console.log(`Duration: ${duration.toFixed(2)}s`);
        console.log(`Model: ${data.model}`);
    } catch (error) {
        console.log('REQUEST FAILED:');
        console.log(error.message);
    }
}

async function testRetry() {
    console.log('');
    console.log('='.repeat(60));
    console.log('RETRY TEST (simulate 3 attempts)');
    console.log('='.repeat(60));

    for (let attempt = 1; attempt <= 3; attempt++) {
        console.log(`\nAttempt ${attempt}/3...`);

        try {
            const response = await fetch(ENDPOINT, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                    Authorization: `Bearer ${API_KEY}`,
                    'HTTP-Referer': 'http://localhost:3000',
                    'X-Title': 'Auto-AI Twitter Bot',
                },
                body: JSON.stringify({
                    model: MODEL,
                    messages: [
                        {
                            role: 'user',
                            content: 'Say exactly: "Test successful"',
                        },
                    ],
                    max_tokens: 10,
                }),
            });

            if (response.ok) {
                console.log(`Success!`);
                const data = await response.json();
                console.log(`Response: ${data.choices[0]?.message?.content}`);
                return;
            }

            console.log(`Failed: ${response.status}`);
        } catch (error) {
            console.log(`Error: ${error.message}`);
        }

        if (attempt < 3) {
            console.log('Waiting 2s before retry...');
            await new Promise((r) => setTimeout(r, 2000));
        }
    }
}

async function main() {
    await testCloudAPI();
    await testRetry();

    console.log('');
    console.log('='.repeat(60));
    console.log('TEST COMPLETE');
    console.log('='.repeat(60));
}

main().catch(console.error);
