/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Cloud Client - Interfaces with OpenRouter API for complex reasoning.
 * Part of the Distributed Agentic Orchestration (DAO) architecture.
 * @module core/cloud-client
 */

import { createLogger } from '../core/logger.js';
import { getSettings } from '../utils/configLoader.js';
import { MultiOpenRouterClient } from '../utils/multi-api.js';
import { FreeApiRouter } from '../utils/free-api-router.js';
import { FreeOpenRouterHelper } from '../utils/free-openrouter-helper.js';
import RequestQueue from './request-queue.js';

const logger = createLogger('cloud-client.js');

/**
 * @typedef {Object} CloudRequest
 * @property {string} [prompt]
 * @property {Object} [payload]
 * @property {string} [payload.systemPrompt]
 * @property {string} [payload.userPrompt]
 * @property {string} [payload.prompt]
 * @property {Object} [context]
 * @property {string} [context.breadcrumbs]
 * @property {*} [context.state]
 * @property {string} [model]
 * @property {number} [maxTokens]
 * @property {number} [temperature]
 */

/**
 * @typedef {Object} CloudResponse
 * @property {boolean} success - Whether the request succeeded
 * @property {string} [content] - Response content from model
 * @property {Object} [data] - Parsed JSON response if applicable
 * @property {string} [error] - Error message if failed
 * @property {Object} [metadata] - Request metadata (tokens, duration, etc.)
 */

/**
 * @class CloudClient
 * @description Manages communication with OpenRouter API for cloud-based LLM reasoning.
 * Now configured via config/settings.json with multi-key fallback support.
 */
class CloudClient {
    static sharedHelper = null;
    static sharedTestResults = null;

    /**
     * Creates a new CloudClient instance
     */
    constructor() {
        /** @type {object|null} Configuration loaded from settings.json */
        this.config = null;

        /** @type {MultiOpenRouterClient|null} Multi-key fallback client */
        this.multiClient = null;

        /** @type {FreeApiRouter|null} Free API router for open_router_free_api config */
        this.freeApiRouter = null;

        /** @type {boolean} Internal toggle used by tests to force free router path */
        this.useFreeRouter = false;

        /** @type {string} OpenRouter API endpoint (single-key mode) */
        this.apiEndpoint = 'https://openrouter.ai/api/v1/chat/completions';

        /** @type {string} OpenRouter API key (single-key mode) */
        this.apiKey = '';

        /** @type {string} Default model to use */
        this.defaultModel = 'openrouter/free';

        /** @type {number} Request timeout in ms */
        this.timeout = 60000;

        /** @type {object} Request statistics */
        this.stats = {
            totalRequests: 0,
            successfulRequests: 0,
            failedRequests: 0,
            totalTokens: 0,
            totalDuration: 0,
            keyFallbacks: 0,
        };

        this.requestQueue = null;

        // Load configuration asynchronously
        this.initPromise = this._loadConfig();
    }

    isReady() {
        return (
            this.config !== null &&
            (this.multiClient !== null ||
                this.freeApiRouter !== null ||
                (this.apiKey && this.apiKey !== 'your_openrouter_api_key_here'))
        );
    }

