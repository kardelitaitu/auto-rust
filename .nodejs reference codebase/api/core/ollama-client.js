/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Ollama Client - Native integration with Ollama API.
 * Handles text and vision requests to standard Ollama endpoints.
 * @module core/ollama-client
 */

import { createLogger } from '../core/logger.js';
import { getSettings } from '../utils/configLoader.js';
import { ensureOllama } from '../utils/local-ollama-manager.js';
import { exec } from 'child_process';

const logger = createLogger('ollama-client.js');

/**
 * @class OllamaClient
 * @description Native client for interacting with Ollama API (local or remote).
 */
class OllamaClient {
    constructor() {
        this.baseUrl = 'http://localhost:11434';
        this.model = 'llava:latest'; // Default fallback
        this.timeout = 180000; // Increased to 3 minutes for vision
        this.config = null;
        this._warmedUp = false;
        this._recentOllamaListAt = 0;
        this._warmupInFlight = null;
    }

    /**
     * Initialize client with configuration
     */
    async initialize() {
        if (this._initializing) {
            return await this._initializing;
        }

        if (this.config) {
            return;
        }

        this._initializing = (async () => {
            try {
                const settings = await getSettings();
                const localConfig = settings.llm?.local || {};

                // Allow override of endpoint and model
                this.baseUrl = localConfig.endpoint || 'http://localhost:11434';
                // Clean up endpoint if it has /api/generate
                this.baseUrl = this.baseUrl
                    .replace(/\/api\/generate$/, '')
                    .replace(/\/api\/chat$/, '');

                this.model = localConfig.model || 'llama3.2-vision';
                const desiredTimeout = localConfig.timeout || 60000;
                if (desiredTimeout < 60000) {
                    logger.warn(
                        `[Ollama] Timeout ${desiredTimeout}ms is low for model load; using 60000ms minimum.`
                    );
                }
                this.timeout = Math.max(desiredTimeout, 60000);

                // Ensure service is running
                await ensureOllama();

                logger.info(`[Ollama] Initialized: ${this.baseUrl} (Model: ${this.model})`);
                this.config = localConfig;
            } catch (error) {
                logger.warn('[Ollama] Failed to load config, using defaults:', error.message);
            } finally {
                this._initializing = null;
            }
        })();

        return await this._initializing;
    }

    /**
     * Send a generation request to Ollama
     * @param {object} request - Request parameters
     * @returns {Promise<object>} - Response data
     */
    async generate(request) {
        await this.initialize();

        // Track request count for debugging
        this._requestCount = (this._requestCount || 0) + 1;
        const requestNum = this._requestCount;
        const startTime = Date.now();

        // Log EVERY request for debugging
        const requestType = request.vision || request.images ? 'VISION' : 'TEXT';
        logger.info(
            `[Ollama] 🔔 REQUEST #${requestNum} START: ${requestType} request to ${this.model}`
        );

        // Pre-flight check: verify Ollama is reachable before making request (using cache)
        const { isOllamaRunning } = await import('../utils/local-ollama-manager.js');
        const ollamaReady = await isOllamaRunning(); // Removed forced true bypass
        if (!ollamaReady) {
            const duration = Date.now() - startTime;
            logger.error(`[Ollama] Service not ready at ${this.baseUrl}. Cannot process request.`);
            return { success: false, error: 'Ollama service not ready', duration };
        }

        // Check if this is a vision request - use chat endpoint for LLaVA
        let isVision = !!request.vision || !!request.images;

        // Respect config setting - disable vision if explicitly set to false
        if (this.config && this.config.vision === false && isVision) {
            logger.debug(
                `[Ollama] Vision disabled in config for model ${this.model}, ignoring image data`
            );
            isVision = false;
        }

        if (!this._warmedUp) {
            logger.info(`[Ollama] Warming up ${this.model}...`);
            if (!this._warmupInFlight) {
                this._warmupInFlight = this._warmupModel()
                    .then(() => {
                        this._warmedUp = true;
                    })
                    .catch((e) => {
                        logger.warn(`[Ollama] Warmup failed: ${e.message}`);
                    })
                    .finally(() => {
                        this._warmupInFlight = null;
                    });
            }
            await this._warmupInFlight;
        }

        // For vision models like llava, we MUST use /api/chat
        if (isVision) {
            return this._chatRequest(request, startTime, requestNum);
        }

        // For text-only, try generate endpoint first (faster for text)
        return this._generateRequest(request, startTime, requestNum);
    }

