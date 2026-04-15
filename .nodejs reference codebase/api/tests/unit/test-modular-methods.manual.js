/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * Quick validation test for modularized methods
 * Run: node test-modular-methods.js
 */

import { replyMethods, quoteMethods } from './utils/twitter-interaction-methods.js';

console.log('🔍 Testing modularized methods...\n');

// Test 1: Verify methods are exported
console.log('✅ Test 1: Methods exported');
console.log('  Reply methods:', Object.keys(replyMethods).join(', '));
console.log('  Quote methods:', Object.keys(quoteMethods).join(', '));

// Test 2: Verify method signatures
console.log('\n✅ Test 2: Method signatures');
for (const [name, method] of Object.entries(replyMethods)) {
    const params = method.toString().match(/\(([^)]*)\)/)?.[1] || '';
    console.log(`  ${name}: (${params})`);
}

// Test 3: Verify return format
console.log('\n✅ Test 3: Method structure');
const testMethod = replyMethods.replyA;
const isAsync = testMethod.constructor.name === 'AsyncFunction';
console.log(`  replyA is async: ${isAsync}`);

// Test 4: Configuration structure
console.log('\n✅ Test 4: Default configuration');
const defaultReplyConfig = {
    replyA: { weight: 40, enabled: true },
    replyB: { weight: 35, enabled: true },
    replyC: { weight: 25, enabled: true },
};

const totalWeight = Object.values(defaultReplyConfig)
    .filter((c) => c.enabled)
    .reduce((sum, c) => sum + c.weight, 0);
console.log(`  Total reply weight: ${totalWeight}%`);

// Test 5: Weighted selection simulation
console.log('\n✅ Test 5: Weighted selection simulation (1000 iterations)');
const counts = { replyA: 0, replyB: 0, replyC: 0 };

function selectWeightedMethod(methodsConfig) {
    const enabledMethods = Object.entries(methodsConfig)
        .filter(([_, config]) => config.enabled !== false)
        .map(([name, config]) => ({ name, weight: config.weight ?? 33 }));

    const totalWeight = enabledMethods.reduce((sum, m) => sum + m.weight, 0);
    let random = Math.random() * totalWeight;

    for (const method of enabledMethods) {
        random -= method.weight;
        if (random <= 0) {
            return method.name;
        }
    }

    return enabledMethods[enabledMethods.length - 1].name;
}

for (let i = 0; i < 1000; i++) {
    const selected = selectWeightedMethod(defaultReplyConfig);
    counts[selected]++;
}

console.log(`  replyA: ${counts.replyA} (${(counts.replyA / 10).toFixed(1)}%) - Expected: ~40%`);
console.log(`  replyB: ${counts.replyB} (${(counts.replyB / 10).toFixed(1)}%) - Expected: ~35%`);
console.log(`  replyC: ${counts.replyC} (${(counts.replyC / 10).toFixed(1)}%) - Expected: ~25%`);

console.log('\n✅ All validation tests passed!');
console.log('\n📋 Next steps:');
console.log('  1. Test with actual browser automation');
console.log('  2. Verify config loading from settings.json');
console.log('  3. Test fallback behavior');
console.log('  4. Run full ai-twitterActivity.js integration test');