    /**
     * Load configuration from settings.json
     * @private
     */
    async _loadConfig() {
        if (this.config) return;

        try {
            const settings = await getSettings();

            // console.log('DEBUG: CloudClient _loadConfig settings:', JSON.stringify(settings, null, 2));
            const cloudEnabled = settings?.llm?.cloud?.enabled === true;
            const freeApiEnabled = settings?.open_router_free_api?.enabled === true;

            // Return early if both cloud and free api are disabled
            if (!cloudEnabled && !freeApiEnabled) {
                if (!this._disabledLogged) {
                    logger.info(
                        '[Cloud] Both Cloud and FreeRouter are disabled, skipping initialization.'
                    );
                    this._disabledLogged = true;
                }
                this._configLoaded = true;
                return;
            }

            logger.debug(
                `[Cloud] _loadConfig called, keys in settings: ${Object.keys(settings || {}).join(', ')}`
            );
            logger.debug(`[Cloud] llm.cloud exists: ${!!settings?.llm?.cloud}`);

            this.config = settings?.llm?.cloud || {};
            logger.debug(
                `[Cloud] this.config after assignment: ${Object.keys(this.config).join(', ')}`
            );

            this.timeout = this.config.timeout || this.timeout;
            this.defaultModel = this.config.defaultModel || this.defaultModel;
            this.apiEndpoint = this.config.endpoint || this.apiEndpoint;
            const queueConfig = this.config.requestQueue || {};
            if (queueConfig.enabled === true) {
                const interval = queueConfig.interval ?? 300;
                this.requestQueue = new RequestQueue({
                    maxConcurrent: 1,
                    intervalMs: interval,
                });
            } else {
                this.requestQueue = null;
            }

            // Check for multi-provider configuration
            const providers = this.config.providers || [];
            logger.debug(`[Cloud] providers array length: ${providers.length}`);

            if (providers.length > 1) {
                // Initialize multi-key client with per-key models
                const apiKeys = providers.map((p) => p.apiKey);
                const models = providers.map((p) => p.model);

                // console.log('DEBUG: Instantiating MultiOpenRouterClient');
                this.multiClient = new MultiOpenRouterClient({
                    apiKeys,
                    models,
                    endpoint:
                        this.config.endpoint || 'https://openrouter.ai/api/v1/chat/completions',
                    timeout: this.timeout,
                    defaultModel: models[0],
                    retryDelay: this.config.retryDelay || 2000,
                });

                logger.info(
                    `CloudClient initialized with ${providers.length} API keys (fallback enabled)`
                );
                logger.debug(`Models configured: ${models.join(', ')}`);
            } else if (providers.length === 1) {
                // Single provider - use legacy single-key mode
                this.apiKey = providers[0].apiKey;
                this.defaultModel = providers[0].model || this.defaultModel;
                logger.info(`CloudClient initialized with single key, model: ${this.defaultModel}`);
            } else {
                // No providers configured
                this.apiKey = '';
                logger.warn('No cloud providers configured. Cloud client will not function.');
            }

            // Check for open_router_free_api configuration
            const freeApiConfig = settings?.open_router_free_api || {};
            if (freeApiConfig.enabled) {
                const allModels = [
                    freeApiConfig.models?.primary,
                    ...(freeApiConfig.models?.fallbacks || []),
                ].filter(Boolean);

                const allApiKeys = freeApiConfig.api_keys || [];

                // Check if we already have results (prevents duplicate tests)
                if (CloudClient.sharedHelper) {
                    const existingResults = CloudClient.sharedHelper.getResults();
                    if (existingResults && existingResults.testDuration > 0) {
                        logger.info(
                            `[Cloud] Reuse cache: ${existingResults.working.length}/${existingResults.total} OK`
                        );
                    } else if (CloudClient.sharedHelper.isTesting()) {
                        logger.info(`[Cloud] Tests already running`);
                    } else {
                        CloudClient.sharedHelper.updateConfig(allApiKeys, allModels);
                        CloudClient.sharedHelper.testAllModelsInBackground();
                    }
                } else {
                    CloudClient.sharedHelper = FreeOpenRouterHelper.getInstance({
                        apiKeys: allApiKeys,
                        models: allModels,
                        proxy: freeApiConfig.proxy?.enabled ? freeApiConfig.proxy.list : null,
                        testTimeout: 10000,
                        batchSize: 5,
                    });
                    CloudClient.sharedHelper.testAllModelsInBackground();
                }

                const optimized = CloudClient.sharedHelper.getOptimizedModelList(
                    freeApiConfig.models?.primary
                );

                this.freeApiRouter = new FreeApiRouter({
                    enabled: true,
                    apiKeys: allApiKeys,
                    primaryModel: optimized.primary || freeApiConfig.models?.primary,
                    fallbackModels:
                        optimized.fallbacks.length > 0
                            ? optimized.fallbacks
                            : freeApiConfig.models?.fallbacks || [],
                    proxyEnabled: freeApiConfig.proxy?.enabled || false,
                    proxyList: freeApiConfig.proxy?.list || [],
                    proxyFallbackToDirect: freeApiConfig.proxy?.fallback_to_direct !== false,
                    timeout: this.timeout,
                });

                const sessionInfo = this.freeApiRouter.getSessionInfo();
                logger.info(
                    `[Cloud] Router: ${sessionInfo.primaryModel} (key ${sessionInfo.apiKeyIndex}/${sessionInfo.totalApiKeys})`
                );
            }

            this._configLoaded = true;
        } catch (error) {
            this.config = null;
            this.multiClient = null;
            this.freeApiRouter = null;
            this.requestQueue = null;
            this.apiKey = '';
            logger.error(`Failed to load cloud config: ${error.message}`);
        }
    }

