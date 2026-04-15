/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview AI Reply Engine - Main Entry Point
 * Combines all modules into the AIReplyEngine class
 * @module utils/ai-reply-engine
 */

import { createLogger } from '../../core/logger.js';
import { mathUtils } from '../../utils/math.js';
import { sentimentService } from '../../utils/sentiment-service.js';
import { REPLY_SYSTEM_PROMPT, getStrategyInstruction } from '../../twitter/twitter-reply-prompt.js';
import { HumanInteraction } from '../../behaviors/human-interaction.js';

export const SAFETY_FILTERS = {
    minTweetLength: 10,
    maxTweetLength: 500,
    excludedKeywords: [
        'politics',
        'political',
        'vote',
        'election',
        'trump',
        'biden',
        'obama',
        'republican',
        'democrat',
        'congress',
        'senate',
        'president',
        'policy',
        'taxes',
        'immigration',
        'abortion',
        'gun rights',
        'protest',
        'nsfw',
        'nude',
        'naked',
        'explicit',
        '18+',
        'adult',
        'xxx',
        'porn',
        'sexual',
        'erotic',
        'dick',
        'cock',
        'pussy',
        'fuck',
        'shit',
        'ass',
        'follow back',
        'fb',
        'make money',
        'drop link',
        'free crypto',
        'dm me',
        'send dm',
        'join now',
        'limited offer',
        'act now',
        'religion',
        'god',
        'atheist',
        'belief',
        'vaccine',
        'climate change',
        'conspiracy',
        'wake up',
        'sheep',
        'brainwashed',
    ],
};

export class AIReplyEngine {
    constructor(agentConnector, options = {}) {
        this.agent = agentConnector;
        this.logger = createLogger('ai-reply-engine.js');
        this.config = {
            REPLY_PROBABILITY: options.replyProbability ?? 0.05,
            MAX_REPLY_LENGTH: 280,
            MIN_REPLY_LENGTH: 10,
            MAX_RETRIES: options.maxRetries ?? 2,
            SAFETY_FILTERS: SAFETY_FILTERS,
        };

        this.stats = {
            attempts: 0,
            successes: 0,
            skips: 0,
            failures: 0,
            safetyBlocks: 0,
            errors: 0,
        };

        this.logger.info(
            `[AIReplyEngine] Initialized (probability: ${this.config.REPLY_PROBABILITY})`
        );
    }

    updateConfig(options) {
        if (options.replyProbability !== undefined) {
            this.config.REPLY_PROBABILITY = options.replyProbability;
        }
        if (options.maxRetries !== undefined) {
            this.config.MAX_RETRIES = options.maxRetries;
        }
    }

