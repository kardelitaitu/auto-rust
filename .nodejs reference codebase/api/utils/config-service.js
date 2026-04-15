/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Centralized Config Service
 * Unifies config access across the codebase with environment variable override support
 *
 * Usage:
 *   import { config } from './utils/config-service.js';
 *
 *   const activity = await config.getTwitterActivity();
 *   const limits = await config.getEngagementLimits();
 *   const timing = await config.getTiming();
 *   const humanization = await config.getHumanization();
 *
 * @module utils/config-service
 */

import { getSettings } from './configLoader.js';
import { createLogger } from '../core/logger.js';

const logger = createLogger('config-service.js');

/**
 * Default values (fallback when settings.json doesn't have values)
 */
const DEFAULTS = {
    twitter: {
        activity: {
            defaultCycles: 10,
            defaultMinDuration: 300,
            defaultMaxDuration: 540,
            engagementLimits: {
                replies: 3,
                retweets: 1,
                quotes: 1,
                likes: 5,
                follows: 2,
                bookmarks: 2,
            },
        },
        reply: {
            probability: 0.6,
            minChars: 10,
            maxChars: 200,
            emojiChance: 0.3,
            questionChance: 0.2,
        },
        quote: {
            probability: 0.2,
        },
        timing: {
            warmupMin: 2000,
            warmupMax: 15000,
            scrollMin: 300,
            scrollMax: 700,
            scrollPauseMin: 1500,
            scrollPauseMax: 4000,
            readMin: 5000,
            readMax: 15000,
            diveRead: 10000,
            globalScrollMultiplier: 1.0,
        },
        phases: {
            warmupPercent: 0.1,
            activePercent: 0.7,
            cooldownPercent: 0.2,
        },
    },
    humanization: {
        mouse: {
            speed: { mean: 1.0, deviation: 0.2 },
            jitter: { x: 10, y: 5 },
        },
        typing: {
            keystrokeDelay: { mean: 80, deviation: 40 },
            errorRate: 0.05,
        },
        session: {
            minMinutes: 5,
            maxMinutes: 9,
            breakBetween: { min: 30, max: 60 },
        },
    },
    llm: {
        local: {
            vllm: { enabled: false },
            ollama: { enabled: false },
            docker: { enabled: false },
        },
        cloud: {
            enabled: true,
        },
    },
};

/**
 * Environment variable mappings
 * These override settings.json when set
 */
const ENV_OVERRIDES = {
    twitter: {
        activity: {
            defaultCycles: 'TWITTER_ACTIVITY_CYCLES',
            defaultMinDuration: 'TWITTER_MIN_DURATION',
            defaultMaxDuration: 'TWITTER_MAX_DURATION',
        },
        reply: {
            probability: 'TWITTER_REPLY_PROBABILITY',
        },
        quote: {
            probability: 'TWITTER_QUOTE_PROBABILITY',
        },
        timing: {
            globalScrollMultiplier: 'GLOBAL_SCROLL_MULTIPLIER',
            warmupMin: 'TWITTER_WARMUP_MIN',
            warmupMax: 'TWITTER_WARMUP_MAX',
            scrollMin: 'TWITTER_SCROLL_MIN',
            scrollMax: 'TWITTER_SCROLL_MAX',
        },
    },
    humanization: {
        mouse: {
            speed: 'HUMAN_MOUSE_SPEED',
            jitter: 'HUMAN_MOUSE_JITTER',
        },
        typing: {
            keystrokeDelay: 'HUMAN_TYPING_DELAY',
            errorRate: 'HUMAN_ERROR_RATE',
        },
        session: {
            minMinutes: 'SESSION_MIN_MINUTES',
            maxMinutes: 'SESSION_MAX_MINUTES',
        },
    },
    llm: {
        local: {
            vllm: { enabled: 'VLLM_ENABLED' },
            ollama: { enabled: 'OLLAMA_ENABLED' },
            docker: { enabled: 'DOCKER_ENABLED' },
        },
        cloud: {
            enabled: 'LLM_CLOUD_ENABLED',
        },
    },
};

/**
 * Get value from environment or settings.json
 */