    /**
     * Test all free models in background
     * Returns immediately, tests run asynchronously
     * @returns {Promise<object>} Initial test status
     */
    async testFreeModels() {
        try {
            const settings = await getSettings();
            const freeApiConfig = settings?.open_router_free_api || {};

            if (!freeApiConfig.enabled) {
                logger.info('[Cloud] Free API Router not enabled, skipping model test');
                return { tested: false, reason: 'not_enabled' };
            }

            const allModels = [
                freeApiConfig.models?.primary,
                ...(freeApiConfig.models?.fallbacks || []),
            ].filter(Boolean);

            if (allModels.length === 0) {
                logger.warn('[Cloud] No models configured for testing');
                return { tested: false, reason: 'no_models' };
            }

            // Check if we need to initialize or update the helper
            if (!CloudClient.sharedHelper) {
                logger.info('[Cloud] Creating new FreeOpenRouterHelper singleton');
                CloudClient.sharedHelper = FreeOpenRouterHelper.getInstance({
                    apiKeys: freeApiConfig.api_keys || [],
                    models: allModels,
                    proxy: freeApiConfig.proxy?.enabled ? freeApiConfig.proxy.list : null,
                    testTimeout: 10000,
                    batchSize: 5,
                });
                // Start tests asynchronously
                CloudClient.sharedHelper.testAllModelsInBackground();
            } else {
                // Update config if needed
                CloudClient.sharedHelper.updateConfig(freeApiConfig.api_keys || [], allModels);

                // Only restart tests if cache is invalid/stale
                if (!CloudClient.sharedHelper.isCacheValid()) {
                    logger.info('[Cloud] Cache stale, restarting background model tests...');
                    CloudClient.sharedHelper.testAllModelsInBackground();
                } else {
                    logger.info('[Cloud] Using cached model test results');
                }
            }

            return {
                tested: true,
                status: CloudClient.sharedHelper.isTesting()
                    ? 'testing_in_progress'
                    : 'using_cached_results',
                totalModels: allModels.length,
                results: CloudClient.sharedHelper.getResults(),
            };
        } catch (error) {
            logger.error('[Cloud] Failed to start model tests:', error.message);
            return { tested: false, error: error.message };
        }
    }

    /**
     * Send a request to the cloud LLM.
     * @param {CloudRequest} request - The request object.
     * @returns {Promise<CloudResponse>} The response from the cloud model.
     */
    async sendRequest(request) {
        // Ensure config is loaded before checking
        await this._loadConfig();
        if (this.requestQueue) {
            try {
                const queued = await this.requestQueue.enqueue(() =>
                    this._sendRequestInternal(request)
                );
                return queued.data;
            } catch (error) {
                const message = error?.error || error?.message || 'Queue request failed';
                return {
                    success: false,
                    error: message,
                    metadata: { duration: error?.duration },
                };
            }
        }

        return this._sendRequestInternal(request);
    }