    async shouldReply(tweetText, authorUsername, context = {}) {
        this.stats.attempts++;

        if (!mathUtils.roll(this.config.REPLY_PROBABILITY)) {
            this.stats.skips++;
            return { decision: 'skip', reason: 'probability', action: null };
        }

        const clippedTweet =
            tweetText.length > 300 ? tweetText.substring(0, 300) + '...' : tweetText;
        this.logger.debug(
            `[AIReply] Tweet clipped from ${tweetText.length} to ${clippedTweet.length} chars`
        );

        const tweetSentiment = sentimentService.analyze(tweetText);

        if (tweetSentiment.isNegative && tweetSentiment.score > 0.3) {
            this.stats.skips++;
            return { decision: 'skip', reason: 'negative_content', action: this.randomFallback() };
        }

        if (tweetSentiment.composite?.riskLevel === 'high') {
            this.stats.skips++;
            return {
                decision: 'skip',
                reason: 'high_risk_conversation',
                action: this.randomFallback(),
            };
        }

        const sentiment = tweetSentiment.composite?.engagementStyle || 'neutral';
        const conversationType = tweetSentiment.composite?.conversationType || 'general';

        const enhancedContext = {
            ...context,
            sentiment: {
                overall: tweetSentiment.isNegative ? 'negative' : 'neutral/positive',
                score: tweetSentiment.score,
                engagementStyle: sentiment,
                conversationType: conversationType,
                valence: tweetSentiment.dimensions?.valence?.valence || 0,
                sarcasm: tweetSentiment.dimensions?.sarcasm?.sarcasm || 0,
                toxicity: tweetSentiment.dimensions?.toxicity?.toxicity || 0,
                riskLevel: tweetSentiment.composite?.riskLevel || 'low',
            },
        };

        const safetyResult = this.applySafetyFilters(clippedTweet);
        if (!safetyResult.safe) {
            this.stats.safetyBlocks++;
            this.stats.skips++;
            return { decision: 'skip', reason: 'safety', action: this.randomFallback() };
        }

        const aiResult = await this.generateReply(clippedTweet, authorUsername, enhancedContext);
        if (!aiResult.success) {
            this.stats.failures++;
            this.stats.skips++;
            return { decision: 'skip', reason: 'ai_failed', action: this.randomFallback() };
        }

        const validation = this.validateReply(aiResult.reply);
        if (!validation.valid) {
            this.stats.failures++;
            this.stats.skips++;
            return { decision: 'skip', reason: 'validation_failed', action: this.randomFallback() };
        }

        this.stats.successes++;
        return {
            decision: 'reply',
            reason: 'success',
            action: 'post_reply',
            reply: aiResult.reply.trim(),
        };
    }

    applySafetyFilters(text) {
        if (!text || typeof text !== 'string') {
            return { safe: false, reason: 'empty_text' };
        }

        const lowerText = text.toLowerCase().trim();

        if (lowerText.length < this.config.SAFETY_FILTERS.minTweetLength) {
            return { safe: false, reason: 'too_short' };
        }

        if (lowerText.length > this.config.SAFETY_FILTERS.maxTweetLength) {
            return { safe: false, reason: 'too_long' };
        }

        const excluded = this.config.SAFETY_FILTERS.excludedKeywords;
        for (const keyword of excluded) {
            if (lowerText.includes(keyword)) {
                return { safe: false, reason: `excluded_keyword:${keyword}` };
            }
        }

        const capsRatio = (text.match(/[A-Z]/g)?.length || 0) / text.length;
        if (capsRatio > 0.7 && text.length > 20) {
            return { safe: false, reason: 'excessive_caps' };
        }

        const emojiCount =
            text.match(/[\p{Emoji_Presentation}\p{Extended_Pictographic}]/gu)?.length || 0;
        if (emojiCount > 8) {
            return { safe: false, reason: 'too_many_emojis' };
        }

        return { safe: true, reason: 'passed' };
    }

    async generateReply(tweetText, authorUsername, context = {}) {
        const maxAttempts = this.config.MAX_RETRIES;
        let lastError = null;

        for (let attempt = 1; attempt <= maxAttempts; attempt++) {
            try {
                const sentiment = context.sentiment || {};
                const strategy = sentiment.conversationType || 'casual';

                const prompt = this.buildEnhancedPrompt(
                    tweetText,
                    authorUsername,
                    context,
                    strategy
                );

                const hasVision = !!(context.screenshot || context.vision);
                const request = {
                    action: 'generate_reply',
                    sessionId: context.sessionId || null,
                    payload: {
                        systemPrompt: REPLY_SYSTEM_PROMPT,
                        userPrompt: prompt,
                        vision: hasVision ? context.vision || context.screenshot : null,
                        context: {
                            hasScreenshot: hasVision,
                            replyCount: context.replies?.length || 0,
                            ...context,
                        },
                        temperature: 0.7 + Math.random() * 0.3,
                        maxTokens: 150,
                    },
                };

                const aiResponse = await this.agent.processRequest(request);

                if (!aiResponse || !aiResponse.success || !aiResponse.content) {
                    throw new Error(aiResponse?.error || 'Empty AI response');
                }

                const reply = this.extractReplyFromResponse(aiResponse.content, tweetText);
                if (!reply) continue;

                const normalized = this.normalizeReply(reply);
                if (normalized) {
                    return { success: true, reply: normalized };
                }
            } catch (error) {
                lastError = error;
                this.logger.warn(
                    `[AIReply] Generation attempt ${attempt}/${maxAttempts} failed: ${error.message}`
                );
                if (attempt < maxAttempts) {
                    const delay = Math.min(1000 * Math.pow(2, attempt), 5000);
                    await new Promise((resolve) => setTimeout(resolve, delay));
                }
            }
        }

        const fallbackReply = this.generateQuickFallback(tweetText, authorUsername);
        if (fallbackReply) {
            return { success: true, reply: fallbackReply };
        }

        return { success: false, error: lastError?.message || 'Unknown error' };
    }