    /**
     * Warmup the model by making a simple request
     */
    async _warmupModel() {
        const isVisionModel =
            this.model.toLowerCase().includes('llava') ||
            this.model.toLowerCase().includes('vision');
        const endpoint = isVisionModel
            ? `${this.baseUrl}/api/chat`
            : `${this.baseUrl}/api/generate`;
        const payload = isVisionModel
            ? {
                  model: this.model,
                  messages: [{ role: 'user', content: 'hi' }],
                  stream: false,
                  options: { temperature: 0.1, num_predict: 1 },
              }
            : {
                  model: this.model,
                  prompt: 'hi',
                  stream: false,
                  options: { temperature: 0.1, num_predict: 1 },
              };

        const controller = new AbortController();
        const warmupTimeoutMs = Math.min(this.timeout, 30000);
        const timeoutId = setTimeout(() => controller.abort(), warmupTimeoutMs);

        try {
            const response = await fetch(endpoint, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(payload),
                signal: controller.signal,
            });
            clearTimeout(timeoutId);

            if (response.ok) {
                logger.info(`[Ollama] Model warmup complete`);
            }
        } catch (e) {
            clearTimeout(timeoutId);
            throw e;
        }
    }

    /**
     * Use /api/chat endpoint (required for LLaVA vision models)
     */
    async _chatRequest(request, startTime, requestNum = '?') {
        const endpoint = `${this.baseUrl}/api/chat`;

        // Convert image to base64 if needed
        let base64Image = null;
        if (request.vision) {
            if (Buffer.isBuffer(request.vision)) {
                base64Image = request.vision.toString('base64');
            } else if (typeof request.vision === 'string') {
                base64Image = request.vision;
            } else {
                base64Image = Buffer.from(request.vision).toString('base64');
            }
        }

        // Build chat messages array - this is what LLaVA expects
        const messages = [];

        // Add system message if present (handle both system and systemPrompt)
        const systemPrompt = request.system || request.systemPrompt;
        if (systemPrompt) {
            messages.push({
                role: 'system',
                content: systemPrompt,
            });
        }

        // For Ollama LLaVA, use the format: [INST] text [/INST]
        // Images can be passed as base64 in the prompt for older versions
        // or using the images array for newer versions
        let promptText = request.prompt;

        // Format for LLaVA's instruction template
        if (base64Image) {
            // Include image reference in the prompt for LLaVA
            promptText = `<image>\n${request.prompt}`;
        }

        const userMessage = {
            role: 'user',
            content: promptText,
        };

        // Add images to the message for Ollama /api/chat endpoint
        if (base64Image) {
            userMessage.images = [base64Image];
        }

        messages.push(userMessage);

        // Build payload
        const payload = {
            model: this.model,
            messages,
            stream: false,
            options: {
                temperature: request.temperature || 0.7,
                num_predict: request.maxTokens || 2048,
            },
        };

        logger.debug(`[Ollama] Using chat endpoint for ${this.model}...`);

        try {
            const controller = new AbortController();
            const timeoutId = setTimeout(() => controller.abort(), this.timeout);

            const response = await fetch(endpoint, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(payload),
                signal: controller.signal,
            });

            clearTimeout(timeoutId);

            if (!response.ok) {
                const errorText = await response.text();
                throw new Error(`Ollama API error ${response.status}: ${errorText}`);
            }

            const data = await response.json();
            const duration = Date.now() - startTime;

            let content = data.message?.content || data.response || '';

            // Post-processing to ensure no mentions or hashtags for Twitter tasks
            if (
                content &&
                (request.prompt?.includes('Tweet from') ||
                    request.systemPrompt?.includes('Twitter'))
            ) {
                // Remove mentions, hashtags, quotes
                let cleaned = content
                    .replace(/@\w+/g, '')
                    .replace(/#\w+/g, '')
                    .replace(/["']/g, '');

                // Only remove emojis if NOT explicitly allowed by prompt instructions
                // Strategies in twitter-reply-prompt.js use "EMOJI" keyword when allowing them
                const allowEmojis =
                    (request.prompt && request.prompt.includes('EMOJI')) ||
                    (request.systemPrompt && request.systemPrompt.includes('EMOJI'));

                if (!allowEmojis) {
                    cleaned = cleaned.replace(
                        /[\u{1F600}-\u{1F64F}\u{1F300}-\u{1F5FF}\u{1F680}-\u{1F6FF}\u{2600}-\u{26FF}\u{2700}-\u{27BF}\u{1F900}-\u{1F9FF}\u{1F1E6}-\u{1F1FF}]/gu,
                        ''
                    );
                }

                content = cleaned.trim();

                // Hard length limit: Take only the first sentence or first 100 chars
                if (
                    content.length > 100 ||
                    content.includes('.') ||
                    content.includes('!') ||
                    content.includes('?')
                ) {
                    const sentences = content.split(/[.!?]+/);
                    if (sentences.length > 0 && sentences[0].trim().length > 0) {
                        let punctuation = '';
                        if (content.includes('?')) {
                            punctuation = '?';
                        } else if (content.includes('.') || content.includes('!')) {
                            // 80% chance to delete the punctuation, 20% to use '.'
                            punctuation = Math.random() < 0.8 ? '' : '.';
                        }
                        content = sentences[0].trim() + punctuation;

                        // NEW: 15% chance for all-lowercase (human-like casual typing)
                        if (Math.random() < 0.15) {
                            content = content.toLowerCase();
                        }

                        // NEW: Apply additional humanization (abbreviations, slang)
                        content = this.applyHumanization(content);

                        // NEW: Apply realistic typos (very low probability)
                        content = this.applyTypos(content);
                    }
                }

                logger.debug(`[Ollama] Cleaned response: ${content}`);
            }

            logger.success(`[Ollama] 🔔 REQUEST #${requestNum} COMPLETED in ${duration}ms`);

            return {
                success: true,
                content: content,
                model: data.model,
                duration: data.total_duration,
                metadata: {
                    eval_count: data.eval_count,
                    eval_duration: data.eval_duration,
                },
            };
        } catch (error) {
            const duration = Date.now() - startTime;

            if (error.name === 'AbortError') {
                logger.error(
                    `[Ollama] 🔔 REQUEST #${requestNum} TIMED OUT after ${this.timeout}ms`
                );
                return { success: false, error: 'Request timeout', duration };
            }

            this._triggerOllamaList(error.message);
            logger.error(`[Ollama] 🔔 REQUEST #${requestNum} FAILED: ${error.message}`);
            return { success: false, error: error.message, duration };
        }
    }

    /**
     * Apply human-like imperfections to text
     * @param {string} text
     * @returns {string}
     */
    applyHumanization(text) {
        if (!text) return text;

        // 1. Abbreviation Replacer (10-15% chance per word match)
        // Only applies if the text is relatively short (casual context)
        const abbreviations = {
            // Common Conversational
            because: 'bc',
            probably: 'prolly',
            'right now': 'rn',
            'to be honest': 'tbh',
            "i don't know": 'idk',
            though: 'tho',
            people: 'ppl',
            please: 'pls',
            thanks: 'thx',
            'thank you': 'ty',
            you: 'u',
            are: 'r',
            really: 'rly',
            without: 'w/o',
            favorite: 'fav',
            seriously: 'srsly',
            'just kidding': 'jk',
            'by the way': 'btw',
            'in my opinion': 'imo',
            'laughing my ass off': 'lmao',
            'rolling on the floor laughing': 'rofl',
            'what the fuck': 'wtf',
            'oh my god': 'omg',
            'i do not care': 'idgaf',
            'never mind': 'nvm',
            'got to go': 'gtg',
            'talk to you later': 'ttyl',
            'be right back': 'brb',
            'for real': 'fr',
            'i mean': 'ion', // Gen-Z specific
            'going to': 'gonna',
            'want to': 'wanna',
            'have to': 'hafta',
            'kind of': 'kinda',
            'sort of': 'sorta',
            'let me': 'lemme',
            'give me': 'gimme',
            something: 'sth',
            everyone: 'every1',
            anyone: 'any1',
            someone: 'some1',
            before: 'b4',
            great: 'gr8',
            later: 'l8r',
            mate: 'm8',
            wait: 'w8',
            okay: 'ok',
            easy: 'ez',
            definitely: 'def',
            obviously: 'obv',
            actually: 'ack',
            message: 'msg',
            pic: 'pic',
            picture: 'pic',
            pictures: 'pics',
            about: 'abt',
            with: 'w/',
            tomorrow: 'tmrw',
            tonight: 'tn',
            yesterday: 'yday',
            morning: 'mornin',
            'good night': 'gn',
            'good morning': 'gm',
        };

        // Pre-compile regexes once for performance (was creating 64 regex objects per call)
        if (!this._abbrRegexes) {
            this._abbrRegexes = Object.entries(abbreviations).map(
                ([full]) => new RegExp(`\\b${full}\\b`, 'gi')
            );
        }

        // Simple word replacement with low probability
        let processed = text;
        const abbrEntries = Object.entries(abbreviations);
        for (let i = 0; i < abbrEntries.length; i++) {
            const [_full, abbr] = abbrEntries[i];
            const regex = this._abbrRegexes[i];
            regex.lastIndex = 0; // Reset regex state
            if (regex.test(processed)) {
                // 15% chance to abbreviate each found word
                if (Math.random() < 0.15) {
                    regex.lastIndex = 0;
                    processed = processed.replace(regex, abbr);
                }
            }
        }

        // 2. Lazy "I" Replacer (20% chance)
        // Replaces standalone "I" with "i" for casual feel
        if (Math.random() < 0.2) {
            processed = processed.replace(/\bI\b/g, 'i');
        }

        // 3. Missing Space Error (1% chance)
        // Simulates forgetting space after punctuation
        if (Math.random() < 0.01) {
            processed = processed.replace(/([,.]) /g, '$1');
        }

        return processed;
    }

    // 4. QWERTY Adjacency Map for Realistic Typos

    /**
     * Introduces realistic QWERTY keyboard typos
     * @param {string} text
     * @returns {string}
     */
    applyTypos(text) {
        if (!text || text.length < 5) return text;

        // QWERTY adjacency map (keys near each other)
        const adjacency = {
            q: 'wsa',
            w: 'qeasd',
            e: 'wrsdf',
            r: 'etdfg',
            t: 'ryfgh',
            y: 'tughj',
            u: 'yihjk',
            i: 'uojkl',
            o: 'ipkl',
            p: 'ol',
            a: 'qwsz',
            s: 'qweadzx',
            d: 'ersfcx',
            f: 'rtgvcd',
            g: 'tyhbvf',
            h: 'yujnbg',
            j: 'uikmnh',
            k: 'iolmj',
            l: 'opk',
            z: 'asx',
            x: 'zsdc',
            c: 'xdfv',
            v: 'cfgb',
            b: 'vghn',
            n: 'bhjm',
            m: 'njk',
        };

        let chars = text.split('');

        // Iterate through characters with a very low probability of typo
        for (let i = 0; i < chars.length; i++) {
            const char = chars[i].toLowerCase();

            // 0.1% chance per character to make a typo (approx 1 in 1000 keystrokes)
            if (adjacency[char] && Math.random() < 0.001) {
                const adjacentKeys = adjacency[char];
                const typo = adjacentKeys[Math.floor(Math.random() * adjacentKeys.length)];

                // Preserve original case
                chars[i] = chars[i] === char.toUpperCase() ? typo.toUpperCase() : typo;

                // Don't make multiple typos in a single short reply
                break;
            }
        }

        return chars.join('');
    }

    /**
     * Use /api/generate endpoint (for text-only models)
     */
    async _generateRequest(request, startTime, _requestNum = '?') {
        const endpoint = `${this.baseUrl}/api/generate`;

        // Handle system prompt
        const systemPrompt = request.system || request.systemPrompt;
        let promptText = request.prompt;

        // Check if this is LLaVA model for specific instruction formatting
        if (this.model.toLowerCase().includes('llava')) {
            if (systemPrompt) {
                promptText = `${systemPrompt}\n\n${request.prompt}`;
            }
            // Wrap in instruction format for LLaVA
            promptText = `[INST] ${promptText} [/INST]`;
            logger.debug(`[Ollama] Using LLaVA instruction format`);
        }

        const payload = {
            model: this.model,
            prompt: promptText,
            stream: false,
            options: {
                temperature: request.temperature || 0.7,
                num_predict: request.maxTokens || 2048,
            },
        };

        // Add system parameter if provided (Ollama native support)
        if (systemPrompt && !this.model.toLowerCase().includes('llava')) {
            payload.system = systemPrompt;
            logger.debug(`[Ollama] Using native system parameter`);
        }

        try {
            logger.debug(`[Ollama] Using generate endpoint for ${this.model}...`);

            const controller = new AbortController();
            const timeoutId = setTimeout(() => controller.abort(), this.timeout);

            const response = await fetch(endpoint, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(payload),
                signal: controller.signal,
            });

            clearTimeout(timeoutId);

            if (!response.ok) {
                const errorText = await response.text();
                throw new Error(`Ollama API error ${response.status}: ${errorText}`);
            }

            const data = await response.json();
            const duration = Date.now() - startTime;

            let content = data.response || '';

            // Post-processing to ensure no mentions or hashtags for Twitter tasks
            if (
                content &&
                (request.prompt?.includes('Tweet from') ||
                    request.systemPrompt?.includes('Twitter'))
            ) {
                // Remove mentions, hashtags, quotes
                let cleaned = content
                    .replace(/@\w+/g, '')
                    .replace(/#\w+/g, '')
                    .replace(/["']/g, '');

                // Only remove emojis if NOT explicitly allowed by prompt instructions
                // Strategies in twitter-reply-prompt.js use "EMOJI" keyword when allowing them
                const allowEmojis =
                    (request.prompt && request.prompt.includes('EMOJI')) ||
                    (request.systemPrompt && request.systemPrompt.includes('EMOJI'));

                if (!allowEmojis) {
                    cleaned = cleaned.replace(
                        /[\u{1F600}-\u{1F64F}\u{1F300}-\u{1F5FF}\u{1F680}-\u{1F6FF}\u{2600}-\u{26FF}\u{2700}-\u{27BF}\u{1F900}-\u{1F9FF}\u{1F1E6}-\u{1F1FF}]/gu,
                        ''
                    );
                }

                content = cleaned.trim();

                // Hard length limit: Take only the first sentence or first 100 chars
                if (
                    content.length > 100 ||
                    content.includes('.') ||
                    content.includes('!') ||
                    content.includes('?')
                ) {
                    const sentences = content.split(/[.!?]+/);
                    if (sentences.length > 0 && sentences[0].trim().length > 0) {
                        let punctuation = '';
                        if (content.includes('?')) {
                            punctuation = '?';
                        } else if (content.includes('.') || content.includes('!')) {
                            // 80% chance to delete the punctuation, 20% to use '.'
                            punctuation = Math.random() < 0.8 ? '' : '.';
                        }
                        content = sentences[0].trim() + punctuation;

                        // NEW: 15% chance for all-lowercase (human-like casual typing)
                        if (Math.random() < 0.15) {
                            content = content.toLowerCase();
                        }

                        // NEW: Apply additional humanization (abbreviations, slang)
                        content = this.applyHumanization(content);

                        // NEW: Apply realistic typos (very low probability)
                        content = this.applyTypos(content);
                    }
                }

                logger.debug(`[Ollama] Cleaned response: ${content}`);
            }

            logger.success(`[Ollama] Generate request completed in ${duration}ms`);

            return {
                success: true,
                content: content,
                model: data.model,
                duration: data.total_duration,
                metadata: {
                    eval_count: data.eval_count,
                    eval_duration: data.eval_duration,
                },
            };
        } catch (error) {
            const duration = Date.now() - startTime;

            if (error.name === 'AbortError') {
                logger.error(`[Ollama] Generate request timed out after ${this.timeout}ms`);
                return { success: false, error: 'Request timeout', duration };
            }

            this._triggerOllamaList(error.message);
            logger.error(`[Ollama] Generate request failed: ${error.message}`);
            return { success: false, error: error.message, duration };
        }
    }

    /**
     * Check if Ollama is accessible
     * @returns {Promise<boolean>}
     */
    async isReady() {
        await this.initialize();
        try {
            // fast check tags endpoint
            const res = await fetch(`${this.baseUrl}/api/tags`, {
                signal: AbortSignal.timeout(2000),
            });
            return res.ok;
        } catch (_e) {
            return false;
        }
    }
    resetStats() {
        // No internal stats to reset currently, but method required by interface
        logger.info('[Ollama] Statistics reset');
    }

    _triggerOllamaList(errorMessage) {
        if (!this._shouldTriggerOllamaList(errorMessage)) {
            return false;
        }
        const now = Date.now();
        if (now - this._recentOllamaListAt < 30000) {
            return false;
        }
        this._recentOllamaListAt = now;
        logger.warn(`[Ollama] ${errorMessage} Triggering 'ollama list'...`);

        const TIMEOUT_MS = 5000;
        let timedOut = false;
        const timeout = setTimeout(() => {
            timedOut = true;
            logger.warn(`[Ollama] 'ollama list' timed out after ${TIMEOUT_MS}ms`);
        }, TIMEOUT_MS);

        // Run 'ollama list' purely to wake up the system processes if asleep, ignore output for speed
        exec('ollama list', { windowsHide: true }, (err, _stdout, _stderr) => {
            clearTimeout(timeout);
            if (timedOut) return;
            if (err) {
                logger.warn(`[Ollama] 'ollama list' failed: ${err.message}`);
                return;
            }
        });
        return true;
    }

    _shouldTriggerOllamaList(errorMessage) {
        if (!errorMessage) return false;
        const msg = String(errorMessage).toLowerCase();
        return (
            msg.includes('model') &&
            (msg.includes('not found') ||
                msg.includes('not loaded') ||
                msg.includes('unknown') ||
                msg.includes('pull'))
        );
    }

    async wakeLocal() {
        return new Promise((resolve) => {
            try {
                exec('cmd /c ollama list', { windowsHide: true }, () => resolve(true));
            } catch {
                resolve(false);
            }
        });
    }

    async checkModel(testPrompt = 'Reply with exactly one word: "ok"', numPredict = 256) {
        await this.initialize();
        const start = Date.now();
        try {
            const isVisionModel =
                this.model.toLowerCase().includes('llava') ||
                this.model.toLowerCase().includes('vision');
            const controller = new AbortController();
            const timeoutId = setTimeout(() => controller.abort(), Math.min(this.timeout, 60000));
            if (isVisionModel) {
                const endpoint = `${this.baseUrl}/api/chat`;
                const payload = {
                    model: this.model,
                    messages: [{ role: 'user', content: testPrompt }],
                    stream: false,
                    options: { temperature: 0.1, num_predict: numPredict },
                };
                const res = await fetch(endpoint, {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify(payload),
                    signal: controller.signal,
                });
                clearTimeout(timeoutId);
                if (!res.ok) {
                    const t = await res.text();
                    return {
                        success: false,
                        error: `HTTP ${res.status}: ${t}`,
                        duration: Date.now() - start,
                    };
                }
                const data = await res.json();
                this._warmedUp = true;
                const content = data.message?.content || '';
                return { success: true, content, duration: Date.now() - start };
            } else {
                const endpoint = `${this.baseUrl}/api/generate`;
                const payload = {
                    model: this.model,
                    prompt: testPrompt,
                    stream: false,
                    options: { temperature: 0.1, num_predict: numPredict },
                };
                const res = await fetch(endpoint, {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify(payload),
                    signal: controller.signal,
                });
                clearTimeout(timeoutId);
                if (!res.ok) {
                    const t = await res.text();
                    return {
                        success: false,
                        error: `HTTP ${res.status}: ${t}`,
                        duration: Date.now() - start,
                    };
                }
                const data = await res.json();
                this._warmedUp = true;
                const content = data.response || '';
                return { success: true, content, duration: Date.now() - start };
            }
        } catch (error) {
            return { success: false, error: error.message, duration: Date.now() - start };
        }
    }
}

export default OllamaClient;
