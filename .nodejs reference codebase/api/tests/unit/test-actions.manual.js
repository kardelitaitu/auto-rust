/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * Test script for Twitter Actions
 * Verifies actions are correctly exported and initialized
 */

import {
    AIReplyAction,
    AIQuoteAction,
    LikeAction,
    BookmarkAction,
    GoHomeAction,
} from './utils/actions/index.js';

console.log('=== Twitter Actions Test ===\n');

// Mock agent for testing
const mockAgent = {
    config: {
        probabilities: {
            reply: 0.6,
            quote: 0.2,
            like: 0.5,
            bookmark: 0.3,
        },
    },
    replyEngine: {
        generateReply: async () => ({ success: true, reply: 'Test reply' }),
    },
    quoteEngine: {
        generateQuote: async () => ({ success: true, quote: 'Test quote' }),
    },
    diveQueue: {
        canEngage: (_type) => true,
        recordEngagement: () => {},
    },
    page: {
        url: () => 'https://x.com/home',
    },
};

async function testActions() {
    console.log('1. Testing AIReplyAction...');
    const replyAction = new AIReplyAction(mockAgent);
    console.log('   Stats:', JSON.stringify(replyAction.getStats()));
    console.log('   Probability:', replyAction.probability);
    console.log('   ✅ AIReplyAction loaded\n');

    console.log('2. Testing AIQuoteAction...');
    const quoteAction = new AIQuoteAction(mockAgent);
    console.log('   Stats:', JSON.stringify(quoteAction.getStats()));
    console.log('   Probability:', quoteAction.probability);
    console.log('   ✅ AIQuoteAction loaded\n');

    console.log('3. Testing LikeAction...');
    const likeAction = new LikeAction(mockAgent);
    console.log('   Stats:', JSON.stringify(likeAction.getStats()));
    console.log('   Probability:', likeAction.probability);
    console.log('   ✅ LikeAction loaded\n');

    console.log('4. Testing BookmarkAction...');
    const bookmarkAction = new BookmarkAction(mockAgent);
    console.log('   Stats:', JSON.stringify(bookmarkAction.getStats()));
    console.log('   Probability:', bookmarkAction.probability);
    console.log('   ✅ BookmarkAction loaded\n');

    console.log('5. Testing GoHomeAction...');
    const goHomeAction = new GoHomeAction(mockAgent);
    console.log('   Stats:', JSON.stringify(goHomeAction.getStats()));
    console.log('   ✅ GoHomeAction loaded\n');

    console.log('=== All Actions Loaded Successfully ===');
}

testActions().catch(console.error);