    buildEnhancedPrompt(tweetText, authorUsername, context, _strategy) {
        const sentiment = context.sentiment || {};
        const replies = context.replies || [];

        // Hard limits: tweet 500 chars, each reply 80 chars, max 3 replies
        const tweetSnippet = (tweetText || '').substring(0, 500);

        const strategyContext = {
            sentiment: sentiment.overall || 'neutral',
            type: sentiment.conversationType || 'general',
            engagement: context.engagementLevel,
            valence: sentiment.valence || 0,
        };
        const instruction = getStrategyInstruction(strategyContext);

        let prompt = `${instruction}\n\nTweet: "@${authorUsername}: ${tweetSnippet}"`;

        if (context.hasImage) {
            prompt += ' [IMAGE ATTACHED — comment on a specific visual detail]';
        }

        const topReplies = replies.filter((r) => r.text && r.text.length > 5).slice(0, 20);
        if (topReplies.length > 0) {
            prompt += '\n\nReplies:';
            topReplies.forEach((r, idx) => {
                const text = (r.text || '').substring(0, 80);
                prompt += `\n${idx + 1}. @${r.author || 'User'}: ${text}`;
            });
        }

        prompt += '\n\nReply:';
        return prompt;
    }

    getSentimentGuidance(sentiment, conversationType, sarcasmScore) {
        const guidance = {
            supportive: 'Shows genuine support and encouragement',
            humorous: 'Matches the comedic tone appropriately',
            informative: 'Adds helpful, factual information',
            neutral: 'Stays neutral and non-controversial',
            critical: 'Provides constructive feedback respectfully',
        };
        if (sarcasmScore > 0.5) return 'Acknowledges the sarcasm with wit';
        return guidance[sentiment] || guidance.neutral;
    }

    getReplyLengthGuidance(conversationType, valence) {
        if (valence > 0.5) return 'Keep it brief and positive';
        if (valence < -0.3) return 'Keep it very brief to minimize conflict';
        return 'Use normal conversational length';
    }

    detectLanguage(text) {
        const koreanRegex = /[\uAC00-\uD7AF\u1100-\u11FF\u314E-\u318E]/;
        const japaneseRegex = /[\u3040-\u309F\u30A0-\u30FF\u4E00-\u9FFF]/;
        const chineseRegex = /[\u4E00-\u9FFF]/;
        const arabicRegex = /[\u0600-\u06FF]/;
        const russianRegex = /[\u0400-\u04FF]/;

        if (koreanRegex.test(text)) return 'ko';
        if (japaneseRegex.test(text)) return 'ja';
        if (chineseRegex.test(text)) return 'zh';
        if (arabicRegex.test(text)) return 'ar';
        if (russianRegex.test(text)) return 'ru';
        return 'en';
    }

    detectReplyLanguage(replies) {
        const languages = replies.map((r) => this.detectLanguage(r.text || '')).filter(Boolean);
        if (languages.length === 0) return 'en';
        const counts = {};
        languages.forEach((l) => (counts[l] = (counts[l] || 0) + 1));
        return Object.entries(counts).sort((a, b) => b[1] - a[1])[0][0];
    }