    async _sendRequestInternal(request) {
        const startTime = Date.now();
        this.stats.totalRequests++;

        // Check if free API router is enabled and use it
        if (this.freeApiRouter && this.freeApiRouter.isReady()) {
            return this._sendWithFreeRouter(request, startTime);
        }

        // Check if we have any API keys configured
        if (!this.multiClient && (!this.apiKey || this.apiKey === 'your_openrouter_api_key_here')) {
            this.stats.failedRequests++;
            const error = 'OpenRouter API key not configured in config/settings.json';
            logger.error(error);
            return { success: false, error };
        }

        const maxTokens = request.maxTokens || 4096;
        const temperature = request.temperature !== undefined ? request.temperature : 0.7;

        // Use multi-client if available (fallback mode)
        if (this.multiClient) {
            return this._sendWithMultiClient(request, startTime, maxTokens, temperature);
        }

        // Legacy single-key mode
        const model = request.model || this.defaultModel;

        try {
            logger.info(`[Cloud] Sending request to ${model}...`);

            // Construct messages array
            const messages = [
                {
                    role: 'system',
                    content:
                        'You are an intelligent agent executing browser automation tasks. Respond with clear, actionable JSON when requested.',
                },
                {
                    role: 'user',
                    content: this._buildPrompt(request),
                },
            ];

            // Make API request
            const response = await this._makeRequest({
                model,
                messages,
                max_tokens: maxTokens,
                temperature,
            });

            const duration = Date.now() - startTime;
            this.stats.totalDuration += duration;
            this.stats.successfulRequests++;

            // Extract response
            const content = response.choices[0]?.message?.content || '';

            // Track token usage
            if (response.usage) {
                this.stats.totalTokens += response.usage.total_tokens || 0;
            }

            logger.success(`[Cloud] Request completed in ${duration}ms`);

            // Try to parse JSON if content looks like JSON
            let data = null;
            if (content.trim().startsWith('{') || content.trim().startsWith('[')) {
                try {
                    data = JSON.parse(content);
                } catch (_e) {
                    logger.debug('[Cloud] Response is not valid JSON');
                }
            }

            return {
                success: true,
                content,
                data,
                metadata: {
                    model,
                    duration,
                    tokens: response.usage?.total_tokens || 0,
                },
            };
        } catch (error) {
            const duration = Date.now() - startTime;
            this.stats.failedRequests++;

            logger.error(`[Cloud] Request failed after ${duration}ms:`, error.message);

            return {
                success: false,
                error: error.message,
                metadata: { duration },
            };
        }
    }

    /**
     * Send request using multi-key fallback client.
     * @private
     */
    async _sendWithMultiClient(request, startTime, maxTokens, temperature) {
        // Build messages in OpenRouter format
        // Support Twitter-style: systemPrompt + userPrompt in payload
        const systemContent =
            request.payload?.systemPrompt ||
            'You are an intelligent agent executing browser automation tasks.';
        const userContent = this._buildPrompt(request);

        const messages = [
            {
                role: 'system',
                content: systemContent,
            },
            {
                role: 'user',
                content: userContent,
            },
        ];

        try {
            logger.info(`[Cloud] Sending request with multi-key fallback (per-key models)...`);

            const response = await this.multiClient.processRequest({
                messages,
                maxTokens,
                temperature,
            });

            const duration = Date.now() - startTime;

            if (response.success) {
                this.stats.successfulRequests++;
                this.stats.totalDuration += duration;

                if (response.tokens) {
                    this.stats.totalTokens += response.tokens.total || 0;
                }

                // Track key fallbacks
                if (response.keyUsed !== undefined && response.keyUsed > 0) {
                    this.stats.keyFallbacks += response.keyUsed;
                    logger.info(
                        `[Cloud] Used key ${response.keyUsed + 1} after ${response.keyUsed} fallback(s)`
                    );
                }

                logger.success(`[Cloud] Request completed in ${duration}ms`);

                // Try to parse JSON
                let data = null;
                const content = response.content || '';
                if (content.trim().startsWith('{') || content.trim().startsWith('[')) {
                    try {
                        data = JSON.parse(content);
                    } catch (_e) {
                        logger.debug('[Cloud] Response is not valid JSON');
                    }
                }

                return {
                    success: true,
                    content,
                    data,
                    metadata: {
                        model: response.model || 'unknown',
                        duration,
                        tokens: response.tokens?.total || 0,
                        keyUsed: response.keyUsed,
                        fallbackFromKey: response.keyUsed > 0,
                    },
                };
            } else {
                this.stats.failedRequests++;
                logger.error(`[Cloud] All keys failed: ${response.error}`);

                return {
                    success: false,
                    error: response.error,
                    metadata: {
                        duration,
                        keysTried: response.keysTried,
                    },
                };
            }
        } catch (error) {
            const duration = Date.now() - startTime;
            this.stats.failedRequests++;

            logger.error(`[Cloud] Multi-key request failed after ${duration}ms:`, error.message);

            return {
                success: false,
                error: error.message,
                metadata: { duration },
            };
        }
    }

