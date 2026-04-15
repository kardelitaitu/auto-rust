/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * Free OpenRouter Helper - Singleton for shared model testing
 * All CloudClient instances share the same helper to avoid redundant tests
 * @module utils/free-openrouter-helper
 */

import { createLogger } from '../core/logger.js';
import { createProxyAgent } from './proxy-agent.js';

const logger = createLogger('free-openrouter-helper.js');

let sharedInstance = null;

export class FreeOpenRouterHelper {
    static getInstance(options = {}) {
        if (!sharedInstance) {
            sharedInstance = new FreeOpenRouterHelper(options);
        }
        return sharedInstance;
    }

    static resetInstance() {
        sharedInstance = null;
    }

    constructor(options = {}) {
        this.apiKeys = options.apiKeys || [];
        this.models = options.models || [];
        this.proxy = options.proxy || null;
        this.endpoint = 'https://openrouter.ai/api/v1/chat/completions';
        this.testTimeout = options.testTimeout || 15000;
        this.currentKeyIndex = 0;
        this.results = null;
        this.testing = false;
        this.testStartTime = null;

        // Mutex for preventing race conditions
        this.testLock = null;

        // Parallel batch size - test multiple models concurrently for speed
        this.batchSize = options.batchSize || 5;

        // Cache TTL: 5 minutes (300000 ms)
        this.CACHE_TTL = 300000;
        this.cacheTimestamp = null;
    }

    _maskKey(key) {
        if (!key) return 'null';
        if (key.length < 8) return '***';
        return `${key.substring(0, 6)}...${key.substring(key.length - 4)}`;
    }

    _getNextApiKey() {
        if (this.apiKeys.length === 0) {
            return null;
        }
        const key = this.apiKeys[this.currentKeyIndex % this.apiKeys.length];
        this.currentKeyIndex++;
        return key;
    }

    _selectProxy() {
        if (!this.proxy || this.proxy.length === 0) {
            return null;
        }
        const index = Math.floor(Math.random() * this.proxy.length);
        return this.proxy[index];
    }

    _parseProxy(proxyString) {
        if (!proxyString) return null;

        const parts = proxyString.split(':');
        if (parts.length !== 4) {
            logger.warn(`[FreeRouterHelper] Invalid proxy format: ${proxyString}`);
            return null;
        }

        return {
            host: parts[0],
            port: parts[1],
            username: parts[2],
            password: parts[3],
        };
    }

    async _testModel(model, apiKey) {
        const startTime = Date.now();

        const testPrompt = [{ role: 'user', content: 'Reply with exactly one word: "ok"' }];

        const payload = {
            model,
            messages: testPrompt,
            max_tokens: 10,
            temperature: 0.1,
            stream: false,
            exclude_reasoning: true,
        };

        try {
            const controller = new AbortController();
            const timeoutId = setTimeout(() => controller.abort(), this.testTimeout);

            try {
                const proxyString = this._selectProxy();
                const proxy = this._parseProxy(proxyString);

                let fetchOptions = {
                    method: 'POST',
                    headers: {
                        'Content-Type': 'application/json',
                        Authorization: `Bearer ${apiKey}`,
                        'HTTP-Referer': 'https://github.com/auto-ai',
                        'X-Title': 'Auto-AI Model Tester',
                    },
                    body: JSON.stringify(payload),
                    signal: controller.signal,
                };

                if (proxy) {
                    const proxyUrl = proxy.username
                        ? `http://${proxy.username}:${proxy.password}@${proxy.host}:${proxy.port}`
                        : `http://${proxy.host}:${proxy.port}`;

                    try {
                        const agent = await createProxyAgent(proxyUrl);
                        const httpAgent = await agent.getAgent();
                        if (httpAgent) {
                            fetchOptions.agent = httpAgent;
                        }
                    } catch (proxyError) {
                        logger.debug(`[FreeRouter] Proxy failed: ${proxyError.message}`);
                    }
                }

                const response = await fetch(this.endpoint, fetchOptions);

                clearTimeout(timeoutId);

                if (!response.ok) {
                    const _errorText = await response.text();
                    throw new Error(`HTTP ${response.status}`);
                }

                const data = await response.json();
                const content = data.choices[0]?.message?.content || '';
                const duration = Date.now() - startTime;

                if (content.toLowerCase().includes('ok')) {
                    return { success: true, duration, error: null };
                } else {
                    return { success: false, duration, error: 'Unexpected response' };
                }
            } catch (error) {
                clearTimeout(timeoutId);
                throw error;
            }
        } catch (error) {
            return { success: false, duration: Date.now() - startTime, error: error.message };
        }
    }

