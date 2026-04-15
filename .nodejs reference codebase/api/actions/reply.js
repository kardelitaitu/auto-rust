/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { getPage, evalPage as eval_ } from '../core/context.js';
import { createLogger } from '../core/logger.js';
import { mathUtils } from '../utils/math.js';
import AgentConnector from '../core/agent-connector.js';
import AIReplyEngine from '../agent/ai-reply-engine/index.js';
import { scroll, focus } from '../interactions/scroll.js';
import { wait } from '../interactions/wait.js';
import { text, exists, visible } from '../interactions/queries.js';
import { click, type } from '../interactions/actions.js';
import metricsCollector from '../utils/metrics.js';
import { sanitizeReplyText } from '../twitter/twitter-reply-prompt.js';

const logger = createLogger('api/reply.js');

/**
 * Automate Replying with AI Context
 *
 * Performs:
 * 1. Context extraction (tweet text + nearby replies)
 * 2. AI reply generation
 * 3. Execution via Reply Icon (Strategy A)
 * 4. Success verification
 *
 * @param {object} [options]
 * @param {string} [options.fallback] - Fallback reply if AI fails
 * @param {number} [options.contextSteps=5] - How many scrolls to collect context
 * @returns {Promise<{success: boolean, method: string, reply: string}>}
 */
export async function replyWithAI(options = {}) {
    const page = getPage();
    const { fallback = 'Interesting perspective! Thanks for sharing.', contextSteps = 5 } = options;

    logger.info(`Starting high-level api.replyWithAI()...`);

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
    const replyEngine = new AIReplyEngine(agentConnector, { replyProbability: 1, maxRetries: 1 });

    const context = { replies, url };
    const generation = await replyEngine.generateReply(tweetText, username, context);

    let finalReplyText = fallback;
    if (generation.success && generation.reply) {
        finalReplyText = sanitizeReplyText(generation.reply);
        logger.info(`AI Reply: "${finalReplyText}"`);
    } else {
        logger.warn(`AI Generation failed, using fallback`);
    }

    // 4. Reset Focus to Tweet (Golden View) - Minimal distance now
    logger.info(`Focusing back on target tweet...`);
    await focus(page.locator(tweetSelector).first());
    await wait(mathUtils.randomInRange(500, 1000));

    // 5. Execute Action (Method A: Click Reply Icon)
    logger.info(`Executing replyMethodA (Reply Icon)...`);

    // Step 5a: Click reply button
    const replyIconSelector = '[data-testid="reply"]';
    logger.info(`[replyWithAI] Clicking reply icon (ghost cursor)...`);
    await click(replyIconSelector);
    await wait(1500);

    // Step 5b: Verify/Find composer
    const composer = await findComposer(page);
    if (!composer) {
        return { success: false, reason: 'composer_not_found', method: 'replyA' };
    }

    // Step 5c: Type reply
    logger.info(`[replyWithAI] Typing reply (${finalReplyText.length} chars, ghost cursor)...`);
    await type(composer, finalReplyText);
    await wait(1000);

    // Step 5d: Post
    const result = await postReply(page, 'replyA');

    if (result.success) {
        logger.info(`✅ api.replyWithAI successful!`);
        metricsCollector.recordTwitterEngagement('reply', 1);
        result.reply = finalReplyText;
    } else {
        logger.error(`❌ api.replyWithAI failed: ${result.reason}`);
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

async function findComposer(_page) {
    const composerSelectors = [
        '[data-testid="tweetTextarea_0"]',
        '[data-testid="tweetTextarea"]',
        '[contenteditable="true"][role="textbox"]',
    ];
    for (const sel of composerSelectors) {
        if (await visible(sel)) return sel;
    }
    return null;
}

async function postReply(page, methodName) {
    const postSelectors = [
        '[data-testid="tweetButton"]',
        '[data-testid="tweetButtonInline"]',
        '[aria-label="Post"]',
        '[aria-label="Reply"]',
    ];

    let foundSelector = null;
    for (const sel of postSelectors) {
        if (await visible(sel)) {
            foundSelector = sel;
            break;
        }
    }

    if (!foundSelector)
        return { success: false, reason: 'post_button_not_found', method: methodName };

    logger.info(`[replyWithAI] Clicking post button (ghost cursor)...`);
    await click(foundSelector);
    await wait(2000);

    // Verify
    const toastSelector = '[data-testid="toast"]';
    if (await visible(toastSelector)) {
        return { success: true, method: methodName };
    }

    // Fallback verification: is composer closed?
    const composer = await findComposer(page);
    if (!composer) return { success: true, method: methodName };

    return { success: false, reason: 'post_failed_composer_remains', method: methodName };
}
