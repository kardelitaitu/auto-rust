/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview AI Reply Engine - Decision Module
 * shouldReply, applySafetyFilters
 * @module utils/ai-reply-engine/decision
 */

import { mathUtils } from '../../utils/math.js';
import { sentimentService } from '../../utils/sentiment-service.js';
import {
    getStrategyInstruction,
    getReplyLengthGuidance,
    getSentimentGuidance,
    REPLY_SYSTEM_PROMPT,
} from '../../twitter/twitter-reply-prompt.js';

export async function shouldReply(engine, tweetText, authorUsername, context = {}) {
    engine.stats.attempts++;

    if (!mathUtils.roll(engine.config.REPLY_PROBABILITY)) {
        engine.stats.skips++;
        return {
            decision: 'skip',
            reason: 'probability',
            action: null,
        };
    }

    const clippedTweet = tweetText.length > 300 ? tweetText.substring(0, 300) + '...' : tweetText;
    engine.logger.debug(
        `[AIReply] Tweet clipped from ${tweetText.length} to ${clippedTweet.length} chars`
    );

    const tweetSentiment = sentimentService.analyze(tweetText);

    engine.logger.info(`[AIReply] Sentiment Analysis:`);
    engine.logger.info(
        `[AIReply]   - Overall: ${tweetSentiment.isNegative ? 'NEGATIVE' : 'NEUTRAL/POSITIVE'} (score: ${tweetSentiment.score.toFixed(2)})`
    );
    engine.logger.info(
        `[AIReply]   - Valence: ${tweetSentiment.dimensions?.valence?.valence?.toFixed(2) || 'N/A'}`
    );
    engine.logger.info(
        `[AIReply]   - Arousal: ${tweetSentiment.dimensions?.arousal?.arousal?.toFixed(2) || 'N/A'}`
    );
    engine.logger.info(
        `[AIReply]   - Dominance: ${tweetSentiment.dimensions?.dominance?.dominance?.toFixed(2) || 'N/A'}`
    );
    engine.logger.info(
        `[AIReply]   - Sarcasm: ${tweetSentiment.dimensions?.sarcasm?.sarcasm?.toFixed(2) || 'N/A'}`
    );
    engine.logger.info(
        `[AIReply]   - Toxicity: ${tweetSentiment.dimensions?.toxicity?.toxicity?.toFixed(2) || 'N/A'}`
    );
    engine.logger.info(`[AIReply]   - Risk Level: ${tweetSentiment.composite?.riskLevel || 'N/A'}`);
    engine.logger.info(
        `[AIReply]   - Engagement Style: ${tweetSentiment.composite?.engagementStyle || 'N/A'}`
    );
    engine.logger.info(
        `[AIReply]   - Conversation Type: ${tweetSentiment.composite?.conversationType || 'N/A'}`
    );

    if (tweetSentiment.isNegative && tweetSentiment.score > 0.3) {
        engine.stats.skips++;
        engine.logger.warn(
            `[AIReply] Skipping negative content (score: ${tweetSentiment.score.toFixed(2)})`
        );
        return {
            decision: 'skip',
            reason: 'negative_content',
            action: randomFallback(engine),
        };
    }

    if (tweetSentiment.composite?.riskLevel === 'high') {
        engine.stats.skips++;
        engine.logger.warn(`[AIReply] Skipping high-risk conversation`);
        return {
            decision: 'skip',
            reason: 'high_risk_conversation',
            action: randomFallback(engine),
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

    const safetyResult = applySafetyFilters(engine, clippedTweet);
    if (!safetyResult.safe) {
        engine.stats.safetyBlocks++;
        engine.stats.skips++;
        engine.logger.debug(`[AIReply] Safety block: ${safetyResult.reason}`);
        return {
            decision: 'skip',
            reason: 'safety',
            action: randomFallback(engine),
        };
    }

    const aiResult = await generateReply(engine, clippedTweet, authorUsername, enhancedContext);

    if (!aiResult.success) {
        engine.stats.failures++;
        engine.stats.skips++;
        return {
            decision: 'skip',
            reason: 'ai_failed',
            action: randomFallback(engine),
        };
    }

    const validation = validateReply(engine, aiResult.reply);
    if (!validation.valid) {
        engine.stats.failures++;
        engine.stats.skips++;
        engine.logger.debug(`[AIReply] Reply validation failed: ${validation.reason}`);
        return {
            decision: 'skip',
            reason: 'validation_failed',
            action: randomFallback(engine),
        };
    }

    engine.stats.successes++;
    engine.logger.info(`[AIReply] Generated reply: "${aiResult.reply.substring(0, 50)}..."`);

    return {
        decision: 'reply',
        reason: 'success',
        action: 'post_reply',
        reply: aiResult.reply.trim(),
    };
}

export function applySafetyFilters(engine, text) {
    if (!text || typeof text !== 'string') {
        return { safe: false, reason: 'empty_text' };
    }

    const lowerText = text.toLowerCase().trim();

    if (lowerText.length < engine.config.SAFETY_FILTERS.minTweetLength) {
        return { safe: false, reason: 'too_short' };
    }

    if (lowerText.length > engine.config.SAFETY_FILTERS.maxTweetLength) {
        return { safe: false, reason: 'too_long' };
    }

    const excluded = engine.config.SAFETY_FILTERS.excludedKeywords;
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

function randomFallback(_engine) {
    const fallbacks = ['like', 'bookmark', 'retweet', 'follow'];
    return fallbacks[Math.floor(Math.random() * fallbacks.length)];
}

function validateReply(engine, reply) {
    if (!reply || typeof reply !== 'string') {
        return { valid: false, reason: 'empty_reply' };
    }

    const trimmed = reply.trim();

    if (trimmed.length < engine.config.MIN_REPLY_LENGTH) {
        return { valid: false, reason: 'too_short' };
    }

    if (trimmed.length > engine.config.MAX_REPLY_LENGTH) {
        return { valid: false, reason: 'too_long' };
    }

    return { valid: true, reason: 'passed' };
}

async function generateReply(engine, tweetText, authorUsername, context = {}) {
    const maxAttempts = engine.config.MAX_RETRIES;
    let lastError = null;

    const { screenshot = null } = context;

    for (let attempt = 1; attempt <= maxAttempts; attempt++) {
        try {
            const sentiment = context.sentiment || {};
            const strategy = sentiment.conversationType || 'casual';

            const prompt = buildEnhancedPrompt(
                engine,
                tweetText,
                authorUsername,
                context,
                strategy
            );

            const request = {
                action: 'generate_reply',
                sessionId: (engine.agent && engine.agent.sessionId) || 'reply-engine',
                payload: {
                    systemPrompt: '',
                    userPrompt: prompt,
                    vision: screenshot,
                    temperature: 0.7 + Math.random() * 0.3,
                    maxTokens: 150,
                    priority: engine.config?.priority || 0,
                    context: {
                        hasScreenshot: !!screenshot,
                        replyCount: context.replies ? context.replies.length : 0,
                    },
                },
            };

            const aiResponse = await engine.agent.processRequest(request);
            const responseText = aiResponse?.content || aiResponse?.text;

            if (!aiResponse || !responseText) {
                throw new Error('Empty AI response');
            }

            const reply = extractReplyFromResponse(engine, responseText, tweetText);

            if (!reply) {
                engine.logger.warn(`[AIReply] Could not extract reply from AI response`);
                continue;
            }

            const normalized = normalizeReply(engine, reply);

            if (normalized) {
                return { success: true, reply: normalized };
            }
        } catch (error) {
            lastError = error;
            engine.logger.warn(
                `[AIReply] Generation attempt ${attempt}/${maxAttempts} failed: ${error.message}`
            );

            if (attempt < maxAttempts) {
                const delay = Math.min(1000 * Math.pow(2, attempt), 5000);
                await new Promise((resolve) => setTimeout(resolve, delay));
            }
        }
    }

    const fallbackReply = generateQuickFallback(engine, tweetText, authorUsername);
    if (fallbackReply) {
        return { success: true, reply: fallbackReply };
    }

    return { success: false, error: lastError?.message || 'Unknown error' };
}

function buildEnhancedPrompt(engine, tweetText, authorUsername, context, strategy) {
    const sentiment = context.sentiment || {};
    const replies = context.replies || [];

    const instruction = getStrategyInstruction(strategy);

    const hasReplies = replies && replies.length > 0;
    const repliesContext = hasReplies
        ? `\n\nRecent replies to this tweet:\n${replies
              .slice(0, 3)
              .map((r) => `- @${r.author}: ${r.text}`)
              .join('\n')}`
        : '';

    const prompt = `${REPLY_SYSTEM_PROMPT}

${instruction}

Tweet to reply to: "${tweetText}"
Author: @${authorUsername}${repliesContext}

Generate a reply that:
- Is natural and conversational
- ${getReplyLengthGuidance(sentiment.conversationType, sentiment.valence)}
- ${getSentimentGuidance(sentiment.engagementStyle, sentiment.conversationType, sentiment.sarcasm)}
- Adds value to the conversation
- Is 1-2 sentences maximum

Reply:`;

    return prompt;
}

function extractReplyFromResponse(engine, content, _originalTweet) {
    if (!content) return null;
    let reply = content.trim();
    const lines = reply.split('\n').filter((line) => line.trim().length > 0);
    for (const line of lines) {
        const cleaned = line.replace(/^[-\d*.)>\s]*/, '').trim();
        if (cleaned.length >= engine.config.MIN_REPLY_LENGTH) {
            return cleaned;
        }
    }
    return null;
}

function normalizeReply(engine, reply) {
    if (!reply) return null;
    let cleaned = reply.trim();
    cleaned = cleaned.replace(/^["']|["']$/g, '');
    cleaned = cleaned.replace(/^(reply:|Response:)/i, '').trim();
    const lines = cleaned.split('\n');
    if (lines.length > 1) {
        cleaned = lines[0];
    }
    if (cleaned.length > engine.config.MAX_REPLY_LENGTH) {
        cleaned = cleaned.substring(0, engine.config.MAX_REPLY_LENGTH - 3) + '...';
    }
    return cleaned.length >= engine.config.MIN_REPLY_LENGTH ? cleaned : null;
}

function generateQuickFallback(_engine, tweetText, _authorUsername) {
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