    async testAllModelsInBackground() {
        // Acquire lock to prevent race conditions
        if (this.testing && this.testLock) {
            logger.info('[FreeRouter] Already testing, waiting...');
            await this.testLock;
            return this.results || { working: [], failed: [], total: 0, testDuration: 0 };
        }

        if (this.testing && !this.testLock) {
            await new Promise((resolve) => setTimeout(resolve, 100));
            return await this.testAllModelsInBackground();
        }

        if (this.results && this.results.testDuration > 0 && this.isCacheValid()) {
            return this.results;
        }

        if (this.models.length === 0) {
            logger.warn('[FreeRouter] No models configured');
            this.results = { working: [], failed: [], total: 0, testDuration: 0 };
            return this.results;
        }

        if (this.apiKeys.length === 0) {
            logger.warn('[FreeRouter] No API keys configured');
            this.results = { working: [], failed: [], total: 0, testDuration: 0 };
            return this.results;
        }

        const proxies =
            this.proxy && this.proxy.length > 0
                ? this.proxy.map((p) => p.split(':')[0]).join(', ')
                : 'direct';
        logger.info(`[FreeRouter] Testing ${this.endpoint} with proxies: ${proxies}`);
        logger.info(`[FreeRouter] Starting background tests (${this.models.length} models)...`);

        // Acquire lock
        this.testing = true;
        this.testLock = (async () => {
            try {
                this.results = {
                    working: [],
                    failed: [],
                    total: this.models.length,
                    testDuration: 0,
                };

                this.testStartTime = Date.now();

                let successCount = 0;

                // Process models in parallel batches
                for (let i = 0; i < this.models.length; i += this.batchSize) {
                    if (!this.testing) {
                        this.results.testDuration = Date.now() - this.testStartTime;
                        return this.results;
                    }

                    const batch = this.models.slice(i, i + this.batchSize);
                    const batchPromises = batch.map(async (model) => {
                        const apiKey = this._getNextApiKey();
                        const result = await this._testModel(model, apiKey);

                        if (result.success) {
                            this.results.working.push(model);
                            successCount++;
                        } else {
                            this.results.failed.push({
                                model,
                                error: result.error?.substring(0, 30) || 'Err',
                                duration: result.duration,
                            });
                        }
                    });

                    await Promise.all(batchPromises);

                    // Small delay between batches to avoid rate limiting
                    if (i + this.batchSize < this.models.length) {
                        await new Promise((resolve) => setTimeout(resolve, 50));
                    }
                }

                this.results.testDuration = Date.now() - this.testStartTime;
                this.testing = false;
                this.testStartTime = null;

                let resultMsg = `[FreeRouter] Done: ${successCount}/${this.models.length} OK (${this.results.testDuration}ms)`;
                logger.info(resultMsg);

                this.cacheTimestamp = Date.now();
                return this.results;
            } finally {
                this.testLock = null;
            }
        })();

        // NON-BLOCKING: Return immediately, tests run in background
        return {
            testing: true,
            message: 'Tests started in background',
            cached: false,
        };
    }

    async waitForTests(maxWait = 60000) {
        const startWait = Date.now();
        while ((this.testing || this.testLock) && Date.now() - startWait < maxWait) {
            await new Promise((resolve) => setTimeout(resolve, 100));
        }
        if (this.testing || this.testLock) {
            logger.warn('[FreeRouterHelper] Wait timeout, tests still in progress');
        }
        return this.results;
    }

    updateConfig(apiKeys, models) {
        if (apiKeys && apiKeys.length > 0) {
            this.apiKeys = apiKeys;
        }
        if (models && models.length > 0) {
            this.models = models;
            this.results = null;
        }
    }

    getResults() {
        // Check if cache is expired
        if (this.results && this.cacheTimestamp) {
            const age = Date.now() - this.cacheTimestamp;
            if (age > this.CACHE_TTL) {
                logger.info(
                    `[FreeRouterHelper] Cache expired (${Math.round(age / 1000)}s old), will refresh on next request`
                );
                // Don't clear immediately, just mark as stale
                return { ...this.results, stale: true, cacheAge: age };
            }
        }
        return this.results;
    }

    /**
     * Check if cached results are still valid
     * @returns {boolean}
     */
    isCacheValid() {
        if (!this.results || !this.cacheTimestamp) return false;
        const age = Date.now() - this.cacheTimestamp;
        return age <= this.CACHE_TTL;
    }

    /**
     * Get cache age in milliseconds
     * @returns {number|null}
     */
    getCacheAge() {
        if (!this.cacheTimestamp) return null;
        return Date.now() - this.cacheTimestamp;
    }

    isTesting() {
        return this.testing;
    }

    getQuickStatus() {
        if (this.testing) {
            return {
                status: 'testing',
                progress: `${this.results?.working?.length || 0}/${this.models.length}`,
            };
        }
        if (this.results && this.results.testDuration > 0) {
            return {
                status: 'done',
                working: this.results.working.length,
                failed: this.results.failed.length,
                total: this.results.total,
                duration: this.results.testDuration,
            };
        }
        return { status: 'idle' };
    }

    getOptimizedModelList(primary = null) {
        const working = this.results?.working || [];

        if (working.length === 0) {
            return { primary: null, fallbacks: [] };
        }

        let primaryModel = primary;
        if (!primaryModel || !working.includes(primaryModel)) {
            primaryModel = working[0];
        }

        const fallbacks = working.filter((m) => m !== primaryModel);

        return { primary: primaryModel, fallbacks };
    }
}

export default FreeOpenRouterHelper;