    /**
     * Send request using FreeApiRouter.
     * @private
     */
    async _sendWithFreeRouter(request, startTime) {
        const maxTokens = request.maxTokens || 4096;
        const temperature = request.temperature !== undefined ? request.temperature : 0.7;

        if (this.freeApiRouter.syncWithHelper) {
            this.freeApiRouter.syncWithHelper();
        }

        const modelsInfo = this.freeApiRouter.getModelsInfo();
        if (
            modelsInfo.testedWorking.length > 0 &&
            modelsInfo.testedWorking.length < modelsInfo.allConfigured.length
        ) {
            logger.info(
                `[Cloud] Using ${modelsInfo.testedWorking.length}/${modelsInfo.allConfigured.length} tested working models`
            );
        }

        // Build messages in OpenRouter format
        const systemContent =
            request.payload?.systemPrompt ||
            'You are an intelligent agent executing browser automation tasks.';
        const userContent = this._buildPrompt(request);

        const messages = [
            {
                role: 'system',
                content: systemContent,
            },
            {
                role: 'user',
                content: userContent,
            },
        ];

        try {
            logger.info(`[Cloud] Sending request with FreeApiRouter...`);

            const response = await this.freeApiRouter.processRequest({
                messages,
                maxTokens,
                temperature,
            });

            const duration = Date.now() - startTime;

            if (response.success) {
                this.stats.successfulRequests++;
                this.stats.totalDuration += duration;

                logger.success(`[Cloud] FreeRouter request completed in ${duration}ms`);

                // Try to parse JSON
                let data = null;
                const content = response.content || '';
                if (content.trim().startsWith('{') || content.trim().startsWith('[')) {
                    try {
                        data = JSON.parse(content);
                    } catch (_e) {
                        logger.debug('[Cloud] Response is not valid JSON');
                    }
                }

                return {
                    success: true,
                    content,
                    data,
                    metadata: {
                        model: response.model,
                        duration,
                        keyUsed: response.keyUsed,
                        proxyUsed: response.proxyUsed,
                        modelFallbacks: response.modelFallbacks || 0,
                        mode: 'free-api-router',
                    },
                };
            } else {
                this.stats.failedRequests++;
                logger.error(`[Cloud] FreeRouter failed: ${response.error}`);

                return {
                    success: false,
                    error: response.error,
                    metadata: {
                        duration,
                        modelsTried: response.modelsTried,
                    },
                };
            }
        } catch (error) {
            const duration = Date.now() - startTime;
            this.stats.failedRequests++;

            logger.error(`[Cloud] FreeRouter request failed after ${duration}ms:`, error.message);

            return {
                success: false,
                error: error.message,
                metadata: { duration },
            };
        }
    }

