/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * Test action config loading from settings.json
 */

import { config } from './utils/config-service.js';

async function testConfig() {
    console.log('=== Testing Config Loading ===\n');

    await config.init();
    const settings = config.get();

    console.log('Twitter Actions Config:');
    console.log(JSON.stringify(settings?.twitter?.actions, null, 2));

    const replyProb = settings?.twitter?.actions?.reply?.probability ?? 0.6;
    const quoteProb = settings?.twitter?.actions?.quote?.probability ?? 0.2;
    const likeProb = settings?.twitter?.actions?.like?.probability ?? 0.5;
    const bookmarkProb = settings?.twitter?.actions?.bookmark?.probability ?? 0.3;

    console.log('\nExtracted Probabilities:');
    console.log(`  reply: ${replyProb * 100}%`);
    console.log(`  quote: ${quoteProb * 100}%`);
    console.log(`  like: ${likeProb * 100}%`);
    console.log(`  bookmark: ${bookmarkProb * 100}%`);

    console.log('\n=== Test Complete ===');
}

testConfig().catch(console.error);
