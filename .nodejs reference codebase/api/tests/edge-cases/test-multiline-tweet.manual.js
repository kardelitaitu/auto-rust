/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Test script for multi-line tweet decoding
 * @module tests/test-multiline-tweet.js
 */

import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

console.log('Testing Multi-Line Tweet Decoding...\n');

// Test cases
const testCases = [
    {
        name: 'Single-line tweet',
        input: 'This is a simple tweet',
        expected: 'This is a simple tweet',
    },
    {
        name: 'Multi-line tweet (2 lines)',
        input: 'Line 1\\nLine 2',
        expected: 'Line 1\nLine 2',
    },
    {
        name: 'Multi-line tweet (with blank line)',
        input: 'Paragraph 1\\n\\nParagraph 2',
        expected: 'Paragraph 1\n\nParagraph 2',
    },
    {
        name: 'Complex multi-line',
        input: 'Breaking news!\\n\\nWe launched v2.0\\n\\nFeatures:\\n- Speed\\n- UI\\n- Fixes',
        expected: 'Breaking news!\n\nWe launched v2.0\n\nFeatures:\n- Speed\n- UI\n- Fixes',
    },
];

// Test the decoding logic
let passed = 0;
let failed = 0;

testCases.forEach((test, index) => {
    const result = test.input.replace(/\\n/g, '\n');
    const success = result === test.expected;

    if (success) {
        passed++;
        console.log(`✅ Test ${index + 1}: ${test.name}`);
    } else {
        failed++;
        console.log(`❌ Test ${index + 1}: ${test.name}`);
        console.log(`   Input:    "${test.input}"`);
        console.log(`   Expected: "${test.expected}"`);
        console.log(`   Got:      "${result}"`);
    }
});

console.log(`\n${'─'.repeat(60)}`);
console.log(`Results: ${passed} passed, ${failed} failed`);

if (failed === 0) {
    console.log('✅ All tests passed!\n');
} else {
    console.log('❌ Some tests failed.\n');
    process.exit(1);
}

// Test actual file reading
console.log('Testing actual file reading...\n');

const TWEET_FILE = path.join(__dirname, '../tasks/twitterTweet.txt');

if (fs.existsSync(TWEET_FILE)) {
    const content = fs.readFileSync(TWEET_FILE, 'utf-8');
    const lines = content
        .split(/\r?\n/)
        .filter((line) => line.trim().length > 0 && !line.trim().startsWith('#'));

    console.log(`Found ${lines.length} tweet(s) in queue:\n`);

    lines.forEach((line, index) => {
        const decoded = line.replace(/\\n/g, '\n');
        const hasLineBreaks = decoded.includes('\n');

        console.log(`[${index + 1}] ${hasLineBreaks ? '📄 Multi-line' : '📝 Single-line'}`);
        console.log(`Raw: ${line.substring(0, 50)}${line.length > 50 ? '...' : ''}`);
        console.log('Decoded:');
        console.log(decoded);
        console.log('─'.repeat(60));
    });
} else {
    console.log(`⚠️  File not found: ${TWEET_FILE}`);
}

console.log('\n✅ Decoding logic is working correctly!');
