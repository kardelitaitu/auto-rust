/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * Free API Router for OpenRouter
 * Handles API key rotation, model cascading fallbacks, and proxy routing
 * With: Config Validation, Circuit Breaker, Rate Limit Tracking, Request Deduplication, Model Performance Tracking, Quick Timeout
 * @module utils/free-api-router
 */

import { createLogger } from '../core/logger.js';
import { createProxyAgent } from './proxy-agent.js';
import CircuitBreaker from '../core/circuit-breaker.js';
import { RateLimitTracker } from './rate-limit-tracker.js';
import { RequestDedupe } from './request-dedupe.js';
import { ModelPerfTracker } from './model-perf-tracker.js';
import { ConfigValidator } from './config-validator.js';
import { ApiKeyTimeoutTracker } from './api-key-timeout-tracker.js';
// import { FreeOpenRouterHelper } from './free-openrouter-helper.js';
import { RouterError, ProxyError, classifyHttpError } from './errors.js';
// import { RateLimitError, ModelError } from './errors.js';

const logger = createLogger('free-api-router.js');

let sharedHelper = null;

export function getSharedHelper() {
    return sharedHelper;
}

export function setSharedHelper(helper) {
    sharedHelper = helper;
}

export class FreeApiRouter {
    constructor(options = {}) {
        this.config = {
            enabled: options.enabled ?? false,
            apiKeys: options.apiKeys || [],
            models: {
                primary: options.primaryModel || 'anthropic/claude-3.5-sonnet:free',
                fallbacks: options.fallbackModels || [],
            },
            proxy: {
                enabled: options.proxyEnabled ?? false,
                fallbackToDirect: options.proxyFallbackToDirect ?? true,
                list: options.proxyList || [],
            },
        };

        this.endpoint = 'https://openrouter.ai/api/v1/chat/completions';

        this.defaultTimeout = options.timeout || 60000;
        this.quickTimeout = options.quickTimeout || 20000;

        this.browserId = options.browserId || 'default';
        this.taskId = options.taskId || 'default';
        this.sessionId = `${this.browserId}:${this.taskId}`;

        this.sessionApiKey = null;
        this.sessionApiKeyIndex = -1;
        this.currentProxyIndex = -1;

        this.stats = {
            totalRequests: 0,
            successes: 0,
            failures: 0,
            circuitBreaks: 0,
            rateLimitHits: 0,
            dedupeHits: 0,
            quickTimeouts: 0,
        };

        if (this.config.enabled) {
            this._initModules();
            this._selectSessionApiKey();
            this._logInitialization();
        }
    }

    _hash(str) {
        let hash = 0;
        for (let i = 0; i < str.length; i++) {
            const char = str.charCodeAt(i);
            hash = (hash << 5) - hash + char;
            hash = hash & hash;
        }
        return Math.abs(hash);
    }

    // Cache for hash results to avoid recomputing
    _hashCache = new Map();

    _getCachedHash(str) {
        if (!this._hashCache.has(str)) {
            this._hashCache.set(str, this._hash(str));
        }
        return this._hashCache.get(str);
    }

    _selectSessionApiKey() {
        if (this.config.apiKeys.length === 0) {
            logger.warn('[FreeRouter] No API keys configured');
            return;
        }

        const sessionKey = `${this.browserId}:${this.taskId}`;
        const hash = this._getCachedHash(sessionKey);
        this.sessionApiKeyIndex = hash % this.config.apiKeys.length;
        this.sessionApiKey = this.config.apiKeys[this.sessionApiKeyIndex];

        logger.info(
            `[FreeRouter] Session ${this.sessionId} -> API Key ${this.sessionApiKeyIndex + 1}/${this.config.apiKeys.length}`
        );
    }

    setTask(browserId, taskId) {
        const oldSessionId = this.sessionId;
        this.browserId = browserId || this.browserId;
        this.taskId = taskId || this.taskId;
        this.sessionId = `${this.browserId}:${this.taskId}`;

        if (oldSessionId !== this.sessionId) {
            logger.info(`[FreeRouter] Session changed: ${oldSessionId} -> ${this.sessionId}`);
            this._selectSessionApiKey();
        }
    }

