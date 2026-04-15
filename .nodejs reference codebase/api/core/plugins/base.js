/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Base Plugin Class
 * Standard interface for all Auto-AI plugins.
 * @module api/plugins/base
 */

export class BasePlugin {
    /**
     * @param {string} name - Unique plugin name
     * @param {string} version - Semantic version
     */
    constructor(name, version = '1.0.0') {
        if (!name) throw new Error('Plugin must have a name');
        this.name = name;
        this.version = version;
        this.context = null; // Will be set by manager
        this.enabled = false;
    }

    /**
     * Lifecycle: Called when plugin is registered
     * @param {object} context - API Context
     */
    async onLoad(context) {
        this.context = context;
    }

    /**
     * Lifecycle: Called when plugin is enabled
     */
    async onEnable() {
        this.enabled = true;
    }

    /**
     * Lifecycle: Called when plugin is disabled
     */
    async onDisable() {
        this.enabled = false;
    }

    /**
     * Lifecycle: Called when plugin is unregistered
     */
    async onUnload() {
        this.context = null;
    }

    /**
     * Determines if the plugin should be enabled for the given URL
     * @param {string} url - Current page URL
     * @returns {boolean}
     */
    matches(_url) {
        return true; // Default: active on all URLs
    }

    /**
     * Return map of hook handlers
     * @returns {Object.<string, Function>}
     */
    getHooks() {
        return {};
    }
}