    /**
     * Build the complete prompt with context.
     * Handles multiple prompt formats: single prompt, or system+user pair.
     * @param {CloudRequest} request - The request object.
     * @returns {string} The complete prompt.
     * @private
     */
    _buildPrompt(request) {
        // Support multiple prompt formats
        let prompt = '';

        // Format 1: Twitter style - systemPrompt + userPrompt in payload
        if (request.payload?.systemPrompt && request.payload?.userPrompt) {
            prompt = request.payload.systemPrompt + '\n\n' + request.payload.userPrompt;
        }
        // Format 2: Single prompt in payload
        else if (request.payload?.prompt) {
            prompt = request.payload.prompt;
        }
        // Format 3: Direct prompt property
        else if (request.prompt) {
            prompt = request.prompt;
        }

        // Add context if provided
        if (request.context) {
            if (request.context.breadcrumbs) {
                prompt = `Context - Recent Actions:\n${request.context.breadcrumbs}\n\n${prompt}`;
            }

            if (request.context.state) {
                prompt = `Context - Current State:\n${JSON.stringify(request.context.state, null, 2)}\n\n${prompt}`;
            }
        }

        return prompt;
    }

    /**
     * Make HTTP request to OpenRouter API.
     * @param {object} payload - The API payload.
     * @returns {Promise<object>} The API response.
     * @private
     */
    async _makeRequest(payload) {
        // If multi-client is active, delegate to it
        if (this.multiClient) {
            throw new Error('_makeRequest called but multi-client is active');
        }

        const controller = new AbortController();
        const timeoutId = setTimeout(() => controller.abort(), this.timeout);

        try {
            const response = await fetch(this.apiEndpoint, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                    Authorization: `Bearer ${this.apiKey}`,
                    'HTTP-Referer': 'https://github.com/auto-ai',
                    'X-Title': 'Auto-AI DAO Framework',
                },
                body: JSON.stringify(payload),
                signal: controller.signal,
            });

            clearTimeout(timeoutId);

            if (!response.ok) {
                const errorText = await response.text();
                throw new Error(`API error ${response.status}: ${errorText}`);
            }

            return await response.json();
        } catch (error) {
            clearTimeout(timeoutId);

            if (error.name === 'AbortError') {
                throw new Error(`Request timeout after ${this.timeout}ms`, { cause: error });
            }

            throw error;
        }
    }

    /**
     * Test connectivity to OpenRouter API.
     * @returns {Promise<boolean>} True if connection successful.
     */
    async testConnection() {
        logger.info('[Cloud] Testing OpenRouter connectivity...');

        const response = await this.sendRequest({
            prompt: 'Reply with a single word: "OK"',
            maxTokens: 10,
            temperature: 0,
        });

        if (response.success) {
            logger.success('[Cloud] Connection test successful');
            return true;
        } else {
            logger.error('[Cloud] Connection test failed:', response.error);
            return false;
        }
    }

    /**
     * Get client statistics.
     * @returns {object} Statistics object.
     */
    getStats() {
        const baseStats = {
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

        // Include freeApiRouter stats if available
        if (this.freeApiRouter && this.freeApiRouter.isReady()) {
            return {
                ...baseStats,
                freeApiRouterStats: this.freeApiRouter.getStats(),
                freeApiRouterSession: this.freeApiRouter.getSessionInfo(),
                mode: 'free-api-router',
            };
        }

        // Include multi-client stats if available
        if (this.multiClient) {
            return {
                ...baseStats,
                multiClientStats: this.multiClient.getStats(),
                mode: 'multi-key-fallback',
            };
        }

        return {
            ...baseStats,
            mode: 'single-key',
        };
    }

    /**
     * Get multi-client instance for direct access.
     * @returns {MultiOpenRouterClient|null}
     */
    getMultiClient() {
        return this.multiClient;
    }

    /**
     * Reset statistics.
     */
    resetStats() {
        this.stats = {
            totalRequests: 0,
            successfulRequests: 0,
            failedRequests: 0,
            totalTokens: 0,
            totalDuration: 0,
        };
        logger.info('[Cloud] Statistics reset');
    }
}

export default CloudClient;