    _initModules() {
        this.circuitBreaker = new CircuitBreaker({
            failureThreshold: 5,
            resetTimeout: 60000,
            halfOpenSuccessThreshold: 1,
        });

        this.rateLimitTracker = new RateLimitTracker({
            cacheDuration: 60000,
            warningThreshold: 0.2,
        });

        this.requestDedupe = new RequestDedupe({
            ttl: 30000,
            maxSize: 1000,
            enabled: true,
        });

        this.modelPerfTracker = new ModelPerfTracker({
            windowSize: 100,
            minSamples: 5,
        });

        this.apiKeyTimeoutTracker = new ApiKeyTimeoutTracker({
            defaultTimeout: this.defaultTimeout,
            quickTimeout: this.quickTimeout,
            slowThreshold: 15000,
            failureThreshold: 3,
        });

        this.configValidator = new ConfigValidator();
    }

    _logInitialization() {
        logger.info(`[FreeRouter] Initialized for session: ${this.sessionId}`);
        logger.info(
            `[FreeRouter] API Keys: ${this.config.apiKeys.length}, Selected: ${this.sessionApiKeyIndex + 1}`
        );
        logger.info(`[FreeRouter] Primary model: ${this.config.models.primary}`);
        logger.info(`[FreeRouter] Fallback models: ${this.config.models.fallbacks.length}`);
        logger.info(
            `[FreeRouter] Timeout: default=${this.defaultTimeout}ms, quick=${this.quickTimeout}ms`
        );

        if (this.config.proxy.enabled) {
            logger.info(`[FreeRouter] Proxy enabled: ${this.config.proxy.list.length} proxies`);
        }

        logger.info(
            '[FreeRouter] Advanced features: CircuitBreaker, RateLimitTracker, RequestDedupe, ModelPerfTracker, ApiKeyTimeoutTracker'
        );
    }

    _selectRequestProxy() {
        if (!this.config.proxy.enabled || this.config.proxy.list.length === 0) {
            return null;
        }

        this.currentProxyIndex = Math.floor(Math.random() * this.config.proxy.list.length);
        return this.config.proxy.list[this.currentProxyIndex];
    }

    _parseProxy(proxyString) {
        if (!proxyString) return null;

        const parts = proxyString.split(':');
        if (parts.length < 2) {
            logger.warn(`[FreeRouter] Invalid proxy format: ${proxyString}`);
            return null;
        }

        return {
            host: parts[0],
            port: parts[1],
            username: parts[2] || null,
            password: parts[3] || null,
        };
    }

    _maskKey(key) {
        if (!key) return 'null';
        if (key.length < 8) return '***';
        return `${key.substring(0, 6)}...${key.substring(key.length - 4)}`;
    }

    _maskProxy(proxyString) {
        if (!proxyString) return null;
        // Format: host:port or host:port:username:password
        const parts = proxyString.split(':');
        if (parts.length >= 4) {
            return `${parts[0]}:${parts[1]}:${parts[2]}:***`;
        }
        return proxyString;
    }

