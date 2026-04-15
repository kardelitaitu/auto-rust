/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * AI Reply Engine Tester - DEBUG VERSION
 * Standalone script to test the LLM mechanism with real config
 * Logs ALL requests and responses in JSON format for debugging
 *
 * Usage: node test-ai-reply-engine.js
 */

import { createLogger } from './utils/logger.js';
import { getSettings } from './utils/configLoader.js';
import CloudClient from './core/cloud-client.js';

const logger = createLogger('test-ai-reply-engine.js');
logger.info('Starting test-ai-reply-engine.js');

const EXAMPLE_TWEETS = [
    {
        author: 'technews',
        text: 'Just discovered that AI can now write code better than most junior developers. The future is here. 🤖',
        url: 'https://twitter.com/technews/status/1234567890',
        replies: [
            { author: 'dev1', text: 'This is terrifying for new grads honestly' },
            { author: 'coder42', text: "Meanwhile I still can't get AI to center a div" },
            {
                author: 'skeptical_sam',
                text: 'Give it a few years and it will be writing entire microservices',
            },
        ],
    },
    {
        author: 'coffee_addict',
        text: 'Third cup of coffee today. Send help. ☕️',
        url: 'https://twitter.com/coffee_addict/status/0987654321',
        replies: [
            { author: 'tea_fan', text: 'Switch to tea, your heart will thank you' },
            { author: 'caffeine_king', text: "That's rookie numbers, I'm on cup five" },
        ],
    },
    {
        author: 'startup_ceo',
        text: 'Building in public is the best marketing strategy. Transparency wins trust.',
        url: 'https://twitter.com/startup_ceo/status/1122334455',
        replies: [
            {
                author: 'founder_jane',
                text: 'Absolutely! Been doing this for 2 years, doubled my audience',
            },
            {
                author: 'marketing_guru',
                text: 'Depends on the audience. Some people hate transparency',
            },
            { author: 'newbie_founder', text: 'This is gold, thanks for sharing' },
        ],
    },
];

function logJson(label, data) {
    console.log('\n' + '█'.repeat(80));
    console.log(`█ ${label}`);
    console.log('█'.repeat(80));
    console.log(JSON.stringify(data, null, 2));
    console.log('█'.repeat(80) + '\n');
}

async function runTest(cloudClient, testIndex) {
    const testTweet = EXAMPLE_TWEETS[testIndex % EXAMPLE_TWEETS.length];

    console.log('\n' + '═'.repeat(80));
    console.log(` TEST ${testIndex + 1}: @${testTweet.author}`);
    console.log('═'.repeat(80));

    logJson('INPUT TWEET DATA', {
        author: testTweet.author,
        text: testTweet.text,
        url: testTweet.url,
        replyCount: testTweet.replies.length,
        replies: testTweet.replies,
    });

    const userPrompt = `Tweet from @${testTweet.author}:
"${testTweet.text}"

Tweet URL: ${testTweet.url}

Other replies to this tweet:
${testTweet.replies.map((r, i) => `${i + 1}. @${r.author}: "${r.text}"`).join('\n')}

Generate ONE natural reply (max 280 chars) that:
- Is casual and conversational
- Adds value to the conversation
- Avoids being generic or spammy

Reply:`;

    const requestPayload = {
        action: 'generate_reply',
        payload: {
            systemPrompt:
                'You are a social media engagement assistant. Generate natural, casual replies.',
            userPrompt: userPrompt,
            maxTokens: 100,
            temperature: 0.7,
        },
        context: {},
    };

    logJson('FULL REQUEST TO CLOUD CLIENT', requestPayload);

    const apiPayload = {
        model: 'nousresearch/hermes-3-llama-3.1-405b:free',
        messages: [
            { role: 'system', content: requestPayload.payload.systemPrompt },
            { role: 'user', content: requestPayload.payload.userPrompt },
        ],
        max_tokens: requestPayload.payload.maxTokens,
        temperature: requestPayload.payload.temperature,
        stream: false,
        exclude_reasoning: true,
    };

    logJson('RAW API REQUEST (sent to OpenRouter)', apiPayload);

    const startTime = Date.now();

    console.log('⏳ Sending request at:', new Date().toISOString());
    console.log('');

    try {
        const response = await cloudClient.sendRequest(requestPayload);

        const duration = Date.now() - startTime;

        console.log('');
        console.log('═'.repeat(80));
        console.log(` RESPONSE RECEIVED - ${duration}ms`);
        console.log('═'.repeat(80));
        console.log('');

        logJson('RAW API RESPONSE (from OpenRouter)', {
            success: response.success,
            content: response.content,
            metadata: response.metadata,
        });

        if (response.metadata?.model) {
            console.log(`🤖 Model Used: ${response.metadata.model}`);
        }
        if (response.metadata?.duration) {
            console.log(`⏱️  Duration: ${response.metadata.duration}ms`);
        }
        if (response.metadata?.proxyUsed) {
            console.log(`🌐 Proxy Used: ${response.metadata.proxyUsed}`);
        }
        if (response.metadata?.modelFallbacks) {
            console.log(`🔄 Model Fallbacks: ${response.metadata.modelFallbacks}`);
        }

        console.log('');
        console.log('═'.repeat(80));
        console.log(` TEST ${testIndex + 1} COMPLETE`);
        console.log('═'.repeat(80));
        console.log('');

        return response;
    } catch (error) {
        console.log('');
        console.log('❌'.repeat(80));
        console.log(` ERROR: ${error.message}`);
        console.log('❌'.repeat(80));
        console.log('');
        return { success: false, error: error.message };
    }
}

