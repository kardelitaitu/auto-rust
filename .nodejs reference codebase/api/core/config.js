/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Centralized Configuration Manager
 * Manages global settings, environment variables, timeouts, and LLM configs.
 *
 * @module api/core/config
 */

import { getSettings as getRawSettings } from '../utils/config.js';
import { createLogger } from './logger.js';

const logger = createLogger('api/core/config.js');

const DEFAULTS = {
    agent: {
        llm: {
            baseUrl: 'http://localhost:11434',
            model: 'qwen3.5:4b',
            temperature: 0.7,
            maxTokens: 2048,
            contextLength: 4096,
            timeoutMs: 120000,
            useVision: true,
            serverType: 'ollama',
            think: false,
            bypassHealthCheck: false,
        },
        runner: {
            maxSteps: 20,
            stepDelay: 2000,
            adaptiveDelay: true,
        },
    },
    timeouts: {
        navigation: 30000,
        element: 10000,
    },
};

export class ConfigurationManager {
    constructor() {
        this._config = null;
        this._overrides = {};
    }

    /**
     * Initialize and merge configurations.
     */
    async init() {
        if (this._config) return this._config;

        try {
            const raw = await getRawSettings();

            // Merge defaults with raw settings
            this._config = {
                ...raw,
                agent: {
                    llm: {
                        ...DEFAULTS.agent.llm,
                        ...(raw.agent?.llm || {}),
                    },
                    runner: {
                        ...DEFAULTS.agent.runner,
                        ...(raw.agent?.runner || {}),
                    },
                },
                timeouts: {
                    ...DEFAULTS.timeouts,
                    ...raw.timeouts,
                },
            };
        } catch (_e) {
            logger.warn('Failed to load raw settings, using defaults');
            this._config = DEFAULTS;
        }

        return this._config;
    }

    /**
     * Get a configuration value by dot-notation path (e.g., 'agent.llm.model')
     * @param {string} path
     * @param {*} defaultValue
     */
    get(path, defaultValue = undefined) {
        if (!this._config) {
            logger.warn('ConfigurationManager not initialized, returning default');
            return defaultValue;
        }

        // Apply temporary override if exists
        if (this._overrides[path] !== undefined) {
            return this._overrides[path];
        }

        const keys = path.split('.');
        let current = this._config;

        for (const key of keys) {
            if (current === null || current === undefined) {
                return defaultValue;
            }
            current = current[key];
        }

        return current !== undefined ? current : defaultValue;
    }

    /**
     * Temporary overrides for a specific run/persona.
     * @param {string} path
     * @param {*} value
     */
    setOverride(path, value) {
        this._overrides[path] = value;
    }

    /**
     * Clear all current overrides.
     */
    clearOverrides() {
        this._overrides = {};
    }

    /**
     * Get full materialized config
     */
    getFullConfig() {
        return this._config || DEFAULTS;
    }

    _getDefaults() {
        return DEFAULTS;
    }
}

export const configManager = new ConfigurationManager();
export default configManager;