    async processRequest(request) {
        this.stats.totalRequests++;

        if (!this.config.enabled) {
            return { success: false, error: 'Free API router not enabled' };
        }

        const { messages, maxTokens = 100, temperature = 0.7 } = request;

        const startTime = Date.now();

        const dedupeResult = this.requestDedupe.check(
            messages,
            this.config.models.primary,
            maxTokens,
            temperature
        );

        if (dedupeResult.hit) {
            this.stats.dedupeHits++;
            this.stats.successes++;

            const warningStatus = this.rateLimitTracker.getWarningStatus(this.sessionApiKey);
            logger.info(`[FreeRouter] Dedupe hit, returning cached response`);

            return {
                success: true,
                content: dedupeResult.response,
                model: this.config.models.primary,
                keyUsed: this.sessionApiKeyIndex,
                fromCache: true,
                warningStatus: warningStatus,
            };
        }

        logger.info(`[FreeRouter] processRequest: calling _tryModelWithKey`);

        // Build list of models to try - prioritize working models from test results
        let modelsToTry;

        // Get tested working models if available
        const testResults = sharedHelper?.getResults();
        const configuredModels = [
            this.config.models.primary,
            ...this.config.models.fallbacks,
        ].filter(Boolean);

        if (testResults && testResults.working && testResults.working.length > 0) {
            // Use tested working models first, in random order
            let workingModels = [...testResults.working];

            // If we have fewer than 3 working models, add more from configured models
            if (workingModels.length < 3) {
                logger.warn(
                    `[FreeRouter] Only ${workingModels.length} working models from tester, adding more from config`
                );
                const additionalModels = configuredModels.filter((m) => !workingModels.includes(m));
                workingModels = [...workingModels, ...additionalModels];
            }

            // Shuffle working models to distribute load
            for (let i = workingModels.length - 1; i > 0; i--) {
                const j = Math.floor(Math.random() * (i + 1));
                [workingModels[i], workingModels[j]] = [workingModels[j], workingModels[i]];
            }
            modelsToTry = workingModels;
            logger.info(
                `[FreeRouter] Using ${workingModels.length} models (shuffled) - ${testResults.working.length} tested working + ${workingModels.length - testResults.working.length} from config`
            );
        } else {
            // Fallback to configured models
            modelsToTry = configuredModels;
            logger.info(
                `[FreeRouter] No test results available, using ${modelsToTry.length} configured models`
            );
        }

        // Track models that got rate limited (429) to skip them
        const rateLimitedModels = new Set();
        const maxRetries = 3;
        let retryCount = 0;

        for (
            let modelIndex = 0;
            modelIndex < modelsToTry.length && retryCount < maxRetries;
            modelIndex++
        ) {
            const model = modelsToTry[modelIndex];

            // Skip if this model was already rate limited in this request
            if (rateLimitedModels.has(model)) {
                logger.info(
                    `[FreeRouter] Skipping ${model} - already rate limited in this request`
                );
                continue;
            }

            const circuitCheck = this.circuitBreaker.check(model, this.sessionApiKey);

            if (!circuitCheck.allowed) {
                this.stats.circuitBreaks++;
                logger.warn(`[FreeRouter] Circuit open for ${model}, skipping`);
                continue;
            }

            const rateLimitStatus = this.rateLimitTracker.getWarningStatus(this.sessionApiKey);

            if (rateLimitStatus === 'exhausted') {
                this.stats.rateLimitHits++;
                logger.warn(`[FreeRouter] Rate limit exhausted for API key, trying fallback model`);
                continue;
            }

            retryCount++;
            logger.info(`[FreeRouter] Attempt ${retryCount}/${maxRetries}: Trying model ${model}`);

            const result = await this._tryModelWithKey(
                model,
                messages,
                maxTokens,
                temperature,
                startTime
            );

            if (result.success) {
                const duration = Date.now() - startTime;

                this.circuitBreaker.recordSuccess(model, this.sessionApiKey);
                this.modelPerfTracker.trackSuccess(model, duration, this.sessionApiKey);
                this.apiKeyTimeoutTracker.trackRequest(this.sessionApiKey, duration, true);
                this.requestDedupe.set(messages, model, result.content, maxTokens, temperature);
                this.rateLimitTracker.trackRequest(this.sessionApiKey, model);

                this.stats.successes++;

                return {
                    success: true,
                    content: result.content,
                    model: model,
                    keyUsed: this.sessionApiKeyIndex,
                    proxyUsed: result.proxyUsed,
                    modelFallbacks: modelIndex,
                    retryCount: retryCount,
                    duration,
                    warningStatus: rateLimitStatus,
                    rateLimitedModels: Array.from(rateLimitedModels),
                };
            }

            const duration = Date.now() - startTime;
            logger.warn(`[FreeRouter] Model ${model} failed: ${result.error}`);

            // Check if it's a server error (429, 503, 500, etc.) - these are retryable with different models
            const isServerError =
                result.error &&
                (result.error.includes('429') ||
                    result.error.includes('503') ||
                    result.error.includes('500') ||
                    result.error.includes('502') ||
                    result.error.includes('504'));

            if (isServerError) {
                if (result.error.includes('429')) {
                    rateLimitedModels.add(model);
                    logger.warn(
                        `[FreeRouter] Model ${model} rate limited (429), will skip in subsequent retries`
                    );
                } else {
                    logger.warn(
                        `[FreeRouter] Model ${model} server error (${result.error.match(/\d{3}/)?.[0] || 'unknown'}), marking for skip`
                    );
                    rateLimitedModels.add(model);
                }
                // Temporarily record circuit breaker failure to avoid hammering
                this.circuitBreaker.recordFailure(model, this.sessionApiKey);
            } else {
                this.circuitBreaker.recordFailure(model, this.sessionApiKey);
            }

            this.modelPerfTracker.trackFailure(model, result.error, this.sessionApiKey);
            this.apiKeyTimeoutTracker.trackRequest(this.sessionApiKey, duration, false);

            if (
                result.usedProxy &&
                this.config.proxy.enabled &&
                this.config.proxy.fallbackToDirect
            ) {
                logger.info('[FreeRouter] Proxy failed, trying direct connection...');

                const directResult = await this._tryModelDirect(
                    model,
                    messages,
                    maxTokens,
                    temperature,
                    startTime
                );

                if (directResult.success) {
                    const directDuration = Date.now() - startTime;

                    this.circuitBreaker.recordSuccess(model, this.sessionApiKey);
                    this.modelPerfTracker.trackSuccess(model, directDuration, this.sessionApiKey);
                    this.apiKeyTimeoutTracker.trackRequest(
                        this.sessionApiKey,
                        directDuration,
                        true
                    );
                    this.requestDedupe.set(
                        messages,
                        model,
                        directResult.content,
                        maxTokens,
                        temperature
                    );

                    this.stats.successes++;
                    this.stats.directFallbacks++;

                    return {
                        success: true,
                        content: directResult.content,
                        model: model,
                        keyUsed: this.sessionApiKeyIndex,
                        proxyUsed: false,
                        modelFallbacks: modelIndex,
                        directFallbackUsed: true,
                        retryCount: retryCount,
                        duration: directDuration,
                    };
                }

                const directDuration = Date.now() - startTime;
                logger.warn(
                    `[FreeRouter] Direct connection also failed for ${model}: ${directResult.error}`
                );
                this.circuitBreaker.recordFailure(model, this.sessionApiKey);
                this.modelPerfTracker.trackFailure(model, directResult.error, this.sessionApiKey);
                this.apiKeyTimeoutTracker.trackRequest(this.sessionApiKey, directDuration, false);
            }

            // Log status before continuing to next model
            if (retryCount < maxRetries && modelIndex < modelsToTry.length - 1) {
                logger.info(
                    `[FreeRouter] Moving to next model. Progress: ${retryCount}/${maxRetries} attempts used, ${modelsToTry.length - modelIndex - 1} models remaining`
                );
            }
        }

        this.stats.failures++;

        return {
            success: false,
            error: 'All models and fallbacks exhausted after 3 retries',
            modelsTried: retryCount,
            rateLimitedModels: Array.from(rateLimitedModels),
        };
    }