async function main() {
    console.log('\n' + '█'.repeat(80));
    console.log('█  AI REPLY ENGINE TESTER - DEBUG MODE');
    console.log('█  Logs ALL JSON requests/responses');
    console.log('█'.repeat(80));
    console.log(`\n📅 Started: ${new Date().toISOString()}`);
    console.log(`📁 Working Directory: ${process.cwd()}`);

    console.log('\n--- LOADING CONFIGURATION ---\n');

    let settings;
    try {
        settings = await getSettings();
        logJson('LOADED SETTINGS (relevant sections)', {
            'llm.cloud.enabled': settings.llm?.cloud?.enabled,
            'llm.cloud.provider': settings.llm?.cloud?.provider,
            'open_router_free_api.enabled': settings.open_router_free_api?.enabled,
            'open_router_free_api.primaryModel': settings.open_router_free_api?.models?.primary,
            'open_router_free_api.fallbackCount':
                settings.open_router_free_api?.models?.fallbacks?.length,
            'open_router_free_api.apiKeysCount': settings.open_router_free_api?.api_keys?.length,
        });
    } catch (error) {
        console.error('Failed to load settings:', error.message);
        process.exit(1);
    }

    console.log('\n--- INITIALIZING CLOUD CLIENT ---\n');

    const cloudClient = new CloudClient();

    await new Promise((resolve) => setTimeout(resolve, 3000));

    // Wait for background model tests to complete (up to 60 seconds)
    console.log('\n--- WAITING FOR MODEL TESTS TO COMPLETE ---\n');

    let waitCount = 0;
    const maxWait = 60;

    while (waitCount < maxWait) {
        const helper = CloudClient.sharedHelper;
        if (helper && helper.getResults() && helper.getResults().testDuration > 0) {
            console.log(
                `✓ Background tests completed: ${helper.getResults().working.length}/${helper.getResults().total} working models`
            );
            break;
        }

        if (waitCount % 5 === 0) {
            console.log(`⏳ Waiting for model tests... (${waitCount}s/${maxWait}s)`);
        }

        await new Promise((resolve) => setTimeout(resolve, 1000));
        waitCount++;
    }

    const routerInfo = cloudClient.freeApiRouter?.getSessionInfo();
    if (routerInfo) {
        logJson('FREE ROUTER SESSION INFO', routerInfo);
    }

    const clientStats = cloudClient.getStats();
    logJson('CLOUD CLIENT INITIAL STATS', clientStats);

    console.log('\n--- RUNNING 3 TESTS ---\n');

    const results = [];

    for (let i = 0; i < 3; i++) {
        console.log(`\n### TEST ${i + 1} OF 3 ###\n`);
        const result = await runTest(cloudClient, i);
        results.push(result);

        if (i < 2) {
            console.log('⏳ Waiting 3 seconds before next test...');
            await new Promise((resolve) => setTimeout(resolve, 3000));
        }
    }

    console.log('\n' + '█'.repeat(80));
    console.log('█  FINAL SUMMARY');
    console.log('█'.repeat(80));

    const finalStats = cloudClient.getStats();
    logJson('FINAL CLOUD CLIENT STATS', {
        mode: finalStats.mode,
        totalRequests: finalStats.totalRequests,
        successfulRequests: finalStats.successfulRequests,
        failedRequests: finalStats.failedRequests,
        successRate: finalStats.successRate,
        avgDuration: finalStats.avgDuration,
        freeApiRouterStats: finalStats.freeApiRouterStats?.router,
    });

    const testResults = {
        total: 3,
        successful: results.filter((r) => r.success).length,
        failed: results.filter((r) => !r.success).length,
        results: results.map((r, i) => ({
            test: i + 1,
            success: r.success,
            model: r.metadata?.model,
            duration: r.metadata?.duration,
            error: r.error,
        })),
    };

    logJson('TEST RESULTS SUMMARY', testResults);

    console.log('\n✅ All tests completed!');
    console.log(`📅 Finished: ${new Date().toISOString()}\n`);
}

main().catch((error) => {
    console.error('\n❌ FATAL ERROR:', error.message);
    console.error(error.stack);
    process.exit(1);
});
