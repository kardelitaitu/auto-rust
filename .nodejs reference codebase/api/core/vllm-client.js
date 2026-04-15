/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview vLLM Client - OpenAI-compatible interface for vLLM servers.
 * Part of the Distributed Agentic Orchestration (DAO) architecture.
 * @module core/vllm-client
 */

import { createLogger } from '../core/logger.js';
import { getSettings } from '../utils/configLoader.js';

const logger = createLogger('vllm-client.js');

/**
 * @class VLLMClient
 * @description Client for vLLM servers with OpenAI-compatible API.
 */
class VLLMClient {
    constructor() {
        this.config = null;
        this.endpoint = '';
        this.model = '';
        this.timeout = 120000;
        this.isEnabled = false;

        this.stats = {
            totalRequests: 0,
            successfulRequests: 0,
            failedRequests: 0,
            totalDuration: 0,
        };

        this._loadConfig();
    }

    /**
     * Load configuration from settings.json
     * @private
     */
    async _loadConfig() {
        try {
            const settings = await getSettings();
            const config = settings.llm?.vllm || {};

            this.isEnabled = config.enabled === true;
            this.endpoint = config.endpoint || 'http://localhost:8000/v1';
            this.model = config.model || 'meta-llama/Llama-3.3-70B-Instruct';
            this.timeout = config.timeout || 120000;

            if (this.isEnabled) {
                logger.info(`[VLLM] Client initialized: ${this.endpoint} (model: ${this.model})`);
            } else {
                logger.info('[VLLM] Client is disabled');
            }
        } catch (error) {
            logger.error('[VLLM] Failed to load config:', error.message);
        }
    }

    /**
     * Send a request to vLLM server
     * @param {object} request - Request with prompt, systemPrompt, userPrompt, etc.
     * @returns {Promise<object>} Response with { success, content, metadata }
     */
    async sendRequest(request) {
        if (!this.isEnabled) {
            return { success: false, error: 'vLLM client disabled' };
        }

        await this._loadConfig();
        this.stats.totalRequests++;

        const startTime = Date.now();

        try {
            const payload = this._buildPayload(request);
            logger.debug(`[VLLM] Sending request to ${this.endpoint}...`);

            const controller = new AbortController();
            const timeoutId = setTimeout(() => controller.abort(), this.timeout);

            const response = await fetch(this.endpoint, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify(payload),
                signal: controller.signal,
            });

            clearTimeout(timeoutId);

            if (!response.ok) {
                const errorText = await response.text();
                throw new Error(`vLLM API error ${response.status}: ${errorText}`);
            }

            const data = await response.json();
            const duration = Date.now() - startTime;

            this.stats.totalDuration += duration;
            this.stats.successfulRequests++;

            const content = data.choices?.[0]?.message?.content || '';

            logger.success(`[VLLM] Request completed in ${duration}ms`);

            return {
                success: true,
                content,
                metadata: {
                    model: this.model,
                    duration,
                    tokens: data.usage?.total_tokens || 0,
                },
            };
        } catch (error) {
            const duration = Date.now() - startTime;
            this.stats.failedRequests++;
            this.stats.totalDuration += duration;

            if (error.name === 'AbortError') {
                logger.error(`[VLLM] Request timed out after ${this.timeout}ms`);
                return { success: false, error: 'Request timeout', metadata: { duration } };
            }

            logger.error(`[VLLM] Request failed: ${error.message}`);
            return { success: false, error: error.message, metadata: { duration } };
        }
    }

    /**
     * Build OpenAI-compatible payload for vLLM
     * @private
     */
    _buildPayload(request) {
        const messages = [];

        if (request.systemPrompt) {
            messages.push({ role: 'system', content: request.systemPrompt });
        }

        if (request.prompt) {
            messages.push({ role: 'user', content: request.prompt });
        } else if (request.userPrompt) {
            messages.push({ role: 'user', content: request.userPrompt });
        }

        return {
            model: this.model,
            messages,
            temperature: request.temperature || 0.7,
            max_tokens: request.maxTokens || request.max_tokens || 2048,
            stream: false,
        };
    }

    /**
     * Test connectivity to vLLM server
     * @returns {Promise<boolean>}
     */
    async testConnection() {
        if (!this.isEnabled) {
            logger.warn('[VLLM] Cannot test - client is disabled');
            return false;
        }

        try {
            logger.info('[VLLM] Testing connectivity...');

            const response = await this.sendRequest({
                prompt: 'Reply with a single word: "OK"',
                maxTokens: 5,
                temperature: 0,
            });

            if (response.success) {
                logger.success('[VLLM] Connection test successful');
                return true;
            } else {
                logger.error('[VLLM] Connection test failed:', response.error);
                return false;
            }
        } catch (error) {
            logger.error('[VLLM] Connection test error:', error.message);
            return false;
        }
    }

    /**
     * Get client statistics
     * @returns {object}
     */
    getStats() {
        return {
            ...this.stats,
            avgDuration:
                this.stats.totalRequests > 0
                    ? Math.round(this.stats.totalDuration / this.stats.totalRequests)
                    : 0,
            successRate:
                this.stats.totalRequests > 0
                    ? ((this.stats.successfulRequests / this.stats.totalRequests) * 100).toFixed(
                          2
                      ) + '%'
                    : '0%',
        };
    }

    /**
     * Reset statistics
     */
    resetStats() {
        this.stats = {
            totalRequests: 0,
            successfulRequests: 0,
            failedRequests: 0,
            totalDuration: 0,
        };
        logger.info('[VLLM] Statistics reset');
    }
}

export default VLLMClient;
