// Tavily API Key Test Script
// Set TAVILY_API_KEY environment variable or replace placeholder new test
// Get your key from https://app.tavily.com

const { tavily } = require('@tavily/core');

async function testTavily() {
    // Try environment variable first, then fallback to placeholder
    const apiKey = process.env.TAVILY_API_KEY || 'YOUR_API_KEY_HERE';

    if (apiKey === 'YOUR_API_KEY_HERE') {
        console.error('❌ Please set TAVILY_API_KEY environment variable or edit the script');
        console.error('   Option 1 (recommended): set TAVILY_API_KEY=tvly-dev-... before running');
        console.error('   Option 2: Replace YOUR_API_KEY_HERE in this file');
        console.error('   Get your key from: https://app.tavily.com');
        process.exit(1);
    }

    console.log('🔍 Testing Tavily API key...');

    try {
        const client = tavily({ apiKey });

        const response = await client.search('What is Tavily?', {
            searchDepth: 'basic',
            maxResults: 2,
        });

        console.log('✅ API key is valid!');
        console.log('📊 Search results:');
        console.log(JSON.stringify(response, null, 2));
    } catch (error) {
        console.error('❌ API key test failed:');
        console.error(`   Error: ${error.message}`);

        if (error.message.includes('Invalid API key') || error.message.includes('401')) {
            console.error('   → Your API key is invalid or expired');
            console.error('   → Get a new key from: https://app.tavily.com');
        } else if (error.message.includes('rate limit')) {
            console.error('   → Rate limit exceeded. Wait and try again.');
        }

        process.exit(1);
    }
}

testTavily();