function getFromEnvOrSettings(envKey, settingsValue, type = 'string') {
    const envValue = process.env[envKey];
    if (envValue === undefined || envValue === '') {
        return settingsValue;
    }

    try {
        switch (type) {
            case 'number':
                return parseFloat(envValue);
            case 'boolean':
                return envValue.toLowerCase() === 'true';
            case 'json':
                return JSON.parse(envValue);
            default:
                return envValue;
        }
    } catch (error) {
        logger.warn(`[Config] Failed to parse env var ${envKey}: ${error.message}`);
        return settingsValue;
    }
}

/**
 * Apply environment overrides to a settings object
 */
function applyEnvOverrides(settings, envMap, path = '') {
    // Ensure settings is an object (initialize if null/undefined)
    const safeSettings = settings || {};

    const result = Array.isArray(safeSettings) ? [...safeSettings] : { ...safeSettings };

    for (const [key, value] of Object.entries(envMap)) {
        const currentPath = path ? `${path}.${key}` : key;

        if (typeof value === 'object' && value !== null && !Array.isArray(value)) {
            // Nested override - recurse
            // Create nested structure if missing
            if (!result[key]) {
                result[key] = {};
            }

            if (typeof result[key] === 'object') {
                result[key] = applyEnvOverrides(result[key], value, currentPath);
            }
        } else if (typeof value === 'string') {
            // Direct env var mapping
            // If key doesn't exist, we can't infer type from value (undefined).
            // Default to string, or check if we can infer from defaults?
            // Current implementation uses result[key] type.
            // If result[key] is undefined, type is 'undefined'.
            // getFromEnvOrSettings handles undefined?

            const currentValue = result[key];
            const type = typeof currentValue !== 'undefined' ? typeof currentValue : 'string';

            // If value is missing in settings, we might want to try to cast based on env var content?
            // But getFromEnvOrSettings takes a type.
            // For now, let's stick to string if missing, or maybe we should look up DEFAULTS?
            // Looking up defaults is hard here because we don't know where we are in the tree easily without mapping.

            // Improvement: Try to infer type from string value (true/false/numbers) if current value is undefined
            let effectiveType = type;
            if (currentValue === undefined) {
                const envVal = process.env[value];
                if (envVal === 'true' || envVal === 'false') effectiveType = 'boolean';
                else if (!isNaN(parseFloat(envVal))) effectiveType = 'number';
            }

            const newValue = getFromEnvOrSettings(value, currentValue, effectiveType);
            if (newValue !== undefined) {
                result[key] = newValue;
            }
        }
    }

    return result;
}

/**
 * Centralized Config Service
 */
class ConfigService {
    constructor() {
        this._settings = null;
        this._initialized = false;
        this._initPromise = null;
    }

    /**
     * Initialize and load settings
     */
    async init() {
        if (this._initialized) return;
        if (this._initPromise) return this._initPromise;

        this._initPromise = (async () => {
            try {
                const settings = await getSettings();
                this._settings = settings || {};
                this._initialized = true;
                logger.info('[Config] Service initialized');
            } catch (error) {
                logger.warn(`[Config] Failed to load settings: ${error.message}`);
                this._settings = {};
                this._initialized = true;
            } finally {
                this._initPromise = null;
            }
        })();

        return this._initPromise;
    }

    /**
     * Ensure service is initialized
     */
    async ensureInit() {
        if (!this._initialized) {
            await this.init();
        }
    }

    /**
     * Get full settings (from settings.json with env overrides)
     */
    async getSettings() {
        await this.ensureInit();

        if (!this._settings._envApplied) {
            this._settings = applyEnvOverrides(this._settings, ENV_OVERRIDES);
            this._settings._envApplied = true;
        }

        return this._settings;
    }

    /**
     * Get a specific config section with defaults
     */
    async get(section, subsection = null) {
        const settings = await this.getSettings();

        if (!subsection) {
            return this._getWithDefaults(settings, section, DEFAULTS[section]);
        }

        const sectionData = settings[section];
        const defaultsData = DEFAULTS[section]?.[subsection];
        return this._getWithDefaults(sectionData, subsection, defaultsData);
    }

