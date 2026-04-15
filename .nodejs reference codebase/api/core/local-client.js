/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Local Client - Router for local LLM providers (vLLM, Ollama, Docker).
 * Part of the Distributed Agentic Orchestration (DAO) architecture.
 * @module core/local-client
 */

import { createLogger } from '../core/logger.js';
import { getSettings } from '../utils/configLoader.js';
import VLLMClient from './vllm-client.js';
import OllamaClient from './ollama-client.js';

const logger = createLogger('local-client.js');

/**
 * @class LocalClient
 * @description Facade for local LLM interactions. Routes to specific provider implementation.
 * Routing: vLLM (if enabled) → Ollama (if enabled)
 */
class LocalClient {
    constructor() {
        this.vllmClient = null;
        this.ollamaClient = null;
        this.vllmEnabled = false;
        this.ollamaEnabled = false;

        this.stats = {
            totalRequests: 0,
            successfulRequests: 0,
            failedRequests: 0,
            totalDuration: 0,
            vllmRequests: 0,
            ollamaRequests: 0,
        };

        this._loadConfig();
    }

    /**
     * Load configuration and initialize providers
     * @private
     */
    async _loadConfig() {
        if (this._configLoaded) return;

        try {
            const settings = await getSettings();
            const localConfig = settings.llm?.local || {};
            const vllmConfig = settings.llm?.vllm || {};

            this.vllmEnabled = vllmConfig.enabled === true;
            this.ollamaEnabled = localConfig.enabled === true;

            if (this.vllmEnabled && !this.vllmClient) {
                this.vllmClient = new VLLMClient();
                logger.info('[Local] vLLM client initialized');
            }

            if (this.ollamaEnabled && !this.ollamaClient) {
                this.ollamaClient = new OllamaClient();
                await this.ollamaClient.initialize();
                logger.info('[Local] Ollama client initialized');
            }

            if (!this.vllmEnabled && !this.ollamaEnabled) {
                logger.info('[Local] All local clients are disabled');
            }

            this._configLoaded = true;
        } catch (error) {
            logger.error('[Local] Failed to load config:', error.message);
        }
    }

    /**
     * Send a request to local LLM providers.
     * Routing: vLLM (if enabled) → Ollama (if enabled)
     * @param {object} request - The request object.
     * @returns {Promise<object>} The response.
     */
    async sendRequest(request) {
        await this._loadConfig();

        if (!this.vllmEnabled && !this.ollamaEnabled) {
            return { success: false, error: 'All local clients disabled' };
        }

        this.stats.totalRequests++;
        const startTime = Date.now();
        let lastError = null;

        // Try vLLM first if enabled
        if (this.vllmEnabled && this.vllmClient) {
            logger.debug('[Local] Trying vLLM first...');
            this.stats.vllmRequests++;

            try {
                const result = await this.vllmClient.sendRequest(request);

                if (result.success) {
                    const duration = Date.now() - startTime;
                    this.stats.totalDuration += duration;
                    this.stats.successfulRequests++;
                    logger.success(`[Local] vLLM response in ${duration}ms`);
                    return {
                        ...result,
                        metadata: {
                            ...result.metadata,
                            routedTo: 'vllm',
                        },
                    };
                }

                lastError = result.error;
                logger.warn(`[Local] vLLM failed: ${lastError}`);
            } catch (error) {
                lastError = error.message;
                logger.warn(`[Local] vLLM exception: ${lastError}`);
            }
        }

        // Try Ollama if enabled
        if (this.ollamaEnabled && this.ollamaClient) {
            logger.debug('[Local] Falling back to Ollama...');
            this.stats.ollamaRequests++;

            try {
                const result = await this.ollamaClient.generate(request);

                if (result.success) {
                    const duration = Date.now() - startTime;
                    this.stats.totalDuration += duration;
                    this.stats.successfulRequests++;
                    logger.success(`[Local] Ollama response in ${duration}ms`);
                    return {
                        ...result,
                        metadata: {
                            ...result.metadata,
                            routedTo: 'ollama',
                        },
                    };
                }

                lastError = result.error;
                logger.warn(`[Local] Ollama failed: ${lastError}`);
            } catch (error) {
                lastError = error.message;
                logger.warn(`[Local] Ollama exception: ${lastError}`);
            }
        }

        // All local providers failed
        const duration = Date.now() - startTime;
        this.stats.failedRequests++;
        this.stats.totalDuration += duration;

        logger.error(`[Local] All local providers failed. Last error: ${lastError}`);
        return {
            success: false,
            error: lastError || 'All local providers failed',
            metadata: {
                duration,
                providersTried: ['vllm', 'ollama'].filter((p) =>
                    p === 'vllm' ? this.vllmEnabled : this.ollamaEnabled
                ),
            },
        };
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
            vllmEnabled: this.vllmEnabled,
            ollamaEnabled: this.ollamaEnabled,
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
            vllmRequests: 0,
            ollamaRequests: 0,
        };
        if (this.vllmClient) this.vllmClient.resetStats();
        if (this.ollamaClient) this.ollamaClient.resetStats();
        logger.info('[Local] Statistics reset');
    }
}

export default LocalClient;