    async _tryModelWithKey(model, messages, maxTokens, temperature, _startTime) {
        logger.info(`[FreeRouter] _tryModelWithKey: model=${model}`);
        const proxyString = this._selectRequestProxy();
        logger.info(`[FreeRouter] _tryModelWithKey: proxy=${this._maskProxy(proxyString)}`);
        const proxy = this._parseProxy(proxyString);

        const payload = {
            model,
            messages,
            max_tokens: maxTokens,
            temperature,
            stream: false,
            exclude_reasoning: true,
        };

        const requestTimeout = this.apiKeyTimeoutTracker.getTimeoutForKey(this.sessionApiKey);

        try {
            if (proxy) {
                return await this._callThroughProxy(proxy, payload, requestTimeout);
            } else {
                return await this._callDirect(payload, requestTimeout);
            }
        } catch (error) {
            return { success: false, error: error.message, usedProxy: !!proxy };
        }
    }

    async _callDirect(payload, timeout) {
        logger.info(`[FreeRouter] _callDirect: starting request to ${payload.model}`);
        const controller = new AbortController();
        const timeoutId = setTimeout(() => {
            controller.abort();
            this.stats.quickTimeouts++;
        }, timeout);

        try {
            const response = await fetch(this.endpoint, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                    Authorization: `Bearer ${this.sessionApiKey}`,
                    'HTTP-Referer': 'https://github.com/auto-ai',
                    'X-Title': 'Auto-AI Free Router',
                },
                body: JSON.stringify(payload),
                signal: controller.signal,
            });

            clearTimeout(timeoutId);

            if (!response.ok) {
                const errorText = await response.text();
                throw classifyHttpError(response.status, errorText, {
                    endpoint: this.endpoint,
                    model: payload.model,
                });
            }

            const data = await response.json();

            const message = data.choices[0]?.message;
            let content = message?.content || '';

            // DEBUG: Log raw API response to debug empty responses
            // Check if reasoning_content has the actual response (some models put it there)
            const reasoningContent = message?.reasoning_content || '';

            if (!content && reasoningContent) {
                logger.warn(
                    `[FreeRouter] ⚠️ Content empty but reasoning_content has ${reasoningContent.length} chars - using reasoning_content`
                );
                content = reasoningContent;
            }