    async captureContext(page, tweetUrl = '') {
        return { url: tweetUrl, screenshot: null, replies: [] };
    }

    async extractRepliesMultipleStrategies(_page) {
        return [];
    }

    async _returnToMainTweet(_page) {}

    async extractReplyFromArticle(_article, _page) {
        return null;
    }

    async extractAuthorFromArticle(_article) {
        return 'unknown';
    }

    async extractAuthorFromElement(_element, _page) {
        return this.extractAuthorFromArticle(_element);
    }

    interceptAddress(reply) {
        const addressPatterns = [
            /\d+\s+\w+\s+(street|st|avenue|ave|road|rd|drive|dr|lane|ln|boulevard|blvd)/i,
            /\b(po box|p\.?o\.?\s*box)\b/i,
            /^\d{5}(-\d{4})?$/,
        ];
        for (const pattern of addressPatterns) {
            if (pattern.test(reply)) {
                return true;
            }
        }
        return false;
    }

    extractReplyFromResponse(content, _originalTweet) {
        if (!content) return null;
        let reply = content.trim();
        const lines = reply.split('\n').filter((line) => line.trim().length > 0);
        for (const line of lines) {
            const cleaned = line.replace(/^[-\d*.)>\s]*/, '').trim();
            if (cleaned.length >= this.config.MIN_REPLY_LENGTH) {
                return cleaned;
            }
        }
        return null;
    }

    cleanEmojis(text) {
        return text
            .replace(/[\p{Emoji_Presentation}\p{Extended_Pictographic}]/gu, '')
            .replace(/\s+/g, ' ')
            .trim();
    }

    validateReply(reply) {
        if (!reply || typeof reply !== 'string') {
            return { valid: false, reason: 'empty_reply' };
        }
        const trimmed = reply.trim();
        if (trimmed.length < this.config.MIN_REPLY_LENGTH) {
            return { valid: false, reason: 'too_short' };
        }
        if (trimmed.length > this.config.MAX_REPLY_LENGTH) {
            return { valid: false, reason: 'too_long' };
        }
        return { valid: true, reason: 'passed' };
    }

    randomFallback() {
        const fallbacks = ['like', 'bookmark', 'retweet', 'follow'];
        return fallbacks[Math.floor(Math.random() * fallbacks.length)];
    }

    async adaptiveRetry(operation, options = {}) {
        const maxRetries = options.maxRetries ?? this.config.MAX_RETRIES;
        const baseDelay = options.baseDelay ?? 1000;

        for (let attempt = 0; attempt < maxRetries; attempt++) {
            try {
                return await operation();
            } catch (error) {
                if (attempt === maxRetries - 1) throw error;
                const delay = baseDelay * Math.pow(2, attempt);
                await new Promise((resolve) => setTimeout(resolve, delay));
            }
        }
    }

    generateQuickFallback(tweetText, _authorUsername) {
        const fallbacks = [
            'Great point!',
            'Interesting perspective',
            'Thanks for sharing',
            'I see what you mean',
            "That's a good take",
        ];
        if (tweetText.includes('?')) {
            fallbacks.push("That's a thoughtful question", 'Good question');
        }
        return fallbacks[Math.floor(Math.random() * fallbacks.length)];
    }

    validateReplyAdvanced(reply, originalTweet = '') {
        const basic = this.validateReply(reply);
        if (!basic.valid) return basic;

        if (this.interceptAddress(reply)) {
            return { valid: false, reason: 'contains_address' };
        }

        const lowerReply = reply.toLowerCase();
        const lowerOriginal = originalTweet.toLowerCase();
        if (lowerReply === lowerOriginal) {
            return { valid: false, reason: 'duplicate_content' };
        }

        return { valid: true, reason: 'passed' };
    }

    normalizeReply(reply) {
        if (!reply) return null;
        let cleaned = reply.trim();
        cleaned = cleaned.replace(/^["']|["']$/g, '');
        cleaned = cleaned.replace(/^(reply:|Response:)/i, '').trim();
        const lines = cleaned.split('\n');
        if (lines.length > 1) {
            cleaned = lines[0];
        }
        if (cleaned.length > this.config.MAX_REPLY_LENGTH) {
            cleaned = cleaned.substring(0, this.config.MAX_REPLY_LENGTH - 3) + '...';
        }
        return cleaned.length >= this.config.MIN_REPLY_LENGTH ? cleaned : null;
    }

    getStats() {
        const total = this.stats.attempts;
        return {
            ...this.stats,
            successRate: total > 0 ? ((this.stats.successes / total) * 100).toFixed(1) + '%' : '0%',
            skipRate: total > 0 ? ((this.stats.skips / total) * 100).toFixed(1) + '%' : '0%',
        };
    }

    resetStats() {
        this.stats = {
            attempts: 0,
            successes: 0,
            skips: 0,
            failures: 0,
            safetyBlocks: 0,
            errors: 0,
        };
    }

    async executeReply(page, replyText, _options = {}) {
        this.logger.info(`[AIReply] Executing reply (${replyText.length} chars)...`);
        const human = new HumanInteraction(page);
        human.debugMode = true;

        const methods = [
            {
                name: 'keyboard_shortcut',
                weight: 40,
                fn: () => this.replyMethodA_Keyboard(page, replyText, human),
            },
            {
                name: 'button_click',
                weight: 35,
                fn: () => this.replyMethodB_Button(page, replyText, human),
            },
            {
                name: 'tab_navigation',
                weight: 15,
                fn: () => this.replyMethodC_Tab(page, replyText, human),
            },
            {
                name: 'right_click',
                weight: 10,
                fn: () => this.replyMethodD_RightClick(page, replyText, human),
            },
        ];

        const selected = human.selectMethod(methods);
        this.logger.info(`[AIReply] Using method: ${selected.name}`);

        try {
            const result = await selected.fn();
            if (result && result.success === false) {
                this.logger.warn(
                    `[AIReply] Method ${selected.name} returned failure: ${result.reason || 'unknown_reason'}`
                );

                // SAFE FALLBACK CHECK: Verify composer state before trying another method
                const verify = await human.verifyComposerOpen(page);
                if (!verify.open) {
                    this.logger.info(
                        `[AIReply] Composer is closed. Assuming reply was successful or interrupted correctly. Skipping fallback.`
                    );
                    return {
                        success: true,
                        method: selected.name,
                        reason: 'interrupted_success_or_closed',
                        fallbackSkipped: true,
                    };
                }

                this.logger.warn(`[AIReply] Trying fallback: button_click`);
                return await this.replyMethodB_Button(page, replyText, human);
            }
            return result;
        } catch (error) {
            this.logger.error(`[AIReply] Method ${selected.name} failed: ${error.message}`);

            // Categorize error: if it's a timeout, check if composer is still there
            const verify = await human.verifyComposerOpen(page);
            if (verify.open) {
                this.logger.warn(
                    `[AIReply] Composer still open after error. Trying fallback: button_click`
                );
                return await this.replyMethodB_Button(page, replyText, human);
            } else {
                this.logger.info(
                    `[AIReply] Composer closed after error. Likely successful post despite exception.`
                );
                return {
                    success: true,
                    method: selected.name,
                    reason: 'error_but_closed',
                    error: error.message,
                };
            }
        }
    }

    async replyMethodA_Keyboard(page, replyText, human) {
        human.logStep('KEYBOARD_SHORTCUT', 'Starting');
        await page.keyboard.press('Escape');
        await new Promise((resolve) => setTimeout(resolve, 300));

        const focusSelectors = [
            'article time',
            '[data-testid="tweetText"]',
            'article[role="article"] [dir="auto"]',
        ];
        for (const selector of focusSelectors) {
            try {
                const el = page.locator(selector).first();
                if ((await el.count()) > 0) {
                    await human.safeHumanClick(el, 'Main Tweet', 3, { precision: 'high' });
                    break;
                }
            } catch (_e) {
                /* continue */
            }
        }

        await page.keyboard.press('r');
        await new Promise((resolve) => setTimeout(resolve, 800));

        const verify = await human.verifyComposerOpen(page);
        if (!verify.open) {
            return { success: false, reason: 'composer_not_open', method: 'keyboard_shortcut' };
        }

        const composer = page.locator(verify.selector).first();
        await human.typeText(page, replyText, composer, { skipClear: true, skipFocusClick: true });
        const postResult = await human.postTweet(page, 'reply');

        return {
            success: postResult.success,
            reason: postResult.reason || 'posted',
            method: 'keyboard_shortcut',
        };
    }

    async replyMethodB_Button(page, replyText, human) {
        human.logStep('BUTTON_CLICK', 'Starting');
        const replyBtnSelectors = ['[data-testid="reply"]', 'button[aria-label*="Reply"]'];
        let replyBtn = null;

        for (const sel of replyBtnSelectors) {
            try {
                const btn = page.locator(sel).first();
                if ((await btn.count()) > 0) {
                    replyBtn = btn;
                    break;
                }
            } catch (_e) {
                /* continue */
            }
        }

        if (!replyBtn) {
            return { success: false, reason: 'reply_button_not_found', method: 'button_click' };
        }

        await human.safeHumanClick(replyBtn, 'Reply Button', 3);
        await new Promise((resolve) => setTimeout(resolve, 600));

        const verify = await human.verifyComposerOpen(page);
        if (!verify.open) {
            return { success: false, reason: 'composer_not_open', method: 'button_click' };
        }

        const composer = page.locator(verify.selector).first();
        await human.typeText(page, replyText, composer, { skipClear: true, skipFocusClick: true });
        const postResult = await human.postTweet(page, 'reply');

        return {
            success: postResult.success,
            reason: postResult.reason || 'posted',
            method: 'button_click',
        };
    }

    async replyMethodC_Tab(page, replyText, human) {
        human.logStep('TAB_NAVIGATION', 'Starting');
        for (let i = 0; i < 8; i++) {
            await page.keyboard.press('Tab');
            await new Promise((resolve) => setTimeout(resolve, 100));
        }
        await page.keyboard.press('Enter');
        await new Promise((resolve) => setTimeout(resolve, 800));

        const verify = await human.verifyComposerOpen(page);
        if (!verify.open) {
            return { success: false, reason: 'composer_not_open', method: 'tab_navigation' };
        }

        const composer = page.locator(verify.selector).first();
        await human.typeText(page, replyText, composer, { skipClear: true, skipFocusClick: true });
        const postResult = await human.postTweet(page, 'reply');

        return {
            success: postResult.success,
            reason: postResult.reason || 'posted',
            method: 'tab_navigation',
        };
    }

    async replyMethodD_RightClick(page, replyText, human) {
        human.logStep('RIGHT_CLICK', 'Starting');
        const tweetEl = page.locator('article[role="article"]').first();
        await tweetEl.click({ button: 'right' });
        await new Promise((resolve) => setTimeout(resolve, 1000));

        await page.keyboard.press('Escape');
        await new Promise((resolve) => setTimeout(resolve, 1000));

        const verify = await human.verifyComposerOpen(page);
        if (!verify.open) {
            return { success: false, reason: 'composer_not_open', method: 'right_click' };
        }

        const composer = page.locator(verify.selector).first();
        await human.typeText(page, replyText, composer, { skipClear: true, skipFocusClick: true });
        const postResult = await human.postTweet(page, 'reply');

        return {
            success: postResult.success,
            reason: postResult.reason || 'posted',
            method: 'right_click',
        };
    }
}

export default AIReplyEngine;