    /**
     * Get value with defaults fallback
     */
    _getWithDefaults(data, key, defaults) {
        if (!defaults) {
            return data?.[key];
        }

        const value = data?.[key];
        if (value === undefined || value === null) {
            return defaults;
        }

        if (typeof defaults === 'object' && defaults !== null && typeof value === 'object') {
            return { ...defaults, ...value };
        }

        return value;
    }

    // =====================================
    // Twitter Activity Config
    // =====================================

    async getTwitterActivity() {
        return this.get('twitter', 'activity');
    }

    async getEngagementLimits() {
        await this.ensureInit();
        const eng = this._settings?.twitter?.engagement;
        const D = DEFAULTS.twitter.activity.engagementLimits;
        if (eng) {
            return {
                replies: eng.maxReplies ?? D.replies,
                retweets: eng.maxRetweets ?? D.retweets,
                quotes: eng.maxQuotes ?? D.quotes,
                likes: eng.maxLikes ?? D.likes,
                follows: eng.maxFollows ?? D.follows,
                bookmarks: eng.maxBookmarks ?? D.bookmarks,
            };
        }
        return D;
    }

    async getReplyConfig() {
        return this.get('twitter', 'reply');
    }

    async getQuoteConfig() {
        return this.get('twitter', 'quote');
    }

    async getTiming() {
        return this.get('twitter', 'timing');
    }

    async getSessionPhases() {
        return this.get('twitter', 'phases');
    }

    async getGlobalScrollMultiplier() {
        const timing = await this.getTiming();
        return timing?.globalScrollMultiplier ?? 1.0;
    }

    // =====================================
    // Humanization Config
    // =====================================

    async getHumanization() {
        return this.get('humanization') || DEFAULTS.humanization;
    }

    async getMouseConfig() {
        return this.get('humanization', 'mouse') || DEFAULTS.humanization.mouse;
    }

    async getTypingConfig() {
        return this.get('humanization', 'typing') || DEFAULTS.humanization.typing;
    }

    async getSessionConfig() {
        return this.get('humanization', 'session') || DEFAULTS.humanization.session;
    }

    // =====================================
    // LLM Config
    // =====================================

    async getLLMConfig() {
        return this.get('llm') || DEFAULTS.llm;
    }

    async isLocalLLMEnabled() {
        const llm = await this.getLLMConfig();
        return (
            llm?.local?.vllm?.enabled ||
            llm?.local?.ollama?.enabled ||
            llm?.local?.docker?.enabled ||
            false
        );
    }

    async isCloudLLMEnabled() {
        const llm = await this.getLLMConfig();
        return llm?.cloud?.enabled !== false; // Default to true
    }

    // =====================================
    // Convenience Methods
    // =====================================

    /**
     * Get config for a specific profile type
     */
    async getProfileConfig(profileType) {
        const profiles = {
            NewsJunkie: { dive: 0.45, like: 0.6, follow: 0.05 },
            Casual: { dive: 0.25, like: 0.35, follow: 0.02 },
            PowerUser: { dive: 0.55, like: 0.7, follow: 0.08 },
            Balanced: { dive: 0.35, like: 0.5, follow: 0.03 },
        };

        return profiles[profileType] || profiles['Balanced'];
    }

    /**
     * Reset and reload (for testing)
     */
    async reload() {
        this._initialized = false;
        this._settings = null;
        await this.init();
    }
}

// Singleton instance
export const config = new ConfigService();

// Convenience exports for common access patterns
export const getTwitterActivity = () => config.getTwitterActivity();
export const getEngagementLimits = () => config.getEngagementLimits();
export const getReplyConfig = () => config.getReplyConfig();
export const getQuoteConfig = () => config.getQuoteConfig();
export const getTiming = () => config.getTiming();
export const getGlobalScrollMultiplier = () => config.getGlobalScrollMultiplier();
export const getHumanization = () => config.getHumanization();
export const getMouseConfig = () => config.getMouseConfig();
export const getTypingConfig = () => config.getTypingConfig();
export const getSessionConfig = () => config.getSessionConfig();
export const getLLMConfig = () => config.getLLMConfig();

export default config;