            // Log FULL response (not just first 500 chars) when debugging
            if (!content) {
                logger.warn(`[FreeRouter] ⚠️ EMPTY RESPONSE from model ${payload.model}`);
                logger.warn(`[FreeRouter] Full API Response: ${JSON.stringify(data, null, 2)}`);
                logger.warn(`[FreeRouter] Message object: ${JSON.stringify(message, null, 2)}`);
                logger.warn(`[FreeRouter] All choices: ${JSON.stringify(data.choices, null, 2)}`);
            } else {
                logger.debug(
                    `[FreeRouter] Response received (${content.length} chars): ${content.substring(0, 200)}...`
                );
            }

            if (reasoningContent) {
                logger.debug(
                    `[FreeRouter] Reasoning content excluded by API (${reasoningContent.length} chars)`
                );
            }

            return {
                success: true,
                content: content,
            };
        } catch (error) {
            clearTimeout(timeoutId);
            // If it's already an AppError, rethrow it
            if (error.name && error.name.includes('Error') && error.code) {
                throw error;
            }
            // Otherwise wrap it
            throw new RouterError(error.message, { endpoint: this.endpoint }, error);
        }
    }

    async _callThroughProxy(proxy, payload, timeout) {
        const proxyUrl = proxy.username
            ? `http://${proxy.username}:${proxy.password}@${proxy.host}:${proxy.port}`
            : `http://${proxy.host}:${proxy.port}`;

        const controller = new AbortController();
        const timeoutId = setTimeout(() => {
            controller.abort();
            this.stats.quickTimeouts++;
        }, timeout);

        try {
            const agent = await createProxyAgent(proxyUrl);
            const httpAgent = await agent.getAgent();

            if (!httpAgent) {
                throw new ProxyError('Failed to create proxy agent', {
                    proxy: `${proxy.host}:${proxy.port}`,
                });
            }

            const fetchOptions = {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                    Authorization: `Bearer ${this.sessionApiKey}`,
                    'HTTP-Referer': 'https://github.com/auto-ai',
                    'X-Title': 'Auto-AI Free Router',
                },
                body: JSON.stringify(payload),
                signal: controller.signal,
                agent: httpAgent,
            };

            const response = await fetch(this.endpoint, fetchOptions);

            clearTimeout(timeoutId);

            if (!response.ok) {
                const errorText = await response.text();
                throw classifyHttpError(response.status, errorText, {
                    endpoint: this.endpoint,
                    model: payload.model,
                    proxy: `${proxy.host}:${proxy.port}`,
                });
            }

            const data = await response.json();

            const message = data.choices[0]?.message;
            let content = message?.content || '';

            // DEBUG: Log raw API response to debug empty responses
            // Check if reasoning_content has the actual response (some models put it there)
            const reasoningContent = message?.reasoning_content || '';

            if (!content && reasoningContent) {
                logger.warn(
                    `[FreeRouter] ⚠️ Content empty but reasoning_content has ${reasoningContent.length} chars via proxy - using reasoning_content`
                );
                content = reasoningContent;
            }

            // Log FULL response (not just first 500 chars) when debugging
            if (!content) {
                logger.warn(
                    `[FreeRouter] ⚠️ EMPTY RESPONSE from model ${payload.model} via proxy ${proxy.host}`
                );
                logger.warn(`[FreeRouter] Full API Response: ${JSON.stringify(data, null, 2)}`);
                logger.warn(`[FreeRouter] Message object: ${JSON.stringify(message, null, 2)}`);
                logger.warn(`[FreeRouter] All choices: ${JSON.stringify(data.choices, null, 2)}`);
            } else {
                logger.debug(
                    `[FreeRouter] Response received (${content.length} chars): ${content.substring(0, 200)}...`
                );
            }

            if (reasoningContent) {
                logger.debug(
                    `[FreeRouter] Reasoning content excluded by API (${reasoningContent.length} chars)`
                );
            }

            return {
                success: true,
                content: content,
                proxy: `${proxy.host}:${proxy.port}`,
            };
        } catch (error) {
            clearTimeout(timeoutId);
            throw error;
        }
    }

    async _tryModelDirect(model, messages, maxTokens, temperature, _startTime) {
        const payload = {
            model,
            messages,
            max_tokens: maxTokens,
            temperature,
            stream: false,
        };

        const requestTimeout = this.apiKeyTimeoutTracker.getTimeoutForKey(this.sessionApiKey);
        return await this._callDirect(payload, requestTimeout);
    }

    async validateConfig(settings) {
        return this.configValidator.validateConfig(settings);
    }

    async refreshRateLimits() {
        if (this.sessionApiKey) {
            return await this.rateLimitTracker.refreshKey(this.sessionApiKey);
        }
        return null;
    }

    getSessionInfo() {
        return {
            sessionId: this.sessionId,
            browserId: this.browserId,
            taskId: this.taskId,
            apiKeyIndex: this.sessionApiKeyIndex + 1,
            totalApiKeys: this.config.apiKeys.length,
            apiKeyMask: this._maskKey(this.sessionApiKey),
            primaryModel: this.config.models.primary,
            fallbackCount: this.config.models.fallbacks.length,
            proxyEnabled: this.config.proxy.enabled,
            proxyCount: this.config.proxy.list.length,
            timeout: {
                default: this.defaultTimeout + 'ms',
                quick: this.quickTimeout + 'ms',
            },
        };
    }

    getStats() {
        return {
            router: {
                totalRequests: this.stats.totalRequests,
                successes: this.stats.successes,
                failures: this.stats.failures,
                circuitBreaks: this.stats.circuitBreaks,
                rateLimitHits: this.stats.rateLimitHits,
                dedupeHits: this.stats.dedupeHits,
                quickTimeouts: this.stats.quickTimeouts,
                successRate:
                    this.stats.totalRequests > 0
                        ? ((this.stats.successes / this.stats.totalRequests) * 100).toFixed(1) + '%'
                        : '0%',
            },
            circuitBreaker: this.circuitBreaker.getStats(),
            rateLimitTracker: this.rateLimitTracker.getStats(),
            requestDedupe: this.requestDedupe.getStats(),
            modelPerfTracker: this.modelPerfTracker.getStats(),
            apiKeyTimeoutTracker: this.apiKeyTimeoutTracker.getStats(),
        };
    }

    getDetailedStats() {
        return {
            session: this.getSessionInfo(),
            router: this.stats,
            circuitBreakerStates: this.circuitBreaker.getAllStates(),
            rateLimitStatus: this.rateLimitTracker.getCacheStatus(),
            modelPerformance: this.modelPerfTracker.getAllStats(),
            bestModel: this.modelPerfTracker.getBestModel(
                [this.config.models.primary, ...this.config.models.fallbacks],
                this.sessionApiKey
            ),
        };
    }

    resetStats() {
        this.stats = {
            totalRequests: 0,
            successes: 0,
            failures: 0,
            circuitBreaks: 0,
            rateLimitHits: 0,
            dedupeHits: 0,
            quickTimeouts: 0,
        };

        this.circuitBreaker.reset();
        this.rateLimitTracker.invalidateCache();
        this.requestDedupe.clear();
        this.modelPerfTracker.reset();
        this.apiKeyTimeoutTracker.reset();

        logger.info('[FreeRouter] All stats reset');
    }

    isReady() {
        return this.config.enabled && this.sessionApiKey && this.config.apiKeys.length > 0;
    }

    syncWithHelper() {
        if (!sharedHelper) {
            return false;
        }

        const results = sharedHelper.getResults();
        if (!results || results.working.length === 0) {
            return false;
        }

        const workingModels = results.working;
        const primaryModel = this.config.models.primary;

        if (workingModels.includes(primaryModel)) {
            this.config.models.primary = primaryModel;
            this.config.models.fallbacks = workingModels.filter((m) => m !== primaryModel);
        } else {
            this.config.models.primary = workingModels[0];
            this.config.models.fallbacks = workingModels.slice(1);
        }

        logger.info(
            `[FreeRouter] Synced with helper: primary=${this.config.models.primary}, fallbacks=${this.config.models.fallbacks.length}`
        );
        return true;
    }

    getModelsInfo() {
        const results = sharedHelper?.getResults();
        return {
            primary: this.config.models.primary,
            fallbacks: this.config.models.fallbacks,
            testedWorking: results?.working || [],
            testedFailed: results?.failed?.map((f) => f.model) || [],
            totalTested: results?.total || 0,
            allConfigured: [this.config.models.primary, ...this.config.models.fallbacks].filter(
                Boolean
            ),
        };
    }
}

export default FreeApiRouter;
