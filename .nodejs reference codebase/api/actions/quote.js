/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { getPage, evalPage as eval_ } from '../core/context.js';
import { createLogger } from '../core/logger.js';
import { mathUtils } from '../utils/math.js';
import AgentConnector from '../core/agent-connector.js';
import AIQuoteEngine from '../agent/ai-quote-engine.js';
import { HumanInteraction } from '../behaviors/human-interaction.js';
import { scroll, focus } from '../interactions/scroll.js';
import { wait } from '../interactions/wait.js';
import { text, exists } from '../interactions/queries.js';
import metricsCollector from '../utils/metrics.js';
import { sanitizeReplyText } from '../twitter/twitter-reply-prompt.js';

const logger = createLogger('api/quote.js');

/**
 * Automate Quote-Tweeting with AI Context
 *
 * Performs:
 * 1. Context extraction (tweet text + nearby replies)
 * 2. AI quote generation
 * 3. Execution via Retweet Menu (Strategy B)
 *
 * @param {object} [options]
 * @param {string} [options.fallback] - Fallback quote if AI fails
 * @param {number} [options.contextSteps=5] - How many scrolls to collect context
 * @returns {Promise<{success: boolean, method: string, quote: string}>}
 */
export async function quoteWithAI(options = {}) {
    const page = getPage();
    const {
        fallback = 'This is definitely an interesting perspective on things. Worth sharing!',
        contextSteps = 5,
    } = options;

    logger.info(`Starting high-level api.quoteWithAI()...`);

    // 1. Extract Main Tweet
    const tweetSelector = 'article[data-testid="tweet"]';
    const tweetText = await extractMainTweetText();

    // Extract username from URL if possible
    let username = 'unknown';
    const url = page.url();
    const urlParts = url.split('x.com/');
    if (urlParts.length > 1) {
        username = urlParts[1].split('/')[0];
    }

    logger.info(`Context: Tweet by @${username} ("${tweetText.substring(0, 30)}...")`);

    // 2. Extract Replies for Context
    logger.info(`Loading/Reading context (Elastic Scroll)...`);
    const replies = await extractElasticContext(contextSteps);
    logger.info(`Extracted ${replies.length} replies for context`);

    // 3. AI Generation
    const agentConnector = new AgentConnector();
    const quoteEngine = new AIQuoteEngine(agentConnector, { quoteProbability: 1, maxRetries: 1 });
    const human = new HumanInteraction(page);

    const context = { replies, url, isTest: false };
    const generation = await quoteEngine.generateQuote(tweetText, username, context);

    let finalQuoteText = fallback;
    if (generation.success && generation.quote) {
        finalQuoteText = sanitizeReplyText(generation.quote);
        logger.info(`AI Quote: "${finalQuoteText}"`);
    } else {
        logger.warn(`AI Generation failed, using fallback`);
    }

    // 4. Reset Focus to Tweet (Golden View) - Minimal distance now
    logger.info(`Focusing back on target tweet...`);
    await focus(page.locator(tweetSelector).first());
    await wait(mathUtils.randomInRange(500, 1000));

    // 5. Execute Action
    logger.info(`Executing quoteMethodB (Retweet Menu)...`);
    const result = await quoteEngine.quoteMethodB_Retweet(page, finalQuoteText, human);

    if (result.success) {
        logger.info(`✅ api.quoteWithAI successful!`);
        metricsCollector.recordTwitterEngagement('quote', 1);
        result.quote = finalQuoteText;
    } else {
        logger.error(`❌ api.quoteWithAI failed: ${result.reason}`);
    }

    return result;
}

// ─── Private Helpers ─────────────────────────────────────────────────────────

async function extractMainTweetText() {
    const selectors = [
        'article[data-testid="tweet"] div[data-testid="tweetText"]',
        'article[data-testid="tweet"] [data-testid="tweetText"]',
        'article[data-testid="tweet"] [dir="auto"]',
    ];

    for (const selector of selectors) {
        if (await exists(selector)) {
            const val = await text(selector);
            if (val && val.length > 3) return val;
        }
    }
    return 'No text found';
}

/**
 * Elastic Context Extraction:
 * 1. Scroll DOWN quickly to load replies.
 * 2. Scroll UP in steps, extracting text during the upward movement.
 * Returns viewport to near-top.
 */
async function extractElasticContext(steps) {
    const replies = [];
    const seenTexts = new Set();

    // Phase 1: Load (Down)
    logger.info(`[Elastic] Scrolling down to load infinite content...`);
    for (let i = 0; i < 3; i++) {
        await scroll(mathUtils.randomInRange(600, 900));
        await wait(mathUtils.randomInRange(300, 600));
    }

    // Phase 2: Read (Up)
    logger.info(`[Elastic] Scrolling up while extracting context...`);
    for (let step = 0; step < steps; step++) {
        const visibleTexts = await eval_(() => {
            const found = [];
            const selectors = ['[data-testid="tweetText"]', 'article [dir="auto"]'];
            for (const selector of selectors) {
                const elements = document.querySelectorAll(selector);
                elements.forEach((el) => {
                    const txt = el instanceof HTMLElement ? el.innerText.trim() : '';
                    if (txt && txt.length > 3 && txt.length < 300) {
                        found.push(txt.substring(0, 100));
                    }
                });
            }
            return found;
        });

        for (const txt of visibleTexts) {
            const key = txt.substring(0, 30).toLowerCase();
            if (!seenTexts.has(key)) {
                seenTexts.add(key);
                replies.push({ text: txt });
            }
        }

        if (step < steps - 1) {
            // Scroll UP
            await scroll(-mathUtils.randomInRange(500, 700));
            await wait(mathUtils.randomInRange(600, 1000));
        }
    }
    return replies;
}
