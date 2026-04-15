/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview AI Reply Engine - Config Module
 * Constructor, configuration, and stats methods
 * @module utils/ai-reply-engine/config
 */

import { createLogger } from '../../core/logger.js';

// logger - unused in this module

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
}

export default AIReplyEngine;
