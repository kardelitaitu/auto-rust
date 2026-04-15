/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * Multi-OpenRouter API Client
 * Tries multiple API keys in sequence if one fails
 */

import { createLogger } from '../core/logger.js';

const logger = createLogger('multi-api.js');

export class MultiOpenRouterClient {
    constructor(config = {}) {
        this.apiKeys = config.apiKeys || [];
        this.models = config.models || [];
        this.endpoint = config.endpoint || 'https://openrouter.ai/api/v1/chat/completions';
        this.timeout = config.timeout || 120000;
        this.defaultModel = config.model || 'arcee-ai/trinity-large-preview:free';
        this.retryDelay = config.retryDelay || 2000;

        this.stats = {
            totalRequests: 0,
            successes: 0,
            failures: 0,
            apiKeyFallbacks: 0,
        };
    }

    async processRequest(request) {
        this.stats.totalRequests++;

        const { messages, maxTokens = 100, temperature = 0.7 } = request;

        // Try each API key in sequence
        for (let keyIndex = 0; keyIndex < this.apiKeys.length; keyIndex++) {
            const apiKey = this.apiKeys[keyIndex];
            const keyLabel = `Key${keyIndex + 1}/${this.apiKeys.length}`;

            // Use per-key model if available, otherwise use default
            const keyModel = this.models[keyIndex] || this.defaultModel;

            try {
                logger.info(`[MultiAPI] ${keyLabel} - Sending request (model: ${keyModel})...`);

                const response = await this.callAPI(apiKey, messages, {
                    maxTokens,
                    temperature,
                    model: keyModel,
                });

                this.stats.successes++;
                logger.info(`[MultiAPI] ${keyLabel} - Success!`);

                return {
                    success: true,
                    content: response.content,
                    model: response.model,
                    keyUsed: keyIndex,
                    tokens: response.tokens,
                };
            } catch (error) {
                logger.warn(`[MultiAPI] ${keyLabel} - Failed: ${error.message}`);

                // Check if error is rate limit or quota - try next key
                const isRetryable = this.isRetryableError(error);
                if (isRetryable && keyIndex < this.apiKeys.length - 1) {
                    this.stats.apiKeyFallbacks++;
                    logger.info(`[MultiAPI] ${keyLabel} - Rate limited, trying next key...`);
                    await this.sleep(this.retryDelay);
                    continue;
                }

                // If last key or non-retryable error, throw
                if (keyIndex >= this.apiKeys.length - 1) {
                    this.stats.failures++;
                    return {
                        success: false,
                        error: error.message,
                        keysTried: keyIndex + 1,
                        lastKey: keyIndex,
                    };
                }
            }
        }

        this.stats.failures++;
        return {
            success: false,
            error: 'All API keys failed',
            keysTried: this.apiKeys.length,
        };
    }

    async callAPI(apiKey, messages, options) {
        const { maxTokens, temperature, model } = options;

        const controller = new AbortController();
        const timeout = setTimeout(() => controller.abort(), this.timeout);

        try {
            const response = await fetch(this.endpoint, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                    Authorization: `Bearer ${apiKey}`,
                    'HTTP-Referer': 'http://localhost:3000',
                    'X-Title': 'Auto-AI Twitter Bot',
                },
                body: JSON.stringify({
                    model,
                    messages,
                    max_tokens: maxTokens,
                    temperature,
                    stream: false,
                }),
                signal: controller.signal,
            });

            clearTimeout(timeout);

            if (!response.ok) {
                const errorText = await response.text();
                throw new Error(`HTTP ${response.status}: ${errorText}`);
            }

            const data = await response.json();

            return {
                content: data.choices[0]?.message?.content || '',
                model: data.model,
                tokens: {
                    prompt: data.usage?.prompt_tokens || 0,
                    completion: data.usage?.completion_tokens || 0,
                    total: data.usage?.total_tokens || 0,
                },
            };
        } catch (error) {
            clearTimeout(timeout);
            throw error;
        }
    }

    isRetryableError(error) {
        const message = error.message.toLowerCase();

        // Rate limit, quota exceeded, overload
        const retryableCodes = [
            '429',
            'rate limit',
            'quota exceeded',
            'overloaded',
            'service unavailable',
            '503',
            '502',
            'timeout',
        ];

        return retryableCodes.some((code) => message.includes(code));
    }

    sleep(ms) {
        return new Promise((resolve) => setTimeout(resolve, ms));
    }

    getStats() {
        return {
            ...this.stats,
            successRate:
                this.stats.totalRequests > 0
                    ? ((this.stats.successes / this.stats.totalRequests) * 100).toFixed(1) + '%'
                    : '0%',
            keysConfigured: this.apiKeys.length,
        };
    }

    resetStats() {
        this.stats = {
            totalRequests: 0,
            successes: 0,
            failures: 0,
            apiKeyFallbacks: 0,
        };
    }
}

export default MultiOpenRouterClient;
